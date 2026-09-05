// Scenario: "A person opens www/physics.html in a real browser tab and
// watches scenario::physics_demo() actually fall/sink/level under real
// gravity/density physics" — the watchable counterpart to
// src/scenario.rs's own headless
// physics_demo_settles_every_suspended_grain_after_enough_steps test.
//
// Headless-JSON discipline, not a screenshot: reads real canvas pixel data
// via getImageData at the exact grid cell one of the fixture's sand grains
// starts in (src/scenario.rs's physics_demo doc comment: a grain at grid
// (3, 10)), before and after letting the page's own real-time timer loop
// run for a few seconds, and asserts that cell's colour changed from sand
// to something else — proof the grain actually moved under real elapsed
// time, not a frozen or single-frame render.
//
// Prerequisite: the wasm build must already exist at www/pkg/ (see
// tests/e2e/canvas_rectangle.test.mjs's header for the build commands).
//
// Run with:
//   NODE_PATH=/usr/local/lib/node_modules node tests/e2e/physics_demo.test.mjs

import http from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, '..', '..');
const wwwDir = path.join(repoRoot, 'www');

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

async function main() {
  const wasmPath = path.join(wwwDir, 'pkg', 'viewer_bg.wasm');
  const jsPath = path.join(wwwDir, 'pkg', 'viewer.js');
  try {
    await stat(wasmPath);
    await stat(jsPath);
  } catch {
    fail(
      `FAIL physics_demo: build artifacts missing at ${wasmPath} / ${jsPath}. ` +
        `Run scripts/build-wasm.sh first.`
    );
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

    await page.goto(`http://127.0.0.1:${port}/physics.html`);

    await page.waitForFunction(
      () => window.__physicsReady === true || window.__physicsError !== null,
      { timeout: 5000 }
    ).catch(() => {});

    const physicsError = await page.evaluate(() => window.__physicsError);
    const physicsReady = await page.evaluate(() => window.__physicsReady);

    if (!physicsReady) {
      fail(
        `FAIL physics_demo: step_and_paint_physics_demo did not report ready ` +
          `(__physicsError=${physicsError}, pageerror=${JSON.stringify(pageErrors)}).`
      );
      return;
    }

    const constants = await page.evaluate(() => {
      const wasm = window.__wasm;
      return { cellPx: wasm.scenario_cell_px(), sand: Array.from(wasm.scenario_sand_colour_rgb()) };
    });

    // Fixture (src/scenario.rs's physics_demo, 24x16 grid): a sand grain
    // starts at grid (3, 10). Image row is flipped (src/render.rs: grid
    // j=height-1-j), so image_row = 15 - 10 = 5.
    const sampleGrainCell = async () => {
      const imageRow = 15 - 10;
      const x = 3 * constants.cellPx + Math.floor(constants.cellPx / 2);
      const y = imageRow * constants.cellPx + Math.floor(constants.cellPx / 2);
      return page.evaluate(
        ({ x, y }) => {
          const canvas = document.getElementById('canvas');
          const ctx = canvas.getContext('2d');
          return Array.from(ctx.getImageData(x, y, 1, 1).data);
        },
        { x, y }
      );
    };

    const before = await sampleGrainCell();

    // Let the page's own real setInterval timer loop run for several
    // seconds of actual wall-clock time — real elapsed-time-driven
    // stepping, not a manually-forced single call.
    await page.waitForTimeout(4000);

    const after = await sampleGrainCell();
    const ticksAfter = await page.evaluate(() => window.__physicsTicks);

    if (ticksAfter <= 0) {
      fail(`FAIL physics_demo: expected a positive tick count after 4s, got ${ticksAfter}.`);
      return;
    }

    if (colorsEqual(before, after)) {
      fail(
        `FAIL physics_demo: the sand grain's starting cell (3,10) is still ` +
          `rgba(${after.join(',')}) after 4s of real time — it should have fallen away by now.`
      );
      return;
    }

    pass(
      `PASS physics_demo: after ${ticksAfter} real ticks, the sand grain's starting cell changed ` +
        `from rgba(${before.join(',')}) to rgba(${after.join(',')}) — it genuinely fell under real ` +
        'gravity/density physics, driven by real elapsed wall-clock time.'
    );
  } finally {
    await browser.close();
    server.close();
  }
}

function colorsEqual(a, b) {
  return a[0] === b[0] && a[1] === b[1] && a[2] === b[2] && a[3] === b[3];
}

function pass(msg) {
  console.log(msg);
  process.exitCode = 0;
}
function fail(msg) {
  console.error(msg);
  process.exitCode = 1;
}

main().catch((e) => {
  fail(`FAIL physics_demo: unexpected error in test harness: ${e && e.stack ? e.stack : e}`);
});
