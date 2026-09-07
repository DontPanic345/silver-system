// Scenario: "A person opens www/gases.html in a real browser tab and watches
// the CO2 settle" — a headless Playwright check on the gas chamber.
//
// The point of this file is that it does not trust the simulation's own
// summary of itself. It reads two independent things and requires them to
// agree:
//
//  1. The JSON the page exposes: the air's pressure spread must collapse as
//     the bottle vents, the CO2's mean height must fall to the floor, and
//     the conservation residuals must stay at noise.
//  2. The actual canvas pixels: the bottom rows of the image must end up
//     visibly more CO2-coloured than they started, and the top rows less so.
//     That is the claim "the CO2 sank" checked against what a person would
//     actually see, rather than against the number the simulation computed.
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
    // The CO2 starts as a slab near the ceiling, so the top band is the one
    // holding it at the start.
    const topBefore = await bandColour(page, 0.05, 0.25);
    const bottomBefore = await bandColour(page, 0.85, 0.98);

    // Real wall-clock time, not a fast-forward: this is the same rate a
    // person watching the tab would see.
    await page.waitForTimeout(6000);

    const second = await page.evaluate(() => window.__lastReport);
    const topAfter = await bandColour(page, 0.05, 0.25);
    const bottomAfter = await bandColour(page, 0.85, 0.98);

    if (!(second.step > first.step)) {
      fail(`FAIL gases_canvas: simulation did not advance (${first.step} -> ${second.step}).`);
    }

    // The CO2 layer, in numbers.
    if (!(second.co2_mean_height < 4)) {
      fail(
        `FAIL gases_canvas: CO2 did not settle on the floor ` +
          `(mean height ${first.co2_mean_height} -> ${second.co2_mean_height}, ` +
          `in a room ${'32'} rows tall).`
      );
    }
    // ...and in pixels. The measure is redness (red minus green), not plain
    // brightness, and that distinction was found the hard way: the page
    // brightens a gas cell in proportion to its pressure, so as the bottle
    // vents into the room *every* band gets brighter and a brightness test
    // passes for the wrong reason. CO2 paints purple (red above green) where
    // air paints blue-grey (red below green), and the pressure shading adds
    // the same amount to every channel, so red-minus-green tracks how much
    // CO2 is in a band and ignores how compressed it is.
    const brightness = ([r, g]) => r - g;
    if (!(brightness(bottomAfter) > brightness(bottomBefore) + 2)) {
      fail(
        `FAIL gases_canvas: the floor never filled with CO2 in the actual pixels ` +
          `(${JSON.stringify(bottomBefore)} -> ${JSON.stringify(bottomAfter)}).`
      );
    }
    // The other half of "it settled into a layer": the floor must be
    // redder than the ceiling at the end, i.e. the CO2 is *down there* and
    // not spread evenly through the room.
    //
    // Deliberately a comparison between two bands of the same final frame
    // rather than before-and-after at the ceiling. The CO2 falls the height
    // of the room in well under a second of wall-clock time, which is faster
    // than this script can take its "before" sample — an earlier version
    // compared the ceiling against itself and was really comparing two
    // already-settled frames.
    if (!(brightness(bottomAfter) > brightness(topAfter) + 5)) {
      fail(
        `FAIL gases_canvas: no CO2 layer visible on the floor at the end ` +
          `(floor ${JSON.stringify(bottomAfter)} vs ceiling ${JSON.stringify(topAfter)}).`
      );
    }

    // The bottle vents: the air's pressure spread must be shrinking.
    if (!(second.air_pressure_max - second.air_pressure_min <
          first.air_pressure_max - first.air_pressure_min)) {
      fail(
        `FAIL gases_canvas: the air's pressure spread did not shrink ` +
          `(${first.air_pressure_max - first.air_pressure_min} -> ` +
          `${second.air_pressure_max - second.air_pressure_min}).`
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
          `floor redness ${brightness(bottomBefore).toFixed(1)} -> ${brightness(bottomAfter).toFixed(1)}, ` +
          `ceiling redness ${brightness(topBefore).toFixed(1)} -> ${brightness(topAfter).toFixed(1)}, ` +
          `air pressure spread ${(first.air_pressure_max - first.air_pressure_min).toFixed(4)} -> ` +
          `${(second.air_pressure_max - second.air_pressure_min).toFixed(4)}, ` +
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
