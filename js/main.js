// Stable Fluids — Jos Stam, "Real-Time Fluid Dynamics for Games" (2003).
// A grid-based (Eulerian) solver: semi-Lagrangian advection + a Gauss-Seidel
// pressure projection that keeps the velocity field divergence-free, so it
// stays stable at any timestep. Dye ("density") is carried passively by the
// flow and is the only thing we draw.

const N = 200;                 // interior grid is N x N
const SIZE = N + 2;             // + a one-cell border for boundary conditions
const DT = 0.12;
const ITER = 16;                // Gauss-Seidel sweeps per linear solve

const IX = (i, j) => i + SIZE * j;
const cell = () => new Float32Array(SIZE * SIZE);

let u = cell(), v = cell(), uPrev = cell(), vPrev = cell();
let dens = cell(), densPrev = cell();

let visc = 0, diff = 0, fade = 0.008;

// --- solver -----------------------------------------------------------------

// b: which field we're bounding — 1 = horizontal velocity (flip at left/right
// walls), 2 = vertical velocity (flip at top/bottom), 0 = a scalar (copy).
function setBnd(b, x) {
  for (let i = 1; i <= N; i++) {
    x[IX(0, i)]     = b === 1 ? -x[IX(1, i)] : x[IX(1, i)];
    x[IX(N + 1, i)] = b === 1 ? -x[IX(N, i)] : x[IX(N, i)];
    x[IX(i, 0)]     = b === 2 ? -x[IX(i, 1)] : x[IX(i, 1)];
    x[IX(i, N + 1)] = b === 2 ? -x[IX(i, N)] : x[IX(i, N)];
  }
  x[IX(0, 0)]         = 0.5 * (x[IX(1, 0)] + x[IX(0, 1)]);
  x[IX(0, N + 1)]     = 0.5 * (x[IX(1, N + 1)] + x[IX(0, N)]);
  x[IX(N + 1, 0)]     = 0.5 * (x[IX(N, 0)] + x[IX(N + 1, 1)]);
  x[IX(N + 1, N + 1)] = 0.5 * (x[IX(N, N + 1)] + x[IX(N + 1, N)]);
}

function linSolve(b, x, x0, a, c) {
  const invC = 1 / c;
  for (let k = 0; k < ITER; k++) {
    for (let j = 1; j <= N; j++) {
      for (let i = 1; i <= N; i++) {
        x[IX(i, j)] = (x0[IX(i, j)] + a * (
          x[IX(i - 1, j)] + x[IX(i + 1, j)] +
          x[IX(i, j - 1)] + x[IX(i, j + 1)]
        )) * invC;
      }
    }
    setBnd(b, x);
  }
}

function diffuse(b, x, x0, amount) {
  const a = DT * amount * N * N;
  linSolve(b, x, x0, a, 1 + 4 * a);
}

function advect(b, d, d0, velU, velV) {
  const dt0 = DT * N;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      let x = i - dt0 * velU[IX(i, j)];
      let y = j - dt0 * velV[IX(i, j)];
      if (x < 0.5) x = 0.5; else if (x > N + 0.5) x = N + 0.5;
      if (y < 0.5) y = 0.5; else if (y > N + 0.5) y = N + 0.5;
      const i0 = x | 0, i1 = i0 + 1;
      const j0 = y | 0, j1 = j0 + 1;
      const s1 = x - i0, s0 = 1 - s1;
      const t1 = y - j0, t0 = 1 - t1;
      d[IX(i, j)] =
        s0 * (t0 * d0[IX(i0, j0)] + t1 * d0[IX(i0, j1)]) +
        s1 * (t0 * d0[IX(i1, j0)] + t1 * d0[IX(i1, j1)]);
    }
  }
  setBnd(b, d);
}

function project(velU, velV, p, divg) {
  const h = 1 / N;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      divg[IX(i, j)] = -0.5 * h * (
        velU[IX(i + 1, j)] - velU[IX(i - 1, j)] +
        velV[IX(i, j + 1)] - velV[IX(i, j - 1)]
      );
      p[IX(i, j)] = 0;
    }
  }
  setBnd(0, divg);
  setBnd(0, p);
  linSolve(0, p, divg, 1, 4);

  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      velU[IX(i, j)] -= 0.5 * (p[IX(i + 1, j)] - p[IX(i - 1, j)]) / h;
      velV[IX(i, j)] -= 0.5 * (p[IX(i, j + 1)] - p[IX(i, j - 1)]) / h;
    }
  }
  setBnd(1, velU);
  setBnd(2, velV);
}

function velStep() {
  [u, uPrev] = [uPrev, u];
  diffuse(1, u, uPrev, visc);
  [v, vPrev] = [vPrev, v];
  diffuse(2, v, vPrev, visc);
  project(u, v, uPrev, vPrev);
  [u, uPrev] = [uPrev, u];
  [v, vPrev] = [vPrev, v];
  advect(1, u, uPrev, uPrev, vPrev);
  advect(2, v, vPrev, uPrev, vPrev);
  project(u, v, uPrev, vPrev);
}

function densStep() {
  [dens, densPrev] = [densPrev, dens];
  diffuse(0, dens, densPrev, diff);
  [dens, densPrev] = [densPrev, dens];
  advect(0, dens, densPrev, u, v);
  if (fade > 0) {
    const keep = 1 - fade;
    for (let i = 0; i < dens.length; i++) dens[i] *= keep;
  }
}

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
      palette(dens[IX(i + 1, j + 1)], data, 4 * (i + j * N));
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
        ctx.lineTo(x + u[IX(i, j)] * s * 40, y + v[IX(i, j)] * s * 40);
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
  const c = eventCell(e);
  pointer = { ...c, di: 0, dj: 0 };
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
  const { i, j, di, dj } = pointer;
  const R = 2;
  for (let b = -R; b <= R; b++) {
    for (let a = -R; a <= R; a++) {
      const ii = i + a, jj = j + b;
      if (ii < 1 || ii > N || jj < 1 || jj > N) continue;
      dens[IX(ii, jj)] += 0.6;
      u[IX(ii, jj)] += di * 5;
      v[IX(ii, jj)] += dj * 5;
    }
  }
}

// --- controls & loop ----------------------------------------------------

let paused = false;
document.getElementById('pause').addEventListener('click', (e) => {
  paused = !paused;
  e.target.textContent = paused ? 'Resume' : 'Pause';
});
document.getElementById('clear').addEventListener('click', () => {
  u.fill(0); v.fill(0); dens.fill(0);
});
document.getElementById('visc').addEventListener('input', (e) => {
  visc = (e.target.value / 100) * 0.0005;
});
document.getElementById('diff').addEventListener('input', (e) => {
  diff = (e.target.value / 100) * 0.0002;
});
document.getElementById('fade').addEventListener('input', (e) => {
  fade = (e.target.value / 100) * 0.05;
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
    velStep();
    densStep();
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
