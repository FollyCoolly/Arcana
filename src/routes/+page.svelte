<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  type StatusMetric = {
    id: string;
    name: string;
    category: "health" | "performance" | string;
    group: string;
    unit: string;
    value_type: string;
    value: number | null;
    target_max?: number;
    target_min?: number;
    body_parts: string[];
    description?: string;
  };

  type StatusData = {
    definition_version: number;
    value_version: number;
    username: string;
    game_days: number | null;
    bmi: number | null;
    metrics: StatusMetric[];
  };

  type MetricGroup = {
    name: string;
    metrics: StatusMetric[];
  };

  let loading = $state(true);
  let errorMessage = $state<string | null>(null);
  let statusData = $state<StatusData | null>(null);

  function formatGroupName(group: string): string {
    return group
      .split("_")
      .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
      .join(" ");
  }

  function formatValue(value: number): string {
    return Number.isInteger(value) ? String(value) : value.toFixed(1).replace(/\.0$/, "");
  }

  function formatMetricValue(metric: StatusMetric): string {
    if (metric.value === null || metric.value === undefined) {
      return "-";
    }
    return metric.unit ? `${formatValue(metric.value)} ${metric.unit}` : formatValue(metric.value);
  }

  function getCategoryGroups(category: "health" | "performance"): MetricGroup[] {
    if (!statusData) {
      return [];
    }

    const groups = new Map<string, StatusMetric[]>();

    for (const metric of statusData.metrics) {
      if (metric.category !== category) {
        continue;
      }
      if (metric.value === null || metric.value === undefined) {
        continue;
      }

      const list = groups.get(metric.group) ?? [];
      list.push(metric);
      groups.set(metric.group, list);
    }

    return Array.from(groups.entries()).map(([name, metrics]) => ({ name, metrics }));
  }

  function getHealthGroupsWithDerived(): MetricGroup[] {
    if (!statusData) {
      return [];
    }

    const groups = getCategoryGroups("health");

    if (statusData.bmi !== null && statusData.bmi !== undefined) {
      const derivedMetric: StatusMetric = {
        id: "bmi",
        name: "BMI",
        category: "health",
        group: "derived",
        unit: "",
        value_type: "number",
        value: statusData.bmi,
        body_parts: [],
      };

      const existing = groups.find((group) => group.name === "derived");
      if (existing) {
        existing.metrics.unshift(derivedMetric);
      } else {
        groups.push({ name: "derived", metrics: [derivedMetric] });
      }
    }

    return groups;
  }

  async function loadStatusData() {
    loading = true;
    errorMessage = null;

    try {
      statusData = await invoke<StatusData>("load_status_data");
    } catch (error) {
      errorMessage =
        typeof error === "string"
          ? error
          : "Failed to load status data. Check data files in /data.";
      statusData = null;
    } finally {
      loading = false;
    }
  }

  onMount(loadStatusData);
</script>

<main class="status-page">
  <header class="page-header">
    <div>
      <h1>Reality Mod Status</h1>
      <p>
        {#if statusData}
          {statusData.username}
          {#if statusData.game_days !== null}
            · Game Day {statusData.game_days}
          {/if}
        {:else}
          Current snapshot from local JSON data.
        {/if}
      </p>
    </div>
    <button type="button" class="reload-btn" onclick={loadStatusData}>Reload</button>
  </header>

  {#if loading}
    <p class="state-text">Loading status data...</p>
  {:else if errorMessage}
    <p class="state-text error">{errorMessage}</p>
  {:else if statusData}
    <section class="panel">
      <h2>Health</h2>
      {#if getHealthGroupsWithDerived().length === 0}
        <p class="state-text">No health metrics yet.</p>
      {:else}
        {#each getHealthGroupsWithDerived() as group}
          <div class="group-block">
            <h3>{formatGroupName(group.name)}</h3>
            <div class="card-grid">
              {#each group.metrics as metric}
                <article class="metric-card">
                  <p class="metric-name">{metric.name}</p>
                  <p class="metric-value">{formatMetricValue(metric)}</p>
                </article>
              {/each}
            </div>
          </div>
        {/each}
      {/if}
    </section>

    <section class="panel">
      <h2>Performance</h2>
      {#if getCategoryGroups("performance").length === 0}
        <p class="state-text">No performance metrics yet.</p>
      {:else}
        {#each getCategoryGroups("performance") as group}
          <div class="group-block">
            <h3>{formatGroupName(group.name)}</h3>
            <div class="card-grid">
              {#each group.metrics as metric}
                <article class="metric-card">
                  <p class="metric-name">{metric.name}</p>
                  <p class="metric-value">{formatMetricValue(metric)}</p>
                </article>
              {/each}
            </div>
          </div>
        {/each}
      {/if}
    </section>
  {/if}
</main>

<style>
  .status-page {
    min-height: 100vh;
    padding: 2rem 1rem 3rem;
    color: #172026;
    font-family: "IBM Plex Sans", "Noto Sans SC", "Segoe UI", sans-serif;
    background:
      radial-gradient(circle at 0% 0%, rgba(44, 135, 120, 0.2), transparent 40%),
      radial-gradient(circle at 100% 0%, rgba(245, 157, 66, 0.15), transparent 35%),
      #f3f7f6;
  }

  .page-header {
    max-width: 1080px;
    margin: 0 auto 1rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .page-header h1 {
    margin: 0;
    font-size: clamp(1.4rem, 2vw, 2rem);
    font-weight: 700;
  }

  .page-header p {
    margin: 0.25rem 0 0;
    color: #4f5e66;
  }

  .reload-btn {
    border: none;
    border-radius: 999px;
    padding: 0.6rem 1rem;
    color: #fff;
    cursor: pointer;
    background: linear-gradient(135deg, #2b8a78, #20665b);
  }

  .panel {
    max-width: 1080px;
    margin: 0 auto 1rem;
    padding: 1rem;
    border-radius: 14px;
    border: 1px solid rgba(23, 32, 38, 0.08);
    background: rgba(255, 255, 255, 0.75);
    backdrop-filter: blur(3px);
  }

  .panel h2 {
    margin: 0 0 0.75rem;
    font-size: 1.1rem;
  }

  .group-block + .group-block {
    margin-top: 1rem;
  }

  .group-block h3 {
    margin: 0 0 0.6rem;
    color: #2d3b43;
    font-size: 0.95rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .card-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 0.65rem;
  }

  .metric-card {
    padding: 0.75rem;
    border-radius: 10px;
    border: 1px solid rgba(23, 32, 38, 0.08);
    background: #fff;
  }

  .metric-name {
    margin: 0;
    color: #607078;
    font-size: 0.85rem;
  }

  .metric-value {
    margin: 0.35rem 0 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .state-text {
    max-width: 1080px;
    margin: 1rem auto;
    color: #4f5e66;
  }

  .state-text.error {
    color: #b42318;
  }

  @media (max-width: 640px) {
    .page-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .reload-btn {
      width: 100%;
    }
  }
</style>
