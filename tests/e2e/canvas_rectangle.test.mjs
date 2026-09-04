// Scenario (durable): "A person opens the built M0.1 page in a real browser
// tab and watches the rectangle actually change over time, once per fixed
// tick — proof this is a running program, not a single paint call."
//
// This is the round's headless stand-in for a human looking at a screenshot,
// per project preference (see memory: verify sims with headless numbers, not
// screenshots). It serves the built www/ directory over plain HTTP, drives a
// real headless Chromium via Playwright, and reads back actual canvas pixel
// data with getImageData at two distinct points in time (tick 0, then a
// later tick) and asserts they differ — it does not look at a PNG, and it is
// not a single static-frame check.
//
// Round 2 fixes forward the duplication round 1's Refactor flagged: the
// expected colour/coordinate/tick-interval values are no longer hardcoded
// here. They are read at runtime from the already-loaded wasm module via
// `window.__wasm` (see www/index.html), which exposes the same
// `rect_x`/`rect_y`/`rect_w`/`rect_h`/`rect_color_rgb`/`rect_color_rgb_alt`/
// `tick_interval_ms` getters defined once in src/lib.rs. This file no longer
// declares a single RECT_* literal.
//
// Prerequisite (not done by this script): the wasm build must already exist
// at www/pkg/viewer.js + www/pkg/viewer_bg.wasm. Build it with
// scripts/build-wasm.sh, or by hand:
//
//   cargo build --release --target wasm32-unknown-unknown
//   ~/.cargo/bin/wasm-bindgen --target web --out-dir www/pkg \
//     target/wasm32-unknown-unknown/release/viewer.wasm
//
// Run this test with:
//   NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs
//
// Exit code 0 = pass, 1 = fail. Prints a one-line reason either way.

import http from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

// Playwright is installed globally under NODE_PATH=/usr/local/lib/node_modules
// (see .claude/settings.json). Node's ESM loader does NOT consult NODE_PATH
// for `import`/dynamic `import()` (confirmed round 1 — it resolves fine
// under CommonJS `require` but throws ERR_MODULE_NOT_FOUND under `import()`
// with the identical NODE_PATH set). `createRequire` gives us the CJS
// resolver, which does honour NODE_PATH, without switching this whole file
// to CommonJS.
const require = createRequire(import.meta.url);

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, '..', '..');
const wwwDir = path.join(repoRoot, 'www');

