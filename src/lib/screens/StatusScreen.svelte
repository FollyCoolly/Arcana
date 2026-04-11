<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import RadarChart from "$lib/components/RadarChart.svelte";
  import StatusDetailView from "$lib/screens/StatusDetailView.svelte";
  import type { StatusData } from "$lib/types/status";

  let { onBack, statusData: initialStatusData }: { onBack: () => void; statusData: StatusData | null } = $props();

  let loading = $state(false);
  let errorMessage = $state<string | null>(null);
  let statusData = $state<StatusData | null>(initialStatusData);
  let view = $state<"radar" | "detail">("radar");
  let selectedDimensionId = $state<string | null>(null);

  function handleDimensionSelect(id: string) {
    selectedDimensionId = id;
    view = "detail";
  }

  function handleDetailBack() {
    view = "radar";
    selectedDimensionId = null;
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
      if (view === "detail") {
        handleDetailBack();
      } else {
        onBack();
      }
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

  <button type="button" class="rm-back-btn" onclick={() => {
    if (view === "detail") {
      handleDetailBack();
    } else {
      onBack();
    }
  }}>
    <img src="/ui/back.png" alt="Back" class="rm-back-img" />
  </button>

  {#if loading}
    <div class="rm-stage-inner">
      <p class="state-text">Loading status data...</p>
    </div>
  {:else if errorMessage}
    <div class="rm-stage-inner">
      <p class="state-text error">{errorMessage}</p>
    </div>
  {:else if statusData}
    {#if view === "radar"}
      <div class="rm-radar-stage">
        <RadarChart
          dimensions={statusData.dimensions.filter(d => d.enabled)}
          onSelect={handleDimensionSelect}
        />
      </div>
    {:else}
      <StatusDetailView
        {statusData}
        {selectedDimensionId}
        onBack={handleDetailBack}
      />
    {/if}
  {:else}
    <div class="rm-stage-inner">
      <p class="state-text">Status data is not available yet.</p>
    </div>
  {/if}
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
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .rm-radar-stage {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: clamp(1rem, 2vw, 3rem);
  }
</style>
