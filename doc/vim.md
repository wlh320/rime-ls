
# vim 配置示例

以 vim + coc.nvim 为例,

在 `coc-settings.json` 里加入如下配置:

```jsonc
{
  "languageserver": {
    // 其他 LSP
    // ......

    "rime-ls": {
      "command": "/home/wlh/coding/rime-ls/target/release/rime_ls",
      "filetypes": ["text"],
      "initializationOptions": {
        "shared_data_dir": "/usr/share/rime-data",
        "user_data_dir": "/home/wlh/.local/share/rime-ls",
        "log_dir": "/home/wlh/.local/share/rime-ls",
        "max_candidates": 10,
        "trigger_characters": [ ">" ], // not implemented yet
      }
    },

    // 其他 LSP
    // ......
  }
}
```

没有完全测试过, 理论上其他 LSP 能怎么用就可以怎么用

