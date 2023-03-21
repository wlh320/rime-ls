# neovim 配置示例

以 neovim + nvim-cmp 为例

在配置文件中添加如下配置 (填入正确的程序路径和 rime 需要的目录)：

```lua
local start_rime = function()
  local client_id = vim.lsp.start_client({
    name = "rime-ls",
    cmd = { '/home/wlh/coding/rime-ls/target/release/rime_ls' },
    init_options = {
      enabled = false, -- 初始关闭, 手动开启
      shared_data_dir = "/usr/share/rime-data", -- rime 公共目录
      user_data_dir = "~/.local/share/rime-ls", -- 指定用户目录, 最好新建一个
      log_dir = "~/.local/share/rime-ls", -- 日志目录
      max_candidates = 10, -- [v0.2.0 后不再有用] 与 rime 的候选数量配置最好保持一致
      trigger_characters = {}, -- 为空表示全局开启
      schema_trigger_character = "&" -- [since v0.2.0] 当输入此字符串时请求补全会触发 “方案选单”
      always_incomplete = false -- [since v0.2.0] true 强制补全永远刷新整个列表，而不是使用过滤
      max_tokens = 0 -- [since v0.2.0] 大于 0 表示会在删除到这个字符个数的时候，重建所有候选词，而不使用删除字符操作
    },
  });
  vim.lsp.buf_attach_client(0, client_id)
  if client_id then
    vim.lsp.buf_attach_client(0, client_id)
    -- 快捷键手动开启
    -- before v0.1.2
    -- vim.keymap.set('n', '<leader><space>', function() vim.lsp.buf.execute_command({ command = "toggle-rime" }) end)
    -- since v0.1.2
    vim.keymap.set('n', '<leader><space>', function() vim.lsp.buf.execute_command({ command = "rime-ls.toggle-rime" }) end)
    vim.keymap.set('n', '<leader>rs', function() vim.lsp.buf.execute_command({ command = "rime-ls.sync-user-data" }) end)
  end
end
-- 对每个文件都默认开启
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

## 状态栏显示

since v0.1.2, 可以参考以下配置在 lualine 显示 rime-ls 的当前状态:

```lua
local M = {}

function M.setup_rime()
  -- maintain buffer only status
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

  -- setup rime-ls
  local start_rime = function()
    local client_id = vim.lsp.start_client({
      name = "rime-ls",
      cmd = { 'rime_ls' },
      init_options = {
        enabled = false,
        shared_data_dir = "/usr/share/rime-data",
        user_data_dir = "~/.local/share/rime-ls",
        log_dir = "~/.local/share/rime-ls",
        max_candidates = 10, -- [v0.2.0 后不再有用] 与 rime 的候选数量配置最好保持一致
        trigger_characters = {},
        schema_trigger_character = "&" -- [since v0.2.0] 当输入此字符串时请求补全会触发 “方案选单”
      },
    });
    if client_id then
      vim.lsp.buf_attach_client(0, client_id)
      vim.keymap.set('n', '<leader><space>', function() toggle_rime(client_id) end)
      vim.keymap.set('i', '<C-x>', function() toggle_rime(client_id) end)
    end
  end

  -- auto start
  vim.api.nvim_create_autocmd({ 'BufReadPost', 'BufNewFile' }, {
    callback = function()
      start_rime()
    end,
    pattern = '*',
  })

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
end

return M
```

例如存为 `lua/rime.lua` ，然后在 `init.lua` 里 `require('rime').setup_rime()`


## 全局状态

以上配置比较简陋，对每个 buffer 开启一个 LSP server 实例，如果希望保持一个全局的输入法状态，可以参考以下配置，
给 lspconfig 添加一个 custom server：

```lua
local M = {}

