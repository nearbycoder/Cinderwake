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

## Round 1 results (October 6, 2026)

All six scoped items shipped on `improvements`, one commit each. Every commit passed `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, and `cargo test --locked` (61 tests became 77). `--vertical-capture` still reaches 37 of 37 waypoints. Screenshots are in [`media/improvements/`](media/improvements/).

| Item | Commit | How it was verified |
| --- | --- | --- |
| A. HUD at any display scale | `dc9cc56` | `--ui-gallery` at 1.25× before and after (`a-hud-*.jpg`); a temporary 1500 × 700 window pillarboxed correctly; unit test of the viewport at 1×, 1.25×, and 2×. Fullscreen (F11) and macOS Retina were not checked by hand. |
| B. Per-platform save location | `0ae1bd8` | Path-selection unit tests for Linux, macOS, and Windows; a round-trip test in an isolated temporary home; the release binary, launched with a temporary `XDG_DATA_HOME`, read `cinderwake/progress.json` there and created no `~/Library` (observed with inotify). Windows was not run. |
| C. Browser build | `8e547da` | Headless Chrome 154 at pixel ratios 1 and 1.25: loads with no console errors, starts a run, moves, jumps, opens the atlas and pause screen, and keeps progress in `localStorage` across a reload (`c-web-*.jpg`). Frame rate, audio, and other browsers were not checked. Nothing was deployed. |
| D. Options and abandon run | `6bc0bb0` | Settings unit tests (defaults, clamping, tolerant loading); game tests for hit-stop, abandon confirmation, options round trip, and shake scaling; `--ui-gallery` fixtures; real key input in the browser changed shake and hit-stop, the values survived a reload, and X X abandoned the run (`d-*.jpg`). Audio levels were not listened to. |
| E. First-run tips | `209a004` | Tests that each tip fires once, follows the priority order, stays dismissed after the settings round trip, and stays silent in practice or when disabled; a fresh browser run showed the climb, strike, and parry tips during 14 s of real play (`e-*.jpg`). |
| F. Stage difficulty | `47ed498` | Duel-simulation regression test with numbers recorded in [ENGINE.md](ENGINE.md#build-and-verification); a test that stage travel applies the scaling; a temporary fixture capture of the new Crown gallery guards (`f-crown-gallery-guards.jpg`). No human playtest. |

Deferred to a later round: Linux in CI and Linux packaging (item 7), the chest choice (8), rebinding (10), fog of war (12), audio depth (13), content (14), and a smaller web download (15). Gamepad (9) and web hosting are waiting on owner decisions.

## Round 2 scope

Five items, ordered so the riskiest one (rebinding) lands last. Each item gets the round-1 checks: `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, `cargo test --locked`, the capture modes, and screenshots in [`media/improvements/round2/`](media/improvements/round2/).

### A. Telegraphed attacks that commit (round-1 finding)

Round 1's duel simulation showed that holding attack keeps a single warden or brute permanently staggered, so it never completes an attack. Each hit adds 0.22 s of stun, which pauses the windup.

- **Acceptance:** once a guardian has started its windup, ordinary strikes no longer stagger it. Combo finishers, ground slams, fire vessels, arc snares, and parries still do. Brutes stagger only from those heavy sources, even outside a windup. Moths and the Regent keep their current behavior. In the duel simulation, holding attack against a brute costs vitality at every stage, while a build that dodges as the windup starts takes less damage. The round-1 difficulty test still passes, re-tuned if needed.
- **Verify:** new simulation tests (held attack vs dodge-timed); `--motion-capture` still exercises every action; the motion capture is inspected.

### B. Choose at reliquaries (item 8)

- **Acceptance:** opening a chest with a different weapon on offer pauses on a choice: **1** takes the new weapon, **2** keeps the current one. The tier increase applies either way, and damage, reach, and swing time are shown for both. If the offer is the weapon already held, it upgrades directly, as before. The UI and docs describe this accurately.
- **Verify:** game tests for both choices and for the same-weapon case; a new `--ui-gallery` fixture; real key input in the browser build.

### C. Atlas fog of war (item 12)

- **Acceptance:** the atlas and HUD minimap show only the parts of the level the camera has seen. The bellgate is always marked, so the destination is never hidden. Exploration resets with each biome. Practice and staged captures keep the full survey so existing captures don't change.
- **Verify:** unit tests for revealing cells and for what the atlas hides; a `--ui-gallery` fixture with partial exploration; the browser build shows the atlas filling in as the player moves.

### D. Linux in CI, browser build check, Linux package (item 7)

- **Acceptance:** CI runs format, lint, and tests on Ubuntu as well as macOS, and checks that the wasm build compiles. `scripts/package-linux.sh` produces `dist/cinderwake-linux-x86_64.tar.gz` containing the binary, licenses, and a short README.
- **Verify:** run the package script here, extract the tarball inside `target/`, and run `--ui-gallery` from the extracted binary. The workflow can't run here; its commands are run locally where possible and listed as unverified until CI runs on GitHub.

### E. Key rebinding (item 10)

- **Acceptance:** a Controls page reached from Options lets the player rebind the twelve gameplay actions (move left and right, jump, down, strike, glassbolt, dodge, parry, fire vessel, arc snare, heal, interact) to any supported key. Binding a key that's already in use swaps the two bindings. There's a reset to defaults. Bindings are saved with the settings. The HUD key badges, pause list, title line, and tips show the current keys. Arrow keys and mouse buttons remain fixed alternatives, and menu keys stay fixed.
- **Verify:** unit tests for binding, swapping, persistence, and labels; `--ui-gallery` fixtures for the Controls page and a HUD with rebound keys; real key input in the browser build rebinds an action and uses it in play.

Owner decisions still open, and not part of this round: gamepad support, web hosting, shrinking the art, and licenses.

## Round 2 results (October 6, 2026)

All five scoped items shipped on `improvements-2`, one commit each. Every commit passed `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, and `cargo test --locked` (77 tests became 86). `--vertical-capture` still reaches 37 of 37 waypoints, and `--motion-capture` still produces all 300 frames. Screenshots are in [`media/improvements/round2/`](media/improvements/round2/).

| Item | Commit | How it was verified |
| --- | --- | --- |
| A. Guardian poise | `c11adbc` | The planned rule (armor only during windups) wasn't enough. Tracing a duel showed that every light hit's 0.22 s stun and 8-unit shove kept wardens outside their 36-unit attack range. The shipped rule: light hits don't stagger or shove guardians; heavy hits do. `held_attack_no_longer_stun_locks_guardians` makes a guardian unkillable for 5 s: held attack now takes 2–3 strikes from each warden, brute, and archer, where wardens and brutes landed 0 before (1 against the hammer); dodging each telegraph takes 0 warden or brute strikes. The duel test now starts enemies ready to attack, which made the Regent harsher, so its damage scaling dropped from 10% to 5% per stage (stand-still Crown duel: 7–9 s, 62–78% vitality lost). No screenshot: the steel damage number is too small to show clearly in a capture. |
| B. Reliquary choice | `e98e1e0` | A test opens the same seeded chest with each weapon and checks the direct upgrade, both choices, the tier, and that the world stays frozen. Also the `ui-14-reliquary` fixture (`b-reliquary-choice.jpg`). **Not verified with real key input:** three attempts to pilot the browser build to a chest ended in the undercroft, so the **1**/**2** keys are covered only by the shared menu code and unit tests. |
| C. Atlas fog of war | `9c9c5df` | `Survey` unit tests; a game test that exploring reveals the map and travel resets it; the `ui-15-atlas-fog` fixture; in the browser build, the atlas started with only the spawn view and grew as the hero moved (`c-*.jpg`). |
| D. Linux CI and package | `e59b42b` | `scripts/package-linux.sh` built the tarball; the extracted binary ran `--ui-gallery` (16 captures at the time); the CI web job's clippy and debug build commands passed locally. **The new GitHub jobs themselves have not run yet.** |
| E. Key rebinding | `640b0ab` | Unit tests for defaults, swaps, reserved keys, saved-file repair, and the controls-page flow; `ui-16-controls` and `ui-17-rebound-hud` fixtures; in the browser build with real key events, rebinding Jump to K through the menus made K jump and W stop jumping, Space moved to the glassbolt and fired it, and the binding persisted into a new session (`e-*.jpg`). |

Findings for a later round:

- Guardians usually die in under a second against the expected build at every stage (the opening sabre kills a warden in two hits), so single guardians are rarely threatening. Raising guardian health is a balance decision worth a human playtest.
- Scripting a route through a level from outside the game is unreliable. A debug flag that starts at a chosen object would make end-to-end checks of chests and other interactions practical.

Still deferred: audio depth (13), new content (14), and a smaller web download (15). Gamepad support, web hosting, shrinking the art, and licenses remain owner decisions.

## Round 3 scope

Four items, plus one stretch item. The launch flag comes first because the later items and the round-2 reliquary check depend on it. The riskiest item, continuing a run, comes last. Each item gets the usual checks: `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, `cargo test --locked`, the capture modes, and screenshots in [`media/improvements/round3/`](media/improvements/round3/). Every native run uses a temporary `XDG_DATA_HOME`, and the real save folder is checked before and after.

A review of the audio found a defect that moves item 13 up the list. The only music track is a 16-second loop that fades out over its last half second and back in over its first, so the score drops to silence every 16 seconds. It's also the only track, so every biome, the title, and the Keeper sound the same, and kills, glassbolts, thrown tools, banking, and menu choices make no sound at all.

