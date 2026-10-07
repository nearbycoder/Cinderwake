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
