# vim 配置示例

目前没有现成插件，建议根据实际使用情况自行配置。

以 vim + coc.nvim 为例,

在 `coc-settings.json` 里加入如下配置 (填入正确的程序路径和 rime 需要的目录)：

```jsonc
{
  "languageserver": {
    // 其他 LSP
    // ......

    "rime-ls": {
      "command": "/usr/bin/rime_ls",
      "filetypes": ["text"],
      "initializationOptions": {
        "enabled": true,
        "shared_data_dir": "/usr/share/rime-data", // rime 公共目录
        "user_data_dir": "~/.local/share/rime-ls", // 指定用户目录，最好新建一个
        "log_dir": "~/.local/share/rime-ls", // 日志目录
        "max_candidates": 9, // [v0.2.0 后不再有用] 与 rime 的候选数量配置最好保持一致
        "paging_characters": [",", "."], // [since v0.2.4] 这些符号会强制触发一次补全，可用于翻页 见 issue #13
        "trigger_characters": [], // 为空表示全局开启
        "schema_trigger_character": "&", // [since v0.2.0] 当输入此字符串时请求补全会触发 “方案选单”
      },
    },

    // 其他 LSP
    // ......
  },
}
```

没有完全测试过, 理论上其他 LSP 能怎么用就可以怎么用

补充: 通过 `:call CocRequest('rime-ls', 'workspace/executeCommand', { 'command': 'rime-ls.toggle-rime' })`
可以手动控制开启和关闭

# TODO

- [x] 发送 execute command 命令, 手动 toggle
