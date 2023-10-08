# helix 配置示例

helix 自带了对 LSP 的支持，但目前使用 rime-ls 还存在一些小问题，
现在属于勉强能用的状态，使用体验不太好。

## 使用方法

例如为 markdown 文件启用 rime-ls，在 `~/.config/helix/languages.toml` 中增加如下配置：

```toml
[[language]]
name = "markdown"
scope = "source.markdown"
file-types = ["md", "markdown"]
language-server = { command = "/path/to/rime-ls" }
config.shared_data_dir = "/usr/share/rime-data"
config.user_data_dir = "~/.local/share/rime-ls"
config.log_dir = "~/.local/share/rime-ls"
config.max_candidates = 9
config.trigger_characters = []
config.schema_trigger_character = "&"
config.max_tokens = 4
config.always_incomplete = true
```

rime-ls 的配置项参考其他编辑器，都是一样的，改成 toml 的格式即可。

对于 helix 上面的 LSP 的更多配置请参考 helix 的官方文档，例如怎么为所有文件开启某个 LSP server。

## 存在问题

- [ ] 补全触发条件有问题
    - [ ] 在汉字后面输入不会自动触发补全，需通过配置的触发字符手动触发
    - [ ] 最小补全长度为 2，手动设置最小补全长度为 1 会导致当前输入长度为 2 时补全消失

