use monty::{Executor, Exit};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;

/// Represents the expected outcome of a test fixture
#[derive(Debug)]
enum Expectation {
    /// Expect exception with specific message
    Raise(String),
    /// Expect parse error containing message
    ParseError(String),
    /// Expect successful execution, check py_str() output
    ReturnStr(String),
    /// Expect successful execution, check py_repr() output
    Return(String),
    /// Expect successful execution, check py_type() output
    ReturnType(String),
    /// Expect successful execution, check ref counts of named variables.
    /// Only used when `ref-counting` feature is enabled; skipped otherwise.
    RefCounts(#[cfg_attr(not(feature = "ref-counting"), allow(dead_code))] HashMap<String, usize>),
}

/// Parse a Python fixture file into code and expected outcome
///
/// The file MUST have an expectation comment as the LAST line:
/// - `# Raise=ExceptionType('message')` - Exception format
/// - `# ParseError=message` - Parse error format
/// - `# Return.str=value` - Check py_str() output
/// - `# Return=value` - Check py_repr() output
/// - `# Return.type=typename` - Check py_type() output
/// - `# ref-counts={'var': count, ...}` - Check ref counts of named heap variables
fn parse_fixture(content: &str) -> (String, Expectation) {
    let lines: Vec<&str> = content.lines().collect();

    assert!(!lines.is_empty(), "Empty fixture file");

    // Check if first line has an expectation (this is an error)
    if let Some(first_line) = lines.first() {
        if first_line.starts_with("# Return")
            || first_line.starts_with("# Raise")
            || first_line.starts_with("# ParseError")
        {
            panic!("Expectation comment must be on the LAST line, not the first line");
        }
    }

    // Get the last line (must be the expectation)
    let last_line = lines.last().unwrap();
    let expectation_line = last_line;
    let code_lines = &lines[..lines.len() - 1];

    // Parse expectation from comment line
    // Note: Check more specific patterns first (Return.str, Return.type, ref-counts) before general Return
    let expectation = if let Some(expected) = expectation_line.strip_prefix("# ref-counts=") {
        Expectation::RefCounts(parse_ref_counts(expected))
    } else if let Some(expected) = expectation_line.strip_prefix("# Return.str=") {
        Expectation::ReturnStr(expected.to_string())
    } else if let Some(expected) = expectation_line.strip_prefix("# Return.type=") {
        Expectation::ReturnType(expected.to_string())
    } else if let Some(expected) = expectation_line.strip_prefix("# Return=") {
        Expectation::Return(expected.to_string())
    } else if let Some(expected) = expectation_line.strip_prefix("# Raise=") {
        Expectation::Raise(expected.to_string())
    } else if let Some(expected) = expectation_line.strip_prefix("# ParseError=") {
        Expectation::ParseError(expected.to_string())
    } else {
        panic!("Invalid expectation format in comment line: {expectation_line}");
    };

    // Code is everything except the expectation comment line
    let code = code_lines.join("\n");

    (code, expectation)
}

/// Parses the ref-counts format: {'var': count, 'var2': count2}
///
/// Supports both single and double quotes for variable names.
/// Example: {'x': 2, 'y': 1} or {"x": 2, "y": 1}
fn parse_ref_counts(s: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    let trimmed = s.trim().trim_start_matches('{').trim_end_matches('}');
    for pair in trimmed.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let parts: Vec<&str> = pair.split(':').collect();
        assert!(
            parts.len() == 2,
            "Invalid ref-counts pair format: {pair}. Expected 'name': count"
        );
        let name = parts[0].trim().trim_matches('\'').trim_matches('"');
        let count: usize = parts[1]
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("Invalid ref count value: {}", parts[1]));
        counts.insert(name.to_string(), count);
    }
    counts
}

/// Run a test with the given code and expectation
///
/// This function executes Python code via the Executor and validates the result
/// against the expected outcome specified in the fixture.
fn run_test(path: &Path, code: &str, expectation: Expectation) {
    let test_name = path.strip_prefix("test_cases/").unwrap_or(path).display().to_string();

    match Executor::new(code, "test.py", &[]) {
        Ok(mut ex) => {
            let result = ex.run(vec![]);
            match result {
                Ok(Exit::Return(obj)) => match expectation {
                    Expectation::ReturnStr(expected) => {
                        let output = obj.py_str();
                        assert_eq!(output.as_ref(), expected, "[{test_name}] py_str() mismatch");
                    }
                    Expectation::Return(expected) => {
                        let output = obj.py_repr();
                        assert_eq!(output.as_ref(), expected, "[{test_name}] py_repr() mismatch");
                    }
                    Expectation::ReturnType(expected) => {
                        let output = obj.py_type();
                        assert_eq!(output, expected, "[{test_name}] py_type() mismatch");
                    }
                    #[cfg(feature = "ref-counting")]
                    Expectation::RefCounts(expected) => {
                        // Drop obj to release any borrows on the executor
                        drop(obj);
                        let (actual, unique_refs, heap_count) =
                            ex.get_ref_counts().expect("ref counts should be available after run");

                        // Strict matching: verify all heap objects are accounted for by variables
                        // unique_refs = number of unique heap IDs referenced by variables
                        // heap_count = total number of heap objects
                        // If they differ, there are heap objects not reachable from any variable
                        assert_eq!(
                            unique_refs, heap_count,
                            "[{test_name}] Strict matching failed: {heap_count} heap objects exist, \
                             but only {unique_refs} are referenced by variables.\n\
                             Actual ref counts: {actual:?}"
                        );

                        assert_eq!(actual, expected, "[{test_name}] ref-counts mismatch");
                    }
                    #[cfg(not(feature = "ref-counting"))]
                    Expectation::RefCounts(_) => {
                        // Skip ref-count tests when feature is disabled
                    }
                    _ => panic!("[{test_name}] Expected return, got different expectation type"),
                },
                Ok(Exit::Raise(exc)) => {
                    if let Expectation::Raise(expected) = expectation {
                        let output = format!("{}", exc.exc);
                        assert_eq!(output, expected, "[{test_name}] Exception mismatch");
                    } else {
                        panic!("[{test_name}] Unexpected exception: {exc:?}");
                    }
                }
                Err(e) => panic!("[{test_name}] Runtime error: {e:?}"),
            }
        }
        Err(parse_err) => {
            if let Expectation::ParseError(expected) = expectation {
                let err_msg = parse_err.summary();
                assert_eq!(err_msg, expected, "[{test_name}] Parse error mismatch");
            } else {
                panic!("[{test_name}] Unexpected parse error: {parse_err:?}");
            }
        }
    }
}

/// Test function that runs for each Python fixture file
fn run_fixture_test(path: &Path) -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string(path)?;

    let (code, expectation) = parse_fixture(&content);
    run_test(path, &code, expectation);
    Ok(())
}

// Generate tests for all fixture files using datatest-stable harness macro
// All fixtures are now in a flat structure with group prefixes (e.g., id__is_test.py)
datatest_stable::harness!(run_fixture_test, "test_cases", r"^.*\.py$");
