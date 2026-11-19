# CLL সহজে শেখা: প্রথম পাঠ

> **এই নথির অবস্থা:** এটি CLL v0.1-এর draft SPEC ও বর্তমান project নথির ভিত্তিতে তৈরি শিক্ষানবিশ-গাইড। CLL-এর parser/type-checker/backend এখনো তৈরি হয়নি; তাই নিচের উদাহরণগুলো ভাষার নকশা বোঝায়, বর্তমান compiler দিয়ে চালানো যায়—এমন দাবি নয়। SPEC এখনো owner sign-off-এর অপেক্ষায়। কোনো নিয়মে এই গাইড ও `AGENTS.md`, `docs/SPEC.md`, `DECISIONS.md` বা `ROADMAP.md`-এর অমিল হলে ওই মূল নথিগুলোই কর্তৃত্বপূর্ণ।

## ১. CLL কী?

CLL-এর পূর্ণ নাম **Conserving Logic Language**। এটি AI training, নিরাপদ automation এবং systems software-এর জন্য পরিকল্পিত একটি compiled language। এর কেন্দ্রীয় ধারণা:

> তথ্য না হারালে computation উল্টে আগের অবস্থায় ফিরতে পারবে; তথ্য হারালে সেটি code-এর effect হিসেবে ধরা পড়বে।

এতে AI training-এর reversible অংশে intermediate value জমিয়ে রাখার বদলে output থেকে আগের state পুনর্গণনা করার সুযোগ তৈরি হতে পারে। এটি memory কমানোর নকশাগত লক্ষ্য—সব program স্বয়ংক্রিয়ভাবে দ্রুত হবে, এমন নিশ্চয়তা নয়। বাস্তব speed ও memory benchmark দিয়ে প্রমাণ করতে হবে।

CLL-এর আরও দুটি লক্ষ্য হলো capability token ছাড়া file/network/clock access না দেওয়া এবং নিরাপদ code-এ undefined behavior এড়ানো।

## ২. একটি CLL program দেখতে কেমন?

CLL source file-এর extension `.cll`; file UTF-8 text। নিচে একটি ছোট function:

```cll
pure fn add(a: u32, b: u32) -> u32 {
    a + b
}
```

লাইন ধরে পড়লে:

- `pure fn` বলে এটি একটি deterministic, বাইরে থেকে state পরিবর্তন না-করা function।
- `add` function-এর নাম।
- `a: u32` ও `b: u32` হলো নামসহ parameter; `u32` ৩২-bit unsigned integer।
- `-> u32` বলে ফলের type `u32`।
- `{ ... }` function body। শেষ expression `a + b`-ই ফল।

এটি CLL-এর প্রস্তাবিত syntax-এর উদাহরণ; parser এখনো না থাকায় compile/run করা যাবে না। `u32`-এর সর্বোচ্চ মান ছাড়িয়ে সাধারণ `+` করলে CLL-এর specification অনুযায়ী deterministic abort হবে।

## ৩. নাম, type ও value

CLL-এ type স্পষ্ট করে লেখা যায়:

```cll
let count: u32 = 3;
let ratio: f32 = 0.5;
let ready: bool = true;
let title: str = "CLL";
```

`let`-এ তৈরি binding সাধারণত immutable—অর্থাৎ পরে সেটির value বদলানো যায় না। বদলাতে হলে `mut` লিখতে হয়:

```cll
let mut count: u32 = 3;
count = count + 1;
```

CLL-এ `int` নামে অস্পষ্ট integer type নেই। `u8`, `i32`, `u64`-এর মতো type বেছে নিয়ে width নির্দিষ্ট করা হয়। Type-এর মধ্যে অনুমতি ছাড়া conversion হয় না; conversion লাগলে তা স্পষ্টভাবে লিখতে হয়।

সংখ্যার উদাহরণ:

```cll
let small: u8 = 255;
let size: usize = 64;
let hex_mask: u32 = 0xFF;
let one_million: u64 = 1_000_000;
```

শুরুতে `0123`-এর মতো leading-zero integer লিখবেন না; সেটি error। Float-এ `1.0` বা `0.5` লিখুন—`.5` এবং `1.` গ্রহণযোগ্য নয়।

## ৪. সাধারণ হিসাব ও wrapping হিসাব

সাধারণ integer operator overflow করলে abort করে। Wrapping operator নির্দিষ্ট bit-width-এর মধ্যে ঘুরে যায়:

| লেখা | সহজ অর্থ | overflow হলে |
|---|---|---|
| `a + b` | যোগ | abort |
| `a +% b` | একই width-এ wrapping যোগ | wrap করে |
| `a - b` | বিয়োগ | abort |
| `a -% b` | wrapping বিয়োগ | wrap করে |
| `a * b` | গুণ | abort |
| `a *% b` | wrapping গুণ | wrap করে |

