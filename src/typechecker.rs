use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::ast::*;
use crate::diagnostic::{Diagnostic, LangResult, Span};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Unit,
    Named(String),
    Generic(String, Vec<Type>),
    Function(Vec<Type>, Box<Type>),
    List(Box<Type>),
    Range(Box<Type>),
    Ref(Box<Type>, bool /* is_mut */),
    Unknown,
}

impl Type {
    fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Unit => write!(f, "()"),
            Type::Named(name) => write!(f, "{name}"),
            Type::Generic(name, args) => {
                write!(f, "{name}[")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{a}")?;
                }
                write!(f, "]")
            }
            Type::Function(params, result) => {
                write!(f, "fn(")?;
                for (index, param) in params.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }
                write!(f, ") -> {result}")
            }
            Type::List(elem) => write!(f, "[{elem}]"),
            Type::Range(elem) => write!(f, "Range[{elem}]"),
            Type::Ref(inner, false) => write!(f, "&{inner}"),
            Type::Ref(inner, true) => write!(f, "&mut {inner}"),
            Type::Unknown => write!(f, "_"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeReport {
    pub functions: HashMap<String, Type>,
    pub types: Vec<String>,
}

#[derive(Debug, Clone)]
struct Symbol {
    ty: Type,
    mutable: bool,
}

#[derive(Debug, Clone)]
struct BlockCheck {
    value_type: Type,
    returns: bool,
}

#[derive(Debug, Clone)]
struct StmtCheck {
    value_type: Type,
    returns: bool,
}

pub struct TypeChecker {
    scopes: Vec<HashMap<String, Symbol>>,
    functions: HashMap<String, Type>,
    known_types: HashSet<String>,
    enum_variants: HashMap<String, (String, usize)>, // variant_name -> (enum_name, arity)
    current_return: Option<Type>,
    loop_depth: usize,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut known_types = HashSet::new();
        for builtin in [
            "Int", "Float", "Bool", "String", "Option", "Result", "List", "Dict", "Task", "Future",
            "Box", "Rc", "Arc", "Weak", "Cell", "RefCell", "Mutex", "RwLock", "Atomic",
            "Channel", "Cancel", "Duration", "Instant",
        ] {
            known_types.insert(builtin.to_string());
        }

        let mut enum_variants = HashMap::new();
        enum_variants.insert("Some".to_string(), ("Option".to_string(), 1));
        enum_variants.insert("None".to_string(), ("Option".to_string(), 0));
        enum_variants.insert("Ok".to_string(), ("Result".to_string(), 1));
        enum_variants.insert("Err".to_string(), ("Result".to_string(), 1));

        Self {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            known_types,
            enum_variants,
            current_return: None,
            loop_depth: 0,
        }
    }

    pub fn check_program(mut self, program: &Program) -> LangResult<TypeReport> {
        self.predeclare_types(&program.items)?;
        self.predeclare_functions(&program.items)?;

        for item in &program.items {
            self.check_item(item)?;
        }

        let mut types = self.known_types.iter().cloned().collect::<Vec<_>>();
        types.sort();
        Ok(TypeReport {
            functions: self.functions,
            types,
        })
    }

    fn check_item(&mut self, item: &Item) -> LangResult<()> {
        match item {
            Item::Function(function) => self.check_function(function),
            Item::Struct(decl) => self.check_struct(decl),
            Item::Enum(decl) => self.check_enum(decl),
            Item::Const(decl) => {
                let ty = self.check_const_decl(decl)?;
                self.define(&decl.name, ty, false, decl.span.clone())
            }
            Item::Module(module) => {
                self.predeclare_types(&module.items)?;
                self.predeclare_functions(&module.items)?;
                for inner in &module.items {
                    self.check_item(inner)?;
                }
                Ok(())
            }
            Item::Import(_) => Ok(()),
            Item::Trait(_) => Ok(()),
            Item::Impl(block) => {
                for method in &block.methods {
                    self.check_function(method)?;
                }
                Ok(())
            }
            Item::Extern(block) => {
                for item in &block.items {
                    let params = item
                        .params
                        .iter()
                        .map(|p| self.type_from_ref(&p.ty, &item.span))
                        .collect::<LangResult<Vec<_>>>()?;
                    let result = self.type_from_ref(&item.return_type, &item.span)?;
                    self.functions
                        .insert(item.name.clone(), Type::Function(params, Box::new(result)));
                }
                Ok(())
            }
            Item::Statement(stmt) => {
                self.check_statement(stmt)?;
                Ok(())
            }
        }
    }

    fn predeclare_types(&mut self, items: &[Item]) -> LangResult<()> {
        for item in items {
            match item {
                Item::Struct(decl) => {
                    self.known_types.insert(decl.name.clone());
                }
                Item::Enum(decl) => {
                    self.known_types.insert(decl.name.clone());
                    for variant in &decl.variants {
                        self.enum_variants
                            .insert(variant.name.clone(), (decl.name.clone(), variant.payload.len()));
                    }
                }
                Item::Module(module) => {
                    self.predeclare_types(&module.items)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn predeclare_functions(&mut self, items: &[Item]) -> LangResult<()> {
        for item in items {
            match item {
                Item::Function(function) => {
                    let generics = function.generics.iter().cloned().collect();
                    let params = function
                        .params
                        .iter()
                        .map(|param| self.type_from_ref_with(&param.ty, &function.span, &generics))
                        .collect::<LangResult<Vec<_>>>()?;
                    let result = self.type_from_ref_with(
                        &function.return_type,
                        &function.span,
                        &generics,
                    )?;
                    let signature = Type::Function(params, Box::new(result));
                    self.functions.insert(function.name.clone(), signature);
                }
                Item::Module(module) => {
                    self.predeclare_functions(&module.items)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn check_function(&mut self, function: &FunctionDecl) -> LangResult<()> {
        let generics: HashSet<String> = function.generics.iter().cloned().collect();
        let return_type =
            self.type_from_ref_with(&function.return_type, &function.span, &generics)?;
        let previous_return = self.current_return.replace(return_type.clone());

        self.push_scope();
        for param in &function.params {
            let ty = self.type_from_ref_with(&param.ty, &function.span, &generics)?;
            self.define(&param.name, ty, false, function.span.clone())?;
        }

        let body = self.check_block(&function.body)?;
        self.pop_scope();
        self.current_return = previous_return;

        if body.value_type != Type::Unit {
            self.expect_assignable(&return_type, &body.value_type, &function.span)?;
        } else if return_type != Type::Unit && !body.returns {
            // be lenient when return is Unknown (generic) or function is async
            if return_type != Type::Unknown && !function.is_async {
                return Err(error(
                    format!(
                        "function '{}' may finish without returning {}",
                        function.name, return_type
                    ),
                    &function.span,
                ));
            }
        }

        Ok(())
    }

    fn check_struct(&mut self, decl: &StructDecl) -> LangResult<()> {
        let mut generic_set = HashSet::new();
        for g in &decl.generics {
            generic_set.insert(g.clone());
        }
        for field in &decl.fields {
            self.type_from_ref_with(&field.ty, &decl.span, &generic_set)?;
        }
        Ok(())
    }

    fn check_enum(&mut self, decl: &EnumDecl) -> LangResult<()> {
        let mut generic_set = HashSet::new();
        for g in &decl.generics {
            generic_set.insert(g.clone());
        }
        for variant in &decl.variants {
            for ty in &variant.payload {
                self.type_from_ref_with(ty, &decl.span, &generic_set)?;
            }
        }
        Ok(())
    }

    fn check_const_decl(&mut self, decl: &ConstDecl) -> LangResult<Type> {
        let value_ty = self.check_expr(&decl.value)?;
        if let Some(ty_ref) = &decl.ty {
            let annotated = self.type_from_ref(ty_ref, &decl.span)?;
            self.expect_assignable(&annotated, &value_ty, &decl.span)?;
            Ok(annotated)
        } else {
            Ok(value_ty)
        }
    }

    fn check_block(&mut self, block: &Block) -> LangResult<BlockCheck> {
        self.push_scope();
        let mut value_type = Type::Unit;
        let mut returns = false;

        for (index, stmt) in block.statements.iter().enumerate() {
            let check = self.check_statement(stmt)?;
            let is_last = index + 1 == block.statements.len();
            if is_last {
                value_type = check.value_type;
            }
            if check.returns {
                returns = true;
            }
        }

        self.pop_scope();
        Ok(BlockCheck {
            value_type,
            returns,
        })
    }

    fn check_statement(&mut self, stmt: &Stmt) -> LangResult<StmtCheck> {
        match stmt {
            Stmt::Let {
                name,
                ty,
                mutable,
                value,
                span,
            } => {
                let value_ty = self.check_expr(value)?;
                let final_ty = if let Some(ty_ref) = ty {
                    let annotated = self.type_from_ref(ty_ref, span)?;
                    self.expect_assignable(&annotated, &value_ty, span)?;
                    annotated
                } else {
                    value_ty
                };
                self.define(name, final_ty, *mutable, span.clone())?;
                Ok(unit_stmt())
            }
            Stmt::Const(decl) => {
                let ty = self.check_const_decl(decl)?;
                self.define(&decl.name, ty, false, decl.span.clone())?;
                Ok(unit_stmt())
            }
            Stmt::Assign { target, value, span } => {
                let value_ty = self.check_expr(value)?;
                match target {
                    AssignTarget::Name(name) => {
                        let symbol = self
                            .resolve(name)
                            .ok_or_else(|| error(format!("unknown binding '{name}'"), span))?;
                        if !symbol.mutable {
                            return Err(error(
                                format!("cannot assign to immutable binding '{name}'"),
                                span,
                            ));
                        }
                        self.expect_assignable(&symbol.ty, &value_ty, span)?;
                    }
                    AssignTarget::Field { target, .. } => {
                        self.check_expr(target)?;
                    }
                    AssignTarget::Index { target, index } => {
                        self.check_expr(target)?;
                        self.check_expr(index)?;
                    }
                }
                Ok(unit_stmt())
            }
            Stmt::Expr {
                expr,
                has_semicolon,
                ..
            } => {
                let ty = self.check_expr(expr)?;
                Ok(StmtCheck {
                    value_type: if *has_semicolon { Type::Unit } else { ty },
                    returns: false,
                })
            }
            Stmt::Return { value, span } => {
                let expected = self
                    .current_return
                    .clone()
                    .ok_or_else(|| error("'return' is only valid inside a function body", span))?;
                let actual = if let Some(value) = value {
                    self.check_expr(value)?
                } else {
                    Type::Unit
                };
                self.expect_assignable(&expected, &actual, span)?;
                Ok(StmtCheck {
                    value_type: Type::Unit,
                    returns: true,
                })
            }
            Stmt::While {
                condition,
                body,
                span,
            } => {
                let condition_ty = self.check_expr(condition)?;
                self.expect_assignable(&Type::Bool, &condition_ty, span)?;
                self.loop_depth += 1;
                self.check_block(body)?;
                self.loop_depth -= 1;
                Ok(unit_stmt())
            }
            Stmt::For {
                name, iter, body, ..
            } => {
                let iter_ty = self.check_expr(iter)?;
                let elem = match iter_ty {
                    Type::List(elem) => *elem,
                    Type::Range(elem) => *elem,
                    _ => Type::Unknown,
                };
                self.push_scope();
                self.define(name, elem, false, iter.span())?;
                self.loop_depth += 1;
                self.check_block(body)?;
                self.loop_depth -= 1;
                self.pop_scope();
                Ok(unit_stmt())
            }
            Stmt::Break { span } => {
                if self.loop_depth == 0 {
                    return Err(error("'break' is only valid inside a loop", span));
                }
                Ok(unit_stmt())
            }
            Stmt::Continue { span } => {
                if self.loop_depth == 0 {
                    return Err(error("'continue' is only valid inside a loop", span));
                }
                Ok(unit_stmt())
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> LangResult<Type> {
        match expr {
            Expr::Int(_, _) => Ok(Type::Int),
            Expr::Float(_, _) => Ok(Type::Float),
            Expr::Bool(_, _) => Ok(Type::Bool),
            Expr::String(_, _) => Ok(Type::String),
            Expr::Unit(_) => Ok(Type::Unit),
            Expr::Ident(name, span) => {
                if let Some(symbol) = self.resolve(name) {
                    return Ok(symbol.ty);
                }
                if let Some(function) = self.functions.get(name) {
                    return Ok(function.clone());
                }
                if let Some((enum_name, arity)) = self.enum_variants.get(name).cloned() {
                    if arity == 0 {
                        return Ok(Type::Named(enum_name));
                    }
                    return Ok(Type::Function(
                        vec![Type::Unknown; arity],
                        Box::new(Type::Named(enum_name)),
                    ));
                }
                if matches!(
                    name.as_str(),
                    "print" | "println" | "eprint" | "input"
                    | "len" | "push" | "pop" | "insert" | "remove"
                    | "reverse" | "sort" | "sorted" | "reversed"
                    | "sum" | "any" | "all" | "map" | "filter" | "reduce"
                    | "enumerate" | "zip" | "range"
                    | "split" | "join" | "upper" | "lower" | "strip"
                    | "lstrip" | "rstrip" | "starts_with" | "ends_with"
                    | "contains" | "find" | "replace" | "chars"
                    | "str" | "int" | "float" | "bool"
                    | "abs" | "min" | "max" | "pow" | "round" | "floor"
                    | "ceil" | "sqrt" | "hex" | "oct" | "bin" | "ord" | "chr"
                    | "divmod" | "type_of" | "is_int" | "is_float"
                    | "is_string" | "is_bool" | "is_list" | "is_dict" | "is_none"
                    | "assert" | "exit" | "dict"
                    | "to_string" | "read_file" | "write_file"
                    | "args" | "getenv" | "panic"
                    | "is_digit" | "is_alpha" | "is_alnum" | "parse_int"
                    | "string_eq" | "none" | "null"
                ) {
                    return Ok(Type::Function(vec![Type::Unknown], Box::new(Type::Unknown)));
                }
                if matches!(name.as_str(), "Box" | "Rc" | "Arc") {
                    return Ok(Type::Function(
                        vec![Type::Unknown],
                        Box::new(Type::Generic(name.clone(), vec![Type::Unknown])),
                    ));
                }
                if name == "Channel" {
                    return Ok(Type::Function(
                        vec![Type::Unknown],
                        Box::new(Type::Generic("Channel".into(), vec![Type::Unknown])),
                    ));
                }
                if name == "Cancel" {
                    return Ok(Type::Function(
                        vec![],
                        Box::new(Type::Named("Cancel".into())),
                    ));
                }
                if name == "sleep" {
                    return Ok(Type::Function(vec![Type::Int], Box::new(Type::Unit)));
                }
                Err(error(format!("unknown name '{name}'"), span))
            }
            Expr::Path(_, _) => Ok(Type::Unknown),
            Expr::List(items, _) => {
                let mut elem = Type::Unknown;
                for item in items {
                    let ty = self.check_expr(item)?;
                    if elem == Type::Unknown {
                        elem = ty;
                    }
                }
                Ok(Type::List(Box::new(elem)))
            }
            Expr::Range { start, end, span } => {
                let s = self.check_expr(start)?;
                let e = self.check_expr(end)?;
                self.expect_assignable(&Type::Int, &s, span)?;
                self.expect_assignable(&Type::Int, &e, span)?;
                Ok(Type::Range(Box::new(Type::Int)))
            }
            Expr::Unary { op, expr, span } => {
                let ty = self.check_expr(expr)?;
                match op {
                    UnaryOp::Negate if ty.is_numeric() || ty == Type::Unknown => Ok(ty),
                    UnaryOp::Negate => Err(error(format!("cannot negate {ty}"), span)),
                    UnaryOp::Not => {
                        self.expect_assignable(&Type::Bool, &ty, span)?;
                        Ok(Type::Bool)
                    }
                }
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => self.check_binary(left, *op, right, span),
            Expr::Pipeline { left, right, span } => self.check_pipeline(left, right, span),
            Expr::Call { callee, args, span } => self.check_call(callee, args, span),
            Expr::MethodCall { target, args, .. } => {
                self.check_expr(target)?;
                for arg in args {
                    self.check_expr(arg)?;
                }
                Ok(Type::Unknown)
            }
            Expr::Field { target, .. } => {
                self.check_expr(target)?;
                Ok(Type::Unknown)
            }
            Expr::Index { target, index, .. } => {
                let target_ty = self.check_expr(target)?;
                self.check_expr(index)?;
                if let Type::List(elem) = target_ty {
                    Ok(*elem)
                } else {
                    Ok(Type::Unknown)
                }
            }
            Expr::StructLit { name, fields, .. } => {
                for (_, value) in fields {
                    self.check_expr(value)?;
                }
                Ok(Type::Named(name.clone()))
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let condition_ty = self.check_expr(condition)?;
                self.expect_assignable(&Type::Bool, &condition_ty, span)?;
                let then_ty = self.check_block(then_branch)?.value_type;
                let else_ty = if let Some(else_branch) = else_branch {
                    self.check_block(else_branch)?.value_type
                } else {
                    Type::Unit
                };
                self.expect_assignable(&then_ty, &else_ty, span)?;
                Ok(then_ty)
            }
            Expr::Match {
                scrutinee,
                arms,
                span,
            } => {
                let scrutinee_ty = self.check_expr(scrutinee)?;
                let mut result_ty: Option<Type> = None;
                for arm in arms {
                    self.push_scope();
                    self.check_pattern(&arm.pattern, &scrutinee_ty)?;
                    let arm_ty = self.check_expr(&arm.body)?;
                    self.pop_scope();
                    if let Some(expected) = &result_ty {
                        self.expect_assignable(expected, &arm_ty, &arm.span)?;
                    } else {
                        result_ty = Some(arm_ty);
                    }
                }
                result_ty.ok_or_else(|| error("match expression needs at least one arm", span))
            }
            Expr::Lambda {
                params,
                return_type,
                body,
                span,
            } => {
                let param_types = params
                    .iter()
                    .map(|param| self.type_from_ref(&param.ty, span))
                    .collect::<LangResult<Vec<_>>>()?;
                let previous_return = self.current_return.clone();
                self.push_scope();
                for (param, ty) in params.iter().zip(param_types.iter()) {
                    self.define(&param.name, ty.clone(), false, span.clone())?;
                }
                let inferred_return = match body {
                    LambdaBody::Expr(expr) => self.check_expr(expr)?,
                    LambdaBody::Block(block) => self.check_block(block)?.value_type,
                };
                self.pop_scope();
                self.current_return = previous_return;

                let result_ty = if let Some(return_ref) = return_type {
                    let annotated = self.type_from_ref(return_ref, span)?;
                    self.expect_assignable(&annotated, &inferred_return, span)?;
                    annotated
                } else {
                    inferred_return
                };
                Ok(Type::Function(param_types, Box::new(result_ty)))
            }
            Expr::Try { expr, .. } => {
                self.check_expr(expr)?;
                Ok(Type::Unknown)
            }
            Expr::Spawn { expr, .. } => {
                self.check_expr(expr)?;
                Ok(Type::Named("Task".to_string()))
            }
            Expr::Await { expr, .. } => {
                self.check_expr(expr)?;
                Ok(Type::Unknown)
            }
            Expr::Ref { expr, is_mut, .. } => {
                let inner = self.check_expr(expr)?;
                Ok(Type::Ref(Box::new(inner), *is_mut))
            }
            Expr::Region { body, .. } => Ok(self.check_block(body)?.value_type),
            Expr::Block(block) => Ok(self.check_block(block)?.value_type),
            Expr::Dict(pairs, _) => {
                for (k, v) in pairs {
                    self.check_expr(k)?;
                    self.check_expr(v)?;
                }
                Ok(Type::Named("Dict".to_string()))
            }
        }
    }

    fn check_binary(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
        span: &Span,
    ) -> LangResult<Type> {
        let left_ty = self.check_expr(left)?;
        let right_ty = self.check_expr(right)?;

        // Duck typing: if either side is Unknown, accept and return Unknown.
        if left_ty == Type::Unknown || right_ty == Type::Unknown {
            return Ok(Type::Unknown);
        }

        match op {
            BinaryOp::Add => {
                if left_ty == Type::String || right_ty == Type::String {
                    return Ok(Type::String);
                }
                self.expect_same_numeric(&left_ty, &right_ty, span)
            }
            BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => {
                self.expect_same_numeric(&left_ty, &right_ty, span)
            }
            BinaryOp::Equal | BinaryOp::NotEqual => {
                Ok(Type::Bool)
            }
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                self.expect_same_numeric(&left_ty, &right_ty, span)?;
                Ok(Type::Bool)
            }
            BinaryOp::And | BinaryOp::Or => {
                Ok(Type::Bool)
            }
        }
    }

    fn check_pipeline(&mut self, left: &Expr, right: &Expr, span: &Span) -> LangResult<Type> {
        let left_ty = self.check_expr(left)?;
        match right {
            Expr::Call { callee, args, .. } => {
                let mut arg_types = vec![left_ty];
                for arg in args {
                    arg_types.push(self.check_expr(arg)?);
                }
                self.check_call_types(callee, &arg_types, span)
            }
            _ => {
                let function_ty = self.check_expr(right)?;
                self.check_function_application(&function_ty, &[left_ty], span)
            }
        }
    }

    fn check_call(&mut self, callee: &Expr, args: &[Expr], span: &Span) -> LangResult<Type> {
        let arg_types = args
            .iter()
            .map(|arg| self.check_expr(arg))
            .collect::<LangResult<Vec<_>>>()?;
        self.check_call_types(callee, &arg_types, span)
    }

    fn check_call_types(
        &mut self,
        callee: &Expr,
        arg_types: &[Type],
        span: &Span,
    ) -> LangResult<Type> {
        let callee_ty = self.check_expr(callee)?;
        self.check_function_application(&callee_ty, arg_types, span)
    }

    fn check_function_application(
        &self,
        callee_ty: &Type,
        arg_types: &[Type],
        span: &Span,
    ) -> LangResult<Type> {
        if matches!(callee_ty, Type::Unknown) {
            return Ok(Type::Unknown);
        }
        let Type::Function(params, result) = callee_ty else {
            return Err(error(format!("{callee_ty} is not callable"), span));
        };

        if params.len() == 1 && params[0] == Type::Unknown {
            return Ok((**result).clone());
        }

        if params.len() != arg_types.len() {
            return Err(error(
                format!(
                    "expected {} argument(s), got {}",
                    params.len(),
                    arg_types.len()
                ),
                span,
            ));
        }

        for (expected, actual) in params.iter().zip(arg_types) {
            self.expect_assignable(expected, actual, span)?;
        }
        Ok((**result).clone())
    }

    fn check_pattern(&mut self, pattern: &Pattern, scrutinee_ty: &Type) -> LangResult<()> {
        match pattern {
            Pattern::Wildcard(_) => Ok(()),
            Pattern::Ident(name, span) => {
                self.define(name, scrutinee_ty.clone(), false, span.clone())?;
                Ok(())
            }
            Pattern::Int(_, span) => self.expect_assignable(scrutinee_ty, &Type::Int, span),
            Pattern::Float(_, span) => self.expect_assignable(scrutinee_ty, &Type::Float, span),
            Pattern::Bool(_, span) => self.expect_assignable(scrutinee_ty, &Type::Bool, span),
            Pattern::String(_, span) => self.expect_assignable(scrutinee_ty, &Type::String, span),
            Pattern::Unit(span) => self.expect_assignable(scrutinee_ty, &Type::Unit, span),
            Pattern::Variant { payload, span, .. } => {
                for inner in payload {
                    self.check_pattern(inner, &Type::Unknown)?;
                }
                let _ = span;
                Ok(())
            }
        }
    }

    fn expect_same_numeric(&self, left: &Type, right: &Type, span: &Span) -> LangResult<Type> {
        if left == &Type::Unknown {
            return Ok(right.clone());
        }
        if right == &Type::Unknown {
            return Ok(left.clone());
        }
        if left == right && left.is_numeric() {
            Ok(left.clone())
        } else {
            Err(error(
                format!("expected matching numeric types, got {left} and {right}"),
                span,
            ))
        }
    }

    fn expect_assignable(&self, expected: &Type, actual: &Type, span: &Span) -> LangResult<()> {
        if expected == actual || *expected == Type::Unknown || *actual == Type::Unknown {
            return Ok(());
        }
        // A named generic (Option[Int]) is compatible with the same nominal name (Option)
        // — full generic inference is deferred to the native compiler.
        match (expected, actual) {
            (Type::Generic(en, _), Type::Named(an)) if en == an => return Ok(()),
            (Type::Named(en), Type::Generic(an, _)) if en == an => return Ok(()),
            (Type::Generic(en, _), Type::Generic(an, _)) if en == an => return Ok(()),
            (Type::Ref(ei, em), Type::Ref(ai, am)) if em == am => {
                return self.expect_assignable(ei, ai, span);
            }
            (Type::List(ei), Type::List(ai)) => {
                return self.expect_assignable(ei, ai, span);
            }
            _ => {}
        }
        Err(error(format!("expected {expected}, got {actual}"), span))
    }

    fn type_from_ref(&self, ty: &TypeRef, span: &Span) -> LangResult<Type> {
        self.type_from_ref_with(ty, span, &HashSet::new())
    }

    fn type_from_ref_with(
        &self,
        ty: &TypeRef,
        span: &Span,
        generics: &HashSet<String>,
    ) -> LangResult<Type> {
        match ty {
            TypeRef::Named(name) => match name.as_str() {
                "Int" => Ok(Type::Int),
                "Float" => Ok(Type::Float),
                "Bool" => Ok(Type::Bool),
                "String" => Ok(Type::String),
                _ if generics.contains(name) => Ok(Type::Unknown),
                _ if self.known_types.contains(name) => Ok(Type::Named(name.clone())),
                _ => Err(error(format!("unknown type '{name}'"), span)),
            },
            TypeRef::Generic(name, args) => {
                let resolved_args = args
                    .iter()
                    .map(|t| self.type_from_ref_with(t, span, generics))
                    .collect::<LangResult<Vec<_>>>()?;
                if !self.known_types.contains(name) && !generics.contains(name) {
                    return Err(error(format!("unknown type '{name}'"), span));
                }
                Ok(Type::Generic(name.clone(), resolved_args))
            }
            TypeRef::Function(params, result) => {
                let params = params
                    .iter()
                    .map(|param| self.type_from_ref_with(param, span, generics))
                    .collect::<LangResult<Vec<_>>>()?;
                let result = self.type_from_ref_with(result, span, generics)?;
                Ok(Type::Function(params, Box::new(result)))
            }
            TypeRef::List(elem) => {
                let elem = self.type_from_ref_with(elem, span, generics)?;
                Ok(Type::List(Box::new(elem)))
            }
            TypeRef::Ref(inner, is_mut) => {
                let inner = self.type_from_ref_with(inner, span, generics)?;
                Ok(Type::Ref(Box::new(inner), *is_mut))
            }
            TypeRef::Unit => Ok(Type::Unit),
            TypeRef::Infer => Ok(Type::Unknown),
        }
    }

    fn define(&mut self, name: &str, ty: Type, mutable: bool, span: Span) -> LangResult<()> {
        let scope = self.scopes.last_mut().expect("scope stack is never empty");
        if scope.contains_key(name) {
            return Err(Diagnostic::new(
                format!("name '{name}' is already defined in this scope"),
                span,
            ));
        }
        scope.insert(name.to_string(), Symbol { ty, mutable });
        Ok(())
    }

    fn resolve(&self, name: &str) -> Option<Symbol> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

fn unit_stmt() -> StmtCheck {
    StmtCheck {
        value_type: Type::Unit,
        returns: false,
    }
}

fn error(message: impl Into<String>, span: &Span) -> Diagnostic {
    Diagnostic::new(message, span.clone())
}
