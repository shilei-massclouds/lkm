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
const asyncNode = {
  id: 'Async', parent: 'Root', kind: 'system' as const, state: 'Ready',
  structural: false, first_seen: 3
};

const animation: AnimationTrace = {
  schema: 'lkm.spec.signal-animation', version: 1, source: 'fixture.spec',
  inputs: { model_fingerprint: 'sha256:fixture' },
  trace: {
    verdict: 'complete', total_steps: 2,
    root_request: { source: 'Human', target: 'Root', signal: 'Start' }
  },
  steps: [
    {
      index: 0, id: 'sig-0001', cause_id: null, source: 'Human', target: 'Root',
      signal: 'Start', delivery: 'root', handler: { id: 'Root.Transition::Start', kind: 'Transition' },
      outcome: 'completed', reason: null, response: { before_state: 'Base', after_state: 'Ready' }
    },
    {
      index: 1, id: 'sig-0002', cause_id: 'sig-0001', source: 'Root', target: 'Child',
      signal: 'Inspect', delivery: 'drives', handler: { id: 'Child.Action::Inspect', kind: 'Action' },
      outcome: 'completed', reason: null, response: { before_state: null, after_state: null }
    }
  ],
  initial_frame: { index: -1, step_id: null, nodes: [human], sibling_order: { '$root': ['Human'] } },
  frames: [
    { index: 0, step_id: 'sig-0001', nodes: [human, rootReady], sibling_order: { '$root': ['Root', 'Human'] } },
    { index: 1, step_id: 'sig-0002', nodes: [human, rootBase, child, asyncNode], sibling_order: { '$root': ['Root', 'Human'], Root: ['Async', 'Child'] } }
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

describe('deterministic step navigation', () => {
  it('uses buttons and restores the exact previous frame', async () => {
    app = mount(App, { target: document.body, props: { animation } });
    await tick();
    expect(nodes()).toEqual(['Human']);
    const [previous, next] = Array.from(document.querySelectorAll<HTMLButtonElement>('button'));
    expect(previous.disabled).toBe(true);
    next.click();
    await tick();
    expect(nodes()).toEqual(['Root', 'Human']);
    expect(document.querySelector('[data-node-id="Root"]')?.getAttribute('data-state')).toBe('Ready');
    expect(document.body.textContent).toContain('State::Base → State::Ready');
    next.click();
    await tick();
    expect(nodes()).toEqual(['Root', 'Async', 'Child', 'Human']);
    const rootChildren = document.querySelector('[data-children-of="Root"]');
    expect(rootChildren?.closest('[data-node-id="Root"]')).not.toBeNull();
    expect(Array.from(rootChildren?.children || []).map((node) => node.getAttribute('data-node-id'))).toEqual(['Async', 'Child']);
    expect(document.body.textContent).toContain('Action：响应完成');
    previous.click();
    await tick();
    expect(nodes()).toEqual(['Root', 'Human']);
    expect(document.querySelector('[data-node-id="Root"]')?.getAttribute('data-state')).toBe('Ready');
  });

  it('maps ArrowLeft and ArrowRight to the same navigation', async () => {
    app = mount(App, { target: document.body, props: { animation } });
    await tick();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight' }));
    await tick();
    expect(document.querySelector('.counter')?.textContent).toContain('1 / 2');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft' }));
    await tick();
    expect(nodes()).toEqual(['Human']);
    expect(document.querySelector('.counter')?.textContent).toContain('0 / 2');
  });
});
