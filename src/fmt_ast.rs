//! Phase 5.1 native parity — AST-based `mom fmt` pretty-printer.
//!
//! Walks the parsed `Program` and emits a canonical layout. The
//! grandfathered text re-indenter in `crate::fmt` calls into this
//! module when the source parses; if the AST round-trips back to the
//! same shape, the AST-formatted output wins. Otherwise the
//! re-indenter's output is kept so a parse-broken source is never
//! mangled.
//!
//! Style rules (kept deliberately small so they're easy to argue about):
//!
//!   * 4-space indentation, never tabs.
//!   * `,` and `:` always followed by a single space.
//!   * Binary operators surrounded by a single space.
//!   * One blank line between top-level items; no leading or trailing
//!     blanks in a file.
//!   * Empty bodies render as `{}` on the same line as the header.

use crate::ast::*;

pub fn format_program(program: &Program) -> String {
    let mut p = Printer::new();
    p.emit_items(&program.items, 0);
    let mut out = p.into_string();
    // Trim trailing blank runs and ensure single trailing newline.
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

struct Printer {
    out: String,
}

impl Printer {
    fn new() -> Self {
        Self { out: String::new() }
    }

    fn into_string(self) -> String {
        self.out
    }

    fn indent(&mut self, depth: usize) {
        for _ in 0..depth {
            self.out.push_str("    ");
        }
    }

    fn emit_items(&mut self, items: &[Item], depth: usize) {
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                self.out.push('\n');
            }
            self.emit_item(item, depth);
        }
    }

    fn emit_item(&mut self, item: &Item, depth: usize) {
        match item {
            Item::Function(f) => self.emit_function(f, depth),
            Item::Struct(s) => self.emit_struct(s, depth),
            Item::Enum(e) => self.emit_enum(e, depth),
            Item::Const(c) => {
                self.indent(depth);
                if c.is_pub {
                    self.out.push_str("pub ");
                }
                self.out.push_str("const ");
                self.out.push_str(&c.name);
                if let Some(ty) = &c.ty {
                    self.out.push_str(": ");
                    self.out.push_str(&type_ref(ty));
                }
                self.out.push_str(" = ");
                self.out.push_str(&expr_inline(&c.value));
                self.out.push('\n');
            }
            Item::Module(m) => {
                self.indent(depth);
                if m.is_pub {
                    self.out.push_str("pub ");
                }
                self.out.push_str("module ");
                self.out.push_str(&m.name);
                if m.items.is_empty() {
                    self.out.push_str(" {}\n");
                } else {
                    self.out.push_str(" {\n");
                    self.emit_items(&m.items, depth + 1);
                    self.indent(depth);
                    self.out.push_str("}\n");
                }
            }
            Item::Import(i) => {
                self.indent(depth);
                self.out.push_str("import ");
                self.out.push_str(&i.path.join("."));
                if !i.items.is_empty() {
                    self.out.push_str(".{");
                    self.out.push_str(&i.items.join(", "));
                    self.out.push('}');
                }
                self.out.push('\n');
            }
            Item::Trait(t) => {
                self.indent(depth);
                if t.is_pub {
                    self.out.push_str("pub ");
                }
                self.out.push_str("trait ");
                self.out.push_str(&t.name);
                self.emit_generics(&t.generics);
                if t.methods.is_empty() {
                    self.out.push_str(" {}\n");
                } else {
                    self.out.push_str(" {\n");
                    for m in &t.methods {
                        self.indent(depth + 1);
                        self.out.push_str("fn ");
                        self.out.push_str(&m.name);
                        self.out.push('(');
                        self.out.push_str(&render_params(&m.params));
                        self.out.push(')');
                        if !matches!(m.return_type, TypeRef::Unit) {
                            self.out.push_str(" -> ");
                            self.out.push_str(&type_ref(&m.return_type));
                        }
                        self.out.push('\n');
                    }
                    self.indent(depth);
                    self.out.push_str("}\n");
                }
            }
            Item::Impl(b) => {
                self.indent(depth);
                self.out.push_str("impl ");
                self.emit_generics(&b.generics);
                if let Some(trait_name) = &b.trait_name {
                    self.out.push_str(trait_name);
                    self.out.push_str(" for ");
                }
                self.out.push_str(&b.target);
                self.out.push_str(" {\n");
                for (i, m) in b.methods.iter().enumerate() {
                    if i > 0 {
                        self.out.push('\n');
                    }
                    self.emit_function(m, depth + 1);
                }
                self.indent(depth);
                self.out.push_str("}\n");
            }
            Item::Extern(e) => {
                self.indent(depth);
                self.out.push_str("extern ");
                self.out.push_str(&e.language);
                if let Some(lib) = &e.library {
                    self.out.push_str(" \"");
                    self.out.push_str(lib);
                    self.out.push('"');
                }
                self.out.push_str(" {\n");
                for item in &e.items {
                    self.indent(depth + 1);
                    self.out.push_str("fn ");
                    self.out.push_str(&item.name);
                    self.out.push('(');
                    self.out.push_str(&render_params(&item.params));
                    self.out.push(')');
                    if !matches!(item.return_type, TypeRef::Unit) {
                        self.out.push_str(" -> ");
                        self.out.push_str(&type_ref(&item.return_type));
                    }
                    self.out.push('\n');
                }
                self.indent(depth);
                self.out.push_str("}\n");
            }
            Item::Statement(stmt) => self.emit_stmt(stmt, depth),
        }
    }

    fn emit_function(&mut self, f: &FunctionDecl, depth: usize) {
        for attr in &f.attrs {
            self.indent(depth);
            self.out.push_str("#[");
            self.out.push_str(attr);
            self.out.push_str("]\n");
        }
        self.indent(depth);
        if f.is_pub {
            self.out.push_str("pub ");
        }
        if f.is_async {
            self.out.push_str("async ");
        }
        self.out.push_str("fn ");
        self.out.push_str(&f.name);
        self.emit_generics(&f.generics);
        self.out.push('(');
        self.out.push_str(&render_params(&f.params));
        self.out.push(')');
        if !matches!(f.return_type, TypeRef::Unit) {
            self.out.push_str(" -> ");
            self.out.push_str(&type_ref(&f.return_type));
        }
        self.emit_block_after_header(&f.body, depth);
    }

    fn emit_struct(&mut self, s: &StructDecl, depth: usize) {
        self.indent(depth);
        if s.is_pub {
            self.out.push_str("pub ");
        }
        self.out.push_str("struct ");
        self.out.push_str(&s.name);
        self.emit_generics(&s.generics);
        if s.fields.is_empty() {
            self.out.push_str(" {}\n");
            return;
        }
        // Short form: all fields on one line if it fits a single 80-col
        // line; otherwise newline-separated.
        let short = format!(
            " {{ {} }}",
            s.fields
                .iter()
                .map(|f| format!("{}: {}", f.name, type_ref(&f.ty)))
                .collect::<Vec<_>>()
                .join(", ")
        );
        if short.len() + s.name.len() + 8 <= 80 {
            self.out.push_str(&short);
            self.out.push('\n');
        } else {
            self.out.push_str(" {\n");
            for field in &s.fields {
                self.indent(depth + 1);
                self.out.push_str(&field.name);
                self.out.push_str(": ");
                self.out.push_str(&type_ref(&field.ty));
                self.out.push_str(",\n");
            }
            self.indent(depth);
            self.out.push_str("}\n");
        }
    }

    fn emit_enum(&mut self, e: &EnumDecl, depth: usize) {
        self.indent(depth);
        if e.is_pub {
            self.out.push_str("pub ");
        }
        self.out.push_str("enum ");
        self.out.push_str(&e.name);
        self.emit_generics(&e.generics);
        if e.variants.is_empty() {
            self.out.push_str(" {}\n");
            return;
        }
        let rendered: Vec<String> = e.variants.iter().map(render_variant).collect();
        let short = format!(" {{ {} }}", rendered.join(", "));
        if short.len() + e.name.len() + 6 <= 80 {
            self.out.push_str(&short);
            self.out.push('\n');
        } else {
            self.out.push_str(" {\n");
            for v in &rendered {
                self.indent(depth + 1);
                self.out.push_str(v);
                self.out.push_str(",\n");
            }
            self.indent(depth);
            self.out.push_str("}\n");
        }
    }

    fn emit_generics(&mut self, generics: &[String]) {
        if generics.is_empty() {
            return;
        }
        self.out.push('[');
        self.out.push_str(&generics.join(", "));
        self.out.push(']');
    }

    fn emit_block_after_header(&mut self, block: &Block, depth: usize) {
        if block.statements.is_empty() {
            self.out.push_str(" {}\n");
            return;
        }
        self.out.push_str(" {\n");
        for stmt in &block.statements {
            self.emit_stmt(stmt, depth + 1);
        }
        self.indent(depth);
        self.out.push_str("}\n");
    }

    fn emit_stmt(&mut self, stmt: &Stmt, depth: usize) {
        match stmt {
            Stmt::Let { name, ty, mutable, value, .. } => {
                self.indent(depth);
                self.out.push_str("let ");
                if *mutable {
                    self.out.push_str("mut ");
                }
                self.out.push_str(name);
                if let Some(ty) = ty {
                    self.out.push_str(": ");
                    self.out.push_str(&type_ref(ty));
                }
                self.out.push_str(" = ");
                self.out.push_str(&expr_inline(value));
                self.out.push('\n');
            }
            Stmt::Const(c) => {
                self.indent(depth);
                if c.is_pub {
                    self.out.push_str("pub ");
                }
                self.out.push_str("const ");
                self.out.push_str(&c.name);
                if let Some(ty) = &c.ty {
                    self.out.push_str(": ");
                    self.out.push_str(&type_ref(ty));
                }
                self.out.push_str(" = ");
                self.out.push_str(&expr_inline(&c.value));
                self.out.push('\n');
            }
            Stmt::Assign { target, value, .. } => {
                self.indent(depth);
                self.out.push_str(&assign_target(target));
                self.out.push_str(" = ");
                self.out.push_str(&expr_inline(value));
                self.out.push('\n');
            }
            Stmt::Expr { expr, .. } => {
                self.indent(depth);
                self.out.push_str(&expr_inline(expr));
                self.out.push('\n');
            }
            Stmt::Return { value, .. } => {
                self.indent(depth);
                self.out.push_str("return");
                if let Some(v) = value {
                    self.out.push(' ');
                    self.out.push_str(&expr_inline(v));
                }
                self.out.push('\n');
            }
            Stmt::While { condition, body, .. } => {
                self.indent(depth);
                self.out.push_str("while ");
                self.out.push_str(&expr_inline(condition));
                self.emit_block_after_header(body, depth);
            }
            Stmt::For { name, iter, body, .. } => {
                self.indent(depth);
                self.out.push_str("for ");
                self.out.push_str(name);
                self.out.push_str(" in ");
                self.out.push_str(&expr_inline(iter));
                self.emit_block_after_header(body, depth);
            }
            Stmt::Break { .. } => {
                self.indent(depth);
                self.out.push_str("break\n");
            }
            Stmt::Continue { .. } => {
                self.indent(depth);
                self.out.push_str("continue\n");
            }
        }
    }
}

