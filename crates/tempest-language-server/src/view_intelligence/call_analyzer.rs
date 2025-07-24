use crate::view_intelligence::ast_traversal::AstTraversal;
use crate::view_intelligence::types::{Result, ViewAnalysisError, ViewCall, ViewParameter};
use tree_sitter::{Node, Tree};

pub struct FunctionCallAnalyzer;

impl FunctionCallAnalyzer {
    pub fn find_function_calls(tree: &Tree, text: &str) -> Result<Vec<ViewCall>> {
        let call_nodes = AstTraversal::find_nodes_by_kind(tree, "function_call_expression");
        let mut calls = Vec::new();

        for node in call_nodes {
            if let Ok(view_call) = Self::extract_view_call_info(&node, text) {
                calls.push(view_call);
            }
        }

        Ok(calls)
    }

    fn extract_view_call_info(node: &Node, text: &str) -> Result<ViewCall> {
        let function_node =
            node.child_by_field_name("function")
                .ok_or(ViewAnalysisError::ParseError(
                    "Function call missing function field".to_string(),
                ))?;

        let function_name = AstTraversal::extract_node_text(&function_node, text)?;
        let line = node.start_position().row + 1;
        let call_text = AstTraversal::extract_node_text(node, text)?;

        let parameters = Self::parse_function_parameters(node, text)?;

        Ok(ViewCall::with_parameters(
            function_name,
            line,
            call_text,
            parameters,
        ))
    }

    fn parse_function_parameters(node: &Node, text: &str) -> Result<Vec<ViewParameter>> {
        let mut parameters = Vec::new();

        if let Some(arguments_node) = node.child_by_field_name("arguments") {
            AstTraversal::traverse_children(&arguments_node, |child| {
                if child.kind() == "argument" {
                    if let Ok(param) = Self::parse_single_argument(child, text) {
                        parameters.push(param);
                    }
                }
            });
        }

        Ok(parameters)
    }

    fn parse_single_argument(node: &Node, text: &str) -> Result<ViewParameter> {
        let raw_text = AstTraversal::extract_node_text(node, text)?;

        if let Some(name_node) = node.child_by_field_name("name") {
            let name = AstTraversal::extract_node_text(&name_node, text)?;

            let mut value_parts = Vec::new();
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                loop {
                    let child = cursor.node();
                    if cursor.field_name() != Some("name")
                        && !child.kind().contains(':')
                        && child.kind() != ":"
                        && (child.child_count() > 0
                            || !child.kind().chars().all(|c| c.is_ascii_punctuation()))
                    {
                        if let Ok(child_text) = AstTraversal::extract_node_text(&child, text) {
                            value_parts.push(child_text);
                        }
                    }
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }

            let value = value_parts.join(" ");
            return Ok(ViewParameter {
                name: Some(name),
                value,
                raw_text,
            });
        }

        let value = raw_text.clone();
        Ok(ViewParameter {
            name: None,
            value,
            raw_text,
        })
    }
}
