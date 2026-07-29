import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import App from './App.svelte';
import type { AnimationTrace } from './types';

const human = {
  id: 'Human', parent: null, kind: 'external' as const, state: null,
  structural: false, first_seen: 0
};
const rootBase = {
  id: 'Root', parent: null, kind: 'system' as const, state: 'Base',
  structural: false, first_seen: 1
};
const rootReady = { ...rootBase, state: 'Ready' };
const child = {
  id: 'Child', parent: 'Root', kind: 'system' as const, state: 'Base',
  structural: false, first_seen: 2
};
const animation: AnimationTrace = {
  schema: 'lkm.spec.signal-animation', version: 3, source: 'fixture.spec',
  inputs: { model_fingerprint: 'sha256:fixture' },
  trace: {
    verdict: 'complete', total_signals: 2, total_moments: 4,
    root_request: { source: 'Human', target: 'Root', signal: 'Start' },
    boundary: { source: 'Root', target: 'Next', signal: 'Run', normalized_signal: 'Next.Run' }
  },
  moments: [
    {
      index: 0, id: 'sig-0001:request', kind: 'request', event_sequence: 2,
      signal_id: 'sig-0001', cause_id: null, source: 'Human', target: 'Root',
      signal: 'Start', delivery: 'root', handler: {
        id: 'Root.Transition::Start', kind: 'Transition',
        description: '先关闭全部中断分路门控，再清空全部待决中断信号。'
      },
      outcome: 'completed', reason: null, transfer: { from: 'Human', to: 'Root' },
      response: { before_state: 'Base', after_state: 'Ready' }
    },
    {
      index: 1, id: 'sig-0001:feedback', kind: 'feedback', event_sequence: 4,
      signal_id: 'sig-0001', cause_id: null, source: 'Human', target: 'Root',
      signal: 'Start', delivery: 'root', handler: {
        id: 'Root.Transition::Start', kind: 'Transition',
        description: '先关闭全部中断分路门控，再清空全部待决中断信号。'
      },
      outcome: 'completed', reason: null, transfer: { from: 'Root', to: 'Human' },
      response: { before_state: 'Base', after_state: 'Ready' }
    },
    {
      index: 2, id: 'sig-0002:request', kind: 'request', event_sequence: 6,
      signal_id: 'sig-0002', cause_id: 'sig-0001', source: 'Root', target: 'Child',
      signal: 'Inspect', delivery: 'drives', handler: { id: 'Child.Action::Inspect', kind: 'Action' },
      outcome: 'completed', reason: null, transfer: { from: 'Root', to: 'Child' },
      response: { before_state: null, after_state: null }
    },
    {
      index: 3, id: 'sig-0002:feedback', kind: 'feedback', event_sequence: 8,
      signal_id: 'sig-0002', cause_id: 'sig-0001', source: 'Root', target: 'Child',
      signal: 'Inspect', delivery: 'drives', handler: { id: 'Child.Action::Inspect', kind: 'Action' },
      outcome: 'completed', reason: null, transfer: { from: 'Child', to: 'Root' },
      response: { before_state: null, after_state: null }
    }
  ],
  initial_frame: { index: -1, moment_id: null, nodes: [], sibling_order: {} },
  frames: [
    { index: 0, moment_id: 'sig-0001:request', nodes: [human, rootBase], sibling_order: { '$root': ['Human', 'Root'] } },
    { index: 1, moment_id: 'sig-0001:feedback', nodes: [human, rootReady], sibling_order: { '$root': ['Human', 'Root'] } },
    { index: 2, moment_id: 'sig-0002:request', nodes: [human, rootReady, child], sibling_order: { '$root': ['Human', 'Root'], Root: ['Child'] } },
    { index: 3, moment_id: 'sig-0002:feedback', nodes: [human, rootReady, child], sibling_order: { '$root': ['Human', 'Root'], Root: ['Child'] } }
  ]
};

let app: ReturnType<typeof mount> | null = null;

afterEach(async () => {
  if (app) await unmount(app);
  app = null;
  document.body.replaceChildren();
});

function nodes() {
  return Array.from(document.querySelectorAll<HTMLElement>('[data-node-id]')).map((node) => node.dataset.nodeId);
}

async function settle() {
  await new Promise((resolve) => window.setTimeout(resolve, 0));
  await tick();
}

describe('deterministic moment navigation', () => {
  it('uses buttons and restores the exact previous frame', async () => {
    app = mount(App, { target: document.body, props: { animation } });
    await tick();
    expect(nodes()).toEqual([]);
    const [previous, next] = Array.from(document.querySelectorAll<HTMLButtonElement>('button'));
    expect(previous.disabled).toBe(true);
    next.click();
    await settle();
    expect(nodes()).toEqual(['Human', 'Root']);
    expect(document.querySelector('[data-node-id="Root"]')?.getAttribute('data-state')).toBe('Base');
    expect(document.body.textContent).toContain('请求到达：root');
    expect(document.querySelector('.model-description')?.textContent).toContain('先关闭全部中断分路门控');
    next.click();
    await settle();
    expect(document.querySelector('[data-node-id="Root"]')?.getAttribute('data-state')).toBe('Ready');
    expect(document.body.textContent).toContain('反馈 Transition：Base → Ready');
    expect(document.querySelector('.step-copy h2')?.textContent).toContain('Root — Start → Human');
    expect(document.body.textContent).not.toContain('State::');
    next.click();
    await settle();
    expect(nodes()).toEqual(['Human', 'Root', 'Child']);
    const rootChildren = document.querySelector('[data-children-of="Root"]');
    expect(rootChildren?.closest('[data-node-id="Root"]')).not.toBeNull();
    expect(Array.from(rootChildren?.children || []).map((slot) => slot.querySelector('[data-node-id]')?.getAttribute('data-node-id'))).toEqual(['Child']);
    next.click();
    await settle();
    expect(document.body.textContent).toContain('反馈 Action：不改变 lifecycle state');
    expect(document.body.textContent).toContain('边界：Root — Run → Next（发送前）');
    previous.click();
    await tick();
    expect(nodes()).toEqual(['Human', 'Root', 'Child']);
    expect(document.querySelector('[data-node-id="Root"]')?.getAttribute('data-state')).toBe('Ready');
    expect(document.body.textContent).not.toContain('发送前');
  });

  it('maps ArrowLeft and ArrowRight to the same navigation', async () => {
    app = mount(App, { target: document.body, props: { animation } });
    await tick();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight' }));
    await settle();
    expect(document.querySelector('.counter')?.textContent).toContain('时刻 1 / 4');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft' }));
    await tick();
    expect(nodes()).toEqual([]);
    expect(document.querySelector('.counter')?.textContent).toContain('时刻 0 / 4');
  });

  it('renders the compact metadata header without the lede or visible footer', async () => {
    app = mount(App, { target: document.body, props: { animation } });
    await tick();
    expect(document.querySelector('.lede')).toBeNull();
    expect(document.querySelector('footer')).toBeNull();
    expect(document.querySelector('.trace-header')?.textContent).toContain('Source:Human');
    expect(document.querySelector('.trace-header')?.textContent).toContain('Verdict:complete');
    expect(document.querySelector('.trace-header')?.textContent).toContain('Signals:2');
    const protocol = Array.from(document.querySelectorAll('.trace-meta div')).at(-1);
    expect(protocol?.getAttribute('title')).toContain('fixture.spec');
    expect(protocol?.getAttribute('title')).toContain('sha256:fixture');
  });
});
