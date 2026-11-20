# CLL: চলমান compiler slice এবং প্রথম source program

**বর্তমান অবস্থা:** compiler-এর প্রাথমিক runnable slice; CLL v1 নয়।
**নিয়মের মূল উৎস:** `AGENTS.md`, `docs/SPEC.md`, `DECISIONS.md`, `grammar.ebnf`।
**এই ধাপের বাস্তব কাজ:** Rust lexer-এর output থেকে CLL parser/AST, সীমিত reference interpreter এবং `cllc run` CLI।

## চালিয়ে দেখুন

```sh
cargo run -p cllc -- run examples/checksum.cll
```

এই working branch-এ যাচাই করা প্রকৃত output:

```text
874950152
```

## CLL source

```cll
pure fn checksum(word: u32, key: u32) -> u32 {
    (word ^ key) *% 2654435761u32
}

fn main() -> u32 {
    checksum(0x12345678u32, 0x9abcdef0u32)
}
```

এটি cryptographic checksum বা hash নয়; কেবল deterministic 32-bit mixing example। Source-এ প্রকাশ্যভাবে 32-bit `u32` operands, XOR (`^`), এবং modulo-`2^32` wrapping multiplication (`*%`) আছে। `*%` normal checked `*` নয়: SPEC R-EXP-020/021 অনুযায়ী ফল width-এ wrap করে। Interpreter tests checked overflow-কে E0100-style trap করে এবং wrapping addition-এ `255u8 +% 1u8 == 0` যাচাই করে।

## এই slice-এ বাস্তবায়িত

- বিদ্যমান lexer থেকে token ও byte-span-সহ function, parameter, block, let/return/assignment, unary/binary expression এবং named function-call parsing।
- Pratt parser-এ operator precedence; `*`/`*%` `+`/`+%`-এর চেয়ে আগে bind করে।
- `cllc run <file.cll>` দিয়ে function-callসহ integer subset-এর reference execution।
- explicit integer suffix-সহ সীমিত 8/16/32/64-bit integer values; checked basic arithmetic, wrapping `+% -% *%`, bitwise XOR/AND/OR, equality/relational comparison; unsupported construct স্পষ্ট error দেয়।
- syntax, precedence, binding/assignment, function-kind parsing, overflow trap, wrapping result ও end-to-end checksum-এর Rust tests।

## এই program কী প্রমাণ করে—এবং কী প্রমাণ করে না

এটি প্রমাণ করে যে branch-এর CLL source-টি lexer-এ tokenise হয়, parser-এ AST-তে parse হয়, user-defined function call ও integer expressions interpreter-এ execute হয়, এবং নির্দিষ্ট decimal result পাওয়া যায়। এটি কোনো performance claim নয় এবং C/native binary generation-ও নয়।

এই example-এর zero-argument `main() -> u32` **SPEC R-FN-003-এর required `main(caps: Caps) -> ExitCode` contract মেনে চলে না**; interpreter slice-এ entry-point abstraction এখনও করা হয়নি। তাই এটিকে demo harness-এ চলা language slice হিসেবে ধরতে হবে, spec-conforming shipped application হিসেবে নয়।

এখনও অনুপস্থিত: পূর্ণ grammar coverage (generics, struct/enum, arrays, `if`, `match`, `for`, reversible statement forms), name resolution ও static type/effect/capability checker, `rev fn` state/place execution ও inverse, C emitter, conforming `Caps`/`ExitCode` main, I/O/stdlib, debugger, formatter ও LSP। Unsupported syntax/semantics-কে successful বলে ধরা হয় না।

## পরের বাধ্যতামূলক কাজ

`ROADMAP.md` অনুযায়ী আগে M2 conformance suite-কে ≥150 tests-এ নিতে হবে; তারপর M3 parser/AST/formatter-কে পুরো grammar পর্যন্ত বাড়াতে হবে। M4 static semantics, M5 reversible core/reference interpreter, এবং M6 C backend-ийг আলাদা gates ও tests-সহ implement করতে হবে। M1 draft-এর owner-sign-off proposal-গুলো approved বলে ধরা যাবে না।
