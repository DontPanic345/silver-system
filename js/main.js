"use strict";

/**
 * Falling Sand — a continuous-composition cellular automaton.
 *
 * Unlike a classic falling-sand sim (one material id per cell), every cell
 * here holds a *mixture*: fractional amounts of four condensed substances
 * (sand, clay, biomass, water) plus four gases (N2, O2, CO2, smoke). The
 * eight fractions in a cell always sum to 1 — a cell is never "empty", it's
 * just mostly air. Stone and wood are the only pure, immovable materials.
 *
 * Movement is a single density-ordered swap rule applied uniformly to every
 * non-solid cell (dense mixtures sink, buoyant ones rise), and a separate
 * diffusion pass exchanges small amounts of composition between neighbors
 * every tick. That diffusion is what lets sand + clay + biomass + water
 * settle into "soil" over time, purely as an emergent blend — there's no
 * dedicated soil material, just a composition that renders and behaves like
 * one.
 *
 * The grid scans bottom-to-top (so nothing falls through a cell it just
 * vacated this frame) and alternates scan direction left/right per row per
 * frame to avoid directional bias.
 */

// ---- Substances --------------------------------------------------------

// Condensed (liquid/solid-ish) channels — these plus the gas channels
// below always sum to 1 for a given cell.
const SAND = 0, CLAY = 1, BIOMASS = 2, WATER = 3;
const N_COND = 4;

// Gas channels.
const N2 = 0, O2 = 1, CO2 = 2, SMOKE = 3;
const N_GAS = 4;

// Discrete, immovable, non-mixing materials (stored separately — they own
// the whole cell, not a fraction of it).
const SOLID_NONE = 0, SOLID_STONE = 1, SOLID_WOOD = 2;

// Arbitrary density units; higher sinks through lower. Biomass floats on
// water (like leaves/debris), smoke is buoyant (negative — rises through air).
const COND_DENSITY = [5, 6, 0.7, 1]; // sand, clay, biomass, water
const GAS_DENSITY = [0.12, 0.13, 0.18, -0.4]; // n2, o2, co2, smoke

// Sand/clay/biomass colors, indexed by their channel constant (0..2).
const EARTH_COLOR = [
  [217, 177, 88], // sand
  [150, 96, 64],  // clay
  [95, 130, 60],  // biomass
];
// Water renders as vivid blue in open water, but as a dark, low-saturation
// tint when it's soaked into solids — real wet dirt reads as darker dirt,
// not blue-gray. Which one applies is a function of how much solid grit
// shares the cell (see `waterTint` in render()), not of the water fraction.
const WATER_OPEN_COLOR = [58, 160, 255];
const WATER_MUD_TINT = [45, 50, 45];
const SOLID_COLOR = { [SOLID_STONE]: [138, 143, 152], [SOLID_WOOD]: [138, 90, 52] };
const BG_COLOR = [18, 20, 26];
const SMOKE_COLOR = [130, 130, 130];
const CO2_TINT = [150, 140, 110];
const FIRE_GLOW = [255, 120, 40];

// Default atmosphere: every cell starts full of this, not "nothing".
const ATMOSPHERE = [0.78, 0.21, 0.01, 0]; // N2, O2, CO2, smoke

const DIFFUSE_K = 0.14; // condensed<->condensed / gas<->gas mixing rate per tick
const GAS_LEAK_K = 0.1; // gas mixing rate across a condensed/air boundary
const VISCOSITY_SKIP = 0.5; // chance a "wet" (soil-like) cell skips a movement attempt
const EPS = 0.02;

const MAT_INFO = {
  sand: { kind: "cond", ch: SAND },
  clay: { kind: "cond", ch: CLAY },
  biomass: { kind: "cond", ch: BIOMASS },
  water: { kind: "cond", ch: WATER },
  soil: { kind: "mix", mix: [0.35, 0.25, 0.15, 0.25] }, // pre-mixed preset, same substances
  stone: { kind: "solid", id: SOLID_STONE },
  wood: { kind: "solid", id: SOLID_WOOD },
  fire: { kind: "fire" },
  empty: { kind: "air" },
};

