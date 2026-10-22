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

## Configuration

`$XDG_CONFIG_HOME/mote/config.toml` (or `--config PATH`). Every key is
optional, and unknown keys produce a warning.

```toml
fps = 30

[colors]
fg = "#b0b0b0"        # scene color
bg = "reset"

[glyphs]
palette = "ascii95"   # or a literal string of characters, e.g. " .:-=+*#%@/\\|_"
deny = ["U+2500-U+257F"]  # extra forbidden characters (block elements are always forbidden)

[render]
line_thickness = 1.5  # in sub-cell samples

[scene]
follow_mouse = false  # true: plain mouse movement orbits the camera, not just dragging

[scene.camera]
position = [0, 0, 6]
fov_deg = 50

[[scene.objects]]
mesh = { type = "icosahedron" }
position = [-3, 1.5, 0]
spin = [0, 40, 0]     # degrees per second

[[scene.objects]]
mesh = { type = "torus", segments = 24, sides = 12 }
position = [3, -1.5, 0]
rotation = [60, 0, 0]
spin = [10, 25, 0]

[ui.login]
enabled = true
position = "center"   # center | top | bottom
```

Meshes: `tetrahedron`, `cube`, `octahedron`, `icosahedron`, `dodecahedron`,
`torus` (`major`, `minor`, `segments`, `sides`), `sphere` (`segments`, `rings`).

## Using another font (glyphgen)

The atlas holds each palette glyph's shape at the sub-cell sampling resolution
for one font at one size. To build one for another font:

```sh
cargo run --release --bin glyphgen -- --font /path/to/Font.ttf --size-px 16 --out my-atlas.json
```

Then set `glyphs.atlas = "my-atlas.json"` (and optionally `glyphs.font` so mote
can warn if the atlas came from a different file). On startup mote compares the
terminal's real cell size with the atlas and prints the `glyphgen --cell WxH`
to use if they differ. Run `glyphgen --help` for palette, denylist and sampling
options.

## Font

Bundled font: Ioskeley Mono (Term, Nerd Font patched) by Ahmed Hatem, based on
Iosevka, licensed under the SIL Open Font License 1.1. See
`assets/fonts/OFL.txt`.
