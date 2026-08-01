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
    effectPhase = 'idle'
  }: {
    node: AnimationNode;
    level?: number;
    children?: Snippet;
    activeSource?: boolean;
    activeTarget?: boolean;
    responseKind?: 'Transition' | 'Action' | null;
    outcome?: string;
    effectPhase?: 'idle' | 'request' | 'before' | 'feedback' | 'settle' | 'yield' | 'resume' | 'terminal' | 'clear';
  } = $props();
  const stateClass = $derived(node.state ? `state-${node.state.toLowerCase()}` : 'state-none');
</script>

<article
  class:structural={node.structural}
  class:external={node.kind === 'external'}
  class:signal-source={activeSource}
  class:signal-target={activeTarget}
  class:request-arrival={activeTarget && effectPhase === 'request'}
  class:feedback-response={activeTarget && effectPhase === 'feedback'}
  class:action-response={activeTarget && effectPhase === 'feedback' && responseKind === 'Action'}
  class:settling={activeTarget && effectPhase === 'settle'}
  class:yielding={activeTarget && effectPhase === 'yield'}
  class:resuming={activeTarget && effectPhase === 'resume'}
  class:terminal-effect={activeTarget && effectPhase === 'terminal'}
  class:error-response={activeTarget && outcome !== 'completed' && ['feedback', 'settle', 'terminal'].includes(effectPhase)}
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
