//! Phase 5 project scaffolding — `mom new` / `mom init`.
//!
//! Lays down the canonical project layout:
//!
//! ```text
//! my-app/
//!     mom.toml
//!     src/main.mom
//!     tests/smoke_test.mom
//!     .gitignore
//! ```
//!
//! Both commands write the same files; the only difference is whether
//! the destination directory is created (`new`) or already exists
//! (`init`).

use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, LangResult};

pub fn new_project(target: &Path) -> LangResult<ScaffoldReport> {
    if target.exists() {
        return Err(Diagnostic::at_start(format!(
            "destination '{}' already exists",
            target.display()
        )));
    }
    fs::create_dir_all(target).map_err(|err| {
        Diagnostic::at_start(format!(
            "failed to create '{}': {err}",
            target.display()
        ))
    })?;
    scaffold(target)
}

pub fn init_project(target: &Path) -> LangResult<ScaffoldReport> {
    if !target.exists() {
        fs::create_dir_all(target).map_err(|err| {
            Diagnostic::at_start(format!(
                "failed to create '{}': {err}",
                target.display()
            ))
        })?;
    }
    scaffold(target)
}

#[derive(Debug, Clone)]
pub struct ScaffoldReport {
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
}

fn scaffold(target: &Path) -> LangResult<ScaffoldReport> {
    let name = target
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("app")
        .to_string();

    let src = target.join("src");
    let tests = target.join("tests");
    fs::create_dir_all(&src).map_err(|err| io_err(&src, err))?;
    fs::create_dir_all(&tests).map_err(|err| io_err(&tests, err))?;

    let manifest_path = target.join("mom.toml");
    write_unless_exists(&manifest_path, &default_manifest(&name))?;

    let main_path = src.join("main.mom");
    write_unless_exists(&main_path, DEFAULT_MAIN)?;

    let test_path = tests.join("smoke_test.mom");
    write_unless_exists(&test_path, DEFAULT_TEST)?;

    let gitignore_path = target.join(".gitignore");
    write_unless_exists(&gitignore_path, DEFAULT_GITIGNORE)?;

    Ok(ScaffoldReport {
        root: target.to_path_buf(),
        files: vec![manifest_path, main_path, test_path, gitignore_path],
    })
}

fn write_unless_exists(path: &Path, contents: &str) -> LangResult<()> {
    if path.exists() {
        return Ok(());
    }
    fs::write(path, contents).map_err(|err| io_err(path, err))
}

fn io_err(path: &Path, err: std::io::Error) -> Diagnostic {
    Diagnostic::at_start(format!("failed to write '{}': {err}", path.display()))
}

fn default_manifest(name: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
         [dependencies]\n\n\
         [lints]\ndefault = \"warn\"\n"
    )
}

const DEFAULT_MAIN: &str = "fn main() {\n    print(\"hello, mom\")\n}\n";

const DEFAULT_TEST: &str = "// smoke_test.mom — run via `mom test`.\n\
fn main() {\n    let answer = 1 + 1\n    print(answer)\n}\n";

const DEFAULT_GITIGNORE: &str = "/target\n";
