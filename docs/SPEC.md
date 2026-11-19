# SPEC.md — CLL Language Specification v0.1

**Status:** draft for owner sign-off (ROADMAP M1).
**Precedence:** `AGENTS.md` > this file > `DECISIONS.md` > `ROADMAP.md` > issues > chat.

Every normative statement carries an ID of the form `R-<AREA>-<NNN>`. A rule is normative
if and only if it has an ID. Prose without an ID is explanation, and explanation never
overrides a rule. The conformance suite (`tests/`) cites these IDs, and
`cargo xtask policy` fails when a test does not.

Areas: `LEX` lexical, `TYP` types, `MUT` mutability and `inout`, `EFF` effects,
`CAP` capabilities, `FN` function kinds, `REV` reversibility, `MOD` modules,
`EXP` expressions and statements, `RUN` execution and aborts.

---

## 0. How to read this document

Two kinds of statement appear:

* **MUST / MUST NOT** — normative, enforced by the compiler. Always carries an ID.
* *Why* — the reason. Not enforced, but changing a rule without changing its *Why* is
  an RFC.

Where this document and `AGENTS.md` section 2 disagree, `AGENTS.md` wins and this
document is wrong; file an issue against the spec, do not "interpret".

---

## 1. Lexical structure

**R-LEX-001** Source files are UTF-8 without a byte-order mark, extension `.cll`.

**R-LEX-002** Line endings are `\n`. A lone `\r` is an error `E0001`, not whitespace.

**R-LEX-003** An ordinary comment starts with `#` and runs to end of line. A documentation
comment starts with `///` and is attached to the next module item; the lexer recognizes it
before ordinary comments. Block comments do not exist. *Why:* DECISIONS [22]; nested block
comments are a lexer bug farm.

**R-LEX-004** Identifiers match `[A-Za-z_][A-Za-z0-9_]*` and are ASCII only. A non-ASCII
identifier is `E0002`. *Why:* non-ASCII identifiers invite homoglyph attacks in code that
is meant to be auditable.

**R-LEX-005** Integer literals: decimal `123`, `1_000`; hex `0xFF`; binary `0b1010`;
octal `0o17`. A leading zero followed by a digit (`0123`) is `E0003`.
*Why:* DECISIONS [21]; C's octal trap.

**R-LEX-006** Float literals always contain `.` or an exponent: `1.0`, `1e9`, `2.5e-3`.
`1.` and `.5` are `E0004`; write `1.0` and `0.5`. *Why:* one spelling per value.

**R-LEX-007** Integer suffixes are exactly `u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 usize`;
float suffixes are exactly `f16 bf16 f32 f64`. A suffix from the wrong set, or any unknown
suffix immediately following a numeric literal, is `E0005`. A valid integer suffix that
cannot hold the literal is `E0006`, detected at lexing, not at run time. The lexer consumes
the complete adjacent identifier-like suffix before deciding whether it is valid.

**R-LEX-008** String literals are double-quoted UTF-8 with escapes `\n \t \r \\ \" \{ \}`
and `\u{...}`. Any other escape is `E0007`. An unterminated literal is `E0008`.

**R-LEX-009** `{expr}` inside a string literal is interpolation (R-EXP-040).
`\{` is a literal brace.

**R-LEX-010** Reserved words cannot be identifiers: `as as_wrapping break const comptime
continue delocal discard else enum erase esac false fi fn for from if impl in inout
inexact let local loop match mod mut not pub pure rev reverse return rotl rotr secret
struct swap true type unsafe use while with`.
*Why:* `esac`/`fi` close `match`/`if`, so a stray `}` cannot be mistaken for the end of a
reversible block. `impl`, `with`, `while`, `loop`, `esac` are reserved for v2 so that
adding them later is not a breaking change.

**R-LEX-011** Punctuation: `+ - * / % ^ & | ~ ! = == != < <= > >= << >> && || += -= *= /=
%= ^= &= |= <<= >>= +%= -%= *%= +| -| *| -> => :: : ; , . .. ( ) [ ] { } #@`.
`+%=`, `-%=`, `*%=` are single wrapping compound-assignment tokens (R-EXP-021). `+|`,
`-|`, `*|` are saturating expression operators (R-EXP-022); saturating compound assignments
do not exist in v1—write an ordinary assignment with a saturating expression instead.
Maximal munch applies; `a +%= b` is one token sequence, while `a +% = b` is `E0009`.

