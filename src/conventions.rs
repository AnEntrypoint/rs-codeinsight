use std::collections::HashMap;
use crate::analyzer::FileAnalysis;

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

fn detect_js_conventions(analyses: &[&FileAnalysis], paths: &[&str]) -> Vec<String> {
    let mut conventions = Vec::new();

    let mut indent_2 = 0u32;
    let mut indent_4 = 0u32;
    let mut indent_tab = 0u32;
    let mut single_q = 0u32;
    let mut double_q = 0u32;
    let mut semi = 0u32;
    let mut no_semi = 0u32;
    let mut arrow = 0u32;
    let mut regular = 0u32;
    let mut default_exp = 0u32;
    let mut named_exp = 0u32;
    let mut at_imports = 0u32;
    let mut rel_imports = 0u32;

    for a in analyses {
        indent_2 += a.indent_2space;
        indent_4 += a.indent_4space;
        indent_tab += a.indent_tab;
        single_q += a.single_quote_count;
        double_q += a.double_quote_count;
        semi += a.semicolon_lines;
        no_semi += a.no_semicolon_lines;
        arrow += a.arrow_fn_count;
        regular += a.regular_fn_count;
        default_exp += a.default_export_count;
        named_exp += a.named_export_count;

        for imp in &a.import_paths {
            if imp.starts_with("@/") || imp.starts_with("~/") {
                at_imports += 1;
            } else if imp.starts_with("./") || imp.starts_with("../") {
                rel_imports += 1;
            }
        }
    }

    let indent_total = indent_2 + indent_4 + indent_tab;
    if indent_total > 0 {
        if indent_2 >= indent_4 && indent_2 >= indent_tab {
            conventions.push("2-space".to_string());
        } else if indent_4 >= indent_2 && indent_4 >= indent_tab {
            conventions.push("4-space".to_string());
        } else {
            conventions.push("tabs".to_string());
        }
    }

    let q_total = single_q + double_q;
    if q_total > 0 {
        let single_ratio = single_q as f64 / q_total as f64;
        let double_ratio = double_q as f64 / q_total as f64;
        if single_ratio > 0.6 {
            conventions.push("single quotes".to_string());
        } else if double_ratio > 0.6 {
            conventions.push("double quotes".to_string());
        }
    }

    let semi_total = semi + no_semi;
    if semi_total > 0 {
        let semi_ratio = semi as f64 / semi_total as f64;
        let no_semi_ratio = no_semi as f64 / semi_total as f64;
        if semi_ratio > 0.7 {
            conventions.push("semicolons".to_string());
        } else if no_semi_ratio > 0.7 {
            conventions.push("no semicolons".to_string());
        }
    }

    let fn_total = arrow + regular;
    if fn_total > 0 {
        let arrow_ratio = arrow as f64 / fn_total as f64;
        let regular_ratio = regular as f64 / fn_total as f64;
        if arrow_ratio > 0.6 {
            conventions.push("arrow functions".to_string());
        } else if regular_ratio > 0.6 {
            conventions.push("function declarations".to_string());
        }
    }

    let exp_total = default_exp + named_exp;
    if exp_total > 0 {
        let default_ratio = default_exp as f64 / exp_total as f64;
        let named_ratio = named_exp as f64 / exp_total as f64;
        if default_ratio > 0.6 {
            conventions.push("default exports".to_string());
        } else if named_ratio > 0.6 {
            conventions.push("named exports".to_string());
        }
    }

    let imp_total = at_imports + rel_imports;
    if imp_total > 0 {
        let at_ratio = at_imports as f64 / imp_total as f64;
        let rel_ratio = rel_imports as f64 / imp_total as f64;
        if at_ratio > 0.6 {
            conventions.push("@/ imports".to_string());
        } else if rel_ratio > 0.6 {
            conventions.push("relative imports".to_string());
        }
    }

    if let Some(naming) = detect_file_naming(paths) {
        conventions.push(naming);
    }

    conventions
}

fn detect_go_conventions(analyses: &[&FileAnalysis], paths: &[&str]) -> Vec<String> {
    let mut conventions = Vec::new();

    conventions.push("tabs".to_string());

    let mut err_count = 0u32;
    for a in analyses {
        for (pattern, count) in &a.call_patterns {
            if pattern.contains("err") {
                err_count += *count;
            }
        }
    }
    if err_count > 0 {
        conventions.push("if err != nil".to_string());
    }

    if let Some(naming) = detect_file_naming(paths) {
        conventions.push(naming);
    }

    conventions
}

fn detect_python_conventions(analyses: &[&FileAnalysis], paths: &[&str]) -> Vec<String> {
    let mut conventions = Vec::new();

    let mut indent_2 = 0u32;
    let mut indent_4 = 0u32;
    let mut indent_tab = 0u32;
    let mut single_q = 0u32;
    let mut double_q = 0u32;

    for a in analyses {
        indent_2 += a.indent_2space;
        indent_4 += a.indent_4space;
        indent_tab += a.indent_tab;
        single_q += a.single_quote_count;
        double_q += a.double_quote_count;
    }

    let indent_total = indent_2 + indent_4 + indent_tab;
    if indent_total > 0 {
        if indent_4 >= indent_2 && indent_4 >= indent_tab {
            conventions.push("4-space".to_string());
        } else if indent_2 >= indent_4 && indent_2 >= indent_tab {
            conventions.push("2-space".to_string());
        } else {
            conventions.push("tabs".to_string());
        }
    }

    let q_total = single_q + double_q;
    if q_total > 0 {
        let single_ratio = single_q as f64 / q_total as f64;
        let double_ratio = double_q as f64 / q_total as f64;
        if single_ratio > 0.6 {
            conventions.push("single quotes".to_string());
        } else if double_ratio > 0.6 {
            conventions.push("double quotes".to_string());
        }
    }

    if let Some(naming) = detect_file_naming(paths) {
        conventions.push(naming);
    }

    conventions
}

fn detect_rust_conventions(analyses: &[&FileAnalysis], paths: &[&str]) -> Vec<String> {
    let mut conventions = Vec::new();

    let mut unwrap_count = 0u32;
    let mut expect_count = 0u32;

    for a in analyses {
        for (pattern, count) in &a.call_patterns {
            if pattern.ends_with(".unwrap") || pattern.contains(".unwrap()") {
                unwrap_count += *count;
            }
            if pattern.ends_with(".expect") || pattern.contains(".expect(") {
                expect_count += *count;
            }
        }
    }

    let err_total = unwrap_count + expect_count;
    if err_total > 0 {
        if unwrap_count > expect_count {
            conventions.push(".unwrap()".to_string());
        } else {
            conventions.push(".expect()".to_string());
        }
    }

    if let Some(naming) = detect_file_naming(paths) {
        conventions.push(naming);
    }

    conventions
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
