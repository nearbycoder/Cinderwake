#!/usr/bin/env node
// A short scripted session with the browser build, in headless Chromium and
// headless Firefox: loading to the title, the touch controls staying hidden,
// audio starting only after the first key, a few seconds of play, a fidelity
// change surviving a reload, and a scripted controller. Logs and screenshots
// go to target/pages-test/<browser>/.
//
//   npm install --prefix target/web-tools puppeteer-core@25   # once
//   node scripts/test-pages.mjs http://127.0.0.1:8080/Cinderwake/ [chromium|firefox]
//
// Firefox is the installed one (FIREFOX_PATH, default /usr/bin/firefox),
// driven over WebDriver BiDi; Chromium is found as in check-pages.mjs.
import { createRequire } from "node:module";
import { appendFileSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { decodePng, findChromium, looksDrawn, screenStats } from "./check-pages.mjs";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const puppeteer = createRequire(join(repo, "target", "web-tools", "/"))("puppeteer-core");
const [url, ...names] = process.argv.slice(2);
if (!url) {
  console.error("usage: node scripts/test-pages.mjs <url> [chromium|firefox ...]");
  process.exit(2);
}
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

// Share of pixels that changed between two screenshots.
function changed(a, b) {
  const [x, y] = [decodePng(a), decodePng(b)];
  let differ = 0, count = 0;
  for (let i = 0; i < x.pixels.length; i += x.channels * 4) {
    const d = Math.abs(x.pixels[i] - y.pixels[i]) + Math.abs(x.pixels[i + 1] - y.pixels[i + 1])
      + Math.abs(x.pixels[i + 2] - y.pixels[i + 2]);
    if (d > 24) differ++;
    count++;
  }
  return differ / count;
}

// Reports audio contexts' states, started sounds, and the page's state as
// console messages, and offers a scripted controller through
// navigator.getGamepads (window.__pad.buttons is a bit mask). The reports let
// the test watch the page before the first key without page.evaluate, which
// Puppeteer runs as a user gesture (that would unlock audio by itself).
function instrument() {
  const report = (what) => console.log(`__cw ${JSON.stringify(what)}`);
  const Real = window.AudioContext || window.webkitAudioContext;
  let contexts = 0;
  class Watched extends Real {
    constructor(...args) {
      super(...args);
      const audio = contexts++;
      report({ audio, state: this.state });
      this.addEventListener("statechange", () => report({ audio, state: this.state }));
    }
    createBufferSource() {
      const source = super.createBufferSource();
      const start = source.start.bind(source);
      source.start = (...args) => { report({ sound: true }); return start(...args); };
      return source;
    }
  }
  new MutationObserver(() => report({ game: document.documentElement.dataset.game }))
    .observe(document, { subtree: true, attributes: true, attributeFilter: ["data-game"] });
  window.AudioContext = Watched;
  window.webkitAudioContext = Watched;
  window.__pad = { buttons: 0, connected: false };
  const pad = () => ({
    id: "Scripted pad", index: 0, connected: true, mapping: "standard", timestamp: performance.now(),
    axes: [0, 0, 0, 0],
    buttons: Array.from({ length: 17 }, (_, i) => {
      const pressed = (window.__pad.buttons >> i) & 1;
      return { pressed: !!pressed, touched: !!pressed, value: pressed };
    }),
  });
  navigator.getGamepads = () => (window.__pad.connected ? [pad(), null, null, null] : [null, null, null, null]);
}

async function session(name) {
  const out = join(repo, "target", "pages-test", name);
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  const logFile = join(out, "session.log");
  writeFileSync(logFile, "");
  let lines = 0;
  const log = (text) => {
    console.log(`[${name}] ${text}`);
    if (++lines <= 2000) appendFileSync(logFile, `${text}\n`);
  };
  const errors = [];
  const results = [];
  const result = (ok, what) => { results.push({ ok, what }); log(`${ok ? "PASS" : "FAIL"} ${what}`); };
  const profile = join(out, "profile");
  const browser = await puppeteer.launch(name === "firefox"
    ? {
      browser: "firefox", executablePath: process.env.FIREFOX_PATH || "/usr/bin/firefox", headless: true,
      userDataDir: profile,
      // Firefox's default: sound can't start before the page is used.
      extraPrefsFirefox: { "media.autoplay.default": 1, "media.autoplay.blocking_policy": 0, "media.autoplay.block-webaudio": true },
    }
    : {
      executablePath: findChromium(), headless: true, userDataDir: profile,
      // Not --mute-audio: muted pages may start sound without input.
      args: ["--autoplay-policy=document-user-activation-required", "--enable-unsafe-swiftshader"],
    });
  log(`browser: ${await browser.version()}`);
  try {
    const page = await browser.newPage();
    await page.setViewport({ width: 1280, height: 720 });
    // What the instrumentation reported: page state, audio states, sounds.
    const seen = { game: null, audio: [], sounds: 0 };
    let ran = () => {};
    const running = new Promise((r) => { ran = r; });
    page.on("console", (m) => {
      if (m.text().startsWith("__cw ")) {
        const what = JSON.parse(m.text().slice(5));
        if (what.sound) { seen.sounds++; return; }
        log(`reported: ${m.text().slice(5)}`);
        if (what.game) { seen.game = what.game; if (what.game !== "loading") ran(); }
        if (what.audio !== undefined) seen.audio[what.audio] = what.state;
        return;
      }
      log(`console.${m.type()}: ${m.text()}`);
      if (m.type() === "error" || m.type() === "assert") errors.push(m.text());
    });
    page.on("pageerror", (e) => { log(`pageerror: ${e.message}`); errors.push(e.message); });
    page.on("requestfailed", (r) => {
      const why = r.failure()?.errorText || "";
      if (!/abort/i.test(why)) { log(`requestfailed: ${r.url()} ${why}`); errors.push(r.url()); }
    });
    page.on("response", (r) => { if (r.status() >= 400) { log(`HTTP ${r.status()} ${r.url()}`); errors.push(r.url()); } });
    await page.evaluateOnNewDocument(instrument);
    const shot = async (label) => {
      const png = Buffer.from(await page.screenshot({ type: "png" }));
      writeFileSync(join(out, `${label}.png`), png);
      return png;
    };
    // Puppeteer's Firefox "Enter" is the keypad's (WebDriver \uE007), which
    // the game doesn't bind; \uE006 is the main Enter key.
    const enter = name === "firefox" ? "\uE006" : "Enter";
    const tap = async (key, hold = 250) => { await page.keyboard.down(key); await pause(hold); await page.keyboard.up(key); await pause(250); };
    const pressPad = async (button) => {
      await page.evaluate((b) => { window.__pad.connected = true; window.__pad.buttons = 1 << b; }, button);
      await pause(350);
      await page.evaluate(() => { window.__pad.buttons = 0; });
      await pause(350);
    };
    // Waits for the title. The first time, only through console reports.
    const toTitle = async (label, quietly) => {
      const t0 = Date.now();
      if (quietly) {
        await Promise.race([running, pause(180000)]);
      } else {
        await page.waitForFunction(() => ["running", "failed", "stopped"].includes(document.documentElement.dataset.game), { timeout: 180000, polling: 250 });
      }
      const seconds = (Date.now() - t0) / 1000;
      await pause(2500);
      const state = quietly ? seen.game : await page.evaluate(() => document.documentElement.dataset.game);
      const stats = screenStats(await shot(label));
      result(state === "running" && looksDrawn(stats),
        `${label}: state "${state}" after ${seconds.toFixed(1)} s; title drawn (${stats.colours} colours, ${(100 * stats.lit).toFixed(0)}% lit)`);
    };
    const settings = () => page.evaluate(() => {
      const text = localStorage.getItem("cinderwake/settings.json");
      return text ? JSON.parse(text) : null;
    });

    // 1. Load to the title from an empty profile.
    const loadStart = Date.now();
    await page.goto(url, { waitUntil: "load" });
    await toTitle("01-title", true);
    log(`load to running: ${((Date.now() - loadStart) / 1000).toFixed(1)} s including page load`);
    // The on-screen touch controls (web/cinderwake-touch.js) are for phones
    // and tablets; a desktop never shows them.
    const touchShown = () => page.evaluate(() => {
      const touch = document.getElementById("touch");
      return touch ? !touch.hidden && getComputedStyle(touch).display !== "none" : null;
    });
    const touchAtTitle = await touchShown();
    result(touchAtTitle === false, `the on-screen touch controls are hidden at the title (${touchAtTitle === null ? "missing" : touchAtTitle ? "shown" : "hidden"})`);

    // 2. Audio waits for the first key, which goes to the game unclicked.
    const before = { states: [...seen.audio], sounds: seen.sounds };
    log(`audio before input: ${JSON.stringify(before)}`);
    await tap(enter);
    await pause(1500);
    const after = { states: [...seen.audio], sounds: seen.sounds };
    log(`audio after Enter: ${JSON.stringify(after)}`);
    result(before.states.length > 0 && before.states[0] !== "running", `audio held before any input (game context "${before.states[0]}")`);
    result(after.states[0] === "running" && after.sounds > before.sounds,
      `audio running after the first key (game context "${after.states[0]}", ${after.sounds - before.sounds} sounds started)`);
    const focused = await page.evaluate(() => document.activeElement?.id);
    result(focused === "glcanvas", `the game's canvas had keyboard focus without a click ("${focused}")`);

    // 3. A few seconds of play: run right, jump, strike, dodge, run left.
    const runStart = await shot("02-run-start");
    await page.keyboard.down("KeyD");
    await pause(1200);
    await tap(" ", 200);
    await pause(400);
    await page.keyboard.up("KeyD");
    await tap("KeyJ");
    await tap("KeyJ");
    await tap("ShiftLeft");
    await page.keyboard.down("KeyA");
    await pause(800);
    await page.keyboard.up("KeyA");
    const played = await shot("03-after-play");
    const moved = changed(runStart, played);
    result(moved > 0.05, `play changed the screen (${(100 * moved).toFixed(0)}% of pixels)`);
    await tap("Escape");
    await pause(600);
    await shot("04-paused");
    await tap("Escape");

    // 4. A setting survives a reload: F9 steps fidelity from High to Ultra.
    const was = await settings();
    await tap("F9");
    await pause(800);
    const now = await settings();
    log(`fidelity: ${was?.fidelity ?? "(no settings saved yet)"} -> ${now?.fidelity}`);
    result(now?.fidelity === "ultra", `F9 saved fidelity "${now?.fidelity}" to localStorage`);
    await shot("05-ultra");
    await page.reload({ waitUntil: "load" });
    await toTitle("06-title-after-reload");
    const kept = await settings();
    result(kept?.fidelity === "ultra", `after a reload, localStorage still has fidelity "${kept?.fidelity}"`);
    await tap("KeyO");
    await pause(900);
    await shot("07-options-after-reload");
    await tap("Escape");
    await pause(600);

    // 5. A scripted controller starts a run from the title (A).
    const title = await shot("08-title-before-pad");
    await pressPad(0);
    await pause(1500);
    const padRun = await shot("09-after-pad-a");
    const padMoved = changed(title, padRun);
    result(padMoved > 0.05, `controller A left the title (${(100 * padMoved).toFixed(0)}% of pixels changed)`);

    // 6. Fullscreen, where the headless browser allows it.
    await tap("F11");
    await pause(1500);
    const full = await page.evaluate(() => document.fullscreenElement?.id || null);
    log(`fullscreen after F11: ${full ?? "not entered (headless)"}`);
    await shot("10-after-f11");

    const touchAtEnd = await touchShown();
    result(touchAtEnd === false, `the on-screen touch controls stayed hidden through keys and a controller (${touchAtEnd})`);
    const state = await page.evaluate(() => document.documentElement.dataset.game);
    result(state === "running", `still running at the end (state "${state}")`);
    result(errors.length === 0, `no console errors, uncaught exceptions, or failed requests (${errors.length})`);
  } catch (error) {
    result(false, `session stopped: ${error.message}`);
  } finally {
    await browser.close();
    rmSync(profile, { recursive: true, force: true });
  }
  const failed = results.filter((r) => !r.ok).length;
  log(`${failed ? "FAIL" : "PASS"}: ${results.length - failed} of ${results.length} checks passed; log ${logFile}`);
  return failed === 0;
}

let ok = true;
for (const name of names.length ? names : ["chromium", "firefox"]) {
  ok = (await session(name)) && ok;
}
process.exit(ok ? 0 : 1);
