<script lang="ts">
    import { onMount } from "svelte";
    import CollageLabel from "$lib/CollageLabel.svelte";
    import KeyHint from "$lib/KeyHint.svelte";
    import PromptWord from "$lib/PromptWord.svelte";
    import type {
        StatusData,
        DimensionData,
        StatusScoreData,
    } from "$lib/types/status";

    let {
        statusData,
        selectedDimensionId,
        onBack,
    }: {
        statusData: StatusData;
        selectedDimensionId: string | null;
        onBack: () => void;
    } = $props();

    let activeDimensionId = $state<string | "all">(
        selectedDimensionId ?? "all",
    );

    let selectedDimensions = $derived.by<DimensionData[]>(() =>
        statusData.dimensions
            .filter((dimension) => dimension.selected_position !== undefined)
            .sort(
                (left, right) =>
                    (left.selected_position ?? 0) -
                    (right.selected_position ?? 0),
            ),
    );

    let activeDimension = $derived<DimensionData | null>(
        activeDimensionId === "all"
            ? null
            : (selectedDimensions.find((d) => d.id === activeDimensionId) ??
                  null),
    );

    type ScoreGroup = { name: string; scores: StatusScoreData[] };

    let scoreGroups = $derived.by<ScoreGroup[]>(() => {
        if (activeDimension) {
            return [{ name: activeDimension.name, scores: activeDimension.scores }];
        }
        return selectedDimensions.map((dimension) => ({
            name: dimension.name,
            scores: dimension.scores,
        }));
    });

    /** Ordered list of navigable tab IDs */
    let tabIds = $derived<string[]>([
        "all",
        ...selectedDimensions.map((dimension) => dimension.id),
    ]);

    /** Current index in the tab list */
    let activeTabIndex = $derived(
        Math.max(0, tabIds.indexOf(activeDimensionId)),
    );

    function navigatePrev() {
        if (tabIds.length <= 1) return;
        const idx = (activeTabIndex - 1 + tabIds.length) % tabIds.length;
        activeDimensionId = tabIds[idx];
    }

    function navigateNext() {
        if (tabIds.length <= 1) return;
        const idx = (activeTabIndex + 1) % tabIds.length;
        activeDimensionId = tabIds[idx];
    }

    function handleDetailKeydown(event: KeyboardEvent) {
        if (event.key === "q" || event.key === "Q") {
            event.preventDefault();
            navigatePrev();
        } else if (event.key === "e" || event.key === "E") {
            event.preventDefault();
            navigateNext();
        }
    }

    onMount(() => {
        window.addEventListener("keydown", handleDetailKeydown);
        return () => {
            window.removeEventListener("keydown", handleDetailKeydown);
        };
    });
</script>

