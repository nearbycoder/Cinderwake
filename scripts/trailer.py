#!/usr/bin/env python3
"""Edits the README trailer and teaser from the game's own captures.

Usage: python3 scripts/trailer.py <captures> <work>

<captures> holds the output of `--motion-capture`, `--vertical-capture`, and
`--environment-tour` run with `--trailer` (30 frames a second, no labels, and
a `cues.txt` of the sound cues each frame played), plus `--ui-gallery` and
`--gallery` stills, all at the same window size. scripts/record-trailer.sh
makes them. <work> takes the intermediate clips, and must not be in git.

Every picture is a frame the game drew; the edit only cuts, fades, and adds
captions. The soundtrack is the game's own: its biome loops at the default
music volume and, under the scripted captures, the effects those frames
played at the default effects volume, mixed from the WAV files in assets/.
Writes docs/media/cinderwake-demo.mp4, docs/media/demo.gif, and the README's
screenshots in docs/media/.
"""
import array
import json
import math
import os
import re
import subprocess
import sys
import wave
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FONT = ROOT / "assets/title.ttf"
FPS = 30  # `--trailer` saves every second 60 Hz frame
GAME_FPS = 60
RATE = 22050  # the game's audio files
MUSIC_GAIN = 0.5  # Settings::music_gain at the default volume
EFFECTS_GAIN = 0.35  # Settings::effects_gain at the default volume
FADE = 0.25  # fade through black between sections, in seconds
CUT = 0.12  # quicker fade between excerpts of one capture
SUB_SCRIPTED = "Scripted input  ·  live physics and combat"
SUB_ROUTE = "Ordinary movement inputs  ·  guardians removed to show the route"
SUB_TOUR = "Graphics fidelity Ultra  ·  staged camera tour"
SUB_STILL = "Game screens  ·  keyboard, mouse, or controller"

# Each clip: (kind, source, first and last game frame or seconds for a
# still, music track, captions as (from, to, line, sub-line) in clip seconds,
# fade at the start and end). A "menu" still captions below its panel.
CLIPS = [
    ("still", "ui-00-title.png", 2.6, "hearth", [], (0.4, FADE)),
    ("seq", "motion", (50, 710), "aqueduct",
     [(0.2, 2.6, "Drink a flask; the slot fills as you heal", SUB_SCRIPTED),
      (2.8, 5.6, "Double jump, slam, dodge, and chain strikes", SUB_SCRIPTED),
      (5.8, 8.4, "Glassbolts, fire vessels, and arc snares", SUB_SCRIPTED),
      (8.6, 11.0, "Guardians mark every attack before it lands", SUB_SCRIPTED)], (FADE, FADE)),
    ("seq", "vertical", (150, 420), "aqueduct",
     [(0.2, 4.5, "Climb into the upper galleries", SUB_ROUTE)], (FADE, CUT)),
    ("seq", "vertical", (900, 1110), "aqueduct",
     [(0.1, 3.5, "Drop into the undercroft; the camera leads the fall", SUB_ROUTE)], (CUT, CUT)),
    ("seq", "vertical", (1392, 1512), "aqueduct",
     [(0.1, 2.0, "Three elevations, one connected route to the bellgate", SUB_ROUTE)], (CUT, FADE)),
    ("seq", "tour", (120, 252), "aqueduct",
     [(0.1, 2.2, "The Drowned Aqueduct", SUB_TOUR)], (FADE, CUT)),
    ("seq", "tour", (480, 612), "conservatory",
     [(0.1, 2.2, "The Glassroot Conservatory", SUB_TOUR)], (CUT, CUT)),
    ("seq", "tour", (840, 972), "foundry",
     [(0.1, 2.2, "The Ember Foundry: forges and lamps light the scene", SUB_TOUR)], (CUT, CUT)),
    ("seq", "tour", (1200, 1332), "crown",
     [(0.1, 2.2, "The Crown of the Machine", SUB_TOUR)], (CUT, FADE)),
    ("still", "ui-10-crown-boss.png", 1.8, "crown",
     [(0.1, 1.8, "Silence the Brass Regent", "A staged view of the final arena")], (FADE, CUT)),
    ("menu", "ui-14-reliquary.png", 1.8, "hearth",
     [(0.1, 1.8, "Take or leave each reliquary's weapon", SUB_STILL)], (CUT, CUT)),
    ("menu", "ui-06-camp-conservatory.png", 1.8, "hearth",
     [(0.1, 1.8, "Bank embers for lasting upgrades, then choose a branch", SUB_STILL)], (CUT, CUT)),
    ("menu", "ui-41-options-fidelity.png", 1.8, "hearth",
     [(0.1, 1.8, "Graphics fidelity from Low to Ultra, and more options", SUB_STILL)], (CUT, FADE)),
    ("end", "biome-3.png", 3.4, "hearth", [], (FADE, 0.6)),
]
END_LINES = [
    ("CINDERWAKE", 120, 340),
    ("A clockwork action roguelite, built in Rust", 48, 500),
    ("Free and open source  ·  MIT licence", 40, 590),
    ("github.com/nearbycoder/Cinderwake", 40, 650),
]
# README screenshots: file name, then a captured frame (Ultra, scaled to
# 1280 × 720 and reduced to 256 colours to stay well under 1 MB each).
SHOTS = [
    ("combat.png", "motion/frame-00700.png"),
    ("rooftops.png", "vertical/frame-00420.png"),
    ("undercroft.png", "vertical/frame-01050.png"),
    ("foundry.png", "tour/frame-00900.png"),
    ("crown.png", "biome-3.png"),
    ("atlas.png", "ui-37-atlas-marks.png"),
]
TEASER = ("motion", (420, 780))  # game frames, without captions
TEASER_WIDTH = 560  # the README shows it 560 wide
TEASER_FPS = 12  # keeps the GIF under 10 MB


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd)[:240], flush=True)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


