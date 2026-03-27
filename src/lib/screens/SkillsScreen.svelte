<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import P5Text from "$lib/P5Text.svelte";
  import SkillNebula from "$lib/components/SkillNebula.svelte";
  import type { SkillData, SkillWithLevel, SkillNode } from "$lib/types/skill";
  import type { AchievementData } from "$lib/types/achievement";
  import { formatGroupName } from "$lib/utils/format";

  let { onBack, achievementData }: {
    onBack: () => void;
    achievementData: AchievementData | null;
  } = $props();

  let skillLoading = $state(false);
  let skillError = $state<string | null>(null);
  let skillData = $state<SkillData | null>(null);
  let selectedSkill = $state<SkillWithLevel | null>(null);

  const ROMAN_NUMERALS = ["0", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"];

  function toRomanNumeral(n: number): string {
    return ROMAN_NUMERALS[n] ?? String(n);
  }

  function getSkillProgressPercent(s: SkillWithLevel): number {
    if (s.max_points === 0) return 0;
    return (s.current_points / s.max_points) * 100;
  }

  function isNodeUnlocked(achievementId: string): boolean {
    return !!achievementData?.progress[achievementId];
  }

  function getAchievementName(achievementId: string): string {
    if (!achievementData) return achievementId;
    for (const pack of achievementData.packs) {
      for (const ach of pack.achievements) {
        if (ach.id === achievementId) return ach.name;
      }
    }
    const after = achievementId.split("::")[1];
    return after ? formatGroupName(after) : achievementId;
  }

  function computeHexRows(nodes: SkillNode[], cols: number): SkillNode[][] {
    const rows: SkillNode[][] = [];
    let idx = 0;
    let rowIdx = 0;
    while (idx < nodes.length) {
      const rowCols = (rowIdx % 2 === 0) ? cols : cols - 1;
      rows.push(nodes.slice(idx, idx + rowCols));
      idx += rowCols;
      rowIdx++;
    }
    return rows;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      if (selectedSkill) {
        selectedSkill = null;
      } else {
        onBack();
      }
    }
  }

  async function loadSkillData() {
    skillLoading = true;
    skillError = null;

    try {
      skillData = await invoke<SkillData>("load_skills");
    } catch (error) {
      skillError =
        typeof error === "string"
          ? error
          : "Failed to load skill data.";
      skillData = null;
    } finally {
      skillLoading = false;
    }
  }

  onMount(() => {
    if (!skillData && !skillLoading) {
      void loadSkillData();
    }

    window.addEventListener("keydown", handleKeydown);
    return () => {
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

<section class="rm-stage">
  <div class="rm-skills-title">
    <P5Text text="Skills" fontSize={82} />
  </div>

  <button type="button" class="rm-back-btn" onclick={() => {
    if (selectedSkill) {
      selectedSkill = null;
    } else {
      onBack();
    }
  }}>
    <img src="/ui/back.png" alt="Back" class="rm-back-img" />
  </button>

  {#if skillLoading}
    <p class="state-text" style="padding: 2rem;">Loading skills...</p>
  {:else if skillError}
    <p class="state-text error" style="padding: 2rem;">{skillError}</p>
  {:else if skillData && !selectedSkill}
    <div class="rm-nebula-container">
      <SkillNebula
        skills={skillData.skills}
        onCardClick={(skill) => { selectedSkill = skill; }}
      />
    </div>
  {:else if skillData && selectedSkill}
    <div class="rm-skill-detail">
      <div class="rm-skill-detail-left">
        <div class="rm-tarot-card rm-tarot-card--large" class:rm-tarot-card--leveled={selectedSkill.current_level > 0}>
          <div class="rm-tarot-card-inner">
            <div class="rm-tarot-top">
              <span class="rm-tarot-level">{toRomanNumeral(selectedSkill.current_level)}</span>
              <span class="rm-tarot-pack">{selectedSkill.pack_name}</span>
            </div>
            <div class="rm-tarot-art">
              <div class="rm-tarot-star-stack">
                <div class="rm-tarot-star rm-ts-1"></div>
                <div class="rm-tarot-star rm-ts-2"></div>
                <div class="rm-tarot-star rm-ts-3"></div>
                <div class="rm-tarot-star rm-ts-4"></div>
                <div class="rm-tarot-star rm-ts-5"></div>
              </div>
              <div class="rm-tarot-stripe"></div>
            </div>
            <div class="rm-tarot-name-strip">
              <span class="rm-tarot-name">{selectedSkill.skill.name}</span>
            </div>
            <div class="rm-tarot-bottom">
              <div class="rm-tarot-progress">
                <div class="rm-tarot-progress-fill" style:width="{getSkillProgressPercent(selectedSkill)}%"></div>
              </div>
              <span class="rm-tarot-lv">LV {selectedSkill.current_level}</span>
            </div>
          </div>
        </div>

        <div class="rm-skill-stats">
          <div class="rm-skill-stat-row">
            <span class="rm-skill-stat-label">LEVEL</span>
            <span class="rm-skill-stat-value">{selectedSkill.current_level} / {selectedSkill.skill.max_level}</span>
          </div>
          <div class="rm-skill-stat-row">
            <span class="rm-skill-stat-label">POINTS</span>
            <span class="rm-skill-stat-value">{selectedSkill.current_points} / {selectedSkill.max_points}</span>
          </div>
          {#if selectedSkill.next_threshold}
            <div class="rm-skill-stat-row">
              <span class="rm-skill-stat-label">NEXT LV</span>
              <span class="rm-skill-stat-value">{selectedSkill.next_threshold.points_required} pts</span>
            </div>
          {/if}
        </div>

        {#if selectedSkill.skill.description}
          <p class="rm-skill-description">{selectedSkill.skill.description}</p>
        {/if}
      </div>

      <div class="rm-skill-detail-right">
        <div class="rm-skill-detail-header">
          <P5Text text={selectedSkill.skill.name} fontSize={52} />
        </div>

        <div class="rm-skill-node-grid" style="--cols: 8">
          {#each computeHexRows(selectedSkill.skill.nodes, 8) as row, rowIdx}
            <div class="rm-hex-row" class:rm-hex-row--odd={rowIdx % 2 === 1}>
              {#each row as node}
                {@const unlocked = isNodeUnlocked(node.achievement_id)}
                <div class="rm-hex-border">
                  <div
                    class="rm-skill-node-hex"
                    class:rm-skill-node-hex--unlocked={unlocked}
                  >
                    <span class="rm-node-status">{unlocked ? "✓" : "○"}</span>
                    <span class="rm-node-name">{getAchievementName(node.achievement_id)}</span>
                    <span class="rm-node-points">+{node.points}</span>
                  </div>
                </div>
              {/each}
            </div>
          {/each}
        </div>
      </div>
    </div>
  {:else}
    <p class="state-text" style="padding: 2rem;">Skill data is not available yet.</p>
  {/if}
</section>

<style>
  .rm-skills-title {
    position: fixed;
    top: clamp(0.8rem, 1.5vh, 3rem);
    right: clamp(1.2rem, 2.5vw, 5rem);
    z-index: 10;
    pointer-events: none;
  }

  .rm-nebula-container {
    position: absolute;
    inset: 0;
    overflow: hidden;
    animation: rm-nebula-fade-in 400ms ease forwards;
  }

  @keyframes rm-nebula-fade-in {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  .rm-skill-detail {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: clamp(1.5rem, 2vw, 3rem);
    overflow: hidden;
    height: 100%;
    padding: clamp(1.5rem, 2.5vh, 4rem) clamp(2rem, 3vw, 5rem) clamp(6rem, 10vh, 10rem);
    box-sizing: border-box;
  }

  .rm-skill-detail-left {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: clamp(0.8rem, 1vw, 1.5rem);
    overflow-y: auto;
    padding: 0 clamp(1rem, 2vw, 3rem);
  }

  .rm-skill-stats {
    width: clamp(400px, 27.5vw, 625px);
    display: flex;
    flex-direction: column;
    gap: clamp(0.2rem, 0.3vw, 0.5rem);
  }

  .rm-skill-stat-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: clamp(0.15rem, 0.2vw, 0.35rem) 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .rm-skill-stat-label {
    font-size: clamp(0.6rem, 0.55vw, 0.95rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: rgba(255, 255, 255, 0.5);
  }

  .rm-skill-stat-value {
    font-size: clamp(0.75rem, 0.7vw, 1.2rem);
    font-weight: 800;
    letter-spacing: 0.04em;
  }

  .rm-skill-description {
    margin: 0;
    width: clamp(400px, 27.5vw, 625px);
    font-size: clamp(0.6rem, 0.55vw, 0.95rem);
    color: rgba(255, 255, 255, 0.55);
    line-height: 1.5;
  }

  .rm-skill-detail-right {
    overflow-y: auto;
    padding-right: clamp(2rem, 4vw, 8rem);
  }

  .rm-skill-detail-header {
    margin-bottom: clamp(1rem, 1.5vw, 2.5rem);
  }

  .rm-skill-node-grid {
    --hex-w: clamp(80px, 6.5vw, 180px);
    --hex-h: calc(var(--hex-w) * 1.1547);
    --cols: 8;
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    width: calc(var(--hex-w) * var(--cols) + var(--hex-w) / 2);
    padding-bottom: calc(var(--hex-h) * 0.25);
  }

  .rm-hex-row {
    display: flex;
    width: 100%;
  }

  .rm-hex-row:not(:first-child) {
    margin-top: calc(var(--hex-h) * -0.25);
  }

  .rm-hex-row--odd {
    padding-left: calc(var(--hex-w) / 2);
  }

  .rm-hex-border {
    width: var(--hex-w);
    height: var(--hex-h);
    clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%);
    background: var(--rm-white);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .rm-skill-node-hex {
    width: calc(100% - 10px);
    height: calc(100% - 10px);
    clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%);
    background: var(--rm-black);
    color: var(--rm-white);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: clamp(0.1rem, 0.2vw, 0.3rem);
    padding: clamp(0.4rem, 0.5vw, 0.8rem) clamp(0.8rem, 1vw, 1.4rem);
    transition: background 150ms ease, color 150ms ease;
  }

  .rm-skill-node-hex--unlocked {
    background: var(--rm-red);
    color: var(--rm-black);
  }

  .rm-node-status {
    font-size: clamp(0.9rem, 1vw, 1.6rem);
    font-weight: 800;
  }

  .rm-node-name {
    font-size: clamp(0.55rem, 0.7vw, 1rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    text-align: center;
    line-height: 1.2;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .rm-node-points {
    font-size: clamp(0.55rem, 0.7vw, 1rem);
    font-weight: 800;
    opacity: 0.7;
  }

  .rm-skill-node-hex--unlocked .rm-node-points {
    opacity: 1;
  }
</style>
