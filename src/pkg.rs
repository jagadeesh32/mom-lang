//! Phase 5 package manager surface — `mom pkg`.
//!
//! No network in Phase 5: this is the on-disk editor for the
//! `[dependencies]` table in `mom.toml` plus a basic `audit` that
//! checks for unpinned versions. The registry/lockfile layer lands
//! in Phase 5.1 once the native stage-2 compiler is up.
//!
//! Subcommands:
//!   * `mom pkg list`               — print every dependency
//!   * `mom pkg add <name> <ver>`   — pin a dependency
//!   * `mom pkg remove <name>`      — drop a dependency
//!   * `mom pkg audit`              — flag unpinned (`*`) entries

use crate::diagnostic::{Diagnostic, LangResult};
use crate::manifest::{Manifest, Value};

const SECTION: &str = "dependencies";

pub fn list(manifest: &Manifest) -> Vec<(String, String)> {
    let Some(table) = manifest.section(SECTION) else {
        return Vec::new();
    };
    let mut out: Vec<(String, String)> = Vec::new();
    for (name, value) in table {
        let rendered = match value {
            Value::String(s) => s.clone(),
            Value::Integer(i) => i.to_string(),
            Value::Bool(b) => b.to_string(),
        };
        out.push((name.clone(), rendered));
    }
    out
}

pub fn add(manifest: &mut Manifest, name: &str, version: &str) -> LangResult<()> {
    validate_name(name)?;
    manifest.upsert(SECTION, name, Value::String(version.to_string()));
    manifest.save()
}

pub fn remove(manifest: &mut Manifest, name: &str) -> LangResult<bool> {
    let removed = manifest.remove(SECTION, name);
    if removed {
        manifest.save()?;
    }
    Ok(removed)
}

#[derive(Debug, Clone)]
pub struct AuditFinding {
    pub name: String,
    pub issue: String,
}

pub fn audit(manifest: &Manifest) -> Vec<AuditFinding> {
    let mut findings: Vec<AuditFinding> = Vec::new();
    let Some(table) = manifest.section(SECTION) else {
        return findings;
    };
    for (name, value) in table {
        match value {
            Value::String(s) if s == "*" => findings.push(AuditFinding {
                name: name.clone(),
                issue: "unpinned version `*` — replace with an exact requirement".to_string(),
            }),
            Value::String(s) if s.trim().is_empty() => findings.push(AuditFinding {
                name: name.clone(),
                issue: "empty version requirement".to_string(),
            }),
            Value::String(_) => {}
            _ => findings.push(AuditFinding {
                name: name.clone(),
                issue: "version must be a string".to_string(),
            }),
        }
    }
    findings
}

fn validate_name(name: &str) -> LangResult<()> {
    if name.is_empty() {
        return Err(Diagnostic::at_start("package name must be non-empty"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(Diagnostic::at_start(format!(
            "invalid package name '{name}' — use `[A-Za-z0-9_-]+`"
        )));
    }
    Ok(())
}
