//! Hand-rolled recursive-descent parser: LaTeX math source -> [`MathNode`].
//! No new dependency -- the vocabulary this covers (fractions, scripts,
//! radicals, `\left`/`\right`, `\sum`/`\int`/etc. with limits, `\text{}`,
//! Greek/operator/relation symbols via [`super::glyphs`]) is small enough
//! that a tokenizer + a few mutually-recursive functions is simpler and
//! more auditable than pulling in a general parser-combinator crate.
//!
//! **Must never panic on malformed input** -- this parses text an LLM
//! produced, not a trusted build artifact. Every failure mode returns
//! [`MathParseError`]; callers (`markdown_to_runs`) fall back to
//! rendering the raw `$...$`/`$$...$$` source literally rather than
//! propagating a panic into a chat bubble.

use super::ast::{AtomClass, BigOpKind, DelimiterKind, MathNode};
use super::glyphs::lookup_command;

#[derive(Debug, Clone, PartialEq)]
pub enum MathParseError {
    /// Ran out of tokens where at least one more was required (e.g. a
    /// dangling `^`/`_` with no following primary, or `\frac` missing an
    /// argument group).
    UnexpectedEnd,
    /// A `\command` this table (see [`super::glyphs`] and this module's
    /// own structural commands) doesn't recognize.
    UnknownCommand(String),
    /// A `{` with no matching `}`, or vice versa.
    UnmatchedBrace,
    /// A `\left`/`\right` pair that doesn't resolve (missing `\right`,
    /// stray `\right` with no `\left`, or an unrecognized delimiter
    /// token).
    UnmatchedDelimiter(String),
}

/// Parses `source` (the raw TeX string from a `$...$`/`$$...$$` span,
/// *without* the surrounding `$` delimiters -- those are already stripped
/// by `pulldown_cmark`'s `Event::InlineMath`/`Event::DisplayMath` before
/// this is called) into a [`MathNode`] tree ready for layout.
pub fn parse_math(source: &str) -> Result<MathNode, MathParseError> {
    let tokens = tokenize(source);
    let mut parser = Parser {
        tokens: &tokens,
        pos: 0,
    };
    let node = parser.parse_row(|t| matches!(t, Token::CloseBrace))?;
    // A top-level stray `}` (no matching `{`) is the one case
    // `parse_row`'s stop-on-`CloseBrace` condition can exit early without
    // actually consuming everything -- check we reached the end.
    if parser.pos != tokens.len() {
        return Err(MathParseError::UnmatchedBrace);
    }
    Ok(node)
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// A command word after `\`, letters only (e.g. `frac`, `sqrt`,
    /// `left`, `sum`, `alpha`).
    Command(String),
    /// `\` followed by a single non-letter character -- LaTeX's escaping
    /// convention for characters that are otherwise structural, chiefly
    /// `\{`, `\}`, and `\|` (a double-bar delimiter).
    Escaped(char),
    /// Any other single character, including whitespace (kept, not
    /// dropped, so `\text{...}` can reconstruct literal spacing -- see
    /// `Parser::parse_text_literal`; every other call site skips
    /// whitespace `Char`s itself via `Parser::skip_space`).
    Char(char),
    Caret,
    Underscore,
    OpenBrace,
    CloseBrace,
}

