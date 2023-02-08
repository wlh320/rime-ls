use std::{net::SocketAddr, str::FromStr};

use rime_ls::lsp::Backend;
use rime_ls::rime::Rime;
use tokio::net::{TcpListener, TcpStream};
use tower_lsp::{LspService, Server};

async fn run_stdio() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(Backend::new).finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}

async fn run_tcp(stream: TcpStream) {
    let (read, write) = tokio::io::split(stream);

    let (service, socket) = LspService::build(Backend::new).finish();
    Server::new(read, write, socket).serve(service).await;
}

async fn run_tcp_forever(bind_addr: SocketAddr) -> tokio::io::Result<()> {
    println!("Listening on: {}", &bind_addr);
    let listener = TcpListener::bind(bind_addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(run_tcp(stream));
    }
}

fn usage() {
    println!("rime_ls v{}", env!("CARGO_PKG_VERSION"));
    println!("Usage: rime_ls [--listen <bind_addr>]")
}

#[tokio::main]
async fn main() {
    // set handler to finalize rime
    // TODO: it is ugly
    ctrlc::set_handler(move || {
        println!("Ctrl-C pressed.");
        if Rime::is_initialized() {
            Rime::global().destroy();
        }
        std::process::exit(0); // 0?
    })
    .expect("Error setting Ctrl-C handler");

    let mut args = std::env::args();
    match args.nth(1).as_deref() {
        None => run_stdio().await,
        Some("--listen") => {
            let addr = args.next().unwrap_or("127.0.0.1:9257".to_owned());
            let addr = SocketAddr::from_str(&addr).unwrap();
            run_tcp_forever(addr).await.unwrap();
        }
        _ => usage(),
    }
}
