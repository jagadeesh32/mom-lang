//! Phase 5 doc generator — `mom doc`.
//!
//! Walks the parsed program and the original source to collect every
//! `pub` item together with its leading `//` comment block. Emits a
//! single Markdown document that lists modules, types, traits, and
//! functions with their doc strings and signatures.
//!
//! This is intentionally the simplest possible API generator — the
//! native toolchain will replace it with a search-indexed site.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::ast::{FunctionDecl, Item, ModuleDecl, Program, TypeRef};
use crate::diagnostic::{Diagnostic, LangResult};

pub fn generate_docs(source: &str, program: &Program, crate_name: &str) -> LangResult<String> {
    let comments = collect_doc_comments(source)?;
    let mut out = String::new();
    writeln!(out, "# `{}` — API reference", crate_name).ok();
    writeln!(out).ok();

    let mut sections: Vec<(String, String)> = Vec::new();
    for item in &program.items {
        render_item(item, &comments, &[], &mut sections);
    }

    if sections.is_empty() {
        writeln!(out, "_(no public items)_").ok();
        return Ok(out);
    }

    for (heading, body) in sections {
        writeln!(out, "## {}", heading).ok();
        writeln!(out).ok();
        out.push_str(&body);
        writeln!(out).ok();
    }

    Ok(out)
}

/// Map from "line where the item begins" → the doc comment block that
/// immediately precedes it.
fn collect_doc_comments(source: &str) -> LangResult<HashMap<usize, String>> {
    let mut comments: HashMap<usize, String> = HashMap::new();
    let lines: Vec<&str> = source.lines().collect();

    let mut buf: Vec<String> = Vec::new();
    let mut buf_start: Option<usize> = None;

    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim_start();
        if line.starts_with("//") {
            let body = line.trim_start_matches('/').trim();
            buf.push(body.to_string());
            if buf_start.is_none() {
                buf_start = Some(i + 1);
            }
            continue;
        }
        if line.is_empty() {
            // Blank line ends a doc block.
            buf.clear();
            buf_start = None;
            continue;
        }
        if !buf.is_empty() {
            let text = buf.join("\n");
            comments.insert(i + 1, text);
            buf.clear();
            buf_start = None;
        }
    }

    let _ = buf_start;
    if !comments.is_empty() {
        // Comments map is intentionally returned even if empty.
    }
    Ok::<_, Diagnostic>(comments)
}

fn render_item(
    item: &Item,
    comments: &HashMap<usize, String>,
    path: &[String],
    sections: &mut Vec<(String, String)>,
) {
    match item {
        Item::Function(f) => {
            if !f.is_pub {
                return;
            }
            let heading = qualified(path, &f.name);
            let doc = comments
                .get(&f.span.line)
                .cloned()
                .unwrap_or_else(|| "_(no description)_".to_string());
            let mut body = String::new();
            body.push_str("```mom\n");
            body.push_str(&render_fn_signature(f));
            body.push_str("\n```\n\n");
            body.push_str(&doc);
            body.push('\n');
            sections.push((format!("fn `{}`", heading), body));
        }
        Item::Struct(s) => {
            if !s.is_pub {
                return;
            }
            let heading = qualified(path, &s.name);
            let doc = comments
                .get(&s.span.line)
                .cloned()
                .unwrap_or_else(|| "_(no description)_".to_string());
            let mut body = String::new();
            body.push_str("```mom\nstruct ");
            body.push_str(&s.name);
            if !s.generics.is_empty() {
                body.push('[');
                body.push_str(&s.generics.join(", "));
                body.push(']');
            }
            body.push_str(" { … }\n```\n\n");
            body.push_str(&doc);
            body.push('\n');
            sections.push((format!("struct `{}`", heading), body));
        }
        Item::Enum(e) => {
            if !e.is_pub {
                return;
            }
            let heading = qualified(path, &e.name);
            let doc = comments
                .get(&e.span.line)
                .cloned()
                .unwrap_or_else(|| "_(no description)_".to_string());
            let variants: Vec<String> = e.variants.iter().map(|v| v.name.clone()).collect();
            let mut body = String::new();
            body.push_str("```mom\nenum ");
            body.push_str(&e.name);
            body.push_str(" { ");
            body.push_str(&variants.join(" | "));
            body.push_str(" }\n```\n\n");
            body.push_str(&doc);
            body.push('\n');
            sections.push((format!("enum `{}`", heading), body));
        }
        Item::Trait(t) => {
            if !t.is_pub {
                return;
            }
            let heading = qualified(path, &t.name);
            let doc = comments
                .get(&t.span.line)
                .cloned()
                .unwrap_or_else(|| "_(no description)_".to_string());
            let mut body = String::new();
            body.push_str("```mom\ntrait ");
            body.push_str(&t.name);
            body.push_str(" { … }\n```\n\n");
            body.push_str(&doc);
            body.push('\n');
            sections.push((format!("trait `{}`", heading), body));
        }
        Item::Const(c) => {
            if !c.is_pub {
                return;
            }
            let heading = qualified(path, &c.name);
            let doc = comments
                .get(&c.span.line)
                .cloned()
                .unwrap_or_else(|| "_(no description)_".to_string());
            let mut body = String::new();
            body.push_str("```mom\nconst ");
            body.push_str(&c.name);
            body.push_str("\n```\n\n");
            body.push_str(&doc);
            body.push('\n');
            sections.push((format!("const `{}`", heading), body));
        }
        Item::Module(m) => render_module(m, comments, path, sections),
        Item::Import(_) | Item::Statement(_) | Item::Impl(_) | Item::Extern(_) => {}
    }
}

fn render_module(
    decl: &ModuleDecl,
    comments: &HashMap<usize, String>,
    path: &[String],
    sections: &mut Vec<(String, String)>,
) {
    if !decl.is_pub {
        return;
    }
    let mut child_path: Vec<String> = path.to_vec();
    child_path.push(decl.name.clone());
    for inner in &decl.items {
        render_item(inner, comments, &child_path, sections);
    }
}

fn qualified(path: &[String], name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{}::{}", path.join("::"), name)
    }
}

fn render_fn_signature(f: &FunctionDecl) -> String {
    let mut out = String::new();
    if f.is_pub {
        out.push_str("pub ");
    }
    if f.is_async {
        out.push_str("async ");
    }
    out.push_str("fn ");
    out.push_str(&f.name);
    if !f.generics.is_empty() {
        out.push('[');
        out.push_str(&f.generics.join(", "));
        out.push(']');
    }
    out.push('(');
    let params: Vec<String> = f
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, render_type(&p.ty)))
        .collect();
    out.push_str(&params.join(", "));
    out.push(')');
    if !matches!(f.return_type, TypeRef::Unit) {
        out.push_str(" -> ");
        out.push_str(&render_type(&f.return_type));
    }
    out
}

fn render_type(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Named(n) => n.clone(),
        TypeRef::Generic(n, args) => {
            let inner: Vec<String> = args.iter().map(render_type).collect();
            format!("{}[{}]", n, inner.join(", "))
        }
        TypeRef::Function(params, ret) => {
            let p: Vec<String> = params.iter().map(render_type).collect();
            format!("fn({}) -> {}", p.join(", "), render_type(ret))
        }
        TypeRef::List(inner) => format!("[{}]", render_type(inner)),
        TypeRef::Ref(inner, true) => format!("&mut {}", render_type(inner)),
        TypeRef::Ref(inner, false) => format!("&{}", render_type(inner)),
        TypeRef::Unit => "()".to_string(),
        TypeRef::Infer => "_".to_string(),
    }
}
