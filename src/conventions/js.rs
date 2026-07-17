use crate::analyzer::FileAnalysis;

pub fn detect_js_conventions(analyses: &[&FileAnalysis], paths: &[&str]) -> Vec<String> {
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

    if let Some(naming) = super::detect_file_naming(paths) {
        conventions.push(naming);
    }

    conventions
}
