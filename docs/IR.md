# IR.md — CLL intermediate representations

**Status:** draft v0.1 (ROADMAP M1).
**Normative reference:** `docs/SPEC.md`. This file describes *how* the pipeline
represents what the SPEC says; it never adds meaning of its own.

DECISIONS [55] fixes the pipeline and DECISIONS [56] fixes SSA MIR:

```text
source .cll
   |  lexer            (M3)
   v
tokens
   |  parser           (M3)
   v
AST                 syntax only: spans, nesting, no names resolved
   |  resolve          (M4)
   v
HIR                 names resolved, desugared, generic params bound
   |  lower            (M4)
   v
MIR (SSA)           basic blocks, one definition per value, effects attached
   |                   \
   |  interpreter       \  C emitter        (M5, M6)
   v                     v
 result                 .c -> cc -> ELF
```

**The reverse transformation happens on MIR** (DECISIONS [56]). That is the whole
reason MIR is SSA: reversing a straight-line SSA sequence is mechanical, and the
`e`-does-not-read-`a` rule (SPEC R-REV-030) becomes a use-def question with an exact
answer instead of a source-level guess.

---

## 1. AST

Syntax only. Every node carries a `Span` (byte offsets plus the file id) because
every diagnostic needs one (AGENTS.md section 4).

```rust
pub struct Span { pub file: FileId, pub start: u32, pub end: u32 }

pub struct Ast {
    pub uses: Vec<UseDecl>,
    pub items: Vec<Item>,
}

pub struct Item {
    pub docs: Vec<String>,
    pub visibility: Visibility,
    pub kind: ItemKind,
    pub span: Span,
}

pub enum Visibility { Private, Public }

pub enum ItemKind {
    Fn(FnDecl),
    RevFn(RevFnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Const(ConstDecl),
    TypeAlias(TypeAlias),
}

pub struct FnDecl {
    pub name: Ident,
    pub kind: FnKind,          // Ordinary | Pure
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub body: Block,
    pub annotations: Vec<Annotation>,
    pub span: Span,
}

/// SPEC R-FN-010: no return-type clause; all parameters are inout state.
pub struct RevFnDecl {
    pub name: Ident,
    pub params: Vec<Param>,     // all have ParamMode::Inout (R-REV-001)
    pub body: Block,
    pub span: Span,
}

pub enum GenericParam {
    Type { name: Ident, bounds: Vec<Path> },
    Const { name: Ident, ty: TypeExpr },
}

pub enum ParamMode { Value, Inout }
pub struct Param { pub mode: ParamMode, pub name: Ident, pub ty: TypeExpr, pub span: Span }
pub enum Annotation { Conserving }

pub struct Block { pub statements: Vec<Stmt>, pub tail: Option<Expr>, pub span: Span }

pub enum Stmt {
    Let(LetStmt),
    Assign(AssignStmt),
    Expr(Expr),                 // expression statement; reversible primitives are separate
    RevIf(RevIfStmt),           // `if c {..} else {..} fi c2`  (R-REV-040)
    Return(Option<Expr>),
    Reverse(ReverseStmt),       // `reverse f(..)` / `reverse { .. }`
    Local(LocalStmt),           // R-REV-050
    Delocal(DelocalStmt),       // R-REV-050
    Discard(Expr),
    Unsafe { justification: String, body: Block },
    Break,
    Continue,
    /// P1..P8 of SPEC R-REV-020. Deliberately a *statement*: see R-REV-022.
    RevPrimitive(RevPrimitive),
}

pub enum Expr {
    Literal(Literal), Name(Path), Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>), Cast(Box<Expr>, TypeExpr),
    Call { callee: Box<Expr>, args: Vec<Expr> }, Index(Box<Expr>, Box<Expr>),
    Field(Box<Expr>, Ident), Try(Box<Expr>), Array(Vec<Expr>), Struct(StructExpr),
    If { condition: Box<Expr>, then_block: Block, else_block: Block },
    Match { value: Box<Expr>, arms: Vec<MatchArm> },
    For { binding: Ident, start: Box<Expr>, end: Box<Expr>, body: Block },
    Comptime(Block), Block(Block),
}

pub enum TypeExpr { Path(Path), Apply(Path, Vec<GenericArg>), Fixed(Box<Expr>, Box<Expr>),
    Tensor { element: Box<TypeExpr>, dimensions: Vec<ConstExpr> },
    Array(Box<TypeExpr>, Box<Expr>), Slice(Box<TypeExpr>), Capability(CapabilityType),
    Secret(Box<TypeExpr>), Unit }
pub enum GenericArg { Type(TypeExpr), Const(ConstExpr), UnresolvedName(Path) }

pub enum RevPrimitive {
    AddEq  { place: Place, rhs: Expr },
    SubEq  { place: Place, rhs: Expr },
    XorEq  { place: Place, rhs: Expr },
    MulEq  { place: Place, factor: ConstExpr },   // R-REV-024: odd, comptime
    Swap   { a: Place, b: Place },                // R-REV-026: distinct
    Rotl   { place: Place, k: ConstExpr },        // R-REV-023: comptime
    Rotr   { place: Place, k: ConstExpr },
    Not    { place: Place },
}
```

