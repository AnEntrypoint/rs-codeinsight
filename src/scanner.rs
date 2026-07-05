use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct DevNote {
    pub file: String,
    pub line: u32,
    pub kind: String,
    pub text: String,
}

#[derive(Default, Clone)]
pub struct SecurityIssue {
    pub file: String,
    pub line: u32,
    pub kind: String,
    pub detail: String,
}

#[derive(Default)]
pub struct ScanResults {
    pub todos: Vec<DevNote>,
    pub fixmes: Vec<DevNote>,
    pub hacks: Vec<DevNote>,
    pub security: Vec<SecurityIssue>,
}

pub fn scan_source(rel_path: &str, source: &str) -> ScanResults {
    let mut results = ScanResults::default();
    let mut in_block_comment = false;

    for (line_num, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        let ln = (line_num + 1) as u32;

        let line_started_in_block_comment = in_block_comment;
        in_block_comment = update_block_comment_state(trimmed, in_block_comment);

        if let Some(pos) = trimmed.find("TODO") {
            if is_comment_context(trimmed, pos) {
                let text = extract_note_text(trimmed, pos + 4);
                results.todos.push(DevNote {
                    file: rel_path.to_string(), line: ln,
                    kind: "TODO".into(), text,
                });
            }
        }
        if let Some(pos) = trimmed.find("FIXME") {
            if is_comment_context(trimmed, pos) {
                let text = extract_note_text(trimmed, pos + 5);
                results.fixmes.push(DevNote {
                    file: rel_path.to_string(), line: ln,
                    kind: "FIXME".into(), text,
                });
            }
        }
        if let Some(pos) = trimmed.find("HACK") {
            if is_comment_context(trimmed, pos) {
                let text = extract_note_text(trimmed, pos + 4);
                results.hacks.push(DevNote {
                    file: rel_path.to_string(), line: ln,
                    kind: "HACK".into(), text,
                });
            }
        }

        if trimmed.contains("eval(") && !trimmed.starts_with("//") && !trimmed.starts_with("*") {
            let is_safe = trimmed.contains("JSON.parse") || trimmed.contains("// safe");
            if !is_safe {
                results.security.push(SecurityIssue {
                    file: rel_path.to_string(), line: ln,
                    kind: "eval".into(),
                    detail: "eval() usage".into(),
                });
            }
        }

        let lower = trimmed.to_lowercase();
        let is_test_file = {
            let normalized = rel_path.replace('\\', "/");
            let file_name = normalized.rsplit('/').next().unwrap_or(&normalized);
            let dir_segments = normalized.rsplit_once('/').map(|(dirs, _)| dirs);
            let has_test_dir_segment = dir_segments
                .map(|dirs| dirs.split('/').any(|seg| seg == "test" || seg == "tests" || seg == "__tests__"))
                .unwrap_or(false);
            let stem_ends_with_test = file_name.rsplit_once('.')
                .map(|(stem, _)| stem.ends_with(".test") || stem.ends_with("_test") || stem.ends_with(".spec") || stem.ends_with("_spec"))
                .unwrap_or(false);
            has_test_dir_segment || stem_ends_with_test || rel_path.contains(".security.")
        };
        let is_json_file = rel_path.ends_with(".json");
        let is_jsx_html_attr = lower.contains("type=\"password\"") || lower.contains("placeholder=")
            || lower.contains("label=") || lower.contains("aria-")
            || lower.contains("htmlfor=") || lower.contains("classname=")
            || lower.contains("name=\"password\"") || lower.contains("name=\"token\"")
            || lower.contains("id=\"password\"") || lower.contains("id=\"token\"")
            || lower.contains("id=\"confirmpassword\"") || lower.contains("id=\"newpassword\"")
            || lower.contains("id=\"currentpassword\"")
            || lower.contains("autocomplete=")
            || trimmed.starts_with("id=");
        let is_type_def = trimmed.starts_with("type ") || trimmed.starts_with("interface ")
            || trimmed.starts_with("export type") || trimmed.starts_with("export interface");
        let is_react_form = lower.contains("usestate") || lower.contains("useform") || lower.contains("formdata");
        let is_empty_check = trimmed.contains("== \"\"") || trimmed.contains("!= \"\"")
            || trimmed.contains("=== '") || trimmed.contains("!== '")
            || trimmed.contains("=== \"") || trimmed.contains("!== \"");
        let is_const_enum = {
            if let Some(pos) = trimmed.find("= \"") {
                let after = &trimmed[pos + 3..];
                if let Some(end) = after.find('"') {
                    let val = &after[..end];
                    val.chars().all(|c| c.is_uppercase() || c == '_')
                        || val.starts_with("PASSWORD_") || val.starts_with("X-")
                        || val.starts_with("csrf") || val.starts_with("session")
                } else { false }
            } else if let Some(pos) = trimmed.find("= '") {
                let after = &trimmed[pos + 3..];
                if let Some(end) = after.find('\'') {
                    let val = &after[..end];
                    val.chars().all(|c| c.is_uppercase() || c == '_')
                        || val.starts_with("PASSWORD_") || val.starts_with("X-")
                        || val.starts_with("csrf") || val.starts_with("session")
                } else { false }
            } else { false }
        };
        let is_html_element = trimmed.starts_with('<') || lower.contains("<label")
            || lower.contains("<input") || lower.contains("<a ") || lower.contains("<h1")
            || lower.contains("<p ") || lower.contains("<selectitem");
        let is_prop_value = trimmed.contains("description=\"") || trimmed.contains("endpoint=\"")
            || trimmed.contains("value=\"") || trimmed.contains("href=\"")
            || trimmed.contains("style=\"") || trimmed.contains("class=\"");
        let is_error_msg = lower.contains("is required") || lower.contains("do not match")
            || lower.contains("must be") || lower.contains("please") || lower.contains("confirm your");
        let is_path_pattern = lower.contains("/api/") || lower.contains("path ==")
            || lower.contains("path ===");
        let is_email_template = lower.contains("<h1") || lower.contains("<p ")
            || lower.contains("style=\"%s\"") || lower.contains("class=\"");

        if (lower.contains("password") || lower.contains("secret") || lower.contains("api_key")
            || lower.contains("apikey") || lower.contains("token"))
            && (trimmed.contains("= \"") || trimmed.contains("= '") || trimmed.contains("=\"") || trimmed.contains("='"))
            && !trimmed.starts_with("//") && !trimmed.starts_with("*")
            && !lower.contains("process.env") && !lower.contains("env.")
            && !lower.contains("example") && !lower.contains("placeholder")
            && !lower.contains("test") && !lower.contains("mock")
            && !is_test_file && !is_json_file
            && !is_jsx_html_attr && !is_type_def && !is_react_form
            && !is_empty_check && !is_const_enum && !is_html_element
            && !is_prop_value && !is_error_msg && !is_path_pattern && !is_email_template
        {
            results.security.push(SecurityIssue {
                file: rel_path.to_string(), line: ln,
                kind: "secret".into(),
                detail: "Possible hardcoded secret".into(),
            });
        }

        if (trimmed.contains("SELECT") || trimmed.contains("INSERT") || trimmed.contains("DELETE")
            || trimmed.contains("UPDATE"))
            && (trimmed.contains("${") || trimmed.contains("` +") || trimmed.contains("' +"))
            && !line_started_in_block_comment
            && !trimmed.starts_with("//") && !trimmed.starts_with("*") && !trimmed.starts_with('#')
            && !trimmed.starts_with("\"\"\"") && !trimmed.starts_with("'''")
        {
            results.security.push(SecurityIssue {
                file: rel_path.to_string(), line: ln,
                kind: "sql_injection".into(),
                detail: "SQL with string interpolation".into(),
            });
        }
    }

    results
}

