//! `xtask`: repository automation for the CLL project.
//!
//! Invoked as `cargo xtask <command>`. Zero third-party dependencies by design
//! (AGENTS.md section 3: compiler tooling dependencies stay minimal and pinned).

mod policy;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
xtask - CLL repository automation

USAGE:
    cargo xtask <COMMAND> [OPTIONS]

COMMANDS:
    policy [--root <DIR>]
        Enforce the repository rules that CI gates on (AGENTS.md sections 3, 8, 9):
          1. no foreign-language source under examples/, tests/, stdlib/, automation/
          2. every conformance test declares a SPEC rule id and an expectation
          3. every error code in docs/errors/ is cited by at least one test

    help
        Print this message.

OPTIONS:
    --root <DIR>   Repository root to check (default: current directory)

EXIT STATUS:
    0  all checks passed
    1  one or more policy violations
    2  usage error
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut command: Option<String> = None;
    let mut root: PathBuf = PathBuf::from(".");
    let mut index = 0;

    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--root" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("error: --root needs a directory\n\n{USAGE}");
                    return ExitCode::from(2);
                };
                root = PathBuf::from(value);
                index += 2;
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown option `{other}`\n\n{USAGE}");
                return ExitCode::from(2);
            }
            other => {
                if command.is_some() {
                    eprintln!("error: unexpected extra argument `{other}`\n\n{USAGE}");
                    return ExitCode::from(2);
                }
                command = Some(other.to_owned());
                index += 1;
            }
        }
    }

    match command.as_deref() {
        Some("policy") => run_policy(&root),
        Some("help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("error: unknown command `{other}`\n\n{USAGE}");
            ExitCode::from(2)
        }
        None => {
            println!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run_policy(root: &std::path::Path) -> ExitCode {
    match policy::check(root) {
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
        Ok(report) => {
            for violation in &report.violations {
                eprintln!("{violation}");
            }
            if report.violations.is_empty() {
                println!(
                    "policy: ok ({} files scanned, {} conformance tests, {} error codes)",
                    report.files_scanned, report.tests_scanned, report.error_codes
                );
                ExitCode::SUCCESS
            } else {
                eprintln!(
                    "\npolicy: FAILED with {} violation(s).\n\
                     Fix each one or, if the rule itself is wrong, write an RFC in rfcs/ \
                     (AGENTS.md section 7).",
                    report.violations.len()
                );
                ExitCode::FAILURE
            }
        }
    }
}