fn render_params(params: &[Param]) -> String {
    params
        .iter()
        .map(|p| {
            if matches!(p.ty, TypeRef::Infer) {
                p.name.clone()
            } else {
                format!("{}: {}", p.name, type_ref(&p.ty))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_variant(v: &Variant) -> String {
    if v.payload.is_empty() {
        v.name.clone()
    } else {
        let parts: Vec<String> = v.payload.iter().map(type_ref).collect();
        format!("{}({})", v.name, parts.join(", "))
    }
}

fn assign_target(target: &AssignTarget) -> String {
    match target {
        AssignTarget::Name(name) => name.clone(),
        AssignTarget::Field { target, name } => {
            format!("{}.{}", expr_inline(target), name)
        }
        AssignTarget::Index { target, index } => {
            format!("{}[{}]", expr_inline(target), expr_inline(index))
        }
    }
}

fn type_ref(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Named(name) => name.clone(),
        TypeRef::Generic(name, args) => {
            let parts: Vec<String> = args.iter().map(type_ref).collect();
            format!("{}[{}]", name, parts.join(", "))
        }
        TypeRef::Function(params, ret) => {
            let parts: Vec<String> = params.iter().map(type_ref).collect();
            format!("fn({}) -> {}", parts.join(", "), type_ref(ret))
        }
        TypeRef::List(inner) => format!("[{}]", type_ref(inner)),
        TypeRef::Ref(inner, is_mut) => {
            if *is_mut {
                format!("&mut {}", type_ref(inner))
            } else {
                format!("&{}", type_ref(inner))
            }
        }
        TypeRef::Unit => "()".to_string(),
        TypeRef::Infer => "_".to_string(),
    }
}

fn expr_inline(expr: &Expr) -> String {
    match expr {
        Expr::Int(value, _) => value.to_string(),
        Expr::Float(value, _) => format!("{value:?}"),
        Expr::Bool(value, _) => if *value { "true".to_string() } else { "false".to_string() },
        Expr::String(value, _) => format!("\"{}\"", escape_string(value)),
        Expr::Unit(_) => "()".to_string(),
        Expr::Ident(name, _) => name.clone(),
        Expr::Path(segments, _) => segments.join("."),
        Expr::List(items, _) => {
            let parts: Vec<String> = items.iter().map(expr_inline).collect();
            format!("[{}]", parts.join(", "))
        }
        Expr::Range { start, end, .. } => {
            format!("{}..{}", expr_inline(start), expr_inline(end))
        }
        Expr::Unary { op, expr, .. } => {
            let sym = match op {
                UnaryOp::Negate => "-",
                UnaryOp::Not => "!",
            };
            format!("{sym}{}", paren_unary(expr))
        }
        Expr::Binary { left, op, right, .. } => {
            format!(
                "{} {} {}",
                paren_binary(left, *op, true),
                bin_op(*op),
                paren_binary(right, *op, false),
            )
        }
        Expr::Pipeline { left, right, .. } => {
            format!("{} |> {}", expr_inline(left), expr_inline(right))
        }
        Expr::Call { callee, args, .. } => {
            let parts: Vec<String> = args.iter().map(expr_inline).collect();
            format!("{}({})", expr_inline(callee), parts.join(", "))
        }
        Expr::MethodCall { target, name, args, .. } => {
            let parts: Vec<String> = args.iter().map(expr_inline).collect();
            format!("{}.{}({})", expr_inline(target), name, parts.join(", "))
        }
        Expr::Field { target, name, .. } => {
            format!("{}.{}", expr_inline(target), name)
        }
        Expr::Index { target, index, .. } => {
            format!("{}[{}]", expr_inline(target), expr_inline(index))
        }
        Expr::StructLit { name, fields, .. } => {
            let parts: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, expr_inline(v)))
                .collect();
            if parts.is_empty() {
                format!("{name} {{}}")
            } else {
                format!("{name} {{ {} }}", parts.join(", "))
            }
        }
        Expr::If { condition, then_branch, else_branch, .. } => {
            let mut out = format!("if {} {}", expr_inline(condition), block_inline(then_branch));
            if let Some(else_b) = else_branch {
                out.push_str(" else ");
                out.push_str(&block_inline(else_b));
            }
            out
        }
        Expr::Match { scrutinee, arms, .. } => {
            let mut out = format!("match {} {{ ", expr_inline(scrutinee));
            let parts: Vec<String> = arms
                .iter()
                .map(|a| format!("{} => {}", pattern_inline(&a.pattern), expr_inline(&a.body)))
                .collect();
            out.push_str(&parts.join(", "));
            out.push_str(" }");
            out
        }
        Expr::Lambda { params, return_type, body, .. } => {
            let header = if params.is_empty() {
                "fn()".to_string()
            } else {
                format!("fn({})", render_params(params))
            };
            let ret = match return_type {
                Some(t) => format!(" -> {}", type_ref(t)),
                None => String::new(),
            };
            let body_str = match body {
                LambdaBody::Expr(e) => format!(" => {}", expr_inline(e)),
                LambdaBody::Block(b) => format!(" {}", block_inline(b)),
            };
            format!("{header}{ret}{body_str}")
        }
        Expr::Try { expr, .. } => format!("{}?", expr_inline(expr)),
        Expr::Spawn { expr, .. } => format!("spawn {}", expr_inline(expr)),
        Expr::Await { expr, .. } => format!("await {}", expr_inline(expr)),
        Expr::Ref { expr, is_mut, .. } => {
            if *is_mut {
                format!("&mut {}", expr_inline(expr))
            } else {
                format!("&{}", expr_inline(expr))
            }
        }
        Expr::Region { name, body, .. } => {
            format!("region {name} {}", block_inline(body))
        }
        Expr::Block(block) => block_inline(block),
    }
}

fn escape_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn pattern_inline(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard(_) => "_".to_string(),
        Pattern::Ident(name, _) => name.clone(),
        Pattern::Int(v, _) => v.to_string(),
        Pattern::Float(v, _) => format!("{v:?}"),
        Pattern::Bool(v, _) => if *v { "true".to_string() } else { "false".to_string() },
        Pattern::String(v, _) => format!("\"{}\"", escape_string(v)),
        Pattern::Unit(_) => "()".to_string(),
        Pattern::Variant { name, payload, .. } => {
            if payload.is_empty() {
                name.clone()
            } else {
                let parts: Vec<String> = payload.iter().map(pattern_inline).collect();
                format!("{name}({})", parts.join(", "))
            }
        }
    }
}

