// Persists Cinderwake progress and settings in localStorage, and reads launch
// options from the page address. Rust passes keys and values as UTF-8 byte
// ranges in wasm memory (see src/storage.rs and src/launch.rs).
(function () {
  "use strict";
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();
  const text = (ptr, len) => decoder.decode(new Uint8Array(wasm_memory.buffer, ptr, len));
  const stored = (key) => {
    try {
      return window.localStorage.getItem(key);
    } catch (_) {
      return null; // Storage can be blocked by privacy settings.
    }
  };
  miniquad_add_plugin({
    name: "cinderwake_storage",
    version: 1,
    register_plugin: function (importObject) {
      importObject.env.cinderwake_storage_len = function (key, keyLen) {
        const value = stored(text(key, keyLen));
        return value === null ? -1 : encoder.encode(value).length;
      };
      importObject.env.cinderwake_storage_read = function (key, keyLen, out, outLen) {
        const value = stored(text(key, keyLen));
        if (value !== null) {
          new Uint8Array(wasm_memory.buffer, out, outLen).set(encoder.encode(value).subarray(0, outLen));
        }
      };
      // Launch options from the page address, such as ?start=chest.
      importObject.env.cinderwake_query_len = function (key, keyLen) {
        const value = new URLSearchParams(window.location.search).get(text(key, keyLen));
        return value === null ? -1 : encoder.encode(value).length;
      };
      importObject.env.cinderwake_query_read = function (key, keyLen, out, outLen) {
        const value = new URLSearchParams(window.location.search).get(text(key, keyLen));
        if (value !== null) {
          new Uint8Array(wasm_memory.buffer, out, outLen).set(encoder.encode(value).subarray(0, outLen));
        }
      };
      importObject.env.cinderwake_storage_write = function (key, keyLen, value, valueLen) {
        try {
          window.localStorage.setItem(text(key, keyLen), text(value, valueLen));
          return 1;
        } catch (_) {
          return 0;
        }
      };
    },
  });
})();
