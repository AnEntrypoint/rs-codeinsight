use std::collections::{HashMap, HashSet};
use md5::{Md5, Digest};
use tree_sitter::{Node, Tree};

#[derive(Default)]
pub struct FileStats {
    pub functions: u32,
    pub classes: u32,
    pub imports: u32,
    pub exports: u32,
    pub complexity: u32,
    pub lines: u32,
}

#[derive(Default)]
pub struct FuncInfo {
    pub name: String,
    pub start_line: u32,
    pub lines: u32,
    pub params: u32,
}

#[derive(Default)]
pub struct FileAnalysis {
    pub stats: FileStats,
    pub func_names: Vec<FuncInfo>,
    pub class_names: Vec<String>,
    pub import_paths: HashSet<String>,
    pub exported_names: HashSet<String>,
    pub func_hashes: HashMap<String, String>,
    pub max_depth: u32,
    pub branches: u32,
    pub call_patterns: HashMap<String, u32>,
    pub async_count: u32,
    pub await_count: u32,
    pub promise_count: u32,
    pub callback_count: u32,
    pub try_catch_count: u32,
    pub throw_count: u32,
    pub constants: Vec<(String, String)>,
    pub global_state: Vec<String>,
    pub env_vars: HashSet<String>,
    pub urls: HashSet<String>,
    pub file_io_count: u32,
    pub json_op_count: u32,
    pub sql_count: u32,
    pub http_routes: Vec<String>,
    pub fetch_count: u32,
    pub event_listeners: u32,
    pub event_emitters: u32,
    pub identifiers: HashMap<String, u32>,
}

pub fn analyze_tree(tree: &Tree, source: &str) -> FileAnalysis {
    let mut analysis = FileAnalysis::default();
    analysis.stats.lines = source.lines().count() as u32;
    traverse(tree.root_node(), source, &mut analysis, 0);
    analysis
}

