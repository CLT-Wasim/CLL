//! Repository policy enforcement, mirroring the CI gates in AGENTS.md section 8.
//!
//! Three checks, each mapping to a numbered rule so a violation message can be
//! traced back to the document that demands it:
//!
//! * `P-LANG` (AGENTS.md section 3): no `.py`, `.c`, `.cpp`, `.h`, `.js`, `.ts`,
//!   `.sh`, `.rb`, `.go` file may live under `examples/`, `tests/`, `stdlib/` or
//!   `automation/`. The only documented exceptions are the benchmark baselines and
//!   the `PyTorch` interop tests, which live in their own repositories.
//! * `P-TEST` (AGENTS.md section 8.3): every `tests/**/*.cll` file starts with a
//!   directive header naming the SPEC rule it exercises and what is expected.
//! * `P-ERRDOC` (DECISIONS [141]): every `docs/errors/E####.md` page must be cited
//!   by at least one test, so no error code ships untested.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Directories where only CLL source may appear (AGENTS.md section 3).
const CLL_ONLY_DIRS: [&str; 4] = ["examples", "tests", "stdlib", "automation"];

/// Extensions forbidden inside [`CLL_ONLY_DIRS`].
const FORBIDDEN_EXTENSIONS: [&str; 9] = ["py", "c", "h", "cpp", "hpp", "js", "ts", "sh", "go"];

/// A single policy failure, with the rule it breaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub rule: &'static str,
    pub path: PathBuf,
    pub line: Option<usize>,
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = self.path.display();
        match self.line {
            Some(line) => write!(f, "{}: {}:{}: {}", self.rule, path, line, self.message),
            None => write!(f, "{}: {}: {}", self.rule, path, self.message),
        }
    }
}

/// Outcome of a policy run.
#[derive(Debug, Default)]
pub struct Report {
    pub violations: Vec<Violation>,
    pub files_scanned: usize,
    pub tests_scanned: usize,
    pub error_codes: usize,
}

/// Runs every policy check under `root`.
///
/// # Errors
/// Returns an error when `root` is not a directory, because silently passing on a
/// mistyped path would defeat the purpose of the gate.
pub fn check(root: &Path) -> Result<Report, String> {
    if !root.is_dir() {
        return Err(format!("`{}` is not a directory", root.display()));
    }

    let mut report = Report::default();
    let mut cited_errors: BTreeSet<String> = BTreeSet::new();

    report
        .violations
        .extend(check_language_policy(root, &mut report.files_scanned));
    report.violations.extend(check_test_headers(
        root,
        &mut report.tests_scanned,
        &mut cited_errors,
    ));
    report.error_codes = count_error_docs(root);
    report
        .violations
        .extend(check_error_docs_cited(root, &cited_errors));

    report
        .violations
        .sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
    Ok(report)
}

fn extension_of(path: &Path) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
}

fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

/// P-LANG: rejects foreign-language files in the CLL-only directories.
fn check_language_policy(root: &Path, files_scanned: &mut usize) -> Vec<Violation> {
    let mut violations = Vec::new();
    for dir in CLL_ONLY_DIRS {
        let base = root.join(dir);
        if !base.is_dir() {
            continue;
        }
        for entry in walk(&base) {
            *files_scanned += 1;
            let Some(ext) = extension_of(&entry) else {
                continue;
            };
            if FORBIDDEN_EXTENSIONS.contains(&ext) {
                violations.push(Violation {
                    rule: "P-LANG",
                    path: relative(root, &entry),
                    line: None,
                    message: format!(
                        "`{ext}` file is forbidden here; AGENTS.md section 3 requires CLL source. \
                         If CLL cannot express this, write NEEDS_FEATURE.md instead of routing around it."
                    ),
                });
            }
        }
    }
    violations
}

