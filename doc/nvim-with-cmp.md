# neovim + nvim-cmp 配置示例

在此给出一些 `nvim-cmp` 的相关配置注意事项：

- [按需调整 cmp 的排序](#按需调整-cmp-的排序)
- [还原输入法体验](#还原输入法体验)
  - [选词功能](#选词功能)
  - [中文标点](#中文标点)
  - [输入过快补全列表消失](#输入过快补全列表消失)
  - [五笔或者双形用户](#五笔或者双形用户)
    - [顶字上屏](#顶字上屏)

## 按需调整 cmp 的排序

cmp 会对补全候选进行排序,
为了更好的使用体验, 还需要配置 nvim-cmp

```lua
-- cmp 會自己排序, 要配置裏把 sort_text 手動提前
local cmp = require 'cmp'
local compare = require 'cmp.config.compare'
cmp.setup {
  -- 其他设置 blabla
  -- ......

  -- 设置排序顺序
  sorting = {
    comparators = {
      compare.sort_text,
      compare.offset,
      compare.exact,
      compare.score,
      compare.recently_used,
      compare.kind,
      compare.length,
      compare.order,
    }
  },

  -- 其他配置 blabla
  -- ......
}
```

## 还原输入法体验

你可以把 rime-ls 当成单纯的代码补全来用，也可以进行额外的配置让它更像普通的输入法。

这部分配置来自 Kaiser-Yang 的博客文章 [使用 nvim-cmp + rime-ls 配置 nvim 中文输入法](https://kaiser-yang.github.io/blog/2024/nvim-input-method/)。

完整配置可以参考 [Kaiser-Yang 的 dotfiles](https://github.com/Kaiser-Yang/dotfiles/commit/3f027f0e2ebd7e123c2efae0a1b2d3d843756fa6) 

为了不影响其他不需要中文输入地方的体验，这一部分的实现逻辑是在启动 rime-ls 时增加 key-mapping ，
关闭时删除 key-mapping 实现。因此配置的整体结构如下：

```lua
map.set({ 'n', 'i' }, '<c-space>', function()
  -- We must check the status before the toggle
  if vim.g.rime_enabled then
    -- delete key mappings here
  else
    -- add key mappings here
  end
  -- toggle Rime
  vim.cmd('RimeToggle')
end, opts())
```

删除按键部分没有难度，后文将重点讨论添加按键部分。

给出几个下面会用到的 helper 函数：

```lua
local cmp = require'cmp'

-- NOTE: there is no 'z' in the alphabet
-- The alphabet your schema used
local alphabet = "abcdefghijklmnopqrstuvwxy"

-- Feed keys with term code
-- keys: the keys to feed
-- mode: n for no-remap, m for re-map
local function feedkeys(keys, mode)
    local termcodes = vim.api.nvim_replace_termcodes(keys, true, true, true)
    vim.api.nvim_feedkeys(termcodes, mode, false)
end

-- return true if the content contains Chinese characters
local function contain_chinese_character(content)
    for i = 1, #content do
        local byte = string.byte(content, i)
        if byte >= 0xE4 and byte <= 0xE9 then
            return true
        end
    end
    return false
end

-- Return true if the cmp-entry is acceptable
-- you can define some other rules here to accept more
-- in this situation we only think the entry
-- whose content contains Chinese or is like time is acceptable
local function rime_entry_acceptable(entry)
    return entry ~= nil and entry.source.name == "nvim_lsp"
        and entry.source.source.client.name == "rime_ls"
        and (entry.word:match("%d%d%d%d%-%d%d%-%d%d %d%d:%d%d:%d%d%") or contain_chinese_character(entry.word))
end

-- Return the first n entries which make rime_entry_acceptable(entry) return true
-- we call those entries rime-ls entries below
local function get_n_rime_ls_entries(n)
    if not cmp.visible() then
        return {}
    end
    local entries = cmp.get_entries()
    local result = {}
    if entries == nil or #entries == 0 then
        return result
    end
    for _, entry in ipairs(entries) do
        if rime_entry_acceptable(entry) then
            result[#result + 1] = entry
            if #result == n then
                break;
            end
        end
    end
    return result
end

-- Confirm the rime-ls entry
-- We use nvim_set_current_line to get the completion text,
-- because cmp.comfirm is slow and configured with throttle
-- for which make the completion list not pop up when typing fast
local function confirm_rime_ls_entry(entry)
    local line = vim.api.nvim_get_current_line()
    local cursor_column = vim.api.nvim_win_get_cursor(0)[2]
    local start = entry.source_insert_range.start.character
    local new_line =
        string.sub(line, 1, start) ..
        entry.word ..
        string.sub(line, cursor_column + 1)
    vim.api.nvim_set_current_line(new_line)
    vim.api.nvim_win_set_cursor(0, { vim.api.nvim_win_get_cursor(0)[1], start + #entry.word })
end

-- check if the content of txt is all in allowed
local function match_alphabet(txt, allowed)
    return string.match(txt, '^[' .. allowed .. ']+$') ~= nil
end

-- check if the last character will trigger rime-ls
local function last_character_in_alphabet()
    local cursor_column = vim.api.nvim_win_get_cursor(0)[2]
    if cursor_column == 0 then
        return false;
    end
    local trigger_alphabet = alphabet
    -- INFO:
    -- If you bind number with select_or_confirm_rime(x, false)
    -- uncomment this line to select more than once
    -- trigger_alphabet = trigger_alphabet .. "1234567890"

    return match_alphabet(string.sub(vim.api.nvim_get_current_line(),
            cursor_column,
            cursor_column),
        trigger_alphabet)
end

-- index: the index-th rime-ls entry
-- select_with_no_num: set true to upload the index-th rime-ls entry directly
--                     set false to feed index as number at first, then upload if only one rime-ls entry
-- return true if the number of rime-ls entries is enough
local function select_or_confirm_rime(index, select_with_no_num)
    if not last_character_in_alphabet() then
        return false
    end
    local rime_ls_entries = get_n_rime_ls_entries(index)
    if #rime_ls_entries < index then
        return false
    end
    if select_with_no_num then
        confirm_rime_ls_entry(rime_ls_entries[index])
        return true
    end
    local cursor_column = vim.api.nvim_win_get_cursor(0)[2]
    local line = vim.api.nvim_get_current_line()
    local new_line = string.sub(line, 1, cursor_column) .. tostring(index) .. string.sub(line, cursor_column + 1)
    vim.api.nvim_set_current_line(new_line)
    vim.api.nvim_win_set_cursor(0, { vim.api.nvim_win_get_cursor(0)[1], cursor_column + 1 })
    -- must trigger complete manually here,
    -- otherwise we can not get the new list after inputting a number
    cmp.complete({ config = { sources = { { name = 'nvim_lsp' } } } })
    local first_rime_ls_entry = get_n_rime_ls_entries(2)
    if #first_rime_ls_entry ~= 1 then
        return true
    end
    confirm_rime_ls_entry(first_rime_ls_entry[1])
    return true;
end

-- When k and v are not equal and v is not nil, feed v with remap mode,
-- otherwise feed k with no-remap mode.
-- This function is used when the original key is mapped to other functionality
local function feed_key_helper(k, v)
    if v == nil or k == v then
        feedkeys(k, 'n')
    else
        feedkeys(v, 'm')
    end
end
```

**注意**：后文使用的按键可能被某些插件绑定了一些其他的功能
（例如 `auto-pairs` 绑定 `<space>` 来在括号中插入两个空格），或者你自己绑定了一些功能，
如果你想同时保留两部分功能，你可以参考 <a href="#upload-word">选词功能</a> 部分的配置。

### <a id="upload-word">选词功能</a>

相关 issue ：[用数字选词以后还需要一次空格才能上屏幕](https://github.com/wlh320/rime-ls/issues/20)。

这里以空格首选，分号次选，单引号三选，数字键依次对应候选 1-9 为例，
先定义按键列表：

```lua
local mapped_key = {
    ['<space>'] = '<f30>',
    [';'] = ';',
    ["'"] = '<f31>',
    ['1'] = '1',
    ['2'] = '2',
    ['3'] = '3',
    ['4'] = '4',
    ['5'] = '5',
    ['6'] = '6',
    ['7'] = '7',
    ['8'] = '8',
    ['9'] = '9',
}
```

上面的空格和单引号已经被绑定了，这里以空格为例，先进行如下操作：

```lua
-- find a key will never be used, here we use <f30>
-- bind <f30> to the functionality that <space> should have
-- bind <space> to the functionality that <space> should have
map.set({ 'i' }, '<f30>', '<c-]><c-r>=AutoPairsSpace()<cr>', opts())
map.set({ 'i' }, '<space>', '<c-]><c-r>=AutoPairsSpace()<cr>', opts())
```

绑定上屏功能：

```lua
map.set({ 'i' }, k, function()
    -- when having selected an entry we do not upload
    -- if you want to upload, comment those lines
    if cmp.visible() and cmp.get_selected_entry() ~= nil then
        feed_key_helper(k, v)
        return
    end
    -- NOTE: if you want to use a key to select more than once change true to false
    if k == '<space>' and not select_or_confirm_rime(1, true) or
        k == ';' and not select_or_confirm_rime(2, true) or
        k == "'" and not select_or_confirm_rime(3, true) or
        k:match('[0-9]') and not select_or_confirm_rime(tonumber(k), true) then
        feed_key_helper(k, v);
    end
end, opts())
```

`select_or_confirm_rime(1, true)` 意味着会直接 `confirm` 第一个 rime-ls 候选词，
这表示你不能再进行后续后续选择。如果你喜欢输入句子，而不是单个词，你可以将 `true` 改为 `false`。

Kaiser-Yang 建议非语句流形码用户全部使用 `select_or_confirm_rime(x, true)`；
其他用户空格和二三候选使用 `select_or_confirm_rime(x, true)`，
数字按键使用 `select_or_confirm_rime(x, false)`。

### 中文标点

相关 issue ：[英文标点符号后空格才能触发补全](https://github.com/wlh320/rime-ls/issues/10)。

如果使用 `rime-ls` 进行标点输入，因为 lsp 的特性必须伴随一次选择才能触发上屏，
所以在输入标点时会有一些不同的体验。这里提供一种解决方案，即在输入标点时，先输入标点，
然后再输入空格来实现插入中文标点, 首先设置 rime-ls 为西文标点模式，然后增加如下配置：

```lua
local mapped_punc = {
    [','] = '，',
    ['.'] = '。',
    [':'] = '：',
    [';'] = '；',
    ['?'] = '？',
    ['\\'] = '、'
    -- ...
    -- add more you want here
}
-- Chinese punctuations
for k, v in pairs(mapped_punc) do
    map.set({ 'i' }, k .. '<space>', function()
        -- when typing comma or period with space,
        -- upload the first rime_ls entry and make comma and period in Chinese edition
        -- if you don't want this just comment those lines
        if k == ',' or k == '.' then
            select_or_confirm_rime(1, true)
        end

        feed_key_helper(v, v)
    end, opts())
end
```

### 输入过快补全列表消失

相关 issue ：[feat: add rime-ls.get-first-candidate command](https://github.com/wlh320/rime-ls/pull/41)。

使用以上配置后，该问题已经解决。

### 五笔或者双形用户

确保 `max_token` 为最大码长，`always_incomplete` 为 `true`，这样可以保证每次输入都会重新生成候选词。

```lua
require('lspconfig').rime_ls.setup {
  init_options = {
    -- ...
    max_tokens = 4, -- 强制在删除到4字的时候重建一次候选词，避免用退格造成的空列表的问题
    always_incomplete = true, -- 将 incomplete 永远设为 true，防止任何时候的过滤代替候选词重建
    -- ...
  },
  on_attach = rime_on_attach,
  capabilities = capabilities,
}
```

#### 顶字上屏

相关 issue ：[如何实现顶字上屏](https://github.com/wlh320/rime-ls/issues/43)。


```lua
-- the max_code of your schema, wubi and flypy are 4
local max_code = 4

local function auto_upload_on_max_code(k)
    local cursor_column = vim.api.nvim_win_get_cursor(0)[2]
    if cursor_column >= max_code then
        local content_before_cursor = string.sub(vim.api.nvim_get_current_line(), 1, cursor_column)
        local code = string.sub(content_before_cursor, cursor_column - max_code + 1, cursor_column)
        if match_alphabet(code, alphabet) then
            -- This is for wubi users using 'z' as reverse look up
            -- If 'z' is you alphabet key, just comment this condition check
            if not string.match(content_before_cursor, 'z[' .. alphabet .. ']*$') then
                local first_rime_ls_entry = get_n_rime_ls_entries(1)
                if #first_rime_ls_entry ~= 1 then
                    -- clear the wrong code
                    -- uncomment this if you don't want to clear the wrong code
                    vim.api.nvim_win_set_cursor(0, { vim.api.nvim_win_get_cursor(0)[1], cursor_column - max_code })
                    vim.api.nvim_set_current_line(string.sub(content_before_cursor, 1, cursor_column - max_code) ..
                        string.sub(content_before_cursor, cursor_column + 1))
                else
                    confirm_rime_ls_entry(first_rime_ls_entry[1])
                    -- update the new cursor column
                    cursor_column = vim.api.nvim_win_get_cursor(0)[2]
                end
            end
        end
    end
    local line = vim.api.nvim_get_current_line()
    local new_line = string.sub(line, 1, cursor_column) .. k .. string.sub(line, cursor_column + 1)
    vim.api.nvim_set_current_line(new_line)
    vim.api.nvim_win_set_cursor(0, { vim.api.nvim_win_get_cursor(0)[1], cursor_column + 1 })
end

for i = 1, #alphabet do
    local k = alphabet:sub(i, i)
    map.set({ 'i' }, k, function() auto_upload_on_max_code(k) end, opts())
end
```

