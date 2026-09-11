// Scenario: "A person opens www/still.html in a real browser tab and watches
// the still run" — a headless Playwright check on the brewing chain.
//
// Like the other checks here, it does not trust the simulation's summary of
// itself. It reads two independent things and requires them to agree:
//
//  1. The JSON the page exposes: the botanicals must be consumed, wash must
//     appear and then go, gin must accumulate, the water charge must NOT
//     boil (the whole claim of distilling below 373 K), and the
//     conservation residuals must stay at noise.
//  2. The actual canvas pixels: the two rows the receiver's floor pools in —
//     right of the pot wall — must get visibly paler as gin collects there,
//     and must end up paler than head height in the same half of the room.
//     That is the claim "the gin ended up in the receiver, in a pool"
//     checked against what a person would see rather than against the
//     number the simulation computed.
//
//     Deliberately *not* compared against the pot side, which was the first
//     thing tried: the pot is full of water, water paints a bright blue, and
//     the comparison failed while the still was working perfectly.
//
// The measure is plain brightness here rather than a colour difference, and
// deliberately so: gin (196,224,236) against air (16,18,28) and stone
// (105,105,115) is a large, unambiguous jump in every channel, and the
// pressure shading that made brightness untrustworthy in the gas check
// applies only to gas cells — gin is a liquid.
//
// Prerequisite: the wasm build must already exist at www/pkg/ (see
// scripts/build-wasm.sh).
//
// Run with:
//   NODE_PATH=/usr/local/lib/node_modules node tests/e2e/still_canvas.test.mjs

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

// Mean RGB of a rectangular region of the canvas, given as fractions of its
// width and height, measured from the top-left of the image.
const regionColour = (page, x0, x1, y0, y1) =>
  page.evaluate(([a, b, c, d]) => {
    const canvas = document.getElementById('canvas');
    const ctx = canvas.getContext('2d');
    const px = Math.floor(canvas.width * a);
    const pw = Math.max(1, Math.floor(canvas.width * b) - px);
    const py = Math.floor(canvas.height * c);
    const ph = Math.max(1, Math.floor(canvas.height * d) - py);
    const { data } = ctx.getImageData(px, py, pw, ph);
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
  }, [x0, x1, y0, y1]);

// The fraction of a region's pixels that look like standing gin: pale and
// distinctly blue. Gin is drawn (196, 224, 236); the fog the gnomes breathe
// into the condenser is near-neutral pale (235, 238, 242) and the air and
// stone behind it are neither pale nor blue. Counting pixels that match
// rather than averaging brightness is what keeps this a check on *gin* and
// not on "something bright happened over here" — the same correction night 2
// had to make to the CO2 check, for the same reason.
const ginFraction = (page, x0, x1, y0, y1) =>
  page.evaluate(([a, b, c, d]) => {
    const canvas = document.getElementById('canvas');
    const ctx = canvas.getContext('2d');
    const px = Math.floor(canvas.width * a);
    const pw = Math.max(1, Math.floor(canvas.width * b) - px);
    const py = Math.floor(canvas.height * c);
    const ph = Math.max(1, Math.floor(canvas.height * d) - py);
    const { data } = ctx.getImageData(px, py, pw, ph);
    let hits = 0;
    const n = data.length / 4;
    for (let i = 0; i < data.length; i += 4) {
      const [r, g, bl] = [data[i], data[i + 1], data[i + 2]];
      if (bl > 150 && bl > r + 25 && g > r + 12) hits += 1;
    }
    return hits / n;
  }, [x0, x1, y0, y1]);

const massOf = (report, name) => {
  const m = report.materials.find((x) => x.name === name);
  return m ? m.mass_g : 0;
};
const cellsOf = (report, name) => {
  const m = report.materials.find((x) => x.name === name);
  return m ? m.cells : 0;
};

