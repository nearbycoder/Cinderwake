# Cinderwake — improvement plan

[← Back to the game](../README.md) · [Engine guide](ENGINE.md)

This plan comes from a review on October 6, 2026, made on CachyOS Linux (Wayland, 1.25× fractional display scaling, AMD Radeon 8060S) with Rust 1.96.1. It is based on the code, the automated checks, and the game's own capture modes. Nobody played a full run by hand for this review, so balance observations come from reading the code, not from playtesting.

## Baseline on Linux

| Check | Result |
| --- | --- |
| `cargo fmt --check` | Pass |
| `cargo clippy --locked --all-targets -- -D warnings` | Pass |
| `cargo test --locked` | Pass: 61 tests |
| `cargo build --release --locked` | Pass; the window opens through XWayland |
| `--ui-gallery` | Runs, but **every screen shows the HUD and menus misplaced** (see item 1) |
| `--motion-capture --profile-render` | 300 frames; CPU simulation and draw submission average 2.44 ms, p95 6.62 ms |
| `--vertical-capture` | 37 of 37 waypoints reached in 25.2 s; `finished=true` |
| `--gallery` | Four biome images; the whole process takes 1.6 s from launch to exit |
| `wasm32-unknown-unknown` release build | Compiles with `-C link-arg=--allow-undefined` (newer `rust-lld` rejects the JS imports otherwise). The output is 42 MB, because all art is embedded. Not yet run in a browser |

The world rendering, animation, particles, bloom, and route traversal all look correct in the Linux captures. The game itself works on Linux; the problems are in presentation and platform plumbing.

## Ranked improvements

Impact means how much the change helps a real player. Effort: **S** is under half a day, **M** is about a day, **L** is several days. Risk covers regressions and anything that can't be verified on this machine.

| # | Improvement | Impact | Effort | Risk |
| --- | --- | --- | --- | --- |
| 1 | **Fix the HUD and menus on high-DPI and scaled displays** | Critical | S | Low |
| 2 | **Per-platform save location** (Linux and Windows) | High | S | Low |
| 3 | **Playable browser build** | High | M | Medium |
| 4 | **Settings and accessibility panel** | High | M | Low |
| 5 | **Contextual first-run onboarding** | High | M | Low |
| 6 | **Difficulty curve across stages** | Medium-high | M | Medium |
| 7 | Linux in CI, plus Linux packaging | Medium | S | Low |
| 8 | Chest choice instead of a forced weapon swap | Medium | S | Low |
| 9 | Gamepad support | Medium-high | M | High |
| 10 | Control rebinding | Medium | M | Medium |
| 11 | Abandon run and quit from the menus | Low-medium | S | Low |
| 12 | Atlas fog of war | Low-medium | S | Low |
| 13 | Audio depth: biome music, more effects, separate buses | Medium | M | Low |
| 14 | Content: more weapons with their own art, skills, a second boss | High | L | High |
| 15 | Smaller web download (recompress or downscale embedded art) | Medium (web only) | M | Medium |

### Findings behind the ranking

1. **HUD and menus on scaled displays.** `src/main.rs` builds the HUD camera with `viewport: Some((x, y, w, h))` using logical sizes from `screen_width()` and `screen_height()`. Macroquad expects viewports in physical framebuffer pixels. At a 1.25 scale, the whole interface is drawn into the bottom-left 1024 × 576 pixels: the top HUD row starts at y = 144, the right-hand panels end near x = 1010, and menu dimming leaves a lit strip on the right. This shows on every `--ui-gallery` frame here. It will also affect Windows at 125% or 150% scaling and browsers on high-DPI screens. Scaling the viewport by `screen_dpi_scale()` should fix it. I couldn't check macOS Retina behaviour on this machine.
2. **Save location.** `Save::path()` always uses `$HOME/Library/Application Support/Cinderwake/progress.json`, so on Linux it creates a `~/Library` folder in the home directory. It should use `$XDG_DATA_HOME` (falling back to `~/.local/share`) on Linux and `%APPDATA%` on Windows. The macOS path stays the same, so existing saves are unaffected. ENGINE.md already notes that the path is macOS-shaped everywhere.
3. **Browser build.** Macroquad targets WebGL, and the shaders are already `#version 100`. Known blockers: the `std::time::Instant::now()` call at the top of every loop iteration panics on `wasm32-unknown-unknown`; saving through `std::fs` fails in the browser; the repo has no HTML page or build script; and the linker needs `--allow-undefined`. Browsers also block audio until the player interacts with the page. The 42 MB download is large, but a downloadable game can live with it. This is the biggest gain in how many people can play, since there is currently no binary to download.
4. **Settings.** The only options are mute (M) and post-processing (F9). Screen shake (up to 8 units per hit) and full-screen flashes have no reduction setting, and music and effects volumes are fixed in code at 0.5 and 0.35. None of these choices are saved.
5. **Onboarding.** The title screen teaches three controls (move, jump, strike). Dodge, parry, slam, drop-through, flasks, and tools are listed only on the pause screen and in the HUD key badges. Parry reflects shots and drop-through matters for the route, but nothing in the game introduces either.
6. **Difficulty curve.** `Level::generate(seed, biome, save.wins)` scales enemy health only by lifetime wins (+12% per win). It ignores stage, and enemy damage never scales. Meanwhile the player gains weapon tiers, memories, and Keeper upgrades. Later stages therefore get easier relative to the player's power, and the Crown has only the Regent and two guards. I'll check this with a headless simulation, not by feel.
7. **CI and Linux packaging.** CI runs only on `macos-latest`. A matrix entry for `ubuntu-latest` (with the ALSA, X11, and GL development packages) would keep Linux building. A `scripts/package-linux.sh` that produces a tarball would be the Linux counterpart to the macOS app script.
8. **Chests.** Opening a chest replaces your weapon with a random one, so you can lose a hammer you liked. Showing the offered weapon and letting the player take or leave it (leaving it for copper) gives them a real choice.
9. **Gamepad.** Macroquad 0.4 has no gamepad API. The native option is `gilrs` (on Linux it needs libudev, which this machine has). No controller is attached here, so I could only test the input mapping in code, not on real hardware. The browser build would need separate work.
10–15. Smaller or deferred items. Item 14 would need new generated art; the repo's convention is to record prompts in `PROMPTS.md`, and that work is outside this round.