### A. Start next to a chosen object (round-2 finding)

- **Acceptance:** `--start-at <target>` on the desktop and `?start=<target>` in the browser build start a run beside a chosen object: `chest`, `memory`, `well`, `forge`, `cache`, `gate`, `keeper` (the Keeper screen after the first stage), or `regent` (the Crown, just before the arena). It uses the practice seed and isolation: no progress or settings are read or written, and no tips are shown. Guardians close to the start point are removed, so the object can be used straight away. An unknown target prints the valid list and exits on the desktop; in the browser it's ignored. ENGINE.md documents it as a testing tool.
- **Verify:** a unit test for every target checks that the object is within reach on a real support, or that the right screen is open. In the browser build, real key presses at `?start=chest` open the reliquary, and **1** and **2** each work. That closes the check round 2 couldn't do. A native run confirms nothing is written to the data directory.

### B. A score for each biome, with seamless loops (item 13)

- **Acceptance:** `scripts/synthesize.py` renders five original loops: a hearth theme (title, Keeper, death, and victory), the Aqueduct, the Conservatory, the Foundry, and the Crown. Each loop wraps without a fade, so the last notes ring into the start. The game crossfades over about 1.5 s when the scene changes, and the music volume and mute options still apply. The script checks every loop: no clipping, a small jump at the seam, and loudness near the seam within 2 dB of the whole track. The README and ENGINE.md give the download-size cost.
- **Verify:** unit tests for which track plays on each screen and biome and for the crossfade envelope; the script's own loop checks; spectrogram images of the old and new seams; the browser build loads and starts audio after a key press with no console errors. **Nobody will listen to it on this machine**, so how good it sounds is for the owner to judge.

### C. Sound for silent actions (item 13)

- **Acceptance:** new synthesized effects for a guardian's death, glassbolt fire, throwing a fire vessel or arc snare, banking at a bellgate or buying from the Keeper, menu choices (memory, reliquary, Keeper route, options changes), and a heavier sound for the combo finisher. The volume and mute options apply to all of them. Each effect has a matching embedded file, and the build fails if they don't line up.
- **Verify:** game tests showing that each event queues its cue exactly once, a count check between cues and files, the script's clipping check, and spectrograms. As with B, nobody will listen to them here.

### D. Continue a run after quitting

At the moment, closing the game or browser tab partway through a run loses the whole run. That costs the most in the browser build, where a tab is easy to close.

- **Acceptance:** arriving in a new biome or reaching the Keeper saves a checkpoint (`run.json`, or `cinderwake/run.json` in browser storage) with the run's seed, stage, biome, weapon, tier, memories, copper, mutation, vitality, kills, and time. If a checkpoint exists, the title screen offers **Enter** to continue (naming the biome) and **N** to start a new run. Continuing restores the run at the start of that biome, or at the Keeper. Embers carried since the last bellgate are lost, as on death, and continuing doesn't count as a new descent. Death, victory, abandoning, and starting a new run all delete the checkpoint. Practice and capture modes never touch it. An unreadable checkpoint is ignored.
- **Verify:** unit tests for the round trip (travel, checkpoint, restore, same level and build), deletion on death, victory, abandon, and new run, and tolerance of a corrupt file; a `--ui-gallery` fixture for the title with a run to continue; in the browser build, real keys reach the Keeper (using `?start=gate`), then a reload offers to continue and continuing restores it.

### E. Stretch: remember fullscreen

- **Acceptance:** a **Fullscreen** row on the options page, kept in sync with F11 and saved; the desktop game starts fullscreen if it was saved that way.
- **Verify:** settings tests and the options gallery fixture; a native launch with fullscreen saved. This item ships only if A–D are done.

Owner decisions still open, and not part of this round: gamepad support, web hosting, shrinking the art, licenses, and guardian health.

## Round 3 results (October 6, 2026)

All four scoped items and the stretch item shipped on `improvements-3`, one commit each. Every commit passed `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, and `cargo test --locked` (86 tests became 97). A guard confirmed that the test suite left its temporary data folder empty. `--vertical-capture` still reaches 37 of 37 waypoints, `--motion-capture` still produces 300 frames (CPU submission mean 1.11 ms, p95 1.62 ms), and `--ui-gallery` now has 19 fixtures. The existing 18 matched their earlier captures within the gallery's normal run-to-run variation (about 50 dB PSNR). Every native run used a temporary `XDG_DATA_HOME`, and `~/.local/share/cinderwake` didn't exist before or after the round. Screenshots and audio plots are in [`media/improvements/round3/`](media/improvements/round3/).

| Item | Commit | How it was verified |
| --- | --- | --- |
| A. `--start-at` | `50f0bbe` | Unit tests for argument parsing, every target's position or screen, and using each object straight away. Natively, an unknown target exits with status 2 and a 6-second run wrote nothing. In the browser build with real key presses, `?start=chest` opened the reliquary, **1** took the Furnace Maul and **2** kept the sabre (`a-web-*.jpg`). That closes the check round 2 couldn't do. |
| B. Biome music | `2367972` | The synthesis script's loop checks: all five loops pass, with seam loudness within ±1.6 dB, and the old track fails at −10.5 dB (`b-loop-seams-old-vs-new.jpg`, `b-music-spectrograms.jpg`). Unit tests cover the track for every screen and biome and an equal-power, 1.5-second crossfade. In the browser, logging the audio sources showed the hearth loop on the title and the Aqueduct loop starting with the run. **Nobody listened**, so the music's quality is unjudged. The loops add 3.7 MB, which takes the wasm from about 49 MB to 52.5 MB. |
| C. New effects | `1c3cd66` | A game test drives each event (glassbolt, both tools, the combo finisher, wound versus kill, forge and cache refusals, banking, Keeper purchase and refusal, memory and reliquary choices) and checks it queues its cue exactly once. A test checks that every cue's file loads in order. Spectrograms helped remove clicks from `deny`, `select`, and `bank` (`c-new-effect-spectrograms.jpg`). In the browser, real keys triggered the glassbolt, throw, bank, select, and deny sounds. **Not listened to.** |
| D. Continue a run | `9d447cd` | Unit tests for the round trip (same level, build, and time; carried embers lost; no new descent), Keeper checkpoints, every deletion path, practice isolation, and damaged or impossible files. The test build now stores files in memory, so no test can write to a real data folder. The `ui-18-title-continue` fixture. In the browser with real keys: an injected Keeper checkpoint continued to the Keeper, travelling saved the Foundry arrival, a reload offered and restored it (`d-web-reload-then-continue.jpg`), abandoning deleted it, and **N** replaced it. The plan said to reach the Keeper with `?start=gate`, but practice runs never save, so the checkpoint was injected instead. A native launch with a saved checkpoint ran cleanly, but its title screen couldn't be captured. |
| E. Remember fullscreen | `74e53b7` | Settings tests; the options fixture with its new row (`e-options-fullscreen-row.jpg`). A temporary probe, since removed, measured the native window at launch: 1024 × 576 logical units windowed, and 3072 × 1728 (3840 × 2160 physical) with fullscreen saved. The browser build deliberately doesn't restore it. |

Findings for a later round:

- Quitting and continuing restarts the current biome from its entrance with the build you arrived with, so it can undo a bad fight. That is the usual save-on-arrival trade-off, but if it matters, deleting the checkpoint once it's loaded (continue once) would close it.
- The browser lets **F11** through to its own fullscreen as well as the game's. This was already true before this round.
- A `--start-at` run doesn't save, so anything that depends on saving still needs injected browser storage to test.

Still deferred: new content (14) and a smaller web download (15). Shrinking the art is an owner decision, and the new audio made the download 3.7 MB larger. Gamepad support, web hosting, licenses, and guardian health remain owner decisions.

## Round 4 scope

Four items. Gamepad support (item 9) has been waiting since round 1 because no controller is attached to this machine. It can now be checked with more than unit tests: the kernel's `uinput` interface is writable here, so a test script can create a virtual controller that the game reads through the same path as a real one, and the browser build can be given a scripted controller through the Gamepad API. That isn't the same as a person holding a pad, and the report will say so. The two gamepad items come first because they are the largest; the smaller items follow. Each item gets the usual checks: `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, `cargo test --locked`, the capture modes, and screenshots in [`media/improvements/round4/`](media/improvements/round4/). Every native run uses a temporary `XDG_DATA_HOME`, and the real save folder is checked before and after.

### A. Play with a gamepad (item 9)

A fixed layout in the Xbox naming most pads use: left stick or D-pad to move and drop, **A** jump, **X** strike, **B** dodge, **Y** interact, **RB** parry, **RT** glassbolt, **LB** fire vessel, **LT** arc snare, **D-pad up** flask, **Start** pause, **View/Select** atlas.

- **Acceptance:** on the desktop (through `gilrs`), every gameplay action works from a controller, alongside the keyboard and mouse. The stick has a dead zone. Plugging a controller in or out mid-game is handled, and if the platform's controller support can't start, the game still runs on the keyboard. The browser build reads the same layout through the Gamepad API. A button pressed to leave a menu doesn't also act in the game. The README's controls table lists the layout, and the Linux build notes name the one new system library (`libudev`).
- **Verify:** unit tests for the mapping, the dead zone, and combining pad and keyboard input; natively, a virtual controller created through `uinput` drives the release binary (move, jump, strike, dodge, a tool) with screenshots; in headless Chrome, a scripted controller does the same. No physical controller will be tested.

