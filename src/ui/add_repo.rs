use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use crate::state::types::AddRepoDialog;

pub fn render_add_repo_dialog(frame: &mut Frame, area: Rect, dialog: &AddRepoDialog) {
    let dialog_width = 70u16.min(area.width.saturating_sub(4));
    let dialog_height = 8u16;

    let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
    let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect { x, y, width: dialog_width, height: dialog_height };

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Add Repository ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let [label_area, input_area, hint2_area, error_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner)[..] else {
        return;
    };

    frame.render_widget(
        Paragraph::new("Repository path (absolute or ~/...):"),
        label_area,
    );

    let input_text = if dialog.is_adding {
        format!("Adding {}...", dialog.path_input)
    } else {
        let mut s = dialog.path_input.clone();
        if dialog.cursor_pos <= s.len() {
            s.insert(dialog.cursor_pos, '│');
        }
        s
    };

    let input_style = if dialog.is_adding {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Green)),
            Span::styled(input_text, input_style),
        ])),
        input_area,
    );

    frame.render_widget(
        Paragraph::new(Span::styled(
            "The repo will be added to ~/.config/gitopiary/config.toml",
            Style::default().fg(Color::DarkGray),
        )),
        hint2_area,
    );

    if let Some(err) = &dialog.error {
        frame.render_widget(
            Paragraph::new(Span::styled(err.as_str(), Style::default().fg(Color::Red))),
            error_area,
        );
    }

    frame.render_widget(
        Paragraph::new(Span::styled(
            "Enter: add  Esc: cancel",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Right),
        hint_area,
    );
}
