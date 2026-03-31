# gitopiary

A full-screen TUI for managing git worktrees across multiple repositories. The left panel shows your worktrees grouped by repo, and the right panel is an embedded terminal that opens in the selected worktree's directory.

## Features

- Browse worktrees across multiple repos with git status (dirty, ahead/behind)
- Embedded PTY terminal per worktree with full TUI support (colors, box-drawing, mouse)
- Pull request badges from GitHub (`gh` CLI)
- Create and delete worktrees from the UI
- Add repos interactively
- Open worktrees in Zed editor
- Mouse click to focus panes, drag to select and copy text
- Idle indicator for terminals that have finished running a command
- Persistent cache for instant startup

## Requirements

- Rust toolchain
- `gh` CLI (for PR info)
- `zed` (optional, for the editor shortcut)

## Install

```sh
cargo install --path .
```

Or build a release binary:

```sh
cargo build --release
# binary at target/release/gitopiary
```

## Configuration

Create `~/.config/gitopiary/config.toml`:

```toml
refresh_interval_secs = 300

[[repos]]
path = "/path/to/your/repo"
name = "my-repo"  # optional display name
```

You can also add repos from within the app with `A`.

## Keybindings

Default keybindings:

| Key | Action |
|---|---|
| `j` / `k` | Navigate worktrees |
| `Enter` / `Space` | Focus terminal (creates session if needed) |
| `Ctrl+Space` | Unfocus terminal, back to list |
| `e` | Open worktree in Zed |
| `n` | New worktree |
| `d` | Delete worktree |
| `A` | Add repo |
| `r` | Refresh |
| `q` | Quit |

In the terminal pane, mouse scroll works for both mouse-aware programs and regular shell output. Drag to select text (copies to clipboard on release).

### Custom keybindings

Override any default keybinding by adding a `[keybindings]` section to your config file. When you override an action, **all** default bindings for that action are replaced. Actions you don't mention keep their defaults.

```toml
[keybindings]
esc = "unfocus_terminal"
ctrl+r = "refresh"
x = "quit"
```

If the config contains an unrecognised key name or action, gitopiary exits with an error at startup.

**Key format:** Key strings are case-insensitive. Letters: `a`--`z`. Special keys: `enter`, `space`, `tab`, `esc`, `up`, `down`, `left`, `right`, `backspace`, `delete`, `home`, `end`, `f1`--`f12`. One modifier may be prepended: `ctrl+`, `shift+`, or `alt+`.

**Available actions:** `move_down`, `move_up`, `focus_terminal`, `unfocus_terminal`, `quit`, `new_worktree`, `add_repo`, `open_editor`, `refresh`, `delete_worktree`.
