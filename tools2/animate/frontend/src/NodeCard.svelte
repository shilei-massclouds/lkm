<script lang="ts">
  import type { AnimationNode } from './types';

  let { node }: { node: AnimationNode } = $props();
  const stateClass = $derived(node.state ? `state-${node.state.toLowerCase()}` : 'state-none');
</script>

<article
  class:structural={node.structural}
  class:external={node.kind === 'external'}
  class="node-card {stateClass}"
  data-node-id={node.id}
  data-state={node.state || 'none'}
>
  <span class="node-kind">{node.structural ? 'Structure' : node.kind === 'external' ? 'External' : 'System'}</span>
  <strong>{node.id}</strong>
  <span class="node-state">{node.state ? `State::${node.state}` : node.structural ? 'No state shown' : 'Signal source'}</span>
</article>