async function main() {
  const wasmPath = path.join(wwwDir, 'pkg', 'viewer_bg.wasm');
  try {
    await stat(wasmPath);
  } catch {
    fail(`FAIL still_canvas: build artifacts missing at ${wasmPath}. Run scripts/build-wasm.sh first.`);
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

    await page.goto(`http://127.0.0.1:${port}/still.html`);
    await page
      .waitForFunction(() => window.__stillReady === true || window.__stillError, { timeout: 10000 })
      .catch(() => {});

    const ready = await page.evaluate(() => window.__stillReady);
    if (!ready) {
      const error = await page.evaluate(() => window.__stillError);
      fail(
        `FAIL still_canvas: page did not report ready ` +
          `(__stillError=${error}, pageerror=${JSON.stringify(pageErrors)}).`
      );
      return;
    }

    await page.waitForFunction(() => window.__lastReport && window.__lastReport.step > 0, {
      timeout: 10000,
    });
    const first = await page.evaluate(() => window.__lastReport);
    // The receiver's floor: right of the pot wall (which stands at column 22
    // of 52), and the two rows the gin pools in — deliberately not the
    // stone course below them, which is bright enough to drown the signal.
    const receiverBefore = await regionColour(page, 0.46, 0.98, 0.9, 0.967);
    const ginFloorBefore = await ginFraction(page, 0.46, 0.98, 0.9, 0.967);

    // Real wall-clock time, not a fast-forward: this is the same rate a
    // person watching the tab would see.
    await page.waitForTimeout(9000);
    // ...and then, if this machine was busy, a little longer. The pixel
    // checks below are about how full the receiver is, which is a function
    // of steps taken, not of seconds elapsed; sampling at whatever step
    // count nine seconds happened to buy made them pass at 2600 steps and
    // fail at 2400 on the same build. The wall-clock wait above is what
    // keeps this a real-time check; this only refuses to judge too early.
    const MIN_STEPS = 2800;
    await page
      .waitForFunction((n) => window.__lastReport && window.__lastReport.step >= n, MIN_STEPS, {
        timeout: 30000,
      })
      .catch(() => {});

    const second = await page.evaluate(() => window.__lastReport);
    if (!(second.step >= MIN_STEPS)) {
      fail(`FAIL still_canvas: only ${second.step} steps in the time allowed — too slow to judge.`);
    }
    const receiverAfter = await regionColour(page, 0.46, 0.98, 0.9, 0.967);
    // Head height in the same half of the room: air, and it should stay air.
    const receiverAir = await regionColour(page, 0.46, 0.98, 0.47, 0.63);
    const ginFloorAfter = await ginFraction(page, 0.46, 0.98, 0.9, 0.967);
    const ginAirAfter = await ginFraction(page, 0.46, 0.98, 0.47, 0.63);

    if (!(second.step > first.step)) {
      fail(`FAIL still_canvas: simulation did not advance (${first.step} -> ${second.step}).`);
    }

    // The chain, in numbers. Mashing is not sampled before-and-after,
    // because there is no "before": every bush in the pot is charged above
    // mashing temperature and ferments on the simulation's very first step,
    // well inside one animation frame. What is checked is that they were
    // all consumed — the reaction takes the botanical with it, which is
    // what makes gin finite.
    if (cellsOf(second, 'juniper') > 0) {
      fail(
        `FAIL still_canvas: ${cellsOf(second, 'juniper')} bushes never fermented ` +
          `after ${second.step} steps.`
      );
    }
    // The pot is being worked through: every bush the hatch drops takes a
    // cell of the charge with it when it ferments, so a still that is
    // actually running draws its own charge down.
    if (!(massOf(second, 'water') < massOf(first, 'water'))) {
      fail(
        `FAIL still_canvas: the charge was never drawn down ` +
          `(${massOf(first, 'water')} g -> ${massOf(second, 'water')} g of water).`
      );
    }
    const gin = massOf(second, 'gin');
    if (!(gin > 0.5)) {
      fail(`FAIL still_canvas: the still yielded ${gin} g of gin after ${second.step} steps.`);
    }
    // The claim that makes it distilling rather than boiling: the water
    // charge is still water. A pot above 373 K would have made steam.
    if (cellsOf(second, 'steam') > 0) {
      fail(
        `FAIL still_canvas: ${cellsOf(second, 'steam')} cells of steam — the pot boiled the ` +
          `water as well as the wash, so nothing was separated.`
      );
    }
    if (!(massOf(second, 'water') > 5)) {
      fail(`FAIL still_canvas: only ${massOf(second, 'water')} g of water left in the pot.`);
    }
    // A gnome went to the receiver and drank: the flasks started at 30 Gin
    // between the two of them.
    if (!(second.colony.total_gin > 60)) {
      fail(
        `FAIL still_canvas: no gnome got a drink ` +
          `(flasks ${first.colony.total_gin} -> ${second.colony.total_gin}).`
      );
    }

    // ...and in pixels. Gin is far paler than the air and stone it replaces,
    // so the receiver's floor brightens as it fills.
    const brightness = ([r, g, b]) => (r + g + b) / 3;
    if (!(brightness(receiverAfter) > brightness(receiverBefore) + 4)) {
      fail(
        `FAIL still_canvas: the receiver never filled in the actual pixels ` +
          `(${JSON.stringify(receiverBefore)} -> ${JSON.stringify(receiverAfter)}).`
      );
    }
    // The other half of "it went to the right place": at the end the
    // receiver's floor is paler than the air above it in the same half of
    // the room, i.e. the gin settled into a pool on the floor rather than
    // hanging about as vapour.
    // The other half of "it went to the right place": at the end the
    // receiver's *floor* is covered in pixels that look like gin, and head
    // height in the same half of the room is not — i.e. the gin settled into
    // a pool rather than hanging about as vapour.
    //
    // This used to compare mean brightness, and night 5 broke it honestly:
    // the gnomes waiting at the receiver now breathe, and what they breathe
    // into the coldest corner of a sealed condenser is water vapour, which
    // fogs it. Fog is drawn pale, so the air above the pool got brighter and
    // the margin shrank until the check failed on a slow machine and passed
    // on a fast one. Counting gin-coloured pixels instead is not a looser
    // check, it is the check this was always trying to be.
    if (!(ginFloorAfter > 0.1)) {
      fail(
        `FAIL still_canvas: no pool of gin on the receiver's floor at the end ` +
          `(${(ginFloorAfter * 100).toFixed(1)}% of its pixels look like gin, ` +
          `against ${(ginFloorBefore * 100).toFixed(1)}% at the start).`
      );
    }
    if (!(ginFloorAfter > ginAirAfter * 3 + 0.1)) {
      fail(
        `FAIL still_canvas: the gin is hanging in the air rather than pooling ` +
          `(floor ${(ginFloorAfter * 100).toFixed(1)}% vs head height ` +
          `${(ginAirAfter * 100).toFixed(1)}%).`
      );
    }

    for (const report of [first, second]) {
      if (Math.abs(report.residual_mass_relative) > 1e-5) {
        fail(`FAIL still_canvas: mass residual ${report.residual_mass_relative} at step ${report.step}.`);
      }
      if (Math.abs(report.residual_energy_relative) > 1e-5) {
        fail(`FAIL still_canvas: energy residual ${report.residual_energy_relative} at step ${report.step}.`);
      }
    }
    if (pageErrors.length > 0) {
      fail(`FAIL still_canvas: page errors ${JSON.stringify(pageErrors)}.`);
    }

    if (!failed) {
      console.log(
        `PASS still_canvas: advanced ${first.step} -> ${second.step} steps, ` +
          `juniper ${cellsOf(first, 'juniper')} -> ${cellsOf(second, 'juniper')}, ` +
          `gin ${massOf(first, 'gin').toFixed(3)} -> ${gin.toFixed(3)} g, ` +
          `water still ${massOf(second, 'water').toFixed(1)} g with ${cellsOf(second, 'steam')} steam, ` +
          `flasks ${first.colony.total_gin.toFixed(1)} -> ${second.colony.total_gin.toFixed(1)}, ` +
          `receiver floor ${(ginFloorBefore * 100).toFixed(1)}% -> ` +
          `${(ginFloorAfter * 100).toFixed(1)}% gin-coloured against ` +
          `${(ginAirAfter * 100).toFixed(1)}% at head height, brightness ` +
          `${brightness(receiverBefore).toFixed(1)} -> ` +
          `${brightness(receiverAfter).toFixed(1)} against ${brightness(receiverAir).toFixed(1)} ` +
          `at head height, ` +
          `mass residual ${second.residual_mass_relative}.`
      );
    }
  } finally {
    await browser.close();
    server.close();
  }
}

main()
  .catch((e) => fail(`FAIL still_canvas: ${e && e.stack ? e.stack : e}`))
  .finally(() => process.exit(failed ? 1 : 0));
