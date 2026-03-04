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
    <div class="rm-star-stack" aria-hidden="true">
      <div class="rm-star rm-star-1"></div>
      <div class="rm-star rm-star-2"></div>
      <div class="rm-star rm-star-3"></div>
      <div class="rm-star rm-star-4"></div>
      <div class="rm-star rm-star-5"></div>
    </div>

    <aside class="rm-command">
      <ul class="rm-menu" class:rm-menu-locked={currentScreen !== "main"}>
        {#each MENU_ITEMS as item, index}
          <li class="rm-menu-line">
            <button
              type="button"
              class="rm-menu-item"
              class:is-focused={currentScreen === "main" && focusedMenuIndex === index}
              class:is-active={currentScreen === "status" && item.id === "status"}
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

    {#if currentScreen === "status"}
      <section class="rm-stage">
        <div class="rm-stage-inner">
          <header class="rm-status-head">
            <div>
              <p class="rm-kicker">Status Submenu</p>
              <h2>Status</h2>
              <p class="rm-status-subtitle">
                {#if statusData}
                  {statusData.username}
                  {#if statusData.game_days !== null}
                    · Game Day {statusData.game_days}
                  {/if}
                {:else}
                  Local snapshot from status JSON files.
                {/if}
              </p>
            </div>
            <button type="button" class="rm-reload-btn" onclick={loadStatusData}>Reload</button>
          </header>

          {#if loading}
            <p class="state-text">Loading status data...</p>
          {:else if errorMessage}
            <p class="state-text error">{errorMessage}</p>
          {:else if statusData}
            <section class="rm-status-block">
              <h3>Health</h3>
              {#if getHealthGroupsWithDerived().length === 0}
                <p class="state-text">No health metrics yet.</p>
              {:else}
                {#each getHealthGroupsWithDerived() as group}
                  <div class="rm-group-block">
                    <h4>{formatGroupName(group.name)}</h4>
                    <div class="rm-metric-grid">
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
            </section>

            <section class="rm-status-block">
              <h3>Performance</h3>
              {#if getCategoryGroups("performance").length === 0}
                <p class="state-text">No performance metrics yet.</p>
              {:else}
                {#each getCategoryGroups("performance") as group}
                  <div class="rm-group-block">
                    <h4>{formatGroupName(group.name)}</h4>
                    <div class="rm-metric-grid">
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
            </section>
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
    --rm-red: #ff0033;
    position: relative;
    min-height: 100vh;
    color: var(--rm-white);
    background: rgba(10, 12, 16, 0.36);
    font-family: "Noto Sans SC", "Source Han Sans SC", "Microsoft YaHei", sans-serif;
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
    left: 48%;
    width: clamp(320px, 43vw, 620px);
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
    transform: scale(0.76);
  }

  .rm-star-3 {
    background: var(--rm-white);
    transform: scale(0.62);
  }

  .rm-star-4 {
    background: var(--rm-black);
    transform: scale(0.48);
  }

  .rm-star-5 {
    background: var(--rm-white);
    transform: scale(0.34);
  }

  .rm-command {
    position: absolute;
    left: clamp(0.6rem, 6vw, 6rem);
    top: clamp(1.2rem, 12vh, 9rem);
    width: min(41vw, 560px);
    z-index: 2;
    transform: rotate(-8deg);
  }

  .rm-menu {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .rm-menu.rm-menu-locked {
    pointer-events: none;
    opacity: 0.72;
  }

  .rm-menu-line {
    margin: 0.35rem 0;
  }

  .rm-menu-line:nth-child(odd) .rm-menu-item {
    transform: skewX(-10deg) translateX(-0.45rem) rotate(1deg);
  }

  .rm-menu-line:nth-child(even) .rm-menu-item {
    transform: skewX(-10deg) translateX(0.8rem) rotate(-1.5deg);
  }

  .rm-menu-item {
    width: 100%;
    border: 0;
    padding: 0.4rem 0.85rem;
    display: flex;
    align-items: center;
    gap: 0.56rem;
    cursor: pointer;
    color: var(--rm-white);
    background: var(--rm-black);
    box-shadow: 0.2rem 0.2rem 0 var(--rm-white), 0.34rem 0.34rem 0 var(--rm-red);
    transition: transform 140ms ease, box-shadow 140ms ease, background-color 140ms ease;
  }

  .rm-menu-item span {
    transform: skewX(10deg);
  }

  .rm-menu-text {
    font-size: clamp(0.95rem, 2vw, 1.25rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .rm-menu-item:not(.is-disabled):hover,
  .rm-menu-item.is-focused {
    background: var(--rm-red);
    box-shadow: 0.2rem 0.2rem 0 var(--rm-white), 0.4rem 0.4rem 0 var(--rm-black);
  }

  .rm-menu-item.is-active {
    background: var(--rm-white);
    color: var(--rm-black);
  }

  .rm-menu-item.is-disabled {
    opacity: 0.62;
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

  .rm-stage {
    position: absolute;
    right: clamp(0.6rem, 5vw, 4rem);
    top: clamp(2rem, 15vh, 8rem);
    width: min(52vw, 740px);
    max-height: min(74vh, 720px);
    border: 0.2rem solid var(--rm-white);
    background: var(--rm-black);
    transform: skewX(-5deg) rotate(-1deg);
    box-shadow: 0.42rem 0.42rem 0 var(--rm-red);
    z-index: 2;
    overflow: auto;
  }

  .rm-stage-inner {
    transform: skewX(5deg);
    padding: 1rem;
  }

  .rm-status-head h2 {
    margin: 0.3rem 0 0;
    font-size: clamp(1.8rem, 5vw, 2.7rem);
    line-height: 0.9;
    text-transform: uppercase;
    font-family: "Bebas Neue", "Archivo Black", Impact, sans-serif;
    text-shadow: 0.18rem 0.18rem 0 var(--rm-red);
  }

  .rm-kicker {
    margin: 0;
    color: var(--rm-red);
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .rm-status-head {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .rm-status-subtitle {
    margin: 0.3rem 0 0;
    line-height: 1.45;
  }

  .rm-reload-btn {
    border: 0.14rem solid var(--rm-white);
    padding: 0.55rem 0.85rem;
    color: var(--rm-white);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    background: var(--rm-red);
    clip-path: polygon(0 0, 100% 0, 90% 100%, 0 100%);
  }

  .rm-status-block {
    margin-top: 0.9rem;
    border: 0.14rem solid var(--rm-black);
    padding: 0.9rem;
    color: var(--rm-black);
    background: var(--rm-white);
    box-shadow: 0.28rem 0.28rem 0 var(--rm-red);
  }

  .rm-status-block h3 {
    margin: 0;
    font-size: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .rm-group-block + .rm-group-block {
    margin-top: 0.9rem;
  }

  .rm-group-block h4 {
    margin: 0.7rem 0 0.45rem;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.07em;
  }

  .rm-metric-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 0.5rem;
  }

  .rm-metric-card {
    border: 0.12rem solid var(--rm-black);
    padding: 0.58rem;
    background: var(--rm-white);
  }

  .rm-metric-name {
    margin: 0;
    font-size: 0.81rem;
    line-height: 1.2;
  }

  .rm-metric-value {
    margin: 0.28rem 0 0;
    font-size: 1rem;
    font-weight: 700;
    color: var(--rm-red);
  }

  .state-text {
    margin: 0.85rem 0 0;
  }

  .state-text.error {
    color: var(--rm-red);
    font-weight: 700;
  }

  .rm-status-block .state-text {
    color: var(--rm-black);
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

    .rm-stage {
      position: relative;
      right: auto;
      top: auto;
      width: calc(100% - 1.2rem);
      max-height: none;
      margin: 0.6rem auto 1rem;
      transform: skewX(0) rotate(0);
    }

    .rm-stage-inner {
      transform: none;
    }

  }
</style>
