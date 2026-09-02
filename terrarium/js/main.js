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
const N2 = 0, O2 = 1, CO2 = 2, SMOKE = 3, VAPOR = 4;
const N_GAS = 5;

// Discrete, immovable, non-mixing materials (stored separately — they own
// the whole cell, not a fraction of it). GLASS behaves exactly like stone
// for movement/combustion — it's tracked as its own id purely so render can
// draw it translucent, and so later phases can treat the inner surface as a
// condensation site.
const SOLID_NONE = 0, SOLID_STONE = 1, SOLID_WOOD = 2, SOLID_GLASS = 3;

// Arbitrary density units; higher sinks through lower. Biomass floats on
// water (like leaves/debris), smoke is buoyant (negative — rises through air).
const COND_DENSITY = [5, 6, 0.7, 1]; // sand, clay, biomass, water
// Water vapour is buoyant like smoke (warm, humid air rises) but a touch
// less so — it's what carries evaporated water up to the cool glass.
const GAS_DENSITY = [0.12, 0.13, 0.18, -0.4, -0.3]; // n2, o2, co2, smoke, vapor

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
const GLASS_COLOR = [178, 206, 214]; // pale blue-grey; blended lightly over the background
const GLASS_ALPHA = 0.28;
const BG_COLOR = [18, 20, 26];
const SMOKE_COLOR = [130, 130, 130];
const VAPOR_FOG = [120, 140, 165]; // cool pale haze for humid air
const GLASS_FOG = [206, 222, 230]; // pale bloom on the pane where vapour/dew gathers
const CO2_TINT = [150, 140, 110];
const FIRE_GLOW = [255, 120, 40];

// Default atmosphere: every cell starts full of this, not "nothing".
const ATMOSPHERE = [0.78, 0.21, 0.01, 0, 0]; // N2, O2, CO2, smoke, vapor

const DIFFUSE_K = 0.14; // condensed<->condensed / gas<->gas mixing rate per tick
const GAS_LEAK_K = 0.1; // gas mixing rate across a condensed/air boundary
const VISCOSITY_SKIP = 0.5; // chance a "wet" (soil-like) cell skips a movement attempt
const EPS = 0.02;

// ---- Closed water cycle (Phase 1) -----------------------------------
// Liquid water with open headspace above it turns into VAPOR gas in the same
// cell (mass just moves cond -> gas, so the cell still sums to 1); the vapour
// rises on its own via the buoyancy rule. Where vapour touches the cool inner
// glass it condenses back to liquid in the cell beside the pane, which then
// runs down the glass and pools using the ordinary gravity code. Evaporation
// tracks the light/heat of the day; condensation is stronger at night.
const EVAP_BASE = 0.00022;   // per tick, at full daylight, for fully-exposed water
const EVAP_NIGHT = 0.25;    // fraction of that rate still running at midnight
const EVAP_SAT = 0.09;      // local vapour fraction at which the air is "full" and evaporation stops
const CONDENSE_BASE = 0.09; // per tick fraction of a cell's vapour that condenses on glass
const CONDENSE_NIGHT_BONUS = 2.5; // condensation multiplier swing from noon->midnight
const VAPOR_SUPERSAT = 0.02; // vapour fraction above which it also condenses as mid-air mist
const VAPOR_MIST_K = 0.9;  // how fast that excess mist forms

// ---- Day/night clock -------------------------------------------------
// A single global clock advances one unit per simulation tick and wraps
// every `dayTicks`. `lightLevel` (0 at midnight, 1 at noon, smooth in
// between) is the one value everything downstream reads — render dims the
// scene by it now; later phases scale evaporation and plant growth by it.
let dayTicks = 1200;
let clockTick = dayTicks / 2; // start at high noon, not midnight
let lightLevel = 1;

function updateClock() {
  clockTick = (clockTick + 1) % dayTicks;
  const phase = clockTick / dayTicks; // 0..1 through the day
  lightLevel = -Math.cos(phase * Math.PI * 2) * 0.5 + 0.5;
}

