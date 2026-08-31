"use strict";

/**
 * Falling Sand — a small cellular-automaton particle simulator.
 *
 * The grid is a flat typed array of material ids. Each simulation step
 * scans the grid bottom-to-top (so nothing falls through a cell it just
 * vacated this frame) and alternates scan direction left/right per row
 * per frame to avoid a directional drift bias.
 */

// ---- Materials -------------------------------------------------------

const EMPTY = 0;
const SAND = 1;
const WATER = 2;
const STONE = 3;
const WOOD = 4;
const FIRE = 5;
const SMOKE = 6;

const STATIC = new Set([EMPTY, STONE, WOOD]); // wood only moves via combustion
const DENSITY = { [EMPTY]: 0, [SAND]: 3, [WATER]: 1, [STONE]: 99, [WOOD]: 99, [FIRE]: -1, [SMOKE]: -2 };

// Base colors [r,g,b] per material; actual pixel color gets small per-cell jitter.
const BASE_COLOR = {
  [EMPTY]: [12, 13, 17],
  [SAND]: [217, 177, 88],
  [WATER]: [58, 160, 255],
  [STONE]: [138, 143, 152],
  [WOOD]: [138, 90, 52],
  [FIRE]: [255, 90, 46],
  [SMOKE]: [90, 92, 98],
};

// ---- Canvas / grid setup ----------------------------------------------

const canvas = document.getElementById("sim");
const ctx = canvas.getContext("2d", { alpha: false });
const wrap = document.querySelector(".canvas-wrap");

const CELL_PX = 5; // displayed size of one simulation cell, in CSS pixels

let cols = 0;
let rows = 0;
let grid, nextVariant, life, moved; // typed arrays, sized cols*rows
let imageData, pixels; // for fast blit

function idx(x, y) {
  return y * cols + x;
}

function inBounds(x, y) {
  return x >= 0 && x < cols && y >= 0 && y < rows;
}

function allocate(newCols, newRows) {
  cols = newCols;
  rows = newRows;
  grid = new Uint8Array(cols * rows);
  nextVariant = new Uint8Array(cols * rows); // per-cell color jitter, stable per particle
  life = new Uint8Array(cols * rows); // used by fire/smoke as a countdown
  moved = new Uint8Array(cols * rows); // scratch: has this cell already been simulated this frame?

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

  // Preserve existing particles when the canvas grows/shrinks, anchored
  // to the bottom-left so the user doesn't lose their scene on a resize.
  const old = grid;
  const oldCols = cols;
  const oldRows = rows;

  allocate(newCols, newRows);

  if (old) {
    const yOffset = rows - oldRows;
    for (let y = 0; y < oldRows; y++) {
      const ny = y + yOffset;
      if (ny < 0 || ny >= rows) continue;
      for (let x = 0; x < oldCols && x < cols; x++) {
        const v = old[y * oldCols + x];
        if (v !== EMPTY) grid[idx(x, ny)] = v;
      }
    }
  }
}

// ---- Drawing (input) --------------------------------------------------

let currentMaterial = SAND;
let brushSize = 5;
let isPointerDown = false;
let lastPointerCell = null;

function setCell(x, y, mat) {
  if (!inBounds(x, y)) return;
  const i = idx(x, y);
  grid[i] = mat;
  nextVariant[i] = (Math.random() * 16) | 0;
  life[i] = mat === FIRE ? 18 + ((Math.random() * 10) | 0) : mat === SMOKE ? 40 + ((Math.random() * 30) | 0) : 0;
}

