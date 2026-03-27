use std::fs;
use std::path::Path;

pub struct Config {
    pub ignore_dirs: Vec<String>,
    pub ignore_files: Vec<String>,
    pub max_file_size: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ignore_dirs: Vec::new(),
            ignore_files: Vec::new(),
            max_file_size: 200 * 1024,
        }
    }
}

pub fn load_config(root: &Path) -> Config {
    let config_path = root.join(".codeinsight.toml");
    let content = match fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return Config::default(),
    };

    let mut config = Config::default();
    let mut current_section = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Section header
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed[1..trimmed.len() - 1].trim().to_string();
            continue;
        }

        // Key = value
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim();
            let value = trimmed[eq_pos + 1..].trim();

            match current_section.as_str() {
                "ignore" => {
                    if key == "dirs" {
                        config.ignore_dirs = parse_string_array(value);
                    } else if key == "files" {
                        config.ignore_files = parse_string_array(value);
                    }
                }
                "limits" => {
                    if key == "max_file_size" {
                        if let Ok(v) = value.parse::<u64>() {
                            config.max_file_size = v;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    config
}

fn parse_string_array(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Vec::new();
    }

    let inner = &trimmed[1..trimmed.len() - 1];
    inner
        .split(',')
        .filter_map(|item| {
            let s = item.trim();
            if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                Some(s[1..s.len() - 1].to_string())
            } else {
                None
            }
        })
        .collect()
}
