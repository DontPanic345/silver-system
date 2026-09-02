// Shared field metrics — DOM-free, import-safe under plain node.
//
// Conservation laws, monotonic trends and centre-of-mass direction are what the
// scenario suite asserts on (AC test-suite shape), so the reductions that back
// those assertions live here once and are reused by every consumer: the headless
// suite, js/scenarios.js, and the round-5 web page's live readout.
//
// Every reduction is over the INTERIOR (1..N in each axis), excluding the
// one-cell boundary ring, and takes the field array explicitly so the same
// helper serves temp, dens, or any later channel.

import { IX } from './fluid.js';

// Sum of an interior field. For a uniform heat capacity this is proportional to
// sensible thermal energy when arr is f.temp; it is interior mass when arr is a
// density/phase-fraction channel.
export const interiorSum = (f, arr) => {
  const { N, SIZE } = f;
  let s = 0;
  for (let j = 1; j <= N; j++) for (let i = 1; i <= N; i++) s += arr[IX(SIZE, i, j)];
  return s;
};

// Min/max of an interior field, as { lo, hi }.
export const interiorRange = (f, arr) => {
  const { N, SIZE } = f;
  let lo = Infinity, hi = -Infinity;
  for (let j = 1; j <= N; j++) for (let i = 1; i <= N; i++) {
    const v = arr[IX(SIZE, i, j)];
    if (v < lo) lo = v;
    if (v > hi) hi = v;
  }
  return { lo, hi };
};

// Weighted centre of mass of an interior field along one axis (1-indexed).
//   axis    : 'i' (column / horizontal) or 'j' (row / vertical; smaller = up)
//   weight  : maps a cell value to a non-negative weight; default identity
//   empty   : returned when the total weight is 0 (default: grid centre)
// Assert on the DIRECTION this moves, not its absolute value.
export const weightedCentroid = (f, arr, opts = {}) => {
  const { N, SIZE } = f;
  const axis = opts.axis || 'j';
  const weight = opts.weight || ((v) => v);
  let wsum = 0, csum = 0;
  for (let j = 1; j <= N; j++) for (let i = 1; i <= N; i++) {
    const w = weight(arr[IX(SIZE, i, j)]);
    wsum += w;
    csum += w * (axis === 'i' ? i : j);
  }
  if (wsum === 0) return opts.empty ?? (N + 1) / 2;
  return csum / wsum;
};
