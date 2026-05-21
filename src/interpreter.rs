use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::rc::Rc;

use crate::ast::*;
use crate::diagnostic::{Diagnostic, LangResult, Span};

// ── Value ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
    List(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<HashMap<String, Value>>>),
    Range(i64, i64),
    Struct(String, Rc<RefCell<HashMap<String, Value>>>),
    Variant(String, String, Vec<Value>),
    Task(Box<Value>),
    Pointer(String, Rc<RefCell<Value>>),
    Ref(Box<Value>, bool),
    Channel(Rc<ChannelInner>),
    Cancel(Rc<RefCell<bool>>),
    Function(FunctionDecl),
    Lambda(LambdaValue),
    Builtin(Builtin),
    Method(String, FunctionDecl),
}

#[derive(Debug, Clone)]
pub struct LambdaValue {
    params: Vec<Param>,
    body: LambdaBody,
    captured_scopes: Vec<HashMap<String, Binding>>,
}

#[derive(Debug)]
pub struct ChannelInner {
    pub capacity: Option<usize>,
    pub queue: RefCell<VecDeque<Value>>,
    pub closed: RefCell<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    // I/O
    Print, Println, Eprint, Input,
    // String conversion
    ToString, Str, Int_, Float_, Bool_,
    // Math
    Abs, Min, Max, Pow, Round, Floor, Ceil, Sqrt,
    // Collections
    Len, Push, Pop, Insert, Remove, Reverse, Sort, Sorted, Reversed,
    Sum, Any, All,
    Map, Filter, Reduce, Enumerate, Zip,
    Range_, RangeStep,
    // String operations
    Split, Join, Upper, Lower, Strip, Lstrip, Rstrip,
    StartsWith, EndsWith, Contains, Find, Replace, Chars,
    // Type checks
    TypeOf, IsInt, IsFloat, IsString, IsBool, IsList, IsDict, IsNone,
    // System
    Args, Getenv, Sleep, ReadFile, WriteFile, Exit,
    // Panic / assert
    Panic, Assert,
    // Numeric helpers
    Hex, Oct, Bin, Ord, Chr, DivMod,
    // Dict
    Dict_,
    // Pointer / channel
    BoxNew, RcNew, ArcNew, ChannelNew, CancelNew,
    // Old builtins (compat)
    IsDigit, IsAlpha, IsAlnum, ParseInt, StringEq,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(v)    => write!(f, "{v}"),
            Value::Float(v)  => {
                if v.fract() == 0.0 && v.abs() < 1e15 {
                    write!(f, "{v:.1}")
                } else {
                    write!(f, "{v}")
                }
            }
            Value::Bool(v)   => write!(f, "{v}"),
            Value::String(v) => write!(f, "{v}"),
            Value::Unit      => write!(f, "none"),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, v) in items.borrow().iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Value::Dict(map) => {
                write!(f, "{{")?;
                let m = map.borrow();
                let mut entries: Vec<_> = m.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "\"{k}\": {v}")?;
                }
                write!(f, "}}")
            }
            Value::Range(s, e) => write!(f, "{s}..{e}"),
            Value::Struct(name, fields) => {
                write!(f, "{name} {{ ")?;
                let fields = fields.borrow();
                let mut entries: Vec<_> = fields.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, " }}")
            }
            Value::Variant(_e, name, payload) => {
                write!(f, "{name}")?;
                if !payload.is_empty() {
                    write!(f, "(")?;
                    for (i, v) in payload.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{v}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Value::Task(inner) => write!(f, "<task {inner}>"),
            Value::Pointer(kind, cell) => write!(f, "{kind}({})", cell.borrow()),
            Value::Ref(inner, _) => write!(f, "{inner}"),
            Value::Channel(chan) => {
                let cap = chan.capacity.map(|c| c.to_string())
                    .unwrap_or_else(|| "∞".to_string());
                write!(f, "<channel len={} cap={}>", chan.queue.borrow().len(), cap)
            }
            Value::Cancel(state) => write!(f, "<cancel {}>",
                if *state.borrow() { "signalled" } else { "live" }),
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::Lambda(_)      => write!(f, "<lambda>"),
            Value::Builtin(b)     => write!(f, "<builtin {b:?}>"),
            Value::Method(t, func) => write!(f, "<method {t}.{}>", func.name),
        }
    }
}

// ── Interpreter internals ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Binding {
    value: Value,
    mutable: bool,
}

#[derive(Debug, Clone)]
enum Flow {
    Value(Value),
    Return(Value),
    Break,
    Continue,
}

