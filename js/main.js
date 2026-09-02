// On-screen driver for the stable-fluids solver in fluid.js: canvas rendering,
// pointer input, and the controls. The physics all lives in the module.

import { createFluid, step, splat, IX } from './fluid.js';

const N = 180;
const f = createFluid(N, { fade: 0.008 });

// Mouse motion is measured in grid cells per frame; the solver wants velocity
// in grid-widths per unit time (advection backtrace is dt·N·velocity), so a
// cell of drag maps to a small velocity.
const DRAG_TO_VEL = 0.015;

// --- rendering -------------------------------------------------------------

const canvas = document.getElementById('sim');
const ctx = canvas.getContext('2d');
const field = document.createElement('canvas');
field.width = field.height = N;
const fctx = field.getContext('2d');
const img = fctx.createImageData(N, N);
ctx.imageSmoothingEnabled = true;

let showVectors = false;

// warm dye palette: dark -> ember -> pale gold
function palette(d, out, o) {
  const t = d > 1 ? 1 : d;
  out[o]     = 255 * Math.min(1, t * 1.6);
  out[o + 1] = 255 * Math.min(1, Math.max(0, t * 1.4 - 0.25));
  out[o + 2] = 255 * Math.min(1, Math.max(0, t * 2.2 - 1.1));
  out[o + 3] = 255;
}

function render() {
  const data = img.data;
  for (let j = 0; j < N; j++) {
    for (let i = 0; i < N; i++) {
      palette(f.dens[IX(f.SIZE, i + 1, j + 1)], data, 4 * (i + j * N));
    }
  }
  fctx.putImageData(img, 0, 0);
  ctx.drawImage(field, 0, 0, canvas.width, canvas.height);

  if (showVectors) {
    const s = canvas.width / N;
    ctx.strokeStyle = 'rgba(120,200,255,0.35)';
    ctx.beginPath();
    for (let j = 1; j <= N; j += 7) {
      for (let i = 1; i <= N; i += 7) {
        const x = (i - 0.5) * s, y = (j - 0.5) * s;
        ctx.moveTo(x, y);
        ctx.lineTo(x + f.u[IX(f.SIZE, i, j)] * s * 40, y + f.v[IX(f.SIZE, i, j)] * s * 40);
      }
    }
    ctx.stroke();
  }
}

// --- interaction ---------------------------------------------------------

let pointer = null; // {i, j, di, dj}

function eventCell(e) {
  const r = canvas.getBoundingClientRect();
  const i = Math.floor(((e.clientX - r.left) / r.width) * N) + 1;
  const j = Math.floor(((e.clientY - r.top) / r.height) * N) + 1;
  return { i: Math.max(1, Math.min(N, i)), j: Math.max(1, Math.min(N, j)) };
}

canvas.addEventListener('pointerdown', (e) => {
  canvas.setPointerCapture(e.pointerId);
  pointer = { ...eventCell(e), di: 0, dj: 0 };
});
canvas.addEventListener('pointermove', (e) => {
  if (!pointer) return;
  const c = eventCell(e);
  pointer.di = c.i - pointer.i;
  pointer.dj = c.j - pointer.j;
  pointer.i = c.i;
  pointer.j = c.j;
});
const endPointer = () => { pointer = null; };
canvas.addEventListener('pointerup', endPointer);
canvas.addEventListener('pointercancel', endPointer);

function applyPointer() {
  if (!pointer) return;
  splat(f, pointer.i, pointer.j, 2, 0.6,
    pointer.di * DRAG_TO_VEL, pointer.dj * DRAG_TO_VEL);
}

// --- controls & loop ----------------------------------------------------

let paused = false;
document.getElementById('pause').addEventListener('click', (e) => {
  paused = !paused;
  e.target.textContent = paused ? 'Resume' : 'Pause';
});
document.getElementById('clear').addEventListener('click', () => {
  f.u.fill(0); f.v.fill(0); f.dens.fill(0);
});
document.getElementById('visc').addEventListener('input', (e) => {
  f.visc = (e.target.value / 100) * 0.0005;
});
document.getElementById('diff').addEventListener('input', (e) => {
  f.diff = (e.target.value / 100) * 0.0002;
});
document.getElementById('fade').addEventListener('input', (e) => {
  f.fade = (e.target.value / 100) * 0.05;
});
document.getElementById('vectors').addEventListener('change', (e) => {
  showVectors = e.target.checked;
});
document.getElementById('grid').textContent = `${N}×${N}`;

let frames = 0, fpsClock = performance.now(), stepMs = 0;

function frame() {
  if (!paused) {
    applyPointer();
    const t = performance.now();
    step(f);
    stepMs = stepMs * 0.9 + (performance.now() - t) * 0.1;
  }
  render();

  frames++;
  const now = performance.now();
  if (now - fpsClock >= 500) {
    document.getElementById('fps').textContent = Math.round((frames * 1000) / (now - fpsClock));
    document.getElementById('step').textContent = stepMs.toFixed(1);
    frames = 0;
    fpsClock = now;
  }
  requestAnimationFrame(frame);
}

requestAnimationFrame(frame);
