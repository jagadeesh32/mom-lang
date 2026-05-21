//! HIR — High-level IR.
//!
//! Phase 1 deliverable scaffold. The native compiler lowers the AST into
//! the HIR before borrow checking, monomorphization, and MIR lowering.
//! The current C-codegen backend lowers the AST directly; the HIR types
//! are defined here so that downstream passes (borrow checker, generics,
//! LLVM backend) have a stable target to evolve against.
//!
//! See `docs/compiler.md` for the layered pipeline.

use crate::diagnostic::Span;

/// A fully name-resolved function in HIR form.
#[derive(Debug, Clone)]
pub struct HirFunction {
    pub name: String,
    pub generics: Vec<String>,
    pub params: Vec<HirParam>,
    pub return_type: HirType,
    pub body: HirBlock,
    pub is_async: bool,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HirParam {
    pub name: String,
    pub ty: HirType,
}

#[derive(Debug, Clone)]
pub struct HirBlock {
    pub statements: Vec<HirStmt>,
    pub result: Option<Box<HirExpr>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum HirStmt {
    Let {
        name: String,
        mutable: bool,
        ty: HirType,
        value: HirExpr,
        span: Span,
    },
    Assign {
        name: String,
        value: HirExpr,
        span: Span,
    },
    Expr {
        expr: HirExpr,
        span: Span,
    },
    Return {
        value: Option<HirExpr>,
        span: Span,
    },
    While {
        condition: HirExpr,
        body: HirBlock,
        span: Span,
    },
    ForRange {
        name: String,
        start: HirExpr,
        end: HirExpr,
        body: HirBlock,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum HirExpr {
    Int(i64, Span),
    Bool(bool, Span),
    Unit(Span),
    Ident(String, Span),
    Unary {
        op: HirUnaryOp,
        expr: Box<HirExpr>,
        span: Span,
    },
    Binary {
        op: HirBinaryOp,
        left: Box<HirExpr>,
        right: Box<HirExpr>,
        span: Span,
    },
    Call {
        callee: String,
        args: Vec<HirExpr>,
        span: Span,
    },
    If {
        condition: Box<HirExpr>,
        then_branch: HirBlock,
        else_branch: Option<HirBlock>,
        span: Span,
    },
    Block(HirBlock),
}

#[derive(Debug, Clone, Copy)]
pub enum HirUnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum HirBinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

/// Phase 1 supported HIR types. The native backend grows this enum to
/// cover Float / String / List / Struct / Enum / generics.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HirType {
    Int,
    Bool,
    Unit,
    /// Placeholder for types not yet supported in native codegen.
    /// The Phase-1 codegen rejects programs that produce this in
    /// reachable positions.
    Opaque(String),
}

/// Top-level HIR program — a flat list of functions after module
/// flattening.
#[derive(Debug, Clone, Default)]
pub struct HirProgram {
    pub functions: Vec<HirFunction>,
}
