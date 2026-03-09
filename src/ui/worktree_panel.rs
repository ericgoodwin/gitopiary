use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use crate::pty::manager::PtyManager;
use crate::state::types::{AppState, FlatListItem, PanelFocus, PrState, Worktree};
use crate::ui::theme;

pub fn render_worktree_panel(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    pty_manager: &PtyManager,
) {
    let focused = state.focus == PanelFocus::WorktreeList;

    let block = Block::default()
        .title(" Worktrees ")
        .borders(Borders::ALL)
        .border_style(theme::border_style(focused));

    // Inner width after block borders — used to budget the branch name width.
    let inner_width = area.width.saturating_sub(2) as usize;

    let items = build_list_items(state, pty_manager, inner_width);
    let selected_flat = state.selected_flat_idx();

    let mut list_state = ListState::default();
    list_state.select(Some(selected_flat));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::selected_style());

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn build_list_items(
    state: &AppState,
    pty_manager: &PtyManager,
    inner_width: usize,
) -> Vec<ListItem<'static>> {
    let flat_items = state.flat_list_items();
    let mut list_items = vec![];

    for item in flat_items {
        match item {
            FlatListItem::Repo { idx, .. } => {
                if let Some(repo) = state.repos.get(idx) {
                    let icon = if repo.is_expanded { "▼" } else { "▶" };
                    let wt_count = repo.worktrees.len();
                    let line = Line::from(vec![
                        Span::styled(
                            format!("{} {} ", icon, repo.display_name),
                            theme::repo_header_style(),
                        ),
                        Span::styled(
                            format!("({} worktree{})", wt_count, if wt_count == 1 { "" } else { "s" }),
                            theme::dim(),
                        ),
                    ]);
                    list_items.push(ListItem::new(line));
                }
            }
            FlatListItem::Worktree { repo_idx, worktree_idx, is_selected } => {
                if let Some(repo) = state.repos.get(repo_idx) {
                    if let Some(wt) = repo.worktrees.get(worktree_idx) {
                        let idle = pty_manager
                            .get(&wt.path)
                            .map_or(false, |s| s.is_idle());
                        let line = build_worktree_line(wt, is_selected, idle, inner_width);
                        list_items.push(ListItem::new(line));
                    }
                }
            }
        }
    }

    list_items
}

fn build_worktree_line(
    wt: &Worktree,
    _is_selected: bool,
    idle: bool,
    inner_width: usize,
) -> Line<'static> {
    // Calculate how many display columns the stats portion needs so we can
    // give the branch name exactly what's left.
    let stats_cols = stats_display_width(wt);

    // Layout: 2 cols for indicator ("✓ " / "  ") + branch + stats.
    let branch_budget = inner_width.saturating_sub(2).saturating_sub(stats_cols);
    let branch = fit_branch(&wt.branch, branch_budget);

    let mut spans = vec![];

    if idle {
        spans.push(Span::styled(
            "✓ ",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ));
    } else {
        spans.push(Span::raw("  "));
    }

    spans.push(Span::styled(
        branch,
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    ));

    if wt.status.is_dirty {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("● {}", wt.status.uncommitted_changes),
            Style::default().fg(theme::COLOR_DIRTY),
        ));
    }

    if wt.status.ahead > 0 {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("↑{}", wt.status.ahead),
            Style::default().fg(theme::COLOR_AHEAD),
        ));
    }
    if wt.status.behind > 0 {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("↓{}", wt.status.behind),
            Style::default().fg(theme::COLOR_BEHIND),
        ));
    }

    if let Some(pr) = &wt.pr {
        spans.push(Span::raw("  "));
        let (color, label) = match pr.state {
            PrState::Open if pr.is_draft => (theme::COLOR_PR_DRAFT, "draft"),
            PrState::Open => (theme::COLOR_PR_OPEN, "open"),
            PrState::Closed => (theme::COLOR_PR_CLOSED, "closed"),
            PrState::Merged => (theme::COLOR_PR_MERGED, "merged"),
        };
        spans.push(Span::styled(
            format!("[#{} {}]", pr.number, label),
            Style::default().fg(color),
        ));
    }

    Line::from(spans)
}

/// Count the display columns consumed by every element that comes *after* the
/// branch name (dirty marker, ahead/behind, PR badge).  All the special
/// characters used here (●, ↑, ↓, etc.) are narrow (1 column) in every
/// standard monospace terminal font.
fn stats_display_width(wt: &Worktree) -> usize {
    let mut w = 0;

    if wt.status.is_dirty {
        // "  ● N…"  =  2 spaces + ● + space + digits
        w += 2 + 1 + 1 + digit_count(wt.status.uncommitted_changes as u64);
    }
    if wt.status.ahead > 0 {
        // " ↑N…"  =  space + ↑ + digits
        w += 1 + 1 + digit_count(wt.status.ahead as u64);
    }
    if wt.status.behind > 0 {
        w += 1 + 1 + digit_count(wt.status.behind as u64);
    }
    if let Some(pr) = &wt.pr {
        let label_len = match pr.state {
            PrState::Open if pr.is_draft => "draft".len(),
            PrState::Open => "open".len(),
            PrState::Closed => "closed".len(),
            PrState::Merged => "merged".len(),
        };
        // "  [#N label]"  =  2 + 1 + 1 + digits + 1 + label + 1
        w += 2 + 1 + 1 + digit_count(pr.number) + 1 + label_len + 1;
    }

    w
}

fn digit_count(n: u64) -> usize {
    if n == 0 { 1 } else { n.ilog10() as usize + 1 }
}

/// Truncate `branch` to fit within `budget` display columns.
/// Appends "…" when truncation occurs.
fn fit_branch(branch: &str, budget: usize) -> String {
    if budget == 0 {
        return String::new();
    }
    // Branch names are always ASCII in practice, so char count == byte count.
    // Using char_indices keeps us safe for any edge cases.
    if branch.chars().count() <= budget {
        return branch.to_string();
    }
    // Reserve 1 column for "…".
    let keep = budget.saturating_sub(1);
    let cutoff = branch
        .char_indices()
        .nth(keep)
        .map(|(i, _)| i)
        .unwrap_or(branch.len());
    format!("{}…", &branch[..cutoff])
}
