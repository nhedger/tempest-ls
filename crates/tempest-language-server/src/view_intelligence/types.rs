use std::collections::HashMap;

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

pub type Result<T> = std::result::Result<T, ViewAnalysisError>;

#[derive(Debug, Clone)]
pub struct ImportInfo {
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
