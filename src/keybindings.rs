use std::collections::HashMap;
use crossterm::event::{KeyCode, KeyModifiers};
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    MoveDown,
    MoveUp,
    FocusTerminal,
    UnfocusTerminal,
    Quit,
    NewWorktree,
    AddRepo,
    OpenEditor,
    Refresh,
    DeleteWorktree,
}

#[derive(Debug, Clone)]
pub struct Keybindings {
    map: HashMap<(KeyCode, KeyModifiers), Action>,
}

impl Keybindings {
    pub fn get(&self, code: KeyCode, modifiers: KeyModifiers) -> Option<&Action> {
        self.map.get(&(code, modifiers))
    }

    /// Build keybindings by merging user overrides on top of defaults.
    ///
    /// When a user overrides an action, all default bindings for that action
    /// are removed first, so the user's config fully replaces the defaults
    /// for any action they touch.
    pub fn from_config(overrides: &HashMap<String, String>) -> Result<Self> {
        // Parse all overrides first — fail fast on any error.
        let mut parsed: Vec<((KeyCode, KeyModifiers), Action)> = Vec::new();
        for (key_str, action_str) in overrides {
            let (code, mods) = parse_key(key_str)
                .map_err(|e| anyhow!("invalid keybinding '{}': {}", key_str, e))?;
            let action = parse_action(action_str)
                .map_err(|e| anyhow!("invalid action for '{}': {}", key_str, e))?;
            parsed.push(((code, mods), action));
        }

        let mut kb = Keybindings::default();

        // Collect the set of actions being overridden, then remove all
        // default bindings for those actions.
        let overridden_actions: std::collections::HashSet<Action> =
            parsed.iter().map(|(_, action)| *action).collect();
        kb.map.retain(|_, action| !overridden_actions.contains(action));

        for ((code, mods), action) in parsed {
            kb.map.insert((code, mods), action);
        }

        Ok(kb)
    }

    /// Return a human-readable key string for the shortest binding to the given action.
    /// Returns `None` if the action is not bound.
    pub fn hint_for(&self, action: Action) -> Option<String> {
        self.map
            .iter()
            .filter(|(_, a)| **a == action)
            .map(|((code, mods), _)| format_key(*code, *mods))
            .min_by_key(|s| s.len())
    }
}

fn format_key(code: KeyCode, mods: KeyModifiers) -> String {
    let mut parts = Vec::new();

    if mods.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl".to_string());
    }
    if mods.contains(KeyModifiers::ALT) {
        parts.push("Alt".to_string());
    }
    if mods.contains(KeyModifiers::SHIFT) {
        parts.push("Shift".to_string());
    }

    let key_name = match code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::F(n) => format!("F{}", n),
        _ => format!("{:?}", code),
    };

    parts.push(key_name);
    parts.join("+")
}

/// Parse a key string like "ctrl+space", "j", "shift+a", "f12" into
/// a (KeyCode, KeyModifiers) pair. Case-insensitive.
pub fn parse_key(input: &str) -> Result<(KeyCode, KeyModifiers)> {
    let input = input.trim();
    if input.is_empty() {
        return Err(anyhow!("empty key string"));
    }

    let lower = input.to_lowercase();
    let parts: Vec<&str> = lower.split('+').collect();

    let mut modifiers = KeyModifiers::NONE;
    let key_part;

    if parts.len() == 1 {
        key_part = parts[0];
    } else if parts.len() == 2 {
        match parts[0] {
            "ctrl" => modifiers |= KeyModifiers::CONTROL,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "alt" => modifiers |= KeyModifiers::ALT,
            other => return Err(anyhow!("unknown modifier: '{}'", other)),
        }
        key_part = parts[1];
    } else {
        return Err(anyhow!("invalid key string: '{}'", input));
    }

    let code = match key_part {
        "enter" => KeyCode::Enter,
        "space" => KeyCode::Char(' '),
        "tab" => KeyCode::Tab,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "esc" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        s if s.starts_with('f') && s.len() > 1 => {
            let num: u8 = s[1..].parse()
                .map_err(|_| anyhow!("invalid function key: '{}'", key_part))?;
            if !(1..=12).contains(&num) {
                return Err(anyhow!("function key out of range: '{}'", key_part));
            }
            KeyCode::F(num)
        }
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            if ch.is_ascii_alphanumeric() || ch.is_ascii_punctuation() {
                KeyCode::Char(ch)
            } else {
                return Err(anyhow!("unsupported key: '{}'", key_part));
            }
        }
        _ => return Err(anyhow!("unknown key: '{}'", key_part)),
    };

    Ok((code, modifiers))
}

