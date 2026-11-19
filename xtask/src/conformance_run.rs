//! The conformance runner: `cargo xtask test`.
//!
//! It walks `tests/conformance/`, parses each header, and checks the expectation
//! against what the compiler currently does. The runner is honest by design: a
//! test whose expectation cannot yet be evaluated is reported as **not
//! implemented**, never as a pass. A suite that passes because nothing runs is
//! worse than no suite (AGENTS.md section 9).
//!
//! # What "not implemented" means here
//!
//! The frontend exists (lexer), the parser does not. So today the runner can
//! only evaluate tests whose expectation is decided by lexing alone: the
//! `compile-error E00xx` lexical cases. Every other test is counted as
//! `unimplemented`, printed in a separate bucket, and makes the command exit
//! non-zero unless `--allow-unimplemented` is passed. The number is visible in
//! the summary so it cannot be quietly forgotten.

use std::fmt::Write as _;

use cllc::conformance::{self, Expect};
use cllc::lexer;

/// What happened to one test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The compiler behaved exactly as the header required.
    Pass,
    /// The compiler disagreed with the header.
    Fail { detail: String },
    /// The expectation needs a compiler stage that does not exist yet.
    Unimplemented { missing: &'static str },
}

/// The result of running one test.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub rule: String,
    pub expect: Expect,
    pub outcome: Outcome,
}

/// Aggregated results.
#[derive(Debug, Default, Clone)]
pub struct Summary {
    pub passed: usize,
    pub failed: usize,
    pub unimplemented: usize,
    pub results: Vec<TestResult>,
}

impl Summary {
    /// True when nothing failed and (optionally) nothing was unimplemented.
    #[must_use]
    pub const fn is_success(&self, allow_unimplemented: bool) -> bool {
        self.failed == 0 && (allow_unimplemented || self.unimplemented == 0)
    }

    /// Human-readable report, one line per non-passing test plus a summary.
    #[must_use]
    pub fn report(&self) -> String {
        let mut out = String::new();
        for result in &self.results {
            match &result.outcome {
                Outcome::Pass => {}
                Outcome::Fail { detail } => {
                    let _ = writeln!(
                        out,
                        "FAIL  {}  [{}]  expected {}: {detail}",
                        result.name, result.rule, result.expect
                    );
                }
                Outcome::Unimplemented { missing } => {
                    let _ = writeln!(
                        out,
                        "TODO  {}  [{}]  needs {missing}",
                        result.name, result.rule
                    );
                }
            }
        }
        if !out.is_empty() {
            out.push('\n');
        }
        let _ = writeln!(
            out,
            "conformance: {} passed, {} failed, {} not implemented (of {})",
            self.passed,
            self.failed,
            self.unimplemented,
            self.results.len()
        );
        out
    }
}

/// Runs every conformance test under `root`.
///
/// # Errors
/// Returns a message when the suite directory is missing or unreadable.
pub fn run(root: &std::path::Path) -> Result<Summary, String> {
    let cases = conformance::discover(root)?;
    let mut summary = Summary::default();
    for case in cases {
        let result = run_one(&case);
        match result.outcome {
            Outcome::Pass => summary.passed += 1,
            Outcome::Fail { .. } => summary.failed += 1,
            Outcome::Unimplemented { .. } => summary.unimplemented += 1,
        }
        summary.results.push(result);
    }
    Ok(summary)
}

