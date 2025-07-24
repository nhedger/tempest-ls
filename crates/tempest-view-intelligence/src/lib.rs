use lsp_types::MessageType;
use tower_lsp_server::Client;
use tree_sitter::Tree;

// Module declarations
mod ast_traversal;
mod call_analyzer;
mod formatter;
mod import_analyzer;
mod types;

// Re-export main public types and analyzers for external use
pub use call_analyzer::FunctionCallAnalyzer;
use formatter::ViewAnalysisFormatter;
pub use import_analyzer::ImportAnalyzer;
pub use types::{
    ImportInfo, Result, ViewAnalysisError, ViewAnalysisResult, ViewCall, ViewImportType,
    ViewParameter,
};

// Tests module
#[cfg(test)]
mod tests;

/// Main API for view intelligence analysis
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

    /// Find Tempest view() calls in a document (without LSP client dependency)
    /// Returns a Result containing the view calls found in the document
    pub fn find_view_calls(tree: &Tree, text: &str) -> Result<Vec<ViewCall>> {
        let imports = ImportAnalyzer::analyze_imports(tree, text)?;
        let all_calls = FunctionCallAnalyzer::find_function_calls(tree, text)?;

        let view_calls = all_calls
            .into_iter()
            .filter(|call| imports.contains_key(&call.function_name))
            .collect();

        Ok(view_calls)
    }

    /// Get complete analysis result for a document (without LSP client dependency)
    /// Returns both imports and filtered view calls
    pub fn analyze(tree: &Tree, text: &str) -> Result<ViewAnalysisResult> {
        let mut result = ViewAnalysisResult::new();

        result.imports = ImportAnalyzer::analyze_imports(tree, text)?;
        let all_calls = FunctionCallAnalyzer::find_function_calls(tree, text)?;

        result.calls = all_calls
            .into_iter()
            .filter(|call| result.imports.contains_key(&call.function_name))
            .collect();

        Ok(result)
    }
}
