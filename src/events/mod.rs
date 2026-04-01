pub mod handler;

use std::path::PathBuf;
use crate::github::pr::PrInfo;
use crate::state::types::Repository;

#[derive(Debug)]
pub enum AppEvent {
    Crossterm(crossterm::event::Event),
    PtyOutput { worktree_path: PathBuf },
    /// Git status for a single repo is ready — show it immediately.
    RepoLoaded(Repository),
    /// PR data for a repo arrived — patch badges onto already-visible worktrees.
    PrsFetched { repo_path: PathBuf, prs: Vec<PrInfo> },
    /// All repos in a refresh cycle have finished (git + PRs).
    RefreshDone,
    RefreshError(String),
    WorktreeCreated { repo_path: PathBuf, worktree_path: PathBuf },
    WorktreeCreateError(String),
    WorktreeDeleted { repo_path: PathBuf, worktree_path: PathBuf },
    WorktreeDeleteError(String),
    RepoAdded(PathBuf),
    RepoAddError(String),
    /// Open an external editor for the given worktree path.
    /// Handled in the event loop so the terminal can be suspended/restored.
    OpenEditor(PathBuf),
    /// Periodic 1-second heartbeat used to update idle indicators.
    Tick,
    Quit,
}
