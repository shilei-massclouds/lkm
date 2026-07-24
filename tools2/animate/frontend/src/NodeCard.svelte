<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { AnimationNode } from './types';

  let { node, children }: { node: AnimationNode; children?: Snippet } = $props();
  const stateClass = $derived(node.state ? `state-${node.state.toLowerCase()}` : 'state-none');
</script>

<article
  class:structural={node.structural}
  class:external={node.kind === 'external'}
  class="node-card {stateClass}"
  data-node-id={node.id}
  data-parent-id={node.parent || '$root'}
  data-state={node.state || 'none'}
>
  <div class="node-identity">
    <span class="node-kind">{node.structural ? 'Structure' : node.kind === 'external' ? 'External' : 'System'}</span>
    <strong>{node.id}</strong>
    <span class="node-state">{node.state ? `State::${node.state}` : node.structural ? 'No state shown' : 'Signal source'}</span>
  </div>
  {#if children}
    {@render children()}
  {/if}
</article>