const MAT_INFO = {
  sand: { kind: "cond", ch: SAND },
  clay: { kind: "cond", ch: CLAY },
  biomass: { kind: "cond", ch: BIOMASS },
  water: { kind: "cond", ch: WATER },
  soil: { kind: "mix", mix: [0.35, 0.25, 0.15, 0.25] }, // pre-mixed preset, same substances
  stone: { kind: "solid", id: SOLID_STONE },
  glass: { kind: "solid", id: SOLID_GLASS },
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

  comp = Array.from({ length: N_COND }, () => new Float32Array(n));
  gas = Array.from({ length: N_GAS }, () => new Float32Array(n));
  solid = new Uint8Array(n);
  woodHp = new Uint8Array(n);
  burning = new Uint8Array(n);
  variant = new Uint8Array(n);
  moved = new Uint8Array(n);

  for (let i = 0; i < n; i++) {
    for (let c = 0; c < N_GAS; c++) gas[c][i] = ATMOSPHERE[c];
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

function gasTotal(i) {
  return gas[N2][i] + gas[O2][i] + gas[CO2][i] + gas[SMOKE][i] + gas[VAPOR][i];
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
    gas[SMOKE][i] * GAS_DENSITY[SMOKE] +
    gas[VAPOR][i] * GAS_DENSITY[VAPOR]
  );
}

function clearCell(i) {
  comp[SAND][i] = 0; comp[CLAY][i] = 0; comp[BIOMASS][i] = 0; comp[WATER][i] = 0;
  for (let c = 0; c < N_GAS; c++) gas[c][i] = ATMOSPHERE[c];
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
  for (let c = 0; c < N_GAS; c++) gas[c][i] = 0;
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

// ---- Terrarium jar ----------------------------------------------------
// A glass enclosure stamped over the scene: two side walls, a floor, and a
// shouldered top with a neck opening in the middle that "Seal Lid" closes.

let jarNeck = null; // { x0, x1, row, thick } — the gap in the shoulder
let lidSealed = false;

function setSolidCell(x, y, id) {
  if (!inBounds(x, y)) return;
  const i = idx(x, y);
  clearCell(i);
  solid[i] = id;
  variant[i] = (Math.random() * 16) | 0;
}

function buildJar() {
  const wallThick = 2;
  const marginX = Math.max(3, Math.floor(cols * 0.06));
  const floorTop = rows - Math.max(3, Math.floor(rows * 0.05));
  const shoulderRow = Math.max(2, Math.floor(rows * 0.08));
  const neckHalf = Math.max(2, Math.floor(cols * 0.1));
  const midX = cols >> 1;
  const rightX = cols - 1 - marginX;

  for (let y = shoulderRow; y < rows; y++) {
    for (let t = 0; t < wallThick; t++) {
      setSolidCell(marginX + t, y, SOLID_GLASS);
      setSolidCell(rightX - t, y, SOLID_GLASS);
    }
  }
  for (let y = floorTop; y < rows; y++) {
    for (let x = marginX; x <= rightX; x++) setSolidCell(x, y, SOLID_GLASS);
  }
  for (let t = 0; t < wallThick; t++) {
    for (let x = marginX; x <= rightX; x++) {
      if (Math.abs(x - midX) > neckHalf) setSolidCell(x, shoulderRow + t, SOLID_GLASS);
    }
  }

  jarNeck = { x0: midX - neckHalf, x1: midX + neckHalf, row: shoulderRow, thick: wallThick };
  lidSealed = false;
}

function toggleLidSeal() {
  if (!jarNeck) return;
  lidSealed = !lidSealed;
  for (let t = 0; t < jarNeck.thick; t++) {
    for (let x = jarNeck.x0; x <= jarNeck.x1; x++) {
      if (!inBounds(x, jarNeck.row + t)) continue;
      const i = idx(x, jarNeck.row + t);
      if (lidSealed) {
        clearCell(i);
        solid[i] = SOLID_GLASS;
      } else if (solid[i] === SOLID_GLASS) {
        clearCell(i);
      }
    }
  }
}

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

// Scans along row `y` in direction `dx`, through cells a liquid parcel could
// actually travel across (open, at the same level), for the nearest column it
// could then *fall* from — i.e. where the cell one row down is displaceable.
// Returns that column's step distance (1..FLOW_REACH), or 0 if there's no such
// spot within reach. Water then moves the whole way there in one tick and
// drops, which is what lets a pile cross its own flat steps and level out
// instead of freezing into a staircase — while a genuinely flat pool or a
// single-cell ripple offers no fall point and simply comes to rest.
const FLOW_REACH = 48;
function flowStep(x, y, dx, dens) {
  for (let step = 1; step <= FLOW_REACH; step++) {
    const nx = x + dx * step;
    if (!inBounds(nx, y)) return 0;
    const head = idx(nx, y);
    if (moved[head] || solid[head] !== SOLID_NONE || density(head) >= dens - EPS) return 0;
    const belowY = y + 1;
    if (belowY < rows) {
      const b = idx(nx, belowY);
      if (!moved[b] && solid[b] === SOLID_NONE && density(b) < dens - EPS) return step;
    }
  }
  return 0;
}

function stepCell(x, y) {
  const i = idx(x, y);
  if (moved[i] || solid[i] !== SOLID_NONE) return;

  const m = mt(i);
  const waterFrac = m > 0.01 ? comp[WATER][i] / m : 0;
  const isLiquid = m > 0.05 && waterFrac > 0.55;
  const isWet = m > 0.05 && waterFrac > 0.12 && waterFrac <= 0.55;
  if (isWet && Math.random() < VISCOSITY_SKIP) return;

  const dens = density(i);

  // Straight down first — same rule for everything.
  if (tryMove(x, y, 0, 1, dens)) return;

  // Which side to try first is a *stable* checkerboard by position, not a
  // per-frame flip — flipping the preference every tick is what made falling
  // streams and settling puddles strobe left/right.
  const pref = (x + y) & 1 ? 1 : -1;

  if (isLiquid) {
    // Adjacent diagonal drop first (the common pile-edge case).
    if (tryMove(x, y, pref, 1, dens)) return;
    if (tryMove(x, y, -pref, 1, dens)) return;
    // Otherwise, if this is a surface cell, look out along the surface for a
    // spot to fall from — nearer side wins — and slide the whole way there.
    const surface = y === 0 || solid[idx(x, y - 1)] === SOLID_NONE && mt(idx(x, y - 1)) < 0.5;
    if (surface) {
      const sl = flowStep(x, y, -1, dens);
      const sr = flowStep(x, y, 1, dens);
      let f = 0, dist = 0;
      if (sl && sr) { f = sl <= sr ? -1 : 1; dist = Math.min(sl, sr); }
      else if (sl) { f = -1; dist = sl; }
      else if (sr) { f = 1; dist = sr; }
      if (f !== 0 && !moved[idx(x + f * dist, y)]) {
        swapCells(i, idx(x + f * dist, y));
        return;
      }
    }
  } else {
    if (tryMove(x, y, pref, 1, dens)) return;
    if (tryMove(x, y, -pref, 1, dens)) return;
  }

  // Buoyancy: rise past whatever's directly above if this cell is lighter
  // (this is how smoke climbs through air and gas bubbles rise through water).
  if (y > 0) {
    const up = idx(x, y - 1);
    if (solid[up] === SOLID_NONE && !moved[up] && dens < density(up) - EPS) {
      if (tryMove(x, y, 0, -1, dens)) return;
      if (tryMove(x, y, pref, -1, dens)) return;
      if (tryMove(x, y, -pref, -1, dens)) return;
    }
  }
}

function stepMovement() {
  // Scan bottom-to-top; alternate the horizontal scan direction per row and
  // per frame so the sweep order itself doesn't bias the result one way.
  for (let y = rows - 1; y >= 0; y--) {
    if ((y + frameParity) % 2 === 0) {
      for (let x = 0; x < cols; x++) stepCell(x, y);
    } else {
      for (let x = cols - 1; x >= 0; x--) stepCell(x, y);
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
    // At least one side is air-dominant. The two gas headspaces mix toward a
    // common *composition* (what ratio of N2/O2/CO2/smoke/vapour each holds),
    // but a cell's matter/air split is never touched here — movement and the
    // water cycle own that. Exchanging `d` of each species between the cells
    // (weighted by the smaller headspace, so a nearly-packed cell trades
    // little) keeps every species mass-exact *and*, because both sides'
    // concentrations sum to 1, leaves each cell's total gas volume unchanged:
    // condensed matter can't leak into a neighbour just by bordering open air.
    let gi = 0, gj = 0;
    for (let c = 0; c < N_GAS; c++) { gi += gas[c][i]; gj += gas[c][j]; }
    if (gi < 1e-6 || gj < 1e-6) return; // no headspace on one side — nothing to mix
    const h = (gi < gj ? gi : gj) * GAS_LEAK_K;
    for (let c = 0; c < N_GAS; c++) {
      const d = (gas[c][j] / gj - gas[c][i] / gi) * h;
      gas[c][i] += d;
      gas[c][j] -= d;
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
const NEIGHBORS4 = [[1, 0], [-1, 0], [0, 1], [0, -1]];
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

// ---- Simulation step: the closed water cycle (Phase 1) -----------------
// Every transfer here moves an exact amount between liquid and vapour and
// leaves each cell still summing to 1, so `Σ comp[WATER] + Σ gas[VAPOR]` is
// conserved by construction — the whole-system water total never changes,
// which is the regression check for this feature (see the headless harness).

let humidityPct = 0; // mean vapour fraction of the free air, as a percentage

function stepWaterCycle() {
  // Evaporation follows the day's heat; condensation runs cooler-and-stronger
  // at night. lightLevel is 0 at midnight, 1 at noon.
  const heat = EVAP_NIGHT + (1 - EVAP_NIGHT) * lightLevel;
  const evapRate = EVAP_BASE * heat;
  const condRate = CONDENSE_BASE * (1 + CONDENSE_NIGHT_BONUS * (1 - lightLevel));
  let airCells = 0, vaporSum = 0;

  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const i = idx(x, y);
      if (solid[i] !== SOLID_NONE) continue;

      // Track humidity over the air that actually holds moisture (the jar
      // interior, in practice) rather than diluting it with the dry room.
      if (mt(i) < 0.2 && gas[VAPOR][i] > 0.002) { airCells++; vaporSum += gas[VAPOR][i]; }

      // --- Evaporation: a liquid-water surface gives water to the air above
      // as vapour, and an equal amount of that air moves down to replace it.
      const w = comp[WATER][i];
      if (w > 0.05 && y > 0) {
        const up = idx(x, y - 1);
        const upGas = gasTotal(up);
        if (solid[up] === SOLID_NONE && mt(up) < 0.4 && upGas > 0.1) {
          // Evaporation slows to nothing as the air above approaches
          // saturation — this is what lets a sealed jar settle into a steady
          // humidity instead of drying the pool out into the air.
          const localRH = gas[VAPOR][up] / upGas;
          const sat = Math.max(0, 1 - localRH / EVAP_SAT);
          const e = Math.min(evapRate * sat, w - 0.03, upGas - 0.05);
          if (e > 0) {
            comp[WATER][i] -= e;
            for (let c = 0; c < N_GAS; c++) {
              const share = (gas[c][up] / upGas) * e;
              gas[c][up] -= share;
              gas[c][i] += share;
            }
            gas[VAPOR][up] += e;
          }
        }
      }

      // --- Condensation: vapour against the cool glass returns to liquid in
      // the cell touching the pane (it then runs down the glass and pools via
      // the ordinary gravity code). Very humid mid-air also gives up a little
      // as mist, which keeps a sealed jar from saturating without limit.
      const v = gas[VAPOR][i];
      if (v > 1e-4) {
        let onGlass = false;
        for (let k = 0; k < 4; k++) {
          const nx = x + NEIGHBORS4[k][0], ny = y + NEIGHBORS4[k][1];
          if (inBounds(nx, ny) && solid[idx(nx, ny)] === SOLID_GLASS) { onGlass = true; break; }
        }
        let c = 0;
        if (onGlass) c = v * condRate;
        else if (v > VAPOR_SUPERSAT) c = (v - VAPOR_SUPERSAT) * VAPOR_MIST_K;
        if (c > 0) {
          gas[VAPOR][i] -= c;
          comp[WATER][i] += c;
        }
      }
    }
  }

  // Expressed relative to the saturation point, so ~100% reads as "the air
  // is full and evaporation has stalled".
  humidityPct = airCells ? Math.min(100, (vaporSum / airCells / EVAP_SAT) * 100) : 0;
}

function step() {
  moved.fill(0);
  updateClock();
  stepMovement();
  stepDiffusion();
  stepCombustion();
  stepWaterCycle();
}

// ---- Render --------------------------------------------------------------

function clamp8(v) {
  return v < 0 ? 0 : v > 255 ? 255 : v;
}

const NIGHT_FLOOR = 0.16; // scene brightness at midnight relative to noon

function render() {
  let count = 0;
  const n = cols * rows;
  const ambient = NIGHT_FLOOR + (1 - NIGHT_FLOOR) * lightLevel;
  for (let i = 0; i < n; i++) {
    const o = i * 4;
    const jitter = (variant[i] - 8) * 1.5;

    if (solid[i] === SOLID_STONE) {
      const c = SOLID_COLOR[SOLID_STONE];
      pixels[o] = clamp8((c[0] + jitter) * ambient); pixels[o + 1] = clamp8((c[1] + jitter) * ambient); pixels[o + 2] = clamp8((c[2] + jitter) * ambient); pixels[o + 3] = 255;
      count++;
      continue;
    }
    if (solid[i] === SOLID_GLASS) {
      const a = GLASS_ALPHA;
      let r = BG_COLOR[0] * (1 - a) + GLASS_COLOR[0] * a;
      let g = BG_COLOR[1] * (1 - a) + GLASS_COLOR[1] * a;
      let b = BG_COLOR[2] * (1 - a) + GLASS_COLOR[2] * a;
      // Fogging: bloom the pane where adjacent air is humid or dew has
      // condensed on it — the visible half of the water cycle.
      const gx = i % cols, gy = (i - gx) / cols;
      let mist = 0;
      for (let k = 0; k < 4; k++) {
        const nx = gx + NEIGHBORS4[k][0], ny = gy + NEIGHBORS4[k][1];
        if (!inBounds(nx, ny)) continue;
        const j = idx(nx, ny);
        if (solid[j] !== SOLID_NONE) continue;
        mist += gas[VAPOR][j] * 11 + (comp[WATER][j] < 0.3 ? comp[WATER][j] : 0.3) * 2.5;
      }
      if (mist > 0.85) mist = 0.85;
      r += (GLASS_FOG[0] - r) * mist;
      g += (GLASS_FOG[1] - g) * mist;
      b += (GLASS_FOG[2] - b) * mist;
      pixels[o] = clamp8((r + jitter * 0.3) * ambient); pixels[o + 1] = clamp8((g + jitter * 0.3) * ambient); pixels[o + 2] = clamp8((b + jitter * 0.3) * ambient); pixels[o + 3] = 255;
      count++;
      continue;
    }
    if (solid[i] === SOLID_WOOD) {
      const c = SOLID_COLOR[SOLID_WOOD];
      const glow = burning[i] ? 0.5 + 0.5 * Math.sin(i * 0.7 + performance.now() * 0.02) : 0;
      const r = c[0] * ambient * (1 - glow) + FIRE_GLOW[0] * glow;
      const g = c[1] * ambient * (1 - glow) + FIRE_GLOW[1] * glow;
      const b = c[2] * ambient * (1 - glow) + FIRE_GLOW[2] * glow;
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
      // Humid air reads as a faint cool haze lifting the background.
      const fog = Math.min(0.55, gas[VAPOR][i] * 12) * (1 - smoke);
      r += (VAPOR_FOG[0] - r) * fog;
      g += (VAPOR_FOG[1] - g) * fog;
      b += (VAPOR_FOG[2] - b) * fog;
      pixels[o] = clamp8(r * ambient); pixels[o + 1] = clamp8(g * ambient); pixels[o + 2] = clamp8(b * ambient); pixels[o + 3] = 255;
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

    // Ambient day/night dimming — applied to lit matter, not to fire glow.
    r *= ambient;
    g *= ambient;
    b *= ambient;

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

  if (fpsTimer === 0) {
    const isDay = lightLevel > 0.5;
    document.getElementById("light").textContent = Math.round(lightLevel * 100);
    document.getElementById("daynight").textContent = isDay ? "day" : "night";
    document.getElementById("humidity").textContent = humidityPct.toFixed(1);
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
  jarNeck = null;
  lidSealed = false;
  sealLidBtn.textContent = "Seal Lid";
  sealLidBtn.classList.remove("active");
  clearSave();
});

const sealLidBtn = document.getElementById("sealLidBtn");
document.getElementById("buildJarBtn").addEventListener("click", () => {
  buildJar();
  sealLidBtn.textContent = "Seal Lid";
  sealLidBtn.classList.remove("active");
});
sealLidBtn.addEventListener("click", () => {
  toggleLidSeal();
  sealLidBtn.textContent = lidSealed ? "Open Lid" : "Seal Lid";
  sealLidBtn.classList.toggle("active", lidSealed);
});

const dayLengthSlider = document.getElementById("dayLength");
const dayLengthVal = document.getElementById("dayLengthVal");
dayLengthSlider.addEventListener("input", () => {
  dayTicks = parseInt(dayLengthSlider.value, 10);
  dayLengthVal.textContent = dayTicks;
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

// ---- Save / restore (localStorage snapshot) --------------------------
// Building a jar and waiting for the water cycle to get going takes real
// time, so the whole grid is snapshotted to localStorage every few seconds
// (and on tab-hide / unload) and restored on load. Fractions are quantised
// to a byte each — plenty for a scene that re-settles on its own — keeping a
// typical grid well under the storage quota.

const SAVE_KEY = "fallingsand.terrarium.v1";
const SAVE_EVERY_MS = 4000;
const SNAP_BYTES_PER_CELL = N_COND + N_GAS + 3; // comp + gas + solid/woodHp/burning

function snapshot() {
  const n = cols * rows;
  const q = new Uint8Array(n * SNAP_BYTES_PER_CELL);
  let p = 0;
  for (let c = 0; c < N_COND; c++) for (let i = 0; i < n; i++) q[p++] = Math.round(comp[c][i] * 255);
  for (let c = 0; c < N_GAS; c++) for (let i = 0; i < n; i++) q[p++] = Math.round(gas[c][i] * 255);
  for (let i = 0; i < n; i++) q[p++] = solid[i];
  for (let i = 0; i < n; i++) q[p++] = woodHp[i];
  for (let i = 0; i < n; i++) q[p++] = burning[i];
  let bin = "";
  for (let k = 0; k < q.length; k += 8192) bin += String.fromCharCode.apply(null, q.subarray(k, k + 8192));
  return JSON.stringify({
    v: 1, cols, rows, clockTick, dayTicks, lidSealed, jarNeck, data: btoa(bin),
  });
}

function saveState() {
  try { localStorage.setItem(SAVE_KEY, snapshot()); } catch (e) { /* quota exceeded or storage disabled */ }
}

function clearSave() {
  try { localStorage.removeItem(SAVE_KEY); } catch (e) { /* ignore */ }
}

function loadState() {
  let raw;
  try { raw = localStorage.getItem(SAVE_KEY); } catch (e) { return false; }
  if (!raw) return false;
  let s;
  try { s = JSON.parse(raw); } catch (e) { return false; }
  if (!s || s.v !== 1 || s.cols !== cols || s.rows !== rows) return false;

  const n = cols * rows;
  const need = n * SNAP_BYTES_PER_CELL;
  let bin;
  try { bin = atob(s.data); } catch (e) { return false; }
  if (bin.length !== need) return false;

  const q = new Uint8Array(need);
  for (let k = 0; k < need; k++) q[k] = bin.charCodeAt(k);
  let p = 0;
  for (let c = 0; c < N_COND; c++) for (let i = 0; i < n; i++) comp[c][i] = q[p++] / 255;
  for (let c = 0; c < N_GAS; c++) for (let i = 0; i < n; i++) gas[c][i] = q[p++] / 255;
  for (let i = 0; i < n; i++) solid[i] = q[p++];
  for (let i = 0; i < n; i++) woodHp[i] = q[p++];
  for (let i = 0; i < n; i++) burning[i] = q[p++];

  // Undo the quantisation drift so every cell sums back to exactly 1.
  for (let i = 0; i < n; i++) {
    if (solid[i] !== SOLID_NONE) continue;
    let sum = 0;
    for (let c = 0; c < N_COND; c++) sum += comp[c][i];
    for (let c = 0; c < N_GAS; c++) sum += gas[c][i];
    if (sum > 1e-4) {
      const inv = 1 / sum;
      for (let c = 0; c < N_COND; c++) comp[c][i] *= inv;
      for (let c = 0; c < N_GAS; c++) gas[c][i] *= inv;
    }
  }

  clockTick = (s.clockTick | 0) % (s.dayTicks || dayTicks);
  dayTicks = s.dayTicks || dayTicks;
  lidSealed = !!s.lidSealed;
  jarNeck = s.jarNeck || null;
  return true;
}

setInterval(() => { if (running) saveState(); }, SAVE_EVERY_MS);
window.addEventListener("beforeunload", saveState);
document.addEventListener("visibilitychange", () => { if (document.hidden) saveState(); });

// ---- Boot --------------------------------------------------------------

resize();

if (loadState()) {
  dayLengthSlider.value = dayTicks;
  dayLengthVal.textContent = dayTicks;
  sealLidBtn.textContent = lidSealed ? "Open Lid" : "Seal Lid";
  sealLidBtn.classList.toggle("active", lidSealed);
}

setActiveMaterialButton();
requestAnimationFrame(loop);
