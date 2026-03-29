use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Parse tsconfig.json (and subdirectory tsconfigs one level deep) to extract
/// path alias mappings.  Returns a map like `"@/" -> "src/"`.
pub fn parse_tsconfig_paths(root: &Path) -> HashMap<String, String> {
    let mut aliases: HashMap<String, String> = HashMap::new();

    // Collect candidate tsconfig files: root + immediate subdirectories
    let mut candidates: Vec<std::path::PathBuf> = vec![root.join("tsconfig.json")];
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                candidates.push(entry.path().join("tsconfig.json"));
            }
        }
    }

    for tsconfig_path in &candidates {
        if let Ok(content) = fs::read_to_string(tsconfig_path) {
            parse_tsconfig_paths_from_content(&content, &mut aliases);
        }
    }

    aliases
}

fn parse_tsconfig_paths_from_content(content: &str, aliases: &mut HashMap<String, String>) {
    // Find "paths" object inside "compilerOptions"
    let compiler_pos = match content.find("\"compilerOptions\"") {
        Some(p) => p,
        None => return,
    };
    let after_compiler = &content[compiler_pos..];

    // Find the opening brace of compilerOptions value
    let brace_start = match after_compiler.find('{') {
        Some(b) => compiler_pos + b,
        None => return,
    };

    // Find matching closing brace (simple nesting counter)
    let co_block = match find_matching_brace(&content[brace_start..]) {
        Some(end) => &content[brace_start..brace_start + end + 1],
        None => return,
    };

    // Find "paths" inside compilerOptions block
    let paths_pos = match co_block.find("\"paths\"") {
        Some(p) => p,
        None => return,
    };
    let after_paths = &co_block[paths_pos..];

    let paths_brace = match after_paths.find('{') {
        Some(b) => b,
        None => return,
    };

    let paths_block = match find_matching_brace(&after_paths[paths_brace..]) {
        Some(end) => &after_paths[paths_brace + 1..paths_brace + end],
        None => return,
    };

    // Parse entries like `"@/*": ["./src/*"]` or `"@components/*": ["components/*"]`
    // Split by lines for simpler parsing
    for line in paths_block.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        // Expect pattern: "alias_pattern": ["replacement_pattern"]
        if !trimmed.contains(':') {
            continue;
        }
        let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }
        let key = parts[0].trim().trim_matches('"');
        let val_part = parts[1].trim();

        // Extract the first value from the array ["./src/*"]
        if !val_part.starts_with('[') {
            continue;
        }
        let inner = val_part
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim();
        let first_val = inner
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"');

        if key.is_empty() || first_val.is_empty() {
            continue;
        }

        // Convert glob patterns to prefix/replacement:
        //   "@/*"       -> alias_prefix = "@/"
        //   "./src/*"   -> replacement  = "src/"
        //   "src/*"     -> replacement  = "src/"
        let alias_prefix = key.trim_end_matches('*');
        let replacement = first_val
            .trim_start_matches("./")
            .trim_end_matches('*');

        // Ensure both end with /
        let alias_prefix = if alias_prefix.ends_with('/') {
            alias_prefix.to_string()
        } else if alias_prefix.is_empty() {
            continue;
        } else {
            format!("{}/", alias_prefix.trim_end_matches('/'))
        };

        let replacement = if replacement.ends_with('/') {
            replacement.to_string()
        } else if replacement.is_empty() {
            String::new()
        } else {
            format!("{}/", replacement.trim_end_matches('/'))
        };

        aliases.entry(alias_prefix).or_insert(replacement);
    }
}

fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Default)]
pub struct ProjectContext {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub project_type: String,
    pub framework: Option<String>,
    pub scripts: HashMap<String, String>,
    pub dependencies: Vec<String>,
    pub dev_dependencies: Vec<String>,
    pub package_manager: Option<String>,
    pub readme_excerpt: Option<String>,
    pub frameworks: Vec<String>,
    pub go_modules: Vec<String>,
}