function M.setup_rime()
  -- global status
  vim.g.rime_enabled = false

  -- update lualine
  local function rime_status()
    if vim.g.rime_enabled then
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

  -- add rime-ls to lspconfig as a custom server
  -- see `:h lspconfig-new`
  local lspconfig = require('lspconfig')
  local configs = require('lspconfig.configs')
  if not configs.rime_ls then
    configs.rime_ls = {
      default_config = {
        name = "rime_ls",
        cmd = { 'rime_ls' },
        -- cmd = vim.lsp.rpc.connect('127.0.0.1', 9257),
        filetypes = { '*' },
        single_file_support = true,
      },
      settings = {},
      docs = {
        description = [[
https://www.github.com/wlh320/rime-ls

A language server for librime
]],
      }
    }
  end

  local rime_on_attach = function(client, _)
    local toggle_rime = function()
      client.request('workspace/executeCommand',
        { command = "rime-ls.toggle-rime" },
        function(_, result, ctx, _)
          if ctx.client_id == client.id then
            vim.g.rime_enabled = result
          end
        end
      )
    end
    -- keymaps for executing command
    vim.keymap.set('n', '<leader><space>', function() toggle_rime() end)
    vim.keymap.set('i', '<C-x>', function() toggle_rime() end)
    vim.keymap.set('n', '<leader>rs', function() vim.lsp.buf.execute_command({ command = "rime-ls.sync-user-data" }) end)
  end

  -- nvim-cmp supports additional completion capabilities, so broadcast that to servers
  local capabilities = vim.lsp.protocol.make_client_capabilities()
  capabilities = require('cmp_nvim_lsp').default_capabilities(capabilities)

  lspconfig.rime_ls.setup {
    init_options = {
      enabled = vim.g.rime_enabled,
      shared_data_dir = "/usr/share/rime-data",
      user_data_dir = "~/.local/share/rime-ls",
      log_dir = "~/.local/share/rime-ls",
      max_candidates = 9,
      trigger_characters = {},
      schema_trigger_character = "&" -- [since v0.2.0] 当输入此字符串时请求补全会触发 “方案选单”
    },
    on_attach = rime_on_attach,
    capabilities = capabilities,
  }
end

return M
```

## 空格键补全

为了取得与外部输入法更加相似的体验，可以通过配置 cmp 实现用空格键补全并用回车键直接输入，参考配置如下：

```lua
cmp.setup {
  -- 其他内容
  -- ...
  mapping = cmp.mapping.preset.insert {
    -- 其他内容
    -- ...
    ['<Space>'] = cmp.mapping(function(fallback)
      local entry = cmp.get_selected_entry()
      if entry == nil then
        entry = cmp.core.view:get_first_entry()
      end
      if entry and entry.source.name == "nvim_lsp"
        and entry.source.source.client.name == "rime_ls" then
        cmp.confirm({
          behavior = cmp.ConfirmBehavior.Replace,
          select = true,
        })
      else
        fallback()
      end
    end, {'i', 's'}),
    ['<CR>'] = cmp.mapping(function(fallback)
      local entry = cmp.get_selected_entry()
      if entry == nil then
        entry = cmp.core.view:get_first_entry()
      end
      if entry and entry.source.name == 'nvim_lsp'
        and entry.source.source.client.name == 'rime_ls' then
        cmp.abort()
      else
        if entry ~= nil then
          cmp.confirm({
            behavior = cmp.ConfirmBehavior.Replace,
            select = true
          })
        else
          fallback()
        end
      end
    end, {'i', 's'}),
    -- 其他内容
    -- ...
  }
  -- 其他内容
  -- ...
}
```

以上配置通过判断当前补全项是否由 rime-ls 提供来决定是否启用空格补全。

## 通过 TCP 远程使用

将运行命令修改为 `cmd = vim.lsp.rpc.connect('<ip>', <port>)`

## 五笔或者双形用户

```lua
require('lspconfig').rime_ls.setup {
  init_options = {
    enabled = vim.g.rime_enabled,
    shared_data_dir = "/usr/share/rime-data",
    user_data_dir = "~/.local/share/rime-ls",
    log_dir = "~/.local/share/rime-ls",
    max_candidates = 9,
    trigger_characters = {},
    schema_trigger_character = "&" -- [since v0.2.0] 当输入此字符串时请求补全会触发 “方案选单”
    max_tokens = 4, -- 强制在删除到4字的时候重建一次候选词，避免用退格造成的空列表的问题
    always_incomplete = true, -- 将 incomplete 永远设为 true，防止任何时候的过滤代替候选词重建
  },
  on_attach = rime_on_attach,
  capabilities = capabilities,
}
```

