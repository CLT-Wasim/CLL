# AGENTS.md: CLL Master Directive

**Binding on every agent and every human working on any CLL repository.**
Precedence when documents disagree: `AGENTS.md` > `docs/SPEC.md` (once it exists) > `DECISIONS.md` > `ROADMAP.md` > issue/PR text > chat messages.
If two rules conflict and you cannot tell which wins, STOP and write `QUESTION.md` (section 13). Never guess silently.

---

## 1. Mission

CLL (**Conserving Logic Language**, extension `.cll`) is a new compiled systems language for:
1. **AI training and inference** that needs less memory and more safety than Python (reversible blocks recompute instead of store; shapes are checked at compile time).
2. **Safe automation** (agent tools, scripts) where authority is explicit: no function can touch files, network or clock unless it was handed a capability token.
3. **General secure software** with C/C++-class speed and zero undefined behavior.

Order of priority when goals conflict: **correctness/soundness > security > memory > speed > convenience.**

This is a real language, not a demo. Every deliverable is permanent, tested, spec-backed code.

## 2. The formula (central thesis)

> **Information is conserved unless the programmer declares it is being destroyed.**

Physical basis: Landauer's principle (erasing one bit costs at least kT*ln2) and no-cloning. These are *limits*, and we turn the limits into compiler-checked rules.

### 2.1 Exact meaning (read slowly, every word matters)

1. A `rev fn` is a **state transformer**. Its parameters are `inout` state. It is built ONLY from *state-bijective primitives* (full table in SPEC):
   - `a +%= e`, `a -%= e`, `a ^= e` where `e` is any deterministic pure expression **that does not read `a`** (syntactic check including aliasing).
   - `a *%= c` where `c` is an **odd** compile-time constant (invertible mod 2^n).
   - `rotl(a, k)`, `rotr(a, k)`, `swap(a, b)`, `not(a)`.
   - `if c { A } else { B } fi c2` (reversible conditional, section 2.3).
   - bounded `for` over a comptime-constant range (loop variable immutable, bound not mutated in body).
   - calls to other `rev fn` (and `reverse f(...)`, which calls the derived inverse).
   - `local x = e; ...; delocal x = e2` (ancilla discipline, section 2.4).
2. The compiler **derives the inverse syntactically** by reversing statement order and replacing each primitive with its inverse. It never *decides* whether an arbitrary function is a bijection (that is undecidable). Anything not in the table is **rejected**, never assumed safe.
3. **Guarantee, stated honestly:** for every `rev fn f`, `reverse f(f(x)) == x` bit-exactly, **or the program aborts at a checked assertion. Never a silent wrong answer.**
4. `couple`: the RevNet trick is just rule 1 with a rich `e`: `x1 +%= F(x2)`. `F` may be ANY deterministic pure function (conv, ReLU, float math inside), it need not be a bijection, because the inverse is `x1 -%= F(x2)` and `x2` is untouched. Conditions: (a) `F`'s result enters the state through `quantize` into fixed-point/integer, (b) `F` is **bit-reproducible** between forward and recompute (section 2.5).
5. **Float is not a bijection** (`(a+b)-b != a`). Float lives only inside pure `F`, or in ordinary `fn` carrying the `inexact` effect. Never in the state updated by a `rev fn`.
6. **Information loss is an effect.** `erase` is attached (inferred) to operations in the SPEC's lossy-primitive table: `/`, `%`, `>>`, `&`, `|`, `min`, `max`, narrowing `as`, overwriting assignment, explicit `discard(x)`. A `rev fn` may not contain any `erase` operation, and `#[conserving]` asserts the same for any function.

### 2.2 Function kinds (exactly four)
| kind | may do | may not do |
|---|---|---|
| `fn` | anything allowed by its capability params and effects | n/a |
| `pure fn` | deterministic computation, floats allowed (carries `inexact`) | mutation outside locals, capabilities, nondeterminism |
| `rev fn` | the primitives in 2.1 | erase, float state, `Result`, recursion (v1), trap/saturating arithmetic |
| (operator) `reverse f(..)` / `reverse { .. }` | run the derived inverse | applying to non-`rev` code |