pub struct Interpreter {
    functions: HashMap<String, FunctionDecl>,
    methods: HashMap<String, HashMap<String, FunctionDecl>>,
    enum_variants: HashMap<String, (String, usize)>,
    extern_names: HashMap<String, (String, Option<String>)>,
    scopes: Vec<HashMap<String, Binding>>,
    output: String,
    probe: Option<Rc<RefCell<crate::prof::ProfilerState>>>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut enum_variants = HashMap::new();
        enum_variants.insert("Some".to_string(), ("Option".to_string(), 1));
        enum_variants.insert("None".to_string(), ("Option".to_string(), 0));
        enum_variants.insert("Ok".to_string(),   ("Result".to_string(), 1));
        enum_variants.insert("Err".to_string(),  ("Result".to_string(), 1));
        Self {
            functions: HashMap::new(),
            methods: HashMap::new(),
            enum_variants,
            extern_names: HashMap::new(),
            scopes: vec![HashMap::new()],
            output: String::new(),
            probe: None,
        }
    }

    pub fn attach_probe(&mut self, probe: Rc<RefCell<crate::prof::ProfilerState>>) {
        self.probe = Some(probe);
    }

    pub fn run_program(mut self, program: &Program) -> LangResult<String> {
        self.register_items(&program.items);
        for item in &program.items {
            match item {
                Item::Const(decl) => {
                    let value = self.eval_value(&decl.value)?;
                    self.define(&decl.name, value, false, decl.span.clone())?;
                }
                Item::Statement(stmt) => match self.eval_stmt(stmt)? {
                    Flow::Value(_) => {}
                    Flow::Return(_) => {
                        return Err(error("'return' is only valid inside a function body", stmt_span(stmt)));
                    }
                    Flow::Break | Flow::Continue => {
                        return Err(error("loop control escaped its loop", stmt_span(stmt)));
                    }
                },
                _ => {}
            }
        }
        if let Some(main) = self.functions.get("main").cloned() {
            self.call_function(&main, Vec::new(), &main.span)?;
        }
        Ok(self.output)
    }

    fn register_items(&mut self, items: &[Item]) {
        for item in items {
            match item {
                Item::Function(f) => { self.functions.insert(f.name.clone(), f.clone()); }
                Item::Enum(decl) => {
                    for variant in &decl.variants {
                        self.enum_variants.insert(
                            variant.name.clone(),
                            (decl.name.clone(), variant.payload.len()),
                        );
                    }
                }
                Item::Impl(block) => {
                    let entry = self.methods.entry(block.target.clone()).or_default();
                    for method in &block.methods {
                        entry.insert(method.name.clone(), method.clone());
                    }
                }
                Item::Extern(block) => {
                    for item in &block.items {
                        self.extern_names.insert(
                            item.name.clone(),
                            (block.language.clone(), block.library.clone()),
                        );
                    }
                }
                Item::Module(module) => { self.register_items(&module.items); }
                _ => {}
            }
        }
    }

    // ── Statement evaluation ──────────────────────────────────────────────────

    fn eval_stmt(&mut self, stmt: &Stmt) -> LangResult<Flow> {
        match stmt {
            Stmt::Let { name, mutable, value, span, .. } => {
                let value = match self.eval_expr(value)? {
                    Flow::Value(v) => v,
                    ctrl => return Ok(ctrl),
                };
                self.define(name, value, *mutable, span.clone())?;
                Ok(Flow::Value(Value::Unit))
            }
            Stmt::Const(decl) => {
                let value = self.eval_value(&decl.value)?;
                self.define(&decl.name, value, false, decl.span.clone())?;
                Ok(Flow::Value(Value::Unit))
            }
            Stmt::Assign { target, value, span } => {
                let value = match self.eval_expr(value)? {
                    Flow::Value(v) => v,
                    ctrl => return Ok(ctrl),
                };
                self.assign_target(target, value, span)?;
                Ok(Flow::Value(Value::Unit))
            }
            Stmt::Expr { expr, has_semicolon, .. } => match self.eval_expr(expr)? {
                Flow::Value(_) if *has_semicolon => Ok(Flow::Value(Value::Unit)),
                flow => Ok(flow),
            },
            Stmt::Return { value, .. } => {
                let v = if let Some(expr) = value {
                    match self.eval_expr(expr)? {
                        Flow::Value(v) => v,
                        ctrl => return Ok(ctrl),
                    }
                } else {
                    Value::Unit
                };
                Ok(Flow::Return(v))
            }
            Stmt::While { condition, body, span } => {
                loop {
                    let cond = self.eval_value(condition)?;
                    let is_true = match cond {
                        Value::Bool(b) => b,
                        Value::Int(n) => n != 0,
                        _ => return Err(error("while condition must be Bool or Int", span)),
                    };
                    if !is_true { break; }
                    match self.eval_block(body)? {
                        Flow::Value(_) => {}
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Break => break,
                        Flow::Continue => continue,
                    }
                }
                Ok(Flow::Value(Value::Unit))
            }
            Stmt::For { name, iter, body, span } => self.eval_for(name, iter, body, span),
            Stmt::Break { .. }    => Ok(Flow::Break),
            Stmt::Continue { .. } => Ok(Flow::Continue),
        }
    }

    fn eval_for(&mut self, name: &str, iter: &Expr, body: &Block, span: &Span) -> LangResult<Flow> {
        let iter_value = self.eval_value(iter)?;
        let items: Vec<Value> = match iter_value {
            Value::List(items) => items.borrow().clone(),
            Value::Range(start, end) => (start..end).map(Value::Int).collect(),
            Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
            other => return Err(error(format!("cannot iterate over {other}"), span)),
        };
        for item in items {
            self.push_scope();
            self.define(name, item, false, span.clone())?;
            let flow = self.eval_block(body)?;
            self.pop_scope();
            match flow {
                Flow::Value(_) => {}
                Flow::Return(v) => return Ok(Flow::Return(v)),
                Flow::Break => break,
                Flow::Continue => continue,
            }
        }
        Ok(Flow::Value(Value::Unit))
    }

    fn assign_target(&mut self, target: &AssignTarget, value: Value, span: &Span) -> LangResult<()> {
        match target {
            AssignTarget::Name(name) => self.assign(name, value, span),
            AssignTarget::Field { target: t, name } => {
                let tv = self.eval_value(t)?;
                match tv {
                    Value::Struct(_, fields) => {
                        fields.borrow_mut().insert(name.clone(), value);
                        Ok(())
                    }
                    Value::Dict(map) => {
                        map.borrow_mut().insert(name.clone(), value);
                        Ok(())
                    }
                    other => Err(error(format!("cannot set field on {other}"), span)),
                }
            }
            AssignTarget::Index { target: t, index } => {
                let tv = self.eval_value(t)?;
                let iv = self.eval_value(index)?;
                match (tv, iv) {
                    (Value::List(items), Value::Int(idx)) => {
                        let mut items = items.borrow_mut();
                        let len = items.len() as i64;
                        let real_idx = if idx < 0 { len + idx } else { idx };
                        if real_idx < 0 || real_idx >= len {
                            return Err(error(format!("index {idx} out of range (len={len})"), span));
                        }
                        items[real_idx as usize] = value;
                        Ok(())
                    }
                    (Value::Dict(map), Value::String(key)) => {
                        map.borrow_mut().insert(key, value);
                        Ok(())
                    }
                    (target, idx) => Err(error(format!("cannot index {target} with {idx}"), span)),
                }
            }
        }
    }

    // ── Expression evaluation ─────────────────────────────────────────────────

    fn eval_expr(&mut self, expr: &Expr) -> LangResult<Flow> {
        match expr {
            Expr::Int(v, _)    => Ok(Flow::Value(Value::Int(*v))),
            Expr::Float(v, _)  => Ok(Flow::Value(Value::Float(*v))),
            Expr::Bool(v, _)   => Ok(Flow::Value(Value::Bool(*v))),
            Expr::String(v, _) => Ok(Flow::Value(Value::String(v.clone()))),
            Expr::Unit(_)      => Ok(Flow::Value(Value::Unit)),
            Expr::Ident(name, span) => self.resolve_name(name, span),
            Expr::Path(segments, span) => {
                if let Some(last) = segments.last() {
                    self.resolve_name(last, span)
                } else {
                    Err(error("empty path", span))
                }
            }
            Expr::List(items, _) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    match self.eval_expr(item)? {
                        Flow::Value(v) => values.push(v),
                        ctrl => return Ok(ctrl),
                    }
                }
                Ok(Flow::Value(Value::List(Rc::new(RefCell::new(values)))))
            }
            Expr::Dict(pairs, _) => {
                let mut map = HashMap::new();
                for (key_expr, val_expr) in pairs {
                    let key = match self.eval_value(key_expr)? {
                        Value::String(s) => s,
                        Value::Int(n)    => n.to_string(),
                        Value::Bool(b)   => b.to_string(),
                        other => return Err(error(
                            format!("dict keys must be String/Int/Bool, got {other}"),
                            &key_expr.span(),
                        )),
                    };
                    let val = match self.eval_expr(val_expr)? {
                        Flow::Value(v) => v,
                        ctrl => return Ok(ctrl),
                    };
                    map.insert(key, val);
                }
                Ok(Flow::Value(Value::Dict(Rc::new(RefCell::new(map)))))
            }
            Expr::Range { start, end, span } => {
                match (self.eval_value(start)?, self.eval_value(end)?) {
                    (Value::Int(a), Value::Int(b)) => Ok(Flow::Value(Value::Range(a, b))),
                    _ => Err(error("range bounds must be Int", span)),
                }
            }
            Expr::Unary { op, expr, span } => {
                let v = match self.eval_expr(expr)? {
                    Flow::Value(v) => v,
                    ctrl => return Ok(ctrl),
                };
                Ok(Flow::Value(self.eval_unary(*op, v, span)?))
            }
            Expr::Binary { left, op, right, span } => self.eval_binary(left, *op, right, span),
            Expr::Pipeline { left, right, span }   => self.eval_pipeline(left, right, span),
            Expr::Call { callee, args, span } => {
                if let Expr::Ident(name, _) = callee.as_ref() {
                    if let Some((enum_name, _)) = self.enum_variants.get(name).cloned() {
                        let args = self.eval_args(args)?;
                        return Ok(Flow::Value(Value::Variant(enum_name, name.clone(), args)));
                    }
                }
                let callee_v = match self.eval_expr(callee)? {
                    Flow::Value(v) => v,
                    ctrl => return Ok(ctrl),
                };
                let arg_vals = self.eval_args(args)?;
                Ok(Flow::Value(self.call_value(callee_v, arg_vals, span)?))
            }
            Expr::MethodCall { target, name, args, span } => {
                self.eval_method_call(target, name, args, span)
            }
            Expr::Field { target, name, span } => {
                let tv = self.eval_value(target)?;
                match tv {
                    Value::Struct(sname, fields) => {
                        if let Some(v) = fields.borrow().get(name) {
                            Ok(Flow::Value(v.clone()))
                        } else if let Some(m) = self.methods.get(&sname).and_then(|m| m.get(name)) {
                            Ok(Flow::Value(Value::Method(sname.clone(), m.clone())))
                        } else {
                            Err(error(format!("type '{sname}' has no field '{name}'"), span))
                        }
                    }
                    Value::Dict(map) => {
                        if let Some(v) = map.borrow().get(name) {
                            Ok(Flow::Value(v.clone()))
                        } else {
                            Ok(Flow::Value(Value::Unit))
                        }
                    }
                    Value::List(items) if name == "len" => {
                        Ok(Flow::Value(Value::Int(items.borrow().len() as i64)))
                    }
                    other => Err(error(format!("cannot read '{name}' on {other}"), span)),
                }
            }
            Expr::Index { target, index, span } => {
                let tv = self.eval_value(target)?;
                let iv = self.eval_value(index)?;
                match (tv, iv) {
                    (Value::List(items), Value::Int(idx)) => {
                        let items = items.borrow();
                        let len = items.len() as i64;
                        let real = if idx < 0 { len + idx } else { idx };
                        if real < 0 || real >= len {
                            return Err(error(format!("index {idx} out of range (len={len})"), span));
                        }
                        Ok(Flow::Value(items[real as usize].clone()))
                    }
                    (Value::String(s), Value::Int(idx)) => {
                        let chars: Vec<char> = s.chars().collect();
                        let len = chars.len() as i64;
                        let real = if idx < 0 { len + idx } else { idx };
                        if real < 0 || real >= len {
                            return Err(error(format!("index {idx} out of range (len={len})"), span));
                        }
                        Ok(Flow::Value(Value::String(chars[real as usize].to_string())))
                    }
                    (Value::Dict(map), Value::String(key)) => {
                        let v = map.borrow().get(&key).cloned()
                            .unwrap_or(Value::Unit);
                        Ok(Flow::Value(v))
                    }
                    (target, idx) => Err(error(format!("cannot index {target} with {idx}"), span)),
                }
            }
            Expr::StructLit { name, fields, .. } => {
                let mut map = HashMap::new();
                for (fname, fexpr) in fields {
                    let v = match self.eval_expr(fexpr)? {
                        Flow::Value(v) => v,
                        ctrl => return Ok(ctrl),
                    };
                    map.insert(fname.clone(), v);
                }
                Ok(Flow::Value(Value::Struct(name.clone(), Rc::new(RefCell::new(map)))))
            }
            Expr::If { condition, then_branch, else_branch, span } => {
                let cond = self.eval_value(condition)?;
                let is_true = match cond {
                    Value::Bool(b) => b,
                    Value::Int(n)  => n != 0,
                    Value::Unit    => false,
                    _ => return Err(error("if condition must be Bool", span)),
                };
                if is_true {
                    self.eval_block(then_branch)
                } else if let Some(else_b) = else_branch {
                    self.eval_block(else_b)
                } else {
                    Ok(Flow::Value(Value::Unit))
                }
            }
            Expr::Match { scrutinee, arms, span } => {
                let s = self.eval_value(scrutinee)?;
                for arm in arms {
                    if let Some(bindings) = self.pattern_matches(&arm.pattern, &s) {
                        self.push_scope();
                        for (n, v) in bindings {
                            self.define(&n, v, false, arm.span.clone())?;
                        }
                        let result = self.eval_expr(&arm.body);
                        self.pop_scope();
                        return result;
                    }
                }
                Err(error("non-exhaustive match reached at runtime", span))
            }
            Expr::Lambda { params, body, .. } => {
                Ok(Flow::Value(Value::Lambda(LambdaValue {
                    params: params.clone(),
                    body: body.clone(),
                    captured_scopes: self.scopes.clone(),
                })))
            }
            Expr::Try { expr, span } => {
                let v = match self.eval_expr(expr)? {
                    Flow::Value(v) => v,
                    ctrl => return Ok(ctrl),
                };
                match &v {
                    Value::Variant(e, name, payload) if e == "Result" && name == "Ok" && payload.len() == 1 => {
                        Ok(Flow::Value(payload[0].clone()))
                    }
                    Value::Variant(e, name, _) if e == "Result" && name == "Err" => {
                        Ok(Flow::Return(v))
                    }
                    Value::Variant(e, name, payload) if e == "Option" && name == "Some" && payload.len() == 1 => {
                        Ok(Flow::Value(payload[0].clone()))
                    }
                    Value::Variant(e, name, _) if e == "Option" && name == "None" => {
                        Ok(Flow::Return(v))
                    }
                    other => Err(error(format!("'?' is only valid on Result or Option, got {other}"), span)),
                }
            }
            Expr::Spawn { expr, .. } => {
                let v = self.eval_value(expr)?;
                Ok(Flow::Value(Value::Task(Box::new(v))))
            }
            Expr::Await { expr, .. } => {
                let v = self.eval_value(expr)?;
                match v {
                    Value::Task(inner) => Ok(Flow::Value(*inner)),
                    other => Ok(Flow::Value(other)),
                }
            }
            Expr::Ref { expr, is_mut, .. } => {
                let v = self.eval_value(expr)?;
                Ok(Flow::Value(Value::Ref(Box::new(v), *is_mut)))
            }
            Expr::Region { body, .. } => self.eval_block(body),
            Expr::Block(block)        => self.eval_block(block),
        }
    }

    fn resolve_name(&mut self, name: &str, span: &Span) -> LangResult<Flow> {
        if let Some(v) = self.resolve(name) {
            return Ok(Flow::Value(v));
        }
        if let Some(f) = self.functions.get(name) {
            return Ok(Flow::Value(Value::Function(f.clone())));
        }
        if let Some((enum_name, arity)) = self.enum_variants.get(name).cloned() {
            if arity == 0 {
                return Ok(Flow::Value(Value::Variant(enum_name, name.to_string(), vec![])));
            }
            return Err(error(format!("variant '{name}' takes {arity} argument(s); use {name}(...)"), span));
        }
        // Built-in functions
        let builtin = match name {
            // I/O
            "print"       => Builtin::Print,
            "println"     => Builtin::Println,
            "eprint"      => Builtin::Eprint,
            "input"       => Builtin::Input,
            // Type conversion
            "str" | "to_string" => Builtin::Str,
            "int"         => Builtin::Int_,
            "float"       => Builtin::Float_,
            "bool"        => Builtin::Bool_,
            // Math
            "abs"         => Builtin::Abs,
            "min"         => Builtin::Min,
            "max"         => Builtin::Max,
            "pow"         => Builtin::Pow,
            "round"       => Builtin::Round,
            "floor"       => Builtin::Floor,
            "ceil"        => Builtin::Ceil,
            "sqrt"        => Builtin::Sqrt,
            // Collections
            "len"         => Builtin::Len,
            "push"        => Builtin::Push,
            "pop"         => Builtin::Pop,
            "insert"      => Builtin::Insert,
            "remove"      => Builtin::Remove,
            "reverse"     => Builtin::Reverse,
            "sort"        => Builtin::Sort,
            "sorted"      => Builtin::Sorted,
            "reversed"    => Builtin::Reversed,
            "sum"         => Builtin::Sum,
            "any"         => Builtin::Any,
            "all"         => Builtin::All,
            "map"         => Builtin::Map,
            "filter"      => Builtin::Filter,
            "reduce"      => Builtin::Reduce,
            "enumerate"   => Builtin::Enumerate,
            "zip"         => Builtin::Zip,
            "range"       => Builtin::Range_,
            // String ops
            "split"       => Builtin::Split,
            "join"        => Builtin::Join,
            "upper"       => Builtin::Upper,
            "lower"       => Builtin::Lower,
            "strip"       => Builtin::Strip,
            "lstrip"      => Builtin::Lstrip,
            "rstrip"      => Builtin::Rstrip,
            "starts_with" => Builtin::StartsWith,
            "ends_with"   => Builtin::EndsWith,
            "contains"    => Builtin::Contains,
            "find"        => Builtin::Find,
            "replace"     => Builtin::Replace,
            "chars"       => Builtin::Chars,
            // Type checks
            "type_of"     => Builtin::TypeOf,
            "is_int"      => Builtin::IsInt,
            "is_float"    => Builtin::IsFloat,
            "is_string"   => Builtin::IsString,
            "is_bool"     => Builtin::IsBool,
            "is_list"     => Builtin::IsList,
            "is_dict"     => Builtin::IsDict,
            "is_none"     => Builtin::IsNone,
            // System
            "args"        => Builtin::Args,
            "getenv"      => Builtin::Getenv,
            "sleep"       => Builtin::Sleep,
            "read_file"   => Builtin::ReadFile,
            "write_file"  => Builtin::WriteFile,
            "exit"        => Builtin::Exit,
            // Panic/assert
            "panic"       => Builtin::Panic,
            "assert"      => Builtin::Assert,
            // Numeric helpers
            "hex"         => Builtin::Hex,
            "oct"         => Builtin::Oct,
            "bin"         => Builtin::Bin,
            "ord"         => Builtin::Ord,
            "chr"         => Builtin::Chr,
            "divmod"      => Builtin::DivMod,
            // Dict
            "dict"        => Builtin::Dict_,
            // Pointer/channel
            "Box"         => Builtin::BoxNew,
            "Rc"          => Builtin::RcNew,
            "Arc"         => Builtin::ArcNew,
            "Channel"     => Builtin::ChannelNew,
            "Cancel"      => Builtin::CancelNew,
            // Compat
            "is_digit"    => Builtin::IsDigit,
            "is_alpha"    => Builtin::IsAlpha,
            "is_alnum"    => Builtin::IsAlnum,
            "parse_int"   => Builtin::ParseInt,
            "string_eq"   => Builtin::StringEq,
            _ => return Err(error(format!("unknown name '{name}'"), span)),
        };
        Ok(Flow::Value(Value::Builtin(builtin)))
    }

    // ── Builtin dispatch ──────────────────────────────────────────────────────

    fn call_builtin(&mut self, builtin: Builtin, args: Vec<Value>, span: &Span) -> LangResult<Value> {
        macro_rules! check_args {
            ($n:expr) => {
                if args.len() != $n {
                    return Err(error(format!("expected {}", $n), span));
                }
            };
        }

        match builtin {
            // ── I/O ──────────────────────────────────────────────────────────
            Builtin::Print | Builtin::Println => {
                let s = args.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ");
                self.output.push_str(&s);
                self.output.push('\n');
                Ok(Value::Unit)
            }
            Builtin::Eprint => {
                match args.as_slice() {
                    [v] => { eprintln!("{v}"); Ok(Value::Unit) }
                    _ => Err(error("eprint expects 1 argument", span)),
                }
            }
            Builtin::Input => {
                let prompt = match args.as_slice() {
                    [] => String::new(),
                    [Value::String(s)] => s.clone(),
                    [v] => v.to_string(),
                    _ => return Err(error("input expects 0 or 1 argument", span)),
                };
                if !prompt.is_empty() {
                    eprint!("{prompt}");
                }
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)
                    .map_err(|e| error(format!("input error: {e}"), span))?;
                Ok(Value::String(line.trim_end_matches(['\n', '\r']).to_string()))
            }

            // ── Type conversion ───────────────────────────────────────────────
            Builtin::Str | Builtin::ToString => {
                if args.is_empty() { return Ok(Value::Builtin(builtin)); }
                Ok(Value::String(args.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ")))
            }
            Builtin::Int_ => {
                check_args!(1);
                match &args[0] {
                    Value::Int(n)    => Ok(Value::Int(*n)),
                    Value::Float(f)  => Ok(Value::Int(*f as i64)),
                    Value::Bool(b)   => Ok(Value::Int(if *b { 1 } else { 0 })),
                    Value::String(s) => s.trim().parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| error(format!("cannot convert '{}' to Int", s), span)),
                    other => Err(error(format!("cannot convert {other} to Int"), span)),
                }
            }
            Builtin::Float_ => {
                check_args!(1);
                match &args[0] {
                    Value::Float(f)  => Ok(Value::Float(*f)),
                    Value::Int(n)    => Ok(Value::Float(*n as f64)),
                    Value::Bool(b)   => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
                    Value::String(s) => s.trim().parse::<f64>()
                        .map(Value::Float)
                        .map_err(|_| error(format!("cannot convert '{}' to Float", s), span)),
                    other => Err(error(format!("cannot convert {other} to Float"), span)),
                }
            }
            Builtin::Bool_ => {
                check_args!(1);
                Ok(Value::Bool(value_is_truthy(&args[0])))
            }

            // ── Math ──────────────────────────────────────────────────────────
            Builtin::Abs => {
                check_args!(1);
                match &args[0] {
                    Value::Int(n)   => Ok(Value::Int(n.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    other => Err(error(format!("abs: expected Int or Float, got {other}"), span)),
                }
            }
            Builtin::Min => {
                if args.len() == 1 {
                    // min(list)
                    if let Value::List(items) = &args[0] {
                        let items = items.borrow();
                        if items.is_empty() {
                            return Err(error("min() on empty list", span));
                        }
                        return items.iter().cloned().try_fold(items[0].clone(), |acc, v| {
                            numeric_cmp_min(acc, v, span)
                        });
                    }
                }
                if args.len() < 2 { return Err(error("min expects 2+ arguments or a list", span)); }
                args.into_iter().try_fold(Value::Unit, |acc, v| {
                    if matches!(acc, Value::Unit) { Ok(v) } else { numeric_cmp_min(acc, v, span) }
                })
            }
            Builtin::Max => {
                if args.len() == 1 {
                    if let Value::List(items) = &args[0] {
                        let items = items.borrow();
                        if items.is_empty() {
                            return Err(error("max() on empty list", span));
                        }
                        return items.iter().cloned().try_fold(items[0].clone(), |acc, v| {
                            numeric_cmp_max(acc, v, span)
                        });
                    }
                }
                if args.len() < 2 { return Err(error("max expects 2+ arguments or a list", span)); }
                args.into_iter().try_fold(Value::Unit, |acc, v| {
                    if matches!(acc, Value::Unit) { Ok(v) } else { numeric_cmp_max(acc, v, span) }
                })
            }
            Builtin::Pow => {
                check_args!(2);
                match (&args[0], &args[1]) {
                    (Value::Int(b), Value::Int(e)) if *e >= 0 => {
                        Ok(Value::Int(b.pow(*e as u32)))
                    }
                    (Value::Int(b), Value::Int(e)) => {
                        Ok(Value::Float((*b as f64).powi(*e as i32)))
                    }
                    (Value::Float(b), Value::Int(e)) => Ok(Value::Float(b.powi(*e as i32))),
                    (Value::Float(b), Value::Float(e)) => Ok(Value::Float(b.powf(*e))),
                    (Value::Int(b), Value::Float(e)) => Ok(Value::Float((*b as f64).powf(*e))),
                    _ => Err(error("pow expects numeric arguments", span)),
                }
            }
            Builtin::Round => {
                check_args!(1);
                match &args[0] {
                    Value::Float(f) => Ok(Value::Int(f.round() as i64)),
                    Value::Int(n)   => Ok(Value::Int(*n)),
                    _ => Err(error("round expects Float or Int", span)),
                }
            }
            Builtin::Floor => {
                check_args!(1);
                match &args[0] {
                    Value::Float(f) => Ok(Value::Int(f.floor() as i64)),
                    Value::Int(n)   => Ok(Value::Int(*n)),
                    _ => Err(error("floor expects Float or Int", span)),
                }
            }
            Builtin::Ceil => {
                check_args!(1);
                match &args[0] {
                    Value::Float(f) => Ok(Value::Int(f.ceil() as i64)),
                    Value::Int(n)   => Ok(Value::Int(*n)),
                    _ => Err(error("ceil expects Float or Int", span)),
                }
            }
            Builtin::Sqrt => {
                check_args!(1);
                match &args[0] {
                    Value::Float(f) => Ok(Value::Float(f.sqrt())),
                    Value::Int(n)   => Ok(Value::Float((*n as f64).sqrt())),
                    _ => Err(error("sqrt expects Float or Int", span)),
                }
            }

            // ── Collections ───────────────────────────────────────────────────
            Builtin::Len => {
                check_args!(1);
                match &args[0] {
                    Value::List(items) => Ok(Value::Int(items.borrow().len() as i64)),
                    Value::String(s)   => Ok(Value::Int(s.chars().count() as i64)),
                    Value::Dict(map)   => Ok(Value::Int(map.borrow().len() as i64)),
                    Value::Range(a, b) => Ok(Value::Int((b - a).max(0))),
                    other => Err(error(format!("len: unsupported type {other}"), span)),
                }
            }
            Builtin::Push => {
                if args.len() != 2 { return Err(error("push expects 2 arguments", span)); }
                match &args[0] {
                    Value::List(items) => {
                        items.borrow_mut().push(args[1].clone());
                        Ok(Value::Unit)
                    }
                    other => Err(error(format!("push: expected List, got {other}"), span)),
                }
            }
            Builtin::Pop => {
                check_args!(1);
                match &args[0] {
                    Value::List(items) => {
                        let v = items.borrow_mut().pop();
                        Ok(match v {
                            Some(v) => Value::Variant("Option".into(), "Some".into(), vec![v]),
                            None    => Value::Variant("Option".into(), "None".into(), vec![]),
                        })
                    }
                    other => Err(error(format!("pop: expected List, got {other}"), span)),
                }
            }
            Builtin::Insert => {
                if args.len() != 3 { return Err(error("insert expects 3 arguments (list, idx, value)", span)); }
                match (&args[0], &args[1]) {
                    (Value::List(items), Value::Int(idx)) => {
                        let mut items = items.borrow_mut();
                        let len = items.len() as i64;
                        let real = if *idx < 0 { (len + idx).max(0) } else { (*idx).min(len) };
                        items.insert(real as usize, args[2].clone());
                        Ok(Value::Unit)
                    }
                    _ => Err(error("insert: expected (List, Int, value)", span)),
                }
            }
            Builtin::Remove => {
                if args.len() != 2 { return Err(error("remove expects 2 arguments (list, idx)", span)); }
                match (&args[0], &args[1]) {
                    (Value::List(items), Value::Int(idx)) => {
                        let mut items = items.borrow_mut();
                        let len = items.len() as i64;
                        let real = if *idx < 0 { len + idx } else { *idx };
                        if real < 0 || real >= len {
                            return Err(error(format!("remove: index {idx} out of range"), span));
                        }
                        Ok(items.remove(real as usize))
                    }
                    _ => Err(error("remove: expected (List, Int)", span)),
                }
            }
            Builtin::Reverse => {
                check_args!(1);
                match &args[0] {
                    Value::List(items) => { items.borrow_mut().reverse(); Ok(Value::Unit) }
                    other => Err(error(format!("reverse: expected List, got {other}"), span)),
                }
            }
            Builtin::Sort => {
                check_args!(1);
                match &args[0] {
                    Value::List(items) => {
                        items.borrow_mut().sort_by(compare_values);
                        Ok(Value::Unit)
                    }
                    other => Err(error(format!("sort: expected List, got {other}"), span)),
                }
            }
            Builtin::Sorted => {
                check_args!(1);
                match &args[0] {
                    Value::List(items) => {
                        let mut copy = items.borrow().clone();
                        copy.sort_by(compare_values);
                        Ok(Value::List(Rc::new(RefCell::new(copy))))
                    }
                    other => Err(error(format!("sorted: expected List, got {other}"), span)),
                }
            }
            Builtin::Reversed => {
                check_args!(1);
                match &args[0] {
                    Value::List(items) => {
                        let mut copy = items.borrow().clone();
                        copy.reverse();
                        Ok(Value::List(Rc::new(RefCell::new(copy))))
                    }
                    Value::Range(a, b) => {
                        let v: Vec<Value> = (*a..*b).rev().map(Value::Int).collect();
                        Ok(Value::List(Rc::new(RefCell::new(v))))
                    }
                    other => Err(error(format!("reversed: expected List, got {other}"), span)),
                }
            }
            Builtin::Sum => {
                check_args!(1);
                match &args[0] {
                    Value::List(items) => {
                        let items = items.borrow();
                        let mut total = Value::Int(0);
                        for v in items.iter() {
                            total = add_values(total, v.clone(), span)?;
                        }
                        Ok(total)
                    }
                    other => Err(error(format!("sum: expected List, got {other}"), span)),
                }
            }
            Builtin::Any => {
                check_args!(1);
                match &args[0] {
                    Value::List(items) => {
                        Ok(Value::Bool(items.borrow().iter().any(value_is_truthy)))
                    }
                    other => Err(error(format!("any: expected List, got {other}"), span)),
                }
            }
            Builtin::All => {
                check_args!(1);
                match &args[0] {
                    Value::List(items) => {
                        Ok(Value::Bool(items.borrow().iter().all(value_is_truthy)))
                    }
                    other => Err(error(format!("all: expected List, got {other}"), span)),
                }
            }
            Builtin::Map => {
                if args.len() != 2 { return Err(error("map expects (fn, list)", span)); }
                let func = args[0].clone();
                match &args[1] {
                    Value::List(items) => {
                        let items = items.borrow().clone();
                        let mut results = Vec::with_capacity(items.len());
                        for item in items {
                            results.push(self.call_value(func.clone(), vec![item], span)?);
                        }
                        Ok(Value::List(Rc::new(RefCell::new(results))))
                    }
                    other => Err(error(format!("map: expected List, got {other}"), span)),
                }
            }
            Builtin::Filter => {
                if args.len() != 2 { return Err(error("filter expects (fn, list)", span)); }
                let func = args[0].clone();
                match &args[1] {
                    Value::List(items) => {
                        let items = items.borrow().clone();
                        let mut results = Vec::new();
                        for item in items {
                            let keep = self.call_value(func.clone(), vec![item.clone()], span)?;
                            if value_is_truthy(&keep) {
                                results.push(item);
                            }
                        }
                        Ok(Value::List(Rc::new(RefCell::new(results))))
                    }
                    other => Err(error(format!("filter: expected List, got {other}"), span)),
                }
            }
            Builtin::Reduce => {
                if args.len() != 2 { return Err(error("reduce expects (fn, list)", span)); }
                let func = args[0].clone();
                match &args[1] {
                    Value::List(items) => {
                        let items = items.borrow().clone();
                        if items.is_empty() {
                            return Err(error("reduce on empty list", span));
                        }
                        let mut acc = items[0].clone();
                        for item in items.into_iter().skip(1) {
                            acc = self.call_value(func.clone(), vec![acc, item], span)?;
                        }
                        Ok(acc)
                    }
                    other => Err(error(format!("reduce: expected List, got {other}"), span)),
                }
            }
            Builtin::Enumerate => {
                check_args!(1);
                match &args[0] {
                    Value::List(items) => {
                        let items = items.borrow();
                        let result: Vec<Value> = items.iter().enumerate().map(|(i, v)| {
                            Value::List(Rc::new(RefCell::new(vec![Value::Int(i as i64), v.clone()])))
                        }).collect();
                        Ok(Value::List(Rc::new(RefCell::new(result))))
                    }
                    Value::Range(a, b) => {
                        let result: Vec<Value> = (*a..*b).enumerate().map(|(i, n)| {
                            Value::List(Rc::new(RefCell::new(vec![Value::Int(i as i64), Value::Int(n)])))
                        }).collect();
                        Ok(Value::List(Rc::new(RefCell::new(result))))
                    }
                    other => Err(error(format!("enumerate: expected List or Range, got {other}"), span)),
                }
            }
            Builtin::Zip => {
                if args.len() != 2 { return Err(error("zip expects 2 lists", span)); }
                match (&args[0], &args[1]) {
                    (Value::List(a), Value::List(b)) => {
                        let a = a.borrow(); let b = b.borrow();
                        let result: Vec<Value> = a.iter().zip(b.iter()).map(|(x, y)| {
                            Value::List(Rc::new(RefCell::new(vec![x.clone(), y.clone()])))
                        }).collect();
                        Ok(Value::List(Rc::new(RefCell::new(result))))
                    }
                    _ => Err(error("zip expects 2 Lists", span)),
                }
            }
            Builtin::Range_ => {
                match args.as_slice() {
                    [Value::Int(n)] => {
                        let v: Vec<Value> = (0..*n).map(Value::Int).collect();
                        Ok(Value::List(Rc::new(RefCell::new(v))))
                    }
                    [Value::Int(a), Value::Int(b)] => {
                        let v: Vec<Value> = (*a..*b).map(Value::Int).collect();
                        Ok(Value::List(Rc::new(RefCell::new(v))))
                    }
                    [Value::Int(a), Value::Int(b), Value::Int(step)] => {
                        if *step == 0 { return Err(error("range step cannot be 0", span)); }
                        let mut v = Vec::new();
                        let mut i = *a;
                        if *step > 0 { while i < *b { v.push(Value::Int(i)); i += step; } }
                        else         { while i > *b { v.push(Value::Int(i)); i += step; } }
                        Ok(Value::List(Rc::new(RefCell::new(v))))
                    }
                    _ => Err(error("range expects (n), (start, end), or (start, end, step)", span)),
                }
            }
            Builtin::RangeStep => {
                // handled above in Range_
                Err(error("internal: unexpected RangeStep", span))
            }

            // ── String operations ─────────────────────────────────────────────
            Builtin::Split => {
                match args.as_slice() {
                    [Value::String(s), Value::String(delim)] => {
                        let parts: Vec<Value> = if delim.is_empty() {
                            s.chars().map(|c| Value::String(c.to_string())).collect()
                        } else {
                            s.split(delim.as_str()).map(|p| Value::String(p.to_string())).collect()
                        };
                        Ok(Value::List(Rc::new(RefCell::new(parts))))
                    }
                    [Value::String(s)] => {
                        let parts: Vec<Value> = s.split_whitespace()
                            .map(|p| Value::String(p.to_string()))
                            .collect();
                        Ok(Value::List(Rc::new(RefCell::new(parts))))
                    }
                    _ => Err(error("split expects (str) or (str, delim)", span)),
                }
            }
            Builtin::Join => {
                match args.as_slice() {
                    [Value::List(items), Value::String(sep)] => {
                        let parts: Vec<String> = items.borrow().iter()
                            .map(|v| v.to_string()).collect();
                        Ok(Value::String(parts.join(sep)))
                    }
                    [Value::List(items)] => {
                        let parts: Vec<String> = items.borrow().iter()
                            .map(|v| v.to_string()).collect();
                        Ok(Value::String(parts.join("")))
                    }
                    _ => Err(error("join expects (list) or (list, sep)", span)),
                }
            }
            Builtin::Upper => {
                check_args!(1);
                match &args[0] {
                    Value::String(s) => Ok(Value::String(s.to_uppercase())),
                    other => Err(error(format!("upper: expected String, got {other}"), span)),
                }
            }
            Builtin::Lower => {
                check_args!(1);
                match &args[0] {
                    Value::String(s) => Ok(Value::String(s.to_lowercase())),
                    other => Err(error(format!("lower: expected String, got {other}"), span)),
                }
            }
            Builtin::Strip => {
                check_args!(1);
                match &args[0] {
                    Value::String(s) => Ok(Value::String(s.trim().to_string())),
                    other => Err(error(format!("strip: expected String, got {other}"), span)),
                }
            }
            Builtin::Lstrip => {
                check_args!(1);
                match &args[0] {
                    Value::String(s) => Ok(Value::String(s.trim_start().to_string())),
                    other => Err(error(format!("lstrip: expected String, got {other}"), span)),
                }
            }
            Builtin::Rstrip => {
                check_args!(1);
                match &args[0] {
                    Value::String(s) => Ok(Value::String(s.trim_end().to_string())),
                    other => Err(error(format!("rstrip: expected String, got {other}"), span)),
                }
            }
            Builtin::StartsWith => {
                if args.len() != 2 { return Err(error("starts_with expects (str, prefix)", span)); }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(prefix)) => Ok(Value::Bool(s.starts_with(prefix.as_str()))),
                    _ => Err(error("starts_with expects 2 Strings", span)),
                }
            }
            Builtin::EndsWith => {
                if args.len() != 2 { return Err(error("ends_with expects (str, suffix)", span)); }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(suffix)) => Ok(Value::Bool(s.ends_with(suffix.as_str()))),
                    _ => Err(error("ends_with expects 2 Strings", span)),
                }
            }
            Builtin::Contains => {
                if args.len() != 2 { return Err(error("contains expects (str, sub)", span)); }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(sub)) => Ok(Value::Bool(s.contains(sub.as_str()))),
                    (Value::List(items), v) => Ok(Value::Bool(items.borrow().iter().any(|x| values_equal(x, v)))),
                    _ => Err(error("contains expects (String, String) or (List, value)", span)),
                }
            }
            Builtin::Find => {
                if args.len() != 2 { return Err(error("find expects (str, sub)", span)); }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(sub)) => {
                        Ok(match s.find(sub.as_str()) {
                            Some(idx) => Value::Variant("Option".into(), "Some".into(),
                                vec![Value::Int(idx as i64)]),
                            None => Value::Variant("Option".into(), "None".into(), vec![]),
                        })
                    }
                    _ => Err(error("find expects (String, String)", span)),
                }
            }
            Builtin::Replace => {
                if args.len() != 3 { return Err(error("replace expects (str, old, new)", span)); }
                match (&args[0], &args[1], &args[2]) {
                    (Value::String(s), Value::String(old), Value::String(new)) => {
                        Ok(Value::String(s.replace(old.as_str(), new.as_str())))
                    }
                    _ => Err(error("replace expects 3 Strings", span)),
                }
            }
            Builtin::Chars => {
                check_args!(1);
                match &args[0] {
                    Value::String(s) => {
                        let chars: Vec<Value> = s.chars().map(|c| Value::String(c.to_string())).collect();
                        Ok(Value::List(Rc::new(RefCell::new(chars))))
                    }
                    other => Err(error(format!("chars: expected String, got {other}"), span)),
                }
            }

            // ── Type checks ───────────────────────────────────────────────────
            Builtin::TypeOf => {
                check_args!(1);
                let name = match &args[0] {
                    Value::Int(_)    => "Int",
                    Value::Float(_)  => "Float",
                    Value::Bool(_)   => "Bool",
                    Value::String(_) => "String",
                    Value::Unit      => "none",
                    Value::List(_)   => "List",
                    Value::Dict(_)   => "Dict",
                    Value::Struct(n, _) => n,
                    Value::Variant(_, _, _) => "Variant",
                    Value::Function(_) => "Function",
                    Value::Lambda(_) => "Lambda",
                    _ => "Unknown",
                };
                Ok(Value::String(name.to_string()))
            }
            Builtin::IsInt    => { check_args!(1); Ok(Value::Bool(matches!(args[0], Value::Int(_)))) }
            Builtin::IsFloat  => { check_args!(1); Ok(Value::Bool(matches!(args[0], Value::Float(_)))) }
            Builtin::IsString => { check_args!(1); Ok(Value::Bool(matches!(args[0], Value::String(_)))) }
            Builtin::IsBool   => { check_args!(1); Ok(Value::Bool(matches!(args[0], Value::Bool(_)))) }
            Builtin::IsList   => { check_args!(1); Ok(Value::Bool(matches!(args[0], Value::List(_)))) }
            Builtin::IsDict   => { check_args!(1); Ok(Value::Bool(matches!(args[0], Value::Dict(_)))) }
            Builtin::IsNone   => { check_args!(1); Ok(Value::Bool(matches!(args[0], Value::Unit))) }

            // ── System ────────────────────────────────────────────────────────
            Builtin::Args => {
                let argv: Vec<Value> = std::env::args().skip(1).map(Value::String).collect();
                Ok(Value::List(Rc::new(RefCell::new(argv))))
            }
            Builtin::Getenv => match args.as_slice() {
                [Value::String(key)] => Ok(match std::env::var(key) {
                    Ok(v) => Value::Variant("Option".into(), "Some".into(), vec![Value::String(v)]),
                    Err(_) => Value::Variant("Option".into(), "None".into(), vec![]),
                }),
                _ => Err(error("getenv expects 1 String argument", span)),
            },
            Builtin::Sleep => match args.as_slice() {
                [Value::Int(ms)] => {
                    std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
                    Ok(Value::Unit)
                }
                _ => Err(error("sleep expects 1 Int (milliseconds)", span)),
            },
            Builtin::ReadFile => match args.as_slice() {
                [Value::String(path)] => Ok(match std::fs::read_to_string(path) {
                    Ok(c) => Value::Variant("Result".into(), "Ok".into(), vec![Value::String(c)]),
                    Err(e) => Value::Variant("Result".into(), "Err".into(), vec![Value::String(e.to_string())]),
                }),
                _ => Err(error("read_file expects 1 String (path)", span)),
            },
            Builtin::WriteFile => match args.as_slice() {
                [Value::String(path), Value::String(content)] => Ok(match std::fs::write(path, content) {
                    Ok(()) => Value::Variant("Result".into(), "Ok".into(), vec![Value::Unit]),
                    Err(e) => Value::Variant("Result".into(), "Err".into(), vec![Value::String(e.to_string())]),
                }),
                _ => Err(error("write_file expects (path: String, content: String)", span)),
            },
            Builtin::Exit => {
                let code = match args.as_slice() {
                    [] => 0,
                    [Value::Int(n)] => *n as i32,
                    _ => return Err(error("exit expects 0 or 1 Int argument", span)),
                };
                std::process::exit(code);
            }

            // ── Panic / assert ────────────────────────────────────────────────
            Builtin::Panic => match args.as_slice() {
                [v] => Err(error(format!("panic: {v}"), span)),
                _   => Err(error("panic expects 1 argument", span)),
            },
            Builtin::Assert => {
                match args.as_slice() {
                    [cond] => {
                        if !value_is_truthy(cond) {
                            return Err(error("assertion failed", span));
                        }
                        Ok(Value::Unit)
                    }
                    [cond, Value::String(msg)] => {
                        if !value_is_truthy(cond) {
                            return Err(error(format!("assertion failed: {msg}"), span));
                        }
                        Ok(Value::Unit)
                    }
                    _ => Err(error("assert expects (cond) or (cond, msg)", span)),
                }
            }

            // ── Numeric helpers ───────────────────────────────────────────────
            Builtin::Hex => match args.as_slice() {
                [Value::Int(n)] => Ok(Value::String(format!("0x{:x}", n))),
                _ => Err(error("hex expects 1 Int", span)),
            },
            Builtin::Oct => match args.as_slice() {
                [Value::Int(n)] => Ok(Value::String(format!("0o{:o}", n))),
                _ => Err(error("oct expects 1 Int", span)),
            },
            Builtin::Bin => match args.as_slice() {
                [Value::Int(n)] => Ok(Value::String(format!("0b{:b}", n))),
                _ => Err(error("bin expects 1 Int", span)),
            },
            Builtin::Ord => match args.as_slice() {
                [Value::String(s)] if s.chars().count() == 1 => {
                    Ok(Value::Int(s.chars().next().unwrap() as i64))
                }
                _ => Err(error("ord expects a single-character String", span)),
            },
            Builtin::Chr => match args.as_slice() {
                [Value::Int(n)] => {
                    let c = char::from_u32(*n as u32)
                        .ok_or_else(|| error(format!("chr: {n} is not a valid codepoint"), span))?;
                    Ok(Value::String(c.to_string()))
                }
                _ => Err(error("chr expects 1 Int", span)),
            },
            Builtin::DivMod => match args.as_slice() {
                [Value::Int(a), Value::Int(b)] => {
                    if *b == 0 { return Err(error("divmod: division by zero", span)); }
                    Ok(Value::List(Rc::new(RefCell::new(vec![
                        Value::Int(a / b),
                        Value::Int(a % b),
                    ]))))
                }
                _ => Err(error("divmod expects 2 Ints", span)),
            },

            // ── Dict ──────────────────────────────────────────────────────────
            Builtin::Dict_ => {
                Ok(Value::Dict(Rc::new(RefCell::new(HashMap::new()))))
            }

            // ── Pointer / channel ─────────────────────────────────────────────
            Builtin::BoxNew | Builtin::RcNew | Builtin::ArcNew => {
                check_args!(1);
                let kind = match builtin {
                    Builtin::BoxNew => "Box",
                    Builtin::RcNew  => "Rc",
                    _               => "Arc",
                };
                Ok(Value::Pointer(kind.to_string(), Rc::new(RefCell::new(args.into_iter().next().unwrap()))))
            }
            Builtin::ChannelNew => {
                let cap = match args.as_slice() {
                    [] => None,
                    [Value::Int(n)] if *n >= 0 => Some(*n as usize),
                    _ => return Err(error("Channel expects 0 or 1 non-negative Int argument", span)),
                };
                Ok(Value::Channel(Rc::new(ChannelInner {
                    capacity: cap,
                    queue: RefCell::new(VecDeque::new()),
                    closed: RefCell::new(false),
                })))
            }
            Builtin::CancelNew => {
                if !args.is_empty() { return Err(error("Cancel() takes no arguments", span)); }
                Ok(Value::Cancel(Rc::new(RefCell::new(false))))
            }

            // ── Compat ────────────────────────────────────────────────────────
            Builtin::IsDigit => match args.as_slice() {
                [Value::String(s)] => Ok(Value::Bool(
                    s.len() == 1 && s.chars().next().unwrap().is_ascii_digit())),
                _ => Err(error("is_digit expects 1 String", span)),
            },
            Builtin::IsAlpha => match args.as_slice() {
                [Value::String(s)] => Ok(Value::Bool(
                    s.len() == 1 && (s.chars().next().unwrap().is_ascii_alphabetic()
                        || s.chars().next().unwrap() == '_'))),
                _ => Err(error("is_alpha expects 1 String", span)),
            },
            Builtin::IsAlnum => match args.as_slice() {
                [Value::String(s)] => Ok(Value::Bool(s.len() == 1 && {
                    let c = s.chars().next().unwrap();
                    c.is_ascii_alphanumeric() || c == '_'
                })),
                _ => Err(error("is_alnum expects 1 String", span)),
            },
            Builtin::ParseInt => match args.as_slice() {
                [Value::String(s)] => Ok(match s.trim().parse::<i64>() {
                    Ok(n) => Value::Variant("Option".into(), "Some".into(), vec![Value::Int(n)]),
                    Err(_) => Value::Variant("Option".into(), "None".into(), vec![]),
                }),
                _ => Err(error("parse_int expects 1 String", span)),
            },
            Builtin::StringEq => match args.as_slice() {
                [Value::String(a), Value::String(b)] => Ok(Value::Bool(a == b)),
                _ => Err(error("string_eq expects 2 Strings", span)),
            },
        }
    }

    // ── Method dispatch ───────────────────────────────────────────────────────

    fn eval_method_call(
        &mut self, target: &Expr, name: &str, args: &[Expr], span: &Span,
    ) -> LangResult<Flow> {
        let tv = self.eval_value(target)?;
        let mut arg_vals = Vec::with_capacity(args.len() + 1);
        arg_vals.push(tv.clone());
        for arg in args {
            match self.eval_expr(arg)? {
                Flow::Value(v) => arg_vals.push(v),
                ctrl => return Ok(ctrl),
            }
        }

        match (&tv, name) {
            // List methods
            (Value::List(items), "len") => return Ok(Flow::Value(Value::Int(items.borrow().len() as i64))),
            (Value::List(items), "push") => {
                if arg_vals.len() != 2 { return Err(error("push expects 1 argument", span)); }
                items.borrow_mut().push(arg_vals[1].clone());
                return Ok(Flow::Value(Value::Unit));
            }
            (Value::List(items), "pop") => {
                let v = items.borrow_mut().pop();
                return Ok(Flow::Value(match v {
                    Some(v) => Value::Variant("Option".into(), "Some".into(), vec![v]),
                    None    => Value::Variant("Option".into(), "None".into(), vec![]),
                }));
            }
            (Value::List(items), "contains") => {
                if arg_vals.len() != 2 { return Err(error("contains expects 1 argument", span)); }
                let found = items.borrow().iter().any(|x| values_equal(x, &arg_vals[1]));
                return Ok(Flow::Value(Value::Bool(found)));
            }
            (Value::List(items), "index") | (Value::List(items), "find") => {
                if arg_vals.len() != 2 { return Err(error("index expects 1 argument", span)); }
                let pos = items.borrow().iter().position(|x| values_equal(x, &arg_vals[1]));
                return Ok(Flow::Value(match pos {
                    Some(i) => Value::Variant("Option".into(), "Some".into(), vec![Value::Int(i as i64)]),
                    None => Value::Variant("Option".into(), "None".into(), vec![]),
                }));
            }
            (Value::List(items), "sort") => {
                items.borrow_mut().sort_by(compare_values);
                return Ok(Flow::Value(Value::Unit));
            }
            (Value::List(items), "reverse") => {
                items.borrow_mut().reverse();
                return Ok(Flow::Value(Value::Unit));
            }
            (Value::List(items), "clear") => {
                items.borrow_mut().clear();
                return Ok(Flow::Value(Value::Unit));
            }
            (Value::List(items), "extend") => {
                if arg_vals.len() != 2 { return Err(error("extend expects 1 argument", span)); }
                if let Value::List(other) = &arg_vals[1] {
                    let other_clone = other.borrow().clone();
                    items.borrow_mut().extend(other_clone);
                    return Ok(Flow::Value(Value::Unit));
                }
                return Err(error("extend expects a List argument", span));
            }
            (Value::List(items), "count") => {
                if arg_vals.len() != 2 { return Err(error("count expects 1 argument", span)); }
                let n = items.borrow().iter().filter(|x| values_equal(x, &arg_vals[1])).count();
                return Ok(Flow::Value(Value::Int(n as i64)));
            }
            (Value::List(items), "join") => {
                let sep = if arg_vals.len() == 2 {
                    match &arg_vals[1] { Value::String(s) => s.clone(), v => v.to_string() }
                } else { String::new() };
                let parts: Vec<String> = items.borrow().iter().map(|v| v.to_string()).collect();
                return Ok(Flow::Value(Value::String(parts.join(&sep))));
            }
            (Value::List(items), "copy") => {
                return Ok(Flow::Value(Value::List(Rc::new(RefCell::new(items.borrow().clone())))));
            }
            // String methods
            (Value::String(s), "len") => return Ok(Flow::Value(Value::Int(s.chars().count() as i64))),
            (Value::String(s), "upper") => return Ok(Flow::Value(Value::String(s.to_uppercase()))),
            (Value::String(s), "lower") => return Ok(Flow::Value(Value::String(s.to_lowercase()))),
            (Value::String(s), "strip" | "trim") => return Ok(Flow::Value(Value::String(s.trim().to_string()))),
            (Value::String(s), "split") => {
                let parts: Vec<Value> = if arg_vals.len() == 2 {
                    match &arg_vals[1] {
                        Value::String(delim) => s.split(delim.as_str()).map(|p| Value::String(p.to_string())).collect(),
                        _ => return Err(error("split expects String delimiter", span)),
                    }
                } else {
                    s.split_whitespace().map(|p| Value::String(p.to_string())).collect()
                };
                return Ok(Flow::Value(Value::List(Rc::new(RefCell::new(parts)))));
            }
            (Value::String(s), "replace") => {
                if arg_vals.len() != 3 { return Err(error("replace expects (old, new)", span)); }
                match (&arg_vals[1], &arg_vals[2]) {
                    (Value::String(old), Value::String(new)) => {
                        return Ok(Flow::Value(Value::String(s.replace(old.as_str(), new.as_str()))));
                    }
                    _ => return Err(error("replace expects 2 String arguments", span)),
                }
            }
            (Value::String(s), "starts_with") => {
                if arg_vals.len() != 2 { return Err(error("starts_with expects 1 argument", span)); }
                if let Value::String(prefix) = &arg_vals[1] {
                    return Ok(Flow::Value(Value::Bool(s.starts_with(prefix.as_str()))));
                }
                return Err(error("starts_with expects String argument", span));
            }
            (Value::String(s), "ends_with") => {
                if arg_vals.len() != 2 { return Err(error("ends_with expects 1 argument", span)); }
                if let Value::String(suffix) = &arg_vals[1] {
                    return Ok(Flow::Value(Value::Bool(s.ends_with(suffix.as_str()))));
                }
                return Err(error("ends_with expects String argument", span));
            }
            (Value::String(s), "contains") => {
                if arg_vals.len() != 2 { return Err(error("contains expects 1 argument", span)); }
                if let Value::String(sub) = &arg_vals[1] {
                    return Ok(Flow::Value(Value::Bool(s.contains(sub.as_str()))));
                }
                return Err(error("contains expects String argument", span));
            }
            (Value::String(s), "chars") => {
                let chars: Vec<Value> = s.chars().map(|c| Value::String(c.to_string())).collect();
                return Ok(Flow::Value(Value::List(Rc::new(RefCell::new(chars)))));
            }
            (Value::String(s), "to_string" | "clone") => {
                return Ok(Flow::Value(Value::String(s.clone())));
            }
            (Value::String(s), "parse_int" | "to_int") => {
                return Ok(Flow::Value(match s.trim().parse::<i64>() {
                    Ok(n) => Value::Variant("Option".into(), "Some".into(), vec![Value::Int(n)]),
                    Err(_) => Value::Variant("Option".into(), "None".into(), vec![]),
                }));
            }
            (Value::String(s), "parse_float" | "to_float") => {
                return Ok(Flow::Value(match s.trim().parse::<f64>() {
                    Ok(f) => Value::Variant("Option".into(), "Some".into(), vec![Value::Float(f)]),
                    Err(_) => Value::Variant("Option".into(), "None".into(), vec![]),
                }));
            }
            // Int methods
            (Value::Int(n), "to_string") => return Ok(Flow::Value(Value::String(n.to_string()))),
            (Value::Int(n), "abs") => return Ok(Flow::Value(Value::Int(n.abs()))),
            (Value::Int(n), "to_float") => return Ok(Flow::Value(Value::Float(*n as f64))),
            // Float methods
            (Value::Float(f), "to_string") => return Ok(Flow::Value(Value::String(f.to_string()))),
            (Value::Float(f), "round") => return Ok(Flow::Value(Value::Int(f.round() as i64))),
            (Value::Float(f), "floor") => return Ok(Flow::Value(Value::Int(f.floor() as i64))),
            (Value::Float(f), "ceil") => return Ok(Flow::Value(Value::Int(f.ceil() as i64))),
            (Value::Float(f), "sqrt") => return Ok(Flow::Value(Value::Float(f.sqrt()))),
            (Value::Float(f), "abs") => return Ok(Flow::Value(Value::Float(f.abs()))),
            // Dict methods
            (Value::Dict(map), "get") => {
                if arg_vals.len() != 2 { return Err(error("get expects 1 argument", span)); }
                let key = match &arg_vals[1] {
                    Value::String(s) => s.clone(),
                    v => v.to_string(),
                };
                let v = map.borrow().get(&key).cloned().unwrap_or(Value::Unit);
                return Ok(Flow::Value(v));
            }
            (Value::Dict(map), "set") => {
                if arg_vals.len() != 3 { return Err(error("set expects 2 arguments (key, value)", span)); }
                let key = match &arg_vals[1] {
                    Value::String(s) => s.clone(),
                    v => v.to_string(),
                };
                map.borrow_mut().insert(key, arg_vals[2].clone());
                return Ok(Flow::Value(Value::Unit));
            }
            (Value::Dict(map), "contains" | "has_key") => {
                if arg_vals.len() != 2 { return Err(error("contains expects 1 argument", span)); }
                let key = match &arg_vals[1] { Value::String(s) => s.clone(), v => v.to_string() };
                return Ok(Flow::Value(Value::Bool(map.borrow().contains_key(&key))));
            }
            (Value::Dict(map), "remove" | "delete") => {
                if arg_vals.len() != 2 { return Err(error("remove expects 1 argument", span)); }
                let key = match &arg_vals[1] { Value::String(s) => s.clone(), v => v.to_string() };
                map.borrow_mut().remove(&key);
                return Ok(Flow::Value(Value::Unit));
            }
            (Value::Dict(map), "keys") => {
                let keys: Vec<Value> = map.borrow().keys()
                    .map(|k| Value::String(k.clone())).collect();
                return Ok(Flow::Value(Value::List(Rc::new(RefCell::new(keys)))));
            }
            (Value::Dict(map), "values") => {
                let vals: Vec<Value> = map.borrow().values().cloned().collect();
                return Ok(Flow::Value(Value::List(Rc::new(RefCell::new(vals)))));
            }
            (Value::Dict(map), "items") => {
                let items: Vec<Value> = map.borrow().iter().map(|(k, v)| {
                    Value::List(Rc::new(RefCell::new(vec![Value::String(k.clone()), v.clone()])))
                }).collect();
                return Ok(Flow::Value(Value::List(Rc::new(RefCell::new(items)))));
            }
            (Value::Dict(map), "len") => return Ok(Flow::Value(Value::Int(map.borrow().len() as i64))),
            (Value::Dict(map), "clear") => { map.borrow_mut().clear(); return Ok(Flow::Value(Value::Unit)); }
            // Channel methods
            (Value::Channel(chan), meth) => {
                return self.dispatch_channel(chan, meth, &arg_vals, span);
            }
            (Value::Cancel(state), meth) => {
                return self.dispatch_cancel(state, meth, &arg_vals, span);
            }
            (Value::Pointer(_, cell), "get") => return Ok(Flow::Value(cell.borrow().clone())),
            (Value::Pointer(_, cell), "set") => {
                if arg_vals.len() != 2 { return Err(error("set expects 1 argument", span)); }
                *cell.borrow_mut() = arg_vals[1].clone();
                return Ok(Flow::Value(Value::Unit));
            }
            (Value::Struct(sname, _), _) => {
                if let Some(method) = self.methods.get(sname).and_then(|m| m.get(name)).cloned() {
                    let v = self.call_function(&method, arg_vals, span)?;
                    return Ok(Flow::Value(v));
                }
            }
            _ => {}
        }

        Err(error(format!("no method '{name}' on {tv}"), span))
    }

    fn dispatch_channel(&mut self, chan: &Rc<ChannelInner>, method: &str, args: &[Value], span: &Span) -> LangResult<Flow> {
        match method {
            "send" => {
                if args.len() != 2 { return Err(error("Channel.send expects 1 argument", span)); }
                if *chan.closed.borrow() { return Err(error("send on closed channel", span)); }
                chan.queue.borrow_mut().push_back(args[1].clone());
                Ok(Flow::Value(Value::Unit))
            }
            "recv" | "try_recv" => {
                let v = chan.queue.borrow_mut().pop_front();
                Ok(Flow::Value(match v {
                    Some(v) => Value::Variant("Option".into(), "Some".into(), vec![v]),
                    None    => Value::Variant("Option".into(), "None".into(), vec![]),
                }))
            }
            "len"      => Ok(Flow::Value(Value::Int(chan.queue.borrow().len() as i64))),
            "is_empty" => Ok(Flow::Value(Value::Bool(chan.queue.borrow().is_empty()))),
            "close"    => { *chan.closed.borrow_mut() = true; Ok(Flow::Value(Value::Unit)) }
            "capacity" => Ok(Flow::Value(match chan.capacity {
                Some(c) => Value::Variant("Option".into(), "Some".into(), vec![Value::Int(c as i64)]),
                None    => Value::Variant("Option".into(), "None".into(), vec![]),
            })),
            other => Err(error(format!("no method '{other}' on Channel"), span)),
        }
    }

    fn dispatch_cancel(&mut self, state: &Rc<RefCell<bool>>, method: &str, _args: &[Value], span: &Span) -> LangResult<Flow> {
        match method {
            "signal" | "cancel" => { *state.borrow_mut() = true; Ok(Flow::Value(Value::Unit)) }
            "is_cancelled" | "is_signalled" => Ok(Flow::Value(Value::Bool(*state.borrow()))),
            other => Err(error(format!("no method '{other}' on Cancel"), span)),
        }
    }

    // ── Binary / unary ────────────────────────────────────────────────────────

    fn eval_binary(&mut self, left: &Expr, op: BinaryOp, right: &Expr, span: &Span) -> LangResult<Flow> {
        if op == BinaryOp::And {
            let lv = self.eval_value(left)?;
            if !value_is_truthy(&lv) { return Ok(Flow::Value(Value::Bool(false))); }
            let rv = self.eval_value(right)?;
            return Ok(Flow::Value(Value::Bool(value_is_truthy(&rv))));
        }
        if op == BinaryOp::Or {
            let lv = self.eval_value(left)?;
            if value_is_truthy(&lv) { return Ok(Flow::Value(lv)); }
            let rv = self.eval_value(right)?;
            return Ok(Flow::Value(rv));
        }
        let lv = match self.eval_expr(left)?  { Flow::Value(v) => v, c => return Ok(c) };
        let rv = match self.eval_expr(right)? { Flow::Value(v) => v, c => return Ok(c) };
        Ok(Flow::Value(apply_binary(lv, op, rv, span)?))
    }

    fn eval_pipeline(&mut self, left: &Expr, right: &Expr, span: &Span) -> LangResult<Flow> {
        let lv = match self.eval_expr(left)? { Flow::Value(v) => v, c => return Ok(c) };
        match right {
            Expr::Call { callee, args, .. } => {
                let callee_v = match self.eval_expr(callee)? { Flow::Value(v) => v, c => return Ok(c) };
                let mut vals = vec![lv];
                vals.extend(self.eval_args(args)?);
                Ok(Flow::Value(self.call_value(callee_v, vals, span)?))
            }
            _ => {
                let callee_v = match self.eval_expr(right)? { Flow::Value(v) => v, c => return Ok(c) };
                Ok(Flow::Value(self.call_value(callee_v, vec![lv], span)?))
            }
        }
    }

    fn eval_unary(&self, op: UnaryOp, v: Value, span: &Span) -> LangResult<Value> {
        match (op, v) {
            (UnaryOp::Negate, Value::Int(n))   => Ok(Value::Int(-n)),
            (UnaryOp::Negate, Value::Float(f)) => Ok(Value::Float(-f)),
            (UnaryOp::Not, Value::Bool(b))     => Ok(Value::Bool(!b)),
            (UnaryOp::Not, v)                  => Ok(Value::Bool(!value_is_truthy(&v))),
            (UnaryOp::Negate, other)           => Err(error(format!("cannot negate {other}"), span)),
        }
    }

    // ── Function calls ────────────────────────────────────────────────────────

    fn eval_args(&mut self, args: &[Expr]) -> LangResult<Vec<Value>> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            match self.eval_expr(arg)? {
                Flow::Value(v) => values.push(v),
                Flow::Return(v) => return Ok(vec![v]),
                Flow::Break | Flow::Continue => {
                    return Err(error("loop control is not valid in call arguments", &arg.span()));
                }
            }
        }
        Ok(values)
    }

    fn eval_value(&mut self, expr: &Expr) -> LangResult<Value> {
        match self.eval_expr(expr)? {
            Flow::Value(v) => Ok(v),
            Flow::Return(_) => Err(error("return is not valid in this expression", &expr.span())),
            Flow::Break     => Err(error("break is not valid in this expression", &expr.span())),
            Flow::Continue  => Err(error("continue is not valid in this expression", &expr.span())),
        }
    }

    fn eval_block(&mut self, block: &Block) -> LangResult<Flow> {
        self.push_scope();
        let mut last = Value::Unit;
        for (i, stmt) in block.statements.iter().enumerate() {
            let is_last = i + 1 == block.statements.len();
            match self.eval_stmt(stmt)? {
                Flow::Value(v) => { if is_last { last = v; } }
                ctrl => { self.pop_scope(); return Ok(ctrl); }
            }
        }
        self.pop_scope();
        Ok(Flow::Value(last))
    }

    fn call_value(&mut self, callee: Value, args: Vec<Value>, span: &Span) -> LangResult<Value> {
        match callee {
            Value::Function(f) => {
                if let Some((lang, _)) = self.extern_names.get(&f.name).cloned() {
                    return Err(error(
                        format!("extern {lang} function '{}' is not callable in the interpreter", f.name),
                        span,
                    ));
                }
                self.call_function(&f, args, span)
            }
            Value::Lambda(lambda) => self.call_lambda(&lambda, args, span),
            Value::Method(_t, f)  => self.call_function(&f, args, span),
            Value::Builtin(b)     => self.call_builtin(b, args, span),
            other => Err(error(format!("{other} is not callable"), span)),
        }
    }

    fn call_function(&mut self, function: &FunctionDecl, args: Vec<Value>, span: &Span) -> LangResult<Value> {
        let probe = self.probe.clone();
        let name = function.name.clone();
        if let Some(p) = &probe { p.borrow_mut().enter(&name); }
        let result = self.call_function_inner(function, args, span);
        if let Some(p) = &probe { p.borrow_mut().exit(&name); }
        result
    }

    fn call_function_inner(&mut self, function: &FunctionDecl, args: Vec<Value>, span: &Span) -> LangResult<Value> {
        if args.len() != function.params.len() {
            return Err(error(
                format!("function '{}' expects {} argument(s), got {}",
                    function.name, function.params.len(), args.len()),
                span,
            ));
        }
        let saved_scopes = self.scopes.clone();
        let global = self.scopes.first().cloned().unwrap_or_default();
        self.scopes = vec![global];
        self.push_scope();
        for (param, value) in function.params.iter().zip(args) {
            self.define(&param.name, value, false, function.span.clone())?;
        }
        let flow = self.eval_block(&function.body);
        let updated_global = self.scopes.first().cloned().unwrap_or_default();
        self.scopes = saved_scopes;
        if let Some(g) = self.scopes.first_mut() { *g = updated_global; }
        match flow? {
            Flow::Value(v) | Flow::Return(v) => Ok(v),
            Flow::Break | Flow::Continue => Err(error("loop control escaped function body", span)),
        }
    }

    fn call_lambda(&mut self, lambda: &LambdaValue, args: Vec<Value>, span: &Span) -> LangResult<Value> {
        let probe = self.probe.clone();
        if let Some(p) = &probe { p.borrow_mut().enter("<lambda>"); }
        let result = self.call_lambda_inner(lambda, args, span);
        if let Some(p) = &probe { p.borrow_mut().exit("<lambda>"); }
        result
    }

    fn call_lambda_inner(&mut self, lambda: &LambdaValue, args: Vec<Value>, span: &Span) -> LangResult<Value> {
        if args.len() != lambda.params.len() {
            return Err(error(
                format!("lambda expects {} argument(s), got {}", lambda.params.len(), args.len()),
                span,
            ));
        }
        let saved = self.scopes.clone();
        self.scopes = lambda.captured_scopes.clone();
        self.push_scope();
        for (param, value) in lambda.params.iter().zip(args) {
            self.define(&param.name, value, false, span.clone())?;
        }
        let result = match &lambda.body {
            LambdaBody::Expr(expr)   => self.eval_expr(expr),
            LambdaBody::Block(block) => self.eval_block(block),
        };
        self.scopes = saved;
        match result? {
            Flow::Value(v) | Flow::Return(v) => Ok(v),
            Flow::Break | Flow::Continue => Err(error("loop control escaped lambda body", span)),
        }
    }

    // ── Pattern matching ──────────────────────────────────────────────────────

    fn pattern_matches(&self, pattern: &Pattern, value: &Value) -> Option<Vec<(String, Value)>> {
        match (pattern, value) {
            (Pattern::Wildcard(_), _) => Some(Vec::new()),
            (Pattern::Variant { name, payload, .. }, Value::Variant(_, vname, vpayload))
                if name == vname && payload.len() == vpayload.len() =>
            {
                let mut bindings = Vec::new();
                for (p, v) in payload.iter().zip(vpayload.iter()) {
                    bindings.extend(self.pattern_matches(p, v)?);
                }
                Some(bindings)
            }
            (Pattern::Ident(name, _), Value::Variant(_, vname, vpayload))
                if vpayload.is_empty() && name == vname
                    && self.enum_variants.get(name).map(|(_, a)| *a == 0).unwrap_or(false) =>
            {
                Some(Vec::new())
            }
            (Pattern::Ident(name, _), Value::Variant(_, vname, _))
                if self.enum_variants.get(name).map(|(_, a)| *a == 0).unwrap_or(false)
                    && name != vname =>
            {
                None
            }
            (Pattern::Ident(name, _), value)      => Some(vec![(name.clone(), value.clone())]),
            (Pattern::Int(p, _), Value::Int(v))   if p == v => Some(Vec::new()),
            (Pattern::Float(p, _), Value::Float(v)) if p == v => Some(Vec::new()),
            (Pattern::Bool(p, _), Value::Bool(v)) if p == v => Some(Vec::new()),
            (Pattern::String(p, _), Value::String(v)) if p == v => Some(Vec::new()),
            (Pattern::Unit(_), Value::Unit)        => Some(Vec::new()),
            _ => None,
        }
    }

    // ── Scope helpers ─────────────────────────────────────────────────────────

    fn define(&mut self, name: &str, value: Value, mutable: bool, span: Span) -> LangResult<()> {
        let scope = self.scopes.last_mut().expect("scope stack is never empty");
        if scope.contains_key(name) {
            return Err(Diagnostic::new(format!("name '{name}' is already defined in this scope"), span));
        }
        scope.insert(name.to_string(), Binding { value, mutable });
        Ok(())
    }

    fn assign(&mut self, name: &str, value: Value, span: &Span) -> LangResult<()> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.get_mut(name) {
                if !binding.mutable {
                    return Err(error(format!("cannot assign to immutable binding '{name}'"), span));
                }
                binding.value = value;
                return Ok(());
            }
        }
        Err(error(format!("unknown binding '{name}'"), span))
    }

    fn resolve(&self, name: &str) -> Option<Value> {
        self.scopes.iter().rev()
            .find_map(|scope| scope.get(name).map(|b| b.value.clone()))
    }

    fn push_scope(&mut self) { self.scopes.push(HashMap::new()); }
    fn pop_scope(&mut self)  { self.scopes.pop(); }
}

