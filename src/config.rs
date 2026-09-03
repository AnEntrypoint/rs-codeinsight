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

    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed[1..trimmed.len() - 1].trim().to_string();
            if current_section != "ignore" && current_section != "limits" {
                eprintln!(
                    "warning: .codeinsight.toml:{}: unknown section [{}]",
                    line_no + 1,
                    current_section
                );
            }
            continue;
        }

        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim();
            let value = trimmed[eq_pos + 1..].trim();

            match current_section.as_str() {
                "ignore" => {
                    if key == "dirs" {
                        config.ignore_dirs = parse_string_array(value);
                    } else if key == "files" {
                        config.ignore_files = parse_string_array(value);
                    } else {
                        eprintln!(
                            "warning: .codeinsight.toml:{}: unknown key '{}' in [ignore]",
                            line_no + 1,
                            key
                        );
                    }
                }
                "limits" => {
                    if key == "max_file_size" {
                        if let Ok(v) = value.parse::<u64>() {
                            config.max_file_size = v;
                        } else {
                            eprintln!(
                                "warning: .codeinsight.toml:{}: invalid value for 'max_file_size': '{}'",
                                line_no + 1,
                                value
                            );
                        }
                    } else {
                        eprintln!(
                            "warning: .codeinsight.toml:{}: unknown key '{}' in [limits]",
                            line_no + 1,
                            key
                        );
                    }
                }
                "" => {
                    eprintln!(
                        "warning: .codeinsight.toml:{}: key '{}' outside any section, ignored",
                        line_no + 1,
                        key
                    );
                }
                other => {
                    eprintln!(
                        "warning: .codeinsight.toml:{}: unknown key '{}' in [{}]",
                        line_no + 1,
                        key,
                        other
                    );
                }
            }
        } else {
            eprintln!(
                "warning: .codeinsight.toml:{}: malformed line, expected 'key = value': {}",
                line_no + 1,
                trimmed
            );
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
    split_top_level_commas(inner)
        .into_iter()
        .filter_map(|item| {
            let s = item.trim();
            if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
                Some(unescape_double_quoted(&s[1..s.len() - 1]))
            } else if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
                Some(s[1..s.len() - 1].to_string())
            } else if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect()
}

fn unescape_double_quoted(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn split_top_level_commas(inner: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote_char: Option<char> = None;
    let mut escaped = false;

    for c in inner.chars() {
        match quote_char {
            Some(q) => {
                current.push(c);
                if escaped {
                    escaped = false;
                } else if q == '"' && c == '\\' {
                    escaped = true;
                } else if c == q {
                    quote_char = None;
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    quote_char = Some(c);
                    current.push(c);
                } else if c == ',' {
                    parts.push(std::mem::take(&mut current));
                } else {
                    current.push(c);
                }
            }
        }
    }
    parts.push(current);
    parts
}
