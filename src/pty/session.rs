use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tokio::sync::mpsc::UnboundedSender;
use crate::events::AppEvent;

pub struct PtySession {
    pub parser: Arc<Mutex<vt100::Parser>>,
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn MasterPty + Send>,
    /// Child process handle — kept alive so we can kill the shell on drop.
    child: Box<dyn portable_pty::Child + Send + Sync>,
    pub stop_flag: Arc<AtomicBool>,
    pub reader_thread: Option<JoinHandle<()>>,
    pub size: PtySize,
    /// Milliseconds since UNIX_EPOCH when the reader last received output.
    /// Zero means no output received yet.
    pub last_output_ms: Arc<AtomicU64>,
    /// Set once the reader has seen any output at all (so we can distinguish
    /// "never ran anything" from "ran something and it finished").
    pub has_had_output: Arc<AtomicBool>,
}

impl PtySession {
    /// Returns true when the shell has had activity and has been quiet for
    /// at least 2 seconds — a strong signal it's back at a prompt.
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

impl PtySession {
    pub fn new(
        worktree_path: PathBuf,
        shell: &str,
        size: PtySize,
        tx: UnboundedSender<AppEvent>,
    ) -> Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(size.clone())
            .map_err(|e| anyhow::anyhow!("Failed to open PTY: {}", e))?;

        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(&worktree_path);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| anyhow::anyhow!("Failed to spawn shell: {}", e))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| anyhow::anyhow!("Failed to get PTY writer: {}", e))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| anyhow::anyhow!("Failed to get PTY reader: {}", e))?;

        let parser = Arc::new(Mutex::new(vt100::Parser::new(size.rows, size.cols, 0)));
        let parser_clone = Arc::clone(&parser);

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = Arc::clone(&stop_flag);

        let last_output_ms = Arc::new(AtomicU64::new(0));
        let last_output_ms_clone = Arc::clone(&last_output_ms);

        let has_had_output = Arc::new(AtomicBool::new(false));
        let has_had_output_clone = Arc::clone(&has_had_output);

        let reader_thread = std::thread::spawn(move || {
            let mut translator = AcsTranslator::new();
            let mut buf = [0u8; 4096];
            loop {
                if stop_flag_clone.load(Ordering::Relaxed) {
                    break;
                }
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let translated = translator.process(&buf[..n]);
                        parser_clone.lock().unwrap().process(&translated);

                        let now_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        last_output_ms_clone.store(now_ms, Ordering::Relaxed);
                        has_had_output_clone.store(true, Ordering::Relaxed);

                        tx.send(AppEvent::PtyOutput {
                            worktree_path: worktree_path.clone(),
                        })
                        .ok();
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            parser,
            writer,
            master: pair.master,
            child,
            stop_flag,
            reader_thread: Some(reader_thread),
            size,
            last_output_ms,
            has_had_output,
        })
    }

    pub fn write_input(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data).context("Failed to write to PTY")?;
        self.writer.flush().context("Failed to flush PTY writer")?;
        Ok(())
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        let new_size = PtySize { rows, cols, pixel_width: 0, pixel_height: 0 };
        self.master
            .resize(new_size.clone())
            .map_err(|e| anyhow::anyhow!("Failed to resize PTY: {}", e))?;
        self.parser.lock().unwrap().set_size(rows, cols);
        self.size = new_size;
        Ok(())
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        // Kill the child process first. This closes the slave end of the PTY,
        // which causes reader.read() to return EIO and unblocks the reader thread.
        // Without this the reader thread blocks indefinitely on read() and the
        // app hangs on quit whenever a terminal session is open.
        let _ = self.child.kill();

        self.stop_flag.store(true, Ordering::Relaxed);

        if let Some(thread) = self.reader_thread.take() {
            let _ = thread.join();
        }
    }
}

// ---------------------------------------------------------------------------
// ACS → Unicode translator
//
// vt100 0.15 parses ESC(0 / ESC(B but explicitly does not translate the DEC
// special graphics characters.  We intercept the byte stream here, track the
// graphics-mode state ourselves, and replace each ACS byte with its UTF-8
// box-drawing / symbol equivalent before vt100 ever sees it.
//
// Sequences handled:
//   ESC ( 0  — designate G0 as DEC special graphics  → graphics on
//   ESC ( B  — designate G0 as ASCII                 → graphics off
//   ESC ) 0  — designate G1 as DEC special graphics  (tracked for SO/SI)
//   ESC ) B  — designate G1 as ASCII
//   SO (0x0E) — shift out: activate G1
//   SI (0x0F) — shift in:  activate G0
// ---------------------------------------------------------------------------

struct AcsTranslator {
    state: AcsState,
    g0_is_graphics: bool,
    g1_is_graphics: bool,
    use_g1: bool,
}

#[derive(PartialEq)]
enum AcsState {
    Normal,
    Esc,
    EscParen,
    EscRParen,
}

impl AcsTranslator {
    fn new() -> Self {
        Self {
            state: AcsState::Normal,
            g0_is_graphics: false,
            g1_is_graphics: false,
            use_g1: false,
        }
    }

    fn in_graphics(&self) -> bool {
        if self.use_g1 { self.g1_is_graphics } else { self.g0_is_graphics }
    }

    fn process(&mut self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(input.len() * 3);

        for &b in input {
            match self.state {
                AcsState::Normal => match b {
                    0x1b => self.state = AcsState::Esc,
                    0x0e => { self.use_g1 = true; }
                    0x0f => { self.use_g1 = false; }
                    _ if self.in_graphics() => {
                        match acs_to_utf8(b) {
                            Some(s) => out.extend_from_slice(s.as_bytes()),
                            None => out.push(b),
                        }
                    }
                    _ => out.push(b),
                },

                AcsState::Esc => match b {
                    b'(' => self.state = AcsState::EscParen,
                    b')' => self.state = AcsState::EscRParen,
                    _ => {
                        out.push(0x1b);
                        out.push(b);
                        self.state = AcsState::Normal;
                    }
                },

                AcsState::EscParen => {
                    match b {
                        b'0' => self.g0_is_graphics = true,
                        b'B' => self.g0_is_graphics = false,
                        _ => {
                            out.push(0x1b);
                            out.push(b'(');
                            out.push(b);
                        }
                    }
                    self.state = AcsState::Normal;
                }

                AcsState::EscRParen => {
                    match b {
                        b'0' => self.g1_is_graphics = true,
                        b'B' => self.g1_is_graphics = false,
                        _ => {
                            out.push(0x1b);
                            out.push(b')');
                            out.push(b);
                        }
                    }
                    self.state = AcsState::Normal;
                }
            }
        }

        out
    }
}

fn acs_to_utf8(b: u8) -> Option<&'static str> {
    match b {
        b'`' => Some("◆"),
        b'a' => Some("▒"),
        b'f' => Some("°"),
        b'g' => Some("±"),
        b'i' => Some("␋"),
        b'j' => Some("┘"),
        b'k' => Some("┐"),
        b'l' => Some("┌"),
        b'm' => Some("└"),
        b'n' => Some("┼"),
        b'o' => Some("⎺"),
        b'p' => Some("⎻"),
        b'q' => Some("─"),
        b'r' => Some("⎼"),
        b's' => Some("⎽"),
        b't' => Some("├"),
        b'u' => Some("┤"),
        b'v' => Some("┴"),
        b'w' => Some("┬"),
        b'x' => Some("│"),
        b'y' => Some("≤"),
        b'z' => Some("≥"),
        b'{' => Some("π"),
        b'|' => Some("≠"),
        b'}' => Some("£"),
        b'~' => Some("·"),
        _ => None,
    }
}
