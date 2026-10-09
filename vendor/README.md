# Vendored crates

## macroquad 0.4.16

The library part of [Macroquad](https://github.com/not-fl3/macroquad) 0.4.16
as published on crates.io (`src/`, `js/`, its README and its MIT and Apache-2.0
licenses), used through `[patch.crates-io]` in the top-level `Cargo.toml`.
Examples, tests, and the dev-profile table were left out of `Cargo.toml`, and
the crate's own compiler warnings are allowed.

One change, in `src/text/atlas.rs`: a glyph is placed at `cursor_x + GAP`, but
the check for room on the current row tested `cursor_x + width < atlas width`.
A glyph ending one pixel short of the edge therefore failed the bounds check
that follows and doubled the atlas in both directions, re-packing everything,
instead of starting a new row. Caching the interface's text could repeat that
until the atlas reached 8192 × 8192: about 270 MB in the game's memory and as
much again on the GPU, from text that fits in 1024 × 1024. That showed up at a
device pixel ratio of 1 (desktop windows, and 517 MB of WebAssembly memory in
the browser build) and with less luck on phones. The fixed check asks whether
the glyph fits where it will actually be drawn.

0.4.16 is the newest release at the time of writing (October 2026). To return
to the published crate, delete this folder and the `[patch.crates-io]` table.
