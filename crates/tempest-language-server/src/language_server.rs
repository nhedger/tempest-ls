use crate::document::Document;
use crate::view_intelligence::ViewIntelligence;
use dashmap::DashMap;
use lsp_types::{
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
    InitializedParams, MessageType, ServerCapabilities, ServerInfo, TextDocumentItem,
    TextDocumentSyncCapability, TextDocumentSyncOptions, Uri,
};
use std::process::exit;
use tempest_php_parser::PhpParser;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::{Client, LanguageServer};

pub struct TempestLanguageServer {
    pub(crate) parser: PhpParser,
    pub(crate) client: Client,
    documents: DashMap<Uri, Document>,
}

impl TempestLanguageServer {
    /// Create a new Tempest Language Server instance.
    pub fn new(client: Client) -> Self {
        let parser = match PhpParser::new() {
            Ok(parser) => parser,
            Err(_) => exit(1),
        };

        Self {
            parser,
            client,
            documents: DashMap::new(),
        }
    }

    /// Register a document with the server
    ///
    /// This function will parse the document and store it in the server's internal list of documents.
    pub async fn register_document(&self, text_document: TextDocumentItem) {
        // If the document is not a PHP file, skip
        if text_document.language_id != "php" && text_document.language_id != "tempest-view" {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("Skipping non-PHP document: {}", *text_document.uri),
                )
                .await;
            return;
        }

        let parsed = match self.parser.parse(&text_document.text, None) {
            Ok(tree) => tree,
            Err(_) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("Could not parse document: {}", *text_document.uri),
                    )
                    .await;
                return;
            }
        };

        let document = Document {
            uri: text_document.uri.clone(),
            text: text_document.text.clone(),
            tree: parsed,
            version: text_document.version,
        };

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "Registered document {}, parsed as:\n\n {}",
                    *text_document.uri,
                    document.tree.root_node().to_sexp()
                ),
            )
            .await;

        // Analyze the document for Tempest view() calls
        ViewIntelligence::analyze_document(
            &self.client,
            &document.tree,
            &document.text,
            &text_document.uri.to_string(),
        )
        .await;

        self.documents.insert(text_document.uri.clone(), document);
    }

    /// Unregister a document from the server
    ///
    /// This function will remove a document from the server's internal list of documents.
    pub async fn unregister_document(&self, uri: Uri) {
        self.documents.remove(&uri);

        self.client
            .log_message(MessageType::INFO, format!("Unregistered document {}", *uri))
            .await;
    }
}

impl LanguageServer for TempestLanguageServer {
    /// Handle the initialization request
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "Tempest Language Server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        ..TextDocumentSyncOptions::default()
                    },
                )),
                ..ServerCapabilities::default()
            },
        })
    }

    /// Handle the server being initialized
    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    /// Handle a text document being opened
    ///
    /// This function will be triggered whenever a text document is opened in the editor.
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.register_document(params.text_document).await;
    }

    /// Handle a text document being closed
    ///
    /// This function will be triggered whenever a text document is closed in the editor.
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.unregister_document(params.text_document.uri).await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
