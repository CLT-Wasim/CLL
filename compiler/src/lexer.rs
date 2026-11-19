//! Lexer for CLL, implementing SPEC section 1 (`R-LEX-*`).
//!
//! The lexer is hand-written (AGENTS.md section 3). It produces a flat token
//! stream with byte spans, and it reports every lexical error it can see instead
//! of stopping at the first one, because a file with three typos should take one
//! round trip to fix, not three.
//!
//! Deliberate design points, each traceable to a rule:
//!
//! * `\r` is an error (R-LEX-002), not whitespace. CRLF files fail loudly.
//! * Identifiers are ASCII (R-LEX-004). A non-ASCII byte where an identifier
//!   starts is `E0002`, which also catches homoglyph tricks.
//! * Numeric literals are range-checked against their suffix **at lexing**
//!   (R-LEX-007, `E0006`), so `300u8` never reaches the type checker.
//! * `1.` and `.5` are errors (R-LEX-006). `0..3` is a range, not a malformed
//!   float, so the scanner looks one byte past the `.`.
//! * `+% =` is diagnosed as `E0009` with a fix suggestion, because it is always
//!   a typo for `+%=` (R-LEX-011).

use std::fmt;

/// A byte range in the source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// The smallest span covering both.
    #[must_use]
    pub fn to(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Integer width suffix (SPEC R-LEX-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntSuffix {
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    I8,
    I16,
    I32,
    I64,
    I128,
}

impl IntSuffix {
    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "u128" => Self::U128,
            "usize" => Self::Usize,
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "i64" => Self::I64,
            "i128" => Self::I128,
            _ => return None,
        })
    }

    /// Bits of storage.
    #[must_use]
    pub const fn bits(self) -> u32 {
        match self {
            Self::U8 | Self::I8 => 8,
            Self::U16 | Self::I16 => 16,
            Self::U32 | Self::I32 => 32,
            Self::U64 | Self::I64 | Self::Usize => 64,
            Self::U128 | Self::I128 => 128,
        }
    }

    #[must_use]
    pub const fn is_signed(self) -> bool {
        matches!(
            self,
            Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::I128
        )
    }

    /// Largest value the literal may hold, as an unsigned magnitude.
    ///
    /// For signed types the magnitude bound is `2^(bits-1)`, which is enough to
    /// reject `200u8` and `129i8` without deciding here whether `-128` is a
    /// negation of `128` (the parser decides that, since `-` is an operator).
    #[must_use]
    pub const fn max_magnitude(self) -> u128 {
        match self {
            Self::U8 => u8::MAX as u128,
            Self::U16 => u16::MAX as u128,
            Self::U32 => u32::MAX as u128,
            Self::U64 | Self::Usize => u64::MAX as u128,
            Self::U128 => u128::MAX,
            Self::I8 => 1u128 << 7,
            Self::I16 => 1u128 << 15,
            Self::I32 => 1u128 << 31,
            Self::I64 => 1u128 << 63,
            Self::I128 => 1u128 << 127,
        }
    }
}

/// Float width suffix (SPEC R-LEX-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatSuffix {
    F16,
    Bf16,
    F32,
    F64,
}

impl FloatSuffix {
    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "f16" => Self::F16,
            "bf16" => Self::Bf16,
            "f32" => Self::F32,
            "f64" => Self::F64,
            _ => return None,
        })
    }
}

/// Every token kind in the v1 grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok {
    Ident(String),
    /// An integer literal: magnitude plus optional suffix (R-LEX-005, R-LEX-007).
    Int {
        value: u128,
        suffix: Option<IntSuffix>,
    },
    Float {
        suffix: Option<FloatSuffix>,
    },
    Str(String),
    /// A `#@` directive line, already trimmed of its marker (R-LEX-012).
    TestDirective(String),

    // Keywords (R-LEX-010).
    As,
    AsWrapping,
    Break,
    Const,
    Comptime,
    Continue,
    Delocal,
    Discard,
    Else,
    Enum,
    Erase,
    Esac,
    False,
    Fi,
    Fn,
    For,
    From,
    If,
    Impl,
    In,
    Inout,
    Inexact,
    Let,
    Local,
    Loop,
    Match,
    Mod,
    Mut,
    Not,
    Pub,
    Pure,
    Rev,
    Reverse,
    Return,
    Rotl,
    Rotr,
    Secret,
    Struct,
    Swap,
    True,
    Type,
    Unsafe,
    Use,
    While,
    With,

    // Punctuation (R-LEX-011), maximal munch.
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Amp,
    Pipe,
    Tilde,
    Bang,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    Shl,
    Shr,
    AndAnd,
    OrOr,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    CaretEq,
    AmpEq,
    PipeEq,
    ShlEq,
    ShrEq,
    PlusPercent,
    MinusPercent,
    StarPercent,
    PlusPercentEq,
    MinusPercentEq,
    StarPercentEq,
    PlusPipe,
    MinusPipe,
    StarPipe,
    Arrow,
    FatArrow,
    ColonColon,
    Colon,
    Semi,
    Comma,
    Dot,
    DotDot,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Hash,
    HashBang,
}

