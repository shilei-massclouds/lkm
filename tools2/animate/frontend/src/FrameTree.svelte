<script lang="ts">
  import NodeCard from './NodeCard.svelte';
  import type { AnimationFrame } from './types';

  let { frame }: { frame: AnimationFrame } = $props();
  const nodes = $derived(new Map(frame.nodes.map((node) => [node.id, node])));

  function childrenOf(parent: string) {
    return frame.sibling_order[parent] || [];
  }
</script>

{#snippet renderNode(id: string)}
  {@const node = nodes.get(id)}
  {#if node}
    <NodeCard {node}>
      {@const childIds = childrenOf(id)}
      {#if childIds.length}
        <div class="node-children" data-children-of={id}>
          {#each childIds as childId (childId)}
            {@render renderNode(childId)}
          {/each}
        </div>
      {/if}
    </NodeCard>
  {/if}
{/snippet}

<div class="frame-forest" data-frame-index={frame.index}>
  {#each childrenOf('$root') as rootId (rootId)}
    {@render renderNode(rootId)}
  {/each}
</div>
