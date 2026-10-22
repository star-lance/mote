//! Widgets drawn into the ratatui frame.

use ratatui::buffer::Buffer;

use crate::glyphs::DenyList;

pub mod login;
pub mod scene_view;
pub mod text_input;

/// Searches a frame for denied characters.
///
/// # Arguments
/// * `buf` - the rendered frame
/// * `deny` - characters that may not be output
///
/// # Returns
/// The position and character of the first match, or `None`.
pub fn find_denied(buf: &Buffer, deny: &DenyList) -> Option<(u16, u16, char)> {
    let area = buf.area;
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(c) = buf[(x, y)].symbol().chars().find(|&c| deny.is_denied(c)) {
                return Some((x, y, c));
            }
        }
    }
    None
}

// These tests were written by Claude Code to ensure code coverage.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyphs::DenyList;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn find_denied_detects_block_char() {
        let mut b = Buffer::empty(Rect::new(0, 0, 4, 2));
        assert!(find_denied(&b, &DenyList::new(&[])).is_none());
        b[(2, 1)].set_symbol("█");
        assert_eq!(find_denied(&b, &DenyList::new(&[])), Some((2, 1, '█')));
    }
}
