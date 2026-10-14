//! The stand-in login panel and the authentication boundary.

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

use crate::config::{ColorConfig, LoginPosition};
use crate::glyphs::DenyList;
use crate::input::Event;
use crate::ui::text_input::TextInput;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("authentication not implemented")]
    NotImplemented,
}

/// The authentication boundary. A real implementation (PAM/greetd) replaces this.
pub fn authenticate(_username: &str, _password: &str) -> Result<(), AuthError> {
    Err(AuthError::NotImplemented)
}

/// Credentials from a submitted form. Deliberately not `Debug`, so the
/// password can't end up in a log by accident.
pub struct LoginSubmit {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    User,
    Pass,
}

pub struct LoginPanel {
    deny: DenyList,
    border: border::Set<'static>,
    user: TextInput,
    pass: TextInput,
    field: Field,
    status: String,
}

/// ASCII fallback for when box-drawing characters are denied.
const ASCII_BORDER: border::Set<'static> = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

const PANEL_SIZE: (u16, u16) = (40, 8);
const MIN_SIZE: (u16, u16) = (20, 5);

/// Where the panel goes on `screen`, shrunk to fit; `None` if the screen is too small.
pub fn panel_rect(screen: Rect, pos: LoginPosition) -> Option<Rect> {
    if screen.width < MIN_SIZE.0 || screen.height < MIN_SIZE.1 {
        return None;
    }
    let w = PANEL_SIZE.0.min(screen.width);
    let h = PANEL_SIZE.1.min(screen.height);
    let x = screen.x + (screen.width - w) / 2;
    let spare = screen.height - h;
    let y = screen.y
        + match pos {
            LoginPosition::Center => spare / 2,
            LoginPosition::Top => spare.min(1),
            LoginPosition::Bottom => spare - spare.min(1),
        };
    Some(Rect::new(x, y, w, h))
}

impl LoginPanel {
    /// Characters in `deny` are never accepted as input or drawn.
    pub fn new(deny: DenyList) -> Self {
        let p = border::PLAIN;
        let box_chars = [
            p.top_left,
            p.top_right,
            p.bottom_left,
            p.bottom_right,
            p.vertical_left,
            p.horizontal_top,
        ];
        let border = if box_chars.concat().chars().any(|c| deny.is_denied(c)) {
            ASCII_BORDER
        } else {
            p
        };
        Self {
            deny,
            border,
            user: TextInput::new(false),
            pass: TextInput::new(true),
            field: Field::User,
            status: String::new(),
        }
    }

    fn active(&mut self) -> &mut TextInput {
        match self.field {
            Field::User => &mut self.user,
            Field::Pass => &mut self.pass,
        }
    }

    pub fn set_status(&mut self, s: impl Into<String>) {
        self.status = s.into();
    }

    /// Returns the credentials when the form is submitted; the password field
    /// is cleared at that point.
    pub fn handle_event(&mut self, ev: &Event) -> Option<LoginSubmit> {
        match ev {
            Event::Paste(s) => {
                let allowed: String = s.chars().filter(|&c| !self.deny.is_denied(c)).collect();
                self.active().insert_str(&allowed);
            }
            Event::Key(k) => match k.code {
                KeyCode::Char(c) if self.deny.is_denied(c) => {}
                KeyCode::Up => self.field = Field::User,
                KeyCode::Down => self.field = Field::Pass,
                _ if self.active().handle_key(k) => {
                    if self.field == Field::User {
                        self.field = Field::Pass;
                    } else {
                        let username = self.user.value().to_string();
                        let password = self.pass.value().to_string();
                        self.pass.clear();
                        return Some(LoginSubmit { username, password });
                    }
                }
                _ => {}
            },
            _ => {}
        }
        None
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, focused: bool, colors: &ColorConfig) {
        let accent = colors.ui_accent;
        let base = Style::default().fg(colors.ui_fg).bg(colors.bg);
        let border_style = if focused { base.fg(accent) } else { base };

        Clear.render(area, buf);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(self.border)
            .title(" login ")
            .style(base)
            .border_style(border_style);
        let inner = block.inner(area);
        block.render(area, buf);

        let width = inner.width.saturating_sub(6) as usize;
        let field_line = |label: &'static str, input: &TextInput, active: bool| {
            let label_style = if active && focused {
                base.fg(accent).add_modifier(Modifier::BOLD)
            } else {
                base
            };
            let mut spans = vec![Span::styled(format!("{label} "), label_style)];
            spans.extend(field_spans(input, width, active && focused, base));
            Line::from(spans)
        };
        let lines = vec![
            Line::default(),
            field_line("user", &self.user, self.field == Field::User),
            field_line("pass", &self.pass, self.field == Field::Pass),
            Line::default(),
            Line::from(Span::styled(self.status.clone(), base)),
        ];
        Paragraph::new(lines)
            .style(base)
            .wrap(Wrap { trim: true })
            .render(inner, buf);
    }
}

