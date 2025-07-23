use lsp_types::Uri;
use tower_lsp_server::Client;
use tree_sitter::Tree;

pub struct Document {
    pub(crate) uri: Uri,
    pub(crate) text: String,
    pub(crate) tree: Tree,
    pub(crate) version: i32,
}