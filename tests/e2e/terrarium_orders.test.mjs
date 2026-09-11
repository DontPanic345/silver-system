// Scenario: "A person opens www/terrarium.html, picks up the dig tool,
// clicks a cell of the meadow, and watches a gnome walk over and dig it" —
// the glass pane (src/order.rs) driven the way a human drives it, in a real
// browser, with a real mouse, and checked in real canvas pixels.
//
// What this asserts that a unit test cannot:
//
//  1. A real `page.mouse.click` on the canvas lands on the cell the person
//     was pointing at. The page has to map a pixel to a grid cell, including
//     the vertical flip (image row 0 is the top of the world, j counts up
//     from the bottom); an off-by-one there is invisible to every Rust test
//     in the repo and glaring to a human.
//  2. The order is *visible* before it is done: the marker outline appears
//     in the actual pixels of that cell.
//  3. The world visibly changes where the person clicked, after real
//     wall-clock time, with nobody driving the simulation but the page's own
//     animation loop. The dug cell is compared against its undug neighbour
//     in the same frame, so the check survives the day/night dimming that
//     night 5 added.
//  4. The mass goes somewhere. While the spoil is in a gnome's hands the
//     page reports it as carried, and the conservation residual stays flat —
//     digging is not a hole in the books.
//  5. Building puts it back: a second click, a second walk, and a cell of
//     air becomes a cell of sand at a place the player chose.
//
// Prerequisite: the wasm build must already exist at www/pkg/ (see
// scripts/build-wasm.sh).
//
// Run with:
//   NODE_PATH=/usr/local/lib/node_modules node tests/e2e/terrarium_orders.test.mjs

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

// The mean channel value of one world cell, read out of the live canvas.
// `inset` picks how far in from the cell's corner to sample: the middle for
// the material itself, a pixel or two in for the order marker's outline.
const cellPixel = (page, i, j, inset) =>
  page.evaluate(
    ([i, j, inset]) => {
      const canvas = document.getElementById('canvas');
      const px = window.__cellPx;
      const rows = canvas.height / px;
      const ctx = canvas.getContext('2d');
      const x = i * px + (inset === null ? Math.floor(px / 2) : inset);
      const y = (rows - 1 - j) * px + (inset === null ? Math.floor(px / 2) : inset);
      const d = ctx.getImageData(x, y, 1, 1).data;
      return { r: d[0], g: d[1], b: d[2], mean: (d[0] + d[1] + d[2]) / 3 };
    },
    [i, j, inset]
  );

// Where in the viewport the middle of world cell (i, j) is — against the
// canvas's *content* box, because it has a border and the page's own
// pixel-to-cell mapping has to allow for that too.
//
// Scrolls the jar into view first. Clicking one of the tool buttons scrolls
// the page to it, which is exactly what a browser does for a person too;
// what a person then does is scroll back, and so does this.
const cellPoint = (page, i, j) =>
  page.evaluate(
    ([i, j]) => {
      const canvas = document.getElementById('canvas');
      canvas.scrollIntoView({ block: 'center' });
      const r = canvas.getBoundingClientRect();
      const px = window.__cellPx * (canvas.clientWidth / canvas.width);
      const rows = canvas.height / window.__cellPx;
      return {
        x: r.left + canvas.clientLeft + (i + 0.5) * px,
        y: r.top + canvas.clientTop + (rows - 1 - j + 0.5) * px,
      };
    },
    [i, j]
  );

// Clicks the middle of world cell (i, j) with a real mouse.
async function clickCell(page, i, j) {
  const point = await cellPoint(page, i, j);
  await page.mouse.click(point.x, point.y);
}

const pickTool = (page, tool) => page.click(`button[data-tool="${tool}"]`);

const report = (page) => page.evaluate(() => window.__lastReport);

async function waitForOrders(page, done, timeout) {
  await page
    .waitForFunction(
      (done) => window.__lastReport && window.__lastReport.orders.completed >= done,
      done,
      { timeout }
    )
    .catch(() => {});
}

