<script lang="ts">
  import { flip } from 'svelte/animate';
  import NodeCard from './NodeCard.svelte';
  import type { AnimationFrame, AnimationPhase, AnimationStep } from './types';

  let {
    frame,
    activeStep = null,
    phase = 'idle',
    reducedMotion = false
  }: {
    frame: AnimationFrame;
    activeStep?: AnimationStep | null;
    phase?: AnimationPhase;
    reducedMotion?: boolean;
  } = $props();
  const nodes = $derived(new Map(frame.nodes.map((node) => [node.id, node])));

  function childrenOf(parent: string) {
    return frame.sibling_order[parent] || [];
  }

  function displayNode(id: string) {
    const node = nodes.get(id);
    if (
      node && activeStep?.target === id && activeStep.handler.kind === 'Transition' &&
      phase === 'send'
    ) {
      return { ...node, state: activeStep.response.before_state };
    }
    return node;
  }
</script>

{#snippet renderNode(id: string)}
  {@const node = displayNode(id)}
  {#if node}
    <NodeCard
      {node}
      activeSource={activeStep?.source === id}
      activeTarget={activeStep?.target === id}
      responseKind={activeStep?.handler.kind || null}
      outcome={activeStep?.outcome || 'completed'}
      responsePhase={phase === 'response'}
    >
      {@const childIds = childrenOf(id)}
      {#if childIds.length}
        <div class="node-children" data-children-of={id}>
          {#each childIds as childId (childId)}
            <div class="node-slot" animate:flip={{ duration: reducedMotion ? 0 : 280 }}>
              {@render renderNode(childId)}
            </div>
          {/each}
        </div>
      {/if}
    </NodeCard>
  {/if}
{/snippet}

<div class="frame-forest" data-frame-index={frame.index}>
  {#each childrenOf('$root') as rootId (rootId)}
    <div class="node-slot" animate:flip={{ duration: reducedMotion ? 0 : 280 }}>
      {@render renderNode(rootId)}
    </div>
  {/each}
</div>
