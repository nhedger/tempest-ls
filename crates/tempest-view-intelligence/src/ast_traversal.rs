use crate::types::{Result, ViewAnalysisError};
use tree_sitter::{Node, Tree};

pub struct AstTraversal;

impl AstTraversal {
    pub fn extract_node_text(node: &Node, text: &str) -> Result<String> {
        node.utf8_text(text.as_bytes())
            .map(|s| s.to_string())
            .map_err(|_| ViewAnalysisError::TextExtractionError)
    }

    pub fn find_nodes_by_kind<'a>(tree: &'a Tree, kind: &str) -> Vec<Node<'a>> {
        let mut nodes = Vec::new();
        let mut cursor = tree.walk();

        loop {
            let node = cursor.node();
            if node.kind() == kind {
                nodes.push(node);
            }

            if cursor.goto_first_child() {
                continue;
            }

            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return nodes;
                }
            }
        }
    }

    pub fn traverse_children<F>(node: &Node, mut callback: F)
    where
        F: FnMut(&Node),
    {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                callback(&cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    pub fn find_child_by_kind<'a>(node: &'a Node, kind: &str) -> Option<Node<'a>> {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == kind {
                    return Some(child);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        None
    }
}
