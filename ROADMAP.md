# ROADMAP.md: CLL milestones

Each milestone has **exit criteria that are commands with expected results**. A milestone is done only when all criteria pass and the report (AGENTS.md section 14) is filed. Order is mandatory; spec precedes code.

Scope narrows per milestone, quality never does: nothing built here is throwaway.

---

## M0: Foundations
- Repos created (AGENTS.md section 6), `main` protected, CODEOWNERS, CI skeleton.
- `cargo xtask policy` implemented (forbidden-language check, SPEC-reference check).
- **Exit:** a PR containing a `.py` file under `examples/` fails CI (demonstrated with a real failing run).

## M1: Specification v0.1
- `docs/SPEC.md` with rule IDs: lexical, types, mutability/`inout`, effects (`erase`, `inexact`), capabilities, and the **complete `rev fn` chapter**: primitive table (forward/inverse/preconditions), conditional, loops, ancillas, `couple`, determinism domain.
- `grammar.ebnf` for the v1 syntax; `docs/IR.md` with AST/HIR/MIR definitions; error catalogue skeleton.
- Red team performs an adversarial read and files at least 10 attack programs against the spec (as drafts of negative tests).
- **Exit:** owner sign-off; every rule has an ID; every primitive has a stated inverse and a stated totality argument.

## M2: Conformance suite v0 (red on purpose)
- At least 150 tests: positive and negative, each citing a rule ID, using the header directives in AGENTS.md section 8.
- Includes the known traps (AGENTS section 5): `x // 2` as `rev`, trap `+` in `rev`, float state in `rev`, `a +%= a`, implicit drop in `rev`, capability escape.
- **Exit:** runner executes all tests; all fail only because the compiler does not exist yet (no harness errors).

## M3: Frontend
- Lexer, parser, AST, canonical formatter, diagnostics with spans.
- **Exit:** all syntax tests pass; `fmt(fmt(x)) == fmt(x)` and `parse(print(ast)) == ast` property tests pass; parser fuzz target runs 30 min with no crash; grammar freeze announced (LSP work may start).

## M4: Static semantics
- Name resolution, type checking and inference, const-generic arithmetic, exhaustiveness, mutability, `inout` exclusivity, effect inference (`erase`, `inexact`), capability checking.
- **Exit:** all type/effect/capability conformance tests pass. Corpus measurement published: fraction of std-candidate functions that are conserving (decision [30] risk check).

## M5: Reversible core
- `rev` checker (primitive table, `e` must not read target, conditional rules, bounded loops, ancilla rules), inverse derivation on MIR, reference interpreter.
- **Exit:**
  - every `rev` negative test is rejected with the right error code;
  - inverse property passes: exhaustive on two-`u8` and one-`u16` state spaces, 10^7 seeded samples on `u64` state;
  - mutation check: corrupting one inverse in the table turns the suite red;
  - 20+ `rev fn` examples (ARX-style mixing, Feistel rounds, reversible counters) in CLL.

## M6: C backend and differential testing
- MIR -> C emitter; generated C flags per AGENTS section 5; source-line mapping for DWARF.
- **Exit:** interpreter vs compiled output identical on all run-ok tests under gcc and clang, `-O0` and `-O2`; UBSan and ASan clean; first benchmark baseline stored; compile-speed gate active.

## M7: Standard library, capabilities, real automation
- `cll-std`: core/alloc/std, deterministic collections, strings, own math library, `Clock`/`Rng`/file/net capabilities with attenuation.
- **Exit:** two real automation tools written **entirely in CLL**, compiled and run:
  1. a directory integrity hasher that holds only `FileRead("/data")`;
  2. a log reducer that holds `FileRead` plus `FileWrite` on separate scopes.
  Negative tests prove that code without the token cannot compile when it touches the resource, and that attenuation cannot widen scope.

## M8: Fixed-point, `quantize`, `couple`, first reversible block
- `fx/ufx` types, `quantize`, own `exp/tanh/...`, `a +%= F(b)` with float inside `F`.
- **Exit:** a 2-block integer/fixed-point reversible network; inverse property bit-exact on random inputs (10^6); determinism: two runs byte-identical; depth sweep 8/16/32/64 on `cll-bench` shows allocator-tracked activation bytes with slope near zero vs a stored-activation baseline written in CLL.

## M9: Tensors, autodiff, interop
- `cll-tensor` shape types with compile-time mismatch errors; `cll-autodiff` source transform; `cll-ffi` C ABI and DLPack zero-copy with PyTorch.
- **Exit:** gradient check against finite differences on 20+ ops; PyTorch round-trip of a tensor without copy (pointer equality test); a deliberate shape mismatch fails at compile time with a fix hint.

## M10: Real training run (decision gate)
- Train a reversible MLP/CNN block on MNIST end-to-end in CLL, then a small CIFAR-10 model if M10a passes.
- Baselines (same model, data, seed): PyTorch plain, PyTorch + gradient checkpointing, PyTorch reversible implementation.
- **Exit:** results table in `cll-bench/results/` with commands and raw output: memory slope per block, step time, final accuracy.
- **Gate:** if accuracy is more than 1.0 percentage point below the float RevNet baseline, stop and write an RFC (decision [146]); do not continue to M11 until the owner picks a path.

## M11: Tooling and hardening
- LSP, built-in `test`/property/bench, `cll doc`, `cll audit`, `cll report --erasure`, REPL.
- Mechanised soundness proof started for the primitive table (Lean or Coq), scoped to inverse derivation only.
- **Exit:** editor integration demo script recorded as a conformance-style test; fuzzing corpus >= 1M executions with zero open crashes; audit package prepared.

## M12: v1.0
- External PL + security audit completed, findings fixed or documented.
- **Exit (all required):**
  1. 10+ real `.cll` programs, 2+ automation, all green in CI.
  2. Inverse property: exhaustive on small widths, sampled on wide, all `rev` examples.
  3. Every known-trap negative test is rejected.
  4. RevNet depth sweep: slope near zero; table vs three baselines published.
  5. Performance: five compute kernels within 1.25x of hand-written C (the hand-written C lives only in `cll-bench/baselines/` as measurement reference; owner may adjust the factor).
  6. Spec frozen, conformance 100%, differential green, sanitizers clean, no open P0.

---

## v2 backlog (designed, not scheduled)
LLVM backend and MLIR tensors; GPU kernels; multithreading (actors, structured concurrency) with deterministic reductions; quantum (`qreg`, linear types, OpenQASM 3/QIR); `match ... esac` and recursion in `rev fn`; refinement types via SMT; incremental compilation; WASM and Windows targets; package registry and signing; sparse tensors; mixed-precision automation; browser playground.
