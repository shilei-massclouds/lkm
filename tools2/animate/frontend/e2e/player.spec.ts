import { expect, test, type Page } from '@playwright/test';
import { mkdtempSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';

const frontend = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repository = resolve(frontend, '../../..');
const pythonPath = [
  'common', 'parse', 'model', 'derive', 'check', 'view', 'render', 'animate', 'pyveri'
].map((name) => join(repository, 'tools2', name, 'src')).join(':');
let generated = '';
let pipelineHtml = '';
let effectsHtml = '';
let mainHtml = '';

function generate(name: string, spec: string, signal: string) {
  const html = join(generated, `${name}.html`);
  const work = join(generated, `${name}-work`);
  const result = spawnSync(
    'python3',
    [
      '-m', 'pyveri', spec, '--signal', signal, '--max-depth', 'all',
      '--max-breadth', 'all', '--work-dir', work, '--html-out', html,
      '-o', join(generated, `${name}.txt`)
    ],
    { cwd: repository, env: { ...process.env, PYTHONPATH: pythonPath }, encoding: 'utf8' }
  );
  if (result.status !== 0 && result.status !== 1) {
    throw new Error(`fixture generation failed (${result.status}): ${result.stderr}`);
  }
  return html;
}

async function openOffline(page: Page, html: string) {
  const requests: string[] = [];
  page.on('request', (request) => requests.push(request.url()));
  await page.goto(pathToFileURL(html).href, { waitUntil: 'load' });
  await expect(page.locator('#lkm-signal-player')).toBeVisible();
  expect(requests).toEqual([pathToFileURL(html).href]);
}

test.beforeAll(() => {
  generated = mkdtempSync(join(tmpdir(), 'lkm-signal-animation-e2e-'));
  pipelineHtml = generate(
    'pipeline', join(repository, 'tools2/tests/fixtures/pipeline.spec'), 'Root.Start'
  );
  effectsHtml = generate(
    'effects', join(repository, 'tools2/tests/fixtures/animation-effects.spec'), 'Root.Begin'
  );
  mainHtml = generate(
    'main', join(repository, 'spec/model/main.spec'), 'ComputerProject.Preset'
  );
});

test.afterAll(() => {
  if (generated) rmSync(generated, { recursive: true, force: true });
});

test('offline controls restore exact hierarchy and sibling order', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await openOffline(page, pipelineHtml);
  const counter = page.locator('.counter');
  await expect(counter).toContainText('步骤 0 / 4');
  await expect(page.locator('[data-node-id]')).toHaveCount(1);
  await expect(page.locator('[data-node-id="Human"]')).toBeVisible();

  await page.keyboard.press('ArrowRight');
  await expect(counter).toContainText('步骤 1 / 4');
  await expect(page.locator('[data-node-id="Root"]')).toHaveAttribute('data-state', 'Ready');
  await page.getByRole('button', { name: '下一步' }).click();
  await expect(counter).toContainText('步骤 2 / 4');
  await expect(page.locator('[data-children-of="Root"]')).toBeVisible();
  await page.keyboard.press('ArrowRight');
  await expect(counter).toContainText('步骤 3 / 4');
  await page.keyboard.press('ArrowRight');
  await expect(counter).toContainText('步骤 4 / 4');
  await expect(page.locator('[data-children-of="Root"] > .node-slot')).toHaveCount(3);
  await expect(page.locator('[data-children-of="Root"] > .node-slot')).toHaveText([
    /Sink/, /Async/, /Child/
  ]);

  await page.keyboard.press('ArrowLeft');
  await page.keyboard.press('ArrowLeft');
  await page.keyboard.press('ArrowLeft');
  await expect(counter).toContainText('步骤 1 / 4');
  await expect(page.locator('[data-node-id]')).toHaveCount(2);
  await expect(page.locator('[data-node-id="Root"]')).toHaveAttribute('data-state', 'Ready');
});

test('ordinary and exceptional self Signals expose their response phases', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await openOffline(page, pipelineHtml);
  const pipelineNext = page.getByRole('button', { name: '下一步' });
  await pipelineNext.click();
  await expect(page.locator('.stage')).toHaveAttribute('data-animation-phase', 'idle');
  await pipelineNext.click();
  await expect(page.locator('.signal-overlay')).toHaveAttribute('data-arrow-kind', 'ordinary');
  await expect(page.locator('[data-node-id="Root"]')).toHaveClass(/signal-source/);
  await expect(page.locator('[data-node-id="Child"]')).toHaveClass(/signal-target/);
  await expect(page.locator('.stage')).toHaveAttribute('data-animation-phase', 'idle');

  await openOffline(page, effectsHtml);
  const next = page.getByRole('button', { name: '下一步' });
  await next.click();
  await expect(page.locator('.signal-overlay')).toHaveAttribute('data-arrow-kind', 'ordinary');
  await page.waitForFunction(() =>
    document.querySelector('[data-node-id="Root"]')?.classList.contains('action-response')
  );
  await expect(page.locator('.stage')).toHaveAttribute('data-animation-phase', 'idle');

  await next.click();
  const selfArrow = page.locator('.signal-overlay');
  await expect(selfArrow).toHaveAttribute('data-arrow-kind', 'self');
  await expect(selfArrow).toHaveAttribute('data-outcome', 'rejected');
  await expect(page.locator('.reason')).toContainText('rejected · no_handler');
  await page.waitForFunction(() =>
    document.querySelector('[data-node-id="Root"]')?.classList.contains('error-response')
  );
});

test('reduced motion settles immediately and disables response movement', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await openOffline(page, effectsHtml);
  const started = Date.now();
  await page.getByRole('button', { name: '下一步' }).click();
  await expect(page.locator('.counter')).toContainText('步骤 1 / 2');
  expect(Date.now() - started).toBeLessThan(500);
  await expect(page.locator('.stage')).toHaveClass(/reduced-motion/);
  await expect(page.locator('[data-node-id="Root"]')).toHaveCSS('animation-name', 'none');
});

test('full main model loads offline, scrolls targets, and restores 277 steps', async ({ page }) => {
  test.setTimeout(60_000);
  expect(statSync(mainHtml).size).toBeLessThan(5 * 1024 * 1024);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const started = Date.now();
  await openOffline(page, mainHtml);
  expect(Date.now() - started).toBeLessThan(5_000);
  const counter = page.locator('.counter');
  await expect(counter).toContainText('步骤 0 / 277');
  let sawScroll = false;
  for (let index = 1; index <= 277; index += 1) {
    await page.keyboard.press('ArrowRight');
    await expect(counter).toContainText(`步骤 ${index} / 277`);
    if (!sawScroll) {
      sawScroll = await page.locator('.stage').evaluate(
        (stage) => stage.scrollTop > 0 || stage.scrollLeft > 0
      );
    }
  }
  expect(sawScroll).toBe(true);
  for (let index = 276; index >= 0; index -= 1) {
    await page.keyboard.press('ArrowLeft');
    await expect(counter).toContainText(`步骤 ${index} / 277`);
  }
  await expect(page.locator('[data-node-id]')).toHaveCount(1);
  await expect(page.locator('[data-node-id="Human"]')).toBeVisible();
});

test('initial offline canvas has a stable visual baseline', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await openOffline(page, pipelineHtml);
  await expect(page.locator('.stage')).toHaveScreenshot('initial-stage.png', {
    animations: 'disabled'
  });
});
