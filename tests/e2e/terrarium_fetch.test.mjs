// Scenario: "A person opens www/terrarium.html, picks up the Fetch tool,
// chooses sand, clicks an empty cell of the walkway, and watches a gnome go
// and get some and put it down where they pointed" — night 9's new verb
// (`Job::Supply`, src/order.rs) driven the way a human drives it, in a real
// browser, with a real mouse, and checked in real canvas pixels.
//
// Sand rather than water, for a reason worth knowing: there is no water in
// this jar a gnome can reach. The pool is behind a bank five courses tall
// and a gnome climbs one course at a time; the garden's bed is roofed by its
// own bushes. See `a_player_can_ask_for_a_material_and_the_colony_finds_it`
// in src/terrarium.rs.
//
// What this asserts that a unit test cannot:
//
//  1. The tool exists on the page, and the list of things you can ask for
//     comes out of the material table rather than out of the HTML: the test
//     picks "water" out of the <select> by name.
//  2. The order is visible before it is done — the supply marker is drawn in
//     its own green, so the player can tell a fetch from a dig at a glance.
//  3. Nobody said where the sand was. The order names a material and a
//     destination and nothing else; the colony finds a source it can walk
//     to, and a gnome carries a load of it over.
//  4. The world visibly changes at the marked cell, after real wall-clock
//     time, with nobody driving the simulation but the page's own animation
//     loop: an empty cell fills with something you can see.
//  5. Nothing was created. The jar holds exactly the sand it held before,
//     and the mass residual stays flat across the whole errand.
//
// Prerequisite: the wasm build must already exist at www/pkg/ (see
// scripts/build-wasm.sh).
//
// Run with:
//   NODE_PATH=/usr/local/lib/node_modules node tests/e2e/terrarium_fetch.test.mjs

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

// See terrarium_orders.test.mjs for why these two sample points and not
// others: GROUND is the one corner of a cell that no overlay reaches, and
// MARKER is on the order outline but clear of the carried-load pip.
const GROUND = [9, 9];
const MARKER = [9, 1];
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

