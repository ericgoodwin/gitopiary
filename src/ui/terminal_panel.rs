use alacritty_terminal::term::TermMode;
use alacritty_terminal::term::cell::Flags as CellFlags;
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::keybindings::{Action, Keybindings};
use crate::pty::session::PtySession;
use crate::ui::theme;

pub fn render_terminal_panel(
    frame: &mut Frame,
    area: Rect,
    session: Option<&PtySession>,
    focused: bool,
    keybindings: &Keybindings,
) -> Rect {
    let title = if focused {
        let key = keybindings.hint_for(Action::UnfocusTerminal).unwrap_or("?".into());
        format!(" Terminal [{}: back] ", key)
    } else {
        let key = keybindings.hint_for(Action::FocusTerminal).unwrap_or("?".into());
        format!(" Terminal [{}: focus] ", key)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(theme::border_style(focused));

    let inner = block.inner(area);

    match session {
        Some(session) => {
            frame.render_widget(block, area);
            let term = session.term.lock();
            render_term_to_buffer(&term, frame.buffer_mut(), inner);
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

    inner
}

fn render_term_to_buffer<T: alacritty_terminal::event::EventListener>(
    term: &alacritty_terminal::term::Term<T>,
    buf: &mut Buffer,
    area: Rect,
) {
    let content = term.renderable_content();
    let colors = content.colors;
    let display_offset = content.display_offset;

    for indexed in content.display_iter {
        let point = indexed.point;
        let cell = &indexed.cell;

        let col = point.column.0 as u16;
        // Convert grid-absolute line to viewport-relative.
        let viewport_line = point.line.0 + display_offset as i32;
        if viewport_line < 0 {
            continue;
        }
        let line = viewport_line as u16;

        if col >= area.width || line >= area.height {
            continue;
        }

        if cell.flags.contains(CellFlags::WIDE_CHAR_SPACER)
            || cell.flags.contains(CellFlags::LEADING_WIDE_CHAR_SPACER)
        {
            continue;
        }

        let fg = convert_color(cell.fg, colors);
        let bg = convert_color(cell.bg, colors);

        let (fg, bg) = if cell.flags.contains(CellFlags::INVERSE) {
            (bg, fg)
        } else {
            (fg, bg)
        };

        let mut modifier = Modifier::empty();
        if cell.flags.contains(CellFlags::BOLD) {
            modifier |= Modifier::BOLD;
        }
        if cell.flags.contains(CellFlags::ITALIC) {
            modifier |= Modifier::ITALIC;
        }
        if cell.flags.intersects(CellFlags::ALL_UNDERLINES) {
            modifier |= Modifier::UNDERLINED;
        }
        if cell.flags.contains(CellFlags::STRIKEOUT) {
            modifier |= Modifier::CROSSED_OUT;
        }
        if cell.flags.contains(CellFlags::DIM) {
            modifier |= Modifier::DIM;
        }
        if cell.flags.contains(CellFlags::HIDDEN) {
            modifier |= Modifier::HIDDEN;
        }

        let style = Style::default().fg(fg).bg(bg).add_modifier(modifier);
        let ch = if cell.c == ' ' || cell.c == '\0' { ' ' } else { cell.c };

        let x = area.x + col;
        let y = area.y + line;

        let buf_cell = &mut buf[(x, y)];
        buf_cell.set_char(ch);
        buf_cell.set_style(style);
    }

    // Render cursor as an inverted block
    let cursor = content.cursor;
    let cx = cursor.point.column.0 as u16;
    let cursor_vline = cursor.point.line.0 + display_offset as i32;
    if cursor_vline >= 0 && content.mode.contains(TermMode::SHOW_CURSOR) {
        let cy = cursor_vline as u16;
        if cx < area.width && cy < area.height {
            let x = area.x + cx;
            let y = area.y + cy;
            let cursor_cell = &mut buf[(x, y)];
            let existing = cursor_cell.style();
            let fg = match existing.fg {
                Some(Color::Reset) | None => Color::Black,
                Some(c) => c,
            };
            let bg = match existing.bg {
                Some(Color::Reset) | None => Color::White,
                Some(c) => c,
            };
            cursor_cell.set_style(Style::default().fg(bg).bg(fg));
        }
    }

    // Render selection overlay
    if let Some(sel_range) = content.selection {
        for line in sel_range.start.line.0..=sel_range.end.line.0 {
            let vline = line + display_offset as i32;
            if vline < 0 {
                continue;
            }
            let line_u16 = vline as u16;
            if line_u16 >= area.height {
                continue;
            }

            let col_start = if sel_range.is_block || line == sel_range.start.line.0 {
                sel_range.start.column.0 as u16
            } else {
                0
            };
            let col_end = if sel_range.is_block || line == sel_range.end.line.0 {
                sel_range.end.column.0 as u16
            } else {
                area.width.saturating_sub(1)
            };

            for col in col_start..=col_end.min(area.width.saturating_sub(1)) {
                let x = area.x + col;
                let y = area.y + line_u16;
                let cell = &mut buf[(x, y)];
                cell.set_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Rgb(100, 150, 220)),
                );
            }
        }
    }
}

fn convert_color(color: AnsiColor, colors: &Colors) -> Color {
    match color {
        AnsiColor::Named(named) => {
            if let Some(rgb) = colors[named] {
                Color::Rgb(rgb.r, rgb.g, rgb.b)
            } else {
                named_color_fallback(named)
            }
        }
        AnsiColor::Spec(rgb) => Color::Rgb(rgb.r, rgb.g, rgb.b),
        AnsiColor::Indexed(idx) => {
            if let Some(rgb) = colors[idx as usize] {
                Color::Rgb(rgb.r, rgb.g, rgb.b)
            } else {
                Color::Indexed(idx)
            }
        }
    }
}

fn named_color_fallback(named: NamedColor) -> Color {
    match named {
        NamedColor::Black => Color::Black,
        NamedColor::Red => Color::Red,
        NamedColor::Green => Color::Green,
        NamedColor::Yellow => Color::Yellow,
        NamedColor::Blue => Color::Blue,
        NamedColor::Magenta => Color::Magenta,
        NamedColor::Cyan => Color::Cyan,
        NamedColor::White => Color::White,
        NamedColor::BrightBlack => Color::DarkGray,
        NamedColor::BrightRed => Color::LightRed,
        NamedColor::BrightGreen => Color::LightGreen,
        NamedColor::BrightYellow => Color::LightYellow,
        NamedColor::BrightBlue => Color::LightBlue,
        NamedColor::BrightMagenta => Color::LightMagenta,
        NamedColor::BrightCyan => Color::LightCyan,
        NamedColor::BrightWhite => Color::White,
        NamedColor::Foreground
        | NamedColor::BrightForeground
        | NamedColor::DimForeground => Color::Rgb(0xdd, 0xdd, 0xdd),
        NamedColor::Background => Color::Rgb(0x00, 0x00, 0x00),
        NamedColor::Cursor => Color::Rgb(0xdd, 0xdd, 0xdd),
        _ => Color::Reset,
    }
}