**R-LEX-012** A `#@` line at the top of a file is a test directive and is only meaningful
in `tests/`. In a non-test file it is `E0010`. *Why:* the directive header must never leak
into shipped programs.

---

## 2. Types

**R-TYP-001** There is no bare `int`. Every integer has an explicit width from R-LEX-007.
`usize` is 64-bit on all v1 targets (DECISIONS [137]).

**R-TYP-002** Integer types are `iN`/`uN` for N in {8,16,32,64,128}, plus `usize`.
Two's complement, no padding bits. `uN` range is `[0, 2^N - 1]`; `iN` is
`[-2^(N-1), 2^(N-1) - 1]`.

**R-TYP-003** Float types are IEEE-754: `f16` (binary16), `bf16` (bfloat16), `f32`, `f64`.
`fp8` and `i4` are storage types: they convert to and from `f16`/`i8` but carry no
arithmetic of their own in v1 (DECISIONS [89]).

**R-TYP-004** Fixed-point types are `fx[I, F]` (signed) and `ufx[I, F]` (unsigned),
`I, F` const-generic integers with `1 <= I`, `1 <= F`, `I + F <= 128`. The stored width is
`I + F` bits. `fx[I,F]` holds `v / 2^F` for integer `v` in the signed range of width `I+F`.
Arithmetic on fixed-point is defined in R-EXP-030..034.

**R-TYP-005** `bool` has exactly two values, `true` and `false`. It is one byte in memory
and its only valid bit patterns are `0` and `1`.

**R-TYP-006** `()` is the unit type, with one value `()`. A function with no `->` clause
returns `()`.

**R-TYP-007** `T` and `U` in `*T` — there are no raw pointers in safe code. References are
second-class and appear only as `inout` parameters (R-MUT-010). There is no `&T`, no `*T`,
no lifetime syntax. *Why:* DECISIONS [32].

**R-TYP-008** Composite types: `struct`, `enum` (sum type, R-TYP-020), `[T; N]` fixed
array, `[T]` slice view, `str` (immutable UTF-8 view), `String` (owned UTF-8 buffer),
`Tensor[D, S1, ..., Sk]` (R-TYP-030).

**R-TYP-009** `Option[T]` is `enum { Some(T), None }` and is the only way to express
absence. There is no `null` (DECISIONS [28]).

**R-TYP-010** `Result[T, E]` is `enum { Ok(T), Err(E) }` and is the only error channel
other than abort (R-RUN-020).

**R-TYP-011** `secret[T]` wraps `T` and forbids: logging, string interpolation, `==`/`!=`
comparison with a non-secret, use as a branch condition, use as an array index, and use as
a loop bound (R-CAP-040). *Why:* DECISIONS [72].

**R-TYP-012** Generic parameters and arguments use square brackets, never angle brackets
(DECISIONS [12]). A type parameter may have one or more compiler/standard-library trait
bounds; a constant parameter is declared with `const`. The exact forms and argument-kind
checking are specified in R-TYP-033..034.

**R-TYP-013** No implicit conversions exist between any two types, including
`i32` → `i64`, `u8` → `usize`, and integer → float. Every conversion is written `as`.
*Why:* DECISIONS [2] removes implicit conversions; they hide value changes.

**R-TYP-014** `as` between integer widths traps on loss (R-EXP-050). `as_wrapping` never
traps and truncates or sign-extends (R-EXP-051). A narrowing `as` carries the `erase`
effect (R-EFF-010); `as_wrapping` carries `erase` as well, because it also loses bits.

**R-TYP-020** An `enum` declares a tag and payloads. Construction is `E::V(x)`; access is
`match` (R-EXP-060). An enum with no payload variants is compared with `==`.

**R-TYP-030** `Tensor[D, S1, ..., Sk]`: `D` is an element type, `S1..Sk` are const-generic
dimensions of type `usize`. The shape is part of the type. `Tensor[f32, 784, 128]` and
`Tensor[f32, 64, 10]` are different types and no operation accepts both where one shape is
required (R-TYP-031).

**R-TYP-031** Const-generic dimension arithmetic supports literals, `+`, and `*` only
(DECISIONS [29]). An expression the compiler cannot evaluate is `E0020`.

**R-TYP-032** A shape mismatch in `matmul`, `+`, `-`, `*`, reshape, or any other
shape-consuming operation is a **compile error** `E0021`, never a runtime check and never
a silently broadcast shape. Broadcasting does not exist in v1.

