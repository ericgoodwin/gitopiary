use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;
use portable_pty::PtySize;
use tokio::sync::mpsc::UnboundedSender;
use crate::events::AppEvent;
use crate::pty::session::PtySession;

pub struct PtyManager {
    sessions: HashMap<PathBuf, PtySession>,
    shell: String,
}

impl PtyManager {
    pub fn new(shell: String) -> Self {
        Self {
            sessions: HashMap::new(),
            shell,
        }
    }

    pub fn get_or_create(
        &mut self,
        worktree_path: &PathBuf,
        size: PtySize,
        tx: UnboundedSender<AppEvent>,
    ) -> Result<&mut PtySession> {
        if !self.sessions.contains_key(worktree_path) {
            let session = PtySession::new(
                worktree_path.clone(),
                &self.shell,
                size,
                tx,
            )?;
            self.sessions.insert(worktree_path.clone(), session);
        }
        Ok(self.sessions.get_mut(worktree_path).unwrap())
    }

    pub fn get(&self, worktree_path: &PathBuf) -> Option<&PtySession> {
        self.sessions.get(worktree_path)
    }

    pub fn get_mut(&mut self, worktree_path: &PathBuf) -> Option<&mut PtySession> {
        self.sessions.get_mut(worktree_path)
    }

    pub fn resize_all(&mut self, rows: u16, cols: u16) {
        for session in self.sessions.values_mut() {
            if let Err(e) = session.resize(rows, cols) {
                tracing::warn!("Failed to resize PTY session: {}", e);
            }
        }
    }

    pub fn resize_session(&mut self, worktree_path: &PathBuf, rows: u16, cols: u16) {
        if let Some(session) = self.sessions.get_mut(worktree_path) {
            if let Err(e) = session.resize(rows, cols) {
                tracing::warn!("Failed to resize PTY session at {:?}: {}", worktree_path, e);
            }
        }
    }

    pub fn remove(&mut self, worktree_path: &PathBuf) {
        self.sessions.remove(worktree_path);
    }

    pub fn has_any_sessions(&self) -> bool {
        !self.sessions.is_empty()
    }
}
