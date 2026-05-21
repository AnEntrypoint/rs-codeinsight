use std::collections::{HashMap, HashSet};
use std::path::Path;

pub struct DepNode {
    pub import_paths: HashSet<String>,
    pub exported_names: HashSet<String>,
    pub imported_by: HashSet<String>,
    pub imports_from: HashSet<String>,
}

pub struct ModuleInfo {
    pub files: u32,
    pub connections: u32,
    pub imports: u32,
    pub exports: u32,
}

pub struct DepGraph {
    pub nodes: HashMap<String, DepNode>,
    pub orphans: HashSet<String>,
    pub entry_points: HashSet<String>,
    pub coupling: HashMap<String, (u32, u32)>,
    pub circular: Vec<Vec<String>>,
    pub cross_module_deps: Vec<(String, String)>,
    pub external_imports: HashMap<String, u32>,
    pub modules: HashMap<String, ModuleInfo>,
}

pub fn build_dep_graph(
    file_analysis: &HashMap<String, (HashSet<String>, HashSet<String>)>,
    path_aliases: &HashMap<String, String>,
    go_modules: &[String],
) -> DepGraph {
    let mut nodes: HashMap<String, DepNode> = HashMap::new();

    for (path, (import_paths, exported_names)) in file_analysis {
        nodes.insert(
            path.clone(),
            DepNode {
                import_paths: import_paths.clone(),
                exported_names: exported_names.clone(),
                imported_by: HashSet::new(),
                imports_from: HashSet::new(),
            },
        );
    }

    let all_files: Vec<String> = nodes.keys().cloned().collect();
    for from_path in &all_files {
        let import_paths: Vec<String> = nodes[from_path].import_paths.iter().cloned().collect();
        let from_dir = Path::new(from_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let is_go = from_path.ends_with(".go");
        let is_py = from_path.ends_with(".py");

        for imp in &import_paths {
            if imp.starts_with("rust_mod:") {
                let mod_name = &imp["rust_mod:".len()..];
                let resolved_list = resolve_rust_import(mod_name, from_path, file_analysis);
                for resolved in resolved_list {
                    if let Some(node) = nodes.get_mut(&resolved) {
                        node.imported_by.insert(from_path.clone());
                    }
                    if let Some(node) = nodes.get_mut(from_path.as_str()) {
                        node.imports_from.insert(resolved);
                    }
                }
                continue;
            }

            if is_go {
                let resolved_list = resolve_go_import(imp, file_analysis, go_modules);
                if !resolved_list.is_empty() {
                    for resolved in resolved_list {
                        if let Some(node) = nodes.get_mut(&resolved) {
                            node.imported_by.insert(from_path.clone());
                        }
                        if let Some(node) = nodes.get_mut(from_path.as_str()) {
                            node.imports_from.insert(resolved);
                        }
                    }
                    continue;
                }
            }

            if is_py {
                let resolved_list = resolve_python_import(imp, &from_dir, file_analysis);
                if !resolved_list.is_empty() {
                    for resolved in resolved_list {
                        if let Some(node) = nodes.get_mut(&resolved) {
                            node.imported_by.insert(from_path.clone());
                        }
                        if let Some(node) = nodes.get_mut(from_path.as_str()) {
                            node.imports_from.insert(resolved);
                        }
                    }
                    continue;
                }
            }

            if let Some(resolved) = resolve_alias_import(imp, path_aliases, file_analysis) {
                if let Some(node) = nodes.get_mut(&resolved) {
                    node.imported_by.insert(from_path.clone());
                }
                if let Some(node) = nodes.get_mut(from_path.as_str()) {
                    node.imports_from.insert(resolved);
                }
                continue;
            }

            if let Some(resolved) = resolve_import(imp, &from_dir, file_analysis) {
                if let Some(node) = nodes.get_mut(&resolved) {
                    node.imported_by.insert(from_path.clone());
                }
                if let Some(node) = nodes.get_mut(from_path.as_str()) {
                    node.imports_from.insert(resolved);
                }
            }
        }
    }

    for _level in 0..2 {
        let snapshot: HashMap<String, (HashSet<String>, HashSet<String>)> = nodes
            .iter()
            .map(|(k, v)| (k.clone(), (v.imports_from.clone(), v.imported_by.clone())))
            .collect();

        for (barrel_path, (barrel_imports_from, barrel_imported_by)) in &snapshot {
            if barrel_imported_by.is_empty() || barrel_imports_from.is_empty() {
                continue;
            }
            for importer in barrel_imported_by {
                for target in barrel_imports_from {
                    if importer == target || importer == barrel_path {
                        continue;
                    }
                    if let Some(node) = nodes.get_mut(target.as_str()) {
                        node.imported_by.insert(importer.clone());
                    }
                    if let Some(node) = nodes.get_mut(importer.as_str()) {
                        node.imports_from.insert(target.clone());
                    }
                }
            }
        }
    }

    let mut orphans = HashSet::new();
    let mut entry_points = HashSet::new();
    let mut coupling = HashMap::new();

    for (path, node) in &nodes {
        if node.imported_by.is_empty() && node.imports_from.is_empty() {
            let fname = Path::new(path)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            if !is_entry_file(&fname) {
                orphans.insert(path.clone());
            }
        }
        if node.imports_from.is_empty() && !node.imported_by.is_empty() {
            entry_points.insert(path.clone());
        }
        let in_count = node.imported_by.len() as u32;
        let out_count = node.imports_from.len() as u32;
        if in_count + out_count > 0 {
            coupling.insert(path.clone(), (in_count, out_count));
        }
    }

    let circular = detect_circular(&nodes);

    let project_dirs: HashSet<String> = file_analysis.keys()
        .filter_map(|p| {
            let normalized = p.replace('\\', "/");
            let first = normalized.split('/').next()?;
            if normalized.contains('/') { Some(first.to_string()) } else { None }
        })
        .collect();

    let mut external_imports: HashMap<String, u32> = HashMap::new();
    for (_path, (import_paths, _exported_names)) in file_analysis {
        for imp in import_paths {
            if !imp.starts_with('.') {
                if imp.starts_with("@/") {
                    continue;
                }
                if imp.starts_with("rust_mod:") {
                    continue;
                }
                if imp.contains("/internal/") || imp.contains("/handlers/") || imp.contains("/cmd/") {
                    continue;
                }
                let first_component = imp.split('/').next().unwrap_or(imp);
                if !imp.starts_with('@') && !imp.starts_with("github.com") && project_dirs.contains(first_component) {
                    continue;
                }

                let pkg = if imp.starts_with('@') {
                    let parts: Vec<&str> = imp.splitn(3, '/').collect();
                    if parts.len() >= 2 {
                        format!("{}/{}", parts[0], parts[1])
                    } else {
                        imp.clone()
                    }
                } else if imp.starts_with("github.com/") {
                    imp.rsplit('/').next().unwrap_or(imp).to_string()
                } else {
                    imp.split('/').next().unwrap_or(imp).to_string()
                };
                *external_imports.entry(pkg).or_insert(0) += 1;
            }
        }
    }

    let mut cross_module_deps: Vec<(String, String)> = Vec::new();
    let mut seen_pairs: HashSet<(String, String)> = HashSet::new();
    for (from_path, node) in &nodes {
        let from_module = first_path_component(from_path);
        for to_path in &node.imports_from {
            let to_module = first_path_component(to_path);
            if from_module != to_module && !from_module.is_empty() && !to_module.is_empty() {
                let pair = if from_module < to_module {
                    (from_module.clone(), to_module.clone())
                } else {
                    (to_module.clone(), from_module.clone())
                };
                if seen_pairs.insert(pair) {
                    cross_module_deps.push((from_module.clone(), to_module));
                }
            }
        }
    }

    let mut modules: HashMap<String, ModuleInfo> = HashMap::new();
    for (path, node) in &nodes {
        let module_name = first_path_component(path);
        if module_name.is_empty() || module_name == path.as_str() {
            continue;
        }
        let info = modules.entry(module_name).or_insert_with(|| ModuleInfo {
            files: 0,
            connections: 0,
            imports: 0,
            exports: 0,
        });
        info.files += 1;
        info.connections += node.imported_by.len() as u32 + node.imports_from.len() as u32;
        info.imports += node.imports_from.len() as u32;
        info.exports += node.imported_by.len() as u32;
    }

    DepGraph { nodes, orphans, entry_points, coupling, circular, cross_module_deps, external_imports, modules }
}

fn resolve_import(
    import_path: &str,
    from_dir: &str,
    files: &HashMap<String, (HashSet<String>, HashSet<String>)>,
) -> Option<String> {
    if import_path.starts_with('.') {
        let joined = if from_dir.is_empty() {
            import_path.trim_start_matches("./").to_string()
        } else {
            format!("{}/{}", from_dir, import_path.trim_start_matches("./"))
        };
        let clean = joined.replace('\\', "/");

        if files.contains_key(&clean) {
            return Some(clean);
        }

        let exts = [".js", ".ts", ".jsx", ".tsx", ".mjs", ".cjs"];
        let no_ext = clean
            .trim_end_matches(".js")
            .trim_end_matches(".ts")
            .trim_end_matches(".jsx")
            .trim_end_matches(".tsx")
            .trim_end_matches(".mjs")
            .trim_end_matches(".cjs");

        for ext in &exts {
            let with_ext = format!("{}{}", no_ext, ext);
            if files.contains_key(&with_ext) {
                return Some(with_ext);
            }
        }

        let idx_exts = ["/index.js", "/index.ts", "/index.jsx", "/index.tsx"];
        for ext in &idx_exts {
            let with_idx = format!("{}{}", clean.trim_end_matches('/'), ext);
            if files.contains_key(&with_idx) {
                return Some(with_idx);
            }
        }
    }

    None
}

fn resolve_alias_import(
    import_path: &str,
    path_aliases: &HashMap<String, String>,
    files: &HashMap<String, (HashSet<String>, HashSet<String>)>,
) -> Option<String> {
    for (alias_prefix, replacement) in path_aliases {
        if import_path.starts_with(alias_prefix.as_str()) {
            let rest = &import_path[alias_prefix.len()..];
            let resolved_base = format!("{}{}", replacement, rest).replace('\\', "/");

            if files.contains_key(&resolved_base) {
                return Some(resolved_base);
            }

            let exts = [".js", ".ts", ".jsx", ".tsx", ".mjs", ".cjs"];
            let no_ext = resolved_base
                .trim_end_matches(".js")
                .trim_end_matches(".ts")
                .trim_end_matches(".jsx")
                .trim_end_matches(".tsx")
                .trim_end_matches(".mjs")
                .trim_end_matches(".cjs");

            for ext in &exts {
                let with_ext = format!("{}{}", no_ext, ext);
                if files.contains_key(&with_ext) {
                    return Some(with_ext);
                }
            }

            let idx_exts = ["/index.js", "/index.ts", "/index.jsx", "/index.tsx"];
            for ext in &idx_exts {
                let with_idx = format!("{}{}", resolved_base.trim_end_matches('/'), ext);
                if files.contains_key(&with_idx) {
                    return Some(with_idx);
                }
            }

            let top_dirs: HashSet<String> = files
                .keys()
                .filter_map(|p| {
                    let normalized = p.replace('\\', "/");
                    if normalized.contains('/') {
                        Some(normalized.split('/').next()?.to_string())
                    } else {
                        None
                    }
                })
                .collect();

            for top in &top_dirs {
                let prefixed = format!("{}/{}", top, resolved_base);

                if files.contains_key(&prefixed) {
                    return Some(prefixed);
                }
                for ext in &exts {
                    let with_ext = format!("{}{}", prefixed.trim_end_matches(".js")
                        .trim_end_matches(".ts")
                        .trim_end_matches(".jsx")
                        .trim_end_matches(".tsx"), ext);
                    if files.contains_key(&with_ext) {
                        return Some(with_ext);
                    }
                }
                for ext in &idx_exts {
                    let with_idx = format!("{}{}", prefixed.trim_end_matches('/'), ext);
                    if files.contains_key(&with_idx) {
                        return Some(with_idx);
                    }
                }
            }
        }
    }
    None
}

fn resolve_go_import(
    import_path: &str,
    files: &HashMap<String, (HashSet<String>, HashSet<String>)>,
    go_modules: &[String],
) -> Vec<String> {
    let mut results = Vec::new();

    for module_path in go_modules {
        if import_path.starts_with(module_path.as_str()) {
            let rest = import_path[module_path.len()..]
                .trim_start_matches('/')
                .replace('\\', "/");

            for file_path in files.keys() {
                let normalized = file_path.replace('\\', "/");
                if normalized.ends_with(".go") {

                    if let Some(parent) = normalized.rsplit_once('/') {
                        if parent.0 == rest || parent.0.ends_with(&format!("/{}", rest)) {
                            results.push(file_path.clone());
                        }
                    }
                }
            }
        }
    }

    results
}

fn resolve_python_import(
    import_path: &str,
    from_dir: &str,
    files: &HashMap<String, (HashSet<String>, HashSet<String>)>,
) -> Vec<String> {
    let mut results = Vec::new();

    let as_path = import_path.replace('.', "/");

    let mut candidates: Vec<String> = Vec::new();

    if !from_dir.is_empty() {
        candidates.push(format!("{}/{}.py", from_dir, as_path));
        candidates.push(format!("{}/{}/__init__.py", from_dir, as_path));
    }

    candidates.push(format!("{}.py", as_path));
    candidates.push(format!("{}/__init__.py", as_path));

    let top_dirs: HashSet<String> = files
        .keys()
        .filter_map(|p| {
            let normalized = p.replace('\\', "/");
            if normalized.contains('/') {
                Some(normalized.split('/').next()?.to_string())
            } else {
                None
            }
        })
        .collect();

    for top in &top_dirs {
        candidates.push(format!("{}/{}.py", top, as_path));
        candidates.push(format!("{}/{}/__init__.py", top, as_path));
    }

    for candidate in &candidates {
        let normalized = candidate.replace('\\', "/");
        if files.contains_key(&normalized) {
            if !results.contains(&normalized) {
                results.push(normalized);
            }
        }
    }

    results
}

fn resolve_rust_import(
    mod_name: &str,
    from_path: &str,
    files: &HashMap<String, (HashSet<String>, HashSet<String>)>,
) -> Vec<String> {
    let mut results = Vec::new();
    let normalized_from = from_path.replace('\\', "/");

    let from_dir = if let Some((dir, _file)) = normalized_from.rsplit_once('/') {
        dir.to_string()
    } else {
        String::new()
    };

    let mut candidates: Vec<String> = Vec::new();

    if from_dir.is_empty() {
        candidates.push(format!("{}.rs", mod_name));
        candidates.push(format!("{}/mod.rs", mod_name));
    } else {
        candidates.push(format!("{}/{}.rs", from_dir, mod_name));
        candidates.push(format!("{}/{}/mod.rs", from_dir, mod_name));
    }

    for candidate in &candidates {
        if files.contains_key(candidate) && !results.contains(candidate) {
            results.push(candidate.clone());
        }
    }

    results
}

fn detect_circular(nodes: &HashMap<String, DepNode>) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();

    for node_key in nodes.keys() {
        if !visited.contains(node_key) {
            dfs(node_key, nodes, &mut vec![], &mut visiting, &mut visited, &mut cycles);
        }
    }

    cycles.truncate(5);
    cycles
}