impl Default for Interpreter {
    fn default() -> Self { Self::new() }
}

// ── Free helpers ──────────────────────────────────────────────────────────────

fn value_is_truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b)   => *b,
        Value::Int(n)    => *n != 0,
        Value::Float(f)  => *f != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Unit      => false,
        Value::List(items) => !items.borrow().is_empty(),
        Value::Dict(map)   => !map.borrow().is_empty(),
        _ => true,
    }
}

fn apply_binary(left: Value, op: BinaryOp, right: Value, span: &Span) -> LangResult<Value> {
    match (left, op, right) {
        (Value::Int(l), BinaryOp::Add, Value::Int(r)) => Ok(Value::Int(l + r)),
        (Value::Int(l), BinaryOp::Subtract, Value::Int(r)) => Ok(Value::Int(l - r)),
        (Value::Int(l), BinaryOp::Multiply, Value::Int(r)) => Ok(Value::Int(l * r)),
        (Value::Int(_), BinaryOp::Divide, Value::Int(0)) => Err(error("division by zero", span)),
        (Value::Int(l), BinaryOp::Divide, Value::Int(r)) => Ok(Value::Int(l / r)),
        (Value::Int(_), BinaryOp::Remainder, Value::Int(0)) => Err(error("remainder by zero", span)),
        (Value::Int(l), BinaryOp::Remainder, Value::Int(r)) => Ok(Value::Int(l % r)),
        (Value::Float(l), BinaryOp::Add, Value::Float(r)) => Ok(Value::Float(l + r)),
        (Value::Float(l), BinaryOp::Subtract, Value::Float(r)) => Ok(Value::Float(l - r)),
        (Value::Float(l), BinaryOp::Multiply, Value::Float(r)) => Ok(Value::Float(l * r)),
        (Value::Float(l), BinaryOp::Divide, Value::Float(r)) => Ok(Value::Float(l / r)),
        (Value::Float(l), BinaryOp::Remainder, Value::Float(r)) => Ok(Value::Float(l % r)),
        // Mixed int/float
        (Value::Int(l), op, Value::Float(r)) => apply_binary(Value::Float(l as f64), op, Value::Float(r), span),
        (Value::Float(l), op, Value::Int(r)) => apply_binary(Value::Float(l), op, Value::Float(r as f64), span),
        (Value::String(l), BinaryOp::Add, Value::String(r)) => Ok(Value::String(l + &r)),
        (Value::String(l), BinaryOp::Add, r) => Ok(Value::String(l + &r.to_string())),
        (l, BinaryOp::Add, Value::String(r)) => Ok(Value::String(l.to_string() + &r)),
        (l, BinaryOp::Equal, r)    => Ok(Value::Bool(values_equal(&l, &r))),
        (l, BinaryOp::NotEqual, r) => Ok(Value::Bool(!values_equal(&l, &r))),
        (Value::Int(l), BinaryOp::Less, Value::Int(r))         => Ok(Value::Bool(l < r)),
        (Value::Int(l), BinaryOp::LessEqual, Value::Int(r))    => Ok(Value::Bool(l <= r)),
        (Value::Int(l), BinaryOp::Greater, Value::Int(r))      => Ok(Value::Bool(l > r)),
        (Value::Int(l), BinaryOp::GreaterEqual, Value::Int(r)) => Ok(Value::Bool(l >= r)),
        (Value::Float(l), BinaryOp::Less, Value::Float(r))         => Ok(Value::Bool(l < r)),
        (Value::Float(l), BinaryOp::LessEqual, Value::Float(r))    => Ok(Value::Bool(l <= r)),
        (Value::Float(l), BinaryOp::Greater, Value::Float(r))      => Ok(Value::Bool(l > r)),
        (Value::Float(l), BinaryOp::GreaterEqual, Value::Float(r)) => Ok(Value::Bool(l >= r)),
        (Value::String(l), BinaryOp::Less, Value::String(r))    => Ok(Value::Bool(l < r)),
        (Value::String(l), BinaryOp::Greater, Value::String(r)) => Ok(Value::Bool(l > r)),
        (l, op, r) => Err(error(format!("unsupported operation: {l} {op:?} {r}"), span)),
    }
}

