use crate::call_analyzer::FunctionCallAnalyzer;
use crate::import_analyzer::ImportAnalyzer;
use crate::types::{Result, ViewAnalysisError, ViewCall, ViewImportType};
#[cfg(test)]
use std::collections::HashMap;
use tempest_php_parser::PhpParser;
use tree_sitter::Tree;

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
