mod analyzer;
mod depgraph;
mod formatter;
mod git;
mod lang;
mod locations;
mod models;
mod project;
mod scanner;
mod tooling;

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::Path;

use ignore::WalkBuilder;
use rayon::prelude::*;
use tree_sitter::Parser;

use analyzer::{analyze_tree, FileAnalysis};
use depgraph::{build_dep_graph, detect_dead_code};
use formatter::{format_compact, AggregatedStats, LangStats};
use lang::get_language;

const MAX_FILE_SIZE: u64 = 200 * 1024;

fn main() {
    let root = env::args().nth(1).unwrap_or_else(|| ".".into());
    let root_path = Path::new(&root);

    if !root_path.exists() {
        eprintln!("Path does not exist: {}", root);
        std::process::exit(1);
    }

    let files = collect_files(root_path);
    let all_rel_paths: Vec<String> = files.iter().map(|(r, _, _)| r.clone()).collect();

    let results: Vec<(String, String, FileAnalysis, scanner::ScanResults)> = files
        .into_par_iter()
        .filter_map(|(rel_path, abs_path, lang_name)| {
            let source = fs::read_to_string(&abs_path).ok()?;
            let ext = Path::new(&abs_path)
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();
            let lang_def = get_language(&ext)?;
            let mut parser = Parser::new();
            parser.set_language(&lang_def.language).ok()?;
            let tree = parser.parse(&source, None)?;
            let analysis = analyze_tree(&tree, &source);
            let scan = scanner::scan_source(&rel_path, &source);
            Some((rel_path, lang_name, analysis, scan))
        })
        .collect();

    let mut stats = AggregatedStats {
        files: 0,
        total_lines: 0,
        by_language: HashMap::new(),
    };
    let mut file_metrics: HashMap<String, FileAnalysis> = HashMap::new();
    let mut dep_data: HashMap<String, (HashSet<String>, HashSet<String>)> = HashMap::new();
    let mut all_func_hashes: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut all_scans = scanner::ScanResults::default();

    for (rel_path, lang_name, analysis, scan) in results {
        stats.files += 1;
        stats.total_lines += analysis.stats.lines;

        let lang_stats = stats
            .by_language
            .entry(lang_name.to_string())
            .or_insert_with(LangStats::default);
        lang_stats.files += 1;
        lang_stats.lines += analysis.stats.lines;
        lang_stats.functions += analysis.stats.functions;
        lang_stats.classes += analysis.stats.classes;
        lang_stats.complexity += analysis.stats.complexity;

        dep_data.insert(
            rel_path.clone(),
            (analysis.import_paths.clone(), analysis.exported_names.clone()),
        );

        for (sig, hash) in &analysis.func_hashes {
            all_func_hashes
                .entry(hash.clone())
                .or_default()
                .push((rel_path.clone(), sig.clone()));
        }

        all_scans.todos.extend(scan.todos);
        all_scans.fixmes.extend(scan.fixmes);
        all_scans.hacks.extend(scan.hacks);
        all_scans.security.extend(scan.security);

        file_metrics.insert(rel_path, analysis);
    }

    let dep_graph = build_dep_graph(&dep_data);
    let dead_code = detect_dead_code(&dep_graph);

    let duplicates: Vec<(String, Vec<(String, String)>)> = all_func_hashes
        .into_iter()
        .filter(|(_, instances)| instances.len() > 1)
        .map(|(hash, instances)| (hash, instances))
        .collect();

    let project_ctx = project::analyze_project(root_path);
    let git_ctx = git::analyze_git(root_path);
    let tooling_ctx = tooling::detect_tooling(root_path);
    let test_map = scanner::map_tests(&all_rel_paths);
    let data_layer = models::detect_data_layer(root_path);
    let key_locations = locations::detect_key_locations_from_paths(&all_rel_paths);

    let output = format_compact(
        &stats,
        &file_metrics,
        &dep_graph,
        &dead_code,
        &duplicates,
        &project_ctx,
        &git_ctx,
        &tooling_ctx,
        &all_scans,
        &test_map,
        &data_layer,
        &key_locations,
    );
    println!("{}", output);
}

fn collect_files(root: &Path) -> Vec<(String, String, String)> {
    let mut files = Vec::new();

    let walker = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                "node_modules" | ".git" | "dist" | "build" | "target"
                    | ".next" | ".nuxt" | "coverage" | "__pycache__"
                    | ".venv" | "vendor" | ".cache" | ".output"
            )
        })
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if let Ok(meta) = path.metadata() {
            if meta.len() > MAX_FILE_SIZE {
                continue;
            }
        }

        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();

        if let Some(lang_def) = get_language(&ext) {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            let abs = path.to_string_lossy().to_string();
            files.push((rel, abs, lang_def.name.to_string()));
        }
    }

    files
}
