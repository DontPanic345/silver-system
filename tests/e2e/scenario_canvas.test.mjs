// Scenario (durable): "A person opens www/scenario.html in a real browser
// tab and watches stone_and_water_pool()'s grid rendered as a coloured
// image on a canvas" — round 4 goal 4's wasm/Playwright headless empirical
// check (the native/PNG path is also exercised, in tests/render_native.rs;
// both are used this round, see round-04.md for why).
//
// Reads real canvas pixel data with getImageData at specific coordinates,
// mirroring tests/e2e/canvas_rectangle.test.mjs's established approach —
// per the round's explicit must-not-break condition, that existing file is
// untouched; this is a second, parallel e2e test.
//
// Expected colours/cell size are read at runtime from the already-loaded
// wasm module via window.__wasm (scenario_cell_px, scenario_*_colour_rgb),
// not hardcoded here — same "single source of truth" discipline
// canvas_rectangle.test.mjs already established for the M0.1 rectangle.
//
// Prerequisite: the wasm build must already exist at www/pkg/ (see
// canvas_rectangle.test.mjs's header for the build commands).
//
// Run with:
//   NODE_PATH=/usr/local/lib/node_modules node tests/e2e/scenario_canvas.test.mjs

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
      `FAIL scenario_canvas: build artifacts missing at ${wasmPath} / ${jsPath}. ` +
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

    await page.goto(`http://127.0.0.1:${port}/scenario.html`);

    await page.waitForFunction(
      () => window.__scenarioReady === true || window.__scenarioError !== null,
      { timeout: 5000 }
    ).catch(() => {});

    const scenarioError = await page.evaluate(() => window.__scenarioError);
    const scenarioReady = await page.evaluate(() => window.__scenarioReady);

    if (!scenarioReady) {
      fail(
        `FAIL scenario_canvas: paint_scenario did not report ready ` +
          `(__scenarioError=${scenarioError}, pageerror=${JSON.stringify(pageErrors)}).`
      );
      return;
    }

    const constants = await page.evaluate(() => {
      const wasm = window.__wasm;
      return {
        cellPx: wasm.scenario_cell_px(),
        air: Array.from(wasm.scenario_air_colour_rgb()),
        water: Array.from(wasm.scenario_water_colour_rgb()),
        stone: Array.from(wasm.scenario_stone_colour_rgb()),
      };
    });

    // Fixture layout (src/scenario.rs's stone_and_water_pool, 6x4 grid):
    // stone lump at grid i,j in {0,1}x{0,1}; water pool at grid (5,0)/(5,1);
    // rest air. Image row is flipped (src/render.rs: grid j=height-1-j), so
    // grid j=0 is the *bottom* image row: image_row = 3 - j (height=4).
    const sample = async (gridI, gridJ) => {
      const imageRow = 3 - gridJ;
      const x = gridI * constants.cellPx + Math.floor(constants.cellPx / 2);
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

    const checks = [
      { name: 'stone lump (0,0)', px: await sample(0, 0), expected: constants.stone },
      { name: 'stone lump (1,1)', px: await sample(1, 1), expected: constants.stone },
      { name: 'water pool (5,0)', px: await sample(5, 0), expected: constants.water },
      { name: 'water pool (5,1)', px: await sample(5, 1), expected: constants.water },
      { name: 'background air (3,3)', px: await sample(3, 3), expected: constants.air },
    ];

    for (const check of checks) {
      if (!colorsEqual(check.px, check.expected)) {
        fail(
          `FAIL scenario_canvas: ${check.name} is rgba(${check.px.join(',')}), ` +
            `expected rgba(${check.expected.join(',')},255).`
        );
        return;
      }
    }

    pass(
      'PASS scenario_canvas: paint_scenario renders stone_and_water_pool() with the correct ' +
        'material colour at the stone lump, the water pool, and a background air cell.'
    );
  } finally {
    await browser.close();
    server.close();
  }
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
  fail(`FAIL scenario_canvas: unexpected error in test harness: ${e && e.stack ? e.stack : e}`);
});
