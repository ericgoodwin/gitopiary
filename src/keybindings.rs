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
            .min_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)))
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

    let parts: Vec<&str> = input.split('+').collect();

    let mut modifiers = KeyModifiers::NONE;
    let raw_key_part;

    if parts.len() == 1 {
        raw_key_part = parts[0];
    } else if parts.len() == 2 {
        match parts[0].to_lowercase().as_str() {
            "ctrl" => modifiers |= KeyModifiers::CONTROL,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "alt" => modifiers |= KeyModifiers::ALT,
            other => return Err(anyhow!("unknown modifier: '{}'", other)),
        }
        raw_key_part = parts[1];
    } else {
        return Err(anyhow!("invalid key string: '{}'", input));
    }

    // Lowercase for matching special key names, but preserve original case
    // for single-character keys (e.g., "A" should stay as KeyCode::Char('A')).
    let key_lower = raw_key_part.to_lowercase();
    let code = match key_lower.as_str() {
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
                .map_err(|_| anyhow!("invalid function key: '{}'", raw_key_part))?;
            if !(1..=12).contains(&num) {
                return Err(anyhow!("function key out of range: '{}'", raw_key_part));
            }
            KeyCode::F(num)
        }
        s if s.len() == 1 => {
            let ch = raw_key_part.chars().next().unwrap();
            if ch.is_ascii_alphanumeric() || ch.is_ascii_punctuation() {
                KeyCode::Char(ch)
            } else {
                return Err(anyhow!("unsupported key: '{}'", raw_key_part));
            }
        }
        _ => return Err(anyhow!("unknown key: '{}'", raw_key_part)),
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
    fn parse_key_is_case_insensitive_for_special_keys_and_modifiers() {
        // Special key names are case-insensitive
        assert_eq!(parse_key("Enter").unwrap(), parse_key("enter").unwrap());
        assert_eq!(parse_key("Tab").unwrap(), parse_key("tab").unwrap());
        assert_eq!(parse_key("ESC").unwrap(), parse_key("esc").unwrap());
        // Modifier names are case-insensitive
        assert_eq!(parse_key("CTRL+space").unwrap(), parse_key("ctrl+space").unwrap());
        assert_eq!(parse_key("Shift+tab").unwrap(), parse_key("shift+tab").unwrap());
    }

    #[test]
    fn parse_key_preserves_case_for_single_chars() {
        let (code_upper, _) = parse_key("A").unwrap();
        let (code_lower, _) = parse_key("a").unwrap();
        assert_eq!(code_upper, KeyCode::Char('A'));
        assert_eq!(code_lower, KeyCode::Char('a'));
        assert_ne!(code_upper, code_lower);
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

    // --- Override-replaces-defaults: edge cases ---

    #[test]
    fn override_action_removes_all_defaults_but_leaves_other_actions_fully_intact() {
        // Override only Quit. Every other action's full set of default bindings
        // should remain untouched.
        let mut overrides = HashMap::new();
        overrides.insert("x".to_string(), "quit".to_string());
        let kb = Keybindings::from_config(&overrides).unwrap();

        // MoveDown should retain all three default bindings
        assert_eq!(kb.get(KeyCode::Char('j'), KeyModifiers::NONE), Some(&Action::MoveDown));
        assert_eq!(kb.get(KeyCode::Down, KeyModifiers::NONE), Some(&Action::MoveDown));
        assert_eq!(kb.get(KeyCode::Tab, KeyModifiers::NONE), Some(&Action::MoveDown));

        // MoveUp retains both
        assert_eq!(kb.get(KeyCode::Char('k'), KeyModifiers::NONE), Some(&Action::MoveUp));
        assert_eq!(kb.get(KeyCode::Up, KeyModifiers::NONE), Some(&Action::MoveUp));

        // FocusTerminal retains both
        assert_eq!(kb.get(KeyCode::Enter, KeyModifiers::NONE), Some(&Action::FocusTerminal));
        assert_eq!(kb.get(KeyCode::Char(' '), KeyModifiers::NONE), Some(&Action::FocusTerminal));

        // UnfocusTerminal unchanged
        assert_eq!(kb.get(KeyCode::Char(' '), KeyModifiers::CONTROL), Some(&Action::UnfocusTerminal));

        // Other single-key actions unchanged
        assert_eq!(kb.get(KeyCode::Char('n'), KeyModifiers::NONE), Some(&Action::NewWorktree));
        assert_eq!(kb.get(KeyCode::Char('e'), KeyModifiers::NONE), Some(&Action::OpenEditor));
        assert_eq!(kb.get(KeyCode::Char('r'), KeyModifiers::NONE), Some(&Action::Refresh));
        assert_eq!(kb.get(KeyCode::Char('d'), KeyModifiers::NONE), Some(&Action::DeleteWorktree));
    }

    #[test]
    fn override_reuses_key_from_different_default_action() {
        // Bind 'j' (default MoveDown) to Refresh.
        // This should remove all default MoveDown bindings AND all default Refresh bindings,
        // then map j -> Refresh.
        let mut overrides = HashMap::new();
        overrides.insert("j".to_string(), "refresh".to_string());
        let kb = Keybindings::from_config(&overrides).unwrap();

        // j is now Refresh
        assert_eq!(kb.get(KeyCode::Char('j'), KeyModifiers::NONE), Some(&Action::Refresh));

        // All other default MoveDown bindings are removed (overriding Refresh
        // does not touch MoveDown defaults -- but MoveDown was NOT overridden,
        // wait: actually 'j' was a default for MoveDown, but the override is for Refresh action.
        // The from_config logic removes defaults for overridden *actions* (Refresh),
        // then inserts user keys. The old 'j' -> MoveDown entry gets overwritten
        // by the insert of 'j' -> Refresh.
        // So MoveDown's other bindings (Down, Tab) should still exist.
        assert_eq!(kb.get(KeyCode::Down, KeyModifiers::NONE), Some(&Action::MoveDown));
        assert_eq!(kb.get(KeyCode::Tab, KeyModifiers::NONE), Some(&Action::MoveDown));

        // Default 'r' -> Refresh should be gone (Refresh action was overridden)
        assert_eq!(kb.get(KeyCode::Char('r'), KeyModifiers::NONE), None);
    }

    #[test]
    fn multiple_overrides_for_same_action() {
        // User provides two keys for Quit; all default Quit bindings should be gone,
        // and both user keys should work.
        let mut overrides = HashMap::new();
        overrides.insert("x".to_string(), "quit".to_string());
        overrides.insert("ctrl+q".to_string(), "quit".to_string());
        let kb = Keybindings::from_config(&overrides).unwrap();

        assert_eq!(kb.get(KeyCode::Char('x'), KeyModifiers::NONE), Some(&Action::Quit));
        assert_eq!(kb.get(KeyCode::Char('q'), KeyModifiers::CONTROL), Some(&Action::Quit));

        // Default quit keys gone
        assert_eq!(kb.get(KeyCode::Char('q'), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Char('c'), KeyModifiers::CONTROL), None);
    }

    #[test]
    fn override_swaps_two_actions() {
        // Swap quit and refresh: r -> quit, q -> refresh
        let mut overrides = HashMap::new();
        overrides.insert("r".to_string(), "quit".to_string());
        overrides.insert("q".to_string(), "refresh".to_string());
        let kb = Keybindings::from_config(&overrides).unwrap();

        assert_eq!(kb.get(KeyCode::Char('r'), KeyModifiers::NONE), Some(&Action::Quit));
        assert_eq!(kb.get(KeyCode::Char('q'), KeyModifiers::NONE), Some(&Action::Refresh));

        // Default ctrl+c for Quit should be gone (Quit was overridden)
        assert_eq!(kb.get(KeyCode::Char('c'), KeyModifiers::CONTROL), None);
    }

    #[test]
    fn get_returns_none_for_unbound_key() {
        let kb = Keybindings::default();
        assert_eq!(kb.get(KeyCode::Char('z'), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::F(5), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Char('j'), KeyModifiers::CONTROL), None);
    }

    // --- parse_key edge cases ---

    #[test]
    fn parse_key_trims_whitespace() {
        let (code, mods) = parse_key("  j  ").unwrap();
        assert_eq!(code, KeyCode::Char('j'));
        assert_eq!(mods, KeyModifiers::NONE);
    }

    #[test]
    fn parse_key_rejects_triple_modifier() {
        // ctrl+shift+a has three parts, which the parser does not support
        assert!(parse_key("ctrl+shift+a").is_err());
    }

    #[test]
    fn parse_key_rejects_modifier_with_empty_key() {
        // "ctrl+" splits into ["ctrl", ""] -- empty key part
        assert!(parse_key("ctrl+").is_err());
    }

    #[test]
    fn parse_key_function_key_boundaries() {
        // f0 is out of range (valid range 1..=12)
        assert!(parse_key("f0").is_err());
        // f13 is out of range
        assert!(parse_key("f13").is_err());
        // f1 and f12 are valid (already tested in parse_special_keys, but
        // boundary values are worth calling out explicitly)
        assert!(parse_key("f1").is_ok());
        assert!(parse_key("f12").is_ok());
    }

    #[test]
    fn parse_key_punctuation_characters() {
        // Punctuation like / and . should parse as Char keys
        let (code, mods) = parse_key("/").unwrap();
        assert_eq!(code, KeyCode::Char('/'));
        assert_eq!(mods, KeyModifiers::NONE);

        let (code, mods) = parse_key(".").unwrap();
        assert_eq!(code, KeyCode::Char('.'));
        assert_eq!(mods, KeyModifiers::NONE);
    }

    #[test]
    fn parse_key_left_right_arrow_keys() {
        assert_eq!(parse_key("left").unwrap(), (KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(parse_key("right").unwrap(), (KeyCode::Right, KeyModifiers::NONE));
    }

    #[test]
    fn parse_key_modifier_with_special_key() {
        let (code, mods) = parse_key("ctrl+enter").unwrap();
        assert_eq!(code, KeyCode::Enter);
        assert!(mods.contains(KeyModifiers::CONTROL));

        let (code, mods) = parse_key("shift+tab").unwrap();
        assert_eq!(code, KeyCode::Tab);
        assert!(mods.contains(KeyModifiers::SHIFT));

        let (code, mods) = parse_key("alt+f1").unwrap();
        assert_eq!(code, KeyCode::F(1));
        assert!(mods.contains(KeyModifiers::ALT));
    }

    #[test]
    fn parse_key_rejects_unknown_modifier() {
        assert!(parse_key("super+a").is_err());
        assert!(parse_key("meta+a").is_err());
    }

    // --- parse_action edge cases ---

    #[test]
    fn parse_action_trims_whitespace() {
        assert_eq!(parse_action("  quit  ").unwrap(), Action::Quit);
        assert_eq!(parse_action("\tmove_down\n").unwrap(), Action::MoveDown);
    }

    #[test]
    fn parse_action_is_case_sensitive() {
        // Actions are lowercase only; mixed case should fail
        assert!(parse_action("Move_Down").is_err());
        assert!(parse_action("QUIT").is_err());
        assert!(parse_action("Quit").is_err());
    }

    // --- format_key correctness ---

    #[test]
    fn format_key_produces_expected_strings() {
        assert_eq!(format_key(KeyCode::Char('j'), KeyModifiers::NONE), "j");
        assert_eq!(format_key(KeyCode::Enter, KeyModifiers::NONE), "Enter");
        assert_eq!(format_key(KeyCode::Char(' '), KeyModifiers::NONE), "Space");
        assert_eq!(format_key(KeyCode::Char(' '), KeyModifiers::CONTROL), "Ctrl+Space");
        assert_eq!(format_key(KeyCode::Char('a'), KeyModifiers::SHIFT), "Shift+a");
        assert_eq!(format_key(KeyCode::Char('x'), KeyModifiers::ALT), "Alt+x");
        assert_eq!(format_key(KeyCode::F(5), KeyModifiers::NONE), "F5");
        assert_eq!(format_key(KeyCode::Tab, KeyModifiers::NONE), "Tab");
        assert_eq!(format_key(KeyCode::Esc, KeyModifiers::NONE), "Esc");
    }

    #[test]
    fn format_key_multiple_modifiers() {
        // While parse_key does not support multiple modifiers, format_key should
        // handle them correctly since KeyModifiers is a bitflag.
        let mods = KeyModifiers::CONTROL | KeyModifiers::ALT;
        let result = format_key(KeyCode::Char('x'), mods);
        assert!(result.contains("Ctrl"));
        assert!(result.contains("Alt"));
        assert!(result.contains("x"));
    }

    // --- hint_for after override ---

    #[test]
    fn hint_for_reflects_overrides() {
        let mut overrides = HashMap::new();
        overrides.insert("f1".to_string(), "quit".to_string());
        let kb = Keybindings::from_config(&overrides).unwrap();

        // Only f1 should be bound to Quit, so hint should be "F1"
        assert_eq!(kb.hint_for(Action::Quit), Some("F1".to_string()));

        // Refresh was not overridden and 'r' is its only default binding,
        // so hint should still be "r"
        assert_eq!(kb.hint_for(Action::Refresh), Some("r".to_string()));
    }

    #[test]
    fn hint_for_action_unbound_after_override() {
        // Override Refresh to something, then check if the Refresh action
        // has no default binding for its old key
        let mut overrides = HashMap::new();
        overrides.insert("f2".to_string(), "refresh".to_string());
        let kb = Keybindings::from_config(&overrides).unwrap();

        // 'r' should no longer be bound (Refresh defaults were cleared)
        assert_eq!(kb.get(KeyCode::Char('r'), KeyModifiers::NONE), None);
        // Only f2 is bound to Refresh
        assert_eq!(kb.hint_for(Action::Refresh), Some("F2".to_string()));
    }

    // --- TOML config deserialization integration ---

    #[test]
    fn toml_keybindings_section_deserializes_to_hashmap() {
        let toml_str = r#"
[keybindings]
esc = "unfocus_terminal"
"ctrl+r" = "refresh"
x = "quit"
"#;
        let config: crate::config::Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.keybindings.len(), 3);
        assert_eq!(config.keybindings.get("esc").unwrap(), "unfocus_terminal");
        assert_eq!(config.keybindings.get("ctrl+r").unwrap(), "refresh");
        assert_eq!(config.keybindings.get("x").unwrap(), "quit");
    }

    #[test]
    fn toml_keybindings_round_trip_through_from_config() {
        // Simulate the full path: TOML -> Config -> Keybindings
        let toml_str = r#"
[keybindings]
esc = "unfocus_terminal"
x = "quit"
"#;
        let config: crate::config::Config = toml::from_str(toml_str).unwrap();
        let kb = Keybindings::from_config(&config.keybindings).unwrap();

        // esc -> UnfocusTerminal (user override)
        assert_eq!(kb.get(KeyCode::Esc, KeyModifiers::NONE), Some(&Action::UnfocusTerminal));
        // default ctrl+space for UnfocusTerminal should be gone
        assert_eq!(kb.get(KeyCode::Char(' '), KeyModifiers::CONTROL), None);

        // x -> Quit (user override)
        assert_eq!(kb.get(KeyCode::Char('x'), KeyModifiers::NONE), Some(&Action::Quit));
        // default q and ctrl+c for Quit should be gone
        assert_eq!(kb.get(KeyCode::Char('q'), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Char('c'), KeyModifiers::CONTROL), None);

        // Non-overridden actions remain
        assert_eq!(kb.get(KeyCode::Char('j'), KeyModifiers::NONE), Some(&Action::MoveDown));
    }

    #[test]
    fn toml_missing_keybindings_section_produces_defaults() {
        let toml_str = r#"
refresh_interval_secs = 10
"#;
        let config: crate::config::Config = toml::from_str(toml_str).unwrap();
        assert!(config.keybindings.is_empty());
        let kb = Keybindings::from_config(&config.keybindings).unwrap();
        // Should be full defaults
        assert_eq!(kb.get(KeyCode::Char('q'), KeyModifiers::NONE), Some(&Action::Quit));
        assert_eq!(kb.get(KeyCode::Char('j'), KeyModifiers::NONE), Some(&Action::MoveDown));
    }

    #[test]
    fn from_config_with_one_bad_entry_rejects_entire_config() {
        // If even one entry is invalid, the whole config should fail.
        // This tests that we fail fast rather than silently ignoring bad entries.
        let mut overrides = HashMap::new();
        overrides.insert("x".to_string(), "quit".to_string());
        overrides.insert("banana".to_string(), "refresh".to_string());
        assert!(Keybindings::from_config(&overrides).is_err());
    }

    // --- Additional edge cases ---

    #[test]
    fn parse_key_digit_characters() {
        // Digits are alphanumeric and should parse as Char keys
        let (code, mods) = parse_key("1").unwrap();
        assert_eq!(code, KeyCode::Char('1'));
        assert_eq!(mods, KeyModifiers::NONE);

        let (code, mods) = parse_key("0").unwrap();
        assert_eq!(code, KeyCode::Char('0'));
        assert_eq!(mods, KeyModifiers::NONE);

        // Modifier + digit
        let (code, mods) = parse_key("ctrl+5").unwrap();
        assert_eq!(code, KeyCode::Char('5'));
        assert!(mods.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn parse_key_modifier_with_arrow_keys() {
        let (code, mods) = parse_key("ctrl+up").unwrap();
        assert_eq!(code, KeyCode::Up);
        assert!(mods.contains(KeyModifiers::CONTROL));

        let (code, mods) = parse_key("alt+down").unwrap();
        assert_eq!(code, KeyCode::Down);
        assert!(mods.contains(KeyModifiers::ALT));

        let (code, mods) = parse_key("shift+left").unwrap();
        assert_eq!(code, KeyCode::Left);
        assert!(mods.contains(KeyModifiers::SHIFT));

        let (code, mods) = parse_key("ctrl+right").unwrap();
        assert_eq!(code, KeyCode::Right);
        assert!(mods.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn from_config_override_all_actions_clears_entire_default_map() {
        // Override every action to verify no stale defaults remain
        let mut overrides = HashMap::new();
        overrides.insert("f1".to_string(), "move_down".to_string());
        overrides.insert("f2".to_string(), "move_up".to_string());
        overrides.insert("f3".to_string(), "focus_terminal".to_string());
        overrides.insert("f4".to_string(), "unfocus_terminal".to_string());
        overrides.insert("f5".to_string(), "quit".to_string());
        overrides.insert("f6".to_string(), "new_worktree".to_string());
        overrides.insert("f7".to_string(), "add_repo".to_string());
        overrides.insert("f8".to_string(), "open_editor".to_string());
        overrides.insert("f9".to_string(), "refresh".to_string());
        overrides.insert("f10".to_string(), "delete_worktree".to_string());

        let kb = Keybindings::from_config(&overrides).unwrap();

        // All default keys should be gone
        assert_eq!(kb.get(KeyCode::Char('j'), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Char('k'), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Down, KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Up, KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Tab, KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Enter, KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Char(' '), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Char(' '), KeyModifiers::CONTROL), None);
        assert_eq!(kb.get(KeyCode::Char('q'), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Char('c'), KeyModifiers::CONTROL), None);
        assert_eq!(kb.get(KeyCode::Char('n'), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Char('a'), KeyModifiers::SHIFT), None);
        assert_eq!(kb.get(KeyCode::Char('A'), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Char('e'), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Char('r'), KeyModifiers::NONE), None);
        assert_eq!(kb.get(KeyCode::Char('d'), KeyModifiers::NONE), None);

        // Only function keys should be bound
        assert_eq!(kb.get(KeyCode::F(1), KeyModifiers::NONE), Some(&Action::MoveDown));
        assert_eq!(kb.get(KeyCode::F(2), KeyModifiers::NONE), Some(&Action::MoveUp));
        assert_eq!(kb.get(KeyCode::F(3), KeyModifiers::NONE), Some(&Action::FocusTerminal));
        assert_eq!(kb.get(KeyCode::F(4), KeyModifiers::NONE), Some(&Action::UnfocusTerminal));
        assert_eq!(kb.get(KeyCode::F(5), KeyModifiers::NONE), Some(&Action::Quit));
        assert_eq!(kb.get(KeyCode::F(6), KeyModifiers::NONE), Some(&Action::NewWorktree));
        assert_eq!(kb.get(KeyCode::F(7), KeyModifiers::NONE), Some(&Action::AddRepo));
        assert_eq!(kb.get(KeyCode::F(8), KeyModifiers::NONE), Some(&Action::OpenEditor));
        assert_eq!(kb.get(KeyCode::F(9), KeyModifiers::NONE), Some(&Action::Refresh));
        assert_eq!(kb.get(KeyCode::F(10), KeyModifiers::NONE), Some(&Action::DeleteWorktree));
    }

    #[test]
    fn hint_for_deterministic_tiebreaker_on_equal_length() {
        // When two bindings have the same formatted length, hint_for should
        // return a deterministic result (lexicographically smallest).
        let mut overrides = HashMap::new();
        // Bind both 'a' and 'b' to Quit -- both format to length 1
        overrides.insert("a".to_string(), "quit".to_string());
        overrides.insert("b".to_string(), "quit".to_string());
        let kb = Keybindings::from_config(&overrides).unwrap();

        let hint = kb.hint_for(Action::Quit).unwrap();
        assert_eq!(hint, "a"); // 'a' < 'b' lexicographically
    }

    #[test]
    fn from_config_duplicate_key_last_write_wins() {
        // If the same key appears mapped to two different actions, HashMap
        // deduplicates so only one survives. Verify from_config handles this
        // without panicking -- the exact winner depends on HashMap iteration
        // order, but the result should be valid.
        let mut overrides = HashMap::new();
        overrides.insert("x".to_string(), "quit".to_string());
        // Inserting same key with different action overwrites in HashMap
        overrides.insert("x".to_string(), "refresh".to_string());
        let kb = Keybindings::from_config(&overrides).unwrap();

        // HashMap deduplication means only "refresh" survives
        assert_eq!(kb.get(KeyCode::Char('x'), KeyModifiers::NONE), Some(&Action::Refresh));
    }

    #[test]
    fn toml_keybindings_with_modifier_keys_requiring_quotes() {
        // In TOML, keys with '+' must be quoted. Verify this works correctly.
        let toml_str = r#"
[keybindings]
"ctrl+space" = "unfocus_terminal"
"shift+a" = "add_repo"
"alt+r" = "refresh"
"#;
        let config: crate::config::Config = toml::from_str(toml_str).unwrap();
        let kb = Keybindings::from_config(&config.keybindings).unwrap();

        assert_eq!(kb.get(KeyCode::Char(' '), KeyModifiers::CONTROL), Some(&Action::UnfocusTerminal));
        assert_eq!(kb.get(KeyCode::Char('a'), KeyModifiers::SHIFT), Some(&Action::AddRepo));
        assert_eq!(kb.get(KeyCode::Char('r'), KeyModifiers::ALT), Some(&Action::Refresh));
    }

    #[test]
    fn from_config_override_does_not_affect_key_used_by_unrelated_action() {
        // Override Quit to 'x'. The key 'q' was default-bound to Quit and
        // should be unbound. But 'j' (MoveDown) should be completely unaffected.
        let mut overrides = HashMap::new();
        overrides.insert("x".to_string(), "quit".to_string());
        let kb = Keybindings::from_config(&overrides).unwrap();

        // 'q' gone (Quit defaults removed)
        assert_eq!(kb.get(KeyCode::Char('q'), KeyModifiers::NONE), None);
        // 'j' still MoveDown (different action, not touched)
        assert_eq!(kb.get(KeyCode::Char('j'), KeyModifiers::NONE), Some(&Action::MoveDown));
        // 'x' is Quit
        assert_eq!(kb.get(KeyCode::Char('x'), KeyModifiers::NONE), Some(&Action::Quit));
    }
}