`comptime` nodes are evaluated and replaced by constants before HIR lowering. `If`, `Match`,
and `For` are expressions with the result rules in SPEC R-EXP-002; `For` always has type
`()`. `RevIf` and reversible primitive nodes remain statements and are never desugared into
ordinary expressions. `inout` actual arguments are represented as place expressions and are
validated by name/alias checking, not by the parser.

**Why `RevPrimitive` is a separate statement node and not sugar for
`Assign(AddEq, ...)`:** because SPEC R-REV-022 says so, and because a distinct node
makes the checker's job a `match` over eight constructors instead of a
re-derivation from an assignment's operator token. Sugar would put the reversible
core back in the same bucket as the irreversible `=`.

---

## 2. HIR

AST with names resolved. Concretely, in HIR:

* every `Ident` that names a binding, a function, a type, or a module is replaced by
  an `ItemId`/`LocalId`/`ModuleId` (a `u32` index into a per-compilation arena);
* `let x = e` becomes `Let { local: LocalId, init: Expr }`;
* `a += b` is desugared to `a = a + b` **only for irreversible code**; the reversible
  primitives keep their own nodes (they are not desugared — there is nothing to
  desugar them to);
* generic parameters are bound to concrete or symbolic const/type arguments.

Effect inference starts here (SPEC R-EFF-001): each `HirFn` carries an
`EffectSet { erase: bool, inexact: bool }` that starts empty and is filled by
walking the body bottom-up, then iterating over call edges until a fixed point.

---

## 3. MIR (SSA)

### 3.1 Shape

```rust
pub struct Mir {
    pub funcs: Vec<MirFn>,
    pub globals: Vec<ConstGlobal>,   // `const` items only (R-MUT-030)
}

pub struct MirFn {
    pub id: FnId,
    pub kind: MirFnKind,             // Ordinary | Pure | Rev
    pub params: Vec<MirParam>,       // `inout` params are *places*, see 3.3
    pub blocks: Vec<BasicBlock>,
    pub effects: EffectSet,
    pub caps: Vec<CapParam>,         // SPEC section 7
}

pub struct BasicBlock {
    pub id: BlockId,
    pub insts: Vec<Inst>,
    pub term: Terminator,
}

pub enum Terminator {
    Goto(BlockId),
    Branch { cond: Value, then_bb: BlockId, else_bb: BlockId },
    /// SPEC R-REV-040: the branch that took the `then` arm leaves `witness`
    /// set. The witness is an SSA value, so the inverse reads it directly.
    RevBranch { cond: Value, witness: Place, then_bb: BlockId, else_bb: BlockId },
    Return(Option<Value>),
    Abort { code: ErrorCode, span: Span },
}

pub enum Inst {
    /// A pure, total, side-effect-free computation.
    Bin { op: BinOp, lhs: Value, rhs: Value, dst: Value },
    Un { op: UnOp, src: Value, dst: Value },
    Call { callee: FnId, args: Vec<Value>, inouts: Vec<Place>, dst: Option<Value> },
    Load { place: Place, dst: Value },
    Store { place: Place, value: Value },
    /// P1..P8 from the primitive table, after checking.
    RevPrim(RevPrim),
    Quantize { src: Value, dst_ty: Type, dst: Value },  // R-EXP-032
    AbortIf { cond: Value, code: ErrorCode, span: Span },
}
```