**R-TYP-033** A type parameter is written `T` or `T: Trait + OtherTrait` in a generic
parameter list. Bounds name compiler- or standard-library-provided traits. User-defined
trait and implementation declarations, trait objects, and dynamic dispatch are not v1
features; `impl` remains reserved. Bounds are checked statically after monomorphization.

**R-TYP-034** A constant generic parameter is written `const N: usize` (or another
unsigned integer type). A generic application supplies type arguments or constant
expressions in the declared parameter order. The declaration determines each argument's
kind; a kind mismatch is a compile error. `Tensor` takes one element type followed by one
or more constant dimensions (R-TYP-030). `Option[T]`, `Result[T, E]`, `Box[T]`, and `Vec[T]`
take type arguments only.

---

## 3. Mutability, `inout`, and ownership

**R-MUT-001** Bindings are immutable unless declared `mut`. `let x: i32 = 1` then `x = 2`
is `E0030`.

**R-MUT-002** Shadowing is forbidden. Two `let` bindings with the same name in nested
scopes are `E0031`. *Why:* DECISIONS [18]; a rebound name hides a value the inverse needs.

**R-MUT-010** A parameter is passed by value unless declared `inout`. `inout` gives
exclusive mutable access to the caller's place for the duration of the call; the callee's
writes are visible to the caller after the call.

**R-MUT-011** Exclusivity: while a call with an `inout` argument is active, the caller may
not read or write that place. Passing the same place twice in one call (`f(a, a)`) is
`E0032`. Passing `a` as `inout` while `a` appears in another argument expression is
`E0033`.

**R-MUT-012** An `inout` reference cannot be stored in a struct, captured by a closure,
returned, or placed in a container. It is second-class (DECISIONS [32]).

**R-MUT-020** Values are owned. `Box[T]`, `Vec[T]`, `String` are owned heap types; `Arena`
hands out generational handles whose access after the arena is dropped aborts (R-RUN-030).

**R-MUT-021** Drop order is reverse declaration order at scope exit. Scope-end drop of an
ordinary value is **not** an `erase` event (DECISIONS [46]).

**R-MUT-022** Inside a `rev fn`, any implicit drop is `E0034`; the value must be returned
through `inout` state or uncomputed with `delocal` (R-REV-050).

**R-MUT-030** There is no global mutable state. A module may declare `const` items only
(DECISIONS [17]).

---

## 4. Function kinds

**R-FN-001** There are exactly four function kinds:

| kind | written as | may | may not |
|---|---|---|---|
| ordinary | `fn` | anything its capability parameters and effects permit | nothing extra |
| pure | `pure fn` | deterministic computation, floats allowed (carries `inexact`) | mutation of non-locals, capabilities, nondeterminism |
| reversible | `rev fn` | the primitives of R-REV-020 | `erase`, float state, `Result`, recursion, trapping arithmetic |
| inverse operator | `reverse f(..)`, `reverse { .. }` | run the derived inverse | apply to a non-`rev` function or a non-reversible block |

There is no `approx fn` (DECISIONS [34]).

**R-FN-002** A `pure fn` carries the `inexact` effect if and only if it performs float
arithmetic, calls a libm-style function, or calls anything carrying `inexact` (R-EFF-020).

**R-FN-003** `main` has the signature `fn main(caps: Caps) -> ExitCode`. It is never
`rev`, never `pure`, and it is the only function that receives the root capability bundle
(DECISIONS [125]).

**R-FN-004** Every `pub` item's signature is fully annotated: no inferred parameter or
return types cross a module boundary (DECISIONS [24]).

**R-FN-010** A `rev fn` **returns `()`**. Its observable effect is the final state of its
`inout` parameters and nothing else.
*Why (spec gap G5):* a returned value would have to be reconstructible by the inverse. A
`rev fn` that needs to hand a value back does it through an `inout` parameter, which is
state and therefore invertible. This also makes the inverse's signature identical to the
forward one, which is what `reverse f(...)` requires.

**R-FN-011** A `rev fn` may not be a method, may not be generic in a way that changes the
state shape, and may not be called with a capability token argument. A capability token is
affine and one-way, so passing it through a reversible function would destroy it.

---

## 5. Expressions and statements

**R-EXP-001** Every block is an expression. The value of a block is the value of its last
expression, or `()` if the last item is a statement ending in `;`.

**R-EXP-002** An ordinary `if` expression requires `else`; each branch is a block and both
blocks must have the same type. A `match` is an expression and every arm must have the same
type. A `for` loop is a unit-valued expression. Assignment, declaration, `return`, `break`,
`continue`, reversible primitives, and reversible `if` are statements, not value
expressions.

