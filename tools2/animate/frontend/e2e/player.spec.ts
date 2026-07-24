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
  const firstArrow = page.locator('.signal-overlay');
  await expect(firstArrow).toHaveAttribute('data-arrow-kind', 'ordinary');
  await page.waitForTimeout(250);
  const alignment = await page.locator('.stage').evaluate((stage) => {
    const source = stage.querySelector<HTMLElement>('[data-node-id="Human"]');
    const target = stage.querySelector<HTMLElement>('[data-node-id="Root"]');
    const path = stage.querySelector<SVGPathElement>('.signal-path');
    if (!source || !target || !path) return null;
    const stageRect = stage.getBoundingClientRect();
    const sourceRect = source.getBoundingClientRect();
    const targetRect = target.getBoundingClientRect();
    const numbers = Array.from(path.getAttribute('d')?.matchAll(/-?\d+(?:\.\d+)?/g) || [],
      (match) => Number(match[0]));
    return {
      sourceBeforeTarget: sourceRect.right < targetRect.left,
      startError: Math.abs(numbers[0] - (sourceRect.right - stageRect.left + stage.scrollLeft)),
      endError: Math.abs(numbers[6] - (targetRect.left - stageRect.left + stage.scrollLeft))
    };
  });
  expect(alignment).not.toBeNull();
  expect(alignment?.sourceBeforeTarget).toBe(true);
  expect(alignment?.startError).toBeLessThan(2);
  expect(alignment?.endError).toBeLessThan(2);
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

test('desktop viewports devote the page to the stage without outer scrolling', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const viewport of [
    { width: 1024, height: 768 },
    { width: 1280, height: 900 },
    { width: 1920, height: 1080 }
  ]) {
    await page.setViewportSize(viewport);
    await openOffline(page, pipelineHtml);
    const layout = await page.evaluate(() => ({
      documentHeight: document.documentElement.scrollHeight,
      viewportHeight: document.documentElement.clientHeight,
      stageHeight: document.querySelector('.stage')?.getBoundingClientRect().height || 0,
      headerHeight: document.querySelector('.trace-header')?.getBoundingClientRect().height || 0,
      transportHeight: document.querySelector('.transport')?.getBoundingClientRect().height || 0
    }));
    expect(layout.documentHeight).toBeLessThanOrEqual(layout.viewportHeight);
    expect(layout.stageHeight / viewport.height).toBeGreaterThanOrEqual(0.72);
    expect(layout.headerHeight).toBeLessThan(viewport.height * 0.12);
    expect(layout.transportHeight).toBeLessThan(viewport.height * 0.12);
  }
});

test('mobile layout wraps without page-level horizontal overflow', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await openOffline(page, pipelineHtml);
  for (let index = 0; index < 4; index += 1) await page.keyboard.press('ArrowRight');
  const layout = await page.evaluate(() => {
    const stage = document.querySelector<HTMLElement>('.stage');
    const identities = Array.from(document.querySelectorAll<HTMLElement>('.node-identity'));
    const overlapping = identities.some((identity) => {
      const name = identity.querySelector('strong')?.getBoundingClientRect();
      const state = identity.querySelector('.node-state')?.getBoundingClientRect();
      if (!name || !state) return false;
      return name.left < state.right && name.right > state.left &&
        name.top < state.bottom && name.bottom > state.top;
    });
    return {
      pageOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      stageOverflow: (stage?.scrollWidth || 0) - (stage?.clientWidth || 0),
      overlapping
    };
  });
  expect(layout.pageOverflow).toBeLessThanOrEqual(1);
  expect(layout.stageOverflow).toBeGreaterThan(0);
  expect(layout.overlapping).toBe(false);
});

test('desktop and mobile players have stable visual baselines', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await openOffline(page, pipelineHtml);
  await expect(page.locator('.stage')).toHaveScreenshot('initial-stage.png', {
    animations: 'disabled'
  });
  await expect(page.locator('.player-shell')).toHaveScreenshot('desktop-player.png', {
    animations: 'disabled'
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await openOffline(page, pipelineHtml);
  await expect(page.locator('.player-shell')).toHaveScreenshot('mobile-player.png', {
    animations: 'disabled'
  });
});
