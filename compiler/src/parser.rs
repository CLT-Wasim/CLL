//! Recursive-descent/Pratt parser for CLL surface syntax.
//!
//! The parser consumes the lexer token stream and preserves byte spans on every AST
//! node. It intentionally does not perform name resolution or type checking.

use crate::lexer::{self, FloatSuffix, IntSuffix, Span, Tok, Token};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub kind: FunctionKind,
    pub params: Vec<Parameter>,
    pub return_type: Option<String>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKind {
    Ordinary,
    Pure,
    Reversible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub ty: String,
    pub inout: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub tail: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Let {
        mutable: bool,
        name: String,
        ty: Option<String>,
        value: Expr,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Assign {
        place: Expr,
        op: AssignOp,
        value: Expr,
        span: Span,
    },
    Expr {
        value: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Set,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Xor,
    AddWrap,
    SubWrap,
    MulWrap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Integer {
        value: u128,
        suffix: Option<IntSuffix>,
    },
    Float {
        suffix: Option<FloatSuffix>,
    },
    String(String),
    Bool(bool),
    Name(String),
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Group(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
    BitNot,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    AddWrap,
    SubWrap,
    MulWrap,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub span: Span,
    pub message: String,
    pub expected: String,
}
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "parse error at bytes {}..{}: {}; expected {}",
            self.span.start, self.span.end, self.message, self.expected
        )
    }
}
impl std::error::Error for ParseError {}

