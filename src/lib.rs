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
use formatter::AggregatedStats;
use lang::get_language;

pub struct AnalyzeOptions {
    pub json_mode: bool,
}

pub struct AnalysisOutput {
    pub text: String,
    pub skipped_files: Vec<(String, String)>,
}

pub fn analyze(root: &Path, options: AnalyzeOptions) -> AnalysisOutput {
    let cfg = config::load_config(root);
    let (all_files, collect_skips) = collect_all_files(root, &cfg);
    let files = filter_supported(&all_files);
    analyze_files(root, options, files, all_files, collect_skips)
}

fn filter_supported(all_files: &[(String, String)]) -> Vec<(String, String, String)> {
    all_files
        .iter()
        .filter_map(|(rel, abs)| {
            let ext = Path::new(abs).extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
            get_language(&ext).map(|lang_def| (rel.clone(), abs.clone(), lang_def.name.to_string()))
        })
        .collect()
}

fn analyze_files(root: &Path, options: AnalyzeOptions, files: Vec<(String, String, String)>, all_files: Vec<(String, String)>, precollect_skips: Vec<(String, String)>) -> AnalysisOutput {
    let all_rel_paths: Vec<String> = files.iter().map(|(r, _, _)| r.clone()).collect();
    let data_layer_files: Vec<(String, String)> = all_files;

    let outcomes: Vec<Result<(String, String, FileAnalysis, scanner::ScanResults), (String, String)>> = files
        .into_par_iter()
        .map(|(rel_path, abs_path, lang_name)| {
            let source = match fs::read_to_string(&abs_path) {
                Ok(s) => s,
                Err(_) => return Err((rel_path, "invalid UTF-8 or unreadable".to_string())),
            };
            let ext = Path::new(&abs_path)
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();
            let lang_def = match get_language(&ext) {
                Some(l) => l,
                None => return Err((rel_path, "unsupported language".to_string())),
            };
            let mut parser = Parser::new();
            if parser.set_language(&lang_def.language).is_err() {
                return Err((rel_path, "parser init failed".to_string()));
            }
            let tree = match parser.parse(&source, None) {
                Some(t) => t,
                None => return Err((rel_path, "parse failed".to_string())),
            };
            let analysis = analyze_tree(&tree, &source);
            let scan = scanner::scan_source(&rel_path, &source);
            Ok((rel_path, lang_name, analysis, scan))
        })
        .collect();

    let mut skipped_files: Vec<(String, String)> = precollect_skips;
    let results: Vec<(String, String, FileAnalysis, scanner::ScanResults)> = outcomes
        .into_iter()
        .filter_map(|outcome| match outcome {
            Ok(v) => Some(v),
            Err(skip) => {
                skipped_files.push(skip);
                None
            }
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
        let ls = stats.by_language.entry(lang_name.clone()).or_default();
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
    let data_layer = models::detect_data_layer(root, &data_layer_files);
    let key_locations = locations::detect_key_locations_from_paths(&all_rel_paths);
    let conv = conventions::detect_conventions(&file_metrics, &file_languages);

    let mut text = if options.json_mode {
        json_output::format_json(&stats, &file_metrics, &dep_graph, &dead_code, &duplicates, &project_ctx, &git_ctx, &tooling_ctx, &all_scans, &test_map, &data_layer, &key_locations, &conv, &skipped_files)
    } else {
        formatter::format_compact(&stats, &file_metrics, &dep_graph, &dead_code, &duplicates, &project_ctx, &git_ctx, &tooling_ctx, &all_scans, &test_map, &data_layer, &key_locations, &conv)
    };

    if !skipped_files.is_empty() && !options.json_mode {
        text.push_str(&format!("\n**Skipped:** {} file(s) could not be analyzed:\n", skipped_files.len()));
        for (path, reason) in skipped_files.iter().take(20) {
            text.push_str(&format!("- {} ({})\n", path, reason));
        }
        if skipped_files.len() > 20 {
            text.push_str(&format!("- and {} more\n", skipped_files.len() - 20));
        }
    }

    AnalysisOutput { text, skipped_files }
}

pub fn collect_all_files(root: &Path, config: &config::Config) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let mut files = Vec::new();
    let mut skipped: Vec<(String, String)> = Vec::new();
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
            if name.starts_with(".plugkit-browser-profile") {
                return false;
            }
            if entry.file_type().is_some_and(|t| t.is_dir()) {
                if matches!(name.as_ref(), "node_modules" | ".git" | "dist" | "build" | "target" | ".next" | ".nuxt" | "coverage" | "__pycache__" | ".venv" | "vendor" | ".cache" | ".output" | ".gm") {
                    return false;
                }
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
        let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if file_name.starts_with("._") { continue; }
        if matches_ignore_pattern(&file_name, &ignore_files) { continue; }
        let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");
        match path.metadata() {
            Ok(meta) => {
                if meta.len() > max_file_size {
                    skipped.push((rel, format!("exceeds max_file_size ({} bytes > {} bytes)", meta.len(), max_file_size)));
                    continue;
                }
            }
            Err(e) => {
                skipped.push((rel, format!("metadata unreadable: {e}")));
                continue;
            }
        }
        let abs = path.to_string_lossy().to_string();
        files.push((rel, abs));
    }
    (files, skipped)
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

