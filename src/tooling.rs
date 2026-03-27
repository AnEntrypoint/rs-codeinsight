use std::fs;
use std::path::Path;

#[derive(Default)]
pub struct ToolingContext {
    pub typescript: Option<TsConfig>,
    pub linting: Vec<String>,
    pub testing: Option<String>,
    pub ci: Vec<String>,
    pub has_dockerfile: bool,
    pub env_files: Vec<String>,
    pub has_prettier: bool,
}

pub struct TsConfig {
    pub strict: bool,
    pub target: Option<String>,
}

pub fn detect_tooling(root: &Path) -> ToolingContext {
    let mut ctx = ToolingContext::default();

    // TypeScript
    let tsconfig = root.join("tsconfig.json");
    if tsconfig.exists() {
        if let Ok(content) = fs::read_to_string(&tsconfig) {
            let strict = content.contains("\"strict\": true") || content.contains("\"strict\":true");
            let target = extract_json_value(&content, "target");
            ctx.typescript = Some(TsConfig { strict, target });
        }
    }

    // Linting
    let eslint_files = [
        ".eslintrc", ".eslintrc.js", ".eslintrc.cjs", ".eslintrc.json",
        ".eslintrc.yml", ".eslintrc.yaml", "eslint.config.js", "eslint.config.mjs",
    ];
    for f in &eslint_files {
        if root.join(f).exists() {
            ctx.linting.push("ESLint".into());
            break;
        }
    }
    let biome_files = ["biome.json", "biome.jsonc"];
    for f in &biome_files {
        if root.join(f).exists() {
            ctx.linting.push("Biome".into());
            break;
        }
    }

    // Prettier
    let prettier_files = [
        ".prettierrc", ".prettierrc.js", ".prettierrc.json",
        ".prettierrc.yaml", ".prettierrc.yml", ".prettierrc.toml",
        "prettier.config.js", "prettier.config.mjs",
    ];
    for f in &prettier_files {
        if root.join(f).exists() {
            ctx.has_prettier = true;
            break;
        }
    }

    // Testing framework
    if root.join("jest.config.js").exists()
        || root.join("jest.config.ts").exists()
        || root.join("jest.config.mjs").exists()
    {
        ctx.testing = Some("Jest".into());
    } else if root.join("vitest.config.ts").exists()
        || root.join("vitest.config.js").exists()
    {
        ctx.testing = Some("Vitest".into());
    } else if root.join("mocha").exists() || root.join(".mocharc.yml").exists() {
        ctx.testing = Some("Mocha".into());
    } else if root.join("pytest.ini").exists()
        || root.join("pyproject.toml").exists()
        || root.join("setup.cfg").exists()
    {
        if root.join("pyproject.toml").exists() {
            if let Ok(content) = fs::read_to_string(root.join("pyproject.toml")) {
                if content.contains("[tool.pytest") {
                    ctx.testing = Some("pytest".into());
                }
            }
        }
    }

    // CI
    if root.join(".github/workflows").is_dir() {
        if let Ok(entries) = fs::read_dir(root.join(".github/workflows")) {
            let files: Vec<String> = entries
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.ends_with(".yml") || name.ends_with(".yaml") {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect();
            if !files.is_empty() {
                ctx.ci.push(format!("GitHub Actions ({})", files.join(", ")));
            }
        }
    }
    if root.join(".gitlab-ci.yml").exists() {
        ctx.ci.push("GitLab CI".into());
    }
    if root.join("Jenkinsfile").exists() {
        ctx.ci.push("Jenkins".into());
    }
    if root.join(".circleci").is_dir() {
        ctx.ci.push("CircleCI".into());
    }

    // Docker
    ctx.has_dockerfile = root.join("Dockerfile").exists()
        || root.join("docker-compose.yml").exists()
        || root.join("docker-compose.yaml").exists()
        || root.join("compose.yml").exists()
        || root.join("compose.yaml").exists();

    // Env files
    let env_patterns = [".env", ".env.example", ".env.local", ".env.development", ".env.production"];
    for f in &env_patterns {
        if root.join(f).exists() {
            ctx.env_files.push(f.to_string());
        }
    }

    ctx
}

fn extract_json_value(json: &str, key: &str) -> Option<String> {
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
