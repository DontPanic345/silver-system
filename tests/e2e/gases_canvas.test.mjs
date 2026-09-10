// Scenario: "A person opens www/gases.html in a real browser tab and watches
// the CO2 sink, and then mix" — a headless Playwright check on the gas
// chamber.
//
// The claim changed on night 4. This page used to demonstrate CO2 settling
// into a flat layer on the floor under clean air — which, read against the
// dictation it was meant to answer ("gas is mix. CO2 is heavier but it
// doesn't all fall to the bottom of a room"), was the complaint rather than
// the fix. Gases can share cells now, and the page claims both halves.
//
// The point of this file is that it does not trust the simulation's own
// summary of itself. It reads two independent things and requires them to
// agree:
//
//  1. The JSON the page exposes: the room's pressure spread must collapse
//     as the bottle vents, the CO2's mean height must fall (it sank), the
//     row-by-row CO2 profile must lean toward the floor without being empty
//     at the ceiling (it mixed), and the residuals must stay at noise.
//  2. The actual canvas pixels: a band well above the floor must end up
//     visibly CO2-coloured — which a layer on the floor never is — and the
//     floor band must be more so. That is "it sank, and then it mixed"
//     checked against what a person would see.
//
// Prerequisite: the wasm build must already exist at www/pkg/ (see
// scripts/build-wasm.sh).
//
// Run with:
//   NODE_PATH=/usr/local/lib/node_modules node tests/e2e/gases_canvas.test.mjs

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

// Mean RGB of a horizontal band of the canvas, given as fractions of its
// height measured from the top of the image.
const bandColour = (page, fromFrac, toFrac) =>
  page.evaluate(([a, b]) => {
    const canvas = document.getElementById('canvas');
    const ctx = canvas.getContext('2d');
    const y0 = Math.floor(canvas.height * a);
    const y1 = Math.floor(canvas.height * b);
    const { data } = ctx.getImageData(0, y0, canvas.width, Math.max(1, y1 - y0));
    let r = 0;
    let g = 0;
    let bl = 0;
    const n = data.length / 4;
    for (let i = 0; i < data.length; i += 4) {
      r += data[i];
      g += data[i + 1];
      bl += data[i + 2];
    }
    return [r / n, g / n, bl / n];
  }, [fromFrac, toFrac]);

