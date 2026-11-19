//! Conformance-test discovery and the directive header (SPEC R-LEX-012).
//!
//! A conformance test is a `.cll` file in `tests/conformance/` whose first lines
//! are `#@ key: value` directives. The header is a contract: `rule` says which
//! SPEC rule the test exercises and `expect` says what must happen. A test with
//! no header is not a test, it is a program someone forgot to delete, and
//! `cargo xtask policy` rejects it.
//!
//! This module is the single reader of that format, shared by `xtask policy`
//! (which validates headers) and `xtask test` (which runs the suite). Two
//! implementations of the same format would drift.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// What a test expects to happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expect {
    /// The program must lex, parse, and check with no diagnostics.
    CompileOk,
    /// The program must be rejected with exactly this error code.
    CompileError(String),
    /// The program must compile and print exactly these bytes on stdout.
    RunOk { stdout: String },
}

impl fmt::Display for Expect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CompileOk => write!(f, "compile-ok"),
            Self::CompileError(code) => write!(f, "compile-error {code}"),
            Self::RunOk { stdout } => write!(f, "run-ok (stdout: {stdout:?})"),
        }
    }
}

/// A parsed `#@` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The SPEC rule id this test exercises, e.g. `R-REV-030`.
    pub rule: String,
    pub expect: Expect,
    /// Every directive, including ones this module does not interpret, so an
    /// unknown directive is reported instead of silently ignored.
    pub directives: BTreeMap<String, String>,
    /// 1-based line of the last header line; the body starts after it.
    pub body_line: usize,
}

/// A problem found in a header. Codes match the policy gate's `P-TEST-*` names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "header:{}: {}", self.line, self.message)
    }
}

const KNOWN_DIRECTIVES: [&str; 4] = ["rule", "expect", "inverse", "stdout"];

/// Parses the `#@` header from the start of `source`.
///
/// # Errors
/// Returns one [`HeaderError`] per malformed or missing directive.
pub fn parse_header(source: &str) -> Result<Header, Vec<HeaderError>> {
    let (directives, body_line, mut errors) = collect_directives(source);

    let rule = directives.get("rule").cloned().unwrap_or_else(|| {
        errors.push(HeaderError {
            line: 1,
            message: "missing `#@ rule: <SPEC-ID>` (AGENTS.md section 8.3)".to_owned(),
        });
        String::new()
    });

    if !rule.is_empty() && !is_spec_rule_id(&rule) {
        errors.push(HeaderError {
            line: 1,
            message: format!(
                "`{rule}` is not a SPEC rule id; expected `R-<AREA>-<NNN>`, e.g. `R-REV-030`"
            ),
        });
    }

    let expect = if let Some(text) = directives.get("expect") {
        match parse_expect(text, &directives) {
            Ok(expect) => expect,
            Err(message) => {
                errors.push(HeaderError { line: 1, message });
                Expect::CompileOk
            }
        }
    } else {
        errors.push(HeaderError {
            line: 1,
            message: "missing `#@ expect:`; a test must state its outcome".to_owned(),
        });
        Expect::CompileOk
    };

    if errors.is_empty() {
        Ok(Header {
            rule,
            expect,
            directives,
            body_line,
        })
    } else {
        Err(errors)
    }
}

/// Reads the contiguous `#@` block at the top of the file.
fn collect_directives(source: &str) -> (BTreeMap<String, String>, usize, Vec<HeaderError>) {
    let mut directives = BTreeMap::new();
    let mut errors = Vec::new();
    let mut body_line = 0;

    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        let Some(rest) = raw.strip_prefix("#@") else {
            break;
        };
        body_line = line_number;
        let rest = rest.trim();
        let Some((key, value)) = rest.split_once(':') else {
            errors.push(HeaderError {
                line: line_number,
                message: format!("directive `#@ {rest}` has no `:`; write `#@ key: value`"),
            });
            continue;
        };
        let key = key.trim().to_owned();
        let value = value.trim().to_owned();
        if !KNOWN_DIRECTIVES.contains(&key.as_str()) {
            errors.push(HeaderError {
                line: line_number,
                message: format!(
                    "unknown directive `{key}`; known directives are {}",
                    KNOWN_DIRECTIVES.join(", ")
                ),
            });
            continue;
        }
        if directives.insert(key.clone(), value).is_some() {
            errors.push(HeaderError {
                line: line_number,
                message: format!("directive `{key}` appears twice"),
            });
        }
    }

    (directives, body_line, errors)
}

fn parse_expect(text: &str, directives: &BTreeMap<String, String>) -> Result<Expect, String> {
    match text {
        "compile-ok" => Ok(Expect::CompileOk),
        "run-ok" => {
            let stdout = directives.get("stdout").cloned().ok_or_else(|| {
                "`#@ expect: run-ok` needs `#@ stdout:` with the exact expected output".to_owned()
            })?;
            Ok(Expect::RunOk { stdout })
        }
        other => {
            let Some(code) = other.strip_prefix("compile-error ") else {
                return Err(format!(
                    "unknown expectation `{other}`; expected `compile-ok`, `run-ok`, or \
                     `compile-error <CODE>`"
                ));
            };
            if !is_error_code(code.trim()) {
                return Err(format!(
                    "`{}` is not an error code; expected `E` followed by four digits",
                    code.trim()
                ));
            }
            Ok(Expect::CompileError(code.trim().to_owned()))
        }
    }
}