## Proposed scope for this round

Six items, ordered so the platform fixes land first.

### A. HUD and menus correct at any display scale (item 1)

- **Acceptance:** at 1.25× scaling, every `--ui-gallery` frame matches the layout of `docs/media/*.png`: top HUD row at y ≈ 16, panels reaching the right edge, menu dimming covering the whole frame. A resized window and fullscreen (F11) both letterbox correctly. Nothing changes at 1.0× scaling.
- **Verify:** rerun `--ui-gallery` and inspect the images, run the game in a resized window, and add a unit test for the viewport calculation at scales 1.0, 1.25, and 2.0.

### B. Per-platform save location (item 2)

- **Acceptance:** Linux saves to `$XDG_DATA_HOME/cinderwake/progress.json` (or `~/.local/share/...`), Windows to `%APPDATA%\Cinderwake\`, and macOS keeps its current path. No `~/Library` folder appears on Linux. README and ENGINE.md list all three paths.
- **Verify:** unit tests for path selection with injected environment variables; a real run here confirms the file appears in the expected place and survives a restart. Run with a temporary `XDG_DATA_HOME` so the real home directory isn't touched.

### C. Playable browser build (item 3)

- **Acceptance:** `scripts/build-web.sh` produces `dist/web/` containing `index.html`, the vendored `mq_js_bundle.js` (with its license note), and `cinderwake.wasm`. Served locally, the game reaches the title screen, starts a run, and plays with keyboard input. Progress saves to `localStorage`, or, if that turns out to be too complex, the game says clearly that progress isn't saved in the browser. Timing never panics. The README presents web support as experimental and says what was tested.
- **Verify:** serve with `python3 -m http.server` and load in Chrome/Firefox or the T3 preview pane; take screenshots of the title screen and gameplay, check the console for errors, and confirm a reload keeps progress. I won't publish anything; deploying it (for example to GitHub Pages) is the owner's call.

### D. Settings and accessibility panel (item 4, plus item 11's abandon run)

- **Acceptance:** an Options page reachable from pause and from the title screen, keyboard-driven, with: music volume, effects volume, screen-shake intensity (0–100%), reduced flashes (dims full-screen flash and shockwave particles), hit-stop on/off, and post-processing on/off. Settings are saved in their own file and never affect gameplay randomness. The pause screen also offers "Abandon run", which counts as a death with the usual penalties.
- **Verify:** unit tests that shake and flash scaling are applied and that settings round-trip through save/load; a new `--ui-gallery` fixture for the Options page; checking captures with shake at 0.

### E. Contextual first-run onboarding (item 5)

- **Acceptance:** on early runs, one-time hint banners appear when a mechanic first becomes relevant: first enemy nearby (strike, dodge), first incoming shot (parry reflects), first ledge above (double jump), first ledge drop (S + Space), first damage taken (F to drink a flask), and first chest, memory, well, or forge (E). Each hint is marked as seen and saved, can be turned off in Options, and never pauses play. The title screen's control line lists the core verbs.
- **Verify:** unit tests drive `Game::tick` into each trigger and check that each hint fires exactly once and stays dismissed after save/load; a capture frame shows a hint in play.

### F. Difficulty that rises through the run (item 6)

- **Acceptance:** enemy health and damage scale with stage as well as wins. The Crown gets a small pre-boss encounter. A headless test plays standard player builds against each stage's enemy health budget and confirms that effective difficulty (enemy health relative to player damage) does not drop between stage 0 and the Crown. Existing route and traversal tests still pass.
- **Verify:** a new balance regression test, `--vertical-capture` still finishing, and a short scripted `--demo` run checked for anything obviously broken. The numbers will be first-pass tuning, not production balance, and the README will keep saying so.

## Decisions for the owner

- **Gamepad (item 9):** worth adding `gilrs` with only software testing, or should it wait until it can be tested on real hardware?
- **Web hosting:** the browser build will be produced and tested locally only. Whether and where to host it is the owner's call.
- **Web download size (item 15):** 42 MB is acceptable for now. Shrinking it means storing smaller runtime copies of the art, which departs from the "source outputs remain unchanged" convention, so it needs the owner's approval.
