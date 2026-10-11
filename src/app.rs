//! The main loop.

use std::time::{Duration, Instant};

use anyhow::Context;
use ratatui::crossterm::event;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Paragraph, Widget};

use crate::atlas::Atlas;
use crate::config::{Config, MatcherMode, ValidatedGlyphs};
use crate::input::{Event, Focus, Route, from_crossterm, route};
use crate::render::Renderer;
use crate::scene::{CameraControls, Scene};
use crate::term::{self, Term};
use crate::ui::find_denied;
use crate::ui::scene_view::SceneView;

/// Cell width / height: measured if the terminal reports pixels, else the atlas's.
pub fn effective_cell_aspect(measured: Option<[u16; 2]>, atlas: &Atlas) -> f32 {
    match measured {
        Some([w, h]) if w > 0 && h > 0 => w as f32 / h as f32,
        _ => atlas.cell_aspect(),
    }
}

pub fn run(cfg: Config, glyphs: ValidatedGlyphs, atlas: Atlas) -> anyhow::Result<()> {
    let mut terminal = term::setup().context("setting up terminal")?;
    let result = App::new(cfg, glyphs, atlas).run(&mut terminal);
    let restored = term::restore().context("restoring terminal");
    result.and(restored)
}

struct App {
    cfg: Config,
    glyphs: ValidatedGlyphs,
    atlas: Atlas,
    scene: Scene,
    controls: CameraControls,
    renderer: Renderer,
    focus: Focus,
    show_debug: bool,
    cell_px: Option<[u16; 2]>,
    fps: f32,
    render_ms: f32,
}

impl App {
    fn new(cfg: Config, glyphs: ValidatedGlyphs, atlas: Atlas) -> Self {
        Self {
            scene: Scene::from_config(&cfg.scene, cfg.render.feature_edge_angle_deg),
            controls: CameraControls::default(),
            renderer: Renderer::new(&cfg, &atlas, &glyphs.palette),
            focus: Focus::Scene,
            show_debug: false,
            cell_px: term::cell_pixel_size(),
            fps: 0.0,
            render_ms: 0.0,
            cfg,
            glyphs,
            atlas,
        }
    }

    fn run(&mut self, terminal: &mut Term) -> anyhow::Result<()> {
        let period = Duration::from_secs_f64(1.0 / self.cfg.fps as f64);
        let mut next_frame = Instant::now();
        let mut last_update = Instant::now();
        loop {
            // Handle input until it is time for the next frame.
            while event::poll(next_frame.saturating_duration_since(Instant::now()))? {
                let Some(ev) = from_crossterm(event::read()?) else {
                    continue;
                };
                if !self.handle_event(&ev) {
                    return Ok(());
                }
            }

            let now = Instant::now();
            let dt = (now - last_update).as_secs_f32();
            last_update = now;
            next_frame = (next_frame + period).max(now);
            if dt > 0.0 {
                self.fps = if self.fps == 0.0 {
                    1.0 / dt
                } else {
                    self.fps * 0.9 + 0.1 / dt
                };
            }

            self.scene.update(dt);
            self.draw(terminal)?;
        }
    }

    /// Returns false when the app should quit.
    fn handle_event(&mut self, ev: &Event) -> bool {
        if let Event::Resize { .. } = ev {
            self.cell_px = term::cell_pixel_size();
        }
        match route(ev, self.focus, &self.cfg.keys, None) {
            Route::Quit => return false,
            Route::ToggleMatcher => {
                self.renderer.mode = match self.renderer.mode {
                    MatcherMode::Match => MatcherMode::Ramp,
                    MatcherMode::Ramp => MatcherMode::Match,
                }
            }
            Route::ToggleDebug => self.show_debug = !self.show_debug,
            Route::FocusScene => self.focus = Focus::Scene,
            Route::FocusLogin | Route::Login => {}
            Route::Scene => self.controls.handle_event(ev, &mut self.scene.camera),
        }
        true
    }

    fn draw(&mut self, terminal: &mut Term) -> anyhow::Result<()> {
        let size = terminal.size()?;
        let screen = Rect::new(0, 0, size.width, size.height);
        let colors = &self.cfg.colors;
        let scene_style = Style::default().fg(colors.fg).bg(colors.bg);

        let debug_text = self.show_debug.then(|| {
            let cell = self
                .cell_px
                .map_or("unknown".to_string(), |[w, h]| format!("{w}x{h}px"));
            format!(
                "fps {:.1}  render {:.1}ms\ngrid {}x{}  cell {}  mode {:?}",
                self.fps, self.render_ms, size.width, size.height, cell, self.renderer.mode
            )
        });

        let start = Instant::now();
        let cell_aspect = effective_cell_aspect(self.cell_px, &self.atlas);
        let grid = self
            .renderer
            .render(&self.scene, size.width, size.height, cell_aspect);
        self.render_ms = self.render_ms * 0.9 + start.elapsed().as_secs_f32() * 100.0;

        let frame = terminal.draw(|f| {
            f.render_widget(
                SceneView {
                    grid,
                    style: scene_style,
                },
                screen,
            );
            if let Some(text) = debug_text {
                let style = Style::default().fg(colors.ui_fg).bg(colors.bg);
                let area = Rect {
                    width: screen.width.min(60),
                    height: screen.height.min(2),
                    ..screen
                };
                Paragraph::new(text)
                    .style(style)
                    .render(area, f.buffer_mut());
            }
        })?;
        if cfg!(debug_assertions)
            && let Some(found) = find_denied(frame.buffer, &self.glyphs.deny)
        {
            panic!("denied character in output: {found:?}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_pixel_size_falls_back_to_atlas() {
        let a = Atlas::embedded_default();
        assert_eq!(effective_cell_aspect(None, &a), a.cell_aspect());
        assert_eq!(effective_cell_aspect(Some([10, 20]), &a), 0.5);
    }
}
