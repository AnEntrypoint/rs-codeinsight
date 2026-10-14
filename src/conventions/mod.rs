mod go;
mod js;
mod python;
mod rust;

use std::collections::HashMap;
use crate::analyzer::FileAnalysis;

use go::detect_go_conventions;
use js::detect_js_conventions;
use python::detect_python_conventions;
use rust::detect_rust_conventions;

pub struct LanguageConventions {
    pub language: String,
    pub conventions: Vec<String>,
}

pub fn detect_conventions(
    file_metrics: &HashMap<String, FileAnalysis>,
    file_languages: &HashMap<String, String>,
) -> Vec<LanguageConventions> {
    let mut by_lang: HashMap<String, Vec<&FileAnalysis>> = HashMap::new();
    let mut paths_by_lang: HashMap<String, Vec<&str>> = HashMap::new();

    for (path, analysis) in file_metrics {
        if let Some(lang) = file_languages.get(path) {
            by_lang.entry(lang.clone()).or_default().push(analysis);
            paths_by_lang.entry(lang.clone()).or_default().push(path.as_str());
        }
    }

    let mut result = Vec::new();

    for (lang, analyses) in &by_lang {
        let paths = paths_by_lang.get(lang).map(|v| v.as_slice()).unwrap_or(&[]);
        let conventions = match lang.as_str() {
            "JavaScript" | "TypeScript" | "TSX" | "JSX" => detect_js_conventions(analyses, paths),
            "Go" => detect_go_conventions(analyses, paths),
            "Python" => detect_python_conventions(analyses, paths),
            "Rust" => detect_rust_conventions(analyses, paths),
            _ => vec![],
        };

        if !conventions.is_empty() {
            let display_lang = match lang.as_str() {
                "JavaScript" => "JS",
                "TypeScript" => "TS",
                "TSX" => "TSX",
                "JSX" => "JSX",
                "Python" => "Py",
                "Rust" => "Rs",
                "Go" => "Go",
                _ => lang.as_str(),
            };
            result.push(LanguageConventions {
                language: display_lang.to_string(),
                conventions,
            });
        }
    }

    result.sort_by(|a, b| a.language.cmp(&b.language));

    merge_similar(&mut result);

    result
}

fn detect_file_naming(paths: &[&str]) -> Option<String> {
    let mut kebab = 0u32;
    let mut pascal = 0u32;
    let mut camel = 0u32;
    let mut snake = 0u32;

    for path in paths {
        let file_name = path.rsplit('/').next().unwrap_or(path);
        let stem = file_name.split('.').next().unwrap_or(file_name);

        if stem.len() <= 1 || stem == "index" || stem == "main" || stem == "lib" || stem == "mod" {
            continue;
        }

        if stem.contains('-') {
            kebab += 1;
        } else if stem.contains('_') {
            snake += 1;
        } else if stem.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            pascal += 1;
        } else if stem.chars().any(|c| c.is_uppercase()) {
            camel += 1;
        } else {
        }
    }

    let total = kebab + pascal + camel + snake;
    if total == 0 {
        return None;
    }

    let max = kebab.max(pascal).max(camel).max(snake);
    if max == kebab {
        Some("kebab-case files".to_string())
    } else if max == pascal {
        Some("PascalCase files".to_string())
    } else if max == camel {
        Some("camelCase files".to_string())
    } else {
        Some("snake_case files".to_string())
    }
}

fn merge_similar(result: &mut Vec<LanguageConventions>) {
    let js_idx = result.iter().position(|c| c.language == "JS");
    let ts_idx = result.iter().position(|c| c.language == "TS");
    let tsx_idx = result.iter().position(|c| c.language == "TSX");

    let all_same = match (js_idx, ts_idx, tsx_idx) {
        (Some(j), Some(t), Some(x)) => {
            result[j].conventions == result[t].conventions
                && result[t].conventions == result[x].conventions
        }
        _ => false,
    };

    if all_same {
        if let (Some(j), Some(t), Some(x)) = (js_idx, ts_idx, tsx_idx) {
            let merged_convs = result[j].conventions.clone();
            let mut indices = vec![j, t, x];
            indices.sort_unstable_by(|a, b| b.cmp(a));
            for idx in indices {
                result.remove(idx);
            }
            result.push(LanguageConventions {
                language: "JS/TS/TSX".to_string(),
                conventions: merged_convs,
            });
            result.sort_by(|a, b| a.language.cmp(&b.language));
            return;
        }
    }

    let js_ts_same = match (js_idx, ts_idx) {
        (Some(j), Some(t)) => result[j].conventions == result[t].conventions,
        _ => false,
    };

    if js_ts_same {
        if let (Some(j), Some(t)) = (js_idx, ts_idx) {
            let merged_convs = result[j].conventions.clone();
            let mut indices = vec![j, t];
            indices.sort_unstable_by(|a, b| b.cmp(a));
            for idx in indices {
                result.remove(idx);
            }
            result.push(LanguageConventions {
                language: "JS/TS".to_string(),
                conventions: merged_convs,
            });
            result.sort_by(|a, b| a.language.cmp(&b.language));
            return;
        }
    }

    let ts_tsx_same = match (ts_idx, tsx_idx) {
        (Some(t), Some(x)) => result[t].conventions == result[x].conventions,
        _ => false,
    };

    if ts_tsx_same {
        if let (Some(t), Some(x)) = (ts_idx, tsx_idx) {
            let merged_convs = result[t].conventions.clone();
            let mut indices = vec![t, x];
            indices.sort_unstable_by(|a, b| b.cmp(a));
            for idx in indices {
                result.remove(idx);
            }
            result.push(LanguageConventions {
                language: "TS/TSX".to_string(),
                conventions: merged_convs,
            });
            result.sort_by(|a, b| a.language.cmp(&b.language));
        }
    }
}
