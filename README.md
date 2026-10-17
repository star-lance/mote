# mote

A terminal renderer that draws animated 3D wireframes entirely out of text
characters, with interactive UI (currently a stand-in login panel) on the same
screen. Each cell's character is chosen by comparing the rendered image against
the actual glyph shapes of the font, so the whole picture reads correctly. It
does not use per-cell rules, and never uses block characters.

## Build and run

```sh
cargo build --release
scripts/mote-launch          # dedicated kitty window with the bundled font
```

`scripts/mote-launch` starts kitty with the bundled Ioskeley Mono font (no
installation needed). The font size is pinned, so the glyph atlas matches the
screen exactly. `target/release/mote` also runs in any terminal; matching is
less exact there if the font differs.

Kiosk / greeter example (Wayland):

```sh
cage -- scripts/mote-launch
```

## Keys

| Key | Action |
|---|---|
| Ctrl+C | quit (always) |
| q | quit (when the scene has focus) |
| Esc | move focus from the login panel to the scene |
| Tab / click panel | move focus back to the login panel |
| F2 | toggle matcher: glyph matching ↔ brightness ramp |
| F3 | toggle debug overlay (fps, timings, cell size) |
| mouse drag / scroll | orbit / zoom the camera |

Keys are configurable under `[keys]`.

## Font

Bundled font: Ioskeley Mono (Term, Nerd Font patched) by Ahmed Hatem, based on
Iosevka, licensed under the SIL Open Font License 1.1. See
`assets/fonts/OFL.txt`.
