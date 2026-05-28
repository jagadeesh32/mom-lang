//! Phase 5 linter — `mom lint`.
//!
//! AST visitor that emits diagnostics grouped by **category**:
//!     * `correctness`  — bugs (default: deny)
//!     * `suspicious`   — likely-wrong patterns (default: warn)
//!     * `performance`  — suboptimal patterns (default: allow)
//!     * `style`        — opinions (default: allow)
//!     * `unsafe-audit` — every `extern` / `unsafe`-style escape hatch
//!
//! Per-crate overrides live in `mom.toml`:
//!
//! ```toml
//! [lints]
//! default                = "warn"
//! correctness.shadowing  = "deny"
//! performance.allocation = "warn"
//! style.naming           = "allow"
//! ```
//!
//! The linter never panics on unknown rules and never depends on the
//! borrow checker / type checker — it operates purely on the parsed
//! `Program`, so it stays fast and can run on incomplete code.

use std::collections::HashMap;

use crate::ast::{
    Block, ConstDecl, EnumDecl, Expr, ExternBlock, ImplBlock, Item, ModuleDecl, Pattern, Program,
    Stmt, StructDecl, TraitDecl,
};
use crate::diagnostic::Span;
use crate::manifest::{Manifest, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Correctness,
    Suspicious,
    Performance,
    Style,
    UnsafeAudit,
}

impl Category {
    pub fn slug(self) -> &'static str {
        match self {
            Category::Correctness => "correctness",
            Category::Suspicious => "suspicious",
            Category::Performance => "performance",
            Category::Style => "style",
            Category::UnsafeAudit => "unsafe-audit",
        }
    }

    fn default_severity(self) -> Severity {
        match self {
            Category::Correctness => Severity::Deny,
            Category::Suspicious | Category::UnsafeAudit => Severity::Warn,
            Category::Performance | Category::Style => Severity::Allow,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Allow,
    Warn,
    Deny,
}

impl Severity {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "allow" => Some(Severity::Allow),
            "warn" => Some(Severity::Warn),
            "deny" => Some(Severity::Deny),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Severity::Allow => "allow",
            Severity::Warn => "warn",
            Severity::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub category: Category,
    pub rule: &'static str,
    pub message: String,
    pub span: Span,
    pub severity: Severity,
}

#[derive(Debug, Default, Clone)]
pub struct LintReport {
    pub findings: Vec<Finding>,
}

impl LintReport {
    pub fn deny_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Deny)
            .count()
    }
    pub fn warn_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warn)
            .count()
    }
}

#[derive(Debug, Clone)]
pub struct LintConfig {
    default_severity: Severity,
    overrides: HashMap<String, Severity>,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            default_severity: Severity::Warn,
            overrides: HashMap::new(),
        }
    }
}

impl LintConfig {
    /// Build a config from a `mom.toml` `[lints]` table.
    pub fn from_manifest(manifest: &Manifest) -> Self {
        let mut config = LintConfig::default();
        let Some(table) = manifest.section("lints") else {
            return config;
        };

        if let Some(Value::String(s)) = table.get("default") {
            if let Some(sev) = Severity::parse(s) {
                config.default_severity = sev;
            }
        }

        for (key, value) in table {
            if key == "default" {
                continue;
            }
            if let Value::String(s) = value {
                if let Some(sev) = Severity::parse(s) {
                    config.overrides.insert(key.clone(), sev);
                }
            }
        }
        config
    }

    fn severity_for(&self, category: Category, rule: &str) -> Severity {
        let full = format!("{}.{}", category.slug(), rule);
        if let Some(sev) = self.overrides.get(&full) {
            return *sev;
        }
        if let Some(sev) = self.overrides.get(category.slug()) {
            return *sev;
        }
        // Fall back to the category default rather than the global
        // default so `correctness` still denies by default if the user
        // only set `default = "warn"`.
        let category_default = category.default_severity();
        if category_default == Severity::Deny {
            return Severity::Deny;
        }
        self.default_severity
    }
}

pub fn lint_program(program: &Program, config: &LintConfig) -> LintReport {
    let mut ctx = LintCtx {
        config,
        findings: Vec::new(),
    };
    for item in &program.items {
        ctx.visit_item(item);
    }
    LintReport {
        findings: ctx.findings,
    }
}

struct LintCtx<'a> {
    config: &'a LintConfig,
    findings: Vec<Finding>,
}

impl<'a> LintCtx<'a> {
    fn report(&mut self, category: Category, rule: &'static str, message: String, span: Span) {
        let severity = self.config.severity_for(category, rule);
        if severity == Severity::Allow {
            return;
        }
        self.findings.push(Finding {
            category,
            rule,
            message,
            span,
            severity,
        });
    }

