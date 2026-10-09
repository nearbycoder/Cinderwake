// On-screen controls for phones and tablets, drawn over the canvas: a stick on
// the left, the actions on the right, and pause and atlas buttons at the top.
// They show on touch-first devices (a coarse pointer and no fine one) or after
// a real touch, and hide as soon as a key, the mouse, or a controller is used.
// Buttons use the Gamepad API's "standard" numbers, so src/pad.rs reads them
// in the controller's layout (see src/touch.rs). Menus are tapped on the
// canvas itself, which the loader turns into clicks.
(function () {
  "use strict";
  const coarseOnly = () => matchMedia("(pointer: coarse)").matches && !matchMedia("(any-pointer: fine)").matches;
  const touchFirst = coarseOnly();

  // Phones draw at most two device pixels per CSS pixel: a 3x screen would
  // more than double the pixels drawn each frame, and the size of the
  // interface's cached text, for no visible gain over a 1280 x 720 scene.
  if (touchFirst) {
    const real = Object.getOwnPropertyDescriptor(Window.prototype, "devicePixelRatio")
      || Object.getOwnPropertyDescriptor(window, "devicePixelRatio");
    if (real && real.get) {
      Object.defineProperty(window, "devicePixelRatio", {
        configurable: true,
        get: () => Math.min(2, real.get.call(window)),
      });
    }
  }

  // [id, label, standard button number, size, right, bottom] in CSS pixels
  // from the safe area's bottom-right corner; the thumb's resting arc. On
  // short screens (most phones held sideways) both thumbs' groups are drawn
  // at 85%, which keeps the smallest button at 44 px.
  const ACTIONS = [
    ["jump", "JUMP", 0, 76, 14, 14],
    ["strike", "STRIKE", 2, 66, 100, 14],
    ["bolt", "BOLT", 7, 52, 176, 16],
    ["dodge", "DODGE", 1, 60, 20, 100],
    ["parry", "PARRY", 5, 58, 92, 88],
    ["vessel", "VESSEL", 4, 52, 162, 78],
    ["interact", "USE", 3, 52, 24, 172],
    ["flask", "FLASK", 12, 52, 92, 158],
    ["snare", "SNARE", 6, 52, 156, 140],
  ];
  // [id, label, standard button number, right, top] from the top-right.
  const SYSTEM = [
    ["pause", "II", 9, 10, 10],
    ["map", "MAP", 8, 68, 10],
  ];
  const STICK = { size: 136, radius: 46, left: 20, bottom: 18 };
  const SEEN = ["move", "jump", "attack", "dodge", "parry", "bow", "grenade", "trap", "heal", "interact", "down"];

  const css = `
    #touch { position: fixed; inset: 0; pointer-events: none; z-index: 2;
      font: 700 11px ui-monospace, Menlo, Consolas, monospace; letter-spacing: 0.06em; }
    #touch[hidden], #touch[data-mode="menu"] .play { display: none; }
    #touch .group { position: absolute; width: 240px; height: 240px; }
    #touch .right { right: env(safe-area-inset-right, 0px); bottom: env(safe-area-inset-bottom, 0px); transform-origin: 100% 100%; }
    #touch .left { left: env(safe-area-inset-left, 0px); bottom: env(safe-area-inset-bottom, 0px); transform-origin: 0 100%; }
    @media (max-height: 460px) { #touch .group { transform: scale(0.85); } }
    #touch [data-control] { position: absolute; pointer-events: auto; box-sizing: border-box;
      display: grid; place-content: center; color: #f2d7a2; text-shadow: 0 1px 2px #000;
      background: radial-gradient(circle at 50% 35%, #232a4099, #0b0d1688 70%);
      border: 2px solid #d6ae67a0; border-radius: 50%;
      box-shadow: 0 0 0 1px #0b0d1680, inset 0 0 8px #0b0d16; opacity: 0.78;
      touch-action: none; user-select: none; -webkit-user-select: none; -webkit-touch-callout: none; }
    #touch .tool { border-color: #a1c6c4a0; color: #cfe3e1; }
    #touch .system { border-radius: 10px; width: 52px; height: 44px; }
    #touch [data-control].held { background: radial-gradient(circle at 50% 40%, #d6ae67d0, #6b5a3ec0 75%);
      color: #0b0d16; text-shadow: none; border-color: #f2d7a2; opacity: 1; transform: scale(0.94); }
    #touch .tool.held { background: radial-gradient(circle at 50% 40%, #a1c6c4d0, #2f5654c0 75%); border-color: #cfe3e1; }
    #touch[data-mode="atlas"] [data-control="map"] { border-color: #f2d7a2; background: #6b5a3ec0; }
    #touch-stick { background: radial-gradient(circle, #0b0d1666 30%, #151827a0 70%) !important; border-color: #6b5a3ec0 !important; }
    #touch-stick i { position: absolute; left: 50%; top: 50%; width: 58px; height: 58px; margin: -29px 0 0 -29px;
      border-radius: 50%; border: 2px solid #d6ae67c0; background: radial-gradient(circle at 50% 35%, #3a3346, #151827);
      box-shadow: 0 2px 6px #000a; pointer-events: none; }
    #touch-stick.held i { border-color: #f2d7a2; background: radial-gradient(circle at 50% 35%, #d6ae67, #6b5a3e); }
    #rotate { position: fixed; inset: 0; z-index: 3; display: none; place-content: center; gap: 14px;
      padding: 24px; background: #0b0d16f2; color: #f2d7a2; font: 22px Georgia, serif; text-align: center; }
    #rotate small { color: #a1c6c4; font: 15px Georgia, serif; line-height: 1.4; }
    #rotate b { display: block; margin: 0 auto 6px; width: 34px; height: 56px; border: 3px solid #d6ae67;
      border-radius: 7px; animation: cw-turn 2.4s ease-in-out infinite; }
    @keyframes cw-turn { 0%, 30% { transform: rotate(0); } 60%, 100% { transform: rotate(-90deg); } }
    html.blocked #rotate { display: grid; }`;

  const touchState = { shown: false, mode: "menu", blocked: false };
  // Held controls by touch identifier: { control, element }.
  const held = new Map();
  // Buttons pressed since the game last looked: a tap that starts and ends
  // between two frames still counts once.
  let pressed = 0;
  const stick = { x: 0, y: 0 };
  // What the game reported (src/touch.rs), for the page's own tests.
  const report = { mode: "menu", player: { x: 0, y: 0 }, seen: new Set() };
  let root = null;

  function build() {
    const style = document.createElement("style");
    style.textContent = css;
    document.head.append(style);
    root = document.createElement("div");
    root.id = "touch";
    root.hidden = true;
    root.dataset.mode = "menu";
    root.setAttribute("aria-hidden", "true");
    const add = (parent, id, label, bit, classes, place) => {
      const button = document.createElement("div");
      button.id = `touch-${id}`;
      button.dataset.control = id;
      if (bit !== null) button.dataset.bit = bit;
      button.className = classes;
      button.textContent = label;
      Object.assign(button.style, place);
      parent.append(button);
      return button;
    };
    const group = (side) => {
      const element = document.createElement("div");
      element.className = `group ${side}`;
      root.append(element);
      return element;
    };
    const right = group("right");
    const left = group("left");
    const safe = (side, px) => `calc(env(safe-area-inset-${side}, 0px) + ${px}px)`;
    for (const [id, label, bit, size, r, b] of ACTIONS) {
      const primary = ["jump", "strike", "dodge", "parry"].includes(id);
      add(right, id, label, bit, primary ? "play" : "play tool", {
        width: `${size}px`, height: `${size}px`, right: `${r}px`, bottom: `${b}px`,
        fontSize: size >= 70 ? "13px" : "11px",
      });
    }
    for (const [id, label, bit, r, top] of SYSTEM) {
      add(root, id, label, bit, "play system", { right: safe("right", r), top: safe("top", top) });
    }
    const base = add(left, "stick", "", null, "play", {
      width: `${STICK.size}px`, height: `${STICK.size}px`, left: `${STICK.left}px`, bottom: `${STICK.bottom}px`,
    });
    base.append(document.createElement("i"));
    const rotate = document.createElement("div");
    rotate.id = "rotate";
    rotate.setAttribute("role", "alert");
    rotate.innerHTML = "<b></b>Turn your device sideways<small>Cinderwake plays in landscape.<br>A run in progress waits until then.</small>";
    document.body.append(root, rotate);
    for (const type of ["touchstart", "touchmove", "touchend", "touchcancel"]) {
      root.addEventListener(type, onTouch, { passive: false });
    }
  }

  const controlAt = (x, y) => {
    const element = document.elementFromPoint(x, y);
    return element && element.closest ? element.closest("#touch [data-control]") : null;
  };
  function steer(touch, element) {
    const box = element.getBoundingClientRect();
    // The group may be drawn smaller; the throw shrinks with it.
    const radius = STICK.radius * (box.width / STICK.size);
    let dx = (touch.clientX - (box.left + box.width / 2)) / radius;
    let dy = (touch.clientY - (box.top + box.height / 2)) / radius;
    const length = Math.hypot(dx, dy);
    if (length > 1) { dx /= length; dy /= length; }
    stick.x = dx;
    stick.y = dy;
    element.firstChild.style.transform = `translate(${dx * STICK.radius}px, ${dy * STICK.radius}px)`;
  }
  function release(id) {
    const hold = held.get(id);
    if (!hold) return;
    held.delete(id);
    if (![...held.values()].some((h) => h.element === hold.element)) hold.element.classList.remove("held");
    if (hold.element.id === "touch-stick") {
      stick.x = 0;
      stick.y = 0;
      hold.element.firstChild.style.transform = "";
    }
  }
  function onTouch(event) {
    // No scrolling, zooming, selection, callouts, or emulated mouse clicks.
    event.preventDefault();
    for (const touch of event.changedTouches) {
      const id = touch.identifier;
      if (event.type === "touchstart") {
        const element = touch.target.closest ? touch.target.closest("[data-control]") : null;
        if (!element) continue;
        held.set(id, { element });
        element.classList.add("held");
        if (element.dataset.bit !== undefined) pressed |= 1 << Number(element.dataset.bit);
        if (element.id === "touch-stick") steer(touch, element);
      } else if (event.type === "touchmove") {
        const hold = held.get(id);
        if (!hold) continue;
        if (hold.element.id === "touch-stick") {
          steer(touch, hold.element);
        } else {
          // A thumb sliding from one action to the next presses it instead.
          const now = controlAt(touch.clientX, touch.clientY);
          if (now && now !== hold.element && now.id !== "touch-stick") {
            release(id);
            held.set(id, { element: now });
            now.classList.add("held");
            if (now.dataset.bit !== undefined) pressed |= 1 << Number(now.dataset.bit);
          }
        }
      } else {
        release(id);
      }
    }
  }
  function releaseAll() {
    for (const id of [...held.keys()]) release(id);
    pressed = 0;
  }

  function update() {
    const blocked = touchState.shown && window.innerHeight > window.innerWidth;
    if (blocked !== touchState.blocked) {
      touchState.blocked = blocked;
      document.documentElement.classList.toggle("blocked", blocked);
      if (blocked) releaseAll();
    }
  }
  function show(on) {
    if (on === touchState.shown || !root) return;
    touchState.shown = on;
    root.hidden = !on;
    document.documentElement.dataset.touch = on ? "shown" : "hidden";
    if (!on) releaseAll();
    update();
  }

  function listen() {
    document.addEventListener("touchstart", () => show(true), { capture: true, passive: true });
    // Keys, a real mouse, or a controller hide the controls again.
    document.addEventListener("keydown", (e) => { if (!e.repeat) show(false); }, true);
    for (const type of ["pointerdown", "pointermove"]) {
      document.addEventListener(type, (e) => {
        if (e.pointerType === "mouse" && (type === "pointerdown" || e.movementX || e.movementY)) show(false);
      }, true);
    }
    // No pinch zoom (iOS ignores the viewport's user-scalable) and no
    // double-tap zoom or text selection anywhere on the page.
    for (const type of ["gesturestart", "gesturechange", "dblclick", "selectstart", "contextmenu"]) {
      document.addEventListener(type, (e) => { if (touchState.shown || type.startsWith("gesture")) e.preventDefault(); }, { passive: false });
    }
    window.addEventListener("resize", update);
    window.addEventListener("orientationchange", update);
    window.addEventListener("blur", releaseAll);
    document.addEventListener("visibilitychange", () => { if (document.hidden) releaseAll(); });
  }

  const bits = () => {
    let value = pressed;
    pressed = 0;
    if (!touchState.shown || touchState.blocked) return 0;
    for (const { element } of held.values()) {
      if (element.dataset.bit !== undefined) value |= 1 << Number(element.dataset.bit);
    }
    return value >>> 0;
  };

  window.cinderwakeTouch = {
    // A real controller was used (web/cinderwake-pad.js).
    padUsed: () => show(false),
    shown: () => touchState.shown,
    report: () => ({ mode: report.mode, player: { ...report.player }, seen: [...report.seen], shown: touchState.shown }),
    clearSeen: () => report.seen.clear(),
    // The page position of a point in the game's 1280 x 720 interface,
    // letterboxed into the canvas as src/main.rs does.
    toPage: (x, y) => {
      const box = document.getElementById("glcanvas").getBoundingClientRect();
      const scale = Math.min(box.width / 1280, box.height / 720);
      return {
        x: box.left + (box.width - 1280 * scale) / 2 + x * scale,
        y: box.top + (box.height - 720 * scale) / 2 + y * scale,
      };
    },
  };

  build();
  listen();
  show(touchFirst);

  miniquad_add_plugin({
    name: "cinderwake_touch",
    version: 1,
    register_plugin: function (importObject) {
      importObject.env.cinderwake_touch_buttons = bits;
      importObject.env.cinderwake_touch_axis = function (index) {
        if (!touchState.shown || touchState.blocked) return 0;
        return index === 0 ? stick.x : index === 1 ? stick.y : 0;
      };
      // 1: the controls are shown; 2: the page asks for the device to be turned.
      importObject.env.cinderwake_touch_state = function () {
        return (touchState.shown ? 1 : 0) | (touchState.blocked ? 2 : 0);
      };
      importObject.env.cinderwake_touch_frame = function (mode, input, x, y) {
        const name = ["menu", "play", "atlas"][mode] || "menu";
        if (name !== report.mode) {
          report.mode = name;
          root.dataset.mode = name;
          if (name === "menu") releaseAll();
        }
        report.player.x = x;
        report.player.y = y;
        SEEN.forEach((verb, i) => { if (input & (1 << i)) report.seen.add(verb); });
      };
    },
  });
})();