pub fn map_tests(files: &[String]) -> TestMap {
    let mut source_files: Vec<String> = Vec::new();
    let mut test_files: Vec<String> = Vec::new();
    let mut covered: Vec<(String, String)> = Vec::new();
    let mut uncovered: Vec<String> = Vec::new();

    for f in files {
        let fname = f.rsplit('/').next().unwrap_or(f);
        if fname.contains(".test.") || fname.contains(".spec.")
            || f.contains("/__tests__/") || f.contains("/test/") || f.contains("/tests/")
        {
            test_files.push(f.clone());
        } else {
            source_files.push(f.clone());
        }
    }

    let test_base_names: HashMap<String, String> = test_files
        .iter()
        .map(|t| {
            let base = t.rsplit('/').next().unwrap_or(t)
                .replace(".test.", ".")
                .replace(".spec.", ".");
            (base, t.clone())
        })
        .collect();

    for src in &source_files {
        let src_fname = src.rsplit('/').next().unwrap_or(src);
        if let Some(test_file) = test_base_names.get(src_fname) {
            covered.push((src.clone(), test_file.clone()));
        } else {
            uncovered.push(src.clone());
        }
    }

    TestMap {
        source_count: source_files.len() as u32,
        test_count: test_files.len() as u32,
        covered,
        uncovered,
    }
}

#[derive(Default)]
pub struct TestMap {
    pub source_count: u32,
    pub test_count: u32,
    pub covered: Vec<(String, String)>,
    pub uncovered: Vec<String>,
}

fn is_comment_context(line: &str, pos: usize) -> bool {
    let before = &line[..pos];
    before.contains("//") || before.contains("/*") || before.contains("* ")
        || before.starts_with('#') || before.starts_with("*")
}

fn update_block_comment_state(trimmed: &str, currently_in_block_comment: bool) -> bool {
    let mut in_block = currently_in_block_comment;
    let mut rest = trimmed;
    loop {
        if in_block {
            match rest.find("*/") {
                Some(end) => {
                    in_block = false;
                    rest = &rest[end + 2..];
                }
                None => return true,
            }
        } else {
            match rest.find("/*") {
                Some(start) => {
                    in_block = true;
                    rest = &rest[start + 2..];
                }
                None => return false,
            }
        }
    }
}

fn extract_note_text(line: &str, start: usize) -> String {
    if start >= line.len() {
        return String::new();
    }
    let rest = &line[start..];
    rest.trim_start_matches(|c: char| c == ':' || c == ' ' || c == '-')
        .chars()
        .take(80)
        .collect::<String>()
        .trim()
        .to_string()
}
