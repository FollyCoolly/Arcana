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
  import type { SkillData, SkillWithLevel, SkillNode } from "$lib/types/skill";
  import type { ItemData, ItemWithComputed, ItemSortKey, ItemSortOrder } from "$lib/types/item";
  import type { CraftingData, RecipeWithComputed } from "$lib/types/crafting";
  import SkillNebula from "$lib/components/SkillNebula.svelte";

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

  type MenuScreen = "main" | "status" | "achievements" | "skills" | "items" | "crafting";
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
      enabled: true,
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
      enabled: true,
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
      enabled: true,
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

  let skillLoading = $state(false);
  let skillError = $state<string | null>(null);
  let skillData = $state<SkillData | null>(null);
  let selectedSkill = $state<SkillWithLevel | null>(null);

  let itemLoading = $state(false);
  let itemError = $state<string | null>(null);
  let itemData = $state<ItemData | null>(null);
  let selectedItem = $state<ItemWithComputed | null>(null);
  let itemFilterSource = $state<string | null>(null);
  let itemFilterCategory = $state<string | null>(null);
  let itemSortKey = $state<ItemSortKey>('name');
  let itemSortOrder = $state<ItemSortOrder>('asc');

  let craftingLoading = $state(false);
  let craftingError = $state<string | null>(null);
  let craftingData = $state<CraftingData | null>(null);
  let selectedRecipe = $state<RecipeWithComputed | null>(null);

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

  async function openSkillsScreen() {
    currentScreen = "skills";
    selectedSkill = null;
    if (!skillData && !skillLoading) {
      await loadSkillData();
    }
    if (!achievementData && !achievementLoading) {
      await loadAchievementData();
    }
  }

  // ── Items helpers ──

  const ITEM_SORT_OPTIONS: { key: ItemSortKey; label: string }[] = [
    { key: 'name', label: '名称' },
    { key: 'price', label: '价格' },
    { key: 'daily_cost', label: '日均' },
    { key: 'date', label: '购入' },
    { key: 'days_owned', label: '天数' },
  ];

  async function loadItemData() {
    itemLoading = true;
    itemError = null;

    try {
      itemData = await invoke<ItemData>("load_items");
    } catch (error) {
      itemError =
        typeof error === "string"
          ? error
          : "Failed to load item data.";
      itemData = null;
    } finally {
      itemLoading = false;
    }
  }

  async function openItemsScreen() {
    currentScreen = "items";
    selectedItem = null;
    itemFilterSource = null;
    itemFilterCategory = null;
    if (!itemData && !itemLoading) {
      await loadItemData();
    }
  }

  async function loadCraftingData() {
    craftingLoading = true;
    craftingError = null;

    try {
      craftingData = await invoke<CraftingData>("load_crafting");
    } catch (error) {
      craftingError =
        typeof error === "string"
          ? error
          : "Failed to load crafting data.";
      craftingData = null;
    } finally {
      craftingLoading = false;
    }
  }

  async function openCraftingScreen() {
    currentScreen = "crafting";
    selectedRecipe = null;
    if (!craftingData && !craftingLoading) {
      await loadCraftingData();
    }
  }

  function formatPrice(price: number | null): string {
    if (price === null || price === undefined) return "—";
    return `¥${price.toLocaleString("zh-CN", { minimumFractionDigits: 0, maximumFractionDigits: 0 })}`;
  }

  function formatDailyCost(cost: number | null): string {
    if (cost === null || cost === undefined) return "—";
    return `¥${cost.toFixed(2)}`;
  }

  function toggleItemSort(key: ItemSortKey) {
    if (itemSortKey === key) {
      itemSortOrder = itemSortOrder === 'asc' ? 'desc' : 'asc';
    } else {
      itemSortKey = key;
      itemSortOrder = key === 'name' ? 'asc' : 'desc';
    }
  }

  function getFilteredSortedItems(): ItemWithComputed[] {
    if (!itemData) return [];

    let items = itemData.items;

    if (itemFilterSource) {
      items = items.filter(i => i.source_id === itemFilterSource);
    }
    if (itemFilterCategory) {
      items = items.filter(i => (i.main_category ?? '未分类') === itemFilterCategory);
    }

    const sorted = [...items];
    const dir = itemSortOrder === 'asc' ? 1 : -1;

    sorted.sort((a, b) => {
      switch (itemSortKey) {
        case 'name':
          return dir * a.name.localeCompare(b.name, 'zh-CN');
        case 'price':
          return dir * ((a.price ?? 0) - (b.price ?? 0));
        case 'daily_cost':
          return dir * ((a.daily_cost ?? Infinity) - (b.daily_cost ?? Infinity));
        case 'date':
          return dir * ((a.purchase_date ?? '').localeCompare(b.purchase_date ?? ''));
        case 'days_owned':
          return dir * ((a.days_owned ?? 0) - (b.days_owned ?? 0));
        default:
          return 0;
      }
    });

    return sorted;
  }

  function formatExtraValue(val: unknown): string {
    if (val === null || val === undefined) return "—";
    if (typeof val === 'string') return val;
    if (typeof val === 'number') return String(val);
    if (typeof val === 'boolean') return val ? '是' : '否';
    if (Array.isArray(val)) return val.join(', ');
    return JSON.stringify(val);
  }

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
    // Fallback: format from ID
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
    } else if (item.id === "skills") {
      await openSkillsScreen();
    } else if (item.id === "items") {
      await openItemsScreen();
    } else if (item.id === "crafting") {
      await openCraftingScreen();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      if (currentScreen === "skills" && selectedSkill) {
        selectedSkill = null;
      } else if (currentScreen === "items" && selectedItem) {
        selectedItem = null;
      } else if (currentScreen === "crafting" && selectedRecipe) {
        selectedRecipe = null;
      } else if (currentScreen === "status" || currentScreen === "achievements" || currentScreen === "skills" || currentScreen === "items" || currentScreen === "crafting") {
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

    {#if currentScreen === "skills"}
      <section class="rm-stage">
        <div class="rm-skills-title">
          <P5Text text="Skills" fontSize={82} />
        </div>

        <button type="button" class="rm-back-btn" onclick={() => {
          if (selectedSkill) {
            selectedSkill = null;
          } else {
            currentScreen = "main";
          }
        }}>
          <img src="/ui/back.png" alt="Back" class="rm-back-img" />
        </button>

        {#if skillLoading}
          <p class="state-text" style="padding: 2rem;">Loading skills...</p>
        {:else if skillError}
          <p class="state-text error" style="padding: 2rem;">{skillError}</p>
        {:else if skillData && !selectedSkill}
          <!-- Level 1: 3D Nebula Card Gallery -->
          <div class="rm-nebula-container">
            <SkillNebula
              skills={skillData.skills}
              onCardClick={(skill) => { selectedSkill = skill; }}
            />
          </div>
        {:else if skillData && selectedSkill}
          <!-- Level 2: Skill Detail -->
          <div class="rm-skill-detail">
            <!-- Left: Enlarged card + stats -->
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

            <!-- Right: Skill name + honeycomb node grid -->
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
    {/if}

    {#if currentScreen === "crafting"}
      <section class="rm-stage">
        <div class="rm-items-title">
          <P5Text text="Crafting" fontSize={82} />
        </div>

        <button type="button" class="rm-back-btn" onclick={() => {
          currentScreen = "main";
        }}>
          <img src="/ui/back.png" alt="Back" class="rm-back-img" />
        </button>

        {#if craftingLoading}
          <p class="state-text" style="padding: 2rem;">Loading recipes...</p>
        {:else if craftingError}
          <p class="state-text error" style="padding: 2rem;">{craftingError}</p>
        {:else if craftingData}
          <div class="rm-craft-layout">
            <!-- LEFT: recipe menu -->
            <div class="rm-craft-menu">
              {#each craftingData.recipes as recipe, i}
                <button
                  type="button"
                  class="rm-craft-menu-item"
                  class:is-active={selectedRecipe?.id === recipe.id}
                  style="transform: rotate({i % 2 === 0 ? -0.4 : 0.4}deg);"
                  onclick={() => { selectedRecipe = recipe; }}
                >
                  <div class="rm-craft-menu-stripe">
                    <span class="rm-craft-menu-type">{recipe.recipe_type ?? '—'}</span>
                    {#if recipe.time}
                      <span class="rm-craft-menu-time">{recipe.time}</span>
                    {/if}
                  </div>
                  <div class="rm-craft-menu-body">
                    <span class="rm-craft-menu-name">{recipe.name}</span>
                    <span class="rm-craft-menu-sub">{recipe.ingredient_count} mat · {recipe.step_count} steps</span>
                  </div>
                </button>
              {/each}
            </div>

            <!-- RIGHT: detail -->
            <div class="rm-craft-detail">
              {#if selectedRecipe}
                <!-- Top: ingredients box -->
                <div class="rm-craft-ingredients">
                  <div class="rm-craft-ing-header">
                    <span class="rm-craft-ing-label">INGREDIENTS</span>
                    <span class="rm-craft-ing-count">{selectedRecipe.ingredient_count}</span>
                  </div>
                  <div class="rm-craft-ing-list">
                    {#each selectedRecipe.ingredients as ing, i}
                      <span class="rm-craft-ing-tag" style="transform: rotate({i % 2 === 0 ? -0.6 : 0.6}deg);">{ing}</span>
                    {/each}
                  </div>
                </div>

                <!-- Bottom: paper-style detail panel -->
                <div class="rm-craft-paper">
                  <div class="rm-craft-paper-head">
                    <div class="rm-craft-paper-title-row">
                      <h2 class="rm-craft-paper-title">{selectedRecipe.name}</h2>
                    </div>
                    <div class="rm-craft-paper-tags">
                      {#if selectedRecipe.recipe_type}
                        <span class="rm-craft-paper-tag">{selectedRecipe.recipe_type}</span>
                      {/if}
                      {#if selectedRecipe.difficulty}
                        <span class="rm-craft-paper-tag rm-craft-paper-tag--red">{selectedRecipe.difficulty}</span>
                      {/if}
                      {#if selectedRecipe.servings}
                        <span class="rm-craft-paper-tag">{selectedRecipe.servings}</span>
                      {/if}
                      {#if selectedRecipe.time}
                        <span class="rm-craft-paper-tag">{selectedRecipe.time}</span>
                      {/if}
                    </div>
                  </div>

                  {#if selectedRecipe.steps.length > 0}
                    <div class="rm-craft-steps">
                      <h4 class="rm-craft-section-label">STEPS</h4>
                      <ol class="rm-craft-step-list">
                        {#each selectedRecipe.steps as step, i}
                          <li class="rm-craft-step">
                            <span class="rm-craft-step-num">{String(i + 1).padStart(2, '0')}</span>
                            <span class="rm-craft-step-text">{step}</span>
                          </li>
                        {/each}
                      </ol>
                    </div>
                  {/if}

                  {#if selectedRecipe.tags.length > 0}
                    <div class="rm-craft-foot-tags">
                      {#each selectedRecipe.tags as tag}
                        <span class="rm-craft-foot-tag">{tag}</span>
                      {/each}
                    </div>
                  {/if}

                  {#if selectedRecipe.source}
                    <p class="rm-craft-source">SOURCE: {selectedRecipe.source}</p>
                  {/if}

                  {#if Object.keys(selectedRecipe.extra).length > 0}
                    <div class="rm-craft-foot-tags" style="margin-top: clamp(0.3rem, 0.4vw, 0.5rem);">
                      {#each Object.entries(selectedRecipe.extra) as [key, val]}
                        <span class="rm-craft-paper-tag">{key}: {val}</span>
                      {/each}
                    </div>
                  {/if}
                </div>
              {:else}
                <div class="rm-craft-empty">
                  <span class="rm-craft-empty-text">SELECT A RECIPE</span>
                </div>
              {/if}
            </div>
          </div>
        {:else}
          <p class="state-text" style="padding: 2rem;">Crafting data is not available yet.</p>
        {/if}
      </section>
    {/if}

    {#if currentScreen === "items"}
      <section class="rm-stage">
        <div class="rm-items-title">
          <P5Text text="Items" fontSize={82} />
        </div>

        <button type="button" class="rm-back-btn" onclick={() => {
          if (selectedItem) {
            selectedItem = null;
          } else {
            currentScreen = "main";
          }
        }}>
          <img src="/ui/back.png" alt="Back" class="rm-back-img" />
        </button>

        {#if itemLoading}
          <p class="state-text" style="padding: 2rem;">Loading items...</p>
        {:else if itemError}
          <p class="state-text error" style="padding: 2rem;">{itemError}</p>
        {:else if itemData && !selectedItem}
          <div class="rm-items-layout">
            <!-- LEFT: Stats sidebar -->
            <div class="rm-items-sidebar">
              <div class="rm-items-stat-block">
                <div class="rm-items-stat-row">
                  <span class="rm-items-stat-label">TOTAL</span>
                  <span class="rm-items-stat-value">{itemData.stats.total_items}</span>
                </div>
                <div class="rm-items-stat-row">
                  <span class="rm-items-stat-label">VALUE</span>
                  <span class="rm-items-stat-value">{formatPrice(itemData.stats.total_value)}</span>
                </div>
                <div class="rm-items-stat-row">
                  <span class="rm-items-stat-label">AVG/DAY</span>
                  <span class="rm-items-stat-value rm-items-daily">{formatDailyCost(itemData.stats.average_daily_cost)}</span>
                </div>
              </div>

              <!-- By source -->
              <div class="rm-items-filter-section">
                <h4 class="rm-items-filter-title">Sources</h4>
                <button
                  type="button"
                  class="rm-items-filter-btn"
                  class:is-active={!itemFilterSource}
                  onclick={() => { itemFilterSource = null; }}
                >All</button>
                {#each itemData.stats.by_source as src}
                  <button
                    type="button"
                    class="rm-items-filter-btn"
                    class:is-active={itemFilterSource === src.source_id}
                    onclick={() => { itemFilterSource = itemFilterSource === src.source_id ? null : src.source_id; }}
                  >
                    {src.source_icon} {src.source_name}
                    <span class="rm-items-filter-count">{src.item_count}</span>
                  </button>
                {/each}
              </div>

              <!-- By category -->
              <div class="rm-items-filter-section">
                <h4 class="rm-items-filter-title">Categories</h4>
                <button
                  type="button"
                  class="rm-items-filter-btn"
                  class:is-active={!itemFilterCategory}
                  onclick={() => { itemFilterCategory = null; }}
                >All</button>
                {#each itemData.stats.by_main_category as cat}
                  <button
                    type="button"
                    class="rm-items-filter-btn"
                    class:is-active={itemFilterCategory === cat.name}
                    onclick={() => { itemFilterCategory = itemFilterCategory === cat.name ? null : cat.name; }}
                  >
                    {cat.name}
                    <span class="rm-items-filter-count">{cat.item_count}</span>
                  </button>
                {/each}
              </div>
            </div>

            <!-- RIGHT: Items grid -->
            <div class="rm-items-content">
              <!-- Sort bar -->
              <div class="rm-items-sort-bar">
                {#each ITEM_SORT_OPTIONS as opt}
                  <button
                    type="button"
                    class="rm-items-sort-btn"
                    class:is-active={itemSortKey === opt.key}
                    onclick={() => toggleItemSort(opt.key)}
                  >
                    {opt.label}
                    {#if itemSortKey === opt.key}
                      <span class="rm-items-sort-arrow">{itemSortOrder === 'asc' ? '↑' : '↓'}</span>
                    {/if}
                  </button>
                {/each}
                <span class="rm-items-result-count">{getFilteredSortedItems().length} items</span>
              </div>

              <!-- Item cards grid -->
              <div class="rm-items-grid">
                {#each getFilteredSortedItems() as item}
                  <button
                    type="button"
                    class="rm-item-card"
                    onclick={() => { selectedItem = item; }}
                  >
                    <div class="rm-item-card-header">
                      <span class="rm-item-card-category">{item.main_category ?? '—'}</span>
                      {#if item.sub_category}
                        <span class="rm-item-card-subcategory">{item.sub_category}</span>
                      {/if}
                    </div>
                    <div class="rm-item-card-body">
                      <p class="rm-item-card-name">{item.name}</p>
                      {#if item.brand}
                        <p class="rm-item-card-brand">{item.brand}</p>
                      {/if}
                      <div class="rm-item-card-stats">
                        <span class="rm-item-card-price">{formatPrice(item.price)}</span>
                        <span class="rm-item-card-daily">{formatDailyCost(item.daily_cost)}/d</span>
                      </div>
                      {#if item.days_owned !== null}
                        <p class="rm-item-card-days">{item.days_owned} days</p>
                      {/if}
                    </div>
                  </button>
                {/each}
              </div>
            </div>
          </div>
        {:else if itemData && selectedItem}
          <!-- Item detail view -->
          <div class="rm-item-detail">
            <div class="rm-item-detail-left">
              <div class="rm-item-detail-name">
                <P5Text text={selectedItem.name} fontSize={62} />
              </div>

              {#if selectedItem.brand}
                <p class="rm-item-detail-brand">{selectedItem.brand}</p>
              {/if}

              <div class="rm-item-detail-table">
                {#if selectedItem.price !== null}
                  <div class="rm-item-detail-row">
                    <span class="rm-item-detail-label">PRICE</span>
                    <span class="rm-item-detail-value">{formatPrice(selectedItem.price)}</span>
                  </div>
                {/if}
                {#if selectedItem.daily_cost !== null}
                  <div class="rm-item-detail-row">
                    <span class="rm-item-detail-label">DAILY</span>
                    <span class="rm-item-detail-value rm-items-daily">{formatDailyCost(selectedItem.daily_cost)}/day</span>
                  </div>
                {/if}
                {#if selectedItem.days_owned !== null}
                  <div class="rm-item-detail-row">
                    <span class="rm-item-detail-label">OWNED</span>
                    <span class="rm-item-detail-value">{selectedItem.days_owned} days</span>
                  </div>
                {/if}
                {#if selectedItem.purchase_date}
                  <div class="rm-item-detail-row">
                    <span class="rm-item-detail-label">DATE</span>
                    <span class="rm-item-detail-value">{selectedItem.purchase_date}</span>
                  </div>
                {/if}
                {#if selectedItem.purchase_channel}
                  <div class="rm-item-detail-row">
                    <span class="rm-item-detail-label">FROM</span>
                    <span class="rm-item-detail-value">{selectedItem.purchase_channel}</span>
                  </div>
                {/if}
                {#if selectedItem.main_category}
                  <div class="rm-item-detail-row">
                    <span class="rm-item-detail-label">CATEGORY</span>
                    <span class="rm-item-detail-value">{selectedItem.main_category}{selectedItem.sub_category ? ` / ${selectedItem.sub_category}` : ''}</span>
                  </div>
                {/if}
                {#if selectedItem.color}
                  <div class="rm-item-detail-row">
                    <span class="rm-item-detail-label">COLOR</span>
                    <span class="rm-item-detail-value">{selectedItem.color}</span>
                  </div>
                {/if}
              </div>
            </div>

            <div class="rm-item-detail-right">
              <!-- Extra attributes -->
              {#if Object.keys(selectedItem.extra).length > 0}
                <h4 class="rm-item-extra-title">Attributes</h4>
                <div class="rm-item-extra-list">
                  {#each Object.entries(selectedItem.extra) as [key, val]}
                    <div class="rm-item-extra-row">
                      <span class="rm-item-extra-key">{key}</span>
                      <span class="rm-item-extra-val">{formatExtraValue(val)}</span>
                    </div>
                  {/each}
                </div>
              {/if}

              <!-- Image -->
              {#if selectedItem.image}
                <div class="rm-item-image-wrap">
                  <img
                    src="https://asset.localhost/{selectedItem.image}"
                    alt={selectedItem.name}
                    class="rm-item-image"
                  />
                </div>
              {/if}
            </div>
          </div>
        {:else}
          <p class="state-text" style="padding: 2rem;">Item data is not available yet.</p>
        {/if}
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

  /* ─── Skills screen ─── */

  .rm-skills-title {
    position: fixed;
    top: clamp(0.8rem, 1.5vh, 3rem);
    right: clamp(1.2rem, 2.5vw, 5rem);
    z-index: 10;
    pointer-events: none;
  }

  /* 3D Nebula container */
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

  /* Nebula card — use `scale` property (not `transform`) because CSS3DRenderer
     overwrites inline transform with matrix3d() every frame */
  :global(.rm-nebula-card.rm-tarot-card) {
    cursor: pointer;
    transition: scale 160ms ease;
  }

  :global(.rm-nebula-card.rm-tarot-card:hover) {
    scale: 1.12;
    z-index: 5;
  }

  /* Tarot card — :global because nebula cards are created programmatically outside Svelte template */
  :global(.rm-tarot-card) {
    display: block;
    border: none;
    background: none;
    cursor: pointer;
    padding: 0;
    width: clamp(120px, 10vw, 200px);
    transform: rotate(var(--card-rot, 0deg));
    transition: transform 200ms ease;
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    color: var(--rm-white);
  }

  :global(.rm-tarot-card:hover) {
    transform: translateY(-6px) rotateX(4deg) rotate(var(--card-rot, 0deg));
    z-index: 5;
  }

  :global(.rm-tarot-card-inner) {
    aspect-ratio: 0.6 / 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 2px solid rgba(255, 255, 255, 0.15);
  }

  /* Top band (white) */
  :global(.rm-tarot-top) {
    background: #ffffff;
    color: #000000;
    padding: 3px 5px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-shrink: 0;
  }

  :global(.rm-tarot-level) {
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.06em;
  }

  :global(.rm-tarot-pack) {
    font-size: 7px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.5;
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 60%;
  }

  /* Art area (black) */
  :global(.rm-tarot-art) {
    flex: 1;
    background: #000000;
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0.35;
  }

  :global(.rm-tarot-card--leveled .rm-tarot-art) {
    opacity: 1;
  }

  :global(.rm-tarot-star-stack) {
    position: absolute;
    width: 70%;
    aspect-ratio: 1;
  }

  :global(.rm-tarot-star) {
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

  :global(.rm-ts-1) { background: #ffffff; transform: scale(0.85); }
  :global(.rm-ts-2) { background: #000000; transform: scale(0.68); }
  :global(.rm-ts-3) { background: #ffffff; transform: scale(0.51); }
  :global(.rm-ts-4) { background: #000000; transform: scale(0.34); }
  :global(.rm-ts-5) { background: #ffffff; transform: scale(0.17); }

  :global(.rm-tarot-stripe) {
    position: absolute;
    top: 0;
    left: 40%;
    width: 35%;
    height: 100%;
    background: #E5191C;
    opacity: 0.35;
    transform: skewX(-20deg);
  }

  :global(.rm-tarot-card--leveled .rm-tarot-stripe) {
    opacity: 0.7;
  }

  /* Center name strip (red) */
  :global(.rm-tarot-name-strip) {
    background: #E5191C;
    padding: 2px 5px;
    flex-shrink: 0;
    overflow: hidden;
  }

  :global(.rm-tarot-name) {
    display: block;
    font-size: 8px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #ffffff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Bottom band (black) */
  :global(.rm-tarot-bottom) {
    background: #000000;
    padding: 3px 5px;
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  :global(.rm-tarot-progress) {
    flex: 1;
    height: 4px;
    background: rgba(255, 255, 255, 0.15);
    overflow: hidden;
  }

  :global(.rm-tarot-progress-fill) {
    height: 100%;
    background: #E5191C;
    transition: width 300ms ease;
  }

  :global(.rm-tarot-lv) {
    font-size: 7px;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.6);
    flex-shrink: 0;
  }

  /* Muted state for level-0 cards */
  :global(.rm-tarot-card:not(.rm-tarot-card--leveled) .rm-tarot-card-inner) {
    border-color: rgba(255, 255, 255, 0.08);
  }

  :global(.rm-tarot-card:not(.rm-tarot-card--leveled) .rm-tarot-top) {
    background: rgba(255, 255, 255, 0.3);
  }

  :global(.rm-tarot-card:not(.rm-tarot-card--leveled) .rm-tarot-name-strip) {
    background: rgba(229, 25, 28, 0.35);
  }

  :global(.rm-tarot-card:not(.rm-tarot-card--leveled) .rm-tarot-lv) {
    color: rgba(255, 255, 255, 0.3);
  }

  /* Large card (detail view) — ~2.5× nebula card size */
  :global(.rm-tarot-card--large) {
    width: clamp(400px, 27.5vw, 625px);
    margin-top: clamp(4rem, 10vh, 12rem);
    cursor: default;
    transform: none;
  }

  :global(.rm-tarot-card--large:hover) {
    transform: none;
  }

  /* Skill detail layout — left 1/3, right 2/3 */
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

  /*
   * Honeycomb node grid — pointy-top hexagons
   *
   * Geometry: pointy-top hex → width = size × √3, height = size × 2
   * clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)
   *
   * Tessellation:
   * - Horizontal spacing = hex-w (no gap → shared vertical edges)
   * - Vertical stride = hex-h × 0.75 (top/bottom points nest into adjacent row)
   * - Even rows (0, 2, 4…) flush left; odd rows offset right by hex-w / 2
   * - Use flex wrap with negative top margin for vertical nesting
   *
   * --cols computed dynamically via JS, but CSS handles the layout.
   */
  .rm-skill-node-grid {
    --hex-w: clamp(80px, 6.5vw, 180px);
    --hex-h: calc(var(--hex-w) * 1.1547);  /* 2/√3 ≈ 1.1547 */
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

  /* Hex border wrapper — thick white outline via larger clipped background */
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

  /* Inner hex content — locked: black bg, white text */
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

  /* Unlocked: red bg, black text */
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

  /* ─── Items screen ─── */

  .rm-items-title {
    position: fixed;
    top: clamp(0.8rem, 1.5vh, 3rem);
    right: clamp(1.2rem, 2.5vw, 5rem);
    z-index: 10;
    pointer-events: none;
  }

  .rm-items-layout {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 2fr;
    overflow: hidden;
    height: 100%;
  }

  /* ── Left sidebar ── */
  .rm-items-sidebar {
    overflow-y: auto;
    height: 100%;
    padding: clamp(1.5rem, 2.5vh, 4rem) clamp(1.2rem, 2vw, 3rem) clamp(6rem, 10vh, 10rem) clamp(1.5rem, 2.5vw, 4rem);
    box-sizing: border-box;
  }

  .rm-items-stat-block {
    display: flex;
    flex-direction: column;
    gap: clamp(0.2rem, 0.3vw, 0.5rem);
    margin-bottom: clamp(1.5rem, 2vw, 3rem);
  }

  .rm-items-stat-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: clamp(0.15rem, 0.2vw, 0.35rem) 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .rm-items-stat-label {
    font-size: clamp(0.6rem, 0.55vw, 0.95rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: rgba(255, 255, 255, 0.5);
  }

  .rm-items-stat-value {
    font-size: clamp(0.75rem, 0.7vw, 1.2rem);
    font-weight: 800;
    letter-spacing: 0.04em;
  }

  .rm-items-daily {
    color: var(--rm-red);
  }

  .rm-items-filter-section {
    margin-bottom: clamp(1rem, 1.5vw, 2.5rem);
  }

  .rm-items-filter-title {
    margin: 0 0 clamp(0.4rem, 0.5vw, 0.9rem);
    font-size: clamp(0.72rem, 0.62vw, 1.3rem);
    color: var(--rm-red);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    border-left: 0.2rem solid var(--rm-red);
    padding-left: clamp(0.4rem, 0.5vw, 1rem);
  }

  .rm-items-filter-btn {
    display: block;
    width: fit-content;
    border: none;
    background: transparent;
    color: var(--rm-white);
    cursor: pointer;
    padding: clamp(0.15rem, 0.25vw, 0.4rem) clamp(0.4rem, 0.6vw, 1rem);
    font-family: inherit;
    font-size: clamp(0.65rem, 0.58vw, 1rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.35;
    transition: opacity 140ms ease;
  }

  .rm-items-filter-btn:hover {
    opacity: 0.65;
  }

  .rm-items-filter-btn.is-active {
    opacity: 1;
  }

  .rm-items-filter-count {
    font-size: clamp(0.55rem, 0.5vw, 0.85rem);
    opacity: 0.5;
    margin-left: 0.3em;
  }

  /* ── Right content ── */
  .rm-items-content {
    overflow-y: auto;
    height: 100%;
    padding: clamp(1.5rem, 2.5vh, 4rem) clamp(8rem, 14vw, 20rem) clamp(6rem, 10vh, 10rem) clamp(1.5rem, 2.5vw, 4rem);
    box-sizing: border-box;
  }

  .rm-items-sort-bar {
    display: flex;
    align-items: center;
    gap: clamp(0.3rem, 0.4vw, 0.6rem);
    margin-bottom: clamp(1rem, 1.5vw, 2.5rem);
    flex-wrap: wrap;
  }

  .rm-items-sort-btn {
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: transparent;
    color: var(--rm-white);
    cursor: pointer;
    padding: clamp(0.2rem, 0.25vw, 0.4rem) clamp(0.5rem, 0.6vw, 1rem);
    font-family: inherit;
    font-size: clamp(0.6rem, 0.55vw, 0.95rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.5;
    transition: opacity 140ms ease, background 140ms ease;
  }

  .rm-items-sort-btn:hover {
    opacity: 0.8;
  }

  .rm-items-sort-btn.is-active {
    opacity: 1;
    background: var(--rm-red);
    border-color: var(--rm-red);
  }

  .rm-items-sort-arrow {
    margin-left: 0.2em;
  }

  .rm-items-result-count {
    margin-left: auto;
    font-size: clamp(0.6rem, 0.55vw, 0.95rem);
    font-weight: 700;
    color: rgba(255, 255, 255, 0.4);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .rm-items-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(max(200px, 13vw), 1fr));
    gap: clamp(0.6rem, 0.6vw, 1.2rem);
  }

  .rm-item-card {
    background: var(--rm-black);
    border: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    cursor: pointer;
    transform: rotate(-0.5deg);
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 3% 100%);
    transition: transform 120ms ease;
    text-align: left;
    font-family: inherit;
    color: var(--rm-white);
  }

  .rm-item-card:nth-child(even) {
    transform: rotate(0.5deg);
  }

  .rm-item-card:hover {
    transform: rotate(0deg) scale(1.03);
    z-index: 2;
  }

  .rm-item-card-header {
    display: flex;
    align-items: center;
    gap: clamp(0.3rem, 0.4vw, 0.6rem);
    background: var(--rm-white);
    color: var(--rm-black);
    padding: clamp(0.25rem, 0.35vw, 0.6rem) clamp(0.7rem, 0.9vw, 1.6rem);
    margin: clamp(0.15rem, 0.2vw, 0.35rem) clamp(0.15rem, 0.2vw, 0.35rem) 0;
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 1.5% 100%);
  }

  .rm-item-card-category {
    font-size: clamp(0.6rem, 0.55vw, 0.95rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .rm-item-card-subcategory {
    font-size: clamp(0.5rem, 0.45vw, 0.8rem);
    font-weight: 600;
    opacity: 0.5;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .rm-item-card-body {
    padding: clamp(0.3rem, 0.4vw, 0.7rem) clamp(0.7rem, 0.9vw, 1.6rem) clamp(0.4rem, 0.5vw, 0.8rem) clamp(1rem, 1.2vw, 2rem);
  }

  .rm-item-card-name {
    margin: 0;
    font-size: clamp(0.75rem, 0.7vw, 1.2rem);
    font-weight: 800;
    line-height: 1.3;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .rm-item-card-brand {
    margin: clamp(0.1rem, 0.15vw, 0.2rem) 0 0;
    font-size: clamp(0.55rem, 0.5vw, 0.85rem);
    color: rgba(255, 255, 255, 0.45);
    font-weight: 600;
  }

  .rm-item-card-stats {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-top: clamp(0.3rem, 0.4vw, 0.6rem);
  }

  .rm-item-card-price {
    font-size: clamp(0.85rem, 0.8vw, 1.4rem);
    font-weight: 800;
  }

  .rm-item-card-daily {
    font-size: clamp(0.6rem, 0.55vw, 0.95rem);
    font-weight: 700;
    color: var(--rm-red);
  }

  .rm-item-card-days {
    margin: clamp(0.1rem, 0.15vw, 0.2rem) 0 0;
    font-size: clamp(0.55rem, 0.5vw, 0.85rem);
    color: rgba(255, 255, 255, 0.35);
    font-weight: 600;
  }

  /* ── Item detail ── */

  .rm-item-detail {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: clamp(1.5rem, 2vw, 3rem);
    overflow: hidden;
    height: 100%;
    padding: clamp(1.5rem, 2.5vh, 4rem) clamp(2rem, 3vw, 5rem) clamp(6rem, 10vh, 10rem);
    box-sizing: border-box;
  }

  .rm-item-detail-left {
    overflow-y: auto;
    padding: clamp(1rem, 2vw, 3rem);
  }

  .rm-item-detail-name {
    margin-bottom: clamp(0.5rem, 0.8vw, 1.5rem);
  }

  .rm-item-detail-brand {
    margin: 0 0 clamp(1rem, 1.5vw, 2.5rem);
    font-size: clamp(0.7rem, 0.65vw, 1.1rem);
    color: rgba(255, 255, 255, 0.5);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .rm-item-detail-table {
    display: flex;
    flex-direction: column;
    gap: clamp(0.2rem, 0.3vw, 0.5rem);
  }

  .rm-item-detail-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: clamp(0.15rem, 0.2vw, 0.35rem) 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .rm-item-detail-label {
    font-size: clamp(0.6rem, 0.55vw, 0.95rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: rgba(255, 255, 255, 0.5);
  }

  .rm-item-detail-value {
    font-size: clamp(0.75rem, 0.7vw, 1.2rem);
    font-weight: 800;
    letter-spacing: 0.04em;
  }

  .rm-item-detail-right {
    overflow-y: auto;
    padding: clamp(1rem, 2vw, 3rem);
  }

  .rm-item-extra-title {
    margin: 0 0 clamp(0.4rem, 0.5vw, 0.9rem);
    font-size: clamp(0.72rem, 0.62vw, 1.3rem);
    color: var(--rm-red);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    border-left: 0.2rem solid var(--rm-red);
    padding-left: clamp(0.4rem, 0.5vw, 1rem);
  }

  .rm-item-extra-list {
    display: flex;
    flex-direction: column;
    gap: clamp(0.15rem, 0.2vw, 0.35rem);
    margin-bottom: clamp(1.5rem, 2vw, 3rem);
  }

  .rm-item-extra-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: clamp(0.12rem, 0.15vw, 0.3rem) 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .rm-item-extra-key {
    font-size: clamp(0.6rem, 0.55vw, 0.95rem);
    font-weight: 700;
    color: rgba(255, 255, 255, 0.5);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .rm-item-extra-val {
    font-size: clamp(0.7rem, 0.65vw, 1.1rem);
    font-weight: 700;
  }

  .rm-item-image-wrap {
    margin-top: clamp(1rem, 1.5vw, 2.5rem);
  }

  .rm-item-image {
    max-width: 100%;
    max-height: 50vh;
    display: block;
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 3% 100%);
  }

  /* ─── Crafting screen ─── */

  .rm-craft-layout {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 2fr;
    overflow: hidden;
    height: 100%;
  }

  /* ── Left: recipe menu ── */
  .rm-craft-menu {
    display: flex;
    flex-direction: column;
    gap: clamp(0.3rem, 0.35vw, 0.5rem);
    overflow-y: auto;
    height: 100%;
    padding: clamp(1.5rem, 2.5vh, 4rem) clamp(0.8rem, 1.2vw, 2rem) clamp(6rem, 10vh, 10rem) clamp(1.5rem, 2.5vw, 4rem);
    box-sizing: border-box;
  }

  .rm-craft-menu::-webkit-scrollbar { width: 3px; }
  .rm-craft-menu::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.15); }

  .rm-craft-menu-item {
    display: flex;
    flex-direction: column;
    background: var(--rm-black);
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    color: var(--rm-white);
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 3% 100%);
    transition: transform 120ms ease;
  }

  .rm-craft-menu-item:hover {
    transform: rotate(0deg) scale(1.03) !important;
    z-index: 2;
  }

  .rm-craft-menu-item.is-active {
    outline: 2px solid var(--rm-red);
    outline-offset: -2px;
  }

  .rm-craft-menu-stripe {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--rm-white);
    color: var(--rm-black);
    padding: clamp(0.15rem, 0.2vw, 0.3rem) clamp(0.5rem, 0.7vw, 1.2rem);
    margin: clamp(0.1rem, 0.15vw, 0.25rem) clamp(0.1rem, 0.15vw, 0.25rem) 0;
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 1.5% 100%);
  }

  .rm-craft-menu-type {
    font-size: clamp(0.5rem, 0.45vw, 0.75rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .rm-craft-menu-time {
    font-size: clamp(0.45rem, 0.4vw, 0.65rem);
    font-weight: 600;
    opacity: 0.5;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .rm-craft-menu-body {
    padding: clamp(0.2rem, 0.3vw, 0.5rem) clamp(0.5rem, 0.7vw, 1.2rem) clamp(0.3rem, 0.4vw, 0.6rem) clamp(0.7rem, 0.9vw, 1.5rem);
  }

  .rm-craft-menu-name {
    display: block;
    font-size: clamp(0.65rem, 0.62vw, 1rem);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    line-height: 1.3;
  }

  .rm-craft-menu-sub {
    display: block;
    font-size: clamp(0.45rem, 0.42vw, 0.7rem);
    color: rgba(255, 255, 255, 0.35);
    font-weight: 600;
    margin-top: 1px;
    letter-spacing: 0.04em;
  }

  /* ── Right: detail column ── */
  .rm-craft-detail {
    display: flex;
    flex-direction: column;
    gap: clamp(0.5rem, 0.6vw, 0.8rem);
    overflow-y: auto;
    height: 100%;
    padding: clamp(1.5rem, 2.5vh, 4rem) clamp(8rem, 14vw, 20rem) clamp(6rem, 10vh, 10rem) clamp(1.5rem, 2.5vw, 4rem);
    box-sizing: border-box;
  }

  .rm-craft-detail::-webkit-scrollbar { width: 3px; }
  .rm-craft-detail::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.15); }

  .rm-craft-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
  }

  .rm-craft-empty-text {
    font-size: clamp(0.8rem, 0.8vw, 1.2rem);
    font-weight: 800;
    letter-spacing: 0.15em;
    color: rgba(255, 255, 255, 0.15);
    text-transform: uppercase;
  }

  /* ── Ingredients box ── */
  .rm-craft-ingredients {
    background: var(--rm-black);
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 2% 100%);
    padding: clamp(0.6rem, 0.8vw, 1rem) clamp(0.8rem, 1vw, 1.5rem);
    transform: rotate(-0.3deg);
  }

  .rm-craft-ing-header {
    display: flex;
    align-items: center;
    gap: 0.6em;
    margin-bottom: clamp(0.4rem, 0.5vw, 0.7rem);
  }

  .rm-craft-ing-label {
    font-size: clamp(0.55rem, 0.55vw, 0.85rem);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.15em;
    color: var(--rm-red);
    border-left: 0.2rem solid var(--rm-red);
    padding-left: clamp(0.3rem, 0.4vw, 0.6rem);
  }

  .rm-craft-ing-count {
    font-size: clamp(0.55rem, 0.55vw, 0.85rem);
    font-weight: 700;
    color: rgba(255, 255, 255, 0.35);
  }

  .rm-craft-ing-list {
    display: flex;
    flex-wrap: wrap;
    gap: clamp(0.25rem, 0.3vw, 0.4rem);
  }

  .rm-craft-ing-tag {
    font-size: clamp(0.58rem, 0.55vw, 0.9rem);
    font-weight: 700;
    padding: clamp(0.12rem, 0.15vw, 0.25rem) clamp(0.4rem, 0.5vw, 0.7rem);
    background: var(--rm-white);
    color: var(--rm-black);
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 2% 100%);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  /* ── Paper panel ── */
  .rm-craft-paper {
    flex: 1;
    background: rgba(5, 5, 5, 0.95);
    border: 2px solid rgba(255, 255, 255, 0.35);
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 1% 100%);
    padding: clamp(1.2rem, 1.5vw, 2rem) clamp(1.5rem, 2vw, 2.5rem);
    overflow-y: auto;
    position: relative;
    transform: rotate(0.2deg);
  }

  .rm-craft-paper::before {
    content: '';
    position: absolute;
    inset: 5px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    pointer-events: none;
  }

  .rm-craft-paper::-webkit-scrollbar { width: 3px; }
  .rm-craft-paper::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.12); }

  .rm-craft-paper-head {
    margin-bottom: clamp(1rem, 1.2vw, 1.8rem);
    padding-bottom: clamp(0.6rem, 0.8vw, 1rem);
    border-bottom: 2px solid var(--rm-white);
  }

  .rm-craft-paper-title {
    font-size: clamp(1.2rem, 1.3vw, 2rem);
    font-weight: 800;
    margin: 0 0 clamp(0.3rem, 0.4vw, 0.6rem);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .rm-craft-paper-tags {
    display: flex;
    flex-wrap: wrap;
    gap: clamp(0.3rem, 0.4vw, 0.5rem);
  }

  .rm-craft-paper-tag {
    font-size: clamp(0.55rem, 0.52vw, 0.8rem);
    font-weight: 700;
    padding: clamp(0.1rem, 0.12vw, 0.2rem) clamp(0.4rem, 0.5vw, 0.7rem);
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.6);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 3% 100%);
  }

  .rm-craft-paper-tag--red {
    background: var(--rm-red);
    color: var(--rm-white);
  }

  /* Steps */
  .rm-craft-steps {
    margin-bottom: clamp(1rem, 1.2vw, 1.8rem);
  }

  .rm-craft-section-label {
    font-size: clamp(0.55rem, 0.55vw, 0.85rem);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.15em;
    color: var(--rm-red);
    margin: 0 0 clamp(0.4rem, 0.5vw, 0.7rem);
    border-left: 0.2rem solid var(--rm-red);
    padding-left: clamp(0.3rem, 0.4vw, 0.6rem);
  }

  .rm-craft-step-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: clamp(0.2rem, 0.25vw, 0.35rem);
  }

  .rm-craft-step {
    display: flex;
    align-items: flex-start;
    gap: clamp(0.5rem, 0.6vw, 0.8rem);
    padding: clamp(0.3rem, 0.4vw, 0.5rem) 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .rm-craft-step-num {
    flex-shrink: 0;
    width: clamp(1.4rem, 1.3vw, 1.8rem);
    height: clamp(1.4rem, 1.3vw, 1.8rem);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: clamp(0.5rem, 0.48vw, 0.75rem);
    font-weight: 800;
    background: var(--rm-red);
    color: var(--rm-white);
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 8% 100%);
  }

  .rm-craft-step-text {
    font-size: clamp(0.65rem, 0.62vw, 1rem);
    font-weight: 600;
    color: rgba(255, 255, 255, 0.85);
    line-height: 1.7;
    letter-spacing: 0.01em;
    padding-top: clamp(0.15rem, 0.2vw, 0.3rem);
  }

  /* Tags & source */
  .rm-craft-foot-tags {
    display: flex;
    flex-wrap: wrap;
    gap: clamp(0.25rem, 0.3vw, 0.4rem);
    margin-bottom: clamp(0.5rem, 0.6vw, 0.8rem);
  }

  .rm-craft-foot-tag {
    font-size: clamp(0.5rem, 0.48vw, 0.75rem);
    font-weight: 700;
    padding: clamp(0.08rem, 0.1vw, 0.15rem) clamp(0.3rem, 0.35vw, 0.5rem);
    background: rgba(229, 25, 28, 0.15);
    color: var(--rm-red);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 4% 100%);
  }

  .rm-craft-source {
    font-size: clamp(0.5rem, 0.48vw, 0.75rem);
    font-weight: 700;
    color: rgba(255, 255, 255, 0.3);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    margin: 0;
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
