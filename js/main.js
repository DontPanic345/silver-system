// On-screen driver for the scenario player. All simulation logic lives in
// js/player.js (the DOM-free controller) and js/fluid.js (the solver); this file
// is DOM glue only — it builds the controls, runs the animation frame loop, and
// renders whatever channels the controller reports (AC 16).

import { createController } from './player.js';
import { IX } from './fluid.js';

const controller = createController();
// Exposed for the headless page check (scripts/shot-scenarios.js).
window.controller = controller;

const canvas = document.getElementById('sim');
const ctx = canvas.getContext('2d');
let field = document.createElement('canvas');
let fctx = field.getContext('2d');
let img = null;

function ensureField() {
  const n = controller.sim ? controller.sim.N : 1;
  if (field.width !== n) {
    field.width = field.height = n;
    fctx = field.getContext('2d');
    img = fctx.createImageData(n, n);
  }
}

// --- rendering ------------------------------------------------------------

// warm dye palette: black -> deep orange -> gold -> white.
function densPixel(d, out, o) {
  const t = 1 - Math.exp(-d * 3.0);
  out[o]     += 255 * Math.min(1, t * 2.4);
  out[o + 1] += 255 * Math.min(1, Math.max(0, (t - 0.15) * 1.7));
  out[o + 2] += 255 * Math.min(1, Math.max(0, (t - 0.6) * 2.4));
}

// temperature tint: cold -> blue, warm -> red, relative to the current frame's
// interior magnitude so any scenario's scale is visible.
function tempPixel(value, scale, out, o) {
  const t = Math.max(-1, Math.min(1, value * scale));
  if (t >= 0) {
    out[o]     += 220 * t;
    out[o + 2] += 40 * t;
  } else {
    out[o + 2] += 220 * -t;
    out[o]     += 40 * -t;
  }
}

const CHANNEL_RENDERERS = {
  dens: (sim, data, n, chanScale) => {
    for (let j = 0; j < n; j++)
      for (let i = 0; i < n; i++)
        densPixel(sim.dens[IX(sim.SIZE, i + 1, j + 1)], data, 4 * (i + j * n));
  },
  temp: (sim, data, n) => {
    let mag = 0;
    for (let j = 1; j <= n; j++)
      for (let i = 1; i <= n; i++)
        mag = Math.max(mag, Math.abs(sim.temp[IX(sim.SIZE, i, j)]));
    const scale = mag > 1e-9 ? 1 / mag : 0;
    for (let j = 0; j < n; j++)
      for (let i = 0; i < n; i++)
        tempPixel(sim.temp[IX(sim.SIZE, i + 1, j + 1)], scale, data, 4 * (i + j * n));
  },
};

function render() {
  const sim = controller.sim;
  if (!sim) return;
  ensureField();
  const n = sim.N;
  const data = img.data;
  for (let p = 0; p < data.length; p += 4) {
    data[p] = data[p + 1] = data[p + 2] = 0;
    data[p + 3] = 255;
  }
  for (const ch of controller.channels()) {
    const draw = CHANNEL_RENDERERS[ch];
    if (draw) draw(sim, data, n);
  }
  fctx.putImageData(img, 0, 0);
  ctx.imageSmoothingEnabled = true;
  ctx.drawImage(field, 0, 0, canvas.width, canvas.height);
}

function renderReadouts() {
  const panel = document.getElementById('readouts');
  panel.textContent = '';
  for (const r of controller.readouts()) {
    const row = document.createElement('span');
    row.className = 'readout';
    const v = Math.abs(r.value) >= 1000 ? r.value.toFixed(0) : r.value.toFixed(3);
    row.textContent = `${r.label}: ${v}`;
    panel.appendChild(row);
  }
  const steps = document.createElement('span');
  steps.className = 'readout';
  steps.textContent = `steps: ${controller.stepCount}`;
  panel.appendChild(steps);
}

