use crate::analyzer::FileAnalysis;

pub fn detect_python_conventions(analyses: &[&FileAnalysis], paths: &[&str]) -> Vec<String> {
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

    if let Some(naming) = super::detect_file_naming(paths) {
        conventions.push(naming);
    }

    conventions
}
