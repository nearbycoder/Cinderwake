// Reads controllers through the Gamepad API for src/pad.rs. Buttons use the
// API's "standard" layout numbers; every connected pad is combined.
(function () {
  "use strict";
  const pads = () => {
    try {
      return Array.from(navigator.getGamepads ? navigator.getGamepads() : []).filter((p) => p && p.connected);
    } catch (_) {
      return []; // Blocked by a permissions policy.
    }
  };
  miniquad_add_plugin({
    name: "cinderwake_pad",
    version: 1,
    register_plugin: function (importObject) {
      importObject.env.cinderwake_pad_buttons = function () {
        let bits = 0;
        for (const pad of pads()) {
          pad.buttons.forEach((button, i) => {
            if (i < 32 && (button.pressed || button.value > 0.5)) {
              bits |= 1 << i;
            }
          });
        }
        return bits >>> 0;
      };
      // Connected pads. Browsers list a pad only after one of its buttons
      // has been pressed on the page, and drop it when it's removed.
      importObject.env.cinderwake_pad_count = function () {
        return pads().length;
      };
      // The axis pushed furthest on any pad: 0 is the left stick's x, 1 its y.
      importObject.env.cinderwake_pad_axis = function (index) {
        let value = 0;
        for (const pad of pads()) {
          const axis = pad.axes[index] || 0;
          if (Math.abs(axis) > Math.abs(value)) {
            value = axis;
          }
        }
        return value;
      };
    },
  });
})();
