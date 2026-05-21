pub mod ast;
pub mod bench;
pub mod borrow;
pub mod build;
pub mod codegen;
pub mod dbg;
pub mod diagnostic;
pub mod doc;
pub mod fmt;
pub mod fmt_ast;
pub mod hir;
pub mod interpreter;
pub mod lexer;
pub mod lint;
pub mod lsp;
pub mod manifest;
pub mod mir;
pub mod parser;
pub mod pkg;
pub mod prof;
pub mod scaffold;
pub mod test_runner;
pub mod token;
pub mod typechecker;

use ast::Program;
use diagnostic::LangResult;
use token::Token;
use typechecker::TypeReport;

pub const LANGUAGE_NAME: &str = "mom";
pub const FILE_EXTENSION: &str = "mom";

pub fn lex_source(source: &str) -> LangResult<Vec<Token>> {
    lexer::Lexer::new(source).lex()
}

pub fn parse_source(source: &str) -> LangResult<Program> {
    let tokens = lex_source(source)?;
    parser::Parser::new(tokens).parse_program()
}

pub fn check_source(source: &str) -> LangResult<TypeReport> {
    let program = parse_source(source)?;
    let report = typechecker::TypeChecker::new().check_program(&program)?;
    borrow::BorrowChecker::new().check_program(&program)?;
    Ok(report)
}

pub fn run_source(source: &str) -> LangResult<String> {
    let program = parse_source(source)?;
    typechecker::TypeChecker::new().check_program(&program)?;
    borrow::BorrowChecker::new().check_program(&program)?;
    interpreter::Interpreter::new().run_program(&program)
}
