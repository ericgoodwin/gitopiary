use ratatui::style::{Color, Modifier, Style};

pub const COLOR_SELECTED_BG: Color = Color::Rgb(40, 60, 80);
pub const COLOR_DIRTY: Color = Color::Yellow;
pub const COLOR_AHEAD: Color = Color::Blue;
pub const COLOR_BEHIND: Color = Color::Red;
pub const COLOR_PR_DRAFT: Color = Color::DarkGray;
pub const COLOR_PR_OPEN: Color = Color::Green;
pub const COLOR_PR_CLOSED: Color = Color::Red;
pub const COLOR_PR_MERGED: Color = Color::Magenta;
pub const COLOR_REPO_HEADER: Color = Color::Cyan;
pub const COLOR_BRANCH: Color = Color::White;
pub const COLOR_DIM: Color = Color::DarkGray;
pub const COLOR_BORDER_FOCUSED: Color = Color::Cyan;
pub const COLOR_BORDER_UNFOCUSED: Color = Color::DarkGray;
pub const COLOR_STATUS_BAR: Color = Color::Rgb(30, 30, 30);

pub fn selected_style() -> Style {
    Style::default().bg(COLOR_SELECTED_BG)
}

pub fn bold() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

pub fn dim() -> Style {
    Style::default().fg(COLOR_DIM)
}

pub fn repo_header_style() -> Style {
    Style::default().fg(COLOR_REPO_HEADER).add_modifier(Modifier::BOLD)
}

pub fn border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(COLOR_BORDER_FOCUSED)
    } else {
        Style::default().fg(COLOR_BORDER_UNFOCUSED)
    }
}
