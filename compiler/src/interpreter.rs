//! Small reference-execution core for the parsed CLL integer subset.
//!
//! This is deliberately an executable semantics module, not a claim that all CLL
//! types or statements are implemented. Unsupported constructs fail explicitly.

use crate::parser::{
    self, AssignOp, BinaryOp, Expr, ExprKind, Function, Module, Statement, UnaryOp,
};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Integer {
        bits: u128,
        width: u32,
        signed: bool,
    },
    Bool(bool),
    String(String),
    Unit,
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer {
                bits,
                width,
                signed,
            } => {
                if *signed && *width < 128 && bits & (1u128 << (width - 1)) != 0 {
                    let value = bits.cast_signed() - (1i128 << width);
                    write!(f, "{value}")
                } else {
                    write!(f, "{bits}")
                }
            }
            Self::Bool(value) => write!(f, "{value}"),
            Self::String(value) => write!(f, "{value}"),
            Self::Unit => write!(f, "()"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub message: String,
}
impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for RuntimeError {}

/// Parse and evaluate `fn main() -> integer_type { ... }`; returns the main value.
///
/// # Errors
/// Returns a diagnostic when parsing fails, an unsupported construct is used, or
/// checked evaluation traps.
pub fn run(source: &str) -> Result<Value, String> {
    let module = parser::parse(source)?;
    let mut vm = Vm::new(&module);
    vm.call("main", Vec::new())
        .map_err(|error| error.to_string())
}

struct Binding {
    value: Value,
    mutable: bool,
}
struct Vm<'a> {
    functions: BTreeMap<&'a str, &'a Function>,
    depth: usize,
}
impl<'a> Vm<'a> {
    fn new(module: &'a Module) -> Self {
        Self {
            functions: module
                .functions
                .iter()
                .map(|f| (f.name.as_str(), f))
                .collect(),
            depth: 0,
        }
    }

    fn call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, RuntimeError> {
        if self.depth >= 256 {
            return Err(err("call depth exceeded 256"));
        }
        let function = self
            .functions
            .get(name)
            .copied()
            .ok_or_else(|| err(format!("unknown function `{name}`")))?;
        if args.len() != function.params.len() {
            return Err(err(format!(
                "`{name}` expects {} arguments, got {}",
                function.params.len(),
                args.len()
            )));
        }
        if function.kind == parser::FunctionKind::Reversible {
            return Err(err(
                "calling a rev fn requires place-aware state execution; not yet implemented",
            ));
        }
        let mut env = BTreeMap::new();
        for (param, value) in function.params.iter().zip(args) {
            if param.inout {
                return Err(err(
                    "inout parameters are not yet executable in this interpreter slice",
                ));
            }
            env.insert(
                param.name.clone(),
                Binding {
                    value,
                    mutable: false,
                },
            );
        }
        self.depth += 1;
        let result = self.eval_block(
            function,
            &function.body.statements,
            function.body.tail.as_ref(),
            &mut env,
        );
        self.depth -= 1;
        result
    }

    fn eval_block(
        &mut self,
        _function: &Function,
        statements: &[Statement],
        tail: Option<&Expr>,
        env: &mut BTreeMap<String, Binding>,
    ) -> Result<Value, RuntimeError> {
        for statement in statements {
            match statement {
                Statement::Let {
                    mutable,
                    name,
                    ty,
                    value,
                    ..
                } => {
                    let val = self.eval(value, env)?;
                    let value = if let Some(ty) = ty {
                        coerce_literal_type(val, ty)?
                    } else {
                        val
                    };
                    env.insert(
                        name.clone(),
                        Binding {
                            value,
                            mutable: *mutable,
                        },
                    );
                }
                Statement::Return { value, .. } => {
                    return value
                        .as_ref()
                        .map_or(Ok(Value::Unit), |expr| self.eval(expr, env));
                }
                Statement::Expr { value, .. } => {
                    let _ = self.eval(value, env)?;
                }
                Statement::Assign {
                    place, op, value, ..
                } => {
                    let ExprKind::Name(name) = &place.kind else {
                        return Err(err("interpreter assignment target must be a local name"));
                    };
                    let rhs = self.eval(value, env)?;
                    let binding = env
                        .get_mut(name)
                        .ok_or_else(|| err(format!("unknown local `{name}`")))?;
                    if !binding.mutable {
                        return Err(err(format!("cannot assign to immutable binding `{name}`")));
                    }
                    binding.value = assign(binding.value.clone(), rhs, *op)?;
                }
            }
        }
        tail.map_or(Ok(Value::Unit), |expr| self.eval(expr, env))
    }

