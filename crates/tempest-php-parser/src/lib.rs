use std::sync::Mutex;
use tree_sitter::{Parser, Tree};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PhpParserError {
    #[error("Parser initialization error: {0}")]
    UnableToInitialize(String),

    #[error("Parser lock error: {0}")]
    UnableToAcquireLock(String),

    #[error("Unable to parse source code")]
    UnableToParse,
}

pub struct PhpParser {
    parser: Mutex<Parser>
}

impl PhpParser {
    pub fn new() -> Result<Self, PhpParserError> {
        let mut parser = Parser::new();

        if parser.set_language(&tree_sitter_php::LANGUAGE_PHP.into()).is_err() {
            return Err(PhpParserError::UnableToInitialize("Unable to create parser for PHP".to_string()))
        }

        Ok(Self {
            parser: Mutex::new(parser)
        })
    }

    pub fn parse(&self, source_code: &str, old_tree: Option<&Tree>) -> Result<Tree, PhpParserError> {

        // Acquire a lock on the parser
        let mut parser = match self.parser.lock() {
            Ok(parser) => parser,
            Err(_) => return Err(PhpParserError::UnableToAcquireLock("Could not get a lock on the parser".to_string())),
        };

        // Parse the source code
        match parser.parse(source_code, old_tree) {
            Some(tree) => Ok(tree),
            None => Err(PhpParserError::UnableToParse),
        }
    }
}