fn dfs(
    node: &str,
    nodes: &HashMap<String, DepNode>,
    path: &mut Vec<String>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    if visiting.contains(node) {
        if let Some(start) = path.iter().position(|p| p == node) {
            let mut cycle: Vec<String> = path[start..].to_vec();
            cycle.push(node.to_string());
            cycles.push(cycle);
        }
        return;
    }
    if visited.contains(node) {
        return;
    }

    visiting.insert(node.to_string());
    path.push(node.to_string());

    if let Some(n) = nodes.get(node) {
        for dep in &n.imports_from {
            dfs(dep, nodes, path, visiting, visited, cycles);
        }
    }

    path.pop();
    visiting.remove(node);
    visited.insert(node.to_string());
}

fn first_path_component(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    normalized.split('/').next().unwrap_or("").to_string()
}

fn is_entry_file(name: &str) -> bool {
    let patterns = [
        "index.", "main.", "app.", "server.", "client.",
        "start.", "cli.", "bin.", "boot.", "init.", "entry.", "lib.",
    ];
    patterns.iter().any(|p| name.contains(p))
}

pub struct DeadCode {
    pub orphaned_files: Vec<String>,
    pub unused_exports: Vec<(String, Vec<String>)>,
    pub test_files: Vec<String>,
    pub possibly_dead: Vec<(String, String)>,
}

