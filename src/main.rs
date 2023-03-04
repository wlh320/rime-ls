use rime_ls::lsp::Backend;
use rime_ls::rime::Rime;
use std::{net::SocketAddr, str::FromStr};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::{self, Receiver};
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

async fn run(mut shutdown: Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    match args.nth(1).as_deref() {
        None => {
            tokio::select! {
                _ = shutdown.recv() => (),
                _ = run_stdio() => ()
            }
        }
        Some("--listen") => {
            let addr = args.next().unwrap_or("127.0.0.1:9257".to_owned());
            let addr = SocketAddr::from_str(&addr)?;
            tokio::select! {
                _ = shutdown.recv() => Ok(()),
                Err(e) = run_tcp_forever(addr) => Err(e),
            }?
        }
        _ => usage(),
    }
    Ok(())
}

fn usage() {
    println!("rime_ls v{}", env!("CARGO_PKG_VERSION"));
    println!("Usage: rime_ls [--listen <bind_addr>]")
}

#[tokio::main]
async fn main() {
    // tell things to shutdown
    let (tx, rx) = broadcast::channel(1);
    // waiting for ctrl-c
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        println!("Ctrl-C pressed.");
        tx.send(()).unwrap();
    });
    // run
    if let Err(e) = run(rx).await {
        eprintln!("{e}");
    }
    // finalize rime if necessary
    if Rime::is_initialized() {
        Rime::global().destroy();
    }
}