/// Parse an action string like "move_down" into an Action enum variant.
pub fn parse_action(input: &str) -> Result<Action> {
    match input.trim() {
        "move_down" => Ok(Action::MoveDown),
        "move_up" => Ok(Action::MoveUp),
        "focus_terminal" => Ok(Action::FocusTerminal),
        "unfocus_terminal" => Ok(Action::UnfocusTerminal),
        "quit" => Ok(Action::Quit),
        "new_worktree" => Ok(Action::NewWorktree),
        "add_repo" => Ok(Action::AddRepo),
        "open_editor" => Ok(Action::OpenEditor),
        "refresh" => Ok(Action::Refresh),
        "delete_worktree" => Ok(Action::DeleteWorktree),
        "" => Err(anyhow!("empty action string")),
        other => Err(anyhow!("unknown action: '{}'", other)),
    }
}

impl Default for Keybindings {
    fn default() -> Self {
        let mut map = HashMap::new();
        // Navigation
        map.insert((KeyCode::Char('j'), KeyModifiers::NONE), Action::MoveDown);
        map.insert((KeyCode::Down, KeyModifiers::NONE), Action::MoveDown);
        map.insert((KeyCode::Tab, KeyModifiers::NONE), Action::MoveDown);
        map.insert((KeyCode::Char('k'), KeyModifiers::NONE), Action::MoveUp);
        map.insert((KeyCode::Up, KeyModifiers::NONE), Action::MoveUp);
        // Terminal focus
        map.insert((KeyCode::Enter, KeyModifiers::NONE), Action::FocusTerminal);
        map.insert((KeyCode::Char(' '), KeyModifiers::NONE), Action::FocusTerminal);
        map.insert((KeyCode::Char(' '), KeyModifiers::CONTROL), Action::UnfocusTerminal);
        // Quit
        map.insert((KeyCode::Char('q'), KeyModifiers::NONE), Action::Quit);
        map.insert((KeyCode::Char('c'), KeyModifiers::CONTROL), Action::Quit);
        // Actions
        map.insert((KeyCode::Char('n'), KeyModifiers::NONE), Action::NewWorktree);
        map.insert((KeyCode::Char('a'), KeyModifiers::SHIFT), Action::AddRepo);
        map.insert((KeyCode::Char('A'), KeyModifiers::NONE), Action::AddRepo);
        map.insert((KeyCode::Char('e'), KeyModifiers::NONE), Action::OpenEditor);
        map.insert((KeyCode::Char('r'), KeyModifiers::NONE), Action::Refresh);
        map.insert((KeyCode::Char('d'), KeyModifiers::NONE), Action::DeleteWorktree);

        Self { map }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn default_keybindings_include_all_actions() {
        let kb = Keybindings::default();
        // Every action should have at least one key bound
        assert!(kb.get(KeyCode::Char('j'), KeyModifiers::NONE).is_some());
        assert!(kb.get(KeyCode::Char('k'), KeyModifiers::NONE).is_some());
        assert!(kb.get(KeyCode::Enter, KeyModifiers::NONE).is_some());
        assert!(kb.get(KeyCode::Char(' '), KeyModifiers::CONTROL).is_some());
        assert!(kb.get(KeyCode::Char('q'), KeyModifiers::NONE).is_some());
        assert!(kb.get(KeyCode::Char('c'), KeyModifiers::CONTROL).is_some());
        assert!(kb.get(KeyCode::Char('n'), KeyModifiers::NONE).is_some());
        assert!(kb.get(KeyCode::Char('e'), KeyModifiers::NONE).is_some());
        assert!(kb.get(KeyCode::Char('r'), KeyModifiers::NONE).is_some());
        assert!(kb.get(KeyCode::Char('d'), KeyModifiers::NONE).is_some());
        assert!(kb.get(KeyCode::Down, KeyModifiers::NONE).is_some());
        assert!(kb.get(KeyCode::Up, KeyModifiers::NONE).is_some());
        assert!(kb.get(KeyCode::Tab, KeyModifiers::NONE).is_some());
        assert!(kb.get(KeyCode::Char(' '), KeyModifiers::NONE).is_some());
    }

    #[test]
    fn default_actions_are_correct() {
        let kb = Keybindings::default();
        assert_eq!(kb.get(KeyCode::Char('j'), KeyModifiers::NONE), Some(&Action::MoveDown));
        assert_eq!(kb.get(KeyCode::Char('k'), KeyModifiers::NONE), Some(&Action::MoveUp));
        assert_eq!(kb.get(KeyCode::Char('q'), KeyModifiers::NONE), Some(&Action::Quit));
        assert_eq!(kb.get(KeyCode::Enter, KeyModifiers::NONE), Some(&Action::FocusTerminal));
        assert_eq!(kb.get(KeyCode::Char(' '), KeyModifiers::CONTROL), Some(&Action::UnfocusTerminal));
        assert_eq!(kb.get(KeyCode::Char('n'), KeyModifiers::NONE), Some(&Action::NewWorktree));
        assert_eq!(kb.get(KeyCode::Char('e'), KeyModifiers::NONE), Some(&Action::OpenEditor));
        assert_eq!(kb.get(KeyCode::Char('r'), KeyModifiers::NONE), Some(&Action::Refresh));
        assert_eq!(kb.get(KeyCode::Char('d'), KeyModifiers::NONE), Some(&Action::DeleteWorktree));
    }

    #[test]
    fn shift_a_maps_to_add_repo() {
        let kb = Keybindings::default();
        assert!(
            kb.get(KeyCode::Char('A'), KeyModifiers::NONE).is_some()
            || kb.get(KeyCode::Char('a'), KeyModifiers::SHIFT).is_some()
        );
    }

    #[test]
    fn parse_simple_key() {
        let (code, mods) = parse_key("j").unwrap();
        assert_eq!(code, KeyCode::Char('j'));
        assert_eq!(mods, KeyModifiers::NONE);
    }

    #[test]
    fn parse_special_keys() {
        assert_eq!(parse_key("enter").unwrap(), (KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(parse_key("space").unwrap(), (KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(parse_key("tab").unwrap(), (KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(parse_key("up").unwrap(), (KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(parse_key("down").unwrap(), (KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(parse_key("esc").unwrap(), (KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(parse_key("backspace").unwrap(), (KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(parse_key("delete").unwrap(), (KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(parse_key("home").unwrap(), (KeyCode::Home, KeyModifiers::NONE));
        assert_eq!(parse_key("end").unwrap(), (KeyCode::End, KeyModifiers::NONE));
        assert_eq!(parse_key("f1").unwrap(), (KeyCode::F(1), KeyModifiers::NONE));
        assert_eq!(parse_key("f12").unwrap(), (KeyCode::F(12), KeyModifiers::NONE));
    }

    #[test]
    fn parse_modifier_keys() {
        let (code, mods) = parse_key("ctrl+c").unwrap();
        assert_eq!(code, KeyCode::Char('c'));
        assert!(mods.contains(KeyModifiers::CONTROL));

        let (code, mods) = parse_key("ctrl+space").unwrap();
        assert_eq!(code, KeyCode::Char(' '));
        assert!(mods.contains(KeyModifiers::CONTROL));

        let (code, mods) = parse_key("shift+a").unwrap();
        assert_eq!(code, KeyCode::Char('a'));
        assert!(mods.contains(KeyModifiers::SHIFT));

        let (code, mods) = parse_key("alt+x").unwrap();
        assert_eq!(code, KeyCode::Char('x'));
        assert!(mods.contains(KeyModifiers::ALT));
    }

    #[test]
    fn parse_key_is_case_insensitive() {
        assert_eq!(parse_key("Enter").unwrap(), parse_key("enter").unwrap());
        assert_eq!(parse_key("CTRL+C").unwrap(), parse_key("ctrl+c").unwrap());
        assert_eq!(parse_key("Tab").unwrap(), parse_key("tab").unwrap());
    }

    #[test]
    fn parse_key_rejects_unknown() {
        assert!(parse_key("banana").is_err());
        assert!(parse_key("ctrl+banana").is_err());
        assert!(parse_key("").is_err());
    }

    #[test]
    fn parse_valid_actions() {
        assert_eq!(parse_action("move_down").unwrap(), Action::MoveDown);
        assert_eq!(parse_action("move_up").unwrap(), Action::MoveUp);
        assert_eq!(parse_action("focus_terminal").unwrap(), Action::FocusTerminal);
        assert_eq!(parse_action("unfocus_terminal").unwrap(), Action::UnfocusTerminal);
        assert_eq!(parse_action("quit").unwrap(), Action::Quit);
        assert_eq!(parse_action("new_worktree").unwrap(), Action::NewWorktree);
        assert_eq!(parse_action("add_repo").unwrap(), Action::AddRepo);
        assert_eq!(parse_action("open_editor").unwrap(), Action::OpenEditor);
        assert_eq!(parse_action("refresh").unwrap(), Action::Refresh);
        assert_eq!(parse_action("delete_worktree").unwrap(), Action::DeleteWorktree);
    }

    #[test]
    fn parse_action_rejects_unknown() {
        assert!(parse_action("fly_to_moon").is_err());
        assert!(parse_action("").is_err());
    }

    #[test]
    fn from_config_empty_returns_defaults() {
        let overrides = HashMap::new();
        let kb = Keybindings::from_config(&overrides).unwrap();
        assert_eq!(kb.get(KeyCode::Char('j'), KeyModifiers::NONE), Some(&Action::MoveDown));
        assert_eq!(kb.get(KeyCode::Char('q'), KeyModifiers::NONE), Some(&Action::Quit));
    }

    #[test]
    fn from_config_overrides_single_key() {
        let mut overrides = HashMap::new();
        overrides.insert("x".to_string(), "quit".to_string());
        let kb = Keybindings::from_config(&overrides).unwrap();
        // x should now be quit
        assert_eq!(kb.get(KeyCode::Char('x'), KeyModifiers::NONE), Some(&Action::Quit));
        // default quit bindings (q, ctrl+c) should be removed since user overrode the action
        assert_eq!(kb.get(KeyCode::Char('q'), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Char('c'), KeyModifiers::CONTROL), None);
    }

    #[test]
    fn from_config_displaces_default() {
        // Bind 'r' to 'quit' — removes default quit bindings and default 'r' -> 'refresh'
        let mut overrides = HashMap::new();
        overrides.insert("r".to_string(), "quit".to_string());
        let kb = Keybindings::from_config(&overrides).unwrap();
        assert_eq!(kb.get(KeyCode::Char('r'), KeyModifiers::NONE), Some(&Action::Quit));
    }

    #[test]
    fn from_config_rejects_bad_key() {
        let mut overrides = HashMap::new();
        overrides.insert("banana".to_string(), "quit".to_string());
        assert!(Keybindings::from_config(&overrides).is_err());
    }

    #[test]
    fn from_config_rejects_bad_action() {
        let mut overrides = HashMap::new();
        overrides.insert("x".to_string(), "fly_away".to_string());
        assert!(Keybindings::from_config(&overrides).is_err());
    }

    #[test]
    fn hint_for_returns_shortest_key() {
        let kb = Keybindings::default();
        // MoveDown is bound to j, down, tab — "j" is shortest
        let hint = kb.hint_for(Action::MoveDown).unwrap();
        assert_eq!(hint, "j");
    }

    #[test]
    fn hint_for_includes_modifiers() {
        let kb = Keybindings::default();
        let hint = kb.hint_for(Action::UnfocusTerminal).unwrap();
        assert_eq!(hint, "Ctrl+Space");
    }

    #[test]
    fn hint_for_unbound_returns_none() {
        let mut kb = Keybindings::default();
        // Remove all MoveDown bindings
        kb.map.retain(|_, action| *action != Action::MoveDown);
        assert!(kb.hint_for(Action::MoveDown).is_none());
    }

    #[test]
    fn full_config_round_trip() {
        // Simulate what main.rs does: parse a TOML-like HashMap
        let mut overrides = HashMap::new();
        overrides.insert("x".to_string(), "quit".to_string());
        overrides.insert("ctrl+j".to_string(), "move_down".to_string());

        let kb = Keybindings::from_config(&overrides).unwrap();

        // User overrides work
        assert_eq!(kb.get(KeyCode::Char('x'), KeyModifiers::NONE), Some(&Action::Quit));
        assert_eq!(kb.get(KeyCode::Char('j'), KeyModifiers::CONTROL), Some(&Action::MoveDown));

        // Default bindings for overridden actions are removed
        assert_eq!(kb.get(KeyCode::Char('q'), KeyModifiers::NONE), None); // quit was overridden
        assert_eq!(kb.get(KeyCode::Char('j'), KeyModifiers::NONE), None); // move_down was overridden
        assert_eq!(kb.get(KeyCode::Down, KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Tab, KeyModifiers::NONE), None);

        // Defaults for non-overridden actions still present
        assert_eq!(kb.get(KeyCode::Enter, KeyModifiers::NONE), Some(&Action::FocusTerminal));

        // Hint for overridden action shows only the user's key
        let quit_hint = kb.hint_for(Action::Quit).unwrap();
        assert_eq!(quit_hint, "x");
    }

    #[test]
    fn status_bar_hints_with_defaults() {
        let kb = Keybindings::default();
        // Every action should have a hint
        for action in [
            Action::MoveDown, Action::MoveUp, Action::FocusTerminal,
            Action::UnfocusTerminal, Action::Quit, Action::NewWorktree,
            Action::AddRepo, Action::OpenEditor, Action::Refresh,
            Action::DeleteWorktree,
        ] {
            assert!(kb.hint_for(action).is_some(), "no hint for {:?}", action);
        }
    }
}