fn add_values(a: Value, b: Value, span: &Span) -> LangResult<Value> {
    apply_binary(a, BinaryOp::Add, b, span)
}

fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Int(x), Value::Int(y))       => x.cmp(y),
        (Value::Float(x), Value::Float(y))   => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    }
}

fn numeric_cmp_min(acc: Value, v: Value, _span: &Span) -> LangResult<Value> {
    Ok(match compare_values(&acc, &v) {
        std::cmp::Ordering::Greater => v,
        _ => acc,
    })
}

fn numeric_cmp_max(acc: Value, v: Value, _span: &Span) -> LangResult<Value> {
    Ok(match compare_values(&acc, &v) {
        std::cmp::Ordering::Less => v,
        _ => acc,
    })
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Int(l), Value::Int(r))       => l == r,
        (Value::Float(l), Value::Float(r))   => l == r,
        (Value::Bool(l), Value::Bool(r))     => l == r,
        (Value::String(l), Value::String(r)) => l == r,
        (Value::Unit, Value::Unit)           => true,
        (Value::List(l), Value::List(r)) => {
            let l = l.borrow(); let r = r.borrow();
            l.len() == r.len() && l.iter().zip(r.iter()).all(|(a, b)| values_equal(a, b))
        }
        (Value::Variant(le, ln, lp), Value::Variant(re, rn, rp)) => {
            le == re && ln == rn
                && lp.len() == rp.len()
                && lp.iter().zip(rp.iter()).all(|(a, b)| values_equal(a, b))
        }
        (Value::Channel(a), Value::Channel(b)) => Rc::ptr_eq(a, b),
        (Value::Cancel(a), Value::Cancel(b))   => Rc::ptr_eq(a, b),
        _ => false,
    }
}

fn stmt_span(stmt: &Stmt) -> &Span {
    match stmt {
        Stmt::Let { span, .. } | Stmt::Assign { span, .. } | Stmt::Expr { span, .. }
        | Stmt::Return { span, .. } | Stmt::While { span, .. } | Stmt::For { span, .. }
        | Stmt::Break { span } | Stmt::Continue { span } => span,
        Stmt::Const(d) => &d.span,
    }
}

fn error(message: impl Into<String>, span: &Span) -> Diagnostic {
    Diagnostic::new(message, span.clone())
}
