use lsp_types::MessageType;
use std::collections::HashMap;
use tower_lsp_server::Client;
use tree_sitter::{Node, Tree};

#[derive(Debug)]
pub enum ViewAnalysisError {
    ParseError(String),
    TextExtractionError,
    InvalidImportFormat(String),
}

impl std::fmt::Display for ViewAnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViewAnalysisError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            ViewAnalysisError::TextExtractionError => write!(f, "Failed to extract text from node"),
            ViewAnalysisError::InvalidImportFormat(format) => {
                write!(f, "Invalid import format: {format}")
            }
        }
    }
}

impl std::error::Error for ViewAnalysisError {}

type Result<T> = std::result::Result<T, ViewAnalysisError>;

struct AstTraversal;

impl AstTraversal {
    fn extract_node_text(node: &Node, text: &str) -> Result<String> {
        node.utf8_text(text.as_bytes())
            .map(|s| s.to_string())
            .map_err(|_| ViewAnalysisError::TextExtractionError)
    }

    fn find_nodes_by_kind<'a>(tree: &'a Tree, kind: &str) -> Vec<Node<'a>> {
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

    fn traverse_children<F>(node: &Node, mut callback: F)
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

    fn find_child_by_kind<'a>(node: &'a Node, kind: &str) -> Option<Node<'a>> {
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

