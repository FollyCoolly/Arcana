<script lang="ts">
  import type { DimensionData } from "$lib/types/status";

  let {
    dimensions,
    onSelect,
  }: {
    dimensions: DimensionData[];
    onSelect?: (id: string) => void;
  } = $props();

  const size = 700;
  const cx = size / 2;
  const cy = size / 2;
  const outerR = 220;
  const innerR = outerR * 0.38;
  const labelR = outerR + 80;
  const maxLevel = 5;

  // Star tilted slightly (like P5)
  const tiltOffset = -Math.PI / 2 + 0.05;

  function outerAngle(i: number): number {
    return (Math.PI * 2 * i) / 5 + tiltOffset;
  }

  function innerAngle(i: number): number {
    return outerAngle(i) + Math.PI / 5;
  }

  function outerXY(i: number, r: number): [number, number] {
    const a = outerAngle(i);
    return [cx + r * Math.cos(a), cy + r * Math.sin(a)];
  }

  function innerXY(i: number, r: number): [number, number] {
    const a = innerAngle(i);
    return [cx + r * Math.cos(a), cy + r * Math.sin(a)];
  }

  /** 10-point star polygon */
  function starPoints(oR: number, iR: number): string {
    const pts: string[] = [];
    for (let i = 0; i < 5; i++) {
      pts.push(outerXY(i, oR).join(","));
      pts.push(innerXY(i, iR).join(","));
    }
    return pts.join(" ");
  }

  /**
   * Level-based normalization: maps level to a fixed visual proportion.
   * Lv.1=0.2, Lv.2=0.4, Lv.3=0.6, Lv.4=0.8, Lv.5=1.0
   * This ensures Lv.4 is always visually distinct from Lv.5.
   */
  function normalizedScore(dim: DimensionData): number {
    if (dim.level === null || dim.level === undefined) return 0;
    return Math.min(dim.level / maxLevel, 1.0);
  }

  /**
   * Data star polygon: outer tips scale per dimension, inner valleys FIXED.
   * This preserves the star shape at all scores.
   */
  function dataStarPoints(): string {
    const pts: string[] = [];
    const fixedInnerR = innerR * 0.35;
    for (let i = 0; i < 5; i++) {
      const dim = dimensions[i];
      const ns = dim ? normalizedScore(dim) : 0;
      const tipR = outerR * Math.max(ns, 0.08);
      pts.push(outerXY(i, tipR).join(","));
      pts.push(innerXY(i, fixedInnerR).join(","));
    }
    return pts.join(" ");
  }

  function handleClick(id: string) {
    onSelect?.(id);
  }
</script>

<div class="star-wrapper">
  <svg
    viewBox="0 0 {size} {size}"
    class="star-svg"
    xmlns="http://www.w3.org/2000/svg"
  >
    <!-- Background grid: concentric stars (outer to inner) -->
    {#each Array.from({ length: maxLevel }, (_, i) => maxLevel - i) as level}
      {@const oR = (outerR * level) / maxLevel}
      {@const iR = (innerR * level) / maxLevel}
      <polygon
        points={starPoints(oR, iR)}
        fill={level % 2 === maxLevel % 2 ? "#000" : "#0d0d0d"}
        stroke="#1a1a1a"
        stroke-width="0.8"
      />
    {/each}

    <!-- Spoke lines from center to each outer star tip -->
    {#each Array.from({ length: 5 }) as _, i}
      {@const [ox, oy] = outerXY(i, outerR)}
      <line x1={cx} y1={cy} x2={ox} y2={oy} stroke="#1a1a1a" stroke-width="0.8" />
    {/each}

    <!-- Data star: gold filled shape -->
    <polygon
      points={dataStarPoints()}
      fill="rgba(245, 166, 35, 0.45)"
      stroke="#F5A623"
      stroke-width="2.5"
      stroke-linejoin="round"
    />

    <!-- Vertex dots at outer tips -->
    {#each dimensions as dim, i}
      {@const ns = normalizedScore(dim)}
      {@const tipR = outerR * Math.max(ns, 0.08)}
      {@const [vx, vy] = outerXY(i, tipR)}
      <circle cx={vx} cy={vy} r="5" fill="#F5A623" />
    {/each}
  </svg>

  <!-- Dimension labels at each star tip -->
  {#each dimensions as dim, i}
    {@const [lx, ly] = outerXY(i, labelR)}
    <button
      type="button"
      class="star-label"
      style:left="{(lx / size) * 100}%"
      style:top="{(ly / size) * 100}%"
      onclick={() => handleClick(dim.id)}
    >
      <span class="star-label-name-row">
        <span class="star-label-name">{dim.name}</span>
        {#if dim.level !== null}
          <span class="star-label-level">{dim.level}</span>
        {/if}
      </span>
      <span class="star-label-title">
        {dim.level_title ?? "--"}
      </span>
    </button>
  {/each}
</div>

<style>
  .star-wrapper {
    position: relative;
    width: 100%;
    max-width: min(78vh, 880px);
    aspect-ratio: 1;
    margin: 0 auto;
  }

  .star-svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .star-label {
    position: absolute;
    transform: translate(-50%, -50%);
    background: none;
    border: none;
    padding: clamp(0.3rem, 0.4vw, 0.6rem) clamp(0.5rem, 0.7vw, 1rem);
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.2em;
    transition: transform 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
    white-space: nowrap;
    z-index: 2;
  }

  .star-label:hover {
    transform: translate(-50%, -50%) scale(1.08);
  }

  .star-label-name-row {
    display: flex;
    align-items: center;
    gap: 0.3em;
  }

  .star-label-name {
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(1.5rem, 2vw, 2.8rem);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--rm-white);
    -webkit-text-stroke: 0.05em var(--rm-black);
    paint-order: stroke fill;
    line-height: 1;
  }

  .star-label-level {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: #F5A623;
    color: var(--rm-black);
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(1.1rem, 1.4vw, 2rem);
    font-weight: 800;
    min-width: 1.4em;
    padding: 0.05em 0.2em;
    line-height: 1;
    transform: rotate(-3deg);
  }

  .star-label-title {
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(0.9rem, 1.15vw, 1.6rem);
    font-weight: 700;
    color: rgba(255, 255, 255, 0.75);
    -webkit-text-stroke: 0.03em var(--rm-black);
    paint-order: stroke fill;
    line-height: 1;
  }
</style>
