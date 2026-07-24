<script lang="ts">
  import { onMount } from 'svelte';
  import type { AnimationTrace } from './types';
  import FrameTree from './FrameTree.svelte';

  let { animation }: { animation: AnimationTrace } = $props();
  const request = $derived(animation.trace.root_request);
  let position = $state(-1);
  const frame = $derived(position < 0 ? animation.initial_frame : animation.frames[position]);
  const step = $derived(position < 0 ? null : animation.steps[position]);
  const canPrevious = $derived(position >= 0);
  const canNext = $derived(position < animation.frames.length - 1);

  function previous() {
    if (canPrevious) position -= 1;
  }

  function next() {
    if (canNext) position += 1;
  }

  function responseText() {
    if (!step) return '初始确定帧：尚未发送 Signal。';
    if (step.handler.kind === 'Transition') {
      return `Transition：State::${step.response.before_state} → State::${step.response.after_state}`;
    }
    if (step.handler.kind === 'Action') return 'Action：响应完成，不改变 lifecycle state。';
    return '目标未解析到 handler；保留输入中的确定结果。';
  }

  onMount(() => {
    const onKeydown = (event: KeyboardEvent) => {
      if (event.key === 'ArrowLeft') {
        event.preventDefault();
        previous();
      } else if (event.key === 'ArrowRight') {
        event.preventDefault();
        next();
      }
    };
    window.addEventListener('keydown', onKeydown);
    return () => window.removeEventListener('keydown', onKeydown);
  });
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
  <div class="stage" aria-label="Signal animation canvas" aria-live="polite">
    <FrameTree {frame} />
  </div>
  <section class="transport" aria-label="Signal navigation">
    <button type="button" onclick={previous} disabled={!canPrevious}>上一步</button>
    <div class="step-copy">
      <p class="counter">步骤 {position + 1} / {animation.trace.total_steps}</p>
      {#if step}
        <h2>{step.source} <span>— {step.signal} →</span> {step.target}</h2>
        <p>{responseText()}</p>
        {#if step.outcome !== 'completed'}
          <p class="reason"><strong>{step.outcome}</strong>{step.reason ? ` · ${step.reason}` : ''}</p>
        {/if}
      {:else}
        <h2>初始帧</h2>
        <p>{responseText()}</p>
      {/if}
    </div>
    <button type="button" onclick={next} disabled={!canNext}>下一步</button>
  </section>
  <footer title={animation.inputs.model_fingerprint}>{animation.source}</footer>
</section>
