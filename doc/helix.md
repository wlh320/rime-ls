# helix 配置示例

helix 自带 LSP 支持，只需要修改配置文件

为了更好的用户体验，需要 rime-ls v0.3.0 及之后版本，并且配置 `config.long_filter_text = true`

## 使用方法

例如为 markdown 文件启用 rime-ls，在 `~/.config/helix/languages.toml` 中增加如下配置：

### Before 23.10

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
config.long_filter_text = true
```

### Since 23.10

```toml
[language-server.rime-ls]
command = "/path/to/rime-ls"
config.shared_data_dir = "/usr/share/rime-data"
config.user_data_dir = "~/.local/share/rime-ls"
config.log_dir = "~/.local/share/rime-ls"
config.max_candidates = 9
config.trigger_characters = []
config.schema_trigger_character = "&"
config.max_tokens = 4
config.always_incomplete = true
config.long_filter_text = true

[[language]]
name = "markdown"
scope = "source.markdown"
file-types = ["md", "markdown"]
language-servers = ["rime-ls"]
```

rime-ls 的配置项参考其他编辑器，都是一样的，改成 toml 的格式即可。

对于 helix 上面的 LSP 的更多配置请参考 helix 的官方文档，例如怎么为所有文件开启某个 LSP server。

## 存在问题

- [x] 补全触发条件有问题(**已解决**)
  - [x] 在汉字后面输入不会自动触发补全，需通过配置的触发字符手动触发(since v0.3.0 配置 `config.long_filter_text = true`)
  - [x] 最小补全长度为 2，手动设置最小补全长度为 1 会导致当前输入长度为 2 时补全消失(helix 最新版已无问题)
