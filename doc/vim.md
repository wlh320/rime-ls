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

## 发送 execute command 命令, 手动 toggle

通过 `:call CocAction('runCommand', 'rime-ls.toggle-rime')` 可以手动控制开启和关闭

## 使用命令来补全输入, 而不是等待候选词出现

```vim
function! RimeConfirmCol(col)
  call setcursorcharpos('.', a:col)
  return ''
endfunction

function! RimeConfirm()
  let line = getline('.')
  let [_, _, col, _, _] = getcursorcharpos()
  let col -= 1
  let input = slice(line, 0, col)
  let result = CocAction('runCommand', 'rime-ls.get-first-candidate', input)
  if result is v:null
    return "\<Space>"
  endif
  let text = result['text']
  let real_input = result['input']
  let start_width = col-strchars(real_input)
  let start_text = slice(line, 0, start_width)
  let end_text = slice(line, col)
  call setline('.', '')
  return start_text . text . end_text . "\<C-r>=RimeConfirmCol(" . (start_width + strchars(text) + 1) . ")\<CR>"
endfunction

function! RimeToggle()
  let rime_enable = CocAction('runCommand', 'rime-ls.toggle-rime')
  if rime_enable
    inoremap <silent> <Space> <C-r>=RimeConfirm()<CR>
    echomsg 'Rime enable'
  else
    iunmap <silent><expr> <Space>
    echomsg 'Rime disable'
  endif
  return ''
endfunction

command! RimeToggle call RimeToggle()
```
