use std::env;
use std::fs;
use std::path::Path;

use rs_codeinsight::{analyze, bm25_search, AnalyzeOptions};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let json_mode = args.iter().any(|a| a == "--json");
    let cache_mode = args.iter().any(|a| a == "--cache");
    let read_cache = args.iter().any(|a| a == "--read-cache");

    let search_query: Option<String> = args.windows(2)
        .find(|w| w[0] == "--search")
        .map(|w| w[1].clone());

    let root = args.iter()
        .find(|a| !a.starts_with("--") && !search_query.as_deref().map_or(false, |q| *a == q))
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

    if let Some(query) = search_query {
        let results = bm25_search(&output.bm25_index, &query, 10);
        println!("## Search: {}", query);
        for r in &results {
            println!("  {} (score {:.2})", r.location, r.score);
        }
        return;
    }

    println!("{}", output.text);

    if cache_mode {
        let cache_path = root_path.join(".codeinsight");
        if let Err(e) = fs::write(&cache_path, &output.text) {
            eprintln!("Failed to write cache: {}", e);
        }
    }
}
