# rime-ls

爲 rime 輸入法庫 librime 實現 LSP 協議

早期階段，能實現基本的輸入漢字功能，還很難用

類似項目要麼是專爲某個編輯器實現 rime 前端, 要麼用 LSP 自己實現打字邏輯

如果 rime + LSP 感覺會更通用一些

項目的最終目標是最大可能復用已有的 rime 配置，實現與其他前端類似的體驗，並可用在所有 LSP client

## How it works

用 Rust 包裝的 librime C FFI 接口和 tower-lsp 實現

利用 LSP 的補全功能，將 rime 的候選項作爲補全結果返回給編輯器，
從而無需在編輯器裏面調用外部輸入法，適合代碼時輸入**少量**漢字

目前的實現是利用 tower-lsp 起一個 LSP 服務，直接讀取光標前的拼音,餵給 librime 得到後選項返回。
之後可能改成用 rime 的更多 API 真正模擬打字，實現更多 rime 的功能

## Usage

1. 安裝依賴項 librime
2. 編譯這個倉庫
3. 配置 LSP 客戶端

例如, 在 neovim + nvim-cmp

```lua

-- my rime lsp
start_rime = function ()
  local client_id = vim.lsp.start_client({
    cmd = { '/home/wlh/coding/rime-ls/target/release/rime_ls' },
    settings = {
      shared_data_dir = "/usr/share/rime-data",
      user_data_dir = "/home/wlh/.local/share/rime-ls",
      log_dir = "/home/wlh/.local/share/rime-ls",
      max_candidates = 10,
      trigger_characters = { '>' }, -- not implemented yet
    },
  });
  vim.lsp.buf_attach_client(0, client_id)
end

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
配置完 LSP 和补全插件的可配置项之后，用 `:lua start_rime()` 手動開啓 LSP server

輸入拼音, 就可以看到补全提示

## TODO

- [ ] 實現更多 librime 的功能 (按数字选择, 候选翻页, 部分提交上屏, 特殊字符)
- [ ] 读 LSP 文档, 继续提升补全的使用体验
- [x] 參數可配置 (用户目录, 触发条件, 候选数量)
- [ ] 實現一個更好的 librime 的 rust wrapper 庫
- [ ] 測試其他 LSP clients

## Known Issues

- [x] ~~補全的觸發條件很奇怪，現在我是手動觸發補全寫的這些字~~ 解决, 要设置 is_incomplete 来连续补全
- [x] ~~還沒完成開始這個項目的最初目的, 即直接復用 rime 配置~~ 直接设置不同的用户目录好像可以, 比如我现在可以写简体了, 还需要进一步测试
- [ ] 沒有完全實現 rime 功能, 沒有記錄詞頻, 也沒有上下文 (因为还没获取到补全的反馈)
- [ ] 第一次嘗試從 Rust 調用 C 接口，寫的非常不專業且 unsafe

## Credits

受到以下項目啓發

- [ds-pinyin-lsp](https://github.com/iamcco/ds-pinyin-lsp)
- [cmp-rime](https://github.com/Ninlives/cmp-rime)
- [librime-sys](https://github.com/lotem/librime-sys)
- [tower-lsp-boilerplate](https://github.com/IWANABETHATGUY/tower-lsp-boilerplate)