// A point safely outside the rectangle regardless of which tick painted it
// (rectangle geometry doesn't move this round, only colour does), expected
// to stay untouched (transparent black canvas backing, i.e. alpha 0) at
// every sample. This is a canvas-dimension choice (200x150, see
// www/index.html), not one of the duplicated RECT_* values, so it stays a
// literal here.
const OUTSIDE = { x: 150, y: 120 };

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
      `FAIL canvas_rectangle: build artifacts missing at ${wasmPath} / ${jsPath}. ` +
        `Run scripts/build-wasm.sh (or the two build commands documented at the ` +
        `top of this file) first.`
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

    await page.goto(`http://127.0.0.1:${port}/index.html`);

    // Give the wasm module a moment to instantiate and either resolve or
    // reject its init() promise (see www/index.html).
    await page.waitForFunction(
      () => window.__viewerReady === true || window.__viewerError !== null,
      { timeout: 5000 }
    ).catch(() => {
      // Neither flag got set within the timeout — fall through and let the
      // checks below report the failure with real data instead of a bare
      // timeout message.
    });

    const viewerError = await page.evaluate(() => window.__viewerError);
    const viewerReady = await page.evaluate(() => window.__viewerReady);

    if (!viewerReady) {
      fail(
        `FAIL canvas_rectangle: wasm module did not report ready ` +
          `(__viewerError=${viewerError}, pageerror=${JSON.stringify(pageErrors)}). ` +
          `Expected once Green implements the tick stubs: __viewerReady === true.`
      );
      return;
    }

    // Read the round's shared constants straight from the loaded wasm
    // module — the single source of truth (round 2, goal 2) — rather than
    // hardcoding them here.
    const constants = await page.evaluate(() => {
      const wasm = window.__wasm;
      return {
        rectX: wasm.rect_x(),
        rectY: wasm.rect_y(),
        color: Array.from(wasm.rect_color_rgb()),
        colorAlt: Array.from(wasm.rect_color_rgb_alt()),
        intervalMs: wasm.tick_interval_ms(),
      };
    });

    // A point safely inside the rectangle, regardless of which colour is
    // currently painted there.
    const sampleX = constants.rectX + 5;
    const sampleY = constants.rectY + 5;

    // --- Sample 1: tick 0, immediately after the module reports ready and
    // before any interval has had a chance to fire. ---
    const tick0 = await samplePixels(page, sampleX, sampleY);

    if (!colorsEqual(tick0.inside, constants.color)) {
      fail(
        `FAIL canvas_rectangle: tick-0 pixel at (${sampleX},${sampleY}) is ` +
          `rgba(${tick0.inside.join(',')}), expected rgba(${constants.color.join(',')},255) ` +
          `(the tick-0 / even-tick colour, per rect_color_rgb()).`
      );
      return;
    }
    if (tick0.outside[3] !== 0) {
      fail(
        `FAIL canvas_rectangle: tick-0 pixel at (${OUTSIDE.x},${OUTSIDE.y}), which should be ` +
          `outside the rectangle, is not transparent (alpha=${tick0.outside[3]}). ` +
          `The rectangle may be drawn too large or in the wrong place.`
      );
      return;
    }

    // --- Sample 2: wait for at least one real tick to have fired (real
    // wall-clock interval, not a bypassed/pure-function call — this proves
    // the actual running loop works end to end, per the milestone intent),
    // then sample the same point again. ---
    const waitTimeoutMs = Math.max(5000, constants.intervalMs * 20);
    let sawTick = true;
    await page.waitForFunction(() => window.__tickCount >= 1, { timeout: waitTimeoutMs }).catch(() => {
      sawTick = false;
    });

    if (!sawTick) {
      fail(
        `FAIL canvas_rectangle: window.__tickCount never reached 1 within ` +
          `${waitTimeoutMs}ms (tick_interval_ms()=${constants.intervalMs}). Expected ` +
          `tick_and_draw() to be called on a ${constants.intervalMs}ms setInterval ` +
          `and return an incrementing count.`
      );
      return;
    }

    const tick1 = await samplePixels(page, sampleX, sampleY);

    if (colorsEqual(tick1.inside, tick0.inside)) {
      fail(
        `FAIL canvas_rectangle: pixel at (${sampleX},${sampleY}) is identical at tick 0 and ` +
          `after tick advanced (both rgba(${tick0.inside.join(',')})). Expected the rectangle ` +
          `to visibly change once per tick — color_for_tick() should alternate between ` +
          `rect_color_rgb() and rect_color_rgb_alt().`
      );
      return;
    }
    if (!colorsEqual(tick1.inside, constants.colorAlt)) {
      fail(
        `FAIL canvas_rectangle: post-tick pixel at (${sampleX},${sampleY}) is ` +
          `rgba(${tick1.inside.join(',')}), expected rgba(${constants.colorAlt.join(',')},255) ` +
          `(the odd-tick colour, per rect_color_rgb_alt()).`
      );
      return;
    }
    if (tick1.outside[3] !== 0) {
      fail(
        `FAIL canvas_rectangle: post-tick pixel at (${OUTSIDE.x},${OUTSIDE.y}), which should be ` +
          `outside the rectangle, is not transparent (alpha=${tick1.outside[3]}).`
      );
      return;
    }

    pass(
      'PASS canvas_rectangle: rectangle pixel at tick 0 matches rect_color_rgb(), differs after ' +
        'a real tick and matches rect_color_rgb_alt(), outside pixel untouched throughout.'
    );
  } finally {
    await browser.close();
    server.close();
  }
}

async function samplePixels(page, sampleX, sampleY) {
  const pixels = await page.evaluate(
    ({ sampleX, sampleY, outsideX, outsideY }) => {
      const canvas = document.getElementById('canvas');
      const ctx = canvas.getContext('2d');
      const inside = ctx.getImageData(sampleX, sampleY, 1, 1).data;
      const outside = ctx.getImageData(outsideX, outsideY, 1, 1).data;
      return { inside: Array.from(inside), outside: Array.from(outside) };
    },
    { sampleX, sampleY, outsideX: OUTSIDE.x, outsideY: OUTSIDE.y }
  );
  return pixels;
}

function colorsEqual(rgba, rgb) {
  return rgba[0] === rgb[0] && rgba[1] === rgb[1] && rgba[2] === rgb[2] && rgba[3] === 255;
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
  fail(`FAIL canvas_rectangle: unexpected error in test harness: ${e && e.stack ? e.stack : e}`);
});
