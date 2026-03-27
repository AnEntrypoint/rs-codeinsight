use std::collections::HashMap;
use std::path::Path;

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

    for (line_num, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        let ln = (line_num + 1) as u32;

        // TODO/FIXME/HACK
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

        // Security: eval()
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

        // Security: hardcoded secrets patterns
        let lower = trimmed.to_lowercase();
        let is_test_file = rel_path.contains(".test.") || rel_path.contains(".spec.")
            || rel_path.contains("__tests__") || rel_path.contains("/test/") || rel_path.contains("/tests/")
            || rel_path.contains(".security.") || rel_path.ends_with("_test.go");
        let is_json_file = rel_path.ends_with(".json");
        // JSX/HTML attributes: form inputs, labels, element ids
        let is_jsx_html_attr = lower.contains("type=\"password\"") || lower.contains("placeholder=")
            || lower.contains("label=") || lower.contains("aria-")
            || lower.contains("htmlfor=") || lower.contains("classname=")
            || lower.contains("name=\"password\"") || lower.contains("name=\"token\"")
            || lower.contains("id=\"password\"") || lower.contains("id=\"token\"")
            || lower.contains("id=\"confirmpassword\"") || lower.contains("id=\"newpassword\"")
            || lower.contains("id=\"currentpassword\"")
            || lower.contains("autocomplete=") || lower.contains("autocomplete=\"")
            || (trimmed.starts_with("id=") || trimmed.starts_with("id=\""));
        let is_type_def = trimmed.starts_with("type ") || trimmed.starts_with("interface ")
            || trimmed.starts_with("export type") || trimmed.starts_with("export interface");
        let is_react_form = lower.contains("usestate") || lower.contains("useform") || lower.contains("formdata");
        // Comparison operators: `if token == ""`, `=== 'api-keys'`, etc.
        let is_empty_check = trimmed.contains("== \"\"") || trimmed.contains("!= \"\"")
            || trimmed.contains("=== '") || trimmed.contains("!== '")
            || trimmed.contains("=== \"") || trimmed.contains("!== \"");
        // Constant/enum value: assigned value is all-caps/underscores or is a well-known constant name
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
        // HTML/JSX element (starts with < or contains opening tags)
        let is_html_element = trimmed.starts_with('<') || lower.contains("<label")
            || lower.contains("<input") || lower.contains("<a ") || lower.contains("<h1")
            || lower.contains("<p ") || lower.contains("<selectitem");
        // JSX component prop: keyword is inside a quoted string value of a prop, not a variable assignment
        // e.g. description="...tokens..." or endpoint="GET /api/v1/.../token"
        let is_prop_value = {
            let has_prop_assign = trimmed.contains("description=\"") || trimmed.contains("endpoint=\"")
                || trimmed.contains("value=\"") || trimmed.contains("href=\"")
                || trimmed.contains("style=\"") || trimmed.contains("class=\"");
            has_prop_assign
        };
        // Error/validation message: string contains "is required" or "do not match" etc
        let is_error_msg = lower.contains("is required") || lower.contains("do not match")
            || lower.contains("must be") || lower.contains("please") || lower.contains("confirm your");
        // URL/path pattern: contains /api/ or path ==
        let is_path_pattern = lower.contains("/api/") || lower.contains("path ==")
            || lower.contains("path ===");
        // Email HTML templates
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

        // Security: SQL interpolation
        if (trimmed.contains("SELECT") || trimmed.contains("INSERT") || trimmed.contains("DELETE")
            || trimmed.contains("UPDATE"))
            && (trimmed.contains("${") || trimmed.contains("` +") || trimmed.contains("' +"))
            && !trimmed.starts_with("//")
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
