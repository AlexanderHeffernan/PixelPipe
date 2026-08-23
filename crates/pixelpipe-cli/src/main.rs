mod args;
mod execution;

use std::process::ExitCode;

use args::Cli;
use clap::Parser;
use serde::Serialize;
use serde_json::json;

fn main() -> ExitCode {
    match execution::run(Cli::parse()) {
        Ok(value) => {
            print_json(&value);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": false,
                    "error": error.to_string(),
                }))
                .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"serialization failure\"}".to_owned())
            );
            ExitCode::FAILURE
        }
    }
}

fn print_json(value: &impl Serialize) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("CLI response must serialize")
    );
}
