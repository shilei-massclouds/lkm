<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { AnimationPhase, AnimationTrace } from './types';
  import FrameTree from './FrameTree.svelte';
  import SignalArrow from './SignalArrow.svelte';
  import { signalGeometry, type ArrowGeometry } from './geometry';

  let { animation }: { animation: AnimationTrace } = $props();
  const request = $derived(animation.trace.root_request);
  let position = $state(-1);
  let activeIndex = $state<number | null>(null);
  let phase = $state<AnimationPhase>('idle');
  let reducedMotion = $state(typeof window === 'undefined' || typeof window.matchMedia !== 'function');
  let stageElement = $state<HTMLElement | null>(null);
  let arrowGeometry = $state<ArrowGeometry | null>(null);
  let overlayWidth = $state(1);
  let overlayHeight = $state(1);
  const shownIndex = $derived(activeIndex ?? position);
  const frame = $derived(shownIndex < 0 ? animation.initial_frame : animation.frames[shownIndex]);
  const step = $derived(shownIndex < 0 ? null : animation.steps[shownIndex]);
  const transitioning = $derived(activeIndex !== null);
  const canPrevious = $derived(position >= 0);
  const canNext = $derived(position < animation.frames.length - 1 && !transitioning);

  function previous() {
    if (canPrevious && !transitioning) {
      position -= 1;
      arrowGeometry = null;
    }
  }

  function pause(milliseconds: number) {
    return reducedMotion ? Promise.resolve() : new Promise((resolve) => window.setTimeout(resolve, milliseconds));
  }

  function endpoint(id: string) {
    return Array.from(stageElement?.querySelectorAll<HTMLElement>('[data-node-id]') || [])
      .find((element) => element.dataset.nodeId === id) || null;
  }

  function updateArrow() {
    if (!stageElement || !step) {
      arrowGeometry = null;
      return;
    }
    const source = endpoint(step.source);
    const target = endpoint(step.target);
    if (!source || !target) {
      arrowGeometry = null;
      return;
    }
    const stageRect = stageElement.getBoundingClientRect();
    arrowGeometry = signalGeometry(
      source.getBoundingClientRect(),
      target.getBoundingClientRect(),
      stageRect,
      stageElement.scrollLeft,
      stageElement.scrollTop
    );
    overlayWidth = Math.max(stageElement.scrollWidth, stageElement.clientWidth, 1);
    overlayHeight = Math.max(stageElement.scrollHeight, stageElement.clientHeight, 1);
  }

  function revealTarget() {
    if (!stageElement || !step) return;
    const target = endpoint(step.target);
    if (!target || typeof stageElement.scrollTo !== 'function') return;
    stageElement.scrollTo({
      left: Math.max(0, target.offsetLeft + target.offsetWidth / 2 - stageElement.clientWidth / 2),
      top: Math.max(0, target.offsetTop + target.offsetHeight / 2 - stageElement.clientHeight / 2),
      behavior: reducedMotion ? 'auto' : 'smooth'
    });
  }

  async function next() {
    if (!canNext) return;
    activeIndex = position + 1;
    phase = 'send';
    await tick();
    revealTarget();
    updateArrow();
    await pause(300);
    phase = 'response';
    await pause(420);
    phase = 'clear';
    arrowGeometry = null;
    await pause(180);
    position = activeIndex;
    activeIndex = null;
    phase = 'idle';
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
    const media = typeof window.matchMedia === 'function'
      ? window.matchMedia('(prefers-reduced-motion: reduce)')
      : null;
    const setMotion = () => { reducedMotion = media?.matches ?? true; };
    setMotion();
    const onKeydown = (event: KeyboardEvent) => {
      if (event.key === 'ArrowLeft') {
        event.preventDefault();
        previous();
      } else if (event.key === 'ArrowRight') {
        event.preventDefault();
        next();
      }
    };
    const onResize = () => updateArrow();
    window.addEventListener('keydown', onKeydown);
    window.addEventListener('resize', onResize);
    media?.addEventListener('change', setMotion);
    return () => {
      window.removeEventListener('keydown', onKeydown);
      window.removeEventListener('resize', onResize);
      media?.removeEventListener('change', setMotion);
    };
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
  <div
    class="stage"
    class:reduced-motion={reducedMotion}
    aria-label="Signal animation canvas"
    aria-live="polite"
    data-animation-phase={phase}
    bind:this={stageElement}
  >
    <FrameTree {frame} activeStep={transitioning ? step : null} {phase} {reducedMotion} />
    {#if arrowGeometry && step && (phase === 'send' || phase === 'response')}
      <SignalArrow
        geometry={arrowGeometry}
        signal={step.signal}
        outcome={step.outcome}
        width={overlayWidth}
        height={overlayHeight}
      />
    {/if}
  </div>
  <section class="transport" aria-label="Signal navigation">
    <button type="button" onclick={previous} disabled={!canPrevious}>上一步</button>
    <div class="step-copy">
      <p class="counter">步骤 {shownIndex + 1} / {animation.trace.total_steps}</p>
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
