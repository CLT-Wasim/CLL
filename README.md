# CLL: Conserving Logic Language

**A compiled programming language for AI training, safe automation, and secure systems software, built on one idea: information is conserved unless you declare that you are destroying it.**

> ## STOP. READ THIS FIRST.
>
> **Every person and every AI agent who works on this repository, in any way, MUST read these three files, in this order, before doing anything else:**
>
> 1. **[`AGENTS.md`](./AGENTS.md)**: the binding rules. What you may and may not do.
> 2. **[`DECISIONS.md`](./DECISIONS.md)**: every design decision and the reason behind it.
> 3. **[`ROADMAP.md`](./ROADMAP.md)**: the milestones and the measurable exit criteria.
>
> Do not write code, specs, tests, or issues until you have read all three.
> AI agents: reading them is mandatory, not optional. Work that ignores them will be rejected.
> If these files conflict with anything else (a chat message, an issue, your own habits), the files win.

---

## Status: pre-v1, design and specification phase

Nothing below is shipped yet. Every capability in this README is a **design target** with a stated way to verify it (see `ROADMAP.md`). We publish a number only when a command produced it.

---

## Why CLL exists

Three problems, one language.

**1. AI training wastes memory.**
Training a deep network normally stores every layer's activations for the backward pass, so memory grows with depth. Reversible networks (RevNet) avoid this by recomputing activations from the outputs. Today that is a fragile trick built on floating point, which is not exactly invertible. CLL makes reversibility a **checked property of the language**: the compiler verifies a function is invertible and derives its inverse.

**2. Automation and AI agents run with too much power.**
A script or an agent tool can usually read any file, open any socket, and call anything. One poisoned dependency or one prompt injection and it does damage. In CLL a function can touch the outside world **only through capability tokens it was explicitly handed**. No token, no access, and the compiler enforces it.

**3. Fast languages are unsafe, safe languages are slow or heavy.**
C and C++ are fast but full of undefined behavior. CLL targets C-class speed with **zero undefined behavior**, no garbage collector, and no hidden allocation.

---

## The core idea

Physics sets a price on forgetting: erasing a bit costs energy, and quantum states cannot be copied. CLL turns that principle into compiler rules.

- A computation that loses nothing is **reversible by construction**, and the compiler writes its inverse for you.
- Anything that loses information is visible in the code as an **`erase` effect**, so you can see and audit where data is destroyed.
- Floating point is not exactly invertible, so it is kept out of reversible state and confined to clearly marked, deterministic code.

The guarantee: for a reversible function `f`, running `f` and then its inverse returns the exact original state, **or the program aborts at a checked assertion. Never a silent wrong answer.**

---

## What CLL is designed to do

| capability | what it means | planned release |
|---|---|---|
| Reversible functions (`rev fn`) | compiler-checked, compiler-derived inverse, bit-exact on integer/fixed-point state | v1 |
| Memory-flat reversible training | activation memory does not grow with network depth | v1 |
| Shape-checked tensors | `Tensor[f32, 784, 128]`: a dimension mismatch is a compile error, not a crash after an hour of training | v1 |
| Source-transform autodiff | gradients computed by the compiler on its own IR | v1 |
| Capability-based security | file, network, clock access only through explicit, attenuable tokens | v1 |
| Zero undefined behavior | every operation is total or aborts deterministically | v1 |
| No GC, no hidden allocation | ownership, `inout` state, arenas; predictable memory | v1 |
| Bit-exact determinism | same program and inputs give the same output bytes | v1 |
| Secret types and constant-time ops | `secret[T]` cannot be logged or branched on | v1 |
| PyTorch interop | zero-copy tensors via DLPack and a C ABI, so existing models are not rewritten | v1 |
| First-class low-precision types | `bf16`, `f16`, `fp8`, `i8`, `i4` | v1 |
| Compile-time execution | `comptime`, guaranteed to halt, replaces macros | v1 |
| Built-in tooling | formatter, language server, test, property test, benchmark, docs | v1 |
| GPU kernels, LLVM/MLIR backend | | v2 |
| Quantum programming (linear qubit types, no-cloning enforced) | | v2 |
| Multithreading (actors, structured concurrency) | | v2 |
| WASM sandbox and playground | | v2 |
| Package manager with capability audit | | v2 |

