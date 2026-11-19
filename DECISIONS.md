# DECISIONS.md: CLL decision register

Every decision from the original form ([1]-[118]), the reviewer's additions ([119]-[126]), and new decisions found since ([127]-[148]).
Tags: **[v1]** ships in v1, **[v2]** designed now, built later, **[no]** rejected, **OWNER** needs the owner's value.
Format: decision. *Why.* Changing any entry requires an RFC and an edit here with the reason.

---

## BLOCKING

**[1] Why CLL exists.** A compiled language for AI training and safe automation where information loss is a declared, type-checked effect, and loss-free computation is reversible by construction. *Why:* it is the one axis on which Python and C++ cannot follow; reversible blocks recompute activations instead of storing them.

**[2] Non-goals** [v1]. No OOP inheritance; no exceptions; no GC; no null; no undefined behavior; no macros; no user-defined operators; no implicit conversions; no global mutable state; no own GPU codegen in v1; no multithreading in v1; no self-hosting in v1. *Why:* each removes hidden state or hidden cost, and hidden state breaks both reversibility and determinism.

**[3] Central thesis.** Information is conserved unless declared destroyed. Bijectivity is syntactic: `rev fn` = composition of state-bijective primitives; inverse derived by the compiler; float confined to `F` and `inexact` code. Full text: AGENTS.md section 2. *Why:* the original sentence was unsound; this version is checkable.

**[4] First real user.** **OWNER confirm:** Wasim (GitHub `CLT-Wasim`). Project 1: train a small reversible network (integer/fixed-point coupling blocks) end-to-end in CLL with flat activation memory. Project 2: a capability-restricted automation tool in CLL. *Why:* a language with no real user designs for imaginary needs.

**[5] Definition of done v1.** Measurable list in ROADMAP M12: 10+ real `.cll` programs (2+ automation), inverse property exhaustive on small widths and sampled on wide, negative tests rejected, RevNet depth-sweep memory slope ~0, differential gcc/clang green, external audit complete. *Why:* "better" must be a number.

**[6] Adoption strategy.** Own one niche: memory-flat reversible training plus capability-safe agent automation, while keeping PyTorch usable through FFI/DLPack so nobody must rewrite everything. *Why:* nobody leaves Python for "slightly better".

**[7] Name.** CLL = Conserving Logic Language. File `.cll`; compiler `cllc`; driver `cll` (build, run, test, fmt, doc, bench). *Why:* the name states the thesis.

**[8] Language only or ecosystem?** Language plus a minimal, deliberate ecosystem: std, tensor lib, FFI/PyTorch bridge, formatter, LSP, test/bench tooling, package manager after v1. *Why:* Swift for TensorFlow had a good idea and no ecosystem. The ecosystem is most of the work.

---

## LAYER 1: SURFACE