// ---- Canvas / grid setup ----------------------------------------------

const canvas = document.getElementById("sim");
const ctx = canvas.getContext("2d", { alpha: false });
const wrap = document.querySelector(".canvas-wrap");

const CELL_PX = 6; // displayed size of one simulation cell, in CSS pixels

let cols = 0;
let rows = 0;
// Per-cell state, all typed arrays sized cols*rows (structure-of-arrays for speed).
let comp; // [N_COND] Float32Array — condensed fractions
let gas; // [N_GAS] Float32Array — gas fractions
let solid; // Uint8Array — SOLID_NONE / SOLID_STONE / SOLID_WOOD
let woodHp; // Uint8Array — remaining fuel for burning wood
let burning; // Uint8Array — 0/1 flag, applies to wood or biomass-bearing cells
let variant; // Uint8Array — stable per-cell render jitter
let moved; // Uint8Array — scratch: already simulated this frame?
let imageData, pixels;

function idx(x, y) {
  return y * cols + x;
}

function inBounds(x, y) {
  return x >= 0 && x < cols && y >= 0 && y < rows;
}

function allocate(newCols, newRows) {
  cols = newCols;
  rows = newRows;
  const n = cols * rows;

  comp = [new Float32Array(n), new Float32Array(n), new Float32Array(n), new Float32Array(n)];
  gas = [new Float32Array(n), new Float32Array(n), new Float32Array(n), new Float32Array(n)];
  solid = new Uint8Array(n);
  woodHp = new Uint8Array(n);
  burning = new Uint8Array(n);
  variant = new Uint8Array(n);
  moved = new Uint8Array(n);

  for (let i = 0; i < n; i++) {
    gas[N2][i] = ATMOSPHERE[N2];
    gas[O2][i] = ATMOSPHERE[O2];
    gas[CO2][i] = ATMOSPHERE[CO2];
    gas[SMOKE][i] = ATMOSPHERE[SMOKE];
    variant[i] = (Math.random() * 16) | 0;
  }

  canvas.width = cols;
  canvas.height = rows;
  canvas.style.width = cols * CELL_PX + "px";
  canvas.style.height = rows * CELL_PX + "px";

  imageData = ctx.createImageData(cols, rows);
  pixels = imageData.data;
}

function resize() {
  const w = Math.max(1, wrap.clientWidth);
  const h = Math.max(1, wrap.clientHeight);
  const newCols = Math.max(10, Math.floor(w / CELL_PX));
  const newRows = Math.max(10, Math.floor(h / CELL_PX));
  if (newCols === cols && newRows === rows) return;

  // Preserve existing content when the canvas grows/shrinks, anchored to
  // the bottom-left so the user doesn't lose their scene on a resize.
  const old = { comp, gas, solid, woodHp, burning };
  const oldCols = cols;
  const oldRows = rows;

  allocate(newCols, newRows);

  if (old.solid) {
    const yOffset = rows - oldRows;
    for (let y = 0; y < oldRows; y++) {
      const ny = y + yOffset;
      if (ny < 0 || ny >= rows) continue;
      for (let x = 0; x < oldCols && x < cols; x++) {
        const oi = y * oldCols + x;
        const ni = idx(x, ny);
        for (let c = 0; c < N_COND; c++) comp[c][ni] = old.comp[c][oi];
        for (let c = 0; c < N_GAS; c++) gas[c][ni] = old.gas[c][oi];
        solid[ni] = old.solid[oi];
        woodHp[ni] = old.woodHp[oi];
        burning[ni] = old.burning[oi];
      }
    }
  }
}

// ---- Cell helpers -------------------------------------------------------

function mt(i) {
  return comp[SAND][i] + comp[CLAY][i] + comp[BIOMASS][i] + comp[WATER][i];
}

