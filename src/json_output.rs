use std::collections::HashMap;
use crate::analyzer::FileAnalysis;
use crate::depgraph::{DeadCode, DepGraph};
use crate::git::GitContext;
use crate::lang::lang_abbrev;
use crate::locations::KeyLocations;
use crate::models::DataLayer;
use crate::project::ProjectContext;
use crate::scanner::{ScanResults, TestMap};
use crate::tooling::ToolingContext;
use crate::formatter::AggregatedStats;

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out
}

fn json_str(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

fn json_str_opt(s: &Option<String>) -> String {
    match s {
        Some(v) => json_str(v),
        None => "null".to_string(),
    }
}

fn json_str_array(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|s| json_str(s)).collect();
    format!("[{}]", parts.join(", "))
}

pub fn format_json(
    stats: &AggregatedStats,
    file_metrics: &HashMap<String, FileAnalysis>,
    dep_graph: &DepGraph,
    dead_code: &DeadCode,
    duplicates: &[(String, Vec<(String, String)>)],
    project: &ProjectContext,
    git: &GitContext,
    _tooling: &ToolingContext,
    scans: &ScanResults,
    test_map: &TestMap,
    data_layer: &DataLayer,
    key_locations: &KeyLocations,
) -> String {
    let mut out = String::with_capacity(8192);
    out.push_str("{\n");

    // project
    out.push_str("  \"project\": {\n");
    out.push_str(&format!("    \"name\": {},\n", json_str_opt(&project.name)));
    out.push_str(&format!("    \"version\": {},\n", json_str_opt(&project.version)));
    out.push_str(&format!("    \"type\": {},\n", json_str(&project.project_type)));
    out.push_str(&format!("    \"description\": {},\n", json_str_opt(&project.description)));
    out.push_str(&format!("    \"about\": {}\n", json_str_opt(&project.readme_excerpt)));
    out.push_str("  },\n");

    // stack
    let node_builtins: std::collections::HashSet<&str> = [
        "fs", "path", "os", "util", "crypto", "http", "https", "url", "stream",
        "events", "child_process", "assert", "buffer", "querystring", "zlib",
        "net", "tls", "dns", "cluster", "readline", "worker_threads",
        "node:fs", "node:path", "node:os", "node:util", "node:crypto",
        "node:http", "node:https", "node:url", "node:stream", "node:events",
        "node:child_process", "node:assert", "node:buffer", "node:test",
    ].iter().copied().collect();

    let known_services: &[(&str, &str)] = &[
        ("stripe", "Stripe"), ("@stripe", "Stripe"),
        ("redis", "Redis"), ("ioredis", "Redis"),
        ("prisma", "Prisma"), ("@prisma", "Prisma"),
        ("drizzle-orm", "Drizzle"), ("mongoose", "MongoDB"), ("mongodb", "MongoDB"),
        ("pg", "PostgreSQL"), ("mysql2", "MySQL"),
        ("@aws-sdk", "AWS"), ("aws-sdk", "AWS"),
        ("firebase", "Firebase"), ("@supabase", "Supabase"),
        ("socket.io", "Socket.IO"), ("graphql", "GraphQL"), ("@apollo", "Apollo"),
        ("tailwindcss", "Tailwind"), ("@sentry", "Sentry"), ("sentry", "Sentry"),
        ("zod", "Zod"), ("trpc", "tRPC"), ("@trpc", "tRPC"),
    ];

    let mut stack_parts: Vec<String> = project.frameworks.iter().cloned().collect();
    for (prefix, label) in known_services {
        let found = dep_graph.external_imports.keys().any(|k| {
            k == *prefix || k.starts_with(&format!("{}/", prefix))
        }) || project.dependencies.iter().any(|d| d == *prefix || d.starts_with(&format!("{}/", prefix)))
          || project.dev_dependencies.iter().any(|d| d == *prefix || d.starts_with(&format!("{}/", prefix)));
        if found && !stack_parts.iter().any(|s| s == *label) {
            stack_parts.push(label.to_string());
        }
    }
    if let Some(ref orm) = data_layer.orm {
        if !stack_parts.iter().any(|s| s == orm) {
            stack_parts.push(orm.clone());
        }
    }
    out.push_str(&format!("  \"stack\": {},\n", json_str_array(&stack_parts)));

    // stats
    let total_functions: u32 = stats.by_language.values().map(|l| l.functions).sum();
    let total_classes: u32 = stats.by_language.values().map(|l| l.classes).sum();
    let total_complexity: u32 = stats.by_language.values().map(|l| l.complexity).sum();
    let avg_complexity = if total_functions > 0 {
        (total_complexity as f64 / total_functions as f64 * 10.0).round() / 10.0
    } else {
        0.0
    };
    out.push_str("  \"stats\": {\n");
    out.push_str(&format!("    \"files\": {},\n", stats.files));
    out.push_str(&format!("    \"lines\": {},\n", stats.total_lines));
    out.push_str(&format!("    \"functions\": {},\n", total_functions));
    out.push_str(&format!("    \"classes\": {},\n", total_classes));
    out.push_str(&format!("    \"complexity\": {:.1}\n", avg_complexity));
    out.push_str("  },\n");

    // languages
    let mut langs: Vec<_> = stats.by_language.iter().collect();
    langs.sort_by(|a, b| b.1.lines.cmp(&a.1.lines));
    out.push_str("  \"languages\": {");
    let lang_entries: Vec<String> = langs.iter().map(|(name, data)| {
        let pct = if stats.total_lines > 0 {
            (data.lines as f64 / stats.total_lines as f64 * 100.0) as u32
        } else { 0 };
        format!("{}: {}", json_str(lang_abbrev(name)), pct)
    }).collect();
    out.push_str(&lang_entries.join(", "));
    out.push_str("},\n");

    // scripts
    out.push_str("  \"scripts\": {");
    let mut script_entries: Vec<String> = Vec::new();
    for key in &["dev", "start", "run", "build", "test"] {
        if let Some(val) = project.scripts.get(*key) {
            script_entries.push(format!("{}: {}", json_str(key), json_str(val)));
        }
    }
    out.push_str(&script_entries.join(", "));
    out.push_str("},\n");

    // env
    let mut all_env: Vec<String> = Vec::new();
    for analysis in file_metrics.values() {
        for v in &analysis.env_vars {
            if !all_env.contains(v) {
                all_env.push(v.clone());
            }
        }
    }
    all_env.sort();
    out.push_str(&format!("  \"env\": {},\n", json_str_array(&all_env)));

    // routes
    let mut all_routes: Vec<String> = Vec::new();
    for analysis in file_metrics.values() {
        for r in &analysis.http_routes {
            if !all_routes.contains(r) {
                all_routes.push(r.clone());
            }
        }
    }
    out.push_str(&format!("  \"routes\": {},\n", json_str_array(&all_routes)));

    // models
    out.push_str("  \"models\": {\n");
    out.push_str(&format!("    \"names\": {},\n", json_str_array(&data_layer.model_names)));
    out.push_str(&format!("    \"orm\": {},\n", json_str_opt(&data_layer.orm)));
    out.push_str(&format!("    \"schema\": {}\n", json_str_array(&data_layer.schema_files)));
    out.push_str("  },\n");

    // dirs
    out.push_str("  \"dirs\": [");
    let dir_entries: Vec<String> = key_locations.locations.iter()
        .filter(|l| l.count >= 2)
        .map(|l| format!("{{\"path\": {}, \"count\": {}}}", json_str(&l.path), l.count))
        .collect();
    out.push_str(&dir_entries.join(", "));
    out.push_str("],\n");

    // git
    out.push_str("  \"git\": {\n");
    out.push_str(&format!("    \"branch\": {},\n", json_str_opt(&git.branch)));
    out.push_str(&format!("    \"uncommitted\": {},\n", git.uncommitted.len()));
    out.push_str("    \"hot\": [");
    let hot_entries: Vec<String> = git.hot_files.iter().map(|(f, c)| {
        format!("{{\"file\": {}, \"count\": {}}}", json_str(f), c)
    }).collect();
    out.push_str(&hot_entries.join(", "));
    out.push_str("]\n");
    out.push_str("  },\n");

    // todos
    out.push_str("  \"todos\": [");
    let mut todo_entries: Vec<String> = Vec::new();
    for n in &scans.todos {
        todo_entries.push(format!(
            "{{\"kind\": \"TODO\", \"file\": {}, \"line\": {}, \"text\": {}}}",
            json_str(&n.file), n.line, json_str(&n.text)
        ));
    }
    for n in &scans.fixmes {
        todo_entries.push(format!(
            "{{\"kind\": \"FIXME\", \"file\": {}, \"line\": {}, \"text\": {}}}",
            json_str(&n.file), n.line, json_str(&n.text)
        ));
    }
    for n in &scans.hacks {
        todo_entries.push(format!(
            "{{\"kind\": \"HACK\", \"file\": {}, \"line\": {}, \"text\": {}}}",
            json_str(&n.file), n.line, json_str(&n.text)
        ));
    }
    out.push_str(&todo_entries.join(", "));
    out.push_str("],\n");

    // tests
    let test_pct = if test_map.source_count > 0 {
        (test_map.test_count as f64 / test_map.source_count as f64 * 100.0) as u32
    } else { 0 };
    out.push_str("  \"tests\": {\n");
    out.push_str(&format!("    \"count\": {},\n", test_map.test_count));
    out.push_str(&format!("    \"total\": {},\n", test_map.source_count));
    out.push_str(&format!("    \"coverage\": {}\n", test_pct));
    out.push_str("  },\n");

    // deps
    let mut ext_deps: Vec<_> = dep_graph.external_imports.iter()
        .filter(|(k, _)| !node_builtins.contains(k.as_str()))
        .filter(|(k, _)| !k.starts_with("@/") && !k.starts_with("./") && !k.starts_with("../"))
        .collect();
    ext_deps.sort_by(|a, b| b.1.cmp(a.1));
    out.push_str("  \"deps\": [");
    let dep_entries: Vec<String> = ext_deps.iter().take(10).map(|(k, v)| {
        format!("{{\"name\": {}, \"count\": {}}}", json_str(k), v)
    }).collect();
    out.push_str(&dep_entries.join(", "));
    out.push_str("],\n");

    // security
    out.push_str("  \"security\": [");
    let sec_entries: Vec<String> = scans.security.iter().map(|issue| {
        format!(
            "{{\"kind\": {}, \"file\": {}, \"line\": {}}}",
            json_str(&issue.kind), json_str(&issue.file), issue.line
        )
    }).collect();
    out.push_str(&sec_entries.join(", "));
    out.push_str("],\n");

    // issues
    let large = file_metrics.values().filter(|a| a.stats.lines > 500).count();
    let complex_fns: usize = file_metrics.values()
        .flat_map(|a| a.func_names.iter())
        .filter(|f| f.lines > 50)
        .count();
    out.push_str("  \"issues\": {\n");
    out.push_str(&format!("    \"orphaned\": {},\n", dead_code.orphaned_files.len()));
    out.push_str(&format!("    \"unused_exports\": {},\n", dead_code.unused_exports.len()));
    out.push_str(&format!("    \"single_use\": {},\n", dead_code.possibly_dead.len()));
    out.push_str(&format!("    \"large_files\": {},\n", large));
    out.push_str(&format!("    \"complex_fns\": {},\n", complex_fns));
    out.push_str(&format!("    \"duplicated\": {}\n", duplicates.len()));
    out.push_str("  },\n");

    // exports
    let mut exported_fns: Vec<String> = Vec::new();
    for (path, analysis) in file_metrics {
        for func in &analysis.func_names {
            if analysis.exported_names.contains(&func.name) {
                let fname = path.rsplit('/').next().unwrap_or(path);
                exported_fns.push(format!("{}:{}:{}", fname, func.start_line, func.name));
            }
        }
    }
    out.push_str(&format!("  \"exports\": {}\n", json_str_array(&exported_fns)));

    out.push_str("}\n");
    out
}
