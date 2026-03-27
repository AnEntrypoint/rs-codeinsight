use std::collections::HashSet;
use std::fs;
use std::path::Path;

use ignore::WalkBuilder;

pub struct DataLayer {
    pub model_names: Vec<String>,
    pub schema_files: Vec<String>,
    pub migration_dirs: Vec<String>,
    pub orm: Option<String>,
}

const MAX_FILE_SIZE: u64 = 200 * 1024;

pub fn detect_data_layer(root: &Path) -> DataLayer {
    let mut model_names: Vec<String> = Vec::new();
    let mut schema_files: Vec<String> = Vec::new();
    let mut migration_dirs: Vec<String> = Vec::new();
    let mut orm: Option<String> = None;
    let mut seen_models: HashSet<String> = HashSet::new();

    let walker = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                "node_modules" | ".git" | "dist" | "build" | "target"
                    | ".next" | ".nuxt" | "coverage" | "__pycache__"
                    | ".venv" | "vendor" | ".cache" | ".output"
            )
        })
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        // Check directories for migration dirs
        if path.is_dir() {
            let dir_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if dir_name == "migrations" || dir_name == "migrate" {
                let file_count = count_immediate_files(path);
                if file_count > 0 {
                    migration_dirs.push(format!("{} ({} files)", rel, file_count));
                }
            }
            // Check for db/migrations or prisma/migrations patterns
            if rel.ends_with("db/migrations") || rel.ends_with("prisma/migrations") {
                let file_count = count_immediate_files(path);
                if file_count > 0 && !migration_dirs.iter().any(|d| d.starts_with(&rel)) {
                    migration_dirs.push(format!("{} ({} files)", rel, file_count));
                }
            }
            continue;
        }

        if !path.is_file() {
            continue;
        }

        if let Ok(meta) = path.metadata() {
            if meta.len() > MAX_FILE_SIZE {
                continue;
            }
        }

        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        // (a) Find Prisma schemas
        if file_name == "schema.prisma" {
            schema_files.push(rel.clone());
            if let Ok(content) = fs::read_to_string(path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("model ") {
                        if let Some(name) = trimmed
                            .strip_prefix("model ")
                            .and_then(|rest| rest.split_whitespace().next())
                        {
                            let name = name.trim_end_matches('{').trim().to_string();
                            if !name.is_empty() && seen_models.insert(name.clone()) {
                                model_names.push(name);
                            }
                        }
                    }
                }
                if orm.is_none() {
                    orm = Some("Prisma".into());
                }
            }
            continue;
        }

        // (e) Find SQL schema files
        if file_name == "schema.sql"
            || file_name.ends_with(".up.sql")
            || file_name.starts_with("create_") && file_name.ends_with(".sql")
        {
            schema_files.push(rel.clone());
            continue;
        }

        // (b) Find Go structs with DB tags
        if ext == "go" {
            let parent_name = path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let is_model_dir = matches!(
                parent_name.as_str(),
                "models" | "model" | "entities" | "entity" | "domain"
            );
            if is_model_dir {
                if let Ok(content) = fs::read_to_string(path) {
                    let has_gorm = content.contains("gorm");
                    let has_json_tag = content.contains("json:");
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("type ") && trimmed.contains(" struct") {
                            if let Some(name) = trimmed
                                .strip_prefix("type ")
                                .and_then(|rest| rest.split_whitespace().next())
                            {
                                let name = name.to_string();
                                if !name.is_empty() && seen_models.insert(name.clone()) {
                                    model_names.push(name);
                                }
                            }
                        }
                    }
                    if orm.is_none() {
                        if has_gorm {
                            orm = Some("GORM".into());
                        } else if has_json_tag {
                            orm = Some("Go structs".into());
                        }
                    }
                }
            }
            continue;
        }

        // (c) Find TypeScript/JS model files
        if ext == "ts" || ext == "js" || ext == "tsx" || ext == "jsx" {
            let parent_name = path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let is_model_dir = matches!(
                parent_name.as_str(),
                "models" | "model" | "types" | "entities" | "schemas"
            );
            if is_model_dir {
                if let Ok(content) = fs::read_to_string(path) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        let extracted = extract_ts_model_name(trimmed);
                        if let Some(name) = extracted {
                            if !is_utility_type(&name) && seen_models.insert(name.clone()) {
                                model_names.push(name);
                            }
                        }
                    }
                }
            }

            // (f) Detect ORM from TypeScript/JS files
            if orm.is_none() {
                if let Ok(content) = fs::read_to_string(path) {
                    if content.contains("@Entity") {
                        orm = Some("TypeORM".into());
                    } else if content.contains("mongoose.model") || content.contains("mongoose.Schema") {
                        orm = Some("Mongoose".into());
                    } else if content.contains("sequelize.define") || content.contains("Sequelize") && content.contains("DataTypes") {
                        orm = Some("Sequelize".into());
                    } else if content.contains("pgTable") || content.contains("sqliteTable") || content.contains("mysqlTable") {
                        orm = Some("Drizzle".into());
                    }
                }
            }

            continue;
        }
    }

    // (f) Check for drizzle directory at root
    if orm.is_none() {
        let drizzle_dir = root.join("drizzle");
        if drizzle_dir.is_dir() {
            orm = Some("Drizzle".into());
        }
    }

    DataLayer {
        model_names,
        schema_files,
        migration_dirs,
        orm,
    }
}

fn extract_ts_model_name(line: &str) -> Option<String> {
    // interface XXX
    if line.starts_with("interface ") || line.starts_with("export interface ") {
        let rest = if line.starts_with("export interface ") {
            &line["export interface ".len()..]
        } else {
            &line["interface ".len()..]
        };
        return rest
            .split(|c: char| c.is_whitespace() || c == '{' || c == '<')
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }
    // type XXX =
    if line.starts_with("type ") || line.starts_with("export type ") {
        let rest = if line.starts_with("export type ") {
            &line["export type ".len()..]
        } else {
            &line["type ".len()..]
        };
        return rest
            .split(|c: char| c.is_whitespace() || c == '=' || c == '<')
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }
    // export class XXX
    if line.starts_with("export class ") || line.starts_with("class ") {
        let rest = if line.starts_with("export class ") {
            &line["export class ".len()..]
        } else {
            &line["class ".len()..]
        };
        return rest
            .split(|c: char| c.is_whitespace() || c == '{' || c == '<')
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }
    None
}

fn is_utility_type(name: &str) -> bool {
    let skip_suffixes = ["Props", "Params", "Options", "Config", "Response", "Request"];
    skip_suffixes.iter().any(|s| name.contains(s))
}

fn count_immediate_files(dir: &Path) -> u32 {
    let mut count = 0u32;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                count += 1;
            }
        }
    }
    count
}
