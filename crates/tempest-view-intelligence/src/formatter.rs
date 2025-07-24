use crate::types::{ViewAnalysisResult, ViewImportType};
use lsp_types::MessageType;
use std::collections::HashMap;
use tower_lsp_server::Client;

pub struct ViewAnalysisFormatter;

impl ViewAnalysisFormatter {
    pub async fn log_analysis_results(client: &Client, result: &ViewAnalysisResult, uri: &str) {
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