**[9] Everything is an expression** [v1]: yes. *Why:* simpler grammar and semantics.
**[10] Blocks** [v1]: braces plus one mandatory canonical formatter (`cll fmt`). *Why:* trivially parseable, no style debates, no indentation ambiguity.
**[11] Annotations** [v1]: `x: i32`. *Why:* familiar to Python and Rust users.
**[12] Generics** [v1]: `[T]`, `fn id[T](x: T) -> T`. *Why:* `<T>` is ambiguous with comparison.
**[13] Operators** [v1]: C-like set, `^` is XOR, **bitwise binds tighter than comparison** (fixes C's `a & b == c` trap). Explicit-overflow forms: `+% -% *%` (wrap), `+| -| *|` (saturate), plain `+ - *` trap. Rotate via `rotl/rotr`. *Why:* familiarity plus removal of known C footguns.
**[14] User-defined operators** [no]. *Why:* unreadable code; complicates inverse analysis.
**[15] Pattern matching** [v1]: full, exhaustive, with guards (a catch-all must follow guarded arms). Not allowed inside `rev fn` in v1 (needs tag preservation, [v2] `match ... esac`). *Why:* a missing case is silent information drop.
**[16] Error handling** [v1]: `Result[T, E]` plus `?`; panics abort ([47]). Effect handlers [v2]. `rev fn` cannot return `Result`. *Why:* effect handlers are research-grade; `Result` is enough to ship.
**[17] Modules** [v1]: file = module, `use` explicit, `pub` explicit, no cyclic imports, no globals except `const`. *Why:* no hidden global state.
**[18] Shadowing** [v1]: forbidden. *Why:* a rebound name hides a value needed for inversion and is a classic bug.
**[19] Metaprogramming** [v1]: `comptime` only. No macros, no templates. *Why:* one mechanism.
**[20] Strings** [v1]: UTF-8 only; `str` immutable view, `String` owned; interpolation `"{x}"`; interpolating `secret[T]` is a compile error. *Why:* byte-string semantics are a bug source.
**[21] Numeric literals** [v1]: `1_000`, `0xFF`, `0b1010`, `0o17`, suffixes `u8 i64 f32 f64`; leading-zero octal forbidden. *Why:* cheap in the lexer, no C-style traps.
**[22] Comments** [v1]: `#` line, `///` doc. No block comments. *Why:* nested block comments are a lexer bug farm.

## LAYER 2: STATIC SEMANTICS

**[23] Typing** [v1]: 100% static with inference. *Why:* effects and `rev` checking are static.
**[24] Inference** [v1]: bidirectional, local; signatures of `pub` items fully annotated. *Why:* global inference produces unreadable errors.
**[25] Generics** [v1]: parametric with trait bounds, monomorphized, no dynamic dispatch. Capability bounds rejected (no defined semantics). *Why:* fewer concepts; zero-cost.
**[26] Subtyping/variance** [no]. *Why:* huge complexity cost for little gain.
**[27] Sum types** [v1]: yes, exhaustive.
**[28] Null** [no]: `Option[T]` only.
**[29] Dependent/refinement types**: v1 has **const generics with simple arithmetic** (literals, `+`, `*`), enough for tensor shapes without an SMT solver. Refinements via SMT [v2]. *Why:* SMT is a project in itself.
**[30] Effect system** [v1]: `erase`, `inexact`, inferred, shown in docs for `pub` items; `#[conserving]` asserts no `erase`. **Revised:** `io` is *not* an effect; authority is expressed by capability parameters ([69]). Reserved for v2: `nondet`, `measure`, `alloc`, `diverge`. *Why:* two mechanisms for the same thing is a design bug. *Risk:* if `erase` is inferred on nearly every ordinary function it carries no signal. M4 exit measures the fraction of std functions that are conserving; if too low, RFC to narrow `erase` to explicit `discard` plus non-injective primitives only.
**[31] Linearity** [v1]: affine for resource types (tokens, handles, owned buffers); linear (must-consume) reserved for v2 `qreg`. *Why:* quantum is v2; affine covers v1.
**[32] Aliasing** [v1]: **mutable value semantics**: `inout` parameters, second-class references (cannot be stored in structs or returned), exclusivity checked statically. No borrow checker, no lifetimes. Graphs/trees use arena handles. *Why:* proven (Hylo/Swift), gives memory safety without lifetime annotations, and `inout` is exactly the state-transformer model `rev fn` needs.
**[33] Mutability** [v1]: immutable default, `mut` explicit.
**[34] Purity as type** [v1]: four function kinds: `fn`, `pure fn`, `rev fn`, and the `reverse` operator. No `approx fn` (the `inexact` effect covers it).
**[35] Reversibility mechanism** [v1]: `rev fn` + `reverse f(...)` + `reverse { ... }` + compiler check and derivation on MIR. Exact primitive table in SPEC.
**[36] Comptime** [v1]: total, fuel-limited (default 10^7 steps), no I/O, guaranteed to halt.
**[37] Integer overflow** [v1]: `+` traps (abort, in all builds), `+%` wraps, `+|` saturates. Division by zero and over-width shift abort. Only total wrapping ops allowed in `rev fn`. *Why:* Zig's design, plus totality for inversion.
**[38] Floats** [v1]: strict IEEE-754 binary32/64, round-to-nearest-even, no FMA contraction unless explicit `fma()`, fast-math opt-in via a region attribute, **forbidden** in `F` of any `couple` and in `rev fn`. NaN canonicalized on bit-cast. *Why:* fast-math reorders operations and destroys exactness.
**[39] Undefined behavior** [v1]: **zero** outside `unsafe`. Every operation is total or aborts deterministically. Generated C compiled with `-fwrapv -fno-strict-aliasing -ffp-contract=off`, UBSan/ASan in CI. *Why:* UB is the biggest source of security bugs and non-determinism.
**[40] Determinism** [v1]: bit-exact within the domain in AGENTS.md 2.5. Integer/fixed bit-exact across targets. Float basic ops (+ - * / sqrt) bit-exact across IEEE targets; transcendentals come from std's own CLL implementations. GPU outside domain in v1.

## LAYER 3: RUNTIME AND MEMORY

**[41] Memory management** [v1]: ownership + arenas, no GC. Heap types `Box[T]`, `Vec[T]`, `Arena`; arena allocations are generational handles with checked access (stale handle aborts). *Why:* GC pauses and hidden costs.
**[42] Allocation** [v1]: value types on the stack by default; heap only through explicit `Box/Vec/Arena`; no escape analysis of our own (the C compiler does it). *Why:* no hidden allocation.
**[43] Object model** [v1]: value default; references second-class ([32]).
**[44] Layout** [v1]: default = declaration order, natural alignment (identical to C layout for scalar structs); `packed` and explicit alignment as attributes; ADT layout unspecified but deterministic.
**[45] Zero-cost abstraction** [v1]: strict; any abstraction with runtime cost must be spelled in the source.
**[46] Drop** [v1]: deterministic, reverse declaration order at scope end. Scope-end drop of ordinary values is not an `erase` event. In `rev fn`, implicit drop is a compile error; use `delocal` or return through `inout`.
**[47] Panics** [v1]: abort, with message and source span. No unwinding.
**[48] Static init** [no]: no code before `main`; `const` are comptime values.
**[49] Threads** [v1]: single-threaded. *Why:* concurrency doubles every verification problem.
**[50] Concurrency primitive** [v2]: actors + messages.
**[51] Async** [v2]: structured concurrency.
**[52] Atomics/ordering** [v2]: `unsafe` only.
**[53] FFI** [v1]: C ABI, `unsafe` plus a mandatory `cap Ffi` token. This is also the Python/PyTorch bridge.
**[54] ABI stability** [no in v1].

## LAYER 4: COMPILER PIPELINE

**[55] Pipeline** [v1]: Lexer -> Parser -> AST -> HIR (names resolved, desugared) -> MIR (SSA). Reverse transform and effect checks on MIR.
**[56] IR** [v1]: SSA MIR. *Why:* clean reversal and dataflow.
**[57] Optimization passes** [v1]: 0-5 own passes (constant folding, dead code, inlining of tiny `rev` primitives, bounds-check elimination basics). *Why:* C compiler/LLVM does the heavy lifting.
**[58] Backend roadmap**: v0 reference **interpreter in Rust** (the executable spec); v1 **C emitter**; v2 **LLVM IR**, with **MLIR** for tensor ops; WASM via the LLVM path. Own machine codegen: never. Reverse-native hardware backend: not planned (no commercial reversible hardware exists; CLL's benefit on current CPUs is memory via recompute, not energy). *Why:* a small team cannot beat LLVM.
**[59] JIT/AOT** [v1]: AOT; interpreter exists for the spec and the REPL. JIT [later].
**[60] Incremental compilation** [v2]: needs a stable IR first.
**[61] Separate compilation** [v1]: per module, deterministic outputs.
**[62] Build system** [v1]: built into `cll`, minimal manifest `cll.toml`.
**[63] Package manager + registry** [v2, `cll-pkg`]: lockfile plus capability audit ("this dependency requests network"). *Why:* the language needs users first, but the design is fixed now.
**[64] Cross-compilation** [v1]: via the C toolchain targets.
**[65] Binary size** [no target in v1].
**[66] Compile speed** [v1]: budget under 1 s per 10k lines, measured in CI.
**[67] Self-hosting** [no]: not planned; revisit only after v2 with evidence of benefit. *Why:* huge cost, little value for the niche.

## LAYER 5: SECURITY

**[68] Memory safety** [v1]: 100% outside `unsafe`.
**[69] Capabilities** [v1]: tokens `FileRead(path-scope)`, `FileWrite(scope)`, `Net(host-scope)`, `Clock`, `Rng`, `Proc`, `Ffi`. Affine (cannot be copied), can be **attenuated** (`fs.restrict("/data")`), passed explicitly from `main`. No ambient authority. *Why:* a library cannot exfiltrate data or hijack an agent beyond the tokens it was given.
**[70] Sandbox target** [v2]: WASM. Honest limit: language-level capabilities stop buggy or malicious *CLL* code; they do not stop `unsafe`/FFI code, so OS or WASM isolation remains the outer wall.
**[71] Constant-time intrinsics** [v1]: `ct_eq`, `ct_select`, `ct_lt` built in, with a lint that rejects secret-dependent branches and indices.
**[72] Secret types** [v1]: `secret[T]` cannot be logged, interpolated, compared with `==`, or used as branch/index.
**[73] Supply chain** [v2]: signed packages, reproducible builds. Compiler itself: `cargo-deny`, pinned versions.
**[74] Unsafe audit** [v1]: every `unsafe` block carries a mandatory `#[justify("...")]`; `cll audit` lists all of them.
**[75] Side channels**: no UB helps but is not enough; `secret` types plus constant-time intrinsics are both required.
**[76] Crypto agility** [library level].

## LAYER 6: QUANTUM (all [v2]; v1 must not preclude them)

**[77]** Gate-based model. **[78]** `qreg` is a *linear* type, no-cloning enforced by the type checker. **[79]** Clifford+T plus rotations. **[80]** `measure` is an effect that is a refinement of `erase` (irreversible). **[81]** Gates are bijections and reuse the exact `rev fn` rules. **[82]** Logical qubits by default; physical access via `unsafe`-style attribute. **[83]** Simulator first, then OpenQASM 3, then QIR. **[84]** Hybrid classical control allowed with uncompute discipline (same as [2.4]). **[85]** Depolarising noise model in the simulator.
*Why deferred:* quantum is where the thesis fits best long-term, but it is nine decisions of scope; the owner priority is AI, then automation. v1 reserves `qreg`, `measure`, linear types as keywords.

## LAYER 7: AI AND NUMERICS

**[86] Tensor type** [v1]: shape in the type, `Tensor[f32, 784, 128]`; const-generic arithmetic from [29]; mismatch is a compile error.
**[87] Autodiff** [v1]: source transform on MIR (not tracing). For `rev fn`, the backward pass uses the inverse to recompute inputs instead of stored activations.
**[88] Device model** [v2]: GPU backend as a plugin through MLIR. v1 is CPU.
**[89] Precision** [v1]: per-tensor dtype; `bf16`, `f16`, `fp8`, `i8`, `i4` are first-class types (storage + convert; arithmetic where hardware defined).
**[90] PyTorch/JAX interop** [v1]: FFI + DLPack, zero-copy. Do not reinvent kernels.
**[91] Mixed precision** [v2].
**[92] Sparse tensors** [v2].

## LAYER 8: TOOLING

**[93] Errors** [v1]: human text, source span, rule violated, suggested fix; stable codes `E0001+`, each with `docs/errors/E####.md`.
**[94] LSP** [v1, after grammar freezes at M3].
**[95] Debugger** [v1]: DWARF via the C backend (`#line` directives map to `.cll`).
**[96] Profiler** [later].
**[97] REPL** [v1, on the interpreter].
**[98] Test framework** [v1]: built-in `test` blocks plus the conformance runner.
**[99] Property/fuzz testing** [v1]: built in (required for inverse property).
**[100] Benchmark harness** [v1]: built in.
**[101] Formatter** [v1]: one canonical form, idempotent.
**[102] Docs** [v1]: from `///`, `cll doc`.
**[103] Browser playground** [v2, WASM].

## LAYER 9: GOVERNANCE

**[104] License**: Apache-2.0.
**[105] Specification**: prose `SPEC.md` plus executable conformance suite; every rule has an ID; tests cite IDs.
**[106] Change process**: RFC files in `rfcs/`. Agents propose, owner (or a designated reviewer) merges; agents never merge their own spec change.
**[107] Backward compatibility**: before v1.0 anything may change via RFC; after v1.0 never break (editions for changes).
**[108] Versioning**: semver.
**[109] Deprecation**: two releases of warnings.
**[110] Governance**: owner is sole decider until v1.0.
**[111] Community channels**: after v1.0.

## LAYER 10: VERIFICATION

**[112] Conformance suite**: executable test per spec rule, positive and negative.
**[113] Differential testing**: interpreter vs compiled output, gcc and clang, `-O0/-O2`.
**[114] Reversibility proof strategy**: property test `reverse f(f(x)) == x`; exhaustive on small widths, seeded random plus edge values on wide; bit-exact for integer/fixed; no float tolerance applies to `rev` (floats are not in rev state).
**[115] Mechanised proof (Lean/Coq)** [v2]: soundness of the inverse derivation for the primitive table. Small enough to be doable.
**[116] Compiler fuzzing**: required from M3.
**[117] External security/soundness audit**: before v1.0.
**[118] Performance regression gate**: in CI from M6.

---

## REVIEWER ADDITIONS

**[119] Recursion in `rev fn`** [v1: no]. v2: structural recursion with a strictly decreasing measure carried in state. *Why:* the inverse of a non-terminating recursion is meaningless.
**[120] Loops in `rev fn`** [v1]: `for i in a..b` with comptime-constant bounds, immutable index, body must not mutate the bounds. `from ... until` (Janus-style) [v2]. `while` forbidden.
**[121] `reverse { }` scope**: executes the inverse of the enclosed statements; can mutate exactly the variables the statements mutate; no new variable escapes the block.
**[122] `inexact` propagation**: propagates to all callers like any effect. It is absorbed only at `quantize` inside a `couple` expression when the determinism conditions (AGENTS 2.5) are verified; otherwise it propagates.
**[123] Fixed-point types** [v1]: `fx[I, F]` and `ufx[I, F]` (e.g. `fx[16, 16]`), stored in `I+F` bits, `+%`/`-%` form a ring (bijective). Multiplication/division are lossy (`erase`) and appear only inside `F` after `quantize`, never as rev state updates.
**[124] Errors in `rev fn`**: none returned. A violated assertion aborts; the inverse of abort is abort. *Why:* an error path is not reversible.
**[125] `main`** [v1]: `fn main(caps: Caps) -> ExitCode`, not `rev`, receives the root capability bundle and hands out attenuated tokens.
**[126] "Memory flat in depth" metric**: allocator-tracked bytes + device bytes, depth 8/16/32/64, slope per block, against three baselines (AGENTS section 10). Pass: slope within noise of zero (target < 1% of one block's activation size).

## NEW DECISIONS

**[127] Update model** [v1]: all `rev` state changes are in-place `inout` updates (`a op= e`). *Why:* value-returning `a+b` is not a bijection.
**[128] `e` must not read the updated variable** [v1]: checked syntactically, through aliasing and through callee `inout` arguments.
**[129] Reversible conditional** [v1]: `if c {A} else {B} fi c2`, rules in AGENTS 2.3; runtime assertion where not statically provable.
**[130] Bounded for** [v1]: see [120].
**[131] Ancilla discipline** [v1]: `local/delocal` with checked equality; implicit drop forbidden in `rev fn`.
**[132] Quantize** [v1]: `quantize[fx[I,F]](f)` rounds to nearest-even, saturates on out-of-range, **aborts on NaN/inf**. *Why:* total and deterministic; divergence in training is surfaced, not hidden.
**[133] Determinism domain** [v1]: AGENTS 2.5, tested by running twice and by cross-compiler diff.
**[134] Own math library** [v1]: `exp, log, tanh, sigmoid, erf, ...` in CLL, fixed algorithms, with documented ULP error bounds. *Why:* libm differs between platforms.
**[135] Deterministic collections** [v1]: `Map`/`Set` iterate in a defined order (insertion or sorted); no hash seed randomization.
**[136] Time and randomness** [v1]: `Clock` and `Rng` are capabilities; counter-based PRNG (Philox-style) in std so streams are reproducible and recomputable.
**[137] Platform targets** [v1]: 64-bit only: Linux x86-64, Linux aarch64, macOS arm64. Windows and WASM [v2]. `usize` = 64-bit.
**[138] Integer types** [v1]: explicit widths `i8..i128`, `u8..u128`, `usize`; there is no bare `int`. Casts are explicit; narrowing cast is `erase` and traps on loss unless written `as_wrapping`.
**[139] Standard library layers** [v1]: `core` (no alloc), `alloc`, `std`; all in `cll-std`, written in CLL.
**[140] Dependency policy**: compiler deps minimal, pinned, `cargo-deny`; runtime/std: zero third-party dependencies.
**[141] Diagnostics stability**: error codes never reused; each has a doc page and at least one test.
**[142] Spec IDs**: every SPEC rule has an ID (`R-REV-014`); tests, errors and PRs cite IDs.
**[143] Soundness policy**: soundness bugs are P0, fixed with a permanent regression test; the red-team agent role is mandatory.
**[144] Inverse naming**: for `rev fn f`, the compiler provides `f` and `reverse f`; no user-visible name `f_inv`.
**[145] Reports**: `cll report --erasure` lists erase operations per function; `cll audit` lists `unsafe`, FFI and capability use per package.
**[146] Training-accuracy risk gate**: see ROADMAP M10. If bit-exact fixed-point coupling costs more than 1.0 percentage point accuracy vs a float RevNet baseline, an RFC chooses between wider fixed-point, higher-precision accumulators, or float coupling with a documented drift bound under `inexact`. **OWNER** may change the threshold.
**[147] Source of truth for numbers**: results live in `cll-bench/results/*.md`, each with command, raw output, hardware, commit.
**[148] Release cadence**: milestone-based, not date-based.

**[149] `fi` witness must start false** [v1, SPEC R-REV-040]. *Why:* AGENTS.md 2.3 requires `c2` to be true after the `then` branch, but if `c2` was already true before the statement the inverse cannot tell which branch ran. The witness must be a bit that was `false` and is now `true`. Missing/trivial witness = `E0148`. **OWNER sign-off pending (M1).**

**[150] Rotation amount is a comptime constant** [v1, SPEC R-REV-023]. *Why:* a runtime `k` would have to be unchanged between forward and reverse for the syntactic inverse to be correct, which the derivation cannot check. Runtime `k` = `E0142`. **OWNER sign-off pending (M1).**

**[151] Reversible primitives are statements, not expressions** [v1, SPEC R-REV-021/022]. *Why:* the state-update model requires every table entry to mutate a place in one step. `not(a)`, `swap(a,b)`, `rotl(a,k)`, `rotr(a,k)` produce no value; using one in value position = `E0141`. **OWNER sign-off pending (M1).**

**[152] `local`/`delocal` exact semantics** [v1, SPEC R-REV-050]. `delocal x = e2` asserts the place equals `e2`, then removes it from the state. The inverse is `local x = e2; reverse S; delocal x = e`. *Why:* without removal the ancilla stays in the state as garbage and memory grows with call depth, defeating the purpose of reversibility. Unbalanced = `E0153`. **OWNER sign-off pending (M1).**

**[153] `rev fn` returns `()`** [v1, SPEC R-FN-010]. Values leave through `inout` state only. *Why:* a returned value would have to be reconstructible by the inverse; state is invertible, a return value is not, and the inverse's signature must equal the forward signature for `reverse f(...)` to type-check. **OWNER sign-off pending (M1).**

**[154] `e`-does-not-read-target is a place-based, transitive check** [v1, SPEC R-REV-030..032]. The check is on places, not names: two `inout` parameters bound to the same place by the caller are the same place; a call whose `inout` argument is the target violates the rule; a `pure fn` cannot alias because it may not take `inout` at all. Rejection on doubt = `E0146`. *Why:* `a +%= a` is `a := 2a`, not injective mod 2^n. False acceptance is P0; false rejection is a checker bug. **OWNER sign-off pending (M1).**

---

### M1 surface-syntax implementation defaults (review proposal)

The following defaults close omissions found while checking `grammar.ebnf` against the
v1 decisions. They are written into the v0.2 SPEC draft so implementation can proceed;
they are **proposals for owner review**, not a claim that M1 sign-off has already happened.

**[155] Generic parameters and arguments** [v1, proposed]: type parameters use `[T: Trait + Other]`; constant parameters use `[const N: usize]`. Type applications accept arguments in declaration order, and the declaration determines whether each argument is a type or a constant expression. V1 bounds name compiler/standard-library traits only; user-defined trait/implementation declarations and dynamic dispatch remain out of scope. *Why:* support the documented `Option[T]`, `Result[T,E]`, and shape-typed `Tensor` without adding runtime dispatch.

**[156] Compile-time expression syntax** [v1, proposed]: `comptime { ... }` evaluates a deterministic, capability-free block before code generation. It uses the existing 10^7-step fuel limit and reports `E0011` on runtime-only input or fuel exhaustion. *Why:* make DECISIONS [19], [36] expressible without macros or unbounded evaluation.

**[157] Saturating compound assignment** [v1, proposed]: `+|=`, `-|=`, and `*|=` do not exist. Saturation is expressed with `+|`, `-|`, `*|` inside an explicit ordinary assignment such as `x = x +| y`; that assignment carries `erase`. *Why:* avoid a second, misleading update-token family in addition to the lossy arithmetic operator.

**[158] Justified unsafe block syntax** [v1, proposed]: `#[justify("reason")] unsafe { ... }`; the non-empty attribute immediately precedes the block. *Why:* make DECISIONS [74] and SPEC R-CAP-030 parseable with one unambiguous placement.

**[159] Documentation-comment priority** [v1, proposed]: `///` is a documentation-comment token recognized before ordinary `#` line comments and attaches to the next module item. No `#///` spelling is introduced. *Why:* preserve DECISIONS [22] and ensure the ordinary-comment token cannot swallow documentation.

## RISKS (owner's R1-R4, resolved)

**[R1]** Physics gives limits (Landauer, no-cloning, Bekenstein); the language turns limits into compiler rules. Accepted as design principle.
**[R2]** Speed now comes from doing less work, not clock speed: no UB, no GC, no hidden allocation. Accepted.
**[R3]** Biggest risk is a small team. Mitigation: milestone scope gates, v2 tags above, red-team role, CI as the enforcer.
**[R4]** Originality on one axis (conservation of information); everything else borrowed deliberately.

## GITHUB / OPERATIONS

**[T1]** Token only as environment secret `GITHUB_PAT`; never pasted into chat, files, or logs.
**[T2]** Fine-grained token: Contents (read/write), Pull requests (read/write), Administration (to create repos), scoped to the CLL repositories.
**[T3]** **OWNER confirm:** owner `CLT-Wasim`; repos `CLL`, `cll-std`, `cll-tensor`, `cll-autodiff`, `cll-ffi`, `cll-bench`, `cll-pkg`.
**[T4]** Public, Apache-2.0. `main` protected on every repo.