### B. Menus and on-screen prompts for the gamepad

- **Acceptance:** every screen can be used with only a controller: title (begin, continue, new descent, options), pause (resume, options, abandon twice), options and the controls page (navigate, change, back), memories, reliquaries, and the Keeper (purchases, route, travel), death, and victory. Choices numbered **1 / 2 / 3** map to **X / Y / B**, which sit left, top, and right on the pad, matching the cards. The HUD badges, title line, pause list, tips, and menu buttons show pad buttons after the controller was last used and keys after the keyboard was, switching back and forth. The controls page shows the fixed pad layout next to the rebindable keys.
- **Verify:** unit tests for the menu mapping and prompt switching; `--ui-gallery` fixtures for the HUD, title, and a choice screen with pad prompts; the virtual controller in the native build and the scripted one in the browser go from the title through a run to the pause screen and options and back.

### C. Pause when the game loses focus

At the moment, switching to another window or browser tab leaves the run playing, so guardians keep attacking an unattended hero.

- **Acceptance:** losing window focus (on X11 and XWayland) or hiding the browser tab during play opens the pause screen. Other screens are unaffected, and capture and test modes never pause themselves. The platform's own focus events are used; macOS and native Wayland windows may report only minimising, which the docs will say.
- **Verify:** a unit test of the rule; in the browser, a focus loss signalled through the page's own focus check pauses the run (screenshot); natively, an attempt to move focus away from the game window, reported honestly if it can't be done here.

### D. Quit from the menus (rest of item 11)

With only a controller there's no way to close the desktop game.

- **Acceptance:** on the desktop, the title screen offers **Esc** or **B** pressed twice to quit, and the pause screen offers **Q** or **View** pressed twice; the first press shows a warning, as abandoning does. Quitting mid-run behaves exactly like closing the window (the run's last checkpoint stays). Settings and progress are already saved when they change, so nothing extra is written. The browser build doesn't offer it.
- **Verify:** unit tests of the confirmation rule on both screens; the virtual controller quits the native release binary from the title (exit status 0, and the temporary data folder unchanged).

Owner decisions still open, and not part of this round: one-use continue saves, web hosting, releases, signing, shrinking the art, the trailer, and guardian health.

## Round 4 results (October 6, 2026)

All four scoped items shipped on `improvements-4`. Items A and B share one commit because the controller's menu handling and its prompts were built and verified together. Every commit passed `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, and `cargo test --locked` (97 tests became 106), and a guard confirmed the test suite left its temporary data folder empty. `--vertical-capture` still reaches 37 of 37 waypoints, `--motion-capture` still produces 300 frames (CPU submission mean 1.36 ms, p95 3.13 ms at a load average of about 24–29, so not comparable with round 3's figures), and `--ui-gallery` now has 24 fixtures. The 19 earlier fixtures matched round 3's captures at 47–58 dB PSNR, except the controls page (39 dB), which now has a controller column. The Linux tarball, extracted, ran the gallery. Every native run used a temporary `XDG_DATA_HOME`, and `~/.local/share/cinderwake` didn't exist before or after the round. The wasm is unchanged in size (52.5 MB). Screenshots are in [`media/improvements/round4/`](media/improvements/round4/).