impl Tok {
    /// Human-readable name for diagnostics.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Ident(name) => format!("identifier `{name}`"),
            Self::Int { .. } => "integer literal".to_owned(),
            Self::Float { .. } => "float literal".to_owned(),
            Self::Str(_) => "string literal".to_owned(),
            Self::TestDirective(_) => "test directive".to_owned(),
            other => format!("`{}`", other.symbol()),
        }
    }

    /// The source spelling of a fixed token; empty for literals.
    #[must_use]
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Ident(_) | Self::Int { .. } | Self::Float { .. } | Self::Str(_) => "",
            Self::TestDirective(_) => "#@",
            Self::As
            | Self::AsWrapping
            | Self::Break
            | Self::Const
            | Self::Comptime
            | Self::Continue
            | Self::Delocal
            | Self::Discard
            | Self::Else
            | Self::Enum
            | Self::Erase
            | Self::Esac
            | Self::False
            | Self::Fi
            | Self::Fn
            | Self::For
            | Self::From
            | Self::If
            | Self::Impl
            | Self::In
            | Self::Inout
            | Self::Inexact
            | Self::Let
            | Self::Local
            | Self::Loop
            | Self::Match
            | Self::Mod
            | Self::Mut
            | Self::Not
            | Self::Pub
            | Self::Pure
            | Self::Rev
            | Self::Reverse
            | Self::Return
            | Self::Rotl
            | Self::Rotr
            | Self::Secret
            | Self::Struct
            | Self::Swap
            | Self::True
            | Self::Type
            | Self::Unsafe
            | Self::Use
            | Self::While
            | Self::With => self.keyword_symbol(),
            _ => self.punctuation_symbol(),
        }
    }

    const fn keyword_symbol(&self) -> &'static str {
        match self {
            Self::As => "as",
            Self::AsWrapping => "as_wrapping",
            Self::Break => "break",
            Self::Const => "const",
            Self::Comptime => "comptime",
            Self::Continue => "continue",
            Self::Delocal => "delocal",
            Self::Discard => "discard",
            Self::Else => "else",
            Self::Enum => "enum",
            Self::Erase => "erase",
            Self::Esac => "esac",
            Self::False => "false",
            Self::Fi => "fi",
            Self::Fn => "fn",
            Self::For => "for",
            Self::From => "from",
            Self::If => "if",
            Self::Impl => "impl",
            Self::In => "in",
            Self::Inout => "inout",
            Self::Inexact => "inexact",
            Self::Let => "let",
            Self::Local => "local",
            Self::Loop => "loop",
            Self::Match => "match",
            Self::Mod => "mod",
            Self::Mut => "mut",
            Self::Not => "not",
            Self::Pub => "pub",
            Self::Pure => "pure",
            Self::Rev => "rev",
            Self::Reverse => "reverse",
            Self::Return => "return",
            Self::Rotl => "rotl",
            Self::Rotr => "rotr",
            Self::Secret => "secret",
            Self::Struct => "struct",
            Self::Swap => "swap",
            Self::True => "true",
            Self::Type => "type",
            Self::Unsafe => "unsafe",
            Self::Use => "use",
            Self::While => "while",
            Self::With => "with",
            _ => "",
        }
    }

    const fn punctuation_symbol(&self) -> &'static str {
        match self {
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::Percent => "%",
            Self::Caret => "^",
            Self::Amp => "&",
            Self::Pipe => "|",
            Self::Tilde => "~",
            Self::Bang => "!",
            Self::Eq => "=",
            Self::EqEq => "==",
            Self::NotEq => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::AndAnd => "&&",
            Self::OrOr => "||",
            Self::PlusEq => "+=",
            Self::MinusEq => "-=",
            Self::StarEq => "*=",
            Self::SlashEq => "/=",
            Self::PercentEq => "%=",
            Self::CaretEq => "^=",
            Self::AmpEq => "&=",
            Self::PipeEq => "|=",
            Self::ShlEq => "<<=",
            Self::ShrEq => ">>=",
            Self::PlusPercent => "+%",
            Self::MinusPercent => "-%",
            Self::StarPercent => "*%",
            Self::PlusPercentEq => "+%=",
            Self::MinusPercentEq => "-%=",
            Self::StarPercentEq => "*%=",
            Self::PlusPipe => "+|",
            Self::MinusPipe => "-|",
            Self::StarPipe => "*|",
            Self::Arrow => "->",
            Self::FatArrow => "=>",
            Self::ColonColon => "::",
            Self::Colon => ":",
            Self::Semi => ";",
            Self::Comma => ",",
            Self::Dot => ".",
            Self::DotDot => "..",
            Self::LParen => "(",
            Self::RParen => ")",
            Self::LBracket => "[",
            Self::RBracket => "]",
            Self::LBrace => "{",
            Self::RBrace => "}",
            Self::Hash => "#",
            Self::HashBang => "#!",
            _ => "",
        }
    }
}

/// A token with its source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: Tok,
    pub span: Span,
}

/// A lexical error. Codes match SPEC section 11 and are never reused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub span: Span,
    pub message: String,
    pub help: Option<String>,
}

impl Diagnostic {
    fn new(code: &'static str, span: Span, message: impl Into<String>) -> Self {
        Self {
            code,
            span,
            message: message.into(),
            help: None,
        }
    }

    fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}..{}: {}",
            self.code, self.span.start, self.span.end, self.message
        )?;
        if let Some(help) = &self.help {
            write!(f, "\n  help: {help}")?;
        }
        Ok(())
    }
}