**R-EXP-010** Expression operators and precedence, tightest first:
`! ~ -` (unary), `as as_wrapping`, `* / % *% *|`, `+ - +% -% +| -|`, `<< >>`, `&`, `^`, `|`,
`== != < <= > >=`, `&&`, `||`. Assignment operators are statement syntax, not expression
operators; saturating compound assignments do not exist in v1. Wrapping compound forms are
`+%= -%= *%=`.

**R-EXP-011** Bitwise operators bind **tighter than** comparison. `a & b == c` parses as
`(a & b) == c`. *Why:* DECISIONS [13]; fixes C's most-reported precedence trap.

**R-EXP-012** `&&` and `||` short-circuit and require `bool` operands. Bitwise `&`/`|` on
`bool` do not short-circuit.

**R-EXP-020** Arithmetic on integers:

| form | on overflow | total? | `erase`? | allowed in `rev fn` |
|---|---|---|---|---|
| `+ - *` | abort (`E0100`) | no | no | **no** |
| `+% -% *%` | wrap | yes | no | **yes** (with R-REV-020 conditions) |
| `+\| -\| *\|` | saturate | yes | yes (two inputs, one output) | **no** |
| `/ %` | divide by zero aborts (`E0101`) | no | yes | **no** |
| `>> <<` | shift ≥ width aborts (`E0102`) | no | `>>` yes, `<<` yes | **no** |

**R-EXP-021** The wrapping forms `+% -% *%` and `+%= -%= *%=` are total on every pair of
values of the same integer type.

**R-EXP-022** The saturating forms `+| -| *|` are total but lossy and are therefore
forbidden in a `rev fn` (AGENTS.md section 5 trap 3).

**R-EXP-030** Fixed-point `+%` and `-%` are component-wise wrapping addition of the
underlying `I+F`-bit integer, so they form a ring and are bijective in their first operand
(R-REV-021).

**R-EXP-031** Fixed-point `*%` is **not** a bijection; it truncates low bits and carries
`erase`. It may appear only inside the `F` expression of a `couple` (R-REV-070).

**R-EXP-032** `quantize[fx[I,F]](x: f32) -> fx[I,F]` rounds to nearest, ties to even,
saturates on out-of-range input, and **aborts** on NaN or infinity (`E0103`).
*Why:* DECISIONS [132]; divergence in training is surfaced, not hidden.

**R-EXP-033** `quantize` carries `inexact` but not `erase`. It is the only place where the
`inexact` effect is absorbed (R-EFF-021).

**R-EXP-040** String interpolation `"{expr}"` requires `expr` to implement `Display`.
Interpolating a `secret[T]` is `E0104` (R-TYP-011).

**R-EXP-050** `as` between integer types traps when the value is not representable in the
target type (`E0105`). `as` between float and integer traps when the float is NaN,
infinite, or outside the target range (`E0106`).

**R-EXP-051** `as_wrapping` truncates toward zero for narrowing and sign-extends for
widening a signed value. It never traps. It carries `erase`.

**R-EXP-060** `match` is exhaustive: a missing variant is `E0110`; an unreachable arm is
`E0111`. A guard arm may not be followed by a catch-all that makes later arms unreachable.
`match` is forbidden inside a `rev fn` in v1 (R-REV-010).

**R-EXP-070** `for i in a..b { .. }` iterates `a, a+1, ..., b-1`. `a` and `b` are
evaluated once. A half-open range is the only form; `a..=b` does not exist.

**R-EXP-071** `while` and `loop` are reserved and unimplemented in v1 (`E0112`).
`break` and `continue` are permitted inside `for` in an ordinary `fn` and forbidden in a
`rev fn`.

**R-EXP-080** `discard(x)` is the only way to drop a value early. It carries `erase`
(R-EFF-011).

**R-EXP-090** `comptime { ... }` is a compile-time expression whose value is evaluated
before code generation. It may use only compile-time values and deterministic pure
computation; it may not use capabilities, `inout`, `unsafe`, I/O, or nondeterminism. Each
evaluation has the fuel limit in DECISIONS [36]; exceeding the limit or depending on a
runtime-only value is compile error `E0011`. A successful evaluation is guaranteed to
terminate within that fuel budget.

---

## 6. Effects

