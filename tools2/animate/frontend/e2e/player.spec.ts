import { expect, test, type Page } from '@playwright/test';
import { mkdtempSync, readFileSync, rmSync, statSync } from 'node:fs';
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
let pipelineMoments = 0;
let effectsMoments = 0;
let alternatingMoments = 0;
let alternatingRequestMoments: number[] = [];
let mainSignals = 0;
let mainMoments = 0;

function generate(
  name: string, spec: string, requestArgs: string[], expectedStatus: number, expectedVerdict: string
) {
  const html = join(generated, `${name}.html`);
  const work = join(generated, `${name}-work`);
  const result = spawnSync(
    'python3',
    [
      '-m', 'pyveri', spec, ...requestArgs, '--max-depth', 'all',
      '--max-breadth', 'all', '--work-dir', work, '--html-out', html,
      '-o', join(generated, `${name}.txt`)
    ],
    { cwd: repository, env: { ...process.env, PYTHONPATH: pythonPath }, encoding: 'utf8' }
  );
  if (result.status !== expectedStatus) {
    throw new Error(`fixture generation failed (${result.status}): ${result.stderr}`);
  }
  const derivation = JSON.parse(readFileSync(join(work, 'derive.json'), 'utf8'));
  if (derivation.verdict !== expectedVerdict) {
    throw new Error(`fixture verdict was ${derivation.verdict}, expected ${expectedVerdict}`);
  }
  const terminalKinds = new Set([
    'response_completed', 'signal_rejected', 'signal_failed', 'signal_truncated',
    'signal_stopped', 'response_stopped'
  ]);
  const visualEvents = derivation.events.filter(
    (event: { kind: string }) => event.kind === 'signal_received' || terminalKinds.has(event.kind)
  );
  const requestMoments = derivation.signals.map((signal: { id: string }) =>
    visualEvents.findIndex(
      (event: { kind: string; signal_id?: string }) =>
        event.kind === 'signal_received' && event.signal_id === signal.id
    ) + 1
  );
  return {
    html,
    totalSignals: derivation.signals.length,
    totalMoments: visualEvents.length,
    requestMoments
  };
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
  const pipeline = generate(
    'pipeline', join(repository, 'tools2/tests/fixtures/pipeline.spec'),
    ['--signal', 'Root.Start'], 0, 'complete'
  );
  pipelineHtml = pipeline.html;
  pipelineMoments = pipeline.totalMoments;
  const effects = generate(
    'effects', join(repository, 'tools2/tests/fixtures/animation-effects.spec'),
    ['--signal', 'Root.Begin'], 1, 'failed'
  );
  effectsHtml = effects.html;
  effectsMoments = effects.totalMoments;
  const alternating = generate(
    'alternating', join(repository, 'tools2/tests/fixtures/alternating-layout.spec'),
    ['--signal', 'Controller.Begin'], 0, 'complete'
  );
  alternatingHtml = alternating.html;
  alternatingMoments = alternating.totalMoments;
  alternatingRequestMoments = alternating.requestMoments;
  const main = generate(
    'main', join(repository, 'spec/model/main.spec'), ['--until', 'Kernel.Enable'], 0, 'reached'
  );
  mainHtml = main.html;
  mainSignals = main.totalSignals;
  mainMoments = main.totalMoments;
});

test.afterAll(() => {
  if (generated) rmSync(generated, { recursive: true, force: true });
});

