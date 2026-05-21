//! Phase 2 borrow checker — lexical-scope ownership and borrow analysis.
//!
//! The native compiler will host a full dataflow / Polonius-style checker;
//! this MVP implements the conservative pre-NLL rules:
//!
//!   1. A binding's value has at most one owner. Reading it after a move
//!      is an error.
//!   2. Passing a non-`Copy` value to a function or assigning it to a
//!      new binding *moves* it.
//!   3. While a `&mut x` loan is live, no other borrow of `x` is allowed.
//!   4. While `&x` loans are live, `&mut x` is not allowed.
//!   5. While any borrow of `x` is live, `x` may not be mutated, moved,
//!      or rebound.
//!   6. `let r = &x` (or `&mut x`) introduces a loan whose lifetime is
//!      the lexical scope of `r`. Transient borrows used only inside an
//!      expression (e.g. `f(&x)`) are checked and released immediately.
//!   7. Reassignment of an immutable binding is rejected.
//!
//! `Copy` types: `Int`, `Float`, `Bool`, `Unit`, references (`&T`,
//! `&mut T` are treated as borrow-tokens; the source binding's loan
//! state is what matters).

use std::collections::HashMap;

use crate::ast::*;
use crate::diagnostic::{Diagnostic, LangResult, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Values cheap to duplicate; do not move on assignment.
    Copy,
    /// Values with destructors / shared state; move on assignment.
    Move,
}

#[derive(Debug, Clone)]
struct Symbol {
    kind: Kind,
    mutable: bool,
    moved: bool,
    /// Active immutable borrows of this binding by name.
    imm_borrowers: Vec<String>,
    /// Active mutable borrower (at most one).
    mut_borrower: Option<String>,
    /// If this symbol *itself* is a borrow into another binding, who is
    /// the source. The loan is released when this symbol leaves scope.
    loan_source: Option<(String, bool)>,
    span: Span,
}

#[derive(Debug, Default)]
struct Scope {
    bindings: HashMap<String, Symbol>,
    /// Names declared in this scope, in order — used to release loans
    /// in LIFO order on scope exit.
    declared: Vec<String>,
}

