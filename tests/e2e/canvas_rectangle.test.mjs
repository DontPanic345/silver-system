// Scenario (durable): "A person opens the built M0.1 page in a real browser
// and sees a coloured rectangle painted onto the canvas."
//
// This is the round's headless stand-in for a human looking at a screenshot,
// per project preference (see memory: verify sims with headless numbers, not
// screenshots). It serves the built www/ directory over plain HTTP, drives a
// real headless Chromium via Playwright, and reads back actual canvas pixel
// data with getImageData — it does not look at a PNG.
//
// Prerequisite (not done by this script): the wasm build must already exist
// at www/pkg/viewer.js + www/pkg/viewer_bg.wasm. Build it first with:
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
// for `import`/dynamic `import()` (confirmed this round — it resolves fine
// under CommonJS `require` but throws ERR_MODULE_NOT_FOUND under `import()`
// with the identical NODE_PATH set). `createRequire` gives us the CJS
// resolver, which does honour NODE_PATH, without switching this whole file
// to CommonJS.
const require = createRequire(import.meta.url);

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, '..', '..');
const wwwDir = path.join(repoRoot, 'www');

// Kept in sync BY HAND with the RECT_* / RECT_COLOR_RGB constants in
// src/lib.rs — see round-01 log, "Compromises I made", for why this
// duplication exists and what would remove it.
const EXPECTED = {
  color: [200, 60, 60], // RECT_COLOR_RGB
  // A point safely inside the rectangle (RECT_X=20, RECT_Y=20, RECT_W=60, RECT_H=40).
  sampleX: 40,
  sampleY: 40,
};
// A point safely outside the rectangle, expected to stay untouched
// (transparent black canvas backing, i.e. alpha 0) both before and after a
// correct draw.
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
        `Run the two build commands documented at the top of this file first.`
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
      // pixel check below report the failure with real data instead of a
      // bare timeout message.
    });

    const viewerError = await page.evaluate(() => window.__viewerError);
    const viewerReady = await page.evaluate(() => window.__viewerReady);

    const pixels = await page.evaluate(
      ({ sampleX, sampleY, outsideX, outsideY }) => {
        const canvas = document.getElementById('canvas');
        const ctx = canvas.getContext('2d');
        const inside = ctx.getImageData(sampleX, sampleY, 1, 1).data;
        const outside = ctx.getImageData(outsideX, outsideY, 1, 1).data;
        return { inside: Array.from(inside), outside: Array.from(outside) };
      },
      { sampleX: EXPECTED.sampleX, sampleY: EXPECTED.sampleY, outsideX: OUTSIDE.x, outsideY: OUTSIDE.y }
    );

    const [r, g, b, a] = pixels.inside;
    const colorMatches =
      r === EXPECTED.color[0] && g === EXPECTED.color[1] && b === EXPECTED.color[2] && a === 255;
    const outsideUntouched = pixels.outside[3] === 0;

    if (!viewerReady) {
      fail(
        `FAIL canvas_rectangle: wasm module did not report ready ` +
          `(__viewerError=${viewerError}, pageerror=${JSON.stringify(pageErrors)}). ` +
          `Expected once Green implements draw(): __viewerReady === true.`
      );
      return;
    }

    if (!colorMatches) {
      fail(
        `FAIL canvas_rectangle: pixel at (${EXPECTED.sampleX},${EXPECTED.sampleY}) is ` +
          `rgba(${r},${g},${b},${a}), expected rgba(${EXPECTED.color.join(',')},255). ` +
          `(__viewerError=${viewerError})`
      );
      return;
    }

    if (!outsideUntouched) {
      fail(
        `FAIL canvas_rectangle: pixel at (${OUTSIDE.x},${OUTSIDE.y}), which should be ` +
          `outside the rectangle, is not transparent (alpha=${pixels.outside[3]}). ` +
          `The rectangle may be drawn too large or in the wrong place.`
      );
      return;
    }

    pass('PASS canvas_rectangle: rectangle pixel matches expected colour, outside pixel untouched.');
  } finally {
    await browser.close();
    server.close();
  }
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