test('offline controls restore exact hierarchy and sibling order', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await openOffline(page, pipelineHtml);
  const counter = page.locator('.counter');
  expect(pipelineMoments).toBe(8);
  await expect(counter).toContainText('时刻 0 / 8');
  await expect(page.locator('[data-node-id]')).toHaveCount(0);

  await page.keyboard.press('ArrowRight');
  await expect(counter).toContainText('时刻 1 / 8');
  await expect(page.locator('[data-node-id="Root"]')).toHaveAttribute('data-state', 'Base');
  await expect(page.locator('[data-node-id="Human"]')).toBeVisible();
  await page.getByRole('button', { name: '下一步' }).click();
  await expect(counter).toContainText('时刻 2 / 8');
  await expect(page.locator('[data-node-id="Child"]')).toBeVisible();
  await page.keyboard.press('ArrowRight');
  await expect(counter).toContainText('时刻 3 / 8');
  await expect(page.locator('[data-node-id="Root"]')).toHaveAttribute('data-state', 'Base');
  await page.keyboard.press('ArrowRight');
  await expect(counter).toContainText('时刻 4 / 8');
  await expect(page.locator('[data-node-id="Root"]')).toHaveAttribute('data-state', 'Ready');
  for (let index = 5; index <= pipelineMoments; index += 1) {
    await page.keyboard.press('ArrowRight');
    await expect(counter).toContainText(`时刻 ${index} / ${pipelineMoments}`);
  }
  await expect(page.locator('[data-children-of="Root"] > .node-slot')).toHaveCount(3);
  await expect(page.locator('[data-children-of="Root"] > .node-slot')).toHaveText([
    /Child/, /Async/, /Sink/
  ]);

  for (let index = pipelineMoments - 1; index >= 1; index -= 1) {
    await page.keyboard.press('ArrowLeft');
  }
  await expect(counter).toContainText(`时刻 1 / ${pipelineMoments}`);
  await expect(page.locator('[data-node-id]')).toHaveCount(2);
  await expect(page.locator('[data-node-id="Root"]')).toHaveAttribute('data-state', 'Base');
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
  const advanceToMoment = async (index: number) => {
    while (!((await counter.textContent()) || '').includes(`时刻 ${index} / ${alternatingMoments}`)) {
      await page.keyboard.press('ArrowRight');
    }
  };
  const advanceToSignal = (signalIndex: number) =>
    advanceToMoment(alternatingRequestMoments[signalIndex - 1]);

  expect(alternatingMoments).toBe(24);
  await advanceToSignal(1);
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

  await advanceToSignal(3);
  const shallow = await contentRects(['Human', 'Controller', 'EarlyRoot', 'LateRoot']);
  expect(shallow.Human!.left).toBeLessThan(shallow.Controller!.left);
  expect(shallow.Controller!.left).toBeLessThan(shallow.EarlyRoot!.left);
  expect(shallow.EarlyRoot!.left).toBeLessThan(shallow.LateRoot!.left);
  expect(Math.abs(shallow.Human!.bottom - shallow.LateRoot!.bottom)).toBeLessThan(2);

  await advanceToSignal(5);
  const firstLevel2 = await contentRects(['Human', 'Controller', 'EarlyRoot', 'LateRoot', 'Level2First']);
  await advanceToSignal(6);
  const secondLevel2 = await contentRects(['Human', 'Controller', 'EarlyRoot', 'LateRoot', 'Level2First', 'Level2Second']);
  expect(Math.abs(secondLevel2.Level2First!.left - firstLevel2.Level2First!.left)).toBeLessThan(2);
  expect(Math.abs(secondLevel2.Level2First!.bottom - firstLevel2.Level2First!.bottom)).toBeLessThan(2);
  expect(secondLevel2.Level2Second!.bottom).toBeLessThan(secondLevel2.Level2First!.top);

  await advanceToSignal(8);
  const firstLevel3 = await contentRects(['Human', 'Controller', 'EarlyRoot', 'LateRoot', 'Level3First']);
  await advanceToSignal(9);
  const secondLevel3 = await contentRects(['Human', 'Controller', 'EarlyRoot', 'LateRoot', 'Level3First', 'Level3Second']);
  expect(Math.abs(secondLevel3.EarlyRoot!.left - firstLevel3.EarlyRoot!.left)).toBeLessThan(2);
  expect(Math.abs(secondLevel3.EarlyRoot!.bottom - firstLevel3.EarlyRoot!.bottom)).toBeLessThan(2);
  expect(secondLevel3.Level3Second!.left).toBeGreaterThan(secondLevel3.Level3First!.right);
  expect(secondLevel3.LateRoot!.left).toBeGreaterThan(firstLevel3.LateRoot!.left);
  expect(Math.abs(secondLevel3.Human!.left - shallow.Human!.left)).toBeLessThan(2);
  expect(Math.abs(secondLevel3.Controller!.left - shallow.Controller!.left)).toBeLessThan(2);

  await advanceToSignal(11);
  const firstLevel4 = await contentRects(['Level4First']);
  await advanceToSignal(12);
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

  const finalLayoutMoment = alternatingRequestMoments[11];
  for (let index = finalLayoutMoment - 1; index >= 1; index -= 1) {
    await page.keyboard.press('ArrowLeft');
    await expect(counter).toContainText(`时刻 ${index} / ${alternatingMoments}`);
  }
  const restored = await contentRects(['Human']);
  const restoredScroll = await page.locator('.stage').evaluate((stage) => ({
    left: stage.scrollLeft,
    top: stage.scrollTop
  }));
  expect(Math.abs(restored.Human!.left - initial.Human!.left)).toBeLessThan(2);
  expect(Math.abs(restored.Human!.bottom - initial.Human!.bottom)).toBeLessThan(2);
  expect(restoredScroll).toEqual(initialScroll);

  await advanceToMoment(finalLayoutMoment);
  const repeatedScroll = await page.locator('.stage').evaluate((stage) => ({
    left: stage.scrollLeft,
    top: stage.scrollTop
  }));
  expect(repeatedScroll).toEqual(finalScroll);
});

