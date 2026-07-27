<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { AnimationPhase, AnimationTrace } from './types';
  import FrameTree from './FrameTree.svelte';
  import SignalArrow from './SignalArrow.svelte';
  import { signalGeometry, type ArrowGeometry } from './geometry';
  import { shouldRenderSignalArrow } from './visibility';

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
  const moment = $derived(shownIndex < 0 ? null : animation.moments[shownIndex]);
  const transitioning = $derived(activeIndex !== null);
  const canPrevious = $derived(position >= 0);
  const canNext = $derived(position < animation.frames.length - 1 && !transitioning);
  const showBoundary = $derived(
    !transitioning && position === animation.frames.length - 1
      ? animation.trace.boundary
      : null
  );

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
    if (!stageElement || !moment || moment.kind !== 'request' || phase !== 'request') {
      arrowGeometry = null;
      return;
    }
    const source = endpoint(moment.source);
    const target = endpoint(moment.target);
    if (!source || !target) {
      arrowGeometry = null;
      return;
    }
    if (!shouldRenderSignalArrow(source, target)) {
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
    if (activeIndex !== null && moment?.kind === 'request' && phase === 'request') {
      arrowFrame = window.requestAnimationFrame(trackArrow);
    }
  }

  function scheduleArrowUpdate() {
    if (arrowFrame === null && activeIndex !== null) {
      arrowFrame = window.requestAnimationFrame(trackArrow);
    }
  }

  function revealTarget() {
    if (!stageElement || !moment) return;
    const target = endpoint(moment.target);
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
    const nextIndex = position + 1;
    activeIndex = nextIndex;
    const activeMoment = animation.moments[nextIndex];
    phase = activeMoment.kind === 'request'
      ? 'request'
      : activeMoment.kind === 'feedback'
        ? 'before'
        : activeMoment.kind;
    await tick();
    if (!restoreFrameScroll(activeIndex)) {
      restoreBottomAnchor(snapshot);
      revealTarget();
    }
    if (activeMoment.kind === 'request') {
      updateArrow();
      scheduleArrowUpdate();
      await pause(480);
    } else if (activeMoment.kind === 'feedback') {
      await pause(300);
      phase = 'feedback';
      await tick();
      await pause(420);
    } else {
      await pause(420);
    }
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
    if (!moment) return '初始确定帧：尚未到达因果时刻。';
    if (moment.kind === 'request') {
      return `请求到达：${moment.delivery} · event ${moment.event_sequence}`;
    }
    if (moment.kind === 'terminal') return `终止：${moment.outcome}，状态保持不变。`;
    const prefix = moment.kind === 'settle' ? '目标内部处理完成' : '反馈';
    if (moment.handler.kind === 'Transition') {
      if (moment.response.before_state === null && moment.response.after_state === null) {
        return `${prefix}：无 lifecycle state 的 Transition 已完成。`;
      }
      return `${prefix} Transition：${moment.response.before_state} → ${moment.response.after_state}`;
    }
    if (moment.handler.kind === 'Action') return `${prefix} Action：不改变 lifecycle state。`;
    return `${prefix}：目标未解析到 handler；保留输入中的确定结果。`;
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
      <div><dt>Signals:</dt><dd>{animation.trace.total_signals}</dd></div>
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
    <FrameTree {frame} activeMoment={transitioning ? moment : null} {phase} {reducedMotion} />
    {#if arrowGeometry && moment && moment.kind === 'request' && phase === 'request'}
      <SignalArrow
        geometry={arrowGeometry}
        signal={moment.signal}
        outcome="completed"
        width={overlayWidth}
        height={overlayHeight}
      />
    {/if}
  </div>
  <section class="transport" aria-label="Signal navigation">
    <button type="button" onclick={previous} disabled={!canPrevious}>上一步</button>
    <div class="step-copy">
      <p class="counter">时刻 {shownIndex + 1} / {animation.trace.total_moments}</p>
      {#if moment}
        <h2>{moment.transfer?.from || moment.source} <span>— {moment.signal} →</span> {moment.transfer?.to || moment.target}</h2>
        <p>{responseText()}</p>
        {#if moment.kind !== 'request' && moment.outcome !== 'completed'}
          <p class="reason"><strong>{moment.outcome}</strong>{moment.reason ? ` · ${moment.reason}` : ''}</p>
        {/if}
      {:else}
        <h2>初始帧</h2>
        <p>{responseText()}</p>
      {/if}
      {#if showBoundary}
        <p class="boundary">
          边界：{showBoundary.source} — {showBoundary.signal} → {showBoundary.target}（发送前）
        </p>
      {/if}
    </div>
    <button type="button" onclick={next} disabled={!canNext}>下一步</button>
  </section>
</section>