function density(i) {
  return (
    comp[SAND][i] * COND_DENSITY[SAND] +
    comp[CLAY][i] * COND_DENSITY[CLAY] +
    comp[BIOMASS][i] * COND_DENSITY[BIOMASS] +
    comp[WATER][i] * COND_DENSITY[WATER] +
    gas[N2][i] * GAS_DENSITY[N2] +
    gas[O2][i] * GAS_DENSITY[O2] +
    gas[CO2][i] * GAS_DENSITY[CO2] +
    gas[SMOKE][i] * GAS_DENSITY[SMOKE]
  );
}

function clearCell(i) {
  comp[SAND][i] = 0; comp[CLAY][i] = 0; comp[BIOMASS][i] = 0; comp[WATER][i] = 0;
  gas[N2][i] = ATMOSPHERE[N2]; gas[O2][i] = ATMOSPHERE[O2]; gas[CO2][i] = ATMOSPHERE[CO2]; gas[SMOKE][i] = ATMOSPHERE[SMOKE];
  solid[i] = SOLID_NONE;
  woodHp[i] = 0;
  burning[i] = 0;
}

// ---- Drawing (input) --------------------------------------------------

let currentMaterial = "sand";
let brushSize = 5;
let isPointerDown = false;
let lastPointerCell = null;

function setCell(x, y, matName) {
  if (!inBounds(x, y)) return;
  const i = idx(x, y);
  const info = MAT_INFO[matName];
  variant[i] = (Math.random() * 16) | 0;

  if (info.kind === "air") {
    clearCell(i);
    return;
  }
  if (info.kind === "solid") {
    clearCell(i);
    solid[i] = info.id;
    if (info.id === SOLID_WOOD) woodHp[i] = 60;
    return;
  }
  if (info.kind === "fire") {
    if (solid[i] === SOLID_WOOD || comp[BIOMASS][i] > 0.02) burning[i] = 1;
    return;
  }
  // "cond" (pure substance) or "mix" (preset blend)
  solid[i] = SOLID_NONE;
  woodHp[i] = 0;
  burning[i] = 0;
  if (info.kind === "cond") {
    comp[SAND][i] = info.ch === SAND ? 1 : 0;
    comp[CLAY][i] = info.ch === CLAY ? 1 : 0;
    comp[BIOMASS][i] = info.ch === BIOMASS ? 1 : 0;
    comp[WATER][i] = info.ch === WATER ? 1 : 0;
  } else {
    comp[SAND][i] = info.mix[SAND];
    comp[CLAY][i] = info.mix[CLAY];
    comp[BIOMASS][i] = info.mix[BIOMASS];
    comp[WATER][i] = info.mix[WATER];
  }
  gas[N2][i] = 0; gas[O2][i] = 0; gas[CO2][i] = 0; gas[SMOKE][i] = 0;
}

function stampBrush(cx, cy) {
  const r = brushSize;
  const r2 = r * r;
  const sparse = currentMaterial === "sand" || currentMaterial === "clay" || currentMaterial === "water";
  for (let dy = -r; dy <= r; dy++) {
    for (let dx = -r; dx <= r; dx++) {
      if (dx * dx + dy * dy > r2) continue;
      if (sparse && Math.random() < 0.25 && (dx || dy)) continue;
      setCell(cx + dx, cy + dy, currentMaterial);
    }
  }
}

function pointerToCell(evt) {
  const rect = canvas.getBoundingClientRect();
  const px = (evt.clientX - rect.left) / rect.width;
  const py = (evt.clientY - rect.top) / rect.height;
  return [Math.floor(px * cols), Math.floor(py * rows)];
}

function lineStamp([x0, y0], [x1, y1]) {
  // Bresenham so fast drags don't leave gaps in the brush stroke.
  let dx = Math.abs(x1 - x0), dy = -Math.abs(y1 - y0);
  let sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1;
  let err = dx + dy;
  let x = x0, y = y0;
  for (let guard = 0; guard < 10000; guard++) {
    stampBrush(x, y);
    if (x === x1 && y === y1) break;
    const e2 = 2 * err;
    if (e2 >= dy) { err += dy; x += sx; }
    if (e2 <= dx) { err += dx; y += sy; }
  }
}