**R-EFF-001** Two effects exist in v1: `erase` and `inexact`. Both are inferred and both
propagate to callers. `io` is not an effect; authority is expressed by capabilities
(R-CAP-010). *Why:* DECISIONS [30].

**R-EFF-010** `erase` is attached to exactly these operations: `/`, `%`, `>>`, `<<`, `&`,
`|`, saturating `+| -| *|`, narrowing `as`, `as_wrapping`, `discard(x)`, overwriting
assignment to a place that already holds a value, `min`, `max`, and `quantize`'s saturation
branch.

**R-EFF-011** An ordinary function that contains any `erase` operation carries `erase`.

**R-EFF-012** `#[conserving]` on a function asserts that it carries no `erase`. A
`#[conserving]` function containing an `erase` operation is `E0120`.

**R-EFF-013** A `rev fn` carries no `erase` by construction; R-REV-030 is the enforcement.

**R-EFF-020** `inexact` is attached to float arithmetic, float comparisons other than
`==`/`!=` on non-NaN values, calls to libm-style functions, and `quantize`.

**R-EFF-021** `inexact` is absorbed only at `quantize` inside a `couple` expression
(R-REV-071), and only when the determinism conditions of R-RUN-040 hold. Elsewhere it
propagates to the caller.

**R-EFF-030** Effects are printed in `cll doc` output for every `pub` item, so a caller
can see what a function loses and what it approximates without reading its body.

---

## 7. Capabilities

**R-CAP-001** The capability types are `FileRead(scope)`, `FileWrite(scope)`,
`Net(scope)`, `Clock`, `Rng`, `Proc`, `Ffi`. Each is affine: it can be moved but not
copied, and it is consumed by the operation that uses it.

**R-CAP-002** There is no ambient authority. A function may touch the filesystem, network,
clock, process table, or foreign code **only** through a token it received as a parameter
(DECISIONS [69]).

**R-CAP-003** A token can be **attenuated** but never widened: `read.restrict("/data")`
yields a token for a subset of the original scope. `restrict` to a wider scope is
`E0130`. *Why:* attenuation is the whole point; widening would be a privilege escalation
expressible in the source.

**R-CAP-004** The compiler proves statically that a function which never receives a
`FileRead` token contains no call that reads a file. Calling `read(...)` without a token in
scope is `E0131`.

**R-CAP-005** `main` receives `Caps` and distributes attenuated tokens. `Caps` is the only
value with ambient authority and it cannot be passed to a function that also receives
untrusted input. *Why:* DECISIONS [125]; the root bundle must be handed out deliberately.

**R-CAP-010** `cap` appears in a signature as an ordinary parameter type:
`fn hash(dir: FileRead("/data")) -> Hash`.

**R-CAP-020** A `rev fn` may not take or return any capability type (R-FN-011).

**R-CAP-030** An unsafe block is written `#[justify("reason")] unsafe { ... }`; the
non-empty justification must immediately precede that block. A missing or empty reason is
`E0132`. `cll audit` lists every `unsafe` block, `Ffi` use, and capability use per package.

**R-CAP-040** `secret[T]`: see R-TYP-011. Branching on a secret, indexing with a secret,
printing a secret, or comparing a secret with `==` is `E0133`.

**R-CAP-041** Constant-time intrinsics `ct_eq`, `ct_select`, `ct_lt` are available for
every integer width and for `secret[T]`; they are the only way to branch on secret data.

---

## 8. Reversibility

This chapter is the core of the language. It is deliberately the most formal.

### 8.1 The model

**R-REV-001** A `rev fn` is a **state transformer**. Its parameters are `inout` state. It
has no return value (R-FN-010). The state is the tuple of all `inout` parameters in
declaration order.

**R-REV-002** A `rev fn` is built **only** from the primitives of R-REV-020, in the forms
of R-REV-021..R-REV-026. Anything else is `E0140`.

**R-REV-003** The compiler derives the inverse **syntactically**: it reverses statement
order and replaces each primitive with its stated inverse. It never decides whether an
arbitrary function is a bijection, because that is undecidable.
*Why:* AGENTS.md section 5 trap 1; `x // 2` is pure, integer, and not injective.

**R-REV-004** Guarantee, stated exactly: for every `rev fn f`, executing `f` and then
`reverse f` on the same state restores that state **bit for bit**, or the program aborts at
a checked assertion. A silent wrong answer is a P0 soundness bug (DECISIONS [143]).

**R-REV-005** The inverse of a `rev fn` is not user-nameable. `reverse f(...)` runs it
(DECISIONS [144]).

