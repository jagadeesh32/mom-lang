//! Phase 5 manifest reader/writer for `mom.toml`.
//!
//! Pure-std subset of TOML — supports:
//!     * `[section]` / `[parent.child]` headers
//!     * `key = "string"` / `key = integer` / `key = true|false`
//!     * `# line comments`
//!
//! Round-trippable enough to power `mom new`, `mom pkg`, and `[lints]`
//! lookups. The native `mom` toolchain will replace this with a full
//! parser; this is intentionally tiny.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, LangResult};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Integer(i64),
    Bool(bool),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

pub type Table = BTreeMap<String, Value>;

#[derive(Debug, Clone, Default)]
pub struct Manifest {
    pub path: PathBuf,
    pub sections: BTreeMap<String, Table>,
    pub section_order: Vec<String>,
}

impl Manifest {
    pub fn parse(path: PathBuf, source: &str) -> LangResult<Self> {
        let mut sections: BTreeMap<String, Table> = BTreeMap::new();
        let mut order: Vec<String> = Vec::new();
        let mut current: String = String::new();

        for (lineno, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(rest) = line.strip_prefix('[') {
                let name = rest
                    .strip_suffix(']')
                    .ok_or_else(|| Diagnostic::at_start(format!(
                        "{}: unterminated section header on line {}",
                        path.display(),
                        lineno + 1
                    )))?
                    .trim()
                    .to_string();
                if !sections.contains_key(&name) {
                    sections.insert(name.clone(), Table::new());
                    order.push(name.clone());
                }
                current = name;
                continue;
            }

            let (key, value) = line.split_once('=').ok_or_else(|| {
                Diagnostic::at_start(format!(
                    "{}: expected `key = value` on line {}",
                    path.display(),
                    lineno + 1
                ))
            })?;
            let key = key.trim().to_string();
            let value = parse_value(value.trim()).ok_or_else(|| {
                Diagnostic::at_start(format!(
                    "{}: invalid value on line {}",
                    path.display(),
                    lineno + 1
                ))
            })?;

            if current.is_empty() {
                return Err(Diagnostic::at_start(format!(
                    "{}: key `{}` defined outside a section",
                    path.display(),
                    key
                )));
            }
            sections.get_mut(&current).unwrap().insert(key, value);
        }

        Ok(Manifest {
            path,
            sections,
            section_order: order,
        })
    }

    pub fn load(path: PathBuf) -> LangResult<Self> {
        let source = fs::read_to_string(&path).map_err(|err| {
            Diagnostic::at_start(format!("failed to read '{}': {err}", path.display()))
        })?;
        Self::parse(path, &source)
    }

    pub fn find(start: &Path) -> Option<PathBuf> {
        for ancestor in start.ancestors() {
            let candidate = ancestor.join("mom.toml");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    pub fn section(&self, name: &str) -> Option<&Table> {
        self.sections.get(name)
    }

    pub fn upsert(&mut self, section: &str, key: &str, value: Value) {
        if !self.sections.contains_key(section) {
            self.sections.insert(section.to_string(), Table::new());
            self.section_order.push(section.to_string());
        }
        self.sections
            .get_mut(section)
            .unwrap()
            .insert(key.to_string(), value);
    }

    pub fn remove(&mut self, section: &str, key: &str) -> bool {
        self.sections
            .get_mut(section)
            .map(|t| t.remove(key).is_some())
            .unwrap_or(false)
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for (i, section) in self.section_order.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push('[');
            out.push_str(section);
            out.push_str("]\n");
            if let Some(table) = self.sections.get(section) {
                for (key, value) in table {
                    out.push_str(key);
                    out.push_str(" = ");
                    out.push_str(&render_value(value));
                    out.push('\n');
                }
            }
        }
        out
    }

    pub fn save(&self) -> LangResult<()> {
        fs::write(&self.path, self.render()).map_err(|err| {
            Diagnostic::at_start(format!(
                "failed to write '{}': {err}",
                self.path.display()
            ))
        })
    }
}

fn parse_value(text: &str) -> Option<Value> {
    if let Some(stripped) = text.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return Some(Value::String(stripped.to_string()));
    }
    if text == "true" {
        return Some(Value::Bool(true));
    }
    if text == "false" {
        return Some(Value::Bool(false));
    }
    if let Ok(int) = text.parse::<i64>() {
        return Some(Value::Integer(int));
    }
    None
}

fn render_value(value: &Value) -> String {
    match value {
        Value::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
        Value::Integer(i) => i.to_string(),
        Value::Bool(b) => b.to_string(),
    }
}