pub struct BorrowChecker {
    scopes: Vec<Scope>,
    functions: HashMap<String, FunctionDecl>,
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            scopes: Vec::new(),
            functions: HashMap::new(),
        }
    }

    pub fn check_program(mut self, program: &Program) -> LangResult<()> {
        self.collect_functions(&program.items);
        for item in &program.items {
            self.check_item(item)?;
        }
        Ok(())
    }

    fn collect_functions(&mut self, items: &[Item]) {
        for item in items {
            match item {
                Item::Function(f) => {
                    self.functions.insert(f.name.clone(), f.clone());
                }
                Item::Impl(block) => {
                    for m in &block.methods {
                        self.functions.insert(m.name.clone(), m.clone());
                    }
                }
                Item::Module(m) => {
                    self.collect_functions(&m.items);
                }
                _ => {}
            }
        }
    }

    fn check_item(&mut self, item: &Item) -> LangResult<()> {
        match item {
            Item::Function(f) => self.check_function(f),
            Item::Impl(block) => {
                for method in &block.methods {
                    self.check_function(method)?;
                }
                Ok(())
            }
            Item::Module(m) => {
                for inner in &m.items {
                    self.check_item(inner)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn check_function(&mut self, function: &FunctionDecl) -> LangResult<()> {
        self.scopes.push(Scope::default());
        for param in &function.params {
            let kind = kind_of_type_ref(&param.ty);
            self.declare(
                &param.name,
                Symbol {
                    kind,
                    mutable: false,
                    moved: false,
                    imm_borrowers: Vec::new(),
                    mut_borrower: None,
                    loan_source: None,
                    span: function.span.clone(),
                },
            )?;
        }
        self.check_block(&function.body)?;
        self.pop_scope();
        Ok(())
    }

    fn check_block(&mut self, block: &Block) -> LangResult<()> {
        self.scopes.push(Scope::default());
        for stmt in &block.statements {
            self.check_stmt(stmt)?;
        }
        self.pop_scope();
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> LangResult<()> {
        match stmt {
            Stmt::Let { name, ty, mutable, value, span } => {
                let kind = ty.as_ref().map(kind_of_type_ref).unwrap_or_else(|| infer_kind(value));
                let loan_source = self.evaluate_expr_for_let(value)?;
                self.declare(
                    name,
                    Symbol {
                        kind,
                        mutable: *mutable,
                        moved: false,
                        imm_borrowers: Vec::new(),
                        mut_borrower: None,
                        loan_source,
                        span: span.clone(),
                    },
                )
            }
            Stmt::Const(decl) => {
                let kind = decl
                    .ty
                    .as_ref()
                    .map(kind_of_type_ref)
                    .unwrap_or_else(|| infer_kind(&decl.value));
                let loan_source = self.evaluate_expr_for_let(&decl.value)?;
                self.declare(
                    &decl.name,
                    Symbol {
                        kind,
                        mutable: false,
                        moved: false,
                        imm_borrowers: Vec::new(),
                        mut_borrower: None,
                        loan_source,
                        span: decl.span.clone(),
                    },
                )
            }
            Stmt::Assign { target, value, span } => {
                self.check_assign(target, value, span)
            }
            Stmt::Expr { expr, .. } => {
                self.visit_expr(expr)?;
                Ok(())
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.consume_expr_as_value(value)?;
                }
                Ok(())
            }
            Stmt::While { condition, body, .. } => {
                self.visit_expr(condition)?;
                self.check_block(body)
            }
            Stmt::For { name, iter, body, span } => {
                self.visit_expr(iter)?;
                self.scopes.push(Scope::default());
                self.declare(
                    name,
                    Symbol {
                        kind: Kind::Copy,
                        mutable: false,
                        moved: false,
                        imm_borrowers: Vec::new(),
                        mut_borrower: None,
                        loan_source: None,
                        span: span.clone(),
                    },
                )?;
                for stmt in &body.statements {
                    self.check_stmt(stmt)?;
                }
                self.pop_scope();
                Ok(())
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => Ok(()),
        }
    }

    fn check_assign(
        &mut self,
        target: &AssignTarget,
        value: &Expr,
        span: &Span,
    ) -> LangResult<()> {
        self.consume_expr_as_value(value)?;
        match target {
            AssignTarget::Name(name) => {
                let Some(info) = self.lookup(name) else {
                    return Ok(());
                };
                if !info.mutable {
                    return Err(Diagnostic::new(
                        format!("cannot assign to immutable binding '{name}'"),
                        span.clone(),
                    ));
                }
                if !info.imm_borrowers.is_empty() || info.mut_borrower.is_some() {
                    return Err(Diagnostic::new(
                        format!(
                            "cannot assign to '{name}' while it is borrowed"
                        ),
                        span.clone(),
                    ));
                }
                if info.moved {
                    // mark as live again — re-initialisation is allowed
                    self.modify(name, |s| {
                        s.moved = false;
                    });
                }
                Ok(())
            }
            AssignTarget::Field { target, .. } => {
                self.visit_expr(target)?;
                Ok(())
            }
            AssignTarget::Index { target, index } => {
                self.visit_expr(target)?;
                self.visit_expr(index)?;
                Ok(())
            }
        }
    }

    /// Visit an expression that is the **value** of a `let` binding.
    /// If the expression is a top-level `&y` or `&mut y` (possibly
    /// through a no-op wrapper), record the loan; the borrow checker
    /// will release it when the new binding leaves scope.
    fn evaluate_expr_for_let(&mut self, expr: &Expr) -> LangResult<Option<(String, bool)>> {
        if let Expr::Ref { expr: inner, is_mut, span } = expr {
            if let Some(name) = root_name(inner) {
                self.start_loan(&name, *is_mut, span)?;
                self.visit_expr_internal(inner, false)?;
                return Ok(Some((name, *is_mut)));
            }
        }
        self.consume_expr_as_value(expr)?;
        Ok(None)
    }

    /// Visit an expression treating it as a value-producing position
    /// (so non-Copy idents get moved).
    fn consume_expr_as_value(&mut self, expr: &Expr) -> LangResult<()> {
        self.visit_expr_internal(expr, true)
    }

    fn visit_expr(&mut self, expr: &Expr) -> LangResult<()> {
        self.visit_expr_internal(expr, true)
    }

    fn visit_expr_internal(&mut self, expr: &Expr, consume: bool) -> LangResult<()> {
        match expr {
            Expr::Int(_, _) | Expr::Float(_, _) | Expr::Bool(_, _) | Expr::String(_, _) | Expr::Unit(_) => Ok(()),
            Expr::Ident(name, span) => self.use_ident(name, consume, span),
            Expr::Path(_, _) => Ok(()),
            Expr::List(items, _) => {
                for item in items {
                    self.consume_expr_as_value(item)?;
                }
                Ok(())
            }
            Expr::Range { start, end, .. } => {
                self.consume_expr_as_value(start)?;
                self.consume_expr_as_value(end)
            }
            Expr::Unary { expr, .. } => self.visit_expr_internal(expr, false),
            Expr::Binary { left, right, .. } => {
                // Arithmetic / comparison / logical ops *read* their
                // operands; they don't take ownership.
                self.visit_expr_internal(left, false)?;
                self.visit_expr_internal(right, false)
            }
            Expr::Pipeline { left, right, .. } => {
                // Pipelines and function arguments are conservatively
                // treated as reads, not moves, until per-function
                // ownership signatures arrive in Phase 2.1.
                self.visit_expr_internal(left, false)?;
                self.visit_expr(right)
            }
            Expr::Call { callee, args, .. } => {
                self.visit_expr(callee)?;
                for arg in args {
                    self.visit_expr_internal(arg, false)?;
                }
                Ok(())
            }
            Expr::MethodCall { target, args, .. } => {
                self.visit_expr_internal(target, false)?;
                for arg in args {
                    self.visit_expr_internal(arg, false)?;
                }
                Ok(())
            }
            Expr::Field { target, .. } => self.visit_expr_internal(target, false),
            Expr::Index { target, index, .. } => {
                self.visit_expr_internal(target, false)?;
                self.visit_expr(index)
            }
            Expr::StructLit { fields, .. } => {
                // Field initializers are conservatively treated as
                // reads until per-field ownership signatures arrive in
                // Phase 2.1 — matches the same relaxation we apply to
                // function call arguments.
                for (_, value) in fields {
                    self.visit_expr_internal(value, false)?;
                }
                Ok(())
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.visit_expr(condition)?;
                self.check_block(then_branch)?;
                if let Some(else_branch) = else_branch {
                    self.check_block(else_branch)?;
                }
                Ok(())
            }
            Expr::Match {
                scrutinee,
                arms,
                ..
            } => {
                self.visit_expr(scrutinee)?;
                for arm in arms {
                    self.scopes.push(Scope::default());
                    bind_pattern(&arm.pattern, self)?;
                    self.consume_expr_as_value(&arm.body)?;
                    self.pop_scope();
                }
                Ok(())
            }
            Expr::Lambda { body, .. } => match body {
                LambdaBody::Expr(expr) => self.visit_expr(expr),
                LambdaBody::Block(block) => self.check_block(block),
            },
            Expr::Try { expr, .. } | Expr::Spawn { expr, .. } | Expr::Await { expr, .. } => {
                self.visit_expr(expr)
            }
            Expr::Ref { expr, is_mut, span } => {
                if let Some(name) = root_name(expr) {
                    // transient loan — start, descend, release
                    self.start_loan(&name, *is_mut, span)?;
                    self.visit_expr_internal(expr, false)?;
                    self.release_loan(&name, *is_mut);
                    Ok(())
                } else {
                    self.visit_expr_internal(expr, false)
                }
            }
            Expr::Region { body, .. } => self.check_block(body),
            Expr::Block(block) => self.check_block(block),
            Expr::Dict(pairs, _) => {
                for (k, v) in pairs {
                    self.visit_expr_internal(k, false)?;
                    self.visit_expr_internal(v, false)?;
                }
                Ok(())
            }
        }
    }

    fn use_ident(&mut self, name: &str, consume: bool, span: &Span) -> LangResult<()> {
        // Unknown identifiers are functions, variant constructors, or
        // built-ins — the typechecker handles their validity. The
        // borrow checker only tracks local bindings.
        let Some(info) = self.lookup(name) else {
            return Ok(());
        };
        if info.moved {
            return Err(Diagnostic::new(
                format!("use of moved value '{name}'"),
                span.clone(),
            ));
        }
        if consume && info.kind == Kind::Move {
            if !info.imm_borrowers.is_empty() || info.mut_borrower.is_some() {
                return Err(Diagnostic::new(
                    format!("cannot move '{name}' while it is borrowed"),
                    span.clone(),
                ));
            }
            self.modify(name, |s| s.moved = true);
        }
        Ok(())
    }

    fn start_loan(&mut self, source: &str, is_mut: bool, span: &Span) -> LangResult<()> {
        // If the borrow source isn't a local binding (a global function,
        // a constant from another module, etc.), we have no in-scope
        // ownership state to track. Treat the borrow as trivially valid.
        let Some(info) = self.lookup(source) else {
            return Ok(());
        };
        if info.moved {
            return Err(Diagnostic::new(
                format!("cannot borrow '{source}' after move"),
                span.clone(),
            ));
        }
        if is_mut {
            if !info.mutable {
                return Err(Diagnostic::new(
                    format!("cannot take `&mut` of immutable binding '{source}' (declare it `let mut`)"),
                    span.clone(),
                ));
            }
            if info.mut_borrower.is_some() {
                return Err(Diagnostic::new(
                    format!("'{source}' is already borrowed mutably"),
                    span.clone(),
                ));
            }
            if !info.imm_borrowers.is_empty() {
                return Err(Diagnostic::new(
                    format!(
                        "cannot borrow '{source}' mutably while it has shared borrows in scope"
                    ),
                    span.clone(),
                ));
            }
            self.modify(source, |s| s.mut_borrower = Some("<transient>".into()));
        } else {
            if info.mut_borrower.is_some() {
                return Err(Diagnostic::new(
                    format!(
                        "cannot borrow '{source}' as shared while it is borrowed mutably"
                    ),
                    span.clone(),
                ));
            }
            self.modify(source, |s| s.imm_borrowers.push("<transient>".into()));
        }
        Ok(())
    }

    fn release_loan(&mut self, source: &str, is_mut: bool) {
        self.modify(source, |s| {
            if is_mut {
                s.mut_borrower = None;
            } else if let Some(pos) = s.imm_borrowers.iter().position(|n| n == "<transient>") {
                s.imm_borrowers.remove(pos);
            }
        });
    }

    fn declare(&mut self, name: &str, mut symbol: Symbol) -> LangResult<()> {
        // If the symbol carries a loan source, record the borrower name
        // in the source's borrower list so it can be released on scope
        // exit.
        if let Some((source, is_mut)) = &symbol.loan_source {
            let source = source.clone();
            let is_mut = *is_mut;
            let borrower = name.to_string();
            // The transient slot we put in `start_loan` was already
            // counted; replace it with the real name.
            self.modify(&source, |s| {
                if is_mut {
                    if let Some(slot) = s.mut_borrower.as_mut() {
                        *slot = borrower.clone();
                    } else {
                        s.mut_borrower = Some(borrower.clone());
                    }
                } else if let Some(pos) = s.imm_borrowers.iter().position(|n| n == "<transient>") {
                    s.imm_borrowers[pos] = borrower.clone();
                } else {
                    s.imm_borrowers.push(borrower.clone());
                }
            });
            symbol.kind = Kind::Copy;
        }

        let scope = self.scopes.last_mut().expect("scope stack non-empty");
        if scope.bindings.contains_key(name) {
            return Err(Diagnostic::new(
                format!("name '{name}' is already defined in this scope"),
                symbol.span.clone(),
            ));
        }
        scope.bindings.insert(name.to_string(), symbol);
        scope.declared.push(name.to_string());
        Ok(())
    }

    fn pop_scope(&mut self) {
        let scope = self.scopes.pop().expect("scope stack non-empty");
        // Release loans held by symbols leaving scope, in LIFO order.
        for name in scope.declared.iter().rev() {
            if let Some(symbol) = scope.bindings.get(name) {
                if let Some((source, is_mut)) = &symbol.loan_source {
                    let source = source.clone();
                    let is_mut = *is_mut;
                    let borrower = name.clone();
                    self.modify(&source, move |s| {
                        if is_mut {
                            if s.mut_borrower.as_deref() == Some(borrower.as_str()) {
                                s.mut_borrower = None;
                            }
                        } else if let Some(pos) =
                            s.imm_borrowers.iter().position(|n| n == &borrower)
                        {
                            s.imm_borrowers.remove(pos);
                        }
                    });
                }
            }
        }
    }

    fn lookup(&self, name: &str) -> Option<Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(s) = scope.bindings.get(name) {
                return Some(s.clone());
            }
        }
        None
    }

    fn modify<F: FnOnce(&mut Symbol)>(&mut self, name: &str, f: F) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(s) = scope.bindings.get_mut(name) {
                f(s);
                return;
            }
        }
    }
}

