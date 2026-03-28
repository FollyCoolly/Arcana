<script lang="ts">
  type Dimension = { key: string; label: string; level: number; score: number };

  let {
    dimensions,
    maxLevel = 5,
    selectedKey = $bindable<string | null>(null),
    onSelect,
  }: {
    dimensions: Dimension[];
    maxLevel?: number;
    selectedKey?: string | null;
    onSelect?: (key: string) => void;
  } = $props();

  const size = 600;
  const cx = size / 2;
  const cy = size / 2;
  const outerR = 210;
  const labelR = outerR + 58;

  function angle(i: number): number {
    return (Math.PI * 2 * i) / dimensions.length - Math.PI / 2;
  }

  function vertexXY(i: number, r: number): [number, number] {
    const a = angle(i);
    return [cx + r * Math.cos(a), cy + r * Math.sin(a)];
  }

  function polyPoints(r: number): string {
    return dimensions.map((_, i) => vertexXY(i, r).join(",")).join(" ");
  }

  function dataPoints(): string {
    return dimensions
      .map((d, i) => {
        const r = outerR * Math.min(d.score, 1);
        return vertexXY(i, r).join(",");
      })
      .join(" ");
  }

  /** Skewed label badge box path (parallelogram) centered around (cx, cy). */
  function labelBadge(i: number): { x: number; y: number; rot: number } {
    const [lx, ly] = vertexXY(i, labelR);
    const a = angle(i);
    const deg = (a * 180) / Math.PI;
    return { x: lx, y: ly, rot: deg > 90 || deg < -90 ? deg + 180 : deg };
  }

  function handleClick(key: string) {
    selectedKey = key;
    onSelect?.(key);
  }
</script>

<div class="radar-wrapper">
  <svg
    viewBox="0 0 {size} {size}"
    class="radar-svg"
    xmlns="http://www.w3.org/2000/svg"
  >
    <!-- Grid bands: black/gray alternating filled octagons, outer-to-inner -->
    {#each Array.from({ length: maxLevel }, (_, i) => maxLevel - i) as level}
      <polygon
        points={polyPoints((outerR * level) / maxLevel)}
        fill={level % 2 === maxLevel % 2 ? "#000" : "#1a1a1a"}
        stroke="none"
      />
    {/each}

    <!-- Data triangles: alternating two yellows -->
    {#each dimensions as d, i}
      {@const r0 = outerR * Math.min(d.score, 1)}
      {@const r1 = outerR * Math.min(dimensions[(i + 1) % dimensions.length].score, 1)}
      {@const [x0, y0] = vertexXY(i, r0)}
      {@const [x1, y1] = vertexXY((i + 1) % dimensions.length, r1)}
      <polygon
        points="{cx},{cy} {x0},{y0} {x1},{y1}"
        fill={i % 2 === 0 ? "rgba(245, 166, 35, 0.55)" : "rgba(210, 140, 25, 0.55)"}
        stroke="none"
      />
    {/each}

    <!-- Data vertex dots -->
    {#each dimensions as d, i}
      {@const r = outerR * Math.min(d.score, 1)}
      {@const [vx, vy] = vertexXY(i, r)}
      <circle cx={vx} cy={vy} r="5" fill="#F5A623" />
    {/each}
  </svg>

  <!-- HTML labels overlaid on top - P5-style badges -->
  {#each dimensions as d, i}
    {@const badge = labelBadge(i)}
    <button
      type="button"
      class="radar-badge"
      style:left="{(badge.x / size) * 100}%"
      style:top="{(badge.y / size) * 100}%"
      onclick={() => handleClick(d.key)}
    >
      <span class="radar-badge-label">{d.label}</span>
      <span class="radar-badge-level">Lv{d.level}</span>
    </button>
  {/each}
</div>

<style>
  .radar-wrapper {
    position: relative;
    width: 100%;
    max-width: min(85vh, 1120px);
    aspect-ratio: 1;
    margin: 0 auto;
  }

  .radar-svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .radar-badge {
    position: absolute;
    transform: translate(-50%, -50%);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: baseline;
    gap: 0.35em;
    transition: transform 120ms ease;
    white-space: nowrap;
  }

  .radar-badge:hover {
    transform: translate(-50%, -50%) scale(1.08);
  }

  .radar-badge-label {
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(2.1rem, 2.55vw, 3.45rem);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    color: var(--rm-white);
    -webkit-text-stroke: 0.06em var(--rm-black);
    paint-order: stroke fill;
    line-height: 1;
  }

  .radar-badge-level {
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(1.1rem, 1.3vw, 1.8rem);
    font-weight: 700;
    color: rgba(255, 255, 255, 0.5);
    -webkit-text-stroke: 0.06em var(--rm-black);
    paint-order: stroke fill;
    line-height: 1;
  }
</style>
