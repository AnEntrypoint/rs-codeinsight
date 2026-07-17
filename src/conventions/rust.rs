use crate::analyzer::FileAnalysis;

pub fn detect_rust_conventions(analyses: &[&FileAnalysis], paths: &[&str]) -> Vec<String> {
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

    if let Some(naming) = super::detect_file_naming(paths) {
        conventions.push(naming);
    }

    conventions
}