| Item | Commit | How it was verified |
| --- | --- | --- |
| A. Play with a controller | `0357453` | Unit tests for the layout, dead zone, diagonal running, held-over buttons, and combining with the keyboard. A new [`scripts/virtual-pad.py`](../scripts/virtual-pad.py) created a virtual controller through `uinput`; read by gilrs, it drove the release build from the title through movement, double jump, strikes, dodge, all three tools, and the atlas (`a-native-*.jpg`, recorded with a new `--snapshot-every` testing flag because the XWayland window couldn't be captured from outside). In headless Chrome, a scripted pad that replaced `navigator.getGamepads` did the same with no console errors (`a-web-scripted-pad.jpg`). **No physical controller was used.** The first planned stick curve reached only 85% speed on a diagonal push, so a full run now starts at 70% travel. |
| B. Menus and prompts | `0357453` | A test drives every menu with the controller alone: title, atlas, pause, options, controls page, abandon, death, memory, reliquary, and the Keeper's route and travel. Four new gallery fixtures show controller prompts (`b-pad-prompt-fixtures.jpg`). Natively and in the browser, the pad went from the title through pause and options and back; the browser also took a reliquary weapon with **X**, and one key press switched the prompts back to keys. In headless Chrome (12 fps under load), 120 ms taps were sometimes missed between frames, and 300 ms taps worked. A real 60 fps game polls every 17 ms. |
| C. Pause on focus loss | `e8a36e0` | A unit test of the rule. Natively, opening a second game window over the first paused the run, a pad **Start** press while unfocused was ignored, and **B** resumed after the second window closed (`c-native-focus-loss.jpg`). In headless Chrome, hiding the page through its visibility state paused the run, which stayed paused when shown again until **Esc** (`c-web-hidden-tab.jpg`). macOS and native Wayland weren't checked. |
| D. Quit from the menus | `8822e55` | Unit tests of the confirmation, and the menu test above. Two **B** presses from the virtual controller on the title closed the release build with exit status 0, and its data folder was byte-for-byte unchanged (`d-native-title-quit-warning.jpg`). The `ui-23-paused-quit` fixture (`d-paused-quit-fixture.jpg`). |

Findings for a later round:

- The opening biome has no checkpoint: a run is saved only from the second biome on. The README said "arriving in each biome"; it now says so accurately, and the quit warning says which case applies. Whether the first biome should save too belongs with the owner's decision on continue-saves.
- The controller layout is fixed. Rebinding pad buttons, and PlayStation or Nintendo button names, would need a physical controller to check.
- While `scripts/virtual-pad.py` runs, every program on the machine that reads controllers sees its device. The scripts here lasted under 40 seconds each.

Still deferred: new content (14) and a smaller web download (15). Owner decisions still open: one-use continue saves (and with it, whether the first biome saves), web hosting, releases and signing, shrinking the art, the trailer, and guardian health.

## Round 5 scope

Four items. They were chosen for what a player meets in every session, and each one can be checked on this machine. A review of the code for this round found two small defects that shaped the list: the death and victory screens accept **Enter** or the controller's **A** the instant they open, so a player still pressing jump as the hero falls skips straight into a new run; and the desktop window still carries Miniquad's default logo as its icon, because the game never set its own. Each item gets the usual checks: `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, `cargo test --locked`, the capture modes, and screenshots in [`media/improvements/round5/`](media/improvements/round5/). Every native run uses a temporary `XDG_DATA_HOME`, and the real save folder is checked before and after.

### A. A recap at the end of every run

At the moment the death screen says only how many guardians fell, the time, and the banked embers. It doesn't say what killed you, how far you got, or whether you did better than before.

- **Acceptance:** the death and victory screens show what ended the run (the kind of guardian, the Regent, a hazard, or abandoning), where (biome and stage), the build (weapon and tier, the three memories, the mutation), the embers lost or banked, and personal records: most guardians felled (already saved), deepest stage reached, and fastest victory. A record broken by this run is marked as new. The title screen shows the fastest victory once there is one. The new records are extra fields in `progress.json` that older files simply lack; checkpoint saving is unchanged. Practice runs never change records. Both screens ignore **Enter** and **A** for their first second, so a button held or mashed as the run ends doesn't skip the recap. The pause screen gains one line with the run's stage, weapon, tier, and mutation, which play never shows.
- **Verify:** game tests that each damage source (warden, archer, moth, brute, Regent strike and bolt, hazard, abandoning) is named as the cause; that records update once, only when beaten, and never in practice; that an older `progress.json` still loads; and that confirming is ignored for the first second. New `--ui-gallery` fixtures for the death recap, the victory recap with new records, and the pause line. The browser build reaches the death screen with real keys and shows the recap.

### B. A game-speed option

Guardians telegraph and strike within a fraction of a second, and parries need precise timing. There's no way to slow the game down for players who need more time to react.

- **Acceptance:** an options row, **Game speed**, from 100% down to 50% in 10% steps, saved with the other settings. It slows the whole simulation evenly (movement, enemies, projectiles, timers), so nothing gets easier or harder except the time available to react. Menus, music, and crossfades run at normal speed. The pause screen and the run recap show the speed when it's below 100%. Capture, gallery, and practice modes ignore it. Recorded times use simulated seconds, so a slower speed doesn't shorten a fastest-victory record.
- **Verify:** settings tests (default, clamping, round trip, an older file without the field); a unit test that the main loop's simulated time per real second scales with the setting; the options gallery fixture with the new row; in the browser build with real keys, set 50% and measure the run timer against the wall clock.

### C. The game's own window icon

- **Acceptance:** the desktop window uses the Cinderwake crest as its icon at 16, 32, and 64 pixels, in place of Miniquad's logo, on Linux (X11 and XWayland), Windows, and macOS when run outside the app bundle. On Linux the window also reports `cinderwake` as its class, which desktops use to match windows to launchers.
- **Verify:** a unit test that the icon images have the sizes Miniquad expects and aren't blank; natively on XWayland, read the window's `_NET_WM_ICON` and `WM_CLASS` with `xprop` and render the icon data to an image. Windows and macOS can't be checked here.

### D. Browser shortcuts that fight the game

Miniquad's browser loader blocks the browser's default action for Space, the arrows, Tab, and F1–F10, but not **F11**, so pressing it toggles the browser's fullscreen and the game's at once (a round 3 finding).

- **Acceptance:** in the browser build, **F11** is handled only by the game. No other keys change behaviour. The README and ENGINE.md describe it.
- **Verify:** in headless Chrome with real key events through the DevTools protocol, check that the F11 key event's default action is prevented and that the game's own fullscreen toggle still runs. Whether a desktop browser's window chrome behaves can't be seen in headless mode, and the report will say so.

Not in this round: controller button rebinding (round 4 found it needs a physical controller to check), more Keeper mutations or weapons (new content changes the Keeper's three-choice layout and needs a design decision and playtesting), and anything to do with saving the opening biome. Owner decisions still open: one-use continue saves and whether the first biome saves, web hosting, releases and signing, shrinking the art, the trailer, a physical-controller test, and guardian health.

## Round 5 results (October 6, 2026)

All four scoped items shipped on `improvements-5`, one commit each. Every commit passed `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, and `cargo test --locked` (106 tests became 111), and a guard confirmed the test suite left its temporary data folder empty. `--vertical-capture` still reaches 37 of 37 waypoints in 25.2 s, `--motion-capture` still produces 300 frames (CPU submission mean 1.15 ms, p95 1.70 ms at a load average of about 22), and `--ui-gallery` still has 24 fixtures. Compared with a build of `main`, the 17 fixtures this round didn't touch matched at 51–64 dB PSNR, and the 7 that differ are the screens it changed (title, the three pause screens, death, victory, options). The Linux tarball, extracted, ran the gallery. The wasm is unchanged in size (52.6 MB). Every native run used a temporary `XDG_DATA_HOME`, which stayed empty, and `~/.local/share/cinderwake` didn't exist before or after the round. Screenshots are in [`media/improvements/round5/`](media/improvements/round5/).

| Item | Commit | How it was verified |
| --- | --- | --- |
| A. Run recap and records | `7c5fc7f` | Game tests that every damage source (warden, brute, moth, and Regent strikes; archer and Regent bolts; spikes; abandoning) is named as the cause, that records count as new only when they beat a saved one (never on a file from before records, never in practice), that a slower win keeps the record, and that the result screens wait a second. A save test for older and out-of-range files. The controller menu test now checks that **A** is ignored on the death screen until the second has passed. New fixture content for death, victory, the title's fastest win, and the pause status line (`a-*.jpg`). In headless Chrome with real keys: abandoning opened the recap with the button hidden (`a-web-recap-opening.jpg`), two **Enter** presses within 523 ms were ignored, a later **Enter** started a new run, and `progress.json` gained `best_stage`. |
| B. Game speed | `87b4bd0` | Settings tests (default, limits, saving, older files); a test that one real second runs 1, 0.7, or 0.5 simulated seconds in play and 1 in menus and scripted modes, and that half speed reaches the same position in twice the frames. Options and pause fixtures (`b-options-game-speed.jpg`, `b-pause-game-speed.jpg`). In headless Chrome, 50% set with real keys was saved, and about 20 s of play showed 00:09 on the run timer against 00:18 at 100% (`b-web-timer-after-20s-100-vs-50.jpg`, load average about 25; the browser's frame rate under load explains 18 rather than 20). |
| C. Window icon | `79b4109` | A unit test of the three icon sizes (opaque centre, clear corner, teal glass). The release build's XWayland window reported `WM_CLASS "cinderwake"` and a `_NET_WM_ICON` with 16, 32, and 64 pixel images, rendered back to `c-window-icon-from-xprop.png`. Windows and macOS were not checked, nor whether a given taskbar shows it. |
| D. Browser F11 | `0653b60` | In headless Chrome with DevTools key events: before the change F11's default action wasn't prevented; after it, it is, the game's canvas still enters and leaves fullscreen, and Space, the arrows, Tab, F5, J, and Escape behave as before. Headless mode has no browser window, so the double toggle itself couldn't be seen. |

Findings for a later round:

- If a browser player leaves fullscreen with **Esc**, the game still thinks it's fullscreen, so the next **F11** only catches up and a second one re-enters. Miniquad doesn't report the browser's fullscreen state to the game.
- The recap shows the game speed as currently set, not a history of the run, and records don't distinguish runs played below 100%. Recorded times are simulated seconds, so a slower speed can't shorten one, but whether records should be kept apart for slowed runs is a design choice.
- Miniquad's browser loader also blocks F5 (reload) while the game has focus. That was already true and was left alone.

Still deferred: controller button rebinding (needs a physical controller), new content (14), and a smaller web download (15). Owner decisions still open: one-use continue saves and whether the first biome saves, web hosting, releases and signing, shrinking the art, the trailer, a physical-controller test, and guardian health.

## Round 6 scope

Four items, plus one stretch item. A review of the code for this round found that a damaged `progress.json` is silently replaced: `Save::load` falls back to default progress and `Game::start` writes it straight back, so one bad write (a full disk, a crash mid-copy, a hand edit) loses every banked ember, upgrade, and victory without a word. It also found that a guardian's only warning before it strikes is a 2 × 4 pixel mark above its head, with no sound, which matters more now that the game-speed option exists for players who need time to react. Each item gets the usual checks: `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, `cargo test --locked`, the capture modes, and screenshots in [`media/improvements/round6/`](media/improvements/round6/). Every native run uses a temporary `XDG_DATA_HOME`, and the real save folder is checked before and after.

### A. Keep a save the game can't read

- **Acceptance:** if `progress.json` or `settings.json` exists but can't be read, the game copies its bytes to `progress.unreadable.json` (or `settings.unreadable.json`) before anything can overwrite it, then carries on with defaults as now. An existing copy is never overwritten, so a second failure can't destroy the first. The title screen says what happened and names the file. The browser build does the same under its `cinderwake/` keys. Practice and capture modes still read nothing. Files that read correctly behave exactly as before.
- **Verify:** unit tests (through the in-memory test storage) for a damaged progress file and settings file: the copy is made once, the damaged bytes survive starting a run, and a good file makes no copy; a `--ui-gallery` fixture for the title notice; a native release run with a damaged file in a temporary `XDG_DATA_HOME` (both files checked afterwards); in headless Chrome, damaged `localStorage` produces the copy and the notice.

### B. Clearer warnings before a guardian strikes

- **Acceptance:** while a warden, brute, archer, or the Regent winds up, a larger pulsing warning mark shows above it, and the ground shows how far the strike will reach (wardens and brutes, and the Regent's lunge); an archer shows a faint line along its aim. The reach shown comes from the same numbers the hit test uses, so it can't drift. A short, new synthesized "tell" sound plays when a guardian that's on screen starts winding up, at most once every 0.2 s; the effects volume and mute apply. Timing, damage, and every other number stay the same; only what the player can see and hear changes.
- **Verify:** game tests that a windup queues exactly one tell, that off-screen guardians stay silent, and that the throttle holds; a test that the drawn reach matches the hit test's range; a new `--ui-gallery` fixture with guardians winding up; the synthesis script's clipping check and a spectrogram of the new cue; `--motion-capture` still produces every frame. Nobody will listen to the cue here.

### C. Pause when a controller disconnects

- **Acceptance:** if a controller is removed during play, the run pauses and the pause screen says the controller was disconnected; reconnecting doesn't unpause on its own. Other screens and capture modes are unaffected. Desktop (gilrs) and browser (Gamepad API) both do this.
- **Verify:** a unit test of the rule; natively, `scripts/virtual-pad.py` starts a run with a virtual controller and then removes it, and a snapshot shows the paused run; in headless Chrome, a scripted pad that disappears does the same.

### D. The browser's fullscreen state stays in step

- **Acceptance:** in the browser build, leaving fullscreen through the browser (its **Esc**) updates the game's fullscreen setting, so the next **F11** enters fullscreen again on the first press and the options row shows the true state. Desktop behaviour is unchanged.
- **Verify:** in headless Chrome with DevTools key events: F11 enters fullscreen, leaving through the page's own `document.exitFullscreen()` (what the browser's Esc does) turns the setting off, and one F11 press re-enters. A desktop browser's window chrome can't be seen in headless mode.

### E. Stretch: click menus with the mouse

- **Acceptance:** the title's begin and continue buttons, the death and victory buttons, the memory, reliquary, and Keeper choice cards, and the Keeper's routes respond to a left click; a click that closes a menu doesn't also strike. Ships only if A–D are done.
- **Verify:** unit tests of the hit areas against the drawn layout; real mouse clicks in headless Chrome.

Not in this round: controller button rebinding (needs a physical controller), more Keeper mutations or weapons (a design decision), and anything about how runs are saved or recorded (one-use continues, the first biome, separate records for slowed runs). Owner decisions still open: those saving questions, web hosting, releases and signing, shrinking the art, the trailer, a physical-controller test, and guardian health.

## Round 6 results (October 7, 2026)

All four scoped items and the stretch item shipped on `improvements-6`, one commit each. Every commit passed `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, and `cargo test --locked` (111 tests became 121). A guarded run of the suite, with `TMPDIR` and `XDG_DATA_HOME` pointed at empty folders under `target/`, left both empty. `--vertical-capture` still reaches 37 of 37 waypoints in 25.2 s, `--motion-capture` still produces 300 frames (CPU submission mean 1.17 ms, p95 1.69 ms at a load average of about 29), and `--ui-gallery` now has 27 fixtures. The 24 earlier fixtures matched a build of `main` at 51.6–63.5 dB PSNR, the gallery's normal run-to-run variation. The Linux tarball, extracted, ran the gallery. The wasm grew by about 10 KB, for the new sound, and is still 52.6 MB. Every native run used a temporary `XDG_DATA_HOME`, and `~/.local/share/cinderwake` didn't exist before or after the round. Browser checks used headless Chrome driven through the DevTools protocol, at load averages of about 22–30. Screenshots are in [`media/improvements/round6/`](media/improvements/round6/).

| Item | Commit | How it was verified |
| --- | --- | --- |
| A. Keep an unreadable save | `b02a4b4` | Game and storage tests: damaged progress and settings files are copied once, the copy survives starting a run, a different damaged file gets `-2`, the same one isn't copied twice, empty and readable files make no copy, and the ninth copy is the last. The `ui-24-title-unreadable-save` fixture. Natively, the release build launched with a truncated `progress.json` and a garbage `settings.json` showed the notice, and when a virtual controller started a run, fresh progress was written while both copies matched the damaged files byte for byte (`a-native-damaged-save-title.jpg`). In the browser, damaged `localStorage` entries gave the same notice and copies, and **Enter** wrote fresh progress without touching the copy (`a-web-damaged-storage-title.jpg`). |
| B. Attack warnings | `5fd1ac4` | Tests that a windup sounds one tell, guardians starting within 0.2 s share it, a later windup gets its own, and an archer off screen is silent; a test that strikes land just inside `strike_reach` and miss just outside it, for wardens, brutes, and the Regent, across and up; a test that the drawn reach and the Regent's volley follow the same numbers. The `ui-25-attack-warnings` fixture shows a brute, a warden, and an archer mid-windup (`b-attack-warnings-*.jpg`). The tell passes the synthesis script's clipping check, sits at −24.5 dBFS RMS among effects at −18.6 to −25.5, and every earlier audio file regenerated byte for byte (`b-tell-spectrogram.jpg`). **Nobody listened to it**, and the Regent's volley fan was checked only by unit test, not seen in a capture. |
| C. Pause when a controller is removed | `32d5d0e` | Tests of the rule (play pauses, other screens don't, resuming clears the note) and of the connection count. Natively, `scripts/virtual-pad.py` started a run and walked, and when it exited and removed its device, the run paused under **Controller disconnected** with keyboard prompts (`c-native-*.jpg`). In the browser, a scripted pad vanishing from `navigator.getGamepads()` did the same, and reconnecting restored the usual heading with the run still paused (`c-web-scripted-pad-removed.jpg`). No physical controller. |
| D. Browser fullscreen in step | `50c1e24` | A unit test of the rule. In headless Chrome: F11 entered fullscreen, `document.exitFullscreen()` (what the browser's Esc does) left it and saved the setting as off, one more F11 re-entered, and the options row read **Off** after leaving (`d-web-options-after-browser-exit.jpg`). A desktop browser's own Esc and window chrome can't be exercised in headless mode. |
| E. Click menus (stretch) | `8253374` | A test that clicks through every target (title, pause, memory, reliquary, Keeper rows, route, and travel, and the recap's button after its first second), and one for mapping clicks through the letterbox. Fixtures compared with the previous build to confirm the shared rectangles didn't move anything. In headless Chrome with real mouse events, at pixel ratios 1 and 1.25: a click beside the title's button did nothing, the button began a run, holding it didn't swing the sabre while a fresh click did, the Furnace Maul's card took it, and the Keeper's destination and button took the run to the Foundry (`e-web-*.jpg`). Native mouse clicks weren't driven: no input tool is installed, and a virtual mouse would move the shared desktop's real pointer. |

Findings for a later round:

- The options and controls pages, the quit and abandon confirmations, and **N** for a new descent still need the keyboard or a controller, and buttons don't highlight under the cursor.
- If a browser refuses a fullscreen request, the setting can still say on while the page isn't.
- Miniquad's loader logs that the `cinderwake_pad` plugin is "not used in the rust code", because the pad plugin has no version export. It's harmless and predates this round.

Still deferred: controller button rebinding (needs a physical controller), new content (14), and a smaller web download (15). Owner decisions still open: one-use continue saves and whether the first biome saves, separate records for slowed runs, web hosting, releases and signing, shrinking the art, the trailer, a physical-controller test, and guardian health.

## Round 7 scope

Five items. Round 6 made the mouse choose buttons and cards, but a mouse player still has to reach for the keyboard for the options and controls pages, for opening options, abandoning, quitting, muting, or starting a new descent, and nothing on screen reacts to the cursor. The first four items finish that, so the game can be played from the title to the end of a run with a mouse alone. The fifth closes two small browser findings from round 6. Saving, balance, and content stay as they are. Each item gets the usual checks: `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, `cargo test --locked`, the capture modes, and screenshots in [`media/improvements/round7/`](media/improvements/round7/). Every native run uses a temporary `XDG_DATA_HOME`, and the real save folder is checked before and after.

### A. Options and controls pages with the mouse

- **Acceptance:** on the options page, a click on a switch row flips it, a click on the **Controls** row opens the controls page, a click on a volume, shake, or speed bar sets that level, and small **<** and **>** arrows beside the selected bar step it down and up (so 0% and the speed limit can be reached). Clicking a row's name selects it. **Back** is a button. On the controls page, a click on an action starts listening for its new key (as **Enter** does), a click on **Restore default keys** restores them, a click while listening cancels (mouse buttons can't be bound), and **Back** closes the page. Keyboard and controller behaviour doesn't change.
- **Verify:** unit tests that click the centre of every target through the same menu code the game runs; the options and controls fixtures in `--ui-gallery`; in headless Chrome with real mouse events, set a volume, flip a switch, rebind a key, and go back, with the saved settings checked afterwards.

### B. Title and pause actions with the mouse

- **Acceptance:** the footer lines on the title (new descent, options, mute, quit) and the pause screen (options, abandon, quit, mute) become click targets in fixed slots, still showing their keys. The confirmations stay clickable: the first click on **Abandon run** or **Quit** turns that link into **Confirm abandon** or **Confirm quit** and shows the warning above it, and a second click does it. **New descent** moves from the continue line into the title's footer. The browser build still offers no quitting. Keyboard and controller behaviour doesn't change.
- **Verify:** unit tests clicking every link, including both confirmations and new descent; updated title, pause, abandon, and quit fixtures; in headless Chrome, abandon a run and open options from pause using only clicks.

### C. Buttons and rows that answer the cursor

- **Acceptance:** while the mouse is in use, the button, card, row, or link under it is highlighted, and the cursor becomes a hand over anything clickable. Moving the mouse over an options or controls row selects it, so its help line shows. A key press or controller input hides the highlight until the mouse moves again, so a still cursor never fights the keyboard's selection. Highlights come from the same rectangles as the clicks, so they can't drift apart.
- **Verify:** a unit test that hovering finds the same targets clicking does and that a key press clears it; a test that moving over a row selects it and a still pointer doesn't; new `--ui-gallery` fixtures with the pointer over a button, a card, and an options arrow; in headless Chrome, the canvas cursor style reads `pointer` over a button and `default` off it.

### D. No cursor over the game in play

- **Acceptance:** the mouse cursor is hidden while a run is being played (including the atlas) and shown on every menu, so it doesn't sit over the action. Leaving play by pausing, dying, or any menu brings it back at once.
- **Verify:** a unit test of the rule for each screen; in headless Chrome, the canvas cursor style reads `none` in play and returns on the pause screen. The desktop cursor can't be observed here without moving the shared desktop's pointer, so it is checked only through the same code path.

### E. Browser fullscreen refusals and a loader warning

- **Acceptance:** in the browser build, if the page asks for fullscreen and the browser hasn't entered it a second later (it refused, as browsers can), the setting turns itself off, is saved, and a short notice says the browser didn't allow it. Miniquad's loader no longer logs that the `cinderwake_pad` plugin is "not used in the rust code". Desktop behaviour is unchanged.
- **Verify:** a unit test of the refusal rule; in headless Chrome, `canvas.requestFullscreen` replaced with one that rejects, then F11: the setting reads off afterwards and the notice shows; the console has no plugin warning.

Not in this round: controller button rebinding (needs a physical controller), new weapons or mutations (a design decision), anything about saving or records, and guardian health. Owner decisions still open: one-use continue saves and whether the first biome saves, separate records for slowed runs, web hosting, releases and signing, shrinking the art, the trailer, a physical-controller test, and guardian health.

## Round 7 results (October 7, 2026)

All five scoped items shipped on `improvements-7`, one commit each. Every commit passed `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, and `cargo test --locked` (121 tests became 128), with the suite's `XDG_DATA_HOME` pointed at an empty folder under `target/` that stayed empty. `--vertical-capture` still reaches 37 of 37 waypoints in 25.2 s, `--motion-capture` still produces 300 frames (CPU submission mean 1.16 ms, p95 1.72 ms at a load average of about 26–29), and `--ui-gallery` now has 31 fixtures. Compared with a build of `main`, the 17 fixtures this round didn't touch matched at 50.5–56.3 dB PSNR, and the 10 that differ (33–40 dB) are the screens it changed: the four title fixtures, the four pause fixtures, the options page, and the controls page (whose listening note now says a click cancels). The Linux tarball, extracted, ran the gallery. The wasm is 52,588,057 bytes, still 52.6 MB. Every native run used a temporary `XDG_DATA_HOME`, and `~/.local/share/cinderwake` didn't exist before or after the round. Browser checks used headless Chrome through Playwright with real mouse and key events, at load averages of about 24–39 (frames ran at about 11 per second). Screenshots are in [`media/improvements/round7/`](media/improvements/round7/).

| Item | Commit | How it was verified |
| --- | --- | --- |
| A. Options and controls by mouse | `79981d2` | A test clicks through both pages via the same menu code the game runs: a bar's name only selects it, a bar sets its level, the arrows step music down to 0% and back, speed stops at 50%, a switch flips and flips back, an action listens and a key binds it, a click cancels listening, restore and both **Back** buttons work. A settings test for `set_level`. In the browser, clicks set the music to 30%, turned hit-stop off, and rebound Jump to G, and `settings.json` held all three (`a-web-options-and-controls-by-mouse.jpg`). |
| B. Title and pause links | `f20c856` | A test clicks every link: mute twice, options and back, quit then **Confirm quit**, new descent with a run to continue, and pause options, quit, and abandon twice (arming one confirmation clears the other). The title, pause, abandon, quit, and controller fixtures (`b-title-footer-links.jpg`). In the browser, **Abandon run** then **Confirm abandon** opened the recap, and the pause screen's **Options** link opened the options (`b-web-pause-confirm-abandon.jpg`). |
| C. Hover highlights | `ba2fdb5` | Tests that the pointer gives a hand over a target, that a still mouse keeps its place, that a key press or controller input clears it, and that only a moving mouse selects an options or controls row (never while listening for a key). Four new fixtures (`c-hover-*.jpg`). In the browser the canvas cursor read `pointer` over buttons and links and `default` off them. One reading of `pointer` just after the pause screen's **Options** link opened the options page was taken before the next frame ran; read again over two seconds it was `default` throughout. |
| D. Cursor hidden in play | `10dc173` | A test that the cursor hides in play and the atlas, shows on every other screen, and returns when losing focus pauses. In the browser the canvas cursor read `none` after beginning, rising again, and resuming, and `default` on the pause screen. The desktop cursor wasn't observed (see below). |
| E. Browser fullscreen and plugin warning | `9be606b` | A unit test of the one-second rule. In the browser, with `requestFullscreen` made to reject, F11 left the setting on for a moment, then off, saved, with the notice in play (`e-web-fullscreen-refused-notice.jpg`); with the real function, F11 entered fullscreen and the setting stayed on. The deliberate rejection surfaced as an uncaught page error, because Miniquad's loader doesn't catch it. The loader's "not used" message for `cinderwake_pad` is gone; it still prints one for Macroquad's own `sapp_jsutils` and `quad_net`. |

Not verified: native mouse clicks, the desktop hand cursor, and hiding the cursor on the desktop. No input tool is installed here, and a virtual mouse would move the shared desktop's pointer, so the desktop runs the same code as the browser but wasn't driven. A desktop browser's own fullscreen refusal (rather than a scripted one) wasn't seen either.

Findings for a later round:

- The options page's footer still lists only keys (**W / S**, **A / D**, **M**), and the controls page's hint still says **ENTER**; a mouse player doesn't need them, but they could mention clicking.
- The atlas has no close button, so a mouse player closes it with **Tab** or **View**. It's part of play, where a click strikes.
- Miniquad doesn't catch a rejected fullscreen request, so a real refusal logs one uncaught error in the browser's console. Catching it would mean patching the vendored loader.

Still deferred: controller button rebinding (needs a physical controller), new content (14), and a smaller web download (15). Owner decisions still open: one-use continue saves and whether the first biome saves, separate records for slowed runs, web hosting, releases and signing, shrinking the art, the trailer, a physical-controller test, and guardian health.

## Round 8 scope

Five items. Rounds 6 and 7 finished the menus; this round goes back to what a player meets in a fight and every time the game opens. A review for this round found three gaps. A guardian can wind up out of view: archers aim from 300 units across and 200 up or down, the view is 640 × 360, and the HUD panels cover its top and bottom strips, so an archer above or below (or behind a panel) can shoot with no warning at all, and the round-6 tell sound deliberately stays silent for guardians off screen. Low vitality is shown only by the bar turning red in the top-left corner. And the desktop window always opens at 1280 × 720 pixels, a third of the width of a 4K screen, however the player last sized it. The last two items close round-7 findings. Saving, balance, and content stay as they are. Each item gets the usual checks: `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, `cargo test --locked`, the capture modes, and screenshots in [`media/improvements/round8/`](media/improvements/round8/). Every native run uses a temporary `XDG_DATA_HOME`, and the real save folder is checked before and after.

### A. Markers for threats out of view

- **Acceptance:** during play, when a guardian is winding up or a hostile bolt is flying toward the hero outside the part of the view the HUD doesn't cover, a marker at that edge of the play area points toward it. A windup's marker uses the same yellow-to-red colour as its on-screen warning mark; a bolt's is smaller. Markers stay clear of the HUD panels, disappear once the threat is in view, and aren't drawn with the atlas open or on menus. Only threats near enough to matter are marked (a windup always qualifies, since guardians only wind up near the hero). Timing, damage, and the tell sound don't change.
- **Verify:** unit tests of the marker rule (a threat in view gets none; one above, below, left, or right gets one on that edge, inside the play area; a bolt flying away gets none; a far one gets none); a new `--ui-gallery` fixture with an archer winding up above the view and a bolt arriving from the side; `--motion-capture` still produces every frame.

### B. A warning at low vitality

- **Acceptance:** at 30% vitality or less during play, the screen's edges take a soft red tint that pulses slowly and grows stronger as vitality falls, and, if a flask is left, the flask panel glows with its key. With **Reduce flashes** on, the tint holds steady instead of pulsing. Above 30%, nothing changes. It is drawn with the HUD, so lighting and bloom settings don't hide it.
- **Verify:** unit tests of the warning level (none above 30%, rising toward 0, steady with reduced flashes, none on menus); the existing low-health fixture (`ui-02`) shows it; a fixture with reduced flashes on matches the rule.

### C. Hints that mention the mouse, and an effects volume preview

- **Acceptance:** when the mouse was used last, the options page's footer says clicks choose and set rows and bars, the controls page's footer says a click on an action rebinds it, and the atlas's footer stays as it is (the atlas is part of play, where a click strikes). With keys or a controller, the footers are unchanged. Changing the effects volume by key, controller, or click plays one short sample effect at the new level (unless muted), so the level can be judged without leaving the page; the music volume is already audible.
- **Verify:** unit tests that each footer follows the last device and that changing the effects level queues exactly one sample effect (and none when muted or when another row changes); new `--ui-gallery` fixtures for the options and controls pages with the mouse footers.

### D. A browser's fullscreen refusal no longer logs an error

- **Acceptance:** in the browser build, a refused fullscreen request no longer surfaces as an uncaught error in the console. The fix lives in the game's own page (`web/index.html`), not in the vendored loader. Round 7's behaviour stays: the setting turns itself off and the notice shows.
- **Verify:** in headless Chrome with `requestFullscreen` made to reject: F11, then no uncaught page error, the setting off, and the notice; with the real function, F11 still enters fullscreen.

### E. The desktop window opens at its last size (Linux)

- **Acceptance:** on Linux (X11 and XWayland), resizing the window is remembered in `settings.json`, and the next launch opens at that size. Sizes below 640 × 360 pixels, or a size saved while fullscreen, are ignored. Fullscreen keeps working as before. Practice, capture, and gallery modes neither read nor write it, so their output is unchanged. macOS and Windows keep opening at the default size, because Miniquad measures windows differently there and neither can be checked here.
- **Verify:** settings tests (round trip, too small, older file without the field); natively, with a temporary `XDG_DATA_HOME`, resize the running window from outside (no `xdotool` or `xwininfo` is installed, so through a short X11 call from Python), quit, check `settings.json`, relaunch, and read the new window's size from a `--snapshot-every` image. If the window can't be resized from outside on this desktop, the check uses a written size only and the report says so.

Not in this round: controller button rebinding (needs a physical controller), new weapons or mutations (a design decision), anything about saving runs or records, a close button for the atlas (the atlas is part of play, where a click strikes, and a mouse-only player can't move anyway), and guardian health. Owner decisions still open: one-use continue saves and whether the first biome saves, separate records for slowed runs, web hosting, releases and signing, shrinking the art, the trailer, a physical-controller test, and guardian health.

## Round 8 results (October 7, 2026)

All five scoped items shipped on `improvements-8`, one commit each, though item C turned out smaller than planned (see below). Every commit passed `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, and `cargo test --locked` (128 tests became 136), with the suite's `XDG_DATA_HOME` and `TMPDIR` pointed at empty folders under `target/` that stayed empty. `--vertical-capture` still reaches 37 of 37 waypoints in 25.2 s, `--motion-capture` still produces 300 frames (CPU submission mean 1.19 ms, p95 1.70 ms at a load average of about 20–27), and `--ui-gallery` now has 35 fixtures. Compared with a build of `main`, 29 of the 31 earlier fixtures matched at 48.7–60.7 dB PSNR (the lowest, the pause screen, differs only along text edges); the two that differ are the screens this round changed (`ui-02`, now with the low-vitality tint, at 27.9 dB, and `ui-29`, whose options footer now describes clicks, at 37.1 dB). The Linux tarball, extracted, ran the gallery. The wasm is 52,598,957 bytes, still 52.6 MB. Every native run used a temporary `XDG_DATA_HOME`, and `~/.local/share/cinderwake` didn't exist before or after the round. Screenshots are in [`media/improvements/round8/`](media/improvements/round8/).

| Item | Commit | How it was verified |
| --- | --- | --- |
| A. Threats out of view | `4d0815f` | Tests that a windup in view gets no marker; one above, below, left, right, or hidden behind the top HUD panels gets one on that edge, inside the play area, pointing toward it; a bolt is marked only while arriving from within 420 units and only if hostile; a bolt beside a windup shares its marker; and nothing shows over the atlas or on menus. The new `ui-31-threats-out-of-view` fixture (`a-threats-out-of-view.jpg`). Not seen in live play: the motion capture's script doesn't put a threat out of view. |
| B. Low vitality | `77b5925` | A test of the warning level (none above 30%, about a third at it, rising toward none left, kept behind the atlas, absent on menus and after death) and of the tint pulsing, or holding steady with reduced flashes. The `ui-02` fixture at 15% with no flask (`b-low-vitality-pulse.jpg`) and the new `ui-32-low-vitality-steady` at 23% with a flask, flashes reduced (`b-low-vitality-steady-flask.jpg`). Whether the tint is strong enough, or too strong, needs a human eye. |
| C. Hints for the mouse | `32051ba` | A test that the options and controls footers and the Keeper's route line describe clicks after the mouse moves, controller buttons after a pad press, clicks again once the mouse moves, and keys after a key press. New `ui-33-hover-controls-row` and `ui-34-hover-camp-route` fixtures, and `ui-29` with the new options footer (`c-mouse-hints.jpg`). **The planned effects-volume preview already existed:** every options change already plays the select cue at the new level, so nothing was added for it. |
| D. Quiet browser refusal | `902b3ab` | In headless Chrome through Playwright, with `requestFullscreen` made to reject: the old page logged one uncaught `pageerror` ("Permissions check failed"), the new one none, and both turned the setting off and showed the notice (`d-web-fullscreen-refused-quietly.jpg`). With the real function, F11 entered and a second F11 left fullscreen, with no errors. A desktop browser's own refusal wasn't seen. |
| E. Window size (Linux) | `e9ac07a` | Settings tests (round trip, files without the field, sizes too small or too large) and a test of the one-second settle rule (dragging writes once, fullscreen never, the default and an unchanged size not at all). Natively, with a temporary `XDG_DATA_HOME`: `[1600, 900]` written into `settings.json` opened a 1600 × 900 window (X11 geometry and a `--snapshot-every` frame agree) without rewriting the file; with nothing saved, nothing was written; `--start-at chest` ignored the saved size and left the file unchanged; `[300, 200]` opened at 1280 × 720. **Saving after a real resize wasn't seen:** KWin under XWayland ignored both a plain X11 resize request and the EWMH `_NET_MOVERESIZE_WINDOW` message from a helper script, and no input tool is installed to drag the window's edge, so that path is covered by the unit test only. macOS and Windows are unchanged by design. |

Findings for a later round:

- The window opens at its last size but not its last position (not attempted this round), and a maximised window comes back the same size rather than maximised.
- Guardians winding up off screen still make no tell sound; the marker is visual only. Whether off-screen windups should sound too (perhaps quieter) is a design choice best made by listening.
- The low-vitality warning has no sound, deliberately, since nobody can listen here.

Still deferred: controller button rebinding (needs a physical controller), new content (14), and a smaller web download (15). Owner decisions still open: one-use continue saves and whether the first biome saves, separate records for slowed runs, web hosting, releases and signing, shrinking the art, the trailer, a physical-controller test, and guardian health.

## Round 9 scope

Four items. A review for this round found a problem a player would notice all the time. The game simulates at a fixed 120 steps per second and draws whatever the latest step left behind. A display that refreshes at any other rate (144 or 165 Hz monitors, 75 or 100 Hz, or a browser whose frames arrive a millisecond early or late) gets some frames that repeat the previous step and others that jump two steps ahead, so scrolling and running judder, even at 60 Hz once frame times wobble. This machine's display runs at 120 Hz, the one rate that should step evenly, and even there a small timing wobble produces the same uneven pairs. The browser build's page also shows a fixed "Kindling the city…" line during its 52.6 MB download, with no progress and no message if the download fails or the browser can't start WebGL, so a slow or broken load looks the same as a working one. Menus step a volume, speed, or row only once per press, so going from 100% to 0% takes ten presses. Finally, round 8's window-size saving was only unit-tested, because the desktop's KWin ignored resizes from outside. A private nested KWin (`kwin_wayland --virtual`, with its own Xwayland and D-Bus) runs here and can resize windows from its own scripting, so test windows can stay off the shared desktop.

Saving, balance, and content stay as they are. Each item gets the usual checks: `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, `cargo test --locked`, the capture modes, and screenshots in [`media/improvements/round9/`](media/improvements/round9/). Every native run uses a temporary `XDG_DATA_HOME` inside a private nested KWin, and the real save folder is checked before and after.

### A. Smooth motion between simulation steps

- **Acceptance:** during play, the world is drawn between the last two simulation steps, in proportion to the time left over (standard fixed-step interpolation), so each frame shows the moment it represents. That applies to the camera, the hero, guardians, and bolts. Anything that jumps (travel, respawning on a safe ledge, a new level) is drawn where it lands, never partway. The simulation, its inputs, and its outcomes don't change; only drawing does, and the view lags the newest step by at most one step (8.3 ms). Menus, staged views, and the capture modes draw whole steps as before, so every `--ui-gallery` fixture and capture stays byte-identical to `main`'s, given the same run-to-run variation.
- **Verify:** unit tests that a hero running at constant speed moves an even distance every frame at 60, 120 (with timing wobble), 144, and 165 Hz with blending, but repeats or skips steps without it; that a teleport isn't blended; and that blending leaves the game state exactly as it found it. A new `--pacing-check` testing mode records 8 seconds of real frame times in a practice run and reports how many frames repeated or skipped a step without blending, and how far each frame's shown moment strays from its real time with and without blending. It runs in the nested KWin, and the report gives its refresh rate and the load average. Comparing `--ui-gallery` fixtures and `--motion-capture` frames against a build of `main` confirms they're unchanged. Whether it *looks* smoother on a 144 Hz monitor needs a human with one.

### B. Browser loading progress and failure messages

- **Acceptance:** while `cinderwake.wasm` downloads, the page shows a progress bar with megabytes received out of the total, then "Preparing the city…" while the game starts. If the download fails (network error or an HTTP error), the browser can't start WebGL, or the WebAssembly can't be compiled or started, the page says so in plain words with a suggestion (reload, or try another browser), instead of the loading line staying up forever. If the game later stops with a WebAssembly error, a notice says it stopped and that progress up to the last save is kept. The vendored loader isn't changed. A successful load looks as before once the game draws its first frame.
- **Verify:** headless Chrome through Playwright: with the network throttled, screenshots of the bar partway; a server that answers 404 for the wasm shows the download error; Chrome started with WebGL disabled shows the WebGL message; a normal load reaches the title with no console errors; the page's error listener shown with a simulated WebAssembly error. Real slow connections and other browsers aren't checked.

### C. Held directions repeat in menus

- **Acceptance:** holding up or down (keys, arrows, D-pad, or stick) on the options and controls pages moves through the rows, and holding left or right on a volume, shake, or speed bar keeps stepping it: the first repeat after 0.4 s, then every 0.1 s. Releasing stops at once. A single press still moves exactly one step. The repeat stops at the ends of a bar or list rather than wrapping past them. While a key is being listened for on the controls page, nothing repeats. Every other screen behaves as before.
- **Verify:** unit tests of the repeat timing (one press, a hold through several repeats, release, switching direction, the ends) and of holding through the options and controls pages with keys and with a controller; in the release build inside the nested KWin, a real held key (sent through XTEST to the nested display only) steps the music volume down, and `settings.json` shows the result.

### D. Window-size saving, checked with a real resize

- **Acceptance:** round 8's rule holds with a real window manager: resizing the window by the window manager's own action saves the new size once it has held for a second, and the next launch opens at that size. If it doesn't, the defect is fixed here.
- **Verify:** in the nested KWin, launch the release build with a temporary `XDG_DATA_HOME`, resize its window through a KWin script, wait, close it through KWin, check `settings.json`, relaunch, and read the new window's size. Maximising is tried the same way. This only covers X11 through Xwayland under KWin.

Not in this round: controller button rebinding and rumble (both need a physical controller, and a virtual one is visible to every program on this shared machine), the window's position (Miniquad reports it relative to the window frame on X11, and Wayland doesn't let a program place itself), new weapons or mutations (a design decision), anything about saving runs or records, a close button for the atlas, guardian health, and sounds for off-screen windups or low vitality (they need someone to listen). Owner decisions still open: one-use continue saves and whether the first biome saves, separate records for slowed runs, web hosting, releases and signing, shrinking the art, the trailer, a physical-controller test, the red tint's strength, and guardian health.

## Round 9 results (October 7, 2026)

All four scoped items shipped on `improvements-9`, one commit each. Item D found no defect, so its commit holds the check, its documentation, and the script that made it possible. Every commit passed `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, and `cargo test --locked` (136 tests became 143), with the suite's `XDG_DATA_HOME` and `TMPDIR` pointed at empty folders under `target/` that stayed empty. Every native run this round happened inside a private nested KWin (`kwin_wayland --virtual`, now [`scripts/nested-kwin.sh`](../scripts/nested-kwin.sh)) with a temporary `XDG_DATA_HOME`, so no test window appeared on the shared desktop. `--vertical-capture` still reaches 37 of 37 waypoints in 25.2 s, `--motion-capture` still produces 300 frames (CPU submission mean 1.32 ms, p95 2.39 ms at a load average of about 16–25), and `--ui-gallery` still has 35 fixtures. Compared with a build of `main`, 21 of the 35 fixtures were byte-identical and the rest matched at 81 dB PSNR or better, and 276 of the 300 motion frames were byte-identical, the rest at 62.8 dB or better. Two runs of `main` itself differ the same way, so drawing is unchanged wherever blending is off. The Linux tarball, extracted, ran the gallery. The wasm is 52,611,047 bytes, still 52.6 MB. `~/.local/share/cinderwake` didn't exist before or after the round. Screenshots are in [`media/improvements/round9/`](media/improvements/round9/).

| Item | Commit | How it was verified |
| --- | --- | --- |
| A. Drawing between steps | `c32e21f` | Tests run a hero at full speed through the main loop's stepping at 60, 120 (with a ±0.5 ms wobble), 144, and 165 Hz: blended, its drawn speed varied from frame to frame by under 0.01%; with whole steps only, some frames didn't move at all. Other tests: teleports aren't blended, a newly fired bolt is drawn back along its flight, and a 144 Hz run among the opening's guardians ends in exactly the same state whether or not every frame was blended. The new `--pacing-check` ran four times in the nested KWin (virtual output about 60 Hz, load 20–28): 4.6–55.8% of frames ran other than the usual two steps, which whole-step drawing shows as judder. Captures unchanged (above). **Not seen on a high-refresh monitor:** the desktop display here is 120 Hz but wasn't used, and nobody has looked at it on a 144 Hz monitor. |
| B. Browser loading | `637249b` | Headless Chrome through Playwright: throttled to about 4 MB/s, the bar read 10.7 of 52.6 MB at 3.5 s and 22.0 at 6.6 s, and the title appeared at 16 s with no console errors (`b-web-download-progress.jpg`); a server without the file gave the download message (`b-web-download-failed.jpg`); `--disable-3d-apis` gave the WebGL message (`b-web-no-webgl.jpg`); a normal load reached the title with no errors, also with the final build. The crash notice was checked only with a simulated `WebAssembly.RuntimeError` thrown from the page (`b-web-stopped-notice.jpg`). Real slow networks, compressed hosting, and other browsers weren't checked. |
| C. Held menu directions | `0e246cd` | Tests of the repeat timing and of holding through the options and controls pages with keys and a controller: a short hold is one step, a long one runs a bar to 0% with one cue per change, rows stop at the ends, switches flip once, and nothing moves while a key is being listened for. In the release build in the nested KWin, one second of **A** held through XTEST took the music from 100% to 30%, saved; a build of `main` went to 90% (`c-native-held-key-music-30.jpg`, load about 28). Not tried with a physical controller. |
| D. Window size, real resize | `48b8926` | In the nested KWin, KWin's own scripting resized the window to a 1440 × 830 frame: `[1440, 802]` (the client size) was saved; maximising saved `[1920, 1052]` and restoring saved `[1440, 802]` again; closing through KWin exited with status 0; the next launch opened at 1440 × 802 (`d-native-reopened-1440x802.jpg`). This closes round 8's unverified path for X11 through Xwayland under KWin 6.7. |

Findings for a later round:

- With blending on, particles, floating numbers, and animation frames still advance in whole steps. That's hard to notice next to the camera and characters, but blending particles the same way would finish the job.
- A maximised window is saved at its maximised size and reopens that large but not maximised (seen with a real maximise this round). Restoring the maximised state would need Miniquad to report it.
- The options footer doesn't say that holding repeats; it's the usual behaviour, so it was left alone to keep the fixtures unchanged.
- Window position, controller rumble, and controller button rebinding stay deferred: Miniquad reports the position relative to the window frame on X11, and the controller items need a physical controller (a virtual one is visible to every program on a shared machine).

Still deferred: new content (14) and a smaller web download (15). Owner decisions still open: one-use continue saves and whether the first biome saves, separate records for slowed runs, web hosting, releases and signing, shrinking the art, the trailer, a physical-controller test, the red tint's strength, a sound for off-screen windups and low vitality, and guardian health.

## Round 10 scope

Four items. Rounds 6 to 9 covered the menus, warnings, and smoothness; a review for this round went back to the moment-to-moment feel of a fight and found that the game quietly drops presses. Only the jump is buffered (0.12 s). A dodge pressed while the 0.65 s dodge recovery is still running, a parry during its 0.52 s recovery, a tapped strike during a swing's recovery or the last frames of a dodge, and a tool pressed a moment early all do nothing at all, so a player who presses slightly early, as players do, sees the game ignore them. Nothing on screen explains it either: the HUD shows the four slots' recovery, but not the dodge's or the parry's, and a press on a slot that's still recovering, or on the flask with none left or at full vitality, looks exactly like a key that isn't bound. Interaction prompts don't say what an object needs or gives: the forge always reads "60 copper" whether or not the hero has it, a sealed cache doesn't say how many guardians are still needed, the bellgate doesn't say it will bank the carried embers, and the Regent's locked gate reads like any other. Round 9's last finding closes the round: particles and floating numbers still move in whole steps while everything around them is drawn between steps.

Saving, balance, damage, and content stay as they are. Each item gets the usual checks: `cargo fmt --check`, strict Clippy for native and `wasm32-unknown-unknown`, `cargo test --locked`, the capture modes, and screenshots in [`media/improvements/round10/`](media/improvements/round10/). Every native run happens inside a private nested KWin ([`scripts/nested-kwin.sh`](../scripts/nested-kwin.sh)) with a temporary `XDG_DATA_HOME`, and the real save folder is checked before and after.

### A. Presses made a moment early still happen

- **Acceptance:** a press of dodge, parry, fire vessel, or arc snare, or a fresh press of strike or glassbolt, made up to 0.15 s before that action is ready (its recovery is ending, or a dodge or flask is still running) happens as soon as it's ready, once. A press made earlier than that is dropped, as now. Holding strike or glassbolt repeats exactly as before, and releasing never adds an extra swing. Jump keeps its own buffer. Cooldowns, damage, and timings don't change. Practice, capture, and demo scripts run through the same rule.
- **Verify:** unit tests driving `Game::tick` with real step timing: a dodge, parry, tool, or tapped strike pressed 0.1 s early happens when ready and only once; pressed 0.3 s early, it doesn't; a held strike swings at the same moments as on `main`; a tap during a dodge's last 0.1 s swings as it ends. The existing balance and stun-lock duels still pass with their thresholds. `--motion-capture` and `--vertical-capture` still finish (the motion script's presses may now land, which the report states).

### B. The HUD shows why a press did nothing

- **Acceptance:** the dodge and parry badges dim and fill while recovering, like the four slots. When a press can't be honoured even after the buffer (a slot still recovering, the flask with none left or at full vitality), that slot's frame flashes briefly in the refusal colour and, for the flask, says **EMPTY** or **FULL** for a moment. With **Reduce flashes** on it shows as a steady outline instead of a flash. No sound is added (sound choices need someone to listen).
- **Verify:** unit tests that each refused press marks exactly its own slot, that a buffered press that later happens doesn't, and that the mark fades; a new `--ui-gallery` fixture with the dodge recovering and a refused fire vessel and flask; the other fixtures unchanged against a build of `main`.

### C. Interaction prompts say what they need and give

- **Acceptance:** the forge's prompt shows the copper carried against the 60 needed and is dimmed when it's short; a sealed cache shows guardians felled out of 8 (or that the Crown Rune opens it); the bellgate says how many carried embers it will bank; the Regent's gate says the Regent holds it shut. Pressing interact does exactly what it did before. Other prompts are unchanged.
- **Verify:** unit tests of the prompt text and its dimmed state for each case; a new `--ui-gallery` fixture with a short-funded forge prompt; `--start-at forge`, `cache`, and `gate` checked in the release build inside the nested KWin with real key presses through XTEST, with a screenshot.

### D. Particles and floating numbers drawn between steps

- **Acceptance:** during play, with drawing between steps on, particles and floating damage numbers are drawn the leftover fraction of a step back along their motion, like bolts, and put back after the frame, so the simulation never sees a blended value. Settled debris never sinks below its floor. Menus, staged views, and capture modes draw whole steps, so their output matches `main`'s within the captures' own run-to-run variation.
- **Verify:** unit tests that a moving particle and a rising number are drawn an even distance per frame at 144 Hz, that showing and restoring leaves particles and numbers exactly as they were, that settled shards stay on their floor, and that a blended 144 Hz run ends in the same state as an unblended one; `--ui-gallery` fixtures compared against a build of `main`.

Not in this round: controller button rebinding and rumble (need a physical controller), the window's position, a maximised window coming back maximised (Miniquad doesn't report it), new weapons or mutations, anything about saving runs or records, guardian health, sounds for refused presses, off-screen windups, or low vitality (they need someone to listen), and a smaller web download (shrinking the art is the owner's call). Owner decisions still open: one-use continue saves and whether the first biome saves, separate records for slowed runs, web hosting, releases and signing, shrinking the art, the trailer, a physical-controller test, the red tint's strength, sounds for off-screen windups and low vitality, and guardian health.
