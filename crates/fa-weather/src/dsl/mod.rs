//! The weather-rule DSL (arch/Feeds.md section 4.1): hand-written lexer and parser.

mod ast;
mod lexer;
mod parser;

pub use ast::{Agg, Cond, Field, Op, Rule, Window};
pub use parser::parse;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize)]
#[error("{message}")]
pub struct ParseError {
    /// Char offset into the condition text.
    pub position: usize,
    pub message: String,
}
