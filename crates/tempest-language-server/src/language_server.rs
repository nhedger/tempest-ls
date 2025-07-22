use lsp_types::{InitializeParams, InitializeResult, InitializedParams, MessageType};
use tower_lsp_server::{Client, LanguageServer};
use tower_lsp_server::jsonrpc::Result;

#[derive(Debug)]
pub struct TempestLanguageServer {
    pub(crate) client: Client,
}

impl LanguageServer for TempestLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult::default())
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}