fn block_inline(block: &Block) -> String {
    if block.statements.is_empty() {
        return "{}".to_string();
    }
    // Tail-expression-only block stays one line.
    if block.statements.len() == 1 {
        if let Stmt::Expr { expr, has_semicolon: false, .. } = &block.statements[0] {
            return format!("{{ {} }}", expr_inline(expr));
        }
    }
    let mut out = String::from("{\n");
    for stmt in &block.statements {
        // 1-level indent inside an inline block — caller is responsible
        // for any outer indent. We use 4 spaces consistently.
        match stmt {
            Stmt::Let { name, ty, mutable, value, .. } => {
                out.push_str("    let ");
                if *mutable {
                    out.push_str("mut ");
                }
                out.push_str(name);
                if let Some(ty) = ty {
                    out.push_str(": ");
                    out.push_str(&type_ref(ty));
                }
                out.push_str(" = ");
                out.push_str(&expr_inline(value));
                out.push('\n');
            }
            Stmt::Expr { expr, .. } => {
                out.push_str("    ");
                out.push_str(&expr_inline(expr));
                out.push('\n');
            }
            Stmt::Return { value, .. } => {
                out.push_str("    return");
                if let Some(v) = value {
                    out.push(' ');
                    out.push_str(&expr_inline(v));
                }
                out.push('\n');
            }
            other => {
                // For complex statements inside an inline block, fall
                // back to a one-line approximation. The outer caller's
                // re-indenter is the safety net.
                out.push_str("    ");
                out.push_str(&format!("{other:?}"));
                out.push('\n');
            }
        }
    }
    out.push('}');
    out
}

fn bin_op(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Remainder => "%",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

fn precedence(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::Equal | BinaryOp::NotEqual => 3,
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => 4,
        BinaryOp::Add | BinaryOp::Subtract => 5,
        BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => 6,
    }
}

fn paren_unary(expr: &Expr) -> String {
    match expr {
        Expr::Binary { .. } | Expr::Pipeline { .. } => format!("({})", expr_inline(expr)),
        _ => expr_inline(expr),
    }
}

fn paren_binary(expr: &Expr, parent: BinaryOp, is_left: bool) -> String {
    match expr {
        Expr::Binary { op, .. } => {
            let pp = precedence(parent);
            let cp = precedence(*op);
            if cp < pp || (cp == pp && !is_left) {
                format!("({})", expr_inline(expr))
            } else {
                expr_inline(expr)
            }
        }
        _ => expr_inline(expr),
    }
}
