use rex::diagnostics::Severity;
use rex::{lexer, parser};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return ExitCode::from(2);
    };
    let Some(path) = args.next() else {
        eprintln!("missing source file path");
        print_usage();
        return ExitCode::from(2);
    };

    if args.next().is_some() {
        eprintln!("too many arguments");
        print_usage();
        return ExitCode::from(2);
    }

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read `{path}`: {error}");
            return ExitCode::from(1);
        }
    };

    match command.as_str() {
        "lex" => {
            let output = lexer::lex(&source);
            println!("{output}");
            exit_for_diagnostics(&output.diagnostics)
        }
        "parse" => {
            let output = parser::parse(&source);
            println!("diagnostics:");
            if output.diagnostics.is_empty() {
                println!("  none");
            } else {
                for diagnostic in &output.diagnostics {
                    println!("  {diagnostic:?}");
                }
            }
            println!("ast:");
            println!("{:#?}", output.ast);
            exit_for_diagnostics(&output.diagnostics)
        }
        _ => {
            eprintln!("unknown command `{command}`");
            print_usage();
            ExitCode::from(2)
        }
    }
}

fn exit_for_diagnostics(diagnostics: &[rex::diagnostics::Diagnostic]) -> ExitCode {
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  rex lex <source>");
    eprintln!("  rex parse <source>");
}
