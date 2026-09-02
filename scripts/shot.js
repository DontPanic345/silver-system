// Render the sim headless and save a PNG — a way to eyeball the fluid without
// pulling an image into the assistant's context.
//
//   node scripts/shot.js [outfile] [--seconds N] [--strokes N]
//
// Serves the repo on an ephemeral port, loads index.html in headless Chromium,
// drags a few strokes across the canvas, lets the flow develop, then writes a
// screenshot of #sim. Prints the output path.

import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, resolve } from 'node:path';
import { createRequire } from 'node:module';

// playwright is installed globally; NODE_PATH only reaches CommonJS require,
// not ESM import, so pull it in that way.
const { chromium } = createRequire(import.meta.url)('playwright');

const ROOT = resolve(import.meta.dirname, '..');
const args = process.argv.slice(2);
const opt = (flag, def) => {
  const i = args.indexOf(flag);
  return i >= 0 ? Number(args[i + 1]) : def;
};
const outfile = resolve(args.find((a) => !a.startsWith('--') && a !== String(opt('--seconds')) && a !== String(opt('--strokes'))) || 'scratch/fluid.png');
const seconds = opt('--seconds', 4);
const strokes = opt('--strokes', 3);

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
await page.goto(`http://localhost:${port}/index.html`);
await page.waitForSelector('#sim');

const box = await page.locator('#sim').boundingBox();
const rnd = (() => { let s = 12345; return () => (s = (s * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff; })();
for (let n = 0; n < strokes; n++) {
  const y = box.y + box.height * (0.2 + 0.6 * rnd());
  await page.mouse.move(box.x + box.width * 0.1, y);
  await page.mouse.down();
  for (let t = 1; t <= 12; t++) {
    await page.mouse.move(box.x + box.width * (0.1 + 0.8 * (t / 12)), y + Math.sin(t / 2) * 30 * rnd());
    await page.waitForTimeout(16);
  }
  await page.mouse.up();
  await page.waitForTimeout(120);
}

await page.waitForTimeout(seconds * 1000);
await page.locator('#sim').screenshot({ path: outfile });

const stats = await page.evaluate(() => ({
  fps: document.getElementById('fps').textContent,
  step: document.getElementById('step').textContent,
}));

await browser.close();
server.close();
console.log(`wrote ${outfile}  (${stats.fps} fps, ${stats.step} ms/step)`);
