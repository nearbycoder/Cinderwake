# Showcase media

Everything here is drawn by Cinderwake itself at graphics fidelity **Ultra**, recorded at 1920 × 1080 inside a private nested KWin by [`scripts/record-trailer.sh`](../../scripts/record-trailer.sh) and edited by [`scripts/trailer.py`](../../scripts/trailer.py). The captures run at fixed simulated time, one frame per step of the script, so Ultra keeps every frame however long each takes to draw.

| File | What it shows |
| --- | --- |
| `cinderwake-demo.mp4` | The 43-second trailer, 1920 × 1080 at 30 fps, H.264 and AAC, loudness −16 LUFS. Sections: the title screen; 11 s of the `--motion-capture` fight (scripted input, live physics and combat); three excerpts of the `--vertical-capture` route with guardians and hazards removed; 2.2 s of each biome from the `--environment-tour` camera tour; the staged Regent arena; the reliquary, Keeper, and options screens from `--ui-gallery`; and an end card over a dimmed frame of the Crown. |
| `demo.gif` | Six seconds of the same fight, without captions or sound, 560 pixels wide at 12 fps. |
| `combat.png` | The scripted fight: a guardian winding up with its warning mark and reach shown. |
| `rooftops.png` | The upper galleries, from the route's ordinary movement inputs. |
| `undercroft.png` | The undercroft on the same route. |
| `foundry.png` | The Ember Foundry from the staged camera tour. |
| `crown.png` | The staged Crown of the Machine view with the Brass Regent (`--gallery`). |
| `atlas.png` | The staged atlas fixture with every kind of object marked, fully surveyed. |

The trailer's soundtrack is the game's own: each section's biome loop (the hearth loop for the title and menus) at the default music volume and, under the fight and the route, the sound effects those frames played at the default effects volume, read from the `cues.txt` the `--trailer` captures write. There is no narration. Captions, fades, and the end card's text are the only things added in the edit; there are no concept-art substitutes or simulated interface overlays. The screenshots are scaled to 1280 × 720 and reduced to 256 colours. Capture modes do not read or write player progression.

See [the engine guide](../ENGINE.md#recording-the-trailer) for the commands. Source artwork and ImageGen prompts live in [`assets/`](../../assets/); the footage uses those assets rendered by the Rust engine.