---

## A taste of the language

These are **design sketches**. Syntax is not frozen until the frontend milestone, but the semantics shown are the planned ones.

**A reversible function and its guaranteed inverse**

```
rev fn mix(inout a: u32, inout b: u32) {
    a +%= b          # wrapping add, inverse is -%=
    b ^= a           # XOR; the right side does not read b, so it is legal
    rotl(a, 7)       # rotation, inverse is rotr
}

mut a: u32 = 123
mut b: u32 = 456
mix(a, b)
reverse mix(a, b)    # a == 123 and b == 456 again, guaranteed
```

**Information loss is rejected inside a reversible function**

```
rev fn bad(inout x: u32) {
    x = x / 2        # compile error: division loses information (5 and 4 both give 2)
}
```

**The RevNet trick: an arbitrary function inside a reversible update**

```
rev fn block(inout x1: Tensor[fx[16, 16], 128],
             inout x2: Tensor[fx[16, 16], 128],
             w: Weights) {
    # F can be any deterministic function, including float math inside.
    # The inverse only needs x1 -%= F(x2), so no activations are stored.
    x1 +%= quantize(relu(matmul(w, x2)))
}
```

**Capabilities: an automation tool that can read one folder and nothing else**

```
fn main(caps: Caps) -> ExitCode {
    let data = caps.file_read("/data")   # attenuated token: read-only, /data only
    summarize(data)                      # cannot open a socket or write a file
}
```

**Shape errors caught at compile time**

```
let a: Tensor[f32, 784, 128] = ...
let b: Tensor[f32, 64, 10]   = ...
let c = matmul(a, b)   # compile error: inner dimensions 128 and 64 differ
```

---

## How it compares (design targets, not measurements)

| | Python | C++ | Rust | CLL (target) |
|---|---|---|---|---|
| Memory safety without GC | no GC-free option | no | yes | yes |
| Zero undefined behavior | mostly | no | outside `unsafe` | outside `unsafe` |
| Tensor shapes checked at compile time | no | partly, via templates | partly, via crates | yes, in the type system |
| Authority restricted by the type system | no | no | by convention and libraries | yes, capability tokens |
| Reversible computation checked by the compiler | no | no | no | yes |
| Ecosystem and libraries | huge | huge | large | tiny at first; PyTorch interop bridges the gap |

The honest weak point is the last row. A new language has no ecosystem, which is why interop with Python and C is part of v1 and not an afterthought.

---

## How it is built

- **Compiler and tooling:** Rust.
- **Backends:** C first (so the C compiler does the heavy optimization), LLVM IR and MLIR in v2.
- **Everything written in CLL:** the standard library, every example, every test, every automation tool.
- **Quality system:** a specification with rule IDs, an executable conformance suite, an inverse-property test (`reverse f(f(x)) == x`, exhaustive on small widths), differential testing between the interpreter and compiled code on both gcc and clang, sanitizers, fuzzing, a red-team role that attacks the compiler, and an external audit before v1.0.

## Ecosystem

| repository | purpose |
|---|---|
| `CLL` | specification, compiler, conformance tests |
| `cll-std` | standard library |
| `cll-tensor` | shape-typed tensors, fixed-point types, reversible blocks |
| `cll-autodiff` | automatic differentiation |
| `cll-ffi` | C ABI, DLPack, PyTorch bridge |
| `cll-bench` | benchmarks and baselines |
| `cll-pkg` | package manager (after v1) |

## What CLL deliberately is not

No inheritance, no exceptions, no null, no garbage collector, no macros, no user-defined operators, no undefined behavior. These are removed on purpose: each one hides state or cost, and hidden state breaks both reversibility and determinism.

## Contributing

Read the three files at the top, then follow `AGENTS.md`. All changes go through pull requests, and `main` is protected.
