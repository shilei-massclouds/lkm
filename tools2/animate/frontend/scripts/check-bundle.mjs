import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const temporary = mkdtempSync(join(tmpdir(), 'lkm-signal-player-'));
try {
  execFileSync(process.execPath, ['node_modules/vite/bin/vite.js', 'build'], {
    cwd: new URL('..', import.meta.url),
    env: { ...process.env, LKM_ANIMATE_OUT_DIR: temporary },
    stdio: 'inherit'
  });
  for (const file of ['player.js', 'player.css']) {
    const expected = readFileSync(new URL(`../dist/${file}`, import.meta.url));
    const actual = readFileSync(join(temporary, file));
    if (!expected.equals(actual)) throw new Error(`frontend bundle is stale: ${file}`);
  }
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