async function main() {
  const wasmPath = path.join(wwwDir, 'pkg', 'viewer_bg.wasm');
  try {
    await stat(wasmPath);
  } catch {
    fail(`FAIL terrarium_orders: build artifacts missing at ${wasmPath}. Run scripts/build-wasm.sh first.`);
    return;
  }

  const { chromium } = require('playwright');
  const server = serveDir(wwwDir);
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const port = server.address().port;

  const browser = await chromium.launch();
  try {
    // Tall enough that the whole jar is inside the viewport: a real mouse
    // cannot click a cell that is scrolled off the bottom of the window,
    // and neither can this.
    const page = await browser.newPage({ viewport: { width: 1280, height: 1100 } });
    const pageErrors = [];
    page.on('pageerror', (e) => pageErrors.push(String(e)));

    await page.goto(`http://127.0.0.1:${port}/terrarium.html`);
    await page
      .waitForFunction(() => window.__terrariumReady === true || window.__terrariumError !== null, {
        timeout: 10000,
      })
      .catch(() => {});
    if (!(await page.evaluate(() => window.__terrariumReady))) {
      fail(
        `FAIL terrarium_orders: page did not report ready ` +
          `(__terrariumError=${await page.evaluate(() => window.__terrariumError)}, ` +
          `pageerror=${JSON.stringify(pageErrors)}).`
      );
      return;
    }
    await page.waitForFunction(() => window.__lastReport && window.__lastReport.step > 20, {
      timeout: 10000,
    });

    // The meadow the gnomes walk on: sand at j = 3, and the cell next door
    // is the control this test compares against.
    const HOLE = [10, 3];
    const CONTROL = [13, 3];
    const WALL = [12, 4];

    // Pause the jar to mark it up, the way a person would. Cells are sampled
    // one pixel in from their corner rather than in the middle: the middle
    // is where a gnome standing in the cell is drawn, and this test is about
    // the ground, not about who happens to be on it.
    const CORNER = 1;
    await page.click('#pause');
    await page.evaluate(() => window.__repaint());
    const before = await cellPixel(page, ...HOLE, CORNER);
    const control = await cellPixel(page, ...CONTROL, CORNER);
    if (!(before.mean > 40)) {
      fail(`FAIL terrarium_orders: the meadow cell reads as dark already (${JSON.stringify(before)}).`);
    }

    // --- Point at it first: the inspector is the "see" half of the pane ---
    await page.mouse.move(10, 10);
    const point = await cellPoint(page, ...HOLE);
    await page.mouse.move(point.x, point.y);
    await page
      .waitForFunction(() => /\(10, 3\) sand/.test(document.getElementById('inspector').textContent),
        { timeout: 3000 })
      .catch(() => {});
    const readout = await page.textContent('#inspector');
    if (!/\(10, 3\) sand/.test(readout)) {
      fail(`FAIL terrarium_orders: inspector says "${readout}" for the sand at (10, 3).`);
    }

    // --- Write "dig" on it with a real click ---
    await pickTool(page, 'dig');
    await clickCell(page, ...HOLE);
    const marked = await report(page);
    const ordered = await page.evaluate(() => window.__cell(10, 3));
    if (ordered.order !== 'dig') {
      fail(`FAIL terrarium_orders: the click landed on (${ordered.i}, ${ordered.j}) — ${ordered.order}.`);
    }
    // The marker, in pixels: the outline is drawn in the job's amber, two
    // pixels thick, so a pixel near the corner is much redder than blue.
    // Checked with the jar paused, because a gnome standing next to the cell
    // will dig it on the very next step and there would be nothing to see.
    await page.evaluate(() => window.__repaint());
    const marker = await cellPixel(page, ...HOLE, CORNER);
    if (!(marker.r > marker.b + 60)) {
      fail(`FAIL terrarium_orders: no dig marker visible at the cell (${JSON.stringify(marker)}).`);
    }

    // --- And watch somebody do it ---
    await page.click('#pause');
    await waitForOrders(page, 1, 30000);
    const dug = await report(page);
    if (dug.orders.completed < 1) {
      fail(`FAIL terrarium_orders: nobody dug the hole within 30 s (${JSON.stringify(dug.orders)}).`);
    }
    if (!(dug.colony.carried_g > 0)) {
      fail(`FAIL terrarium_orders: the spoil vanished — carried_g is ${dug.colony.carried_g}.`);
    }
    // The ledger is holding exactly the spoil against the world, so the
    // residual — the thing that says nothing was created or destroyed off
    // the books — must not have moved.
    if (!(Math.abs(dug.residual_mass_relative) < 1e-6)) {
      fail(`FAIL terrarium_orders: mass residual ${dug.residual_mass_relative} after digging.`);
    }

    const after = await cellPixel(page, ...HOLE, CORNER);
    const controlAfter = await cellPixel(page, ...CONTROL, CORNER);
    // Compared against the undug meadow beside it *in the same frame*, so
    // this survives the whole jar dimming at night.
    if (!(after.mean < 0.6 * controlAfter.mean)) {
      fail(
        `FAIL terrarium_orders: the hole does not look like a hole — ` +
          `dug cell ${after.mean.toFixed(1)} vs meadow ${controlAfter.mean.toFixed(1)}.`
      );
    }

    // --- Now put it back down somewhere else ---
    await pickTool(page, 'build');
    // Read the empty cell *before* marking it: the order's own outline is
    // drawn on the cell's border, which is exactly where this samples.
    const airBefore = await cellPixel(page, ...WALL, CORNER);
    await clickCell(page, ...WALL);
    await waitForOrders(page, 2, 30000);
    const built = await report(page);
    if (built.orders.completed < 2) {
      fail(`FAIL terrarium_orders: nobody built the wall within 30 s (${JSON.stringify(built.orders)}).`);
    }
    if (!(built.colony.carried_g < 1e-6)) {
      fail(`FAIL terrarium_orders: hands still full after building (${built.colony.carried_g} g).`);
    }
    const wall = await page.evaluate(() => window.__cell(12, 4));
    if (wall.material !== 'sand') {
      fail(`FAIL terrarium_orders: the built cell is ${wall.material}, not sand.`);
    }
    const wallPx = await cellPixel(page, ...WALL, CORNER);
    if (!(wallPx.mean > airBefore.mean + 20)) {
      fail(
        `FAIL terrarium_orders: the wall is not visible — ` +
          `${wallPx.mean.toFixed(1)} where the air read ${airBefore.mean.toFixed(1)}.`
      );
    }
    if (!(Math.abs(built.residual_mass_relative) < 1e-6)) {
      fail(`FAIL terrarium_orders: mass residual ${built.residual_mass_relative} after building.`);
    }
    if (pageErrors.length > 0) {
      fail(`FAIL terrarium_orders: page errors ${JSON.stringify(pageErrors)}.`);
    }

    if (!failed) {
      console.log(
        `PASS terrarium_orders: clicked (10, 3) at step ${marked.step}, dug by step ${dug.step} ` +
          `(cell ${before.mean.toFixed(0)} -> ${after.mean.toFixed(0)} mean channel against ` +
          `${controlAfter.mean.toFixed(0)} beside it, ${dug.colony.carried_g.toFixed(2)} g in hand), ` +
          `built at (12, 4) by step ${built.step} ` +
          `(${airBefore.mean.toFixed(0)} -> ${wallPx.mean.toFixed(0)}), ` +
          `mass residual ${built.residual_mass_relative}, control cell steady at ` +
          `${control.mean.toFixed(0)} -> ${controlAfter.mean.toFixed(0)}.`
      );
    }
  } finally {
    await browser.close();
    server.close();
  }
}

main()
  .catch((e) => fail(`FAIL terrarium_orders: ${e && e.stack ? e.stack : e}`))
  .finally(() => process.exit(failed ? 1 : 0));
