#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;

#[derive(Default)]
pub struct GitContext {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub recent_commits: Vec<String>,
    pub uncommitted: Vec<String>,
    pub hot_files: Vec<(String, u32)>,
}

#[cfg(target_arch = "wasm32")]
pub fn analyze_git(_root: &Path) -> GitContext { GitContext::default() }

#[cfg(not(target_arch = "wasm32"))]
pub fn analyze_git(root: &Path) -> GitContext {
    let mut ctx = GitContext::default();

    let git_dir = root.join(".git");
    if !git_dir.exists() {
        return ctx;
    }
    ctx.is_repo = true;

    ctx.branch = run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string());

    if let Some(log) = run_git(root, &["log", "--oneline", "-5", "--no-decorate"]) {
        ctx.recent_commits = log.lines().map(|l| l.to_string()).collect();
    }

    if let Some(status) = run_git(root, &["status", "--porcelain", "--short"]) {
        ctx.uncommitted = status
            .lines()
            .filter(|l| !l.is_empty())
            .map(parse_porcelain_line)
            .collect();
    }

    // AMD: churn should count added/deleted files too, not just edits to existing ones
    if let Some(shortlog) = run_git(
        root,
        &["log", "--format=%H", "--diff-filter=AMD", "--name-only", "-100", "--no-decorate"],
    ) {
        let mut file_counts: HashMap<String, u32> = HashMap::new();
        for line in shortlog.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.len() == 40 {
                continue;
            }
            *file_counts.entry(trimmed.to_string()).or_insert(0) += 1;
        }
        let mut sorted: Vec<_> = file_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        ctx.hot_files = sorted.into_iter().take(8).collect();
    }

    ctx
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_porcelain_line(line: &str) -> String {
    if line.len() < 2 || !line.is_char_boundary(2) {
        return line.trim().to_string();
    }
    let status_code = &line[..2];
    let rest = line[2..].trim_start();

    let is_rename_or_copy = status_code.contains('R') || status_code.contains('C');
    if is_rename_or_copy {
        if let Some(arrow_pos) = rest.find(" -> ") {
            return rest[arrow_pos + 4..].to_string();
        }
    }
    rest.to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn run_git(root: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(root);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let output = cmd.output().ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}
