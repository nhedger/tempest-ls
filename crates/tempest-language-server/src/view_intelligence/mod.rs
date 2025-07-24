use lsp_types::MessageType;
use tower_lsp_server::Client;
use tree_sitter::Tree;

// Module declarations
mod ast_traversal;
mod call_analyzer;
mod formatter;
mod import_analyzer;
mod types;

// Re-export main public types (others available via full path if needed)
use call_analyzer::FunctionCallAnalyzer;
use formatter::ViewAnalysisFormatter;
use import_analyzer::ImportAnalyzer;
pub use types::ViewAnalysisResult;

// Tests module
#[cfg(test)]
mod tests;

/// Main API for view intelligence analysis
pub struct ViewIntelligence;

impl ViewIntelligence {
    /// Analyze a document for Tempest view function usage
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
