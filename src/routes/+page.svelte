<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import P5Text from "$lib/P5Text.svelte";
  import P5Calendar from "$lib/P5Calendar.svelte";
  import P5MenuItem from "$lib/P5MenuItem.svelte";
  import type { LetterConfig } from "$lib/P5MenuItem.svelte";
  import type { AchievementData, Achievement, PackAchievements } from "$lib/types/achievement";

  type StatusMetric = {
    id: string;
    name: string;
    category: "health" | "performance" | string;
    group: string;
    sub_group?: string;
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

  type MenuScreen = "main" | "status" | "achievements";
  type MenuItemId = "status" | "skills" | "achievements" | "items" | "gallery" | "crafting";

  type MenuItem = {
    id: MenuItemId;
    label: string;
    description: string;
    enabled: boolean;
  };

  const MENU_ITEMS: MenuItem[] = [
    {
      id: "status",
      label: "Status",
      description: "Body and life metrics from local JSON snapshots.",
      enabled: true,
    },
    {
      id: "skills",
      label: "Skills",
      description: "Skill tree progression linked to achievements.",
      enabled: false,
    },
    {
      id: "achievements",
      label: "Achievements",
      description: "Milestones and timeline tracking.",
      enabled: true,
    },
    {
      id: "items",
      label: "Items",
      description: "Personal inventory and purchase awareness.",
      enabled: false,
    },
    {
      id: "gallery",
      label: "Gallery",
      description: "Books, media, and games aggregation hub.",
      enabled: false,
    },
    {
      id: "crafting",
      label: "Crafting",
      description: "Recipe and material planning module.",
      enabled: false,
    },
  ];

  const MENU_LETTER_DATA: Record<MenuItemId, LetterConfig[]> = {
    // Uppercase: 0.70–1.20em  |  Lowercase: 0.75–1.00em
    status: [
      { char: 'S', size: '1.18em', yOffset: -3, rotate: -6, weight: 800 },
      { char: 't', size: '0.82em', yOffset: 4, rotate: 4, color: 'black', outline: true },
      { char: 'A', size: '0.85em', yOffset: 1, rotate: -2 },
      { char: 't', size: '0.92em', yOffset: -1, rotate: 5, color: 'black', rounded: true },
      { char: 'U', size: '0.7em', yOffset: 3, rotate: -4 },
      { char: 's', size: '0.78em', yOffset: -2, rotate: 6, color: 'black' },
    ],
    skills: [
      { char: 'S', size: '1.15em', yOffset: -2, rotate: -4, weight: 800 },
      { char: 'K', size: '0.78em', yOffset: 3, rotate: 5 },
      { char: 'i', size: '0.88em', yOffset: -1, rotate: -3, color: 'black', rounded: true },
      { char: 'L', size: '1.1em',  yOffset: 2, rotate: 4 },
      { char: 'l', size: '0.80em', yOffset: -2, rotate: -5, color: 'black', outline: true },
      { char: 'S', size: '0.76em', yOffset: 1, rotate: 3 },
    ],
    achievements: [
      { char: 'A', size: '1.18em', yOffset: -3, rotate: -5, weight: 800 },
      { char: 'c', size: '0.82em', yOffset: 3, rotate: 4, color: 'black' },
      { char: 'H', size: '1.0em',  yOffset: -1, rotate: -3 },
      { char: 'i', size: '0.88em', yOffset: 2, rotate: 5, color: 'black', outline: true },
      { char: 'E', size: '0.82em', yOffset: -2, rotate: -2 },
      { char: 'v', size: '0.95em', yOffset: 4, rotate: 3, color: 'black', rounded: true },
      { char: 'E', size: '1.12em', yOffset: -1, rotate: -4 },
      { char: 'M', size: '0.75em', yOffset: 2, rotate: 2 },
      { char: 'e', size: '0.78em', yOffset: -3, rotate: -3, color: 'black' },
      { char: 'N', size: '1.1em',  yOffset: 1, rotate: 5 },
      { char: 't', size: '0.92em', yOffset: -2, rotate: -4, color: 'black', outline: true },
      { char: 'S', size: '0.88em', yOffset: 3, rotate: 3 },
    ],
    items: [
      { char: 'I', size: '1.15em', yOffset: -2, rotate: -5, weight: 800 },
      { char: 't', size: '0.85em', yOffset: 3, rotate: 4, color: 'black', outline: true },
      { char: 'E', size: '0.80em', yOffset: -1, rotate: -3 },
      { char: 'm', size: '0.97em', yOffset: 2, rotate: 5, color: 'black', rounded: true },
      { char: 'S', size: '1.08em', yOffset: -3, rotate: -4 },
    ],
    gallery: [
      { char: 'G', size: '1.18em', yOffset: -3, rotate: -6, weight: 800 },
      { char: 'a', size: '0.88em', yOffset: 4, rotate: 3, color: 'black', rounded: true },
      { char: 'L', size: '0.78em', yOffset: -1, rotate: -4 },
      { char: 'l', size: '0.76em', yOffset: 2, rotate: 5, color: 'black', outline: true },
      { char: 'E', size: '1.1em',  yOffset: -2, rotate: -3 },
      { char: 'r', size: '0.93em', yOffset: 3, rotate: 4, color: 'black' },
      { char: 'Y', size: '1.02em', yOffset: -1, rotate: -5 },
    ],
    crafting: [
      { char: 'C', size: '1.16em', yOffset: -2, rotate: -5, weight: 800 },
      { char: 'r', size: '0.80em', yOffset: 3, rotate: 4, color: 'black', outline: true },
      { char: 'A', size: '1.00em', yOffset: -1, rotate: -3 },
      { char: 'f', size: '0.90em', yOffset: 2, rotate: 5, color: 'black', rounded: true },
      { char: 'T', size: '0.95em', yOffset: -3, rotate: -4 },
      { char: 'i', size: '0.85em', yOffset: 1, rotate: 3, color: 'black' },
      { char: 'N', size: '0.8em', yOffset: -2, rotate: -5 },
      { char: 'g', size: '0.78em', yOffset: 4, rotate: 4, color: 'black', outline: true },
    ],
  };

  const DEFAULT_FOCUS_INDEX = Math.max(0, MENU_ITEMS.findIndex((item) => item.enabled));

  const STRENGTH_SUBGROUP_ORDER = ["chest", "back", "shoulders", "biceps", "triceps", "legs", "core"];

  let currentScreen = $state<MenuScreen>("main");
  let focusedMenuIndex = $state(DEFAULT_FOCUS_INDEX);

  let loading = $state(false);
  let errorMessage = $state<string | null>(null);
  let statusData = $state<StatusData | null>(null);
  let menuFeedback = $state<string | null>(null);

  let achievementLoading = $state(false);
  let achievementError = $state<string | null>(null);
  let achievementData = $state<AchievementData | null>(null);
  let selectedPackIndex = $state(0);

  let commandRef = $state<HTMLElement | undefined>(undefined);
  let menuItemRefs = $state<(HTMLButtonElement | undefined)[]>([]);

  let menuFeedbackTimer: ReturnType<typeof setTimeout> | null = null;
  let unlistenSummonEvent: UnlistenFn | null = null;

  // Per-item quad config: rotation, clip-path shape
  const MENU_QUAD_CONFIGS: { rot: number; clip: string }[] = [
    { rot: -35, clip: 'polygon(10% 25%, 70% 0%, 95% 99%, 10% 80%)' },   // Status
    { rot: -27, clip: 'polygon(1% 8%, 98% 2%, 97% 92%, 3% 98%)' },     // Skills
    { rot: -20, clip: 'polygon(2% 0%, 99% 6%, 96% 96%, 0% 88%)' },     // Achievements
    { rot: -8,  clip: 'polygon(0% 10%, 100% 3%, 98% 94%, 2% 100%)' },  // Items
    { rot: -2,  clip: 'polygon(1% 4%, 97% 0%, 100% 90%, 3% 96%)' },    // Gallery
    { rot: 2,   clip: 'polygon(0% 6%, 98% 0%, 100% 100%, 2% 92%)' },   // Crafting
  ];

  $effect(() => {
    const idx = focusedMenuIndex;
    const btn = menuItemRefs[idx];
    const container = commandRef;
    if (!btn || !container) return;

    const btnRect = btn.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();

    // Center of the focused item relative to the container
    const centerX = btnRect.left + btnRect.width / 2 - containerRect.left;
    const centerY = btnRect.top + btnRect.height / 2 - containerRect.top;

    const quadW = btn.offsetWidth * 1.6;
    const quadH = btn.offsetHeight * 1.4;
    const cfg = MENU_QUAD_CONFIGS[idx] ?? { rot: 0, clip: 'polygon(0% 0%, 100% 0%, 100% 100%, 0% 100%)' };

    container.style.setProperty('--quad-x', `${centerX - quadW / 2}px`);
    container.style.setProperty('--quad-y', `${centerY - quadH / 2}px`);
    container.style.setProperty('--quad-w', `${quadW}px`);
    container.style.setProperty('--quad-h', `${quadH}px`);
    container.style.setProperty('--quad-rot', `${cfg.rot}deg`);
    container.style.setProperty('--quad-clip', cfg.clip);
  });

  function resetToMainMenu() {
    currentScreen = "main";
    focusedMenuIndex = DEFAULT_FOCUS_INDEX;
    menuFeedback = null;
  }

  function setMenuFeedback(message: string) {
    menuFeedback = message;

    if (menuFeedbackTimer) {
      clearTimeout(menuFeedbackTimer);
    }

    menuFeedbackTimer = setTimeout(() => {
      menuFeedback = null;
      menuFeedbackTimer = null;
    }, 1600);
  }

  function isMenuItemSelectable(item: MenuItem | undefined): boolean {
    return !!item && item.enabled;
  }

  function setFocusedMenuIndex(index: number) {
    if (isMenuItemSelectable(MENU_ITEMS[index])) {
      focusedMenuIndex = index;
    }
  }

  function moveMenuFocus(offset: number) {
    const itemCount = MENU_ITEMS.length;
    if (itemCount === 0) {
      return;
    }

    let nextIndex = focusedMenuIndex;
    for (let i = 0; i < itemCount; i += 1) {
      nextIndex = (nextIndex + offset + itemCount) % itemCount;
      if (isMenuItemSelectable(MENU_ITEMS[nextIndex])) {
        focusedMenuIndex = nextIndex;
        return;
      }
    }
  }

  async function hideInterface() {
    resetToMainMenu();
    try {
      await getCurrentWindow().hide();
    } catch (error) {
      setMenuFeedback("Hide failed");
      console.error("Failed to hide window", error);
    }
  }

  function formatGroupName(group: string): string {
    return group
      .split("_")
      .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
      .join(" ");
  }

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
    // append any remaining unlisted sub_groups
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

  async function openStatusScreen() {
    currentScreen = "status";
    if (!statusData && !loading) {
      await loadStatusData();
    }
  }

  async function loadAchievementData() {
    achievementLoading = true;
    achievementError = null;

    try {
      achievementData = await invoke<AchievementData>("load_achievements");
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

  async function openAchievementsScreen() {
    currentScreen = "achievements";
    if (!achievementData && !achievementLoading) {
      await loadAchievementData();
    }
  }

  type CategoryGroup = {
    category: string;
    achievements: Achievement[];
  };

  function getPackCategories(pack: PackAchievements): CategoryGroup[] {
    const groups = new Map<string, Achievement[]>();

    for (const achievement of pack.achievements) {
      const list = groups.get(achievement.category) ?? [];
      list.push(achievement);
      groups.set(achievement.category, list);
    }

    return Array.from(groups.entries()).map(([category, achievements]) => ({
      category,
      achievements,
    }));
  }

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

  async function activateMenuItem(index: number) {
    if (currentScreen !== "main") {
      return;
    }

    const item = MENU_ITEMS[index];
    if (!item) {
      return;
    }

    if (!isMenuItemSelectable(item)) {
      return;
    }

    focusedMenuIndex = index;

    if (item.id === "status") {
      await openStatusScreen();
    } else if (item.id === "achievements") {
      await openAchievementsScreen();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      if (currentScreen === "status" || currentScreen === "achievements") {
        currentScreen = "main";
      } else {
        void hideInterface();
      }
      return;
    }

    if (currentScreen !== "main") {
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      moveMenuFocus(1);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      moveMenuFocus(-1);
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      void activateMenuItem(focusedMenuIndex);
    }
  }

  onMount(() => {
    const appWindow = getCurrentWindow();

    window.addEventListener("keydown", handleKeydown);

    if (!statusData) {
      void loadStatusData();
    }

    appWindow.listen("reality://summoned", () => {
      resetToMainMenu();
    }).then((unlisten) => {
      unlistenSummonEvent = unlisten;
    });

    return () => {
      window.removeEventListener("keydown", handleKeydown);
      if (unlistenSummonEvent) {
        unlistenSummonEvent();
      }
      if (menuFeedbackTimer) {
        clearTimeout(menuFeedbackTimer);
      }
    };
  });
</script>

<main class="rm-overlay">
  <section class="rm-scene">
    {#if currentScreen === "main"}
      <div class="rm-calendar-widget">
        <P5Calendar />
      </div>
      {#if statusData}
        <div class="rm-player-info" aria-label="Player info">
          <span class="rm-player-name">{statusData.username}</span>
          <span class="rm-player-days">Day {statusData.game_days ?? "—"}</span>
        </div>
      {/if}
      <div class="rm-star-left" aria-hidden="true">
        <div class="rm-star-stack">
          <div class="rm-star rm-star-1"></div>
          <div class="rm-star rm-star-2"></div>
          <div class="rm-star rm-star-3"></div>
          <div class="rm-star rm-star-4"></div>
          <div class="rm-star rm-star-5"></div>
          <div class="rm-star rm-star-6"></div>
          <div class="rm-star rm-star-7"></div>
          <div class="rm-star rm-star-8"></div>
        </div>
        <div class="rm-star-stack rm-star-small">
          <div class="rm-star rm-sm-1"></div>
          <div class="rm-star rm-sm-2"></div>
          <div class="rm-star rm-sm-3"></div>
          <div class="rm-star rm-sm-4"></div>
          <div class="rm-star rm-sm-5"></div>
          <div class="rm-star rm-sm-6"></div>
        </div>
      </div>
      <div class="rm-star-right" aria-hidden="true">
        <div class="rm-star-stack">
          <div class="rm-star rm-star-solid"></div>
        </div>
        <div class="rm-star-stack rm-star-small">
          <div class="rm-star rm-sm-solid"></div>
        </div>
      </div>
      <div class="rm-diagonal-line" aria-hidden="true"></div>
    {/if}

    {#if currentScreen === "main"}
      <aside class="rm-command" bind:this={commandRef}>
        <ul class="rm-menu">
          {#each MENU_ITEMS as item, index}
            <li class="rm-menu-line" style:position="relative" style:z-index={focusedMenuIndex === index ? 10 : 0}>
              <button
                type="button"
                class="rm-menu-item"
                class:is-focused={focusedMenuIndex === index}
                class:is-disabled={!item.enabled}
                aria-disabled={!item.enabled}
                onclick={() => void activateMenuItem(index)}
                onmouseenter={() => setFocusedMenuIndex(index)}
                bind:this={menuItemRefs[index]}
              >
                <P5MenuItem letters={MENU_LETTER_DATA[item.id]} active={focusedMenuIndex === index} />
              </button>
            </li>
          {/each}
        </ul>
        <div class="rm-selection-quad" aria-hidden="true"></div>

        <footer class="rm-command-foot">
          {#if menuFeedback}
            <p class="rm-feedback">{menuFeedback}</p>
          {/if}
        </footer>
      </aside>
    {/if}

    {#if currentScreen === "status"}
      <section class="rm-stage">
        <img src="/ui/Status.png" alt="Status" class="rm-status-title-img" />

        <button type="button" class="rm-back-btn" onclick={() => currentScreen = "main"}>
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

              <!-- Strength with sub_groups -->
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

              <!-- Endurance -->
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
    {/if}

    {#if currentScreen === "achievements"}
      <section class="rm-stage">
        <div class="rm-achievement-title">
          <P5Text text="Achievements" fontSize={82} />
        </div>

        <button type="button" class="rm-back-btn" onclick={() => currentScreen = "main"}>
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
                {#each getPackCategories(pack) as group}
                  <div class="rm-ach-category-block">
                    <h4 class="rm-ach-category-title">{formatGroupName(group.category)}</h4>
                    <div class="rm-achievement-grid">
                      {#each group.achievements as achievement}
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
                  </div>
                {/each}
              {/if}
            </div>
          {:else}
            <p class="state-text">Achievement data is not available yet.</p>
          {/if}
        </div>
      </section>
    {/if}
  </section>
</main>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    width: 100%;
    height: 100%;
    background: transparent;
    overflow: hidden;
  }

  .rm-overlay {
    --rm-black: #000000;
    --rm-white: #ffffff;
    --rm-red: #E5191C;
    position: relative;
    min-height: 100vh;
    color: var(--rm-white);
    background: rgba(30, 0, 0, 0.8);
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
  }

  .rm-scene {
    position: relative;
    width: 100%;
    height: 100vh;
    overflow: hidden;
  }

  .rm-calendar-widget {
    position: absolute;
    top: 1.5rem;
    left: 1.5rem;
    width: clamp(250px, 14.6vw, 600px);
    z-index: 3;
    pointer-events: none;
  }

  .rm-player-info {
    position: absolute;
    top: 1.5rem;
    right: 1.5rem;
    z-index: 3;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.1rem;
    pointer-events: none;
  }

  .rm-player-name,
  .rm-player-days {
    color: var(--rm-white);
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    -webkit-text-stroke: 0.04em var(--rm-black);
    paint-order: stroke fill;
  }

  .rm-player-name,
  .rm-player-days {
    font-size: clamp(1.5rem, 2.1vw, 3rem);
  }

  .rm-star-stack {
    position: absolute;
    top: 50%;
    left: 35%;
    width: 80vh;
    aspect-ratio: 1;
    transform: translate(-50%, -50%) rotate(-14deg);
    z-index: 0;
    pointer-events: none;
  }

  .rm-star {
    position: absolute;
    inset: 0;
    clip-path: polygon(
      50% 0%,
      61.2% 34.5%,
      97.6% 34.5%,
      68.2% 55.9%,
      79.4% 90.5%,
      50% 69.1%,
      20.6% 90.5%,
      31.8% 55.9%,
      2.4% 34.5%,
      38.8% 34.5%
    );
  }

  /* Big star: 8 layers, step 0.12 */
  .rm-star-1 { background: var(--rm-white); transform: scale(0.92); }
  .rm-star-2 { background: var(--rm-black); transform: scale(0.80); }
  .rm-star-3 { background: var(--rm-white); transform: scale(0.68); }
  .rm-star-4 { background: var(--rm-black); transform: scale(0.56); }
  .rm-star-5 { background: var(--rm-white); transform: scale(0.44); }
  .rm-star-6 { background: var(--rm-black); transform: scale(0.32); }
  .rm-star-7 { background: var(--rm-white); transform: scale(0.20); }
  .rm-star-8 { background: var(--rm-black); transform: scale(0.08); }

  .rm-star-solid {
    background: var(--rm-black);
    transform: scale(0.92);
  }

  /* Small star: offset position & different rotation */
  .rm-star-small {
    top: 62%;
    left: 35%;
    transform: translate(-50%, -50%) rotate(20deg);
  }

  /* Small star: white edge(0.04 extra) + 5 layers(step 0.10) */
  .rm-sm-1 { background: var(--rm-white); transform: scale(0.50); }  /* white edge */
  .rm-sm-2 { background: var(--rm-black); transform: scale(0.48); }  /* layer 1 */
  .rm-sm-3 { background: var(--rm-white); transform: scale(0.38); }  /* layer 2 */
  .rm-sm-4 { background: var(--rm-black); transform: scale(0.28); }  /* layer 3 */
  .rm-sm-5 { background: var(--rm-white); transform: scale(0.18); }  /* layer 4 */
  .rm-sm-6 { background: var(--rm-black); transform: scale(0.08); }  /* layer 5 */

  .rm-sm-solid {
    background: var(--rm-black);
    transform: scale(0.52);
  }

  .rm-star-left {
    position: absolute;
    inset: 0;
    clip-path: polygon(0 0, 20% 0, 50% 100%, 0 100%);
    z-index: 0;
    pointer-events: none;
  }

  .rm-star-right {
    position: absolute;
    inset: 0;
    clip-path: polygon(20% 0, 100% 0, 100% 100%, 50% 100%);
    z-index: 0;
    pointer-events: none;
  }

  .rm-diagonal-line {
    position: absolute;
    inset: 0;
    clip-path: polygon(19.85% 0%, 20.15% 0%, 50.15% 100%, 49.85% 100%);
    background: var(--rm-white);
    z-index: 1;
    pointer-events: none;
  }

  .rm-command {
    position: absolute;
    left: 30%;
    top: 50%;
    width: min(75vw, 1200px);
    z-index: 2;
    transform: translateY(-50%);
  }

  .rm-menu {
    margin: 0;
    padding: 0;
    list-style: none;
  }

.rm-menu-line {
    margin: -1rem 0;
  }

  /* Diagonal staircase: each line shifts right to follow the 20%→50% diagonal */
  .rm-menu-line:nth-child(1) { margin-left: 1.5vw; }
  .rm-menu-line:nth-child(2) { margin-left: 5vw; }
  .rm-menu-line:nth-child(3) { margin-left: 1vw; }
  .rm-menu-line:nth-child(4) { margin-left: 7.5vw; }
  .rm-menu-line:nth-child(5) { margin-left: 7vw; }
  .rm-menu-line:nth-child(6) { margin-left: 10vw; }

  /* Per-item: small rotation + irregular quadrilateral clip-path */
  .rm-menu-line:nth-child(1) .rm-menu-item {
    transform: rotate(-30deg);
    clip-path: polygon(0% 10%, 100% 0%, 90% 88%, 10% 96%);
  }
  .rm-menu-line:nth-child(2) .rm-menu-item {
    transform: rotate(-27deg);
    clip-path: polygon(0% 5%, 99% 10%, 96% 94%, 2% 100%);
  }
  .rm-menu-line:nth-child(3) .rm-menu-item {
    transform: rotate(-20deg);
    clip-path: polygon(2% 0%, 100% 8%, 98% 100%, 0% 90%);
  }
  .rm-menu-line:nth-child(4) .rm-menu-item {
    transform: rotate(-8deg);
    clip-path: polygon(0% 6%, 98% 0%, 100% 92%, 1% 100%);
  }
  .rm-menu-line:nth-child(5) .rm-menu-item {
    transform: rotate(-2deg);
    clip-path: polygon(1% 0%, 100% 4%, 97% 96%, 0% 100%);
  }
  .rm-menu-line:nth-child(6) .rm-menu-item {
    transform: rotate(2deg);
    clip-path: polygon(0% 8%, 99% 0%, 100% 100%, 3% 92%);
  }

  .rm-menu-item {
    width: fit-content;
    border: 0;
    padding: 1rem 4rem;
    display: flex;
    align-items: center;
    gap: 0.2rem;
    cursor: pointer;
    color: var(--rm-white);
    background: var(--rm-black);
    transition: background-color 140ms ease;
  }


  .rm-menu-item:not(.is-disabled):hover,
  .rm-menu-item.is-focused {
    background: var(--rm-red);
  }


  .rm-menu-item.is-disabled {
    cursor: default;
  }

  .rm-menu-item:focus-visible {
    outline: 0.16rem solid var(--rm-white);
    outline-offset: 0.12rem;
  }

  .rm-selection-quad {
    position: absolute;
    left: var(--quad-x);
    top: var(--quad-y);
    width: var(--quad-w);
    height: var(--quad-h);
    transform: rotate(var(--quad-rot));
    z-index: 15;
    background: var(--rm-red);
    mix-blend-mode: difference;
    clip-path: var(--quad-clip, polygon(0% 0%, 100% 0%, 100% 100%, 0% 100%));
    pointer-events: none;
    transition: left 120ms ease, top 120ms ease, width 120ms ease, height 120ms ease, transform 120ms ease, clip-path 120ms ease;
  }

  .rm-command-foot {
    margin-top: 1rem;
    transform: rotate(2deg);
  }

  .rm-feedback {
    margin: 0;
    font-size: 0.8rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--rm-white);
    background: var(--rm-red);
    display: inline-block;
    padding: 0.24rem 0.42rem;
  }

  /* ─── Status screen ─── */

  .rm-stage {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    z-index: 2;
  }

  .rm-status-title-img {
    position: fixed;
    top: clamp(0.8rem, 1.5vh, 3rem);
    right: clamp(1.2rem, 2.5vw, 5rem);
    height: clamp(9rem, 15vh, 27rem);
    width: auto;
    z-index: 10;
    pointer-events: none;
  }

  .rm-back-btn {
    position: fixed;
    bottom: clamp(1.5rem, 3vh, 3.5rem);
    right: clamp(1.5rem, 3vw, 4rem);
    z-index: 10;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    transform: rotate(-2deg);
    transition: transform 120ms ease;
  }

  .rm-back-btn:hover {
    transform: rotate(-2deg) scale(1.06);
  }

  .rm-back-img {
    display: block;
    height: clamp(4rem, 7.2vh, 8rem);
    width: auto;
  }

  /* Two-column layout */
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

  .rm-col-heading {
    margin: 0 0 clamp(0.75rem, 1vw, 1.75rem);
    font-size: clamp(0.75rem, 0.65vw, 1.4rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: rgba(255, 255, 255, 0.5);
  }

  .rm-group-block + .rm-group-block {
    margin-top: clamp(1rem, 1.5vw, 2.5rem);
  }

  .rm-group-title {
    margin: 0 0 clamp(0.4rem, 0.5vw, 0.9rem);
    font-size: clamp(0.7rem, 0.6vw, 1.3rem);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: rgba(255, 255, 255, 0.4);
  }

  /* Metric grids */
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

  /* Strength sub-groups */
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

  .state-text {
    margin: 0.85rem 0 0;
    color: rgba(255, 255, 255, 0.7);
  }

  .state-text.error {
    color: var(--rm-red);
    font-weight: 700;
  }

  /* ─── Achievement screen ─── */

  .rm-achievement-title {
    position: fixed;
    top: clamp(0.8rem, 1.5vh, 3rem);
    right: clamp(1.2rem, 2.5vw, 5rem);
    z-index: 10;
    pointer-events: none;
  }

  /* Two-panel layout: sidebar 1/3, content 2/3 */
  .rm-ach-layout {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 2fr;
    overflow: hidden;
    height: 100%;
  }

  /* ── Left sidebar ── */
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

  /* ── Right content ── */
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

  .rm-ach-category-block + .rm-ach-category-block {
    margin-top: clamp(1.5rem, 2vw, 3rem);
  }

  .rm-ach-category-title {
    margin: 0 0 clamp(0.4rem, 0.5vw, 0.9rem);
    font-size: clamp(0.72rem, 0.62vw, 1.3rem);
    color: var(--rm-red);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    border-left: 0.2rem solid var(--rm-red);
    padding-left: clamp(0.4rem, 0.5vw, 1rem);
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

  @media (max-width: 980px) {
    .rm-command {
      position: relative;
      left: auto;
      top: auto;
      width: 100%;
      max-width: 660px;
      margin: 0.9rem auto 0;
      transform: rotate(0);
      padding: 0 0.6rem;
      box-sizing: border-box;
    }

    .rm-selection-quad {
      display: none;
    }

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