There is **no** `approx fn` keyword. The `inexact` effect already says it.

### 2.3 Reversible conditional
`if c { A } else { B } fi c2`: after the branch, `c2` must be true iff the `then` branch ran. Neither branch may modify variables read by `c`, and neither branch may modify variables read by `c2` in a way that changes which branch ran. The compiler proves this statically when decidable and otherwise inserts a runtime assertion that aborts on violation. `match` is **not** allowed inside `rev fn` in v1.

### 2.4 Ancillas (Bennett garbage)
A `rev fn` that needs scratch must return it to its initial value: `local t = e; ...; delocal t = e2` where `e2` must equal the value of `t` at that point (checked). Implicit drop inside `rev fn` is a compile error. This is what prevents silent "garbage output" growth.

### 2.5 Determinism domain
Bit-reproducibility holds when: same compiler build, same target triple, **no fast-math, no FMA contraction** (generated C uses `-ffp-contract=off`), no libm transcendental calls (the std library ships its own `exp/log/tanh/...` written in CLL with fixed algorithms), no unordered reductions, no hash-randomization. GPU kernels are outside the domain in v1.

## 3. Language policy (non-negotiable; this is what went wrong before)

Three layers, never mixed:

| layer | language | where |
|---|---|---|
| Compiler, interpreter, formatter, LSP, CI tooling (`xtask`) | **Rust** | `compiler/`, `xtask/` |
| Compiler output | **C (v1)**, **LLVM IR / MLIR (v2)** | `build/` only, git-ignored, generated |
| Every program, example, test, stdlib module, automation tool, benchmark kernel | **CLL only** | `examples/`, `tests/`, `stdlib/`, `automation/`, `bench/` |

Rules:
- `.py`, `.c`, `.cpp`, `.js`, `.sh` are **forbidden** in `examples/ tests/ stdlib/ automation/`. A CI job fails the build if one appears.
- Allowed exceptions: Python only in `cll-bench/baselines/` (PyTorch comparison models) and `cll-ffi/tests/pytorch_interop/` (interop tests that must call PyTorch). Nothing else.
- If you cannot express something in CLL, **that is a missing language feature**. Do not route around it with another language. Add the feature (through RFC) or write `NEEDS_FEATURE.md` and stop.
- Writing an "automation program" in Python/C "because the compiler is not ready yet" is a **failure**, not progress. Finish the compiler.

Rust crates (suggested, deviations need a line in `DECISIONS.md`): hand-written lexer and Pratt/recursive-descent parser (best diagnostics), `ariadne` or `miette` (diagnostics), `insta` (snapshots), `proptest` (properties), `cargo-fuzz`, `criterion`, `cargo-deny` (supply chain). Add `salsa` only when incremental compilation is scheduled.

## 4. What "real" means (quality bar)

- **Narrow scope per milestone, never narrow quality.** A milestone may implement fewer features; every feature it does implement is complete, specified, tested, documented, and permanent.
- No throwaway scripts, no copy-pasted tutorial interpreters, no "arithmetic calculator" starter code.
- Architecture must match the pipeline in DECISIONS [55]: Lexer -> Parser -> AST -> HIR -> MIR(SSA) -> {interpreter | C emitter}. Reverse transformation lives on MIR.
- Every error message: human text, source span, the rule violated, and a suggested fix.

## 5. Known traps (already found; never reintroduce)

1. "Every pure integer function is a bijection" is **false** (`x // 2`). Bijectivity is **syntactic** (composition of known-bijective primitives), never inferred.
2. `a + b` returning a value is not a bijection; only the state map `(a, b) -> (a + b, b)` is. Model everything as in-place state updates.
3. Trapping `+` and saturating `+|` are partial/non-injective. Forbidden in `rev fn`. Only `+%`, `-%`, `^`, rotate, swap, odd `*%`.
4. Float + "reversible" + "pure" + "bit-exact" cannot all hold. Float only inside `F` / `inexact` code.
5. `e` in `a op= e` must not read `a`. Check through aliasing.
6. A conditional is irreversible unless the branch taken is recoverable from the output (`fi` assertion). Same for ADT `match`, which is why it is excluded from `rev fn` v1.
7. Libm and FMA contraction break cross-platform bit-exactness. Std ships its own math; C is compiled with FMA contraction off.
8. Generated C must itself have **zero UB**: `-fwrapv -fno-strict-aliasing -ffp-contract=off`, all shifts/divisions/indices checked, UBSan+ASan in CI, differential run on **both** gcc and clang.
9. "Memory flat in depth" must be measured against the *fair* baselines: PyTorch reversible implementation **and** gradient checkpointing. Beating plain PyTorch without checkpointing proves nothing.
10. Bit-exact fixed-point coupling may hurt trained accuracy. This is an open empirical risk, handled by the decision gate in ROADMAP M10.

