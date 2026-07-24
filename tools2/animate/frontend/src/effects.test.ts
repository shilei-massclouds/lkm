import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import FrameTree from './FrameTree.svelte';
import NodeCard from './NodeCard.svelte';
import SignalArrow from './SignalArrow.svelte';
import type { AnimationFrame, AnimationStep } from './types';

let component: ReturnType<typeof mount> | null = null;
afterEach(async () => {
  if (component) await unmount(component);
  component = null;
  document.body.replaceChildren();
});

const node = {
  id: 'Root', parent: null, kind: 'system' as const, state: 'Ready',
  structural: false, first_seen: 0
};
const frame: AnimationFrame = {
  index: 0, step_id: 'sig-0001', nodes: [node], sibling_order: { '$root': ['Root'] }
};
const transition: AnimationStep = {
  index: 0, id: 'sig-0001', cause_id: null, source: 'Root', target: 'Root', signal: 'Start',
  delivery: 'root', handler: { id: 'Root.Transition::Start', kind: 'Transition' },
  outcome: 'completed', reason: null, response: { before_state: 'Base', after_state: 'Ready' }
};

describe('Signal response effects', () => {
  it('shows a Transition before state during send and after state during response', async () => {
    component = mount(FrameTree, {
      target: document.body,
      props: { frame, activeStep: transition, phase: 'send', reducedMotion: true }
    });
    await tick();
    expect(document.querySelector('[data-node-id="Root"]')?.getAttribute('data-state')).toBe('Base');
    await unmount(component);
    component = mount(FrameTree, {
      target: document.body,
      props: { frame, activeStep: transition, phase: 'response', reducedMotion: true }
    });
    await tick();
    expect(document.querySelector('[data-node-id="Root"]')?.getAttribute('data-state')).toBe('Ready');
  });

  it('marks Action and exceptional responses without inventing a state', async () => {
    component = mount(NodeCard, {
      target: document.body,
      props: {
        node,
        activeTarget: true,
        responseKind: 'Action',
        outcome: 'rejected',
        responsePhase: true
      }
    });
    await tick();
    const card = document.querySelector('[data-node-id="Root"]');
    expect(card?.classList.contains('action-response')).toBe(true);
    expect(card?.classList.contains('error-response')).toBe(true);
    expect(card?.getAttribute('data-state')).toBe('Ready');
  });

  it('distinguishes stateless systems from structural and external nodes', async () => {
    component = mount(NodeCard, {
      target: document.body,
      props: { node: { ...node, state: null } }
    });
    await tick();
    const identity = document.querySelector('.node-identity');
    expect(document.body.textContent).toContain('Stateless');
    expect(identity?.querySelector('strong')?.textContent).toBe('Root');
    expect(identity?.querySelector('.node-state')?.textContent).toBe('Stateless');
    expect(identity?.querySelector('.node-kind')).toBeNull();
    expect(document.body.textContent).not.toContain('SYSTEM');

    await unmount(component);
    component = mount(NodeCard, {
      target: document.body,
      props: { node: { ...node, structural: true, state: null } }
    });
    await tick();
    expect(document.querySelector('.node-kind')?.textContent).toBe('Structure');

    await unmount(component);
    component = mount(NodeCard, {
      target: document.body,
      props: { node: { ...node, kind: 'external', state: null } }
    });
    await tick();
    expect(document.querySelector('.node-kind')?.textContent).toBe('External');
  });

  it('renders a red dashed self-loop contract for exceptional self Signals', async () => {
    component = mount(SignalArrow, {
      target: document.body,
      props: {
        geometry: { path: 'M 10 10 C 20 30, 0 30, 5 10', labelX: 10, labelY: 32, self: true },
        signal: 'Retry', outcome: 'failed', width: 200, height: 120
      }
    });
    await tick();
    const arrow = document.querySelector('svg');
    expect(arrow?.classList.contains('error')).toBe(true);
    expect(arrow?.getAttribute('data-arrow-kind')).toBe('self');
    expect(arrow?.getAttribute('aria-label')).toContain('failed');
  });
});
