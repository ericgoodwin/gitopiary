pub mod add_repo;
pub mod delete_worktree;
pub mod new_worktree;
pub mod terminal_panel;
pub mod theme;
pub mod worktree_panel;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};
use crate::app::App;
use crate::state::types::PanelFocus;
use crate::ui::add_repo::render_add_repo_dialog;
use crate::ui::new_worktree::render_new_worktree_dialog;
use crate::ui::terminal_panel::render_terminal_panel;
use crate::ui::worktree_panel::render_worktree_panel;

/// Draw the full UI and return the exact inner dimensions of the terminal
/// panel so callers can keep PTY sizes pixel-perfect.
pub fn draw(frame: &mut Frame, app: &App) -> (u16, u16) {
    let area = frame.area();

    let [main_area, status_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    let [left, right] = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(60),
    ])
    .areas(main_area);

    render_worktree_panel(frame, left, &app.state, &app.pty_manager);

    let active_session = app
        .state
        .selected_worktree_path()
        .and_then(|p| app.pty_manager.get(p));

    let terminal_inner = render_terminal_panel(
        frame,
        right,
        active_session,
        app.state.focus == PanelFocus::Terminal,
    );

    render_status_bar(frame, status_area, app);

    if let Some(dialog) = &app.state.new_worktree_dialog {
        render_new_worktree_dialog(frame, area, dialog);
    }

    if let Some(dialog) = &app.state.add_repo_dialog {
        render_add_repo_dialog(frame, area, dialog);
    }

    if let Some(dialog) = &app.state.delete_worktree_dialog {
        delete_worktree::render_delete_worktree_dialog(frame, area, dialog);
    }

    (terminal_inner.width, terminal_inner.height)
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let mut parts = vec![];

    if app.state.is_refreshing {
        parts.push(Span::styled(
            " ⟳ refreshing ",
            Style::default().fg(Color::Yellow),
        ));
    }

    parts.push(Span::styled(
        " j/k: navigate  Enter/Space: terminal  Ctrl+Space: unfocus  e: editor  n: new  d: delete  A: add repo  r: refresh  q: quit",
        Style::default().fg(Color::DarkGray),
    ));

    let status = Paragraph::new(Line::from(parts))
        .block(Block::default().style(Style::default().bg(theme::COLOR_STATUS_BAR)));

    frame.render_widget(status, area);
}