    fn eval(
        &mut self,
        expr: &Expr,
        env: &mut BTreeMap<String, Binding>,
    ) -> Result<Value, RuntimeError> {
        match &expr.kind {
            ExprKind::Integer { value, suffix } => {
                let (width, signed) = match suffix {
                    Some(crate::lexer::IntSuffix::U8) => (8, false),
                    Some(crate::lexer::IntSuffix::U16) => (16, false),
                    Some(crate::lexer::IntSuffix::U32) => (32, false),
                    Some(crate::lexer::IntSuffix::U64 | crate::lexer::IntSuffix::Usize) | None => {
                        (64, false)
                    }
                    Some(crate::lexer::IntSuffix::I8) => (8, true),
                    Some(crate::lexer::IntSuffix::I16) => (16, true),
                    Some(crate::lexer::IntSuffix::I32) => (32, true),
                    Some(crate::lexer::IntSuffix::I64) => (64, true),
                    Some(crate::lexer::IntSuffix::U128 | crate::lexer::IntSuffix::I128) => {
                        return Err(err("u128/i128 interpretation is not yet implemented"));
                    }
                };
                Ok(Value::Integer {
                    bits: *value,
                    width,
                    signed,
                })
            }
            ExprKind::Bool(value) => Ok(Value::Bool(*value)),
            ExprKind::String(value) => Ok(Value::String(value.clone())),
            ExprKind::Float { .. } => {
                Err(err("floating-point interpretation is not yet implemented"))
            }
            ExprKind::Name(name) => env
                .get(name)
                .map(|binding| binding.value.clone())
                .ok_or_else(|| err(format!("unknown name `{name}`"))),
            ExprKind::Group(inner) => self.eval(inner, env),
            ExprKind::Unary { op, value } => {
                let value = self.eval(value, env)?;
                unary(value, *op)
            }
            ExprKind::Binary { op, left, right } => {
                let lhs = self.eval(left, env)?;
                if *op == BinaryOp::And && lhs == Value::Bool(false) {
                    return Ok(Value::Bool(false));
                }
                if *op == BinaryOp::Or && lhs == Value::Bool(true) {
                    return Ok(Value::Bool(true));
                }
                let rhs = self.eval(right, env)?;
                binary(lhs, rhs, *op)
            }
            ExprKind::Call { callee, args } => {
                let ExprKind::Name(name) = &callee.kind else {
                    return Err(err("only named function calls are supported"));
                };
                let values = args
                    .iter()
                    .map(|arg| self.eval(arg, env))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call(name, values)
            }
        }
    }
}

