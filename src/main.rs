use std::env;
use std::fs;
use std::path::Path;

use rs_codeinsight::{
    analyze, analyze_with_files, collect_files, compute_freshness_digest,
    compute_freshness_digest_from_files, config, AnalyzeOptions,
};

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
        let digest_path = root_path.join(".codeinsight.digest");
        let live_digest = compute_freshness_digest(root_path);
        let cached_digest = fs::read_to_string(&digest_path).unwrap_or_default();
        if cached_digest.trim() == live_digest {
            if let Ok(content) = fs::read_to_string(&cache_path) {
                print!("{}", content);
                return;
            }
        }
        eprintln!("[codeinsight cache stale or missing; running fresh analyze]");
        let cfg = config::load_config(root_path);
        let files = collect_files(root_path, &cfg);
        let fresh_digest = compute_freshness_digest_from_files(root_path, &files);
        let output = analyze_with_files(root_path, AnalyzeOptions { json_mode }, files);
        let _ = fs::write(&cache_path, &output.text);
        let _ = fs::write(&digest_path, &fresh_digest);
        print!("{}", output.text);
        return;
    }

    let output = analyze(root_path, AnalyzeOptions { json_mode });

    println!("{}", output.text);

    if cache_mode {
        let cache_path = root_path.join(".codeinsight");
        let digest_path = root_path.join(".codeinsight.digest");
        let live_digest = compute_freshness_digest(root_path);
        if let Err(e) = fs::write(&cache_path, &output.text) {
            eprintln!("Failed to write cache: {}", e);
        }
        if let Err(e) = fs::write(&digest_path, &live_digest) {
            eprintln!("Failed to write cache digest: {}", e);
        }
    }
}