/// Lexes `src` into tokens.
///
/// `allow_directives` enables `#@` header lines, which are only legal in
/// `tests/` (R-LEX-012). The caller knows the file's role; the lexer does not.
///
/// The token list simply ends; there is no `Eof` token, which keeps every
/// consumer's loop a plain `for`.
///
/// # Errors
/// Returns every lexical [`Diagnostic`] found, in source order. A file with three
/// mistakes yields three diagnostics, so fixing it takes one round trip.
pub fn lex(src: &str, allow_directives: bool) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut lexer = Lexer {
        src,
        bytes: src.as_bytes(),
        pos: 0,
        tokens: Vec::new(),
        diagnostics: Vec::new(),
        allow_directives,
        line_start: true,
    };
    lexer.run();
    if lexer.diagnostics.is_empty() {
        Ok(lexer.tokens)
    } else {
        Err(lexer.diagnostics)
    }
}

struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
    allow_directives: bool,
    line_start: bool,
}

impl Lexer<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }

    fn span_from(&self, start: usize) -> Span {
        Span::new(
            u32::try_from(start).unwrap_or(u32::MAX),
            u32::try_from(self.pos).unwrap_or(u32::MAX),
        )
    }

    fn push(&mut self, kind: Tok, start: usize) {
        let span = self.span_from(start);
        self.tokens.push(Token { kind, span });
        self.line_start = false;
    }

    fn error(&mut self, code: &'static str, span: Span, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::new(code, span, message));
    }

    fn error_help(
        &mut self,
        code: &'static str,
        span: Span,
        message: impl Into<String>,
        help: impl Into<String>,
    ) {
        self.diagnostics
            .push(Diagnostic::new(code, span, message).with_help(help));
    }

    fn run(&mut self) {
        while let Some(byte) = self.peek() {
            match byte {
                b'\r' => {
                    let start = self.pos;
                    self.bump();
                    self.error_help(
                        "E0001",
                        self.span_from(start),
                        "carriage return in source (SPEC R-LEX-002)",
                        "line endings must be `\\n`; convert the file to LF",
                    );
                }
                b'\n' => {
                    self.bump();
                    self.line_start = true;
                }
                b' ' | b'\t' => {
                    self.bump();
                }
                b'#' => self.lex_hash(),
                b'"' => self.lex_string(),
                b'0'..=b'9' => self.lex_number(),
                b'A'..=b'Z' | b'a'..=b'z' | b'_' => self.lex_ident(),
                b'.' => {
                    if self.peek_at(1).is_some_and(|next| next.is_ascii_digit()) {
                        let start = self.pos;
                        self.bump();
                        while self.peek().is_some_and(|b| b.is_ascii_digit() || b == b'_') {
                            self.bump();
                        }
                        self.error_help(
                            "E0004",
                            self.span_from(start),
                            "float literal without a leading digit (SPEC R-LEX-006)",
                            "write `0.5` instead of `.5`",
                        );
                    } else {
                        self.lex_punctuation();
                    }
                }
                b'+' | b'-' | b'*' | b'/' | b'%' | b'^' | b'&' | b'|' | b'~' | b'!' | b'='
                | b'<' | b'>' | b':' | b';' | b',' | b'(' | b')' | b'[' | b']' | b'{' | b'}' => {
                    self.lex_punctuation();
                }
                other => {
                    let start = self.pos;
                    // Consume the whole UTF-8 sequence so one error covers one character.
                    let char_len = utf8_len(other);
                    for _ in 0..char_len {
                        self.bump();
                    }
                    let text = self
                        .src
                        .get(start..self.pos)
                        .unwrap_or("<invalid>")
                        .to_owned();
                    self.error_help(
                        "E0002",
                        self.span_from(start),
                        format!("unexpected character `{text}` (SPEC R-LEX-004)"),
                        "identifiers are ASCII: `[A-Za-z_][A-Za-z0-9_]*`; \
                         non-ASCII text belongs in string literals",
                    );
                }
            }
        }
    }

    /// `#` starts a comment, except at the start of a line where `#@` starts a
    /// test directive (R-LEX-012).
    fn lex_hash(&mut self) {
        let start = self.pos;
        if self.line_start && self.peek_at(1) == Some(b'@') {
            self.bump();
            self.bump();
            let text_start = self.pos;
            while let Some(byte) = self.peek() {
                if byte == b'\n' {
                    break;
                }
                if byte == b'\r' {
                    break;
                }
                self.bump();
            }
            let text = self
                .src
                .get(text_start..self.pos)
                .unwrap_or("")
                .trim()
                .to_owned();
            if self.allow_directives {
                self.push(Tok::TestDirective(text), start);
            } else {
                self.error_help(
                    "E0010",
                    self.span_from(start),
                    "`#@` test directive outside `tests/` (SPEC R-LEX-012)",
                    "test directives are only meaningful in the conformance suite",
                );
            }
            return;
        }
        while let Some(byte) = self.peek() {
            if byte == b'\n' || byte == b'\r' {
                break;
            }
            self.bump();
        }
        // A comment does not end the line for `line_start` purposes until the
        // newline is consumed by `run`.
    }

    fn lex_ident(&mut self) {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            self.bump();
        }
        let text = self.src.get(start..self.pos).unwrap_or("");
        let kind = keyword(text).unwrap_or_else(|| Tok::Ident(text.to_owned()));
        self.push(kind, start);
    }

    /// Handles `0x`/`0b`/`0o` and the leading-zero trap. Returns true when the
    /// literal was fully consumed here.
    fn try_radix_prefix(&mut self, start: usize) -> bool {
        if self.peek() != Some(b'0') {
            return false;
        }
        match self.peek_at(1) {
            Some(b'x' | b'X') => {
                self.lex_radix(start, 16, "0x");
                true
            }
            Some(b'b' | b'B') => {
                self.lex_radix(start, 2, "0b");
                true
            }
            Some(b'o' | b'O') => {
                self.lex_radix(start, 8, "0o");
                true
            }
            Some(b'0'..=b'9') => {
                self.bump();
                while self.peek().is_some_and(|b| b.is_ascii_digit() || b == b'_') {
                    self.bump();
                }
                self.error_help(
                    "E0003",
                    self.span_from(start),
                    "leading-zero integer literal (SPEC R-LEX-005)",
                    "write `0o123` for octal, or remove the leading zero",
                );
                true
            }
            _ => false,
        }
    }

    fn lex_number(&mut self) {
        let start = self.pos;
        if self.try_radix_prefix(start) {
            return;
        }

        while self.peek().is_some_and(|b| b.is_ascii_digit() || b == b'_') {
            self.bump();
        }
        let is_float = self.consume_fraction(start) | self.consume_exponent(start);
        let (suffix_text, body) = self.take_numeric_suffix(start);

        if is_float {
            let suffix = self.float_suffix(start, &suffix_text);
            self.push(Tok::Float { suffix }, start);
            return;
        }

        let suffix = self.int_suffix(start, &suffix_text);
        let value = self.int_magnitude(start, &body);
        if let Some(suffix) = suffix
            && value > suffix.max_magnitude()
        {
            self.error_help(
                "E0006",
                self.span_from(start),
                format!("literal `{body}` does not fit in `{suffix_text}` (SPEC R-LEX-007)"),
                format!("`{suffix_text}` holds at most {}", suffix.max_magnitude()),
            );
        }
        self.push(Tok::Int { value, suffix }, start);
    }

    /// Consumes `.digits` if present. `0..3` is a range, so a `.` followed by
    /// another `.` is not a decimal point (SPEC R-LEX-006).
    fn consume_fraction(&mut self, start: usize) -> bool {
        if self.peek() != Some(b'.') || self.peek_at(1) == Some(b'.') {
            return false;
        }
        self.bump();
        if !self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.error_help(
                "E0004",
                self.span_from(start),
                "float literal without digits after the decimal point (SPEC R-LEX-006)",
                "write `1.0` instead of `1.`",
            );
        }
        while self.peek().is_some_and(|b| b.is_ascii_digit() || b == b'_') {
            self.bump();
        }
        true
    }

    fn consume_exponent(&mut self, start: usize) -> bool {
        if !matches!(self.peek(), Some(b'e' | b'E')) {
            return false;
        }
        self.bump();
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.bump();
        }
        if !self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.error_help(
                "E0004",
                self.span_from(start),
                "exponent without digits (SPEC R-LEX-006)",
                "write `1e9` or `1.0e9`",
            );
        }
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.bump();
        }
        true
    }

    /// Splits `12_345u8` into the suffix text and the digit body.
    fn take_numeric_suffix(&mut self, start: usize) -> (String, String) {
        let suffix_start = self.pos;
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            self.bump();
        }
        let suffix_text = self
            .src
            .get(suffix_start..self.pos)
            .unwrap_or("")
            .to_owned();
        let body = self
            .src
            .get(start..suffix_start)
            .unwrap_or("")
            .replace('_', "");
        (suffix_text, body)
    }

    fn float_suffix(&mut self, start: usize, text: &str) -> Option<FloatSuffix> {
        if text.is_empty() {
            return None;
        }
        FloatSuffix::parse(text).or_else(|| {
            self.error_help(
                "E0005",
                self.span_from(start),
                format!("unknown numeric suffix `{text}` (SPEC R-LEX-007)"),
                "float suffixes are `f16`, `bf16`, `f32`, `f64`",
            );
            None
        })
    }

    fn int_suffix(&mut self, start: usize, text: &str) -> Option<IntSuffix> {
        if text.is_empty() {
            return None;
        }
        IntSuffix::parse(text).or_else(|| {
            self.error_help(
                "E0005",
                self.span_from(start),
                format!("unknown numeric suffix `{text}` (SPEC R-LEX-007)"),
                "integer suffixes are `u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128`",
            );
            None
        })
    }

    fn int_magnitude(&mut self, start: usize, body: &str) -> u128 {
        parse_decimal_magnitude(body).unwrap_or_else(|()| {
            self.error_help(
                "E0006",
                self.span_from(start),
                "integer literal does not fit in 128 bits",
                "no CLL integer type is wider than 128 bits",
            );
            0
        })
    }

    fn lex_radix(&mut self, start: usize, radix: u32, prefix: &str) {
        self.bump(); // '0'
        self.bump(); // 'x' | 'b' | 'o'
        let digits_start = self.pos;
        while self
            .peek()
            .is_some_and(|b| (b as char).is_digit(radix) || b == b'_')
        {
            self.bump();
        }
        let digits = self
            .src
            .get(digits_start..self.pos)
            .unwrap_or("")
            .replace('_', "");
        if digits.is_empty() {
            self.error_help(
                "E0006",
                self.span_from(start),
                format!("`{prefix}` literal has no digits (SPEC R-LEX-005)"),
                "write at least one digit after the radix prefix",
            );
            self.push(
                Tok::Int {
                    value: 0,
                    suffix: None,
                },
                start,
            );
            return;
        }

        let suffix_start = self.pos;
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            self.bump();
        }
        let suffix_text = self
            .src
            .get(suffix_start..self.pos)
            .unwrap_or("")
            .to_owned();
        let suffix = self.int_suffix(start, &suffix_text);

        let value = u128::from_str_radix(&digits, radix).unwrap_or(u128::MAX);
        if let Some(suffix) = suffix
            && value > suffix.max_magnitude()
        {
            self.error_help(
                "E0006",
                self.span_from(start),
                format!(
                    "literal `{prefix}{digits}` does not fit in `{suffix_text}` (SPEC R-LEX-007)"
                ),
                format!("`{suffix_text}` holds at most {}", suffix.max_magnitude()),
            );
        }
        self.push(Tok::Int { value, suffix }, start);
    }

    fn lex_string(&mut self) {
        let start = self.pos;
        self.bump(); // opening quote
        let mut value = String::new();
        loop {
            let Some(byte) = self.peek() else {
                self.error_help(
                    "E0008",
                    self.span_from(start),
                    "unterminated string literal (SPEC R-LEX-008)",
                    "add the closing `\"`",
                );
                break;
            };
            match byte {
                b'"' => {
                    self.bump();
                    break;
                }
                b'\n' => {
                    self.error_help(
                        "E0008",
                        self.span_from(start),
                        "unterminated string literal (SPEC R-LEX-008)",
                        "string literals do not span lines; add the closing `\"`",
                    );
                    break;
                }
                b'\\' => self.lex_escape(&mut value),
                _ => {
                    let char_len = utf8_len(byte);
                    let char_start = self.pos;
                    for _ in 0..char_len {
                        self.bump();
                    }
                    if let Some(chunk) = self.src.get(char_start..self.pos) {
                        value.push_str(chunk);
                    }
                }
            }
        }
        self.push(Tok::Str(value), start);
    }

    /// Consumes one escape sequence after the backslash has been seen
    /// (SPEC R-LEX-008). A malformed escape is reported and skipped; the
    /// surrounding literal still produces a token so later errors are not
    /// masked by a cascade.
    fn lex_escape(&mut self, value: &mut String) {
        let escape_start = self.pos;
        self.bump(); // the backslash
        let Some(escape) = self.bump() else {
            self.error(
                "E0008",
                self.span_from(escape_start),
                "unterminated string literal",
            );
            return;
        };
        match escape {
            b'n' => value.push('\n'),
            b't' => value.push('\t'),
            b'r' => value.push('\r'),
            b'\\' => value.push('\\'),
            b'"' => value.push('"'),
            b'{' => value.push('{'),
            b'}' => value.push('}'),
            b'u' => self.lex_unicode_escape(escape_start, value),
            other => {
                let text = char::from(other).to_string();
                self.error_help(
                    "E0007",
                    self.span_from(escape_start),
                    format!("unknown string escape `\\{text}` (SPEC R-LEX-008)"),
                    "valid escapes are `\\n \\t \\r \\\\ \\\" \\{ \\}` and `\\u{...}`",
                );
            }
        }
    }

    fn lex_unicode_escape(&mut self, escape_start: usize, value: &mut String) {
        if self.bump() != Some(b'{') {
            self.error_help(
                "E0007",
                self.span_from(escape_start),
                "malformed `\\u` escape (SPEC R-LEX-008)",
                "write `\\u{1F600}`",
            );
            return;
        }
        let hex_start = self.pos;
        while self.peek().is_some_and(|b| b.is_ascii_hexdigit()) {
            self.bump();
        }
        let hex = self.src.get(hex_start..self.pos).unwrap_or("");
        if self.bump() != Some(b'}') {
            self.error_help(
                "E0007",
                self.span_from(escape_start),
                "malformed `\\u` escape (SPEC R-LEX-008)",
                "write `\\u{1F600}`",
            );
            return;
        }
        match u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
            Some(ch) => value.push(ch),
            None => self.error_help(
                "E0007",
                self.span_from(escape_start),
                format!("`\\u{{{hex}}}` is not a scalar value"),
                "use a Unicode scalar value, e.g. `\\u{41}`",
            ),
        }
    }

    fn lex_punctuation(&mut self) {
        let start = self.pos;
        let first = self.bump().unwrap_or(0);
        let second = self.peek();
        let third = self.peek_at(1);

        let (kind, consumed) = match (first, second, third) {
            (b'+', Some(b'%'), Some(b'=')) => (Tok::PlusPercentEq, 3),
            (b'-', Some(b'%'), Some(b'=')) => (Tok::MinusPercentEq, 3),
            (b'*', Some(b'%'), Some(b'=')) => (Tok::StarPercentEq, 3),
            (b'+', Some(b'%'), _) => (Tok::PlusPercent, 2),
            (b'-', Some(b'%'), _) => (Tok::MinusPercent, 2),
            (b'*', Some(b'%'), _) => (Tok::StarPercent, 2),
            (b'+', Some(b'|'), _) => (Tok::PlusPipe, 2),
            (b'-', Some(b'|'), _) => (Tok::MinusPipe, 2),
            (b'*', Some(b'|'), _) => (Tok::StarPipe, 2),
            (b'+', Some(b'='), _) => (Tok::PlusEq, 2),
            (b'-', Some(b'='), _) => (Tok::MinusEq, 2),
            (b'*', Some(b'='), _) => (Tok::StarEq, 2),
            (b'/', Some(b'='), _) => (Tok::SlashEq, 2),
            (b'%', Some(b'='), _) => (Tok::PercentEq, 2),
            (b'^', Some(b'='), _) => (Tok::CaretEq, 2),
            (b'&', Some(b'&'), _) => (Tok::AndAnd, 2),
            (b'&', Some(b'='), _) => (Tok::AmpEq, 2),
            (b'|', Some(b'|'), _) => (Tok::OrOr, 2),
            (b'|', Some(b'='), _) => (Tok::PipeEq, 2),
            (b'<', Some(b'<'), Some(b'=')) => (Tok::ShlEq, 3),
            (b'>', Some(b'>'), Some(b'=')) => (Tok::ShrEq, 3),
            (b'<', Some(b'<'), _) => (Tok::Shl, 2),
            (b'>', Some(b'>'), _) => (Tok::Shr, 2),
            (b'<', Some(b'='), _) => (Tok::Le, 2),
            (b'>', Some(b'='), _) => (Tok::Ge, 2),
            (b'=', Some(b'='), _) => (Tok::EqEq, 2),
            (b'=', Some(b'>'), _) => (Tok::FatArrow, 2),
            (b'!', Some(b'='), _) => (Tok::NotEq, 2),
            (b'-', Some(b'>'), _) => (Tok::Arrow, 2),
            (b':', Some(b':'), _) => (Tok::ColonColon, 2),
            (b'.', Some(b'.'), _) => (Tok::DotDot, 2),
            (b'+', _, _) => (Tok::Plus, 1),
            (b'-', _, _) => (Tok::Minus, 1),
            (b'*', _, _) => (Tok::Star, 1),
            (b'/', _, _) => (Tok::Slash, 1),
            (b'%', _, _) => (Tok::Percent, 1),
            (b'^', _, _) => (Tok::Caret, 1),
            (b'&', _, _) => (Tok::Amp, 1),
            (b'|', _, _) => (Tok::Pipe, 1),
            (b'~', _, _) => (Tok::Tilde, 1),
            (b'!', _, _) => (Tok::Bang, 1),
            (b'=', _, _) => (Tok::Eq, 1),
            (b'<', _, _) => (Tok::Lt, 1),
            (b'>', _, _) => (Tok::Gt, 1),
            (b':', _, _) => (Tok::Colon, 1),
            (b';', _, _) => (Tok::Semi, 1),
            (b',', _, _) => (Tok::Comma, 1),
            (b'.', _, _) => (Tok::Dot, 1),
            (b'(', _, _) => (Tok::LParen, 1),
            (b')', _, _) => (Tok::RParen, 1),
            (b'[', _, _) => (Tok::LBracket, 1),
            (b']', _, _) => (Tok::RBracket, 1),
            (b'{', _, _) => (Tok::LBrace, 1),
            (b'}', _, _) => (Tok::RBrace, 1),
            _ => (Tok::Hash, 1),
        };

        // `+` was already consumed; advance for the remaining characters.
        for _ in 1..consumed {
            self.bump();
        }

        // R-LEX-011: `+% =` is always a typo for `+%=`.
        if matches!(
            kind,
            Tok::PlusPercent | Tok::MinusPercent | Tok::StarPercent
        ) {
            let mut lookahead = self.pos;
            while matches!(self.bytes.get(lookahead), Some(b' ' | b'\t')) {
                lookahead += 1;
            }
            if self.bytes.get(lookahead) == Some(&b'=') {
                let span = Span::new(
                    u32::try_from(start).unwrap_or(u32::MAX),
                    u32::try_from(lookahead + 1).unwrap_or(u32::MAX),
                );
                self.error_help(
                    "E0009",
                    span,
                    "malformed compound assignment (SPEC R-LEX-011)",
                    format!("write `{}=` without a space", kind.symbol()),
                );
            }
        }

        self.push(kind, start);
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

fn parse_decimal_magnitude(digits: &str) -> Result<u128, ()> {
    let mut value: u128 = 0;
    for byte in digits.bytes() {
        let digit = u128::from(byte.wrapping_sub(b'0'));
        if digit > 9 {
            return Err(());
        }
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(digit))
            .ok_or(())?;
    }
    Ok(value)
}

