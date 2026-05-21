//! MIR — Mid-level IR.
//!
//! Phase 1 deliverable scaffold. The MIR is a three-address-code, SSA-ish
//! representation suitable for optimization passes and backend codegen.
//! The current C-codegen backend skips MIR for the supported subset and
//! lowers the AST/HIR directly. MIR types are defined here so the LLVM
//! backend, inliner, borrow checker, and bounds-check eliminator can
//! evolve against a stable target.
//!
//! Design notes:
//! - Functions are CFGs of `MirBlock`s.
//! - Each block ends with exactly one `MirTerminator`.
//! - Locals are addressed by `LocalId`.
//! - SSA form is established lazily by the dominance + phi pass.

use crate::hir::HirType;
use crate::diagnostic::Span;

pub type LocalId = u32;
pub type BlockId = u32;

#[derive(Debug, Clone)]
pub struct MirFunction {
    pub name: String,
    pub params: Vec<MirLocal>,
    pub locals: Vec<MirLocal>,
    pub blocks: Vec<MirBlock>,
    pub return_type: HirType,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MirLocal {
    pub id: LocalId,
    pub name: Option<String>,
    pub ty: HirType,
}

#[derive(Debug, Clone)]
pub struct MirBlock {
    pub id: BlockId,
    pub statements: Vec<MirStmt>,
    pub terminator: MirTerminator,
}

#[derive(Debug, Clone)]
pub enum MirStmt {
    Assign {
        dest: LocalId,
        rvalue: MirRvalue,
    },
    /// Run a side-effecting expression for its effect alone.
    Discard { value: MirRvalue },
}

#[derive(Debug, Clone)]
pub enum MirRvalue {
    Const(MirConst),
    Use(LocalId),
    BinaryOp {
        op: MirBinaryOp,
        left: LocalId,
        right: LocalId,
    },
    UnaryOp {
        op: MirUnaryOp,
        operand: LocalId,
    },
    Call {
        callee: String,
        args: Vec<LocalId>,
    },
}

#[derive(Debug, Clone)]
pub enum MirConst {
    Int(i64),
    Bool(bool),
    Unit,
}

#[derive(Debug, Clone, Copy)]
pub enum MirBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy)]
pub enum MirUnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum MirTerminator {
    Return { value: Option<LocalId> },
    Goto { target: BlockId },
    Branch {
        condition: LocalId,
        then_block: BlockId,
        else_block: BlockId,
    },
    Unreachable,
}

#[derive(Debug, Clone, Default)]
pub struct MirProgram {
    pub functions: Vec<MirFunction>,
}
