# CLL implementation status and handoff

**Last updated:** 2026-11-19
**Scope:** v1 compiler and language; this file must be updated at every milestone.

## Start here

Read in this order before changing code:

1. `AGENTS.md` — binding project rules, language split, task protocol and gates.
2. `docs/SPEC.md` — normative language behavior; this M1 branch proposes v0.2-draft changes.
3. `DECISIONS.md` — design rationale and explicit review status. Decisions [149]–[154] remain marked owner sign-off pending; additions [155]–[159] are implementation proposals, not accepted owner decisions.
4. `ROADMAP.md` — mandatory milestones and measurable exit criteria.
5. `grammar.ebnf` and `docs/IR.md` — surface syntax and AST/HIR/MIR contracts.
6. `docs/ERRORS.md`, `docs/errors/`, and `tests/conformance/` — stable diagnostics and executable rule references.

Compiler/interpreter/formatter/xtask are Rust. C is the v1 generated backend target. CLL remains the program, test, standard-library, automation and benchmark-kernel language. Do not write a toy interpreter or route unfinished CLL programs through another language.

## Current implementation (truth, not a completion claim)

`main` is based on `749c4b1` (M3 lexer and honest conformance runner). The compiler has a hand-written lexer and lexical tests, a conformance header parser/runner, and the reversible primitive-table utility. It does **not** yet have a CLL parser/AST implementation, name/type/effect/capability checker, general MIR, reference interpreter, C emitter, or standard library. No claim of a finished language or performance result is justified yet.

The M1 working branch contains a v0.2-draft SPEC/grammar/IR reconciliation, a single E0011 diagnostic page and conformance case, plus a fix to the `P-ERRDOC` policy so error pages are matched against `compile-error E####` expectations rather than SPEC rule IDs. The design additions [155]–[159] are proposals for review. The branch must not be represented as merged or owner-signed-off; `main` remains protected.

## Baseline and gates

Rust toolchain used in the sandbox: rustc 1.100.0, cargo 1.100.0; components `rustfmt` and `clippy` installed. The last complete gate run reported:

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo test --workspace`: 55 passed, 0 failed (44 compiler-library + 11 xtask tests).
- `cargo run -p xtask -- policy`: pass; 40 files scanned, 40 conformance tests, 1 error page.
- `cargo run -p xtask -- test --allow-unimplemented`: 5 passed, 0 failed, 35 not implemented. The TODOs are honestly reported because parser/checker/interpreter stages are absent.

These results must be rerun after later edits; they are not M1/M2 completion evidence by themselves.

## Next work (mandatory ROADMAP order)

1. **M1 — SPEC v0.2 draft:** finalize and review the SPEC, complete EBNF cross-audit, IR contract and owner sign-off. Do not change pending decisions to “approved” without explicit owner review. No direct push/merge to `main`.
2. **M2 — conformance suite v0:** raise the suite from 40 to at least 150 well-formed positive and negative `.cll` cases, each with a real SPEC rule and expected outcome. Add at least ten adversarial soundness/capability/UB/determinism cases. Keep them TODO until implementation exists; do not fabricate passes.
3. **M3 — frontend:** parser, AST spans, diagnostics and canonical formatter; add syntax tests first, then parse/print round-trip, formatter-idempotence and fuzz coverage.
4. **M4 — static semantics:** name resolution, types/inference, const generics, exhaustive matches, mutability/inout, effects and capabilities.
5. **M5 — reversible core:** checker, inverse derivation on MIR, reference interpreter, exhaustive/sampled inverse properties and soundness regression tests.
6. **M6 — C backend:** MIR-to-C, differential gcc/clang runs at O0/O2, sanitizers, source spans and benchmark baseline.
7. Continue M7–M12 in `ROADMAP.md`: standard library/capabilities, fixed-point/coupling, tensors/interop, real training, tooling/security audit, v1 release gates. v2 backlog is separate and is not part of v1 completion.

## Per-task workflow

One milestone/task per branch and PR. Cite the SPEC rules; add positive and negative tests before implementation; run the full gates and include real outputs; update `DECISIONS.md` and this status file when behavior/status changes. Do not merge your own SPEC/RFC change. At each task boundary report: what changed, commit/PR, exact commands/results, tests added/passed/failed, unresolved issues, and next step.
