use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tui_term::widget::PseudoTerminal;
use crate::pty::session::PtySession;
use crate::state::types::TextSelection;
use crate::ui::theme;

/// Renders the terminal panel and returns the inner area so the caller can
/// keep PTY sizes exactly in sync with the rendered dimensions.
pub fn render_terminal_panel(
    frame: &mut Frame,
    area: Rect,
    session: Option<&PtySession>,
    focused: bool,
    selection: Option<&TextSelection>,
) -> Rect {
    let title = if focused {
        " Terminal [Ctrl+\\: back] "
    } else {
        " Terminal [Enter: focus] "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(theme::border_style(focused));

    let inner = block.inner(area);

    match session {
        Some(session) => {
            let parser = session.parser.lock().unwrap();
            let widget = PseudoTerminal::new(parser.screen()).block(block);
            frame.render_widget(widget, area);
        }
        None => {
            frame.render_widget(block, area);
            let help = Paragraph::new(Line::from(vec![
                Span::styled(
                    "Select a worktree and press ",
                    Style::default().fg(theme::COLOR_DIM),
                ),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to open a terminal",
                    Style::default().fg(theme::COLOR_DIM),
                ),
            ]));
            frame.render_widget(help, inner);
        }
    }

    // Draw selection highlight overlay on top of the rendered terminal.
    if let Some(sel) = selection {
        render_selection_overlay(frame.buffer_mut(), inner, sel);
    }

    inner
}

fn render_selection_overlay(buf: &mut Buffer, inner: Rect, sel: &TextSelection) {
    let ((sr, sc), (er, ec)) = sel.ordered();

    for row in sr..=er {
        let col_start = if row == sr { sc } else { 0 };
        let col_end = if row == er {
            ec
        } else {
            inner.width.saturating_sub(1)
        };

        for col in col_start..=col_end {
            let x = inner.x + col;
            let y = inner.y + row;
            if x < inner.x + inner.width && y < inner.y + inner.height {
                let cell = &mut buf[(x, y)];
                cell.set_style(Style::default().fg(Color::Black).bg(Color::Rgb(100, 150, 220)));
            }
        }
    }
}