fn tokenize(source: &str) -> Vec<Token> {
    let mut chars = source.chars().peekable();
    let mut tokens = Vec::new();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.peek().copied() {
                Some(next) if next.is_ascii_alphabetic() => {
                    let mut name = String::new();
                    while let Some(&c2) = chars.peek() {
                        if c2.is_ascii_alphabetic() {
                            name.push(c2);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    tokens.push(Token::Command(name));
                }
                Some(next) => {
                    chars.next();
                    tokens.push(Token::Escaped(next));
                }
                None => {}
            },
            '^' => tokens.push(Token::Caret),
            '_' => tokens.push(Token::Underscore),
            '{' => tokens.push(Token::OpenBrace),
            '}' => tokens.push(Token::CloseBrace),
            other => tokens.push(Token::Char(other)),
        }
    }
    tokens
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(Token::Char(c)) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    /// Parses atoms until `stop` matches the next token or input is
    /// exhausted. Whitespace between atoms is insignificant in math mode
    /// (standard LaTeX behavior) and is skipped here, not treated as its
    /// own atom.
    fn parse_row(&mut self, stop: impl Fn(&Token) -> bool) -> Result<MathNode, MathParseError> {
        let mut atoms = Vec::new();
        loop {
            self.skip_space();
            match self.peek() {
                None => break,
                Some(tok) if stop(tok) => break,
                _ => atoms.push(self.parse_atom()?),
            }
        }
        Ok(match atoms.len() {
            1 => atoms.into_iter().next().unwrap(),
            _ => MathNode::Row(atoms),
        })
    }

    /// One atom: a primary, optionally followed by `^`/`_` (in either
    /// order, at most one of each -- `x_i^2` and `x^2_i` are both valid
    /// and equivalent). `\sum`-style big operators consume their own
    /// `_`/`^` as limits inside `parse_primary`/`parse_bigop`, so this
    /// loop never double-wraps one in an extra `Script`.
    fn parse_atom(&mut self) -> Result<MathNode, MathParseError> {
        let base = self.parse_primary()?;
        let mut sup = None;
        let mut sub = None;
        loop {
            self.skip_space();
            match self.peek() {
                Some(Token::Caret) if sup.is_none() => {
                    self.advance();
                    self.skip_space();
                    sup = Some(Box::new(self.parse_primary()?));
                }
                Some(Token::Underscore) if sub.is_none() => {
                    self.advance();
                    self.skip_space();
                    sub = Some(Box::new(self.parse_primary()?));
                }
                _ => break,
            }
        }
        Ok(if sup.is_some() || sub.is_some() {
            MathNode::Script {
                base: Box::new(base),
                sup,
                sub,
            }
        } else {
            base
        })
    }

    fn parse_primary(&mut self) -> Result<MathNode, MathParseError> {
        self.skip_space();
        match self.advance() {
            None => Err(MathParseError::UnexpectedEnd),
            Some(Token::OpenBrace) => {
                let node = self.parse_row(|t| matches!(t, Token::CloseBrace))?;
                self.expect_close_brace()?;
                Ok(node)
            }
            Some(Token::CloseBrace) => Err(MathParseError::UnmatchedBrace),
            Some(Token::Caret) | Some(Token::Underscore) => Err(MathParseError::UnexpectedEnd),
            Some(Token::Escaped(c)) => Ok(MathNode::Symbol {
                glyph: *c,
                class: AtomClass::Ord,
            }),
            Some(Token::Char(c)) => Ok(symbol_for_char(*c)),
            Some(Token::Command(name)) => {
                let name = name.clone();
                self.parse_command(&name)
            }
        }
    }

    fn expect_close_brace(&mut self) -> Result<(), MathParseError> {
        match self.advance() {
            Some(Token::CloseBrace) => Ok(()),
            _ => Err(MathParseError::UnmatchedBrace),
        }
    }

    /// Parses one `{...}` group as a required argument (used by
    /// `\frac`/`\sqrt`) -- a bare single token (`\frac12`, valid LaTeX)
    /// falls back to `parse_primary` so `\frac12` and `\frac{1}{2}`
    /// behave the same, matching real LaTeX's "an argument is either a
    /// group or a single token" rule.
    fn parse_argument(&mut self) -> Result<MathNode, MathParseError> {
        self.parse_primary()
    }

    fn parse_command(&mut self, name: &str) -> Result<MathNode, MathParseError> {
        match name {
            "frac" => {
                let num = self.parse_argument()?;
                let den = self.parse_argument()?;
                Ok(MathNode::Frac {
                    num: Box::new(num),
                    den: Box::new(den),
                })
            }
            "sqrt" => {
                let radicand = self.parse_argument()?;
                Ok(MathNode::Sqrt(Box::new(radicand)))
            }
            "text" => {
                self.skip_space();
                match self.advance() {
                    Some(Token::OpenBrace) => Ok(MathNode::Text(self.parse_text_literal()?)),
                    _ => Err(MathParseError::UnexpectedEnd),
                }
            }
            "left" => self.parse_delimited(),
            "right" => Err(MathParseError::UnmatchedDelimiter(
                "\\right with no matching \\left".to_string(),
            )),
            "sum" => self.parse_bigop(BigOpKind::Sum),
            "prod" => self.parse_bigop(BigOpKind::Prod),
            "int" => self.parse_bigop(BigOpKind::Int),
            "oint" => self.parse_bigop(BigOpKind::Oint),
            "bigcup" => self.parse_bigop(BigOpKind::BigCup),
            "bigcap" => self.parse_bigop(BigOpKind::BigCap),
            "lim" => self.parse_bigop(BigOpKind::Lim),
            "max" => self.parse_bigop(BigOpKind::Max),
            "min" => self.parse_bigop(BigOpKind::Min),
            _ => match lookup_command(name) {
                Some((glyph, class)) => Ok(MathNode::Symbol { glyph, class }),
                None => Err(MathParseError::UnknownCommand(name.to_string())),
            },
        }
    }

    /// Accumulates literal characters (including whitespace, unlike every
    /// other parsing path here) until the matching `}` -- `\text{...}`'s
    /// content is upright prose, not math to recurse into. Does not
    /// support nested braces inside `\text{}` (not a realistic need for
    /// short chat-reply labels like `\text{Hz}`).
    fn parse_text_literal(&mut self) -> Result<String, MathParseError> {
        let mut text = String::new();
        loop {
            match self.advance() {
                Some(Token::CloseBrace) => return Ok(text),
                Some(Token::Char(c)) => text.push(*c),
                Some(Token::Escaped(c)) => text.push(*c),
                Some(Token::Caret) => text.push('^'),
                Some(Token::Underscore) => text.push('_'),
                Some(Token::OpenBrace) => return Err(MathParseError::UnmatchedBrace),
                Some(Token::Command(name)) => {
                    let name = name.clone();
                    match lookup_command(&name) {
                        Some((glyph, _)) => text.push(glyph),
                        None => return Err(MathParseError::UnknownCommand(name)),
                    }
                }
                None => return Err(MathParseError::UnmatchedBrace),
            }
        }
    }

    fn parse_bigop(&mut self, kind: BigOpKind) -> Result<MathNode, MathParseError> {
        let mut lower = None;
        let mut upper = None;
        loop {
            self.skip_space();
            match self.peek() {
                Some(Token::Underscore) if lower.is_none() => {
                    self.advance();
                    self.skip_space();
                    lower = Some(Box::new(self.parse_primary()?));
                }
                Some(Token::Caret) if upper.is_none() => {
                    self.advance();
                    self.skip_space();
                    upper = Some(Box::new(self.parse_primary()?));
                }
                _ => break,
            }
        }
        Ok(MathNode::BigOp { kind, lower, upper })
    }

    fn parse_delimited(&mut self) -> Result<MathNode, MathParseError> {
        let open = self.parse_delimiter_token()?;
        let body = self.parse_row(|t| matches!(t, Token::Command(name) if name == "right"))?;
        self.skip_space();
        match self.advance() {
            Some(Token::Command(name)) if name == "right" => {}
            _ => {
                return Err(MathParseError::UnmatchedDelimiter(
                    "\\left with no matching \\right".to_string(),
                ))
            }
        }
        let close = self.parse_delimiter_token()?;
        Ok(MathNode::Delimited {
            open,
            body: Box::new(body),
            close,
        })
    }

    /// Consumes and classifies the one token that must follow `\left`/
    /// `\right`: a bracket/paren/brace/bar character, `\lfloor`/`\rfloor`/
    /// `\lceil`/`\rceil`, `\langle`/`\rangle`, or `.` (LaTeX's "no visible
    /// delimiter on this side").
    fn parse_delimiter_token(&mut self) -> Result<Option<DelimiterKind>, MathParseError> {
        self.skip_space();
        match self.advance() {
            Some(Token::Char('(')) | Some(Token::Char(')')) => Ok(Some(DelimiterKind::Paren)),
            Some(Token::Char('[')) | Some(Token::Char(']')) => Ok(Some(DelimiterKind::Bracket)),
            Some(Token::Escaped('{')) | Some(Token::Escaped('}')) => Ok(Some(DelimiterKind::Brace)),
            Some(Token::Char('|')) => Ok(Some(DelimiterKind::Bar)),
            Some(Token::Escaped('|')) => Ok(Some(DelimiterKind::DoubleBar)),
            Some(Token::Char('.')) => Ok(None),
            Some(Token::Command(name)) if name == "lfloor" || name == "rfloor" => {
                Ok(Some(DelimiterKind::Floor))
            }
            Some(Token::Command(name)) if name == "lceil" || name == "rceil" => {
                Ok(Some(DelimiterKind::Ceil))
            }
            Some(Token::Command(name)) if name == "langle" => Ok(Some(DelimiterKind::AngleLeft)),
            Some(Token::Command(name)) if name == "rangle" => Ok(Some(DelimiterKind::AngleRight)),
            Some(other) => Err(MathParseError::UnmatchedDelimiter(format!(
                "{other:?} is not a valid \\left/\\right delimiter"
            ))),
            None => Err(MathParseError::UnexpectedEnd),
        }
    }
}