pub fn analyze_project(root: &Path) -> ProjectContext {
    let mut ctx = ProjectContext {
        project_type: "unknown".into(),
        ..Default::default()
    };

    let pkg_path = root.join("package.json");
    if let Ok(content) = fs::read_to_string(&pkg_path) {
        parse_package_json(&content, &mut ctx);
    }

    // Check root and immediate subdirs for README
    let mut readme_dirs = vec![root.to_path_buf()];
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                readme_dirs.push(entry.path());
            }
        }
    }
    'readme: for dir in &readme_dirs {
        for name in &["README.md", "readme.md", "README.txt", "README"] {
            let readme_path = dir.join(name);
            if let Ok(content) = fs::read_to_string(&readme_path) {
                let excerpt: String = content
                    .split("\n\n")
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .replace('\n', " ")
                    .trim_start_matches(|c: char| c == '#' || c == ' ')
                    .chars()
                    .take(300)
                    .collect();
                let trimmed = excerpt.trim().to_string();
                if !trimmed.is_empty() {
                    ctx.readme_excerpt = Some(trimmed);
                    break 'readme;
                }
            }
        }
    }

    if root.join("yarn.lock").exists() {
        ctx.package_manager = Some("yarn".into());
    } else if root.join("pnpm-lock.yaml").exists() {
        ctx.package_manager = Some("pnpm".into());
    } else if root.join("bun.lockb").exists() || root.join("bun.lock").exists() || root.join("bunfig.toml").exists() {
        ctx.package_manager = Some("bun".into());
    } else if root.join("package-lock.json").exists() {
        ctx.package_manager = Some("npm".into());
    }

    // Add root framework if detected
    if let Some(ref fw) = ctx.framework {
        if !ctx.frameworks.contains(fw) {
            ctx.frameworks.push(fw.clone());
        }
    }

    // Scan immediate subdirectories for package.json and go.mod
    let js_frameworks: &[(&str, &str)] = &[
        ("next", "Next.js"), ("react", "React"), ("vue", "Vue"),
        ("express", "Express"), ("fastify", "Fastify"), ("koa", "Koa"),
        ("svelte", "Svelte"), ("@angular/core", "Angular"),
        ("nuxt", "Nuxt"), ("remix", "Remix"), ("astro", "Astro"),
        ("hono", "Hono"), ("elysia", "Elysia"),
    ];

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }

            // Check subdirectory package.json
            let sub_pkg = path.join("package.json");
            if let Ok(content) = fs::read_to_string(&sub_pkg) {
                for (dep, name) in js_frameworks {
                    if content.contains(&format!("\"{}\"", dep)) && !ctx.frameworks.contains(&name.to_string()) {
                        ctx.frameworks.push(name.to_string());
                    }
                }
                // Extract scripts if root had none
                if ctx.scripts.is_empty() {
                    let dir_name = path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    for script_name in &["start", "dev", "build", "test"] {
                        if let Some(val) = extract_script(&content, script_name) {
                            ctx.scripts.insert(
                                script_name.to_string(),
                                format!("cd {} && {}", dir_name, val),
                            );
                        }
                    }
                }
                // Use subdir project info if root had none
                if ctx.name.is_none() {
                    ctx.name = extract_string(&content, "name");
                    ctx.version = extract_string(&content, "version");
                    ctx.description = extract_string(&content, "description");
                }
            }

            // Check subdirectory go.mod
            let sub_gomod = path.join("go.mod");
            if let Ok(content) = fs::read_to_string(&sub_gomod) {
                parse_go_mod(&content, &mut ctx);
                // Detect Go entry point
                let dir_name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let cmd_dir = path.join("cmd");
                if cmd_dir.is_dir() {
                    if let Ok(cmds) = fs::read_dir(&cmd_dir) {
                        for cmd in cmds.flatten() {
                            if cmd.path().is_dir() {
                                let cmd_name = cmd.file_name().to_string_lossy().to_string();
                                if !ctx.scripts.contains_key("run") {
                                    ctx.scripts.insert(
                                        "run".into(),
                                        format!("cd {} && go run ./cmd/{}", dir_name, cmd_name),
                                    );
                                }
                            }
                        }
                    }
                } else if path.join("main.go").exists() && !ctx.scripts.contains_key("run") {
                    ctx.scripts.insert("run".into(), format!("cd {} && go run .", dir_name));
                }
            }
        }
    }

    // Check root go.mod
    let root_gomod = root.join("go.mod");
    if let Ok(content) = fs::read_to_string(&root_gomod) {
        parse_go_mod(&content, &mut ctx);
    }

    // Check for Cargo.toml (Rust)
    if root.join("Cargo.toml").exists() && ctx.project_type == "unknown" {
        ctx.project_type = "rust".into();
    }

    // Check for Python markers
    if (root.join("pyproject.toml").exists() || root.join("requirements.txt").exists())
        && ctx.project_type == "unknown"
    {
        ctx.project_type = "python".into();
    }

    ctx
}

