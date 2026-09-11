// Scenario: "A person opens www/terrarium.html in a real browser tab and
// watches the gnome terrarium run" — a headless Playwright check that the
// wasm build genuinely simulates in the browser, rather than merely loading.
//
// Four things are asserted, and none of them is a screenshot:
//
//  1. The world *advances*: the step counter moves and the canvas pixels
//     change between two samples taken a second apart. A page that loaded
//     the module but never ticked would pass a "did it render" check and
//     fail this one.
//  2. The conservation invariant holds in the browser build too. The page
//     exposes the same JSON snapshot the headless runner prints, so this
//     reads `residual_mass_relative` / `residual_energy_relative` straight
//     out of the running simulation — the numbers, not a picture of them.
//  3. The water cycle runs, in the browser build, without anything boiling:
//     the pool has evaporated and the air has given it back as dew and
//     rain, and the jar is somewhere a gnome can live.
//  4. The jar has a day, in the actual pixels: the mean channel value of the
//     whole canvas at midday is measurably higher than at midnight, sampled
//     by walking the live page forward until it has seen both. The garden
//     grows (gaining more than its own night respiration spends), something
//     breathes carbon dioxide out, and nowhere in the jar is the air too thin
//     to breathe.
//  5. The pool is level, in the actual pixels. Night 4 shipped — briefly, and
//     with every other test green — a pool that stood as a slope against the
//     jar's far wall; it was caught only by looking at a rendered frame.
//     This makes that look a check: the top of the water must sit at the same
//     height, to within two cells, all the way across.
//
// Prerequisite: the wasm build must already exist at www/pkg/ (see
// scripts/build-wasm.sh).
//
// Run with:
//   NODE_PATH=/usr/local/lib/node_modules node tests/e2e/terrarium_canvas.test.mjs

import http from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, '..', '..');
const wwwDir = path.join(repoRoot, 'www');

let failed = false;
function fail(message) {
  failed = true;
  console.error(message);
}