fn symbol_for_char(c: char) -> MathNode {
    let class = match c {
        '+' | '-' | '*' | '/' => AtomClass::Bin,
        '=' | '<' | '>' => AtomClass::Rel,
        '(' | '[' => AtomClass::Open,
        ')' | ']' => AtomClass::Close,
        ',' | ';' => AtomClass::Punct,
        _ => AtomClass::Ord,
    };
    MathNode::Symbol { glyph: c, class }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(glyph: char, class: AtomClass) -> MathNode {
        MathNode::Symbol { glyph, class }
    }

    #[test]
    fn a_single_variable_parses_as_one_symbol() {
        assert_eq!(parse_math("x"), Ok(sym('x', AtomClass::Ord)));
    }

    #[test]
    fn frac_one_half_parses_into_a_frac_node() {
        let expected = MathNode::Frac {
            num: Box::new(sym('1', AtomClass::Ord)),
            den: Box::new(sym('2', AtomClass::Ord)),
        };
        assert_eq!(parse_math("\\frac{1}{2}"), Ok(expected));
    }

    #[test]
    fn frac_without_braces_takes_single_tokens() {
        let expected = MathNode::Frac {
            num: Box::new(sym('1', AtomClass::Ord)),
            den: Box::new(sym('2', AtomClass::Ord)),
        };
        assert_eq!(parse_math("\\frac12"), Ok(expected));
    }

    #[test]
    fn x_squared_plus_y_sub_i_parses_both_scripts() {
        let node = parse_math("x^2 + y_i").expect("should parse");
        let MathNode::Row(atoms) = node else {
            panic!("expected a Row, got {node:?}")
        };
        assert_eq!(atoms.len(), 3, "expected [x^2, +, y_i], got {atoms:?}");
        assert_eq!(
            atoms[0],
            MathNode::Script {
                base: Box::new(sym('x', AtomClass::Ord)),
                sup: Some(Box::new(sym('2', AtomClass::Ord))),
                sub: None
            }
        );
        assert_eq!(
            atoms[2],
            MathNode::Script {
                base: Box::new(sym('y', AtomClass::Ord)),
                sup: None,
                sub: Some(Box::new(sym('i', AtomClass::Ord)))
            }
        );
    }

    #[test]
    fn both_scripts_at_once_are_a_single_script_node() {
        let node = parse_math("x_i^2").expect("should parse");
        assert_eq!(
            node,
            MathNode::Script {
                base: Box::new(sym('x', AtomClass::Ord)),
                sup: Some(Box::new(sym('2', AtomClass::Ord))),
                sub: Some(Box::new(sym('i', AtomClass::Ord))),
            }
        );
    }

    #[test]
    fn sqrt_x_plus_1_parses_into_a_sqrt_node() {
        let node = parse_math("\\sqrt{x+1}").expect("should parse");
        let MathNode::Sqrt(inner) = node else {
            panic!("expected Sqrt, got {node:?}")
        };
        assert_eq!(
            *inner,
            MathNode::Row(vec![
                sym('x', AtomClass::Ord),
                sym('+', AtomClass::Bin),
                sym('1', AtomClass::Ord)
            ])
        );
    }

    #[test]
    fn sum_with_limits_parses_lower_and_upper() {
        let node = parse_math("\\sum_{i=1}^{n} i").expect("should parse");
        let MathNode::Row(atoms) = node else {
            panic!("expected Row, got {node:?}")
        };
        let MathNode::BigOp { kind, lower, upper } = &atoms[0] else {
            panic!("expected BigOp first, got {:?}", atoms[0])
        };
        assert_eq!(*kind, BigOpKind::Sum);
        assert!(lower.is_some(), "expected a lower limit");
        assert!(upper.is_some(), "expected an upper limit");
    }

    #[test]
    fn left_paren_right_paren_parses_into_delimited() {
        let node = parse_math("\\left(x\\right)").expect("should parse");
        assert_eq!(
            node,
            MathNode::Delimited {
                open: Some(DelimiterKind::Paren),
                body: Box::new(sym('x', AtomClass::Ord)),
                close: Some(DelimiterKind::Paren),
            }
        );
    }

    #[test]
    fn greek_letters_resolve_via_the_glyph_table() {
        let node = parse_math("\\alpha + \\beta").expect("should parse");
        let MathNode::Row(atoms) = node else {
            panic!("expected Row, got {node:?}")
        };
        assert_eq!(atoms[0], sym('\u{03B1}', AtomClass::Ord));
        assert_eq!(atoms[2], sym('\u{03B2}', AtomClass::Ord));
    }

    #[test]
    fn text_command_preserves_literal_spacing() {
        let node = parse_math("\\text{Hello World}").expect("should parse");
        assert_eq!(node, MathNode::Text("Hello World".to_string()));
    }

    #[test]
    fn an_unknown_command_is_a_clean_error_not_a_panic() {
        assert_eq!(
            parse_math("\\notarealcommand"),
            Err(MathParseError::UnknownCommand(
                "notarealcommand".to_string()
            ))
        );
    }

    #[test]
    fn an_unmatched_left_with_no_right_is_a_clean_error() {
        assert!(matches!(
            parse_math("\\left(x"),
            Err(MathParseError::UnmatchedDelimiter(_))
        ));
    }

    #[test]
    fn a_stray_right_with_no_left_is_a_clean_error() {
        assert!(matches!(
            parse_math("x\\right)"),
            Err(MathParseError::UnmatchedDelimiter(_))
        ));
    }

    #[test]
    fn an_unmatched_brace_is_a_clean_error() {
        assert_eq!(
            parse_math("\\frac{1}{2"),
            Err(MathParseError::UnmatchedBrace)
        );
    }
}
