import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import FrameTree from './FrameTree.svelte';
import NodeCard from './NodeCard.svelte';
import SignalArrow from './SignalArrow.svelte';
import type { AnimationFrame, AnimationMoment } from './types';
import { shouldRenderSignalArrow } from './visibility';

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
  index: 0, moment_id: 'sig-0001:feedback', nodes: [node], sibling_order: { '$root': ['Root'] }
};
const transition: AnimationMoment = {
  index: 0, id: 'sig-0001:feedback', kind: 'feedback', event_sequence: 4,
  signal_id: 'sig-0001', cause_id: null, source: 'Root', target: 'Root', signal: 'Start',
  delivery: 'root', handler: { id: 'Root.Transition::Start', kind: 'Transition' },
  outcome: 'completed', reason: null, transfer: { from: 'Root', to: 'Root' },
  response: { before_state: 'Base', after_state: 'Ready' }
};

describe('Signal response effects', () => {
  it('shows a Transition before state before commit and after state during response', async () => {
    component = mount(FrameTree, {
      target: document.body,
      props: { frame, activeMoment: transition, phase: 'before', reducedMotion: true }
    });
    await tick();
    expect(document.querySelector('[data-node-id="Root"]')?.getAttribute('data-state')).toBe('Base');
    await unmount(component);
    component = mount(FrameTree, {
      target: document.body,
      props: { frame, activeMoment: transition, phase: 'feedback', reducedMotion: true }
    });
    await tick();
    expect(document.querySelector('[data-node-id="Root"]')?.getAttribute('data-state')).toBe('Ready');
  });

  it('marks request arrival and Action feedback without inventing a state', async () => {
    component = mount(NodeCard, {
      target: document.body,
      props: { node, activeTarget: true, effectPhase: 'request' }
    });
    await tick();
    expect(document.querySelector('[data-node-id="Root"]')?.classList.contains('request-arrival')).toBe(true);

    await unmount(component);
    component = mount(NodeCard, {
      target: document.body,
      props: {
        node,
        activeTarget: true,
        responseKind: 'Action',
        outcome: 'rejected',
        effectPhase: 'feedback'
      }
    });
    await tick();
    const card = document.querySelector('[data-node-id="Root"]');
    expect(card?.classList.contains('feedback-response')).toBe(true);
    expect(card?.classList.contains('action-response')).toBe(true);
    expect(card?.classList.contains('error-response')).toBe(true);
    expect(card?.getAttribute('data-state')).toBe('Ready');
  });

  it('keeps settle distinct from feedback while preserving exceptional effects', async () => {
    component = mount(NodeCard, {
      target: document.body,
      props: { node, activeTarget: true, responseKind: 'Transition', effectPhase: 'settle' }
    });
    await tick();
    let card = document.querySelector('[data-node-id="Root"]');
    expect(card?.classList.contains('settling')).toBe(true);
    expect(card?.classList.contains('feedback-response')).toBe(false);

    await unmount(component);
    component = mount(NodeCard, {
      target: document.body,
      props: {
        node, activeTarget: true, responseKind: 'Transition', outcome: 'failed', effectPhase: 'settle'
      }
    });
    await tick();
    card = document.querySelector('[data-node-id="Root"]');
    expect(card?.classList.contains('settling')).toBe(true);
    expect(card?.classList.contains('error-response')).toBe(true);
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

  it('alternates row and bottom-to-top layouts through four levels', async () => {
    const nodes = [
      node,
      { ...node, id: 'Level2First', parent: 'Root', first_seen: 1 },
      { ...node, id: 'Level2Second', parent: 'Root', first_seen: 2 },
      { ...node, id: 'Level3First', parent: 'Level2First', first_seen: 3 },
      { ...node, id: 'Level3Second', parent: 'Level2First', first_seen: 4 },
      { ...node, id: 'Level4First', parent: 'Level3First', first_seen: 5 },
      { ...node, id: 'Level4Second', parent: 'Level3First', first_seen: 6 }
    ];
    const deepFrame: AnimationFrame = {
      index: 0,
      moment_id: 'sig-0001:terminal',
      nodes,
      sibling_order: {
        '$root': ['Root'],
        Root: ['Level2First', 'Level2Second'],
        Level2First: ['Level3First', 'Level3Second'],
        Level3First: ['Level4First', 'Level4Second']
      }
    };
    component = mount(FrameTree, { target: document.body, props: { frame: deepFrame } });
    await tick();

    const layouts = Array.from(document.querySelectorAll<HTMLElement>('[data-layout]'))
      .map((element) => [element.dataset.level, element.dataset.layout, element.dataset.alignment]);
    expect(layouts).toEqual([
      ['1', 'row', 'bottom'], ['2', 'up', 'left'],
      ['3', 'row', 'bottom'], ['4', 'up', 'left']
    ]);
    expect(document.querySelector('[data-node-id="Root"]')?.getAttribute('data-children-position'))
      .toBe('above-identity');
  });

  it('uses compact wrapping identity bands and bare state names', async () => {
    component = mount(NodeCard, {
      target: document.body,
      props: { node: { ...node, id: 'A-very-long-system-name-that-must-wrap' } }
    });
    await tick();
    const identity = document.querySelector('.node-identity') as Element;
    expect(identity.textContent).toContain('Ready');
    expect(identity.textContent).not.toContain('State::');
    expect(identity.getAttribute('data-width-policy')).toBe('compact');
  });

  it('suppresses only ancestor-to-descendant arrows in the rendered hierarchy', async () => {
    const nodes = [
      node,
      { ...node, id: 'Level2First', parent: 'Root', first_seen: 1 },
      { ...node, id: 'Level2Second', parent: 'Root', first_seen: 2 },
      { ...node, id: 'Level3First', parent: 'Level2First', first_seen: 3 },
      { ...node, id: 'Level3Second', parent: 'Level2First', first_seen: 4 },
      { ...node, id: 'Level4First', parent: 'Level3First', first_seen: 5 }
    ];
    const deepFrame: AnimationFrame = {
      index: 0,
      moment_id: 'sig-0001:terminal',
      nodes,
      sibling_order: {
        '$root': ['Root'],
        Root: ['Level2First', 'Level2Second'],
        Level2First: ['Level3First', 'Level3Second'],
        Level3First: ['Level4First']
      }
    };
    component = mount(FrameTree, { target: document.body, props: { frame: deepFrame } });
    await tick();
    const endpoint = (id: string) =>
      document.querySelector<HTMLElement>(`[data-node-id="${id}"]`)!;

    expect(shouldRenderSignalArrow(endpoint('Root'), endpoint('Level2First'))).toBe(false);
    expect(shouldRenderSignalArrow(endpoint('Root'), endpoint('Level4First'))).toBe(false);
    expect(shouldRenderSignalArrow(endpoint('Root'), endpoint('Root'))).toBe(true);
    expect(shouldRenderSignalArrow(endpoint('Level2First'), endpoint('Level2Second'))).toBe(true);
    expect(shouldRenderSignalArrow(endpoint('Level4First'), endpoint('Level3Second'))).toBe(true);
    expect(shouldRenderSignalArrow(endpoint('Level4First'), endpoint('Root'))).toBe(true);

    await unmount(component);
    component = mount(FrameTree, {
      target: document.body,
      props: {
        frame: deepFrame,
        activeMoment: {
          ...transition,
          id: 'sig-0001:request',
          kind: 'request',
          source: 'Root',
          target: 'Level4First',
          transfer: { from: 'Root', to: 'Level4First' }
        },
        phase: 'request',
        reducedMotion: true
      }
    });
    await tick();
    expect(document.querySelector('[data-node-id="Level4First"]')?.classList.contains('request-arrival'))
      .toBe(true);
  });

  it('renders a red dashed self-loop contract for exceptional self Signals', async () => {
    component = mount(SignalArrow, {
      target: document.body,
      props: {
        geometry: {
          path: 'M 10 10 C 20 30, 0 30, 5 10', labelX: 10, labelY: 32,
          self: true, direction: 'self'
        },
        signal: 'Retry', outcome: 'failed', width: 200, height: 120
      }
    });
    await tick();
    const arrow = document.querySelector('svg');
    expect(arrow?.classList.contains('error')).toBe(true);
    expect(arrow?.getAttribute('data-arrow-kind')).toBe('self');
    expect(arrow?.getAttribute('aria-label')).toContain('failed');
    for (const id of ['signal-arrowhead', 'signal-arrowhead-error']) {
      const marker = document.querySelector(`#${id}`);
      expect(marker?.getAttribute('markerWidth')).toBe('5');
      expect(marker?.getAttribute('markerHeight')).toBe('4');
      expect(marker?.getAttribute('refX')).toBe('5');
      expect(marker?.getAttribute('refY')).toBe('2');
      expect(marker?.querySelector('path')?.getAttribute('d')).toBe('M 0 0 L 5 2 L 0 4 z');
    }
  });
});
