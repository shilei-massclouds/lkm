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
  let arrowFrame: number | null = null;
  const frameScroll = new Map<number, { left: number; top: number }>();
  const shownIndex = $derived(activeIndex ?? position);
  const frame = $derived(shownIndex < 0 ? animation.initial_frame : animation.frames[shownIndex]);
  const step = $derived(shownIndex < 0 ? null : animation.steps[shownIndex]);
  const transitioning = $derived(activeIndex !== null);
  const canPrevious = $derived(position >= 0);
  const canNext = $derived(position < animation.frames.length - 1 && !transitioning);

  function scrollSnapshot() {
    return stageElement
      ? { height: stageElement.scrollHeight, top: stageElement.scrollTop }
      : null;
  }

  function restoreBottomAnchor(snapshot: { height: number; top: number } | null) {
    if (!stageElement || !snapshot) return;
    const maximum = Math.max(0, stageElement.scrollHeight - stageElement.clientHeight);
    stageElement.scrollTop = Math.min(
      maximum,
      Math.max(0, snapshot.top + stageElement.scrollHeight - snapshot.height)
    );
  }

  function rememberScroll(index: number) {
    if (stageElement) {
      frameScroll.set(index, { left: stageElement.scrollLeft, top: stageElement.scrollTop });
    }
  }

  function restoreFrameScroll(index: number) {
    if (!stageElement) return false;
    const saved = frameScroll.get(index);
    if (!saved) return false;
    stageElement.scrollLeft = Math.min(
      Math.max(0, stageElement.scrollWidth - stageElement.clientWidth),
      saved.left
    );
    stageElement.scrollTop = Math.min(
      Math.max(0, stageElement.scrollHeight - stageElement.clientHeight),
      saved.top
    );
    return true;
  }

  async function previous() {
    if (canPrevious && !transitioning) {
      const snapshot = scrollSnapshot();
      rememberScroll(position);
      position -= 1;
      arrowGeometry = null;
      await tick();
      if (!restoreFrameScroll(position)) restoreBottomAnchor(snapshot);
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
    if (!stageElement || !step || (phase !== 'send' && phase !== 'response')) {
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
      stageElement.scrollTop,
      source === target
    );
    overlayWidth = Math.max(stageElement.scrollWidth, stageElement.clientWidth, 1);
    overlayHeight = Math.max(stageElement.scrollHeight, stageElement.clientHeight, 1);
  }

  function trackArrow() {
    arrowFrame = null;
    updateArrow();
    if (activeIndex !== null && (phase === 'send' || phase === 'response')) {
      arrowFrame = window.requestAnimationFrame(trackArrow);
    }
  }

  function scheduleArrowUpdate() {
    if (arrowFrame === null && activeIndex !== null) {
      arrowFrame = window.requestAnimationFrame(trackArrow);
    }
  }

  function revealTarget() {
    if (!stageElement || !step) return;
    const target = endpoint(step.target);
    if (!target || typeof stageElement.scrollTo !== 'function') return;
    const targetRect = target.getBoundingClientRect();
    const stageRect = stageElement.getBoundingClientRect();
    stageElement.scrollTo({
      left: Math.max(
        0,
        stageElement.scrollLeft + targetRect.left - stageRect.left +
          targetRect.width / 2 - stageElement.clientWidth / 2
      ),
      top: Math.max(
        0,
        stageElement.scrollTop + targetRect.top - stageRect.top +
          targetRect.height / 2 - stageElement.clientHeight / 2
      ),
      behavior: reducedMotion ? 'auto' : 'smooth'
    });
  }

  async function next() {
    if (!canNext) return;
    const snapshot = scrollSnapshot();
    rememberScroll(position);
    activeIndex = position + 1;
    phase = 'send';
    await tick();
    if (!restoreFrameScroll(activeIndex)) {
      restoreBottomAnchor(snapshot);
      revealTarget();
    }
    updateArrow();
    scheduleArrowUpdate();
    await pause(300);
    phase = 'response';
    await pause(420);
    phase = 'clear';
    arrowGeometry = null;
    await pause(180);
    position = activeIndex;
    activeIndex = null;
    phase = 'idle';
    rememberScroll(position);
    if (arrowFrame !== null) {
      window.cancelAnimationFrame(arrowFrame);
      arrowFrame = null;
    }
  }

  function responseText() {
    if (!step) return '初始确定帧：尚未发送 Signal。';
    if (step.handler.kind === 'Transition') {
      if (step.response.before_state === null && step.response.after_state === null) {
        return 'Transition：无 lifecycle state 的响应已完成。';
      }
      return `Transition：${step.response.before_state} → ${step.response.after_state}`;
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
    const onStageScroll = () => {
      scheduleArrowUpdate();
      if (activeIndex === null && phase === 'idle') rememberScroll(position);
    };
    const observer = typeof ResizeObserver === 'function'
      ? new ResizeObserver(scheduleArrowUpdate)
      : null;
    if (stageElement) {
      stageElement.scrollTop = Math.max(0, stageElement.scrollHeight - stageElement.clientHeight);
      stageElement.scrollLeft = 0;
      rememberScroll(-1);
    }
    window.addEventListener('keydown', onKeydown);
    window.addEventListener('resize', onResize);
    stageElement?.addEventListener('scroll', onStageScroll, { passive: true });
    if (stageElement) observer?.observe(stageElement);
    media?.addEventListener('change', setMotion);
    return () => {
      window.removeEventListener('keydown', onKeydown);
      window.removeEventListener('resize', onResize);
      stageElement?.removeEventListener('scroll', onStageScroll);
      observer?.disconnect();
      if (arrowFrame !== null) window.cancelAnimationFrame(arrowFrame);
      media?.removeEventListener('change', setMotion);
    };
  });
</script>

<svelte:head>
  <meta name="color-scheme" content="light dark" />
</svelte:head>

<section class="player-shell" aria-labelledby="trace-title">
  <header class="trace-header">
    <div class="trace-heading">
      <p class="eyebrow">tools2 · Signal trace</p>
      <h1 id="trace-title">{request.target}.{request.signal}</h1>
    </div>
    <dl class="trace-meta">
      <div><dt>Source:</dt><dd>{request.source}</dd></div>
      <div><dt>Verdict:</dt><dd>{animation.trace.verdict}</dd></div>
      <div><dt>Signals:</dt><dd>{animation.trace.total_steps}</dd></div>
      <div title={`Source file: ${animation.source}\nModel fingerprint: ${animation.inputs.model_fingerprint}`}>
        <dt>Protocol:</dt><dd>{animation.schema} v{animation.version}</dd>
      </div>
    </dl>
  </header>
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
</section>