def clip_seconds(clip):
    kind, _, span = clip[0], clip[1], clip[2]
    if kind == "seq":
        return (span[1] - span[0]) / GAME_FPS
    return span


def caption_filters(work, index, captions, height, low):
    """drawtext filters for one clip; text goes through files to avoid escaping.
    Captions sit above the HUD's lower bar, or over it (`low`) below a menu."""
    out = []
    scale = height / 1080
    top = 978 if low else 846
    for n, (t0, t1, line, sub) in enumerate(captions):
        alpha = f"max(0\\,min(1\\,min((t-{t0})/0.2\\,({t1}-t)/0.2)))"
        for k, (text, size, y) in enumerate([(line, 50, top), (sub, 27, top + 60)]):
            path = work / f"caption-{index:02}-{n}-{k}.txt"
            path.write_text(text)
            out.append(
                f"drawtext=fontfile={FONT}:textfile={path}:fontsize={round(size * scale)}"
                f":fontcolor={'0xf2d7a2' if k == 0 else '0xc9d6d3'}"
                f":x=(w-text_w)/2:y={round(y * scale)}"
                f":box=1:boxcolor=0x0b0f1a@0.62:boxborderw={round((14 if k == 0 else 8) * scale)}"
                f":shadowcolor=0x000000@0.8:shadowx=2:shadowy=2"
                f":alpha='{alpha}':enable='between(t\\,{t0}\\,{t1})'"
            )
    return out


