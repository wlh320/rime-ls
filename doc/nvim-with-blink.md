# neovim + blink.cmp 配置示例

在此给出一些 `blink.cmp` 的相关配置注意事项（以 `blink.cmp v0.8.0` 为例）：

* [开启 long_filter_text](#开启-long_filter_text)
* [修改默认的 LSP 过滤规则](#修改默认的-lsp-过滤规则)
* [还原输入法体验](#还原输入法体验)
    * [选词功能](#选词功能)
    * [五笔或者双形用户](#五笔或者双形用户)
        * [顶字上屏](#顶字上屏)
    * [完整配置](#完整配置)

# 开启 long_filter_text

`blink.cmp` 对于候选词的过滤比较严格，需要将 rime-ls 的 `long_filter_text` 配置设置为 `true`。

# 修改默认的 LSP 过滤规则

`blink.cmp` 将 LSP 服务器提供的 Text 类型补全也过滤掉了，为了启用 rime-ls，需要修改相关配置:

```lua
    sources = {
        -- ...
        providers = {
            lsp = {
                transform_items = function(_, items)
                    -- the default transformer will do this
                    for _, item in ipairs(items) do
                        if item.kind == require('blink.cmp.types').CompletionItemKind.Snippet then
                            item.score_offset = item.score_offset - 3
                        end
                    end

                    -- you can define your own filter for rime item
                    -- for example only accept the item whose 'label' contains no punctuations
                    -- return vim.tbl_filter(function(item)
                    --     return not is_rime_item(item) or rime_item_acceptable(item)
                    -- end, items)
                    -- or just return items
                    return items
                end
            }
        },
        -- ...
    },

```

# 还原输入法体验

这部分配置来自 [使用 nvim-cmp + rime-ls 配置 nvim 中文输入法](https://kaiser-yang.github.io/blog/2024/nvim-input-method/)。

先定义几个功能函数：

```lua
-- Check if item is acceptable, you can define rules by yourself
function rime_item_acceptable(item)
    return
        not contains_unacceptable_character(item.label)
        or
        item.label:match("%d%d%d%d%-%d%d%-%d%d %d%d:%d%d:%d%d%")
end

-- Get the first n rime items' index in the completion list
function get_n_rime_item_index(n, items)
    if items == nil then
        items = require('blink.cmp.completion.list').items
    end
    local result = {}
    if items == nil or #items == 0 then
        return result
    end
    for i, item in ipairs(items) do
        if is_rime_item(item) and rime_item_acceptable(item) then
            result[#result + 1] = i
            if #result == n then
                break;
            end
        end
    end
    return result
end
```

## 选词功能

相关 issue ：[用数字选词以后还需要一次空格才能上屏幕](https://github.com/wlh320/rime-ls/issues/20)。

数字键依次对应候选 1-9：

```lua
-- auto upload when there is only one rime item after inputting a number
require('blink.cmp.completion.list').show_emitter:on(function(event)
    if not vim.g.rime_enabled then return end
    local col = vim.fn.col('.') - 1
    -- if you don't want use number to select, change the match pattern by yourself
    if event.context.line:sub(col, col):match("%d") == nil then return end
    local rime_item_index = get_n_rime_item_index(2, event.items)
    if #rime_item_index ~= 1 then return end
    require('blink.cmp').accept({ index = rime_item_index[1] })
end)
```

空格首选，分号次选，单引号三选：

```lua
keymap = {
    ['<space>'] = {
        function(cmp)
            if not vim.g.rime_enabled then return false end
            local rime_item_index = get_n_rime_item_index(1)
            if #rime_item_index ~= 1 then return false end
            -- If you want to select more than once,
            -- just update this cmp.accept with vim.api.nvim_feedkeys('1', 'n', true)
            -- The rest can be updated similarly
            return cmp.accept({ index = rime_item_index[1] })
        end,
        'fallback' },
    [';'] = {
        -- FIX: can not work when binding ;<space> to other functionality
        -- such inputting a Chinese punctuation
        function(cmp)
            if not vim.g.rime_enabled then return false end
            local rime_item_index = get_n_rime_item_index(2)
            if #rime_item_index ~= 2 then return false end
            return cmp.accept({ index = rime_item_index[2] })
        end, 'fallback' },
    ['\''] = {
        function(cmp)
            if not vim.g.rime_enabled then return false end
            local rime_item_index = get_n_rime_item_index(3)
            if #rime_item_index ~= 3 then return false end
            return cmp.accept({ index = rime_item_index[3] })
        end, 'fallback' }
}
```

## 五笔或者双形用户

确保 `always_incomplete` 为 `true`，这样可以保证每次输入都会重新生成候选词。

```lua
require('lspconfig').rime_ls.setup {
  init_options = {
    -- ...
    always_incomplete = true, -- 将 incomplete 永远设为 true，防止任何时候的过滤代替候选词重建
    -- ...
  },
  on_attach = rime_on_attach,
  capabilities = capabilities,
}
```

### 顶字上屏

相关 issue ：[如何实现顶字上屏](https://github.com/wlh320/rime-ls/issues/43)。

```lua
-- NOTE: there is no 'z' in the alphabet
local alphabet = "abcdefghijklmnopqrstuvwxy"
local max_code = 4
-- Select first entry when typing more than max_code
for i = 1, #alphabet do
    local k = alphabet:sub(i, i)
    vim.keymap.set({ 'i' }, k, function()
        local cursor_column = vim.api.nvim_win_get_cursor(0)[2]
        local confirmed = false
        if vim.g.rime_enabled and cursor_column >= max_code then
            local content_before_cursor = string.sub(vim.api.nvim_get_current_line(), 1, cursor_column)
            local code = string.sub(content_before_cursor, cursor_column - max_code + 1, cursor_column)
            if match_alphabet(code) then
                -- This is for wubi users using 'z' as reverse look up
                if not string.match(content_before_cursor, 'z[' .. alphabet .. ']*$') then
                    local first_rime_item_index = get_n_rime_item_index(1)
                    if #first_rime_item_index ~= 1 then
                        -- clear the wrong code
                        for _ = 1, max_code do
                            feedkeys('<bs>', 'n')
                        end
                    else
                        require('blink.cmp').accept({ index = first_rime_item_index[1] })
                        confirmed = true
                    end
                end
            end
        end
        if confirmed then
            vim.schedule(function() feedkeys(k, 'n') end)
        else
            feedkeys(k, 'n')
        end
    end, opts())
end
```

## 完整配置

查看 [conifguration for blink-cmp](https://github.com/Kaiser-Yang/dotfiles/commit/9901e409c4ae61aae2cb49d99a613e48459eb74b)
