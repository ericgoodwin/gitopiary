use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;
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
        rows: u16,
        cols: u16,
        tx: UnboundedSender<AppEvent>,
    ) -> Result<&mut PtySession> {
        if !self.sessions.contains_key(worktree_path) {
            let session = PtySession::new(
                worktree_path.clone(),
                &self.shell,
                rows,
                cols,
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
            session.resize(rows, cols);
        }
    }

    pub fn remove(&mut self, worktree_path: &PathBuf) {
        self.sessions.remove(worktree_path);
    }

    pub fn has_any_sessions(&self) -> bool {
        !self.sessions.is_empty()
    }
}