## 6. Repositories (owner grants write access; agents create and push)

| repo | content | language |
|---|---|---|
| `CLL` | `docs/` (SPEC, errors), `grammar.ebnf`, `compiler/`, `xtask/`, `tests/` (conformance), `rfcs/`, `DECISIONS.md`, `ROADMAP.md`, `AGENTS.md` | Rust + CLL |
| `cll-std` | core, alloc, collections (deterministic), strings, math (own libm), time/rng capabilities | CLL |
| `cll-tensor` | shape-typed tensors, fixed-point types, `couple`, reversible block library | CLL |
| `cll-autodiff` | source-transform autodiff over MIR | Rust (pass) + CLL (lib) |
| `cll-ffi` | C ABI, DLPack, PyTorch bridge | Rust + CLL |
| `cll-bench` | benchmarks, baselines, results tables | CLL (+ Python baselines) |
| `cll-pkg` | package manager, lockfile, capability audit | Rust (after v1) |

Every repo has: `AGENTS.md` (pointer to the `CLL` one + local rules), CI, CODEOWNERS, protected `main` (PR-only, CI green required). No agent merges its own spec/RFC change.

## 7. How multiple agents work without drifting

Agents never coordinate by reading each other's code. They coordinate through **four contracts**:
1. `docs/SPEC.md` + `grammar.ebnf`: the single source of truth for behavior.
2. `docs/IR.md`: AST/HIR/MIR definitions. The boundary between frontend, checker, and backends.
3. `tests/` conformance suite: executable meaning of the spec.
4. CI gates (section 8).

**Roles** (one agent may hold several, never both author and reviewer of the same change):
- *Spec author*: writes SPEC sections, primitive/inverse table, effect rules, error catalogue.
- *Frontend*: lexer, parser, formatter, diagnostics.
- *Checker*: name resolution, types, exhaustiveness, ownership/`inout` exclusivity, effects, `rev` checker, inverse derivation.
- *Backend*: interpreter, C emitter.
- *Library*: stdlib, tensor, autodiff library, capabilities.
- *Red team* (mandatory, independent): writes adversarial `.cll` programs trying to break soundness: a `rev fn` that is not invertible, a capability escape, a UB in generated C, a determinism leak. **A soundness bug is P0; everything stops until it is fixed and a regression test is added.**
- *Bench*: measurements per section 10.

**Task protocol** (each task = one branch = one PR):
1. Cite the SPEC section (or write the RFC first if the spec is silent/wrong).
2. Add failing conformance tests (positive AND negative) first.
3. Implement.
4. Run the full gate (section 8), paste real command output in the PR.
5. Update `DECISIONS.md` if any decision changed, with the reason.

Directory ownership is in CODEOWNERS. Cross-ownership change = RFC in `rfcs/NNNN-title.md`.

## 8. CI gates (all required for merge)

1. `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.
2. `cargo xtask policy`: fails on any forbidden-language file (section 3), missing SPEC reference, or untested error code. (`xtask` is Rust.)
3. Conformance runner: every `tests/**/*.cll` passes. Test header directives:
   - `#@ expect: run-ok`, `#@ stdout: <text>`
   - `#@ expect: compile-error E0123 line 7`
   - `#@ inverse: fname` (runs the inverse property check on that `rev fn`)
