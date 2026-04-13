<script lang="ts">
  import CallingCardText from "$lib/CallingCardText.svelte";
  import type { StatusData, StatusMetric, DimensionData, MetricGroup } from "$lib/types/status";
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

  let activeDimensionId = $state<string | "all">(selectedDimensionId ?? "all");

  /** Get the active dimension definition, if any */
  let activeDimension = $derived<DimensionData | null>(
    activeDimensionId === "all"
      ? null
      : statusData.dimensions.find(d => d.id === activeDimensionId) ?? null
  );

  /** Set of metric IDs in the active dimension */
  let dimensionMetricIds = $derived<Set<string>>(
    activeDimension
      ? new Set(activeDimension.metrics.map(m => m.metric_id))
      : new Set()
  );

  /** Filter and group metrics for display */
  let metricGroups = $derived.by<MetricGroup[]>(() => {
    // Determine which metrics to show
    let metricsToShow: StatusMetric[];

    if (activeDimensionId === "all") {
      // Show all metrics that have values
      metricsToShow = statusData.metrics.filter(m => m.value !== null);
    } else {
      // Show only metrics in the active dimension (include no-data metrics)
      metricsToShow = statusData.metrics.filter(
        m => dimensionMetricIds.has(m.id)
      );

      // Also include system metrics in the dimension
      if (activeDimension) {
        for (const dm of activeDimension.metrics) {
          if (dm.metric_id.startsWith("sys_") && dm.value !== null) {
            // Create a pseudo-metric for display
            const sysName = dm.metric_id
              .replace(/^sys_/, "")
              .split("_")
              .map(s => s.charAt(0).toUpperCase() + s.slice(1))
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

    return Array.from(groups.entries()).map(([name, metrics]) => ({ name, metrics }));
  });

  /** Get contribution for a metric in the active dimension */
  function getContribution(metricId: string): number | null {
    if (!activeDimension) return null;
    const dm = activeDimension.metrics.find(m => m.metric_id === metricId);
    return dm?.contribution ?? null;
  }
</script>

<div class="detail-stage">
  <!-- Dimension tab bar -->
  <nav class="detail-tabs">
    <button
      type="button"
      class="detail-tab"
      class:active={activeDimensionId === "all"}
      onclick={() => { activeDimensionId = "all"; }}
    >
      All
    </button>
    {#each statusData.dimensions.filter(d => d.enabled) as dim}
      <button
        type="button"
        class="detail-tab"
        class:active={activeDimensionId === dim.id}
        onclick={() => { activeDimensionId = dim.id; }}
      >
        {dim.name}
        {#if dim.level !== null}
          <span class="tab-level">Lv.{dim.level}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <!-- Dimension summary bar (when a dimension is selected) -->
  {#if activeDimension && activeDimension.level !== null}
    <div class="dimension-summary">
      <span class="dimension-summary-name">{activeDimension.name}</span>
      <span class="dimension-summary-level">Lv.{activeDimension.level}</span>
      <span class="dimension-summary-title">{activeDimension.level_title}</span>
    </div>
  {/if}

  <!-- Metric groups -->
  <div class="detail-content">
    {#if metricGroups.length === 0}
      <p class="state-text">No metrics with data.</p>
    {:else}
      {#each metricGroups as group}
        <div class="detail-group">
          <CallingCardText text={formatGroupName(group.name)} fontSize={52} />
          <div class="detail-metric-grid">
            {#each group.metrics as metric}
              {@const contribution = getContribution(metric.id)}
              <article class="rm-metric-card">
                <p class="rm-metric-name">{metric.name}</p>
                <p class="rm-metric-value">{formatMetricValue(metric.value, metric.unit)}</p>
                {#if contribution !== null}
                  <div class="rm-metric-bar-wrap">
                    <div
                      class="rm-metric-bar"
                      style:width="{Math.min(contribution * 100, 100)}%"
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
  }

  .detail-tabs {
    display: flex;
    gap: clamp(0.3rem, 0.5vw, 0.8rem);
    padding: 0 clamp(1rem, 2vw, 3rem);
    flex-shrink: 0;
    overflow-x: auto;
  }

  .detail-tab {
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(0.85rem, 0.9vw, 1.4rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: clamp(0.4rem, 0.5vw, 0.8rem) clamp(0.8rem, 1vw, 1.5rem);
    border: 2px solid var(--rm-white);
    background: var(--rm-black);
    color: var(--rm-white);
    cursor: pointer;
    clip-path: polygon(0% 0%, 100% 0%, 96% 100%, 4% 100%);
    transition: all 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
    white-space: nowrap;
    display: flex;
    align-items: baseline;
    gap: 0.4em;
  }

  .detail-tab:hover {
    background: var(--rm-white);
    color: var(--rm-black);
  }

  .detail-tab.active {
    background: var(--rm-red);
    color: var(--rm-white);
    border-color: var(--rm-red);
  }

  .tab-level {
    font-size: 0.75em;
    opacity: 0.7;
  }

  .dimension-summary {
    display: flex;
    align-items: baseline;
    gap: clamp(0.5rem, 0.8vw, 1.2rem);
    padding: clamp(0.5rem, 0.8vw, 1.2rem) clamp(1rem, 2vw, 3rem);
    flex-shrink: 0;
  }

  .dimension-summary-name {
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(1.4rem, 1.6vw, 2.4rem);
    font-weight: 800;
    color: var(--rm-white);
    text-transform: uppercase;
  }

  .dimension-summary-level {
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(1.2rem, 1.4vw, 2rem);
    font-weight: 800;
    color: #F5A623;
  }

  .dimension-summary-title {
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(1rem, 1.1vw, 1.6rem);
    font-weight: 700;
    color: rgba(255, 255, 255, 0.6);
  }

  .detail-content {
    flex: 1;
    overflow-y: auto;
    padding: clamp(0.5rem, 1vw, 1.5rem) clamp(1rem, 2vw, 3rem) clamp(1.5rem, 2vw, 3rem);
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

  .rm-metric-name {
    margin: clamp(0.2rem, 0.25vw, 0.45rem) clamp(0.2rem, 0.25vw, 0.45rem) 0 clamp(0.2rem, 0.25vw, 0.45rem);
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

  .rm-metric-value {
    margin: 0;
    background: var(--rm-black);
    color: var(--rm-white);
    padding: clamp(0.25rem, 0.35vw, 0.6rem) clamp(0.7rem, 0.9vw, 1.6rem) clamp(0.25rem, 0.35vw, 0.6rem) clamp(1.2rem, 1.4vw, 2.4rem);
    font-size: clamp(1.1rem, 1.1vw, 2.2rem);
    font-weight: 700;
    line-height: 1.2;
  }

  .rm-metric-bar-wrap {
    height: 3px;
    background: #222;
    margin: 0 clamp(0.2rem, 0.25vw, 0.45rem) clamp(0.2rem, 0.25vw, 0.45rem);
  }

  .rm-metric-bar {
    height: 100%;
    background: #F5A623;
    transition: width 260ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }
</style>
