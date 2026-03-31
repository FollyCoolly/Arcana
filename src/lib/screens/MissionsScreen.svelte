<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import P5Text from "$lib/P5Text.svelte";
  import type { MissionData, MissionResponse } from "$lib/types/mission";

  let { onBack }: { onBack: () => void } = $props();

  let loading = $state(false);
  let error = $state<string | null>(null);
  let missionData = $state<MissionData | null>(null);
  let sortIndex = $state(0);
  let selectedIndex = $state(0);
  let rowRefs = $state<(HTMLElement | undefined)[]>([]);

  type SortOption = { key: string; label: string };
  const SORT_OPTIONS: SortOption[] = [
    { key: "newest", label: "NEW" },
    { key: "status", label: "STATE" },
    { key: "progress", label: "DIFFICULTY" },
  ];

  // Carousel: compute visible order so active is always in the center
  // Returns [leftIndex, centerIndex, rightIndex]
  let sortCarousel = $derived.by(() => {
    const len = SORT_OPTIONS.length;
    const center = sortIndex;
    const left = (center - 1 + len) % len;
    const right = (center + 1) % len;
    return [left, center, right] as const;
  });

  const STATUS_ORDER: Record<string, number> = { active: 0, completed: 1, archived: 2 };

  let sortedMissions = $derived.by(() => {
    if (!missionData) return [];
    const list = [...missionData.missions];
    const opt = SORT_OPTIONS[sortIndex];
    switch (opt.key) {
      case "newest":
        return list.sort((a, b) => (b.created_at ?? "").localeCompare(a.created_at ?? ""));
      case "status":
        return list.sort((a, b) => (STATUS_ORDER[a.status] ?? 9) - (STATUS_ORDER[b.status] ?? 9));
      case "progress":
        return list.sort((a, b) => (b.progress ?? 0) - (a.progress ?? 0));
      default:
        return list;
    }
  });

  // Clamp selectedIndex when list changes
  $effect(() => {
    const len = sortedMissions.length;
    if (len === 0) {
      selectedIndex = 0;
    } else if (selectedIndex >= len) {
      selectedIndex = len - 1;
    }
  });

  // Auto-scroll selected row into view
  $effect(() => {
    const el = rowRefs[selectedIndex];
    if (el) el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  });

  function cycleSort(dir: number) {
    sortIndex = (sortIndex + dir + SORT_OPTIONS.length) % SORT_OPTIONS.length;
    selectedIndex = 0;
  }

  function progressGrade(progress?: number): string {
    if (progress == null) return "--";
    if (progress >= 80) return "A";
    if (progress >= 60) return "B";
    if (progress >= 40) return "C";
    if (progress >= 20) return "D";
    return "E";
  }

  function statusLabel(status: string): string {
    switch (status) {
      case "active": return "ACTIVE";
      case "completed": return "CLEAR!";
      case "archived": return "ARCHIVED";
      default: return status.toUpperCase();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onBack();
      return;
    }
    if (event.key === "q" || event.key === "Q") {
      event.preventDefault();
      cycleSort(-1);
      return;
    }
    if (event.key === "e" || event.key === "E") {
      event.preventDefault();
      cycleSort(1);
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (sortedMissions.length > 0) {
        selectedIndex = Math.min(selectedIndex + 1, sortedMissions.length - 1);
      }
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);

    async function load() {
      loading = true;
      error = null;
      try {
        missionData = await invoke<MissionData>("load_missions");
      } catch (e) {
        error = String(e);
      } finally {
        loading = false;
      }
    }

    void load();

    return () => {
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

<section class="rm-stage">
  <div class="rm-missions-panel">
    <!-- Sort carousel: Q shifts right-to-left, E shifts left-to-right, center is active -->
    <header class="rm-missions-sort-bar">
      <button class="rm-sort-key-hint" onclick={() => cycleSort(-1)} aria-label="Previous sort">Q</button>
      <div class="rm-sort-carousel">
        <button
          class="rm-sort-item rm-sort-side"
          onclick={() => cycleSort(-1)}
        >
          {SORT_OPTIONS[sortCarousel[0]].label}
        </button>
        <span class="rm-sort-item rm-sort-center">
          {SORT_OPTIONS[sortCarousel[1]].label}
        </span>
        <button
          class="rm-sort-item rm-sort-side"
          onclick={() => cycleSort(1)}
        >
          {SORT_OPTIONS[sortCarousel[2]].label}
        </button>
      </div>
      <button class="rm-sort-key-hint" onclick={() => cycleSort(1)} aria-label="Next sort">E</button>
    </header>

    <!-- Column headers -->
    <div class="rm-missions-col-headers">
      <span class="rm-col-header rm-col-status">STATUS</span>
      <span class="rm-col-header rm-col-name">MISSION</span>
      <span class="rm-col-header rm-col-grade">DIFFICULTY</span>
    </div>

    <!-- Mission list -->
    <div class="rm-missions-scroll">
      {#if loading}
        <p class="state-text">Loading...</p>
      {:else if error}
        <p class="state-text error">{error}</p>
      {:else if sortedMissions.length > 0}
        <ul class="rm-missions-list">
          {#each sortedMissions as mission, i (mission.id)}
            <li
              class="rm-mission-row"
              class:is-selected={selectedIndex === i}
              class:is-completed={mission.status === "completed"}
              class:is-archived={mission.status === "archived"}
              bind:this={rowRefs[i]}
              onclick={() => { selectedIndex = i; }}
              onmouseenter={() => { selectedIndex = i; }}
            >
              <span
                class="rm-mission-stamp"
                class:stamp-active={mission.status === "active"}
                class:stamp-clear={mission.status === "completed"}
                class:stamp-archived={mission.status === "archived"}
              >
                {statusLabel(mission.status)}
              </span>

              <span class="rm-mission-name">{mission.title}</span>

              <span class="rm-mission-grade" data-grade={progressGrade(mission.progress)}>
                {progressGrade(mission.progress)}
              </span>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="state-text">No missions yet.</p>
      {/if}
    </div>
  </div>

  <div class="rm-missions-title">
    <P5Text text="MiSSiONS" />
  </div>

  <button type="button" class="rm-back-btn" onclick={() => onBack()}>
    <img src="/ui/back.png" alt="Back" class="rm-back-img" />
  </button>
</section>

<style>
  /* ── Panel ── */
  .rm-missions-panel {
    position: absolute;
    top: 0;
    right: 0;
    width: 75%;
    height: 100%;
    transform-origin: bottom right;
    transform: rotate(-3deg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: rgba(0, 0, 0, 0.85);
    border-left: 3px solid rgba(255, 255, 255, 0.12);
  }

  /* ── Sort carousel ── */
  .rm-missions-sort-bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #000000;
    border-bottom: 2px solid rgba(255, 255, 255, 0.12);
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    padding: clamp(0.4rem, 0.5vw, 0.7rem) 0;
    gap: 0;
  }

  .rm-sort-carousel {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: clamp(0.8rem, 1.2vw, 2rem);
    flex: 1;
    overflow: hidden;
  }

  .rm-sort-item {
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    white-space: nowrap;
    transition: font-size 180ms ease, color 180ms ease, transform 180ms ease, opacity 180ms ease;
  }

  .rm-sort-center {
    font-size: clamp(1.1rem, 1.5vw, 2rem);
    color: #E5191C;
    transform: scale(1);
    padding: clamp(0.15rem, 0.2vw, 0.3rem) clamp(0.6rem, 0.8vw, 1.2rem);
    background: rgba(229, 25, 28, 0.12);
    clip-path: polygon(3% 0%, 100% 5%, 97% 100%, 0% 95%);
  }

  .rm-sort-side {
    font-size: clamp(0.65rem, 0.7vw, 1rem);
    color: rgba(255, 255, 255, 0.3);
    border: none;
    background: none;
    cursor: pointer;
    padding: 0;
  }

  .rm-sort-side:hover {
    color: rgba(255, 255, 255, 0.6);
  }

  .rm-sort-key-hint {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: clamp(1.6rem, 2vw, 2.4rem);
    height: clamp(1.6rem, 2vw, 2.4rem);
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(0.6rem, 0.6vw, 0.9rem);
    font-weight: 800;
    color: rgba(255, 255, 255, 0.35);
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    cursor: pointer;
    margin: 0 clamp(0.3rem, 0.4vw, 0.6rem);
    transition: background 120ms ease, color 120ms ease;
  }

  .rm-sort-key-hint:hover {
    background: rgba(255, 255, 255, 0.12);
    color: rgba(255, 255, 255, 0.6);
  }

  /* ── Column headers ── */
  .rm-missions-col-headers {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding: clamp(0.3rem, 0.4vw, 0.6rem) clamp(1rem, 1.2vw, 2rem);
    border-bottom: 2px solid rgba(255, 255, 255, 0.15);
    background: rgba(255, 255, 255, 0.03);
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
  }

  .rm-col-header {
    font-size: clamp(0.55rem, 0.5vw, 0.8rem);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: rgba(255, 255, 255, 0.35);
  }

  .rm-col-status {
    width: clamp(5rem, 8vw, 9rem);
    flex-shrink: 0;
  }

  .rm-col-name {
    flex: 1;
  }

  .rm-col-grade {
    width: clamp(3rem, 5vw, 6rem);
    flex-shrink: 0;
    text-align: center;
  }

  /* ── Scroll area ── */
  .rm-missions-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 0 0 4rem 0;
  }

  .rm-missions-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  /* ── Mission rows ── */
  .rm-mission-row {
    display: flex;
    align-items: center;
    padding: clamp(0.6rem, 0.8vw, 1.2rem) clamp(1rem, 1.2vw, 2rem);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    cursor: pointer;
    transition: background 100ms ease, transform 100ms ease;
    clip-path: polygon(0% 4%, 100% 0%, 100% 96%, 0% 100%);
    position: relative;
  }

  .rm-mission-row:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .rm-mission-row.is-selected {
    background: #E5191C;
    transform: scaleY(1.08);
    clip-path: polygon(0% 0%, 100% 3%, 100% 97%, 0% 100%);
    z-index: 2;
  }

  .rm-mission-row.is-completed {
    opacity: 0.55;
  }

  .rm-mission-row.is-archived {
    opacity: 0.3;
  }

  .rm-mission-row.is-selected.is-completed,
  .rm-mission-row.is-selected.is-archived {
    opacity: 1;
  }

  /* ── Status stamp ── */
  .rm-mission-stamp {
    width: clamp(5rem, 8vw, 9rem);
    flex-shrink: 0;
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(0.7rem, 0.75vw, 1.1rem);
    font-weight: 900;
    font-style: italic;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.3);
  }

  .rm-mission-stamp.stamp-active {
    color: #E5191C;
    transform: rotate(-2deg);
    text-shadow: 1px 1px 0 rgba(0, 0, 0, 0.5);
  }

  .rm-mission-stamp.stamp-clear {
    color: rgba(255, 255, 255, 0.7);
    transform: rotate(-1deg);
  }

  .rm-mission-row.is-selected .rm-mission-stamp {
    color: #ffffff;
  }

  /* ── Mission name ── */
  .rm-mission-name {
    flex: 1;
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(0.85rem, 0.9vw, 1.4rem);
    font-weight: 800;
    color: #ffffff;
    letter-spacing: 0.03em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Grade letter ── */
  .rm-mission-grade {
    width: clamp(3rem, 5vw, 6rem);
    flex-shrink: 0;
    text-align: center;
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(1.4rem, 1.8vw, 2.5rem);
    font-weight: 900;
    color: #000000;
    background: #ffffff;
    padding: clamp(0.1rem, 0.15vw, 0.25rem) 0;
    clip-path: polygon(5% 0%, 100% 4%, 95% 100%, 0% 96%);
    line-height: 1.2;
  }

  .rm-mission-grade[data-grade="A"] {
    color: #E5191C;
  }

  .rm-mission-grade[data-grade="E"] {
    opacity: 0.5;
  }

  .rm-mission-grade[data-grade="--"] {
    font-size: clamp(0.9rem, 1vw, 1.5rem);
    opacity: 0.3;
  }

  .rm-mission-row.is-selected .rm-mission-grade {
    background: #ffffff;
    color: #E5191C;
  }

  /* ── Title position ── */
  .rm-missions-title {
    position: absolute;
    top: 2rem;
    left: 1.5rem;
    z-index: 2;
    pointer-events: none;
  }
</style>