function serveDir(dir) {
  return http.createServer(async (req, res) => {
    try {
      const urlPath = req.url === '/' ? '/index.html' : req.url;
      const filePath = path.join(dir, decodeURIComponent(urlPath.split('?')[0]));
      if (!filePath.startsWith(dir)) {
        res.writeHead(403);
        res.end();
        return;
      }
      const data = await readFile(filePath);
      const ext = path.extname(filePath);
      const type =
        { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' }[ext] ||
        'application/octet-stream';
      res.writeHead(200, { 'content-type': type });
      res.end(data);
    } catch (e) {
      res.writeHead(404);
      res.end(String(e));
    }
  });
}

const canvasFingerprint = (page) =>
  page.evaluate(() => {
    const canvas = document.getElementById('canvas');
    const ctx = canvas.getContext('2d');
    const { data } = ctx.getImageData(0, 0, canvas.width, canvas.height);
    let hash = 0;
    for (let i = 0; i < data.length; i += 97) {
      hash = (hash * 31 + data[i]) >>> 0;
    }
    return hash;
  });

async function main() {
  const wasmPath = path.join(wwwDir, 'pkg', 'viewer_bg.wasm');
  try {
    await stat(wasmPath);
  } catch {
    fail(`FAIL terrarium_canvas: build artifacts missing at ${wasmPath}. Run scripts/build-wasm.sh first.`);
    return;
  }

  const { chromium } = require('playwright');
  const server = serveDir(wwwDir);
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const port = server.address().port;

  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const pageErrors = [];
    page.on('pageerror', (e) => pageErrors.push(String(e)));

    await page.goto(`http://127.0.0.1:${port}/terrarium.html`);
    await page
      .waitForFunction(() => window.__terrariumReady === true || window.__terrariumError !== null, {
        timeout: 10000,
      })
      .catch(() => {});

    const error = await page.evaluate(() => window.__terrariumError);
    const ready = await page.evaluate(() => window.__terrariumReady);
    if (!ready) {
      fail(
        `FAIL terrarium_canvas: page did not report ready ` +
          `(__terrariumError=${error}, pageerror=${JSON.stringify(pageErrors)}).`
      );
      return;
    }

    await page.waitForFunction(() => window.__lastReport && window.__lastReport.step > 0, {
      timeout: 10000,
    });
    const first = await page.evaluate(() => window.__lastReport);
    const firstPixels = await canvasFingerprint(page);

    await page.waitForTimeout(1200);
    const second = await page.evaluate(() => window.__lastReport);
    const secondPixels = await canvasFingerprint(page);

    // Then give the water cycle real time to turn over.
    await page.waitForTimeout(6000);
    await page
      .waitForFunction(() => window.__lastReport && window.__lastReport.step >= 3000, {
        timeout: 30000,
      })
      .catch(() => {});
    const third = await page.evaluate(() => window.__lastReport);

    // --- The day, in actual pixels ---
    //
    // The scene is dimmed by how much daylight reaches each cell, so a frame
    // at midday is measurably brighter than a frame at midnight. This walks
    // the page forward until it has seen one of each and compares the mean
    // channel value of the whole canvas — no hue, no single pixel, and
    // nothing a still image could fake.
    const meanBrightness = () =>
      page.evaluate(() => {
        const canvas = document.getElementById('canvas');
        const ctx = canvas.getContext('2d');
        const { data } = ctx.getImageData(0, 0, canvas.width, canvas.height);
        let sum = 0, n = 0;
        for (let i = 0; i < data.length; i += 4) {
          sum += data[i] + data[i + 1] + data[i + 2];
          n += 3;
        }
        return sum / n;
      });

    const sample = async (want) => {
      const deadline = Date.now() + 25000;
      while (Date.now() < deadline) {
        const day = await page.evaluate(() => window.__lastReport?.air?.daylight ?? -1);
        if (want === 'day' ? day > 0.8 : day === 0) {
          return { day, brightness: await meanBrightness() };
        }
        await page.waitForTimeout(120);
      }
      return null;
    };
    const night = await sample('night');
    const noon = await sample('day');

    // The top of the water in each column across the pool: the highest
    // pixel row that is water-blue, scanning down. Water paints blue well
    // above red; air, stone and sand do not.
    const surface = await page.evaluate(() => {
      const canvas = document.getElementById('canvas');
      const ctx = canvas.getContext('2d');
      const { data, width, height } = ctx.getImageData(0, 0, canvas.width, canvas.height);
      const tops = [];
      // The pool spans roughly the right two-thirds of the jar, inside the
      // walls; sample columns well inside it.
      for (let x = Math.floor(width * 0.4); x < Math.floor(width * 0.93); x += 6) {
        let top = -1;
        for (let y = Math.floor(height * 0.4); y < height; y++) {
          const i = (y * width + x) * 4;
          const [r, g, b] = [data[i], data[i + 1], data[i + 2]];
          if (b > 120 && b > r + 60 && b > g + 30) {
            top = y;
            break;
          }
        }
        tops.push(top);
      }
      return tops;
    });

    if (!(second.step > first.step)) {
      fail(`FAIL terrarium_canvas: simulation did not advance (${first.step} -> ${second.step}).`);
    }
    if (firstPixels === secondPixels) {
      fail('FAIL terrarium_canvas: canvas pixels never changed — the world is not visibly running.');
    }
    for (const report of [first, second, third]) {
      if (Math.abs(report.residual_mass_relative) > 1e-6) {
        fail(
          `FAIL terrarium_canvas: mass residual ${report.residual_mass_relative} at step ${report.step}.`
        );
      }
      if (Math.abs(report.residual_energy_relative) > 1e-4) {
        fail(
          `FAIL terrarium_canvas: energy residual ${report.residual_energy_relative} at step ${report.step}.`
        );
      }
    }
    if (!(third.step >= 3000)) {
      fail(`FAIL terrarium_canvas: only ${third.step} steps in the time allowed — too slow to judge.`);
    }
    const v = third.vapour;
    if (!(v.evaporated_g > 0.3 && v.rained_g > 0.2)) {
      fail(`FAIL terrarium_canvas: the water cycle is not turning over (${JSON.stringify(v)}).`);
    }
    const steamCells = third.materials.find((m) => m.name === 'steam').cells;
    if (steamCells > 0) {
      fail(`FAIL terrarium_canvas: ${steamCells} cells of steam — something is boiling.`);
    }
    if (!(third.mean_temperature_k < 320)) {
      fail(`FAIL terrarium_canvas: the jar is at ${third.mean_temperature_k} K, too hot for gnomes.`);
    }
    const found = surface.filter((y) => y >= 0);
    const cellPx = 12;
    if (found.length < surface.length * 0.8) {
      fail(`FAIL terrarium_canvas: no pool found in the pixels (${JSON.stringify(surface)}).`);
    } else {
      // Trim the tails before measuring. The intent of this check is "the
      // pool's surface is level", and a drop falling from the fountain —
      // which is blue, and is genuinely twelve cells above the water — is
      // not part of the pool's surface. Trimming one column each end was too
      // little: the fountain's drop is wide enough to be sampled twice.
      const sorted = [...found].sort((a, b) => a - b);
      const trim = Math.max(1, Math.floor(sorted.length * 0.1));
      const body = sorted.slice(trim, sorted.length - trim);
      const spread = (body[body.length - 1] - body[0]) / cellPx;
      if (spread > 2) {
        fail(
          `FAIL terrarium_canvas: the pool is not level — its surface spans ${spread.toFixed(1)} ` +
            `cells across the jar (${JSON.stringify(surface)}).`
        );
      }
    }
    if (!night || !noon) {
      fail(
        `FAIL terrarium_canvas: never saw both a midday and a midnight frame ` +
          `(night=${JSON.stringify(night)}, noon=${JSON.stringify(noon)}).`
      );
    } else if (!(noon.brightness > night.brightness * 1.05)) {
      fail(
        `FAIL terrarium_canvas: the jar does not visibly darken at night — ` +
          `mean channel ${noon.brightness.toFixed(2)} at midday against ` +
          `${night.brightness.toFixed(2)} at midnight.`
      );
    }
    const life = third.life;
    if (!(life.grown_g > 0)) {
      fail(`FAIL terrarium_canvas: the garden never grew (${JSON.stringify(life)}).`);
    }
    if (!(life.grown_g > life.respired_g)) {
      fail(
        `FAIL terrarium_canvas: the garden's night beat its day ` +
          `(${JSON.stringify(life)}) — it is losing weight, not growing.`
      );
    }
    if (!(third.air.co2_g > 0)) {
      fail('FAIL terrarium_canvas: nothing in the jar ever breathed out any carbon dioxide.');
    }
    if (!(third.air.min_breathable_atm > 0.08)) {
      fail(
        `FAIL terrarium_canvas: the thinnest air in the jar is ` +
          `${third.air.min_breathable_atm} atm — a gnome is suffocating.`
      );
    }
    if (pageErrors.length > 0) {
      fail(`FAIL terrarium_canvas: page errors ${JSON.stringify(pageErrors)}.`);
    }

    if (!failed) {
      console.log(
        `PASS terrarium_canvas: advanced ${first.step} -> ${second.step} -> ${third.step} steps, ` +
          `pixels changed, mass residual ${third.residual_mass_relative}, ` +
          `energy residual ${third.residual_energy_relative}, ` +
          `evaporated ${v.evaporated_g.toFixed(3)} g, rained ${v.rained_g.toFixed(3)} g, ` +
          `mean ${third.mean_temperature_k.toFixed(1)} K, ` +
          `grown ${third.life.grown_g.toFixed(4)} g against ${third.life.respired_g.toFixed(4)} g respired, ` +
          `CO2 ${third.air.co2_g.toFixed(4)} g, thinnest air ${third.air.min_breathable_atm.toFixed(3)} atm, ` +
          `midday ${noon ? noon.brightness.toFixed(2) : '?'} vs midnight ` +
          `${night ? night.brightness.toFixed(2) : '?'} mean channel, ` +
          `${third.colony.embodied} gnomes embodied.`
      );
    }
  } finally {
    await browser.close();
    server.close();
  }
}

main()
  .catch((e) => fail(`FAIL terrarium_canvas: ${e && e.stack ? e.stack : e}`))
  .finally(() => process.exit(failed ? 1 : 0));
