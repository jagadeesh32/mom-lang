//! Phase 5 test runner — `mom test`.
//!
//! Discovery rules (in order):
//!   1. Every `*.mom` file under `tests/`.
//!   2. Every `*_test.mom` file under `src/`.
//!
//! Each discovered file is parsed, type-checked, borrow-checked, then
//! executed by the bootstrap interpreter. A test "passes" when the
//! interpreter finishes without error and the program exits with the
//! `Unit` value. Programs may `print(...)` debug output; that output
//! is captured and only shown on failure.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::diagnostic::{Diagnostic, LangResult};
use crate::{borrow, interpreter, parse_source, typechecker};

#[derive(Debug, Clone)]
pub struct TestOutcome {
    pub path: PathBuf,
    pub passed: bool,
    pub message: String,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Default)]
pub struct TestReport {
    pub outcomes: Vec<TestOutcome>,
}

impl TestReport {
    pub fn passed(&self) -> usize {
        self.outcomes.iter().filter(|o| o.passed).count()
    }
    pub fn failed(&self) -> usize {
        self.outcomes.iter().filter(|o| !o.passed).count()
    }
    pub fn total(&self) -> usize {
        self.outcomes.len()
    }
    pub fn all_passed(&self) -> bool {
        self.failed() == 0
    }
}

pub fn discover(root: &Path) -> LangResult<Vec<PathBuf>> {
    let mut hits: Vec<PathBuf> = Vec::new();
    walk(&root.join("tests"), &mut |p| {
        if has_extension(p, "mom") {
            hits.push(p.to_path_buf());
        }
    })?;
    walk(&root.join("src"), &mut |p| {
        if has_extension(p, "mom") {
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                if stem.ends_with("_test") {
                    hits.push(p.to_path_buf());
                }
            }
        }
    })?;
    hits.sort();
    Ok(hits)
}

pub fn run_all(root: &Path) -> LangResult<TestReport> {
    let files = discover(root)?;
    let mut outcomes: Vec<TestOutcome> = Vec::with_capacity(files.len());
    for path in files {
        outcomes.push(run_one(&path));
    }
    Ok(TestReport { outcomes })
}

fn run_one(path: &Path) -> TestOutcome {
    let start = Instant::now();
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(err) => {
            return TestOutcome {
                path: path.to_path_buf(),
                passed: false,
                message: format!("read failed: {err}"),
                elapsed_ms: start.elapsed().as_millis(),
            };
        }
    };

    match execute(&source) {
        Ok(out) => TestOutcome {
            path: path.to_path_buf(),
            passed: true,
            message: out,
            elapsed_ms: start.elapsed().as_millis(),
        },
        Err(diag) => TestOutcome {
            path: path.to_path_buf(),
            passed: false,
            message: diag.to_string(),
            elapsed_ms: start.elapsed().as_millis(),
        },
    }
}

fn execute(source: &str) -> LangResult<String> {
    let program = parse_source(source)?;
    typechecker::TypeChecker::new().check_program(&program)?;
    borrow::BorrowChecker::new().check_program(&program)?;
    interpreter::Interpreter::new().run_program(&program)
}

fn walk(dir: &Path, on_file: &mut dyn FnMut(&Path)) -> LangResult<()> {
    if !dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|err| {
        Diagnostic::at_start(format!("failed to read '{}': {err}", dir.display()))
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            Diagnostic::at_start(format!("failed to read '{}': {err}", dir.display()))
        })?;
        let path = entry.path();
        if path.is_dir() {
            walk(&path, on_file)?;
        } else {
            on_file(&path);
        }
    }
    Ok(())
}

fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some(ext)
}
