<script lang="ts">
  import { flip } from 'svelte/animate';
  import NodeCard from './NodeCard.svelte';
  import type { AnimationFrame, AnimationMoment, AnimationPhase } from './types';

  let {
    frame,
    activeMoment = null,
    phase = 'idle',
    reducedMotion = false
  }: {
    frame: AnimationFrame;
    activeMoment?: AnimationMoment | null;
    phase?: AnimationPhase;
    reducedMotion?: boolean;
  } = $props();
  const nodes = $derived(new Map(frame.nodes.map((node) => [node.id, node])));

  function childrenOf(parent: string) {
    return frame.sibling_order[parent] || [];
  }

  function layoutFor(level: number) {
    return level % 2 === 1 ? 'row' : 'up';
  }

  function displayNode(id: string) {
    const node = nodes.get(id);
    if (
      node && activeMoment?.kind === 'feedback' && activeMoment.target === id &&
      activeMoment.handler.kind === 'Transition' && phase === 'before'
    ) {
      return { ...node, state: activeMoment.response.before_state };
    }
    return node;
  }
</script>

{#snippet renderNode(id: string, level: number)}
  {@const node = displayNode(id)}
  {#if node}
    <NodeCard
      {node}
      {level}
      activeSource={activeMoment?.source === id}
      activeTarget={activeMoment?.target === id}
      responseKind={activeMoment?.handler.kind || null}
      outcome={activeMoment?.outcome || 'completed'}
      effectPhase={phase}
    >
      {@const childIds = childrenOf(id)}
      {#if childIds.length}
        {@const childLevel = level + 1}
        <div
          class="node-children layout-{layoutFor(childLevel)}"
          data-children-of={id}
          data-level={childLevel}
          data-layout={layoutFor(childLevel)}
          data-alignment={layoutFor(childLevel) === 'row' ? 'bottom' : 'left'}
        >
          {#each childIds as childId (childId)}
            <div class="node-slot" animate:flip={{ duration: reducedMotion ? 0 : 280 }}>
              {@render renderNode(childId, childLevel)}
            </div>
          {/each}
        </div>
      {/if}
    </NodeCard>
  {/if}
{/snippet}

<div
  class="frame-forest layout-row"
  data-frame-index={frame.index}
  data-level="1"
  data-layout="row"
  data-alignment="bottom"
>
  {#each childrenOf('$root') as rootId (rootId)}
    <div class="node-slot" animate:flip={{ duration: reducedMotion ? 0 : 280 }}>
      {@render renderNode(rootId, 1)}
    </div>
  {/each}
</div>
