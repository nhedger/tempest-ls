mod language_server;
mod document;

use clap::Parser;
use tower_lsp_server::{LspService, Server};
use crate::language_server::TempestLanguageServer;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Tempest Language Server",
    long_about = None
)]
struct Cli {
    /// Use stdio as the communication channel
    /// This does nothing for now, we always use stdio
    #[arg(long)]
    stdio: bool,
}

#[tokio::main]
async fn main() {
    let _args = Cli::parse();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| TempestLanguageServer::new(client));

    Server::new(stdin, stdout, socket).serve(service).await;
}