/// The visible window of a field, with the cursor cell reversed when active.
fn field_spans(
    input: &TextInput,
    width: usize,
    show_cursor: bool,
    style: Style,
) -> Vec<Span<'static>> {
    let chars: Vec<char> = input.display().chars().collect();
    let cursor = input.cursor();
    let width = width.max(1);
    let start = (cursor + 1).saturating_sub(width);
    let visible: Vec<char> = chars.iter().skip(start).take(width).copied().collect();
    if !show_cursor {
        return vec![Span::styled(visible.into_iter().collect::<String>(), style)];
    }
    let at = cursor - start;
    let before: String = visible.iter().take(at).collect();
    let under = visible.get(at).copied().unwrap_or(' ');
    let after: String = visible.iter().skip(at + 1).collect();
    vec![
        Span::styled(before, style),
        Span::styled(under.to_string(), style.add_modifier(Modifier::REVERSED)),
        Span::styled(after, style),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ColorConfig, LoginPosition};
    use crate::glyphs::DenyList;
    use crate::input::Event;
    use ratatui::buffer::Buffer;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::layout::Rect;

    fn key(c: KeyCode) -> Event {
        Event::Key(KeyEvent::new(c, KeyModifiers::NONE))
    }

    #[test]
    fn full_login_flow() {
        let mut p = LoginPanel::new(DenyList::new(&[]));
        for c in "star".chars() {
            p.handle_event(&key(KeyCode::Char(c)));
        }
        assert!(p.handle_event(&key(KeyCode::Enter)).is_none());
        p.handle_event(&Event::Paste("hunter2".into()));
        let s = p.handle_event(&key(KeyCode::Enter)).unwrap();
        assert_eq!(
            (s.username.as_str(), s.password.as_str()),
            ("star", "hunter2")
        );
        assert!(matches!(
            authenticate(&s.username, &s.password),
            Err(AuthError::NotImplemented)
        ));
    }

    #[test]
    fn long_status_is_not_truncated() {
        let mut p = LoginPanel::new(DenyList::new(&[]));
        p.set_status("login stub: authentication not implemented (user 'starq')");
        let area = Rect::new(0, 0, 40, 8);
        let mut b = Buffer::empty(area);
        p.render(area, &mut b, true, &ColorConfig::default());
        let text: String = b.content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("implemented"), "{text}");
    }

    #[test]
    #[allow(clippy::single_range_in_vec_init)]
    fn denied_border_chars_fall_back_to_ascii() {
        let deny = DenyList::new(&[0x2500..=0x257F]);
        let p = LoginPanel::new(deny.clone());
        let area = Rect::new(0, 0, 40, 8);
        let mut b = Buffer::empty(area);
        p.render(area, &mut b, true, &ColorConfig::default());
        assert_eq!(crate::ui::find_denied(&b, &deny), None);
        assert_eq!(b[(0, 0)].symbol(), "+");
    }

    #[test]
    fn denied_input_chars_are_dropped() {
        let deny = DenyList::new(&[]);
        let mut p = LoginPanel::new(deny.clone());
        p.handle_event(&Event::Paste("a\u{2588}b\u{E0A0}".into()));
        p.handle_event(&key(KeyCode::Char('\u{2580}')));
        let area = Rect::new(0, 0, 40, 8);
        let mut b = Buffer::empty(area);
        p.render(area, &mut b, true, &ColorConfig::default());
        assert_eq!(crate::ui::find_denied(&b, &deny), None);
        p.handle_event(&key(KeyCode::Enter));
        let s = p.handle_event(&key(KeyCode::Enter)).unwrap();
        assert_eq!(s.username, "ab");
    }

    #[test]
    fn panel_rect_fits_or_hides() {
        assert_eq!(
            panel_rect(Rect::new(0, 0, 80, 24), LoginPosition::Center),
            Some(Rect::new(20, 8, 40, 8))
        );
        assert!(
            panel_rect(Rect::new(0, 0, 30, 6), LoginPosition::Center)
                .unwrap()
                .width
                <= 30
        );
        assert_eq!(
            panel_rect(Rect::new(0, 0, 10, 3), LoginPosition::Center),
            None
        );
        assert_eq!(panel_rect(Rect::new(0, 0, 1, 1), LoginPosition::Top), None);
    }

    #[test]
    fn rendered_panel_has_no_denied_chars_and_masks() {
        let mut p = LoginPanel::new(DenyList::new(&[]));
        p.handle_event(&key(KeyCode::Down));
        p.handle_event(&Event::Paste("secret".into()));
        let area = Rect::new(0, 0, 40, 8);
        let mut b = Buffer::empty(area);
        p.render(area, &mut b, true, &ColorConfig::default());
        assert!(crate::ui::find_denied(&b, &DenyList::new(&[])).is_none());
        let text: String = b.content().iter().map(|c| c.symbol()).collect();
        assert!(!text.contains("secret") && text.contains("******"));
    }
}
