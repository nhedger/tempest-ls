use lsp_types::Uri;
use tree_sitter::Tree;

#[allow(dead_code)] // TODO: uri and version are not used yet
pub struct Document {
    pub(crate) uri: Uri,
    pub(crate) text: String,
    pub(crate) tree: Tree,
    pub(crate) version: i32,
}