উদাহরণ: `u8`-এ `255 +% 1`-এর ফল `0`। সাধারণ `255 + 1` হলে abort হওয়ার কথা। `+%`-এর wrapping যোগ এবং `-%`-এর wrapping বিয়োগ reversible update-এ কাজে লাগে; কিন্তু একই চিহ্নের সব operation reversible নয়।

## ৫. শর্ত ও পুনরাবৃত্তি

সাধারণ function-এ branch দিয়ে সিদ্ধান্ত বোঝানো যায়:

```cll
fn classify(value: i32) -> i32 {
    if value >= 0 {
        return 1;
    } else {
        return -1;
    }
}
```

`if`-এর condition `bool` হতে হবে। `else` না থাকলে সেটি ঐচ্ছিক; এখানে দুই পথই দেখানো হয়েছে যাতে কোন অবস্থায় কোন ফল আসে তা পরিষ্কার হয়।

নির্দিষ্ট পরিসরে `for` চালানো যায়:

```cll
fn sum_first_five() -> u32 {
    let mut total: u32 = 0;
    for i in 0..5 {
        total += i;
    }
    total
}
```

`0..5` হলো অর্ধ-খোলা range: `i` হবে `0, 1, 2, 3, 4`; `5` অন্তর্ভুক্ত নয়। `while`/অনির্দিষ্ট loop v1-এ সমর্থিত নয়।

উপরের snippet-গুলো semantics ব্যাখ্যার জন্য; বর্তমান compiler-এ parse বা execute করা যাবে না।

## ৬. Function ও ফল ফেরত দেওয়া

সাধারণ function `fn` দিয়ে লেখা হয়; `pure fn`-এ deterministic computation হয় এবং বাইরে থেকে state বদলানো বা capability ব্যবহার করা যায় না। Explicit `return` না লিখলে শেষ expression ফল হয়:

```cll
fn double(value: u32) -> u32 {
    value * 2
}
```

`Result[T, E]` দিয়ে ordinary function-এ সফলতা/ব্যর্থতা প্রকাশ করা যায়; `Option[T]` অনুপস্থিত value বোঝায়। `rev fn`-এর return type থাকে না—তার ফল state parameter-এ থাকে।

## ৭. তথ্য-ক্ষয় ও effect

CLL v1-এ effect দুটি: `erase` এবং `inexact`।

- `erase` বোঝায় কোনো operation তথ্য হারাতে পারে—যেমন division, bit shift, narrowing conversion, saturating arithmetic বা value overwrite। Ordinary function-এ এমন operation থাকলে effect inference তা চিহ্নিত করবে।
- `inexact` সাধারণত floating-point computation-এর সঙ্গে যুক্ত।
- `#[conserving]` দিয়ে দাবি করা যায় function-টিতে `erase` নেই; দাবি ভুল হলে compiler error হওয়ার কথা।

এগুলো শুধু annotation নয়: compiler যেন code-এর আচরণ বিশ্লেষণ করে caller-দেরও effect জানায়, সেটিই নকশা।

## ৮. Capability: অনুমতি হাতে নিয়ে কাজ

CLL-এর নকশায় কোনো function নিজের অজান্তে file, network বা clock ব্যবহার করতে পারবে না। কাজের অনুমতি parameter হিসেবে পেতে হবে:

```cll
fn read_report(file: FileRead("/data")) -> Report {
    # এখানে report পড়ার কাজ হবে
}
```

এটি capability-এর ধারণা বোঝানোর schematic উদাহরণ; `Report` ও read API standard library-তে এখনো সংজ্ঞায়িত নয়। `FileRead("/data")`-এর token শুধু নির্দিষ্ট scope-এর অনুমতি দেয়। অনুমতি কমানো যায়, বাড়ানো যায় না। `main`-ই root `Caps` পায় এবং প্রয়োজনমতো সীমিত token দেয়।

## ৯. Reversible function: compiler-এর বিশেষ নিশ্চয়তা

`rev fn` হলো state পরিবর্তনকারী function, যার inverse compiler নির্দিষ্ট নিয়ম থেকে তৈরি করবে। সাধারণ code-এর মতোই সহজভাবে call করার উদ্দেশ্য; কিন্তু body-তে কেবল অনুমোদিত reversible operation লেখা যাবে।

```cll
rev fn add_to(inout total: u32, amount: u32) {
    total +%= amount
}
```

এখানে `total`-এ wrapping যোগ হয়। `amount` target `total` পড়ে না, তাই update-টি উল্টানো যায়। পরে inverse দরকার হলে:

```cll
reverse add_to(total, amount)
```