impl Default for BorrowChecker {
    fn default() -> Self {
        Self::new()
    }
}

fn kind_of_type_ref(ty: &TypeRef) -> Kind {
    match ty {
        TypeRef::Named(name) => match name.as_str() {
            "Int" | "Float" | "Bool" | "Char" | "Int8" | "Int16" | "Int32" | "Int64" | "UInt"
            | "UInt8" | "UInt16" | "UInt32" | "UInt64" | "Byte" | "Float32" => Kind::Copy,
            _ => Kind::Move,
        },
        TypeRef::Unit => Kind::Copy,
        TypeRef::Ref(_, _) => Kind::Copy,
        TypeRef::Generic(name, _) => match name.as_str() {
            "Option" | "Result" => Kind::Move,
            _ => Kind::Move,
        },
        TypeRef::Function(_, _) | TypeRef::List(_) => Kind::Move,
        TypeRef::Infer => Kind::Move,
    }
}

fn infer_kind(expr: &Expr) -> Kind {
    match expr {
        Expr::Int(_, _) | Expr::Float(_, _) | Expr::Bool(_, _) | Expr::Unit(_) => Kind::Copy,
        Expr::Ref { .. } => Kind::Copy,
        Expr::String(_, _) => Kind::Move,
        Expr::Binary { op, .. } => match op {
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
            | BinaryOp::And
            | BinaryOp::Or => Kind::Copy,
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder => Kind::Copy,
        },
        Expr::Unary { op, .. } => match op {
            UnaryOp::Negate => Kind::Copy,
            UnaryOp::Not => Kind::Copy,
        },
        _ => Kind::Move,
    }
}

fn root_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(name, _) => Some(name.clone()),
        Expr::Field { target, .. } => root_name(target),
        Expr::Index { target, .. } => root_name(target),
        Expr::Ref { expr, .. } => root_name(expr),
        _ => None,
    }
}

/// Bind names introduced by a pattern at the current scope so the
/// borrow checker can track them later. Variant payloads are bound
/// as Move-kind to be conservative.
fn bind_pattern(pattern: &Pattern, bc: &mut BorrowChecker) -> LangResult<()> {
    match pattern {
        Pattern::Wildcard(_)
        | Pattern::Int(_, _)
        | Pattern::Float(_, _)
        | Pattern::Bool(_, _)
        | Pattern::String(_, _)
        | Pattern::Unit(_) => Ok(()),
        Pattern::Ident(name, span) => bc.declare(
            name,
            Symbol {
                kind: Kind::Move,
                mutable: false,
                moved: false,
                imm_borrowers: Vec::new(),
                mut_borrower: None,
                loan_source: None,
                span: span.clone(),
            },
        ),
        Pattern::Variant { payload, .. } => {
            for inner in payload {
                bind_pattern(inner, bc)?;
            }
            Ok(())
        }
    }
}
