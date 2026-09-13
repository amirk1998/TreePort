use std::io::{self, Write};
use std::process::ExitCode;

use treeport::cli::Config;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match Config::parse(&args).and_then(treeport::run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            let _ = writeln!(io::stderr(), "{msg}");
            ExitCode::FAILURE
        }
    }
}
