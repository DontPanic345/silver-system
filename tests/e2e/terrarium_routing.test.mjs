// Scenario: "A person designates a dig at the far end of the jar, behind
// the garden, and watches a gnome find its own way there — and designates a
// second one inside the jar's wall, where nobody can get, and sees the
// difference on the glass."
//
// This is night 8's headline checked the way a human would check it. Until
// night 8 a gnome steered by the sign of the difference in column, so an
// order it could not walk to in a straight line was an order that might
// never be done, and the marker for "on its way" and the marker for "you
// have walled this off from us" were the same picture.
//
// What this asserts that a unit test cannot:
//
//  1. A real click on a cell at the *far* end of the jar — past the compost
//     heap, behind the garden — is carried out, by a gnome that has to route
//     the length of the walkway and climb to get there, in real wall-clock
//     time with nobody driving the simulation but the page's own loop.
//  2. The bush it dug is gone from the actual pixels, compared against a
//     bush still standing in the same frame (so the check survives the
//     day/night dimming).
//  3. An order nobody can route to is drawn differently — dashed and dim —
//     and the page says so in words. This is a *pixel* difference between
//     two markers of the same job colour on screen at the same time.
//
// Prerequisite: the wasm build must already exist at www/pkg/ (see
// scripts/build-wasm.sh).
//
// Run with:
//   NODE_PATH=/usr/local/lib/node_modules node tests/e2e/terrarium_routing.test.mjs

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

// One pixel of one world cell, out of the live canvas. `at` is [dx, dy]
// within the cell. The order marker is a 2px outline on the border, and it
// is drawn *dashed* when nobody can reach the cell: the dash period is two
// thickness units, so [0, 0] is painted either way and [2, 0] is painted
// only on a solid marker. GROUND is the one corner no overlay reaches.
const GROUND = [9, 9];
const MARKER = [0, 0];
const GAP = [2, 0];
const cellPixel = (page, i, j, at) =>
  page.evaluate(
    ([i, j, at]) => {
      const canvas = document.getElementById('canvas');
      const px = window.__cellPx;
      const rows = canvas.height / px;
      const ctx = canvas.getContext('2d');
      const d = ctx.getImageData(i * px + at[0], (rows - 1 - j) * px + at[1], 1, 1).data;
      return { r: d[0], g: d[1], b: d[2], mean: (d[0] + d[1] + d[2]) / 3 };
    },
    [i, j, at]
  );

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

async function clickCell(page, i, j) {
  const point = await cellPoint(page, i, j);
  await page.mouse.click(point.x, point.y);
}

const pickTool = (page, tool) => page.click(`button[data-tool="${tool}"]`);
const report = (page) => page.evaluate(() => window.__lastReport);

// The amber of a dig marker, as painted: much redder than blue.
const isMarker = (px) => px.r > px.b + 60;

