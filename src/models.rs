use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub struct DataLayer {
    pub model_names: Vec<String>,
    pub schema_files: Vec<String>,
    pub migration_dirs: Vec<String>,
    pub orm: Option<String>,
}

pub fn detect_data_layer(root: &Path, files: &[(String, String)]) -> DataLayer {
    let mut model_names: Vec<String> = Vec::new();
    let mut schema_files: Vec<String> = Vec::new();
    let mut migration_dirs: Vec<String> = Vec::new();
    let mut orm: Option<String> = None;
    let mut seen_models: HashSet<String> = HashSet::new();
    let mut seen_migration_dirs: HashSet<String> = HashSet::new();

    for (rel, abs) in files {
        let path = Path::new(abs.as_str());

        if let Some(parent) = path.parent() {
            let parent_rel = parent
                .strip_prefix(root)
                .unwrap_or(parent)
                .to_string_lossy()
                .replace('\\', "/");
            let dir_name = parent
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let is_migration_dir = dir_name == "migrations"
                || dir_name == "migrate"
                || parent_rel.ends_with("db/migrations")
                || parent_rel.ends_with("prisma/migrations");
            if is_migration_dir && seen_migration_dirs.insert(parent_rel.clone()) {
                let file_count = count_immediate_files(parent);
                if file_count > 0 {
                    migration_dirs.push(format!("{} ({} files)", parent_rel, file_count));
                }
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

        if file_name == "schema.sql"
            || file_name.ends_with(".up.sql")
            || file_name.starts_with("create_") && file_name.ends_with(".sql")
        {
            schema_files.push(rel.clone());
            continue;
        }

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
