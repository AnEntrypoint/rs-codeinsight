use std::env;
use std::fs;
use std::path::Path;

use rs_codeinsight::{analyze, AnalyzeOptions};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let json_mode = args.iter().any(|a| a == "--json");
    let cache_mode = args.iter().any(|a| a == "--cache");
    let read_cache = args.iter().any(|a| a == "--read-cache");

    let root = args.iter()
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| ".".into());

    let root_path = Path::new(&root);

    if !root_path.exists() {
        eprintln!("Path does not exist: {}", root);
        std::process::exit(1);
    }

    if read_cache {
        let cache_path = root_path.join(".codeinsight");
        match fs::read_to_string(&cache_path) {
            Ok(content) => { print!("{}", content); return; }
            Err(_) => { eprintln!("No cache file found at {}", cache_path.display()); std::process::exit(1); }
        }
    }

    let output = analyze(root_path, AnalyzeOptions { json_mode });

    println!("{}", output.text);

    if cache_mode {
        let cache_path = root_path.join(".codeinsight");
        if let Err(e) = fs::write(&cache_path, &output.text) {
            eprintln!("Failed to write cache: {}", e);
        }
    }
}
