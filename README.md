# rime-ls

为 rime 输入法核心库 librime (的部分功能) 实现 LSP 协议, 从而通过编辑器的代码补全功能输入汉字.

项目还处在**早期阶段**, 各方面相对不成熟.

目标是提供 rime + LSP 的通用解决方案, 在不同编辑器内实现与其他 rime 前端类似的输入体验.

## Features

- 用 rime 能输入的东西按理说都能输入 ( 汉字, 标点, emoji ...)
- 支持按数字选择补全项
- 支持候选词翻页
- 多种触发方式
    - 默认开启, 随时补全, 用快捷键控制关闭 (写大量汉字)
    - 平时关闭, 检测到配置的特殊字符或光标前有非英文字符时触发补全 (写少量汉字)
- 可以按配置其他 rime 输入法的方式去配置 (只有能影响候选项的配置是有用的)
- 可以同步系统中已有 rime 输入法的词频
- 可以通过 TCP 远程使用 (无任何加密，谨慎使用) (since v0.1.3)

https://user-images.githubusercontent.com/14821247/213079440-f0ab2ddd-5e44-4e41-bd85-81da2bd2957f.mp4

## Usage

> **Warning**
> 第一次启动时 rime 需要做大量工作, 可能会很慢

1. 下载 Release 或自己从源码编译
2. 将编译好的二进制文件放在喜欢的目录下
3. 配置 LSP 客戶端, 例如:
    - [neovim + nvim-cmp](doc/nvim.md)
    - [vim + coc.nvim](doc/vim.md)
    - [vscode](doc/vscode.md)
4. 像配置其他 Rime 输入法一样在 rime-ls 的用户目录进行配置
5. 輸入拼音, 就可以看到补全提示
6. 可以通过修改 rime-ls 的配置项控制补全行为

## Build

### Ubuntu

1. 配置 Rust 环境, 安装额外依赖 `clang` 和 `librime-dev`
2. 编译
    - `librime >= 1.6` => `cargo build --release`
    - `librime < 1.6` => `cargo build --release --features=no_log_dir`

### ArchLinux

可以通过我在 AUR 上打的包 [rime-ls](https://aur.archlinux.org/packages/rime-ls) 安装

其他 linux 发行版类似

### Windows

1. 配置 Rust 环境, 安装额外依赖 `clang` 和 `librime`
2. 依赖的 `librime-sys` 包没有针对 Windows 优化, 直接编译可能失败, 需要先下载到本地,
手动修改下 `build.rs` 引入头文件. 例如,
```diff
diff --git a/build.rs b/build.rs
index a53dd2c..e51a63e 100644
--- a/build.rs
+++ b/build.rs
@@ -11,6 +11,7 @@ fn main() {

     let bindings = bindgen::Builder::default()
         .header("wrapper.h")
+        .clang_arg("-IC:\\Users\\wlh\\Downloads\\rime-1.7.3-win32\\dist\\include")
         .generate()
         .expect("Unable to generate bindings");
```
3. 修改本项目的 `Cargo.toml` 指向本地的依赖
4. 用 `i686` 的 target 编译 (因为 librime 只给了 32 位的 dll)

### 个人词库同步

> **Warning**
> **不推荐**与系统中的已有 rime 输入法共用一个用户目录, 免得出什么问题
>
> 使用前备份自己的数据, 避免因作者对 rime API 理解不到位可能造成的数据损失

可以通过 rime 的 sync 功能将系统中已安装的 rime 输入法的词库同步过来。

如需同步词库, 可以在 rime-ls 自己的用户目录下的 `installation.yaml`
添加`sync_dir: "/<existing user data dir>/sync"` 配置项。

通过 LSP 的 `workspace/executeCommand` 手動調用 `rime-ls.sync_user_data` 的命令进行同步 (since v0.1.2)

## TODO

- [x] 實現更多 librime 的功能
    - [x] 按数字键选择候选项
    - [x] 与 rime API 同步翻页
    - [x] 与 rime API 同步提交
    - [x] 输入标点符号
    - [x] 输入方案选择
- [x] 实现更友好的触发条件
    - [x] ~~计划实现光标前面有汉字就开启, 但发现不同编辑器行为不一致, 搁置~~ 多加了一次正则匹配解决了, 不知道性能如何
- [ ] 读 LSP 文档, 继续提升补全的使用体验
- [x] 參數可配置 (用户目录, 触发条件, 候选数量)
- [ ] 實現一個更好的 librime 的 rust wrapper 庫
- [x] 測試其他 LSP clients
- [x] 测试不同操作系统和 librime 版本
- [ ] 测试与不同 rime 配置的兼容性

## Known Issues

- [x] ~~補全的觸發條件很奇怪，現在我是手動觸發補全寫的這些字~~ 解决, 要设置 is_incomplete 来连续补全
- [x] ~~還沒完成開始這個項目的最初目的, 即直接復用 rime 配置~~ 维护一份独立的用户目录，与外部输入法互相同步词频
- [x] ~~沒有完全實現 rime 功能, 只是读取了候选项, 沒有把选到的字真正提交~~ v0.2.0 基本解决
- [ ] 第一次嘗試從 Rust 調用 C 接口，寫的非常不專業且 unsafe
- [ ] 同时开启多个共用同一个用户目录的程序时，会因为用户数据库的锁导致不工作

## Credits

受到以下項目啓發

- [ds-pinyin-lsp](https://github.com/iamcco/ds-pinyin-lsp)
- [cmp-rime](https://github.com/Ninlives/cmp-rime)
- [librime-sys](https://github.com/lotem/librime-sys)
- [tower-lsp-boilerplate](https://github.com/IWANABETHATGUY/tower-lsp-boilerplate)

