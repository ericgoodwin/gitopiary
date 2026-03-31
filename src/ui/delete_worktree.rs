use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use crate::state::types::DeleteWorktreeDialog;

pub fn render_delete_worktree_dialog(
    frame: &mut Frame,
    area: Rect,
    dialog: &DeleteWorktreeDialog,
) {
    let dialog_width = 60u16.min(area.width.saturating_sub(4));
    let dialog_height = 6u16;

    let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
    let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect { x, y, width: dialog_width, height: dialog_height };

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Delete Worktree ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let [msg_area, _blank, error_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner)[..] else {
        return;
    };

    if dialog.is_deleting {
        let msg = Paragraph::new(Span::styled(
            format!("Deleting {}...", dialog.branch_name),
            Style::default().fg(Color::Yellow),
        ));
        frame.render_widget(msg, msg_area);
    } else {
        let msg = Paragraph::new(Line::from(vec![
            Span::raw("Delete worktree "),
            Span::styled(&dialog.branch_name, Style::default().fg(Color::White)),
            Span::raw("?"),
        ]));
        frame.render_widget(msg, msg_area);
    }

    if let Some(err) = &dialog.error {
        let error = Paragraph::new(Span::styled(
            err.as_str(),
            Style::default().fg(Color::Red),
        ));
        frame.render_widget(error, error_area);
    }

    let hint = Paragraph::new(Span::styled(
        "y/Enter: confirm  any other key: cancel",
        Style::default().fg(Color::DarkGray),
    ))
    .alignment(Alignment::Right);
    frame.render_widget(hint, hint_area);
}
