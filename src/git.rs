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

    let (branch_out, log_out, status_out, shortlog_out) = spawn_git_concurrently(root);

    ctx.branch = branch_out.map(|s| s.trim().to_string());

    if let Some(log) = log_out {
        ctx.recent_commits = log.lines().map(|l| l.to_string()).collect();
    }

    if let Some(status) = status_out {
        ctx.uncommitted = status
            .lines()
            .filter(|l| !l.is_empty())
            .map(parse_porcelain_line)
            .collect();
    }

    if let Some(shortlog) = shortlog_out {
        let mut file_counts: HashMap<String, u32> = HashMap::new();
        for line in shortlog.lines() {
            let trimmed = line.trim();
            let is_commit_hash = trimmed.len() == 40 && trimmed.bytes().all(|b| b.is_ascii_hexdigit());
            if trimmed.is_empty() || is_commit_hash {
                continue;
            }
            *file_counts.entry(trimmed.to_string()).or_insert(0) += 1;
        }
        let mut sorted: Vec<_> = file_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
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
const GIT_QUERIES: [&[&str]; 4] = [
    &["rev-parse", "--abbrev-ref", "HEAD"],
    &["log", "--oneline", "-5", "--no-decorate"],
    &["status", "--porcelain", "--short"],
    &["log", "--format=%H", "--diff-filter=AMD", "--name-only", "-100", "--no-decorate"],
];

#[cfg(not(target_arch = "wasm32"))]
fn spawn_git_concurrently(
    root: &Path,
) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let children: Vec<Option<std::process::Child>> = GIT_QUERIES
        .iter()
        .map(|args| spawn_git(root, args))
        .collect();

    let mut outputs = children.into_iter().map(|child| {
        let output = child?.wait_with_output().ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
    });

    let branch = outputs.next().flatten();
    let log = outputs.next().flatten();
    let status = outputs.next().flatten();
    let shortlog = outputs.next().flatten();
    (branch, log, status, shortlog)
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_git(root: &Path, args: &[&str]) -> Option<std::process::Child> {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd.spawn().ok()
}
