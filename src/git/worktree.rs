use std::path::PathBuf;
use anyhow::{bail, Context, Result};
use tokio::process::Command;

pub async fn create_worktree(repo_path: &PathBuf, branch_name: &str) -> Result<PathBuf> {
    // Worktree goes in a sibling directory: <parent>/<branch_name>
    let parent = repo_path
        .parent()
        .with_context(|| "Repo path has no parent")?;

    let worktree_path = parent.join(branch_name);

    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("worktree")
        .arg("add")
        .arg("-b")
        .arg(branch_name)
        .arg(&worktree_path)
        .output()
        .await
        .with_context(|| "Failed to run git worktree add")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree add failed: {}", stderr.trim());
    }

    Ok(worktree_path)
}

pub async fn remove_worktree(repo_path: &PathBuf, worktree_path: &PathBuf) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("worktree")
        .arg("remove")
        .arg("--force")
        .arg(worktree_path)
        .output()
        .await
        .with_context(|| "Failed to run git worktree remove")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree remove failed: {}", stderr.trim());
    }

    Ok(())
}
