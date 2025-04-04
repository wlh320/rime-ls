# neovim 配置示例

作者目前没有实现官方 neovim 插件，可以使用其他用户开发的插件，或者根据下面的代码片段自行配置。

- [使用其他用户开发的插件](#使用其他用户开发的插件)
- [初始化 rime-ls](#初始化-rime-ls)
- [通过 TCP 远程使用](#通过-tcp-远程使用)
- [状态栏显示](#状态栏显示)
- [特定 buffer 无法使用问题](#特定-buffer-无法使用问题)
- [v0.10.2 后偶尔无法补全的问题](#v0.10.2-后偶尔无法补全的问题)
- [与 nvim-cmp 相关的额外配置](#与-nvim-cmp-相关的额外配置)
- [与 blink.cmp 相关的额外配置](#与-blink.cmp-相关的额外配置)


## 使用其他用户开发的插件

目前没有官方实现的 neovim 插件，但已有用户将相关功能封装成了插件，例如：

- [liubianshi/cmp-lsp-rimels](https://github.com/liubianshi/cmp-lsp-rimels)

也可参考 issues 里其他用户和下面的配置片段。

## 初始化 rime-ls

### 使用 vim.lsp.config

如果你使用 nvim 0.11 及之后版本，你可以在配置目录下创建 `lsp/rime_ls.lua` 配置，
然后用 `vim.lsp.enable('rime_ls')` 启用 rime_ls

文件的内容可以参考[作者的配置](https://github.com/wlh320/wlh-dotfiles/blob/aa9be6ffbe587452a42520626befc10ed5a614b8/config/nvim/lsp/rime_ls.lua#L1)

上述配置会为全部 buffer 开启 rime_ls，文档类的文件可以直接使用，其余文件类型用特殊符号触发补全。

### 使用 lspconfig

基于 lspconfig 全局开启 rime-ls：

可以参考[作者的配置](https://github.com/wlh320/wlh-dotfiles/blob/1a26b72172368de2895a3bd21ce94b7b17a9da38/config/nvim/lua/rime.lua#L3)
给 nvim-lspconfig 添加一个 custom server

上述代码定义了一个 `setup_rime()` 函数，在配置 lspconfig 的位置手动调用一下，
即可为全部 buffer 开启该服务，用一个全局变量 `vim.g.rime_enabled` 做开关控制是否真正使用。

## 通过 TCP 远程使用

在本机开多个 nvim 进程时会随之开启多个 rime-ls 进程，由于 rime 会给数据库加锁导致不能同时使用。
为了不产生冲突，可以只开一个 rime-ls 进程，不同客户端通过 TCP 远程使用。

需要 rime_ls 以 TCP 模式运行: `rime_ls --listen <bind_addr>`

客户端在上述初始化代码中将运行命令修改为 `cmd = vim.lsp.rpc.connect('<ip>', <port>)`。


## 状态栏显示

since v0.1.2, 可以参考以下配置在 lualine 显示 rime-ls 的当前状态:

其他插件同理，都依赖于 rime-ls 的 `rime-ls.toggle-rime` 命令的返回结果

```lua
-- toggle rime
vim.b.rime_enabled = false
local toggle_rime = function(client_id)
  vim.lsp.buf_request(0, 'workspace/executeCommand',
    { command = "rime-ls.toggle-rime" },
    function(_, result, ctx, _)
      if ctx.client_id == client_id then
        vim.b.rime_enabled = result
      end
    end
  )
end

-- update lualine
local function rime_status()
  if vim.b.rime_enabled then
    return 'ㄓ'
  else
    return ''
  end
end

require('lualine').setup({
  sections = {
    lualine_x = { rime_status, 'encoding', 'fileformat', 'filetype' },
  }
})
```

## 特定 buffer 无法使用问题

相关 issue ：[copilot-chat窗口不能正常触发](https://github.com/wlh320/rime-ls/issues/29)。

该问题往往是因为这些 buffer 被设置了 hidden 属性，导致 rime-ls 没有启动。
可以通过运行 `:LspStart rime_ls` 来手动启动 rime-ls，
或者使用以下自动命令（这里两种方法均需要使用全局 LSP 配置）：

```lua
-- update the variable to your needs
local rime_ls_filetypes = {'*'}
vim.api.nvim_create_autocmd('FileType', {
  pattern = rime_ls_filetypes,
  callback = function(env)
    -- some buffers cannot attach client automatically, we must attach manually.
    local rime_ls_client = vim.lsp.get_clients({ name = 'rime_ls' })
    if #rime_ls_client == 0 then
      vim.cmd('LspStart rime_ls')
      rime_ls_client = vim.lsp.get_clients({ name = 'rime_ls' })
    end
    if #rime_ls_client > 0 then
      vim.lsp.buf_attach_client(env.buf, rime_ls_client[1].id)
    end
  end
})
```

即使有了上面的配置，在 No Name 文件中依然不能自动启动，对于这种情况，需要先保存 No Name 文件，
然后再重新打开。

## v0.10.2 后偶尔无法触发补全的问题

相关 issue #38

有以下解决方案:
1. 降级或升级使用 v0.11 及以后版本，都不会有该问题
2. 不降级，配置 rime-ls 使用 UTF-8 编码，参考这里
   https://github.com/wlh320/wlh-dotfiles/blob/85d41a30588642617177374b4cea2ec96c1b2740/config/nvim/init.lua#L451
   这样配完与其他 lsp server 共用时 lspconfig 还会报 warning。如果确认你用的其他 lsp server 也支持
   UTF-8，可以再加一句配置像这样
   https://github.com/wlh320/wlh-dotfiles/blob/85d41a30588642617177374b4cea2ec96c1b2740/config/nvim/init.lua#L457

## 与 nvim-cmp 相关的额外配置

上面是对不同补全插件都适用的代码，为了更好的体验还需要配置一下补全插件。

如果用 nvim-cmp 看[这个文档](./nvim-with-cmp.md)

## 与 blink.cmp 相关的额外配置

如果用 blink.cmp 看[这个文档](./nvim-with-blink.md)
