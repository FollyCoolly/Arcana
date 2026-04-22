<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import KeyHint from "$lib/KeyHint.svelte";
    import PromptWord from "$lib/PromptWord.svelte";
    import MenuItem from "$lib/MenuItem.svelte";
    import type { LetterConfig } from "$lib/MenuItem.svelte";
    import type { ItemData, ItemWithComputed } from "$lib/types/item";

    type SortKey = "name" | "days_owned" | "price" | "daily_cost";
    type SortDir = "asc" | "desc";

    let { onBack }: { onBack: () => void } = $props();

    let itemLoading = $state(false);
    let itemError = $state<string | null>(null);
    let itemData = $state<ItemData | null>(null);
    let selectedItem = $state<ItemWithComputed | null>(null);
    let itemFilterCategory = $state<string | null>(null);
    let selectedIndex = $state(0);
    let rowRefs = $state<(HTMLElement | undefined)[]>([]);
    let listRef = $state<HTMLElement | undefined>(undefined);
    let catNavRef = $state<HTMLElement | undefined>(undefined);
    let catBtnRefs = $state<(HTMLButtonElement | undefined)[]>([]);
    let scrollRatio = $state(0);
    let thumbRatio = $state(1);
    let sortKey = $state<SortKey>("name");
    let sortDir = $state<SortDir>("asc");

    function toggleSort(key: SortKey) {
        if (sortKey === key) {
            sortDir = sortDir === "asc" ? "desc" : "asc";
        } else {
            sortKey = key;
            sortDir = "asc";
        }
        selectedIndex = 0;
        selectedItem = null;
    }

    function getSortIndicator(key: SortKey): string {
        if (sortKey !== key) return "";
        return sortDir === "asc" ? " ▲" : " ▼";
    }

    function getPillContent(
        item: ItemWithComputed,
    ): { main: string; unit: string } | null {
        if (sortKey === "name") return null;
        if (sortKey === "days_owned") {
            if (item.days_owned === null || item.days_owned === undefined)
                return { main: "D", unit: "—" };
            return { main: "D", unit: String(item.days_owned) };
        }
        if (sortKey === "price") {
            if (item.price === null || item.price === undefined)
                return { main: "￥", unit: "—" };
            return { main: "￥", unit: formatMoney(item.price) };
        }
        if (sortKey === "daily_cost") {
            if (item.daily_cost === null || item.daily_cost === undefined)
                return { main: "￥", unit: "—" };
            return { main: "￥", unit: formatMoney(item.daily_cost) };
        }
        return null;
    }

    function formatMoney(n: number): string {
        if (n >= 100) return Math.round(n).toLocaleString("en-US");
        if (n >= 10) return n.toFixed(1);
        return n.toFixed(2);
    }

    /* ── Category label mapping (Chinese → English for MenuItem display) ── */
    const CATEGORY_LABELS: Record<string, string> = {
        衣物: "WEAR",
        鞋子: "SHOES",
        配饰: "GEAR",
        电子产品: "TECH",
        生活电器: "HOME",
        手办: "FIGS",
        家具: "DECO",
        实体书: "BOOKS",
        专辑: "DISC",
    };

    function getCategoryLabel(name: string): string {
        return CATEGORY_LABELS[name] ?? name;
    }

    /* ── Category sidebar: letter configs + quad effect ── */
    const QUAD_CONFIGS: { rot: number; clip: string }[] = [
        { rot: -8, clip: "polygon(3% 5%, 97% 0%, 95% 95%, 1% 100%)" },
        { rot: -4, clip: "polygon(1% 8%, 99% 2%, 97% 92%, 3% 98%)" },
        { rot: -1, clip: "polygon(2% 0%, 98% 6%, 96% 96%, 0% 88%)" },
        { rot: 1, clip: "polygon(0% 6%, 98% 0%, 100% 94%, 2% 100%)" },
        { rot: 3, clip: "polygon(1% 4%, 97% 0%, 100% 90%, 3% 96%)" },
        { rot: -2, clip: "polygon(0% 8%, 99% 0%, 100% 100%, 2% 92%)" },
    ];

    const letterCache = new Map<string, LetterConfig[]>();

    function getSourceLetterConfigs(
        name: string,
        index: number,
    ): LetterConfig[] {
        const key = `${index}:${name}`;
        if (letterCache.has(key)) return letterCache.get(key)!;
        const SIZES = [
            "0.75em",
            "0.82em",
            "0.88em",
            "0.92em",
            "1.0em",
            "1.08em",
            "1.15em",
        ];
        const OFFSETS = [-3, -2, -1, 0, 1, 2, 3, 4];
        const ROTATES = [-6, -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5];
        const letters: LetterConfig[] = name.split("").map((char, i) => {
            if (char === " ")
                return { char: " ", size: "0.5em", yOffset: 0, rotate: 0 };
            const seed = index * 37 + i * 13;
            const colorVariant = (seed * 3) % 5;
            return {
                char,
                size: SIZES[seed % SIZES.length],
                yOffset: OFFSETS[(seed * 7) % OFFSETS.length],
                rotate: ROTATES[(seed * 11) % ROTATES.length],
                weight: i === 0 ? 800 : 700,
                color: colorVariant === 0 ? ("black" as const) : undefined,
                outline: colorVariant === 0 && seed % 2 === 0,
                rounded: colorVariant === 0 && seed % 2 !== 0,
            };
        });
        letterCache.set(key, letters);
        return letters;
    }

    $effect(() => {
        if (!itemData || !catNavRef) return;
        void itemFilterCategory;
        const idx = itemData.stats.by_category.findIndex(
            (c) => c.name === itemFilterCategory,
        );
        if (idx < 0) return;
        const btn = catBtnRefs[idx];
        if (!btn) return;
        const btnRect = btn.getBoundingClientRect();
        const containerRect = catNavRef.getBoundingClientRect();
        const centerX = btnRect.left + btnRect.width / 2 - containerRect.left;
        const centerY = btnRect.top + btnRect.height / 2 - containerRect.top;
        const quadW = btn.offsetWidth * 1.5;
        const quadH = btn.offsetHeight * 1.3;
        const cfg = QUAD_CONFIGS[idx % QUAD_CONFIGS.length];
        catNavRef.style.setProperty("--cat-quad-x", `${centerX - quadW / 2}px`);
        catNavRef.style.setProperty("--cat-quad-y", `${centerY - quadH / 2}px`);
        catNavRef.style.setProperty("--cat-quad-w", `${quadW}px`);
        catNavRef.style.setProperty("--cat-quad-h", `${quadH}px`);
        catNavRef.style.setProperty("--cat-quad-rot", `${cfg.rot}deg`);
        catNavRef.style.setProperty("--cat-quad-clip", cfg.clip);
    });

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
            const hideThreshold = t < 0 ? 0.7 : 0.45;
            if (abs > hideThreshold) {
                row.style.visibility = "hidden";
                row.style.pointerEvents = "none";
            } else {
                row.style.visibility = "visible";
                row.style.pointerEvents = "";
            }
        }
    }

    function updateScrollIndicator() {
        if (!listRef) return;
        const max = listRef.scrollHeight - listRef.clientHeight;
        scrollRatio = max > 0 ? listRef.scrollTop / max : 0;
        thumbRatio =
            listRef.scrollHeight > 0
                ? Math.min(1, listRef.clientHeight / listRef.scrollHeight)
                : 1;
    }

    function onListScroll() {
        cancelAnimationFrame(rafId);
        rafId = requestAnimationFrame(() => {
            updateFanEffect();
            updateScrollIndicator();
        });
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
        void itemData;
        void selectedItem;
        void sortKey;
        void sortDir;
        if (listRef) {
            // Run synchronously so newly-mounted rows don't flash as "visible"
            // before the next paint, then again after layout settles.
            updateFanEffect();
            updateScrollIndicator();
            requestAnimationFrame(() =>
                requestAnimationFrame(() => {
                    updateFanEffect();
                    updateScrollIndicator();
                }),
            );
        }
    });

    function getFilteredSortedItems(): ItemWithComputed[] {
        if (!itemData) return [];
        const items = itemFilterCategory
            ? itemData.items.filter((i) => i.category === itemFilterCategory)
            : itemData.items;
        const dir = sortDir === "asc" ? 1 : -1;
        const nullsLast = (v: number | null) =>
            v === null || v === undefined ? Number.POSITIVE_INFINITY : v;
        return [...items].sort((a, b) => {
            if (sortKey === "name") {
                return dir * a.name.localeCompare(b.name, "zh-CN");
            }
            if (sortKey === "days_owned") {
                return (
                    dir * (nullsLast(a.days_owned) - nullsLast(b.days_owned))
                );
            }
            if (sortKey === "price") {
                return dir * (nullsLast(a.price) - nullsLast(b.price));
            }
            if (sortKey === "daily_cost") {
                return (
                    dir * (nullsLast(a.daily_cost) - nullsLast(b.daily_cost))
                );
            }
            return 0;
        });
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
            onBack();
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
            if (itemData.stats.by_category.length > 0) {
                itemFilterCategory = itemData.stats.by_category[0].name;
            }
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
        <div class="rm-items-layout">
            <svg
                class="rm-items-divider"
                viewBox="-700 0 800 1000"
                preserveAspectRatio="xMaxYMid meet"
                aria-hidden="true"
            >
                <path
                    d="M -685.48 -200 A 1993 1993 0 0 1 -685.48 1200"
                    fill="none"
                    stroke="var(--rm-black)"
                    stroke-width="48"
                />
                <path
                    d="M -634.27 -200 A 2041 2041 0 0 1 -634.27 1200"
                    fill="none"
                    stroke="var(--rm-gray)"
                    stroke-width="48"
                />
                <path
                    d="M -583.24 -200 A 2089 2089 0 0 1 -583.24 1200"
                    fill="none"
                    stroke="var(--rm-black)"
                    stroke-width="48"
                />
                <path
                    d="M -532.37 -200 A 2137 2137 0 0 1 -532.37 1200"
                    fill="none"
                    stroke="var(--rm-gray)"
                    stroke-width="48"
                />
                <path
                    d="M -481.63 -200 A 2185 2185 0 0 1 -481.63 1200"
                    fill="none"
                    stroke="var(--rm-black)"
                    stroke-width="48"
                />
                <path
                    d="M -431.02 -200 A 2233 2233 0 0 1 -431.02 1200"
                    fill="none"
                    stroke="var(--rm-gray)"
                    stroke-width="48"
                />
                <path
                    d="M -380.53 -200 A 2281 2281 0 0 1 -380.53 1200"
                    fill="none"
                    stroke="var(--rm-black)"
                    stroke-width="48"
                />
                <path
                    d="M -330.16 -200 A 2329 2329 0 0 1 -330.16 1200"
                    fill="none"
                    stroke="var(--rm-gray)"
                    stroke-width="48"
                />
                <path
                    d="M -279.88 -200 A 2377 2377 0 0 1 -279.88 1200"
                    fill="none"
                    stroke="var(--rm-black)"
                    stroke-width="48"
                />
                <path
                    d="M -229.7 -200 A 2425 2425 0 0 1 -229.7 1200"
                    fill="none"
                    stroke="var(--rm-gray)"
                    stroke-width="48"
                />
                <path
                    d="M -179.6 -200 A 2473 2473 0 0 1 -179.6 1200"
                    fill="none"
                    stroke="var(--rm-black)"
                    stroke-width="48"
                />
                <path
                    d="M -129.6 -200 A 2521 2521 0 0 1 -129.6 1200"
                    fill="none"
                    stroke="var(--rm-gray)"
                    stroke-width="48"
                />
                <path
                    d="M -79.68 -200 A 2569 2569 0 0 1 -79.68 1200"
                    fill="none"
                    stroke="var(--rm-black)"
                    stroke-width="48"
                />
                <path
                    d="M 0 0 A 2596 2596 0 0 1 0 1000"
                    fill="none"
                    stroke="var(--rm-white)"
                    stroke-width="6"
                />
            </svg>
            <!-- LEFT: Category nav + stats -->
            <div class="rm-items-sidebar">
                <nav class="rm-items-cat-nav" bind:this={catNavRef}>
                    <ul class="rm-items-cat-list">
                        {#each itemData.stats.by_category as cat, i}
                            <li
                                class="rm-items-cat-line"
                                style:z-index={itemFilterCategory === cat.name
                                    ? 10
                                    : 0}
                            >
                                <button
                                    type="button"
                                    class="rm-items-cat-btn"
                                    class:is-active={itemFilterCategory ===
                                        cat.name}
                                    bind:this={catBtnRefs[i]}
                                    onclick={() => {
                                        itemFilterCategory = cat.name;
                                        selectedItem = null;
                                    }}
                                    onmouseenter={() => {
                                        itemFilterCategory = cat.name;
                                        selectedItem = null;
                                    }}
                                >
                                    <MenuItem
                                        letters={getSourceLetterConfigs(
                                            getCategoryLabel(cat.name),
                                            i,
                                        )}
                                        active={itemFilterCategory === cat.name}
                                    />
                                </button>
                            </li>
                        {/each}
                    </ul>
                    <div class="rm-items-cat-quad" aria-hidden="true"></div>
                </nav>

                <button type="button" class="rm-back-btn" onclick={onBack}>
                    <KeyHint key="Esc" fontSize={36} />
                    <PromptWord text="Back" fontSize={72} />
                </button>
            </div>

            <!-- Tangent scroll indicator (read-only, decorative) -->
            <div
                class="rm-items-scroll-indicator"
                aria-hidden="true"
                style="--thumb-ratio: {thumbRatio}; --scroll-ratio: {scrollRatio};"
            >
                <div class="rm-items-scroll-track">
                    <div class="rm-items-scroll-thumb"></div>
                </div>
            </div>

            <!-- RIGHT: Sort + list + summary -->
            <div class="rm-items-content">
                <div class="rm-items-sort-bar">
                    <button
                        type="button"
                        class="rm-ach-tab"
                        class:active={sortKey === "name"}
                        onclick={() => toggleSort("name")}
                    >
                        Name{getSortIndicator("name")}
                    </button>
                    <button
                        type="button"
                        class="rm-ach-tab"
                        class:active={sortKey === "days_owned"}
                        onclick={() => toggleSort("days_owned")}
                    >
                        Owned{getSortIndicator("days_owned")}
                    </button>
                    <button
                        type="button"
                        class="rm-ach-tab"
                        class:active={sortKey === "price"}
                        onclick={() => toggleSort("price")}
                    >
                        Price{getSortIndicator("price")}
                    </button>
                    <button
                        type="button"
                        class="rm-ach-tab"
                        class:active={sortKey === "daily_cost"}
                        onclick={() => toggleSort("daily_cost")}
                    >
                        Daily{getSortIndicator("daily_cost")}
                    </button>
                </div>
                <div class="rm-items-list" bind:this={listRef}>
                    {#each getFilteredSortedItems() as item, i}
                        {@const pill = getPillContent(item)}
                        <button
                            type="button"
                            class="rm-item-row"
                            class:is-selected={selectedItem?.id === item.id}
                            bind:this={rowRefs[i]}
                            style="width: {getItemWidthPercent(
                                item,
                            )}%; --row-clip: {getItemClipPath(item)};"
                            onclick={() => {
                                selectedIndex = i;
                                selectedItem = item;
                            }}
                            onmouseenter={() => {
                                selectedIndex = i;
                                selectedItem = item;
                            }}
                        >
                            {#if selectedItem?.id === item.id}
                                <span
                                    class="rm-item-selection-tri"
                                    aria-hidden="true"
                                ></span>
                            {/if}
                            <span class="rm-item-row-name">{item.name}</span>
                            {#if pill}
                                <span class="rm-item-pill" aria-hidden="true">
                                    <span class="rm-item-pill-main"
                                        >{pill.main}</span
                                    >
                                    <span class="rm-item-pill-unit"
                                        >{pill.unit}</span
                                    >
                                </span>
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
        grid-template-columns: clamp(26rem, 42vw, 46rem) 1fr;
        overflow: hidden;
        height: 100vh;
        margin: 0;
        position: relative;
    }

    .rm-items-divider {
        position: absolute;
        top: 0;
        left: clamp(26rem, 42vw, 46rem);
        height: 100vh;
        /* width derived from viewBox aspect ratio (800:1000 = 4:5) */
        width: 80vh;
        /* align viewBox x=0 (white arc's rightmost boundary) to the grid column border.
           viewBox x=0 is at 700/800 = 87.5% of SVG box width. */
        transform: translateX(-87.5%);
        z-index: 0;
        pointer-events: none;
        overflow: visible;
    }

    /* ── Left sidebar: category nav + stats ── */
    .rm-items-sidebar {
        display: flex;
        flex-direction: column;
        height: 100%;
        padding: clamp(1.5rem, 2.5vh, 3rem) clamp(1.5rem, 2vw, 3rem)
            clamp(1rem, 1.5vh, 2rem);
        box-sizing: border-box;
        overflow-y: auto;
        position: relative;
        z-index: 1;
    }

    .rm-items-cat-nav {
        position: relative;
        isolation: isolate;
        overflow: hidden;
        margin-bottom: auto;
        padding: 3.2rem 0 3.2rem 0.8rem;
    }

    .rm-items-cat-list {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
    }

    .rm-items-cat-line {
        margin: -0.64rem 0;
        position: relative;
    }

    .rm-items-cat-line:nth-child(odd) {
        margin-left: 0;
    }

    .rm-items-cat-line:nth-child(even) {
        margin-left: 3.2vw;
    }

    .rm-items-cat-btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        border: none;
        background: var(--rm-black);
        cursor: pointer;
        padding: 1.28rem 3.2rem 1.28rem 2.56rem;
        width: fit-content;
        transition: background-color 140ms ease;
    }

    .rm-items-cat-btn:not(.is-active):hover {
        background: var(--rm-red);
    }

    .rm-items-cat-btn.is-active {
        background: var(--rm-red);
        z-index: 1;
    }

    .rm-items-cat-btn :global(.p5m) {
        font-size: clamp(3.2rem, 5.12vw, 4.8rem);
    }

    /* Per-item rotation + clip-path via <li> wrapper */
    .rm-items-cat-line:nth-child(6n + 1) .rm-items-cat-btn {
        transform: rotate(-5deg);
        clip-path: polygon(0% 8%, 100% 0%, 98% 92%, 2% 100%);
    }
    .rm-items-cat-line:nth-child(6n + 2) .rm-items-cat-btn {
        transform: rotate(-3deg);
        clip-path: polygon(1% 5%, 99% 0%, 97% 96%, 0% 100%);
    }
    .rm-items-cat-line:nth-child(6n + 3) .rm-items-cat-btn {
        transform: rotate(-1deg);
        clip-path: polygon(2% 0%, 100% 4%, 96% 100%, 0% 92%);
    }
    .rm-items-cat-line:nth-child(6n + 4) .rm-items-cat-btn {
        transform: rotate(1deg);
        clip-path: polygon(0% 6%, 98% 0%, 100% 94%, 3% 100%);
    }
    .rm-items-cat-line:nth-child(6n + 5) .rm-items-cat-btn {
        transform: rotate(2deg);
        clip-path: polygon(1% 0%, 97% 4%, 99% 100%, 2% 96%);
    }
    .rm-items-cat-line:nth-child(6n + 6) .rm-items-cat-btn {
        transform: rotate(-2deg);
        clip-path: polygon(0% 4%, 100% 0%, 98% 96%, 1% 100%);
    }

    .rm-items-cat-quad {
        position: absolute;
        left: var(--cat-quad-x, -9999px);
        top: var(--cat-quad-y, -9999px);
        width: var(--cat-quad-w, 0);
        height: var(--cat-quad-h, 0);
        transform: rotate(var(--cat-quad-rot, 0deg));
        z-index: 15;
        background: var(--rm-red);
        mix-blend-mode: difference;
        clip-path: var(
            --cat-quad-clip,
            polygon(0% 0%, 100% 0%, 100% 100%, 0% 100%)
        );
        pointer-events: none;
        transition:
            left 120ms ease,
            top 120ms ease,
            width 120ms ease,
            height 120ms ease,
            transform 120ms ease,
            clip-path 120ms ease;
    }

    /* ── Tangent scroll indicator (read-only) ──
       Tangent to the white arc at viewBox (17.58, 900).
       In screen coords: 1.758vh right of divider line, at 90vh from top.
       Tangent angle from vertical: atan(400 / 2569) ≈ 8.85° to the right-up. */
    .rm-items-scroll-indicator {
        position: absolute;
        top: 90vh;
        left: clamp(26rem, 42vw, 46rem);
        /* 1.758vh (tangent-point x offset) + 0.6vh (white arc stroke width).
           translate Y by -100% so the bar's bottom edge sits at top:90vh. */
        transform: translate(2.358vh, -100%) rotate(8.85deg);
        transform-origin: left bottom;
        z-index: 10;
        pointer-events: none;
        /* 40vh tall, 28px wide (2× achievements scrollbar size) */
        height: 40vh;
        width: 28px;
    }

    .rm-items-scroll-track {
        position: relative;
        width: 100%;
        height: 100%;
        background: var(--rm-black);
        border: 4px solid var(--rm-white);
        box-sizing: border-box;
    }

    .rm-items-scroll-thumb {
        position: absolute;
        left: 0;
        right: 0;
        /* Height: proportion of viewport/content */
        height: calc(var(--thumb-ratio, 1) * 100%);
        /* Top: scroll ratio × (track height - thumb height) */
        top: calc(
            var(--scroll-ratio, 0) * (100% - var(--thumb-ratio, 1) * 100%)
        );
        background: var(--rm-white);
    }

    /* ── Right content: list ── */
    .rm-items-content {
        display: flex;
        flex-direction: column;
        height: 100%;
        padding: clamp(0.5rem, 1vh, 1rem) clamp(1rem, 2vw, 3rem)
            clamp(0.5rem, 1vh, 1rem) 0;
        box-sizing: border-box;
        overflow: hidden;
    }

    /* ── Sort bar (above the list, aligned to list's visual x-start) ── */
    .rm-items-sort-bar {
        display: flex;
        align-items: center;
        gap: clamp(0.3rem, 0.5vw, 0.8rem);
        flex-wrap: wrap;
        padding: clamp(1rem, 2vh, 2rem) 0 clamp(0.5rem, 1vh, 1rem) 11vw;
        position: relative;
        z-index: 2;
    }

    .rm-ach-tab {
        position: relative;
        z-index: 0;
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(1rem, 1.1vw, 1.6rem);
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        padding: clamp(0.5rem, 0.6vw, 0.9rem) clamp(1rem, 1.2vw, 1.8rem);
        border: none;
        background: var(--rm-white);
        color: var(--rm-white);
        cursor: pointer;
        clip-path: polygon(0% 0%, 100% 0%, 96% 100%, 4% 100%);
        transition: all 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
        white-space: nowrap;
    }

    .rm-ach-tab::before {
        content: "";
        position: absolute;
        inset: 4px;
        background: var(--rm-black);
        clip-path: polygon(0% 0%, 100% 0%, 96% 100%, 4% 100%);
        z-index: -1;
        transition: background 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    .rm-ach-tab:hover {
        transform: scale(1.06);
    }

    .rm-ach-tab.active {
        background: var(--rm-white);
        color: var(--rm-black);
    }

    .rm-ach-tab.active::before {
        background: var(--rm-white);
    }

    /* ── Item list: radial fan from left ── */
    .rm-items-list {
        flex: 1;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        /* Generous vertical padding so items can scroll through the center zone */
        padding: 40vh 0 40vh 0;
        padding-left: 8vw;
        transform: translate(3vw, -8vh) rotate(8.85deg);
        transform-origin: 0% 0%;
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
        transform-origin: 0% 50%;
        will-change: transform;
        background: transparent;
        /* Row itself is NOT clipped, so children (triangle, text) can
           extend / stack freely. Background with clip-path lives on ::before. */
    }

    /* Row background: a clipped polygon under everything else. */
    .rm-item-row::before {
        content: "";
        position: absolute;
        inset: 0;
        background: var(--rm-black);
        clip-path: var(--row-clip);
        transition: background 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
        z-index: 0;
    }

    .rm-item-row:hover::before {
        background: rgba(255, 255, 255, 0.12);
    }

    .rm-item-row.is-selected::before,
    .rm-item-row.is-selected:hover::before {
        background: var(--rm-black);
    }

    /* Selection triangle — sits above the row background, below the text.
       Short slanted edge on the left (1.3× row height), tip 20% past right. */
    .rm-item-selection-tri {
        position: absolute;
        top: -15%;
        bottom: -15%;
        left: -8%;
        right: -20%;
        background: var(--rm-red);
        clip-path: polygon(8% 0%, 100% 70%, 0% 100%);
        pointer-events: none;
        z-index: 1;
    }

    .rm-item-row-name {
        font-family: "方正兰亭黑_GBK", inherit;
        font-size: clamp(1.3rem, 1.5vw, 2.2rem);
        letter-spacing: 0.02em;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        flex: 1;
        min-width: 0;
        position: relative;
        z-index: 2;
    }

    /* ── Sort-value pill (inside right side of row quadrilateral) ── */
    .rm-item-pill {
        position: absolute;
        right: clamp(1.5rem, 2vw, 3rem);
        top: 50%;
        transform: translateY(-50%);
        display: flex;
        align-items: center;
        justify-content: space-between;
        width: clamp(7.5rem, 9.5vw, 11.5rem);
        height: clamp(3.2rem, 4vw, 4.8rem);
        padding: 0 clamp(1.2rem, 1.4vw, 2rem);
        background: var(--rm-white);
        color: var(--rm-black);
        border-radius: 9999px;
        font-family:
            "方正兰亭黑_GBK", "Inter", "SF Pro Display", "Helvetica Neue",
            Arial, sans-serif;
        font-size: clamp(1.4rem, 1.6vw, 2rem);
        font-weight: 700;
        font-variant-numeric: tabular-nums;
        letter-spacing: 0.01em;
        white-space: nowrap;
        pointer-events: none;
        z-index: 3;
    }

    .rm-item-pill-main {
        text-align: left;
    }

    .rm-item-pill-unit {
        text-align: right;
        font-weight: 600;
    }

    /* ── Gallery detail (reused for item detail view) ── */

    /* ── Gallery detail (reused for item detail view) ── */
</style>
