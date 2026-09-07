// Scenario: "A person opens www/terrarium.html in a real browser tab and
// watches the gnome terrarium run" — a headless Playwright check that the
// wasm build genuinely simulates in the browser, rather than merely loading.
//
// Two things are asserted, and neither is a screenshot:
//
//  1. The world *advances*: the step counter moves and the canvas pixels
//     change between two samples taken a second apart. A page that loaded
//     the module but never ticked would pass a "did it render" check and
//     fail this one.
//  2. The conservation invariant holds in the browser build too. The page
//     exposes the same JSON snapshot the headless runner prints, so this
//     reads `residual_mass_relative` / `residual_energy_relative` straight
//     out of the running simulation — the numbers, not a picture of them.
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

    if (!(second.step > first.step)) {
      fail(`FAIL terrarium_canvas: simulation did not advance (${first.step} -> ${second.step}).`);
    }
    if (firstPixels === secondPixels) {
      fail('FAIL terrarium_canvas: canvas pixels never changed — the world is not visibly running.');
    }
    for (const report of [first, second]) {
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
    if (pageErrors.length > 0) {
      fail(`FAIL terrarium_canvas: page errors ${JSON.stringify(pageErrors)}.`);
    }

    if (!failed) {
      console.log(
        `PASS terrarium_canvas: advanced ${first.step} -> ${second.step} steps, ` +
          `pixels changed, mass residual ${second.residual_mass_relative}, ` +
          `energy residual ${second.residual_energy_relative}, ` +
          `${second.colony.embodied} gnomes embodied.`
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
