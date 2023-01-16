# rime-ls

为 rime 输入法核心库 librime 实现 LSP 协议, 从而通过编辑器的代码补全功能输入汉字.

项目还处在**早期阶段**, 各方面都非常不成熟.

目标是提供 rime + LSP 的通用解决方案, 在不同编辑器内实现与其他 rime 前端类似的输入体验.

## Features

- 按数字选择补全项
- 多种触发方式
    - 默认开启, 随时补全, 用快捷键控制关闭 (写大量汉字)
    - 用特殊字符触发补全 (写少量汉字)

## Build

### Ubuntu

1. 配置 Rust 环境, 安装额外依赖 `clang` 和 `librime-dev`
2. 编译 
    - `librime >= 1.6` => `cargo build --release`
    - `librime < 1.6` => `cargo build --release --features=no_log_dir`

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

## Usage

1. 将编译好的二进制文件放在喜欢的目录下
2. 配置 LSP 客戶端, 例如: 
    - [neovim + nvim-cmp](doc/nvim.md)
    - [vim + coc.nvim](doc/vim.md)
    - [vscode](doc/vscode.md)
3. 默认輸入拼音, 就可以看到补全提示
4. 可以通过改变配置控制补全行为

## TODO

- [ ] 實現更多 librime 的功能
    - [x] 按数字键选择候选项
    - [ ] 与 rime API 同步翻页
    - [ ] 与 rime API 同步提交
- [x] 实现更友好的触发条件
    - [ ] 计划实现光标前面有汉字就开启, 但发现不同编辑器行为不一致, 搁置
- [ ] 读 LSP 文档, 继续提升补全的使用体验
- [x] 參數可配置 (用户目录, 触发条件, 候选数量)
- [ ] 實現一個更好的 librime 的 rust wrapper 庫
- [x] 測試其他 LSP clients
- [x] 测试不同操作系统和 librime 版本
- [ ] 测试与不同 rime 配置的兼容性

## Known Issues

- [x] ~~補全的觸發條件很奇怪，現在我是手動觸發補全寫的這些字~~ 解决, 要设置 is_incomplete 来连续补全
- [x] ~~還沒完成開始這個項目的最初目的, 即直接復用 rime 配置~~ 直接设置不同的用户目录好像可以, 比如我现在可以写简体了, 还需要进一步测试
- [ ] 沒有完全實現 rime 功能, 只是读取了候选项, 沒有把选到的字真正提交 
(因为还没获取到补全的反馈, 计划自己处理用户输入再与 rime 交互, 感觉有点麻烦, 可能搁置)
- [ ] 第一次嘗試從 Rust 調用 C 接口，寫的非常不專業且 unsafe

## Credits

受到以下項目啓發

- [ds-pinyin-lsp](https://github.com/iamcco/ds-pinyin-lsp)
- [cmp-rime](https://github.com/Ninlives/cmp-rime)
- [librime-sys](https://github.com/lotem/librime-sys)
- [tower-lsp-boilerplate](https://github.com/IWANABETHATGUY/tower-lsp-boilerplate)

