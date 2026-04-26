use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

#[derive(Default)]
pub struct GitContext {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub recent_commits: Vec<String>,
    pub uncommitted: Vec<String>,
    pub hot_files: Vec<(String, u32)>,
}

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
            .map(|l| {
                let trimmed = l.trim();
                if trimmed.len() > 3 {
                    trimmed[3..].to_string()
                } else {
                    trimmed.to_string()
                }
            })
            .collect();
    }

    if let Some(shortlog) = run_git(
        root,
        &["log", "--format=%H", "--diff-filter=M", "--name-only", "-100", "--no-decorate"],
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