<div class="detail-stage">
    <!-- Dimension tab bar -->
    <nav class="detail-tabs">
        <button
            type="button"
            class="detail-tab"
            class:active={activeDimensionId === "all"}
            onclick={() => {
                activeDimensionId = "all";
            }}
        >
            All
        </button>
        {#each selectedDimensions as dim}
            <button
                type="button"
                class="detail-tab"
                class:active={activeDimensionId === dim.id}
                onclick={() => {
                    activeDimensionId = dim.id;
                }}
            >
                {dim.name}
            </button>
        {/each}
    </nav>

    <!-- Dimension summary bar (when a dimension is selected) -->
    {#if activeDimension}
        <div class="dimension-summary">
            <CollageLabel text={activeDimension.name} />
            {#if activeDimension.level > 0}
                <span class="dim-level">
                    <span class="dim-level-frag" style:transform="rotate(-3deg)"
                        >Lv.</span
                    >
                    <span
                        class="dim-level-frag dim-level-inv"
                        style:transform="rotate(4deg)"
                        >{activeDimension.level >
                        activeDimension.level_thresholds.length
                            ? "MAX"
                            : activeDimension.level}</span
                    >
                </span>
            {/if}
            {#if activeDimension.level_title}
                <CollageLabel text={activeDimension.level_title} />
            {/if}
        </div>
    {/if}

    <!-- Score groups -->
    <div class="detail-content">
        {#if scoreGroups.length === 0}
            <p class="state-text">No Status scores are available.</p>
        {:else}
            {#each scoreGroups as group}
                <div class="detail-group">
                    <PromptWord text={group.name} fontSize={52} />
                    <div class="detail-metric-grid">
                        {#each group.scores as score}
                            {@const isMissing = score.score === null}
                            <article
                                class="rm-metric-card"
                                class:rm-metric-maxed={!isMissing &&
                                    score.score !== null &&
                                    score.score >= 100}
                                class:rm-metric-missing={isMissing}
                            >
                                <p class="rm-metric-name">{score.name}</p>
                                <p class="rm-metric-value">
                                    {score.score === null
                                        ? "—"
                                        : `${score.score.toFixed(1)} / 100`}
                                </p>
                                <div class="rm-metric-bar-wrap">
                                    <div
                                        class="rm-metric-bar"
                                        style:width="{Math.min(
                                            score.score ?? 0,
                                            100,
                                        )}%"
                                    ></div>
                                </div>
                                {#if score.missing_record_ids?.length}
                                    <p class="rm-metric-missing-records">
                                        Missing: {score.missing_record_ids.join(", ")}
                                    </p>
                                {/if}
                            </article>
                        {/each}
                    </div>
                </div>
            {/each}
        {/if}
    </div>

    <!-- Prev / Next dimension nav -->
    {#if tabIds.length > 1}
        <div class="rm-detail-nav-hints">
            <button
                type="button"
                class="rm-detail-nav-btn"
                onclick={() => navigatePrev()}
            >
                <KeyHint key="Q" fontSize={36} />
                <PromptWord text="Prev" fontSize={72} />
            </button>
            <button
                type="button"
                class="rm-detail-nav-btn"
                onclick={() => navigateNext()}
            >
                <KeyHint key="E" fontSize={36} />
                <PromptWord text="Next" fontSize={72} />
            </button>
        </div>
    {/if}
</div>

<style>
    .detail-stage {
        flex: 1;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        padding-top: clamp(0.5rem, 1vh, 1.5rem);
        padding-bottom: clamp(8rem, 16vh, 14rem);
        width: 80%;
    }

    .detail-tabs {
        display: flex;
        gap: clamp(0.3rem, 0.5vw, 0.8rem);
        padding: 0 clamp(1rem, 2vw, 3rem);
        flex-shrink: 0;
        overflow-x: auto;
        overflow-y: hidden;
    }

    .detail-tab {
        position: relative;
        z-index: 0;
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(1.5rem, 1.575vw, 2.4rem);
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        padding: clamp(0.825rem, 0.975vw, 1.5rem) clamp(1.65rem, 1.95vw, 3rem);
        border: none;
        background: var(--rm-white);
        color: var(--rm-white);
        cursor: pointer;
        clip-path: polygon(0% 0%, 100% 0%, 96% 100%, 4% 100%);
        transition: all 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
        white-space: nowrap;
        display: flex;
        align-items: baseline;
        gap: 0.4em;
    }

    .detail-tab::before {
        content: "";
        position: absolute;
        inset: 6px;
        background: var(--rm-black);
        clip-path: polygon(0% 0%, 100% 0%, 96% 100%, 4% 100%);
        z-index: -1;
        transition: background 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    .detail-tab:hover {
        transform: scale(1.06);
    }

    .detail-tab.active {
        background: var(--rm-white);
        color: var(--rm-black);
    }

    .detail-tab.active::before {
        background: var(--rm-white);
    }

    .dimension-summary {
        display: flex;
        align-items: center;
        gap: clamp(0.9rem, 1.5vw, 2.25rem);
        padding: clamp(0.5rem, 0.8vw, 1.2rem) clamp(1rem, 2vw, 3rem);
        margin-top: clamp(0.8rem, 1.2vw, 2rem);
        flex-shrink: 0;
        font-size: clamp(2.4rem, 2.7vw, 4.2rem);
    }

    .dim-level {
        display: inline-flex;
        align-items: center;
        white-space: nowrap;
        gap: -0.05em;
    }

    .dim-level-frag {
        display: inline-block;
        background: var(--rm-gold, #f5a623);
        color: var(--rm-black, #000);
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-weight: 800;
        font-size: 1em;
        line-height: 1;
        padding: 0.06em 0.08em 0.12em;
        transform-origin: center center;
        box-shadow: 0.04em 0.06em 0 rgba(0, 0, 0, 0.35);
    }

    .dim-level-frag.dim-level-inv {
        background: var(--rm-black, #000);
        color: var(--rm-gold, #f5a623);
        box-shadow:
            0 0 0 0.07em var(--rm-gold, #f5a623),
            0.04em 0.06em 0 rgba(0, 0, 0, 0.35);
        margin-left: -0.03em;
    }

    .detail-content {
        flex: 1;
        overflow-y: auto;
        padding: clamp(0.5rem, 1vw, 1.5rem) clamp(1rem, 2vw, 3rem)
            clamp(1.5rem, 2vw, 3rem);
        scrollbar-gutter: stable;
    }

    /* Custom scrollbar: black track, white thumb, no border-radius, 60% height */
    .detail-content::-webkit-scrollbar {
        width: 14px;
    }

    .detail-content::-webkit-scrollbar-track {
        background: var(--rm-black, #000);
        border: 4px solid var(--rm-white, #fff);
        border-radius: 0;
        margin-top: 12vh;
        margin-bottom: 12vh;
    }

    .detail-content::-webkit-scrollbar-thumb {
        background: var(--rm-white, #fff);
        border-radius: 0;
        border: none;
    }

    .detail-content::-webkit-scrollbar-thumb:hover {
        background: var(--rm-white, #fff);
    }

    .detail-group + .detail-group {
        margin-top: clamp(1rem, 1.5vw, 2.5rem);
    }

    .detail-metric-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(max(180px, 12vw), 1fr));
        gap: clamp(0.5rem, 0.5vw, 1rem);
    }

    .rm-metric-card {
        background: var(--rm-black);
        border: none;
        padding: 0;
        display: flex;
        flex-direction: column;
        transform: rotate(-0.8deg);
        clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 4% 100%);
    }

    .rm-metric-card:nth-child(even) {
        transform: rotate(0.8deg);
    }

    .rm-metric-card.rm-metric-maxed {
        background: var(--rm-gold, #f5a623);
    }

    .rm-metric-card.rm-metric-missing {
        opacity: 0.7;
    }

    .rm-metric-name {
        margin: clamp(0.2rem, 0.25vw, 0.45rem) clamp(0.2rem, 0.25vw, 0.45rem) 0
            clamp(0.2rem, 0.25vw, 0.45rem);
        background: var(--rm-white);
        color: var(--rm-black);
        padding: clamp(0.3rem, 0.4vw, 0.7rem) clamp(0.7rem, 0.9vw, 1.6rem);
        font-size: clamp(0.7rem, 0.65vw, 1.2rem);
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.1em;
        line-height: 1.2;
        clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 1.8% 100%);
    }

    .rm-metric-maxed .rm-metric-name {
        background: var(--rm-black);
        color: var(--rm-gold, #f5a623);
    }

    .rm-metric-value {
        margin: 0;
        background: var(--rm-black);
        color: var(--rm-white);
        padding: clamp(0.25rem, 0.35vw, 0.6rem) clamp(0.7rem, 0.9vw, 1.6rem)
            clamp(0.25rem, 0.35vw, 0.6rem) clamp(1.2rem, 1.4vw, 2.4rem);
        font-size: clamp(1.1rem, 1.1vw, 2.2rem);
        font-weight: 700;
        line-height: 1.2;
    }

    .rm-metric-maxed .rm-metric-value {
        background: var(--rm-gold, #f5a623);
        color: var(--rm-black);
    }

    .rm-metric-bar-wrap {
        height: 3px;
        background: #222;
        margin: 0 clamp(0.2rem, 0.25vw, 0.45rem) clamp(0.2rem, 0.25vw, 0.45rem);
    }

    .rm-metric-bar {
        height: 100%;
        background: #f5a623;
        transition: width 260ms cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    .rm-metric-missing-records {
        margin: 0;
        padding: 0.4rem 0.8rem 0.65rem;
        background: var(--rm-black, #000);
        color: var(--rm-white, #fff);
        font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
        font-size: clamp(0.65rem, 0.65vw, 1rem);
        overflow-wrap: anywhere;
    }

    /* ── Prev / Next navigation ── */
    .rm-detail-nav-hints {
        position: fixed;
        bottom: clamp(1.5rem, 3vh, 3.5rem);
        left: clamp(13rem, 16vw, 22rem);
        z-index: 10;
        display: flex;
        align-items: center;
        gap: clamp(0.6rem, 1vw, 1.5rem);
    }

    .rm-detail-nav-btn {
        display: flex;
        align-items: center;
        gap: 0;
        background: none;
        border: none;
        cursor: pointer;
        padding: 0;
        transform: rotate(-1deg);
        transition: transform 120ms ease;
    }

    .rm-detail-nav-btn:hover {
        transform: rotate(-1deg) scale(1.06);
    }

    .rm-detail-nav-btn :global(.p5-prompt-word) {
        margin-left: -1rem;
    }
</style>