/// `R-<AREA>-<NNN>` where AREA is upper-case ASCII letters.
#[must_use]
pub fn is_spec_rule_id(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("R-") else {
        return false;
    };
    let Some((area, number)) = rest.rsplit_once('-') else {
        return false;
    };
    !area.is_empty()
        && area.bytes().all(|b| b.is_ascii_uppercase())
        && number.len() == 3
        && number.bytes().all(|b| b.is_ascii_digit())
}

/// `E` followed by exactly four digits.
#[must_use]
pub fn is_error_code(text: &str) -> bool {
    text.strip_prefix('E')
        .is_some_and(|digits| digits.len() == 4 && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// Loads every conformance test under `root/tests/conformance`.
///
/// # Errors
/// Returns a message when the directory cannot be read, so a missing suite is a
/// failure rather than an empty pass.
pub fn discover(root: &Path) -> Result<Vec<TestCase>, String> {
    let dir = root.join("tests").join("conformance");
    if !dir.is_dir() {
        return Err(format!(
            "{}: conformance suite directory is missing",
            dir.display()
        ));
    }
    let mut paths = Vec::new();
    collect_cll_files(&dir, &mut paths).map_err(|err| format!("{}: {err}", dir.display()))?;
    paths.sort();

    let mut tests = Vec::new();
    for path in paths {
        let source =
            std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        tests.push(TestCase {
            name: relative,
            source,
            path,
        });
    }
    Ok(tests)
}

fn collect_cll_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_cll_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "cll") {
            out.push(path);
        }
    }
    Ok(())
}

/// One conformance test file.
#[derive(Debug, Clone)]
pub struct TestCase {
    /// Path relative to the repository root, used in output.
    pub name: String,
    pub source: String,
    pub path: std::path::PathBuf,
}

impl TestCase {
    /// Parses this test's header.
    ///
    /// # Errors
    /// Propagates every header problem.
    pub fn header(&self) -> Result<Header, Vec<HeaderError>> {
        parse_header(&self.source)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_complete_header() {
        let header = parse_header(
            "#@ rule: R-REV-030\n#@ expect: compile-error E0146\n#@ inverse: mix\nfn x() {}\n",
        )
        .expect("valid header");
        assert_eq!(header.rule, "R-REV-030");
        assert_eq!(header.expect, Expect::CompileError("E0146".to_owned()));
        assert_eq!(header.body_line, 3, "three directives precede the body");
        assert_eq!(
            header.directives.get("inverse").map(String::as_str),
            Some("mix")
        );
    }

    #[test]
    fn run_ok_requires_stdout() {
        let err = parse_header("#@ rule: R-EXP-001\n#@ expect: run-ok\n").expect_err("no stdout");
        assert!(
            err.iter().any(|e| e.message.contains("needs `#@ stdout:`")),
            "{err:?}"
        );
    }

    #[test]
    fn missing_rule_is_reported() {
        let err = parse_header("#@ expect: compile-ok\n").expect_err("no rule");
        assert!(
            err.iter().any(|e| e.message.contains("missing `#@ rule:")),
            "{err:?}"
        );
    }

    #[test]
    fn missing_expect_is_reported() {
        let err = parse_header("#@ rule: R-LEX-001\n").expect_err("no expect");
        assert!(
            err.iter()
                .any(|e| e.message.contains("missing `#@ expect:")),
            "{err:?}"
        );
    }

    #[test]
    fn unknown_directive_is_rejected() {
        let err = parse_header("#@ rule: R-LEX-001\n#@ expect: compile-ok\n#@ flavour: salt\n")
            .expect_err("unknown directive");
        assert!(
            err.iter()
                .any(|e| e.message.contains("unknown directive `flavour`")),
            "{err:?}"
        );
    }

    #[test]
    fn malformed_rule_id_is_rejected() {
        for bad in ["REV-030", "R-rev-030", "R-REV-30", "R-REV-0300", "R--030"] {
            let err = parse_header(&format!("#@ rule: {bad}\n#@ expect: compile-ok\n"))
                .expect_err("bad rule id");
            assert!(
                err.iter().any(|e| e.message.contains("not a SPEC rule id")),
                "{bad}: {err:?}"
            );
        }
    }

    #[test]
    fn malformed_error_code_is_rejected() {
        let err = parse_header("#@ rule: R-LEX-001\n#@ expect: compile-error E14\n")
            .expect_err("bad code");
        assert!(
            err.iter().any(|e| e.message.contains("not an error code")),
            "{err:?}"
        );
    }

    #[test]
    fn plain_comments_after_the_header_are_not_directives() {
        let header =
            parse_header("#@ rule: R-LEX-001\n#@ expect: compile-ok\n# this is prose\nfn x() {}\n")
                .expect("valid");
        assert_eq!(header.body_line, 2);
    }

    #[test]
    fn spec_rule_id_shape() {
        assert!(is_spec_rule_id("R-REV-030"));
        assert!(is_spec_rule_id("R-CAP-041"));
        assert!(!is_spec_rule_id("R-rev-030"));
        assert!(!is_spec_rule_id("R-REV-0301"));
    }
}
