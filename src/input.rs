//! Input events, key bindings, and routing between the UI and the scene.

use anyhow::Context;
use ratatui::crossterm::event::{
    self as ct, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::layout::{Position, Rect};
use serde::Deserialize;

use crate::config::KeyConfig;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Key(KeyEvent),
    MouseDown {
        col: u16,
        row: u16,
        button: MouseButton,
    },
    MouseDrag {
        col: u16,
        row: u16,
        button: MouseButton,
    },
    MouseMove {
        col: u16,
        row: u16,
    },
    Scroll(i8),
    Paste(String),
    Resize,
}

/// Converts a crossterm event, or returns `None` for events mote ignores.
pub fn from_crossterm(ev: ct::Event) -> Option<Event> {
    Some(match ev {
        ct::Event::Key(k) if k.kind != KeyEventKind::Release => Event::Key(k),
        ct::Event::Mouse(m) => {
            let (col, row) = (m.column, m.row);
            match m.kind {
                MouseEventKind::Down(button) => Event::MouseDown { col, row, button },
                MouseEventKind::Drag(button) => Event::MouseDrag { col, row, button },
                MouseEventKind::Moved => Event::MouseMove { col, row },
                MouseEventKind::ScrollUp => Event::Scroll(-1),
                MouseEventKind::ScrollDown => Event::Scroll(1),
                _ => return None,
            }
        }
        ct::Event::Paste(s) => Event::Paste(s),
        ct::Event::Resize(..) => Event::Resize,
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct KeyBinding {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

impl TryFrom<String> for KeyBinding {
    type Error = anyhow::Error;

    /// Parses a case-insensitive key name such as `q`, `esc`, `f2` or `ctrl+x`.
    fn try_from(s: String) -> anyhow::Result<Self> {
        let lower = s.trim().to_lowercase();
        let mut mods = KeyModifiers::NONE;
        let mut rest = lower.as_str();
        loop {
            if let Some(r) = rest.strip_prefix("ctrl+") {
                mods |= KeyModifiers::CONTROL;
                rest = r;
            } else if let Some(r) = rest.strip_prefix("alt+") {
                mods |= KeyModifiers::ALT;
                rest = r;
            } else {
                break;
            }
        }
        let code = match rest {
            "esc" | "escape" => KeyCode::Esc,
            "tab" => KeyCode::Tab,
            "enter" | "return" => KeyCode::Enter,
            "space" => KeyCode::Char(' '),
            c if c.chars().count() == 1 => KeyCode::Char(c.chars().next().unwrap()),
            f => KeyCode::F(
                f.strip_prefix('f')
                    .and_then(|n| n.parse().ok())
                    .filter(|n| (1..=12).contains(n))
                    .with_context(|| format!("invalid key '{s}'"))?,
            ),
        };
        Ok(KeyBinding { code, mods })
    }
}

impl KeyBinding {
    pub const fn new(code: KeyCode) -> Self {
        Self {
            code,
            mods: KeyModifiers::NONE,
        }
    }

    /// Checks a key press against this binding, ignoring Shift.
    pub fn matches(&self, k: &KeyEvent) -> bool {
        let code = match k.code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            other => other,
        };
        code == self.code && k.modifiers - KeyModifiers::SHIFT == self.mods
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Login,
    Scene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Quit,
    ToggleMatcher,
    ToggleDebug,
    FocusScene,
    FocusLogin,
    Login,
    Scene,
}

/// Decides who handles an event. `login_rect` is `None` when the login panel isn't shown.
pub fn route(ev: &Event, focus: Focus, keys: &KeyConfig, login_rect: Option<Rect>) -> Route {
    let shown = login_rect.is_some();
    let login = shown && focus == Focus::Login;
    match ev {
        Event::Key(k)
            if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            Route::Quit
        }
        Event::Key(k) if keys.toggle_matcher.matches(k) => Route::ToggleMatcher,
        Event::Key(k) if keys.toggle_debug.matches(k) => Route::ToggleDebug,
        Event::Key(k) if login && keys.focus_scene.matches(k) => Route::FocusScene,
        Event::Key(k) if !login && shown && keys.focus_login.matches(k) => Route::FocusLogin,
        Event::Key(k) if !login && keys.quit.matches(k) => Route::Quit,
        Event::Key(_) | Event::Paste(_) if login => Route::Login,
        Event::MouseDown { col, row, .. }
            if login_rect.is_some_and(|r| r.contains(Position::new(*col, *row))) =>
        {
            Route::Login
        }
        _ => Route::Scene,
    }
}

// These tests were written by Claude Code to ensure code coverage.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KeyConfig;
    use ratatui::crossterm::event::MouseEvent;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton};
    use ratatui::layout::Rect;

    fn kb() -> KeyConfig {
        KeyConfig::default()
    }
    fn key(c: KeyCode, m: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(c, m))
    }

    #[test]
    fn routes() {
        let rect = Some(Rect::new(10, 5, 40, 8));
        let r = |ev, f| route(&ev, f, &kb(), rect);
        let ctrl_c = || key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let q = || key(KeyCode::Char('q'), KeyModifiers::NONE);
        let click = |col, row| Event::MouseDown {
            col,
            row,
            button: MouseButton::Left,
        };
        assert_eq!(route(&ctrl_c(), Focus::Login, &kb(), None), Route::Quit);
        assert_eq!(r(ctrl_c(), Focus::Login), Route::Quit);
        assert_eq!(r(ctrl_c(), Focus::Scene), Route::Quit);
        assert_eq!(r(q(), Focus::Login), Route::Login);
        assert_eq!(r(q(), Focus::Scene), Route::Quit);
        assert_eq!(
            r(key(KeyCode::Esc, KeyModifiers::NONE), Focus::Login),
            Route::FocusScene
        );
        assert_eq!(
            r(key(KeyCode::Tab, KeyModifiers::NONE), Focus::Scene),
            Route::FocusLogin
        );
        assert_eq!(r(click(12, 6), Focus::Scene), Route::Login);
        assert_eq!(r(click(0, 0), Focus::Login), Route::Scene);
        assert_eq!(
            route(
                &key(KeyCode::F(2), KeyModifiers::NONE),
                Focus::Login,
                &kb(),
                None
            ),
            Route::ToggleMatcher
        );
    }

    #[test]
    fn keybinding_parse() {
        let parse = |s: &str| KeyBinding::try_from(s.to_string());
        assert!(
            parse("ctrl+x")
                .unwrap()
                .matches(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL))
        );
        assert!(parse("F12").is_ok());
        assert!(parse("f13").is_err());
        assert!(parse("hyper+q").is_err());
    }

    #[test]
    fn from_crossterm_maps_mouse_and_paste_and_ignores_releases() {
        let mouse = |kind| {
            ct::Event::Mouse(MouseEvent {
                kind,
                column: 3,
                row: 4,
                modifiers: KeyModifiers::NONE,
            })
        };
        assert_eq!(
            from_crossterm(mouse(MouseEventKind::Down(MouseButton::Left))),
            Some(Event::MouseDown {
                col: 3,
                row: 4,
                button: MouseButton::Left
            })
        );
        assert_eq!(
            from_crossterm(mouse(MouseEventKind::ScrollUp)),
            Some(Event::Scroll(-1))
        );
        assert_eq!(
            from_crossterm(ct::Event::Paste("x".into())),
            Some(Event::Paste("x".into()))
        );
        let mut release = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(from_crossterm(ct::Event::Key(release)), None);
        assert_eq!(from_crossterm(ct::Event::FocusGained), None);
    }
}
