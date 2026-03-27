use std::collections::{HashMap, HashSet};
use crate::analyzer::FileAnalysis;
use crate::depgraph::{DeadCode, DepGraph, ModuleInfo};
use crate::git::GitContext;
use crate::lang::lang_abbrev;
use crate::locations::KeyLocations;
use crate::models::DataLayer;
use crate::project::ProjectContext;
use crate::scanner::{ScanResults, TestMap};
use crate::tooling::ToolingContext;

pub struct AggregatedStats {
    pub files: u32,
    pub total_lines: u32,
    pub by_language: HashMap<String, LangStats>,
}

#[derive(Default)]
pub struct LangStats {
    pub files: u32,
    pub lines: u32,
    pub functions: u32,
    pub classes: u32,
    pub complexity: u32,
}

pub fn format_compact(
    stats: &AggregatedStats,
    file_metrics: &HashMap<String, FileAnalysis>,
    dep_graph: &DepGraph,
    dead_code: &DeadCode,
    duplicates: &[(String, Vec<(String, String)>)],
    project: &ProjectContext,
    git: &GitContext,
    tooling: &ToolingContext,
    scans: &ScanResults,
    test_map: &TestMap,
    data_layer: &DataLayer,
    key_locations: &KeyLocations,
) -> String {
    let mut out = String::with_capacity(8192);

    // Project info
    if let Some(ref name) = project.name {
        let ver = project.version.as_deref().unwrap_or("");
        let ptype = &project.project_type;
        out.push_str(&format!("## 🎯 **{}** v{} ({})\n", name, ver, ptype));
        if let Some(ref desc) = project.description {
            out.push_str(&format!("{}\n", desc));
        }
        out.push('\n');
    }

    // Quick start
    let mut quick = Vec::new();
    if project.project_type == "cli" {
        if let Some(ref name) = project.name {
            quick.push(format!("`npx {}`", name));
        }
    }
    if let Some(dev) = project.scripts.get("dev") {
        quick.push(format!("`npm run dev` → {}", dev));
    } else if let Some(start) = project.scripts.get("start") {
        quick.push(format!("`npm start` → {}", start));
    }
    if let Some(build) = project.scripts.get("build") {
        quick.push(format!("`npm run build` → {}", build));
    }
    if let Some(test) = project.scripts.get("test") {
        quick.push(format!("`npm test` → {}", test));
    }
    if !quick.is_empty() {
        out.push_str("## 🚀 Quick Start\n\n");
        for q in &quick {
            out.push_str(&format!("{}\n", q));
        }
        out.push('\n');
    }

    // Header
    let total_fn: u32 = stats.by_language.values().map(|l| l.functions).sum();
    let total_cls: u32 = stats.by_language.values().map(|l| l.classes).sum();
    let total_cx: u32 = stats.by_language.values().map(|l| l.complexity).sum();
    let avg_cx = if total_fn > 0 {
        format!("{:.1}", total_cx as f64 / total_fn as f64)
    } else {
        "0".into()
    };

    out.push_str(&format!(
        "# {}f {}L {}fn {}cls cx{}\n",
        stats.files, fmt_k(stats.total_lines), total_fn, total_cls, avg_cx
    ));
    out.push_str("*Legend: f=files L=lines fn=functions cls=classes cx=avg-complexity | file:line:name(NL)=location Np=params | ↑N=imports-from ↓N=imported-by (N)=occurrences (+N)=more | 🔄circular 🏝️isolated 🔥complex 📋duplicated 📁large*\n\n");

    // Langs
    let mut langs: Vec<_> = stats.by_language.iter().collect();
    langs.sort_by(|a, b| b.1.lines.cmp(&a.1.lines));
    let lang_str: Vec<String> = langs.iter().take(4).map(|(name, data)| {
        let pct = if stats.total_lines > 0 { (data.lines as f64 / stats.total_lines as f64 * 100.0) as u32 } else { 0 };
        format!("{}:{}%", lang_abbrev(name), pct)
    }).collect();
    out.push_str(&format!("**Langs:** {}\n\n", lang_str.join(" ")));

    // Tech Stack
    {
        let mut stack_items: Vec<String> = Vec::new();

        // 1. Add explicitly detected frameworks
        for fw in &project.frameworks {
            if !stack_items.contains(fw) {
                stack_items.push(fw.clone());
            }
        }

        // 2. Detect well-known packages from external imports
        let known_packages: &[(&[&str], &str)] = &[
            (&["stripe", "@stripe"], "Stripe"),
            (&["redis", "ioredis"], "Redis"),
            (&["prisma", "@prisma/client"], "Prisma"),
            (&["drizzle-orm"], "Drizzle"),
            (&["mongoose", "mongodb"], "MongoDB"),
            (&["pg", "postgres"], "PostgreSQL"),
            (&["mysql2"], "MySQL"),
            (&["aws-sdk", "@aws-sdk"], "AWS SDK"),
            (&["firebase"], "Firebase"),
            (&["supabase", "@supabase"], "Supabase"),
            (&["socket.io"], "Socket.IO"),
            (&["graphql", "@apollo"], "GraphQL"),
            (&["tailwindcss"], "Tailwind"),
            (&["sass"], "Sass"),
            (&["webpack"], "Webpack"),
            (&["vite"], "Vite"),
            (&["turbo"], "Turbo"),
            (&["docker"], "Docker"),
            (&["sentry", "@sentry"], "Sentry"),
            (&["zod"], "Zod"),
            (&["joi"], "Joi"),
            (&["trpc", "@trpc"], "tRPC"),
        ];

        for (patterns, display_name) in known_packages {
            for pattern in *patterns {
                if dep_graph.external_imports.contains_key(*pattern) && !stack_items.contains(&display_name.to_string()) {
                    stack_items.push(display_name.to_string());
                    break;
                }
            }
        }

        // 3. Detect from Go modules
        for go_mod in &project.go_modules {
            let go_known: &[(&str, &str)] = &[
                ("stripe", "Stripe"), ("redis", "Redis"), ("mongo", "MongoDB"),
                ("postgres", "PostgreSQL"), ("mysql", "MySQL"), ("grpc", "gRPC"),
            ];
            for (pattern, name) in go_known {
                if go_mod.contains(pattern) && !stack_items.contains(&name.to_string()) {
                    stack_items.push(name.to_string());
                }
            }
        }

        // Build languages list from stats
        let mut lang_names: Vec<String> = Vec::new();
        let mut lang_sorted: Vec<_> = stats.by_language.iter().collect();
        lang_sorted.sort_by(|a, b| b.1.lines.cmp(&a.1.lines));
        for (name, _) in lang_sorted.iter().take(4) {
            lang_names.push(name.to_string());
        }

        if !stack_items.is_empty() || !lang_names.is_empty() {
            out.push_str("## 🛠️ Tech Stack\n\n");
            if !stack_items.is_empty() {
                out.push_str(&format!("**Stack:** {}\n", stack_items.join(", ")));
            }
            if !lang_names.is_empty() {
                out.push_str(&format!("**Languages:** {}\n", lang_names.join(", ")));
            }
            out.push('\n');
        }
    }

    // Code Patterns
    let (mut total_async, mut total_await, mut total_promise, mut total_callback) = (0u32, 0, 0, 0);
    let (mut total_try, mut total_throw) = (0u32, 0);
    let mut all_constants: Vec<String> = Vec::new();
    let mut all_global_state: Vec<String> = Vec::new();
    let mut internal_calls: HashMap<String, u32> = HashMap::new();
    for analysis in file_metrics.values() {
        total_async += analysis.async_count;
        total_await += analysis.await_count;
        total_promise += analysis.promise_count;
        total_callback += analysis.callback_count;
        total_try += analysis.try_catch_count;
        total_throw += analysis.throw_count;
        for (name, _) in &analysis.constants {
            if all_constants.len() < 20 { all_constants.push(name.clone()); }
        }
        for name in &analysis.global_state {
            if all_global_state.len() < 10 { all_global_state.push(name.clone()); }
        }
        for (name, count) in &analysis.call_patterns {
            if !name.contains('.') && name.chars().next().map(|c| c.is_lowercase()).unwrap_or(false) {
                *internal_calls.entry(name.clone()).or_insert(0) += count;
            }
        }
    }
    let has_code_patterns = total_async > 0 || total_try > 0 || !all_constants.is_empty() || !internal_calls.is_empty();
    if has_code_patterns {
        out.push_str("## ⚡ Code Patterns\n\n");
        if total_async > 0 || total_promise > 0 || total_callback > 0 {
            let mut parts = Vec::new();
            if total_async > 0 { parts.push(format!("async({})", total_async)); }
            if total_await > 0 { parts.push(format!("await({})", total_await)); }
            if total_promise > 0 { parts.push(format!("Promise({})", total_promise)); }
            if total_callback > 0 { parts.push(format!("callbacks({})", total_callback)); }
            out.push_str(&format!("**Async:** {}\n", parts.join(", ")));
        }
        if total_try > 0 {
            out.push_str(&format!("**Errors:** try/catch({})\n", total_try));
        }
        if !internal_calls.is_empty() {
            let mut calls: Vec<_> = internal_calls.iter().collect();
            calls.sort_by(|a, b| b.1.cmp(a.1));
            let top: Vec<String> = calls.iter().take(8).map(|(n, c)| format!("{}({})", n, c)).collect();
            out.push_str(&format!("**Internal calls:** {}\n", top.join(", ")));
        }
        if !all_constants.is_empty() {
            let shown: Vec<&str> = all_constants.iter().take(6).map(|s| s.as_str()).collect();
            let extra = if all_constants.len() > 6 { format!(" (+{})", all_constants.len() - 6) } else { String::new() };
            out.push_str(&format!("**Constants:** {}{}\n", shown.join(", "), extra));
        }
        if !all_global_state.is_empty() {
            out.push_str(&format!("**Global state:** {}\n", all_global_state.join(", ")));
        }
        out.push('\n');
    }

    // I/O & Integration
    let mut all_env: Vec<String> = Vec::new();
    let mut all_urls: Vec<String> = Vec::new();
    let (mut total_file_io, mut total_json, mut total_sql, mut total_fetch) = (0u32, 0, 0, 0);
    let (mut total_listeners, mut total_emitters) = (0u32, 0);
    let mut all_routes: Vec<String> = Vec::new();
    for analysis in file_metrics.values() {
        for v in &analysis.env_vars { if !all_env.contains(v) && all_env.len() < 15 { all_env.push(v.clone()); } }
        for u in &analysis.urls { if all_urls.len() < 10 { all_urls.push(u.clone()); } }
        total_file_io += analysis.file_io_count;
        total_json += analysis.json_op_count;
        total_sql += analysis.sql_count;
        total_fetch += analysis.fetch_count;
        total_listeners += analysis.event_listeners;
        total_emitters += analysis.event_emitters;
        for r in &analysis.http_routes { if all_routes.len() < 10 { all_routes.push(r.clone()); } }
    }
    let has_io = !all_env.is_empty() || !all_urls.is_empty() || total_file_io > 0 || total_fetch > 0;
    if has_io {
        out.push_str("## 🔗 I/O & Integration\n\n");
        if !all_env.is_empty() {
            let extra = if all_env.len() > 8 { format!(" (+{})", all_env.len() - 8) } else { String::new() };
            let shown: Vec<&str> = all_env.iter().take(8).map(|s| s.as_str()).collect();
            out.push_str(&format!("**Env vars:** {}{}\n", shown.join(", "), extra));
        }
        if !all_urls.is_empty() {
            let shown: Vec<String> = all_urls.iter().take(5).map(|u| {
                if u.len() > 40 { format!("{}...", &u[..37]) } else { u.clone() }
            }).collect();
            let extra = if all_urls.len() > 5 { format!(" (+{})", all_urls.len() - 5) } else { String::new() };
            out.push_str(&format!("**URLs:** {}{}\n", shown.join(", "), extra));
        }
        if !all_routes.is_empty() {
            out.push_str(&format!("**Routes:** {}\n", all_routes.join(", ")));
        }
        let mut storage = Vec::new();
        if total_file_io > 0 { storage.push(format!("files({})", total_file_io)); }
        if total_json > 0 { storage.push(format!("JSON({})", total_json)); }
        if total_sql > 0 { storage.push(format!("SQL({})", total_sql)); }
        if !storage.is_empty() {
            out.push_str(&format!("**Storage:** {}\n", storage.join(", ")));
        }
        if total_listeners > 0 || total_emitters > 0 {
            out.push_str(&format!("**Events:** emit({}), listen({})\n", total_emitters, total_listeners));
        }
        out.push('\n');
    }

    // Features (group files by first directory component)
    {
        let mut dir_files: HashMap<String, Vec<String>> = HashMap::new();
        for path in file_metrics.keys() {
            let normalized = path.replace('\\', "/");
            let parts: Vec<&str> = normalized.split('/').collect();
            if parts.len() >= 2 {
                let dir_name = parts[0].to_string();
                let file_name = parts.last().unwrap_or(&"").to_string();
                dir_files.entry(dir_name).or_default().push(file_name);
            }
        }
        let mut dirs_with_files: Vec<(String, Vec<String>)> = dir_files
            .into_iter()
            .filter(|(_, files)| files.len() >= 2)
            .collect();
        dirs_with_files.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        if !dirs_with_files.is_empty() {
            out.push_str("## 📂 Features\n\n");
            for (dir, files) in dirs_with_files.iter().take(5) {
                let fn_count: u32 = file_metrics.iter()
                    .filter(|(p, _)| p.replace('\\', "/").starts_with(&format!("{}/", dir)))
                    .map(|(_, a)| a.stats.functions)
                    .sum();
                let top_files: Vec<&str> = files.iter().take(3).map(|s| s.as_str()).collect();
                out.push_str(&format!("**{}:** {}f, {}fn ({})\n", dir, files.len(), fn_count, top_files.join(", ")));
            }
            out.push('\n');
        }
    }

    // Data Layer
    {
        let has_data = !data_layer.model_names.is_empty()
            || !data_layer.schema_files.is_empty()
            || data_layer.orm.is_some()
            || !data_layer.migration_dirs.is_empty();
        if has_data {
            out.push_str("## \u{1F5C4}\u{FE0F} Data Layer\n\n");
            if !data_layer.model_names.is_empty() {
                let limit = 15;
                let shown: Vec<&str> = data_layer.model_names.iter().take(limit).map(|s| s.as_str()).collect();
                let extra = if data_layer.model_names.len() > limit {
                    format!(" (+{})", data_layer.model_names.len() - limit)
                } else {
                    String::new()
                };
                out.push_str(&format!("**Models:** {}{}\n", shown.join(", "), extra));
            }
            if !data_layer.schema_files.is_empty() {
                let shown: Vec<&str> = data_layer.schema_files.iter().take(5).map(|s| s.as_str()).collect();
                out.push_str(&format!("**Schema:** {}\n", shown.join(", ")));
            }
            if let Some(ref orm_name) = data_layer.orm {
                out.push_str(&format!("**ORM:** {}\n", orm_name));
            }
            if !data_layer.migration_dirs.is_empty() {
                let shown: Vec<&str> = data_layer.migration_dirs.iter().take(3).map(|s| s.as_str()).collect();
                out.push_str(&format!("**Migrations:** {}\n", shown.join(", ")));
            }
            out.push('\n');
        }
    }

    // Key Locations
    {
        let visible: Vec<_> = key_locations.locations.iter()
            .filter(|loc| loc.count >= 2)
            .take(10)
            .collect();
        if !visible.is_empty() {
            out.push_str("## \u{1F4CD} Key Locations\n\n");
            for loc in &visible {
                out.push_str(&format!("**{}:** {} ({} files)\n", loc.label, loc.path, loc.count));
            }
            out.push('\n');
        }
    }

    // Git Context
    if git.is_repo {
        let mut git_parts = Vec::new();
        if let Some(ref branch) = git.branch {
            git_parts.push(format!("**Branch:** {}", branch));
        }
        if !git.recent_commits.is_empty() {
            let commits: Vec<&str> = git.recent_commits.iter().take(5).map(|s| s.as_str()).collect();
            git_parts.push(format!("**Last commits:** {}", commits.join(" | ")));
        }
        if !git.uncommitted.is_empty() {
            let files: Vec<&str> = git.uncommitted.iter().take(5).map(|s| s.as_str()).collect();
            let extra = if git.uncommitted.len() > 5 { format!(" (+{})", git.uncommitted.len() - 5) } else { String::new() };
            git_parts.push(format!("**Uncommitted:** {} file(s): {}{}", git.uncommitted.len(), files.join(", "), extra));
        }
        if !git.hot_files.is_empty() {
            let hot: Vec<String> = git.hot_files.iter().take(6).map(|(f, c)| {
                let fname = f.rsplit('/').next().unwrap_or(f);
                format!("{}({})", fname, c)
            }).collect();
            git_parts.push(format!("**Hot files:** {}", hot.join(", ")));
        }
        if !git_parts.is_empty() {
            out.push_str("## 📝 Recent Activity\n\n");
            for part in &git_parts { out.push_str(&format!("{}\n", part)); }
            out.push('\n');
        }
    }

    // Tooling
    {
        let mut tool_parts = Vec::new();
        if let Some(ref ts) = tooling.typescript {
            let mode = if ts.strict { "strict" } else { "standard" };
            let target = ts.target.as_deref().unwrap_or("default");
            tool_parts.push(format!("**TypeScript:** {} mode, target {}", mode, target));
        }
        let mut lint_tools = tooling.linting.clone();
        if tooling.has_prettier { lint_tools.push("Prettier".into()); }
        if !lint_tools.is_empty() {
            tool_parts.push(format!("**Linting:** {}", lint_tools.join(" + ")));
        }
        if let Some(ref test_fw) = tooling.testing {
            tool_parts.push(format!("**Testing:** {} ({} test files)", test_fw, test_map.test_count));
        } else if test_map.test_count > 0 {
            tool_parts.push(format!("**Testing:** {} test files", test_map.test_count));
        }
        if !tooling.ci.is_empty() {
            tool_parts.push(format!("**CI:** {}", tooling.ci.join(", ")));
        }
        if tooling.has_dockerfile {
            tool_parts.push("**Container:** Docker".into());
        }
        if !tooling.env_files.is_empty() {
            tool_parts.push(format!("**Env:** {}", tooling.env_files.join(", ")));
        }
        if !tool_parts.is_empty() {
            out.push_str("## 🔧 Tooling\n\n");
            for part in &tool_parts { out.push_str(&format!("{}\n", part)); }
            out.push('\n');
        }
    }

    // Developer Notes (TODO/FIXME/HACK)
    if !scans.todos.is_empty() || !scans.fixmes.is_empty() || !scans.hacks.is_empty() {
        out.push_str("## 📌 Developer Notes\n\n");
        if !scans.todos.is_empty() {
            let items: Vec<String> = scans.todos.iter().take(5).map(|n| {
                let fname = n.file.rsplit('/').next().unwrap_or(&n.file);
                if n.text.is_empty() { format!("{}:{}", fname, n.line) }
                else { format!("{}:{} \"{}\"", fname, n.line, n.text) }
            }).collect();
            let extra = if scans.todos.len() > 5 { format!(" (+{})", scans.todos.len() - 5) } else { String::new() };
            out.push_str(&format!("**TODO({}):** {}{}\n", scans.todos.len(), items.join(", "), extra));
        }
        if !scans.fixmes.is_empty() {
            let items: Vec<String> = scans.fixmes.iter().take(3).map(|n| {
                let fname = n.file.rsplit('/').next().unwrap_or(&n.file);
                if n.text.is_empty() { format!("{}:{}", fname, n.line) }
                else { format!("{}:{} \"{}\"", fname, n.line, n.text) }
            }).collect();
            out.push_str(&format!("**FIXME({}):** {}\n", scans.fixmes.len(), items.join(", ")));
        }
        if !scans.hacks.is_empty() {
            let items: Vec<String> = scans.hacks.iter().take(3).map(|n| {
                let fname = n.file.rsplit('/').next().unwrap_or(&n.file);
                if n.text.is_empty() { format!("{}:{}", fname, n.line) }
                else { format!("{}:{} \"{}\"", fname, n.line, n.text) }
            }).collect();
            out.push_str(&format!("**HACK({}):** {}\n", scans.hacks.len(), items.join(", ")));
        }
        out.push('\n');
    }

    // Security
    if !scans.security.is_empty() {
        out.push_str("## 🔒 Security\n\n");
        let mut by_kind: HashMap<&str, Vec<String>> = HashMap::new();
        for issue in &scans.security {
            let fname = issue.file.rsplit('/').next().unwrap_or(&issue.file);
            by_kind.entry(&issue.kind).or_default().push(format!("{}:{}", fname, issue.line));
        }
        for (kind, locations) in &by_kind {
            let label = match *kind {
                "eval" => "eval() usage",
                "secret" => "Possible hardcoded secrets",
                "sql_injection" => "SQL interpolation",
                _ => kind,
            };
            let shown: Vec<&str> = locations.iter().take(5).map(|s| s.as_str()).collect();
            let extra = if locations.len() > 5 { format!(" (+{})", locations.len() - 5) } else { String::new() };
            out.push_str(&format!("**{}:** {}{}\n", label, shown.join(", "), extra));
        }
        out.push('\n');
    }

    // Test Coverage Map
    if test_map.test_count > 0 || !test_map.uncovered.is_empty() {
        out.push_str("## 🧪 Test Map\n\n");
        if !test_map.covered.is_empty() {
            let items: Vec<String> = test_map.covered.iter().take(5).map(|(src, test)| {
                let sf = src.rsplit('/').next().unwrap_or(src);
                let tf = test.rsplit('/').next().unwrap_or(test);
                format!("{} → {}", sf, tf)
            }).collect();
            let extra = if test_map.covered.len() > 5 { format!(" (+{})", test_map.covered.len() - 5) } else { String::new() };
            out.push_str(&format!("**Covered:** {}{}\n", items.join(", "), extra));
        }
        if !test_map.uncovered.is_empty() && test_map.test_count > 0 {
            let items: Vec<String> = test_map.uncovered.iter().take(6).map(|s| {
                s.rsplit('/').next().unwrap_or(s).to_string()
            }).collect();
            let extra = if test_map.uncovered.len() > 6 { format!(" (+{})", test_map.uncovered.len() - 6) } else { String::new() };
            out.push_str(&format!("**Uncovered:** {}{}\n", items.join(", "), extra));
        }
        let ratio = if test_map.source_count > 0 {
            format!("{}%", (test_map.test_count as f64 / test_map.source_count as f64 * 100.0) as u32)
        } else { "0%".into() };
        out.push_str(&format!("**Test ratio:** {} test / {} source ({})\n", test_map.test_count, test_map.source_count, ratio));
        out.push('\n');
    }

    // Code Organization
    let mut large_files_list: Vec<(&str, u32)> = file_metrics.iter()
        .filter(|(_, a)| a.stats.lines > 200)
        .map(|(p, a)| (p.as_str(), a.stats.lines))
        .collect();
    large_files_list.sort_by(|a, b| b.1.cmp(&a.1));
    let many_params: Vec<String> = file_metrics.iter()
        .flat_map(|(path, a)| a.func_names.iter().filter(|f| f.params > 5).map(move |f| {
            let fname = path.rsplit('/').next().unwrap_or(path);
            format!("{}:{}:{}({}p)", fname, f.start_line, f.name, f.params)
        }))
        .collect();
    if !large_files_list.is_empty() || !many_params.is_empty() {
        out.push_str("## 📊 Code Organization\n\n");
        if !large_files_list.is_empty() {
            let items: Vec<String> = large_files_list.iter().take(5).map(|(p, l)| format!("{}:{}L", p, l)).collect();
            out.push_str(&format!("**Large files:** {}\n", items.join(", ")));
        }
        if !many_params.is_empty() {
            out.push_str(&format!("**Many params:** {}\n", many_params.iter().take(5).cloned().collect::<Vec<_>>().join(", ")));
        }
        out.push('\n');
    }

    // Architecture
    if !dep_graph.coupling.is_empty() && stats.files >= 3 {
        let mut connections: Vec<_> = dep_graph.coupling.iter()
            .map(|(path, (in_c, out_c))| {
                let fname = path.rsplit('/').next().unwrap_or(path);
                let fname_no_ext = fname.rsplit_once('.').map(|(n, _)| n).unwrap_or(fname);
                (path.as_str(), fname_no_ext, *in_c, *out_c, in_c + out_c)
            }).collect();
        connections.sort_by(|a, b| b.4.cmp(&a.4));
        let top: Vec<String> = connections.iter().take(8)
            .map(|(_, name, in_c, out_c, _)| format!("{}(↑{}↓{})", name, out_c, in_c)).collect();
        out.push_str(&format!("## 🔄 Architecture\n\n**Key files:** {}\n", top.join(", ")));
        if !dep_graph.circular.is_empty() {
            let cycles: Vec<String> = dep_graph.circular.iter().take(3).map(|c| {
                c.iter().map(|p| p.rsplit('/').next().unwrap_or(p)).collect::<Vec<_>>().join("→")
            }).collect();
            out.push_str(&format!("**🔄 Circular:** {}\n", cycles.join("; ")));
        }

        // External deps
        if !dep_graph.external_imports.is_empty() {
            let node_builtins: HashSet<&str> = [
                "fs", "path", "os", "util", "crypto", "http", "https", "url",
                "stream", "events", "child_process", "assert", "buffer",
                "querystring", "zlib", "net", "tls", "dns", "cluster",
                "readline", "worker_threads",
            ].iter().copied().collect();
            let mut ext_sorted: Vec<_> = dep_graph.external_imports.iter()
                .filter(|(name, _)| !node_builtins.contains(name.as_str()))
                .collect();
            ext_sorted.sort_by(|a, b| b.1.cmp(a.1));
            if !ext_sorted.is_empty() {
                let items: Vec<String> = ext_sorted.iter().take(6)
                    .map(|(name, count)| format!("{}({})", name, count)).collect();
                out.push_str(&format!("**External deps:** {}\n", items.join(", ")));
            }
        }

        // Cross-module
        if !dep_graph.cross_module_deps.is_empty() {
            let mut seen: HashSet<String> = HashSet::new();
            let mut items: Vec<String> = Vec::new();
            for (from_mod, to_mod) in &dep_graph.cross_module_deps {
                let key = format!("{}→{}", from_mod, to_mod);
                if seen.insert(key.clone()) {
                    items.push(key);
                }
                if items.len() >= 6 { break; }
            }
            if !items.is_empty() {
                out.push_str(&format!("**Cross-module:** {}\n", items.join(", ")));
            }
        }

        // Module flow
        if !dep_graph.modules.is_empty() {
            let mut net_importers: Vec<String> = Vec::new();
            let mut net_exporters: Vec<String> = Vec::new();
            for (name, info) in &dep_graph.modules {
                if info.imports > info.exports {
                    net_importers.push(name.clone());
                } else if info.exports > info.imports {
                    net_exporters.push(name.clone());
                }
            }
            if !net_importers.is_empty() || !net_exporters.is_empty() {
                let mut parts = Vec::new();
                if !net_exporters.is_empty() {
                    let shown: Vec<&str> = net_exporters.iter().take(3).map(|s| s.as_str()).collect();
                    parts.push(format!("providers: {}", shown.join(", ")));
                }
                if !net_importers.is_empty() {
                    let shown: Vec<&str> = net_importers.iter().take(3).map(|s| s.as_str()).collect();
                    parts.push(format!("consumers: {}", shown.join(", ")));
                }
                out.push_str(&format!("**Module flow:** {}\n", parts.join(" | ")));
            }
        }

        out.push('\n');
    }

    // API Surface
    let mut exported_fns: Vec<(String, String, u32, u32)> = Vec::new();
    for (path, analysis) in file_metrics {
        for func in &analysis.func_names {
            if analysis.exported_names.contains(&func.name) {
                let fname = path.rsplit('/').next().unwrap_or(path);
                exported_fns.push((fname.to_string(), func.name.clone(), func.start_line, func.params));
            }
        }
    }
    let mut all_classes: Vec<(String, u32)> = Vec::new();
    for (_, analysis) in file_metrics {
        for name in &analysis.class_names {
            all_classes.push((name.clone(), 1));
        }
    }
    if !exported_fns.is_empty() || !all_classes.is_empty() {
        out.push_str("## 🔌 API Surface\n\n");
        if !exported_fns.is_empty() {
            let items: Vec<String> = exported_fns.iter().take(12)
                .map(|(f, name, line, p)| format!("{}:{}:{}({}p)", f, line, name, p)).collect();
            let extra = if exported_fns.len() > 12 { format!(" (+{})", exported_fns.len() - 12) } else { String::new() };
            out.push_str(&format!("**Exported fns:** {}{}\n", items.join(", "), extra));
        }
        if !all_classes.is_empty() {
            let items: Vec<String> = all_classes.iter().take(6).map(|(n, _)| n.clone()).collect();
            let extra = if all_classes.len() > 6 { format!(" (+{})", all_classes.len() - 6) } else { String::new() };
            out.push_str(&format!("**Classes:** {}{}\n", items.join(", "), extra));
        }
        // Entry files
        if !dep_graph.entry_points.is_empty() {
            let items: Vec<String> = dep_graph.entry_points.iter().take(5).map(|p| {
                let fname = p.rsplit('/').next().unwrap_or(p);
                fname.rsplit_once('.').map(|(n, _)| n).unwrap_or(fname).to_string()
            }).collect();
            out.push_str(&format!("**Entry files:** {}\n", items.join(", ")));
        }
        out.push('\n');
    }

    // Issues
    let mut issues = Vec::new();
    if !dep_graph.circular.is_empty() {
        issues.push(format!("🔄 {} circular dependency chain(s)", dep_graph.circular.len()));
    }
    if !duplicates.is_empty() {
        let dup_detail: Vec<String> = duplicates.iter().take(3).map(|(_, instances)| {
            let files: Vec<String> = instances.iter().take(3).map(|(f, _)| {
                f.rsplit('/').next().unwrap_or(f).to_string()
            }).collect();
            format!("{}({}×)", files.join(", "), instances.len())
        }).collect();
        let extra = if duplicates.len() > 3 { format!(" (+{})", duplicates.len() - 3) } else { String::new() };
        issues.push(format!("📋 Duplication: {}{}", dup_detail.join(" | "), extra));
    }
    let large_count = file_metrics.values().filter(|a| a.stats.lines > 500).count();
    if large_count > 0 {
        issues.push(format!("📁 {} file(s) over 500 lines", large_count));
    }
    let complex_fns: usize = file_metrics.values().flat_map(|a| a.func_names.iter()).filter(|f| f.lines > 50).count();
    if complex_fns > 0 {
        issues.push(format!("🔥 {} function(s) over 50 lines", complex_fns));
    }
    if !issues.is_empty() {
        out.push_str("## 🚨 Issues\n\n");
        for issue in &issues { out.push_str(&format!("- {}\n", issue)); }
        out.push('\n');
    }

    // Dead Code
    let has_dead = !dead_code.orphaned_files.is_empty() || !dead_code.unused_exports.is_empty() || !dead_code.test_files.is_empty();
    if has_dead {
        out.push_str("## 🧹 Dead Code & Tests\n\n");
        if !dead_code.unused_exports.is_empty() {
            let items: Vec<String> = dead_code.unused_exports.iter().take(5).map(|(path, exports)| {
                let fname = path.rsplit('/').next().unwrap_or(path);
                format!("{}({})", fname, exports.join(","))
            }).collect();
            let extra = if dead_code.unused_exports.len() > 5 { format!(" (+{})", dead_code.unused_exports.len() - 5) } else { String::new() };
            out.push_str(&format!("**Unused exports:** {}{}\n", items.join(", "), extra));
        }
        if !dead_code.orphaned_files.is_empty() {
            let names: Vec<String> = dead_code.orphaned_files.iter().take(6).map(|p| {
                p.rsplit('/').next().unwrap_or(p).to_string()
            }).collect();
            let extra = if dead_code.orphaned_files.len() > 6 { format!(" (+{})", dead_code.orphaned_files.len() - 6) } else { String::new() };
            out.push_str(&format!("**Orphaned:** {}{}\n", names.join(", "), extra));
        }
        if !dead_code.test_files.is_empty() {
            out.push_str(&format!("**Test files:** {}\n", dead_code.test_files.len()));
        }
        // Possibly dead
        if !dead_code.possibly_dead.is_empty() {
            let items: Vec<String> = dead_code.possibly_dead.iter().take(5).map(|(path, importer)| {
                let fname = path.rsplit('/').next().unwrap_or(path);
                format!("{} (only used by {})", fname, importer)
            }).collect();
            let extra = if dead_code.possibly_dead.len() > 5 { format!(" (+{})", dead_code.possibly_dead.len() - 5) } else { String::new() };
            out.push_str(&format!("**Possibly dead:** {}{}\n", items.join(", "), extra));
        }
        out.push('\n');
    }

    // Modules
    if stats.files >= 5 && !dep_graph.modules.is_empty() {
        out.push_str("## 📦 Modules\n\n");
        let mut mod_list: Vec<(&String, &ModuleInfo)> = dep_graph.modules.iter().collect();
        mod_list.sort_by(|a, b| b.1.connections.cmp(&a.1.connections));
        for (name, info) in mod_list.iter().take(6) {
            out.push_str(&format!("{}: {}f, {}cx, {}↑, {}↓\n", name, info.files, info.connections, info.imports, info.exports));
        }
        out.push('\n');
    }

    // File Index
    if stats.files <= 30 {
        out.push_str("## 📄 File Index\n\n");
        let mut files: Vec<_> = file_metrics.iter().collect();
        files.sort_by(|a, b| b.1.stats.lines.cmp(&a.1.stats.lines));
        for (path, analysis) in files.iter().take(20) {
            let fname = path.rsplit('/').next().unwrap_or(path);
            let dep_info = dep_graph.coupling.get(*path);
            let (in_c, out_c) = dep_info.copied().unwrap_or((0, 0));
            let flags = build_flags(analysis, dep_graph, path);
            let mut line = format!("**{}** {}L", fname, analysis.stats.lines);
            if analysis.stats.functions > 0 || analysis.stats.classes > 0 {
                if out_c > 0 { line.push_str(&format!(" ↑{}", out_c)); }
                if in_c > 0 { line.push_str(&format!(" ↓{}", in_c)); }
            }
            if !flags.is_empty() { line.push_str(&format!(" {}", flags)); }
            if !analysis.func_names.is_empty() {
                let fns: Vec<String> = analysis.func_names.iter().take(5)
                    .map(|f| format!("{}", f.name)).collect();
                let extra = if analysis.func_names.len() > 5 { format!(", (+{})", analysis.func_names.len() - 5) } else { String::new() };
                line.push_str(&format!(" fn: {}{}", fns.join(", "), extra));
            }
            if !analysis.exported_names.is_empty() && analysis.func_names.is_empty() {
                let exports: Vec<&str> = analysis.exported_names.iter().take(6).map(|s| s.as_str()).collect();
                line.push_str(&format!(" exports: {}", exports.join(", ")));
            }
            out.push_str(&format!("{}\n", line));
        }
        out.push('\n');
    }

    out.trim_end().to_string()
}

fn build_flags(analysis: &FileAnalysis, dep_graph: &DepGraph, path: &str) -> String {
    let mut flags = Vec::new();
    if dep_graph.circular.iter().any(|c| c.contains(&path.to_string())) { flags.push("🔄"); }
    if dep_graph.orphans.contains(path) { flags.push("🏝️"); }
    if analysis.func_names.iter().any(|f| f.lines > 50) { flags.push("🔥"); }
    if analysis.stats.lines > 500 { flags.push("📁"); }
    flags.join("")
}

fn fmt_k(n: u32) -> String {
    if n >= 1000 { format!("{:.1}k", n as f64 / 1000.0) } else { n.to_string() }
}
