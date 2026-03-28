use std::collections::HashMap;
use std::fmt::Write;
use crate::analyzer::FileAnalysis;
use crate::conventions::LanguageConventions;
use crate::depgraph::{DeadCode, DepGraph};
use crate::git::GitContext;
use crate::lang::{lang_abbrev, KNOWN_SERVICES, NODE_BUILTINS};
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
    conventions: &[LanguageConventions],
) -> String {
    let mut out = String::with_capacity(4096);

    let total_fn: u32 = stats.by_language.values().map(|l| l.functions).sum();
    let total_cls: u32 = stats.by_language.values().map(|l| l.classes).sum();
    let total_cx: u32 = stats.by_language.values().map(|l| l.complexity).sum();
    let avg_cx = if total_fn > 0 { format!("{:.1}", total_cx as f64 / total_fn as f64) } else { "0".into() };

    // ## 🎯 Project header
    if let Some(ref name) = project.name {
        let ver = project.version.as_deref().unwrap_or("");
        let desc = project.description.as_deref().unwrap_or("");
        let badges = if project.project_type == "npm" || project.project_type == "node" {
            format!(" [![npm](https://img.shields.io/npm/v/{name})](https://www.npmjs.com/package/{name})")
        } else if project.project_type == "rust" {
            format!(" [![crates.io](https://img.shields.io/crates/v/{name})](https://crates.io/crates/{name})")
        } else {
            String::new()
        };
        let ver_str = if !ver.is_empty() { format!(" v{ver}") } else { String::new() };
        let desc_str = if !desc.is_empty() { format!(" — {desc}") } else { String::new() };
        let _ = writeln!(out, "## 🎯 {name}{ver_str}{badges}{desc_str}");
        out.push('\n');
    } else if let Some(ref excerpt) = project.readme_excerpt {
        if !excerpt.is_empty() {
            let _ = writeln!(out, "## 🎯 {excerpt}");
            out.push('\n');
        }
    }

    // Quick start scripts
    let mut scripts_parts = Vec::new();
    if let Some(dev) = project.scripts.get("dev") { scripts_parts.push(format!("`{dev}`")); }
    else if let Some(start) = project.scripts.get("start") { scripts_parts.push(format!("`{start}`")); }
    if let Some(build) = project.scripts.get("build") { scripts_parts.push(format!("`{build}`")); }
    if let Some(test) = project.scripts.get("test") { scripts_parts.push(format!("`{test}`")); }
    if !scripts_parts.is_empty() {
        let _ = writeln!(out, "## 🚀 Quick Start\n\n{}\n", scripts_parts.join(" | "));
    }

    // Stats line + legend
    let _ = writeln!(out, "# {}f {}L {}fn {}cls cx{}", stats.files, fmt_k(stats.total_lines), total_fn, total_cls, avg_cx);
    out.push_str("*Legend: f=files L=lines fn=functions cls=classes cx=avg-complexity | file:line:name(NL)=location Np=params | ↑N=imports-from ↓N=imported-by (N)=occurrences (+N)=more | 🔄circular 🏝️isolated 🔥complex 📋duplicated 📁large*\n\n");

    // Langs
    let mut langs: Vec<_> = stats.by_language.iter().collect();
    langs.sort_by(|a, b| b.1.lines.cmp(&a.1.lines));
    let lang_str: Vec<String> = langs.iter().take(4).map(|(name, data)| {
        let pct = if stats.total_lines > 0 { (data.lines as f64 / stats.total_lines as f64 * 100.0) as u32 } else { 0 };
        format!("{}:{}%", lang_abbrev(name), pct)
    }).collect();
    let _ = writeln!(out, "**Langs:** {}\n", lang_str.join(" "));

    // ## 🛠️ Tech Stack
    let mut stack_parts: Vec<String> = project.frameworks.iter().cloned().collect();
    for (prefix, label) in KNOWN_SERVICES {
        let found = dep_graph.external_imports.keys().any(|k| k == *prefix || k.starts_with(&format!("{prefix}/")))
            || project.dependencies.iter().any(|d| d == *prefix || d.starts_with(&format!("{prefix}/")))
            || project.dev_dependencies.iter().any(|d| d == *prefix || d.starts_with(&format!("{prefix}/")));
        if found && !stack_parts.iter().any(|s| s == *label) {
            stack_parts.push(label.to_string());
        }
    }
    if let Some(ref orm) = data_layer.orm {
        if !stack_parts.iter().any(|s| s == orm) { stack_parts.push(orm.clone()); }
    }

    let mut all_patterns: HashMap<String, u32> = HashMap::new();
    let mut all_identifiers: HashMap<String, u32> = HashMap::new();
    for a in file_metrics.values() {
        for (k, v) in &a.call_patterns { *all_patterns.entry(k.clone()).or_default() += v; }
        for (k, v) in &a.identifiers { *all_identifiers.entry(k.clone()).or_default() += v; }
    }
    let noise: &[&str] = &["console.log","console.error","console.warn","process.exit","JSON.stringify","JSON.parse","require","path.join","path.resolve","parseInt","parseFloat","Object.keys","Object.entries","Object.assign","Array.from","String","Number","Boolean"];
    let mut sorted_patterns: Vec<_> = all_patterns.iter().filter(|(k, _)| !noise.contains(&k.as_str())).collect();
    sorted_patterns.sort_by(|a, b| b.1.cmp(a.1));
    let mut sorted_ids: Vec<_> = all_identifiers.iter()
        .filter(|(k, _)| k.len() >= 3 && k.len() <= 25)
        .collect();
    sorted_ids.sort_by(|a, b| b.1.cmp(a.1));

    let has_stack = !stack_parts.is_empty() || !sorted_patterns.is_empty() || !sorted_ids.is_empty();
    if has_stack {
        out.push_str("## 🛠️ Tech Stack\n\n");
        if !stack_parts.is_empty() {
            let _ = writeln!(out, "**Stack:** {}", stack_parts.join(", "));
        }
        if !sorted_patterns.is_empty() {
            let pat_str: Vec<String> = sorted_patterns.iter().take(6).map(|(k, v)| format!("{k}({v})")).collect();
            let _ = writeln!(out, "**Patterns:** {}", pat_str.join(", "));
        }
        if !sorted_ids.is_empty() {
            let id_str: Vec<String> = sorted_ids.iter().take(6).map(|(k, v)| format!("{k}({v})")).collect();
            let _ = writeln!(out, "**Top IDs:** {}", id_str.join(", "));
        }
        out.push('\n');
    }

    // ## ⚡ Code Patterns
    let total_async: u32 = file_metrics.values().map(|a| a.async_count).sum();
    let total_await: u32 = file_metrics.values().map(|a| a.await_count).sum();
    let total_promise: u32 = file_metrics.values().map(|a| a.promise_count).sum();
    let total_try: u32 = file_metrics.values().map(|a| a.try_catch_count).sum();
    let total_throw: u32 = file_metrics.values().map(|a| a.throw_count).sum();

    let mut all_calls: HashMap<String, u32> = HashMap::new();
    for a in file_metrics.values() {
        for (k, v) in &a.call_patterns { *all_calls.entry(k.clone()).or_default() += v; }
    }
    let mut sorted_calls: Vec<_> = all_calls.iter().filter(|(k, _)| !noise.contains(&k.as_str())).collect();
    sorted_calls.sort_by(|a, b| b.1.cmp(a.1));

    let has_code_patterns = total_async > 0 || total_try > 0 || !sorted_calls.is_empty();
    if has_code_patterns {
        out.push_str("## ⚡ Code Patterns\n\n");
        let mut async_parts = Vec::new();
        if total_async > 0 { async_parts.push(format!("async({total_async})")); }
        if total_await > 0 { async_parts.push(format!("await({total_await})")); }
        if total_promise > 0 { async_parts.push(format!("Promise({total_promise})")); }
        if !async_parts.is_empty() { let _ = writeln!(out, "**Async:** {}", async_parts.join(", ")); }
        let mut err_parts = Vec::new();
        if total_try > 0 { err_parts.push(format!("try/catch({total_try})")); }
        if total_throw > 0 { err_parts.push(format!("throw({total_throw})")); }
        if !err_parts.is_empty() { let _ = writeln!(out, "**Errors:** {}", err_parts.join(", ")); }
        if !sorted_calls.is_empty() {
            let call_str: Vec<String> = sorted_calls.iter().take(8).map(|(k, v)| format!("{k}({v})")).collect();
            let _ = writeln!(out, "**Internal calls:** {}", call_str.join(", "));
        }
        out.push('\n');
    }

    // ## 🔗 I/O & Integration
    let all_env: Vec<String> = file_metrics.values().flat_map(|a| a.env_vars.iter().cloned()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    let total_sql: u32 = file_metrics.values().map(|a| a.sql_count).sum();
    let total_files: u32 = file_metrics.values().map(|a| a.file_io_count).sum();
    let total_json: u32 = file_metrics.values().map(|a| a.json_op_count).sum();
    let total_fetch: u32 = file_metrics.values().map(|a| a.fetch_count).sum();
    let total_emit: u32 = file_metrics.values().map(|a| a.event_emitters).sum();
    let total_listen: u32 = file_metrics.values().map(|a| a.event_listeners).sum();
    let all_routes: Vec<String> = file_metrics.values().flat_map(|a| a.http_routes.iter().cloned()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();

    let has_io = !all_env.is_empty() || total_sql > 0 || total_files > 0 || total_fetch > 0 || !all_routes.is_empty();
    if has_io {
        out.push_str("## 🔗 I/O & Integration\n\n");
        if !all_env.is_empty() {
            let _ = writeln!(out, "**Env vars:** {}", all_env.join(", "));
        }
        if total_fetch > 0 || !all_routes.is_empty() {
            let mut http_parts = Vec::new();
            if total_fetch > 0 { http_parts.push(format!("fetch({total_fetch})")); }
            if !all_routes.is_empty() {
                let routes = all_routes.iter().take(4).cloned().collect::<Vec<_>>().join(", ");
                let more = if all_routes.len() > 4 { format!(" (+{})", all_routes.len() - 4) } else { String::new() };
                http_parts.push(format!("routes: {routes}{more}"));
            }
            let _ = writeln!(out, "**HTTP:** {}", http_parts.join(", "));
        }
        let mut storage_parts = Vec::new();
        if total_sql > 0 { storage_parts.push(format!("SQL({total_sql})")); }
        if total_files > 0 { storage_parts.push(format!("files({total_files})")); }
        if total_json > 0 { storage_parts.push(format!("JSON({total_json})")); }
        if !storage_parts.is_empty() { let _ = writeln!(out, "**Storage:** {}", storage_parts.join(", ")); }
        if total_emit > 0 || total_listen > 0 {
            let _ = writeln!(out, "**Events:** emit({total_emit}), listen({total_listen})");
        }
        if !data_layer.model_names.is_empty() {
            let _ = writeln!(out, "**Models:** {}", data_layer.model_names.join(", "));
        }
        out.push('\n');
    }

    // ## 📊 Code Organization
    let mut large_files: Vec<(&String, u32)> = {
        let mut seen: std::collections::HashSet<(String, u32)> = std::collections::HashSet::new();
        let mut v: Vec<(&String, u32)> = file_metrics.iter()
            .map(|(p, a)| (p, a.stats.lines))
            .filter(|(p, l)| *l >= 200 && !p.ends_with(".json") && !p.ends_with(".lock") && !p.ends_with("-lock.json"))
            .filter(|(p, l)| seen.insert((p.rsplit('/').next().unwrap_or(p).to_string(), *l)))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    };

    let mut long_fns: Vec<(String, u32, String, u32)> = Vec::new();
    let mut many_param_fns: Vec<(String, u32, String, u32)> = Vec::new();
    let mut all_classes: Vec<(String, u32, String)> = Vec::new();
    let mut seen_fns: std::collections::HashSet<(String, u32, String)> = std::collections::HashSet::new();
    let mut seen_cls: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (path, a) in file_metrics {
        let parts: Vec<&str> = path.split('/').collect();
        let short = if parts.len() >= 2 { format!("{}/{}", parts[parts.len()-2], parts[parts.len()-1]) } else { path.clone() };
        let base = parts[parts.len()-1].to_string();
        for f in &a.func_names {
            let key = (base.clone(), f.start_line, f.name.clone());
            if seen_fns.contains(&key) { continue; }
            seen_fns.insert(key);
            if f.lines > 50 { long_fns.push((short.clone(), f.start_line, f.name.clone(), f.lines)); }
            if f.params > 7 { many_param_fns.push((short.clone(), f.start_line, f.name.clone(), f.params)); }
        }
        for c in &a.class_names {
            if c.len() >= 3 && seen_cls.insert(c.clone()) {
                all_classes.push((short.clone(), 0, c.clone()));
            }
        }
    }
    long_fns.sort_by(|a, b| b.3.cmp(&a.3));
    many_param_fns.sort_by(|a, b| b.3.cmp(&a.3));

    let has_org = !large_files.is_empty() || !long_fns.is_empty() || !many_param_fns.is_empty() || !all_classes.is_empty();
    if has_org {
        out.push_str("## 📊 Code Organization\n\n");
        if !large_files.is_empty() {
            let list: Vec<String> = large_files.iter().take(6).map(|(p, l)| {
                let parts: Vec<&str> = p.split('/').collect();
                let f = if parts.len() >= 2 { format!("{}/{}", parts[parts.len()-2], parts[parts.len()-1]) } else { p.to_string() };
                format!("{f}:{l}L")
            }).collect();
            let more = if large_files.len() > 6 { format!(" (+{})", large_files.len() - 6) } else { String::new() };
            let _ = writeln!(out, "**Large files:** {}{more}", list.join(", "));
        }
        if !long_fns.is_empty() {
            let list: Vec<String> = long_fns.iter().take(6).map(|(f, l, n, ln)| format!("{f}:{l}:{n}({ln}L)")).collect();
            let more = if long_fns.len() > 6 { format!(" (+{})", long_fns.len() - 6) } else { String::new() };
            let _ = writeln!(out, "**Long funcs:** {}{more}", list.join(", "));
        }
        if !many_param_fns.is_empty() {
            let list: Vec<String> = many_param_fns.iter().take(6).map(|(f, l, n, p)| format!("{f}:{l}:{n}({p}p)")).collect();
            let more = if many_param_fns.len() > 6 { format!(" (+{})", many_param_fns.len() - 6) } else { String::new() };
            let _ = writeln!(out, "**Many params:** {}{more}", list.join(", "));
        }
        if !all_classes.is_empty() {
            let list: Vec<String> = all_classes.iter().take(8).map(|(f, l, n)| format!("{f}:{l}:{n}")).collect();
            let more = if all_classes.len() > 8 { format!(" (+{})", all_classes.len() - 8) } else { String::new() };
            let _ = writeln!(out, "**Classes:** {}{more}", list.join(", "));
        }
        out.push('\n');
    }

    // ## 🔄 Architecture (layered flow, only when >=3 files)
    if stats.files >= 3 && !dep_graph.coupling.is_empty() {
        out.push_str("## 🔄 Architecture\n\n");
        let mut connections: Vec<(&String, u32, u32)> = dep_graph.coupling.iter()
            .map(|(f, (in_n, out_n))| (f, *out_n, *in_n))
            .collect();
        connections.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)));

        fn fname(p: &str) -> &str { p.rsplit('/').next().unwrap_or(p).split('.').next().unwrap_or(p) }

        let l0: Vec<_> = connections.iter().filter(|(_, o, i)| *o == 0 && *i > 0).collect();
        if !l0.is_empty() {
            let s: Vec<String> = l0.iter().take(8).map(|(f, _, i)| format!("{}({}↓)", fname(f), i)).collect();
            let more = if l0.len() > 8 { format!(" (+{})", l0.len() - 8) } else { String::new() };
            let _ = writeln!(out, "**L0 [pure exports]:** {}{more}", s.join(", "));
        }
        let l1: Vec<_> = connections.iter().filter(|(_, o, i)| *o >= 1 && *o <= 3 && *i >= 3).collect();
        if !l1.is_empty() {
            let s: Vec<String> = l1.iter().take(8).map(|(f, o, i)| format!("{}({}↑{}↓)", fname(f), o, i)).collect();
            let _ = writeln!(out, "**L1 [low imports]:** {}", s.join(", "));
        }
        let l2: Vec<_> = connections.iter().filter(|(_, o, i)| *o >= 4 && *i >= 2).collect();
        if !l2.is_empty() {
            let s: Vec<String> = l2.iter().take(8).map(|(f, o, i)| format!("{}({}↑{}↓)", fname(f), o, i)).collect();
            let _ = writeln!(out, "**L2 [mid flow]:** {}", s.join(", "));
        }
        let l3: Vec<_> = connections.iter().filter(|(_, o, i)| *i == 0 && *o > 0).collect();
        if !l3.is_empty() {
            let s: Vec<String> = l3.iter().take(8).map(|(f, o, _)| format!("{}({}↑)", fname(f), o)).collect();
            let more = if l3.len() > 8 { format!(" (+{})", l3.len() - 8) } else { String::new() };
            let _ = writeln!(out, "**L3 [pure imports]:** {}{more}", s.join(", "));
        }
        if !dep_graph.cross_module_deps.is_empty() {
            let cross: Vec<String> = dep_graph.cross_module_deps.iter().take(6)
                .map(|(a, b)| format!("{a}→{b}")).collect();
            let _ = writeln!(out, "**Cross-module:** {}", cross.join(", "));
        }
        let hubs: Vec<_> = connections.iter().filter(|(_, o, i)| o + i >= 5).take(5).collect();
        if !hubs.is_empty() {
            let s: Vec<String> = hubs.iter().map(|(f, o, i)| format!("{}({}↑{}↓)", fname(f), o, i)).collect();
            let _ = writeln!(out, "**Hubs:** {}", s.join(", "));
        }
        if !dep_graph.circular.is_empty() {
            let cycles: Vec<String> = dep_graph.circular.iter().take(2)
                .map(|cycle| cycle.iter().take(3).map(|f| fname(f)).collect::<Vec<_>>().join("→"))
                .collect();
            let more = if dep_graph.circular.len() > 2 { format!(" (+{} cycles)", dep_graph.circular.len() - 2) } else { String::new() };
            let _ = writeln!(out, "**🔄 Circular:** {}{more}", cycles.join(" | "));
        }
        let mut ext_deps: Vec<_> = dep_graph.external_imports.iter()
            .filter(|(k, _)| !NODE_BUILTINS.contains(&k.as_str()))
            .filter(|(k, _)| !k.starts_with("@/") && !k.starts_with("./") && !k.starts_with("../"))
            .collect();
        ext_deps.sort_by(|a, b| b.1.cmp(a.1));
        if !ext_deps.is_empty() {
            let s: Vec<String> = ext_deps.iter().take(6).map(|(k, _)| k.to_string()).collect();
            let _ = writeln!(out, "**External:** {}", s.join(", "));
        }
        out.push('\n');
    }

    // ## 🔌 API Surface
    let mut exported_fns: Vec<String> = Vec::new();
    for (path, a) in file_metrics {
        let fname = path.rsplit('/').next().unwrap_or(path);
        for f in &a.func_names {
            if a.exported_names.contains(&f.name) {
                exported_fns.push(format!("{}:{}:{}({}p)", fname, f.start_line, f.name, f.params));
            }
        }
    }
    let entry_pts: Vec<String> = dep_graph.entry_points.iter().take(5)
        .map(|e| e.rsplit('/').next().unwrap_or(e).split('.').next().unwrap_or(e).to_string())
        .collect();
    let has_api = !exported_fns.is_empty() || !all_classes.is_empty() || !entry_pts.is_empty();
    if has_api {
        out.push_str("## 🔌 API Surface\n\n");
        if !exported_fns.is_empty() {
            let shown = exported_fns.len().min(12);
            let list = exported_fns[..shown].join(", ");
            let more = if exported_fns.len() > 12 { format!(" (+{})", exported_fns.len() - 12) } else { String::new() };
            let _ = writeln!(out, "**Exported fns:** {list}{more}");
        }
        if !all_classes.is_empty() {
            let list: Vec<String> = all_classes.iter().take(6).map(|(_, _, n)| n.clone()).collect();
            let more = if all_classes.len() > 6 { format!(" (+{})", all_classes.len() - 6) } else { String::new() };
            let _ = writeln!(out, "**Classes:** {}{more}", list.join(", "));
        }
        if !entry_pts.is_empty() {
            let more = if dep_graph.entry_points.len() > 5 { format!(" (+{})", dep_graph.entry_points.len() - 5) } else { String::new() };
            let _ = writeln!(out, "**Entry files:** {}{more}", entry_pts.join(", "));
        }
        out.push('\n');
    }

    // ## 🚨 Issues
    let mut issues = Vec::new();
    let complex_fns: Vec<_> = {
        let mut v: Vec<(String, u32, String, u32)> = Vec::new();
        let mut seen: std::collections::HashSet<(String, u32, String)> = std::collections::HashSet::new();
        for (path, a) in file_metrics {
            let fname = path.rsplit('/').next().unwrap_or(path).to_string();
            for f in &a.func_names {
                let key = (fname.clone(), f.start_line, f.name.clone());
                if f.lines > 100 && seen.insert(key) { v.push((fname.clone(), f.start_line, f.name.clone(), f.lines)); }
            }
        }
        v.sort_by(|a, b| b.3.cmp(&a.3));
        v
    };
    if !complex_fns.is_empty() {
        let list: Vec<String> = complex_fns.iter().take(4).map(|(f, l, n, ln)| format!("{f}:{l}:{n}({ln}L)")).collect();
        let more = if complex_fns.len() > 4 { format!(" (+{})", complex_fns.len() - 4) } else { String::new() };
        issues.push(format!("🔥 Complex funcs: {}{more}", list.join(", ")));
    }
    let lf: Vec<_> = {
        let mut seen: std::collections::HashSet<(String, u32)> = std::collections::HashSet::new();
        file_metrics.iter()
            .filter(|(p, a)| a.stats.lines > 500 && !p.ends_with(".json") && !p.ends_with(".lock"))
            .filter(|(p, a)| seen.insert((p.rsplit('/').next().unwrap_or(p).to_string(), a.stats.lines)))
            .collect()
    };
    if !lf.is_empty() {
        let list: Vec<String> = lf.iter().take(3).map(|(p, a)| {
            let parts: Vec<&str> = p.split('/').collect();
            let f = if parts.len() >= 2 { format!("{}/{}", parts[parts.len()-2], parts[parts.len()-1]) } else { p.to_string() };
            format!("{f}:{}L", a.stats.lines)
        }).collect();
        let more = if lf.len() > 3 { format!(" (+{})", lf.len() - 3) } else { String::new() };
        issues.push(format!("📁 Large files: {}{more}", list.join(", ")));
    }
    if !dep_graph.circular.is_empty() {
        issues.push(format!("🔄 {} circular dep{}", dep_graph.circular.len(), if dep_graph.circular.len() > 1 { "s" } else { "" }));
    }
    if !duplicates.is_empty() {
        issues.push(format!("📋 {} duplicated groups", duplicates.len()));
    }
    if !scans.security.is_empty() {
        let mut by_kind: HashMap<&str, std::collections::BTreeSet<String>> = HashMap::new();
        for issue in &scans.security {
            let f = issue.file.rsplit('/').next().unwrap_or(&issue.file);
            by_kind.entry(&issue.kind).or_default().insert(format!("{f}:{}", issue.line));
        }
        for (kind, locs) in &by_kind {
            let label = match *kind { "eval" => "eval()", "secret" => "hardcoded secrets", "sql_injection" => "SQL injection", _ => kind };
            let list: Vec<&String> = locs.iter().take(6).collect();
            let more = if locs.len() > 6 { format!(" (+{})", locs.len() - 6) } else { String::new() };
            issues.push(format!("🔐 {label} in {}{more}", list.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
        }
    }
    if !issues.is_empty() {
        out.push_str("## 🚨 Issues\n\n");
        for issue in &issues { let _ = writeln!(out, "- {issue}"); }
        out.push('\n');
    }

    // ## 🧹 Dead Code & Tests
    let has_dead = !dead_code.orphaned_files.is_empty() || !dead_code.unused_exports.is_empty() || test_map.test_count > 0;
    if has_dead {
        out.push_str("## 🧹 Dead Code & Tests\n\n");
        if !dead_code.orphaned_files.is_empty() {
            let list: Vec<&str> = dead_code.orphaned_files.iter().take(6)
                .map(|f| f.rsplit('/').next().unwrap_or(f.as_str())).collect();
            let more = if dead_code.orphaned_files.len() > 6 { format!(" (+{})", dead_code.orphaned_files.len() - 6) } else { String::new() };
            let _ = writeln!(out, "**Orphaned:** {}{more}", list.join(", "));
        }
        if !dead_code.unused_exports.is_empty() {
            let mut seen_ue: std::collections::HashSet<String> = std::collections::HashSet::new();
            let deduped_ue: Vec<_> = dead_code.unused_exports.iter()
                .filter(|(f, _)| seen_ue.insert(f.rsplit('/').next().unwrap_or(f).to_string()))
                .take(6).collect();
            let list: Vec<String> = deduped_ue.iter().map(|(f, names)| {
                let fname = f.rsplit('/').next().unwrap_or(f);
                let ns = names.iter().take(2).cloned().collect::<Vec<_>>().join(", ");
                format!("{fname}: {ns}")
            }).collect();
            let more = if dead_code.unused_exports.len() > 6 { format!(" (+{})", dead_code.unused_exports.len() - 6) } else { String::new() };
            let _ = writeln!(out, "**Unused exports:** {}{more}", list.join(" | "));
        }
        if test_map.test_count > 0 || test_map.source_count > 0 {
            let pct = if test_map.source_count > 0 { (test_map.test_count as f64 / test_map.source_count as f64 * 100.0) as u32 } else { 0 };
            let _ = writeln!(out, "**Tests:** {}/{} ({}%)", test_map.test_count, test_map.source_count, pct);
        }
        if !scans.todos.is_empty() || !scans.fixmes.is_empty() || !scans.hacks.is_empty() {
            let mut notes = Vec::new();
            for n in &scans.todos {
                let f = n.file.rsplit('/').next().unwrap_or(&n.file);
                notes.push(if n.text.is_empty() { format!("TODO {f}:{}", n.line) } else { format!("TODO {f}:{} \"{}\"", n.line, n.text) });
            }
            for n in &scans.fixmes {
                let f = n.file.rsplit('/').next().unwrap_or(&n.file);
                notes.push(if n.text.is_empty() { format!("FIXME {f}:{}", n.line) } else { format!("FIXME {f}:{} \"{}\"", n.line, n.text) });
            }
            for n in &scans.hacks {
                let f = n.file.rsplit('/').next().unwrap_or(&n.file);
                notes.push(if n.text.is_empty() { format!("HACK {f}:{}", n.line) } else { format!("HACK {f}:{} \"{}\"", n.line, n.text) });
            }
            let _ = writeln!(out, "**Notes:** {}", notes.join(" | "));
        }
        out.push('\n');
    }

    // ## 📦 Modules
    if stats.files >= 5 && !dep_graph.modules.is_empty() {
        let mut mods: Vec<_> = dep_graph.modules.iter().collect();
        mods.sort_by(|a, b| b.1.connections.cmp(&a.1.connections));
        let mods: Vec<_> = mods.into_iter().take(6).collect();
        if !mods.is_empty() {
            out.push_str("## 📦 Modules\n\n");
            for (name, m) in &mods {
                let _ = writeln!(out, "- {name}: {}f, {}cx, {}↑{}↓", m.files, m.connections, m.imports, m.exports);
            }
            out.push('\n');
        }
    }

    // ## 📄 File Index
    out.push_str("## 📄 File Index\n\n");
    let mut sorted_files: Vec<_> = file_metrics.iter().collect();
    sorted_files.sort_by_key(|(p, _)| p.as_str());
    let file_limit = if sorted_files.len() > 30 { 20 } else { sorted_files.len() };
    let file_overflow = sorted_files.len().saturating_sub(file_limit);
    for (path, a) in &sorted_files[..file_limit] {
        let mut parts = Vec::new();
        if !a.exported_names.is_empty() {
            let names: Vec<&str> = a.exported_names.iter().take(5).map(|s| s.as_str()).collect();
            parts.push(format!("exports: [{}]", names.join("], [")));
        }
        if !a.func_names.is_empty() {
            let fns: Vec<&str> = a.func_names.iter().take(4).map(|f| f.name.as_str()).collect();
            let more = if a.func_names.len() > 4 { format!(" (+{})", a.func_names.len() - 4) } else { String::new() };
            parts.push(format!("fn: {}{more}", fns.join(", ")));
        }
        let line = if parts.is_empty() {
            format!("**{path}** {}L\n", a.stats.lines)
        } else {
            format!("**{path}** {}L {}\n", a.stats.lines, parts.join(" "))
        };
        out.push_str(&line);
    }
    if file_overflow > 0 {
        let _ = writeln!(out, "*+{file_overflow} more files*");
    }

    // Tooling + conventions + git — appended after file index as metadata
    let mut meta = Vec::new();
    if git.is_repo {
        if let Some(ref branch) = git.branch {
            let mut gp = vec![format!("branch: {branch}")];
            if !git.uncommitted.is_empty() { gp.push(format!("{} uncommitted", git.uncommitted.len())); }
            meta.push(format!("Git: {}", gp.join(", ")));
        }
        if !git.hot_files.is_empty() {
            let mut deduped: HashMap<&str, u32> = HashMap::new();
            for (f, c) in &git.hot_files {
                let base = f.rsplit('/').next().unwrap_or(f);
                *deduped.entry(base).or_default() += c;
            }
            let mut hot: Vec<_> = deduped.iter().collect();
            hot.sort_by(|a, b| b.1.cmp(a.1));
            let hot_str: Vec<String> = hot.iter().take(6).map(|(f, c)| format!("{f}({c})")).collect();
            meta.push(format!("Hot: {}", hot_str.join(", ")));
        }
    }
    let mut tool_parts = Vec::new();
    if let Some(ref ts) = tooling.typescript {
        tool_parts.push(format!("TS {}", if ts.strict { "strict" } else { "standard" }));
    }
    tool_parts.extend(tooling.linting.iter().cloned());
    if tooling.has_prettier { tool_parts.push("Prettier".into()); }
    if let Some(ref fw) = tooling.testing { tool_parts.push(fw.clone()); }
    tool_parts.extend(tooling.ci.iter().cloned());
    if tooling.has_dockerfile { tool_parts.push("Docker".into()); }
    if !tool_parts.is_empty() { meta.push(format!("Tooling: {}", tool_parts.join(", "))); }
    if !conventions.is_empty() {
        for conv in conventions {
            meta.push(format!("Conv[{}]: {}", conv.language, conv.conventions.join(", ")));
        }
    }
    if !meta.is_empty() {
        out.push('\n');
        for m in &meta { let _ = writeln!(out, "{m}"); }
    }

    out.trim_end().to_string()
}

fn fmt_k(n: u32) -> String {
    if n >= 1000 { format!("{:.1}k", n as f64 / 1000.0) } else { n.to_string() }
}