/// A conformance test header: `#@` lines before the first non-comment line.
#[derive(Debug, Default)]
struct Header {
    rule: Option<String>,
    expectations: Vec<String>,
    /// 1-based line numbers of `#@` directives, for diagnostics.
    directive_lines: Vec<usize>,
}

/// P-TEST: every conformance test must declare a SPEC rule and an expectation.
fn check_test_headers(
    root: &Path,
    tests_scanned: &mut usize,
    cited_errors: &mut BTreeSet<String>,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let tests_dir = root.join("tests");
    if !tests_dir.is_dir() {
        return violations;
    }

    for path in walk(&tests_dir) {
        if extension_of(&path) != Some("cll") {
            continue;
        }
        *tests_scanned += 1;
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                violations.push(Violation {
                    rule: "P-TEST",
                    path: relative(root, &path),
                    line: None,
                    message: format!("cannot read test file: {error}"),
                });
                continue;
            }
        };
        let header = parse_header(&text);
        let rel = relative(root, &path);

        match &header.rule {
            Some(_) => {}
            None => violations.push(Violation {
                rule: "P-TEST",
                path: rel.clone(),
                line: Some(1),
                message: "missing `#@ rule: <SPEC-ID>` directive; \
                          every test cites the SPEC rule it exercises (AGENTS.md section 8.3)"
                    .to_owned(),
            }),
        }

        if header.expectations.is_empty() {
            violations.push(Violation {
                rule: "P-TEST",
                path: rel,
                line: Some(1),
                message: "missing `#@ expect:` directive; a test must state its expected outcome"
                    .to_owned(),
            });
        }

        for expectation in &header.expectations {
            if let Some(code) = expected_error_code(expectation) {
                cited_errors.insert(code.to_owned());
            }
        }
    }
    violations
}

fn expected_error_code(expectation: &str) -> Option<&str> {
    let rest = expectation.strip_prefix("compile-error ")?;
    let code = rest.split_whitespace().next()?;
    let digits = code.strip_prefix('E')?;
    (digits.len() == 4 && digits.bytes().all(|byte| byte.is_ascii_digit())).then_some(code)
}

/// Reads `#@ key: value` directives from the top of a test file.
fn parse_header(text: &str) -> Header {
    let mut header = Header::default();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let Some(rest) = line.strip_prefix("#@") else {
            if line.starts_with('#') {
                continue;
            }
            break;
        };
        header.directive_lines.push(index + 1);
        let rest = rest.trim();
        let (key, value) = match rest.split_once(':') {
            Some((key, value)) => (key.trim(), value.trim()),
            None => (rest, ""),
        };
        match key {
            "rule" if !value.is_empty() => header.rule = Some(value.to_owned()),
            "expect" if !value.is_empty() => header.expectations.push(value.to_owned()),
            _ => {}
        }
    }
    header
}

/// Counts `docs/errors/E####.md` pages.
fn count_error_docs(root: &Path) -> usize {
    let dir = root.join("docs").join("errors");
    if !dir.is_dir() {
        return 0;
    }
    walk(&dir)
        .into_iter()
        .filter(|path| extension_of(path) == Some("md"))
        .count()
}

/// P-ERRDOC: every documented error code must be named in a conformance expectation.
fn check_error_docs_cited(root: &Path, cited: &BTreeSet<String>) -> Vec<Violation> {
    let dir = root.join("docs").join("errors");
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut violations = Vec::new();
    for path in walk(&dir) {
        if extension_of(&path) != Some("md") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if cited.contains(stem) {
            continue;
        }
        violations.push(Violation {
            rule: "P-ERRDOC",
            path: relative(root, &path),
            line: None,
            message: format!(
                "error code `{stem}` is not cited by any test; \
                 DECISIONS [141] requires every code to have a test"
            ),
        });
    }
    violations
}