pub fn detect_dead_code(graph: &DepGraph) -> DeadCode {
    let mut dead = DeadCode {
        orphaned_files: Vec::new(),
        unused_exports: Vec::new(),
        test_files: Vec::new(),
        possibly_dead: Vec::new(),
    };

    let mut reexport_targets: HashSet<String> = HashSet::new();
    for (path, node) in &graph.nodes {
        let fname = Path::new(path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let is_reexporter = (fname.contains("index.") || fname.contains("lib.") || fname.contains("main."))
            && !node.imports_from.is_empty()
            && !node.exported_names.is_empty();
        if is_reexporter {
            for target in &node.imports_from {
                reexport_targets.insert(target.clone());
            }
        }
    }

    for (path, node) in &graph.nodes {
        let fname = Path::new(path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        if fname.contains(".test.") || fname.contains(".spec.")
            || path.contains("/test/") || path.contains("/__tests__/")
        {
            dead.test_files.push(path.clone());
            continue;
        }

        let is_nextjs_page = matches!(fname.as_str(),
            "page.tsx" | "page.ts" | "page.jsx" | "page.js"
            | "layout.tsx" | "layout.ts" | "loading.tsx"
            | "error.tsx" | "not-found.tsx")
            || path.contains("/app/") || path.contains("/pages/");

        let is_go_file = fname.ends_with(".go");

        let is_config_file = fname.contains(".config.") || fname.contains(".setup.")
            || fname.starts_with("tailwind.") || fname.starts_with("postcss.")
            || fname.starts_with("next.config.") || fname.starts_with("vite.config.")
            || fname.starts_with("tsconfig.");

        let skip_orphan_check = is_nextjs_page || is_go_file || is_config_file;

        let has_importers = !node.imported_by.is_empty() || reexport_targets.contains(path);

        if !has_importers && !node.exported_names.is_empty() && !is_entry_file(&fname) && !skip_orphan_check {
            let fname_lower = fname.to_lowercase();
            if !fname_lower.contains("config") {
                let exports: Vec<String> = node.exported_names.iter().take(3).cloned().collect();
                dead.unused_exports.push((path.clone(), exports));
            }
        }

        if !has_importers && node.imports_from.is_empty() && !is_entry_file(&fname) && !skip_orphan_check {
            dead.orphaned_files.push(path.clone());
        }
    }

    for (path, node) in &graph.nodes {
        let fname = Path::new(path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        if fname.contains(".test.") || fname.contains(".spec.")
            || path.contains("/test/") || path.contains("/__tests__/")
        {
            continue;
        }

        if node.imported_by.len() == 1 && node.imports_from.is_empty() && !is_entry_file(&fname) {
            let single_importer = node.imported_by.iter().next().unwrap().clone();
            let importer_fname = Path::new(&single_importer)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            dead.possibly_dead.push((path.clone(), importer_fname));
        }
    }

    dead
}
