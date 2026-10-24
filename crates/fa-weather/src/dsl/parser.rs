use super::ParseError;
use super::ast::*;
use super::lexer::{Tok, Token, lex};

pub fn parse(src: &str) -> Result<Rule, ParseError> {
    let toks = lex(src)?;
    let mut p = Parser { toks, i: 0 };
    let mut groups = vec![vec![p.cond()?]];
    loop {
        let t = p.peek().clone();
        match &t.tok {
            Tok::End => break,
            Tok::Ident(s) if s == "and" => {
                p.i += 1;
                let c = p.cond()?;
                if let Some(g) = groups.last_mut() {
                    g.push(c);
                }
            }
            Tok::Ident(s) if s == "or" => {
                p.i += 1;
                groups.push(vec![p.cond()?]);
            }
            _ => {
                return Err(err(
                    t.pos,
                    "expected `and`, `or` or the end of the rule".into(),
                ));
            }
        }
    }
    Ok(Rule { groups })
}

fn err(pos: usize, what: String) -> ParseError {
    ParseError {
        position: pos,
        message: format!("{what} at {pos}"),
    }
}

struct Parser {
    toks: Vec<Token>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.toks[self.i.min(self.toks.len() - 1)]
    }

    fn next(&mut self) -> Token {
        let t = self.peek().clone();
        if self.i < self.toks.len() - 1 {
            self.i += 1;
        }
        t
    }

    fn expect(&mut self, want: Tok, what: &str) -> Result<(), ParseError> {
        let t = self.next();
        if t.tok == want {
            Ok(())
        } else {
            Err(err(t.pos, format!("expected {what}")))
        }
    }

    fn cond(&mut self) -> Result<Cond, ParseError> {
        let t = self.next();
        let agg = match &t.tok {
            Tok::Ident(s) => match s.as_str() {
                "min" => Agg::Min,
                "max" => Agg::Max,
                "sum" => Agg::Sum,
                "avg" => Agg::Avg,
                "any" => Agg::Any,
                _ => return Err(err(t.pos, agg_msg())),
            },
            _ => return Err(err(t.pos, agg_msg())),
        };
        self.expect(Tok::LParen, "`(`")?;
        let t = self.next();
        let field = match &t.tok {
            Tok::Ident(s) => Field::ALL.into_iter().find(|f| f.name() == s),
            _ => None,
        }
        .ok_or_else(|| err(t.pos, "expected a field such as temp_c".into()))?;
        self.expect(Tok::Comma, "`,`")?;
        let window = self.window()?;
        self.expect(Tok::RParen, "`)`")?;
        let t = self.next();
        let op = match t.tok {
            Tok::Op(op) => op,
            _ => {
                return Err(err(
                    t.pos,
                    "expected a comparison operator (<=, <, >=, >, == or !=)".into(),
                ));
            }
        };
        let t = self.next();
        let Tok::Num { value, .. } = t.tok else {
            return Err(err(t.pos, "expected a number".into()));
        };
        Ok(Cond {
            agg,
            field,
            window,
            op,
            value,
        })
    }

    fn window(&mut self) -> Result<Window, ParseError> {
        let t = self.next();
        let Tok::Ident(s) = &t.tok else {
            return Err(err(t.pos, window_msg()));
        };
        match s.as_str() {
            "today" => Ok(Window::Today),
            "tonight" => Ok(Window::Tonight),
            "tomorrow" => Ok(Window::Tomorrow),
            "next" => {
                let n = self.next();
                let hours = match n.tok {
                    Tok::Num { value, int: true } if (1.0..=48.0).contains(&value) => value as u32,
                    _ => {
                        return Err(err(
                            n.pos,
                            "expected a whole number of hours from 1 to 48".into(),
                        ));
                    }
                };
                let h = self.next();
                match &h.tok {
                    Tok::Ident(s) if s == "h" => Ok(Window::Next(hours)),
                    _ => Err(err(h.pos, "expected `h` after the number of hours".into())),
                }
            }
            _ => Err(err(t.pos, window_msg())),
        }
    }
}

fn agg_msg() -> String {
    "expected an aggregate (min, max, sum, avg or any)".into()
}

fn window_msg() -> String {
    "expected a window (next Nh, today, tonight or tomorrow)".into()
}
