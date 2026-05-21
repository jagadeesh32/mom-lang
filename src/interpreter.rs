use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::rc::Rc;

use crate::ast::*;
use crate::diagnostic::{Diagnostic, LangResult, Span};

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
    List(Rc<RefCell<Vec<Value>>>),
    Range(i64, i64),
    Struct(String, Rc<RefCell<HashMap<String, Value>>>),
    Variant(String, String, Vec<Value>),
    Task(Box<Value>),
    /// Smart-pointer wrapper. `wrapper_kind` is "Box", "Rc", or "Arc".
    /// In the bootstrap interpreter all three share the same shared
    /// representation; the native runtime distinguishes them.
    Pointer(String, Rc<RefCell<Value>>),
    /// Reference to a value. The bootstrap interpreter auto-derefs
    /// everywhere; the borrow checker enforces the semantic distinction.
    Ref(Box<Value>, bool),
    /// Phase-3 channel. The bootstrap channel is single-threaded and
    /// non-blocking; native concurrency arrives in Phase 3.1.
    Channel(Rc<ChannelInner>),
    /// Cooperative cancellation token.
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

#[derive(Debug, Clone, Copy)]
pub enum Builtin {
    Print,
    Println,
    Len,
    Push,
    ToString,
    BoxNew,
    RcNew,
    ArcNew,
    ChannelNew,
    CancelNew,
    Sleep,
    ReadFile,
    WriteFile,
    Args,
    Getenv,
    Eprint,
    Panic,
    IsDigit,
    IsAlpha,
    IsAlnum,
    ParseInt,
    StringEq,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(value) => write!(f, "{value}"),
            Value::Float(value) => write!(f, "{value}"),
            Value::Bool(value) => write!(f, "{value}"),
            Value::String(value) => write!(f, "{value}"),
            Value::Unit => write!(f, "()"),
            Value::List(items) => {
                let items = items.borrow();
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Value::Range(start, end) => write!(f, "{start}..{end}"),
            Value::Struct(name, fields) => {
                write!(f, "{name} {{ ")?;
                let fields = fields.borrow();
                let mut entries: Vec<_> = fields.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, " }}")
            }
            Value::Variant(_enum, name, payload) => {
                write!(f, "{name}")?;
                if !payload.is_empty() {
                    write!(f, "(")?;
                    for (i, v) in payload.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{v}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Value::Task(inner) => write!(f, "<task {inner}>"),
            Value::Pointer(kind, cell) => {
                let inner = cell.borrow();
                write!(f, "{kind}({inner})")
            }
            // References auto-deref on Display, matching how
            // `println!("{}", &x)` works in Rust.
            Value::Ref(inner, _) => write!(f, "{inner}"),
            Value::Channel(chan) => {
                let cap = chan
                    .capacity
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "∞".to_string());
                write!(f, "<channel len={} cap={}>", chan.queue.borrow().len(), cap)
            }
            Value::Cancel(state) => write!(
                f,
                "<cancel {}>",
                if *state.borrow() { "signalled" } else { "live" }
            ),
            Value::Function(function) => write!(f, "<fn {}>", function.name),
            Value::Lambda(_) => write!(f, "<lambda>"),
            Value::Builtin(b) => write!(f, "<builtin {b:?}>"),
            Value::Method(target, function) => write!(f, "<method {target}.{}>", function.name),
        }
    }
}

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
        enum_variants.insert("Ok".to_string(), ("Result".to_string(), 1));
        enum_variants.insert("Err".to_string(), ("Result".to_string(), 1));

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

    /// Attach a `mom prof`-style probe. The probe receives `enter` /
    /// `exit` events for every `call_function` and `call_lambda`
    /// invocation, including recursive re-entries. Pass `None` (the
    /// default) to disable profiling — there is no per-call overhead
    /// when the probe is absent.
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
                        return Err(error(
                            "'return' is only valid inside a function body",
                            stmt_span(stmt),
                        ));
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
                Item::Function(function) => {
                    self.functions
                        .insert(function.name.clone(), function.clone());
                }
                Item::Enum(decl) => {
                    for variant in &decl.variants {
                        self.enum_variants
                            .insert(variant.name.clone(), (decl.name.clone(), variant.payload.len()));
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
                Item::Module(module) => {
                    self.register_items(&module.items);
                }
                _ => {}
            }
        }
    }

    fn eval_stmt(&mut self, stmt: &Stmt) -> LangResult<Flow> {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                value,
                span,
                ..
            } => {
                let value = match self.eval_expr(value)? {
                    Flow::Value(value) => value,
                    control => return Ok(control),
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
                    Flow::Value(value) => value,
                    control => return Ok(control),
                };
                self.assign_target(target, value, span)?;
                Ok(Flow::Value(Value::Unit))
            }
            Stmt::Expr {
                expr,
                has_semicolon,
                ..
            } => match self.eval_expr(expr)? {
                Flow::Value(_) if *has_semicolon => Ok(Flow::Value(Value::Unit)),
                flow => Ok(flow),
            },
            Stmt::Return { value, .. } => {
                let value = if let Some(value) = value {
                    match self.eval_expr(value)? {
                        Flow::Value(value) => value,
                        control => return Ok(control),
                    }
                } else {
                    Value::Unit
                };
                Ok(Flow::Return(value))
            }
            Stmt::While {
                condition,
                body,
                span,
            } => {
                loop {
                    let condition = self.eval_value(condition)?;
                    let Value::Bool(condition) = condition else {
                        return Err(error("while condition must be Bool", span));
                    };
                    if !condition {
                        break;
                    }

                    match self.eval_block(body)? {
                        Flow::Value(_) => {}
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                        Flow::Break => break,
                        Flow::Continue => continue,
                    }
                }
                Ok(Flow::Value(Value::Unit))
            }
            Stmt::For {
                name,
                iter,
                body,
                span,
            } => self.eval_for(name, iter, body, span),
            Stmt::Break { .. } => Ok(Flow::Break),
            Stmt::Continue { .. } => Ok(Flow::Continue),
        }
    }

    fn eval_for(
        &mut self,
        name: &str,
        iter: &Expr,
        body: &Block,
        span: &Span,
    ) -> LangResult<Flow> {
        let iter_value = self.eval_value(iter)?;
        let items: Vec<Value> = match iter_value {
            Value::List(items) => items.borrow().clone(),
            Value::Range(start, end) => (start..end).map(Value::Int).collect(),
            other => {
                return Err(error(
                    format!("cannot iterate over {other}"),
                    span,
                ));
            }
        };
        for item in items {
            self.push_scope();
            self.define(name, item, false, span.clone())?;
            let flow = self.eval_block(body)?;
            self.pop_scope();
            match flow {
                Flow::Value(_) => {}
                Flow::Return(value) => return Ok(Flow::Return(value)),
                Flow::Break => break,
                Flow::Continue => continue,
            }
        }
        Ok(Flow::Value(Value::Unit))
    }

    fn assign_target(&mut self, target: &AssignTarget, value: Value, span: &Span) -> LangResult<()> {
        match target {
            AssignTarget::Name(name) => self.assign(name, value, span),
            AssignTarget::Field {
                target: target_expr,
                name,
            } => {
                let target_value = self.eval_value(target_expr)?;
                match target_value {
                    Value::Struct(_, fields) => {
                        fields.borrow_mut().insert(name.clone(), value);
                        Ok(())
                    }
                    other => Err(error(
                        format!("cannot set field on {other}"),
                        span,
                    )),
                }
            }
            AssignTarget::Index {
                target: target_expr,
                index,
            } => {
                let target_value = self.eval_value(target_expr)?;
                let index_value = self.eval_value(index)?;
                match (target_value, index_value) {
                    (Value::List(items), Value::Int(idx)) => {
                        let mut items = items.borrow_mut();
                        let len = items.len() as i64;
                        if idx < 0 || idx >= len {
                            return Err(error(
                                format!("index {idx} out of range for list of length {len}"),
                                span,
                            ));
                        }
                        items[idx as usize] = value;
                        Ok(())
                    }
                    (target, idx) => Err(error(
                        format!("cannot index {target} with {idx}"),
                        span,
                    )),
                }
            }
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> LangResult<Flow> {
        match expr {
            Expr::Int(value, _) => Ok(Flow::Value(Value::Int(*value))),
            Expr::Float(value, _) => Ok(Flow::Value(Value::Float(*value))),
            Expr::Bool(value, _) => Ok(Flow::Value(Value::Bool(*value))),
            Expr::String(value, _) => Ok(Flow::Value(Value::String(value.clone()))),
            Expr::Unit(_) => Ok(Flow::Value(Value::Unit)),
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
                        control => return Ok(control),
                    }
                }
                Ok(Flow::Value(Value::List(Rc::new(RefCell::new(values)))))
            }
            Expr::Range { start, end, span } => {
                let s = self.eval_value(start)?;
                let e = self.eval_value(end)?;
                match (s, e) {
                    (Value::Int(a), Value::Int(b)) => Ok(Flow::Value(Value::Range(a, b))),
                    _ => Err(error("range bounds must be Int", span)),
                }
            }
            Expr::Unary { op, expr, span } => {
                let value = match self.eval_expr(expr)? {
                    Flow::Value(value) => value,
                    control => return Ok(control),
                };
                Ok(Flow::Value(self.eval_unary(*op, value, span)?))
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => self.eval_binary(left, *op, right, span),
            Expr::Pipeline { left, right, span } => self.eval_pipeline(left, right, span),
            Expr::Call { callee, args, span } => {
                // direct variant construction by ident
                if let Expr::Ident(name, _) = callee.as_ref() {
                    if let Some((enum_name, _)) = self.enum_variants.get(name).cloned() {
                        let args = self.eval_args(args)?;
                        return Ok(Flow::Value(Value::Variant(
                            enum_name,
                            name.clone(),
                            args,
                        )));
                    }
                    // struct constructor convention `StructName(args)` -> map to fields by name
                    // (skipped: prefer struct literal syntax)
                }
                let callee = match self.eval_expr(callee)? {
                    Flow::Value(value) => value,
                    control => return Ok(control),
                };
                let args = self.eval_args(args)?;
                let value = self.call_value(callee, args, span)?;
                Ok(Flow::Value(value))
            }
            Expr::MethodCall {
                target,
                name,
                args,
                span,
            } => self.eval_method_call(target, name, args, span),
            Expr::Field { target, name, span } => {
                let target_value = self.eval_value(target)?;
                match target_value {
                    Value::Struct(struct_name, fields) => {
                        if let Some(v) = fields.borrow().get(name) {
                            Ok(Flow::Value(v.clone()))
                        } else if let Some(method) =
                            self.methods.get(&struct_name).and_then(|m| m.get(name))
                        {
                            Ok(Flow::Value(Value::Method(
                                struct_name.clone(),
                                method.clone(),
                            )))
                        } else {
                            Err(error(
                                format!("type '{struct_name}' has no field '{name}'"),
                                span,
                            ))
                        }
                    }
                    Value::List(items) if name == "len" => {
                        Ok(Flow::Value(Value::Int(items.borrow().len() as i64)))
                    }
                    other => Err(error(format!("cannot read '{name}' on {other}"), span)),
                }
            }
            Expr::Index { target, index, span } => {
                let target_value = self.eval_value(target)?;
                let index_value = self.eval_value(index)?;
                match (target_value, index_value) {
                    (Value::List(items), Value::Int(idx)) => {
                        let items = items.borrow();
                        let len = items.len() as i64;
                        if idx < 0 || idx >= len {
                            return Err(error(
                                format!("index {idx} out of range for list of length {len}"),
                                span,
                            ));
                        }
                        Ok(Flow::Value(items[idx as usize].clone()))
                    }
                    (Value::String(s), Value::Int(idx)) => {
                        let chars: Vec<char> = s.chars().collect();
                        let len = chars.len() as i64;
                        if idx < 0 || idx >= len {
                            return Err(error(
                                format!("index {idx} out of range for string of length {len}"),
                                span,
                            ));
                        }
                        Ok(Flow::Value(Value::String(chars[idx as usize].to_string())))
                    }
                    (target, idx) => Err(error(
                        format!("cannot index {target} with {idx}"),
                        span,
                    )),
                }
            }
            Expr::StructLit { name, fields, .. } => {
                let mut map = HashMap::new();
                for (field_name, value_expr) in fields {
                    let value = match self.eval_expr(value_expr)? {
                        Flow::Value(v) => v,
                        control => return Ok(control),
                    };
                    map.insert(field_name.clone(), value);
                }
                Ok(Flow::Value(Value::Struct(
                    name.clone(),
                    Rc::new(RefCell::new(map)),
                )))
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let condition = self.eval_value(condition)?;
                let Value::Bool(condition) = condition else {
                    return Err(error("if condition must be Bool", span));
                };
                if condition {
                    self.eval_block(then_branch)
                } else if let Some(else_branch) = else_branch {
                    self.eval_block(else_branch)
                } else {
                    Ok(Flow::Value(Value::Unit))
                }
            }
            Expr::Match {
                scrutinee,
                arms,
                span,
            } => {
                let scrutinee = self.eval_value(scrutinee)?;
                for arm in arms {
                    if let Some(bindings) = self.pattern_matches(&arm.pattern, &scrutinee) {
                        self.push_scope();
                        for (name, value) in bindings {
                            self.define(&name, value, false, arm.span.clone())?;
                        }
                        let result = self.eval_expr(&arm.body);
                        self.pop_scope();
                        return result;
                    }
                }
                Err(error("non-exhaustive match reached at runtime", span))
            }
            Expr::Lambda { params, body, .. } => Ok(Flow::Value(Value::Lambda(LambdaValue {
                params: params.clone(),
                body: body.clone(),
                captured_scopes: self.scopes.clone(),
            }))),
            Expr::Try { expr, span } => {
                let value = match self.eval_expr(expr)? {
                    Flow::Value(v) => v,
                    control => return Ok(control),
                };
                match value {
                    Value::Variant(ref e, ref name, ref payload)
                        if e == "Result" && name == "Ok" && payload.len() == 1 =>
                    {
                        Ok(Flow::Value(payload[0].clone()))
                    }
                    Value::Variant(ref e, ref name, _) if e == "Result" && name == "Err" => {
                        Ok(Flow::Return(value))
                    }
                    Value::Variant(ref e, ref name, ref payload)
                        if e == "Option" && name == "Some" && payload.len() == 1 =>
                    {
                        Ok(Flow::Value(payload[0].clone()))
                    }
                    Value::Variant(ref e, ref name, _) if e == "Option" && name == "None" => {
                        Ok(Flow::Return(value))
                    }
                    other => Err(error(
                        format!("'?' is only valid on Result or Option, got {other}"),
                        span,
                    )),
                }
            }
            Expr::Spawn { expr, .. } => {
                let value = self.eval_value(expr)?;
                Ok(Flow::Value(Value::Task(Box::new(value))))
            }
            Expr::Await { expr, .. } => {
                let value = self.eval_value(expr)?;
                match value {
                    Value::Task(inner) => Ok(Flow::Value(*inner)),
                    other => Ok(Flow::Value(other)),
                }
            }
            Expr::Ref { expr, is_mut, .. } => {
                let value = self.eval_value(expr)?;
                Ok(Flow::Value(Value::Ref(Box::new(value), *is_mut)))
            }
            Expr::Region { body, .. } => self.eval_block(body),
            Expr::Block(block) => self.eval_block(block),
        }
    }

    fn resolve_name(&mut self, name: &str, span: &Span) -> LangResult<Flow> {
        if let Some(value) = self.resolve(name) {
            return Ok(Flow::Value(value));
        }
        if let Some(function) = self.functions.get(name) {
            return Ok(Flow::Value(Value::Function(function.clone())));
        }
        if let Some((enum_name, arity)) = self.enum_variants.get(name).cloned() {
            if arity == 0 {
                return Ok(Flow::Value(Value::Variant(enum_name, name.to_string(), vec![])));
            }
            // Construction without args: produce a closure-like builtin? For MVP require explicit call.
            return Err(error(
                format!("variant '{name}' takes {arity} argument(s); use {name}(...)"),
                span,
            ));
        }
        match name {
            "print" => Ok(Flow::Value(Value::Builtin(Builtin::Print))),
            "println" => Ok(Flow::Value(Value::Builtin(Builtin::Println))),
            "len" => Ok(Flow::Value(Value::Builtin(Builtin::Len))),
            "push" => Ok(Flow::Value(Value::Builtin(Builtin::Push))),
            "to_string" => Ok(Flow::Value(Value::Builtin(Builtin::ToString))),
            "Box" => Ok(Flow::Value(Value::Builtin(Builtin::BoxNew))),
            "Rc" => Ok(Flow::Value(Value::Builtin(Builtin::RcNew))),
            "Arc" => Ok(Flow::Value(Value::Builtin(Builtin::ArcNew))),
            "Channel" => Ok(Flow::Value(Value::Builtin(Builtin::ChannelNew))),
            "Cancel" => Ok(Flow::Value(Value::Builtin(Builtin::CancelNew))),
            "sleep" => Ok(Flow::Value(Value::Builtin(Builtin::Sleep))),
            "read_file" => Ok(Flow::Value(Value::Builtin(Builtin::ReadFile))),
            "write_file" => Ok(Flow::Value(Value::Builtin(Builtin::WriteFile))),
            "args" => Ok(Flow::Value(Value::Builtin(Builtin::Args))),
            "getenv" => Ok(Flow::Value(Value::Builtin(Builtin::Getenv))),
            "eprint" => Ok(Flow::Value(Value::Builtin(Builtin::Eprint))),
            "panic" => Ok(Flow::Value(Value::Builtin(Builtin::Panic))),
            "is_digit" => Ok(Flow::Value(Value::Builtin(Builtin::IsDigit))),
            "is_alpha" => Ok(Flow::Value(Value::Builtin(Builtin::IsAlpha))),
            "is_alnum" => Ok(Flow::Value(Value::Builtin(Builtin::IsAlnum))),
            "parse_int" => Ok(Flow::Value(Value::Builtin(Builtin::ParseInt))),
            "string_eq" => Ok(Flow::Value(Value::Builtin(Builtin::StringEq))),
            _ => Err(error(format!("unknown name '{name}'"), span)),
        }
    }

    fn dispatch_channel(
        &mut self,
        chan: &Rc<ChannelInner>,
        method: &str,
        args: &[Value],
        span: &Span,
    ) -> LangResult<Flow> {
        match method {
            "send" => {
                if args.len() != 2 {
                    return Err(error("Channel.send expects 1 argument", span));
                }
                if *chan.closed.borrow() {
                    return Err(error("Channel.send on closed channel", span));
                }
                if let Some(cap) = chan.capacity {
                    if chan.queue.borrow().len() >= cap {
                        return Err(error(
                            format!(
                                "Channel.send: bounded channel at capacity ({cap}); \
                                 native runtime will backpressure here"
                            ),
                            span,
                        ));
                    }
                }
                chan.queue.borrow_mut().push_back(args[1].clone());
                Ok(Flow::Value(Value::Unit))
            }
            "recv" | "try_recv" => {
                if args.len() != 1 {
                    return Err(error(format!("Channel.{method} takes no arguments"), span));
                }
                let value = chan.queue.borrow_mut().pop_front();
                Ok(Flow::Value(match value {
                    Some(v) => Value::Variant("Option".into(), "Some".into(), vec![v]),
                    None => Value::Variant("Option".into(), "None".into(), vec![]),
                }))
            }
            "len" => {
                if args.len() != 1 {
                    return Err(error("Channel.len takes no arguments", span));
                }
                Ok(Flow::Value(Value::Int(chan.queue.borrow().len() as i64)))
            }
            "is_empty" => {
                if args.len() != 1 {
                    return Err(error("Channel.is_empty takes no arguments", span));
                }
                Ok(Flow::Value(Value::Bool(chan.queue.borrow().is_empty())))
            }
            "close" => {
                if args.len() != 1 {
                    return Err(error("Channel.close takes no arguments", span));
                }
                *chan.closed.borrow_mut() = true;
                Ok(Flow::Value(Value::Unit))
            }
            "capacity" => Ok(Flow::Value(match chan.capacity {
                Some(c) => Value::Variant("Option".into(), "Some".into(), vec![Value::Int(c as i64)]),
                None => Value::Variant("Option".into(), "None".into(), vec![]),
            })),
            other => Err(error(format!("no method '{other}' on Channel"), span)),
        }
    }

    fn dispatch_cancel(
        &mut self,
        state: &Rc<RefCell<bool>>,
        method: &str,
        args: &[Value],
        span: &Span,
    ) -> LangResult<Flow> {
        match method {
            "signal" | "cancel" => {
                if args.len() != 1 {
                    return Err(error(format!("Cancel.{method} takes no arguments"), span));
                }
                *state.borrow_mut() = true;
                Ok(Flow::Value(Value::Unit))
            }
            "is_cancelled" | "is_signalled" => {
                if args.len() != 1 {
                    return Err(error(format!("Cancel.{method} takes no arguments"), span));
                }
                Ok(Flow::Value(Value::Bool(*state.borrow())))
            }
            other => Err(error(format!("no method '{other}' on Cancel"), span)),
        }
    }

    fn eval_method_call(
        &mut self,
        target: &Expr,
        name: &str,
        args: &[Expr],
        span: &Span,
    ) -> LangResult<Flow> {
        let target_value = self.eval_value(target)?;
        let mut arg_values = Vec::with_capacity(args.len() + 1);
        arg_values.push(target_value.clone());
        for arg in args {
            match self.eval_expr(arg)? {
                Flow::Value(v) => arg_values.push(v),
                control => return Ok(control),
            }
        }

        match (&target_value, name) {
            (Value::List(items), "len") => {
                return Ok(Flow::Value(Value::Int(items.borrow().len() as i64)));
            }
            (Value::List(items), "push") => {
                if arg_values.len() != 2 {
                    return Err(error("List.push expects 1 argument", span));
                }
                items.borrow_mut().push(arg_values[1].clone());
                return Ok(Flow::Value(Value::Unit));
            }
            (Value::List(items), "pop") => {
                let popped = items.borrow_mut().pop();
                return Ok(Flow::Value(match popped {
                    Some(v) => Value::Variant("Option".into(), "Some".into(), vec![v]),
                    None => Value::Variant("Option".into(), "None".into(), vec![]),
                }));
            }
            (Value::String(s), "len") => {
                return Ok(Flow::Value(Value::Int(s.chars().count() as i64)));
            }
            (Value::String(s), "to_string") => {
                return Ok(Flow::Value(Value::String(s.clone())));
            }
            (Value::Int(n), "to_string") => {
                return Ok(Flow::Value(Value::String(n.to_string())));
            }
            (Value::Channel(chan), method) => {
                return self.dispatch_channel(chan, method, &arg_values, span);
            }
            (Value::Cancel(state), method) => {
                return self.dispatch_cancel(state, method, &arg_values, span);
            }
            (Value::Pointer(_, cell), method) => match method {
                "get" => return Ok(Flow::Value(cell.borrow().clone())),
                "set" => {
                    if arg_values.len() != 2 {
                        return Err(error("Box/Rc/Arc.set expects 1 argument", span));
                    }
                    *cell.borrow_mut() = arg_values[1].clone();
                    return Ok(Flow::Value(Value::Unit));
                }
                _ => {}
            },
            (Value::Struct(struct_name, _), _) => {
                if let Some(method) = self
                    .methods
                    .get(struct_name)
                    .and_then(|m| m.get(name))
                    .cloned()
                {
                    let value = self.call_function(&method, arg_values, span)?;
                    return Ok(Flow::Value(value));
                }
            }
            _ => {}
        }

        Err(error(
            format!("no method '{name}' on {target_value}"),
            span,
        ))
    }

    fn eval_binary(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
        span: &Span,
    ) -> LangResult<Flow> {
        if op == BinaryOp::And {
            let left = self.eval_value(left)?;
            let Value::Bool(left) = left else {
                return Err(error("'&&' left operand must be Bool", span));
            };
            if !left {
                return Ok(Flow::Value(Value::Bool(false)));
            }
            let right = self.eval_value(right)?;
            let Value::Bool(right) = right else {
                return Err(error("'&&' right operand must be Bool", span));
            };
            return Ok(Flow::Value(Value::Bool(right)));
        }

        if op == BinaryOp::Or {
            let left = self.eval_value(left)?;
            let Value::Bool(left) = left else {
                return Err(error("'||' left operand must be Bool", span));
            };
            if left {
                return Ok(Flow::Value(Value::Bool(true)));
            }
            let right = self.eval_value(right)?;
            let Value::Bool(right) = right else {
                return Err(error("'||' right operand must be Bool", span));
            };
            return Ok(Flow::Value(Value::Bool(right)));
        }

        let left = match self.eval_expr(left)? {
            Flow::Value(value) => value,
            control => return Ok(control),
        };
        let right = match self.eval_expr(right)? {
            Flow::Value(value) => value,
            control => return Ok(control),
        };

        Ok(Flow::Value(self.apply_binary(left, op, right, span)?))
    }

    fn eval_pipeline(&mut self, left: &Expr, right: &Expr, span: &Span) -> LangResult<Flow> {
        let left = match self.eval_expr(left)? {
            Flow::Value(value) => value,
            control => return Ok(control),
        };

        match right {
            Expr::Call { callee, args, .. } => {
                let callee = match self.eval_expr(callee)? {
                    Flow::Value(value) => value,
                    control => return Ok(control),
                };
                let mut values = vec![left];
                values.extend(self.eval_args(args)?);
                Ok(Flow::Value(self.call_value(callee, values, span)?))
            }
            _ => {
                let callee = match self.eval_expr(right)? {
                    Flow::Value(value) => value,
                    control => return Ok(control),
                };
                Ok(Flow::Value(self.call_value(callee, vec![left], span)?))
            }
        }
    }

    fn eval_args(&mut self, args: &[Expr]) -> LangResult<Vec<Value>> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            match self.eval_expr(arg)? {
                Flow::Value(value) => values.push(value),
                Flow::Return(value) => return Ok(vec![value]),
                Flow::Break | Flow::Continue => {
                    return Err(error(
                        "loop control is not valid in call arguments",
                        &arg.span(),
                    ));
                }
            }
        }
        Ok(values)
    }

    fn eval_value(&mut self, expr: &Expr) -> LangResult<Value> {
        match self.eval_expr(expr)? {
            Flow::Value(value) => Ok(value),
            Flow::Return(_) => Err(error(
                "return is not valid in this expression",
                &expr.span(),
            )),
            Flow::Break => Err(error("break is not valid in this expression", &expr.span())),
            Flow::Continue => Err(error(
                "continue is not valid in this expression",
                &expr.span(),
            )),
        }
    }

    fn eval_block(&mut self, block: &Block) -> LangResult<Flow> {
        self.push_scope();
        let mut last = Value::Unit;

        for (index, stmt) in block.statements.iter().enumerate() {
            let is_last = index + 1 == block.statements.len();
            match self.eval_stmt(stmt)? {
                Flow::Value(value) => {
                    if is_last {
                        last = value;
                    }
                }
                control => {
                    self.pop_scope();
                    return Ok(control);
                }
            }
        }

        self.pop_scope();
        Ok(Flow::Value(last))
    }

    fn call_value(&mut self, callee: Value, args: Vec<Value>, span: &Span) -> LangResult<Value> {
        match callee {
            Value::Function(function) => {
                if let Some((lang, _)) = self.extern_names.get(&function.name).cloned() {
                    return Err(error(
                        format!(
                            "extern {lang} function '{}' is not callable in the bootstrap interpreter; \
                             link with the native backend",
                            function.name
                        ),
                        span,
                    ));
                }
                self.call_function(&function, args, span)
            }
            Value::Lambda(lambda) => self.call_lambda(&lambda, args, span),
            Value::Method(_target, function) => self.call_function(&function, args, span),
            Value::Builtin(builtin) => self.call_builtin(builtin, args, span),
            other => Err(error(format!("{other} is not callable"), span)),
        }
    }

    fn call_builtin(&mut self, builtin: Builtin, args: Vec<Value>, span: &Span) -> LangResult<Value> {
        match builtin {
            Builtin::Print | Builtin::Println => {
                if args.len() != 1 {
                    return Err(error(
                        format!("print expects 1 argument, got {}", args.len()),
                        span,
                    ));
                }
                self.output.push_str(&args[0].to_string());
                self.output.push('\n');
                Ok(Value::Unit)
            }
            Builtin::Len => {
                if args.len() != 1 {
                    return Err(error("len expects 1 argument", span));
                }
                match &args[0] {
                    Value::List(items) => Ok(Value::Int(items.borrow().len() as i64)),
                    Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
                    other => Err(error(format!("len: unsupported value {other}"), span)),
                }
            }
            Builtin::Push => {
                if args.len() != 2 {
                    return Err(error("push expects 2 arguments", span));
                }
                match &args[0] {
                    Value::List(items) => {
                        items.borrow_mut().push(args[1].clone());
                        Ok(Value::Unit)
                    }
                    other => Err(error(format!("push: target must be List, got {other}"), span)),
                }
            }
            Builtin::ToString => {
                if args.len() != 1 {
                    return Err(error("to_string expects 1 argument", span));
                }
                Ok(Value::String(args[0].to_string()))
            }
            Builtin::BoxNew | Builtin::RcNew | Builtin::ArcNew => {
                if args.len() != 1 {
                    return Err(error(
                        "Box/Rc/Arc constructor expects 1 argument",
                        span,
                    ));
                }
                let kind = match builtin {
                    Builtin::BoxNew => "Box",
                    Builtin::RcNew => "Rc",
                    Builtin::ArcNew => "Arc",
                    _ => unreachable!(),
                };
                Ok(Value::Pointer(
                    kind.to_string(),
                    Rc::new(RefCell::new(args.into_iter().next().unwrap())),
                ))
            }
            Builtin::ChannelNew => {
                let capacity = match args.len() {
                    0 => None,
                    1 => match &args[0] {
                        Value::Int(n) if *n >= 0 => Some(*n as usize),
                        Value::Int(_) => {
                            return Err(error("Channel capacity must be non-negative", span))
                        }
                        other => {
                            return Err(error(
                                format!("Channel(<Int>) expected, got {other}"),
                                span,
                            ))
                        }
                    },
                    n => {
                        return Err(error(
                            format!("Channel expects 0 or 1 argument, got {n}"),
                            span,
                        ))
                    }
                };
                Ok(Value::Channel(Rc::new(ChannelInner {
                    capacity,
                    queue: RefCell::new(VecDeque::new()),
                    closed: RefCell::new(false),
                })))
            }
            Builtin::CancelNew => {
                if !args.is_empty() {
                    return Err(error("Cancel() takes no arguments", span));
                }
                Ok(Value::Cancel(Rc::new(RefCell::new(false))))
            }
            Builtin::Sleep => {
                // Bootstrap runtime is single-threaded; sleep is a no-op
                // that nonetheless validates its argument.
                match args.as_slice() {
                    [Value::Int(_)] => Ok(Value::Unit),
                    [other] => Err(error(
                        format!("sleep expects Int milliseconds, got {other}"),
                        span,
                    )),
                    _ => Err(error("sleep expects 1 argument", span)),
                }
            }
            Builtin::ReadFile => match args.as_slice() {
                [Value::String(path)] => match std::fs::read_to_string(path) {
                    Ok(content) => Ok(Value::Variant(
                        "Result".into(),
                        "Ok".into(),
                        vec![Value::String(content)],
                    )),
                    Err(err) => Ok(Value::Variant(
                        "Result".into(),
                        "Err".into(),
                        vec![Value::String(err.to_string())],
                    )),
                },
                _ => Err(error("read_file expects 1 String argument", span)),
            },
            Builtin::WriteFile => match args.as_slice() {
                [Value::String(path), Value::String(content)] => {
                    match std::fs::write(path, content) {
                        Ok(()) => Ok(Value::Variant(
                            "Result".into(),
                            "Ok".into(),
                            vec![Value::Unit],
                        )),
                        Err(err) => Ok(Value::Variant(
                            "Result".into(),
                            "Err".into(),
                            vec![Value::String(err.to_string())],
                        )),
                    }
                }
                _ => Err(error(
                    "write_file expects (path: String, content: String)",
                    span,
                )),
            },
            Builtin::Args => {
                if !args.is_empty() {
                    return Err(error("args() takes no arguments", span));
                }
                let argv: Vec<Value> = std::env::args()
                    .skip(1)
                    .map(Value::String)
                    .collect();
                Ok(Value::List(Rc::new(RefCell::new(argv))))
            }
            Builtin::Getenv => match args.as_slice() {
                [Value::String(key)] => Ok(match std::env::var(key) {
                    Ok(v) => Value::Variant("Option".into(), "Some".into(), vec![Value::String(v)]),
                    Err(_) => Value::Variant("Option".into(), "None".into(), vec![]),
                }),
                _ => Err(error("getenv expects 1 String argument", span)),
            },
            Builtin::Eprint => match args.as_slice() {
                [value] => {
                    eprintln!("{value}");
                    Ok(Value::Unit)
                }
                _ => Err(error("eprint expects 1 argument", span)),
            },
            Builtin::Panic => match args.as_slice() {
                [Value::String(msg)] => Err(error(format!("panic: {msg}"), span)),
                _ => Err(error("panic expects a single String argument", span)),
            },
            Builtin::IsDigit => match args.as_slice() {
                [Value::String(s)] => Ok(Value::Bool(
                    s.chars().count() == 1 && s.chars().next().unwrap().is_ascii_digit(),
                )),
                _ => Err(error("is_digit expects 1 String argument", span)),
            },
            Builtin::IsAlpha => match args.as_slice() {
                [Value::String(s)] => Ok(Value::Bool(
                    s.chars().count() == 1
                        && (s.chars().next().unwrap().is_ascii_alphabetic()
                            || s.chars().next().unwrap() == '_'),
                )),
                _ => Err(error("is_alpha expects 1 String argument", span)),
            },
            Builtin::IsAlnum => match args.as_slice() {
                [Value::String(s)] => Ok(Value::Bool(s.chars().count() == 1 && {
                    let c = s.chars().next().unwrap();
                    c.is_ascii_alphanumeric() || c == '_'
                })),
                _ => Err(error("is_alnum expects 1 String argument", span)),
            },
            Builtin::ParseInt => match args.as_slice() {
                [Value::String(s)] => Ok(match s.parse::<i64>() {
                    Ok(n) => Value::Variant("Option".into(), "Some".into(), vec![Value::Int(n)]),
                    Err(_) => Value::Variant("Option".into(), "None".into(), vec![]),
                }),
                _ => Err(error("parse_int expects 1 String argument", span)),
            },
            Builtin::StringEq => match args.as_slice() {
                [Value::String(a), Value::String(b)] => Ok(Value::Bool(a == b)),
                _ => Err(error("string_eq expects 2 String arguments", span)),
            },
        }
    }

    fn call_function(
        &mut self,
        function: &FunctionDecl,
        args: Vec<Value>,
        span: &Span,
    ) -> LangResult<Value> {
        let probe = self.probe.clone();
        let name = function.name.clone();
        if let Some(p) = &probe {
            p.borrow_mut().enter(&name);
        }
        let result = self.call_function_inner(function, args, span);
        if let Some(p) = &probe {
            p.borrow_mut().exit(&name);
        }
        result
    }

    fn call_function_inner(
        &mut self,
        function: &FunctionDecl,
        args: Vec<Value>,
        span: &Span,
    ) -> LangResult<Value> {
        if args.len() != function.params.len() {
            return Err(error(
                format!(
                    "function '{}' expects {} argument(s), got {}",
                    function.name,
                    function.params.len(),
                    args.len()
                ),
                span,
            ));
        }

        let saved_scopes = self.scopes.clone();
        let global_scope = self.scopes.first().cloned().unwrap_or_default();
        self.scopes = vec![global_scope];
        self.push_scope();
        for (param, value) in function.params.iter().zip(args) {
            self.define(&param.name, value, false, function.span.clone())?;
        }

        let flow = self.eval_block(&function.body);
        let updated_global = self.scopes.first().cloned().unwrap_or_default();
        self.scopes = saved_scopes;
        if let Some(global) = self.scopes.first_mut() {
            *global = updated_global;
        }

        match flow? {
            Flow::Value(value) | Flow::Return(value) => Ok(value),
            Flow::Break | Flow::Continue => Err(error("loop control escaped function body", span)),
        }
    }

    fn call_lambda(
        &mut self,
        lambda: &LambdaValue,
        args: Vec<Value>,
        span: &Span,
    ) -> LangResult<Value> {
        let probe = self.probe.clone();
        if let Some(p) = &probe {
            p.borrow_mut().enter("<lambda>");
        }
        let result = self.call_lambda_inner(lambda, args, span);
        if let Some(p) = &probe {
            p.borrow_mut().exit("<lambda>");
        }
        result
    }

    fn call_lambda_inner(
        &mut self,
        lambda: &LambdaValue,
        args: Vec<Value>,
        span: &Span,
    ) -> LangResult<Value> {
        if args.len() != lambda.params.len() {
            return Err(error(
                format!(
                    "lambda expects {} argument(s), got {}",
                    lambda.params.len(),
                    args.len()
                ),
                span,
            ));
        }

        let saved_scopes = self.scopes.clone();
        self.scopes = lambda.captured_scopes.clone();
        self.push_scope();
        for (param, value) in lambda.params.iter().zip(args) {
            self.define(&param.name, value, false, span.clone())?;
        }

        let result = match &lambda.body {
            LambdaBody::Expr(expr) => self.eval_expr(expr),
            LambdaBody::Block(block) => self.eval_block(block),
        };
        self.scopes = saved_scopes;

        match result? {
            Flow::Value(value) | Flow::Return(value) => Ok(value),
            Flow::Break | Flow::Continue => Err(error("loop control escaped lambda body", span)),
        }
    }

    fn eval_unary(&self, op: UnaryOp, value: Value, span: &Span) -> LangResult<Value> {
        match (op, value) {
            (UnaryOp::Negate, Value::Int(value)) => Ok(Value::Int(-value)),
            (UnaryOp::Negate, Value::Float(value)) => Ok(Value::Float(-value)),
            (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
            (UnaryOp::Negate, other) => Err(error(format!("cannot negate {other}"), span)),
            (UnaryOp::Not, other) => Err(error(format!("cannot apply '!' to {other}"), span)),
        }
    }

    fn apply_binary(
        &self,
        left: Value,
        op: BinaryOp,
        right: Value,
        span: &Span,
    ) -> LangResult<Value> {
        match (left, op, right) {
            (Value::Int(left), BinaryOp::Add, Value::Int(right)) => Ok(Value::Int(left + right)),
            (Value::Int(left), BinaryOp::Subtract, Value::Int(right)) => {
                Ok(Value::Int(left - right))
            }
            (Value::Int(left), BinaryOp::Multiply, Value::Int(right)) => {
                Ok(Value::Int(left * right))
            }
            (Value::Int(_), BinaryOp::Divide, Value::Int(0)) => {
                Err(error("division by zero", span))
            }
            (Value::Int(left), BinaryOp::Divide, Value::Int(right)) => Ok(Value::Int(left / right)),
            (Value::Int(_), BinaryOp::Remainder, Value::Int(0)) => {
                Err(error("remainder by zero", span))
            }
            (Value::Int(left), BinaryOp::Remainder, Value::Int(right)) => {
                Ok(Value::Int(left % right))
            }
            (Value::Float(left), BinaryOp::Add, Value::Float(right)) => {
                Ok(Value::Float(left + right))
            }
            (Value::Float(left), BinaryOp::Subtract, Value::Float(right)) => {
                Ok(Value::Float(left - right))
            }
            (Value::Float(left), BinaryOp::Multiply, Value::Float(right)) => {
                Ok(Value::Float(left * right))
            }
            (Value::Float(left), BinaryOp::Divide, Value::Float(right)) => {
                Ok(Value::Float(left / right))
            }
            (Value::String(left), BinaryOp::Add, Value::String(right)) => {
                Ok(Value::String(format!("{left}{right}")))
            }
            (left, BinaryOp::Equal, right) => Ok(Value::Bool(values_equal(&left, &right))),
            (left, BinaryOp::NotEqual, right) => Ok(Value::Bool(!values_equal(&left, &right))),
            (Value::Int(left), BinaryOp::Less, Value::Int(right)) => Ok(Value::Bool(left < right)),
            (Value::Int(left), BinaryOp::LessEqual, Value::Int(right)) => {
                Ok(Value::Bool(left <= right))
            }
            (Value::Int(left), BinaryOp::Greater, Value::Int(right)) => {
                Ok(Value::Bool(left > right))
            }
            (Value::Int(left), BinaryOp::GreaterEqual, Value::Int(right)) => {
                Ok(Value::Bool(left >= right))
            }
            (Value::Float(left), BinaryOp::Less, Value::Float(right)) => {
                Ok(Value::Bool(left < right))
            }
            (Value::Float(left), BinaryOp::LessEqual, Value::Float(right)) => {
                Ok(Value::Bool(left <= right))
            }
            (Value::Float(left), BinaryOp::Greater, Value::Float(right)) => {
                Ok(Value::Bool(left > right))
            }
            (Value::Float(left), BinaryOp::GreaterEqual, Value::Float(right)) => {
                Ok(Value::Bool(left >= right))
            }
            (left, op, right) => Err(error(
                format!("unsupported operation: {left} {op:?} {right}"),
                span,
            )),
        }
    }

    fn define(&mut self, name: &str, value: Value, mutable: bool, span: Span) -> LangResult<()> {
        let scope = self.scopes.last_mut().expect("scope stack is never empty");
        if scope.contains_key(name) {
            return Err(Diagnostic::new(
                format!("name '{name}' is already defined in this scope"),
                span,
            ));
        }
        scope.insert(name.to_string(), Binding { value, mutable });
        Ok(())
    }

    fn assign(&mut self, name: &str, value: Value, span: &Span) -> LangResult<()> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.get_mut(name) {
                if !binding.mutable {
                    return Err(error(
                        format!("cannot assign to immutable binding '{name}'"),
                        span,
                    ));
                }
                binding.value = value;
                return Ok(());
            }
        }
        Err(error(format!("unknown binding '{name}'"), span))
    }

    fn resolve(&self, name: &str) -> Option<Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).map(|binding| binding.value.clone()))
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    /// Match a pattern against a value, returning the (name, value)
    /// bindings the pattern introduces on success. Names that happen
    /// to be zero-arity enum variants take the variant interpretation
    /// over the binding one.
    fn pattern_matches(&self, pattern: &Pattern, value: &Value) -> Option<Vec<(String, Value)>> {
        match (pattern, value) {
            (Pattern::Wildcard(_), _) => Some(Vec::new()),
            (
                Pattern::Variant { name, payload, .. },
                Value::Variant(_enum, vname, vpayload),
            ) if name == vname && payload.len() == vpayload.len() => {
                let mut bindings = Vec::new();
                for (p, v) in payload.iter().zip(vpayload.iter()) {
                    let inner = self.pattern_matches(p, v)?;
                    bindings.extend(inner);
                }
                Some(bindings)
            }
            // Bare ident pattern: prefer "zero-arity variant constructor"
            // over "binding" when the name matches a registered variant.
            (Pattern::Ident(name, _), Value::Variant(_enum, vname, vpayload))
                if vpayload.is_empty()
                    && name == vname
                    && self
                        .enum_variants
                        .get(name)
                        .map(|(_, arity)| *arity == 0)
                        .unwrap_or(false) =>
            {
                Some(Vec::new())
            }
            // A zero-arity variant name in pattern position should *not*
            // match a different variant. Reject and fall through.
            (Pattern::Ident(name, _), Value::Variant(_enum, vname, _))
                if self
                    .enum_variants
                    .get(name)
                    .map(|(_, arity)| *arity == 0)
                    .unwrap_or(false)
                    && name != vname =>
            {
                None
            }
            (Pattern::Ident(name, _), value) => Some(vec![(name.clone(), value.clone())]),
            (Pattern::Int(pattern, _), Value::Int(value)) if pattern == value => Some(Vec::new()),
            (Pattern::Float(pattern, _), Value::Float(value)) if pattern == value => {
                Some(Vec::new())
            }
            (Pattern::Bool(pattern, _), Value::Bool(value)) if pattern == value => Some(Vec::new()),
            (Pattern::String(pattern, _), Value::String(value)) if pattern == value => {
                Some(Vec::new())
            }
            (Pattern::Unit(_), Value::Unit) => Some(Vec::new()),
            _ => None,
        }
    }
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Int(left), Value::Int(right)) => left == right,
        (Value::Float(left), Value::Float(right)) => left == right,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Unit, Value::Unit) => true,
        (Value::List(left), Value::List(right)) => {
            let left = left.borrow();
            let right = right.borrow();
            left.len() == right.len()
                && left.iter().zip(right.iter()).all(|(a, b)| values_equal(a, b))
        }
        (Value::Variant(le, ln, lp), Value::Variant(re, rn, rp)) => {
            le == re
                && ln == rn
                && lp.len() == rp.len()
                && lp.iter().zip(rp.iter()).all(|(a, b)| values_equal(a, b))
        }
        (Value::Channel(a), Value::Channel(b)) => Rc::ptr_eq(a, b),
        (Value::Cancel(a), Value::Cancel(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

fn stmt_span(stmt: &Stmt) -> &Span {
    match stmt {
        Stmt::Let { span, .. }
        | Stmt::Assign { span, .. }
        | Stmt::Expr { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::While { span, .. }
        | Stmt::For { span, .. }
        | Stmt::Break { span }
        | Stmt::Continue { span } => span,
        Stmt::Const(decl) => &decl.span,
    }
}

fn error(message: impl Into<String>, span: &Span) -> Diagnostic {
    Diagnostic::new(message, span.clone())
}
