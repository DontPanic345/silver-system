"use strict";

/*
 * Headless regression checks for the simulation core (js/main.js).
 *
 *   node test/sim-invariants.js
 *
 * There's no build step and no browser here: we stub the handful of DOM
 * objects main.js touches at load, then drive `step()` directly and read the
 * composition arrays back. Covers the things that are easy to break and hard
 * to eyeball — mass conservation, water finding its level, the movement pass
 * not strobing, and the closed water cycle conserving liquid + vapour.
 */

const fs = require("fs");
const path = require("path");

// ---- DOM stubs -------------------------------------------------------------
const CANVAS_W = 480, CANVAS_H = 480;
const ctxStub = {
  createImageData: (w, h) => ({ data: new Uint8ClampedArray(w * h * 4), width: w, height: h }),
  putImageData() {},
};
const els = new Proxy({}, {
  get(t, k) {
    if (!(k in t)) {
      t[k] = {
        textContent: "", value: "1200", style: {}, dataset: {},
        classList: { toggle() {}, add() {}, remove() {}, contains: () => false },
        addEventListener() {}, querySelectorAll: () => [], closest: () => null,
        getContext: () => ctxStub,
        getBoundingClientRect: () => ({ left: 0, top: 0, width: CANVAS_W, height: CANVAS_H }),
      };
    }
    return t[k];
  },
});
global.document = {
  getElementById: (id) => els[id],
  querySelector: (s) => (s === ".canvas-wrap" ? { clientWidth: CANVAS_W, clientHeight: CANVAS_H } : els[s]),
  querySelectorAll: () => [],
  addEventListener() {},
};
global.window = { addEventListener() {} };
global.performance = { now: () => Date.now() };
// Minimal localStorage so the save/restore path is exercisable.
global.localStorage = (() => {
  const m = new Map();
  return {
    getItem: (k) => (m.has(k) ? m.get(k) : null),
    setItem: (k, v) => m.set(k, String(v)),
    removeItem: (k) => m.delete(k),
  };
})();
global.requestAnimationFrame = () => 0;
// Deterministic PRNG so runs are reproducible.
global.Math.random = (() => {
  let s = 0x2545f491;
  return () => (s = (s * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff;
})();

const SRC = fs
  .readFileSync(path.join(__dirname, "..", "js", "main.js"), "utf8")
  .replace("requestAnimationFrame(loop);", "/* test harness: loop disabled */;");

// ---- checks (appended to main.js's source so they share module scope) -----
const CHECKS = String.raw`
{
  let failed = 0;
  const ok = (name, cond, detail) => {
    console.log((cond ? "  PASS " : "  FAIL ") + name + (detail ? "  (" + detail + ")" : ""));
    if (!cond) failed++;
  };
  const fresh = () => { allocate(80, 80); }; // grid cells, not pixels
  const paintRect = (x0, y0, x1, y1, mat) => {
    for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) setCell(x, y, mat);
  };
  const sumWater = () => { let s = 0; for (let k = 0; k < cols * rows; k++) s += comp[WATER][k] + gas[VAPOR][k]; return s; };
  const maxCellError = () => {
    let e = 0;
    for (let k = 0; k < cols * rows; k++) {
      if (solid[k] !== SOLID_NONE) continue;
      let s = 0;
      for (let c = 0; c < N_COND; c++) s += comp[c][k];
      for (let c = 0; c < N_GAS; c++) s += gas[c][k];
      e = Math.max(e, Math.abs(s - 1));
    }
    return e;
  };
  const surfaceProfile = (floorY) => {
    const p = [];
    for (let x = 0; x < cols; x++) {
      let top = floorY;
      for (let y = 0; y < floorY; y++) {
        const k = idx(x, y);
        if (solid[k] === SOLID_NONE && comp[WATER][k] > 0.5) { top = y; break; }
      }
      p.push(floorY - top);
    }
    return p;
  };

  // --- 1. water levels off, conserves, and does not strobe -----------------
  console.log("water leveling + strobe");
  fresh();
  {
    const floorY = rows - 3;
    paintRect(0, floorY, cols - 1, rows - 1, "stone");
    paintRect(4, floorY - 40, 9, floorY - 1, "water");
    const w0 = sumWater();
    for (let t = 0; t < 1500; t++) step();
    const prof = surfaceProfile(floorY);
    const wet = prof.filter((v) => v > 0);
    const first = prof.findIndex((v) => v > 0);
    const last = prof.length - 1 - [...prof].reverse().findIndex((v) => v > 0);
    const interior = prof.slice(first + 3, last - 2);
    const flat = Math.max(...interior) - Math.min(...interior);

    ok("water spreads across the floor", wet.length >= 40, wet.length + " cols");
    ok("interior surface is level (delta <= 2)", flat <= 2, "delta " + flat);
    ok("water mass conserved", Math.abs(sumWater() - w0) < 1e-2, "drift " + (sumWater() - w0).toExponential(2));

    // strobe: count cells that flip wet/dry repeatedly over a short window
    const wetMap = () => {
      const m = new Uint8Array(cols * rows);
      for (let k = 0; k < cols * rows; k++) m[k] = comp[WATER][k] > 0.4 ? 1 : 0;
      return m;
    };
    const flips = new Uint32Array(cols * rows);
    let prev = wetMap();
    for (let t = 0; t < 40; t++) {
      step();
      const cur = wetMap();
      for (let k = 0; k < cur.length; k++) if (cur[k] !== prev[k]) flips[k]++;
      prev = cur;
    }
    let strobing = 0;
    for (let k = 0; k < flips.length; k++) if (flips[k] >= 6) strobing++;
    // A handful of edge cells may still wobble by one cell as the last of the
    // spread settles; the bug we're guarding against was hundreds strobing.
    ok("settled water does not strobe", strobing <= 8, strobing + " flipping cells");
  }

  // --- 2. dry powder still forms a pile (movement not over-fluidised) ------
  console.log("powder pile");
  fresh();
  {
    const floorY = rows - 2;
    paintRect(0, floorY, cols - 1, rows - 1, "stone");
    for (let y = 4; y < 24; y++) for (let x = (cols >> 1); x < (cols >> 1) + 3; x++) setCell(x, y, "sand");
    for (let t = 0; t < 600; t++) step();
    let base = 0, peak = 0;
    for (let x = 0; x < cols; x++) {
      let h = 0;
      for (let y = 0; y < floorY; y++) if (comp[SAND][idx(x, y)] > 0.5) h++;
      if (h > 0) base++;
      if (h > peak) peak = h;
    }
    ok("sand spreads but stays a heap", base > peak && peak > 3, "base " + base + " peak " + peak);
  }

  // --- 3. closed water cycle: liquid + vapour conserved over a full cycle --
  console.log("closed water cycle");
  fresh();
  {
    dayTicks = 600; // shorter days so the check sees a couple of full cycles fast
    clockTick = 0;
    buildJar();
    if (!lidSealed) toggleLidSeal();
    const F = rows - Math.max(3, Math.floor(rows * 0.05));
    for (let y = F - 10; y < F; y++) for (let x = 8; x < cols - 8; x++) setCell(x, y, "water");
    const w0 = sumWater();
    let drift = 0, cellErr = 0, peakVapour = 0;
    for (let t = 0; t < dayTicks * 3; t++) {
      step();
      drift = Math.max(drift, Math.abs(sumWater() - w0));
      cellErr = Math.max(cellErr, maxCellError());
      let v = 0;
      for (let k = 0; k < cols * rows; k++) v += gas[VAPOR][k];
      peakVapour = Math.max(peakVapour, v);
    }
    let onGlass = 0;
    for (let y = 0; y < F - 10; y++) for (let x = 0; x < cols; x++) {
      const k = idx(x, y);
      if (solid[k] !== SOLID_NONE || comp[WATER][k] < 0.02) continue;
      for (const [dx, dy] of NEIGHBORS4) {
        const nx = x + dx, ny = y + dy;
        if (inBounds(nx, ny) && solid[idx(nx, ny)] === SOLID_GLASS) { onGlass += comp[WATER][k]; break; }
      }
    }
    ok("liquid + vapour conserved across the cycle", drift < 1e-2, "drift " + drift.toExponential(2));
    ok("cells stay normalised to 1", cellErr < 1e-3, "max err " + cellErr.toExponential(2));
    ok("evaporation actually runs", peakVapour > 0.5, "peak vapour " + peakVapour.toFixed(2));
    ok("dew collects on the glass", onGlass > 0.3, onGlass.toFixed(2));
  }

  // --- 4. save / restore round-trips the scene ---------------------------
  console.log("save / restore");
  fresh();
  {
    const F = rows - 3;
    paintRect(0, F, cols - 1, rows - 1, "stone");
    paintRect(10, F - 12, cols - 10, F - 1, "water");
    buildJar();
    if (!lidSealed) toggleLidSeal();
    for (let t = 0; t < 200; t++) step();

    const before = [];
    for (let c = 0; c < N_COND; c++) before.push(Float32Array.from(comp[c]));
    for (let c = 0; c < N_GAS; c++) before.push(Float32Array.from(gas[c]));
    const solidBefore = Uint8Array.from(solid);
    const clockBefore = clockTick, sealBefore = lidSealed;

    saveState();
    fresh();                 // wipe to a blank grid
    const restored = loadState();

    let maxDiff = 0, solidMismatch = 0;
    const chans = [];
    for (let c = 0; c < N_COND; c++) chans.push(comp[c]);
    for (let c = 0; c < N_GAS; c++) chans.push(gas[c]);
    for (let ch = 0; ch < chans.length; ch++)
      for (let i = 0; i < cols * rows; i++) maxDiff = Math.max(maxDiff, Math.abs(chans[ch][i] - before[ch][i]));
    for (let i = 0; i < cols * rows; i++) if (solid[i] !== solidBefore[i]) solidMismatch++;

    ok("loadState restores the snapshot", restored, restored ? "" : "returned false");
    ok("solids come back exactly", solidMismatch === 0, solidMismatch + " cells differ");
    ok("fractions come back within quantisation error", maxDiff < 0.01, "max diff " + maxDiff.toFixed(4));
    ok("clock + lid state come back", clockTick === clockBefore && lidSealed === sealBefore,
       "clock " + clockTick + "/" + clockBefore + " lid " + lidSealed + "/" + sealBefore);
  }

  // --- 5. rough perf budget --------------------------------------------
  console.log("performance");
  fresh();
  {
    const F = rows - 2;
    for (let x = 0; x < cols; x++) for (let y = F - 30; y < rows; y++) setCell(x, y, y % 3 ? "water" : "sand");
    const t0 = Date.now();
    for (let t = 0; t < 200; t++) step();
    const ms = (Date.now() - t0) / 200;
    ok("step() under 16ms at " + cols + "x" + rows, ms < 16, ms.toFixed(2) + " ms/step");
  }

  console.log("");
  console.log(failed === 0 ? "ALL CHECKS PASSED" : failed + " CHECK(S) FAILED");
  // main.js starts a setInterval autosave timer that would otherwise keep the
  // process alive, so exit explicitly.
  if (typeof process !== "undefined") process.exit(failed === 0 ? 0 : 1);
}
`;

eval(SRC + CHECKS);
