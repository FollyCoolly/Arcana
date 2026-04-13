<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import CallingCardText from "$lib/CallingCardText.svelte";
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
                { key: "rating", label: "评分" },
                { key: "date", label: "看完" },
            ],
        },
        {
            id: "game",
            label: "Games",
            icon: "G",
            mediaTypes: ["game"],
            sortOptions: [
                { key: "playtime", label: "时长" },
                { key: "rating", label: "评分" },
            ],
        },
        {
            id: "tv",
            label: "TV",
            icon: "T",
            mediaTypes: ["tv"],
            sortOptions: [
                { key: "rating", label: "评分" },
                { key: "date", label: "看完" },
            ],
        },
        {
            id: "movie",
            label: "Movie",
            icon: "M",
            mediaTypes: ["movie"],
            sortOptions: [
                { key: "rating", label: "评分" },
                { key: "date", label: "看完" },
            ],
        },
        {
            id: "book",
            label: "Book",
            icon: "B",
            mediaTypes: ["book"],
            sortOptions: [
                { key: "rating", label: "评分" },
                { key: "date", label: "读完" },
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
    <div class="rm-items-title">
        <CallingCardText text="Gallery" fontSize={82} />
    </div>

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
                            <MenuItem
                                letters={GALLERY_CATEGORY_LETTERS[cat.id]}
                            />
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
                                <span class="rm-items-sort-arrow"
                                    >{gallerySortOrder === "asc"
                                        ? "↑"
                                        : "↓"}</span
                                >
                            {/if}
                        </button>
                    {/each}
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
    .rm-items-title {
        position: fixed;
        top: clamp(0.8rem, 1.5vh, 3rem);
        right: clamp(1.2rem, 2.5vw, 5rem);
        z-index: 10;
        pointer-events: none;
    }

    /* ── Shared filter styles ── */
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

    .rm-items-sort-arrow {
        margin-left: 0.2em;
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
        padding: clamp(1.5rem, 2.5vh, 4rem) clamp(1.2rem, 2vw, 3rem)
            clamp(6rem, 10vh, 10rem) clamp(1.5rem, 2.5vw, 4rem);
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
        transition:
            opacity 140ms ease,
            transform 140ms ease;
    }

    .rm-gallery-nav-item::before {
        content: "";
        position: absolute;
        inset: 0;
        background: var(--rm-red, #e5191c);
        clip-path: polygon(4% 8%, 98% 0%, 96% 94%, 1% 100%);
        transform: skewX(-2deg) rotate(-0.5deg);
        opacity: 0;
        z-index: -1;
        transition:
            opacity 120ms ease,
            transform 120ms ease;
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
        padding: clamp(6rem, 10vh, 12rem) clamp(2rem, 3vw, 5rem)
            clamp(7rem, 12vh, 12rem) clamp(1.5rem, 2.5vw, 4rem);
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