Compiler-এর পরিকল্পিত কাজ হলো inverse-এ `+%=`-কে `-%=` করা, ফলে আগের `total` ফেরত আসে। `amount` উভয়বার একই থাকতে হবে। `rev fn`-এর মধ্যে `/`, সাধারণ `+`, `match`, `while`, float state, `Result` return, overwrite assignment зэрэг irreversible operation-গুলো reject হবে.

**Syntax caveat:** `rev fn` parameter-ийг `inout`-সহ কীভাবে লিখতে হবে তা README/SPEC-এর বর্ণনামূলক উদাহরণ ও `grammar.ebnf`-এর production-এ পুরোপুরি একরূপ নয়। উদাহরণটি পড়তে সহজ রাখার জন্য; এই syntax-কে final বা compiler-validated বলা হচ্ছে না। অসঙ্গতিটি owner/spec sign-off-এ মীমাংসা করতে হবে।

Reversible conditional ও loop-এর জন্য অতিরিক্ত নিয়ম আছে, কারণ inverse-কে জানতে হয় কোন branch/iteration চলেছিল। সেগুলো না বুঝে `rev fn`-এ সাধারণ `if`/loop বসাবেন না; `docs/SPEC.md`-এর R-REV-040/041 পড়ুন।

## ১০. Text ও comment

Comment `#` দিয়ে শুরু হয়ে line-এর শেষে শেষ হয়। Block comment নেই। String double quote-এ লেখা হয় এবং `{...}` দিয়ে interpolation করা যায়:

```cll
let name: str = "Ada";
let greeting: str = "Hello, {name}!";
```

`secret[T]` value string-এ বসানো নিষিদ্ধ, যাতে secret অসতর্কতায় প্রকাশ না পায়।

## ১১. Module ও import

একটি `.cll` file-ই একটি module। অন্য module আনতে স্পষ্ট `use` লিখতে হয়; wildcard import নেই:

```cll
use math::vector
```

বাইরে থেকে ব্যবহারযোগ্য item-এ `pub` এবং সম্পূর্ণ type signature দরকার। Cyclic import নিষিদ্ধ।

## ১২. নতুন শিক্ষার্থীর জন্য শেখার ক্রম

1. `let`, type annotation, সংখ্যা, string ও comment পড়ুন।
2. Function signature ও return expression বুঝুন।
3. Immutable value আগে ব্যবহার করুন; পরিবর্তন দরকার হলে `mut` স্পষ্ট করে দিন।
4. `if`, `match` ও bounded `for` শিখুন।
5. `Result`, `Option`, `erase` ও `inexact` পড়ুন।
6. Capability parameter দিয়ে permission কীভাবে সীমিত হয় বুঝুন।
7. সব শেষে `rev fn`, inverse ও `local`/`delocal` শিখুন—এ অংশের নিয়ম সবচেয়ে কঠোর।

## ১৩. যা এখনো করা হয়নি

বর্তমান repository-তে lexer ও conformance runner-এর অংশ আছে; পূর্ণ parser, type checker, interpreter, C backend এবং standard library এখনো নেই। ফলে এই মুহূর্তে কোনো পূর্ণ CLL program compile/run করার নির্দেশনা দেওয়া সৎ হবে না। M1 SPEC-ও draft; ROADMAP অনুযায়ী owner sign-off-সহ প্রতিটি milestone-এর exit criteria পূরণ করতে হবে।

কোনো syntax এখানে অচল মনে হলে সেটি নীরবে বদলাবেন না: আগে `AGENTS.md` → `docs/SPEC.md` → `DECISIONS.md` → `ROADMAP.md`-এর precedence অনুসরণ করুন। SPEC নীরব/পরস্পরবিরোধী হলে RFC বা `QUESTION.md` প্রক্রিয়া অনুসরণ করুন।

## ১৪. মূল নথি ও পরবর্তী পাঠ

- [`AGENTS.md`](../AGENTS.md): binding engineering ও language rules
- [`docs/SPEC.md`](SPEC.md): rule ID-সহ normative language specification
- [`DECISIONS.md`](../DECISIONS.md): সিদ্ধান্ত এবং তার rationale
- [`ROADMAP.md`](../ROADMAP.md): milestone ও measurable exit criteria
- [`grammar.ebnf`](../grammar.ebnf): syntax-এর formal grammar; SPEC-এর সঙ্গে অমিল হলে SPEC wins

---

**এক বাক্যে মনে রাখুন:** সাধারণ code-এ CLL-এর উদ্দেশ্য হলো পরিষ্কারভাবে কাজ লেখা; `rev fn`-এ compiler-কে প্রমাণ করতে হয় যে state বদলালেও আগের state নিরাপদে ফেরানো যাবে।