test('request, feedback, settle, and exceptional self effects stay distinct', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await openOffline(page, pipelineHtml);
  const pipelineNext = page.getByRole('button', { name: '下一步' });
  await pipelineNext.click();
  const firstArrow = page.locator('.signal-overlay');
  await expect(firstArrow).toHaveAttribute('data-arrow-kind', 'ordinary');
  await expect(page.locator('[data-node-id="Root"]')).toHaveClass(/request-arrival/);
  await expect(firstArrow.locator('#signal-arrowhead')).toHaveAttribute('markerWidth', '5');
  await expect(firstArrow.locator('#signal-arrowhead')).toHaveAttribute('markerHeight', '4');
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
  await expect(page.locator('[data-node-id="Child"]')).toHaveClass(/request-arrival/);
  await expect(page.locator('.stage')).toHaveAttribute('data-animation-phase', 'idle');
  await pipelineNext.click();
  await expect(page.locator('.signal-overlay')).toHaveCount(0);
  await page.waitForFunction(() =>
    document.querySelector('[data-node-id="Child"]')?.classList.contains('action-response')
  );
  await expect(page.locator('.stage')).toHaveAttribute('data-animation-phase', 'idle');

  await pipelineNext.click();
  await page.waitForFunction(() =>
    document.querySelector('[data-node-id="Root"]')?.classList.contains('feedback-response')
  );
  await expect(page.locator('.stage')).toHaveAttribute('data-animation-phase', 'idle');
  await pipelineNext.click();
  await expect(page.locator('[data-node-id="Async"]')).toHaveClass(/request-arrival/);
  await expect(page.locator('.signal-overlay')).toHaveCount(0);
  await expect(page.locator('.stage')).toHaveAttribute('data-animation-phase', 'idle');
  await pipelineNext.click();
  await expect(page.locator('[data-node-id="Async"]')).toHaveClass(/settling/);
  await expect(page.locator('[data-node-id="Async"]')).not.toHaveClass(/feedback-response/);
  await expect(page.locator('.stage')).toHaveAttribute('data-animation-phase', 'idle');
  await expect(page.locator('.step-copy')).toContainText('目标内部处理完成 Transition');

  await openOffline(page, effectsHtml);
  const next = page.getByRole('button', { name: '下一步' });
  await next.click();
  await expect(page.locator('.signal-overlay')).toHaveAttribute('data-arrow-kind', 'ordinary');
  await expect(page.locator('[data-node-id="Root"]')).toHaveClass(/request-arrival/);
  await expect(page.locator('.stage')).toHaveAttribute('data-animation-phase', 'idle');
  await next.click();
  await expect(page.locator('.signal-overlay')).toHaveCount(0);
  await page.waitForFunction(() =>
    document.querySelector('[data-node-id="Root"]')?.classList.contains('action-response')
  );
  await expect(page.locator('.stage')).toHaveAttribute('data-animation-phase', 'idle');

  await next.click();
  const selfArrow = page.locator('.signal-overlay');
  await expect(selfArrow).toHaveAttribute('data-arrow-kind', 'self');
  await expect(selfArrow).toHaveAttribute('data-outcome', 'completed');
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
  await expect(page.locator('.stage')).toHaveAttribute('data-animation-phase', 'idle');
  await next.click();
  await expect(page.locator('.signal-overlay')).toHaveCount(0);
  await expect(page.locator('.reason')).toContainText('rejected · no_handler');
  await expect(page.locator('.step-copy')).toContainText('目标内部处理完成');
  await page.waitForFunction(() =>
    document.querySelector('[data-node-id="Root"]')?.classList.contains('settling') &&
      document.querySelector('[data-node-id="Root"]')?.classList.contains('error-response')
  );
});

