use crate::diagnostic::{Diagnostic, LangResult, Span};
use crate::token::{Keyword, Token, TokenKind};

pub struct Lexer {
    chars: Vec<char>,
    current: usize,
    line: usize,
    column: usize,
    // Python-style indentation state
    indent_stack: Vec<usize>,
    bracket_depth: usize,
    at_line_start: bool,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            current: 0,
            line: 1,
            column: 1,
            indent_stack: vec![0],
            bracket_depth: 0,
            at_line_start: true,
        }
    }

    pub fn lex(mut self) -> LangResult<Vec<Token>> {
        let mut tokens: Vec<Token> = Vec::new();

        'main: loop {
            // ── Indentation handling (Python-style) ──────────────────────────
            // Only process indentation when:
            //   1. we are at the start of a logical line
            //   2. we are not inside any bracket pair  (implicit line joining)
            if self.at_line_start && self.bracket_depth == 0 {
                let indent_span = self.span();
                let indent = self.count_indent();

                // Skip blank lines and comment-only lines.
                // After consuming the indent whitespace, peek at the first
                // real character on this line.
                let first = self.peek();
                if self.is_at_end() {
                    // EOF reached while at line start — exit the loop so the
                    // EOF-cleanup code below can emit pending DEDENTs + Eof.
                    break;
                }
                if matches!(first, Some('\n') | Some('\r')) {
                    // Blank line: consume the newline and stay at line-start.
                    self.consume_eol();
                    // at_line_start stays true
                    continue 'main;
                }
                if first == Some('/') && self.peek_next() == Some('/') {
                    // Comment-only line
                    self.skip_line_comment();
                    self.consume_eol();
                    continue 'main;
                }
                if first == Some('#') && self.peek_next() != Some('[') {
                    // Python-style # comment (not an attribute)
                    self.skip_line_comment_to_end();
                    self.consume_eol();
                    continue 'main;
                }

                // Non-blank, non-comment line: process indentation level.
                self.at_line_start = false;
                let current_level = *self.indent_stack.last().unwrap();

                if indent > current_level {
                    self.indent_stack.push(indent);
                    tokens.push(Token::new(TokenKind::Indent, indent_span.clone()));
                } else if indent < current_level {
                    while self.indent_stack.last().copied().unwrap_or(0) > indent {
                        self.indent_stack.pop();
                        tokens.push(Token::new(TokenKind::Dedent, indent_span.clone()));
                    }
                    if self.indent_stack.last().copied().unwrap_or(0) != indent {
                        return Err(Diagnostic::new("inconsistent indentation", indent_span));
                    }
                }
                // Same level → no INDENT/DEDENT needed.
                continue 'main; // re-enter to lex the first token of this line
            }

            if self.is_at_end() {
                break;
            }

            // ── Newline handling ─────────────────────────────────────────────
            let ch = self.peek().unwrap();

            if ch == '\r' || ch == '\n' {
                let nl_span = self.span();
                self.consume_eol();

                // Emit NEWLINE only outside brackets and only if the last
                // emitted token was not already a NEWLINE / INDENT.
                if self.bracket_depth == 0 {
                    let last = tokens.last().map(|t| &t.kind);
                    if !matches!(
                        last,
                        Some(TokenKind::Newline) | Some(TokenKind::Indent) | None
                    ) {
                        tokens.push(Token::new(TokenKind::Newline, nl_span));
                    }
                    self.at_line_start = true;
                }
                continue 'main;
            }

            // ── Whitespace (non-leading) ──────────────────────────────────────
            if ch == ' ' || ch == '\t' {
                self.advance();
                continue 'main;
            }

            // ── Regular tokens ────────────────────────────────────────────────
            let span = self.span();
            let ch = self.advance();

            match ch {
                '(' => {
                    tokens.push(Token::new(TokenKind::LeftParen, span));
                    self.bracket_depth += 1;
                }
                ')' => {
                    tokens.push(Token::new(TokenKind::RightParen, span));
                    self.bracket_depth = self.bracket_depth.saturating_sub(1);
                }
                '{' => {
                    tokens.push(Token::new(TokenKind::LeftBrace, span));
                    self.bracket_depth += 1;
                }
                '}' => {
                    tokens.push(Token::new(TokenKind::RightBrace, span));
                    self.bracket_depth = self.bracket_depth.saturating_sub(1);
                }
                '[' => {
                    tokens.push(Token::new(TokenKind::LeftBracket, span));
                    self.bracket_depth += 1;
                }
                ']' => {
                    tokens.push(Token::new(TokenKind::RightBracket, span));
                    self.bracket_depth = self.bracket_depth.saturating_sub(1);
                }
                ',' => tokens.push(Token::new(TokenKind::Comma, span)),
                '.' => {
                    let kind = if self.match_char('.') {
                        TokenKind::DotDot
                    } else {
                        TokenKind::Dot
                    };
                    tokens.push(Token::new(kind, span));
                }
                ':' => {
                    let kind = if self.match_char(':') {
                        TokenKind::ColonColon
                    } else {
                        TokenKind::Colon
                    };
                    tokens.push(Token::new(kind, span));
                }
                ';' => tokens.push(Token::new(TokenKind::Semicolon, span)),
                '+' => tokens.push(Token::new(TokenKind::Plus, span)),
                '*' => tokens.push(Token::new(TokenKind::Star, span)),
                '%' => tokens.push(Token::new(TokenKind::Percent, span)),
                '?' => tokens.push(Token::new(TokenKind::Question, span)),
                '!' => {
                    let kind = if self.match_char('=') {
                        TokenKind::BangEqual
                    } else {
                        TokenKind::Bang
                    };
                    tokens.push(Token::new(kind, span));
                }
                '=' => {
                    let kind = if self.match_char('=') {
                        TokenKind::EqualEqual
                    } else if self.match_char('>') {
                        TokenKind::FatArrow
                    } else {
                        TokenKind::Equal
                    };
                    tokens.push(Token::new(kind, span));
                }
                '<' => {
                    let kind = if self.match_char('=') {
                        TokenKind::LessEqual
                    } else {
                        TokenKind::Less
                    };
                    tokens.push(Token::new(kind, span));
                }
                '>' => {
                    let kind = if self.match_char('=') {
                        TokenKind::GreaterEqual
                    } else {
                        TokenKind::Greater
                    };
                    tokens.push(Token::new(kind, span));
                }
                '&' => {
                    let kind = if self.match_char('&') {
                        TokenKind::AndAnd
                    } else {
                        TokenKind::Amp
                    };
                    tokens.push(Token::new(kind, span));
                }
                '|' => {
                    let kind = if self.match_char('|') {
                        TokenKind::OrOr
                    } else if self.match_char('>') {
                        TokenKind::PipeForward
                    } else {
                        TokenKind::Pipe
                    };
                    tokens.push(Token::new(kind, span));
                }
                '-' => {
                    let kind = if self.match_char('>') {
                        TokenKind::Arrow
                    } else {
                        TokenKind::Minus
                    };
                    tokens.push(Token::new(kind, span));
                }
                '/' => {
                    if self.match_char('/') {
                        self.skip_line_comment();
                        // newline will be handled on next iteration
                    } else if self.match_char('*') {
                        self.skip_block_comment(span)?;
                    } else {
                        tokens.push(Token::new(TokenKind::Slash, span));
                    }
                }
                '"' => tokens.push(Token::new(
                    TokenKind::String(self.string(span.clone())?),
                    span,
                )),
                '#' => {
                    // `#[` → attribute hash; bare `#` → Python-style comment
                    if self.peek() == Some('[') {
                        tokens.push(Token::new(TokenKind::Hash, span));
                    } else {
                        self.skip_line_comment_to_end();
                        // newline handled on next iteration
                    }
                }
                ch if ch.is_ascii_digit() => {
                    tokens.push(Token::new(self.number(ch, span.clone())?, span));
                }
                ch if is_ident_start(ch) => {
                    tokens.push(Token::new(self.identifier(ch), span));
                }
                _ => {
                    return Err(Diagnostic::new(
                        format!("unexpected character '{ch}'"),
                        span,
                    ))
                }
            }
        }

        // ── EOF cleanup: emit final NEWLINE + pending DEDENTs ──────────────
        let final_span = self.span();
        let last = tokens.last().map(|t| &t.kind);
        if !matches!(
            last,
            Some(TokenKind::Newline) | Some(TokenKind::Indent) | Some(TokenKind::Dedent) | None
        ) {
            tokens.push(Token::new(TokenKind::Newline, final_span.clone()));
        }
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            tokens.push(Token::new(TokenKind::Dedent, final_span.clone()));
        }
        tokens.push(Token::new(TokenKind::Eof, self.span()));
        Ok(tokens)
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Count leading spaces/tabs and consume them.
    /// Tabs are treated as 4 spaces (like most modern editors).
    fn count_indent(&mut self) -> usize {
        let mut count = 0usize;
        loop {
            match self.peek() {
                Some(' ') => {
                    count += 1;
                    self.advance();
                }
                Some('\t') => {
                    count += 4;
                    self.advance();
                }
                _ => break,
            }
        }
        count
    }

    /// Consume `\r\n` or `\n` (end-of-line).
    fn consume_eol(&mut self) {
        if self.peek() == Some('\r') {
            self.advance();
        }
        if self.peek() == Some('\n') {
            self.advance();
        }
    }

    fn identifier(&mut self, first: char) -> TokenKind {
        let mut ident = String::from(first);
        while let Some(ch) = self.peek() {
            if is_ident_continue(ch) {
                ident.push(self.advance());
            } else {
                break;
            }
        }
        match Keyword::from_ident(&ident) {
            Some(keyword) => TokenKind::Keyword(keyword),
            None => TokenKind::Ident(ident),
        }
    }

    fn number(&mut self, first: char, span: Span) -> LangResult<TokenKind> {
        let mut text = String::from(first);
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '_' {
                text.push(self.advance());
            } else {
                break;
            }
        }

        let mut is_float = false;
        if self.peek() == Some('.')
            && self.peek_next().is_some_and(|ch| ch.is_ascii_digit())
            && self.peek_next() != Some('.')
        {
            is_float = true;
            text.push(self.advance()); // consume '.'
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() || ch == '_' {
                    text.push(self.advance());
                } else {
                    break;
                }
            }
        }

        let normalized = text.replace('_', "");
        if is_float {
            normalized
                .parse::<f64>()
                .map(TokenKind::Float)
                .map_err(|_| Diagnostic::new("invalid float literal", span))
        } else {
            normalized
                .parse::<i64>()
                .map(TokenKind::Int)
                .map_err(|_| Diagnostic::new("invalid integer literal", span))
        }
    }

    fn string(&mut self, span: Span) -> LangResult<String> {
        let mut value = String::new();
        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.advance();
                return Ok(value);
            }
            if ch == '\\' {
                self.advance();
                let escaped = match self.peek() {
                    Some('n') => '\n',
                    Some('r') => '\r',
                    Some('t') => '\t',
                    Some('0') => '\0',
                    Some('"') => '"',
                    Some('\\') => '\\',
                    Some(other) => {
                        return Err(Diagnostic::new(
                            format!("unsupported escape sequence '\\{other}'"),
                            self.span(),
                        ));
                    }
                    None => return Err(Diagnostic::new("unterminated string escape", span)),
                };
                self.advance();
                value.push(escaped);
                continue;
            }
            value.push(self.advance());
        }
        Err(Diagnostic::new("unterminated string literal", span))
    }

    /// Skip from current position to (but not including) the next `\n`.
    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    /// Skip from current position to (but not including) the next `\n`.
    /// Same as skip_line_comment but used for `#` comments.
    fn skip_line_comment_to_end(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            self.advance();
        }
    }

    fn skip_block_comment(&mut self, span: Span) -> LangResult<()> {
        while !self.is_at_end() {
            if self.peek() == Some('*') && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                return Ok(());
            }
            self.advance();
        }
        Err(Diagnostic::new("unterminated block comment", span))
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> char {
        let ch = self.chars[self.current];
        self.current += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        ch
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.current).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.current + 1).copied()
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.chars.len()
    }

    fn span(&self) -> Span {
        Span::new(self.line, self.column)
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}
