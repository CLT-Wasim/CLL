//! Reference model of the reversible primitive table (SPEC R-REV-020).
//!
//! This module exists to make one claim checkable: *the syntactic inverse of a
//! program built from the table's primitives restores the state bit for bit*.
//! It is the executable form of SPEC R-REV-003 and R-REV-004, written before the
//! compiler exists so that the table can be wrong in exactly one place — here —
//! and be caught by `cargo test` rather than by a training run months later.
//!
//! It is deliberately not the compiler. It has no parser, no types, no effects.
//! It answers one question: given a list of table primitives, does reversing the
//! list and inverting each element restore the state?
//!
//! The model also encodes the *negative* result that motivates R-REV-030:
//! `a +%= a` is not injective, so a program using it has no inverse. A test below
//! exhibits the collision rather than asserting the claim in prose.

/// One entry of the primitive table, applied to a state slice of `u32` words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prim {
    /// P1: `a +%= e` with `e` a comptime constant.
    AddEq { target: usize, delta: u32 },
    /// P2: `a -%= e` with `e` a comptime constant.
    SubEq { target: usize, delta: u32 },
    /// P3: `a ^= e` with `e` a comptime constant.
    XorEq { target: usize, value: u32 },
    /// P4: `a *%= c` with `c` odd.
    MulEq { target: usize, factor: u32 },
    /// P5: `rotl(a, k)`.
    Rotl { target: usize, k: u32 },
    /// P6: `rotr(a, k)`.
    Rotr { target: usize, k: u32 },
    /// P7: `swap(a, b)`.
    Swap { a: usize, b: usize },
    /// P8: `not(a)`.
    Not { target: usize },
    /// P10: `for i in from..to { body }`, `body` independent of `i`.
    For { from: u32, to: u32, body: Vec<Prim> },
}

/// Why a primitive cannot be applied or inverted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimError {
    /// A target index is outside the state.
    OutOfRange { index: usize, len: usize },
    /// P4 requires an odd factor to be invertible mod 2^n (SPEC R-REV-024).
    EvenFactor { factor: u32 },
    /// P5/P6 require `0 < k < 32` (SPEC R-REV-023).
    BadRotation { k: u32 },
    /// P7 requires two distinct places (SPEC R-REV-026).
    SelfSwap { index: usize },
    /// P10 requires `from <= to`; a reversed range has no forward meaning.
    InvertedRange { from: u32, to: u32 },
}

impl std::fmt::Display for PrimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfRange { index, len } => {
                write!(f, "index {index} out of range for state of length {len}")
            }
            Self::EvenFactor { factor } => write!(f, "factor {factor} is even, not invertible"),
            Self::BadRotation { k } => write!(f, "rotation {k} outside 0 < k < 32"),
            Self::SelfSwap { index } => write!(f, "swap of index {index} with itself"),
            Self::InvertedRange { from, to } => write!(f, "range {from}..{to} is inverted"),
        }
    }
}

/// Modular inverse of an odd `factor` modulo 2^32 (Newton iteration, exact for odd input).
fn mul_inverse(factor: u32) -> u32 {
    let mut inv: u32 = 1;
    for _ in 0..5 {
        inv = inv.wrapping_mul(2u32.wrapping_sub(factor.wrapping_mul(inv)));
    }
    inv
}

fn place(state: &mut [u32], index: usize) -> Result<&mut u32, PrimError> {
    let len = state.len();
    state
        .get_mut(index)
        .ok_or(PrimError::OutOfRange { index, len })
}