function handlePointerDown(evt) {
  isPointerDown = true;
  lastPointerCell = pointerToCell(evt);
  stampBrush(...lastPointerCell);
  evt.preventDefault();
}

function handlePointerMove(evt) {
  if (!isPointerDown) return;
  const cell = pointerToCell(evt);
  lineStamp(lastPointerCell, cell);
  lastPointerCell = cell;
  evt.preventDefault();
}

function handlePointerUp() {
  isPointerDown = false;
  lastPointerCell = null;
}

canvas.addEventListener("mousedown", handlePointerDown);
window.addEventListener("mousemove", handlePointerMove);
window.addEventListener("mouseup", handlePointerUp);

canvas.addEventListener(
  "touchstart",
  (e) => { if (e.touches[0]) handlePointerDown(e.touches[0]); },
  { passive: false }
);
canvas.addEventListener(
  "touchmove",
  (e) => { if (e.touches[0]) handlePointerMove(e.touches[0]); },
  { passive: false }
);
window.addEventListener("touchend", handlePointerUp);

// ---- Simulation step: movement -----------------------------------------

function swapCells(i, j) {
  for (let c = 0; c < N_COND; c++) { const t = comp[c][i]; comp[c][i] = comp[c][j]; comp[c][j] = t; }
  for (let c = 0; c < N_GAS; c++) { const t = gas[c][i]; gas[c][i] = gas[c][j]; gas[c][j] = t; }
  const b = burning[i]; burning[i] = burning[j]; burning[j] = b;
  moved[i] = 1;
  moved[j] = 1;
}

function tryMove(x, y, dx, dy, dens) {
  const nx = x + dx, ny = y + dy;
  if (!inBounds(nx, ny)) return false;
  const i = idx(x, y);
  const j = idx(nx, ny);
  if (moved[j] || solid[j] !== SOLID_NONE) return false;
  if (dens > density(j) + EPS) {
    swapCells(i, j);
    return true;
  }
  return false;
}

let frameParity = 0;

function stepCell(x, y, dirs) {
  const i = idx(x, y);
  if (moved[i] || solid[i] !== SOLID_NONE) return;

  const m = mt(i);
  const waterFrac = m > 0.01 ? comp[WATER][i] / m : 0;
  const isLiquid = m > 0.05 && waterFrac > 0.55;
  const isWet = m > 0.05 && waterFrac > 0.12 && waterFrac <= 0.55;
  if (isWet && Math.random() < VISCOSITY_SKIP) return;

  const dens = density(i);

  if (tryMove(x, y, 0, 1, dens)) return;
  for (const dx of dirs) if (tryMove(x, y, dx, 1, dens)) return;
  if (isLiquid) for (const dx of dirs) if (tryMove(x, y, dx, 0, dens)) return;

  // Buoyancy: rise past whatever's directly above if this cell is lighter
  // (this is how smoke climbs through air and gas bubbles rise through water).
  if (y > 0) {
    const up = idx(x, y - 1);
    if (solid[up] === SOLID_NONE && !moved[up] && dens < density(up) - EPS) {
      if (tryMove(x, y, 0, -1, dens)) return;
      for (const dx of dirs) if (tryMove(x, y, dx, -1, dens)) return;
    }
  }
}

function stepMovement() {
  const dirsA = [1, -1], dirsB = [-1, 1];
  for (let y = rows - 1; y >= 0; y--) {
    const leftToRight = (y + frameParity) % 2 === 0;
    const dirs = leftToRight ? dirsA : dirsB;
    if (leftToRight) {
      for (let x = 0; x < cols; x++) stepCell(x, y, dirs);
    } else {
      for (let x = cols - 1; x >= 0; x--) stepCell(x, y, dirs);
    }
  }
  frameParity ^= 1;
}

// ---- Simulation step: diffusion (this is what blends materials into soil) --

