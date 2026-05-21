use crate::diagnostic::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Function(FunctionDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Const(ConstDecl),
    Module(ModuleDecl),
    Import(ImportDecl),
    Trait(TraitDecl),
    Impl(ImplBlock),
    Extern(ExternBlock),
    Statement(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub params: Vec<Param>,
    pub return_type: TypeRef,
    pub body: Block,
    pub is_async: bool,
    pub is_pub: bool,
    /// Outer attributes attached via `#[name]` immediately before the
    /// `fn` keyword. Currently a flat list of identifier names; the
    /// native stage-2 grows this to support `#[name(args)]`.
    pub attrs: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: TypeRef,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<Field>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: TypeRef,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub variants: Vec<Variant>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub name: String,
    pub payload: Vec<TypeRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub name: String,
    pub ty: Option<TypeRef>,
    pub value: Expr,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDecl {
    pub name: String,
    pub items: Vec<Item>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub path: Vec<String>,
    pub items: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub methods: Vec<TraitMethod>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplBlock {
    pub trait_name: Option<String>,
    pub target: String,
    pub generics: Vec<String>,
    pub methods: Vec<FunctionDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternBlock {
    pub language: String,
    pub library: Option<String>,
    pub items: Vec<ExternItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternItem {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<TypeRef>,
        mutable: bool,
        value: Expr,
        span: Span,
    },
    Const(ConstDecl),
    Assign {
        target: AssignTarget,
        value: Expr,
        span: Span,
    },
    Expr {
        expr: Expr,
        has_semicolon: bool,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    For {
        name: String,
        iter: Expr,
        body: Block,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Name(String),
    Field { target: Expr, name: String },
    Index { target: Expr, index: Expr },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64, Span),
    Float(f64, Span),
    Bool(bool, Span),
    String(String, Span),
    Unit(Span),
    Ident(String, Span),
    Path(Vec<String>, Span),
    List(Vec<Expr>, Span),
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    Pipeline {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    MethodCall {
        target: Box<Expr>,
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    Field {
        target: Box<Expr>,
        name: String,
        span: Span,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    StructLit {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    If {
        condition: Box<Expr>,
        then_branch: Block,
        else_branch: Option<Block>,
        span: Span,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Lambda {
        params: Vec<Param>,
        return_type: Option<TypeRef>,
        body: LambdaBody,
        span: Span,
    },
    Try {
        expr: Box<Expr>,
        span: Span,
    },
    Spawn {
        expr: Box<Expr>,
        span: Span,
    },
    Await {
        expr: Box<Expr>,
        span: Span,
    },
    Ref {
        expr: Box<Expr>,
        is_mut: bool,
        span: Span,
    },
    Region {
        name: String,
        body: Block,
        span: Span,
    },
    Block(Block),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, span)
            | Expr::Float(_, span)
            | Expr::Bool(_, span)
            | Expr::String(_, span)
            | Expr::Unit(span)
            | Expr::Ident(_, span)
            | Expr::Path(_, span)
            | Expr::List(_, span)
            | Expr::Range { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Pipeline { span, .. }
            | Expr::Call { span, .. }
            | Expr::MethodCall { span, .. }
            | Expr::Field { span, .. }
            | Expr::Index { span, .. }
            | Expr::StructLit { span, .. }
            | Expr::If { span, .. }
            | Expr::Match { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Try { span, .. }
            | Expr::Spawn { span, .. }
            | Expr::Await { span, .. }
            | Expr::Ref { span, .. }
            | Expr::Region { span, .. } => span.clone(),
            Expr::Block(block) => block.span.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LambdaBody {
    Expr(Box<Expr>),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard(Span),
    Ident(String, Span),
    Int(i64, Span),
    Float(f64, Span),
    Bool(bool, Span),
    String(String, Span),
    Unit(Span),
    Variant {
        name: String,
        payload: Vec<Pattern>,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard(span)
            | Pattern::Ident(_, span)
            | Pattern::Int(_, span)
            | Pattern::Float(_, span)
            | Pattern::Bool(_, span)
            | Pattern::String(_, span)
            | Pattern::Unit(span)
            | Pattern::Variant { span, .. } => span.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
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

#[derive(Debug, Clone, PartialEq)]
pub enum TypeRef {
    Named(String),
    Generic(String, Vec<TypeRef>),
    Function(Vec<TypeRef>, Box<TypeRef>),
    List(Box<TypeRef>),
    Ref(Box<TypeRef>, bool /* is_mut */),
    Unit,
    Infer,
}