fn traverse(node: Node, source: &str, analysis: &mut FileAnalysis, depth: u32) {
    if depth > analysis.max_depth {
        analysis.max_depth = depth;
    }

    let kind = node.kind();

    if (kind.contains("function") && kind.contains("declaration"))
        || kind == "method_definition"
        || kind == "function_item"
    {
        analysis.stats.functions += 1;
        let name = extract_name(node, source).unwrap_or_else(|| "anon".into());
        let start_line = node.start_position().row as u32 + 1;
        let text = node_text(node, source);
        let lines = text.lines().count() as u32;
        let params = count_params(node, source);
        let hash = structural_hash(node, source);
        let sig = format!("{}({})", name, params);
        analysis.func_hashes.insert(sig, hash);
        analysis.func_names.push(FuncInfo { name, start_line, lines, params });
    }

    if (kind.contains("class") && kind.contains("declaration"))
        || kind == "struct_item"
        || kind == "enum_item"
        || kind == "interface_declaration"
    {
        analysis.stats.classes += 1;
        if let Some(name) = extract_name(node, source) {
            analysis.class_names.push(name);
        }
    }

    if kind.contains("import") {
        analysis.stats.imports += 1;
        if let Some(path) = extract_import_path(node, source) {
            analysis.import_paths.insert(path);
        }
    }

    if kind.contains("export") {
        analysis.stats.exports += 1;
        if let Some(name) = extract_name(node, source) {
            analysis.exported_names.insert(name);
        }
    }

    if kind == "call_expression" {
        if let Some(func_node) = node.child(0) {
            let text = node_text(func_node, source);
            if text == "require" || text == "import" {
                if let Some(args) = node.child_by_field_name("arguments") {
                    for i in 0..args.child_count() {
                        if let Some(arg) = args.child(i) {
                            if arg.kind() == "string" || arg.kind() == "string_literal" {
                                let path = node_text(arg, source)
                                    .trim_matches(|c| c == '\'' || c == '"')
                                    .to_string();
                                if !path.contains("${") {
                                    analysis.import_paths.insert(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if kind == "assignment_expression" || kind == "expression_statement" {
        let text = node_text(node, source);
        if text.starts_with("module.exports") || text.starts_with("exports.") {
            if let Some(cap) = text.find('{') {
                if let Some(end) = text.find('}') {
                    let inner = &text[cap + 1..end];
                    for name in inner.split(',') {
                        let n = name.split(':').next().unwrap_or("").trim();
                        if !n.is_empty() {
                            analysis.exported_names.insert(n.to_string());
                        }
                    }
                }
            }
        }
    }

    match kind {
        "if_statement" | "while_statement" | "for_statement"
        | "case_statement" | "catch_clause" | "switch_statement"
        | "conditional_expression" => {
            analysis.stats.complexity += 1;
            analysis.branches += 1;
        }
        _ => {}
    }

    // --- call_patterns, file I/O, JSON ops, SQL, fetch, HTTP routes, event patterns, promise ---
    if kind == "call_expression" {
        if let Some(func_node) = node.child(0) {
            let callee = node_text(func_node, source);

            // call_patterns: record function name if under 30 chars
            if callee.len() <= 30 {
                *analysis.call_patterns.entry(callee.to_string()).or_insert(0) += 1;
            }

            // promise detection
            if callee == "Promise" || callee.starts_with("Promise.") {
                analysis.promise_count += 1;
            }

            // fetch detection
            if callee == "fetch" {
                analysis.fetch_count += 1;
            }

            // file I/O detection
            match callee {
                c if c.contains("readFile")
                    || c.contains("writeFile")
                    || c.contains("readdir")
                    || c.contains("mkdir")
                    || c.contains("unlink")
                    || c.contains("stat") =>
                {
                    analysis.file_io_count += 1;
                }
                _ => {}
            }

            // JSON ops detection
            if callee == "JSON.parse" || callee == "JSON.stringify" {
                analysis.json_op_count += 1;
            }

            // SQL detection
            if callee.ends_with(".query")
                || callee.ends_with(".execute")
                || callee.contains("SELECT")
                || callee.contains("INSERT")
            {
                analysis.sql_count += 1;
            }

            // event listeners: .on, .addEventListener
            if callee.ends_with(".on") || callee.ends_with(".addEventListener") {
                analysis.event_listeners += 1;
            }

            // event emitters: .emit, .dispatch
            if callee.ends_with(".emit") || callee.ends_with(".dispatch") {
                analysis.event_emitters += 1;
            }

            // HTTP routes: .get/.post/.put/.delete/.patch with a route string argument
            let route_methods = [".get", ".post", ".put", ".delete", ".patch"];
            for method in &route_methods {
                if callee.ends_with(method) {
                    if let Some(args) = node.child_by_field_name("arguments") {
                        for ai in 0..args.child_count() {
                            if let Some(arg) = args.child(ai) {
                                if arg.kind() == "string"
                                    || arg.kind() == "string_literal"
                                    || arg.kind() == "template_string"
                                {
                                    let val = node_text(arg, source)
                                        .trim_matches(|c| c == '\'' || c == '"' || c == '`')
                                        .to_string();
                                    if val.starts_with('/') {
                                        analysis.http_routes.push(val);
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // --- async / await ---
    if kind == "await_expression" {
        analysis.await_count += 1;
    }
    if kind.contains("async") {
        analysis.async_count += 1;
    } else {
        let text = node_text(node, source);
        if text.starts_with("async ") {
            analysis.async_count += 1;
        }
    }

    // --- callbacks: arrow_function or function_expression whose parent is "arguments" ---
    if kind == "arrow_function" || kind == "function_expression" {
        if let Some(parent) = node.parent() {
            if parent.kind() == "arguments" {
                analysis.callback_count += 1;
            }
        }
    }

    // --- try / throw ---
    if kind == "try_statement" {
        analysis.try_catch_count += 1;
    }
    if kind == "throw_statement" {
        analysis.throw_count += 1;
    }

    // --- constants: module-level const declarations ---
    if kind == "lexical_declaration" {
        let text = node_text(node, source);
        if let Some(parent) = node.parent() {
            let pk = parent.kind();
            if pk == "program" || pk == "export_statement" {
                if text.starts_with("const ") {
                    // extract name = value from "const NAME = VALUE..."
                    let rest = &text["const ".len()..];
                    if let Some(eq_pos) = rest.find('=') {
                        let name = rest[..eq_pos].trim().to_string();
                        let value = rest[eq_pos + 1..].trim().trim_end_matches(';').trim().to_string();
                        if !name.is_empty() {
                            analysis.constants.push((name, value));
                        }
                    }
                } else if text.starts_with("let ") {
                    // global_state: module-level let
                    let rest = &text["let ".len()..];
                    let name = rest.split(&['=', ';', ' ', ','][..]).next().unwrap_or("").trim().to_string();
                    if !name.is_empty() {
                        analysis.global_state.push(name);
                    }
                }
            }
        }
    }

    // --- global_state: module-level var (variable_declaration) ---
    if kind == "variable_declaration" {
        if let Some(parent) = node.parent() {
            let pk = parent.kind();
            if pk == "program" || pk == "export_statement" {
                let text = node_text(node, source);
                let rest = text.trim_start_matches("var ").trim_start_matches("let ");
                let name = rest.split(&['=', ';', ' ', ','][..]).next().unwrap_or("").trim().to_string();
                if !name.is_empty() {
                    analysis.global_state.push(name);
                }
            }
        }
    }

    // --- env_vars: process.env.WORD ---
    {
        let text = node_text(node, source);
        let mut search_from = 0;
        let needle = "process.env.";
        while let Some(pos) = text[search_from..].find(needle) {
            let start = search_from + pos + needle.len();
            let var_name: String = text[start..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !var_name.is_empty() {
                analysis.env_vars.insert(var_name);
            }
            search_from = start;
        }
    }

    // --- urls: http(s) URLs in string/template_string nodes ---
    if kind == "string" || kind == "string_literal" || kind == "template_string" {
        let text = node_text(node, source);
        for prefix in &["https://", "http://"] {
            let mut search_from = 0;
            while let Some(pos) = text[search_from..].find(prefix) {
                let start = search_from + pos;
                let url: String = text[start..]
                    .chars()
                    .take_while(|c| !c.is_whitespace() && *c != '\'' && *c != '"' && *c != '`' && *c != ')')
                    .collect();
                if !url.is_empty() {
                    analysis.urls.insert(url);
                }
                search_from = start + prefix.len();
            }
        }
    }

    // --- SQL in string literals ---
    if kind == "string" || kind == "string_literal" || kind == "template_string" {
        let text = node_text(node, source);
        let upper = text.to_uppercase();
        if upper.contains("SELECT ") || upper.contains("INSERT ") {
            analysis.sql_count += 1;
        }
    }

    // --- identifiers: count all identifier/property_identifier/type_identifier under 50 chars ---
    if kind == "identifier" || kind == "property_identifier" || kind == "type_identifier" {
        let text = node_text(node, source);
        if text.len() < 50 {
            *analysis.identifiers.entry(text.to_string()).or_insert(0) += 1;
        }
    }

    let count = node.child_count();
    for i in 0..count {
        if let Some(child) = node.child(i) {
            traverse(child, source, analysis, depth + 1);
        }
    }
}

fn extract_name(node: Node, source: &str) -> Option<String> {
    if node.kind() == "identifier"
        || node.kind() == "property_identifier"
        || node.kind() == "type_identifier"
    {
        return Some(node_text(node, source).to_string());
    }
    let count = node.child_count();
    for i in 0..count {
        if let Some(child) = node.child(i) {
            if child.kind().contains("identifier") {
                return Some(node_text(child, source).to_string());
            }
        }
    }
    None
}

fn extract_import_path(node: Node, source: &str) -> Option<String> {
    let count = node.child_count();
    for i in 0..count {
        if let Some(child) = node.child(i) {
            if child.kind() == "string"
                || child.kind() == "string_literal"
                || child.kind() == "interpreted_string_literal"
            {
                let path = node_text(child, source)
                    .trim_matches(|c| c == '\'' || c == '"')
                    .to_string();
                return Some(path);
            }
            if let Some(found) = extract_import_path(child, source) {
                return Some(found);
            }
        }
    }
    None
}

fn count_params(node: Node, _source: &str) -> u32 {
    let mut count = 0;
    fn walk(n: Node, count: &mut u32) {
        if n.kind() == "parameter"
            || n.kind() == "formal_parameter"
            || n.kind().contains("param")
        {
            *count += 1;
        }
        for i in 0..n.child_count() {
            if let Some(child) = n.child(i) {
                walk(child, count);
            }
        }
    }
    walk(node, &mut count);
    count
}

fn structural_hash(node: Node, _source: &str) -> String {
    let mut structure = Vec::new();
    fn walk(n: Node, out: &mut Vec<u8>) {
        let kind = n.kind();
        if !kind.contains("identifier") && !kind.contains("comment") {
            out.extend_from_slice(kind.as_bytes());
            out.push(b':');
        }
        for i in 0..n.child_count() {
            if let Some(child) = n.child(i) {
                walk(child, out);
            }
        }
    }
    walk(node, &mut structure);
    let mut hasher = Md5::new();
    hasher.update(&structure);
    let result = hasher.finalize();
    format!("{:x}", result)[..8].to_string()
}

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    let start = node.start_byte();
    let end = node.end_byte().min(source.len());
    &source[start..end]
}