fn run_one(case: &conformance::TestCase) -> TestResult {
    let fail = |detail: String| TestResult {
        name: case.name.clone(),
        rule: String::new(),
        expect: Expect::CompileOk,
        outcome: Outcome::Fail { detail },
    };

    let header = match case.header() {
        Ok(header) => header,
        Err(errors) => {
            let detail = errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            return fail(format!("malformed header: {detail}"));
        }
    };

    // The body starts after the directive block; the lexer must not see the
    // directives as `#@` tokens when the file is not in tests/ - but it is, so
    // directives are legal. We lex the whole file so the spans line up with the
    // editor's line numbers.
    let lexed = lexer::lex(&case.source, true);

    let outcome = match &header.expect {
        Expect::CompileError(code) if is_lexical(code) => match &lexed {
            Ok(_) => Outcome::Fail {
                detail: format!("expected `{code}` but lexing succeeded"),
            },
            Err(diagnostics) => {
                if diagnostics.iter().any(|d| d.code == code) {
                    Outcome::Pass
                } else {
                    let found: Vec<&str> = diagnostics.iter().map(|d| d.code).collect();
                    Outcome::Fail {
                        detail: format!("expected `{code}` but got {found:?}"),
                    }
                }
            }
        },
        Expect::CompileError(_) => {
            // A lexically valid file: the diagnostic must come from a later
            // stage. If lexing already fails, that is a genuine failure.
            match &lexed {
                Ok(_) => Outcome::Unimplemented {
                    missing: "parser and checker",
                },
                Err(diagnostics) => {
                    let found: Vec<&str> = diagnostics.iter().map(|d| d.code).collect();
                    Outcome::Fail {
                        detail: format!(
                            "expected a later-stage error but lexing failed: {found:?}"
                        ),
                    }
                }
            }
        }
        Expect::CompileOk => match &lexed {
            Ok(_) => Outcome::Unimplemented {
                missing: "parser and checker",
            },
            Err(diagnostics) => {
                let found: Vec<&str> = diagnostics.iter().map(|d| d.code).collect();
                Outcome::Fail {
                    detail: format!("expected valid source but lexing failed: {found:?}"),
                }
            }
        },
        Expect::RunOk { .. } => match &lexed {
            Ok(_) => Outcome::Unimplemented {
                missing: "parser, checker, and interpreter",
            },
            Err(diagnostics) => {
                let found: Vec<&str> = diagnostics.iter().map(|d| d.code).collect();
                Outcome::Fail {
                    detail: format!("expected a runnable program but lexing failed: {found:?}"),
                }
            }
        },
    };

    TestResult {
        name: case.name.clone(),
        rule: header.rule,
        expect: header.expect,
        outcome,
    }
}

/// Lexical codes: exactly `E0001`..=`E0010` (SPEC section 11). Everything from
/// `E0020` up needs a stage after the lexer, so it cannot be evaluated yet.
fn is_lexical(code: &str) -> bool {
    let Some(digits) = code.strip_prefix('E') else {
        return false;
    };
    digits.len() == 4
        && digits.bytes().all(|b| b.is_ascii_digit())
        && digits.parse::<u16>().is_ok_and(|n| (1..=10).contains(&n))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn lexical_codes_are_recognised() {
        assert!(is_lexical("E0001"));
        assert!(is_lexical("E0010"));
        assert!(!is_lexical("E0000"), "there is no E0000");
        assert!(!is_lexical("E0011"), "E0011 is not a lexer code");
        assert!(!is_lexical("E0034"), "E0034 is a mutability error");
        assert!(!is_lexical("E0100"));
        assert!(!is_lexical("E0140"));
        assert!(!is_lexical("E14"));
    }

    #[test]
    fn success_requires_no_failures() {
        let summary = Summary {
            passed: 3,
            failed: 1,
            unimplemented: 5,
            results: Vec::new(),
        };
        assert!(!summary.is_success(true));
        assert!(!summary.is_success(false));
    }

    #[test]
    fn unimplemented_blocks_success_unless_allowed() {
        let summary = Summary {
            passed: 3,
            failed: 0,
            unimplemented: 5,
            results: Vec::new(),
        };
        assert!(summary.is_success(true));
        assert!(!summary.is_success(false), "TODO is not a pass");
    }

    #[test]
    fn report_names_every_non_passing_test() {
        let summary = Summary {
            passed: 0,
            failed: 1,
            unimplemented: 1,
            results: vec![
                TestResult {
                    name: "tests/conformance/rev/a.cll".to_owned(),
                    rule: "R-REV-030".to_owned(),
                    expect: Expect::CompileOk,
                    outcome: Outcome::Fail {
                        detail: "boom".to_owned(),
                    },
                },
                TestResult {
                    name: "tests/conformance/rev/b.cll".to_owned(),
                    rule: "R-REV-040".to_owned(),
                    expect: Expect::CompileOk,
                    outcome: Outcome::Unimplemented { missing: "parser" },
                },
            ],
        };
        let report = summary.report();
        assert!(
            report.contains(
                "FAIL  tests/conformance/rev/a.cll  [R-REV-030]  expected compile-ok: boom"
            ),
            "{report}"
        );
        assert!(report.contains("TODO  tests/conformance/rev/b.cll  [R-REV-040]  needs parser"));
        assert!(report.contains("0 passed, 1 failed, 1 not implemented (of 2)"));
    }
}
