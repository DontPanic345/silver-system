// Headless DOM check of the scenario player page — the browser half of AC 12–16
// (`npm test` stays browser-free, AC 35). Serves the repo root, loads index.html
// in headless Chromium, and asserts against the real DOM. Exits non-zero on any
// failure. Also drops a screenshot of #sim at scratch/scenarios.png.
//
//   node scripts/shot-scenarios.js   (npm run shot:scenarios)

import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, resolve } from 'node:path';
import { createRequire } from 'node:module';

const { chromium } = createRequire(import.meta.url)('playwright');

const ROOT = resolve(import.meta.dirname, '..');
const OUT = resolve(ROOT, 'scratch/scenarios.png');
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.png': 'image/png' };

const server = http.createServer(async (req, res) => {
  try {
    const path = join(ROOT, decodeURIComponent(req.url.split('?')[0]));
    const body = await readFile(path === join(ROOT, '/') ? join(ROOT, 'index.html') : path);
    res.writeHead(200, { 'content-type': MIME[extname(path)] || 'application/octet-stream' });
    res.end(body);
  } catch {
    res.writeHead(404).end('not found');
  }
});

await new Promise((r) => server.listen(0, r));
const port = server.address().port;

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 900, height: 1000 } });

let failed = 0;
const check = (name, ok, detail = '') => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failed++;
};

try {
  await page.goto(`http://localhost:${port}/index.html`);
  await page.waitForSelector('#sim');
  await page.waitForFunction(() => window.controller && window.controller.sim);

  // --- AC 12: scenarios listed by name + description --------------------
  const optionText = await page.$$eval('#scenario-select option', (os) => os.map((o) => o.textContent));
  check('scenario <select> renders at least one option', optionText.length >= 1, `${optionText.length} options`);

  const shared = await page.evaluate(async () => {
    const m = await import('/js/scenarios.js');
    return m.scenarios.map((s) => ({ id: s.id, name: s.name, description: s.description }));
  });
  const namesShown = shared.every((s) =>
    optionText.some((t) => t.includes(s.name) && t.includes(s.description)));
  check('every shared scenario name + description appears in an option', namesShown,
    optionText.join(' | '));

  // --- AC 13: select, play, reset -------------------------------------
  await page.selectOption('#scenario-select', 'warm-plume-rises');
  const selected = await page.$eval('#scenario-select', (el) => el.value);
  check('warm-plume-rises can be selected', selected === 'warm-plume-rises', selected);

  const step0 = await page.evaluate(() => window.controller.stepCount);
  const energy0 = await page.evaluate(() => window.controller.readouts().find((r) => /energ/i.test(r.key) || /energ/i.test(r.label)).value);
  check('step count starts at 0', step0 === 0, String(step0));

  await page.click('#play');
  await page.waitForFunction(() => window.controller.stepCount >= 3, null, { timeout: 5000 });
  const stepN = await page.evaluate(() => window.controller.stepCount);
  check('clicking Play advances the step count', stepN >= 3, String(stepN));

  await page.click('#play'); // pause
  await page.click('#reset');
  const stepR = await page.evaluate(() => window.controller.stepCount);
  const energyR = await page.evaluate(() => window.controller.readouts().find((r) => /energ/i.test(r.key) || /energ/i.test(r.label)).value);
  check('Reset returns the step count to 0', stepR === 0, String(stepR));
  check('Reset returns the energy readout to its initial value',
    Math.abs(energyR - energy0) <= 1e-6 * (Math.abs(energy0) + 1), `${energy0} -> ${energyR}`);

  // --- AC 14: readouts panel shows a total-energy row -----------------
  const readoutText = await page.$eval('#readouts', (el) => el.textContent);
  check('readouts panel shows a total-energy row', /energ/i.test(readoutText), readoutText);

  await page.locator('#sim').screenshot({ path: OUT });

  const stats = await page.evaluate(() => ({
    fps: document.getElementById('fps').textContent,
    step: document.getElementById('step-ms').textContent,
  }));
  console.log(`wrote ${OUT}  (${stats.fps} fps, ${stats.step} ms/step)`);
} finally {
  await browser.close();
  server.close();
}

console.log(failed === 0 ? '\nALL CHECKS PASSED' : `\n${failed} CHECK(S) FAILED`);
process.exit(failed === 0 ? 0 : 1);
