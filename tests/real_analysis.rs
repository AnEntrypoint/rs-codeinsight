use std::path::Path;

use rs_codeinsight::{analyze, AnalyzeOptions};

#[test]
fn analyze_own_src_dir_end_to_end() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let output = analyze(&root, AnalyzeOptions { json_mode: false });

    assert!(!output.text.is_empty(), "analyze() produced empty output for real src/ tree");
    assert!(output.text.contains("lib.rs") || output.text.contains("analyzer.rs"),
        "expected known source file names in output, got: {}", output.text);
    assert!(output.skipped_files.is_empty(),
        "expected no skipped files under src/, got: {:?}", output.skipped_files);
}

#[test]
fn analyze_own_src_dir_json_mode() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let output = analyze(&root, AnalyzeOptions { json_mode: true });

    assert!(output.text.trim_start().starts_with('{'),
        "expected JSON object output, got: {}", &output.text[..output.text.len().min(80)]);
    assert!(output.text.contains("\"conventions\""),
        "expected conventions section in JSON output, got: {}", output.text);
    assert!(output.text.contains("\"Rs\""),
        "expected Rust language detected in conventions, got: {}", output.text);
}
