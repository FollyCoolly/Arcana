<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import MenuItem from "$lib/MenuItem.svelte";
    import type { LetterConfig } from "$lib/MenuItem.svelte";
    import type {
        GalleryData,
        MediaItem,
        GallerySortKey,
        GallerySortOrder,
        GalleryCategory,
    } from "$lib/types/gallery";
    import KeyHint from "$lib/KeyHint.svelte";
    import PromptWord from "$lib/PromptWord.svelte";

    let { onBack }: { onBack: () => void } = $props();

    let galleryLoading = $state(false);
    let galleryError = $state<string | null>(null);
    let galleryData = $state<GalleryData | null>(null);
    let selectedMedia = $state<MediaItem | null>(null);

    // Sidebar refs for selection quad
    let sidebarRef = $state<HTMLElement | undefined>(undefined);
    let catBtnRefs = $state<(HTMLButtonElement | undefined)[]>([]);

    let gallerySortKey = $state<GallerySortKey>("rating");
    let gallerySortOrder = $state<GallerySortOrder>("desc");
    let galleryActiveCategory = $state<GalleryCategory>("anime");

    const GALLERY_CATEGORIES: {
        id: GalleryCategory;
        label: string;
        icon: string;
        mediaTypes: string[];
        sortOptions: { key: GallerySortKey; label: string }[];
    }[] = [
        {
            id: "anime",
            label: "Anime",
            icon: "A",
            mediaTypes: ["anime"],
            sortOptions: [
                { key: "rating", label: "Rating" },
                { key: "date", label: "Consume" },
            ],
        },
        {
            id: "game",
            label: "Games",
            icon: "G",
            mediaTypes: ["game"],
            sortOptions: [
                { key: "playtime", label: "Playtime" },
                { key: "rating", label: "Rating" },
            ],
        },
        {
            id: "tv",
            label: "TV",
            icon: "T",
            mediaTypes: ["tv"],
            sortOptions: [
                { key: "rating", label: "Rating" },
                { key: "date", label: "Consume" },
            ],
        },
        {
            id: "movie",
            label: "Movie",
            icon: "M",
            mediaTypes: ["movie"],
            sortOptions: [
                { key: "rating", label: "Rating" },
                { key: "date", label: "Consume" },
            ],
        },
        {
            id: "book",
            label: "Book",
            icon: "B",
            mediaTypes: ["book"],
            sortOptions: [
                { key: "rating", label: "Rating" },
                { key: "date", label: "Consume" },
            ],
        },
    ];

    const GALLERY_CATEGORY_LETTERS: Record<GalleryCategory, LetterConfig[]> = {
        anime: [
            { char: "A", size: "1.15em", yOffset: -2, rotate: -5, weight: 800 },
            {
                char: "n",
                size: "0.85em",
                yOffset: 3,
                rotate: 4,
                color: "black",
                rounded: true,
            },
            { char: "i", size: "0.80em", yOffset: -1, rotate: -3 },
            {
                char: "M",
                size: "1.0em",
                yOffset: 2,
                rotate: 5,
                color: "black",
                outline: true,
            },
            { char: "e", size: "0.78em", yOffset: -2, rotate: -4 },
        ],
        game: [
            { char: "G", size: "1.18em", yOffset: -3, rotate: -6, weight: 800 },
            {
                char: "a",
                size: "0.85em",
                yOffset: 3,
                rotate: 4,
                color: "black",
            },
            { char: "M", size: "0.80em", yOffset: -1, rotate: -3 },
            {
                char: "e",
                size: "0.92em",
                yOffset: 2,
                rotate: 5,
                color: "black",
                rounded: true,
            },
            { char: "S", size: "1.05em", yOffset: -2, rotate: -4 },
        ],
        tv: [
            { char: "T", size: "1.2em", yOffset: -3, rotate: -5, weight: 800 },
            {
                char: "V",
                size: "1.1em",
                yOffset: 2,
                rotate: 4,
                color: "black",
                outline: true,
            },
        ],
        movie: [
            { char: "M", size: "1.15em", yOffset: -2, rotate: -5, weight: 800 },
            {
                char: "o",
                size: "0.82em",
                yOffset: 3,
                rotate: 4,
                color: "black",
                rounded: true,
            },
            { char: "V", size: "0.90em", yOffset: -1, rotate: -3 },
            {
                char: "i",
                size: "0.78em",
                yOffset: 2,
                rotate: 5,
                color: "black",
            },
            { char: "E", size: "1.05em", yOffset: -2, rotate: -4 },
        ],
        book: [
            { char: "B", size: "1.18em", yOffset: -3, rotate: -6, weight: 800 },
            {
                char: "o",
                size: "0.82em",
                yOffset: 3,
                rotate: 3,
                color: "black",
                rounded: true,
            },
            { char: "O", size: "0.90em", yOffset: -1, rotate: -4 },
            {
                char: "K",
                size: "1.05em",
                yOffset: 2,
                rotate: 5,
                color: "black",
                outline: true,
            },
        ],
    };

    const GALLERY_QUAD_CONFIGS: { rot: number; clip: string }[] = [
        { rot: -8, clip: "polygon(3% 5%, 97% 0%, 95% 95%, 1% 100%)" },
        { rot: -4, clip: "polygon(1% 8%, 99% 2%, 97% 92%, 3% 98%)" },
        { rot: -1, clip: "polygon(2% 0%, 98% 6%, 96% 96%, 0% 88%)" },
        { rot: 1, clip: "polygon(0% 6%, 98% 0%, 100% 94%, 2% 100%)" },
        { rot: 3, clip: "polygon(1% 4%, 97% 0%, 100% 90%, 3% 96%)" },
        { rot: -2, clip: "polygon(0% 8%, 99% 0%, 100% 100%, 2% 92%)" },
    ];

    function selectGalleryCategory(id: GalleryCategory) {
        galleryActiveCategory = id;
        const cat = GALLERY_CATEGORIES.find((c) => c.id === id)!;
        gallerySortKey = cat.sortOptions[0].key;
        gallerySortOrder = "desc";
    }

    function getActiveSortOptions(): { key: GallerySortKey; label: string }[] {
        return (
            GALLERY_CATEGORIES.find((c) => c.id === galleryActiveCategory)
                ?.sortOptions ?? []
        );
    }

    function toggleGallerySort(key: GallerySortKey) {
        if (gallerySortKey === key) {
            gallerySortOrder = gallerySortOrder === "asc" ? "desc" : "asc";
        } else {
            gallerySortKey = key;
            gallerySortOrder = "desc";
        }
    }

    function getMediaType(item: MediaItem): string {
        if (!galleryData) return "unknown";
        const src = galleryData.sources.find((s) => s.id === item.source_id);
        return src?.media_type ?? "unknown";
    }

    function getFilteredGalleryItems(): MediaItem[] {
        if (!galleryData) return [];

        const cat = GALLERY_CATEGORIES.find(
            (c) => c.id === galleryActiveCategory,
        );
        if (!cat) return [];

        let items = galleryData.items.filter((i) =>
            cat.mediaTypes.includes(getMediaType(i)),
        );

        const dir = gallerySortOrder === "asc" ? 1 : -1;
        items = [...items].sort((a, b) => {
            switch (gallerySortKey) {
                case "rating": {
                    const ar = a.my_rating ?? a.rating ?? -1;
                    const br = b.my_rating ?? b.rating ?? -1;
                    if (ar !== br) return dir * (ar - br);
                    return a.name.localeCompare(b.name);
                }
                case "playtime": {
                    const ah = (a.extra.playtime_hours as number) ?? -1;
                    const bh = (b.extra.playtime_hours as number) ?? -1;
                    if (ah !== bh) return dir * (ah - bh);
                    return a.name.localeCompare(b.name);
                }
                case "date": {
                    const ad = a.date_finished ?? "";
                    const bd = b.date_finished ?? "";
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
        const rotations = [
            -1.2, 0.8, -0.5, 1.4, -1.0, 0.6, -1.5, 1.1, -0.3, 0.9,
        ];
        return `${rotations[index % rotations.length]}deg`;
    }

    function getDisplayRating(
        item: MediaItem,
    ): { value: number; isPersonal: boolean } | null {
        if (item.my_rating !== null)
            return { value: item.my_rating, isPersonal: true };
        if (item.rating !== null)
            return { value: item.rating, isPersonal: false };
        return null;
    }

    function formatRating(rating: number | null): string {
        if (rating === null || rating === undefined) return "—";
        return rating.toFixed(1);
    }

    function ratingToStars(rating: number): ("full" | "half" | "empty")[] {
        const stars: ("full" | "half" | "empty")[] = [];
        const rounded = Math.round(rating * 2) / 2;
        for (let i = 1; i <= 10; i++) {
            if (i <= rounded) stars.push("full");
            else if (i - 0.5 === rounded) stars.push("half");
            else stars.push("empty");
        }
        return stars;
    }

    function proxyCover(url: string | null | undefined): string | undefined {
        if (!url) return undefined;
        if (url.includes("doubanio.com")) {
            return `http://imgproxy.localhost/${encodeURIComponent(url)}`;
        }
        return url;
    }

    function handleCoverError(e: Event) {
        const img = e.target as HTMLImageElement;
        const retries = Number(img.dataset.retries ?? 0);
        if (retries < 3) {
            img.dataset.retries = String(retries + 1);
            setTimeout(
                () => {
                    const src = img.src;
                    img.src = "";
                    img.src = src;
                },
                1000 * (retries + 1),
            );
            return;
        }
        img.style.display = "none";
        const fallback = img.nextElementSibling as HTMLElement | null;
        if (fallback) fallback.style.display = "flex";
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            event.preventDefault();
            if (selectedMedia) {
                selectedMedia = null;
            } else {
                onBack();
            }
        }
    }

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

    // Selection quad effect for sidebar
    $effect(() => {
        const idx = GALLERY_CATEGORIES.findIndex(
            (c) => c.id === galleryActiveCategory,
        );
        const btn = catBtnRefs[idx];
        const container = sidebarRef;
        if (!btn || !container) return;

        const btnRect = btn.getBoundingClientRect();
        const containerRect = container.getBoundingClientRect();

        const centerX = btnRect.left + btnRect.width / 2 - containerRect.left;
        const centerY = btnRect.top + btnRect.height / 2 - containerRect.top;

        const quadW = btn.offsetWidth * 1.5;
        const quadH = btn.offsetHeight * 1.3;
        const cfg =
            GALLERY_QUAD_CONFIGS[idx % GALLERY_QUAD_CONFIGS.length];

        container.style.setProperty(
            "--pack-quad-x",
            `${centerX - quadW / 2}px`,
        );
        container.style.setProperty(
            "--pack-quad-y",
            `${centerY - quadH / 2}px`,
        );
        container.style.setProperty("--pack-quad-w", `${quadW}px`);
        container.style.setProperty("--pack-quad-h", `${quadH}px`);
        container.style.setProperty("--pack-quad-rot", `${cfg.rot}deg`);
        container.style.setProperty("--pack-quad-clip", cfg.clip);
    });

    onMount(() => {
        if (!galleryData && !galleryLoading) {
            void loadGalleryData();
        }

        window.addEventListener("keydown", handleKeydown);
        return () => {
            window.removeEventListener("keydown", handleKeydown);
        };
    });
</script>

<section class="rm-stage">
    <button
        type="button"
        class="rm-back-btn"
        onclick={() => {
            if (selectedMedia) {
                selectedMedia = null;
            } else {
                onBack();
            }
        }}
    >
        <KeyHint key="Esc" fontSize={36} />
        <PromptWord text="Back" fontSize={72} />
    </button>

    {#if galleryLoading}
        <p class="state-text" style="padding: 2rem;">Loading gallery...</p>
    {:else if galleryError}
        <p class="state-text error" style="padding: 2rem;">{galleryError}</p>
    {:else if galleryData && !selectedMedia}
        <div class="rm-gallery-layout">
            <!-- LEFT: category nav + sort -->
            <div class="rm-gallery-sidebar" bind:this={sidebarRef}>
                <ul class="rm-gallery-pack-list">
                    {#each GALLERY_CATEGORIES as cat, ci}
                        <li
                            class="rm-gallery-pack-line"
                            style:z-index={galleryActiveCategory === cat.id ? 10 : 0}
                        >
                            <button
                                type="button"
                                class="rm-gallery-pack-btn"
                                class:is-active={galleryActiveCategory === cat.id}
                                onclick={() => selectGalleryCategory(cat.id)}
                                onmouseenter={() => selectGalleryCategory(cat.id)}
                                bind:this={catBtnRefs[ci]}
                            >
                                <MenuItem
                                    letters={GALLERY_CATEGORY_LETTERS[cat.id]}
                                    active={galleryActiveCategory === cat.id}
                                />
                            </button>
                        </li>
                    {/each}
                </ul>
                <div class="rm-gallery-pack-quad" aria-hidden="true"></div>

                <div class="rm-gallery-filters">
                    <div class="rm-gallery-filter-row">
                        <PromptWord text="Sort" fontSize={36} />
                        {#each getActiveSortOptions() as opt}
                            <button
                                type="button"
                                class="rm-gallery-tab"
                                class:active={gallerySortKey === opt.key}
                                onclick={() => toggleGallerySort(opt.key)}
                            >
                                {opt.label}{#if gallerySortKey === opt.key}{gallerySortOrder === "asc" ? " ▲" : " ▼"}{/if}
                            </button>
                        {/each}
                    </div>
                </div>
            </div>

            <!-- RIGHT: waterfall cover wall -->
            <div class="rm-gallery-content">
                {#if getFilteredGalleryItems().length === 0}
                    <p class="state-text" style="padding: 2rem;">
                        No items match the current filter.
                    </p>
                {:else}
                    <div class="rm-gallery-wall">
                        {#each getFilteredGalleryItems() as item, i (item.id)}
                            {@const displayRating = getDisplayRating(item)}
                            {@const itemMediaType = getMediaType(item)}
                            <button
                                type="button"
                                class="rm-gallery-card"
                                style="transform: rotate({getCardRotation(i)});"
                                onclick={() => {
                                    selectedMedia = item;
                                }}
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
                                        <div
                                            class="rm-gallery-card-fallback"
                                            style="display:none;"
                                        ></div>
                                    {:else}
                                        <div
                                            class="rm-gallery-card-fallback"
                                        ></div>
                                    {/if}
                                </div>
                                <div class="rm-gallery-card-info">
                                    <span class="rm-gallery-card-name"
                                        >{item.name}</span
                                    >
                                    {#if itemMediaType === "game"}
                                        <div class="rm-gallery-card-game-meta">
                                            {#if item.extra.playtime_hours}
                                                <span
                                                    class="rm-gallery-card-playtime"
                                                    >{item.extra
                                                        .playtime_hours}h</span
                                                >
                                            {/if}
                                            {#if item.extra.achievement_total}
                                                <div
                                                    class="rm-gallery-card-ach"
                                                >
                                                    <div
                                                        class="rm-gallery-card-ach-bar"
                                                    >
                                                        <div
                                                            class="rm-gallery-card-ach-fill"
                                                            style="width: {(
                                                                (((item.extra
                                                                    .achievement_unlocked as number) ??
                                                                    0) /
                                                                    (item.extra
                                                                        .achievement_total as number)) *
                                                                100
                                                            ).toFixed(0)}%"
                                                        ></div>
                                                    </div>
                                                    <span
                                                        class="rm-gallery-card-ach-text"
                                                        >{item.extra
                                                            .achievement_unlocked ??
                                                            0}/{item.extra
                                                            .achievement_total}</span
                                                    >
                                                </div>
                                            {/if}
                                        </div>
                                    {:else if displayRating}
                                        <div
                                            class="rm-gallery-card-stars"
                                            class:is-community={!displayRating.isPersonal}
                                        >
                                            {#each ratingToStars(displayRating.value) as star}
                                                <span
                                                    class="rm-gallery-star rm-gallery-star--{star}"
                                                    >★</span
                                                >
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
                        <img
                            src={proxyCover(selectedMedia.cover)}
                            alt={selectedMedia.name}
                            class="rm-gallery-detail-img"
                        />
                    {:else}
                        <div
                            class="rm-gallery-card-placeholder rm-gallery-detail-placeholder"
                        >
                            <span class="rm-gallery-card-placeholder-text"
                                >{selectedMedia.name.charAt(0)}</span
                            >
                        </div>
                    {/if}
                </div>
                <div class="rm-gallery-detail-info">
                    <h2 class="rm-gallery-detail-title">
                        {selectedMedia.name}
                    </h2>
                    {#if selectedMedia.name_original}
                        <p class="rm-gallery-detail-original">
                            {selectedMedia.name_original}
                        </p>
                    {/if}

                    {#if getMediaType(selectedMedia) === "game"}
                        <!-- Game-specific detail -->
                        <div class="rm-gallery-detail-meta">
                            {#if selectedMedia.extra.playtime_hours != null}
                                <div class="rm-gallery-detail-row">
                                    <span class="rm-gallery-detail-label"
                                        >PLAYTIME</span
                                    >
                                    <span class="rm-gallery-detail-value"
                                        >{selectedMedia.extra
                                            .playtime_hours}h</span
                                    >
                                </div>
                            {/if}
                            {#if selectedMedia.extra.achievement_total}
                                <div class="rm-gallery-detail-row">
                                    <span class="rm-gallery-detail-label"
                                        >ACHIEVEMENTS</span
                                    >
                                    <span class="rm-gallery-detail-value"
                                        >{selectedMedia.extra
                                            .achievement_unlocked ?? 0} / {selectedMedia
                                            .extra.achievement_total}</span
                                    >
                                </div>
                                <div class="rm-gallery-detail-ach-bar">
                                    <div
                                        class="rm-gallery-detail-ach-fill"
                                        style="width: {(
                                            (((selectedMedia.extra
                                                .achievement_unlocked as number) ??
                                                0) /
                                                (selectedMedia.extra
                                                    .achievement_total as number)) *
                                            100
                                        ).toFixed(0)}%"
                                    ></div>
                                </div>
                            {/if}
                            {#if selectedMedia.extra.release_date}
                                <div class="rm-gallery-detail-row">
                                    <span class="rm-gallery-detail-label"
                                        >RELEASE</span
                                    >
                                    <span class="rm-gallery-detail-value"
                                        >{selectedMedia.extra
                                            .release_date}</span
                                    >
                                </div>
                            {/if}
                            {#if selectedMedia.rating !== null}
                                <div class="rm-gallery-detail-row">
                                    <span class="rm-gallery-detail-label"
                                        >METACRITIC</span
                                    >
                                    <span class="rm-gallery-detail-value"
                                        >{formatRating(
                                            selectedMedia.rating,
                                        )}</span
                                    >
                                </div>
                            {/if}
                            {#if selectedMedia.extra.steam_url}
                                <div class="rm-gallery-detail-row">
                                    <span class="rm-gallery-detail-label"
                                        >STEAM</span
                                    >
                                    <span class="rm-gallery-detail-value"
                                        >{selectedMedia.extra.steam_url}</span
                                    >
                                </div>
                            {/if}
                        </div>
                    {:else}
                        <!-- Default (anime/media) detail -->
                        <div class="rm-gallery-detail-meta">
                            {#if selectedMedia.rating !== null}
                                <div class="rm-gallery-detail-row">
                                    <span class="rm-gallery-detail-label"
                                        >RATING</span
                                    >
                                    <span class="rm-gallery-detail-value"
                                        >{formatRating(
                                            selectedMedia.rating,
                                        )}</span
                                    >
                                </div>
                            {/if}
                            {#if selectedMedia.my_rating !== null}
                                <div class="rm-gallery-detail-row">
                                    <span class="rm-gallery-detail-label"
                                        >MY RATING</span
                                    >
                                    <span
                                        class="rm-gallery-detail-value rm-gallery-detail-myrating"
                                        >{formatRating(
                                            selectedMedia.my_rating,
                                        )}</span
                                    >
                                </div>
                            {/if}
                            {#if selectedMedia.episodes !== null}
                                <div class="rm-gallery-detail-row">
                                    <span class="rm-gallery-detail-label"
                                        >EPISODES</span
                                    >
                                    <span class="rm-gallery-detail-value"
                                        >{selectedMedia.episodes}</span
                                    >
                                </div>
                            {/if}
                            {#if selectedMedia.date_started}
                                <div class="rm-gallery-detail-row">
                                    <span class="rm-gallery-detail-label"
                                        >STARTED</span
                                    >
                                    <span class="rm-gallery-detail-value"
                                        >{selectedMedia.date_started}</span
                                    >
                                </div>
                            {/if}
                            {#if selectedMedia.date_finished}
                                <div class="rm-gallery-detail-row">
                                    <span class="rm-gallery-detail-label"
                                        >FINISHED</span
                                    >
                                    <span class="rm-gallery-detail-value"
                                        >{selectedMedia.date_finished}</span
                                    >
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
        <p class="state-text" style="padding: 2rem;">
            Gallery data is not available yet.
        </p>
    {/if}
</section>
<style>

    /* ── Gallery filter tabs ── */
    .rm-gallery-filters {
        display: flex;
        flex-direction: column;
        gap: clamp(0.5rem, 0.6vw, 1rem);
    }

    .rm-gallery-filter-row {
        display: flex;
        align-items: center;
        gap: clamp(0.3rem, 0.5vw, 0.8rem);
        flex-wrap: wrap;
    }

    .rm-gallery-tab {
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
        flex-shrink: 0;
    }

    .rm-gallery-tab::before {
        content: "";
        position: absolute;
        inset: 4px;
        background: var(--rm-black);
        clip-path: polygon(0% 0%, 100% 0%, 96% 100%, 4% 100%);
        z-index: -1;
        transition: background 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    .rm-gallery-tab:hover {
        transform: scale(1.06);
    }

    .rm-gallery-tab.active {
        background: var(--rm-white);
        color: var(--rm-black);
    }

    .rm-gallery-tab.active::before {
        background: var(--rm-white);
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
        position: relative;
        overflow-y: auto;
        height: 100%;
        padding: clamp(1.5rem, 2.5vh, 4rem) clamp(1.2rem, 2vw, 3rem)
            clamp(6rem, 10vh, 10rem) clamp(1.5rem, 2.5vw, 4rem);
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: clamp(1.5rem, 2.5vh, 3rem);
        scrollbar-gutter: stable;
    }

    .rm-gallery-sidebar::-webkit-scrollbar {
        width: 14px;
    }
    .rm-gallery-sidebar::-webkit-scrollbar-track {
        background: var(--rm-black, #000);
        border: 4px solid var(--rm-white, #fff);
        border-radius: 0;
        margin-top: 12vh;
        margin-bottom: 12vh;
    }
    .rm-gallery-sidebar::-webkit-scrollbar-thumb {
        background: var(--rm-white, #fff);
        border-radius: 0;
        border: none;
    }
    .rm-gallery-sidebar::-webkit-scrollbar-thumb:hover {
        background: var(--rm-white, #fff);
    }

    .rm-gallery-pack-list {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
    }

    .rm-gallery-pack-line {
        margin: -1.2rem 0;
        position: relative;
    }

    .rm-gallery-pack-line:nth-child(odd) {
        margin-left: 0;
    }
    .rm-gallery-pack-line:nth-child(even) {
        margin-left: 3vw;
    }

    .rm-gallery-pack-btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        border: none;
        background: var(--rm-black);
        cursor: pointer;
        padding: 1.1rem 2.8rem 1.1rem 2.4rem;
        width: fit-content;
        transition: background-color 140ms ease;
    }

    .rm-gallery-pack-btn:not(.is-active):hover {
        background: var(--rm-red);
    }

    .rm-gallery-pack-btn.is-active {
        background: var(--rm-red);
    }

    /* Per-item rotation + clip-path */
    .rm-gallery-pack-line:nth-child(6n + 1) .rm-gallery-pack-btn {
        transform: rotate(-5deg);
        clip-path: polygon(0% 8%, 100% 0%, 98% 92%, 2% 100%);
    }
    .rm-gallery-pack-line:nth-child(6n + 2) .rm-gallery-pack-btn {
        transform: rotate(-3deg);
        clip-path: polygon(1% 5%, 99% 0%, 97% 96%, 0% 100%);
    }
    .rm-gallery-pack-line:nth-child(6n + 3) .rm-gallery-pack-btn {
        transform: rotate(-1deg);
        clip-path: polygon(2% 0%, 100% 4%, 96% 100%, 0% 92%);
    }
    .rm-gallery-pack-line:nth-child(6n + 4) .rm-gallery-pack-btn {
        transform: rotate(1deg);
        clip-path: polygon(0% 6%, 98% 0%, 100% 94%, 3% 100%);
    }
    .rm-gallery-pack-line:nth-child(6n + 5) .rm-gallery-pack-btn {
        transform: rotate(2deg);
        clip-path: polygon(1% 0%, 97% 4%, 99% 100%, 2% 96%);
    }
    .rm-gallery-pack-line:nth-child(6n + 6) .rm-gallery-pack-btn {
        transform: rotate(-2deg);
        clip-path: polygon(0% 4%, 100% 0%, 98% 96%, 1% 100%);
    }

    .rm-gallery-pack-btn :global(.p5m) {
        font-size: clamp(3.6rem, 7vw, 5.6rem);
    }

    .rm-gallery-pack-quad {
        position: absolute;
        left: var(--pack-quad-x);
        top: var(--pack-quad-y);
        width: var(--pack-quad-w);
        height: var(--pack-quad-h);
        transform: rotate(var(--pack-quad-rot));
        z-index: 15;
        background: var(--rm-red);
        mix-blend-mode: difference;
        clip-path: var(
            --pack-quad-clip,
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

    .rm-gallery-content {
        overflow-y: auto;
        height: 100%;
        padding: clamp(6rem, 10vh, 12rem) clamp(2rem, 3vw, 5rem)
            clamp(7rem, 12vh, 12rem) clamp(1.5rem, 2.5vw, 4rem);
        box-sizing: border-box;
        scrollbar-gutter: stable;
        margin-right: clamp(1rem, 2vw, 3rem);
    }

    .rm-gallery-content::-webkit-scrollbar {
        width: 14px;
    }
    .rm-gallery-content::-webkit-scrollbar-track {
        background: var(--rm-black, #000);
        border: 4px solid var(--rm-white, #fff);
        border-radius: 0;
        margin-top: 12vh;
        margin-bottom: 12vh;
    }
    .rm-gallery-content::-webkit-scrollbar-thumb {
        background: var(--rm-white, #fff);
        border-radius: 0;
        border: none;
    }
    .rm-gallery-content::-webkit-scrollbar-thumb:hover {
        background: var(--rm-white, #fff);
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
        transition:
            transform 120ms ease,
            box-shadow 120ms ease;
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
        scrollbar-gutter: stable;
        margin-right: clamp(1rem, 2vw, 3rem);
    }

    .rm-gallery-detail::-webkit-scrollbar {
        width: 14px;
    }
    .rm-gallery-detail::-webkit-scrollbar-track {
        background: var(--rm-black, #000);
        border: 4px solid var(--rm-white, #fff);
        border-radius: 0;
        margin-top: 12vh;
        margin-bottom: 12vh;
    }
    .rm-gallery-detail::-webkit-scrollbar-thumb {
        background: var(--rm-white, #fff);
        border-radius: 0;
        border: none;
    }
    .rm-gallery-detail::-webkit-scrollbar-thumb:hover {
        background: var(--rm-white, #fff);
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
</style>