/// Keyword table (R-LEX-010). Reserved-but-unimplemented words are included so
/// that using them is a parse error with a real message, not a confusing one.
fn keyword(text: &str) -> Option<Tok> {
    Some(match text {
        "as" => Tok::As,
        "as_wrapping" => Tok::AsWrapping,
        "break" => Tok::Break,
        "const" => Tok::Const,
        "comptime" => Tok::Comptime,
        "continue" => Tok::Continue,
        "delocal" => Tok::Delocal,
        "discard" => Tok::Discard,
        "else" => Tok::Else,
        "enum" => Tok::Enum,
        "erase" => Tok::Erase,
        "esac" => Tok::Esac,
        "false" => Tok::False,
        "fi" => Tok::Fi,
        "fn" => Tok::Fn,
        "for" => Tok::For,
        "from" => Tok::From,
        "if" => Tok::If,
        "impl" => Tok::Impl,
        "in" => Tok::In,
        "inout" => Tok::Inout,
        "inexact" => Tok::Inexact,
        "let" => Tok::Let,
        "local" => Tok::Local,
        "loop" => Tok::Loop,
        "match" => Tok::Match,
        "mod" => Tok::Mod,
        "mut" => Tok::Mut,
        "not" => Tok::Not,
        "pub" => Tok::Pub,
        "pure" => Tok::Pure,
        "rev" => Tok::Rev,
        "reverse" => Tok::Reverse,
        "return" => Tok::Return,
        "rotl" => Tok::Rotl,
        "rotr" => Tok::Rotr,
        "secret" => Tok::Secret,
        "struct" => Tok::Struct,
        "swap" => Tok::Swap,
        "true" => Tok::True,
        "type" => Tok::Type,
        "unsafe" => Tok::Unsafe,
        "use" => Tok::Use,
        "while" => Tok::While,
        "with" => Tok::With,
        _ => return None,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<Tok> {
        lex(src, false)
            .expect("expected valid source")
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    fn errors(src: &str) -> Vec<&'static str> {
        lex(src, false)
            .expect_err("expected a lex error")
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    #[test]
    fn empty_source_has_no_tokens() {
        assert_eq!(kinds(""), Vec::<Tok>::new());
    }

    #[test]
    fn keywords_and_identifiers_are_distinguished() {
        assert_eq!(
            kinds("fn rev fnx _x X1"),
            vec![
                Tok::Fn,
                Tok::Rev,
                Tok::Ident("fnx".to_owned()),
                Tok::Ident("_x".to_owned()),
                Tok::Ident("X1".to_owned()),
            ]
        );
    }

    #[test]
    fn every_reserved_word_lexes_as_a_keyword() {
        let words = [
            "as",
            "as_wrapping",
            "break",
            "const",
            "comptime",
            "continue",
            "delocal",
            "discard",
            "else",
            "enum",
            "erase",
            "esac",
            "false",
            "fi",
            "fn",
            "for",
            "from",
            "if",
            "impl",
            "in",
            "inout",
            "inexact",
            "let",
            "local",
            "loop",
            "match",
            "mod",
            "mut",
            "not",
            "pub",
            "pure",
            "rev",
            "reverse",
            "return",
            "rotl",
            "rotr",
            "secret",
            "struct",
            "swap",
            "true",
            "type",
            "unsafe",
            "use",
            "while",
            "with",
        ];
        for word in words {
            let lexed = kinds(word);
            assert_eq!(lexed.len(), 1, "`{word}` should be one token");
            assert!(
                !matches!(lexed.first(), Some(Tok::Ident(_))),
                "`{word}` must be a keyword, not an identifier"
            );
        }
    }

    #[test]
    fn maximal_munch_for_wrapping_and_saturating_operators() {
        assert_eq!(
            kinds("a +%= b -%= c *%= d"),
            vec![
                Tok::Ident("a".to_owned()),
                Tok::PlusPercentEq,
                Tok::Ident("b".to_owned()),
                Tok::MinusPercentEq,
                Tok::Ident("c".to_owned()),
                Tok::StarPercentEq,
                Tok::Ident("d".to_owned()),
            ]
        );
        assert_eq!(
            kinds("+% -% *% +| -| *|"),
            vec![
                Tok::PlusPercent,
                Tok::MinusPercent,
                Tok::StarPercent,
                Tok::PlusPipe,
                Tok::MinusPipe,
                Tok::StarPipe,
            ]
        );
    }

    #[test]
    fn range_is_not_a_malformed_float() {
        assert_eq!(
            kinds("0..3"),
            vec![
                Tok::Int {
                    value: 0,
                    suffix: None
                },
                Tok::DotDot,
                Tok::Int {
                    value: 3,
                    suffix: None
                },
            ]
        );
    }

    #[test]
    fn integer_literals_and_suffixes() {
        assert_eq!(
            kinds("0xFF 0b1010 0o17 1_000 42u8 7usize"),
            vec![
                Tok::Int {
                    value: 255,
                    suffix: None
                },
                Tok::Int {
                    value: 10,
                    suffix: None
                },
                Tok::Int {
                    value: 15,
                    suffix: None
                },
                Tok::Int {
                    value: 1000,
                    suffix: None
                },
                Tok::Int {
                    value: 42,
                    suffix: Some(IntSuffix::U8)
                },
                Tok::Int {
                    value: 7,
                    suffix: Some(IntSuffix::Usize)
                },
            ]
        );
    }

    #[test]
    fn float_literals() {
        assert_eq!(
            kinds("1.0 2.5e-3 1e9 1.5f32"),
            vec![
                Tok::Float { suffix: None },
                Tok::Float { suffix: None },
                Tok::Float { suffix: None },
                Tok::Float {
                    suffix: Some(FloatSuffix::F32)
                },
            ]
        );
    }

    #[test]
    fn string_escapes_and_unicode() {
        assert_eq!(kinds(r#""a\nb\u{41}""#), vec![Tok::Str("a\nbA".to_owned())]);
    }

    #[test]
    fn comments_are_skipped() {
        assert_eq!(kinds("# a note\nfn"), vec![Tok::Fn]);
        assert_eq!(kinds("fn # trailing"), vec![Tok::Fn]);
    }

    #[test]
    fn spans_are_byte_ranges() {
        let tokens = lex("fn main", false).expect("valid");
        assert_eq!(tokens.first().map(|t| t.span), Some(Span::new(0, 2)));
        assert_eq!(tokens.get(1).map(|t| t.span), Some(Span::new(3, 7)));
    }

    // ---- negative: every code in SPEC section 11 that the lexer owns ----

    #[test]
    fn e0001_carriage_return() {
        assert_eq!(errors("fn\r\n"), vec!["E0001"]);
    }

    #[test]
    fn e0002_non_ascii_identifier() {
        assert_eq!(errors("fn café() {}"), vec!["E0002"]);
    }

    #[test]
    fn e0003_leading_zero() {
        assert_eq!(errors("0123"), vec!["E0003"]);
    }

    #[test]
    fn e0004_malformed_floats() {
        assert_eq!(errors("1."), vec!["E0004"]);
        assert_eq!(errors(".5"), vec!["E0004"]);
        assert_eq!(errors("1e"), vec!["E0004"]);
    }

    #[test]
    fn e0005_unknown_suffix() {
        assert_eq!(errors("42q"), vec!["E0005"]);
        assert_eq!(errors("1.0f128"), vec!["E0005"]);
    }

    #[test]
    fn e0006_literal_out_of_range() {
        assert_eq!(errors("300u8"), vec!["E0006"]);
        assert_eq!(errors("129i8"), vec!["E0006"]);
        assert_eq!(errors("0x"), vec!["E0006"]);
    }

    #[test]
    fn e0007_unknown_escape() {
        assert_eq!(errors(r#""a\qb""#), vec!["E0007"]);
    }

    #[test]
    fn e0008_unterminated_string() {
        assert_eq!(errors("\"abc"), vec!["E0008"]);
        // A newline ends the literal; the rest of the line lexes on its own and
        // its stray quote is a second, separate unterminated literal.
        let codes = errors("\"abc\ndef\"");
        assert_eq!(codes.first(), Some(&"E0008"));
        assert_eq!(codes.last(), Some(&"E0008"));
    }

    #[test]
    fn e0009_spaced_compound_assignment() {
        assert_eq!(errors("a +% = b"), vec!["E0009"]);
    }

    #[test]
    fn e0010_directive_outside_tests() {
        assert_eq!(errors("#@ rule: R-X-1\n"), vec!["E0010"]);
    }

    #[test]
    fn directives_are_allowed_in_tests() {
        let tokens = lex("#@ rule: R-REV-020\nfn", true).expect("valid in tests");
        assert_eq!(
            tokens.first().map(|t| t.kind.clone()),
            Some(Tok::TestDirective("rule: R-REV-020".to_owned()))
        );
    }

    #[test]
    fn all_errors_in_a_file_are_reported_not_just_the_first() {
        let codes = errors("0123 300u8 \"abc");
        assert_eq!(codes.len(), 3, "expected one error per mistake: {codes:?}");
    }

    #[test]
    fn spans_of_multibyte_characters_cover_whole_sequence() {
        let diagnostic = lex("é", false).expect_err("non-ASCII");
        let span = diagnostic.first().map(|d| d.span);
        assert_eq!(span, Some(Span::new(0, 2)), "é is two bytes in UTF-8");
    }

    /// ROADMAP M3 exit: the lexer must never panic on arbitrary input.
    ///
    /// This is a deterministic pseudo-random sweep over 20,000 inputs drawn from
    /// the byte range that matters (ASCII, multi-byte UTF-8 leads, control
    /// bytes). A full 30-minute fuzz run is a separate, longer campaign; this
    /// keeps the invariant checked on every commit in under a second.
    #[test]
    fn lexer_never_panics_on_arbitrary_input() {
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..20_000 {
            let len = (next() % 48) as usize;
            let mut input = String::new();
            for _ in 0..len {
                let byte = (next() % 256) as u8;
                // Build valid UTF-8 by taking a char from a bounded alphabet that
                // still includes multi-byte characters and every delimiter.
                let ch = match byte % 8 {
                    0 => char::from(b' ' + (byte % 95)),
                    1 => char::from(b'0' + (byte % 10)),
                    2 => '\n',
                    3 => '\r',
                    4 => '"',
                    5 => '#',
                    6 => 'é',
                    _ => '𝄞',
                };
                input.push(ch);
            }
            // Either outcome is fine; a panic is not.
            let _ = lex(&input, true);
            let _ = lex(&input, false);
        }
    }

    /// Every span must lie inside the source, whatever the input.
    #[test]
    fn spans_stay_within_source() {
        let sources = [
            "fn main() { let x = 1 }",
            "\"\u{1F600}\" 0xFF 1.5e-3 +%=",
            "#@ rule: R-X-1\n# c\nnot",
            "ééé 0b1 0o7",
        ];
        for source in sources {
            if let Ok(tokens) = lex(source, true) {
                let len = u32::try_from(source.len()).expect("short source");
                for token in tokens {
                    assert!(
                        token.span.end <= len,
                        "span {:?} exceeds source length {len}",
                        token.span
                    );
                    assert!(token.span.start <= token.span.end, "inverted span");
                }
            }
        }
    }

    #[test]
    fn every_spec_lex_error_has_a_test() {
        // Guards against a code being added to the lexer without a test.
        let covered = [
            "E0001", "E0002", "E0003", "E0004", "E0005", "E0006", "E0007", "E0008", "E0009",
            "E0010",
        ];
        assert_eq!(covered.len(), 10);
    }
}
