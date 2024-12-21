# neovim + blink.cmp 配置示例

在此给出一些 `blink.cmp` 的相关配置注意事项（以 `blink.cmp v0.8.0` 为例）：

## 开启 long_filter_text

`blink.cmp` 对于候选词的过滤比较严格，需要将 rime-ls 的 `long_filter_text` 配置设置为 `true`

## 修改默认的 LSP 过滤规则

`blink.cmp` 将 LSP 服务器提供的 Text 类型补全也过滤掉了，为了启用 rime-ls，需要修改相关配置:

```lua
    sources = {
      -- ...
      providers = {
        lsp = {
          transform_items = function(_, items) return items end
        }
      },
      -- ...
    },

```

## 还原输入法体验

### 数字键直接上屏

可以通过 `blink.cmp` 提供的事件接口实现：

```lua
  -- if the last character is a number (and its previous one is not),
  -- and the only completion item is provided by rime-ls, accept it
  require('blink.cmp.completion.list').show_emitter:on(function(event)
    local items = event.items
    if #items ~= 1 then return end
    local line = event.context.line
    local col = vim.fn.col('.') - 1
    if line:sub(col - 1, col):match("%a%d") == nil then return end
    local item = items[1]
    local client = vim.lsp.get_client_by_id(item.client_id)
    if (not client) or client.name ~= "rime_ls" then return end
    require('blink.cmp').accept({ index = 1 })
  end)

```

### 空格补全

作者本人习惯把 rime-ls 视为一个普通的补全来源，还是用默认的补全快捷键。

但 `blink.cmp` 也支持自定义 keymap，按下时执行自定义的函数，因此空格补全也是能做到的。

### 其他

TODO
