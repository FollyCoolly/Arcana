<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import P5Text from "$lib/P5Text.svelte";
  import type { AchievementData, PackAchievements } from "$lib/types/achievement";

  let { onBack, achievementData: externalData = null, onAchievementDataLoaded }: {
    onBack: () => void;
    achievementData?: AchievementData | null;
    onAchievementDataLoaded?: (data: AchievementData) => void;
  } = $props();

  let achievementLoading = $state(false);
  let achievementError = $state<string | null>(null);
  let achievementData = $state<AchievementData | null>(externalData);
  let selectedPackIndex = $state(0);

  function getDifficultyLabel(difficulty: string): string {
    return difficulty.charAt(0).toUpperCase() + difficulty.slice(1);
  }

  function getPackStats(pack: PackAchievements): { total: number; unlocked: number } {
    const total = pack.achievements.length;
    const unlocked = pack.achievements.filter(
      (a) => achievementData?.progress[a.id]
    ).length;
    return { total, unlocked };
  }

  function selectPack(index: number) {
    selectedPackIndex = index;
  }

  function getSelectedPack(): PackAchievements | null {
    return achievementData?.packs[selectedPackIndex] ?? null;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onBack();
    }
  }

  async function loadAchievementData() {
    achievementLoading = true;
    achievementError = null;

    try {
      achievementData = await invoke<AchievementData>("load_achievements");
      if (achievementData && onAchievementDataLoaded) {
        onAchievementDataLoaded(achievementData);
      }
    } catch (error) {
      achievementError =
        typeof error === "string"
          ? error
          : "Failed to load achievement data.";
      achievementData = null;
    } finally {
      achievementLoading = false;
    }
  }

  onMount(() => {
    if (!achievementData && !achievementLoading) {
      void loadAchievementData();
    }

    window.addEventListener("keydown", handleKeydown);
    return () => {
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

<section class="rm-stage">
  <div class="rm-achievement-title">
    <P5Text text="Achievements" fontSize={82} />
  </div>

  <button type="button" class="rm-back-btn" onclick={onBack}>
    <img src="/ui/back.png" alt="Back" class="rm-back-img" />
  </button>

  <div class="rm-ach-layout">
    {#if achievementLoading}
      <p class="state-text">Loading achievements...</p>
    {:else if achievementError}
      <p class="state-text error">{achievementError}</p>
    {:else if achievementData}
      <!-- LEFT: Pack nav -->
      <nav class="rm-ach-sidebar">
        <div class="rm-ach-pack-list">
          {#each achievementData.packs as pack, pi}
            <button
              type="button"
              class="rm-ach-pack-btn"
              class:is-active={pi === selectedPackIndex}
              onclick={() => selectPack(pi)}
            >
              <P5Text text={pack.pack_name} fontSize={28} />
            </button>
          {/each}
        </div>
      </nav>

      <!-- RIGHT: Achievement cards -->
      <div class="rm-ach-content">
        {#if getSelectedPack()}
          {@const pack = getSelectedPack()!}
          {@const stats = getPackStats(pack)}
          <div class="rm-ach-content-header">
            <P5Text text={pack.pack_name} fontSize={52} />
            <span class="rm-ach-stats">{stats.unlocked} / {stats.total}</span>
          </div>
          <div class="rm-achievement-grid">
            {#each pack.achievements as achievement}
              {@const unlocked = achievementData.progress[achievement.id]}
              <article class="rm-achievement-card" class:is-unlocked={!!unlocked}>
                <div class="rm-achievement-card-header">
                  <span class="rm-achievement-status-icon">{unlocked ? "✓" : "○"}</span>
                  <span class="rm-achievement-name">{achievement.name}</span>
                  <span class="rm-difficulty rm-difficulty--{achievement.difficulty}">{getDifficultyLabel(achievement.difficulty)}</span>
                </div>
                <p class="rm-achievement-desc">{achievement.description}</p>
                {#if unlocked?.achieved_at}
                  <p class="rm-achievement-date">{unlocked.achieved_at}</p>
                {/if}
                {#if unlocked?.note}
                  <p class="rm-achievement-note">{unlocked.note}</p>
                {/if}
                {#if achievement.prerequisites.length > 0}
                  <div class="rm-achievement-prereqs">
                    {#each achievement.prerequisites as prereq}
                      <span class="rm-prereq-tag">{prereq.split("::")[1]?.replace(/_/g, " ") ?? prereq}</span>
                    {/each}
                  </div>
                {/if}
              </article>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      <p class="state-text">Achievement data is not available yet.</p>
    {/if}
  </div>
</section>

<style>
  .rm-achievement-title {
    position: fixed;
    top: clamp(0.8rem, 1.5vh, 3rem);
    right: clamp(1.2rem, 2.5vw, 5rem);
    z-index: 10;
    pointer-events: none;
  }

  .rm-ach-layout {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 2fr;
    overflow: hidden;
    height: 100%;
  }

  .rm-ach-sidebar {
    overflow-y: auto;
    height: 100%;
    padding: clamp(1.5rem, 2.5vh, 4rem) clamp(1.2rem, 2vw, 3rem) clamp(6rem, 10vh, 10rem) clamp(1.5rem, 2.5vw, 4rem);
    box-sizing: border-box;
  }

  .rm-ach-pack-list {
    display: flex;
    flex-direction: column;
    gap: clamp(0.1rem, 0.15vw, 0.25rem);
  }

  .rm-ach-pack-btn {
    display: block;
    width: fit-content;
    border: none;
    background: transparent;
    cursor: pointer;
    padding: clamp(0.15rem, 0.25vw, 0.4rem) clamp(0.4rem, 0.6vw, 1rem);
    opacity: 0.35;
    transition: opacity 140ms ease;
  }

  .rm-ach-pack-btn:hover {
    opacity: 0.65;
  }

  .rm-ach-pack-btn.is-active {
    opacity: 1;
  }

  .rm-ach-content {
    overflow-y: auto;
    height: 100%;
    padding: clamp(1.5rem, 2.5vh, 4rem) clamp(8rem, 14vw, 20rem) clamp(6rem, 10vh, 10rem) clamp(1.5rem, 2.5vw, 4rem);
    box-sizing: border-box;
  }

  .rm-ach-content-header {
    display: flex;
    align-items: baseline;
    gap: clamp(0.6rem, 1vw, 1.5rem);
    margin-bottom: clamp(1rem, 1.5vw, 2.5rem);
  }

  .rm-ach-stats {
    font-size: clamp(1.1rem, 1.4vw, 2.2rem);
    font-weight: 800;
    letter-spacing: 0.06em;
    color: var(--rm-black);
    -webkit-text-stroke: 0.05em var(--rm-white);
    paint-order: stroke fill;
  }

  .rm-achievement-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(max(240px, 14vw), 1fr));
    gap: clamp(0.6rem, 0.6vw, 1.2rem);
  }

  .rm-achievement-card {
    background: var(--rm-black);
    padding: 0;
    display: flex;
    flex-direction: column;
    transform: rotate(-0.5deg);
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 3% 100%);
    opacity: 0.55;
    transition: opacity 120ms ease;
  }

  .rm-achievement-card:nth-child(even) {
    transform: rotate(0.5deg);
  }

  .rm-achievement-card.is-unlocked {
    opacity: 1;
  }

  .rm-achievement-card-header {
    display: flex;
    align-items: center;
    gap: clamp(0.3rem, 0.4vw, 0.6rem);
    background: var(--rm-white);
    color: var(--rm-black);
    padding: clamp(0.3rem, 0.4vw, 0.7rem) clamp(0.7rem, 0.9vw, 1.6rem);
    margin: clamp(0.2rem, 0.25vw, 0.45rem) clamp(0.2rem, 0.25vw, 0.45rem) 0;
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 1.5% 100%);
  }

  .rm-achievement-status-icon {
    font-size: clamp(0.8rem, 0.7vw, 1.2rem);
    font-weight: 800;
    flex-shrink: 0;
  }

  .rm-achievement-card.is-unlocked .rm-achievement-status-icon {
    color: var(--rm-red);
  }

  .rm-achievement-name {
    font-size: clamp(0.7rem, 0.65vw, 1.2rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    line-height: 1.2;
    flex: 1;
  }

  .rm-difficulty {
    font-size: clamp(0.55rem, 0.5vw, 0.9rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    flex-shrink: 0;
  }

  .rm-difficulty--beginner { opacity: 0.5; }
  .rm-difficulty--intermediate { opacity: 0.65; }
  .rm-difficulty--advanced { opacity: 0.8; }
  .rm-difficulty--expert { opacity: 0.9; }
  .rm-difficulty--legendary { color: var(--rm-red); opacity: 1; }

  .rm-achievement-desc {
    margin: 0;
    padding: clamp(0.25rem, 0.35vw, 0.6rem) clamp(0.7rem, 0.9vw, 1.6rem) clamp(0.25rem, 0.35vw, 0.6rem) clamp(1.2rem, 1.4vw, 2.4rem);
    font-size: clamp(0.65rem, 0.58vw, 1rem);
    color: rgba(255, 255, 255, 0.7);
    line-height: 1.4;
  }

  .rm-achievement-date {
    margin: 0;
    padding: 0 clamp(0.7rem, 0.9vw, 1.6rem) clamp(0.15rem, 0.2vw, 0.35rem) clamp(1.2rem, 1.4vw, 2.4rem);
    font-size: clamp(0.58rem, 0.5vw, 0.9rem);
    color: var(--rm-red);
    font-weight: 700;
    letter-spacing: 0.04em;
  }

  .rm-achievement-note {
    margin: 0;
    padding: 0 clamp(0.7rem, 0.9vw, 1.6rem) clamp(0.2rem, 0.25vw, 0.4rem) clamp(1.2rem, 1.4vw, 2.4rem);
    font-size: clamp(0.55rem, 0.48vw, 0.85rem);
    color: rgba(255, 255, 255, 0.45);
    font-style: italic;
  }

  .rm-achievement-prereqs {
    display: flex;
    flex-wrap: wrap;
    gap: clamp(0.2rem, 0.25vw, 0.4rem);
    padding: clamp(0.15rem, 0.2vw, 0.35rem) clamp(0.7rem, 0.9vw, 1.6rem) clamp(0.3rem, 0.4vw, 0.6rem) clamp(1.2rem, 1.4vw, 2.4rem);
  }

  .rm-prereq-tag {
    font-size: clamp(0.5rem, 0.45vw, 0.75rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.15);
    padding: 0.1rem 0.4rem;
  }
</style>