### 8.2 The primitive table

**R-REV-020** The complete, closed table. No other construct is reversible.

| # | forward | inverse | precondition |
|---|---|---|---|
| P1 | `a +%= e` | `a -%= e` | `e` does not read `a` (R-REV-030) |
| P2 | `a -%= e` | `a +%= e` | `e` does not read `a` |
| P3 | `a ^= e` | `a ^= e` | `e` does not read `a` |
| P4 | `a *%= c` | `a *%= c_inv` | `c` odd, `c_inv = c^-1 mod 2^n`, both comptime |
| P5 | `rotl(a, k)` | `rotr(a, k)` | `k` comptime constant, `0 < k < n` |
| P6 | `rotr(a, k)` | `rotl(a, k)` | `k` comptime constant, `0 < k < n` |
| P7 | `swap(a, b)` | `swap(a, b)` | `a` and `b` are distinct places |
| P8 | `not(a)` | `not(a)` | — |
| P9 | `if c { A } else { B } fi c2` | reverse `B` then reverse `A`, each guarded by `c2` | R-REV-040 |
| P10 | `for i in a..b { body }` | reverse `body` with `i` from `b-1` down to `a` | R-REV-041 |
| P11 | `f(args)` where `f` is `rev fn` | `reverse f(args)` | `args` are the same places |
| P12 | `local x = e; ...; delocal x = e2` | `local x = e2; ...; delocal x = e` | R-REV-050 |

**R-REV-021** `not(a)` is the bitwise complement on the stored width. It is written as a
statement, `not(a);`, not as an expression.
*Why (spec gap G3):* the table lists state updates; `a = ~a` would be an overwriting
assignment, which is not a state update and carries `erase` (R-EFF-010). Statement form
keeps the model uniform: every entry of the table mutates a place in one step.

**R-REV-022** `swap(a, b)`, `rotl(a, k)`, `rotr(a, k)`, and `not(a)` are **statements**.
They are not expressions and produce no value. Using one where a value is expected is
`E0141`. *Why (spec gap G3):* README's `rotl(a, 7)` is a statement; the grammar must say
so once, not per example.

**R-REV-023** `k` in `rotl`/`rotr` is a **comptime constant**. A runtime rotation amount
is `E0142`. *Why (spec gap G2):* if `k` could be runtime state, the inverse would need `k`
to be unchanged between forward and reverse, which the syntactic derivation cannot
guarantee. A comptime `k` makes the inverse unconditional.

**R-REV-024** `a *%= c` requires `c` to be an **odd comptime constant** whose width equals
`a`'s width. Even `c` is `E0143` (not injective mod 2^n); runtime `c` is `E0144`.

**R-REV-025** The operand of `+%=`, `-%=`, `^=` may be any `pure fn` call or expression
that does not read the target place, including through aliasing (R-REV-030). Float values
may appear inside `e` but never in the state itself.

**R-REV-026** `swap(a, b)` requires `a` and `b` to be distinct places. `swap(a, a)` is
`E0145`. *Why:* `swap(a,a)` is the identity, so its "inverse" is not well-defined as a
state change; rejecting it keeps the table honest.

### 8.3 The `e` does not read `a` rule

**R-REV-030** In `a op= e`, `e` **must not read** `a`. This is checked syntactically and
**through aliasing**: if `e` calls a function whose `inout` argument is `a`, or contains a
place expression that resolves to `a`, the rule is violated (`E0146`).
*Why (spec gap G6):* `a +%= a` is `a := 2a`, which is not injective mod 2^n, so its inverse
does not exist. Aliasing is the only way this bug hides, so the check must be transitive
through calls. The check is on **places**, not names: two `inout` parameters bound to the
same place by the caller are the same place.

**R-REV-031** The check is conservative: if the compiler cannot prove that `e` does not
read `a`, it rejects with `E0146` and names the expression that caused it. A false
rejection is a bug to fix in the checker; a false acceptance is a soundness bug (P0).

**R-REV-032** A `pure fn` called from `e` is checked at its own definition to not take
`inout` parameters at all, so it cannot alias `a` by construction. A `rev fn` called from
`e` is `E0147` (calling reversible code inside an expression is not a state update).

### 8.4 Conditionals and loops

**R-REV-040** `if c { A } else { B } fi c2` is reversible if and only if all of:

1. `c2` is **false before** the statement and **true after** if and only if `A` ran.
   Equivalently: `c2` is a value that is set by exactly one branch.
