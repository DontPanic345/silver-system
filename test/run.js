// Test runner. Runs every probe file to completion and fails if any did —
// unlike `a && b`, a failure in one file does not hide the results of the next.
//
//   npm test
//
// Each file is a standalone script (`node test/<file>.js`) that prints its own
// checks and exits non-zero on failure. Add new files to the list below.

import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const files = ['fluid-probe.js', 'conservation.js', 'temperature.js', 'buoyancy.js', 'scenarios.js', 'player.js'];

let failed = 0;
for (const file of files) {
  const r = spawnSync(process.execPath, [join(here, file)], { stdio: 'inherit' });
  if (r.status !== 0) failed++;
}

console.log(failed === 0 ? '\n== ALL FILES PASSED ==' : `\n== ${failed} FILE(S) FAILED ==`);
process.exit(failed === 0 ? 0 : 1);
