use crate::analyzer::FileAnalysis;

pub fn detect_go_conventions(analyses: &[&FileAnalysis], paths: &[&str]) -> Vec<String> {
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

    if let Some(naming) = super::detect_file_naming(paths) {
        conventions.push(naming);
    }

    conventions
}