2. Neither branch modifies any place read by `c`.
3. Neither branch modifies any place read by `c2` other than setting it as required by (1).

*Why (spec gap G1):* `AGENTS.md` section 2.3 says "`c2` must be true iff the `then` branch
ran". That is necessary but not sufficient. If `c2` was **already true** before the
statement, then after the statement `c2 == true` does not tell the inverse which branch
ran. The inverse needs a witness that distinguishes the two cases, and the witness must be
a bit that was `false` and is now `true`. Condition (1) states exactly that. A missing or
trivially-true `c2` is `E0148`.

**R-REV-041** `for i in a..b { body }` is reversible if and only if: `a` and `b` are
comptime constants; `i` is immutable; `body` does not modify `a` or `b`; `body` is itself
reversible. `while` and `loop` are `E0149`. *Why:* DECISIONS [120]; a loop whose bound
changes has no fixed inverse length.

**R-REV-042** `break` and `continue` are `E0150` inside a `rev fn`: an early exit makes the
number of executed iterations depend on state, so the inverse cannot know where to start.

### 8.5 Ancillas

**R-REV-050** A `rev fn` that needs scratch space writes
`local x = e; ...; delocal x = e2`. `local` introduces a fresh place initialised to `e`;
`delocal` asserts that the place equals `e2` and **removes it from the state**.

*Why (spec gap G4):* "must equal the value of `t` at that point" leaves three questions
open. This rule answers them explicitly:

* **What `delocal` asserts.** The place's current value equals `e2`, evaluated at that
  point. A mismatch aborts (`E0151` at compile time when decidable, otherwise a checked
  runtime assertion that aborts with `E0152`).
* **What the inverse does.** `reverse` of `local x = e; S; delocal x = e2` is
  `local x = e2; reverse S; delocal x = e`. The place is reintroduced with the value it had
  at the end, uncomputed, then removed again. The state outside the ancilla is unchanged.
* **Why removal matters.** Without removal, each call would leave a garbage register
  behind, and memory would grow with call depth, which defeats the entire point of
  reversibility (AGENTS.md section 2.4).

**R-REV-051** An implicit drop inside a `rev fn` is `E0034` (R-MUT-022). Every value
created must leave through `inout` state or through a matched `delocal`.

**R-REV-052** `local`/`delocal` must be balanced within the same function body. A `local`
without a matching `delocal` is `E0153`.

### 8.6 The `couple` rule (RevNet)

**R-REV-060** The RevNet update is the ordinary P1 rule with a rich `e`:
`x1 +%= F(x2)`. `F` may be **any** deterministic pure function, including float math,
matrix products, and non-linearities. It need not be a bijection, because the inverse is
`x1 -%= F(x2)` and `x2` is untouched.

**R-REV-061** `F`'s result must enter the state through `quantize` into a fixed-point or
integer type. A float result may not be assigned into rev state.

**R-REV-062** `F` must be **bit-reproducible**: the same input bytes produce the same
output bytes on recompute (R-RUN-040). A `couple` whose `F` is not bit-reproducible is
`E0154`.

**R-REV-063** `F` may read `x2` and any other place that is not modified by the enclosing
statement, including through the alias rules of R-REV-030.

**R-REV-064** The `couple` form is not new syntax: it is `x1 +%= quantize[fx[I,F]](F(x2))`,
which the checker recognises by structure and reports in `cll report --erasure` as a
coupling site. *Why:* one mechanism, not two; the specification must not grow a second
syntax for the same idea.

### 8.7 What is rejected, and why

**R-REV-070** A `rev fn` containing any of the following is `E0140` with a specific
sub-code: `/`, `%`, `>>`, `<<`, `&`, `|`, `+| -| *|`, narrowing `as`, `as_wrapping`,
`discard`, an overwriting assignment `a = ...`, a `match`, a `while`, a `loop`, a `break`,
a `continue`, a recursive call, a `Result` return, a float assignment into state, or a call
to a non-`rev` function.

**R-REV-071** `inexact` is absorbed only at `quantize` inside a `couple` (R-EFF-021); a
`rev fn` may not carry `inexact` in its signature.

**R-REV-080** Every rejected construct has at least one negative test in `tests/`
(AGENTS.md section 9). A rejection with no test is an incomplete milestone.

---

## 9. Modules

**R-MOD-001** One file is one module. The module name is the file stem.

**R-MOD-002** `use` is explicit; there are no glob imports. `pub` marks exported items.

