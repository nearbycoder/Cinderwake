#!/usr/bin/env node
// Checks that the browser build at a URL reaches the title screen with no
// errors, in headless Chromium driven over the DevTools protocol (no npm
// packages needed, Node 22 or newer). Exits 0 only on success.
//
//   node scripts/check-pages.mjs https://nearbycoder.github.io/Cinderwake/
//   node scripts/check-pages.mjs <url> --screenshot title.png --timeout 240
//
// Chromium is found in CHROMIUM_PATH, then Playwright's cache
// (~/.cache/ms-playwright), then the PATH. It passes when the page reports
// the game running (web/index.html sets <html data-game>), the canvas shows
// a drawn screen rather than a blank one, and no console error, uncaught
// exception, or failed request appeared. The profile is kept under target/.
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { inflateSync } from "node:zlib";

/// Decodes an 8-bit RGB or RGBA PNG (as Chromium writes) into pixels.
export function decodePng(png) {
  let width = 0, height = 0, channels = 0;
  const idat = [];
  for (let at = 8; at < png.length;) {
    const length = png.readUInt32BE(at);
    const type = png.toString("latin1", at + 4, at + 8);
    const data = png.subarray(at + 8, at + 8 + length);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      if (data[8] !== 8 || ![2, 6].includes(data[9]) || data[12] !== 0) {
        throw new Error("unsupported PNG format");
      }
      channels = data[9] === 6 ? 4 : 3;
    } else if (type === "IDAT") {
      idat.push(data);
    }
    at += 12 + length;
  }
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * channels;
  const pixels = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    for (let x = 0; x < stride; x++) {
      const a = x >= channels ? pixels[y * stride + x - channels] : 0;
      const b = y > 0 ? pixels[(y - 1) * stride + x] : 0;
      const c = x >= channels && y > 0 ? pixels[(y - 1) * stride + x - channels] : 0;
      let value = line[x];
      if (filter === 1) value += a;
      else if (filter === 2) value += b;
      else if (filter === 3) value += (a + b) >> 1;
      else if (filter === 4) {
        const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
        value += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      }
      pixels[y * stride + x] = value & 255;
    }
  }
  return { width, height, channels, pixels };
}

/// How much of a screenshot is drawn: distinct colours (4 bits a channel)
/// and the share of pixels that aren't near-black.
export function screenStats(png) {
  const { width, height, channels, pixels } = decodePng(png);
  const colours = new Set();
  let lit = 0, count = 0;
  for (let y = 0; y < height; y += 2) {
    for (let x = 0; x < width; x += 2) {
      const i = (y * width + x) * channels;
      const [r, g, b] = [pixels[i], pixels[i + 1], pixels[i + 2]];
      colours.add(((r >> 4) << 8) | ((g >> 4) << 4) | (b >> 4));
      if (r + g + b > 60) lit++;
      count++;
    }
  }
  return { colours: colours.size, lit: lit / count };
}

/// A screen counts as drawn when it has many colours and some brightness.
export const looksDrawn = (stats) => stats.colours >= 64 && stats.lit >= 0.15;

export function findChromium() {
  if (process.env.CHROMIUM_PATH) return process.env.CHROMIUM_PATH;
  const cache = join(homedir(), ".cache", "ms-playwright");
  const newest = (prefix, tail) => {
    if (!existsSync(cache)) return null;
    const found = readdirSync(cache)
      .filter((d) => d.startsWith(prefix))
      .sort((a, b) => Number(b.split("-").pop()) - Number(a.split("-").pop()))
      .map((d) => join(cache, d, ...tail))
      .find(existsSync);
    return found || null;
  };
  const cached = newest("chromium_headless_shell-", ["chrome-headless-shell-linux64", "chrome-headless-shell"])
    || newest("chromium-", ["chrome-linux64", "chrome"]);
  if (cached) return cached;
  for (const name of ["chromium", "chromium-browser", "google-chrome", "google-chrome-stable"]) {
    for (const dir of (process.env.PATH || "").split(delimiter)) {
      if (existsSync(join(dir, name))) return join(dir, name);
    }
  }
  throw new Error("No Chromium found; set CHROMIUM_PATH.");
}