// --- controls ------------------------------------------------------------

const select = document.getElementById('scenario-select');
for (const s of controller.listScenarios()) {
  const opt = document.createElement('option');
  opt.value = s.id;
  opt.textContent = `${s.name} — ${s.description}`;
  select.appendChild(opt);
}

const playBtn = document.getElementById('play');
const sandboxToggle = document.getElementById('sandbox');
const sandboxPanel = document.getElementById('sandbox-panel');
const paintChannelSel = document.getElementById('paint-channel');
const SANDBOX_GRID = 96;

function syncPlayBtn() {
  playBtn.textContent = controller.playing ? 'Pause' : 'Play';
}

function refreshPaintChannels() {
  paintChannelSel.textContent = '';
  for (const ch of controller.channels()) {
    const opt = document.createElement('option');
    opt.value = ch;
    opt.textContent = ch;
    paintChannelSel.appendChild(opt);
  }
}

function loadCurrentScenario() {
  controller.load(select.value);
  syncPlayBtn();
  renderReadouts();
}

function enterSandbox() {
  controller.loadSandbox(SANDBOX_GRID);
  sandboxPanel.hidden = false;
  refreshPaintChannels();
  syncPlayBtn();
  renderReadouts();
}

select.addEventListener('change', () => {
  if (controller.mode === 'scenario') loadCurrentScenario();
});

playBtn.addEventListener('click', () => {
  if (controller.playing) controller.pause();
  else controller.play();
  syncPlayBtn();
});

document.getElementById('step').addEventListener('click', () => {
  controller.singleStep();
  syncPlayBtn();
  renderReadouts();
});

document.getElementById('reset').addEventListener('click', () => {
  controller.reset();
  syncPlayBtn();
  renderReadouts();
});

sandboxToggle.addEventListener('change', () => {
  if (sandboxToggle.checked) {
    enterSandbox();
  } else {
    sandboxPanel.hidden = true;
    loadCurrentScenario();
  }
});

// --- sandbox painting ---------------------------------------------------

let painting = false;

function paintAt(e) {
  const sim = controller.sim;
  if (!sim || controller.mode !== 'sandbox') return;
  const r = canvas.getBoundingClientRect();
  const i = Math.floor(((e.clientX - r.left) / r.width) * sim.N) + 1;
  const j = Math.floor(((e.clientY - r.top) / r.height) * sim.N) + 1;
  controller.paint({ channel: paintChannelSel.value || 'dens', i, j, radius: 2, amount: 1 });
  renderReadouts();
}

canvas.addEventListener('pointerdown', (e) => {
  canvas.setPointerCapture(e.pointerId);
  painting = true;
  paintAt(e);
});
canvas.addEventListener('pointermove', (e) => { if (painting) paintAt(e); });
const stopPaint = () => { painting = false; };
canvas.addEventListener('pointerup', stopPaint);
canvas.addEventListener('pointercancel', stopPaint);

// --- loop -------------------------------------------------------------

let frames = 0, fpsClock = performance.now(), stepMs = 0, readoutClock = 0;

function frame() {
  const wasStep = controller.playing;
  const t = performance.now();
  controller.tick();
  if (wasStep) stepMs = stepMs * 0.9 + (performance.now() - t) * 0.1;
  render();

  frames++;
  const now = performance.now();
  if (now - fpsClock >= 500) {
    document.getElementById('fps').textContent = Math.round((frames * 1000) / (now - fpsClock));
    document.getElementById('step-ms').textContent = stepMs.toFixed(2);
    document.getElementById('grid').textContent =
      controller.sim ? `${controller.sim.N}×${controller.sim.N}` : '–';
    frames = 0;
    fpsClock = now;
  }
  if (now - readoutClock >= 200) {
    renderReadouts();
    readoutClock = now;
  }
  requestAnimationFrame(frame);
}

loadCurrentScenario();
render();
requestAnimationFrame(frame);
