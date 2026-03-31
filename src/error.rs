use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("PTY error: {0}")]
    Pty(String),

    #[error("Crossterm error: {0}")]
    Crossterm(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