fn parse_go_mod(content: &str, ctx: &mut ProjectContext) {
    // Extract module path from first line
    if let Some(first_line) = content.lines().next() {
        if first_line.starts_with("module ") {
            let module_path = first_line.trim_start_matches("module ").trim().to_string();
            if !ctx.go_modules.contains(&module_path) {
                ctx.go_modules.push(module_path);
            }
        }
    }

    // Detect Go frameworks
    let go_frameworks: &[(&str, &str)] = &[
        ("gin-gonic/gin", "Gin"),
        ("labstack/echo", "Echo"),
        ("gofiber/fiber", "Fiber"),
        ("gorilla/mux", "Gorilla Mux"),
        ("go-chi/chi", "Chi"),
    ];

    for (pattern, name) in go_frameworks {
        if content.contains(pattern) && !ctx.frameworks.contains(&name.to_string()) {
            ctx.frameworks.push(name.to_string());
        }
    }

    // Mark Go as detected
    if ctx.project_type == "unknown" {
        ctx.project_type = "go".into();
    }
}

fn parse_package_json(content: &str, ctx: &mut ProjectContext) {
    ctx.name = extract_string(content, "name");
    ctx.version = extract_string(content, "version");
    ctx.description = extract_string(content, "description");

    let deps = extract_object_keys(content, "dependencies");
    let dev_deps = extract_object_keys(content, "devDependencies");

    if deps.iter().any(|d| d == "next") || dev_deps.iter().any(|d| d == "next") {
        ctx.framework = Some("Next.js".into());
        ctx.project_type = "web-app".into();
    } else if deps.iter().any(|d| d == "react") || dev_deps.iter().any(|d| d == "react") {
        ctx.framework = Some("React".into());
        ctx.project_type = "web-app".into();
    } else if deps.iter().any(|d| d == "vue") || dev_deps.iter().any(|d| d == "vue") {
        ctx.framework = Some("Vue".into());
        ctx.project_type = "web-app".into();
    } else if deps.iter().any(|d| d == "express") || dev_deps.iter().any(|d| d == "express") {
        ctx.framework = Some("Express".into());
        ctx.project_type = "server".into();
    } else if content.contains("\"bin\"") {
        ctx.project_type = "cli".into();
    } else if content.contains("\"main\"") || content.contains("\"exports\"") {
        ctx.project_type = "library".into();
    }

    ctx.dependencies = deps;
    ctx.dev_dependencies = dev_deps;

    for script_name in &["start", "dev", "build", "test"] {
        if let Some(val) = extract_script(content, script_name) {
            ctx.scripts.insert(script_name.to_string(), val);
        }
    }
}

fn extract_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let pos = json.find(&pattern)?;
    let after = &json[pos + pattern.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    if rest.starts_with('"') {
        let end = rest[1..].find('"')?;
        Some(rest[1..1 + end].to_string())
    } else {
        None
    }
}

fn extract_script(json: &str, name: &str) -> Option<String> {
    let scripts_pos = json.find("\"scripts\"")?;
    let scripts_block = &json[scripts_pos..];
    let brace = scripts_block.find('{')?;
    let end_brace = scripts_block[brace..].find('}')?;
    let inner = &scripts_block[brace..brace + end_brace + 1];
    extract_string(inner, name)
}

fn extract_object_keys(json: &str, key: &str) -> Vec<String> {
    let pattern = format!("\"{}\"", key);
    let pos = match json.find(&pattern) {
        Some(p) => p,
        None => return vec![],
    };
    let after = &json[pos + pattern.len()..];
    let brace = match after.find('{') {
        Some(b) => b,
        None => return vec![],
    };
    let end = match after[brace..].find('}') {
        Some(e) => e,
        None => return vec![],
    };
    let inner = &after[brace + 1..brace + end];
    inner
        .split(',')
        .filter_map(|item| {
            let trimmed = item.trim();
            if trimmed.starts_with('"') {
                let end = trimmed[1..].find('"')?;
                Some(trimmed[1..1 + end].to_string())
            } else {
                None
            }
        })
        .collect()
}