    fn visit_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => {
                check_naming(self, &f.name, f.body.span.clone(), "fn", false);
                self.visit_block(&f.body);
            }
            Item::Struct(s) => self.visit_struct(s),
            Item::Enum(e) => self.visit_enum(e),
            Item::Const(c) => self.visit_const(c),
            Item::Module(m) => self.visit_module(m),
            Item::Import(_) => {}
            Item::Trait(t) => self.visit_trait(t),
            Item::Impl(b) => self.visit_impl(b),
            Item::Extern(e) => self.visit_extern(e),
            Item::Statement(s) => self.visit_stmt(s),
        }
    }

    fn visit_struct(&mut self, decl: &StructDecl) {
        check_naming(self, &decl.name, decl.span.clone(), "type", true);
    }

    fn visit_enum(&mut self, decl: &EnumDecl) {
        check_naming(self, &decl.name, decl.span.clone(), "type", true);
    }

    fn visit_trait(&mut self, decl: &TraitDecl) {
        check_naming(self, &decl.name, decl.span.clone(), "type", true);
    }

    fn visit_impl(&mut self, decl: &ImplBlock) {
        for method in &decl.methods {
            check_naming(self, &method.name, method.body.span.clone(), "fn", false);
            self.visit_block(&method.body);
        }
    }

    fn visit_const(&mut self, decl: &ConstDecl) {
        if !decl
            .name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
        {
            self.report(
                Category::Style,
                "naming",
                format!("const `{}` should be SCREAMING_SNAKE_CASE", decl.name),
                decl.span.clone(),
            );
        }
        self.visit_expr(&decl.value);
    }

    fn visit_module(&mut self, decl: &ModuleDecl) {
        for item in &decl.items {
            self.visit_item(item);
        }
    }

    fn visit_extern(&mut self, decl: &ExternBlock) {
        self.report(
            Category::UnsafeAudit,
            "extern-block",
            format!(
                "`extern \"{}\"` crosses the safety boundary — document why",
                decl.language
            ),
            decl.span.clone(),
        );
    }

    fn visit_block(&mut self, block: &Block) {
        let mut seen: HashMap<String, Span> = HashMap::new();
        for stmt in &block.statements {
            if let Stmt::Let { name, span, .. } = stmt {
                if let Some(prev) = seen.get(name) {
                    self.report(
                        Category::Correctness,
                        "shadowing",
                        format!(
                            "binding `{}` shadows an earlier `let` at {}:{}",
                            name, prev.line, prev.column
                        ),
                        span.clone(),
                    );
                }
                seen.insert(name.clone(), span.clone());
            }
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                name, value, span, ..
            } => {
                if !name.starts_with('_')
                    && !name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
                {
                    self.report(
                        Category::Style,
                        "naming",
                        format!("binding `{}` should be snake_case", name),
                        span.clone(),
                    );
                }
                self.visit_expr(value);
            }
            Stmt::Const(c) => self.visit_const(c),
            Stmt::Assign { value, .. } => self.visit_expr(value),
            Stmt::Expr { expr, .. } => self.visit_expr(expr),
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.visit_expr(v);
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                if let Expr::Bool(true, span) = condition {
                    self.report(
                        Category::Suspicious,
                        "infinite-loop",
                        "`while true` should be replaced with `loop` when added; \
                         otherwise document why it is unbounded"
                            .to_string(),
                        span.clone(),
                    );
                }
                self.visit_expr(condition);
                self.visit_block(body);
            }
            Stmt::For { iter, body, .. } => {
                self.visit_expr(iter);
                self.visit_block(body);
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Int(_, _)
            | Expr::Float(_, _)
            | Expr::Bool(_, _)
            | Expr::String(_, _)
            | Expr::Unit(_)
            | Expr::Ident(_, _)
            | Expr::Path(_, _) => {}
            Expr::List(items, _) => {
                for it in items {
                    self.visit_expr(it);
                }
            }
            Expr::Range { start, end, .. } => {
                self.visit_expr(start);
                self.visit_expr(end);
            }
            Expr::Unary { expr, .. } => self.visit_expr(expr),
            Expr::Binary { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::Pipeline { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::Call { callee, args, .. } => {
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::MethodCall { target, args, .. } => {
                self.visit_expr(target);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::Field { target, .. } => self.visit_expr(target),
            Expr::Index { target, index, .. } => {
                self.visit_expr(target);
                self.visit_expr(index);
            }
            Expr::StructLit { fields, .. } => {
                for (_, value) in fields {
                    self.visit_expr(value);
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.visit_expr(condition);
                self.visit_block(then_branch);
                if let Some(e) = else_branch {
                    self.visit_block(e);
                }
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                self.visit_expr(scrutinee);
                self.check_match_wildcard_position(arms);
                for arm in arms {
                    self.visit_expr(&arm.body);
                }
            }
            Expr::Lambda { body, .. } => match body {
                crate::ast::LambdaBody::Expr(e) => self.visit_expr(e),
                crate::ast::LambdaBody::Block(b) => self.visit_block(b),
            },
            Expr::Try { expr, .. }
            | Expr::Spawn { expr, .. }
            | Expr::Await { expr, .. }
            | Expr::Ref { expr, .. } => self.visit_expr(expr),
            Expr::Region { body, .. } => self.visit_block(body),
            Expr::Block(b) => self.visit_block(b),
            Expr::Dict(pairs, _) => {
                for (k, v) in pairs {
                    self.visit_expr(k);
                    self.visit_expr(v);
                }
            }
        }
    }

    fn check_match_wildcard_position(&mut self, arms: &[crate::ast::MatchArm]) {
        let mut wildcard_seen: Option<Span> = None;
        for arm in arms {
            if let Some(prev) = &wildcard_seen {
                self.report(
                    Category::Suspicious,
                    "unreachable-arm",
                    format!(
                        "arm is unreachable: wildcard `_` at {}:{} matches everything",
                        prev.line, prev.column
                    ),
                    arm.span.clone(),
                );
            }
            if matches!(arm.pattern, Pattern::Wildcard(_)) {
                wildcard_seen = Some(arm.pattern.span());
            }
        }
    }
}

fn check_naming(ctx: &mut LintCtx<'_>, name: &str, span: Span, kind: &'static str, pascal: bool) {
    if name.is_empty() {
        return;
    }
    if pascal {
        let first = name.chars().next().unwrap();
        if !first.is_ascii_uppercase() {
            ctx.report(
                Category::Style,
                "naming",
                format!("{} `{}` should be PascalCase", kind, name),
                span,
            );
        }
    } else {
        if name.starts_with('_') {
            return;
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            ctx.report(
                Category::Style,
                "naming",
                format!("{} `{}` should be snake_case", kind, name),
                span,
            );
        }
    }
}
