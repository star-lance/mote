//! A single-line text input with optional masking.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Default)]
pub struct TextInput {
    value: String,
    cursor: usize,
    masked: bool,
}

impl TextInput {
    pub fn new(masked: bool) -> Self {
        Self {
            masked,
            ..Default::default()
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    fn byte_index(&self, char_idx: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_idx)
            .map_or(self.value.len(), |(i, _)| i)
    }

    fn len(&self) -> usize {
        self.value.chars().count()
    }

    /// Inserts text at the cursor, dropping control characters.
    pub fn insert_str(&mut self, s: &str) {
        for c in s.chars().filter(|c| !c.is_control()) {
            let at = self.byte_index(self.cursor);
            self.value.insert(at, c);
            self.cursor += 1;
        }
    }

    /// Applies an editing key; returns `true` if Enter was pressed.
    pub fn handle_key(&mut self, k: &KeyEvent) -> bool {
        if k.modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return false;
        }
        match k.code {
            KeyCode::Char(c) => self.insert_str(c.encode_utf8(&mut [0; 4])),
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                let at = self.byte_index(self.cursor);
                self.value.remove(at);
            }
            KeyCode::Delete if self.cursor < self.len() => {
                let at = self.byte_index(self.cursor);
                self.value.remove(at);
            }
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(self.len()),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.len(),
            KeyCode::Enter => return true,
            _ => {}
        }
        false
    }

    /// Formats the field for display, with one `*` per character when masked.
    pub fn display(&self) -> String {
        if self.masked {
            "*".repeat(self.len())
        } else {
            self.value.clone()
        }
    }
}

// These tests were written by Claude Code to ensure code coverage.
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn k(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    #[test]
    fn multibyte_edit_is_safe() {
        let mut t = TextInput::new(false);
        t.insert_str("héllo→ü"); // 7 chars, cursor at 7
        for _ in 0..3 {
            t.handle_key(&k(KeyCode::Left)); // cursor at 4, before 'o'
        }
        t.handle_key(&k(KeyCode::Backspace)); // deletes the 'l' at index 3
        assert_eq!(t.value(), "hélo→ü");
    }

    #[test]
    fn control_chars_stripped_and_masking() {
        let mut t = TextInput::new(true);
        t.insert_str("ab\x1b[31mc\n");
        assert_eq!(t.value(), "ab[31mc");
        assert_eq!(t.display(), "*******");
    }

    #[test]
    fn cursor_bounds_hold() {
        let mut t = TextInput::new(false);
        t.handle_key(&k(KeyCode::Backspace));
        t.handle_key(&k(KeyCode::Left));
        t.handle_key(&k(KeyCode::Delete));
        t.handle_key(&k(KeyCode::Char('x')));
        t.handle_key(&k(KeyCode::Right));
        t.handle_key(&k(KeyCode::Right));
        assert_eq!(t.value(), "x");
    }

    #[test]
    fn enter_submits() {
        assert!(TextInput::new(false).handle_key(&k(KeyCode::Enter)));
    }
}
