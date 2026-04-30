<script lang="ts">
    import type { DimensionData } from "$lib/types/status";
    import CollageLabel from "$lib/CollageLabel.svelte";

    let {
        dimensions,
        onSelect,
    }: {
        dimensions: DimensionData[];
        onSelect?: (id: string) => void;
    } = $props();

    const size = 700;
    const cx = size * 0.95;
    const cy = size * 0.62;
    const outerR = 550;
    const innerR = outerR * 0.42;
    const labelR = outerR + 160;
    const maxLevel = 5;

    // Background decorative star scale (larger than the grid)
    const bgStarScale = 2.0;
    // Minimum scale for the innermost grid star (so it's not tiny)
    const gridMinScale = 0.35;

    // Star tilted slightly (like P5) + 10° clockwise
    const tiltOffset = -Math.PI / 2 + 0.05 + (3 * Math.PI) / 180;

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

    /** Map a normalized score (0–1) to a grid-aligned scale factor */
    function gridT(ns: number): number {
        return gridMinScale + (1 - gridMinScale) * ns;
    }

    /**
     * Data star polygon: outer tips and inner valleys grid-aligned.
     * Inner valley between tip i and tip i+1 uses the lower of the two
     * adjacent levels so the star never exceeds what the data supports.
     */
    function dataStarPoints(): string {
        const pts: string[] = [];
        for (let i = 0; i < 5; i++) {
            const dim = dimensions[i];
            const ns = dim ? normalizedScore(dim) : 0;
            const tipR = outerR * gridT(ns);
            pts.push(outerXY(i, tipR).join(","));

            const nextDim = dimensions[(i + 1) % 5];
            const nextNs = nextDim ? normalizedScore(nextDim) : 0;
            const valleyR = innerR * gridT(Math.min(ns, nextNs));
            pts.push(innerXY(i, valleyR).join(","));
        }
        return pts.join(" ");
    }

    /**
     * Split the data star into 10 triangles from center to adjacent vertices.
     * Each spoke (center → outer tip) divides the tip into two halves:
     *   - left of spoke  → #E7AE16 (darker gold)
     *   - right of spoke → #FCCC2C (lighter gold)
     * Vertex order is: outer0, inner0, outer1, inner1, …
     * Even-index triangles (center, outer_i, inner_i) = right of spoke → #FCCC2C
     * Odd-index triangles  (center, inner_i, outer_{i+1}) = left of spoke → #E7AE16
     */
    function dataStarTriangles(): Array<{ points: string; fill: string }> {
        const vertices: [number, number][] = [];
        for (let i = 0; i < 5; i++) {
            const dim = dimensions[i];
            const ns = dim ? normalizedScore(dim) : 0;
            const tipR = outerR * gridT(ns);
            vertices.push(outerXY(i, tipR));

            const nextDim = dimensions[(i + 1) % 5];
            const nextNs = nextDim ? normalizedScore(nextDim) : 0;
            const valleyR = innerR * gridT(Math.min(ns, nextNs));
            vertices.push(innerXY(i, valleyR));
        }
        const triangles: Array<{ points: string; fill: string }> = [];
        for (let j = 0; j < 10; j++) {
            const v1 = vertices[j];
            const v2 = vertices[(j + 1) % 10];
            const fill = j % 2 === 0 ? "#FCCC2C" : "#E7AE16";
            triangles.push({
                points: `${cx},${cy} ${v1.join(",")} ${v2.join(",")}`,
                fill,
            });
        }
        return triangles;
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
        overflow="visible"
    >
        <!-- Large gray background star (decorative backdrop) -->
        <polygon
            points={starPoints(outerR * bgStarScale, innerR * bgStarScale)}
            fill="#2e2e2e"
            stroke="none"
            transform="rotate(3, {cx}, {cy})"
        />

        <!-- Background grid: concentric stars (outer to inner) -->
        {#each Array.from({ length: maxLevel }, (_, i) => maxLevel - i) as level}
            {@const t = gridMinScale + (1 - gridMinScale) * (level / maxLevel)}
            {@const oR = outerR * t}
            {@const iR = innerR * t}
            <polygon
                points={starPoints(oR, iR)}
                fill={level % 2 === maxLevel % 2 ? "#000" : "#2d2d2d"}
                stroke="#1a1a1a"
                stroke-width="0.8"
            />
        {/each}

        <!-- Dots at the 5 outer tip vertices of each concentric grid star -->
        {#each Array.from({ length: maxLevel }, (_, i) => maxLevel - i) as level}
            {@const t = gridMinScale + (1 - gridMinScale) * (level / maxLevel)}
            {@const oR = outerR * t}
            {#each Array.from({ length: 5 }) as _, i}
                {@const [px, py] = outerXY(i, oR)}
                <circle cx={px} cy={py} r="4" fill="#555555" />
            {/each}
        {/each}

        <!-- Spoke lines from center to each outer star tip -->
        {#each Array.from({ length: 5 }) as _, i}
            {@const [ox, oy] = outerXY(i, outerR)}
            <line
                x1={cx}
                y1={cy}
                x2={ox}
                y2={oy}
                stroke="#1a1a1a"
                stroke-width="0.8"
            />
        {/each}

        <!-- Data star: solid two-tone fill (split by spoke lines) -->
        {#each dataStarTriangles() as tri}
            <polygon points={tri.points} fill={tri.fill} stroke="none" />
        {/each}
        <!-- Data star outline -->
        <polygon
            points={dataStarPoints()}
            fill="none"
            stroke="#F5A623"
            stroke-width="2.5"
            stroke-linejoin="round"
        />
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
            <CollageLabel
                text={dim.name}
                level={dim.level !== null &&
                dim.level > dim.level_thresholds.length
                    ? "Max"
                    : dim.level}
                title={dim.level_title ?? "--"}
            />
        </button>
    {/each}
</div>

<style>
    .star-wrapper {
        position: relative;
        width: 100%;
        max-width: 55rem;
        aspect-ratio: 1;
        margin: 0 auto;
        overflow: visible;
    }

    :global(:root[data-platform="macos"]) .star-wrapper {
        max-width: min(78vh, 880px);
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
        padding: 0;
        cursor: pointer;
        font-size: clamp(3rem, 4vw, 5.6rem);
        transition: transform 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
        z-index: 2;
    }

    .star-label:hover {
        transform: translate(-50%, -50%) scale(1.08);
    }
</style>
