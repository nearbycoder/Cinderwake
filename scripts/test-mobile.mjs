#!/usr/bin/env node
// Measures the browser build on phone and tablet profiles and drives a short
// session through its on-screen controls with touch events, in headless
// WebKit (iPhone 15, iPad Pro 11) and Chromium (Pixel 7) through Playwright.
// Also confirms the controls stay hidden in desktop Chromium. Logs, a
// summary.json, and screenshots go to target/mobile-test/<profile>/.
//
//   node scripts/test-mobile.mjs http://127.0.0.1:8080/Cinderwake/ [iphone ipad pixel desktop] [--measure-only]
//
// Playwright isn't a dependency of this repository: PLAYWRIGHT_CORE names an
// installed playwright-core (default: the one in ~/Sites/blog), and
// WEBKIT_PATH a Playwright WebKit (default ~/.cache/webkit-libs/webkit-2359).
// Chromium is found as in check-pages.mjs.
//
// Memory is read from Linux's /proc for the browser processes this script
// started: the peak resident size (VmHWM) of the page's web content process,
// which is what iOS limits per tab. The page also reports the WebAssembly
// memory and the bytes it handed to WebGL for textures, render targets, and
// buffers. Headless browsers draw in software here, so frame rates are only
// comparisons between builds, not what a phone does.
import { createRequire } from "node:module";
import { appendFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { findChromium, looksDrawn, screenStats } from "./check-pages.mjs";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const { webkit, chromium, devices } = require(process.env.PLAYWRIGHT_CORE
  || join(homedir(), "Sites", "blog", "node_modules", "playwright-core"));
const webkitPath = process.env.WEBKIT_PATH || join(homedir(), ".cache", "webkit-libs", "webkit-2359", "pw_run.sh");

const args = process.argv.slice(2);
const measureOnly = args.includes("--measure-only");
const outName = (() => {
  const i = args.indexOf("--out");
  return i >= 0 ? args.splice(i, 2)[1] : "mobile-test";
})();
const [url, ...names] = args.filter((a) => !a.startsWith("--"));
if (!url) {
  console.error("usage: node scripts/test-mobile.mjs <url> [iphone ipad pixel desktop] [--measure-only] [--out name]");
  process.exit(2);
}
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

const PROFILES = {
  iphone: { engine: "webkit", device: "iPhone 15 landscape", portrait: "iPhone 15" },
  ipad: { engine: "webkit", device: "iPad Pro 11 landscape", portrait: "iPad Pro 11" },
  pixel: { engine: "chromium", device: "Pixel 7 landscape", portrait: "Pixel 7" },
  desktop: { engine: "chromium", device: null },
};

// Runs in the page before its scripts: counts frames and the bytes given to
// WebGL (textures, renderbuffers, buffers), now and at their peak.
function instrument() {
  const gpu = { now: 0, peak: 0, textures: 0, peakTextures: 0, sizes: new Map() };
  window.__gpu = gpu;
  const bump = (object, bytes) => {
    if (!object) return;
    gpu.now += bytes - (gpu.sizes.get(object) || 0);
    gpu.sizes.set(object, bytes);
    gpu.peak = Math.max(gpu.peak, gpu.now);
    gpu.textures = gpu.sizes.size;
    gpu.peakTextures = Math.max(gpu.peakTextures, gpu.textures);
  };
  const forget = (object) => {
    if (!object || !gpu.sizes.has(object)) return;
    gpu.now -= gpu.sizes.get(object);
    gpu.sizes.delete(object);
  };
  const perPixel = (format, type) => {
    if (type === 0x1401 /* UNSIGNED_BYTE */) {
      return { 0x1906: 1, 0x1909: 1, 0x190a: 2, 0x1907: 3, 0x1908: 4 }[format] || 4;
    }
    return 4;
  };
  for (const Context of [window.WebGLRenderingContext, window.WebGL2RenderingContext]) {
    if (!Context) continue;
    const proto = Context.prototype;
    const bound = new WeakMap();
    const state = (gl) => {
      if (!bound.has(gl)) bound.set(gl, { unit: 0, units: [], buffers: {}, renderbuffer: null });
      return bound.get(gl);
    };
    const wrap = (name, after) => {
      const real = proto[name];
      if (!real) return;
      proto[name] = function (...a) {
        const result = real.apply(this, a);
        try { after.call(this, a, result); } catch (_) { /* measuring only */ }
        return result;
      };
    };
    wrap("activeTexture", function ([unit]) { state(this).unit = unit; });
    wrap("bindTexture", function ([target, texture]) { (state(this).units[state(this).unit] ||= {})[target] = texture; });
    wrap("deleteTexture", function ([texture]) { forget(texture); });
    const texture = (gl, target) => (state(gl).units[state(gl).unit] || {})[target === 0x0de1 ? target : 0x8513] || null;
    wrap("texImage2D", function (a) {
      const [target, level] = a;
      if (level !== 0) return;
      let w, h, format, type;
      if (a.length >= 8) [, , , w, h, , format, type] = a;
      else { [, , , format, type] = a; w = a[5].width; h = a[5].height; }
      bump(texture(this, target), w * h * perPixel(format, type));
    });
    wrap("texStorage2D", function ([target, , , w, h]) { bump(texture(this, target), w * h * 4); });
    wrap("bindBuffer", function ([target, buffer]) { state(this).buffers[target] = buffer; });
    wrap("deleteBuffer", function ([buffer]) { forget(buffer); });
    wrap("bufferData", function ([target, data]) {
      bump(state(this).buffers[target], typeof data === "number" ? data : data ? data.byteLength : 0);
    });
    wrap("bindRenderbuffer", function ([, rb]) { state(this).renderbuffer = rb; });
    wrap("deleteRenderbuffer", function ([rb]) { forget(rb); });
    wrap("renderbufferStorage", function ([, , w, h]) { bump(state(this).renderbuffer, w * h * 4); });
  }
  window.__frames = 0;
  const tick = () => { window.__frames++; requestAnimationFrame(tick); };
  requestAnimationFrame(tick);
}

// Every process descended from this one, with its command and memory.
function descendants() {
  const procs = [];
  for (const pid of readdirSync("/proc").filter((d) => /^\d+$/.test(d))) {
    try {
      const status = readFileSync(`/proc/${pid}/status`, "utf8");
      const ppid = Number(status.match(/PPid:\s+(\d+)/)[1]);
      const kb = (key) => Number((status.match(new RegExp(`${key}:\\s+(\\d+)`)) || [0, 0])[1]);
      procs.push({ pid: Number(pid), ppid, rss: kb("VmRSS"), hwm: kb("VmHWM"),
        cmd: readFileSync(`/proc/${pid}/cmdline`, "utf8").replace(/\0/g, " ").slice(0, 300) });
    } catch (_) { /* gone */ }
  }
  const mine = new Set([process.pid]);
  let grew = true;
  while (grew) {
    grew = false;
    for (const p of procs) if (!mine.has(p.pid) && mine.has(p.ppid)) { mine.add(p.pid); grew = true; }
  }
  return procs.filter((p) => p.pid !== process.pid && mine.has(p.pid));
}
// The process that runs the page's script and WebGL.
const isContent = (p) => /WebProcess|--type=renderer/.test(p.cmd) && !/--extension-process/.test(p.cmd);
const isGpu = (p) => /--type=gpu-process|WebKitGPUProcess/.test(p.cmd);

async function session(name) {
  const profile = PROFILES[name];
  const out = join(repo, "target", outName, name);
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  const logFile = join(out, "session.log");
  writeFileSync(logFile, "");
  let lines = 0;
  const log = (text) => {
    console.log(`[${name}] ${text}`);
    if (++lines <= 3000) appendFileSync(logFile, `${text}\n`);
  };
  const results = [];
  const result = (ok, what) => { results.push({ ok, what }); log(`${ok ? "PASS" : "FAIL"} ${what}`); };
  const summary = { profile: name, url };
  const errors = [];

  const browser = profile.engine === "webkit"
    ? await webkit.launch({ headless: true, executablePath: webkitPath })
    : await chromium.launch({ headless: true, executablePath: findChromium(),
      args: ["--enable-unsafe-swiftshader", "--autoplay-policy=document-user-activation-required"] });
  summary.browser = `${profile.engine} ${browser.version()}`;
  log(`browser: ${summary.browser}; device: ${profile.device || "desktop 1280x720"}`);
  // Peak memory, sampled through the session.
  const peaks = { content: 0, gpu: 0, total: 0, contentCmd: "" };
  let sampling = true;
  const sampler = (async () => {
    while (sampling) {
      const procs = descendants();
      let total = 0;
      for (const p of procs) {
        total += p.rss;
        if (isContent(p) && p.hwm > peaks.content) { peaks.content = p.hwm; peaks.contentCmd = p.cmd.split(" ")[0]; }
        if (isGpu(p)) peaks.gpu = Math.max(peaks.gpu, p.hwm);
      }
      peaks.total = Math.max(peaks.total, total);
      await pause(200);
    }
  })();
  try {
    const options = profile.device ? { ...devices[profile.device] } : { viewport: { width: 1280, height: 720 } };
    const context = await browser.newContext(options);
    const page = await context.newPage();
    await page.addInitScript(instrument);
    page.on("console", (m) => {
      if (m.text() === "fix") return; // quad-snd's audio unlock
      log(`console.${m.type()}: ${m.text()}`);
      if (m.type() === "error") errors.push(m.text());
    });
    page.on("pageerror", (e) => { log(`pageerror: ${e.message}`); errors.push(e.message); });
    page.on("crash", () => { log("page crashed"); errors.push("page crashed"); });
    let downloaded = 0;
    page.on("requestfinished", async (r) => {
      try { downloaded += (await r.sizes()).responseBodySize; } catch (_) { /* gone */ }
    });
    const shot = async (label) => {
      const png = await page.screenshot({ type: "png", timeout: 120000 });
      writeFileSync(join(out, `${label}.png`), png);
      return png;
    };
    const started = Date.now();
    await page.goto(url, { waitUntil: "load" });
    const touchEnv = await page.evaluate(() => ({
      coarse: matchMedia("(pointer: coarse)").matches,
      fine: matchMedia("(any-pointer: fine)").matches,
      touchPoints: navigator.maxTouchPoints,
      webgl2: !!document.createElement("canvas").getContext("webgl2"),
      webgpu: !!navigator.gpu,
      dpr: devicePixelRatio,
      viewport: `${innerWidth}x${innerHeight}`,
    }));
    summary.environment = touchEnv;
    log(`environment: ${JSON.stringify(touchEnv)}`);
    let state = "";
    while (Date.now() - started < 300000) {
      state = await page.evaluate(() => document.documentElement.dataset.game || "");
      if (state !== "loading") break;
      await pause(500);
    }
    summary.secondsToTitle = (Date.now() - started) / 1000;
    result(state === "running", `reached the title (state "${state}" after ${summary.secondsToTitle.toFixed(1)} s, load ${(await loadAverage())})`);
    await pause(2500);
    const title = await shot("1-title");
    const stats = screenStats(title);
    result(looksDrawn(stats), `the title is drawn (${stats.colours} colours, ${(100 * stats.lit).toFixed(0)}% lit)`);
    const memory = async () => page.evaluate(() => ({
      wasm: window.wasm_memory ? window.wasm_memory.buffer.byteLength : null,
      gpuNow: window.__gpu.now, gpuPeak: window.__gpu.peak, gpuObjects: window.__gpu.textures,
      jsHeap: performance.memory ? performance.memory.usedJSHeapSize : null,
      canvas: `${document.getElementById("glcanvas").width}x${document.getElementById("glcanvas").height}`,
    }));
    summary.atTitle = await memory();
    log(`memory at the title: ${JSON.stringify(summary.atTitle)}`);
    const fps = async (seconds) => {
      const before = await page.evaluate(() => window.__frames);
      await pause(seconds * 1000);
      return ((await page.evaluate(() => window.__frames)) - before) / seconds;
    };
    summary.titleFps = await fps(4);
    log(`title frame rate: ${summary.titleFps.toFixed(1)} fps (software rendering, load ${await loadAverage()})`);
    const controls = async () => page.evaluate(() => {
      const touch = document.getElementById("touch");
      return touch ? { shown: !touch.hidden && getComputedStyle(touch).display !== "none", mode: touch.dataset.mode || "" } : null;
    });
    summary.controlsAtTitle = await controls();
    log(`on-screen controls at the title: ${JSON.stringify(summary.controlsAtTitle)}`);

    if (name === "desktop") {
      result(!summary.controlsAtTitle || !summary.controlsAtTitle.shown, "the on-screen controls are hidden on the desktop");
      await page.mouse.move(640, 300);
      await page.keyboard.press("Enter");
      await pause(1500);
      const after = await controls();
      result(!after || !after.shown, "they stay hidden after keys and the mouse are used");
    } else if (!measureOnly) {
      await drive(page, context, profile, { log, result, shot, controls, fps, summary, out });
    } else {
      // What a touch does to today's build: tap the middle of the screen.
      const vp = page.viewportSize();
      await page.touchscreen.tap(vp.width / 2, vp.height * 0.62);
      await pause(1500);
      await shot("2-after-tap");
      summary.afterTap = await page.evaluate(() => document.documentElement.dataset.game);
    }
    summary.atEnd = await memory();
    log(`memory at the end: ${JSON.stringify(summary.atEnd)}`);
    summary.downloadedMB = +(downloaded / 1e6).toFixed(1);
    summary.state = await page.evaluate(() => document.documentElement.dataset.game);
    result(summary.state === "running", `the page still reports running (${summary.state})`);
    await context.close();
  } catch (error) {
    result(false, `session error: ${error.message.split("\n")[0]}`);
  } finally {
    sampling = false;
    await sampler;
    await browser.close().catch(() => {});
  }
  summary.peakContentMB = Math.round(peaks.content / 1024);
  summary.peakGpuProcessMB = Math.round(peaks.gpu / 1024);
  summary.peakAllProcessesMB = Math.round(peaks.total / 1024);
  log(`peak resident memory: content process ${summary.peakContentMB} MB (${peaks.contentCmd}), GPU process ${summary.peakGpuProcessMB} MB, all browser processes together ${summary.peakAllProcessesMB} MB`);
  log(`downloaded ${summary.downloadedMB} MB`);
  for (const e of errors) result(false, `no errors: ${e}`);
  summary.results = results;
  writeFileSync(join(out, "summary.json"), JSON.stringify(summary, null, 1));
  return results;
}

async function loadAverage() {
  return readFileSync("/proc/loadavg", "utf8").split(" ")[0];
}

// The page's own record of what the game saw (web/cinderwake-touch.js).
const game = (page) => page.evaluate(() => window.cinderwakeTouch ? window.cinderwakeTouch.report() : null);

// A touch held on an element: real touch events in Chromium (DevTools
// protocol, several fingers at once). Playwright's WebKit can only tap, and
// this WebKit has no Touch constructor, so held touches there are events of
// the same types carrying plain touch lists, through the same listeners.
function synthetic({ type, points, changed }) {
  const make = (p) => ({ identifier: p.id, target: document.elementFromPoint(p.x, p.y) || document.body,
    clientX: p.x, clientY: p.y, pageX: p.x, pageY: p.y });
  const changedTouches = changed.map(make);
  const event = new Event(type, { bubbles: true, cancelable: true, composed: true });
  Object.defineProperties(event, {
    changedTouches: { value: changedTouches },
    touches: { value: points.map(make) },
    targetTouches: { value: points.map(make) },
  });
  changedTouches[0].target.dispatchEvent(event);
}
async function fingers(page, context, engine) {
  let cdp = null;
  if (engine === "chromium") cdp = await context.newCDPSession(page);
  const held = new Map();
  let next = 1;
  const centre = async (selector, dx = 0, dy = 0) => {
    const box = await page.locator(selector).boundingBox();
    if (!box) throw new Error(`${selector} isn't on screen`);
    return { x: box.x + box.width / 2 + dx * box.width / 2, y: box.y + box.height / 2 + dy * box.height / 2 };
  };
  const send = async (type, changed) => {
    const points = [...held.values()];
    if (cdp) {
      await cdp.send("Input.dispatchTouchEvent", {
        type, touchPoints: (type === "touchEnd" || type === "touchCancel") ? points : points.map((p) => ({ x: p.x, y: p.y, id: p.id })),
      });
    } else {
      await page.evaluate(synthetic, { type: { touchStart: "touchstart", touchMove: "touchmove", touchEnd: "touchend" }[type], points, changed });
    }
  };
  return {
    async down(selector, dx = 0, dy = 0) {
      const p = { id: next++, ...(await centre(selector, dx, dy)) };
      held.set(selector, p);
      await send("touchStart", [p]);
      return p;
    },
    async move(selector, dx, dy) {
      const p = held.get(selector);
      Object.assign(p, await centre(selector, dx, dy));
      await send("touchMove", [p]);
    },
    async up(selector) {
      const p = held.get(selector);
      held.delete(selector);
      if (cdp) await send("touchEnd", [p]);
      else await page.evaluate(synthetic, { type: "touchend", points: [...held.values()], changed: [p] });
    },
    async tap(selector, ms = 90) {
      if (cdp) {
        await this.down(selector);
        await pause(ms);
        await this.up(selector);
      } else {
        // A real tap, which is all Playwright's WebKit offers.
        const { x, y } = await centre(selector);
        await page.touchscreen.tap(x, y);
      }
    },
  };
}

async function drive(page, context, profile, { log, result, shot, controls, fps, summary }) {
  const vp = page.viewportSize();
  // Portrait first: the game asks to be turned.
  await page.setViewportSize({ width: Math.min(vp.width, vp.height), height: Math.max(vp.width, vp.height) });
  await pause(800);
  const rotate = await page.evaluate(() => {
    const r = document.getElementById("rotate");
    return !!r && getComputedStyle(r).display !== "none";
  });
  await shot("2-portrait");
  result(rotate, "in portrait the page asks to turn the device");
  await page.setViewportSize(vp);
  await pause(800);
  const shown = await controls();
  result(!!shown && shown.shown, `the on-screen controls appear on this touch device (${JSON.stringify(shown)})`);
  const finger = await fingers(page, context, profile.engine);
  // Menus by tap: the title's own Start / Continue button is tapped on the canvas.
  const before = await game(page);
  log(`game before the first tap: ${JSON.stringify(before)}`);
  const audio = () => page.evaluate(() => (window.cinderwakeAudio ? window.cinderwakeAudio() : []).join(","));
  // Playwright's page.evaluate counts as a user gesture in Chromium, so
  // whether sound truly waited for the first touch is only known when the
  // page hadn't been activated before it.
  const activated = await page.evaluate(() => navigator.userActivation ? navigator.userActivation.hasBeenActive : null);
  const audioBefore = await audio();
  // The title's Start / Continue plaque, in the game's 1280 x 720 interface.
  const box = await page.evaluate(() => window.cinderwakeTouch.toPage(354, 515));
  log(`the title's start button is at ${JSON.stringify(box)}`);
  if (box) await page.touchscreen.tap(box.x, box.y);
  let started = false;
  for (let i = 0; i < 20 && !started; i++) {
    await pause(250);
    started = (await game(page))?.mode === "play";
  }
  result(started, "tapping the title's start button starts a run");
  const audioAfter = await audio();
  log(`audio contexts: ${audioBefore} before the first touch, ${audioAfter} after`);
  if (activated === false) {
    result(!audioBefore.includes("running") && audioAfter.split(",")[0] === "running",
      `sound waits for the first touch, then starts (${audioBefore} → ${audioAfter})`);
  } else if (profile.engine === "webkit") {
    log(`NOTE sound can't be checked here: headless WebKit has no sound device (${audioBefore} → ${audioAfter})`);
  } else {
    result(audioAfter.split(",")[0] === "running",
      `sound is running after the first touch (${audioBefore} → ${audioAfter}; the page counted as activated before it, so waiting for the touch isn't shown)`);
  }
  await pause(2500); // the run's intro
  await shot("3-playing");
  const mode = await controls();
  result(mode && mode.mode === "play", `the play controls show during a run (${JSON.stringify(mode)})`);
  summary.playFps = await fps(4);
  log(`play frame rate: ${summary.playFps.toFixed(1)} fps (software rendering, load ${await loadAverage()})`);

  // Run right with the stick while jumping and striking: three fingers.
  await page.evaluate(() => window.cinderwakeTouch.clearSeen());
  const start = await game(page);
  await finger.down("#touch-stick", 0.9, 0);
  await pause(400);
  await finger.tap("#touch-jump");
  await pause(150);
  const mid = await game(page);
  await finger.down("#touch-strike");
  await pause(500);
  await shot("4-run-jump-strike");
  await finger.up("#touch-strike");
  await finger.up("#touch-stick");
  await pause(300);
  const ran = await game(page);
  log(`player: start ${JSON.stringify(start.player)}, mid-jump ${JSON.stringify(mid.player)}, after ${JSON.stringify(ran.player)}`);
  result(ran.player.x > start.player.x + 40, `holding the stick right runs right (x ${start.player.x.toFixed(0)} → ${ran.player.x.toFixed(0)})`);
  result(mid.player.y < start.player.y - 10 || ran.seen.includes("jump"), `the jump button jumps (y ${start.player.y.toFixed(0)} → ${mid.player.y.toFixed(0)})`);
  result(ran.seen.includes("attack"), "the strike button strikes while the stick is held");
  // A random run can leave play (a pickup's choice, or a fall into
  // guardians); then the session carries on from a fixed practice start.
  const practice = async () => {
    await page.goto(`${url}${url.includes("?") ? "&" : "?"}start=well`);
    for (let i = 0; i < 240 && (await game(page).catch(() => null))?.mode !== "play"; i++) await pause(500);
    await pause(1500);
  };
  const keepPlaying = async () => {
    const now = await game(page);
    if (now.mode === "play") return;
    log(`the run left play (mode "${now.mode}"); continuing from the practice start ?start=well`);
    await shot("left-play");
    await practice();
  };
  // Each remaining verb once; the page records what reached the game.
  for (const [id, verb] of [["dodge", "dodge"], ["parry", "parry"], ["bolt", "bow"], ["vessel", "grenade"],
    ["snare", "trap"], ["flask", "heal"], ["interact", "interact"]]) {
    await keepPlaying();
    await page.evaluate(() => window.cinderwakeTouch.clearSeen());
    await finger.tap(`#touch-${id}`, 120);
    await pause(250);
    const seen = (await game(page)).seen;
    result(seen.includes(verb), `the ${id} button reaches the game as ${verb}`);
  }
  await keepPlaying();
  await page.evaluate(() => window.cinderwakeTouch.clearSeen());
  await finger.down("#touch-stick", 0, 0.95);
  await pause(400);
  const down = await game(page);
  await finger.up("#touch-stick");
  result(down.seen.includes("down"), "pushing the stick down reaches the game as down (drop, slam, look below)");
  // The atlas doesn't stop the world, so the rest happens at the practice
  // start, where no guardian is near (a random run's can end it meanwhile).
  log("continuing from the practice start ?start=well for the atlas and pause");
  await practice();
  await finger.tap("#touch-map");
  await pause(500);
  await shot("5-atlas");
  result((await game(page)).mode === "atlas", "the atlas button opens the atlas");
  await finger.tap("#touch-map");
  await pause(300);
  await finger.tap("#touch-pause");
  await pause(700);
  await shot("6-paused");
  const paused = await game(page);
  result(paused.mode === "menu", `the pause button pauses (${paused.mode})`);
  const pausedMode = await controls();
  result(pausedMode.mode === "menu", `menus hide the play buttons (${pausedMode.mode})`);
  // The pause screen's Resume plaque.
  const resume = await page.evaluate(() => window.cinderwakeTouch.toPage(640, 521));
  if (resume) await page.touchscreen.tap(resume.x, resume.y);
  await pause(700);
  result((await game(page)).mode === "play", "tapping Resume on the pause screen resumes");
  // A keyboard press hides the controls; a touch brings them back.
  await page.keyboard.press("ArrowRight");
  await pause(300);
  const typed = await controls();
  result(!typed.shown, "a key press hides the on-screen controls");
  await page.touchscreen.tap(vp.width / 2, vp.height / 3);
  await pause(300);
  result((await controls()).shown, "the next touch shows them again");
  await shot("7-end");
}

const chosen = names.length ? names : ["iphone", "ipad", "pixel", "desktop"];
let failed = 0;
for (const name of chosen) {
  if (!PROFILES[name]) { console.error(`unknown profile ${name}`); process.exit(2); }
  const results = await session(name);
  failed += results.filter((r) => !r.ok).length;
}
console.log(failed ? `FAIL: ${failed} check(s) failed` : "PASS: every check passed");
process.exit(failed ? 1 : 0);
