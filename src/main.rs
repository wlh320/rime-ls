use rime_ls::lsp::Backend;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let shared_data_dir = "/usr/share/rime-data/";
    let user_data_dir = "/home/wlh/.local/share/rime-ls/";
    let log_dir = "/tmp";

    let (service, socket) =
        LspService::build(|client| Backend::new(client, shared_data_dir, user_data_dir, log_dir))
            .finish();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    Server::new(stdin, stdout, socket).serve(service).await;
}
