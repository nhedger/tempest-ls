mod language_server;
mod workspace;

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

    println!("Starting tempest language server...");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_php::LANGUAGE_PHP.into()).unwrap();

    let (service, socket) = LspService::new(|client| TempestLanguageServer { client, parser });

    Server::new(stdin, stdout, socket).serve(service).await;
}