**R-MOD-003** Cyclic imports are `E0160`.

**R-MOD-004** A module may contain `const` items and no mutable globals (R-MUT-030).

---

## 10. Execution, aborts, and determinism

**R-RUN-001** Every operation is total or aborts deterministically. There is no undefined
behaviour outside `unsafe` (DECISIONS [39]).

**R-RUN-010** Aborts print the error code, a human message, and a source span, then
terminate with exit code 1. There is no unwinding (DECISIONS [47]).

**R-RUN-020** `Result[T,E]` with `?` is the error channel. A `rev fn` may not return
`Result` (DECISIONS [124]); the inverse of an abort is an abort.

**R-RUN-030** A stale arena handle aborts with `E0161`.

**R-RUN-040** **Determinism domain.** Bit-exact reproduction holds when all of: the same
compiler build; the same target triple; no fast-math; no FMA contraction (generated C uses
`-ffp-contract=off`); no libm transcendental calls (the standard library ships its own
`exp`, `log`, `tanh`, `sigmoid`, `erf` written in CLL with documented ULP bounds); no
unordered reductions; no hash-seed randomisation.

**R-RUN-041** Integer and fixed-point arithmetic is bit-exact on every v1 target. Float
`+ - * / sqrt` is bit-exact on every IEEE-754 target. Transcendentals are bit-exact **only**
against the standard library's own implementation, not against a platform libm.

**R-RUN-042** GPU kernels are outside the determinism domain in v1.

---

## 11. Error codes

Codes are permanent and never reused (DECISIONS [141]). Each has a page in
`docs/errors/E####.md` and at least one test that cites it. The skeleton in this version:

| code | meaning |
|---|---|
| E0001 | carriage return in source |
| E0002 | non-ASCII identifier |
| E0003 | leading-zero integer literal |
| E0004 | malformed float literal |
| E0005 | unknown numeric suffix |
| E0006 | literal out of range for its suffix |
| E0007 | unknown string escape |
| E0008 | unterminated string literal |
| E0009 | malformed compound assignment |
| E0010 | test directive in a non-test file |
| E0011 | compile-time evaluation depends on runtime data or exceeds its fuel budget |
| E0020 | non-evaluable const-generic dimension |
| E0021 | tensor shape mismatch |
| E0030 | assignment to immutable binding |
| E0031 | shadowed binding |
| E0032 | duplicate `inout` place in one call |
| E0033 | `inout` argument aliases another argument |
| E0034 | implicit drop inside `rev fn` |
| E0100 | integer overflow in trapping arithmetic |
| E0101 | division or remainder by zero |
| E0102 | shift width out of range |
| E0103 | `quantize` of NaN or infinity |
| E0104 | interpolating a secret |
| E0105 | lossy integer `as` |
| E0106 | out-of-range float-to-integer `as` |
| E0110 | non-exhaustive `match` |
| E0111 | unreachable `match` arm |
| E0112 | `while`/`loop` used in v1 |
| E0120 | `#[conserving]` violated by an `erase` operation |
| E0130 | capability widening |
| E0131 | file/network/clock use without a token |
| E0132 | `unsafe` without a justification |
| E0133 | secret used as branch, index, or display |
| E0140 | non-reversible construct inside `rev fn` |
| E0141 | reversible primitive used as an expression |
| E0142 | runtime rotation amount |
| E0143 | even multiplier in `*%=` |
| E0144 | runtime multiplier in `*%=` |
| E0145 | `swap` of a place with itself |
| E0146 | `e` reads the target of `op=` |
| E0147 | `rev fn` call inside an `e` expression |
| E0148 | missing or trivially-true `fi` witness |
| E0149 | loop inside `rev fn` that is not a bounded `for` |
| E0150 | `break`/`continue` inside `rev fn` |
| E0151 | statically false `delocal` |
| E0152 | `delocal` assertion failed at run time |
| E0153 | unbalanced `local`/`delocal` |
| E0154 | `couple` with a non-reproducible `F` |
| E0160 | cyclic import |
| E0161 | stale arena handle |

---

## 12. Change log

**v0.1** — initial draft. Introduces rule IDs, the closed primitive table, and six
resolutions listed as decisions [149]..[154] in `DECISIONS.md`.

**v0.2-draft** — expands generic/const-generic rules, defines `comptime` and unsafe-block
syntax, clarifies lexical and assignment tokenization, and aligns expression semantics with
the surface grammar. This is a review proposal; ROADMAP M1 owner sign-off remains required.
