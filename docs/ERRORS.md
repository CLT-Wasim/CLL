# Error catalogue (index)

Normative source: `docs/SPEC.md` section 11. Codes are permanent and are never
reused (DECISIONS [141]).

Each code gets its own page `docs/errors/E####.md` **when it gains its first test**,
because `cargo xtask policy` (rule `P-ERRDOC`) fails when a documented code has no
test citing it. Writing the page before the test would make the gate lie, so the
index below is the contract and the pages follow the tests.

## Lexical (E00xx)

| code | meaning | SPEC rule |
|---|---|---|
| E0001 | carriage return in source | R-LEX-002 |
| E0002 | non-ASCII identifier | R-LEX-004 |
| E0003 | leading-zero integer literal | R-LEX-005 |
| E0004 | malformed float literal | R-LEX-006 |
| E0005 | unknown numeric suffix | R-LEX-007 |
| E0006 | literal out of range for its suffix | R-LEX-007 |
| E0007 | unknown string escape | R-LEX-008 |
| E0008 | unterminated string literal | R-LEX-008 |
| E0009 | malformed compound assignment | R-LEX-011 |
| E0010 | test directive in a non-test file | R-LEX-012 |

## Compile-time evaluation (E001x)

| code | meaning | SPEC rule |
|---|---|---|
| E0011 | compile-time evaluation depends on runtime data or exceeds its fuel budget | R-EXP-090 |

## Types and const generics (E002x)

| code | meaning | SPEC rule |
|---|---|---|
| E0020 | non-evaluable const-generic dimension | R-TYP-031 |
| E0021 | tensor shape mismatch | R-TYP-032 |

## Mutability, `inout`, ownership (E003x)

| code | meaning | SPEC rule |
|---|---|---|
| E0030 | assignment to immutable binding | R-MUT-001 |
| E0031 | shadowed binding | R-MUT-002 |
| E0032 | duplicate `inout` place in one call | R-MUT-011 |
| E0033 | `inout` argument aliases another argument | R-MUT-011 |
| E0034 | implicit drop inside `rev fn` | R-MUT-022, R-REV-051 |

## Execution aborts (E01xx)

| code | meaning | SPEC rule |
|---|---|---|
| E0100 | integer overflow in trapping arithmetic | R-EXP-020 |
| E0101 | division or remainder by zero | R-EXP-020 |
| E0102 | shift width out of range | R-EXP-020 |
| E0103 | `quantize` of NaN or infinity | R-EXP-032 |
| E0104 | interpolating a secret | R-EXP-040, R-TYP-011 |
| E0105 | lossy integer `as` | R-EXP-050 |
| E0106 | out-of-range float-to-integer `as` | R-EXP-050 |
| E0110 | non-exhaustive `match` | R-EXP-060 |
| E0111 | unreachable `match` arm | R-EXP-060 |
| E0112 | `while`/`loop` used in v1 | R-EXP-071 |

## Effects and capabilities (E012x, E013x)

| code | meaning | SPEC rule |
|---|---|---|
| E0120 | `#[conserving]` violated by an `erase` operation | R-EFF-012 |
| E0130 | capability widening | R-CAP-003 |
| E0131 | file/network/clock use without a token | R-CAP-004 |
| E0132 | `unsafe` without a justification | R-CAP-030 |
| E0133 | secret used as branch, index, or display | R-CAP-040 |

## Reversibility (E014x, E015x)

| code | meaning | SPEC rule |
|---|---|---|
| E0140 | non-reversible construct inside `rev fn` | R-REV-002, R-REV-070 |
| E0141 | reversible primitive used as an expression | R-REV-022 |
| E0142 | runtime rotation amount | R-REV-023 |
| E0143 | even multiplier in `*%=` | R-REV-024 |
| E0144 | runtime multiplier in `*%=` | R-REV-024 |
| E0145 | `swap` of a place with itself | R-REV-026 |
| E0146 | `e` reads the target of `op=` | R-REV-030 |
| E0147 | `rev fn` call inside an `e` expression | R-REV-032 |
| E0148 | missing or trivially-true `fi` witness | R-REV-040 |
| E0149 | loop inside `rev fn` that is not a bounded `for` | R-REV-041 |
| E0150 | `break`/`continue` inside `rev fn` | R-REV-042 |
| E0151 | statically false `delocal` | R-REV-050 |
| E0152 | `delocal` assertion failed at run time | R-REV-050 |
| E0153 | unbalanced `local`/`delocal` | R-REV-052 |
| E0154 | `couple` with a non-reproducible `F` | R-REV-062 |

## Modules and runtime (E016x)

| code | meaning | SPEC rule |
|---|---|---|
| E0160 | cyclic import | R-MOD-003 |
| E0161 | stale arena handle | R-RUN-030 |
