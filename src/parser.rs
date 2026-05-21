use crate::ast::*;
use crate::diagnostic::{Diagnostic, LangResult, Span};
use crate::token::{Keyword, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse_program(&mut self) -> LangResult<Program> {
        let mut items = Vec::new();
        while !self.is_at_end() {
            self.skip_indent_tokens();
            if self.is_at_end() {
                break;
            }
            self.parse_top_level_into(&mut items)?;
        }
        Ok(Program { items })
    }

    fn parse_top_level_into(&mut self, items: &mut Vec<Item>) -> LangResult<()> {
        let attrs = self.parse_outer_attrs()?;
        // Attributes may be on their own line(s); skip whitespace before the item.
        self.skip_indent_tokens();
        let is_pub = self.match_keyword(Keyword::Pub);
        self.skip_newlines();

        if self.check_keyword(Keyword::Actor) {
            if !attrs.is_empty() {
                return Err(Diagnostic::new(
                    "attributes are only supported on functions in this release",
                    self.peek().span.clone(),
                ));
            }
            self.parse_actor_sugar(items, is_pub)?;
            self.skip_indent_tokens();
            return Ok(());
        }

        let is_async = self.match_keyword(Keyword::Async);

        if self.check_keyword(Keyword::Fn) {
            items.push(Item::Function(
                self.parse_function_with_attrs(is_pub, is_async, attrs)?,
            ));
            self.skip_indent_tokens();
            return Ok(());
        }
        if !attrs.is_empty() {
            return Err(Diagnostic::new(
                "attributes are only supported on functions in this release",
                self.peek().span.clone(),
            ));
        }
        if is_async {
            return Err(Diagnostic::new(
                "'async' must be followed by a function declaration",
                self.peek().span.clone(),
            ));
        }
        if self.check_keyword(Keyword::Struct) {
            items.push(Item::Struct(self.parse_struct(is_pub)?));
            self.skip_indent_tokens();
            return Ok(());
        }
        if self.check_keyword(Keyword::Enum) {
            items.push(Item::Enum(self.parse_enum(is_pub)?));
            self.skip_indent_tokens();
            return Ok(());
        }
        if self.check_keyword(Keyword::Const) {
            items.push(Item::Const(self.parse_const_decl(is_pub)?));
            self.skip_indent_tokens();
            return Ok(());
        }
        if self.check_keyword(Keyword::Module) {
            items.push(Item::Module(self.parse_module(is_pub)?));
            self.skip_indent_tokens();
            return Ok(());
        }
        if self.check_keyword(Keyword::Import) || self.check_keyword(Keyword::Use) {
            items.push(Item::Import(self.parse_import()?));
            self.skip_indent_tokens();
            return Ok(());
        }
        if self.check_keyword(Keyword::Trait) {
            items.push(Item::Trait(self.parse_trait(is_pub)?));
            self.skip_indent_tokens();
            return Ok(());
        }
        if self.check_keyword(Keyword::Impl) {
            items.push(Item::Impl(self.parse_impl()?));
            self.skip_indent_tokens();
            return Ok(());
        }
        if self.check_keyword(Keyword::Extern) {
            items.push(Item::Extern(self.parse_extern()?));
            self.skip_indent_tokens();
            return Ok(());
        }

        items.push(Item::Statement(self.parse_statement()?));
        self.skip_indent_tokens();
        Ok(())
    }

    // ── Actor sugar ───────────────────────────────────────────────────────────

    fn parse_actor_sugar(&mut self, items: &mut Vec<Item>, is_pub: bool) -> LangResult<()> {
        let span = self.expect_keyword(Keyword::Actor, "expected 'actor'")?;
        let name = self.expect_ident("expected actor name")?;

        let mut fields: Vec<Field> = Vec::new();
        let mut receive_arms: Vec<MatchArm> = Vec::new();
        let mut saw_receive = false;

        // Both old `{ }` and new `:` + indent styles
        if self.match_kind(&TokenKind::LeftBrace) {
            // Old brace style
            self.skip_indent_tokens();
            while !self.check_kind(&TokenKind::RightBrace) && !self.is_at_end() {
                if self.match_keyword(Keyword::Receive) {
                    saw_receive = true;
                    receive_arms = self.parse_receive_arms_brace()?;
                    self.skip_indent_tokens();
                    continue;
                }
                let state_ident = self.expect_ident("expected 'state' or 'receive' in actor body")?;
                if state_ident != "state" {
                    return Err(Diagnostic::new(
                        format!("expected 'state' or 'receive', got '{state_ident}'"),
                        self.previous().span.clone(),
                    ));
                }
                let field_name = self.expect_ident("expected field name after 'state'")?;
                self.expect_kind(&TokenKind::Colon, "expected ':' after field name")?;
                let ty = self.parse_type_ref()?;
                fields.push(Field { name: field_name, ty });
                self.match_kind(&TokenKind::Comma);
                self.skip_indent_tokens();
            }
            self.expect_kind(&TokenKind::RightBrace, "expected '}' to close actor")?;
        } else {
            // New indent style
            self.expect_kind(&TokenKind::Colon, "expected ':' or '{' after actor name")?;
            self.match_kind(&TokenKind::Newline);
            self.expect_kind(&TokenKind::Indent, "expected indented actor body")?;
            while !self.check_kind(&TokenKind::Dedent) && !self.is_at_end() {
                self.skip_newlines();
                if self.check_kind(&TokenKind::Dedent) { break; }
                if self.match_keyword(Keyword::Receive) {
                    saw_receive = true;
                    receive_arms = self.parse_receive_arms_indent()?;
                    self.match_kind(&TokenKind::Newline);
                    continue;
                }
                let state_ident = self.expect_ident("expected 'state' or 'receive' in actor body")?;
                if state_ident != "state" {
                    return Err(Diagnostic::new(
                        format!("expected 'state' or 'receive', got '{state_ident}'"),
                        self.previous().span.clone(),
                    ));
                }
                let field_name = self.expect_ident("expected field name after 'state'")?;
                self.expect_kind(&TokenKind::Colon, "expected ':' after field name")?;
                let ty = self.parse_type_ref()?;
                fields.push(Field { name: field_name, ty });
                self.match_kind(&TokenKind::Newline);
            }
            self.match_kind(&TokenKind::Dedent);
        }

        items.push(Item::Struct(StructDecl {
            name: name.clone(),
            generics: Vec::new(),
            fields,
            is_pub,
            span: span.clone(),
        }));

        if saw_receive {
            let step_body = Block {
                statements: vec![Stmt::Expr {
                    expr: Expr::Match {
                        scrutinee: Box::new(Expr::Ident("msg".to_string(), span.clone())),
                        arms: receive_arms,
                        span: span.clone(),
                    },
                    has_semicolon: false,
                    span: span.clone(),
                }],
                span: span.clone(),
            };
            let step_fn = FunctionDecl {
                name: "step".to_string(),
                generics: Vec::new(),
                params: vec![
                    Param { name: "self".to_string(), ty: TypeRef::Infer },
                    Param { name: "msg".to_string(), ty: TypeRef::Infer },
                ],
                return_type: TypeRef::Named(name.clone()),
                body: step_body,
                is_async: false,
                is_pub: true,
                attrs: Vec::new(),
                span: span.clone(),
            };
            items.push(Item::Impl(ImplBlock {
                trait_name: None,
                target: name,
                generics: Vec::new(),
                methods: vec![step_fn],
                span,
            }));
        }
        Ok(())
    }

    fn parse_receive_arms_brace(&mut self) -> LangResult<Vec<MatchArm>> {
        self.expect_kind(&TokenKind::LeftBrace, "expected '{' before receive arms")?;
        let mut arms = Vec::new();
        self.skip_indent_tokens();
        while !self.check_kind(&TokenKind::RightBrace) && !self.is_at_end() {
            let arm_span = self.peek().span.clone();
            let pattern = self.parse_pattern()?;
            self.expect_kind(&TokenKind::FatArrow, "expected '=>' in receive arm")?;
            let body = self.parse_expression(0)?;
            arms.push(MatchArm { pattern, body, span: arm_span });
            self.match_kind(&TokenKind::Comma);
            self.skip_indent_tokens();
        }
        self.expect_kind(&TokenKind::RightBrace, "expected '}' after receive arms")?;
        Ok(arms)
    }

    fn parse_receive_arms_indent(&mut self) -> LangResult<Vec<MatchArm>> {
        self.expect_kind(&TokenKind::Colon, "expected ':' before receive arms")?;
        self.match_kind(&TokenKind::Newline);
        self.expect_kind(&TokenKind::Indent, "expected indented receive arms")?;
        let mut arms = Vec::new();
        while !self.check_kind(&TokenKind::Dedent) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_kind(&TokenKind::Dedent) { break; }
            let arm_span = self.peek().span.clone();
            let pattern = self.parse_pattern()?;
            self.expect_kind(&TokenKind::FatArrow, "expected '=>' in receive arm")?;
            let body = self.parse_expression(0)?;
            arms.push(MatchArm { pattern, body, span: arm_span });
            self.match_kind(&TokenKind::Newline);
        }
        self.match_kind(&TokenKind::Dedent);
        Ok(arms)
    }

    // ── Function ──────────────────────────────────────────────────────────────

    fn parse_function(&mut self, is_pub: bool, is_async: bool) -> LangResult<FunctionDecl> {
        self.parse_function_with_attrs(is_pub, is_async, Vec::new())
    }

    fn parse_function_with_attrs(
        &mut self,
        is_pub: bool,
        is_async: bool,
        attrs: Vec<String>,
    ) -> LangResult<FunctionDecl> {
        let span = self.expect_keyword(Keyword::Fn, "expected 'fn'")?;
        let name = self.expect_ident("expected function name")?;
        let generics = self.parse_generics()?;
        let params = self.parse_params()?;
        let return_type = if self.match_kind(&TokenKind::Arrow) {
            self.parse_type_ref()?
        } else {
            TypeRef::Unit
        };
        self.skip_where_clauses()?;
        let body = self.parse_block()?;
        Ok(FunctionDecl {
            name,
            generics,
            params,
            return_type,
            body,
            is_async,
            is_pub,
            attrs,
            span,
        })
    }

    fn parse_outer_attrs(&mut self) -> LangResult<Vec<String>> {
        let mut attrs = Vec::new();
        while self.check_kind(&TokenKind::Hash) {
            self.advance();
            self.expect_kind(&TokenKind::LeftBracket, "expected '[' after '#'")?;
            let name = self.expect_ident("expected attribute name after '#['")?;
            self.expect_kind(&TokenKind::RightBracket, "expected ']' to close attribute")?;
            attrs.push(name);
        }
        Ok(attrs)
    }

    fn parse_params(&mut self) -> LangResult<Vec<Param>> {
        self.expect_kind(&TokenKind::LeftParen, "expected '('")?;
        let mut params = Vec::new();
        if !self.check_kind(&TokenKind::RightParen) {
            loop {
                if self.match_keyword(Keyword::Self_) {
                    params.push(Param { name: "self".to_string(), ty: TypeRef::Infer });
                } else {
                    let name = self.expect_ident("expected parameter name")?;
                    // Type annotation is optional (duck typing)
                    let ty = if self.match_kind(&TokenKind::Colon) {
                        self.parse_type_ref()?
                    } else {
                        TypeRef::Infer
                    };
                    params.push(Param { name, ty });
                }
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
                if self.check_kind(&TokenKind::RightParen) {
                    break;
                }
            }
        }
        self.expect_kind(&TokenKind::RightParen, "expected ')' after parameters")?;
        Ok(params)
    }

    // ── Struct ────────────────────────────────────────────────────────────────

    fn parse_struct(&mut self, is_pub: bool) -> LangResult<StructDecl> {
        let span = self.expect_keyword(Keyword::Struct, "expected 'struct'")?;
        let name = self.expect_ident("expected struct name")?;
        let generics = self.parse_generics()?;
        let fields = self.parse_struct_body()?;
        Ok(StructDecl { name, generics, fields, is_pub, span })
    }

    fn parse_struct_body(&mut self) -> LangResult<Vec<Field>> {
        if self.match_kind(&TokenKind::LeftBrace) {
            // Old brace style
            let mut fields = Vec::new();
            self.skip_indent_tokens();
            while !self.check_kind(&TokenKind::RightBrace) && !self.is_at_end() {
                self.match_keyword(Keyword::Pub);
                let field_name = self.expect_ident("expected field name")?;
                self.expect_kind(&TokenKind::Colon, "expected ':' after field name")?;
                let ty = self.parse_type_ref()?;
                fields.push(Field { name: field_name, ty });
                self.match_kind(&TokenKind::Comma);
                self.skip_indent_tokens();
            }
            self.expect_kind(&TokenKind::RightBrace, "expected '}' after struct fields")?;
            Ok(fields)
        } else {
            // New Python style: struct Name:\n    field: Type
            self.expect_kind(&TokenKind::Colon, "expected ':' or '{' for struct body")?;
            self.match_kind(&TokenKind::Newline);
            self.expect_kind(&TokenKind::Indent, "expected indented struct fields")?;
            let mut fields = Vec::new();
            while !self.check_kind(&TokenKind::Dedent) && !self.is_at_end() {
                self.skip_newlines();
                if self.check_kind(&TokenKind::Dedent) { break; }
                self.match_keyword(Keyword::Pub);
                let field_name = self.expect_ident("expected field name")?;
                self.expect_kind(&TokenKind::Colon, "expected ':' after field name")?;
                let ty = self.parse_type_ref()?;
                fields.push(Field { name: field_name, ty });
                self.match_kind(&TokenKind::Newline);
            }
            self.match_kind(&TokenKind::Dedent);
            Ok(fields)
        }
    }

    // ── Enum ──────────────────────────────────────────────────────────────────

    fn parse_enum(&mut self, is_pub: bool) -> LangResult<EnumDecl> {
        let span = self.expect_keyword(Keyword::Enum, "expected 'enum'")?;
        let name = self.expect_ident("expected enum name")?;
        let generics = self.parse_generics()?;
        let variants = self.parse_enum_body()?;
        Ok(EnumDecl { name, generics, variants, is_pub, span })
    }

    fn parse_enum_body(&mut self) -> LangResult<Vec<Variant>> {
        let use_brace = self.match_kind(&TokenKind::LeftBrace);
        if !use_brace {
            self.expect_kind(&TokenKind::Colon, "expected ':' or '{' for enum body")?;
            self.match_kind(&TokenKind::Newline);
            self.expect_kind(&TokenKind::Indent, "expected indented enum variants")?;
        } else {
            self.skip_indent_tokens();
        }

        let mut variants = Vec::new();
        let done = if use_brace {
            TokenKind::RightBrace
        } else {
            TokenKind::Dedent
        };

        while !self.check_kind(&done) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_kind(&done) { break; }
            let variant_name = self.expect_ident("expected variant name")?;
            let mut payload = Vec::new();
            if self.match_kind(&TokenKind::LeftParen) {
                if !self.check_kind(&TokenKind::RightParen) {
                    loop {
                        payload.push(self.parse_type_ref()?);
                        if !self.match_kind(&TokenKind::Comma) { break; }
                        if self.check_kind(&TokenKind::RightParen) { break; }
                    }
                }
                self.expect_kind(&TokenKind::RightParen, "expected ')' after variant payload")?;
            }
            variants.push(Variant { name: variant_name, payload });
            if use_brace {
                self.match_kind(&TokenKind::Comma);
                self.skip_indent_tokens();
            } else {
                self.match_kind(&TokenKind::Newline);
            }
        }

        if use_brace {
            self.expect_kind(&TokenKind::RightBrace, "expected '}' after enum variants")?;
        } else {
            self.match_kind(&TokenKind::Dedent);
        }
        Ok(variants)
    }

    // ── Generics / where ──────────────────────────────────────────────────────

    fn parse_generics(&mut self) -> LangResult<Vec<String>> {
        let mut generics = Vec::new();
        if !self.match_kind(&TokenKind::LeftBracket) {
            return Ok(generics);
        }
        if !self.check_kind(&TokenKind::RightBracket) {
            loop {
                generics.push(self.expect_ident("expected generic parameter name")?);
                if !self.match_kind(&TokenKind::Comma) { break; }
                if self.check_kind(&TokenKind::RightBracket) { break; }
            }
        }
        self.expect_kind(&TokenKind::RightBracket, "expected ']' after generic parameters")?;
        Ok(generics)
    }

    fn skip_where_clauses(&mut self) -> LangResult<()> {
        if !self.match_keyword(Keyword::Where) {
            return Ok(());
        }
        // Skip tokens until we reach a block opener: `{` or a `:` before Newline
        loop {
            if self.is_at_end() { break; }
            if self.check_kind(&TokenKind::LeftBrace) { break; }
            if self.check_kind(&TokenKind::Colon) {
                // Check if the colon is the block-opening colon (before newline)
                if matches!(
                    self.peek_n(1).map(|t| &t.kind),
                    Some(TokenKind::Newline) | Some(TokenKind::Eof) | None
                ) {
                    break;
                }
            }
            self.advance();
        }
        Ok(())
    }

    // ── Const ─────────────────────────────────────────────────────────────────

    fn parse_const_decl(&mut self, is_pub: bool) -> LangResult<ConstDecl> {
        let span = self.expect_keyword(Keyword::Const, "expected 'const'")?;
        let name = self.expect_ident("expected constant name")?;
        let ty = if self.match_kind(&TokenKind::Colon) {
            Some(self.parse_type_ref()?)
        } else {
            None
        };
        self.expect_kind(&TokenKind::Equal, "expected '=' in constant declaration")?;
        let value = self.parse_expression(0)?;
        self.match_kind(&TokenKind::Semicolon);
        Ok(ConstDecl { name, ty, value, is_pub, span })
    }

    // ── Module ────────────────────────────────────────────────────────────────

    fn parse_module(&mut self, is_pub: bool) -> LangResult<ModuleDecl> {
        let span = self.expect_keyword(Keyword::Module, "expected 'module'")?;
        let name = self.expect_ident("expected module name")?;

        let mut items = Vec::new();
        if self.match_kind(&TokenKind::LeftBrace) {
            self.skip_indent_tokens();
            while !self.check_kind(&TokenKind::RightBrace) && !self.is_at_end() {
                self.skip_indent_tokens();
                if self.check_kind(&TokenKind::RightBrace) { break; }
                self.parse_top_level_into(&mut items)?;
            }
            self.expect_kind(&TokenKind::RightBrace, "expected '}' to close module")?;
        } else {
            self.expect_kind(&TokenKind::Colon, "expected ':' or '{' to open module")?;
            self.match_kind(&TokenKind::Newline);
            self.expect_kind(&TokenKind::Indent, "expected indented module body")?;
            while !self.check_kind(&TokenKind::Dedent) && !self.is_at_end() {
                self.skip_newlines();
                if self.check_kind(&TokenKind::Dedent) { break; }
                self.parse_top_level_into(&mut items)?;
            }
            self.match_kind(&TokenKind::Dedent);
        }
        Ok(ModuleDecl { name, items, is_pub, span })
    }

    // ── Import ────────────────────────────────────────────────────────────────

    fn parse_import(&mut self) -> LangResult<ImportDecl> {
        let span = if self.match_keyword(Keyword::Import) {
            self.previous().span.clone()
        } else {
            self.expect_keyword(Keyword::Use, "expected 'import' or 'use'")?
        };
        let mut path = vec![self.expect_ident("expected import path")?];
        while self.match_kind(&TokenKind::Dot) || self.match_kind(&TokenKind::ColonColon) {
            if self.check_kind(&TokenKind::LeftBrace) { break; }
            path.push(self.expect_ident("expected import segment")?);
        }
        let mut items = Vec::new();
        if self.match_kind(&TokenKind::LeftBrace) {
            if !self.check_kind(&TokenKind::RightBrace) {
                loop {
                    items.push(self.expect_ident("expected imported name")?);
                    if !self.match_kind(&TokenKind::Comma) { break; }
                    if self.check_kind(&TokenKind::RightBrace) { break; }
                }
            }
            self.expect_kind(&TokenKind::RightBrace, "expected '}' in import list")?;
        }
        self.match_kind(&TokenKind::Semicolon);
        Ok(ImportDecl { path, items, span })
    }

    // ── Trait ─────────────────────────────────────────────────────────────────

    fn parse_trait(&mut self, is_pub: bool) -> LangResult<TraitDecl> {
        let span = self.expect_keyword(Keyword::Trait, "expected 'trait'")?;
        let name = self.expect_ident("expected trait name")?;
        let generics = self.parse_generics()?;
        let methods = self.parse_trait_body()?;
        Ok(TraitDecl { name, generics, methods, is_pub, span })
    }

    fn parse_trait_body(&mut self) -> LangResult<Vec<TraitMethod>> {
        let use_brace = self.match_kind(&TokenKind::LeftBrace);
        if !use_brace {
            self.expect_kind(&TokenKind::Colon, "expected ':' or '{' to open trait")?;
            self.match_kind(&TokenKind::Newline);
            self.expect_kind(&TokenKind::Indent, "expected indented trait body")?;
        } else {
            self.skip_indent_tokens();
        }

        let done = if use_brace { TokenKind::RightBrace } else { TokenKind::Dedent };
        let mut methods = Vec::new();

        while !self.check_kind(&done) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_kind(&done) { break; }
            self.match_keyword(Keyword::Pub);
            let method_span = self.expect_keyword(Keyword::Fn, "expected 'fn' in trait method")?;
            let method_name = self.expect_ident("expected method name")?;
            let _ = self.parse_generics()?;
            let params = self.parse_params()?;
            let return_type = if self.match_kind(&TokenKind::Arrow) {
                self.parse_type_ref()?
            } else {
                TypeRef::Unit
            };
            self.match_kind(&TokenKind::Semicolon);
            if !use_brace { self.match_kind(&TokenKind::Newline); }
            methods.push(TraitMethod { name: method_name, params, return_type, span: method_span });
        }

        if use_brace {
            self.expect_kind(&TokenKind::RightBrace, "expected '}' to close trait")?;
        } else {
            self.match_kind(&TokenKind::Dedent);
        }
        Ok(methods)
    }

    // ── Impl ──────────────────────────────────────────────────────────────────

    fn parse_impl(&mut self) -> LangResult<ImplBlock> {
        let span = self.expect_keyword(Keyword::Impl, "expected 'impl'")?;
        let generics = self.parse_generics()?;
        let first = self.expect_ident("expected type or trait name")?;
        let (trait_name, target) = if self.match_keyword(Keyword::For) {
            let target = self.expect_ident("expected target type name")?;
            if self.match_kind(&TokenKind::LeftBracket) {
                while !self.check_kind(&TokenKind::RightBracket) && !self.is_at_end() {
                    self.advance();
                }
                self.expect_kind(&TokenKind::RightBracket, "expected ']' in impl target")?;
            }
            (Some(first), target)
        } else {
            if self.match_kind(&TokenKind::LeftBracket) {
                while !self.check_kind(&TokenKind::RightBracket) && !self.is_at_end() {
                    self.advance();
                }
                self.expect_kind(&TokenKind::RightBracket, "expected ']' in impl target")?;
            }
            (None, first)
        };

        let methods = self.parse_impl_body()?;
        Ok(ImplBlock { trait_name, target, generics, methods, span })
    }

    fn parse_impl_body(&mut self) -> LangResult<Vec<FunctionDecl>> {
        let use_brace = self.match_kind(&TokenKind::LeftBrace);
        if !use_brace {
            self.expect_kind(&TokenKind::Colon, "expected ':' or '{' to open impl block")?;
            self.match_kind(&TokenKind::Newline);
            self.expect_kind(&TokenKind::Indent, "expected indented impl body")?;
        } else {
            self.skip_indent_tokens();
        }

        let done = if use_brace { TokenKind::RightBrace } else { TokenKind::Dedent };
        let mut methods = Vec::new();

        while !self.check_kind(&done) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_kind(&done) { break; }
            let is_pub = self.match_keyword(Keyword::Pub);
            let is_async = self.match_keyword(Keyword::Async);
            methods.push(self.parse_function(is_pub, is_async)?);
            if !use_brace { self.skip_newlines(); }
        }

        if use_brace {
            self.expect_kind(&TokenKind::RightBrace, "expected '}' to close impl block")?;
        } else {
            self.match_kind(&TokenKind::Dedent);
        }
        Ok(methods)
    }

    // ── Extern ────────────────────────────────────────────────────────────────

    fn parse_extern(&mut self) -> LangResult<ExternBlock> {
        let span = self.expect_keyword(Keyword::Extern, "expected 'extern'")?;
        let language = self.expect_ident("expected language tag, e.g. c or cpp")?;
        let library = if let TokenKind::String(value) = &self.peek().kind {
            let lib = value.clone();
            self.advance();
            Some(lib)
        } else {
            None
        };

        let use_brace = self.match_kind(&TokenKind::LeftBrace);
        if !use_brace {
            self.expect_kind(&TokenKind::Colon, "expected ':' or '{' to open extern block")?;
            self.match_kind(&TokenKind::Newline);
            self.expect_kind(&TokenKind::Indent, "expected indented extern body")?;
        } else {
            self.skip_indent_tokens();
        }

        let done = if use_brace { TokenKind::RightBrace } else { TokenKind::Dedent };
        let mut items = Vec::new();

        while !self.check_kind(&done) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_kind(&done) { break; }
            let item_span = self.expect_keyword(Keyword::Fn, "expected 'fn' in extern block")?;
            let name = self.expect_ident("expected extern function name")?;
            let params = self.parse_params()?;
            let return_type = if self.match_kind(&TokenKind::Arrow) {
                self.parse_type_ref()?
            } else {
                TypeRef::Unit
            };
            self.match_kind(&TokenKind::Semicolon);
            if !use_brace { self.match_kind(&TokenKind::Newline); }
            items.push(ExternItem { name, params, return_type, span: item_span });
        }

        if use_brace {
            self.expect_kind(&TokenKind::RightBrace, "expected '}' to close extern block")?;
        } else {
            self.match_kind(&TokenKind::Dedent);
        }
        Ok(ExternBlock { language, library, items, span })
    }

    // ── Block ─────────────────────────────────────────────────────────────────

    /// Parse a block: either old-style `{ ... }` or new Python-style `:\n indent ... dedent`.
    fn parse_block(&mut self) -> LangResult<Block> {
        if self.check_kind(&TokenKind::LeftBrace) {
            self.parse_block_brace()
        } else {
            self.parse_block_indent()
        }
    }

    /// Old brace-style block `{ stmts }`.
    fn parse_block_brace(&mut self) -> LangResult<Block> {
        let span = self.expect_kind(&TokenKind::LeftBrace, "expected '{'")?;
        let mut statements = Vec::new();
        loop {
            self.skip_indent_tokens();
            if self.check_kind(&TokenKind::RightBrace) || self.is_at_end() { break; }
            statements.push(self.parse_statement()?);
        }
        self.expect_kind(&TokenKind::RightBrace, "expected '}' after block")?;
        Ok(Block { statements, span })
    }

    /// New Python-style block `:\n indent stmts dedent`.
    fn parse_block_indent(&mut self) -> LangResult<Block> {
        let span = self.peek().span.clone();
        self.expect_kind(&TokenKind::Colon, "expected ':' or '{' to begin block")?;
        self.match_kind(&TokenKind::Newline);
        self.expect_kind(&TokenKind::Indent, "expected indented block after ':'")?;

        let mut statements = Vec::new();
        loop {
            self.skip_newlines();
            if self.check_kind(&TokenKind::Dedent) || self.is_at_end() { break; }
            statements.push(self.parse_statement()?);
            self.match_kind(&TokenKind::Newline);
        }
        self.match_kind(&TokenKind::Dedent);
        Ok(Block { statements, span })
    }

    // ── Statements ────────────────────────────────────────────────────────────

    fn parse_statement(&mut self) -> LangResult<Stmt> {
        if self.check_keyword(Keyword::Let) {
            return self.parse_let_statement();
        }
        if self.check_keyword(Keyword::Const) {
            return Ok(Stmt::Const(self.parse_const_decl(false)?));
        }
        if self.check_keyword(Keyword::Return) {
            return self.parse_return_statement();
        }
        if self.check_keyword(Keyword::While) {
            return self.parse_while_statement();
        }
        if self.check_keyword(Keyword::For) {
            return self.parse_for_statement();
        }
        if self.match_keyword(Keyword::Break) {
            let span = self.previous().span.clone();
            self.match_kind(&TokenKind::Semicolon);
            return Ok(Stmt::Break { span });
        }
        if self.match_keyword(Keyword::Continue) {
            let span = self.previous().span.clone();
            self.match_kind(&TokenKind::Semicolon);
            return Ok(Stmt::Continue { span });
        }

        let span = self.peek().span.clone();
        let expr = self.parse_expression(0)?;
        if self.match_kind(&TokenKind::Equal) {
            let target = self.expr_to_assign_target(expr, &span)?;
            let value = self.parse_expression(0)?;
            self.match_kind(&TokenKind::Semicolon);
            return Ok(Stmt::Assign { target, value, span });
        }
        let has_semicolon = self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::Expr { expr, has_semicolon, span })
    }

    fn expr_to_assign_target(&self, expr: Expr, span: &Span) -> LangResult<AssignTarget> {
        match expr {
            Expr::Ident(name, _) => Ok(AssignTarget::Name(name)),
            Expr::Field { target, name, .. } => {
                Ok(AssignTarget::Field { target: *target, name })
            }
            Expr::Index { target, index, .. } => {
                Ok(AssignTarget::Index { target: *target, index: *index })
            }
            _ => Err(Diagnostic::new(
                "left-hand side of '=' must be a name, field, or index",
                span.clone(),
            )),
        }
    }

    fn parse_let_statement(&mut self) -> LangResult<Stmt> {
        let span = self.expect_keyword(Keyword::Let, "expected 'let'")?;
        let mutable = self.match_keyword(Keyword::Mut);
        let name = self.expect_ident("expected binding name")?;
        let ty = if self.match_kind(&TokenKind::Colon) {
            Some(self.parse_type_ref()?)
        } else {
            None
        };
        self.expect_kind(&TokenKind::Equal, "expected '=' in binding declaration")?;
        let value = self.parse_expression(0)?;
        self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::Let { name, ty, mutable, value, span })
    }

    fn parse_return_statement(&mut self) -> LangResult<Stmt> {
        let span = self.expect_keyword(Keyword::Return, "expected 'return'")?;
        let value = if self.check_kind(&TokenKind::Semicolon)
            || self.check_kind(&TokenKind::RightBrace)
            || self.check_kind(&TokenKind::Newline)
            || self.check_kind(&TokenKind::Dedent)
            || self.is_at_end()
        {
            None
        } else {
            Some(self.parse_expression(0)?)
        };
        self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::Return { value, span })
    }

    fn parse_while_statement(&mut self) -> LangResult<Stmt> {
        let span = self.expect_keyword(Keyword::While, "expected 'while'")?;
        let condition = self.parse_control_expression()?;
        let body = self.parse_block()?;
        Ok(Stmt::While { condition, body, span })
    }

    fn parse_for_statement(&mut self) -> LangResult<Stmt> {
        let span = self.expect_keyword(Keyword::For, "expected 'for'")?;
        let name = self.expect_ident("expected binding name in 'for'")?;
        self.expect_keyword(Keyword::In, "expected 'in' after 'for' binding")?;
        let iter = self.parse_control_expression()?;
        let body = self.parse_block()?;
        Ok(Stmt::For { name, iter, body, span })
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    fn parse_control_expression(&mut self) -> LangResult<Expr> {
        self.parse_expression_with(0, false)
    }

    fn parse_expression(&mut self, min_bp: u8) -> LangResult<Expr> {
        self.parse_expression_with(min_bp, true)
    }

    fn parse_expression_with(&mut self, min_bp: u8, allow_struct_lit: bool) -> LangResult<Expr> {
        let mut left = self.parse_prefix(allow_struct_lit)?;

        loop {
            // Postfix operators
            if self.check_kind(&TokenKind::LeftParen) {
                if 18 < min_bp { break; }
                left = self.finish_call(left)?;
                continue;
            }
            if self.check_kind(&TokenKind::Dot) {
                if 18 < min_bp { break; }
                left = self.finish_member(left)?;
                continue;
            }
            if self.check_kind(&TokenKind::LeftBracket) {
                if 18 < min_bp { break; }
                left = self.finish_index(left)?;
                continue;
            }
            if self.check_kind(&TokenKind::Question) {
                if 17 < min_bp { break; }
                let span = self.peek().span.clone();
                self.advance();
                left = Expr::Try { expr: Box::new(left), span };
                continue;
            }

            let Some((op, left_bp, right_bp)) = self.current_infix() else { break };
            if left_bp < min_bp { break; }

            let span = self.peek().span.clone();
            self.advance();
            let right = self.parse_expression_with(right_bp, allow_struct_lit)?;

            left = match op {
                InfixOp::Pipeline => Expr::Pipeline {
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                },
                InfixOp::Range => Expr::Range {
                    start: Box::new(left),
                    end: Box::new(right),
                    span,
                },
                other => Expr::Binary {
                    left: Box::new(left),
                    op: other.into_binary().expect("binary operator"),
                    right: Box::new(right),
                    span,
                },
            };
        }
        Ok(left)
    }

    fn parse_prefix(&mut self, allow_struct_lit: bool) -> LangResult<Expr> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Int(value) => Ok(Expr::Int(value, token.span)),
            TokenKind::Float(value) => Ok(Expr::Float(value, token.span)),
            TokenKind::String(value) => Ok(Expr::String(value, token.span)),
            TokenKind::Ident(name) => {
                // struct literal: Name { field: value, ... }
                if allow_struct_lit
                    && self.check_kind(&TokenKind::LeftBrace)
                    && self.looks_like_struct_lit()
                {
                    return self.parse_struct_lit(name, token.span);
                }
                Ok(Expr::Ident(name, token.span))
            }
            TokenKind::Keyword(Keyword::True) => Ok(Expr::Bool(true, token.span)),
            TokenKind::Keyword(Keyword::False) => Ok(Expr::Bool(false, token.span)),
            TokenKind::Keyword(Keyword::None_) => Ok(Expr::Unit(token.span)),
            TokenKind::Keyword(Keyword::Self_) => {
                Ok(Expr::Ident("self".to_string(), token.span))
            }
            TokenKind::Bang | TokenKind::Keyword(Keyword::Not) => {
                let expr = self.parse_expression_with(15, allow_struct_lit)?;
                Ok(Expr::Unary { op: UnaryOp::Not, expr: Box::new(expr), span: token.span })
            }
            TokenKind::Minus => {
                let expr = self.parse_expression_with(15, allow_struct_lit)?;
                Ok(Expr::Unary { op: UnaryOp::Negate, expr: Box::new(expr), span: token.span })
            }
            TokenKind::Amp => {
                let is_mut = self.match_keyword(Keyword::Mut);
                let expr = self.parse_expression_with(15, allow_struct_lit)?;
                Ok(Expr::Ref { expr: Box::new(expr), is_mut, span: token.span })
            }
            TokenKind::LeftParen => {
                if self.match_kind(&TokenKind::RightParen) {
                    Ok(Expr::Unit(token.span))
                } else {
                    let expr = self.parse_expression(0)?;
                    self.expect_kind(&TokenKind::RightParen, "expected ')' after expression")?;
                    Ok(expr)
                }
            }
            TokenKind::LeftBrace => {
                // Dict literal: { key: value, ... } or old block expression
                self.parse_dict_literal(token.span)
            }
            TokenKind::LeftBracket => self.parse_list_literal(token.span),
            TokenKind::Keyword(Keyword::If) => self.parse_if_expression(token.span),
            TokenKind::Keyword(Keyword::Match) => self.parse_match_expression(token.span),
            TokenKind::Keyword(Keyword::Fn) => self.parse_lambda(token.span),
            TokenKind::Keyword(Keyword::Spawn) => {
                let expr = self.parse_expression(15)?;
                Ok(Expr::Spawn { expr: Box::new(expr), span: token.span })
            }
            TokenKind::Keyword(Keyword::Await) => {
                let expr = self.parse_expression(15)?;
                Ok(Expr::Await { expr: Box::new(expr), span: token.span })
            }
            TokenKind::Keyword(Keyword::Region) => {
                let name = self.expect_ident("expected a name after 'region'")?;
                let body = self.parse_block()?;
                Ok(Expr::Region { name, body, span: token.span })
            }
            _ => Err(Diagnostic::new("expected expression", token.span)),
        }
    }

    fn looks_like_struct_lit(&self) -> bool {
        // `Name { ident :` or `Name { }`
        match (
            self.peek_n(0).map(|t| &t.kind),
            self.peek_n(1).map(|t| &t.kind),
            self.peek_n(2).map(|t| &t.kind),
        ) {
            (Some(TokenKind::LeftBrace), Some(TokenKind::Ident(_)), Some(TokenKind::Colon)) => true,
            (Some(TokenKind::LeftBrace), Some(TokenKind::RightBrace), _) => true,
            _ => false,
        }
    }

    fn parse_struct_lit(&mut self, name: String, span: Span) -> LangResult<Expr> {
        self.expect_kind(&TokenKind::LeftBrace, "expected '{' in struct literal")?;
        let mut fields = Vec::new();
        while !self.check_kind(&TokenKind::RightBrace) && !self.is_at_end() {
            let field_name = self.expect_ident("expected field name in struct literal")?;
            self.expect_kind(&TokenKind::Colon, "expected ':' after field name")?;
            let value = self.parse_expression(0)?;
            fields.push((field_name, value));
            if !self.match_kind(&TokenKind::Comma) { break; }
        }
        self.expect_kind(&TokenKind::RightBrace, "expected '}' in struct literal")?;
        Ok(Expr::StructLit { name, fields, span })
    }

    fn parse_dict_literal(&mut self, span: Span) -> LangResult<Expr> {
        // `{` already consumed
        let mut pairs = Vec::new();
        while !self.check_kind(&TokenKind::RightBrace) && !self.is_at_end() {
            let key = self.parse_expression(0)?;
            self.expect_kind(&TokenKind::Colon, "expected ':' in dict literal")?;
            let value = self.parse_expression(0)?;
            pairs.push((key, value));
            if !self.match_kind(&TokenKind::Comma) { break; }
        }
        self.expect_kind(&TokenKind::RightBrace, "expected '}' in dict literal")?;
        Ok(Expr::Dict(pairs, span))
    }

    fn parse_list_literal(&mut self, span: Span) -> LangResult<Expr> {
        let mut items = Vec::new();
        if !self.check_kind(&TokenKind::RightBracket) {
            loop {
                items.push(self.parse_expression(0)?);
                if !self.match_kind(&TokenKind::Comma) { break; }
                if self.check_kind(&TokenKind::RightBracket) { break; }
            }
        }
        self.expect_kind(&TokenKind::RightBracket, "expected ']' in list literal")?;
        Ok(Expr::List(items, span))
    }

    fn parse_if_expression(&mut self, span: Span) -> LangResult<Expr> {
        let condition = self.parse_control_expression()?;
        let then_branch = self.parse_block()?;
        // Skip blank lines between if-body and else/elif
        self.skip_newlines();
        let else_branch = if self.match_keyword(Keyword::Else) {
            if self.match_keyword(Keyword::If) {
                let nested_span = self.previous().span.clone();
                let nested = self.parse_if_expression(nested_span)?;
                Some(Block {
                    statements: vec![Stmt::Expr {
                        expr: nested,
                        has_semicolon: false,
                        span: self.previous().span.clone(),
                    }],
                    span: self.previous().span.clone(),
                })
            } else {
                Some(self.parse_block()?)
            }
        } else if self.match_keyword(Keyword::Elif) {
            // elif = else if (Python sugar)
            let nested_span = self.previous().span.clone();
            let nested = self.parse_if_expression(nested_span)?;
            Some(Block {
                statements: vec![Stmt::Expr {
                    expr: nested,
                    has_semicolon: false,
                    span: self.previous().span.clone(),
                }],
                span: self.previous().span.clone(),
            })
        } else {
            None
        };
        Ok(Expr::If { condition: Box::new(condition), then_branch, else_branch, span })
    }

    fn parse_match_expression(&mut self, span: Span) -> LangResult<Expr> {
        let scrutinee = self.parse_control_expression()?;
        let arms = self.parse_match_arms()?;
        Ok(Expr::Match { scrutinee: Box::new(scrutinee), arms, span })
    }

    fn parse_match_arms(&mut self) -> LangResult<Vec<MatchArm>> {
        let use_brace = self.match_kind(&TokenKind::LeftBrace);
        if !use_brace {
            self.expect_kind(&TokenKind::Colon, "expected ':' or '{' before match arms")?;
            self.match_kind(&TokenKind::Newline);
            self.expect_kind(&TokenKind::Indent, "expected indented match arms")?;
        } else {
            self.skip_indent_tokens();
        }

        let done = if use_brace { TokenKind::RightBrace } else { TokenKind::Dedent };
        let mut arms = Vec::new();

        while !self.check_kind(&done) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_kind(&done) { break; }
            let arm_span = self.peek().span.clone();
            let pattern = self.parse_pattern()?;
            self.expect_kind(&TokenKind::FatArrow, "expected '=>' in match arm")?;
            let body = self.parse_expression(0)?;
            arms.push(MatchArm { pattern, body, span: arm_span });
            if use_brace {
                self.match_kind(&TokenKind::Comma);
                self.skip_indent_tokens();
            } else {
                self.match_kind(&TokenKind::Newline);
            }
        }

        if use_brace {
            self.expect_kind(&TokenKind::RightBrace, "expected '}' after match arms")?;
        } else {
            self.match_kind(&TokenKind::Dedent);
        }
        Ok(arms)
    }

    fn parse_pattern(&mut self) -> LangResult<Pattern> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Ident(name) if name == "_" => Ok(Pattern::Wildcard(token.span)),
            TokenKind::Ident(name) => {
                if self.match_kind(&TokenKind::LeftParen) {
                    let mut payload = Vec::new();
                    if !self.check_kind(&TokenKind::RightParen) {
                        loop {
                            payload.push(self.parse_pattern()?);
                            if !self.match_kind(&TokenKind::Comma) { break; }
                            if self.check_kind(&TokenKind::RightParen) { break; }
                        }
                    }
                    self.expect_kind(&TokenKind::RightParen, "expected ')' after variant payload")?;
                    return Ok(Pattern::Variant { name, payload, span: token.span });
                }
                Ok(Pattern::Ident(name, token.span))
            }
            TokenKind::Int(value) => Ok(Pattern::Int(value, token.span)),
            TokenKind::Float(value) => Ok(Pattern::Float(value, token.span)),
            TokenKind::Minus => {
                let next = self.advance().clone();
                match next.kind {
                    TokenKind::Int(value) => Ok(Pattern::Int(-value, token.span)),
                    TokenKind::Float(value) => Ok(Pattern::Float(-value, token.span)),
                    _ => Err(Diagnostic::new("expected numeric literal after '-'", next.span)),
                }
            }
            TokenKind::String(value) => Ok(Pattern::String(value, token.span)),
            TokenKind::Keyword(Keyword::True) => Ok(Pattern::Bool(true, token.span)),
            TokenKind::Keyword(Keyword::False) => Ok(Pattern::Bool(false, token.span)),
            TokenKind::LeftParen => {
                self.expect_kind(&TokenKind::RightParen, "expected ')' in unit pattern")?;
                Ok(Pattern::Unit(token.span))
            }
            _ => Err(Diagnostic::new("expected pattern", token.span)),
        }
    }

    fn parse_lambda(&mut self, span: Span) -> LangResult<Expr> {
        let params = self.parse_params()?;
        let return_type = if self.match_kind(&TokenKind::Arrow) {
            Some(self.parse_type_ref()?)
        } else {
            None
        };

        let body = if self.match_kind(&TokenKind::FatArrow) {
            LambdaBody::Expr(Box::new(self.parse_expression(0)?))
        } else {
            LambdaBody::Block(self.parse_block()?)
        };

        Ok(Expr::Lambda { params, return_type, body, span })
    }

    // ── Call / member / index ─────────────────────────────────────────────────

    fn finish_call(&mut self, callee: Expr) -> LangResult<Expr> {
        let span = self.expect_kind(&TokenKind::LeftParen, "expected '('")?;
        let mut args = Vec::new();
        if !self.check_kind(&TokenKind::RightParen) {
            loop {
                args.push(self.parse_expression(0)?);
                if !self.match_kind(&TokenKind::Comma) { break; }
                if self.check_kind(&TokenKind::RightParen) { break; }
            }
        }
        self.expect_kind(&TokenKind::RightParen, "expected ')' after call arguments")?;
        Ok(Expr::Call { callee: Box::new(callee), args, span })
    }

    fn finish_member(&mut self, target: Expr) -> LangResult<Expr> {
        let span = self.expect_kind(&TokenKind::Dot, "expected '.'")?;
        let name = self.expect_ident("expected field or method name after '.'")?;
        if self.check_kind(&TokenKind::LeftParen) {
            self.advance();
            let mut args = Vec::new();
            if !self.check_kind(&TokenKind::RightParen) {
                loop {
                    args.push(self.parse_expression(0)?);
                    if !self.match_kind(&TokenKind::Comma) { break; }
                    if self.check_kind(&TokenKind::RightParen) { break; }
                }
            }
            self.expect_kind(&TokenKind::RightParen, "expected ')' after method arguments")?;
            Ok(Expr::MethodCall { target: Box::new(target), name, args, span })
        } else {
            Ok(Expr::Field { target: Box::new(target), name, span })
        }
    }

    fn finish_index(&mut self, target: Expr) -> LangResult<Expr> {
        let span = self.expect_kind(&TokenKind::LeftBracket, "expected '['")?;
        let index = self.parse_expression(0)?;
        self.expect_kind(&TokenKind::RightBracket, "expected ']' after index expression")?;
        Ok(Expr::Index { target: Box::new(target), index: Box::new(index), span })
    }

    // ── Types ─────────────────────────────────────────────────────────────────

    fn parse_type_ref(&mut self) -> LangResult<TypeRef> {
        if self.match_kind(&TokenKind::Amp) {
            let is_mut = self.match_keyword(Keyword::Mut);
            let inner = self.parse_type_ref()?;
            return Ok(TypeRef::Ref(Box::new(inner), is_mut));
        }
        if self.match_kind(&TokenKind::LeftParen) {
            self.expect_kind(&TokenKind::RightParen, "expected ')' in unit type")?;
            return Ok(TypeRef::Unit);
        }
        if self.match_keyword(Keyword::Fn) {
            let params = self.parse_type_list(TokenKind::LeftParen, TokenKind::RightParen)?;
            self.expect_kind(&TokenKind::Arrow, "expected '->' in function type")?;
            let result = self.parse_type_ref()?;
            return Ok(TypeRef::Function(params, Box::new(result)));
        }
        if self.match_kind(&TokenKind::LeftBracket) {
            let elem = self.parse_type_ref()?;
            self.expect_kind(&TokenKind::RightBracket, "expected ']' in list type")?;
            return Ok(TypeRef::List(Box::new(elem)));
        }
        // `_` = infer
        if let TokenKind::Ident(name) = &self.peek().kind {
            if name == "_" {
                self.advance();
                return Ok(TypeRef::Infer);
            }
        }
        let name = self.expect_ident("expected type name")?;
        if self.match_kind(&TokenKind::LeftBracket) {
            let mut args = Vec::new();
            if !self.check_kind(&TokenKind::RightBracket) {
                loop {
                    args.push(self.parse_type_ref()?);
                    if !self.match_kind(&TokenKind::Comma) { break; }
                    if self.check_kind(&TokenKind::RightBracket) { break; }
                }
            }
            self.expect_kind(&TokenKind::RightBracket, "expected ']' after type arguments")?;
            return Ok(TypeRef::Generic(name, args));
        }
        Ok(TypeRef::Named(name))
    }

    fn parse_type_list(&mut self, open: TokenKind, close: TokenKind) -> LangResult<Vec<TypeRef>> {
        self.expect_kind(&open, "expected opening delimiter")?;
        let mut types = Vec::new();
        if !self.check_kind(&close) {
            loop {
                types.push(self.parse_type_ref()?);
                if !self.match_kind(&TokenKind::Comma) { break; }
                if self.check_kind(&close) { break; }
            }
        }
        self.expect_kind(&close, "expected closing delimiter")?;
        Ok(types)
    }

    // ── Operator precedence ───────────────────────────────────────────────────

    fn current_infix(&self) -> Option<(InfixOp, u8, u8)> {
        match &self.peek().kind {
            TokenKind::OrOr | TokenKind::Keyword(Keyword::Or) => Some((InfixOp::Or, 1, 2)),
            TokenKind::AndAnd | TokenKind::Keyword(Keyword::And) => Some((InfixOp::And, 3, 4)),
            TokenKind::PipeForward => Some((InfixOp::Pipeline, 5, 6)),
            TokenKind::DotDot     => Some((InfixOp::Range, 5, 6)),
            TokenKind::EqualEqual => Some((InfixOp::Equal, 7, 8)),
            TokenKind::BangEqual  => Some((InfixOp::NotEqual, 7, 8)),
            TokenKind::Less       => Some((InfixOp::Less, 9, 10)),
            TokenKind::LessEqual  => Some((InfixOp::LessEqual, 9, 10)),
            TokenKind::Greater    => Some((InfixOp::Greater, 9, 10)),
            TokenKind::GreaterEqual => Some((InfixOp::GreaterEqual, 9, 10)),
            TokenKind::Plus       => Some((InfixOp::Add, 11, 12)),
            TokenKind::Minus      => Some((InfixOp::Subtract, 11, 12)),
            TokenKind::Star       => Some((InfixOp::Multiply, 13, 14)),
            TokenKind::Slash      => Some((InfixOp::Divide, 13, 14)),
            TokenKind::Percent    => Some((InfixOp::Remainder, 13, 14)),
            _ => None,
        }
    }

    // ── Low-level helpers ─────────────────────────────────────────────────────

    fn expect_ident(&mut self, message: &str) -> LangResult<String> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Ident(name) => Ok(name),
            _ => Err(Diagnostic::new(message, token.span)),
        }
    }

    fn expect_kind(&mut self, expected: &TokenKind, message: &str) -> LangResult<Span> {
        if self.check_kind(expected) {
            Ok(self.advance().span.clone())
        } else {
            Err(Diagnostic::new(message, self.peek().span.clone()))
        }
    }

    fn expect_keyword(&mut self, expected: Keyword, message: &str) -> LangResult<Span> {
        if self.check_keyword(expected) {
            Ok(self.advance().span.clone())
        } else {
            Err(Diagnostic::new(message, self.peek().span.clone()))
        }
    }

    fn match_kind(&mut self, expected: &TokenKind) -> bool {
        if self.check_kind(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_keyword(&mut self, expected: Keyword) -> bool {
        if self.check_keyword(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check_kind(&self, expected: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(expected)
    }

    fn check_keyword(&self, expected: Keyword) -> bool {
        matches!(self.peek().kind, TokenKind::Keyword(kw) if kw == expected)
    }

    /// Skip NEWLINE tokens (but NOT INDENT/DEDENT).
    fn skip_newlines(&mut self) {
        while matches!(self.peek().kind, TokenKind::Newline | TokenKind::Semicolon) {
            self.advance();
        }
    }

    /// Skip INDENT, DEDENT, NEWLINE, and Semicolon tokens.
    /// Used inside old-style `{ }` blocks where these are spurious.
    fn skip_indent_tokens(&mut self) {
        while matches!(
            self.peek().kind,
            TokenKind::Indent | TokenKind::Dedent | TokenKind::Newline | TokenKind::Semicolon
        ) {
            self.advance();
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn peek_n(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.current + offset)
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
}

// ── InfixOp helper ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InfixOp {
    Add, Subtract, Multiply, Divide, Remainder,
    Equal, NotEqual, Less, LessEqual, Greater, GreaterEqual,
    And, Or, Pipeline, Range,
}

impl InfixOp {
    fn into_binary(self) -> Option<BinaryOp> {
        Some(match self {
            InfixOp::Add       => BinaryOp::Add,
            InfixOp::Subtract  => BinaryOp::Subtract,
            InfixOp::Multiply  => BinaryOp::Multiply,
            InfixOp::Divide    => BinaryOp::Divide,
            InfixOp::Remainder => BinaryOp::Remainder,
            InfixOp::Equal     => BinaryOp::Equal,
            InfixOp::NotEqual  => BinaryOp::NotEqual,
            InfixOp::Less      => BinaryOp::Less,
            InfixOp::LessEqual => BinaryOp::LessEqual,
            InfixOp::Greater   => BinaryOp::Greater,
            InfixOp::GreaterEqual => BinaryOp::GreaterEqual,
            InfixOp::And       => BinaryOp::And,
            InfixOp::Or        => BinaryOp::Or,
            InfixOp::Pipeline | InfixOp::Range => return None,
        })
    }
}
