<script lang="ts">
  import type { AnimationTrace } from './types';
  import NodeCard from './NodeCard.svelte';

  let { animation }: { animation: AnimationTrace } = $props();
  const request = $derived(animation.trace.root_request);
</script>

<svelte:head>
  <meta name="color-scheme" content="light dark" />
</svelte:head>

<section class="player-shell" aria-labelledby="trace-title">
  <header>
    <p class="eyebrow">tools2 · deterministic Signal trace</p>
    <h1 id="trace-title">{request.target}.{request.signal}</h1>
    <p class="lede">This first determined frame contains only the initial Signal source and the ancestors required to preserve structure.</p>
  </header>
  <dl class="trace-meta">
    <div><dt>Source</dt><dd>{request.source}</dd></div>
    <div><dt>Verdict</dt><dd>{animation.trace.verdict}</dd></div>
    <div><dt>Signals</dt><dd>{animation.trace.total_steps}</dd></div>
    <div><dt>Protocol</dt><dd>{animation.schema} v{animation.version}</dd></div>
  </dl>
  <div class="stage" aria-label="Signal animation canvas">
    <div class="initial-frame" data-frame-index={animation.initial_frame.index}>
      {#each animation.initial_frame.nodes as node (node.id)}
        <NodeCard {node} />
      {/each}
    </div>
  </div>
  <footer title={animation.inputs.model_fingerprint}>{animation.source}</footer>
</section>