function diffusePair(i, j) {
  if (solid[i] !== SOLID_NONE || solid[j] !== SOLID_NONE) return;
  const mi = mt(i), mj = mt(j);
  if (mi > 0.05 && mj > 0.05) {
    // Both are "ground"/liquid cells: mix everything (condensed + gas) at a
    // single uniform rate. Because both cells already sum to 1, this exactly
    // conserves each cell's total — no renormalization needed.
    for (let c = 0; c < N_COND; c++) {
      const d = (comp[c][j] - comp[c][i]) * DIFFUSE_K;
      comp[c][i] += d; comp[c][j] -= d;
    }
    for (let c = 0; c < N_GAS; c++) {
      const d = (gas[c][j] - gas[c][i]) * DIFFUSE_K;
      gas[c][i] += d; gas[c][j] -= d;
    }
  } else {
    // At least one side is air-dominant. Only the *composition* of each
    // side's gas headspace mixes (what ratio of N2/O2/CO2/smoke it holds) —
    // never the absolute amount. A cell's matter/air split is decided by
    // movement and combustion, never by this pass, so each side's total gas
    // volume (its headspace, 1 - matter) is fixed going in and holds exactly
    // going out: condensed matter can't "evaporate" into a neighbor just
    // because it happens to sit next to open air.
    const gi = gas[N2][i] + gas[O2][i] + gas[CO2][i] + gas[SMOKE][i];
    const gj = gas[N2][j] + gas[O2][j] + gas[CO2][j] + gas[SMOKE][j];
    if (gi < 1e-6 || gj < 1e-6) return; // no headspace on one side — nothing to mix
    for (let c = 0; c < N_GAS; c++) {
      const ni = gas[c][i] / gi, nj = gas[c][j] / gj;
      const d = (nj - ni) * GAS_LEAK_K;
      gas[c][i] = (ni + d) * gi;
      gas[c][j] = (nj - d) * gj;
    }
  }
}

function stepDiffusion() {
  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const i = idx(x, y);
      if (x + 1 < cols) diffusePair(i, i + 1);
      if (y + 1 < rows) diffusePair(i, i + cols);
    }
  }
}

// ---- Simulation step: combustion ----------------------------------------

const NEIGHBORS8 = [[1, 0], [-1, 0], [0, 1], [0, -1], [1, 1], [-1, 1], [1, -1], [-1, -1]];
const BURN_RATE = 0.05;
const IGNITE_CHANCE = 0.028;

function stepCombustion() {
  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const i = idx(x, y);
      if (!burning[i]) continue;

      if (solid[i] === SOLID_WOOD) {
        if (woodHp[i] > 0 && Math.random() < 0.5) woodHp[i]--;
        if (Math.random() < 0.5) {
          // Puff smoke + CO2 into a random neighbor, converting some of its
          // own N2/O2 — never adding mass, just changing what's in the air.
          // Capping `take` by what's actually there to convert keeps this
          // exact even against a packed neighbor with little or no headspace.
          const [dx, dy] = NEIGHBORS8[(Math.random() * NEIGHBORS8.length) | 0];
          const nx = x + dx, ny = y + dy;
          if (inBounds(nx, ny)) {
            const j = idx(nx, ny);
            if (solid[j] === SOLID_NONE) {
              const take = Math.min(0.05, gas[N2][j], gas[O2][j]);
              if (take > 0.0005) {
                gas[N2][j] -= take * 0.5;
                gas[O2][j] -= take * 0.5;
                gas[SMOKE][j] += take * 0.6;
                gas[CO2][j] += take * 0.4;
              }
            }
          }
        }
        if (woodHp[i] <= 0) {
          clearCell(i);
          gas[SMOKE][i] = 0.7; gas[CO2][i] = 0.2; gas[N2][i] = 0.1; gas[O2][i] = 0;
          continue;
        }
      } else {
        const burn = Math.min(comp[BIOMASS][i], BURN_RATE);
        comp[BIOMASS][i] -= burn;
        gas[CO2][i] += burn * 0.6;
        gas[SMOKE][i] += burn * 0.4;
        if (comp[WATER][i] > 0.2 || comp[BIOMASS][i] <= 0.005) {
          burning[i] = 0;
          continue;
        }
      }

      // Spread to flammable neighbors.
      for (const [dx, dy] of NEIGHBORS8) {
        const nx = x + dx, ny = y + dy;
        if (!inBounds(nx, ny)) continue;
        const j = idx(nx, ny);
        if (burning[j]) continue;
        const flammable = solid[j] === SOLID_WOOD || comp[BIOMASS][j] > 0.05;
        if (!flammable) continue;
        const wet = solid[j] === SOLID_NONE && mt(j) > 0.05 && comp[WATER][j] / mt(j) > 0.25;
        if (wet) continue;
        if (Math.random() < IGNITE_CHANCE) burning[j] = 1;
      }
    }
  }
}

