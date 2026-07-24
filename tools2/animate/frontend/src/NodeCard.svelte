<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { AnimationNode } from './types';

  let {
    node,
    level = 1,
    children,
    activeSource = false,
    activeTarget = false,
    responseKind = null,
    outcome = 'completed',
    responsePhase = false
  }: {
    node: AnimationNode;
    level?: number;
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
  data-level={level}
  data-state={node.state || 'none'}
  data-children-position="above-identity"
>
  <div class="node-identity" data-width-policy="compact">
    {#if node.structural || node.kind === 'external'}
      <span class="node-kind">{node.structural ? 'Structure' : 'External'}</span>
    {/if}
    <strong>{node.id}</strong>
    <span class="node-state">{node.state
      ? node.state
      : node.structural
        ? 'No state shown'
        : node.kind === 'external'
          ? 'Endpoint'
          : 'Stateless'}</span>
  </div>
  {#if children}
    {@render children()}
  {/if}
</article>
