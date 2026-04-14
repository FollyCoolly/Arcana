<script lang="ts">
    import CollageLabel from "$lib/CollageLabel.svelte";
    import PromptWord from "$lib/PromptWord.svelte";
    import type {
        StatusData,
        StatusMetric,
        DimensionData,
        MetricGroup,
    } from "$lib/types/status";
    import { formatGroupName, formatMetricValue } from "$lib/utils/format";

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

    /** Get the active dimension definition, if any */
    let activeDimension = $derived<DimensionData | null>(
        activeDimensionId === "all"
            ? null
            : (statusData.dimensions.find((d) => d.id === activeDimensionId) ??
                  null),
    );

    /** Set of metric IDs in the active dimension */
    let dimensionMetricIds = $derived<Set<string>>(
        activeDimension
            ? new Set(activeDimension.metrics.map((m) => m.metric_id))
            : new Set(),
    );

    /** Filter and group metrics for display */
    let metricGroups = $derived.by<MetricGroup[]>(() => {
        // Determine which metrics to show
        let metricsToShow: StatusMetric[];

        if (activeDimensionId === "all") {
            // Show all metrics that have values
            metricsToShow = statusData.metrics.filter((m) => m.value !== null);
        } else {
            // Show only metrics in the active dimension (include no-data metrics)
            metricsToShow = statusData.metrics.filter((m) =>
                dimensionMetricIds.has(m.id),
            );

            // Also include system metrics in the dimension
            if (activeDimension) {
                for (const dm of activeDimension.metrics) {
                    if (dm.metric_id.startsWith("sys_") && dm.value !== null) {
                        // Create a pseudo-metric for display
                        const sysName = dm.metric_id
                            .replace(/^sys_/, "")
                            .split("_")
                            .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
                            .join(" ");
                        metricsToShow.push({
                            id: dm.metric_id,
                            name: sysName,
                            group: "system",
                            unit: "count",
                            value_type: "number",
                            value: dm.value,
                        });
                    }
                }
            }
        }

        // Group by group field
        const groups = new Map<string, StatusMetric[]>();
        for (const metric of metricsToShow) {
            const list = groups.get(metric.group) ?? [];
            list.push(metric);
            groups.set(metric.group, list);
        }

        return Array.from(groups.entries()).map(([name, metrics]) => ({
            name,
            metrics,
        }));
    });

    /** Get contribution for a metric in the active dimension (or max across all dimensions in "all" view) */
    function getContribution(metricId: string): number | null {
        if (activeDimension) {
            const dm = activeDimension.metrics.find(
                (m) => m.metric_id === metricId,
            );
            return dm?.contribution ?? null;
        }

        // "All" view: find the max contribution across all enabled dimensions
        let max: number | null = null;
        for (const dim of statusData.dimensions) {
            if (!dim.enabled) continue;
            const dm = dim.metrics.find((m) => m.metric_id === metricId);
            if (dm?.contribution != null) {
                max =
                    max === null
                        ? dm.contribution
                        : Math.max(max, dm.contribution);
            }
        }
        return max;
    }
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
        {#each statusData.dimensions.filter((d) => d.enabled) as dim}
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
            {#if activeDimension.level !== null}
                <span class="dim-level">
                    <span class="dim-level-frag" style:transform="rotate(-3deg)"
                        >Lv.</span
                    >
                    <span
                        class="dim-level-frag dim-level-inv"
                        style:transform="rotate(4deg)"
                        >{activeDimension.level >=
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

    <!-- Metric groups -->
    <div class="detail-content">
        {#if metricGroups.length === 0}
            <p class="state-text">No metrics with data.</p>
        {:else}
            {#each metricGroups as group}
                <div class="detail-group">
                    <PromptWord
                        text={formatGroupName(group.name)}
                        fontSize={52}
                    />
                    <div class="detail-metric-grid">
                        {#each group.metrics as metric}
                            {@const contribution = getContribution(metric.id)}
                            <article
                                class="rm-metric-card"
                                class:rm-metric-maxed={contribution === null ||
                                    contribution >= 1}
                            >
                                <p class="rm-metric-name">{metric.name}</p>
                                <p class="rm-metric-value">
                                    {formatMetricValue(
                                        metric.value,
                                        metric.unit,
                                    )}
                                </p>
                                {#if contribution !== null && contribution < 1}
                                    <div class="rm-metric-bar-wrap">
                                        <div
                                            class="rm-metric-bar"
                                            style:width="{Math.min(
                                                contribution * 100,
                                                100,
                                            )}%"
                                        ></div>
                                    </div>
                                {/if}
                            </article>
                        {/each}
                    </div>
                </div>
            {/each}
        {/if}
    </div>
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
</style>
