# neovim 配置示例

以 neovim + nvim-cmp 为例

在配置文件中添加如下配置

```lua
-- my rime lsp
start_rime = function ()
  local client_id = vim.lsp.start_client({
    cmd = { '/home/wlh/coding/rime-ls/target/release/rime_ls' },
    init_options = {
      shared_data_dir = "/usr/share/rime-data",
      user_data_dir = "/home/wlh/.local/share/rime-ls",
      log_dir = "/home/wlh/.local/share/rime-ls",
      max_candidates = 10,
      trigger_characters = { '>' }, -- not implemented yet
    },
  });
  vim.lsp.buf_attach_client(0, client_id)
end
```

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

之后，用 `:lua start_rime()` 就可以手动开启 rime-ls 的 LSP 服务了.
当然, 也可以映射成快捷键或 autocmd

