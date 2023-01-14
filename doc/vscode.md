# VSCode 配置示例

既然声称是 LSP, 必然得能在 VSCode 上使用

项目还不成熟, 也没时间真的写一个插件, 目前只是做一下简单的可行性验证

用官方的 LSP 插件的[例子](https://github.com/microsoft/vscode-extension-samples/tree/main/lsp-sample)
稍加修改, 把启动 server 部分的代码改成启动可执行文件

```typescript
export function activate(context: vscode.ExtensionContext) {

	// Server
	const executable = {
		command: "C:\\Users\\wlh23\\.local\\bin\\rime_ls.exe",
		args: []
	};
	const serverOptions: ServerOptions = {
		run: executable,
		debug: executable,
	};

	// Client
	const clientOptions: LanguageClientOptions = {
		// Register the server for plain text documents
		documentSelector: [{ scheme: 'file', language: 'plaintext' }],
		initializationOptions: {
			shared_data_dir: "C:\\Program Files (x86)\\Rime\\weasel-0.14.3\\data",
			user_data_dir: "C:\\Users\\wlh23\\AppData\\Roaming\\Rime",
			log_dir: "C:\\Users\\wlh23\\AppData\\Roaming\\Rime",
			max_candidates: 10,
			trigger_characters: [],
		}
	};
	console.log(clientOptions);

	// Create the language client and start the client.
	client = new LanguageClient(
		'Rime_LSP_Example',
		'Rime LSP Example',
		serverOptions,
		clientOptions
	);

	// Start the client. This will also launch the server
	client.start();
};
```

只是简单验证了是可用的, 但是目前的使用体验并不好