4. Inverse property: `reverse f(f(x)) == x`; **exhaustive** over all states for small widths (e.g. two `u8` = 65,536 states), seeded random plus edge values for wider.
5. Differential: interpreter vs C backend (gcc and clang, `-O0` and `-O2`) identical stdout and exit code on every run-ok test.
6. Sanitizers on generated C: UBSan + ASan clean.
7. Fuzzing: parser and checker fuzz targets run N minutes per PR, corpus committed. Crashes are P0.
8. Performance gate: no benchmark regresses more than 5% vs stored baseline (from M6 on).
9. Compile-speed gate: stays under budget in DECISIONS [66].

## 9. Verification program

- **Conformance suite**: >= 1 positive and >= 1 negative test per SPEC rule. Test count only counts if each test references a SPEC section.
- **Soundness suite** (red team): every attempted exploit becomes a permanent test.
- **Property tests**: inverse property; formatter idempotence (`fmt(fmt(x)) == fmt(x)`); parse/print round trip; determinism (run twice, compare bytes).
- **Mutation check**: deliberately corrupt an inverse in the table; the suite must go red. If it stays green, the suite is too weak.
- **External audit** by independent PL + security reviewers before v1.0.

## 10. Measurement protocol

- Memory: allocator-tracked bytes plus device-allocated bytes (not RSS). Sweep depth 8/16/32/64 blocks. Report slope per block.
- Baselines: PyTorch plain, PyTorch + gradient checkpointing, PyTorch reversible implementation. Same model, data, seed, precision.
- Speed: same machine, warm-up, >= 30 runs, median and p95, CPU model and flags recorded.
- Every claim in any doc or message includes: command, raw output, commit hash. **No number without a command that produced it.**

## 11. Definition of done

Per milestone: exit criteria in `ROADMAP.md`, all gates green, report filed.
v1.0: all milestones through M12 complete (spec frozen, audit done, conformance 100%).

## 12. Forbidden behavior

- Writing Python/C/JS/shell where CLL is required (section 3).
- Claiming success without running it. Claiming "faster / better / safer" without a measured baseline comparison.
- Silently changing a design decision; silently weakening a test; deleting a failing test; `#[ignore]` without an issue and owner approval.
- Adding a feature not in SPEC, or implementing something the spec forbids "just to make an example work".
- Inventing semantics when the spec is silent. Write an RFC.
- Optimizing before the conformance suite for that feature is green.
- Pasting secrets (tokens) anywhere. Tokens come only from environment secrets.

## 13. When to stop and ask the owner (write `QUESTION.md`, then wait)

- Two spec rules conflict, or a rule makes an example impossible.
- A soundness hole is found and the fix changes the language.
- A tradeoff between speed and correctness, or scope and schedule.
- A decision in `DECISIONS.md` marked **OWNER** needs a value.
- Any request to spend money, publish a package, or make a repo public.

`QUESTION.md` format: context, the options (2-4), your recommendation, what each option costs.

## 14. Report format (end of every task)

1. What was built (files, commit).
2. Which SPEC sections it implements.
3. Commands run and their **real output**.
4. Tests: passed / failed / new.
5. What is uncertain, incomplete, or suspicious.
6. Next step.

Failures reported honestly are worth more than green results nobody can trust.

## 15. Provenance: how these decisions were reached

The owner's goal: a language that beats Python for AI training and automation and rivals C++ for systems work, built by AI agents plus one owner. A 126-item decision form was filled, then reviewed adversarially twice. Review found:
- the original thesis sentence was unsound (`x // 2`), so bijectivity became syntactic;
- trap/saturating arithmetic is partial, so it is banned from `rev fn`;
- float cannot be reversible, so float is confined to `F` and to `inexact` code;
- RevNet needs the `couple` rule, which fell out of the in-place update model;
- conditionals, loops, recursion, ancillas, and `main` needed explicit rules (decisions [119]-[148]);
- a previous agent mixed up the compiler's host language with the language being built; section 3 exists to end that;
- scope was cut so v1 ships: quantum, GPU codegen, package registry, concurrency are v2.

The single original axis is **conservation of information**. Everything else (LLVM, ownership via `inout`, tensor shapes, comptime, effects) is deliberately borrowed from proven designs: Zig, Rust, Swift/Hylo, Koka, Janus, MLIR.
