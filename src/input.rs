//! Input events, key bindings, and routing between the UI and the scene.

use anyhow::{Context, bail};
use ratatui::crossterm::event::{
    self as ct, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::layout::{Position, Rect};
use serde::Deserialize;

use crate::config::KeyConfig;

/// Terminal input, normalized.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Key(KeyEvent),
    MouseDown {
        col: u16,
        row: u16,
        button: MouseButton,
    },
    MouseUp {
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
    /// `delta` is -1 for up, +1 for down.
    Scroll {
        col: u16,
        row: u16,
        delta: i8,
    },
    Paste(String),
    Resize {
        cols: u16,
        rows: u16,
    },
}

pub fn from_crossterm(ev: ct::Event) -> Option<Event> {
    Some(match ev {
        ct::Event::Key(k) if k.kind != KeyEventKind::Release => Event::Key(k),
        ct::Event::Mouse(m) => {
            let (col, row) = (m.column, m.row);
            match m.kind {
                MouseEventKind::Down(button) => Event::MouseDown { col, row, button },
                MouseEventKind::Up(button) => Event::MouseUp { col, row, button },
                MouseEventKind::Drag(button) => Event::MouseDrag { col, row, button },
                MouseEventKind::Moved => Event::MouseMove { col, row },
                MouseEventKind::ScrollUp => Event::Scroll {
                    col,
                    row,
                    delta: -1,
                },
                MouseEventKind::ScrollDown => Event::Scroll { col, row, delta: 1 },
                _ => return None,
            }
        }
        ct::Event::Paste(s) => Event::Paste(s),
        ct::Event::Resize(cols, rows) => Event::Resize { cols, rows },
        _ => return None,
    })
}

/// A key plus modifiers, written in config as e.g. `"q"`, `"f2"` or `"ctrl+x"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct KeyBinding {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

impl TryFrom<String> for KeyBinding {
    type Error = anyhow::Error;

    fn try_from(s: String) -> anyhow::Result<Self> {
        Self::parse(&s)
    }
}

impl KeyBinding {
    pub const fn new(code: KeyCode) -> Self {
        Self {
            code,
            mods: KeyModifiers::NONE,
        }
    }

    /// Parses `"q"`, `"esc"`, `"tab"`, `"enter"`, `"f1"`..`"f12"`, with optional
    /// `ctrl+` / `alt+` prefixes. Case-insensitive.
    pub fn parse(s: &str) -> anyhow::Result<KeyBinding> {
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
            f if f.starts_with('f') && f.len() > 1 => {
                let n: u8 = f[1..]
                    .parse()
                    .with_context(|| format!("invalid key '{s}'"))?;
                if !(1..=12).contains(&n) {
                    bail!("invalid key '{s}'");
                }
                KeyCode::F(n)
            }
            c if c.chars().count() == 1 => KeyCode::Char(c.chars().next().unwrap()),
            _ => bail!("invalid key '{s}'"),
        };
        Ok(KeyBinding { code, mods })
    }

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

/// Who handles an event.
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

/// Decides who handles `ev`. `login_rect` is `None` when no login panel is shown.
pub fn route(ev: &Event, focus: Focus, keys: &KeyConfig, login_rect: Option<Rect>) -> Route {
    let focused = if login_rect.is_some() && focus == Focus::Login {
        Route::Login
    } else {
        Route::Scene
    };
    match ev {
        Event::Key(k) => {
            if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
                Route::Quit
            } else if keys.toggle_matcher.matches(k) {
                Route::ToggleMatcher
            } else if keys.toggle_debug.matches(k) {
                Route::ToggleDebug
            } else if focused == Route::Login && keys.focus_scene.matches(k) {
                Route::FocusScene
            } else if focused == Route::Scene && login_rect.is_some() && keys.focus_login.matches(k)
            {
                Route::FocusLogin
            } else if focused == Route::Scene && keys.quit.matches(k) {
                Route::Quit
            } else {
                focused
            }
        }
        Event::Paste(_) => focused,
        Event::MouseDown { col, row, .. } => match login_rect {
            Some(r) if r.contains(Position::new(*col, *row)) => Route::Login,
            _ => Route::Scene,
        },
        _ => Route::Scene,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KeyConfig;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton};
    use ratatui::layout::Rect;

    fn kb() -> KeyConfig {
        KeyConfig::default()
    }
    fn key(c: KeyCode, m: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(c, m))
    }

    #[test]
    fn ctrl_c_always_quits() {
        for f in [Focus::Login, Focus::Scene] {
            assert!(matches!(
                route(
                    &key(KeyCode::Char('c'), KeyModifiers::CONTROL),
                    f,
                    &kb(),
                    None
                ),
                Route::Quit
            ));
        }
    }

    #[test]
    fn q_types_into_login_but_quits_scene() {
        let r = Some(Rect::new(10, 5, 40, 8));
        assert!(matches!(
            route(
                &key(KeyCode::Char('q'), KeyModifiers::NONE),
                Focus::Login,
                &kb(),
                r
            ),
            Route::Login
        ));
        assert!(matches!(
            route(
                &key(KeyCode::Char('q'), KeyModifiers::NONE),
                Focus::Scene,
                &kb(),
                r
            ),
            Route::Quit
        ));
    }

    #[test]
    fn focus_keys_and_click() {
        let r = Some(Rect::new(10, 5, 40, 8));
        assert!(matches!(
            route(
                &key(KeyCode::Esc, KeyModifiers::NONE),
                Focus::Login,
                &kb(),
                r
            ),
            Route::FocusScene
        ));
        assert!(matches!(
            route(
                &key(KeyCode::Tab, KeyModifiers::NONE),
                Focus::Scene,
                &kb(),
                r
            ),
            Route::FocusLogin
        ));
        assert!(matches!(
            route(
                &Event::MouseDown {
                    col: 12,
                    row: 6,
                    button: MouseButton::Left
                },
                Focus::Scene,
                &kb(),
                r
            ),
            Route::Login
        ));
        assert!(matches!(
            route(
                &Event::MouseDown {
                    col: 0,
                    row: 0,
                    button: MouseButton::Left
                },
                Focus::Login,
                &kb(),
                r
            ),
            Route::Scene
        ));
    }

    #[test]
    fn f2_toggles_matcher_from_login() {
        assert!(matches!(
            route(
                &key(KeyCode::F(2), KeyModifiers::NONE),
                Focus::Login,
                &kb(),
                None
            ),
            Route::ToggleMatcher
        ));
    }

    #[test]
    fn keybinding_parse() {
        assert!(
            KeyBinding::parse("ctrl+x")
                .unwrap()
                .matches(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL))
        );
        assert!(KeyBinding::parse("F12").is_ok());
        assert!(KeyBinding::parse("hyper+q").is_err());
    }
}