async function main() {
  const wasmPath = path.join(wwwDir, 'pkg', 'viewer_bg.wasm');
  try {
    await stat(wasmPath);
  } catch {
    fail(`FAIL gases_canvas: build artifacts missing at ${wasmPath}. Run scripts/build-wasm.sh first.`);
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

    await page.goto(`http://127.0.0.1:${port}/gases.html`);
    await page
      .waitForFunction(() => window.__chamberReady === true || window.__chamberError, {
        timeout: 10000,
      })
      .catch(() => {});

    const ready = await page.evaluate(() => window.__chamberReady);
    if (!ready) {
      const error = await page.evaluate(() => window.__chamberError);
      fail(
        `FAIL gases_canvas: page did not report ready ` +
          `(__chamberError=${error}, pageerror=${JSON.stringify(pageErrors)}).`
      );
      return;
    }

    await page.waitForFunction(() => window.__lastReport && window.__lastReport.step > 0, {
      timeout: 10000,
    });
    const first = await page.evaluate(() => window.__lastReport);
    // The same band the mixing is judged in, sampled before any CO2 can have
    // reached it (the slab starts at the ceiling): the colour of clean air
    // and walls in that part of the picture.
    const clean = await bandColour(page, 0.55, 0.72);

    // Real wall-clock time, not a fast-forward: the same rate a person
    // watching the tab would see. The page steps as fast as its frame budget
    // allows, so how many steps that is depends on the machine; wait for
    // enough simulated time for the mixing to show, with a ceiling.
    await page.waitForTimeout(6000);
    await page
      .waitForFunction(() => window.__lastReport && window.__lastReport.step >= 4000, {
        timeout: 40000,
      })
      .catch(() => {});

    const second = await page.evaluate(() => window.__lastReport);
    // Bands measured from the top of the image. The floor band is the
    // bottom tenth of the room; the middle band sits well above it, below
    // the shelf — clean air, if the CO2 had merely settled.
    const floorBand = await bandColour(page, 0.86, 0.96);
    const middleBand = await bandColour(page, 0.55, 0.72);

    if (!(second.step > first.step)) {
      fail(`FAIL gases_canvas: simulation did not advance (${first.step} -> ${second.step}).`);
    }
    if (!(second.step >= 4000)) {
      fail(`FAIL gases_canvas: only ${second.step} steps in the time allowed — too slow to judge.`);
    }

    // It sank: heavier than air, by the same rule that sinks sand.
    if (!(second.co2_mean_height < first.co2_mean_height / 2)) {
      fail(
        `FAIL gases_canvas: the CO2 did not sink ` +
          `(mean height ${first.co2_mean_height} -> ${second.co2_mean_height}).`
      );
    }
    // ...and then it mixed: the profile leans toward the floor but does not
    // run to zero at the ceiling.
    const prof = second.co2_profile.filter((f) => f >= 0);
    const floor = prof[0];
    const ceiling = prof[prof.length - 1];
    if (!(floor > 2 * ceiling)) {
      fail(`FAIL gases_canvas: no lean toward the floor (floor ${floor}, ceiling ${ceiling}).`);
    }
    if (!(ceiling > 0.1 * floor)) {
      fail(
        `FAIL gases_canvas: the CO2 is lying on the floor as a layer ` +
          `(floor ${floor}, ceiling ${ceiling}).`
      );
    }

    // ...and in pixels. The measure is redness (red minus green), not plain
    // brightness, and that distinction was found the hard way: the page
    // brightens a gas cell in proportion to its pressure, so as the bottle
    // vents into the room *every* band gets brighter and a brightness test
    // passes for the wrong reason. CO2 paints amber (red well above green)
    // where air paints blue-grey (red below green), and the pressure shading
    // adds the same amount to every channel, so red-minus-green tracks how
    // much CO2 is in a band and ignores how compressed it is.
    const redness = ([r, g]) => r - g;
    if (!(redness(middleBand) > redness(clean) + 4)) {
      fail(
        `FAIL gases_canvas: no CO2 visible above the floor — a layer, not a mixture ` +
          `(middle ${JSON.stringify(middleBand)} vs clean air ${JSON.stringify(clean)}).`
      );
    }
    if (!(redness(floorBand) > redness(middleBand) + 2)) {
      fail(
        `FAIL gases_canvas: the floor is not visibly richer than the room above it ` +
          `(floor ${JSON.stringify(floorBand)} vs middle ${JSON.stringify(middleBand)}).`
      );
    }

    // The bottle vents: the room's pressure spread must collapse.
    if (!(second.pressure_max - second.pressure_min <
          0.2 * (first.pressure_max - first.pressure_min))) {
      fail(
        `FAIL gases_canvas: the room's pressure spread did not collapse ` +
          `(${first.pressure_max - first.pressure_min} -> ` +
          `${second.pressure_max - second.pressure_min}).`
      );
    }

    for (const report of [first, second]) {
      if (Math.abs(report.residual_mass_relative) > 1e-5) {
        fail(`FAIL gases_canvas: mass residual ${report.residual_mass_relative} at step ${report.step}.`);
      }
      if (Math.abs(report.residual_energy_relative) > 1e-5) {
        fail(`FAIL gases_canvas: energy residual ${report.residual_energy_relative} at step ${report.step}.`);
      }
    }
    if (pageErrors.length > 0) {
      fail(`FAIL gases_canvas: page errors ${JSON.stringify(pageErrors)}.`);
    }

    if (!failed) {
      console.log(
        `PASS gases_canvas: advanced ${first.step} -> ${second.step} steps, ` +
          `CO2 mean height ${first.co2_mean_height.toFixed(2)} -> ${second.co2_mean_height.toFixed(2)}, ` +
          `profile floor ${(100 * floor).toFixed(1)}% ceiling ${(100 * ceiling).toFixed(1)}%, ` +
          `redness clean ${redness(clean).toFixed(1)} middle ${redness(middleBand).toFixed(1)} ` +
          `floor ${redness(floorBand).toFixed(1)}, ` +
          `pressure spread ${(first.pressure_max - first.pressure_min).toFixed(4)} -> ` +
          `${(second.pressure_max - second.pressure_min).toFixed(4)}, ` +
          `mass residual ${second.residual_mass_relative}.`
      );
    }
  } finally {
    await browser.close();
    server.close();
  }
}

main()
  .catch((e) => fail(`FAIL gases_canvas: ${e && e.stack ? e.stack : e}`))
  .finally(() => process.exit(failed ? 1 : 0));