async function main() {
  const wasmPath = path.join(wwwDir, 'pkg', 'viewer_bg.wasm');
  try {
    await stat(wasmPath);
  } catch {
    fail(`FAIL terrarium_routing: build artifacts missing at ${wasmPath}. Run scripts/build-wasm.sh first.`);
    return;
  }

  const { chromium } = require('playwright');
  const server = serveDir(wwwDir);
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const port = server.address().port;

  const browser = await chromium.launch();
  try {
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
        `FAIL terrarium_routing: page did not report ready ` +
          `(__terrariumError=${await page.evaluate(() => window.__terrariumError)}, ` +
          `pageerror=${JSON.stringify(pageErrors)}).`
      );
      return;
    }
    await page.waitForFunction(() => window.__lastReport && window.__lastReport.step > 20, {
      timeout: 10000,
    });

    // The deepest bush in the garden, at the far west end of the jar, with
    // a bush beside it to compare against. The gnomes live on the walkway
    // in the middle, so getting here is a walk and a climb.
    const BUSH = [2, 4];
    const NEIGHBOUR = [4, 4];
    // Inside the jar's own wall: visible, and no gnome can ever stand
    // within reach of it.
    const SEALED = [1, 12];

    await page.click('#pause');
    await page.evaluate(() => window.__repaint());
    const bush = await page.evaluate(() => window.__cell(2, 4));
    if (bush.material !== 'juniper') {
      fail(`FAIL terrarium_routing: (2, 4) is ${bush.material}, not the garden.`);
      return;
    }
    const bushBefore = await cellPixel(page, ...BUSH, GROUND);

    // --- Designate the sealed one first, and let the colony look at it ---
    //
    // Order: the sealed cell goes first and the jar runs one survey, so the
    // colony has already said it cannot get there; then the bush is marked
    // with the jar *paused*, so it is still standing when both markers are
    // read out of the same frame. Marking both and then letting it run does
    // not work — somebody digs the bush within a second or two, which is
    // the whole point of the rest of this test.
    await pickTool(page, 'dig');
    await clickCell(page, ...SEALED);
    const sealedOrder = await page.evaluate(() => window.__cell(1, 12));
    if (sealedOrder.order !== 'dig') {
      fail(
        `FAIL terrarium_routing: the sealed click landed on ` +
          `(${sealedOrder.i}, ${sealedOrder.j}) — ${sealedOrder.order}.`
      );
    }
    await page.click('#pause');
    await page
      .waitForFunction(() => window.__lastReport && window.__lastReport.orders.unreachable >= 1, {
        timeout: 20000,
      })
      .catch(() => {});
    await page.click('#pause');
    const surveyed = await report(page);
    if (surveyed.orders.unreachable < 1) {
      fail(
        `FAIL terrarium_routing: the colony never reported the sealed cell out of reach ` +
          `(${JSON.stringify(surveyed.orders)}).`
      );
    }

    // --- Now mark the far bush, paused, and read both markers at once ---
    await clickCell(page, ...BUSH);
    const bushOrder = await page.evaluate(() => window.__cell(2, 4));
    if (bushOrder.order !== 'dig') {
      fail(
        `FAIL terrarium_routing: the garden click landed on ` +
          `(${bushOrder.i}, ${bushOrder.j}) — ${bushOrder.order}.`
      );
    }
    await page.evaluate(() => window.__repaint());
    const solid = {
      on: await cellPixel(page, ...BUSH, MARKER),
      gap: await cellPixel(page, ...BUSH, GAP),
    };
    const dashed = {
      on: await cellPixel(page, ...SEALED, MARKER),
      gap: await cellPixel(page, ...SEALED, GAP),
    };
    if (!isMarker(solid.on) || !isMarker(solid.gap)) {
      fail(
        `FAIL terrarium_routing: the reachable marker is not solid amber ` +
          `(${JSON.stringify(solid)}).`
      );
    }
    if (isMarker(dashed.gap)) {
      fail(
        `FAIL terrarium_routing: the unreachable marker is not dashed — its gap reads ` +
          `${JSON.stringify(dashed.gap)}.`
      );
    }
    if (!(dashed.on.r > dashed.on.b && dashed.on.mean < 0.6 * solid.on.mean)) {
      fail(
        `FAIL terrarium_routing: the unreachable marker is not a dimmed marker ` +
          `(${JSON.stringify(dashed.on)} against ${JSON.stringify(solid.on)}).`
      );
    }
    const stuckText = await page.textContent('#ostuck');
    if (!/[1-9]/.test(stuckText)) {
      fail(`FAIL terrarium_routing: the page says "${stuckText}" orders are out of reach.`);
    }

    // --- And watch somebody walk the length of the jar to the other one ---
    await page.click('#pause');
    await page
      .waitForFunction(() => window.__lastReport && window.__lastReport.orders.completed >= 1, {
        timeout: 60000,
      })
      .catch(() => {});
    const done = await report(page);
    if (done.orders.completed < 1) {
      fail(
        `FAIL terrarium_routing: nobody reached the far end of the jar in 60 s ` +
          `(${JSON.stringify(done.orders)}).`
      );
    }
    if (!(done.colony.carried_g > 0)) {
      fail(`FAIL terrarium_routing: the bush vanished — carried_g is ${done.colony.carried_g}.`);
    }
    if (!(Math.abs(done.residual_mass_relative) < 1e-6)) {
      fail(`FAIL terrarium_routing: mass residual ${done.residual_mass_relative}.`);
    }
    // The cut bush against a bush still standing, in the same frame.
    const bushAfter = await cellPixel(page, ...BUSH, GROUND);
    const neighbour = await cellPixel(page, ...NEIGHBOUR, GROUND);
    if (!(bushAfter.g < neighbour.g - 10 || bushAfter.mean < 0.7 * neighbour.mean)) {
      fail(
        `FAIL terrarium_routing: the dug cell still looks like a bush ` +
          `(${JSON.stringify(bushAfter)} against ${JSON.stringify(neighbour)}).`
      );
    }
    if (pageErrors.length > 0) {
      fail(`FAIL terrarium_routing: page errors ${JSON.stringify(pageErrors)}.`);
    }

    if (!failed) {
      console.log(
        `PASS terrarium_routing: dug (2, 4) at the far end by step ${done.step} ` +
          `(bush ${bushBefore.mean.toFixed(0)} -> ${bushAfter.mean.toFixed(0)} against ` +
          `${neighbour.mean.toFixed(0)} beside it, ${done.colony.carried_g.toFixed(2)} g in hand); ` +
          `the sealed order reads unreachable on the page and draws dashed ` +
          `(${dashed.on.mean.toFixed(0)}/${dashed.gap.mean.toFixed(0)} against ` +
          `${solid.on.mean.toFixed(0)}/${solid.gap.mean.toFixed(0)} solid).`
      );
    }
  } finally {
    await browser.close();
    server.close();
  }
}

main()
  .catch((e) => fail(`FAIL terrarium_routing: ${e && e.stack ? e.stack : e}`))
  .finally(() => process.exit(failed ? 1 : 0));
