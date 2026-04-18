<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import KeyHint from "$lib/KeyHint.svelte";
    import PromptWord from "$lib/PromptWord.svelte";
    import type {
        ItemData,
        ItemWithComputed,
        ItemSortKey,
        ItemSortOrder,
    } from "$lib/types/item";

    let { onBack }: { onBack: () => void } = $props();

    let itemLoading = $state(false);
    let itemError = $state<string | null>(null);
    let itemData = $state<ItemData | null>(null);
    let selectedItem = $state<ItemWithComputed | null>(null);
    let itemFilterSource = $state<string | null>(null);
    let itemFilterCategory = $state<string | null>(null);
    let itemSortKey = $state<ItemSortKey>("name");
    let itemSortOrder = $state<ItemSortOrder>("asc");
    let selectedIndex = $state(0);
    let rowRefs = $state<(HTMLElement | undefined)[]>([]);
    let listRef = $state<HTMLElement | undefined>(undefined);

    /* ── Deterministic hash for per-item visual variation ── */
    function itemHash(s: string): number {
        let h = 0;
        for (let i = 0; i < s.length; i++)
            h = ((h << 5) - h + s.charCodeAt(i)) | 0;
        return ((h >>> 0) % 10000) / 10000;
    }

    function getItemWidthPercent(item: ItemWithComputed): number {
        const nameLen = Math.min(item.name.length, 20);
        const base = 49 + (nameLen / 20) * 2;
        const jitter = (itemHash(item.id) - 0.5) * 2;
        return Math.max(48, Math.min(52, base + jitter));
    }

    function getItemClipPath(item: ItemWithComputed): string {
        const h1 = itemHash(item.id + "_rt");
        const h2 = itemHash(item.id + "_rb");
        const h3 = itemHash(item.id + "_rs");
        const h4 = itemHash(item.id + "_rx");
        // Trapezoid: narrow left edge, wide right edge (radiating from left)
        const leftInset = 10 + h3 * 4; // 10-14% from top/bottom
        // Right edge Y: per-item variation
        const rtY = h1 * 10; // 0% – 10%
        const rbY = 90 + h2 * 10; // 90% – 100%
        // Right edge X: slanted (one corner pulled inward 5-15%)
        const slant = 1 + h4 * 2; // 1% – 3% inset
        const topPulled = h4 > 0.5;
        const rtX = topPulled ? 100 - slant : 100;
        const rbX = topPulled ? 100 : 100 - slant;
        return `polygon(0% ${leftInset.toFixed(1)}%, ${rtX.toFixed(1)}% ${rtY.toFixed(1)}%, ${rbX.toFixed(1)}% ${rbY.toFixed(1)}%, 0% ${(100 - leftInset).toFixed(1)}%)`;
    }

    /* ── Scroll-driven radial fan effect ── */
    let rafId = 0;

    function updateFanEffect() {
        if (!listRef) return;
        const rect = listRef.getBoundingClientRect();
        const centerY = rect.top + rect.height / 2;
        const halfH = rect.height / 2;
        if (halfH === 0) return;

        for (const row of rowRefs) {
            if (!row) continue;
            const rr = row.getBoundingClientRect();
            const mid = rr.top + rr.height / 2;
            const t = Math.max(-1.2, Math.min(1.2, (mid - centerY) / halfH));
            const abs = Math.abs(t);

            // Scale: 1.0 at center → 0.8 at edges (mild perspective)
            const scale = 1.0 - abs * 0.2;
            // Fan rotation: radiate from left to right
            const rotate = t * 12;
            row.style.transform = `scale(${scale.toFixed(3)}) rotate(${rotate.toFixed(1)}deg)`;
            const hideThreshold = t < 0 ? 0.45 : 0.75;
            if (abs > hideThreshold) {
                row.style.visibility = "hidden";
                row.style.pointerEvents = "none";
            } else {
                row.style.visibility = "visible";
                row.style.pointerEvents = "";
            }
        }
    }

    function onListScroll() {
        cancelAnimationFrame(rafId);
        rafId = requestAnimationFrame(updateFanEffect);
    }

    // Wire up scroll listener
    $effect(() => {
        const el = listRef;
        if (!el) return;
        el.addEventListener("scroll", onListScroll, { passive: true });
        return () => {
            el.removeEventListener("scroll", onListScroll);
            cancelAnimationFrame(rafId);
        };
    });

    // Re-run fan on data/filter/sort changes
    $effect(() => {
        void itemFilterCategory;
        void itemFilterSource;
        void itemSortKey;
        void itemSortOrder;
        void itemData;
        void selectedItem;
        if (listRef) {
            requestAnimationFrame(() => requestAnimationFrame(updateFanEffect));
        }
    });

    const ITEM_SORT_OPTIONS: { key: ItemSortKey; label: string }[] = [
        { key: "name", label: "名称" },
        { key: "price", label: "价格" },
        { key: "daily_cost", label: "日均" },
        { key: "date", label: "购入" },
        { key: "days_owned", label: "天数" },
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
            itemSortOrder = itemSortOrder === "asc" ? "desc" : "asc";
        } else {
            itemSortKey = key;
            itemSortOrder = key === "name" ? "asc" : "desc";
        }
    }

    function getItemSortValue(item: ItemWithComputed): string {
        switch (itemSortKey) {
            case "price":
                return formatPrice(item.price);
            case "daily_cost":
                return formatDailyCost(item.daily_cost) + "/d";
            case "date":
                return item.purchase_date ?? "—";
            case "days_owned":
                return item.days_owned !== null ? item.days_owned + "天" : "—";
            default:
                return "";
        }
    }

    function getFilteredSortedItems(): ItemWithComputed[] {
        if (!itemData) return [];

        let items = itemData.items;

        if (itemFilterSource) {
            items = items.filter((i) => i.source_id === itemFilterSource);
        }
        if (itemFilterCategory) {
            items = items.filter(
                (i) => (i.main_category ?? "未分类") === itemFilterCategory,
            );
        }

        const sorted = [...items];
        const dir = itemSortOrder === "asc" ? 1 : -1;

        sorted.sort((a, b) => {
            switch (itemSortKey) {
                case "name":
                    return dir * a.name.localeCompare(b.name, "zh-CN");
                case "price":
                    return dir * ((a.price ?? 0) - (b.price ?? 0));
                case "daily_cost":
                    return (
                        dir *
                        ((a.daily_cost ?? Infinity) -
                            (b.daily_cost ?? Infinity))
                    );
                case "date":
                    return (
                        dir *
                        (a.purchase_date ?? "").localeCompare(
                            b.purchase_date ?? "",
                        )
                    );
                case "days_owned":
                    return dir * ((a.days_owned ?? 0) - (b.days_owned ?? 0));
                default:
                    return 0;
            }
        });

        return sorted;
    }

    function getFilteredItemStats(): {
        total: number;
        value: number;
        avgDaily: number;
    } {
        const items = getFilteredSortedItems();
        const total = items.length;
        const value = items.reduce((sum, i) => sum + (i.price ?? 0), 0);
        const dailyCosts = items
            .map((i) => i.daily_cost)
            .filter((c): c is number => c !== null);
        const avgDaily =
            dailyCosts.length > 0
                ? dailyCosts.reduce((a, b) => a + b, 0) / dailyCosts.length
                : 0;
        return { total, value, avgDaily };
    }

    function formatExtraValue(val: unknown): string {
        if (val === null || val === undefined) return "—";
        if (typeof val === "string") return val;
        if (typeof val === "number") return String(val);
        if (typeof val === "boolean") return val ? "是" : "否";
        if (Array.isArray(val)) return val.join(", ");
        return JSON.stringify(val);
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            event.preventDefault();
            if (selectedItem) {
                selectedItem = null;
            } else {
                onBack();
            }
            return;
        }
        {
            const items = getFilteredSortedItems();
            if (event.key === "ArrowDown") {
                event.preventDefault();
                if (items.length > 0) {
                    selectedIndex = Math.min(
                        selectedIndex + 1,
                        items.length - 1,
                    );
                    selectedItem = items[selectedIndex];
                    rowRefs[selectedIndex]?.scrollIntoView({
                        block: "nearest",
                        behavior: "smooth",
                    });
                }
                return;
            }
            if (event.key === "ArrowUp") {
                event.preventDefault();
                if (items.length > 0) {
                    selectedIndex = Math.max(selectedIndex - 1, 0);
                    selectedItem = items[selectedIndex];
                    rowRefs[selectedIndex]?.scrollIntoView({
                        block: "nearest",
                        behavior: "smooth",
                    });
                }
                return;
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
                typeof error === "string" ? error : "Failed to load item data.";
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

    {#if itemLoading}
        <p class="state-text" style="padding: 2rem;">Loading items...</p>
    {:else if itemError}
        <p class="state-text error" style="padding: 2rem;">{itemError}</p>
    {:else if itemData}
        {@const filteredStats = getFilteredItemStats()}
        <div class="rm-items-layout">
            <!-- LEFT: Category nav + stats -->
            <div class="rm-items-sidebar">
                <nav class="rm-items-cat-nav">
                    <button
                        type="button"
                        class="rm-items-cat-btn"
                        class:is-active={!itemFilterCategory}
                        onclick={() => {
                            itemFilterCategory = null;
                            selectedItem = null;
                        }}
                    >
                        <span class="rm-items-cat-label">ALL</span>
                        <span class="rm-items-cat-count"
                            >{itemData.stats.total_items}</span
                        >
                    </button>
                    {#each itemData.stats.by_main_category as cat, i}
                        <button
                            type="button"
                            class="rm-items-cat-btn"
                            class:is-active={itemFilterCategory === cat.name}
                            class:rm-items-cat-even={i % 2 === 0}
                            onclick={() => {
                                itemFilterCategory =
                                    itemFilterCategory === cat.name
                                        ? null
                                        : cat.name;
                                selectedItem = null;
                            }}
                        >
                            <span class="rm-items-cat-label">{cat.name}</span>
                            <span class="rm-items-cat-count"
                                >{cat.item_count}</span
                            >
                        </button>
                    {/each}
                </nav>

                <div class="rm-items-stat-block">
                    <div class="rm-items-stat-row">
                        <span class="rm-items-stat-label">TOTAL</span>
                        <span class="rm-items-stat-value"
                            >{filteredStats.total}</span
                        >
                    </div>
                    <div class="rm-items-stat-row">
                        <span class="rm-items-stat-label">VALUE</span>
                        <span class="rm-items-stat-value"
                            >{formatPrice(filteredStats.value)}</span
                        >
                    </div>
                    <div class="rm-items-stat-row">
                        <span class="rm-items-stat-label">AVG/DAY</span>
                        <span class="rm-items-stat-value rm-items-daily"
                            >{formatDailyCost(filteredStats.avgDaily)}</span
                        >
                    </div>
                </div>

                <button type="button" class="rm-back-btn" onclick={onBack}>
                    <KeyHint key="Esc" fontSize={36} />
                    <PromptWord text="Back" fontSize={72} />
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
                                <span class="rm-items-sort-arrow"
                                    >{itemSortOrder === "asc" ? "↑" : "↓"}</span
                                >
                            {/if}
                        </button>
                    {/each}
                    <span class="rm-items-result-count"
                        >{filteredStats.total}</span
                    >
                </div>

                <div class="rm-items-list" bind:this={listRef}>
                    {#each getFilteredSortedItems() as item, i}
                        {@const sortVal = getItemSortValue(item)}
                        <button
                            type="button"
                            class="rm-item-row"
                            class:is-selected={selectedItem?.id === item.id}
                            bind:this={rowRefs[i]}
                            style="width: {getItemWidthPercent(
                                item,
                            )}%; clip-path: {getItemClipPath(item)};"
                            onclick={() => {
                                selectedIndex = i;
                                selectedItem = item;
                            }}
                            onmouseenter={() => {
                                selectedIndex = i;
                                selectedItem = item;
                            }}
                        >
                            <span class="rm-item-row-name">{item.name}</span>
                            {#if sortVal}
                                <span class="rm-item-row-attr">{sortVal}</span>
                            {/if}
                        </button>
                    {/each}
                </div>
            </div>
        </div>
    {:else}
        <p class="state-text" style="padding: 2rem;">
            Item data is not available yet.
        </p>
    {/if}
</section>

<style>

    .rm-items-layout {
        display: grid;
        grid-template-columns: clamp(10rem, 20vw, 18rem) 1fr;
        overflow: hidden;
        height: 90vh;
        margin: auto 0;
    }

    /* ── Left sidebar: category nav + stats ── */
    .rm-items-sidebar {
        display: flex;
        flex-direction: column;
        height: 100%;
        padding: clamp(1rem, 1.5vh, 2rem) clamp(1rem, 1.5vw, 2.5rem)
            clamp(1rem, 1.5vh, 2rem);
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
        transition:
            opacity 140ms ease,
            background 140ms ease,
            color 140ms ease;
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

    /* ── Right content: sort + list + summary ── */
    .rm-items-content {
        display: flex;
        flex-direction: column;
        height: 100%;
        padding: clamp(0.5rem, 1vh, 1rem) clamp(1rem, 2vw, 3rem)
            clamp(0.5rem, 1vh, 1rem) 0;
        box-sizing: border-box;
        overflow: hidden;
    }

    .rm-items-sort-bar {
        display: flex;
        align-items: center;
        gap: clamp(0.3rem, 0.4vw, 0.6rem);
        margin-bottom: clamp(0.4rem, 0.5vw, 0.8rem);
        padding-left: clamp(1rem, 1.5vw, 2.5rem);
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
        transition:
            opacity 140ms ease,
            background 140ms ease;
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

    /* ── Item list: radial fan from left ── */
    .rm-items-list {
        flex: 1;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        /* Generous vertical padding so items can scroll through the center zone */
        padding: 25vh 0 40vh 0;
        padding-left: 15vw;
        transform: rotate(-8deg);
        transform-origin: 0% 50%;
        gap: 0;
        scrollbar-width: none;
    }

    .rm-items-list::-webkit-scrollbar {
        display: none;
    }

    .rm-item-row {
        display: flex;
        align-items: center;
        border: none;
        color: var(--rm-white);
        cursor: pointer;
        padding: clamp(2.4rem, 6vw, 5rem) clamp(1.2rem, 1.5vw, 2.5rem);
        margin-bottom: -2vw;
        font-family: inherit;
        font-size: clamp(1rem, 1vw, 1.6rem);
        font-weight: 800;
        text-align: left;
        flex-shrink: 0;
        position: relative;

        /* Fan radiates from left — pin transform origin to left edge */
        transform-origin: 0% 50%;
        will-change: transform;

        background: var(--rm-black);
        /* clip-path is set per-item via inline style */
        transition: background 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    .rm-item-row:hover {
        background: rgba(255, 255, 255, 0.12);
        z-index: 3;
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
        color: rgba(255, 255, 255, 0.45);
        white-space: nowrap;
        font-size: 0.85em;
        letter-spacing: 0.04em;
    }

    .rm-item-row.is-selected .rm-item-row-attr {
        color: rgba(255, 255, 255, 0.9);
    }

    /* ── Bottom summary bar ── */

    /* ── Gallery detail (reused for item detail view) ── */
</style>
