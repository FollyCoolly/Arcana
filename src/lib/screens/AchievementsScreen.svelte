<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import P5Text from "$lib/P5Text.svelte";
  import P5MenuItem from "$lib/P5MenuItem.svelte";
  import type { LetterConfig } from "$lib/P5MenuItem.svelte";
  import type { AchievementData, Achievement, Difficulty, PackAchievements } from "$lib/types/achievement";

  type SortKey = 'default' | 'name' | 'difficulty' | 'unlocked';
  type SortDir = 'asc' | 'desc';

  let { onBack, achievementData: externalData = null, onAchievementDataLoaded }: {
    onBack: () => void;
    achievementData?: AchievementData | null;
    onAchievementDataLoaded?: (data: AchievementData) => void;
  } = $props();

  let achievementLoading = $state(false);
  let achievementError = $state<string | null>(null);
  let achievementData = $state<AchievementData | null>(externalData);
  let selectedPackIndex = $state(0);

  // Sidebar refs for selection quad
  let sidebarRef = $state<HTMLElement | undefined>(undefined);
  let packBtnRefs = $state<(HTMLButtonElement | undefined)[]>([]);

  // Filter & sort state
  let searchQuery = $state('');
  let selectedTags = $state<Set<string>>(new Set());
  let selectedDifficulties = $state<Set<string>>(new Set());
  let showUnlockedOnly = $state(false);
  let sortKey = $state<SortKey>('default');
  let sortDir = $state<SortDir>('asc');

  const DIFFICULTIES: Difficulty[] = ['beginner', 'intermediate', 'advanced', 'expert', 'legendary'];

  const PACK_QUAD_CONFIGS: { rot: number; clip: string }[] = [
    { rot: -8,  clip: 'polygon(3% 5%, 97% 0%, 95% 95%, 1% 100%)' },
    { rot: -4,  clip: 'polygon(1% 8%, 99% 2%, 97% 92%, 3% 98%)' },
    { rot: -1,  clip: 'polygon(2% 0%, 98% 6%, 96% 96%, 0% 88%)' },
    { rot: 1,   clip: 'polygon(0% 6%, 98% 0%, 100% 94%, 2% 100%)' },
    { rot: 3,   clip: 'polygon(1% 4%, 97% 0%, 100% 90%, 3% 96%)' },
    { rot: -2,  clip: 'polygon(0% 8%, 99% 0%, 100% 100%, 2% 92%)' },
  ];

  // Memoized letter configs for pack names
  const packLetterCache = new Map<string, LetterConfig[]>();

  function getPackLetterConfigs(packName: string, packIndex: number): LetterConfig[] {
    const key = `${packIndex}:${packName}`;
    if (packLetterCache.has(key)) return packLetterCache.get(key)!;

    const SIZES = ['0.75em', '0.82em', '0.88em', '0.92em', '1.0em', '1.08em', '1.15em'];
    const OFFSETS = [-3, -2, -1, 0, 1, 2, 3, 4];
    const ROTATES = [-6, -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5];

    const letters: LetterConfig[] = packName.split('').map((char, i) => {
      if (char === ' ') return { char: ' ', size: '0.5em', yOffset: 0, rotate: 0 };
      const seed = packIndex * 37 + i * 13;
      const colorVariant = (seed * 3) % 5;
      return {
        char,
        size: SIZES[seed % SIZES.length],
        yOffset: OFFSETS[(seed * 7) % OFFSETS.length],
        rotate: ROTATES[(seed * 11) % ROTATES.length],
        weight: i === 0 ? 800 : 700,
        color: colorVariant === 0 ? 'black' as const : undefined,
        outline: colorVariant === 0 && (seed % 2 === 0),
        rounded: colorVariant === 0 && (seed % 2 !== 0),
      };
    });

    packLetterCache.set(key, letters);
    return letters;
  }

  // Derived: available tags in selected pack
  let availableTags = $derived.by(() => {
    const pack = achievementData?.packs[selectedPackIndex];
    if (!pack) return [];
    const tagSet = new Set<string>();
    for (const a of pack.achievements) {
      for (const t of a.tags) tagSet.add(t);
    }
    return [...tagSet].sort();
  });

  const DIFFICULTY_ORDER: Record<string, number> = {
    beginner: 0, intermediate: 1, advanced: 2, expert: 3, legendary: 4,
  };

  // Derived: filtered + sorted achievements
  let filteredAchievements = $derived.by((): Achievement[] => {
    const pack = achievementData?.packs[selectedPackIndex];
    if (!pack) return [];
    const q = searchQuery.trim().toLowerCase();
    const filtered = pack.achievements.filter(a => {
      if (q && !a.name.toLowerCase().includes(q) && !a.description.toLowerCase().includes(q)) return false;
      if (selectedTags.size > 0 && !a.tags.some(t => selectedTags.has(t))) return false;
      if (selectedDifficulties.size > 0 && !selectedDifficulties.has(a.difficulty)) return false;
      if (showUnlockedOnly && !achievementData?.progress[a.id]) return false;
      return true;
    });

    if (sortKey === 'default') return filtered;

    const dir = sortDir === 'asc' ? 1 : -1;
    return filtered.toSorted((a, b) => {
      if (sortKey === 'name') return dir * a.name.localeCompare(b.name);
      if (sortKey === 'difficulty') return dir * ((DIFFICULTY_ORDER[a.difficulty] ?? 0) - (DIFFICULTY_ORDER[b.difficulty] ?? 0));
      if (sortKey === 'unlocked') {
        const ua = achievementData?.progress[a.id] ? 1 : 0;
        const ub = achievementData?.progress[b.id] ? 1 : 0;
        return dir * (ub - ua);
      }
      return 0;
    });
  });

  let hasActiveFilters = $derived(
    searchQuery !== '' || selectedTags.size > 0 || selectedDifficulties.size > 0 || showUnlockedOnly || sortKey !== 'default'
  );

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

  function resetFilters() {
    searchQuery = '';
    selectedTags = new Set();
    selectedDifficulties = new Set();
    showUnlockedOnly = false;
    sortKey = 'default';
    sortDir = 'asc';
  }

  function toggleSort(key: SortKey) {
    if (sortKey === key) {
      if (sortDir === 'asc') { sortDir = 'desc'; }
      else { sortKey = 'default'; sortDir = 'asc'; }
    } else {
      sortKey = key;
      sortDir = 'asc';
    }
  }

  function getSortIndicator(key: SortKey): string {
    if (sortKey !== key) return '';
    return sortDir === 'asc' ? ' ▲' : ' ▼';
  }

  function selectPack(index: number) {
    selectedPackIndex = index;
    resetFilters();
  }

  function toggleTag(tag: string) {
    const next = new Set(selectedTags);
    if (next.has(tag)) next.delete(tag); else next.add(tag);
    selectedTags = next;
  }

  function toggleDifficulty(d: string) {
    const next = new Set(selectedDifficulties);
    if (next.has(d)) next.delete(d); else next.add(d);
    selectedDifficulties = next;
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

  // Selection quad effect for sidebar
  $effect(() => {
    const idx = selectedPackIndex;
    const btn = packBtnRefs[idx];
    const container = sidebarRef;
    if (!btn || !container) return;

    const btnRect = btn.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();

    const centerX = btnRect.left + btnRect.width / 2 - containerRect.left;
    const centerY = btnRect.top + btnRect.height / 2 - containerRect.top;

    const quadW = btn.offsetWidth * 1.5;
    const quadH = btn.offsetHeight * 1.3;
    const cfg = PACK_QUAD_CONFIGS[idx % PACK_QUAD_CONFIGS.length];

    container.style.setProperty('--pack-quad-x', `${centerX - quadW / 2}px`);
    container.style.setProperty('--pack-quad-y', `${centerY - quadH / 2}px`);
    container.style.setProperty('--pack-quad-w', `${quadW}px`);
    container.style.setProperty('--pack-quad-h', `${quadH}px`);
    container.style.setProperty('--pack-quad-rot', `${cfg.rot}deg`);
    container.style.setProperty('--pack-quad-clip', cfg.clip);
  });

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
      <!-- LEFT: Pack nav (main menu style) -->
      <nav class="rm-ach-sidebar" bind:this={sidebarRef}>
        <ul class="rm-ach-pack-list">
          {#each achievementData.packs as pack, pi}
            <li
              class="rm-ach-pack-line"
              style:z-index={pi === selectedPackIndex ? 10 : 0}
            >
              <button
                type="button"
                class="rm-ach-pack-btn"
                class:is-active={pi === selectedPackIndex}
                onclick={() => selectPack(pi)}
                onmouseenter={() => { selectedPackIndex = pi; resetFilters(); }}
                bind:this={packBtnRefs[pi]}
              >
                <P5MenuItem
                  letters={getPackLetterConfigs(pack.pack_name, pi)}
                  active={pi === selectedPackIndex}
                />
              </button>
              <!-- Tag chips expand under selected pack -->
              {#if pi === selectedPackIndex && availableTags.length > 0}
                <div class="rm-ach-sidebar-tags">
                  {#each availableTags as tag}
                    <button
                      type="button"
                      class="rm-ach-chip rm-ach-chip--tag"
                      class:is-active={selectedTags.has(tag)}
                      onclick={() => toggleTag(tag)}
                    >
                      {tag}
                    </button>
                  {/each}
                </div>
              {/if}
            </li>
          {/each}
        </ul>
        <div class="rm-ach-pack-quad" aria-hidden="true"></div>
      </nav>

      <!-- RIGHT: Achievement content -->
      <div class="rm-ach-content">
        {#if getSelectedPack()}
          {@const pack = getSelectedPack()!}
          {@const stats = getPackStats(pack)}

          <div class="rm-ach-content-header">
            <P5Text text={pack.pack_name} fontSize={52} />
            <span class="rm-ach-stats">{stats.unlocked} / {stats.total}</span>
          </div>

          <!-- FILTER BAR -->
          <div class="rm-ach-filters">
            <!-- Row 1: Search + Sort buttons -->
            <div class="rm-ach-filter-row">
              <div class="rm-ach-search-wrap">
                <span class="rm-ach-search-icon" aria-hidden="true">&#x25C8;</span>
                <input
                  type="search"
                  class="rm-ach-search"
                  placeholder="SEARCH..."
                  bind:value={searchQuery}
                  autocomplete="off"
                  spellcheck="false"
                />
                {#if searchQuery}
                  <button type="button" class="rm-ach-search-clear" onclick={() => { searchQuery = ''; }}>&#x2715;</button>
                {/if}
              </div>

              <span class="rm-ach-filter-label">SORT</span>
              <button type="button" class="rm-ach-chip" class:is-active={sortKey === 'name'} onclick={() => toggleSort('name')}>
                Name{getSortIndicator('name')}
              </button>
              <button type="button" class="rm-ach-chip" class:is-active={sortKey === 'difficulty'} onclick={() => toggleSort('difficulty')}>
                Diff{getSortIndicator('difficulty')}
              </button>
              <button type="button" class="rm-ach-chip" class:is-active={sortKey === 'unlocked'} onclick={() => toggleSort('unlocked')}>
                Unlock{getSortIndicator('unlocked')}
              </button>
            </div>

            <!-- Row 2: Difficulty chips + Unlocked toggle -->
            <div class="rm-ach-filter-row">
              <span class="rm-ach-filter-label">DIFF</span>
              {#each DIFFICULTIES as diff}
                <button
                  type="button"
                  class="rm-ach-chip rm-ach-chip--diff rm-ach-chip--{diff}"
                  class:is-active={selectedDifficulties.has(diff)}
                  onclick={() => toggleDifficulty(diff)}
                >
                  {getDifficultyLabel(diff)}
                </button>
              {/each}

              <span class="rm-ach-filter-divider"></span>

              <button
                type="button"
                class="rm-ach-chip is-active"
                onclick={() => { showUnlockedOnly = !showUnlockedOnly; }}
              >
                {showUnlockedOnly ? 'UNLOCKED' : 'ALL'}
              </button>
            </div>

            <!-- Filter meta -->
            <div class="rm-ach-filter-meta">
              <span class="rm-ach-filter-result">{filteredAchievements.length} / {stats.total}</span>
              {#if hasActiveFilters}
                <button type="button" class="rm-ach-filter-clear" onclick={resetFilters}>
                  CLEAR ALL
                </button>
              {/if}
            </div>
          </div>

          <!-- Achievement grid -->
          {#if filteredAchievements.length === 0}
            <p class="rm-ach-empty">No achievements match the current filters.</p>
          {:else}
            <div class="rm-achievement-grid">
              {#each filteredAchievements as achievement}
                {@const unlocked = achievementData!.progress[achievement.id]}
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

  /* ── Sidebar (main menu style) ── */

  .rm-ach-sidebar {
    position: relative;
    overflow-y: auto;
    height: 100%;
    padding: clamp(1.5rem, 2.5vh, 4rem) clamp(1.2rem, 2vw, 3rem) clamp(6rem, 10vh, 10rem) clamp(1.5rem, 2.5vw, 4rem);
    box-sizing: border-box;
  }

  .rm-ach-pack-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .rm-ach-pack-line {
    margin: -0.6rem 0;
    position: relative;
  }

  .rm-ach-pack-line:nth-child(odd)  { margin-left: 0; }
  .rm-ach-pack-line:nth-child(even) { margin-left: 1.5vw; }

  .rm-ach-pack-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    border: none;
    background: var(--rm-black);
    cursor: pointer;
    padding: 0.55rem 1.4rem 0.55rem 1.2rem;
    width: fit-content;
    transition: background-color 140ms ease;
  }

  .rm-ach-pack-btn:not(.is-active):hover {
    background: var(--rm-red);
  }

  .rm-ach-pack-btn.is-active {
    background: var(--rm-red);
  }

  /* Per-item rotation + clip-path */
  .rm-ach-pack-line:nth-child(6n+1) .rm-ach-pack-btn { transform: rotate(-5deg); clip-path: polygon(0% 8%, 100% 0%, 98% 92%, 2% 100%); }
  .rm-ach-pack-line:nth-child(6n+2) .rm-ach-pack-btn { transform: rotate(-3deg); clip-path: polygon(1% 5%, 99% 0%, 97% 96%, 0% 100%); }
  .rm-ach-pack-line:nth-child(6n+3) .rm-ach-pack-btn { transform: rotate(-1deg); clip-path: polygon(2% 0%, 100% 4%, 96% 100%, 0% 92%); }
  .rm-ach-pack-line:nth-child(6n+4) .rm-ach-pack-btn { transform: rotate(1deg);  clip-path: polygon(0% 6%, 98% 0%, 100% 94%, 3% 100%); }
  .rm-ach-pack-line:nth-child(6n+5) .rm-ach-pack-btn { transform: rotate(2deg);  clip-path: polygon(1% 0%, 97% 4%, 99% 100%, 2% 96%); }
  .rm-ach-pack-line:nth-child(6n+6) .rm-ach-pack-btn { transform: rotate(-2deg); clip-path: polygon(0% 4%, 100% 0%, 98% 96%, 1% 100%); }

  .rm-ach-pack-btn :global(.p5m) {
    font-size: clamp(1.8rem, 3.5vw, 2.8rem);
  }

  .rm-ach-pack-quad {
    position: absolute;
    left: var(--pack-quad-x);
    top: var(--pack-quad-y);
    width: var(--pack-quad-w);
    height: var(--pack-quad-h);
    transform: rotate(var(--pack-quad-rot));
    z-index: 15;
    background: var(--rm-red);
    mix-blend-mode: difference;
    clip-path: var(--pack-quad-clip, polygon(0% 0%, 100% 0%, 100% 100%, 0% 100%));
    pointer-events: none;
    transition: left 120ms ease, top 120ms ease, width 120ms ease, height 120ms ease,
                transform 120ms ease, clip-path 120ms ease;
  }

  /* ── Content area ── */

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

  /* ── Filter bar ── */

  .rm-ach-filters {
    display: flex;
    flex-direction: column;
    gap: clamp(0.5rem, 0.6vw, 1rem);
    margin-bottom: clamp(1rem, 1.2vw, 2rem);
    padding-bottom: clamp(0.8rem, 1vw, 1.6rem);
    border-bottom: 2px solid rgba(255, 255, 255, 0.08);
  }

  .rm-ach-search-wrap {
    display: flex;
    align-items: center;
    background: var(--rm-black);
    border: 2px solid rgba(255, 255, 255, 0.15);
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 2% 100%);
    transform: skewX(-3deg);
    width: fit-content;
    min-width: clamp(160px, 20vw, 360px);
  }

  .rm-ach-search-icon {
    padding: 0 0.5rem 0 clamp(0.5rem, 0.7vw, 1.2rem);
    font-size: clamp(0.7rem, 0.65vw, 1.1rem);
    color: var(--rm-red);
    flex-shrink: 0;
    transform: skewX(3deg);
  }

  .rm-ach-search {
    background: transparent;
    border: none;
    outline: none;
    color: var(--rm-white);
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(0.7rem, 0.65vw, 1.1rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: clamp(0.3rem, 0.4vw, 0.6rem) clamp(0.5rem, 0.7vw, 1rem) clamp(0.3rem, 0.4vw, 0.6rem) 0;
    width: clamp(120px, 15vw, 260px);
    transform: skewX(3deg);
  }

  .rm-ach-search::-webkit-search-cancel-button { display: none; }

  .rm-ach-search::placeholder {
    color: rgba(255, 255, 255, 0.25);
  }

  .rm-ach-search-clear {
    background: none;
    border: none;
    color: rgba(255, 255, 255, 0.45);
    cursor: pointer;
    padding: 0 clamp(0.4rem, 0.5vw, 0.8rem);
    font-size: clamp(0.6rem, 0.55vw, 0.9rem);
    transform: skewX(3deg);
    transition: color 120ms ease;
  }

  .rm-ach-search-clear:hover {
    color: var(--rm-red);
  }

  .rm-ach-filter-row {
    display: flex;
    align-items: center;
    gap: clamp(0.2rem, 0.3vw, 0.5rem);
    flex-wrap: wrap;
  }

  .rm-ach-filter-divider {
    width: 2px;
    height: clamp(0.8rem, 0.8vw, 1.4rem);
    background: rgba(255, 255, 255, 0.12);
    flex-shrink: 0;
    margin: 0 clamp(0.15rem, 0.2vw, 0.35rem);
  }

  .rm-ach-filter-label {
    font-size: clamp(0.52rem, 0.46vw, 0.8rem);
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--rm-red);
    flex-shrink: 0;
    padding-right: clamp(0.3rem, 0.4vw, 0.6rem);
    border-right: 2px solid var(--rm-red);
    margin-right: clamp(0.2rem, 0.25vw, 0.4rem);
    align-self: center;
  }

  .rm-ach-chip {
    border: 1px solid rgba(255, 255, 255, 0.2);
    background: var(--rm-black);
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
    padding: clamp(0.15rem, 0.2vw, 0.35rem) clamp(0.45rem, 0.55vw, 0.9rem);
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(0.55rem, 0.5vw, 0.85rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    clip-path: polygon(4% 0%, 100% 0%, 96% 100%, 0% 100%);
    transform: skewX(-3deg);
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
    flex-shrink: 0;
  }

  .rm-ach-chip:hover {
    border-color: rgba(255, 255, 255, 0.45);
    color: rgba(255, 255, 255, 0.8);
  }

  .rm-ach-chip.is-active {
    background: var(--rm-red);
    color: var(--rm-white);
    border-color: var(--rm-red);
  }

  /* ── Sidebar tag chips ── */

  .rm-ach-sidebar-tags {
    display: flex;
    flex-wrap: wrap;
    gap: clamp(0.15rem, 0.2vw, 0.35rem);
    padding: clamp(0.4rem, 0.5vw, 0.8rem) 0 clamp(0.2rem, 0.3vw, 0.5rem) clamp(0.8rem, 1vw, 1.5rem);
    max-height: clamp(5rem, 8vh, 10rem);
    overflow-y: auto;
  }

  .rm-ach-sidebar-tags::-webkit-scrollbar { width: 3px; }
  .rm-ach-sidebar-tags::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.2); }

  .rm-ach-filter-meta {
    display: flex;
    align-items: center;
    gap: clamp(0.5rem, 0.7vw, 1.2rem);
  }

  .rm-ach-filter-result {
    font-size: clamp(0.58rem, 0.52vw, 0.9rem);
    font-weight: 800;
    color: rgba(255, 255, 255, 0.35);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .rm-ach-filter-clear {
    border: none;
    background: transparent;
    color: var(--rm-red);
    cursor: pointer;
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(0.52rem, 0.46vw, 0.8rem);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: 0;
    opacity: 0.7;
    transition: opacity 120ms ease;
  }

  .rm-ach-filter-clear:hover {
    opacity: 1;
  }

  .rm-ach-empty {
    margin: clamp(1.5rem, 2vw, 3rem) 0;
    font-size: clamp(0.7rem, 0.65vw, 1.1rem);
    color: rgba(255, 255, 255, 0.35);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-weight: 700;
  }

  /* ── Achievement grid ── */

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
