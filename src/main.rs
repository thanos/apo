//! APO CLI entry point.

use std::process::ExitCode;

use apo::cli::Cli;
use apo::report::json_to_string;
use apo::{analyze_and_write, OutputFormat};
use clap::Parser;
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let config = match cli.into_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    match analyze_and_write(&config) {
        Ok((report, written)) => {
            match config.format {
                OutputFormat::Json => {
                    if let Ok(s) = json_to_string(&report) {
                        println!("{s}");
                    }
                }
                OutputFormat::Markdown | OutputFormat::Both => {
                    if let Some(score) = report.policy.overall_score {
                        eprintln!(
                            "apo: repository hygiene score {:.1}/100 ({} findings)",
                            score,
                            report.findings.len()
                        );
                    }
                }
            }
            for path in written {
                eprintln!("wrote {}", path.display());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
