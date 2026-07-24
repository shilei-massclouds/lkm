<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { AnimationNode } from './types';

  let {
    node,
    children,
    activeSource = false,
    activeTarget = false,
    responseKind = null,
    outcome = 'completed',
    responsePhase = false
  }: {
    node: AnimationNode;
    children?: Snippet;
    activeSource?: boolean;
    activeTarget?: boolean;
    responseKind?: 'Transition' | 'Action' | null;
    outcome?: string;
    responsePhase?: boolean;
  } = $props();
  const stateClass = $derived(node.state ? `state-${node.state.toLowerCase()}` : 'state-none');
</script>

<article
  class:structural={node.structural}
  class:external={node.kind === 'external'}
  class:signal-source={activeSource}
  class:signal-target={activeTarget}
  class:responding={activeTarget && responsePhase}
  class:action-response={activeTarget && responsePhase && responseKind === 'Action'}
  class:error-response={activeTarget && responsePhase && outcome !== 'completed'}
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
