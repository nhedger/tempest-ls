use crate::view_intelligence::ast_traversal::AstTraversal;
use crate::view_intelligence::types::{ImportInfo, Result, ViewAnalysisError, ViewImportType};
use std::collections::HashMap;
use tree_sitter::{Node, Tree};

pub struct ImportAnalyzer;

impl ImportAnalyzer {
    pub fn analyze_imports(tree: &Tree, text: &str) -> Result<HashMap<String, ViewImportType>> {
        let mut imports = HashMap::new();

        Self::add_direct_namespace_imports(&mut imports);

        let import_infos = Self::extract_import_statements(tree, text)?;

        for import_info in import_infos {
            if import_info.namespace == "Tempest" && import_info.function_name == "view" {
                let (import_name, import_type) = match import_info.alias {
                    Some(alias) => (
                        alias.clone(),
                        ViewImportType::FunctionImportWithAlias(alias),
                    ),
                    None => (
                        import_info.function_name.clone(),
                        ViewImportType::FunctionImport,
                    ),
                };

                imports.insert(import_name, import_type);
            }
        }

        Ok(imports)
    }

    fn add_direct_namespace_imports(imports: &mut HashMap<String, ViewImportType>) {
        imports.insert(
            "\\Tempest\\view".to_string(),
            ViewImportType::DirectNamespace,
        );
        imports.insert("Tempest\\view".to_string(), ViewImportType::DirectNamespace);
    }

    fn extract_import_statements(tree: &Tree, text: &str) -> Result<Vec<ImportInfo>> {
        let import_nodes = AstTraversal::find_nodes_by_kind(tree, "namespace_use_declaration");
        let mut import_infos = Vec::new();

        for node in import_nodes {
            if let Ok(import_info) = Self::parse_use_declaration_ast(&node, text) {
                import_infos.push(import_info);
            }
        }

        Ok(import_infos)
    }

    fn parse_use_declaration_ast(node: &Node, text: &str) -> Result<ImportInfo> {
        let is_function_import = Self::is_function_use_declaration(node, text)?;

        if !is_function_import {
            return Err(ViewAnalysisError::InvalidImportFormat(
                "Not a function import".to_string(),
            ));
        }

        Self::extract_use_clause_info(node, text)
    }

    fn is_function_use_declaration(node: &Node, text: &str) -> Result<bool> {
        if AstTraversal::find_child_by_kind(node, "function").is_some() {
            return Ok(true);
        }

        if let Some(clause_node) = AstTraversal::find_child_by_kind(node, "namespace_use_clause") {
            if let Ok(clause_text) = AstTraversal::extract_node_text(&clause_node, text) {
                return Ok(clause_text.trim_start().starts_with("function"));
            }
        }

        Ok(false)
    }

    fn extract_use_clause_info(node: &Node, text: &str) -> Result<ImportInfo> {
        if let Some(clause_node) = AstTraversal::find_child_by_kind(node, "namespace_use_clause") {
            return Self::parse_single_use_clause(&clause_node, text);
        }

        if let Some(group_node) = AstTraversal::find_child_by_kind(node, "namespace_use_group") {
            return Self::parse_grouped_use_clauses(&group_node, text);
        }

        Err(ViewAnalysisError::InvalidImportFormat(
            "No valid use clause found".to_string(),
        ))
    }

    fn parse_single_use_clause(node: &Node, text: &str) -> Result<ImportInfo> {
        let mut qualified_name_parts = Vec::new();
        let mut alias = None;

        AstTraversal::traverse_children(node, |child| match child.kind() {
            "qualified_name" => {
                if let Ok(parts) = Self::extract_qualified_name_parts(child, text) {
                    qualified_name_parts = parts;
                }
            }
            "name" => {
                if let Ok(name) = AstTraversal::extract_node_text(child, text) {
                    alias = Some(name);
                }
            }
            _ => {}
        });

        if qualified_name_parts.len() < 2 {
            return Err(ViewAnalysisError::InvalidImportFormat(
                "Invalid qualified name".to_string(),
            ));
        }

        let function_name = qualified_name_parts.pop().ok_or_else(|| {
            ViewAnalysisError::InvalidImportFormat(
                "Failed to extract function name from qualified name".to_string(),
            )
        })?;
        let namespace = qualified_name_parts.join("\\");

        Ok(ImportInfo {
            namespace,
            function_name,
            alias,
        })
    }

    fn parse_grouped_use_clauses(node: &Node, text: &str) -> Result<ImportInfo> {
        let mut namespace = String::new();
        let mut found_view = false;

        AstTraversal::traverse_children(node, |child| match child.kind() {
            "qualified_name" => {
                if let Ok(parts) = Self::extract_qualified_name_parts(child, text) {
                    if let Some(first_part) = parts.first() {
                        namespace = first_part.clone();
                    }
                }
            }
            "namespace_use_clause" => {
                AstTraversal::traverse_children(child, |clause_child| {
                    if clause_child.kind() == "name" {
                        if let Ok(name) = AstTraversal::extract_node_text(clause_child, text) {
                            if name == "view" {
                                found_view = true;
                            }
                        }
                    }
                });
            }
            _ => {}
        });

        if !found_view {
            return Err(ViewAnalysisError::InvalidImportFormat(
                "view function not found in grouped import".to_string(),
            ));
        }

        if namespace.is_empty() {
            namespace = "Tempest".to_string();
        }

        Ok(ImportInfo {
            namespace,
            function_name: "view".to_string(),
            alias: None,
        })
    }

    fn extract_qualified_name_parts(node: &Node, text: &str) -> Result<Vec<String>> {
        let mut parts = Vec::new();

        AstTraversal::traverse_children(node, |child| match child.kind() {
            "namespace_name" => {
                if let Ok(namespace_parts) = Self::extract_namespace_parts(child, text) {
                    parts.extend(namespace_parts);
                }
            }
            "name" => {
                if let Ok(name) = AstTraversal::extract_node_text(child, text) {
                    parts.push(name);
                }
            }
            _ => {}
        });

        Ok(parts)
    }

    fn extract_namespace_parts(node: &Node, text: &str) -> Result<Vec<String>> {
        let mut parts = Vec::new();

        AstTraversal::traverse_children(node, |child| {
            if child.kind() == "name" {
                if let Ok(name) = AstTraversal::extract_node_text(child, text) {
                    parts.push(name);
                }
            }
        });

        Ok(parts)
    }
}