def render_clip(captures, work, index, clip, size):
    kind, source, span, _, captions, (fade_in, fade_out) = clip
    seconds = clip_seconds(clip)
    height = size[1]
    out = work / f"clip-{index:02}.mkv"
    filters = []
    if kind == "seq":
        frames = work / f"frames-{index:02}"
        frames.mkdir(exist_ok=True)
        for old in frames.iterdir():
            old.unlink()
        for n, f in enumerate(range(span[0], span[1], GAME_FPS // FPS)):
            src = captures / source / f"frame-{f:05}.png"
            if not src.exists():
                sys.exit(f"missing {src}")
            (frames / f"{n:05}.png").symlink_to(src)
        inputs = ["-framerate", FPS, "-i", frames / "%05d.png"]
    else:
        inputs = ["-loop", 1, "-framerate", FPS, "-t", seconds, "-i", captures / source]
    filters.append("format=rgb24")
    if kind == "end":
        filters.append("colorchannelmixer=rr=0.38:gg=0.38:bb=0.42")
        for n, (text, px, y) in enumerate(END_LINES):
            path = work / f"end-{n}.txt"
            path.write_text(text)
            filters.append(
                f"drawtext=fontfile={FONT}:textfile={path}:fontsize={round(px * height / 1080)}"
                f":fontcolor=0xf2d7a2:x=(w-text_w)/2:y={round(y * height / 1080)}"
                f":shadowcolor=0x000000@0.8:shadowx=3:shadowy=3"
            )
    filters += caption_filters(work, index, captions, height, kind == "menu")
    if fade_in:
        filters.append(f"fade=t=in:st=0:d={fade_in}")
    if fade_out:
        filters.append(f"fade=t=out:st={seconds - fade_out:.3f}:d={fade_out}")
    script = work / f"clip-{index:02}.filter"
    script.write_text(",\n".join(filters))
    run(["ffmpeg", "-v", "error", "-y", *inputs, "-/vf", script,
         "-frames:v", round(seconds * FPS), "-c:v", "ffv1", "-pix_fmt", "bgr0", out])
    return out


def read_wav(path):
    with wave.open(str(path)) as w:
        assert w.getframerate() == RATE and w.getnchannels() == 1 and w.getsampwidth() == 2, path
        data = array.array("h", w.readframes(w.getnframes()))
    return [s / 32768 for s in data]


def mix_audio(captures, work, total):
    """Music loops in sequence with short crossfades, plus logged effects."""
    n = round(total * RATE)
    mix = [0.0] * n
    loops = {}
    # Music: each run of clips with the same track plays its loop through;
    # a new track starts from the top, as the game's crossfade does.
    runs, t = [], 0.0
    for clip in CLIPS:
        seconds = clip_seconds(clip)
        if runs and runs[-1][0] == clip[3]:
            runs[-1][2] = t + seconds
        else:
            runs.append([clip[3], t, t + seconds])
        t += seconds
    xf = 0.5
    for i, (track, start, end) in enumerate(runs):
        loop = loops.setdefault(track, read_wav(ROOT / f"assets/music/{track}.wav"))
        a = max(0.0, start - (xf / 2 if i else 0))
        b = min(total, end + (xf / 2 if i + 1 < len(runs) else 0))
        for s in range(round(a * RATE), min(n, round(b * RATE))):
            x = s / RATE
            gain = 1.0
            if i and x < a + xf:
                gain = math.sin((x - a) / xf * math.pi / 2)
            if i + 1 < len(runs) and x > b - xf:
                gain = min(gain, math.sin((b - x) / xf * math.pi / 2))
            if i + 1 == len(runs) and x > total - 1.6:
                gain = min(gain, max(0.0, (total - x) / 1.6))
            if i == 0 and x < 0.4:
                gain = min(gain, x / 0.4)
            mix[s] += loop[(s - round(a * RATE)) % len(loop)] * gain * MUSIC_GAIN
    # Effects: the cues each captured frame played, at that frame's time.
    effects, placed, t = {}, 0, 0.0
    for clip in CLIPS:
        seconds = clip_seconds(clip)
        if clip[0] == "seq":
            first, last = clip[2]
            log = captures / clip[1] / "cues.txt"
            for line in log.read_text().splitlines():
                frame, cue = line.split()
                frame = int(frame)
                if first <= frame < last:
                    sound = effects.setdefault(cue, read_wav(ROOT / f"assets/{cue}.wav"))
                    at = round((t + (frame - first) / GAME_FPS) * RATE)
                    for k, v in enumerate(sound[: max(0, n - at)]):
                        mix[at + k] += v * EFFECTS_GAIN
                    placed += 1
        t += seconds
    print(f"audio: {len(runs)} music runs, {placed} effects", flush=True)
    path = work / "mix.wav"
    peak = max(1.0, max(abs(v) for v in mix))
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(array.array("h", (round(v / peak * 32767) for v in mix)).tobytes())
    return path


def loudness_filter(wav):
    """Two-pass EBU R128 normalisation of the stereo mix to -16 LUFS, true
    peak -1.5 dB. (Stereo first: a mono file measures 3 dB quieter.)"""
    target = "I=-16:TP=-1.5:LRA=11"
    stereo = "aresample=48000,pan=stereo|c0=c0|c1=c0"
    out = subprocess.run(
        ["ffmpeg", "-hide_banner", "-i", str(wav), "-af",
         f"{stereo},loudnorm={target}:print_format=json", "-f", "null", "-"],
        capture_output=True, text=True, check=True).stderr
    m = json.loads(re.search(r"\{[^{}]*\"input_i\"[^{}]*\}", out).group(0))
    return (f"{stereo},loudnorm={target}:measured_I={m['input_i']}"
            f":measured_TP={m['input_tp']}:measured_LRA={m['input_lra']}"
            f":measured_thresh={m['input_thresh']}:offset={m['target_offset']}:linear=true,"
            "aresample=48000")


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    captures, work = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
    work.mkdir(parents=True, exist_ok=True)
    probe = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "stream=width,height", "-of", "csv=p=0",
         str(captures / "ui-00-title.png")], capture_output=True, text=True, check=True)
    size = tuple(int(v) for v in probe.stdout.strip().split(","))
    clips = [render_clip(captures, work, i, c, size) for i, c in enumerate(CLIPS)]
    total = sum(clip_seconds(c) for c in CLIPS)
    listing = work / "clips.txt"
    listing.write_text("".join(f"file '{c}'\n" for c in clips))
    audio = mix_audio(captures, work, total)
    media = ROOT / "docs/media"
    mp4 = media / "cinderwake-demo.mp4"
    common = ["-f", "concat", "-safe", "0", "-i", listing, "-i", audio,
              "-map", "0:v", "-map", "1:a", "-c:v", "libx264", "-preset", "slow",
              "-b:v", os.environ.get("TRAILER_BITRATE", "7000k"), "-pix_fmt", "yuv420p",
              "-profile:v", "high", "-r", FPS]
    passlog = work / "x264"
    run(["nice", "-n", "10", "ffmpeg", "-v", "error", "-y", *common, "-pass", 1,
         "-passlogfile", passlog, "-an", "-f", "null", "/dev/null"])
    run(["nice", "-n", "10", "ffmpeg", "-v", "error", "-y", *common, "-pass", 2,
         "-passlogfile", passlog, "-af", loudness_filter(audio), "-c:a", "aac", "-b:a", "192k",
         "-ar", 48000, "-movflags", "+faststart", "-t", f"{total:.3f}", mp4])
    # The teaser: a caption-free stretch of the scripted fight.
    source, (first, last) = TEASER
    frames = work / "teaser-frames"
    frames.mkdir(exist_ok=True)
    for old in frames.iterdir():
        old.unlink()
    for n, f in enumerate(range(first, last, GAME_FPS // FPS)):
        (frames / f"{n:05}.png").symlink_to(captures / source / f"frame-{f:05}.png")
    gif = media / "demo.gif"
    run(["nice", "-n", "10", "ffmpeg", "-v", "error", "-y", "-framerate", FPS, "-i",
         frames / "%05d.png", "-vf",
         f"fps={TEASER_FPS},scale={TEASER_WIDTH}:-1:flags=lanczos,split[a][b];"
         "[a]palettegen=max_colors=160:stats_mode=diff[p];"
         "[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle",
         "-loop", 0, gif])
    for name, source in SHOTS:
        run(["ffmpeg", "-v", "error", "-y", "-i", captures / source, "-vf",
             "scale=1280:720:flags=lanczos,split[a][b];[a]palettegen=max_colors=256:stats_mode=single[p];"
             "[b][p]paletteuse=dither=floyd_steinberg", media / name])
    print(f"wrote {mp4} ({total:.2f} s), {gif}, and {len(SHOTS)} screenshots")


if __name__ == "__main__":
    main()
