//! Draws a [`CellGrid`] into a ratatui buffer.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::matcher::CellGrid;

pub struct SceneView<'a> {
    pub grid: &'a CellGrid,
    pub style: Style,
}

impl Widget for SceneView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cols = (area.width as usize).min(self.grid.cols);
        let rows = (area.height as usize).min(self.grid.rows);
        for row in 0..rows {
            for col in 0..cols {
                buf[(area.x + col as u16, area.y + row as u16)]
                    .set_char(self.grid.get(col, row))
                    .set_style(self.style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::CellGrid;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Style;
    use ratatui::widgets::Widget;

    #[test]
    fn scene_view_writes_grid_and_clips() {
        let mut g = CellGrid::new(3, 1);
        g.cells = vec!['a', 'b', 'c'];
        let mut b = Buffer::empty(Rect::new(0, 0, 2, 1));
        SceneView {
            grid: &g,
            style: Style::default(),
        }
        .render(b.area, &mut b);
        assert_eq!(b[(0, 0)].symbol(), "a");
        assert_eq!(b[(1, 0)].symbol(), "b");
    }
}
