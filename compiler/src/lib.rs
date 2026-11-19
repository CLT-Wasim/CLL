//! `cllc`: the CLL compiler.
//!
//! # Scope
//!
//! This crate is the compiler for CLL (Conserving Logic Language). It follows the
//! pipeline fixed by DECISIONS [55]:
//!
//! ```text
//! Lexer -> Parser -> AST -> HIR -> MIR (SSA) -> { interpreter | C emitter }
//! ```
//!
//! The reverse transformation and the effect checks live on MIR (DECISIONS [56]).
//!
//! # Current contents
//!
//! Only [`revtable`] exists so far. It is the executable model of SPEC section 8.2,
//! the reversible primitive table. It is written before the frontend on purpose:
//! the table is the one part of the language where being wrong is unrecoverable
//! later, so it is proven first and everything else is built to match it.
//!
//! Nothing in this crate is speculative API. Each module states which SPEC rules it
//! implements.

#![forbid(unsafe_code)]

pub mod revtable;