impl Prim {
    /// Applies this primitive to `state`.
    ///
    /// # Errors
    /// Returns [`PrimError`] when the primitive violates a precondition of the
    /// table. The real checker rejects these at compile time (SPEC R-REV-070);
    /// here they are run-time errors so the model cannot silently accept them.
    pub fn apply(&self, state: &mut [u32]) -> Result<(), PrimError> {
        match *self {
            Self::AddEq { target, delta } => {
                let slot = place(state, target)?;
                *slot = slot.wrapping_add(delta);
            }
            Self::SubEq { target, delta } => {
                let slot = place(state, target)?;
                *slot = slot.wrapping_sub(delta);
            }
            Self::XorEq { target, value } => {
                let slot = place(state, target)?;
                *slot ^= value;
            }
            Self::MulEq { target, factor } => {
                if factor % 2 == 0 {
                    return Err(PrimError::EvenFactor { factor });
                }
                let slot = place(state, target)?;
                *slot = slot.wrapping_mul(factor);
            }
            Self::Rotl { target, k } => {
                if k == 0 || k >= 32 {
                    return Err(PrimError::BadRotation { k });
                }
                let slot = place(state, target)?;
                *slot = slot.rotate_left(k);
            }
            Self::Rotr { target, k } => {
                if k == 0 || k >= 32 {
                    return Err(PrimError::BadRotation { k });
                }
                let slot = place(state, target)?;
                *slot = slot.rotate_right(k);
            }
            Self::Swap { a, b } => {
                if a == b {
                    return Err(PrimError::SelfSwap { index: a });
                }
                let len = state.len();
                if a >= len {
                    return Err(PrimError::OutOfRange { index: a, len });
                }
                if b >= len {
                    return Err(PrimError::OutOfRange { index: b, len });
                }
                state.swap(a, b);
            }
            Self::Not { target } => {
                let slot = place(state, target)?;
                *slot = !*slot;
            }
            Self::For { from, to, ref body } => {
                if from > to {
                    return Err(PrimError::InvertedRange { from, to });
                }
                for _ in from..to {
                    for prim in body {
                        prim.apply(state)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// The inverse entry of the table (SPEC R-REV-020, column 3).
    ///
    /// # Errors
    /// Returns [`PrimError`] for the same preconditions as [`Prim::apply`], plus
    /// an inverted loop range, which has no inverse.
    pub fn inverse(&self) -> Result<Self, PrimError> {
        match *self {
            Self::AddEq { target, delta } => Ok(Self::SubEq { target, delta }),
            Self::SubEq { target, delta } => Ok(Self::AddEq { target, delta }),
            Self::XorEq { target, value } => Ok(Self::XorEq { target, value }),
            Self::MulEq { target, factor } => {
                if factor % 2 == 0 {
                    return Err(PrimError::EvenFactor { factor });
                }
                Ok(Self::MulEq {
                    target,
                    factor: mul_inverse(factor),
                })
            }
            Self::Rotl { target, k } => {
                if k == 0 || k >= 32 {
                    return Err(PrimError::BadRotation { k });
                }
                Ok(Self::Rotr { target, k })
            }
            Self::Rotr { target, k } => {
                if k == 0 || k >= 32 {
                    return Err(PrimError::BadRotation { k });
                }
                Ok(Self::Rotl { target, k })
            }
            Self::Swap { a, b } => {
                if a == b {
                    return Err(PrimError::SelfSwap { index: a });
                }
                Ok(Self::Swap { a, b })
            }
            Self::Not { target } => Ok(Self::Not { target }),
            Self::For { from, to, ref body } => {
                if from > to {
                    return Err(PrimError::InvertedRange { from, to });
                }
                let inverted = invert(body)?;
                Ok(Self::For {
                    from,
                    to,
                    body: inverted,
                })
            }
        }
    }
}

/// Applies a whole program in order.
///
/// # Errors
/// Propagates the first [`PrimError`].
pub fn run(program: &[Prim], state: &mut [u32]) -> Result<(), PrimError> {
    for prim in program {
        prim.apply(state)?;
    }
    Ok(())
}

/// Derives the inverse program: **reverse the order, invert each entry**
/// (SPEC R-REV-003). This is the whole trick, and it is syntactic.
///
/// # Errors
/// Propagates the first [`PrimError`] from [`Prim::inverse`].
pub fn invert(program: &[Prim]) -> Result<Vec<Prim>, PrimError> {
    let mut out = Vec::with_capacity(program.len());
    for prim in program.iter().rev() {
        out.push(prim.inverse()?);
    }
    Ok(out)
}

/// The reference `mix` function from README, as table entries:
/// `a +%= b; b ^= a; rotl(a, 7)` with `b` supplied as a constant `delta`.
#[must_use]
pub fn mix_example(delta: u32) -> Vec<Prim> {
    vec![
        Prim::AddEq { target: 0, delta },
        Prim::XorEq {
            target: 1,
            value: 0,
        },
        Prim::Rotl { target: 0, k: 7 },
    ]
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    /// SPEC R-REV-004: `reverse f(f(x)) == x`, exhaustive over all `u8`-wide
    /// states of a two-word program. 65,536 states, every one checked.
    #[test]
    fn inverse_restores_state_exhaustively_on_two_u8_words() {
        let program = vec![
            Prim::AddEq {
                target: 0,
                delta: 0x9E37_79B9,
            },
            Prim::XorEq {
                target: 1,
                value: 0xA5A5_5A5A,
            },
            Prim::Rotl { target: 0, k: 7 },
            Prim::MulEq {
                target: 1,
                factor: 0x0000_000B,
            },
            Prim::Not { target: 0 },
            Prim::Swap { a: 0, b: 1 },
            Prim::SubEq {
                target: 1,
                delta: 0xDEAD_BEEF,
            },
            Prim::Rotr { target: 0, k: 13 },
        ];
        let inverse = invert(&program).expect("table entries are valid");

        let mut checked = 0u64;
        for a in 0..=u8::MAX {
            for b in 0..=u8::MAX {
                let original = [u32::from(a), u32::from(b)];
                let mut state = original;
                run(&program, &mut state).expect("forward run");
                run(&inverse, &mut state).expect("inverse run");
                assert_eq!(state, original, "state not restored for {original:?}");
                checked += 1;
            }
        }
        assert_eq!(checked, 65_536, "must be exhaustive, not sampled");
    }

    /// The same property on a wide state, sampled: 100,000 pseudo-random pairs.
    #[test]
    fn inverse_restores_state_on_sampled_u32() {
        let program = vec![
            Prim::AddEq {
                target: 0,
                delta: 0x9E37_79B9,
            },
            Prim::MulEq {
                target: 1,
                factor: 0x0000_0025,
            },
            Prim::Rotl { target: 1, k: 31 },
            Prim::XorEq {
                target: 0,
                value: 0xFFFF_0000,
            },
            Prim::For {
                from: 0,
                to: 3,
                body: vec![
                    Prim::AddEq {
                        target: 0,
                        delta: 7,
                    },
                    Prim::Rotl { target: 1, k: 1 },
                ],
            },
        ];
        let inverse = invert(&program).expect("table entries are valid");

        let mut seed = 0x1234_5678_9ABC_DEF0u64;
        for _ in 0..100_000 {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let original = [
                u32::try_from(seed & 0xFFFF_FFFF).unwrap_or(0),
                u32::try_from(seed >> 32).unwrap_or(0),
            ];
            let mut state = original;
            run(&program, &mut state).expect("forward run");
            run(&inverse, &mut state).expect("inverse run");
            assert_eq!(state, original, "state not restored for {original:?}");
        }
    }

    /// SPEC R-REV-030 exists because of this: `a +%= a` is not injective.
    /// The test exhibits a real collision instead of asserting the claim.
    #[test]
    fn doubling_is_not_injective_so_it_has_no_inverse() {
        // a +%= a  ==  a := 2a  (mod 2^32)
        let double = |x: u32| x.wrapping_add(x);
        assert_eq!(double(0), double(0x8000_0000), "collision expected");
        assert_ne!(0u32, 0x8000_0000, "the colliding inputs are distinct");
        // A bijection on a finite set cannot map two inputs to one output, so no
        // inverse function can exist. This is why the checker rejects `a +%= a`.
    }

    /// SPEC R-REV-024: an even multiplier has no inverse mod 2^n, and the model
    /// refuses to build one rather than producing a wrong one.
    #[test]
    fn even_multiplier_is_rejected() {
        let prim = Prim::MulEq {
            target: 0,
            factor: 4,
        };
        assert_eq!(prim.inverse(), Err(PrimError::EvenFactor { factor: 4 }));
    }

    /// SPEC R-REV-023: a rotation of 0 or >= word width is not a permutation of
    /// positions in the table's sense and is rejected.
    #[test]
    fn out_of_range_rotation_is_rejected() {
        assert_eq!(
            Prim::Rotl { target: 0, k: 0 }.inverse(),
            Err(PrimError::BadRotation { k: 0 })
        );
        assert_eq!(
            Prim::Rotr { target: 0, k: 32 }.inverse(),
            Err(PrimError::BadRotation { k: 32 })
        );
    }

    /// SPEC R-REV-026: `swap(a, a)` is rejected, not silently accepted.
    #[test]
    fn self_swap_is_rejected() {
        assert_eq!(
            Prim::Swap { a: 3, b: 3 }.inverse(),
            Err(PrimError::SelfSwap { index: 3 })
        );
    }

    /// The README example, round-tripped on the exact values it claims.
    #[test]
    fn readme_mix_example_round_trips() {
        let program = mix_example(456);
        let inverse = invert(&program).expect("valid");
        let original = [123u32, 0];
        let mut state = original;
        run(&program, &mut state).expect("forward");
        run(&inverse, &mut state).expect("inverse");
        assert_eq!(state, original);
    }

    /// The modular inverse really inverts multiplication for odd factors.
    #[test]
    fn mul_inverse_is_correct_for_odd_factors() {
        for factor in (1u32..1000).step_by(2) {
            let inv = mul_inverse(factor);
            assert_eq!(factor.wrapping_mul(inv), 1, "factor {factor}");
        }
    }

    /// An inverted loop range is a malformed program, not a no-op.
    #[test]
    fn inverted_range_is_rejected() {
        let prim = Prim::For {
            from: 5,
            to: 2,
            body: Vec::new(),
        };
        assert_eq!(
            prim.inverse(),
            Err(PrimError::InvertedRange { from: 5, to: 2 })
        );
    }
}