/// Lists every regular file below `dir`, skipping dot-directories and `target/`.
fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || name == "target" {
                continue;
            }
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => stack.push(path),
                Ok(kind) if kind.is_file() => files.push(path),
                _ => {}
            }
        }
    }
    files.sort();
    files
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

    fn header_of(text: &str) -> Header {
        parse_header(text)
    }

    #[test]
    fn header_reads_rule_and_expectations() {
        let header =
            header_of("#@ rule: R-REV-014\n#@ expect: run-ok\n#@ stdout: 42\n\nfn main() {}\n");
        assert_eq!(header.rule.as_deref(), Some("R-REV-014"));
        assert_eq!(header.expectations.len(), 1);
        assert_eq!(
            header.expectations.first().map(String::as_str),
            Some("run-ok")
        );
    }

    #[test]
    fn header_stops_at_first_code_line() {
        let header = header_of("fn main() {}\n#@ rule: R-X-1\n");
        assert!(header.rule.is_none());
    }

    #[test]
    fn header_ignores_plain_comments() {
        let header = header_of("# a note\n#@ rule: R-LEX-001\n#@ expect: run-ok\n");
        assert_eq!(header.rule.as_deref(), Some("R-LEX-001"));
        assert_eq!(header.expectations.len(), 1);
    }

    #[test]
    fn language_policy_flags_python_in_examples() {
        let temp = tempdir();
        fs::create_dir_all(temp.join("examples")).unwrap_or_default();
        fs::write(temp.join("examples").join("tool.py"), "print(1)").unwrap_or_default();
        let mut scanned = 0;
        let violations = check_language_policy(&temp, &mut scanned);
        assert_eq!(violations.len(), 1);
        let first = violations.first();
        assert_eq!(first.map(|v| v.rule), Some("P-LANG"));
        assert_eq!(scanned, 1);
        fs::remove_dir_all(&temp).unwrap_or_default();
    }

    #[test]
    fn language_policy_allows_cll_source() {
        let temp = tempdir();
        fs::create_dir_all(temp.join("stdlib")).unwrap_or_default();
        fs::write(temp.join("stdlib").join("core.cll"), "").unwrap_or_default();
        let mut scanned = 0;
        assert_eq!(check_language_policy(&temp, &mut scanned), []);
        fs::remove_dir_all(&temp).unwrap_or_default();
    }

    #[test]
    fn error_doc_is_satisfied_by_expected_error_code() {
        let temp = tempdir();
        let tests = temp.join("tests").join("conformance");
        let errors = temp.join("docs").join("errors");
        fs::create_dir_all(&tests).unwrap_or_default();
        fs::create_dir_all(&errors).unwrap_or_default();
        fs::write(errors.join("E0011.md"), "# E0011\n").unwrap_or_default();
        fs::write(
            tests.join("bad.cll"),
            "#@ rule: R-EXP-090\n#@ expect: compile-error E0011\nfn bad() {}\n",
        )
        .unwrap_or_default();

        let mut scanned = 0;
        let mut cited_errors = BTreeSet::new();
        let header_violations = check_test_headers(&temp, &mut scanned, &mut cited_errors);
        assert_eq!(header_violations, []);
        assert_eq!(
            check_error_docs_cited(&temp, &cited_errors),
            [],
            "an error page is cited by the test expectation, not by its SPEC rule"
        );
        fs::remove_dir_all(&temp).unwrap_or_default();
    }

    #[test]
    fn test_header_violation_is_reported() {
        let temp = tempdir();
        fs::create_dir_all(temp.join("tests")).unwrap_or_default();
        fs::write(temp.join("tests").join("bad.cll"), "fn main() {}\n").unwrap_or_default();
        let mut scanned = 0;
        let mut cited = BTreeSet::new();
        let violations = check_test_headers(&temp, &mut scanned, &mut cited);
        assert_eq!(violations.len(), 2, "both rule and expect are missing");
        fs::remove_dir_all(&temp).unwrap_or_default();
    }

    fn tempdir() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "xtask-policy-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&base);
        let _ = fs::create_dir_all(&base);
        base
    }
}
