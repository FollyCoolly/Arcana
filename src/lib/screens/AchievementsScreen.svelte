<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import CallingCardText from "$lib/CallingCardText.svelte";

    import MenuItem from "$lib/MenuItem.svelte";
    import type { LetterConfig } from "$lib/MenuItem.svelte";
    import type {
        AchievementData,
        Achievement,
        Difficulty,
        PackAchievements,
    } from "$lib/types/achievement";
    import KeyHint from "$lib/KeyHint.svelte";
    import PromptWord from "$lib/PromptWord.svelte";

    type SortKey = "default" | "name" | "difficulty" | "unlocked";
    type SortDir = "asc" | "desc";

    let {
        onBack,
        achievementData: externalData = null,
        onAchievementDataLoaded,
    }: {
        onBack: () => void;
        achievementData?: AchievementData | null;
        onAchievementDataLoaded?: (data: AchievementData) => void;
    } = $props();

    let achievementLoading = $state(false);
    let achievementError = $state<string | null>(null);
    let achievementData = $state<AchievementData | null>(externalData);
    let selectedPackIndex = $state(0);

    // Sidebar refs for selection quad
    let sidebarRef = $state<HTMLElement | undefined>(undefined);
    let packBtnRefs = $state<(HTMLButtonElement | undefined)[]>([]);

    // Filter & sort state
    let selectedDifficulties = $state<Set<string>>(new Set(["beginner", "intermediate", "advanced", "expert", "legendary"]));
    let showUnlockedOnly = $state(false);
    let sortKey = $state<SortKey>("unlocked");
    let sortDir = $state<SortDir>("asc");

    const DIFFICULTIES: Difficulty[] = [
        "beginner",
        "intermediate",
        "advanced",
        "expert",
        "legendary",
    ];

    const PACK_QUAD_CONFIGS: { rot: number; clip: string }[] = [
        { rot: -8, clip: "polygon(3% 5%, 97% 0%, 95% 95%, 1% 100%)" },
        { rot: -4, clip: "polygon(1% 8%, 99% 2%, 97% 92%, 3% 98%)" },
        { rot: -1, clip: "polygon(2% 0%, 98% 6%, 96% 96%, 0% 88%)" },
        { rot: 1, clip: "polygon(0% 6%, 98% 0%, 100% 94%, 2% 100%)" },
        { rot: 3, clip: "polygon(1% 4%, 97% 0%, 100% 90%, 3% 96%)" },
        { rot: -2, clip: "polygon(0% 8%, 99% 0%, 100% 100%, 2% 92%)" },
    ];

    // Memoized letter configs for pack names
    const packLetterCache = new Map<string, LetterConfig[]>();

    function getPackLetterConfigs(
        packName: string,
        packIndex: number,
    ): LetterConfig[] {
        const key = `${packIndex}:${packName}`;
        if (packLetterCache.has(key)) return packLetterCache.get(key)!;

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

        const letters: LetterConfig[] = packName.split("").map((char, i) => {
            if (char === " ")
                return { char: " ", size: "0.5em", yOffset: 0, rotate: 0 };
            const seed = packIndex * 37 + i * 13;
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

        packLetterCache.set(key, letters);
        return letters;
    }

    const DIFFICULTY_ORDER: Record<string, number> = {
        beginner: 0,
        intermediate: 1,
        advanced: 2,
        expert: 3,
        legendary: 4,
    };

    // Derived: filtered + sorted achievements
    let filteredAchievements = $derived.by((): Achievement[] => {
        const pack = achievementData?.packs[selectedPackIndex];
        if (!pack) return [];
        const filtered = pack.achievements.filter((a) => {
            if (
                selectedDifficulties.size > 0 &&
                !selectedDifficulties.has(a.difficulty)
            )
                return false;
            if (showUnlockedOnly && !achievementData?.progress[a.id])
                return false;
            return true;
        });

        if (sortKey === "default") return filtered;

        const dir = sortDir === "asc" ? 1 : -1;
        return filtered.toSorted((a, b) => {
            if (sortKey === "name") return dir * a.name.localeCompare(b.name);
            if (sortKey === "difficulty")
                return (
                    dir *
                    ((DIFFICULTY_ORDER[a.difficulty] ?? 0) -
                        (DIFFICULTY_ORDER[b.difficulty] ?? 0))
                );
            if (sortKey === "unlocked") {
                const ua = achievementData?.progress[a.id] ? 1 : 0;
                const ub = achievementData?.progress[b.id] ? 1 : 0;
                return dir * (ub - ua);
            }
            return 0;
        });
    });

    function getDifficultyLabel(difficulty: string): string {
        return difficulty.charAt(0).toUpperCase() + difficulty.slice(1);
    }

    function resetFilters() {
        selectedDifficulties = new Set(DIFFICULTIES);
        showUnlockedOnly = false;
        sortKey = "unlocked";
        sortDir = "asc";
    }

    function toggleSort(key: SortKey) {
        if (sortKey === key) {
            if (sortDir === "asc") {
                sortDir = "desc";
            } else {
                sortKey = "default";
                sortDir = "asc";
            }
        } else {
            sortKey = key;
            sortDir = "asc";
        }
    }

    function getSortIndicator(key: SortKey): string {
        if (sortKey !== key) return "";
        return sortDir === "asc" ? " ▲" : " ▼";
    }

    function selectPack(index: number) {
        selectedPackIndex = index;
        resetFilters();
    }

    function toggleDifficulty(d: string) {
        const next = new Set(selectedDifficulties);
        if (next.has(d)) next.delete(d);
        else next.add(d);
        selectedDifficulties = next;
    }

    function getSelectedPack(): PackAchievements | null {
        return achievementData?.packs[selectedPackIndex] ?? null;
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            event.preventDefault();
            onBack();
        }
    }

    // Selection quad effect for sidebar
    $effect(() => {
        const idx = selectedPackIndex;
        const btn = packBtnRefs[idx];
        const container = sidebarRef;
        if (!btn || !container) return;

        const btnRect = btn.getBoundingClientRect();
        const containerRect = container.getBoundingClientRect();

        const centerX = btnRect.left + btnRect.width / 2 - containerRect.left;
        const centerY = btnRect.top + btnRect.height / 2 - containerRect.top;

        const quadW = btn.offsetWidth * 1.5;
        const quadH = btn.offsetHeight * 1.3;
        const cfg = PACK_QUAD_CONFIGS[idx % PACK_QUAD_CONFIGS.length];

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

    async function loadAchievementData() {
        achievementLoading = true;
        achievementError = null;

        try {
            achievementData =
                await invoke<AchievementData>("load_achievements");
            if (achievementData && onAchievementDataLoaded) {
                onAchievementDataLoaded(achievementData);
            }
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

    onMount(() => {
        if (!achievementData && !achievementLoading) {
            void loadAchievementData();
        }

        window.addEventListener("keydown", handleKeydown);
        return () => {
            window.removeEventListener("keydown", handleKeydown);
        };
    });
</script>

<section class="rm-stage">
    <button type="button" class="rm-back-btn" onclick={onBack}>
        <KeyHint key="Esc" fontSize={36} />
        <PromptWord text="Back" fontSize={72} />
    </button>

    <div class="rm-ach-layout">
        {#if achievementLoading}
            <p class="state-text">Loading achievements...</p>
        {:else if achievementError}
            <p class="state-text error">{achievementError}</p>
        {:else if achievementData}
            <!-- LEFT: Pack nav (main menu style) -->
            <nav class="rm-ach-sidebar" bind:this={sidebarRef}>
                <ul class="rm-ach-pack-list">
                    {#each achievementData.packs as pack, pi}
                        <li
                            class="rm-ach-pack-line"
                            style:z-index={pi === selectedPackIndex ? 10 : 0}
                        >
                            <button
                                type="button"
                                class="rm-ach-pack-btn"
                                class:is-active={pi === selectedPackIndex}
                                onclick={() => selectPack(pi)}
                                onmouseenter={() => {
                                    selectedPackIndex = pi;
                                    resetFilters();
                                }}
                                bind:this={packBtnRefs[pi]}
                            >
                                <MenuItem
                                    letters={getPackLetterConfigs(
                                        pack.pack_name,
                                        pi,
                                    )}
                                    active={pi === selectedPackIndex}
                                />
                            </button>
                        </li>
                    {/each}
                </ul>
                <div class="rm-ach-pack-quad" aria-hidden="true"></div>
            </nav>

            <!-- RIGHT: Achievement content -->
            <div class="rm-ach-content">
                {#if getSelectedPack()}
                    {@const pack = getSelectedPack()!}

                    <div class="rm-ach-content-header">
                        <CallingCardText text={pack.pack_name} fontSize={104} />
                        <CallingCardText text="Achievements" fontSize={104} />
                    </div>

                    <!-- FILTER BAR -->
                    <div class="rm-ach-filters">
                        <div class="rm-ach-filter-row">
                            <PromptWord text="Sort" fontSize={36} />
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
                                class:active={sortKey === "difficulty"}
                                onclick={() => toggleSort("difficulty")}
                            >
                                Diff{getSortIndicator("difficulty")}
                            </button>
                            <button
                                type="button"
                                class="rm-ach-tab"
                                class:active={sortKey === "unlocked"}
                                onclick={() => toggleSort("unlocked")}
                            >
                                Unlock{getSortIndicator("unlocked")}
                            </button>

                            <span class="rm-ach-filter-divider"></span>

                            <PromptWord text="Diff" fontSize={36} />
                            {#each DIFFICULTIES as diff}
                                <button
                                    type="button"
                                    class="rm-ach-tab"
                                    class:active={selectedDifficulties.has(
                                        diff,
                                    )}
                                    onclick={() => toggleDifficulty(diff)}
                                >
                                    {getDifficultyLabel(diff)}
                                </button>
                            {/each}

                            <span class="rm-ach-filter-divider"></span>

                            <PromptWord text="Status" fontSize={36} />
                            <button
                                type="button"
                                class="rm-ach-tab active"
                                onclick={() => {
                                    showUnlockedOnly = !showUnlockedOnly;
                                }}
                            >
                                {showUnlockedOnly ? "Unlocked" : "All"}
                            </button>
                        </div>
                    </div>

                    <!-- Achievement grid (scrollable) -->
                    <div class="rm-ach-card-scroll">
                        {#if filteredAchievements.length === 0}
                            <p class="rm-ach-empty">
                                No achievements match the current filters.
                            </p>
                        {:else}
                            <div class="rm-achievement-grid">
                                {#each filteredAchievements as achievement}
                                    {@const unlocked =
                                        achievementData!.progress[
                                            achievement.id
                                        ]}
                                    <article
                                        class="rm-achievement-card"
                                        class:is-unlocked={!!unlocked}
                                    >
                                        <div class="rm-achievement-card-header">
                                            <span
                                                class="rm-achievement-status-icon"
                                                >{unlocked ? "✓" : "○"}</span
                                            >
                                            <span class="rm-achievement-name"
                                                >{achievement.name}</span
                                            >
                                            <span
                                                class="rm-difficulty rm-difficulty--{achievement.difficulty}"
                                                >{getDifficultyLabel(
                                                    achievement.difficulty,
                                                )}</span
                                            >
                                        </div>
                                        <p class="rm-achievement-desc">
                                            {achievement.description}
                                        </p>
                                        {#if unlocked?.achieved_at}
                                            <p class="rm-achievement-date">
                                                {unlocked.achieved_at}
                                            </p>
                                        {/if}
                                        {#if unlocked?.note}
                                            <p class="rm-achievement-note">
                                                {unlocked.note}
                                            </p>
                                        {/if}
                                        {#if achievement.prerequisites.length > 0}
                                            <div class="rm-achievement-prereqs">
                                                {#each achievement.prerequisites as prereq}
                                                    <span class="rm-prereq-tag"
                                                        >{prereq
                                                            .split("::")[1]
                                                            ?.replace(
                                                                /_/g,
                                                                " ",
                                                            ) ?? prereq}</span
                                                    >
                                                {/each}
                                            </div>
                                        {/if}
                                    </article>
                                {/each}
                            </div>
                        {/if}
                    </div>
                {/if}
            </div>
        {:else}
            <p class="state-text">Achievement data is not available yet.</p>
        {/if}
    </div>
</section>

<style>
    .rm-ach-layout {
        flex: 1;
        display: grid;
        grid-template-columns: 1fr 3fr;
        overflow: hidden;
        height: 100%;
    }

    /* ── Sidebar (main menu style) ── */

    .rm-ach-sidebar {
        position: relative;
        overflow-y: auto;
        height: 100%;
        padding: clamp(1.5rem, 2.5vh, 4rem) clamp(1.2rem, 2vw, 3rem)
            clamp(6rem, 10vh, 10rem) clamp(1.5rem, 2.5vw, 4rem);
        box-sizing: border-box;
        scrollbar-gutter: stable;
    }

    .rm-ach-sidebar::-webkit-scrollbar {
        width: 14px;
    }
    .rm-ach-sidebar::-webkit-scrollbar-track {
        background: var(--rm-black, #000);
        border: 4px solid var(--rm-white, #fff);
        border-radius: 0;
        margin-top: 12vh;
        margin-bottom: 12vh;
    }
    .rm-ach-sidebar::-webkit-scrollbar-thumb {
        background: var(--rm-white, #fff);
        border-radius: 0;
        border: none;
    }
    .rm-ach-sidebar::-webkit-scrollbar-thumb:hover {
        background: var(--rm-white, #fff);
    }

    .rm-ach-pack-list {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
    }

    .rm-ach-pack-line {
        margin: -1.2rem 0;
        position: relative;
    }

    .rm-ach-pack-line:nth-child(odd) {
        margin-left: 0;
    }
    .rm-ach-pack-line:nth-child(even) {
        margin-left: 3vw;
    }

    .rm-ach-pack-btn {
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

    .rm-ach-pack-btn:not(.is-active):hover {
        background: var(--rm-red);
    }

    .rm-ach-pack-btn.is-active {
        background: var(--rm-red);
    }

    /* Per-item rotation + clip-path */
    .rm-ach-pack-line:nth-child(6n + 1) .rm-ach-pack-btn {
        transform: rotate(-5deg);
        clip-path: polygon(0% 8%, 100% 0%, 98% 92%, 2% 100%);
    }
    .rm-ach-pack-line:nth-child(6n + 2) .rm-ach-pack-btn {
        transform: rotate(-3deg);
        clip-path: polygon(1% 5%, 99% 0%, 97% 96%, 0% 100%);
    }
    .rm-ach-pack-line:nth-child(6n + 3) .rm-ach-pack-btn {
        transform: rotate(-1deg);
        clip-path: polygon(2% 0%, 100% 4%, 96% 100%, 0% 92%);
    }
    .rm-ach-pack-line:nth-child(6n + 4) .rm-ach-pack-btn {
        transform: rotate(1deg);
        clip-path: polygon(0% 6%, 98% 0%, 100% 94%, 3% 100%);
    }
    .rm-ach-pack-line:nth-child(6n + 5) .rm-ach-pack-btn {
        transform: rotate(2deg);
        clip-path: polygon(1% 0%, 97% 4%, 99% 100%, 2% 96%);
    }
    .rm-ach-pack-line:nth-child(6n + 6) .rm-ach-pack-btn {
        transform: rotate(-2deg);
        clip-path: polygon(0% 4%, 100% 0%, 98% 96%, 1% 100%);
    }

    .rm-ach-pack-btn :global(.p5m) {
        font-size: clamp(3.6rem, 7vw, 5.6rem);
    }

    .rm-ach-pack-quad {
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

    /* ── Content area ── */

    .rm-ach-content {
        display: flex;
        flex-direction: column;
        overflow: hidden;
        height: 100%;
        padding: clamp(1.5rem, 2.5vh, 4rem) clamp(8rem, 14vw, 20rem) 0
            clamp(1.5rem, 2.5vw, 4rem);
        box-sizing: border-box;
    }

    /* Scrollable card region */
    .rm-ach-card-scroll {
        flex: 1;
        overflow-y: auto;
        padding-right: clamp(8rem, 14vw, 20rem);
        padding-bottom: clamp(6rem, 10vh, 10rem);
        scrollbar-gutter: stable;
    }

    .rm-ach-card-scroll::-webkit-scrollbar {
        width: 14px;
    }
    .rm-ach-card-scroll::-webkit-scrollbar-track {
        background: var(--rm-black, #000);
        border: 4px solid var(--rm-white, #fff);
        border-radius: 0;
        margin-top: 12vh;
        margin-bottom: 12vh;
    }
    .rm-ach-card-scroll::-webkit-scrollbar-thumb {
        background: var(--rm-white, #fff);
        border-radius: 0;
        border: none;
    }
    .rm-ach-card-scroll::-webkit-scrollbar-thumb:hover {
        background: var(--rm-white, #fff);
    }

    .rm-ach-content-header {
        display: flex;
        align-items: baseline;
        gap: clamp(0.5rem, 0.8vw, 1.2rem);
        margin-bottom: clamp(1rem, 1.5vw, 2.5rem);
    }

    /* ── Filter bar ── */

    .rm-ach-filters {
        display: flex;
        flex-direction: column;
        gap: clamp(0.5rem, 0.6vw, 1rem);
        margin-bottom: clamp(1rem, 1.2vw, 2rem);
        padding-bottom: clamp(0.8rem, 1vw, 1.6rem);
        border-bottom: 2px solid rgba(255, 255, 255, 0.08);
    }

    .rm-ach-filter-row {
        display: flex;
        align-items: center;
        gap: clamp(0.3rem, 0.5vw, 0.8rem);
        flex-wrap: wrap;
    }

    .rm-ach-filter-divider {
        width: 2px;
        height: clamp(1.2rem, 1.5vw, 2.4rem);
        background: rgba(255, 255, 255, 0.12);
        flex-shrink: 0;
        margin: 0 clamp(0.3rem, 0.5vw, 0.8rem);
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
        flex-shrink: 0;
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

    .rm-ach-empty {
        margin: clamp(1.5rem, 2vw, 3rem) 0;
        font-size: clamp(0.7rem, 0.65vw, 1.1rem);
        color: rgba(255, 255, 255, 0.35);
        text-transform: uppercase;
        letter-spacing: 0.08em;
        font-weight: 700;
    }

    /* ── Achievement grid ── */

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

    .rm-difficulty--beginner {
        opacity: 0.5;
    }
    .rm-difficulty--intermediate {
        opacity: 0.65;
    }
    .rm-difficulty--advanced {
        opacity: 0.8;
    }
    .rm-difficulty--expert {
        opacity: 0.9;
    }
    .rm-difficulty--legendary {
        color: var(--rm-red);
        opacity: 1;
    }

    .rm-achievement-desc {
        margin: 0;
        padding: clamp(0.25rem, 0.35vw, 0.6rem) clamp(0.7rem, 0.9vw, 1.6rem)
            clamp(0.25rem, 0.35vw, 0.6rem) clamp(1.2rem, 1.4vw, 2.4rem);
        font-size: clamp(0.65rem, 0.58vw, 1rem);
        color: rgba(255, 255, 255, 0.7);
        line-height: 1.4;
    }

    .rm-achievement-date {
        margin: 0;
        padding: 0 clamp(0.7rem, 0.9vw, 1.6rem) clamp(0.15rem, 0.2vw, 0.35rem)
            clamp(1.2rem, 1.4vw, 2.4rem);
        font-size: clamp(0.58rem, 0.5vw, 0.9rem);
        color: var(--rm-red);
        font-weight: 700;
        letter-spacing: 0.04em;
    }

    .rm-achievement-note {
        margin: 0;
        padding: 0 clamp(0.7rem, 0.9vw, 1.6rem) clamp(0.2rem, 0.25vw, 0.4rem)
            clamp(1.2rem, 1.4vw, 2.4rem);
        font-size: clamp(0.55rem, 0.48vw, 0.85rem);
        color: rgba(255, 255, 255, 0.45);
        font-style: italic;
    }

    .rm-achievement-prereqs {
        display: flex;
        flex-wrap: wrap;
        gap: clamp(0.2rem, 0.25vw, 0.4rem);
        padding: clamp(0.15rem, 0.2vw, 0.35rem) clamp(0.7rem, 0.9vw, 1.6rem)
            clamp(0.3rem, 0.4vw, 0.6rem) clamp(1.2rem, 1.4vw, 2.4rem);
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
</style>