### 3.2 Why SSA matters for reversibility

Reversing `x1 = x1 + F(x2)` in SSA is: emit `x1_prev = x1_next - F(x2)` where `F(x2)`
is *the same SSA value*, already present in the block. There is no need to search for
"the same expression"; SSA guarantees one definition. If `F(x2)` were recomputed
instead of reused, the check that `F` is bit-reproducible (SPEC R-REV-062) would be
unverifiable — with SSA, the inverse literally reuses the forward value, so
reproducibility is structural rather than a promise.

### 3.3 Places and the `inout` model

An `inout` parameter is not an SSA value; it is a **place** (an addressable location
owned by the caller). SPEC R-MUT-010..012 makes references second-class precisely so
this is expressible:

```rust
pub enum Place {
    Local(LocalId),          // a stack slot inside this function
    Param(ParamId),          // an `inout` parameter: the caller's place
    Field(Box<Place>, u32),  // a struct field of a place
    Index(Box<Place>, Value),
}
```

**The alias rule (SPEC R-REV-030, DECISIONS [154]) is a place-equality check:**
`e` must not read the place being updated. At MIR level, `e`'s value graph is walked;
any `Load` whose place is *equal to* the target place (structural equality on
`Place`, which reaches through `Param` to the caller's place) is a violation
`E0146`. Because places are second-class, two places are equal only if they are the
same place; there is no pointer arithmetic to defeat the check.

### 3.4 The reverse transformation

Given `MirFn { kind: Rev, .. }`, the compiler emits a companion
`MirFn { kind: Rev, name: reverse_of(f), .. }`:

1. Walk the forward function's blocks **in reverse order**, and within each block walk
   its instructions in reverse order.
2. Replace each `RevPrim` with its table inverse (SPEC R-REV-020 column 3):
   `AddEq`↔`SubEq`, `XorEq`↔`XorEq`, `MulEq(c)`→`MulEq(c^-1)`,
   `Rotl`↔`Rotr`, `Swap`→`Swap`, `Not`→`Not`.
3. A `RevBranch` is inverted by entering the arm the witness names and running that
   arm's inverse; the witness is *consumed* by the inverse (it is set back to false).
4. `Local`/`Delocal` swap roles (SPEC R-REV-050).
5. Anything else in a `Rev` function is a compile error `E0140` that should have been
   caught in the checker; the transform asserts this and aborts the compilation
   rather than emitting a wrong inverse.

The transform is **syntactic**. It never proves a bijection, because SPEC R-REV-003
says that is undecidable and out of scope.

---

## 4. Effects on MIR

Each `Inst` declares which effects it carries, exactly as SPEC R-EFF-010 lists:

| instruction | `erase` | `inexact` |
|---|---|---|
| `Bin { Div, Rem, Shr, Shl, And, Or }` | yes | no |
| `Bin { AddSat, SubSat, MulSat }` | yes | no |
| `Bin { Add, Sub, Mul }` | no (aborts on overflow) | no |
| `Bin { AddWrap, SubWrap, MulWrap }` | no | no |
| `Bin { float add/sub/mul/div }` | no | yes |
| `Quantize` | no (aborts on NaN) | yes |
| `Store` to a place that already held a value | yes | no |
| `Call` | union of callee effects, minus absorption | as callee |

Absorption (`inexact` disappearing) happens only at `Quantize` inside a `couple`
(SPEC R-EFF-021, R-REV-071).

---

## 5. Stability

The AST may change freely until M3's grammar freeze. HIR and MIR shapes may change
until M5. After M6 the MIR is the contract between the checker, the interpreter, and
the C emitter (AGENTS.md section 7, contract 2), and changing it needs an RFC.
