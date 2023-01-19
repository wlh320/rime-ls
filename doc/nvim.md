# neovim 配置示例

以 neovim + nvim-cmp 为例

在配置文件中添加如下配置 (填入正确的程序路径和 rime 需要的目录)：

```lua
start_rime = function()
  local client_id = vim.lsp.start_client({
    name = "rime-ls",
    cmd = { '/home/wlh/coding/rime-ls/target/release/rime_ls' },
    init_options = {
      enabled = false, -- 初始关闭, 手动开启
      shared_data_dir = "/usr/share/rime-data", -- rime 公共目录
      user_data_dir = "/home/wlh/.local/share/rime-ls", -- 指定用户目录, 最好新建一个
      log_dir = "/home/wlh/.local/share/rime-ls", -- 日志目录
      max_candidates = 10,
      trigger_characters = {},
    },
  });
  vim.lsp.buf_attach_client(0, client_id)
  -- 快捷键手动开启
  vim.keymap.set('n', '<leader>r', function() vim.lsp.buf.execute_command({ command = "toggle-rime" }) end)
end

vim.api.nvim_create_autocmd('BufReadPost', {
  callback = function()
    start_rime()
  end,
  pattern = '*',
})

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

以上配置进入文件便开启 LSP, 用快捷键切换是否开启补全

