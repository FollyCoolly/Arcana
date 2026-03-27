<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import P5Text from "$lib/P5Text.svelte";
  import type { ItemData, ItemWithComputed, ItemSortKey, ItemSortOrder } from "$lib/types/item";

  let { onBack }: { onBack: () => void } = $props();

  let itemLoading = $state(false);
  let itemError = $state<string | null>(null);
  let itemData = $state<ItemData | null>(null);
  let selectedItem = $state<ItemWithComputed | null>(null);
  let itemDetailMode = $state(false);
  let itemFilterSource = $state<string | null>(null);
  let itemFilterCategory = $state<string | null>(null);
  let itemSortKey = $state<ItemSortKey>('name');
  let itemSortOrder = $state<ItemSortOrder>('asc');

  const ITEM_SORT_OPTIONS: { key: ItemSortKey; label: string }[] = [
    { key: 'name', label: '名称' },
    { key: 'price', label: '价格' },
    { key: 'daily_cost', label: '日均' },
    { key: 'date', label: '购入' },
    { key: 'days_owned', label: '天数' },
  ];

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

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      if (itemDetailMode) {
        itemDetailMode = false;
      } else if (selectedItem) {
        selectedItem = null;
      } else {
        onBack();
      }
    }
  }

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

  onMount(() => {
    if (!itemData && !itemLoading) {
      void loadItemData();
    }

    window.addEventListener("keydown", handleKeydown);
    return () => {
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

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

        <button type="button" class="rm-items-back-btn" onclick={onBack}>
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

<style>
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

  /* ── Gallery detail (reused for item detail view) ── */
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
</style>
