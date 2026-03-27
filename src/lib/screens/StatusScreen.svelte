<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import P5Text from "$lib/P5Text.svelte";
  import type { StatusData, StatusMetric, MetricGroup } from "$lib/types/status";
  import { formatGroupName } from "$lib/utils/format";

  let { onBack, statusData: initialStatusData }: { onBack: () => void; statusData: StatusData | null } = $props();

  let loading = $state(false);
  let errorMessage = $state<string | null>(null);
  let statusData = $state<StatusData | null>(initialStatusData);

  const STRENGTH_SUBGROUP_ORDER = ["chest", "back", "shoulders", "biceps", "triceps", "legs", "core"];

  function formatValue(value: number): string {
    return Number.isInteger(value) ? String(value) : value.toFixed(1).replace(/\.0$/, "");
  }

  function formatUnit(unit: string): string {
    switch (unit) {
      case "":
      case "reps":
        return "";
      case "percent":
        return "%";
      case "kg_each":
        return "ea";
      case "sec":
        return "s";
      case "sec_per_km":
        return "/km";
      default:
        return unit.replace(/_/g, " ");
    }
  }

  function secsToMSS(totalSecs: number): string {
    const m = Math.floor(totalSecs / 60);
    const s = Math.round(totalSecs % 60);
    return `${m}:${String(s).padStart(2, "0")}`;
  }

  function formatMetricValue(metric: StatusMetric): string {
    if (metric.value === null || metric.value === undefined) {
      return "-";
    }

    const v = metric.value;

    if (metric.unit === "sec_per_km") {
      return `${secsToMSS(v)} /km`;
    }

    if (metric.unit === "sec" && v >= 60) {
      return secsToMSS(v);
    }

    const unitStr = formatUnit(metric.unit);
    const numStr = formatValue(v);

    if (!unitStr) {
      return numStr;
    }
    if (unitStr === "%") {
      return `${numStr}%`;
    }
    return `${numStr} ${unitStr}`;
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
        group: "body",
        unit: "",
        value_type: "number",
        value: statusData.bmi,
        body_parts: [],
      };

      const existing = groups.find((group) => group.name === "body");
      if (existing) {
        existing.metrics.unshift(derivedMetric);
      } else {
        groups.push({ name: "body", metrics: [derivedMetric] });
      }
    }

    return groups;
  }

  function getStrengthSubGroups(): { subGroup: string; metrics: StatusMetric[] }[] {
    if (!statusData) {
      return [];
    }

    const bySubGroup = new Map<string, StatusMetric[]>();

    for (const metric of statusData.metrics) {
      if (metric.group !== "strength") continue;
      if (metric.value === null || metric.value === undefined) continue;

      const sg = metric.sub_group ?? "other";
      const list = bySubGroup.get(sg) ?? [];
      list.push(metric);
      bySubGroup.set(sg, list);
    }

    const result: { subGroup: string; metrics: StatusMetric[] }[] = [];
    for (const sg of STRENGTH_SUBGROUP_ORDER) {
      const metrics = bySubGroup.get(sg);
      if (metrics && metrics.length > 0) {
        result.push({ subGroup: sg, metrics });
        bySubGroup.delete(sg);
      }
    }
    for (const [sg, metrics] of bySubGroup.entries()) {
      result.push({ subGroup: sg, metrics });
    }

    return result;
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

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onBack();
    }
  }

  onMount(() => {
    if (!statusData && !loading) {
      void loadStatusData();
    }

    window.addEventListener("keydown", handleKeydown);
    return () => {
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

<section class="rm-stage">
  <img src="/ui/Status.png" alt="Status" class="rm-status-title-img" />

  <button type="button" class="rm-back-btn" onclick={onBack}>
    <img src="/ui/back.png" alt="Back" class="rm-back-img" />
  </button>

  <div class="rm-stage-inner">
    {#if loading}
      <p class="state-text">Loading status data...</p>
    {:else if errorMessage}
      <p class="state-text error">{errorMessage}</p>
    {:else if statusData}
      <!-- LEFT COLUMN: Health -->
      <div class="rm-col-health">
        {#if getHealthGroupsWithDerived().length === 0}
          <p class="state-text">No health metrics yet.</p>
        {:else}
          {#each getHealthGroupsWithDerived() as group}
            <div class="rm-group-block">
              <P5Text text={formatGroupName(group.name)} fontSize={62} />
              <div
                class="rm-metric-grid"
                class:rm-metric-grid--body={group.name === "body"}
                class:rm-metric-grid--vitals={group.name === "vitals"}
                class:rm-metric-grid--circumference={group.name === "circumference"}
              >
                {#each group.metrics as metric}
                  <article class="rm-metric-card">
                    <p class="rm-metric-name">{metric.name}</p>
                    <p class="rm-metric-value">{formatMetricValue(metric)}</p>
                  </article>
                {/each}
              </div>
            </div>
          {/each}
        {/if}
      </div>

      <!-- RIGHT COLUMN: Performance -->
      <div class="rm-col-performance">
        {#if getStrengthSubGroups().length > 0}
          <div class="rm-group-block">
            <P5Text text="Strength" fontSize={62} />
            {#each getStrengthSubGroups() as sg}
              <div class="rm-subgroup-block">
                <h5 class="rm-subgroup-title">{formatGroupName(sg.subGroup)}</h5>
                <div class="rm-metric-grid">
                  {#each sg.metrics as metric}
                    <article class="rm-metric-card">
                      <p class="rm-metric-name">{metric.name}</p>
                      <p class="rm-metric-value">{formatMetricValue(metric)}</p>
                    </article>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {/if}

        {#each getCategoryGroups("performance").filter(g => g.name !== "strength") as group}
          <div class="rm-group-block">
            <P5Text text={formatGroupName(group.name)} fontSize={62} />
            <div
              class="rm-metric-grid"
              class:rm-metric-grid--endurance={group.name === "endurance"}
            >
              {#each group.metrics as metric}
                <article class="rm-metric-card">
                  <p class="rm-metric-name">{metric.name}</p>
                  <p class="rm-metric-value">{formatMetricValue(metric)}</p>
                </article>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <p class="state-text">Status data is not available yet.</p>
    {/if}
  </div>
</section>

<style>
  .rm-status-title-img {
    position: fixed;
    top: clamp(0.8rem, 1.5vh, 3rem);
    right: clamp(1.2rem, 2.5vw, 5rem);
    height: clamp(9rem, 15vh, 27rem);
    width: auto;
    z-index: 10;
    pointer-events: none;
  }

  .rm-stage-inner {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: 0;
    overflow: hidden;
  }

  .rm-col-health,
  .rm-col-performance {
    overflow-y: auto;
    height: 100%;
    padding: clamp(1rem, 1.5vw, 2.5rem) clamp(0.8rem, 2vw, 3.5rem) clamp(1.5rem, 2vw, 3rem);
    box-sizing: border-box;
  }

  .rm-group-block + .rm-group-block {
    margin-top: clamp(1rem, 1.5vw, 2.5rem);
  }

  .rm-metric-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(max(180px, 12vw), 1fr));
    gap: clamp(0.5rem, 0.5vw, 1rem);
  }

  .rm-metric-grid--body {
    grid-template-columns: repeat(auto-fill, minmax(max(200px, 14vw), 1fr));
  }

  .rm-metric-grid--vitals {
    grid-template-columns: repeat(3, 1fr);
  }

  .rm-metric-grid--circumference {
    grid-template-columns: repeat(auto-fill, minmax(max(120px, 8vw), 1fr));
    gap: clamp(0.4rem, 0.5vw, 1rem);
  }

  .rm-metric-grid--endurance {
    grid-template-columns: repeat(auto-fill, minmax(max(180px, 12vw), 1fr));
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

  .rm-subgroup-block {
    margin-top: 0.25rem;
  }

  .rm-subgroup-title {
    margin: clamp(0.75rem, 0.9vw, 1.75rem) 0 clamp(0.3rem, 0.4vw, 0.8rem);
    font-size: clamp(0.72rem, 0.62vw, 1.3rem);
    color: var(--rm-red);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    border-left: 0.2rem solid var(--rm-red);
    padding-left: clamp(0.4rem, 0.5vw, 1rem);
  }

  @media (max-width: 980px) {
    .rm-stage-inner {
      grid-template-columns: 1fr;
    }

    .rm-col-health {
      height: auto;
      overflow-y: visible;
    }

    .rm-col-performance {
      height: auto;
      overflow-y: visible;
    }
  }
</style>
