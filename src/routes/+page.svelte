<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

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

  type MenuScreen = "main" | "status";
  type MenuItemId = "status" | "skills" | "achievements" | "items" | "gallery" | "crafting" | "hide";

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
      enabled: false,
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
    {
      id: "hide",
      label: "Hide",
      description: "Return to reality by hiding the summoned interface.",
      enabled: true,
    },
  ];

  const DEFAULT_FOCUS_INDEX = Math.max(0, MENU_ITEMS.findIndex((item) => item.enabled));

  const STRENGTH_SUBGROUP_ORDER = ["chest", "back", "shoulders", "biceps", "triceps", "legs", "core"];

  let currentScreen = $state<MenuScreen>("main");
  let focusedMenuIndex = $state(DEFAULT_FOCUS_INDEX);

  let loading = $state(false);
  let errorMessage = $state<string | null>(null);
  let statusData = $state<StatusData | null>(null);
  let menuFeedback = $state<string | null>(null);

  let menuFeedbackTimer: ReturnType<typeof setTimeout> | null = null;
  let unlistenSummonEvent: UnlistenFn | null = null;

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

    if (item.id === "hide") {
      await hideInterface();
      return;
    }

    if (item.id === "status") {
      await openStatusScreen();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      if (currentScreen === "status") {
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
      <div class="rm-star-stack" aria-hidden="true">
        <div class="rm-star rm-star-1"></div>
        <div class="rm-star rm-star-2"></div>
        <div class="rm-star rm-star-3"></div>
        <div class="rm-star rm-star-4"></div>
        <div class="rm-star rm-star-5"></div>
        <div class="rm-star rm-star-6"></div>
        <div class="rm-star rm-star-7"></div>
      </div>
    {/if}

    {#if currentScreen === "main"}
      <aside class="rm-command">
        <ul class="rm-menu">
          {#each MENU_ITEMS as item, index}
            <li class="rm-menu-line">
              <button
                type="button"
                class="rm-menu-item"
                class:is-focused={focusedMenuIndex === index}
                class:is-disabled={!item.enabled && item.id !== "hide"}
                aria-disabled={!item.enabled && item.id !== "hide"}
                onclick={() => void activateMenuItem(index)}
                onmouseenter={() => setFocusedMenuIndex(index)}
              >
                <span class="rm-menu-text">{item.label}</span>
              </button>
            </li>
          {/each}
        </ul>

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
              <h3 class="rm-col-heading">Health</h3>
              {#if getHealthGroupsWithDerived().length === 0}
                <p class="state-text">No health metrics yet.</p>
              {:else}
                {#each getHealthGroupsWithDerived() as group}
                  <div class="rm-group-block">
                    <h4 class="rm-group-title">{formatGroupName(group.name)}</h4>
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
              <h3 class="rm-col-heading">Performance</h3>

              <!-- Strength with sub_groups -->
              {#if getStrengthSubGroups().length > 0}
                <div class="rm-group-block">
                  <h4 class="rm-group-title">Strength</h4>
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
                  <h4 class="rm-group-title">{formatGroupName(group.name)}</h4>
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
    --rm-red: #80001a;
    position: relative;
    min-height: 100vh;
    color: var(--rm-white);
    background: rgba(150, 0, 15, 0.8);
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
  }

  .rm-scene {
    position: relative;
    width: 100%;
    height: 100vh;
    overflow: hidden;
  }

  .rm-star-stack {
    position: absolute;
    top: 50%;
    left: 25%;
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
      58% 35%,
      92% 35%,
      65% 54%,
      74% 88%,
      50% 68%,
      26% 88%,
      35% 54%,
      8% 35%,
      42% 35%
    );
  }

  .rm-star-1 {
    background: var(--rm-white);
    transform: scale(0.92);
  }

  .rm-star-2 {
    background: var(--rm-black);
    transform: scale(0.80);
  }

  .rm-star-3 {
    background: var(--rm-white);
    transform: scale(0.68);
  }

  .rm-star-4 {
    background: var(--rm-black);
    transform: scale(0.56);
  }

  .rm-star-5 {
    background: var(--rm-white);
    transform: scale(0.44);
  }

  .rm-star-6 {
    background: var(--rm-black);
    transform: scale(0.32);
  }

  .rm-star-7 {
    background: var(--rm-white);
    transform: scale(0.20);
  }

  .rm-command {
    position: absolute;
    left: 30%;
    top: 50%;
    width: min(66vw, 960px);
    z-index: 2;
    transform: translateY(-50%);
  }

  .rm-menu {
    margin: 0;
    padding: 0;
    list-style: none;
  }

.rm-menu-line {
    margin: 1rem 0;
  }

  .rm-menu-line:nth-child(1) .rm-menu-item { transform: translateX(-1.5rem) rotate(-9deg); }
  .rm-menu-line:nth-child(2) .rm-menu-item { transform: translateX(2rem)    rotate(-6deg); }
  .rm-menu-line:nth-child(3) .rm-menu-item { transform: translateX(-1.5rem) rotate(-3deg); }
  .rm-menu-line:nth-child(4) .rm-menu-item { transform: translateX(2rem)    rotate(0deg);  }
  .rm-menu-line:nth-child(5) .rm-menu-item { transform: translateX(-1.5rem) rotate(3deg);  }
  .rm-menu-line:nth-child(6) .rm-menu-item { transform: translateX(2rem)    rotate(6deg);  }
  .rm-menu-line:nth-child(7) .rm-menu-item { transform: translateX(-1.5rem) rotate(9deg);  }

  .rm-menu-item {
    width: 100%;
    border: 0;
    padding: 1.8rem 3.6rem;
    display: flex;
    align-items: center;
    gap: 0.56rem;
    cursor: pointer;
    color: var(--rm-white);
    background: var(--rm-black);
    clip-path: polygon(0 15%, 100% 0%, 100% 100%, 0 85%);
    transition: background-color 140ms ease;
  }

  .rm-menu-item span {
    display: block;
  }

  .rm-menu-text {
    font-size: clamp(3rem, 7vw, 4.8rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .rm-menu-item:not(.is-disabled):hover,
  .rm-menu-item.is-focused {
    background: var(--rm-red);
  }

  .rm-menu-item.is-active {
    background: var(--rm-white);
    color: var(--rm-black);
  }

  .rm-menu-item.is-disabled {
    cursor: default;
  }

  .rm-menu-item:focus-visible {
    outline: 0.16rem solid var(--rm-white);
    outline-offset: 0.12rem;
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
