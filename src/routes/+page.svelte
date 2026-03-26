<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import P5Text from "$lib/P5Text.svelte";
  import P5Calendar from "$lib/P5Calendar.svelte";
  import P5MenuItem from "$lib/P5MenuItem.svelte";
  import type { LetterConfig } from "$lib/P5MenuItem.svelte";
  import type { AchievementData, Achievement, PackAchievements } from "$lib/types/achievement";
  import type { SkillData, SkillWithLevel, SkillNode } from "$lib/types/skill";
  import type { ItemData, ItemWithComputed, ItemSortKey, ItemSortOrder } from "$lib/types/item";
  import type { GalleryData, MediaItem } from "$lib/types/gallery";
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

  type MenuScreen = "main" | "status" | "achievements" | "skills" | "items" | "gallery" | "tasks";
  type MenuItemId = "status" | "skills" | "achievements" | "items" | "gallery" | "tasks";

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
      enabled: true,
    },
    {
      id: "tasks",
      label: "Tasks",
      description: "Daily and long-term task tracking.",
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
    tasks: [
      { char: 'T', size: '1.15em', yOffset: -3, rotate: -5, weight: 800 },
      { char: 'a', size: '0.88em', yOffset: 3, rotate: 4, color: 'black', rounded: true },
      { char: 'S', size: '1.08em', yOffset: -1, rotate: -3 },
      { char: 'k', size: '0.82em', yOffset: 2, rotate: 5, color: 'black', outline: true },
      { char: 'S', size: '0.78em', yOffset: -2, rotate: -4 },
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
  let itemDetailMode = $state(false);
  let itemFilterSource = $state<string | null>(null);
  let itemFilterCategory = $state<string | null>(null);
  let itemSortKey = $state<ItemSortKey>('name');
  let itemSortOrder = $state<ItemSortOrder>('asc');


  let galleryLoading = $state(false);
  let galleryError = $state<string | null>(null);
  let galleryData = $state<GalleryData | null>(null);
  let selectedMedia = $state<MediaItem | null>(null);

  type GallerySortKey = 'rating' | 'date' | 'playtime';
  type GallerySortOrder = 'asc' | 'desc';
  let gallerySortKey = $state<GallerySortKey>('rating');
  let gallerySortOrder = $state<GallerySortOrder>('desc');

  type GalleryCategory = 'anime' | 'game' | 'tv' | 'movie' | 'book';
  let galleryActiveCategory = $state<GalleryCategory>('anime');

  const GALLERY_CATEGORIES: {
    id: GalleryCategory;
    label: string;
    icon: string;
    mediaTypes: string[];
    sortOptions: { key: GallerySortKey; label: string }[];
  }[] = [
    { id: 'anime', label: 'Anime', icon: 'A', mediaTypes: ['anime'],
      sortOptions: [{ key: 'rating', label: '评分' }, { key: 'date', label: '看完' }] },
    { id: 'game', label: 'Games', icon: 'G', mediaTypes: ['game'],
      sortOptions: [{ key: 'playtime', label: '时长' }, { key: 'rating', label: '评分' }] },
    { id: 'tv', label: 'TV', icon: 'T', mediaTypes: ['tv'],
      sortOptions: [{ key: 'rating', label: '评分' }, { key: 'date', label: '看完' }] },
    { id: 'movie', label: 'Movie', icon: 'M', mediaTypes: ['movie'],
      sortOptions: [{ key: 'rating', label: '评分' }, { key: 'date', label: '看完' }] },
    { id: 'book', label: 'Book', icon: 'B', mediaTypes: ['book'],
      sortOptions: [{ key: 'rating', label: '评分' }, { key: 'date', label: '读完' }] },
  ];

  const GALLERY_CATEGORY_LETTERS: Record<GalleryCategory, LetterConfig[]> = {
    anime: [
      { char: 'A', size: '1.15em', yOffset: -2, rotate: -5, weight: 800 },
      { char: 'n', size: '0.85em', yOffset: 3, rotate: 4, color: 'black', rounded: true },
      { char: 'i', size: '0.80em', yOffset: -1, rotate: -3 },
      { char: 'M', size: '1.0em', yOffset: 2, rotate: 5, color: 'black', outline: true },
      { char: 'e', size: '0.78em', yOffset: -2, rotate: -4 },
    ],
    game: [
      { char: 'G', size: '1.18em', yOffset: -3, rotate: -6, weight: 800 },
      { char: 'a', size: '0.85em', yOffset: 3, rotate: 4, color: 'black' },
      { char: 'M', size: '0.80em', yOffset: -1, rotate: -3 },
      { char: 'e', size: '0.92em', yOffset: 2, rotate: 5, color: 'black', rounded: true },
      { char: 'S', size: '1.05em', yOffset: -2, rotate: -4 },
    ],
    tv: [
      { char: 'T', size: '1.2em', yOffset: -3, rotate: -5, weight: 800 },
      { char: 'V', size: '1.1em', yOffset: 2, rotate: 4, color: 'black', outline: true },
    ],
    movie: [
      { char: 'M', size: '1.15em', yOffset: -2, rotate: -5, weight: 800 },
      { char: 'o', size: '0.82em', yOffset: 3, rotate: 4, color: 'black', rounded: true },
      { char: 'V', size: '0.90em', yOffset: -1, rotate: -3 },
      { char: 'i', size: '0.78em', yOffset: 2, rotate: 5, color: 'black' },
      { char: 'E', size: '1.05em', yOffset: -2, rotate: -4 },
    ],
    book: [
      { char: 'B', size: '1.18em', yOffset: -3, rotate: -6, weight: 800 },
      { char: 'o', size: '0.82em', yOffset: 3, rotate: 3, color: 'black', rounded: true },
      { char: 'O', size: '0.90em', yOffset: -1, rotate: -4 },
      { char: 'K', size: '1.05em', yOffset: 2, rotate: 5, color: 'black', outline: true },
    ],
  };

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
    { rot: 2,   clip: 'polygon(0% 6%, 98% 0%, 100% 100%, 2% 92%)' },   // Tasks
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
    itemDetailMode = false;
    itemFilterSource = null;
    itemFilterCategory = null;
    if (!itemData && !itemLoading) {
      await loadItemData();
    }
  }

  // ── Gallery helpers ──

  async function loadGalleryData() {
    galleryLoading = true;
    galleryError = null;

    try {
      galleryData = await invoke<GalleryData>("load_gallery");
    } catch (error) {
      galleryError =
        typeof error === "string"
          ? error
          : "Failed to load gallery data.";
      galleryData = null;
    } finally {
      galleryLoading = false;
    }
  }

  async function openGalleryScreen() {
    currentScreen = "gallery";
    selectedMedia = null;
    galleryActiveCategory = 'anime';
    gallerySortKey = 'rating';
    gallerySortOrder = 'desc';
    if (!galleryData && !galleryLoading) {
      await loadGalleryData();
    }
  }

  function selectGalleryCategory(id: GalleryCategory) {
    galleryActiveCategory = id;
    const cat = GALLERY_CATEGORIES.find(c => c.id === id)!;
    gallerySortKey = cat.sortOptions[0].key;
    gallerySortOrder = 'desc';
  }

  function getActiveSortOptions(): { key: GallerySortKey; label: string }[] {
    return GALLERY_CATEGORIES.find(c => c.id === galleryActiveCategory)?.sortOptions ?? [];
  }

  function toggleGallerySort(key: GallerySortKey) {
    if (gallerySortKey === key) {
      gallerySortOrder = gallerySortOrder === 'asc' ? 'desc' : 'asc';
    } else {
      gallerySortKey = key;
      gallerySortOrder = 'desc';
    }
  }

  function getFilteredGalleryItems(): MediaItem[] {
    if (!galleryData) return [];

    const cat = GALLERY_CATEGORIES.find(c => c.id === galleryActiveCategory);
    if (!cat) return [];

    let items = galleryData.items.filter(i => cat.mediaTypes.includes(getMediaType(i)));

    const dir = gallerySortOrder === 'asc' ? 1 : -1;
    items = [...items].sort((a, b) => {
      switch (gallerySortKey) {
        case 'rating': {
          const ar = a.my_rating ?? a.rating ?? -1;
          const br = b.my_rating ?? b.rating ?? -1;
          if (ar !== br) return dir * (ar - br);
          return a.name.localeCompare(b.name);
        }
        case 'playtime': {
          const ah = (a.extra.playtime_hours as number) ?? -1;
          const bh = (b.extra.playtime_hours as number) ?? -1;
          if (ah !== bh) return dir * (ah - bh);
          return a.name.localeCompare(b.name);
        }
        case 'date': {
          const ad = a.date_finished ?? '';
          const bd = b.date_finished ?? '';
          if (ad !== bd) return dir * ad.localeCompare(bd);
          return a.name.localeCompare(b.name);
        }
        default:
          return 0;
      }
    });

    return items;
  }

  function getCardRotation(index: number): string {
    const rotations = [-1.2, 0.8, -0.5, 1.4, -1.0, 0.6, -1.5, 1.1, -0.3, 0.9];
    return `${rotations[index % rotations.length]}deg`;
  }

  function getDisplayRating(item: MediaItem): { value: number; isPersonal: boolean } | null {
    if (item.my_rating !== null) return { value: item.my_rating, isPersonal: true };
    if (item.rating !== null) return { value: item.rating, isPersonal: false };
    return null;
  }

  function formatRating(rating: number | null): string {
    if (rating === null || rating === undefined) return "—";
    return rating.toFixed(1);
  }

  /** Return array of 10 entries: 'full' | 'half' | 'empty' for star display */
  function ratingToStars(rating: number): ('full' | 'half' | 'empty')[] {
    const stars: ('full' | 'half' | 'empty')[] = [];
    const rounded = Math.round(rating * 2) / 2; // round to nearest 0.5
    for (let i = 1; i <= 10; i++) {
      if (i <= rounded) stars.push('full');
      else if (i - 0.5 === rounded) stars.push('half');
      else stars.push('empty');
    }
    return stars;
  }

  function getMediaType(item: MediaItem): string {
    if (!galleryData) return 'unknown';
    const src = galleryData.sources.find(s => s.id === item.source_id);
    return src?.media_type ?? 'unknown';
  }

  function proxyCover(url: string | null | undefined): string | undefined {
    if (!url) return undefined;
    if (url.includes('doubanio.com')) {
      return `http://imgproxy.localhost/${encodeURIComponent(url)}`;
    }
    return url;
  }

  function handleCoverError(e: Event) {
    const img = e.target as HTMLImageElement;
    const retries = Number(img.dataset.retries ?? 0);
    if (retries < 3) {
      img.dataset.retries = String(retries + 1);
      setTimeout(() => {
        const src = img.src;
        img.src = '';
        img.src = src;
      }, 1000 * (retries + 1));
      return;
    }
    img.style.display = 'none';
    const fallback = img.nextElementSibling as HTMLElement | null;
    if (fallback) fallback.style.display = 'flex';
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

  function getItemSortValue(item: ItemWithComputed): string {
    switch (itemSortKey) {
      case 'price': return formatPrice(item.price);
      case 'daily_cost': return formatDailyCost(item.daily_cost) + '/d';
      case 'date': return item.purchase_date ?? '—';
      case 'days_owned': return item.days_owned !== null ? item.days_owned + '天' : '—';
      default: return '';
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

  function getFilteredItemStats(): { total: number; value: number; avgDaily: number } {
    const items = getFilteredSortedItems();
    const total = items.length;
    const value = items.reduce((sum, i) => sum + (i.price ?? 0), 0);
    const dailyCosts = items.map(i => i.daily_cost).filter((c): c is number => c !== null);
    const avgDaily = dailyCosts.length > 0 ? dailyCosts.reduce((a, b) => a + b, 0) / dailyCosts.length : 0;
    return { total, value, avgDaily };
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
    } else if (item.id === "gallery") {
      await openGalleryScreen();
    } else if (item.id === "tasks") {
      setMenuFeedback("Tasks module coming soon.");
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      if (currentScreen === "skills" && selectedSkill) {
        selectedSkill = null;
      } else if (currentScreen === "items" && itemDetailMode) {
        itemDetailMode = false;
      } else if (currentScreen === "items" && selectedItem) {
        selectedItem = null;
      } else if (currentScreen === "gallery" && selectedMedia) {
        selectedMedia = null;
      } else if (currentScreen === "status" || currentScreen === "achievements" || currentScreen === "skills" || currentScreen === "items" || currentScreen === "gallery") {
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


    {#if currentScreen === "gallery"}
      <section class="rm-stage">
        <div class="rm-items-title">
          <P5Text text="Gallery" fontSize={82} />
        </div>

        <button type="button" class="rm-back-btn" onclick={() => {
          if (selectedMedia) {
            selectedMedia = null;
          } else {
            currentScreen = "main";
          }
        }}>
          <img src="/ui/back.png" alt="Back" class="rm-back-img" />
        </button>

        {#if galleryLoading}
          <p class="state-text" style="padding: 2rem;">Loading gallery...</p>
        {:else if galleryError}
          <p class="state-text error" style="padding: 2rem;">{galleryError}</p>
        {:else if galleryData && !selectedMedia}
          <div class="rm-gallery-layout">
            <!-- LEFT: category nav + sort -->
            <div class="rm-gallery-sidebar">
              <nav class="rm-gallery-nav">
                {#each GALLERY_CATEGORIES as cat}
                  <button
                    type="button"
                    class="rm-gallery-nav-item"
                    class:is-active={galleryActiveCategory === cat.id}
                    onclick={() => selectGalleryCategory(cat.id)}
                  >
                    <span class="rm-gallery-nav-icon">{cat.icon}</span>
                    <P5MenuItem letters={GALLERY_CATEGORY_LETTERS[cat.id]} />
                  </button>
                {/each}
              </nav>

              <div class="rm-items-filter-section">
                <h4 class="rm-items-filter-title">Sort</h4>
                {#each getActiveSortOptions() as opt}
                  <button
                    type="button"
                    class="rm-items-filter-btn"
                    class:is-active={gallerySortKey === opt.key}
                    onclick={() => toggleGallerySort(opt.key)}
                  >
                    {opt.label}
                    {#if gallerySortKey === opt.key}
                      <span class="rm-items-sort-arrow">{gallerySortOrder === 'asc' ? '↑' : '↓'}</span>
                    {/if}
                  </button>
                {/each}
              </div>
            </div>

            <!-- RIGHT: waterfall cover wall -->
            <div class="rm-gallery-content">
              {#if getFilteredGalleryItems().length === 0}
                <p class="state-text" style="padding: 2rem;">No items match the current filter.</p>
              {:else}
                <div class="rm-gallery-wall">
                  {#each getFilteredGalleryItems() as item, i (item.id)}
                    {@const displayRating = getDisplayRating(item)}
                    {@const itemMediaType = getMediaType(item)}
                    <button
                      type="button"
                      class="rm-gallery-card"
                      style="transform: rotate({getCardRotation(i)});"
                      onclick={() => { selectedMedia = item; }}
                    >
                      <div class="rm-gallery-card-frame">
                        {#if item.cover}
                          <img
                            src={proxyCover(item.cover)}
                            alt={item.name}
                            class="rm-gallery-card-img"
                            loading="lazy"
                            onerror={handleCoverError}
                          />
                          <div class="rm-gallery-card-fallback" style="display:none;">
                          </div>
                        {:else}
                          <div class="rm-gallery-card-fallback">
                          </div>
                        {/if}
                      </div>
                      <div class="rm-gallery-card-info">
                        <span class="rm-gallery-card-name">{item.name}</span>
                        {#if itemMediaType === 'game'}
                          <div class="rm-gallery-card-game-meta">
                            {#if item.extra.playtime_hours}
                              <span class="rm-gallery-card-playtime">{item.extra.playtime_hours}h</span>
                            {/if}
                            {#if item.extra.achievement_total}
                              <div class="rm-gallery-card-ach">
                                <div class="rm-gallery-card-ach-bar">
                                  <div class="rm-gallery-card-ach-fill" style="width: {((item.extra.achievement_unlocked as number ?? 0) / (item.extra.achievement_total as number) * 100).toFixed(0)}%"></div>
                                </div>
                                <span class="rm-gallery-card-ach-text">{item.extra.achievement_unlocked ?? 0}/{item.extra.achievement_total}</span>
                              </div>
                            {/if}
                          </div>
                        {:else if displayRating}
                          <div class="rm-gallery-card-stars" class:is-community={!displayRating.isPersonal}>
                            {#each ratingToStars(displayRating.value) as star}
                              <span class="rm-gallery-star rm-gallery-star--{star}">★</span>
                            {/each}
                          </div>
                        {/if}
                      </div>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        {:else if galleryData && selectedMedia}
          <!-- Detail view -->
          <div class="rm-gallery-detail">
            <div class="rm-gallery-detail-inner">
              <div class="rm-gallery-detail-cover">
                {#if selectedMedia.cover}
                  <img src={proxyCover(selectedMedia.cover)} alt={selectedMedia.name} class="rm-gallery-detail-img" />
                {:else}
                  <div class="rm-gallery-card-placeholder rm-gallery-detail-placeholder">
                    <span class="rm-gallery-card-placeholder-text">{selectedMedia.name.charAt(0)}</span>
                  </div>
                {/if}
              </div>
              <div class="rm-gallery-detail-info">
                <h2 class="rm-gallery-detail-title">{selectedMedia.name}</h2>
                {#if selectedMedia.name_original}
                  <p class="rm-gallery-detail-original">{selectedMedia.name_original}</p>
                {/if}

                {#if getMediaType(selectedMedia) === 'game'}
                  <!-- Game-specific detail -->
                  <div class="rm-gallery-detail-meta">
                    {#if selectedMedia.extra.playtime_hours != null}
                      <div class="rm-gallery-detail-row">
                        <span class="rm-gallery-detail-label">PLAYTIME</span>
                        <span class="rm-gallery-detail-value">{selectedMedia.extra.playtime_hours}h</span>
                      </div>
                    {/if}
                    {#if selectedMedia.extra.achievement_total}
                      <div class="rm-gallery-detail-row">
                        <span class="rm-gallery-detail-label">ACHIEVEMENTS</span>
                        <span class="rm-gallery-detail-value">{selectedMedia.extra.achievement_unlocked ?? 0} / {selectedMedia.extra.achievement_total}</span>
                      </div>
                      <div class="rm-gallery-detail-ach-bar">
                        <div class="rm-gallery-detail-ach-fill" style="width: {((selectedMedia.extra.achievement_unlocked as number ?? 0) / (selectedMedia.extra.achievement_total as number) * 100).toFixed(0)}%"></div>
                      </div>
                    {/if}
                    {#if selectedMedia.extra.release_date}
                      <div class="rm-gallery-detail-row">
                        <span class="rm-gallery-detail-label">RELEASE</span>
                        <span class="rm-gallery-detail-value">{selectedMedia.extra.release_date}</span>
                      </div>
                    {/if}
                    {#if selectedMedia.rating !== null}
                      <div class="rm-gallery-detail-row">
                        <span class="rm-gallery-detail-label">METACRITIC</span>
                        <span class="rm-gallery-detail-value">{formatRating(selectedMedia.rating)}</span>
                      </div>
                    {/if}
                    {#if selectedMedia.extra.steam_url}
                      <div class="rm-gallery-detail-row">
                        <span class="rm-gallery-detail-label">STEAM</span>
                        <span class="rm-gallery-detail-value">{selectedMedia.extra.steam_url}</span>
                      </div>
                    {/if}
                  </div>
                {:else}
                  <!-- Default (anime/media) detail -->
                  <div class="rm-gallery-detail-meta">
                    {#if selectedMedia.rating !== null}
                      <div class="rm-gallery-detail-row">
                        <span class="rm-gallery-detail-label">RATING</span>
                        <span class="rm-gallery-detail-value">{formatRating(selectedMedia.rating)}</span>
                      </div>
                    {/if}
                    {#if selectedMedia.my_rating !== null}
                      <div class="rm-gallery-detail-row">
                        <span class="rm-gallery-detail-label">MY RATING</span>
                        <span class="rm-gallery-detail-value rm-gallery-detail-myrating">{formatRating(selectedMedia.my_rating)}</span>
                      </div>
                    {/if}
                    {#if selectedMedia.episodes !== null}
                      <div class="rm-gallery-detail-row">
                        <span class="rm-gallery-detail-label">EPISODES</span>
                        <span class="rm-gallery-detail-value">{selectedMedia.episodes}</span>
                      </div>
                    {/if}
                    {#if selectedMedia.date_started}
                      <div class="rm-gallery-detail-row">
                        <span class="rm-gallery-detail-label">STARTED</span>
                        <span class="rm-gallery-detail-value">{selectedMedia.date_started}</span>
                      </div>
                    {/if}
                    {#if selectedMedia.date_finished}
                      <div class="rm-gallery-detail-row">
                        <span class="rm-gallery-detail-label">FINISHED</span>
                        <span class="rm-gallery-detail-value">{selectedMedia.date_finished}</span>
                      </div>
                    {/if}
                  </div>
                {/if}

                {#if selectedMedia.tags.length > 0}
                  <div class="rm-gallery-detail-tags">
                    {#each selectedMedia.tags as tag}
                      <span class="rm-gallery-detail-tag">{tag}</span>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          </div>
        {:else}
          <p class="state-text" style="padding: 2rem;">Gallery data is not available yet.</p>
        {/if}
      </section>
    {/if}

    {#if currentScreen === "items"}
      <section class="rm-stage">
        <div class="rm-items-title">
          <P5Text text="Items" fontSize={82} />
        </div>

        {#if itemLoading}
          <p class="state-text" style="padding: 2rem;">Loading items...</p>
        {:else if itemError}
          <p class="state-text error" style="padding: 2rem;">{itemError}</p>
        {:else if itemData && !itemDetailMode}
          {@const filteredStats = getFilteredItemStats()}
          <div class="rm-items-layout">
            <!-- LEFT: Category nav + stats -->
            <div class="rm-items-sidebar">
              <nav class="rm-items-cat-nav">
                <button
                  type="button"
                  class="rm-items-cat-btn"
                  class:is-active={!itemFilterCategory}
                  onclick={() => { itemFilterCategory = null; selectedItem = null; }}
                >
                  <span class="rm-items-cat-label">ALL</span>
                  <span class="rm-items-cat-count">{itemData.stats.total_items}</span>
                </button>
                {#each itemData.stats.by_main_category as cat, i}
                  <button
                    type="button"
                    class="rm-items-cat-btn"
                    class:is-active={itemFilterCategory === cat.name}
                    class:rm-items-cat-even={i % 2 === 0}
                    onclick={() => { itemFilterCategory = itemFilterCategory === cat.name ? null : cat.name; selectedItem = null; }}
                  >
                    <span class="rm-items-cat-label">{cat.name}</span>
                    <span class="rm-items-cat-count">{cat.item_count}</span>
                  </button>
                {/each}
              </nav>

              <div class="rm-items-stat-block">
                <div class="rm-items-stat-row">
                  <span class="rm-items-stat-label">TOTAL</span>
                  <span class="rm-items-stat-value">{filteredStats.total}</span>
                </div>
                <div class="rm-items-stat-row">
                  <span class="rm-items-stat-label">VALUE</span>
                  <span class="rm-items-stat-value">{formatPrice(filteredStats.value)}</span>
                </div>
                <div class="rm-items-stat-row">
                  <span class="rm-items-stat-label">AVG/DAY</span>
                  <span class="rm-items-stat-value rm-items-daily">{formatDailyCost(filteredStats.avgDaily)}</span>
                </div>
              </div>

              <button type="button" class="rm-items-back-btn" onclick={() => { currentScreen = "main"; }}>
                <img src="/ui/back.png" alt="Back" class="rm-back-img" />
              </button>
            </div>

            <!-- RIGHT: Sort + list + summary -->
            <div class="rm-items-content">
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
                <span class="rm-items-result-count">{filteredStats.total}</span>
              </div>

              <div class="rm-items-list">
                {#each getFilteredSortedItems() as item, i}
                  {@const sortVal = getItemSortValue(item)}
                  <button
                    type="button"
                    class="rm-item-row"
                    class:is-selected={selectedItem?.id === item.id}
                    onclick={() => { selectedItem = item; }}
                  >
                    <span class="rm-item-row-name">{item.name}</span>
                    {#if sortVal}
                      <span class="rm-item-row-attr">{sortVal}</span>
                    {/if}
                  </button>
                {/each}
              </div>

              <div class="rm-items-summary">
                {#if selectedItem}
                  <span class="rm-items-summary-chip">{formatPrice(selectedItem.price)}</span>
                  {#if selectedItem.days_owned !== null}
                    <span class="rm-items-summary-chip">{selectedItem.days_owned}天</span>
                  {/if}
                  <span class="rm-items-summary-chip rm-items-daily">{formatDailyCost(selectedItem.daily_cost)}/d</span>
                  {#if selectedItem.color}
                    <span class="rm-items-summary-chip">{selectedItem.color}</span>
                  {/if}
                  {#each Object.entries(selectedItem.extra).slice(0, 2) as [, val]}
                    <span class="rm-items-summary-chip">{formatExtraValue(val)}</span>
                  {/each}
                  <button
                    type="button"
                    class="rm-items-detail-btn"
                    onclick={() => { itemDetailMode = true; }}
                  >详情 →</button>
                {:else}
                  <span class="rm-items-summary-hint">选择物品查看摘要</span>
                {/if}
              </div>
            </div>
          </div>

        {:else if itemData && itemDetailMode && selectedItem}
          <!-- Item detail view (Gallery-style) -->
          <div class="rm-gallery-detail">
            <button type="button" class="rm-items-back-btn rm-items-back-btn--detail" onclick={() => { itemDetailMode = false; }}>
              <img src="/ui/back.png" alt="Back" class="rm-back-img" />
            </button>

            <div class="rm-gallery-detail-inner">
              {#if selectedItem.image}
                <div class="rm-gallery-detail-cover">
                  <img
                    src={convertFileSrc(selectedItem.image)}
                    alt={selectedItem.name}
                    class="rm-gallery-detail-img"
                  />
                </div>
              {/if}

              <div class="rm-gallery-detail-info">
                <h2 class="rm-gallery-detail-title">{selectedItem.name}</h2>
                {#if selectedItem.brand}
                  <p class="rm-gallery-detail-original">{selectedItem.brand}</p>
                {/if}

                <div class="rm-gallery-detail-meta">
                  {#if selectedItem.price !== null}
                    <div class="rm-gallery-detail-row">
                      <span class="rm-gallery-detail-label">PRICE</span>
                      <span class="rm-gallery-detail-value">{formatPrice(selectedItem.price)}</span>
                    </div>
                  {/if}
                  {#if selectedItem.daily_cost !== null}
                    <div class="rm-gallery-detail-row">
                      <span class="rm-gallery-detail-label">DAILY</span>
                      <span class="rm-gallery-detail-value" style="color: var(--rm-red)">{formatDailyCost(selectedItem.daily_cost)}/day</span>
                    </div>
                  {/if}
                  {#if selectedItem.days_owned !== null}
                    <div class="rm-gallery-detail-row">
                      <span class="rm-gallery-detail-label">OWNED</span>
                      <span class="rm-gallery-detail-value">{selectedItem.days_owned} days</span>
                    </div>
                  {/if}
                  {#if selectedItem.purchase_date}
                    <div class="rm-gallery-detail-row">
                      <span class="rm-gallery-detail-label">DATE</span>
                      <span class="rm-gallery-detail-value">{selectedItem.purchase_date}</span>
                    </div>
                  {/if}
                  {#if selectedItem.purchase_channel}
                    <div class="rm-gallery-detail-row">
                      <span class="rm-gallery-detail-label">FROM</span>
                      <span class="rm-gallery-detail-value">{selectedItem.purchase_channel}</span>
                    </div>
                  {/if}
                  {#if selectedItem.main_category}
                    <div class="rm-gallery-detail-row">
                      <span class="rm-gallery-detail-label">CATEGORY</span>
                      <span class="rm-gallery-detail-value">{selectedItem.main_category}{selectedItem.sub_category ? ` / ${selectedItem.sub_category}` : ''}</span>
                    </div>
                  {/if}
                  {#if selectedItem.color}
                    <div class="rm-gallery-detail-row">
                      <span class="rm-gallery-detail-label">COLOR</span>
                      <span class="rm-gallery-detail-value">{selectedItem.color}</span>
                    </div>
                  {/if}
                </div>

                {#if Object.keys(selectedItem.extra).length > 0}
                  <div class="rm-gallery-detail-tags">
                    {#each Object.entries(selectedItem.extra) as [key, val]}
                      <span class="rm-gallery-detail-tag">{key}: {formatExtraValue(val)}</span>
                    {/each}
                  </div>
                {/if}
              </div>
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
    display: grid;
    grid-template-columns: clamp(10rem, 20vw, 18rem) 1fr;
    overflow: hidden;
    height: 75vh;
    margin: auto 0;
  }

  /* ── Left sidebar: category nav + stats ── */
  .rm-items-sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: clamp(1rem, 1.5vh, 2rem) clamp(1rem, 1.5vw, 2.5rem) clamp(1rem, 1.5vh, 2rem);
    box-sizing: border-box;
    overflow-y: auto;
  }

  .rm-items-cat-nav {
    display: flex;
    flex-direction: column;
    gap: clamp(0.3rem, 0.4vw, 0.6rem);
    margin-bottom: auto;
  }

  .rm-items-cat-btn {
    display: flex;
    align-items: center;
    gap: clamp(0.3rem, 0.4vw, 0.6rem);
    width: fit-content;
    border: none;
    background: rgba(255, 255, 255, 0.06);
    color: var(--rm-white);
    cursor: pointer;
    padding: clamp(0.3rem, 0.4vw, 0.6rem) clamp(0.8rem, 1vw, 1.6rem);
    font-family: inherit;
    font-size: clamp(0.65rem, 0.6vw, 1rem);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    clip-path: polygon(4% 0%, 100% 0%, 96% 100%, 0% 100%);
    transform: skewX(-5deg);
    opacity: 0.45;
    transition: opacity 140ms ease, background 140ms ease, color 140ms ease;
  }

  /* Snake layout: even items align right */
  .rm-items-cat-btn.rm-items-cat-even {
    align-self: flex-end;
  }

  .rm-items-cat-btn:hover {
    opacity: 0.75;
    background: rgba(255, 255, 255, 0.12);
  }

  .rm-items-cat-btn.is-active {
    opacity: 1;
    background: var(--rm-red);
    color: var(--rm-white);
  }

  .rm-items-cat-label {
    transform: skewX(5deg);
  }

  .rm-items-cat-count {
    transform: skewX(5deg);
    font-size: 0.75em;
    opacity: 0.55;
  }

  .rm-items-cat-btn.is-active .rm-items-cat-count {
    opacity: 0.8;
  }

  .rm-items-stat-block {
    display: flex;
    flex-direction: column;
    gap: clamp(0.15rem, 0.2vw, 0.4rem);
    margin-top: clamp(1.5rem, 2.5vh, 3rem);
    padding-top: clamp(1rem, 1.5vh, 2rem);
    border-top: 2px solid rgba(255, 255, 255, 0.08);
  }

  .rm-items-stat-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: clamp(0.1rem, 0.15vw, 0.25rem) 0;
  }

  .rm-items-stat-label {
    font-size: clamp(0.55rem, 0.5vw, 0.85rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: rgba(255, 255, 255, 0.4);
  }

  .rm-items-stat-value {
    font-size: clamp(0.7rem, 0.65vw, 1.1rem);
    font-weight: 800;
    letter-spacing: 0.04em;
  }

  .rm-items-daily {
    color: var(--rm-red);
  }

  .rm-items-back-btn {
    display: block;
    margin-top: clamp(1rem, 1.5vh, 2rem);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    width: fit-content;
  }

  .rm-items-back-btn--detail {
    position: fixed;
    bottom: clamp(1.5rem, 2.5vh, 4rem);
    left: clamp(1.5rem, 2.5vw, 4rem);
    z-index: 10;
  }

  /* ── Right content: sort + list + summary ── */
  .rm-items-content {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: clamp(1rem, 1.5vh, 2rem) clamp(6rem, 10vw, 16rem) clamp(1rem, 1.5vh, 2rem) clamp(1.5rem, 2vw, 3rem);
    box-sizing: border-box;
    overflow: hidden;
  }

  .rm-items-sort-bar {
    display: flex;
    align-items: center;
    gap: clamp(0.3rem, 0.4vw, 0.6rem);
    margin-bottom: clamp(0.6rem, 0.8vw, 1.2rem);
    flex-wrap: wrap;
    flex-shrink: 0;
  }

  .rm-items-sort-btn {
    border: none;
    background: rgba(255, 255, 255, 0.06);
    color: var(--rm-white);
    cursor: pointer;
    padding: clamp(0.2rem, 0.25vw, 0.4rem) clamp(0.5rem, 0.6vw, 1rem);
    font-family: inherit;
    font-size: clamp(0.58rem, 0.52vw, 0.9rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.45;
    transform: skewX(-3deg);
    transition: opacity 140ms ease, background 140ms ease;
  }

  .rm-items-sort-btn:hover {
    opacity: 0.75;
  }

  .rm-items-sort-btn.is-active {
    opacity: 1;
    background: var(--rm-red);
  }

  .rm-items-sort-arrow {
    margin-left: 0.2em;
  }

  .rm-items-result-count {
    margin-left: auto;
    font-size: clamp(0.55rem, 0.5vw, 0.85rem);
    font-weight: 800;
    color: rgba(255, 255, 255, 0.35);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  /* ── Item list (parallelogram) ── */
  .rm-items-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: clamp(0.1rem, 0.15vw, 0.25rem);
    transform: skewX(-4deg);
    border: 2px solid var(--rm-white);
    padding: clamp(0.3rem, 0.4vw, 0.6rem);
    background: var(--rm-black);
  }

  .rm-item-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border: none;
    background: transparent;
    color: var(--rm-white);
    cursor: pointer;
    padding: clamp(0.4rem, 0.55vw, 0.9rem) clamp(1rem, 1.2vw, 2rem);
    font-family: inherit;
    font-size: clamp(1.1rem, 1.1vw, 1.8rem);
    font-weight: 800;
    text-align: left;
    transition: background 140ms ease;
    flex-shrink: 0;
    transform: skewX(4deg);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .rm-item-row:last-child {
    border-bottom: none;
  }

  .rm-item-row:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .rm-item-row.is-selected {
    background: var(--rm-red);
  }

  .rm-item-row-name {
    letter-spacing: 0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }

  .rm-item-row-attr {
    flex-shrink: 0;
    margin-left: clamp(0.8rem, 1vw, 1.6rem);
    font-weight: 700;
    opacity: 0.6;
    white-space: nowrap;
    font-size: 0.85em;
  }

  .rm-item-row.is-selected .rm-item-row-attr {
    opacity: 0.9;
  }

  /* ── Bottom summary bar ── */
  .rm-items-summary {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: clamp(0.4rem, 0.5vw, 0.8rem);
    padding: clamp(0.6rem, 0.8vw, 1.2rem) clamp(0.8rem, 1vw, 1.6rem);
    border-top: 2px solid rgba(255, 255, 255, 0.1);
    min-height: clamp(2rem, 3vw, 3.5rem);
    flex-wrap: wrap;
  }

  .rm-items-summary-chip {
    font-size: clamp(0.6rem, 0.55vw, 0.95rem);
    font-weight: 800;
    letter-spacing: 0.04em;
    padding: clamp(0.15rem, 0.2vw, 0.3rem) clamp(0.4rem, 0.5vw, 0.8rem);
    background: rgba(255, 255, 255, 0.08);
    clip-path: polygon(3% 0%, 100% 0%, 97% 100%, 0% 100%);
  }

  .rm-items-summary-hint {
    font-size: clamp(0.6rem, 0.55vw, 0.95rem);
    font-weight: 600;
    color: rgba(255, 255, 255, 0.25);
    letter-spacing: 0.04em;
  }

  .rm-items-detail-btn {
    margin-left: auto;
    border: none;
    background: var(--rm-red);
    color: var(--rm-white);
    cursor: pointer;
    padding: clamp(0.3rem, 0.35vw, 0.5rem) clamp(0.8rem, 1vw, 1.6rem);
    font-family: inherit;
    font-size: clamp(0.6rem, 0.55vw, 0.95rem);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    clip-path: polygon(4% 0%, 100% 0%, 96% 100%, 0% 100%);
    transform: skewX(-3deg);
    transition: opacity 140ms ease;
  }

  .rm-items-detail-btn:hover {
    opacity: 0.85;
  }

  /* ── Shared filter styles (used by Gallery) ── */

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

  /* ── Gallery ── */

  .rm-gallery-layout {
    flex: 1;
    display: grid;
    grid-template-columns: auto 1fr;
    overflow: hidden;
    height: 100%;
  }

  .rm-gallery-sidebar {
    overflow-y: auto;
    height: 100%;
    padding: clamp(1.5rem, 2.5vh, 4rem) clamp(1.2rem, 2vw, 3rem) clamp(6rem, 10vh, 10rem) clamp(1.5rem, 2.5vw, 4rem);
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: clamp(1.5rem, 2.5vh, 3rem);
  }

  .rm-gallery-nav {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: clamp(0.3rem, 0.5vh, 0.6rem);
  }

  .rm-gallery-nav-item {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: clamp(0.4rem, 0.8vw, 1rem);
    background: none;
    border: none;
    cursor: pointer;
    padding: clamp(0.3rem, 0.5vh, 0.6rem) clamp(0.6rem, 1vw, 1.2rem);
    opacity: 0.35;
    transition: opacity 140ms ease, transform 140ms ease;
  }

  .rm-gallery-nav-item::before {
    content: '';
    position: absolute;
    inset: 0;
    background: var(--rm-red, #E5191C);
    clip-path: polygon(4% 8%, 98% 0%, 96% 94%, 1% 100%);
    transform: skewX(-2deg) rotate(-0.5deg);
    opacity: 0;
    z-index: -1;
    transition: opacity 120ms ease, transform 120ms ease;
  }

  .rm-gallery-nav-item:hover {
    opacity: 0.7;
  }

  .rm-gallery-nav-item.is-active {
    opacity: 1;
  }

  .rm-gallery-nav-item.is-active::before {
    opacity: 1;
    transform: skewX(-3deg) rotate(-1deg);
  }

  .rm-gallery-nav-item :global(.p5m) {
    font-size: clamp(2.1rem, 5.25vw, 3.75rem);
  }

  .rm-gallery-nav-icon {
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(1.3rem, 2.2vw, 2.2rem);
    font-weight: 900;
    color: var(--rm-white);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: clamp(1.8rem, 3vw, 3rem);
    transform: rotate(-8deg) skewX(-5deg);
    line-height: 1;
  }

  .rm-gallery-content {
    overflow-y: auto;
    height: 100%;
    padding: clamp(6rem, 10vh, 12rem) clamp(2rem, 3vw, 5rem) clamp(7rem, 12vh, 12rem) clamp(1.5rem, 2.5vw, 4rem);
    box-sizing: border-box;
  }

  .rm-gallery-wall {
    display: flex;
    flex-wrap: wrap;
    gap: clamp(0.8rem, 1vw, 1.6rem);
  }

  .rm-gallery-card {
    display: block;
    width: calc((100% - 4 * clamp(0.8rem, 1vw, 1.6rem)) / 5);
    border: none;
    background: var(--rm-white);
    cursor: pointer;
    padding: clamp(0.3rem, 0.4vw, 0.55rem);
    padding-bottom: clamp(0.4rem, 0.5vw, 0.7rem);
    box-sizing: border-box;
    transition: transform 120ms ease, box-shadow 120ms ease;
    position: relative;
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.35),
      0 4px 8px rgba(0, 0, 0, 0.2);
  }

  .rm-gallery-card:hover {
    z-index: 2;
    transform: rotate(0deg) scale(1.04) !important;
    box-shadow:
      0 2px 6px rgba(0, 0, 0, 0.4),
      0 8px 20px rgba(0, 0, 0, 0.3);
  }

  .rm-gallery-card-frame {
    position: relative;
    width: 100%;
    overflow: hidden;
    background: var(--rm-black);
  }

  .rm-gallery-card-img {
    display: block;
    width: 100%;
    height: auto;
    object-fit: cover;
  }

  .rm-gallery-card-fallback {
    width: 100%;
    aspect-ratio: 3 / 4;
    background: var(--rm-black);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .rm-gallery-card-info {
    padding: clamp(0.25rem, 0.35vw, 0.5rem) clamp(0.1rem, 0.15vw, 0.2rem) 0;
    display: flex;
    flex-direction: column;
    gap: clamp(0.08rem, 0.1vw, 0.15rem);
  }

  .rm-gallery-card-name {
    font-size: clamp(0.48rem, 0.45vw, 0.75rem);
    font-weight: 800;
    color: var(--rm-black);
    letter-spacing: 0.02em;
    text-align: left;
    line-height: 1.2;
  }

  .rm-gallery-card-stars {
    display: flex;
    gap: 0;
    line-height: 1;
  }

  .rm-gallery-star {
    font-size: clamp(0.38rem, 0.36vw, 0.6rem);
    line-height: 1;
  }

  .rm-gallery-star--full {
    color: var(--rm-red);
  }

  .rm-gallery-star--half {
    color: var(--rm-red);
    opacity: 0.4;
  }

  .rm-gallery-star--empty {
    color: rgba(0, 0, 0, 0.15);
  }

  .rm-gallery-card-stars.is-community .rm-gallery-star--full,
  .rm-gallery-card-stars.is-community .rm-gallery-star--half {
    color: var(--rm-black);
    opacity: 0.35;
  }

  /* ── Game card meta ── */

  .rm-gallery-card-game-meta {
    display: flex;
    flex-direction: column;
    gap: clamp(0.06rem, 0.08vw, 0.12rem);
  }

  .rm-gallery-card-playtime {
    font-size: clamp(0.42rem, 0.4vw, 0.65rem);
    font-weight: 900;
    color: var(--rm-black);
    letter-spacing: 0.06em;
    line-height: 1;
  }

  .rm-gallery-card-ach {
    display: flex;
    align-items: center;
    gap: clamp(0.15rem, 0.2vw, 0.3rem);
  }

  .rm-gallery-card-ach-bar {
    flex: 1;
    height: clamp(2px, 0.2vw, 4px);
    background: var(--rm-black);
    position: relative;
    clip-path: polygon(2% 0%, 100% 0%, 98% 100%, 0% 100%);
  }

  .rm-gallery-card-ach-fill {
    position: absolute;
    top: 0;
    left: 0;
    height: 100%;
    background: var(--rm-red);
  }

  .rm-gallery-card-ach-text {
    font-size: clamp(0.32rem, 0.3vw, 0.5rem);
    font-weight: 700;
    color: rgba(0, 0, 0, 0.45);
    letter-spacing: 0.04em;
    line-height: 1;
    white-space: nowrap;
  }

  /* ── Gallery detail ── */

  .rm-gallery-detail {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: clamp(2rem, 4vh, 6rem) clamp(2rem, 4vw, 6rem);
    box-sizing: border-box;
    overflow-y: auto;
    height: 100%;
  }

  .rm-gallery-detail-inner {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: clamp(1.5rem, 2.5vw, 4rem);
    max-width: 70%;
    width: 100%;
  }

  .rm-gallery-detail-cover {
    width: clamp(330px, 33vw, 780px);
    flex-shrink: 0;
  }

  .rm-gallery-detail-img {
    display: block;
    width: 100%;
    height: auto;
    clip-path: polygon(3% 0%, 100% 2%, 97% 100%, 0% 97%);
  }

  .rm-gallery-detail-placeholder {
    aspect-ratio: 3 / 4;
  }

  .rm-gallery-detail-info {
    display: flex;
    flex-direction: column;
    gap: clamp(0.75rem, 1.2vw, 1.8rem);
  }

  .rm-gallery-detail-title {
    margin: 0;
    font-size: clamp(2.1rem, 2.7vw, 4.5rem);
    font-weight: 900;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    line-height: 1.1;
  }

  .rm-gallery-detail-original {
    margin: 0;
    font-size: clamp(0.975rem, 0.9vw, 1.5rem);
    color: rgba(255, 255, 255, 0.45);
    font-weight: 600;
    letter-spacing: 0.03em;
  }

  .rm-gallery-detail-meta {
    display: flex;
    flex-direction: column;
    gap: clamp(0.225rem, 0.3vw, 0.45rem);
  }

  .rm-gallery-detail-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: clamp(0.225rem, 0.3vw, 0.525rem) 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .rm-gallery-detail-label {
    font-size: clamp(0.825rem, 0.75vw, 1.275rem);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: rgba(255, 255, 255, 0.5);
  }

  .rm-gallery-detail-value {
    font-size: clamp(0.975rem, 0.9vw, 1.5rem);
    font-weight: 800;
    letter-spacing: 0.04em;
  }

  .rm-gallery-detail-myrating {
    color: var(--rm-red);
  }

  .rm-gallery-detail-tags {
    display: flex;
    flex-wrap: wrap;
    gap: clamp(0.3rem, 0.4vw, 0.6rem);
    margin-top: clamp(0.3rem, 0.4vw, 0.6rem);
  }

  .rm-gallery-detail-tag {
    font-size: clamp(0.75rem, 0.69vw, 1.125rem);
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--rm-white);
    background: rgba(229, 25, 28, 0.2);
    clip-path: polygon(4% 0%, 100% 0%, 96% 100%, 0% 100%);
    padding: clamp(0.15rem, 0.2vw, 0.3rem) clamp(0.5rem, 0.6vw, 1rem);
  }

  /* ── Game detail achievement bar ── */

  .rm-gallery-detail-ach-bar {
    width: 100%;
    height: clamp(6px, 0.5vw, 10px);
    background: var(--rm-black);
    position: relative;
    clip-path: polygon(1% 0%, 100% 0%, 99% 100%, 0% 100%);
    margin-top: clamp(0.15rem, 0.2vw, 0.3rem);
  }

  .rm-gallery-detail-ach-fill {
    position: absolute;
    top: 0;
    left: 0;
    height: 100%;
    background: var(--rm-red);
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