function step() {
  moved.fill(0);
  stepMovement();
  stepDiffusion();
  stepCombustion();
}

// ---- Render --------------------------------------------------------------

function clamp8(v) {
  return v < 0 ? 0 : v > 255 ? 255 : v;
}

function render() {
  let count = 0;
  const n = cols * rows;
  for (let i = 0; i < n; i++) {
    const o = i * 4;
    const jitter = (variant[i] - 8) * 1.5;

    if (solid[i] === SOLID_STONE) {
      const c = SOLID_COLOR[SOLID_STONE];
      pixels[o] = clamp8(c[0] + jitter); pixels[o + 1] = clamp8(c[1] + jitter); pixels[o + 2] = clamp8(c[2] + jitter); pixels[o + 3] = 255;
      count++;
      continue;
    }
    if (solid[i] === SOLID_WOOD) {
      const c = SOLID_COLOR[SOLID_WOOD];
      const glow = burning[i] ? 0.5 + 0.5 * Math.sin(i * 0.7 + performance.now() * 0.02) : 0;
      const r = c[0] * (1 - glow) + FIRE_GLOW[0] * glow;
      const g = c[1] * (1 - glow) + FIRE_GLOW[1] * glow;
      const b = c[2] * (1 - glow) + FIRE_GLOW[2] * glow;
      pixels[o] = clamp8(r + jitter); pixels[o + 1] = clamp8(g + jitter); pixels[o + 2] = clamp8(b + jitter); pixels[o + 3] = 255;
      count++;
      continue;
    }

    const m = mt(i);
    if (m < 0.04) {
      const smoke = Math.min(1, gas[SMOKE][i] * 3);
      const co2 = Math.min(1, gas[CO2][i] * 2);
      let r = BG_COLOR[0] + (SMOKE_COLOR[0] - BG_COLOR[0]) * smoke + (CO2_TINT[0] - BG_COLOR[0]) * co2 * (1 - smoke);
      let g = BG_COLOR[1] + (SMOKE_COLOR[1] - BG_COLOR[1]) * smoke + (CO2_TINT[1] - BG_COLOR[1]) * co2 * (1 - smoke);
      let b = BG_COLOR[2] + (SMOKE_COLOR[2] - BG_COLOR[2]) * smoke + (CO2_TINT[2] - BG_COLOR[2]) * co2 * (1 - smoke);
      pixels[o] = clamp8(r); pixels[o + 1] = clamp8(g); pixels[o + 2] = clamp8(b); pixels[o + 3] = 255;
      continue;
    }

    count++;
    // How much solid grit shares this cell decides whether water reads as
    // open water (blue) or as dampness soaked into the grit (dark, muted) —
    // that's what keeps a sand+clay+biomass+water mix looking like soil
    // instead of blue-gray sludge.
    const solidsTotal = comp[SAND][i] + comp[CLAY][i] + comp[BIOMASS][i];
    const openness = Math.max(0, Math.min(1, 1 - solidsTotal * 3));
    const wr = WATER_MUD_TINT[0] + (WATER_OPEN_COLOR[0] - WATER_MUD_TINT[0]) * openness;
    const wg = WATER_MUD_TINT[1] + (WATER_OPEN_COLOR[1] - WATER_MUD_TINT[1]) * openness;
    const wb = WATER_MUD_TINT[2] + (WATER_OPEN_COLOR[2] - WATER_MUD_TINT[2]) * openness;

    let r = (comp[SAND][i] * EARTH_COLOR[SAND][0] + comp[CLAY][i] * EARTH_COLOR[CLAY][0] + comp[BIOMASS][i] * EARTH_COLOR[BIOMASS][0] + comp[WATER][i] * wr) / m;
    let g = (comp[SAND][i] * EARTH_COLOR[SAND][1] + comp[CLAY][i] * EARTH_COLOR[CLAY][1] + comp[BIOMASS][i] * EARTH_COLOR[BIOMASS][1] + comp[WATER][i] * wg) / m;
    let b = (comp[SAND][i] * EARTH_COLOR[SAND][2] + comp[CLAY][i] * EARTH_COLOR[CLAY][2] + comp[BIOMASS][i] * EARTH_COLOR[BIOMASS][2] + comp[WATER][i] * wb) / m;

    // Fade toward background as the cell becomes more air (thin mixtures).
    r = BG_COLOR[0] + (r - BG_COLOR[0]) * m;
    g = BG_COLOR[1] + (g - BG_COLOR[1]) * m;
    b = BG_COLOR[2] + (b - BG_COLOR[2]) * m;

    if (burning[i]) {
      const glow = 0.4 + 0.3 * Math.sin(i * 0.9 + performance.now() * 0.03);
      r = r * (1 - glow) + FIRE_GLOW[0] * glow;
      g = g * (1 - glow) + FIRE_GLOW[1] * glow;
      b = b * (1 - glow) + FIRE_GLOW[2] * glow;
    }

    pixels[o] = clamp8(r + jitter);
    pixels[o + 1] = clamp8(g + jitter);
    pixels[o + 2] = clamp8(b + jitter);
    pixels[o + 3] = 255;
  }
  ctx.putImageData(imageData, 0, 0);
  document.getElementById("count").textContent = count;
}

