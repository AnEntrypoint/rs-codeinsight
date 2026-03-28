pub mod analyzer;
pub mod config;
pub mod conventions;
pub mod depgraph;
pub mod formatter;
pub mod git;
pub mod json_output;
pub mod lang;
pub mod locations;
pub mod models;
pub mod project;
pub mod scanner;
pub mod tooling;

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::fs;

use ignore::WalkBuilder;
use rayon::prelude::*;
use tree_sitter::Parser;

use analyzer::{analyze_tree, FileAnalysis};
use formatter::{AggregatedStats, LangStats};
use lang::get_language;

pub struct AnalyzeOptions {
    pub json_mode: bool,
}

pub struct AnalysisOutput {
    pub text: String,
}

pub fn analyze(root: &Path, options: AnalyzeOptions) -> AnalysisOutput {
    let cfg = config::load_config(root);
    let files = collect_files(root, &cfg);
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

    let mut stats = AggregatedStats { files: 0, total_lines: 0, by_language: HashMap::new() };
    let mut file_metrics: HashMap<String, FileAnalysis> = HashMap::new();
    let mut dep_data: HashMap<String, (HashSet<String>, HashSet<String>)> = HashMap::new();
    let mut all_func_hashes: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut all_scans = scanner::ScanResults::default();
    let mut file_languages: HashMap<String, String> = HashMap::new();

    for (rel_path, lang_name, analysis, scan) in results {
        stats.files += 1;
        stats.total_lines += analysis.stats.lines;
        let ls = stats.by_language.entry(lang_name.clone()).or_insert_with(LangStats::default);
        ls.files += 1;
        ls.lines += analysis.stats.lines;
        ls.functions += analysis.stats.functions;
        ls.classes += analysis.stats.classes;
        ls.complexity += analysis.stats.complexity;
        dep_data.insert(rel_path.clone(), (analysis.import_paths.clone(), analysis.exported_names.clone()));
        for (sig, hash) in &analysis.func_hashes {
            all_func_hashes.entry(hash.clone()).or_default().push((rel_path.clone(), sig.clone()));
        }
        all_scans.todos.extend(scan.todos);
        all_scans.fixmes.extend(scan.fixmes);
        all_scans.hacks.extend(scan.hacks);
        all_scans.security.extend(scan.security);
        file_languages.insert(rel_path.clone(), lang_name);
        file_metrics.insert(rel_path, analysis);
    }

    let project_ctx = project::analyze_project(root);
    let path_aliases = project::parse_tsconfig_paths(root);
    let dep_graph = depgraph::build_dep_graph(&dep_data, &path_aliases, &project_ctx.go_modules);
    let dead_code = depgraph::detect_dead_code(&dep_graph);
    let duplicates: Vec<(String, Vec<(String, String)>)> = all_func_hashes
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .collect();
    let git_ctx = git::analyze_git(root);
    let tooling_ctx = tooling::detect_tooling(root);
    let test_map = scanner::map_tests(&all_rel_paths);
    let data_layer = models::detect_data_layer(root);
    let key_locations = locations::detect_key_locations_from_paths(&all_rel_paths);
    let conv = conventions::detect_conventions(&file_metrics, &file_languages);

    let text = if options.json_mode {
        json_output::format_json(&stats, &file_metrics, &dep_graph, &dead_code, &duplicates, &project_ctx, &git_ctx, &tooling_ctx, &all_scans, &test_map, &data_layer, &key_locations, &conv)
    } else {
        formatter::format_compact(&stats, &file_metrics, &dep_graph, &dead_code, &duplicates, &project_ctx, &git_ctx, &tooling_ctx, &all_scans, &test_map, &data_layer, &key_locations, &conv)
    };

    AnalysisOutput { text }
}

pub fn collect_files(root: &Path, config: &config::Config) -> Vec<(String, String, String)> {
    let mut files = Vec::new();
    let max_file_size = config.max_file_size;
    let extra_ignore_dirs: Vec<String> = config.ignore_dirs.clone();
    let ignore_files: Vec<String> = config.ignore_files.clone();

    let walker = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .filter_entry(move |entry| {
            let name = entry.file_name().to_string_lossy();
            if matches!(name.as_ref(), "node_modules" | ".git" | "dist" | "build" | "target" | ".next" | ".nuxt" | "coverage" | "__pycache__" | ".venv" | "vendor" | ".cache" | ".output") {
                return false;
            }
            if entry.path().is_dir() {
                for dir in &extra_ignore_dirs {
                    if name.as_ref() == dir.as_str() { return false; }
                }
            }
            true
        })
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }
        if let Ok(meta) = path.metadata() {
            if meta.len() > max_file_size { continue; }
        }
        let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if matches_ignore_pattern(&file_name, &ignore_files) { continue; }
        let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
        if let Some(lang_def) = get_language(&ext) {
            let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");
            let abs = path.to_string_lossy().to_string();
            files.push((rel, abs, lang_def.name.to_string()));
        }
    }
    files
}

pub fn matches_ignore_pattern(file_name: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if pattern.starts_with("*.") {
            if file_name.ends_with(&pattern[1..]) { return true; }
        } else if pattern == file_name {
            return true;
        }
    }
    false
}
