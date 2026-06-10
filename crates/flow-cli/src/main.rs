//! Binary entry point for the `flow` CLI.

use std::process::ExitCode;

fn main() -> ExitCode {
    let code = flow_cli::run(std::env::args_os());
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