/// Lex and parse one CLL module. Test directives are rejected outside test sources.
///
/// # Errors
/// Returns lexical diagnostics or the first syntax error with a source byte span.
pub fn parse(source: &str) -> Result<Module, String> {
    let tokens = lexer::lex(source, false).map_err(|errors| {
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    Parser { tokens, cursor: 0 }
        .module()
        .map_err(|error| error.to_string())
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}
impl Parser {
    fn module(mut self) -> Result<Module, ParseError> {
        let mut functions = Vec::new();
        while !self.at_end() {
            functions.push(self.function()?);
        }
        Ok(Module { functions })
    }

    fn function(&mut self) -> Result<Function, ParseError> {
        let start = self.current_span();
        let kind = if self.take(&Tok::Pure) {
            self.expect(Tok::Fn, "`fn` after `pure`")?;
            FunctionKind::Pure
        } else if self.take(&Tok::Rev) {
            self.expect(Tok::Fn, "`fn` after `rev`")?;
            FunctionKind::Reversible
        } else {
            self.expect(Tok::Fn, "function declaration")?;
            FunctionKind::Ordinary
        };
        let name = self.ident("function name")?;
        self.expect(Tok::LParen, "`(`")?;
        let mut params = Vec::new();
        if !self.check(&Tok::RParen) {
            loop {
                let param_start = self.current_span();
                let inout = self.take(&Tok::Inout);
                let param_name = self.ident("parameter name")?;
                self.expect(Tok::Colon, "`:` before parameter type")?;
                let ty = self.type_name()?;
                params.push(Parameter {
                    name: param_name,
                    ty,
                    inout,
                    span: param_start.to(self.previous_span()),
                });
                if !self.take(&Tok::Comma) {
                    break;
                }
                if self.check(&Tok::RParen) {
                    break;
                }
            }
        }
        self.expect(Tok::RParen, "`)`")?;
        let return_type = if self.take(&Tok::Arrow) {
            Some(self.type_name()?)
        } else {
            None
        };
        let body = self.block()?;
        let span = start.to(body.span);
        Ok(Function {
            name,
            kind,
            params,
            return_type,
            body,
            span,
        })
    }

    fn type_name(&mut self) -> Result<String, ParseError> {
        let (name, span) = match self.current().map(|t| (&t.kind, t.span)) {
            Some((Tok::Ident(name), span)) => (name.clone(), span),
            Some((Tok::LParen, _span)) => {
                self.bump();
                self.expect(Tok::RParen, "`)` for unit type")?;
                return Ok("()".into());
            }
            _ => return Err(self.error("expected a type name", "type")),
        };
        self.bump();
        let _ = span;
        Ok(name)
    }

    fn block(&mut self) -> Result<Block, ParseError> {
        let start = self.expect(Tok::LBrace, "`{` to start block")?;
        let mut statements = Vec::new();
        let mut tail = None;
        while !self.check(&Tok::RBrace) {
            if self.at_end() {
                return Err(self.error("unterminated block", "`}`"));
            }
            if self.take(&Tok::Semi) {
                continue;
            }
            if self.check(&Tok::Let) {
                statements.push(self.let_statement()?);
                continue;
            }
            if self.check(&Tok::Return) {
                statements.push(self.return_statement()?);
                continue;
            }
            let expr = self.expression(0)?;
            if let Some(op) = self.assignment_op() {
                let place = expr;
                self.bump();
                let value = self.expression(0)?;
                let end = self.expect(Tok::Semi, "`;` after assignment")?;
                let span = place.span.to(end);
                statements.push(Statement::Assign {
                    place,
                    op,
                    value,
                    span,
                });
            } else if self.take(&Tok::Semi) {
                let span = expr.span;
                statements.push(Statement::Expr { value: expr, span });
            } else if self.check(&Tok::RBrace) {
                tail = Some(expr);
                break;
            } else {
                return Err(self.error("expected `;` or end of block", "`;` or `}`"));
            }
        }
        let end = self.expect(Tok::RBrace, "`}` to close block")?;
        Ok(Block {
            statements,
            tail,
            span: start.to(end),
        })
    }

    fn let_statement(&mut self) -> Result<Statement, ParseError> {
        let start = self.expect(Tok::Let, "`let`")?;
        let mutable = self.take(&Tok::Mut);
        let name = self.ident("binding name")?;
        let ty = if self.take(&Tok::Colon) {
            Some(self.type_name()?)
        } else {
            None
        };
        self.expect(Tok::Eq, "`=` in binding")?;
        let value = self.expression(0)?;
        let end = self.expect(Tok::Semi, "`;` after binding")?;
        Ok(Statement::Let {
            mutable,
            name,
            ty,
            value,
            span: start.to(end),
        })
    }

    fn return_statement(&mut self) -> Result<Statement, ParseError> {
        let start = self.expect(Tok::Return, "`return`")?;
        let value = if self.check(&Tok::Semi) {
            None
        } else {
            Some(self.expression(0)?)
        };
        let end = self.expect(Tok::Semi, "`;` after return")?;
        Ok(Statement::Return {
            value,
            span: start.to(end),
        })
    }

    fn expression(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.prefix()?;
        loop {
            if self.check(&Tok::LParen) {
                if 15 < min_bp {
                    break;
                }
                self.bump();
                let mut args = Vec::new();
                if !self.check(&Tok::RParen) {
                    loop {
                        args.push(self.expression(0)?);
                        if !self.take(&Tok::Comma) {
                            break;
                        }
                        if self.check(&Tok::RParen) {
                            break;
                        }
                    }
                }
                let end = self.expect(Tok::RParen, "`)` after arguments")?;
                let span = lhs.span.to(end);
                lhs = Expr {
                    kind: ExprKind::Call {
                        callee: Box::new(lhs),
                        args,
                    },
                    span,
                };
                continue;
            }
            let Some((op, left_bp, right_bp)) = self.infix() else {
                break;
            };
            if left_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.expression(right_bp)?;
            let span = lhs.span.to(rhs.span);
            lhs = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
                span,
            };
        }
        Ok(lhs)
    }

    fn prefix(&mut self) -> Result<Expr, ParseError> {
        let token = self
            .current()
            .cloned()
            .ok_or_else(|| self.error("expected expression at end of file", "expression"))?;
        match token.kind {
            Tok::Int { value, suffix } => {
                self.bump();
                Ok(Expr {
                    kind: ExprKind::Integer { value, suffix },
                    span: token.span,
                })
            }
            Tok::Float { suffix } => {
                self.bump();
                Ok(Expr {
                    kind: ExprKind::Float { suffix },
                    span: token.span,
                })
            }
            Tok::Str(value) => {
                self.bump();
                Ok(Expr {
                    kind: ExprKind::String(value),
                    span: token.span,
                })
            }
            Tok::True | Tok::False => {
                self.bump();
                Ok(Expr {
                    kind: ExprKind::Bool(token.kind == Tok::True),
                    span: token.span,
                })
            }
            Tok::Ident(name) => {
                self.bump();
                Ok(Expr {
                    kind: ExprKind::Name(name),
                    span: token.span,
                })
            }
            Tok::Minus | Tok::Bang | Tok::Tilde => {
                self.bump();
                let op = match token.kind {
                    Tok::Minus => UnaryOp::Negate,
                    Tok::Bang => UnaryOp::Not,
                    _ => UnaryOp::BitNot,
                };
                let value = self.expression(13)?;
                let span = token.span.to(value.span);
                Ok(Expr {
                    kind: ExprKind::Unary {
                        op,
                        value: Box::new(value),
                    },
                    span,
                })
            }
            Tok::LParen => {
                self.bump();
                let inner = self.expression(0)?;
                let end = self.expect(Tok::RParen, "`)`")?;
                Ok(Expr {
                    span: token.span.to(end),
                    kind: ExprKind::Group(Box::new(inner)),
                })
            }
            _ => Err(ParseError {
                span: token.span,
                message: format!("unexpected {}", token.kind.describe()),
                expected: "expression".into(),
            }),
        }
    }

    fn infix(&self) -> Option<(BinaryOp, u8, u8)> {
        let (op, bp) = match self.current()?.kind {
            Tok::OrOr => (BinaryOp::Or, 1),
            Tok::AndAnd => (BinaryOp::And, 2),
            Tok::EqEq => (BinaryOp::Eq, 3),
            Tok::NotEq => (BinaryOp::NotEq, 3),
            Tok::Lt => (BinaryOp::Lt, 3),
            Tok::Le => (BinaryOp::Le, 3),
            Tok::Gt => (BinaryOp::Gt, 3),
            Tok::Ge => (BinaryOp::Ge, 3),
            Tok::Pipe => (BinaryOp::BitOr, 4),
            Tok::Caret => (BinaryOp::BitXor, 5),
            Tok::Amp => (BinaryOp::BitAnd, 6),
            Tok::Shl => (BinaryOp::Shl, 7),
            Tok::Shr => (BinaryOp::Shr, 7),
            Tok::Plus => (BinaryOp::Add, 8),
            Tok::Minus => (BinaryOp::Sub, 8),
            Tok::PlusPercent => (BinaryOp::AddWrap, 8),
            Tok::MinusPercent => (BinaryOp::SubWrap, 8),
            Tok::Star => (BinaryOp::Mul, 9),
            Tok::Slash => (BinaryOp::Div, 9),
            Tok::Percent => (BinaryOp::Rem, 9),
            Tok::StarPercent => (BinaryOp::MulWrap, 9),
            _ => return None,
        };
        Some((op, bp, bp + 1))
    }

    fn assignment_op(&self) -> Option<AssignOp> {
        match self.current()?.kind {
            Tok::Eq => Some(AssignOp::Set),
            Tok::PlusEq => Some(AssignOp::Add),
            Tok::MinusEq => Some(AssignOp::Sub),
            Tok::StarEq => Some(AssignOp::Mul),
            Tok::SlashEq => Some(AssignOp::Div),
            Tok::PercentEq => Some(AssignOp::Rem),
            Tok::CaretEq => Some(AssignOp::Xor),
            Tok::PlusPercentEq => Some(AssignOp::AddWrap),
            Tok::MinusPercentEq => Some(AssignOp::SubWrap),
            Tok::StarPercentEq => Some(AssignOp::MulWrap),
            _ => None,
        }
    }

    fn ident(&mut self, expected: &str) -> Result<String, ParseError> {
        match self.current().cloned() {
            Some(Token {
                kind: Tok::Ident(name),
                ..
            }) => {
                self.bump();
                Ok(name)
            }
            _ => Err(self.error("expected identifier", expected)),
        }
    }
    #[allow(clippy::needless_pass_by_value)] // Tok is a small enum and call sites stay readable.
    fn expect(&mut self, expected: Tok, description: &str) -> Result<Span, ParseError> {
        if self.check(&expected) {
            Ok(self.bump().map_or_default(|t| t.span))
        } else {
            Err(self.error(
                format!(
                    "found {}",
                    self.current()
                        .map_or("end of file".to_owned(), |t| t.kind.describe())
                ),
                description,
            ))
        }
    }
    fn take(&mut self, expected: &Tok) -> bool {
        if self.check(expected) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn check(&self, expected: &Tok) -> bool {
        self.current()
            .is_some_and(|t| std::mem::discriminant(&t.kind) == std::mem::discriminant(expected))
    }
    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }
    fn current_span(&self) -> Span {
        self.current().map_or(Span::default(), |t| t.span)
    }
    fn previous_span(&self) -> Span {
        self.cursor
            .checked_sub(1)
            .and_then(|i| self.tokens.get(i))
            .map_or_default(|t| t.span)
    }
    fn at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }
    fn bump(&mut self) -> Option<&Token> {
        let index = self.cursor;
        self.cursor = self.cursor.saturating_add(1);
        self.tokens.get(index)
    }
    fn error(&self, message: impl Into<String>, expected: impl Into<String>) -> ParseError {
        ParseError {
            span: self.current_span(),
            message: message.into(),
            expected: expected.into(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn parses_typed_function_and_precedence() {
        let module = parse("fn answer(x: u32) -> u32 { x + 2 * 3 }").expect("valid function");
        assert_eq!(module.functions.len(), 1);
        let function = &module.functions[0];
        assert_eq!(function.name, "answer");
        assert_eq!(function.params[0].ty, "u32");
        assert_eq!(function.return_type.as_deref(), Some("u32"));
        let tail = function.body.tail.as_ref().expect("tail expression");
        let ExprKind::Binary {
            op: BinaryOp::Add,
            right,
            ..
        } = &tail.kind
        else {
            panic!("addition at root")
        };
        assert!(matches!(
            right.kind,
            ExprKind::Binary {
                op: BinaryOp::Mul,
                ..
            }
        ));
    }

    #[test]
    fn parses_mutable_bindings_assignments_and_return() {
        let module = parse("fn main() -> i32 { let mut n: i32 = 3; n += 4; return n; }")
            .expect("valid body");
        let statements = &module.functions[0].body.statements;
        assert!(matches!(
            statements[0],
            Statement::Let { mutable: true, .. }
        ));
        assert!(matches!(
            statements[1],
            Statement::Assign {
                op: AssignOp::Add,
                ..
            }
        ));
        assert!(matches!(statements[2], Statement::Return { .. }));
    }

    #[test]
    fn parses_function_kinds_and_inout() {
        let module = parse("pure fn f(x: f32) {} rev fn step(inout x: u32) { x +%= 1; }")
            .expect("valid kinds");
        assert_eq!(module.functions[0].kind, FunctionKind::Pure);
        assert_eq!(module.functions[1].kind, FunctionKind::Reversible);
        assert!(module.functions[1].params[0].inout);
    }

    #[test]
    fn malformed_syntax_has_a_source_span_and_expected_item() {
        let error = parse("fn f(x u32) {}").expect_err("missing colon");
        assert!(error.contains("parse error at bytes"));
        assert!(error.contains("expected `:` before parameter type"));
    }

    #[test]
    fn empty_module_parses() {
        assert_eq!(parse("").expect("empty module").functions, []);
    }
}