#[derive(Debug, Clone)]
struct ImportInfo {
    pub namespace: String,
    pub function_name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewImportType {
    DirectNamespace,
    FunctionImport,
    FunctionImportWithAlias(String),
}

impl ViewImportType {
    pub fn description(&self) -> &'static str {
        match self {
            ViewImportType::DirectNamespace => "direct namespace",
            ViewImportType::FunctionImport => "function import",
            ViewImportType::FunctionImportWithAlias(_) => "function import with alias",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ViewParameter {
    pub name: Option<String>,
    pub value: String,
    pub raw_text: String,
}

#[derive(Debug, Clone)]
pub struct ViewCall {
    pub function_name: String,
    pub line: usize,
    pub text: String,
    pub parameters: Vec<ViewParameter>,
}

impl ViewCall {
    pub fn with_parameters(
        function_name: String,
        line: usize,
        text: String,
        parameters: Vec<ViewParameter>,
    ) -> Self {
        Self {
            function_name,
            line,
            text,
            parameters,
        }
    }
}

#[derive(Debug)]
pub struct ViewAnalysisResult {
    pub imports: HashMap<String, ViewImportType>,
    pub calls: Vec<ViewCall>,
}

impl ViewAnalysisResult {
    pub fn new() -> Self {
        Self {
            imports: HashMap::new(),
            calls: Vec::new(),
        }
    }

    pub fn call_count(&self) -> usize {
        self.calls.len()
    }

    #[allow(dead_code)]
    pub fn has_view_usage(&self) -> bool {
        !self.imports.is_empty() || !self.calls.is_empty()
    }
}

impl Default for ViewAnalysisResult {
    fn default() -> Self {
        Self::new()
    }
}

struct ImportAnalyzer;

impl ImportAnalyzer {
    fn analyze_imports(tree: &Tree, text: &str) -> Result<HashMap<String, ViewImportType>> {
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

struct FunctionCallAnalyzer;

impl FunctionCallAnalyzer {
    fn find_function_calls(tree: &Tree, text: &str) -> Result<Vec<ViewCall>> {
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

struct ViewAnalysisFormatter;

impl ViewAnalysisFormatter {
    async fn log_analysis_results(client: &Client, result: &ViewAnalysisResult, uri: &str) {
        if !result.imports.is_empty() {
            let import_summary = Self::format_import_summary(&result.imports);
            client
                .log_message(
                    MessageType::INFO,
                    format!("Available Tempest view functions in {uri}: {import_summary}"),
                )
                .await;
        }

        for call in &result.calls {
            let import_type = result
                .imports
                .get(&call.function_name)
                .map(|t| t.description())
                .unwrap_or("unknown");

            client
                .log_message(
                    MessageType::INFO,
                    format!(
                        "Found Tempest view call - name: '{}', type: {import_type}, line: {}, text: '{}'",
                        call.function_name, call.line, call.text
                    ),
                )
                .await;

            for (i, param) in call.parameters.iter().enumerate() {
                let param_info = match &param.name {
                    Some(name) => format!("named parameter '{}' = {}", name, param.value),
                    None => format!("positional parameter [{}] = {}", i, param.value),
                };

                client
                    .log_message(
                        MessageType::INFO,
                        format!("  Parameter: {} (raw: '{}')", param_info, param.raw_text),
                    )
                    .await;
            }

            if call.parameters.is_empty() {
                client
                    .log_message(MessageType::INFO, "  No parameters found".to_string())
                    .await;
            }
        }

        if result.call_count() > 0 {
            let summary = Self::format_call_summary(result);
            client.log_message(MessageType::INFO, summary).await;
        }
    }

    fn format_import_summary(imports: &HashMap<String, ViewImportType>) -> String {
        imports
            .iter()
            .map(|(name, import_type)| match import_type {
                ViewImportType::DirectNamespace => {
                    let desc = import_type.description();
                    format!("{name} ({desc})")
                }
                ViewImportType::FunctionImport => {
                    let desc = import_type.description();
                    format!("{name} ({desc})")
                }
                ViewImportType::FunctionImportWithAlias(alias) => {
                    format!("{alias} (alias for view)")
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn format_call_summary(result: &ViewAnalysisResult) -> String {
        let call_details: Vec<String> = result
            .calls
            .iter()
            .map(|call| {
                let import_type = result
                    .imports
                    .get(&call.function_name)
                    .map(|t| match t {
                        ViewImportType::DirectNamespace => "direct",
                        ViewImportType::FunctionImport => "imported",
                        ViewImportType::FunctionImportWithAlias(_) => "aliased",
                    })
                    .unwrap_or("unknown");

                format!("  - Line {}: {} ({import_type})", call.line, call.text)
            })
            .collect();

        let call_count = result.call_count();
        let details = call_details.join("\n");
        format!("Found {call_count} Tempest view() calls:\n{details}")
    }
}

pub struct ViewIntelligence;

impl ViewIntelligence {
    pub async fn analyze_document(client: &Client, tree: &Tree, text: &str, uri: &str) {
        let mut result = ViewAnalysisResult::new();

        match ImportAnalyzer::analyze_imports(tree, text) {
            Ok(imports) => result.imports = imports,
            Err(e) => {
                client
                    .log_message(
                        MessageType::ERROR,
                        format!("Import analysis failed for {uri}: {e}"),
                    )
                    .await;
                return;
            }
        }

        let all_calls = match FunctionCallAnalyzer::find_function_calls(tree, text) {
            Ok(calls) => calls,
            Err(e) => {
                client
                    .log_message(
                        MessageType::ERROR,
                        format!("Function call analysis failed for {uri}: {e}"),
                    )
                    .await;
                return;
            }
        };

        result.calls = all_calls
            .into_iter()
            .filter(|call| result.imports.contains_key(&call.function_name))
            .collect();

        ViewAnalysisFormatter::log_analysis_results(client, &result, uri).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempest_php_parser::PhpParser;

    fn parse_php_code(code: &str) -> Result<Tree> {
        let parser = PhpParser::new().map_err(|e| ViewAnalysisError::ParseError(e.to_string()))?;
        parser
            .parse(code, None)
            .map_err(|e| ViewAnalysisError::ParseError(e.to_string()))
    }

    fn analyze_imports_sync(code: &str) -> Result<HashMap<String, ViewImportType>> {
        let tree = parse_php_code(code)?;
        ImportAnalyzer::analyze_imports(&tree, code)
    }

    fn analyze_calls_sync(code: &str) -> Result<Vec<ViewCall>> {
        let tree = parse_php_code(code)?;
        FunctionCallAnalyzer::find_function_calls(&tree, code)
    }
    #[test]
    fn test_direct_namespace_calls_work() {
        let code = r#"<?php
namespace My\Namespace\Controllers;

use Tempest\View\View;

final readonly class HomeController
{
    public function __invoke(): View
    {
        return Tempest\view(__DIR__ . '/../Views/home.view.php');
    }
    
    public function other(): View
    {
        return \Tempest\view(__DIR__ . '/../Views/other.view.php');
    }
}"#;

        let imports = analyze_imports_sync(code).unwrap();
        let calls = analyze_calls_sync(code).unwrap();

        assert!(imports.contains_key("Tempest\\view"));
        assert!(imports.contains_key("\\Tempest\\view"));

        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].function_name, "Tempest\\view");
        assert_eq!(calls[1].function_name, "\\Tempest\\view");

        assert_eq!(calls[0].parameters.len(), 1);
        assert_eq!(calls[1].parameters.len(), 1);
    }

    #[test]
    fn test_simple_function_import() {
        let code = r#"<?php
namespace My\Namespace\Controllers;

use Tempest\View\View;
use function Tempest\view;

final readonly class HomeController
{
    public function __invoke(): View
    {
        return view(__DIR__ . '/../Views/home.view.php');
    }
}"#;

        let imports = analyze_imports_sync(code).unwrap();
        let calls = analyze_calls_sync(code).unwrap();

        assert!(imports.contains_key("view"));
        assert_eq!(
            imports.get("view").unwrap(),
            &ViewImportType::FunctionImport
        );

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function_name, "view");
    }

    #[test]
    fn test_grouped_function_import() {
        let code = r#"<?php
namespace My\Namespace\Controllers;

use Tempest\View\View;
use function Tempest\{root_path, view};

final readonly class HomeController
{
    public function __invoke(): View
    {
        return view(__DIR__ . '/../Views/home.view.php');
    }
}"#;

        let imports = analyze_imports_sync(code).unwrap();
        let calls = analyze_calls_sync(code).unwrap();

        assert!(imports.contains_key("view"));
        assert_eq!(
            imports.get("view").unwrap(),
            &ViewImportType::FunctionImport
        );

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function_name, "view");
    }

    #[test]
    fn test_aliased_function_import() {
        let code = r#"<?php
namespace My\Namespace\Controllers;

use Tempest\View\View;
use function Tempest\view as SomeMethod;

final readonly class HomeController
{
    public function __invoke(): View
    {
        return SomeMethod(__DIR__ . '/../Views/home.view.php');
    }
}"#;

        let imports = analyze_imports_sync(code).unwrap();
        let calls = analyze_calls_sync(code).unwrap();

        assert!(imports.contains_key("SomeMethod"));
        assert_eq!(
            imports.get("SomeMethod").unwrap(),
            &ViewImportType::FunctionImportWithAlias("SomeMethod".to_string())
        );

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function_name, "SomeMethod");
    }

    #[test]
    fn test_grouped_import_without_view() {
        let code = r#"<?php
namespace My\Namespace\Controllers;

use Tempest\View\View;
use function Tempest\{root_path, helper};

final readonly class HomeController
{
    public function __invoke(): View
    {
        return view(__DIR__ . '/../Views/home.view.php');
    }
}"#;

        let imports = analyze_imports_sync(code).unwrap();
        let calls = analyze_calls_sync(code).unwrap();

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function_name, "view");

        let filtered_calls: Vec<_> = calls
            .into_iter()
            .filter(|call| imports.contains_key(&call.function_name))
            .collect();

        assert_eq!(filtered_calls.len(), 0);
    }

    #[test]
    fn test_mixed_imports_and_calls() {
        let code = r#"<?php
namespace My\Namespace\Controllers;

use Tempest\View\View;
use function Tempest\view;
use function Tempest\view as render;

final readonly class HomeController
{
    public function one(): View
    {
        return view(__DIR__ . '/../Views/one.view.php');
    }
    
    public function two(): View  
    {
        return render(__DIR__ . '/../Views/two.view.php');
    }
    
    public function three(): View
    {
        return Tempest\view(__DIR__ . '/../Views/three.view.php');
    }
    
    public function four(): View
    {
        return \Tempest\view(__DIR__ . '/../Views/four.view.php');
    }
}"#;

        let imports = analyze_imports_sync(code).unwrap();
        let mut calls = analyze_calls_sync(code).unwrap();

        assert!(imports.contains_key("view"));
        assert!(imports.contains_key("render"));
        assert!(imports.contains_key("Tempest\\view"));
        assert!(imports.contains_key("\\Tempest\\view"));

        calls.retain(|call| imports.contains_key(&call.function_name));

        assert_eq!(calls.len(), 4);

        let call_names: Vec<&str> = calls.iter().map(|c| c.function_name.as_str()).collect();
        assert!(call_names.contains(&"view"));
        assert!(call_names.contains(&"render"));
        assert!(call_names.contains(&"Tempest\\view"));
        assert!(call_names.contains(&"\\Tempest\\view"));
    }

    #[test]
    fn test_parameter_parsing() {
        let code = r#"<?php
namespace My\Namespace\Controllers;

use function Tempest\view;

final readonly class HomeController
{
    public function simple(): View
    {
        return view('template.view.php');
    }
    
    public function withData(): View
    {
        return view('template.view.php', ['key' => 'value']);
    }
    
    public function complex(): View
    {
        return view(
            __DIR__ . '/../Views/home.view.php',
            $this->getData(),
            $options
        );
    }
}"#;

        let calls = analyze_calls_sync(code).unwrap();

        let view_calls: Vec<_> = calls
            .into_iter()
            .filter(|call| call.function_name == "view")
            .collect();

        assert_eq!(view_calls.len(), 3);

        assert_eq!(view_calls[0].parameters.len(), 1);
        assert_eq!(view_calls[0].parameters[0].value, "'template.view.php'");
        assert!(view_calls[0].parameters[0].name.is_none());

        assert_eq!(view_calls[1].parameters.len(), 2);
        assert_eq!(view_calls[1].parameters[0].value, "'template.view.php'");
        assert_eq!(view_calls[1].parameters[1].value, "['key' => 'value']");

        assert_eq!(view_calls[2].parameters.len(), 3);
        assert_eq!(
            view_calls[2].parameters[0].value,
            "__DIR__ . '/../Views/home.view.php'"
        );
        assert_eq!(view_calls[2].parameters[1].value, "$this->getData()");
        assert_eq!(view_calls[2].parameters[2].value, "$options");
    }

    #[test]
    fn test_named_parameter_parsing() {
        let code = r#"<?php
namespace Happytodev\Cyclone\Controllers;

use Tempest\View\View;
use function Tempest\{root_path, view};

final readonly class HomeController
{
    public function __invoke(): View
    {
        return view(path: __DIR__ . '/../Views/home.view.php');
    }
}"#;

        let calls = analyze_calls_sync(code).unwrap();

        let view_calls: Vec<_> = calls
            .into_iter()
            .filter(|call| call.function_name == "view")
            .collect();

        assert_eq!(view_calls.len(), 1);
        assert_eq!(view_calls[0].parameters.len(), 1);

        assert_eq!(view_calls[0].parameters[0].name, Some("path".to_string()));
        assert_eq!(
            view_calls[0].parameters[0].value,
            "__DIR__ . '/../Views/home.view.php'"
        );
        assert_eq!(
            view_calls[0].parameters[0].raw_text,
            "path: __DIR__ . '/../Views/home.view.php'"
        );
    }
}
