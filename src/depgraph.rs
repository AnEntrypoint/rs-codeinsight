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

        for imp in &import_paths {
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

    // Collect first-level directory names from the project for filtering local paths
    let project_dirs: HashSet<String> = file_analysis.keys()
        .filter_map(|p| {
            let normalized = p.replace('\\', "/");
            let first = normalized.split('/').next()?;
            if normalized.contains('/') { Some(first.to_string()) } else { None }
        })
        .collect();

    // Build external_imports: non-relative import paths with counts
    let mut external_imports: HashMap<String, u32> = HashMap::new();
    for (_path, (import_paths, _exported_names)) in file_analysis {
        for imp in import_paths {
            if !imp.starts_with('.') {
                // Filter out path aliases (e.g. @/ prefix)
                if imp.starts_with("@/") {
                    continue;
                }
                // Filter out Go local paths
                if imp.contains("/internal/") || imp.contains("/handlers/") || imp.contains("/cmd/") {
                    continue;
                }
                // Filter out imports that match project directory names
                let first_component = imp.split('/').next().unwrap_or(imp);
                if !imp.starts_with('@') && !imp.starts_with("github.com") && project_dirs.contains(first_component) {
                    continue;
                }

                // Use the first segment as the package name (e.g. "@foo/bar" -> "@foo/bar", "express" -> "express")
                let pkg = if imp.starts_with('@') {
                    // Scoped package: take first two segments
                    let parts: Vec<&str> = imp.splitn(3, '/').collect();
                    if parts.len() >= 2 {
                        format!("{}/{}", parts[0], parts[1])
                    } else {
                        imp.clone()
                    }
                } else if imp.starts_with("github.com/") {
                    // Go import: extract last path component for display
                    imp.rsplit('/').next().unwrap_or(imp).to_string()
                } else {
                    imp.split('/').next().unwrap_or(imp).to_string()
                };
                *external_imports.entry(pkg).or_insert(0) += 1;
            }
        }
    }

    // Build cross_module_deps: pairs where first path component differs
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

    // Build modules: group files by first path component
    let mut modules: HashMap<String, ModuleInfo> = HashMap::new();
    for (path, node) in &nodes {
        let module_name = first_path_component(path);
        if module_name.is_empty() || module_name == path.as_str() {
            // File at root, use filename without extension as module
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

    // Identify re-exporters: files whose name contains "index." or "lib." or "main."
    // that both import from and export. For each re-exporter, add a virtual "reexport"
    // marker to the imported_by set of the files it imports from.
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

        // Skip framework-conventional files from orphan/dead detection
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

        // Check importers: a file counts as "imported" if it has real importers
        // OR is re-exported through an index/lib/main file
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

    // Detect possibly dead: files with exactly 1 importer and 0 imports of their own (leaf nodes)
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