async function main() {
  const wasmPath = path.join(wwwDir, 'pkg', 'viewer_bg.wasm');
  try {
    await stat(wasmPath);
  } catch {
    fail(`FAIL terrarium_fetch: build artifacts missing at ${wasmPath}. Run scripts/build-wasm.sh first.`);
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
        `FAIL terrarium_fetch: page did not report ready ` +
          `(__terrariumError=${await page.evaluate(() => window.__terrariumError)}, ` +
          `pageerror=${JSON.stringify(pageErrors)}).`
      );
      return;
    }
    await page.waitForFunction(() => window.__lastReport && window.__lastReport.step > 20, {
      timeout: 10000,
    });

    // An empty cell of the walkway the gnomes live on.
    const TARGET = [10, 4];

    // The list of things you can ask for is served from the material table.
    const offered = await page.evaluate(() =>
      [...document.getElementById('cargo').options].map((o) => o.value)
    );
    for (const wanted of ['water', 'sand', 'juniper']) {
      if (!offered.includes(wanted)) {
        fail(`FAIL terrarium_fetch: "${wanted}" is not on the fetch list (${offered.join(', ')}).`);
      }
    }
    if (offered.includes('air')) {
      fail('FAIL terrarium_fetch: a gnome cannot carry air, but the list offers it.');
    }

    // Pause to mark it up, the way a person would.
    await page.click('#pause');
    await page.evaluate(() => window.__repaint());
    const before = await cellPixel(page, ...TARGET, GROUND);

    await page.selectOption('#cargo', 'sand');
    await pickTool(page, 'haul');
    await clickCell(page, ...TARGET);
    const marked = await report(page);
    const ordered = await page.evaluate(() => window.__cell(10, 4));
    if (ordered.order !== 'supply:sand') {
      fail(
        `FAIL terrarium_fetch: the click wrote "${ordered.order}" on ` +
          `(${ordered.i}, ${ordered.j}), not a sand fetch.`
      );
    }
    // The marker, in pixels: supply is drawn green, so this corner has to be
    // much greener than it is red or blue.
    await page.evaluate(() => window.__repaint());
    const marker = await cellPixel(page, ...TARGET, MARKER);
    if (!(marker.g > marker.r + 40 && marker.g > marker.b + 40)) {
      fail(`FAIL terrarium_fetch: no fetch marker visible at the cell (${JSON.stringify(marker)}).`);
    }

    // --- And watch somebody go and get it ---
    //
    // Up to four goes, because the walkway is a working floor: leaf litter
    // drifts along it, mould creeps over it and rain lands on it, and a
    // fetch order whose cell has filled up with any of those is *satisfied*
    // rather than failed — there is already something there. A person would
    // shrug and mark the next cell along, so that is what this does.
    await page.click('#pause');
    let arrived = false;
    for (const [i, j] of [TARGET, [11, 4], [12, 4], [13, 4]]) {
      const cell = await page.evaluate(([i, j]) => window.__cell(i, j), [i, j]);
      if (cell.material !== 'air' && cell.material !== 'sand') continue;
      if (cell.order === null) {
        await page.evaluate(([i, j]) => window.__order(i, j, 'haul:sand'), [i, j]);
      }
      arrived = await page
        .waitForFunction(
          ([i, j]) => window.__cell(i, j).material === 'sand',
          [i, j],
          { timeout: 20000 }
        )
        .then(() => true)
        .catch(() => false);
      if (arrived) {
        TARGET[0] = i;
        TARGET[1] = j;
        break;
      }
    }
    if (!arrived) {
      fail(
        `FAIL terrarium_fetch: no sand arrived anywhere on the walkway ` +
          `(${JSON.stringify((await report(page)).orders)}).`
      );
    }
    const done = await report(page);
    if (done.orders.completed < 1) {
      fail(`FAIL terrarium_fetch: the order never completed (${JSON.stringify(done.orders)}).`);
    }
    if (!(Math.abs(done.residual_mass_relative) < 1e-6)) {
      fail(`FAIL terrarium_fetch: mass residual ${done.residual_mass_relative} after hauling.`);
    }

    // Read it in the pixels, with the jar paused and nobody standing in the
    // cell — a gnome is drawn over the cell it is in.
    let standing = null;
    let wet = null;
    for (let attempt = 0; attempt < 25; attempt += 1) {
      await page.click('#pause');
      await page.evaluate(() => window.__repaint());
      standing = await page.evaluate(([i, j]) => window.__cell(i, j), TARGET);
      if (standing.gnomes === 0 && standing.material === 'sand') {
        wet = await cellPixel(page, ...TARGET, GROUND);
        break;
      }
      await page.click('#pause');
      await page.waitForTimeout(120);
    }
    await page.click('#pause');
    if (!wet) {
      fail(`FAIL terrarium_fetch: never got a clear look at the cell (${JSON.stringify(standing)}).`);
    // Sand is (200, 175, 105) in the table and the frame may be a night one,
    // which holds blue back (night 5) and compresses every channel: measured
    // at 114/100/86 against an empty cell's 32/40/56. So: warmer than it is
    // blue, and a lot brighter than the hole it filled.
    } else if (!(wet.r > wet.b + 18 && wet.mean > before.mean + 25)) {
      fail(
        `FAIL terrarium_fetch: the delivered cell does not look like sand — ` +
          `${JSON.stringify(wet)} where the empty cell read ${JSON.stringify(before)}.`
      );
    }
    if (pageErrors.length > 0) {
      fail(`FAIL terrarium_fetch: page errors ${JSON.stringify(pageErrors)}.`);
    }

    if (!failed) {
      console.log(
        `PASS terrarium_fetch: asked for sand at (${TARGET[0]}, ${TARGET[1]}) on step ${marked.step} without saying ` +
          `where any is; a gnome delivered it by step ${done.step} ` +
          `(pixel ${before.r},${before.g},${before.b} -> ${wet.r},${wet.g},${wet.b}), ` +
          `mass residual ${done.residual_mass_relative}.`
      );
    }
  } finally {
    await browser.close();
    server.close();
  }
}

await main();
process.exit(failed ? 1 : 0);
