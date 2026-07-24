<script lang="ts">
  import type { ArrowGeometry } from './geometry';

  let {
    geometry,
    signal,
    outcome,
    width,
    height
  }: {
    geometry: ArrowGeometry;
    signal: string;
    outcome: string;
    width: number;
    height: number;
  } = $props();
  const exceptional = $derived(outcome !== 'completed');
</script>

<svg
  class:error={exceptional}
  class:self-loop={geometry.self}
  class="signal-overlay"
  data-arrow-kind={geometry.self ? 'self' : 'ordinary'}
  data-arrow-direction={geometry.direction}
  data-outcome={outcome}
  viewBox={`0 0 ${width} ${height}`}
  {width}
  {height}
  aria-label={`${signal} Signal${exceptional ? `, ${outcome}` : ''}`}
>
  <defs>
    <marker id="signal-arrowhead" markerWidth="10" markerHeight="8" refX="10" refY="4" orient="auto" markerUnits="strokeWidth">
      <path d="M 0 0 L 10 4 L 0 8 z" />
    </marker>
    <marker id="signal-arrowhead-error" markerWidth="10" markerHeight="8" refX="10" refY="4" orient="auto" markerUnits="strokeWidth">
      <path d="M 0 0 L 10 4 L 0 8 z" />
    </marker>
  </defs>
  <path class="signal-path" d={geometry.path} marker-end={exceptional ? 'url(#signal-arrowhead-error)' : 'url(#signal-arrowhead)'} />
  <text x={geometry.labelX} y={geometry.labelY} text-anchor="middle">{signal}</text>
</svg>