fn coerce_literal_type(value: Value, ty: &str) -> Result<Value, RuntimeError> {
    let Value::Integer { bits, .. } = value else {
        return Ok(value);
    };
    let (width, signed) = match ty {
        "u8" => (8, false),
        "u16" => (16, false),
        "u32" => (32, false),
        "u64" | "usize" => (64, false),
        "i8" => (8, true),
        "i16" => (16, true),
        "i32" => (32, true),
        "i64" => (64, true),
        _ => return Err(err(format!("interpreter does not support type `{ty}`"))),
    };
    let mask = (1u128 << width) - 1;
    let exceeds_range = if signed {
        bits >= (1u128 << (width - 1))
    } else {
        bits > mask
    };
    if exceeds_range {
        return Err(err(format!("integer literal does not fit `{ty}`")));
    }
    Ok(Value::Integer {
        bits,
        width,
        signed,
    })
}
fn unary(value: Value, op: UnaryOp) -> Result<Value, RuntimeError> {
    match (op, value) {
        (
            UnaryOp::Negate,
            Value::Integer {
                bits,
                width,
                signed: true,
            },
        ) => {
            let n = signed_value(bits, width);
            let min = -(1i128 << (width - 1));
            let max = (1i128 << (width - 1)) - 1;
            let result = n
                .checked_neg()
                .filter(|n| (min..=max).contains(n))
                .ok_or_else(|| err("integer overflow (E0100)"))?;
            Ok(Value::Integer {
                bits: result.cast_unsigned() & ((1u128 << width) - 1),
                width,
                signed: true,
            })
        }
        (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
        (
            UnaryOp::BitNot,
            Value::Integer {
                bits,
                width,
                signed,
            },
        ) => Ok(Value::Integer {
            bits: (!bits) & ((1u128 << width) - 1),
            width,
            signed,
        }),
        _ => Err(err("invalid unary operand type")),
    }
}
#[allow(clippy::too_many_lines)] // Keep finite operator semantics auditable in one dispatch.
fn binary(lhs: Value, rhs: Value, op: BinaryOp) -> Result<Value, RuntimeError> {
    let (
        Value::Integer {
            bits: a,
            width,
            signed,
        },
        Value::Integer {
            bits: b,
            width: bw,
            signed: bs,
        },
    ) = (lhs.clone(), rhs.clone())
    else {
        return match (lhs, rhs, op) {
            (Value::Bool(a), Value::Bool(b), BinaryOp::And) => Ok(Value::Bool(a && b)),
            (Value::Bool(a), Value::Bool(b), BinaryOp::Or) => Ok(Value::Bool(a || b)),
            (a, b, BinaryOp::Eq) => Ok(Value::Bool(a == b)),
            (a, b, BinaryOp::NotEq) => Ok(Value::Bool(a != b)),
            _ => Err(err("unsupported operand types")),
        };
    };
    if width != bw || signed != bs {
        return Err(err("integer operands have different types"));
    }
    let mask = (1u128 << width) - 1;
    let result = match op {
        BinaryOp::AddWrap => Value::Integer {
            bits: a.wrapping_add(b) & mask,
            width,
            signed,
        },
        BinaryOp::SubWrap => Value::Integer {
            bits: a.wrapping_sub(b) & mask,
            width,
            signed,
        },
        BinaryOp::MulWrap => Value::Integer {
            bits: a.wrapping_mul(b) & mask,
            width,
            signed,
        },
        BinaryOp::BitXor => Value::Integer {
            bits: (a ^ b) & mask,
            width,
            signed,
        },
        BinaryOp::BitAnd => Value::Integer {
            bits: (a & b) & mask,
            width,
            signed,
        },
        BinaryOp::BitOr => Value::Integer {
            bits: (a | b) & mask,
            width,
            signed,
        },
        BinaryOp::Eq => Value::Bool(a == b),
        BinaryOp::NotEq => Value::Bool(a != b),
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
            let (a, b) = if signed {
                (signed_value(a, width), signed_value(b, width))
            } else {
                (a.cast_signed(), b.cast_signed())
            };
            Value::Bool(match op {
                BinaryOp::Lt => a < b,
                BinaryOp::Le => a <= b,
                BinaryOp::Gt => a > b,
                _ => a >= b,
            })
        }
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul => {
            let (a, b) = if signed {
                (signed_value(a, width), signed_value(b, width))
            } else {
                (
                    i128::try_from(a).map_err(|_| err("integer overflow (E0100)"))?,
                    i128::try_from(b).map_err(|_| err("integer overflow (E0100)"))?,
                )
            };
            let value = match op {
                BinaryOp::Add => a.checked_add(b),
                BinaryOp::Sub => a.checked_sub(b),
                _ => a.checked_mul(b),
            }
            .ok_or_else(|| err("integer overflow (E0100)"))?;
            let (min, max) = if signed {
                (-(1i128 << (width - 1)), (1i128 << (width - 1)) - 1)
            } else {
                (
                    0,
                    if width == 127 {
                        i128::MAX
                    } else {
                        (1i128 << width) - 1
                    },
                )
            };
            if value < min || value > max {
                return Err(err("integer overflow (E0100)"));
            }
            Value::Integer {
                bits: value.cast_unsigned() & mask,
                width,
                signed,
            }
        }
        BinaryOp::Div | BinaryOp::Rem => {
            if b == 0 {
                return Err(err("division by zero (E0101)"));
            }
            let (a, b) = if signed {
                (signed_value(a, width), signed_value(b, width))
            } else {
                (a.cast_signed(), b.cast_signed())
            };
            let v = if op == BinaryOp::Div {
                a.checked_div(b)
            } else {
                a.checked_rem(b)
            }
            .ok_or_else(|| err("integer overflow (E0100)"))?;
            Value::Integer {
                bits: v.cast_unsigned() & mask,
                width,
                signed,
            }
        }
        _ => return Err(err("unsupported binary operator in interpreter")),
    };
    Ok(result)
}
fn assign(lhs: Value, rhs: Value, op: AssignOp) -> Result<Value, RuntimeError> {
    let bin = match op {
        AssignOp::Set => return Ok(rhs),
        AssignOp::Add => BinaryOp::Add,
        AssignOp::Sub => BinaryOp::Sub,
        AssignOp::Mul => BinaryOp::Mul,
        AssignOp::Div => BinaryOp::Div,
        AssignOp::Rem => BinaryOp::Rem,
        AssignOp::Xor => BinaryOp::BitXor,
        AssignOp::AddWrap => BinaryOp::AddWrap,
        AssignOp::SubWrap => BinaryOp::SubWrap,
        AssignOp::MulWrap => BinaryOp::MulWrap,
    };
    binary(lhs, rhs, bin)
}
fn signed_value(bits: u128, width: u32) -> i128 {
    let sign = 1u128 << (width - 1);
    if bits & sign == 0 {
        bits.cast_signed()
    } else {
        bits.cast_signed() - (1i128 << width)
    }
}
fn err(message: impl Into<String>) -> RuntimeError {
    RuntimeError {
        message: message.into(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    #[test]
    fn runs_typed_function_calls_and_wrapping_checksum() {
        assert_eq!(
            run(include_str!("../../examples/checksum.cll"))
                .expect("repository example should run")
                .to_string(),
            "874950152"
        );
    }
    #[test]
    fn traps_checked_overflow() {
        assert!(
            run("fn main() -> u8 { 255u8 + 1u8 }")
                .expect_err("overflow must trap")
                .contains("E0100")
        );
    }
    #[test]
    fn wraps_modulo_integer_width() {
        assert_eq!(
            run("fn main() -> u8 { 255u8 +% 1u8 }")
                .expect("wrap is total")
                .to_string(),
            "0"
        );
    }

    #[test]
    fn traps_signed_minimum_negation_overflow() {
        assert!(
            run("fn main() -> i64 { -9223372036854775808i64 }")
                .expect_err("negating the minimum signed integer must trap")
                .contains("E0100")
        );
    }
}
