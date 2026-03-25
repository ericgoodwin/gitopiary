use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;
use alacritty_terminal::event::{Event as AlacrittyEvent, EventListener, Notify, OnResize, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, Msg, Notifier, State};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{self, Term};
use alacritty_terminal::tty::{self, Options as PtyOptions, Pty, Shell};
use tokio::sync::mpsc::UnboundedSender;
use crate::events::AppEvent;

#[derive(Clone)]
pub struct EventProxy {
    worktree_path: PathBuf,
    tx: UnboundedSender<AppEvent>,
    last_output_ms: Arc<AtomicU64>,
    has_had_output: Arc<AtomicBool>,
}

impl EventListener for EventProxy {
    fn send_event(&self, event: AlacrittyEvent) {
        if let AlacrittyEvent::Wakeup = event {
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            self.last_output_ms.store(now_ms, Ordering::Relaxed);
            self.has_had_output.store(true, Ordering::Relaxed);
            self.tx
                .send(AppEvent::PtyOutput {
                    worktree_path: self.worktree_path.clone(),
                })
                .ok();
        }
    }
}

struct TermSize {
    rows: usize,
    cols: usize,
}

impl Dimensions for TermSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

pub struct PtySession {
    pub term: Arc<FairMutex<Term<EventProxy>>>,
    notifier: Notifier,
    event_loop_join: Option<JoinHandle<(EventLoop<Pty, EventProxy>, State)>>,
    pub last_output_ms: Arc<AtomicU64>,
    pub has_had_output: Arc<AtomicBool>,
    pub selection_dragging: bool,
}

impl PtySession {
    pub fn new(
        worktree_path: PathBuf,
        shell: &str,
        rows: u16,
        cols: u16,
        tx: UnboundedSender<AppEvent>,
    ) -> Result<Self> {
        let window_size = WindowSize {
            num_lines: rows,
            num_cols: cols,
            cell_width: 1,
            cell_height: 1,
        };

        let last_output_ms = Arc::new(AtomicU64::new(0));
        let has_had_output = Arc::new(AtomicBool::new(false));

        let event_proxy = EventProxy {
            worktree_path: worktree_path.clone(),
            tx,
            last_output_ms: Arc::clone(&last_output_ms),
            has_had_output: Arc::clone(&has_had_output),
        };

        let config = term::Config {
            scrolling_history: 5000,
            ..Default::default()
        };

        let term_size = TermSize {
            rows: rows as usize,
            cols: cols as usize,
        };
        let term = Term::new(config, &term_size, event_proxy.clone());
        let term = Arc::new(FairMutex::new(term));

        let mut env = HashMap::new();
        env.insert("TERM".into(), "xterm-256color".into());
        env.insert("COLORTERM".into(), "truecolor".into());

        let pty_options = PtyOptions {
            shell: Some(Shell::new(shell.to_string(), vec![])),
            working_directory: Some(worktree_path),
            env,
            ..Default::default()
        };

        let pty = tty::new(&pty_options, window_size, 0)?;

        let event_loop = EventLoop::new(
            Arc::clone(&term),
            event_proxy,
            pty,
            false,
            false,
        )?;
        let notifier = Notifier(event_loop.channel());
        let join = event_loop.spawn();

        Ok(Self {
            term,
            notifier,
            event_loop_join: Some(join),
            last_output_ms,
            has_had_output,
            selection_dragging: false,
        })
    }

    pub fn write_input(&self, data: &[u8]) {
        self.notifier.notify(data.to_vec());
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        let window_size = WindowSize {
            num_lines: rows,
            num_cols: cols,
            cell_width: 1,
            cell_height: 1,
        };
        self.notifier.on_resize(window_size);
        let term_size = TermSize {
            rows: rows as usize,
            cols: cols as usize,
        };
        self.term.lock().resize(term_size);
    }

    pub fn scroll_up(&self, lines: i32) {
        use alacritty_terminal::grid::Scroll;
        self.term.lock().scroll_display(Scroll::Delta(lines));
    }

    pub fn scroll_down(&self, lines: i32) {
        use alacritty_terminal::grid::Scroll;
        self.term.lock().scroll_display(Scroll::Delta(-lines));
    }

    pub fn reset_scroll(&self) {
        use alacritty_terminal::grid::Scroll;
        self.term.lock().scroll_display(Scroll::Bottom);
    }

    pub fn is_idle(&self) -> bool {
        if !self.has_had_output.load(Ordering::Relaxed) {
            return false;
        }
        let last_ms = self.last_output_ms.load(Ordering::Relaxed);
        if last_ms == 0 {
            return false;
        }
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now_ms.saturating_sub(last_ms) > 2_000
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.notifier.0.send(Msg::Shutdown);
        if let Some(join) = self.event_loop_join.take() {
            let _ = join.join();
        }
    }
}
