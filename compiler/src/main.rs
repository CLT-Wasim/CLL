use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        eprintln!("usage: cllc run <source.cll>");
        return ExitCode::from(2);
    };
    if command != "run" {
        eprintln!("error: unsupported command `{command}`; expected `run`");
        return ExitCode::from(2);
    }
    let Some(path) = args.next() else {
        eprintln!("error: `run` requires a .cll source path");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("error: too many arguments");
        return ExitCode::from(2);
    }
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read `{path}`: {error}");
            return ExitCode::FAILURE;
        }
    };
    match cllc::interpreter::run(&source) {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
