# v0.4.3

## Fix

- 通过 TCP 使用在关闭连接时没有删除会话 (应该解决了 #50)

## Doc

- 更新 nixOS 下的使用方法 (Thanks to definfo #59)

## Chore

- 增加 Release 的 CI
- 更新 Docker 相关 CI
  - 同时发布到 ghcr.io 和 Docker Hub
  - 镜像支持 linux/riscv64 平台

# v0.4.2

## Fix

- 会话没有及时删除导致的内存占用增加 #51
  - 自测 Windows / Linux 没问题，有用户反映 macOS 没用，need more test #50

## Doc

- 增加了 neovim v0.11 的配置示例

## Chore

- 不再提示 info 级别的消息，只提示 error 级别 (Thanks to twio142 #53 #54)
- 使用 Alpine 自带的 rime-plugins 包，提升 Docker 镜像打包速度

# v0.4.1

## Feat

- 支持 UTF-8 和 UTF-32 编码，按 LSP 3.17 标准在初始化时与客户端协商 (it also fixes #38)

## Doc

- 增加 nvim + blink.cmp 的配置示例 (Thanks to Kaiser-Yang #46)
- 增加 nvim 下的多种功能的配置示例 (Thanks to Kaiser-Yang #44 #45 #46)

## Chore

- 更新依赖版本
- 增加 Mac 和 Win 环境下的 CI (Thanks to asukaminato0721 #39 #40)

# v0.4.0

## Fix

- 改进文件 didChange 的判断，减少 `unwrap()` 报错
- 避免代码中硬编码用户目录
- 移除部分不必要的类型转换
- 修复第一次补全无法触发方案选单的问题

## Feat

- 放弃 `Rime*` 这种老 API，转用新版本 API
- 可以通过 Github Actions 构建的 Docker 镜像来使用 rime-ls 了
- 增加新的配置项 `show_order_in_label` 用于配置补全项不显示数字 (#28)

## Doc

- NixOS 下的安装使用指南 (Thanks to definfo #32 #33)
- fix typo (Thanks to evpeople #34)

## Chore

- 加入 `Cargo.lock` 便于可重复构建 (Thanks to definfo #32)
- 改进 `Dockerfile`，基于 Alpine 减小镜像大小 (23 MB)

# v0.3.0

## Fix

- 解决某些编辑器无法连续输入的问题：
  - 为 Zed 和 Helix 增加配置项 `long_filter_text`
  - 为 Zed 增加配置项 `show_filter_text_in_label`
- 在查找补全串开头的时候应该从右到左搜索 (Thanks to TwIStOy in #21 )
- 解决使用 dashmap 的一个死锁 (#16)

## Feat

- 增加 Dockerfile 方便通过 TCP 使用 (Thanks to Zwlin98 in #17)

## Doc

- 完善 macOS 下编译的文档 (Thanks to evpeople in #25)

## Chore

- 增加 Release 的编译脚本, 通过 `cargo-zigbuild` 和 `cargo-xwin` 交叉编译其他平台

# v0.2.4

## Fix

- 修复了不定期发生的补全无法触发的问题 #14
- 修复了 termux 环境下的构建失败 #8

## Feat

- 允许通过新配置项 `paging_characters` 自定义触发补全的字符，主要用途是翻页 #13

## Chore

- 有了基础的 GitHub CI #9 (Thanks to eagleoflqj)

# v0.2.3

## Breaking Changes

- 不再默认选中第一个候选项，若要选中，通过新配置项 `preselect_first` 进行配置

## Feat

- 提升了在五笔和音形输入方案下的体验 #7 (Thanks to TwIStOy)
  - 增加了新配置项 `max_tokens`，强制在删除到一定长度时重建一次候选词
  - 增加了新配置项 `always_incomplete`，每次输入重建候选词，防止过滤代替候选词重建

## Doc

- 增加了 macOS 系统下的编译步骤 #6 (Thanks to rainzm)
- 更新了 neovim 配置样例，在 nvim-cmp 下的体验更接近系统输入法
  - 空格汉字上屏之外，新增回车原始串上屏 #7 (Thanks to TwIStOy)
  - 使上述行为只对 rime-ls 提供的补全项生效，不影响其他补全

# v0.2.2

## Fix

- 修复了因内存提前释放导致的日志文件名出错
- 修复了一些特殊场景下对已提交文字的判断错误

## Feat

- 升级 `tower-lsp` 到 `0.19.0` 版本以支持 LSP 3.17.0
- 支持 LSP 3.17.0 的 `label_details` 特性，用于显示每一个候选项的 comment
- 默认选中第一个候选项

# v0.2.1

## Breaking Changes

- 启动时不再自动更新用户词频，改为完全手动操作

## Fix

- 修复了因状态判断出错导致的删除输入后同一位置无法继续输入的问题
- 修复了进入「方案选单」后必须选择才能退出的问题

# v0.2.0

## Breaking Changes

- 配置项 `max_candidates` 不再生效，改为遵从 Rime 配置的每页候选项个数

## Feat

- 与 Rime 的 API 保持同步，不再是只获取候选项
- 支持将通过数字选择的候选项提交给 Rime，从而影响用户词频
- 支持长句子的分多次选择
- 支持 Rime 的「方案选单」功能
- 可能有理论上的性能提升 (不再每次打字都创建 session，未验证效果)

# v0.1.3

## Fix

- 更好的错误处理
- 修复了几处从 rust 传指针给 C 时的典型内存泄漏

## Feat

- 配置项的路径现在支持展开波浪线 `~` 为家目录
- 支持通过 TCP 远程使用 (明文传输，不安全，需要配合加密的 TCP 信道)
- 可能有理论上的性能提升 (未验证)

# v0.1.2

## Breaking Changes

- `execute_command` 所支持的命令的名称都增加了 `rime-ls.` 的前缀

## Fix

- 修复了因更新位置在边界处导致的 LSP server 不能同步更新文档内容的问题

## Feat

- 现在可以通过命令手动触发用户目录同步
- 执行 `rime-ls.toggle-rime` 命令后会返回执行后的当前状态

# v0.1.1

## Fix

- 全局模式下，补全时会删掉拼音前未提交的标点符号

## Feat

- 触发模式下，光标前有非英文字符时可以自动触发补全继续输入

# v0.1.0

第一个基本可用版本