// ---- Main loop -------------------------------------------------------

let running = true;
let lastFrameTime = performance.now();
let fpsAccum = 0;
let fpsFrames = 0;
let fpsTimer = 0;

function loop(now) {
  const dt = now - lastFrameTime;
  lastFrameTime = now;

  fpsAccum += dt;
  fpsFrames++;
  fpsTimer += dt;
  if (fpsTimer > 500) {
    document.getElementById("fps").textContent = Math.round((fpsFrames * 1000) / fpsAccum);
    fpsAccum = 0;
    fpsFrames = 0;
    fpsTimer = 0;
  }

  if (running) {
    step();
    render();
  }
  requestAnimationFrame(loop);
}

// ---- UI wiring -------------------------------------------------------

function setActiveMaterialButton() {
  document.querySelectorAll(".mat-btn").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.mat === currentMaterial);
  });
}

document.getElementById("materials").addEventListener("click", (e) => {
  const btn = e.target.closest(".mat-btn");
  if (!btn) return;
  currentMaterial = btn.dataset.mat;
  setActiveMaterialButton();
});

const brushSlider = document.getElementById("brushSize");
const brushSizeVal = document.getElementById("brushSizeVal");
brushSlider.addEventListener("input", () => {
  brushSize = parseInt(brushSlider.value, 10);
  brushSizeVal.textContent = brushSize;
});

const pauseBtn = document.getElementById("pauseBtn");
pauseBtn.addEventListener("click", () => {
  running = !running;
  pauseBtn.textContent = running ? "Pause" : "Resume";
  pauseBtn.classList.toggle("active", !running);
});

document.getElementById("clearBtn").addEventListener("click", () => {
  for (let i = 0; i < cols * rows; i++) clearCell(i);
});

window.addEventListener("keydown", (e) => {
  if (e.code === "Space") {
    e.preventDefault();
    pauseBtn.click();
  }
});

let resizeTimer = null;
window.addEventListener("resize", () => {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(resize, 120);
});

// ---- Boot --------------------------------------------------------------

resize();
setActiveMaterialButton();
requestAnimationFrame(loop);
