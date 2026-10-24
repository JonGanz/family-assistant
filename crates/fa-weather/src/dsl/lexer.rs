use super::ParseError;
use super::ast::Op;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Num { value: f64, int: bool },
    LParen,
    RParen,
    Comma,
    Op(Op),
    End,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    /// Char offset.
    pub pos: usize,
}

pub fn lex(src: &str) -> Result<Vec<Token>, ParseError> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let pos = i;
        if c.is_whitespace() {
            i += 1;
        } else if c == '(' || c == ')' || c == ',' {
            out.push(Token {
                tok: match c {
                    '(' => Tok::LParen,
                    ')' => Tok::RParen,
                    _ => Tok::Comma,
                },
                pos,
            });
            i += 1;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            out.push(Token {
                tok: Tok::Ident(s.to_ascii_lowercase()),
                pos,
            });
        } else if c.is_ascii_digit()
            || (c == '-' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit()))
            || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit()))
        {
            let start = i;
            if c == '-' {
                i += 1;
            }
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let mut int = true;
            if i < chars.len() && chars[i] == '.' {
                int = false;
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            let value = s.parse::<f64>().map_err(|_| ParseError {
                position: pos,
                message: format!("invalid number `{s}` at {pos}"),
            })?;
            out.push(Token {
                tok: Tok::Num { value, int },
                pos,
            });
        } else if matches!(c, '<' | '>' | '=' | '!') {
            let next = chars.get(i + 1).copied();
            let (op, len) = match (c, next) {
                ('<', Some('=')) => (Some(Op::Le), 2),
                ('<', _) => (Some(Op::Lt), 1),
                ('>', Some('=')) => (Some(Op::Ge), 2),
                ('>', _) => (Some(Op::Gt), 1),
                ('=', Some('=')) => (Some(Op::Eq), 2),
                ('!', Some('=')) => (Some(Op::Ne), 2),
                _ => (None, 1),
            };
            let Some(op) = op else {
                return Err(ParseError {
                    position: pos,
                    message: format!(
                        "expected a comparison operator (<=, <, >=, >, == or !=) at {pos}"
                    ),
                });
            };
            out.push(Token {
                tok: Tok::Op(op),
                pos,
            });
            i += len;
        } else {
            return Err(ParseError {
                position: pos,
                message: format!("unexpected character `{c}` at {pos}"),
            });
        }
    }
    out.push(Token {
        tok: Tok::End,
        pos: chars.len(),
    });
    Ok(out)
}