async function check(url, { timeout, settle, screenshot }) {
  const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  const profile = join(repo, "target", "check-pages", `profile-${process.pid}`);
  mkdirSync(profile, { recursive: true });
  const chrome = spawn(findChromium(), [
    "--headless", "--remote-debugging-port=0", `--user-data-dir=${profile}`,
    "--no-first-run", "--no-default-browser-check", "--mute-audio",
    "--window-size=1280,720", "--enable-unsafe-swiftshader", "about:blank",
  ], { stdio: ["ignore", "ignore", "pipe"] });
  const problems = [];
  const started = Date.now();
  let socket;
  try {
    const endpoint = await new Promise((done, fail) => {
      let text = "";
      const timer = setTimeout(() => fail(new Error("Chromium didn't start")), 30000);
      chrome.stderr.on("data", (chunk) => {
        text += chunk;
        const match = text.match(/DevTools listening on (ws:\/\/\S+)/);
        if (match) { clearTimeout(timer); done(match[1]); }
      });
      chrome.on("exit", (code) => fail(new Error(`Chromium exited (${code}): ${text.slice(-500)}`)));
    });
    socket = new WebSocket(endpoint);
    await new Promise((done, fail) => { socket.onopen = done; socket.onerror = () => fail(new Error("couldn't reach Chromium")); });
    let next = 0;
    const waiting = new Map();
    const handlers = {};
    socket.onmessage = ({ data }) => {
      const message = JSON.parse(data);
      if (message.id !== undefined && waiting.has(message.id)) {
        const { done, fail } = waiting.get(message.id);
        waiting.delete(message.id);
        message.error ? fail(new Error(message.error.message)) : done(message.result);
      } else if (message.method && handlers[message.method]) {
        handlers[message.method](message.params);
      }
    };
    const send = (method, params = {}, sessionId) => new Promise((done, fail) => {
      const id = ++next;
      waiting.set(id, { done, fail });
      socket.send(JSON.stringify({ id, method, params, sessionId }));
    });
    const { targetId } = await send("Target.createTarget", { url: "about:blank" });
    const { sessionId } = await send("Target.attachToTarget", { targetId, flatten: true });
    const page = (method, params) => send(method, params, sessionId);
    const describe = (args) => args.map((a) => a.value ?? a.description ?? a.type).join(" ");
    handlers["Runtime.exceptionThrown"] = ({ exceptionDetails: d }) =>
      problems.push(`uncaught: ${d.exception?.description || d.text}`);
    handlers["Runtime.consoleAPICalled"] = ({ type, args }) => {
      if (type === "error" || type === "assert") problems.push(`console.${type}: ${describe(args)}`);
    };
    handlers["Log.entryAdded"] = ({ entry }) => {
      if (entry.level === "error") problems.push(`${entry.source}: ${entry.text}${entry.url ? ` (${entry.url})` : ""}`);
    };
    let bytes = 0;
    handlers["Network.loadingFinished"] = ({ encodedDataLength }) => { bytes += encodedDataLength; };
    handlers["Network.responseReceived"] = ({ response }) => {
      if (response.status >= 400) problems.push(`HTTP ${response.status}: ${response.url}`);
    };
    handlers["Network.loadingFailed"] = ({ errorText, canceled }) => {
      if (!canceled) problems.push(`request failed: ${errorText}`);
    };
    await Promise.all(["Runtime.enable", "Log.enable", "Page.enable", "Network.enable"].map((m) => page(m)));
    await page("Network.setCacheDisabled", { cacheDisabled: true });
    await page("Emulation.setDeviceMetricsOverride", { width: 1280, height: 720, deviceScaleFactor: 1, mobile: false });
    const navigation = await page("Page.navigate", { url });
    if (navigation.errorText) throw new Error(`couldn't open ${url}: ${navigation.errorText}`);
    const evaluate = async (expression) => (await page("Runtime.evaluate", { expression, returnByValue: true })).result.value;
    let state;
    while (Date.now() - started < timeout * 1000) {
      state = await evaluate("document.documentElement.dataset.game || ''");
      if (state === "running" || state === "failed" || state === "stopped") break;
      await new Promise((r) => setTimeout(r, 500));
    }
    const loaded = (Date.now() - started) / 1000;
    if (state !== "running") {
      const shown = await evaluate("document.getElementById('loading')?.innerText || ''");
      throw new Error(state === "failed" || state === "stopped"
        ? `the page reports "${state}": ${shown.replace(/\s+/g, " ").trim()}`
        : `not running after ${timeout} s (state "${state}"; ${shown.replace(/\s+/g, " ").trim()})`);
    }
    console.log(`running after ${loaded.toFixed(1)} s (${(bytes / 1e6).toFixed(1)} MB downloaded)`);
    await new Promise((r) => setTimeout(r, settle * 1000));
    const shot = Buffer.from((await page("Page.captureScreenshot", { format: "png" })).data, "base64");
    if (screenshot) writeFileSync(screenshot, shot);
    const stats = screenStats(shot);
    const after = await evaluate("document.documentElement.dataset.game");
    console.log(`screen: ${stats.colours} colours, ${(100 * stats.lit).toFixed(0)}% lit; state "${after}"`);
    if (after !== "running") problems.push(`the page reports "${after}" after loading`);
    if (!looksDrawn(stats)) problems.push("the screen looks blank (the title wasn't drawn)");
    await send("Browser.close").catch(() => {});
  } catch (error) {
    problems.push(error.message);
  } finally {
    socket?.close();
    if (chrome.exitCode === null) {
      chrome.kill();
      await new Promise((r) => { chrome.once("exit", r); setTimeout(r, 5000); });
    }
    rmSync(profile, { recursive: true, force: true });
  }
  return problems;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const option = (name, fallback) => {
    const i = args.indexOf(name);
    if (i < 0) return fallback;
    const [value] = args.splice(i, 2).slice(1);
    return value;
  };
  const timeout = Number(option("--timeout", 180));
  const settle = Number(option("--settle", 3));
  const screenshot = option("--screenshot", null);
  const [url] = args;
  if (!url || args.length > 1) {
    console.error("usage: node scripts/check-pages.mjs <url> [--timeout seconds] [--settle seconds] [--screenshot file.png]");
    process.exit(2);
  }
  const problems = await check(url, { timeout, settle, screenshot });
  if (problems.length) {
    console.error(`FAIL ${url}`);
    for (const p of [...new Set(problems)]) console.error(`  ${p}`);
    process.exit(1);
  }
  console.log(`PASS ${url}: the title screen was drawn with no errors`);
}