function stampBrush(cx, cy) {
  const r = brushSize;
  const r2 = r * r;
  for (let dy = -r; dy <= r; dy++) {
    for (let dx = -r; dx <= r; dx++) {
      if (dx * dx + dy * dy > r2) continue;
      // sparse fill for gases/solids to avoid one click filling a solid disc too densely
      if ((currentMaterial === SAND || currentMaterial === WATER) && Math.random() < 0.25 && (dx || dy)) continue;
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
  (e) => {
    if (e.touches[0]) handlePointerDown(e.touches[0]);
  },
  { passive: false }
);
canvas.addEventListener(
  "touchmove",
  (e) => {
    if (e.touches[0]) handlePointerMove(e.touches[0]);
  },
  { passive: false }
);
window.addEventListener("touchend", handlePointerUp);

// ---- Simulation step ---------------------------------------------------

function swap(i, j) {
  const gi = grid[i], vi = nextVariant[i], li = life[i];
  grid[i] = grid[j]; nextVariant[i] = nextVariant[j]; life[i] = life[j];
  grid[j] = gi; nextVariant[j] = vi; life[j] = li;
  moved[i] = 1;
  moved[j] = 1;
}

function tryMove(x, y, dx, dy) {
  const nx = x + dx, ny = y + dy;
  if (!inBounds(nx, ny)) return false;
  const i = idx(x, y);
  const j = idx(nx, ny);
  if (moved[j]) return false;
  const here = grid[i];
  const there = grid[j];
  if (there === EMPTY) {
    swap(i, j);
    return true;
  }
  // Denser particles sink through less dense fluids (sand through water).
  if (DENSITY[here] > 0 && DENSITY[there] >= 0 && DENSITY[here] > DENSITY[there]) {
    swap(i, j);
    return true;
  }
  return false;
}

function simulateSand(x, y) {
  if (tryMove(x, y, 0, 1)) return;
  const dir = Math.random() < 0.5 ? 1 : -1;
  if (tryMove(x, y, dir, 1)) return;
  tryMove(x, y, -dir, 1);
}

function simulateWater(x, y) {
  if (tryMove(x, y, 0, 1)) return;
  const dir = Math.random() < 0.5 ? 1 : -1;
  if (tryMove(x, y, dir, 1)) return;
  if (tryMove(x, y, -dir, 1)) return;
  if (tryMove(x, y, dir, 0)) return;
  tryMove(x, y, -dir, 0);
}

const FIRE_NEIGHBORS = [
  [1, 0], [-1, 0], [0, 1], [0, -1],
  [1, 1], [-1, 1], [1, -1], [-1, -1],
];

function simulateFire(x, y) {
  const i = idx(x, y);
  life[i]--;

  // Ignite adjacent wood, and get doused by adjacent water.
  let dousedByWater = false;
  for (const [dx, dy] of FIRE_NEIGHBORS) {
    const nx = x + dx, ny = y + dy;
    if (!inBounds(nx, ny)) continue;
    const j = idx(nx, ny);
    const mat = grid[j];
    if (mat === WOOD && Math.random() < 0.045) {
      grid[j] = FIRE;
      life[j] = 25 + ((Math.random() * 15) | 0);
      nextVariant[j] = (Math.random() * 16) | 0;
    } else if (mat === WATER) {
      dousedByWater = true;
    }
  }

  if (dousedByWater || life[i] <= 0) {
    grid[i] = Math.random() < 0.6 ? SMOKE : EMPTY;
    life[i] = 40 + ((Math.random() * 30) | 0);
    moved[i] = 1;
    return;
  }

  // Fire drifts upward with some flicker, and can move into empty/smoke.
  const dir = Math.random() < 0.5 ? 1 : -1;
  const choices = [[0, -1], [dir, -1], [0, 0], [dir, 0]];
  for (const [dx, dy] of choices) {
    if (dx === 0 && dy === 0) break; // stay put (flicker in place)
    const nx = x + dx, ny = y + dy;
    if (!inBounds(nx, ny)) continue;
    const j = idx(nx, ny);
    if (moved[j]) continue;
    if (grid[j] === EMPTY || grid[j] === SMOKE) {
      swap(i, j);
      return;
    }
  }
  moved[i] = 1;
}

function simulateSmoke(x, y) {
  const i = idx(x, y);
  life[i]--;
  if (life[i] <= 0) {
    grid[i] = EMPTY;
    life[i] = 0;
    moved[i] = 1;
    return;
  }
  const dir = Math.random() < 0.5 ? 1 : -1;
  if (tryMove(x, y, 0, -1)) return;
  if (tryMove(x, y, dir, -1)) return;
  if (tryMove(x, y, -dir, -1)) return;
  moved[i] = 1;
}

function step() {
  moved.fill(0);
  for (let y = rows - 1; y >= 0; y--) {
    const leftToRight = (y + frameParity) % 2 === 0;
    if (leftToRight) {
      for (let x = 0; x < cols; x++) simulateCell(x, y);
    } else {
      for (let x = cols - 1; x >= 0; x--) simulateCell(x, y);
    }
  }
  frameParity ^= 1;
}

function simulateCell(x, y) {
  const i = idx(x, y);
  if (moved[i]) return;
  const mat = grid[i];
  switch (mat) {
    case SAND:
      simulateSand(x, y);
      break;
    case WATER:
      simulateWater(x, y);
      break;
    case FIRE:
      simulateFire(x, y);
      break;
    case SMOKE:
      simulateSmoke(x, y);
      break;
    default:
      break; // empty, stone, wood: static
  }
}

let frameParity = 0;

// ---- Render --------------------------------------------------------------

function render() {
  let count = 0;
  for (let i = 0; i < grid.length; i++) {
    const mat = grid[i];
    const base = BASE_COLOR[mat];
    const o = i * 4;
    if (mat === EMPTY) {
      pixels[o] = base[0];
      pixels[o + 1] = base[1];
      pixels[o + 2] = base[2];
      pixels[o + 3] = 255;
      continue;
    }
    count++;
    const jitter = (nextVariant[i] - 8) * 2; // -16..16
    pixels[o] = clamp8(base[0] + jitter);
    pixels[o + 1] = clamp8(base[1] + jitter);
    pixels[o + 2] = clamp8(base[2] + jitter);
    pixels[o + 3] = 255;
  }
  ctx.putImageData(imageData, 0, 0);
  document.getElementById("count").textContent = count;
}

function clamp8(v) {
  return v < 0 ? 0 : v > 255 ? 255 : v;
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
    btn.classList.toggle("active", btn.dataset.mat === materialName(currentMaterial));
  });
}

function materialName(mat) {
  return { [SAND]: "sand", [WATER]: "water", [STONE]: "stone", [WOOD]: "wood", [FIRE]: "fire", [EMPTY]: "empty" }[mat];
}

const MATERIAL_BY_NAME = { sand: SAND, water: WATER, stone: STONE, wood: WOOD, fire: FIRE, empty: EMPTY };

document.getElementById("materials").addEventListener("click", (e) => {
  const btn = e.target.closest(".mat-btn");
  if (!btn) return;
  currentMaterial = MATERIAL_BY_NAME[btn.dataset.mat];
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
  grid.fill(EMPTY);
  life.fill(0);
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
