use crate::diagnostic::{Diagnostic, LangResult, Span};
use crate::token::{Keyword, Token, TokenKind};

pub struct Lexer {
    chars: Vec<char>,
    current: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            current: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn lex(mut self) -> LangResult<Vec<Token>> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            let span = self.span();
            let ch = self.advance();

            match ch {
                '(' => tokens.push(Token::new(TokenKind::LeftParen, span)),
                ')' => tokens.push(Token::new(TokenKind::RightParen, span)),
                '{' => tokens.push(Token::new(TokenKind::LeftBrace, span)),
                '}' => tokens.push(Token::new(TokenKind::RightBrace, span)),
                '[' => tokens.push(Token::new(TokenKind::LeftBracket, span)),
                ']' => tokens.push(Token::new(TokenKind::RightBracket, span)),
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
                '#' => tokens.push(Token::new(TokenKind::Hash, span)),
                ' ' | '\r' | '\t' | '\n' => {}
                ch if ch.is_ascii_digit() => {
                    tokens.push(Token::new(self.number(ch, span.clone())?, span));
                }
                ch if is_ident_start(ch) => tokens.push(Token::new(self.identifier(ch), span)),
                _ => {
                    return Err(Diagnostic::new(
                        format!("unexpected character '{ch}'"),
                        span,
                    ))
                }
            }
        }

        tokens.push(Token::new(TokenKind::Eof, self.span()));
        Ok(tokens)
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
            text.push(self.advance());
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

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
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
