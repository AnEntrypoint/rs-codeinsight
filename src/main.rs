//! `codeinsight` CLI — thin front-end over the library's `analyze()` entry.
//!
//! Usage: codeinsight [PATH] [--json]
//!   PATH    directory to analyze (default: current directory)
//!   --json  emit the machine-readable JSON report instead of the compact text
//!
//! Language grammars are feature-gated in the library; build with the
//! `all-languages` feature (or a specific language feature) for the binary to
//! parse anything. With no language feature the analyzer still runs but skips
//! every file as unsupported.

use std::path::PathBuf;
use std::process::ExitCode;

use rs_codeinsight::{analyze, AnalyzeOptions};

fn main() -> ExitCode {
    let mut json_mode = false;
    let mut path: Option<PathBuf> = None;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--json" => json_mode = true,
            "-h" | "--help" => {
                eprintln!("Usage: codeinsight [PATH] [--json]");
                eprintln!("  PATH    directory to analyze (default: .)");
                eprintln!("  --json  emit the JSON report instead of compact text");
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("codeinsight: unknown flag `{}` (try --help)", other);
                return ExitCode::from(2);
            }
            other => {
                if path.is_some() {
                    eprintln!("codeinsight: unexpected extra argument `{}`", other);
                    return ExitCode::from(2);
                }
                path = Some(PathBuf::from(other));
            }
        }
    }

    let root = path.unwrap_or_else(|| PathBuf::from("."));
    if !root.is_dir() {
        eprintln!("codeinsight: `{}` is not a directory", root.display());
        return ExitCode::from(1);
    }

    let output = analyze(&root, AnalyzeOptions { json_mode });
    print!("{}", output.text);
    if !output.text.ends_with('\n') {
        println!();
    }
    for (rel, reason) in &output.skipped_files {
        eprintln!("skipped {}: {}", rel, reason);
    }
    ExitCode::SUCCESS
}