test('reduced motion settles immediately and disables response movement', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await openOffline(page, effectsHtml);
  const started = Date.now();
  await page.getByRole('button', { name: '下一步' }).click();
  await expect(page.locator('.counter')).toContainText(`时刻 1 / ${effectsMoments}`);
  expect(Date.now() - started).toBeLessThan(500);
  await expect(page.locator('.stage')).toHaveClass(/reduced-motion/);
  await expect(page.locator('[data-node-id="Root"]')).toHaveCSS('animation-name', 'none');
});

test('main Kernel boundary trace loads offline, scrolls targets, and restores every moment', async ({ page }) => {
  test.setTimeout(180_000);
  expect(mainSignals).toBe(13);
  expect(mainMoments).toBe(26);
  expect(statSync(mainHtml).size / mainMoments).toBeLessThan(30 * 1024);
  await page.setViewportSize({ width: 700, height: 520 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const started = Date.now();
  await openOffline(page, mainHtml);
  expect(Date.now() - started).toBeLessThan(5_000);
  const counter = page.locator('.counter');
  await expect(page.locator('.trace-meta dd').nth(1)).toHaveText('reached');
  await expect(counter).toContainText(`时刻 0 / ${mainMoments}`);
  let sawScroll = false;
  for (let index = 1; index <= mainMoments; index += 1) {
    await page.keyboard.press('ArrowRight');
    await expect(counter).toContainText(`时刻 ${index} / ${mainMoments}`);
    if (!sawScroll) {
      sawScroll = await page.locator('.stage').evaluate(
        (stage) => stage.scrollTop > 0 || stage.scrollLeft > 0
      );
    }
  }
  expect(sawScroll).toBe(true);
  await expect(page.locator('.boundary')).toContainText('OpenSBI — Enable → Kernel（发送前）');
  const longIdentityTypography = await page.locator('.stage').evaluate((stage) => {
    const identities = Array.from(stage.querySelectorAll<HTMLElement>('.node-identity'));
    const longIdentities = identities.filter((identity) =>
      (identity.querySelector('strong')?.textContent?.length || 0) >= 12
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
  for (let index = mainMoments - 1; index >= 0; index -= 1) {
    await page.keyboard.press('ArrowLeft');
    await expect(counter).toContainText(`时刻 ${index} / ${mainMoments}`);
  }
  await expect(page.locator('[data-node-id]')).toHaveCount(0);
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
  for (let index = 0; index < pipelineMoments; index += 1) await page.keyboard.press('ArrowRight');
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
