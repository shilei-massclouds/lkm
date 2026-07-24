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
let alternatingHtml = '';
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
  alternatingHtml = generate(
    'alternating', join(repository, 'tools2/tests/fixtures/alternating-layout.spec'),
    'Controller.Begin'
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
    /Child/, /Async/, /Sink/
  ]);

  await page.keyboard.press('ArrowLeft');
  await page.keyboard.press('ArrowLeft');
  await page.keyboard.press('ArrowLeft');
  await expect(counter).toContainText('步骤 1 / 4');
  await expect(page.locator('[data-node-id]')).toHaveCount(2);
  await expect(page.locator('[data-node-id="Root"]')).toHaveAttribute('data-state', 'Ready');
});

test('four levels alternate from the lower-left and inflate only outward', async ({ page }) => {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await openOffline(page, alternatingHtml);
  const counter = page.locator('.counter');
  const contentRects = async (ids: string[]) => page.locator('.stage').evaluate((stage, names) => {
    const stageRect = stage.getBoundingClientRect();
    return Object.fromEntries(names.map((name) => {
      const rect = stage.querySelector<HTMLElement>(`[data-node-id="${name}"]`)?.getBoundingClientRect();
      return [name, rect ? {
        left: rect.left - stageRect.left + stage.scrollLeft,
        right: rect.right - stageRect.left + stage.scrollLeft,
        top: rect.top - stageRect.top + stage.scrollTop,
        bottom: rect.bottom - stageRect.top + stage.scrollTop
      } : null];
    }));
  }, ids);
  const advanceTo = async (index: number) => {
    while (!((await counter.textContent()) || '').includes(`步骤 ${index} / 12`)) {
      await page.keyboard.press('ArrowRight');
    }
  };

  const initial = await contentRects(['Human']);
  const initialScroll = await page.locator('.stage').evaluate((stage) => ({
    left: stage.scrollLeft,
    top: stage.scrollTop
  }));
  const stageSize = await page.locator('.stage').evaluate((stage) => ({
    width: stage.clientWidth,
    height: stage.clientHeight
  }));
  expect(initial.Human?.left).toBeLessThan(stageSize.width * 0.08);
  expect(initial.Human?.bottom).toBeGreaterThan(stageSize.height * 0.9);

  await advanceTo(3);
  const shallow = await contentRects(['Human', 'Controller', 'EarlyRoot', 'LateRoot']);
  expect(shallow.Human!.left).toBeLessThan(shallow.Controller!.left);
  expect(shallow.Controller!.left).toBeLessThan(shallow.EarlyRoot!.left);
  expect(shallow.EarlyRoot!.left).toBeLessThan(shallow.LateRoot!.left);
  expect(Math.abs(shallow.Human!.bottom - shallow.LateRoot!.bottom)).toBeLessThan(2);

  await advanceTo(5);
  const firstLevel2 = await contentRects(['Human', 'Controller', 'EarlyRoot', 'LateRoot', 'Level2First']);
  await advanceTo(6);
  const secondLevel2 = await contentRects(['Human', 'Controller', 'EarlyRoot', 'LateRoot', 'Level2First', 'Level2Second']);
  expect(Math.abs(secondLevel2.Level2First!.left - firstLevel2.Level2First!.left)).toBeLessThan(2);
  expect(Math.abs(secondLevel2.Level2First!.bottom - firstLevel2.Level2First!.bottom)).toBeLessThan(2);
  expect(secondLevel2.Level2Second!.bottom).toBeLessThan(secondLevel2.Level2First!.top);

  await advanceTo(8);
  const firstLevel3 = await contentRects(['Human', 'Controller', 'EarlyRoot', 'LateRoot', 'Level3First']);
  await advanceTo(9);
  const secondLevel3 = await contentRects(['Human', 'Controller', 'EarlyRoot', 'LateRoot', 'Level3First', 'Level3Second']);
  expect(Math.abs(secondLevel3.EarlyRoot!.left - firstLevel3.EarlyRoot!.left)).toBeLessThan(2);
  expect(Math.abs(secondLevel3.EarlyRoot!.bottom - firstLevel3.EarlyRoot!.bottom)).toBeLessThan(2);
  expect(secondLevel3.Level3Second!.left).toBeGreaterThan(secondLevel3.Level3First!.right);
  expect(secondLevel3.LateRoot!.left).toBeGreaterThan(firstLevel3.LateRoot!.left);
  expect(Math.abs(secondLevel3.Human!.left - shallow.Human!.left)).toBeLessThan(2);
  expect(Math.abs(secondLevel3.Controller!.left - shallow.Controller!.left)).toBeLessThan(2);

  await advanceTo(11);
  const firstLevel4 = await contentRects(['Level4First']);
  await advanceTo(12);
  const secondLevel4 = await contentRects(['Level4First', 'Level4Second']);
  const finalScroll = await page.locator('.stage').evaluate((stage) => ({
    left: stage.scrollLeft,
    top: stage.scrollTop
  }));
  expect(Math.abs(secondLevel4.Level4First!.left - firstLevel4.Level4First!.left)).toBeLessThan(2);
  expect(Math.abs(secondLevel4.Level4First!.bottom - firstLevel4.Level4First!.bottom)).toBeLessThan(2);
  expect(secondLevel4.Level4Second!.bottom).toBeLessThan(secondLevel4.Level4First!.top);

  const hierarchy = await page.locator('.stage').evaluate((stage) =>
    Array.from(stage.querySelectorAll<HTMLElement>('[data-layout]'))
      .map((element) => [element.dataset.level, element.dataset.layout, element.dataset.alignment])
  );
  expect(hierarchy).toEqual([
    ['1', 'row', 'bottom'], ['2', 'up', 'left'],
    ['3', 'row', 'bottom'], ['4', 'up', 'left']
  ]);

  for (let index = 11; index >= 0; index -= 1) {
    await page.keyboard.press('ArrowLeft');
    await expect(counter).toContainText(`步骤 ${index} / 12`);
  }
  const restored = await contentRects(['Human']);
  const restoredScroll = await page.locator('.stage').evaluate((stage) => ({
    left: stage.scrollLeft,
    top: stage.scrollTop
  }));
  expect(Math.abs(restored.Human!.left - initial.Human!.left)).toBeLessThan(2);
  expect(Math.abs(restored.Human!.bottom - initial.Human!.bottom)).toBeLessThan(2);
  expect(restoredScroll).toEqual(initialScroll);

  await advanceTo(12);
  const repeatedScroll = await page.locator('.stage').evaluate((stage) => ({
    left: stage.scrollLeft,
    top: stage.scrollTop
  }));
  expect(repeatedScroll).toEqual(finalScroll);
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
      path: path.getAttribute('d'),
      startError: Math.abs(numbers[0] - (sourceRect.right - stageRect.left + stage.scrollLeft)),
      endError: Math.abs(numbers[2] - (targetRect.left - stageRect.left + stage.scrollLeft))
    };
  });
  expect(alignment).not.toBeNull();
  expect(alignment?.sourceBeforeTarget).toBe(true);
  expect(alignment?.path).toMatch(/^M [^ ]+ [^ ]+ L [^ ]+ [^ ]+$/);
  expect(alignment?.startError).toBeLessThan(2);
  expect(alignment?.endError).toBeLessThan(2);
  await expect(page.locator('.stage')).toHaveAttribute('data-animation-phase', 'idle');
  await pipelineNext.click();
  await expect(page.locator('.signal-overlay')).toHaveCount(0);
  await expect(page.locator('[data-node-id="Root"]')).toHaveClass(/signal-source/);
  await expect(page.locator('[data-node-id="Child"]')).toHaveClass(/signal-target/);
  await page.waitForFunction(() =>
    document.querySelector('[data-node-id="Child"]')?.classList.contains('action-response')
  );
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
  const upperLoop = await page.locator('.stage').evaluate((stage) => {
    const node = stage.querySelector<HTMLElement>('[data-node-id="Root"]');
    const path = stage.querySelector<SVGPathElement>('.signal-path');
    const label = stage.querySelector<SVGTextElement>('.signal-overlay text');
    if (!node || !path || !label) return null;
    const stageRect = stage.getBoundingClientRect();
    const nodeRect = node.getBoundingClientRect();
    const numbers = Array.from(path.getAttribute('d')?.matchAll(/-?\d+(?:\.\d+)?/g) || [],
      (match) => Number(match[0]));
    return {
      nodeTop: nodeRect.top - stageRect.top + stage.scrollTop,
      controlTop: Math.min(numbers[3], numbers[5]),
      labelY: Number(label.getAttribute('y'))
    };
  });
  expect(upperLoop).not.toBeNull();
  expect(upperLoop!.controlTop).toBeLessThan(upperLoop!.nodeTop);
  expect(upperLoop!.labelY).toBeLessThan(upperLoop!.nodeTop);
  expect(upperLoop!.labelY).toBeGreaterThanOrEqual(0);
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
  const longIdentityTypography = await page.locator('.stage').evaluate((stage) => {
    const identities = Array.from(stage.querySelectorAll<HTMLElement>('.node-identity'));
    const longIdentities = identities.filter((identity) =>
      (identity.querySelector('strong')?.textContent?.length || 0) >= 16
    );
    return {
      count: longIdentities.length,
      sameFontSize: longIdentities.every((identity) => {
        const name = identity.querySelector<HTMLElement>('strong');
        const state = identity.querySelector<HTMLElement>('.node-state');
        return !!name && !!state && getComputedStyle(name).fontSize === getComputedStyle(state).fontSize;
      }),
      overlapping: longIdentities.some((identity) => {
        const name = identity.querySelector('strong')?.getBoundingClientRect();
        const state = identity.querySelector('.node-state')?.getBoundingClientRect();
        return !!name && !!state && name.left < state.right && name.right > state.left &&
          name.top < state.bottom && name.bottom > state.top;
      })
    };
  });
  expect(longIdentityTypography.count).toBeGreaterThan(0);
  expect(longIdentityTypography.sameFontSize).toBe(true);
  expect(longIdentityTypography.overlapping).toBe(false);
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
    await page.keyboard.press('ArrowRight');
    const layout = await page.evaluate(() => ({
      documentHeight: document.documentElement.scrollHeight,
      viewportHeight: document.documentElement.clientHeight,
      stageHeight: document.querySelector('.stage')?.getBoundingClientRect().height || 0,
      headerHeight: document.querySelector('.trace-header')?.getBoundingClientRect().height || 0,
      transportHeight: document.querySelector('.transport')?.getBoundingClientRect().height || 0,
      nameFontSize: getComputedStyle(document.querySelector<HTMLElement>('[data-node-id="Root"] strong')!).fontSize,
      stateFontSize: getComputedStyle(document.querySelector<HTMLElement>('[data-node-id="Root"] .node-state')!).fontSize
    }));
    expect(layout.documentHeight).toBeLessThanOrEqual(layout.viewportHeight);
    expect(layout.stageHeight / viewport.height).toBeGreaterThanOrEqual(0.72);
    expect(layout.headerHeight).toBeLessThan(viewport.height * 0.12);
    expect(layout.transportHeight).toBeLessThan(viewport.height * 0.12);
    expect(layout.stateFontSize).toBe(layout.nameFontSize);
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
      overlapping,
      sameFontSize: identities.every((identity) => {
        const name = identity.querySelector<HTMLElement>('strong');
        const state = identity.querySelector<HTMLElement>('.node-state');
        return !!name && !!state && getComputedStyle(name).fontSize === getComputedStyle(state).fontSize;
      })
    };
  });
  expect(layout.pageOverflow).toBeLessThanOrEqual(1);
  expect(layout.stageOverflow).toBeGreaterThan(0);
  expect(layout.overlapping).toBe(false);
  expect(layout.sameFontSize).toBe(true);
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
