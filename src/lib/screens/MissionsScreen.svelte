<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import KeyHint from "$lib/KeyHint.svelte";
    import PhanSiteProgress from "$lib/PhanSiteProgress.svelte";
    import PromptWord from "$lib/PromptWord.svelte";
    import type {
        MissionData,
        MissionResponse,
        ProgressDisplay,
    } from "$lib/types/mission";

    let {
        onBack,
        missionProgress = null,
    }: { onBack: () => void; missionProgress?: ProgressDisplay | null } =
        $props();

    let loading = $state(false);
    let error = $state<string | null>(null);
    let missionData = $state<MissionData | null>(null);
    let sortIndex = $state(0);
    let selectedIndex = $state(0);
    let detailMission = $state<MissionResponse | null>(null);
    let rowRefs = $state<(HTMLElement | undefined)[]>([]);
    let scrollRef = $state<HTMLElement | undefined>(undefined);
    let scrollRatio = $state(0);
    let thumbRatio = $state(1);

    // Phan-Site mode state
    let phanMode = $state(false);
    let phanSelectedIndex = $state(0);
    let phanDetailMission = $state<MissionResponse | null>(null);

    type SortOption = { key: string; label: string };
    const SORT_OPTIONS: SortOption[] = [
        { key: "newest", label: "Pubtime" },
        { key: "status", label: "State" },
        { key: "progress", label: "Diffuculty" },
    ];

    // Carousel: compute visible order so active is always in the center
    // Returns [leftIndex, centerIndex, rightIndex]
    let sortCarousel = $derived.by(() => {
        const len = SORT_OPTIONS.length;
        const center = sortIndex;
        const left = (center - 1 + len) % len;
        const right = (center + 1) % len;
        return [left, center, right] as const;
    });

    const STATUS_ORDER: Record<string, number> = {
        active: 0,
        completed: 1,
        archived: 2,
    };

    const DIFFICULTY_ORDER: Record<string, number> = {
        S: 0,
        A: 1,
        B: 2,
        C: 3,
        D: 4,
        E: 5,
    };

    let proposedMissions = $derived(
        missionData?.missions.filter((m) => m.status === "proposed") ?? [],
    );

    let sortedMissions = $derived.by(() => {
        if (!missionData) return [];
        const list = missionData.missions.filter(
            (m) => m.status !== "proposed",
        );
        const opt = SORT_OPTIONS[sortIndex];
        switch (opt.key) {
            case "newest":
                return list.sort((a, b) =>
                    (b.created_at ?? "").localeCompare(a.created_at ?? ""),
                );
            case "status":
                return list.sort(
                    (a, b) =>
                        (STATUS_ORDER[a.status] ?? 9) -
                        (STATUS_ORDER[b.status] ?? 9),
                );
            case "progress":
                return list.sort(
                    (a, b) =>
                        (DIFFICULTY_ORDER[a.difficulty ?? ""] ?? 99) -
                        (DIFFICULTY_ORDER[b.difficulty ?? ""] ?? 99),
                );
            default:
                return list;
        }
    });

    // Clamp selectedIndex when list changes
    $effect(() => {
        const len = sortedMissions.length;
        if (len === 0) {
            selectedIndex = 0;
        } else if (selectedIndex >= len) {
            selectedIndex = len - 1;
        }
    });

    // Update scroll indicator when content changes
    $effect(() => {
        sortedMissions;
        requestAnimationFrame(() => updateScrollIndicator());
    });

    // Auto-scroll selected row into view
    $effect(() => {
        const el = rowRefs[selectedIndex];
        if (el) el.scrollIntoView({ block: "nearest", behavior: "smooth" });
    });

    function updateScrollIndicator() {
        if (!scrollRef) return;
        const max = scrollRef.scrollHeight - scrollRef.clientHeight;
        scrollRatio = max > 0 ? scrollRef.scrollTop / max : 0;
        thumbRatio =
            scrollRef.scrollHeight > 0
                ? Math.min(1, scrollRef.clientHeight / scrollRef.scrollHeight)
                : 1;
    }

    function cycleSort(dir: number) {
        sortIndex =
            (sortIndex + dir + SORT_OPTIONS.length) % SORT_OPTIONS.length;
        selectedIndex = 0;
        detailMission = null;
    }

    function openDetail(index: number) {
        selectedIndex = index;
        detailMission = sortedMissions[index] ?? null;
    }

    function closeDetail() {
        detailMission = null;
    }

    function difficultyGrade(difficulty?: string): string {
        return difficulty ?? "--";
    }

    function statusLabel(status: string): string {
        switch (status) {
            case "proposed":
                return "NEW!";
            case "active":
                return "ACTIVE";
            case "completed":
                return "CLEAR!";
            case "archived":
                return "ARCHIVED";
            default:
                return status.toUpperCase();
        }
    }

    let updating = $state(false);

    async function updateMissionStatus(id: string, newStatus: string) {
        updating = true;
        try {
            await invoke("update_mission_status", { id, newStatus });
            missionData = await invoke<MissionData>("load_missions");
            detailMission = null;
            phanDetailMission = null;
        } catch (e) {
            error = String(e);
        } finally {
            updating = false;
        }
    }

    async function refreshMissions() {
        loading = true;
        error = null;
        try {
            missionData = await invoke<MissionData>("load_missions");
        } catch (e) {
            error = String(e);
        } finally {
            loading = false;
        }
    }

    function togglePhanMode() {
        phanMode = !phanMode;
        phanSelectedIndex = 0;
        phanDetailMission = null;
        detailMission = null;
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            event.preventDefault();
            if (phanDetailMission) {
                phanDetailMission = null;
            } else if (phanMode) {
                phanMode = false;
            } else if (detailMission) {
                closeDetail();
            } else {
                onBack();
            }
            return;
        }
        if (event.key === "p" || event.key === "P") {
            event.preventDefault();
            togglePhanMode();
            return;
        }
        if (event.key === "Enter") {
            event.preventDefault();
            if (phanMode) {
                if (proposedMissions.length > 0) {
                    phanDetailMission =
                        proposedMissions[phanSelectedIndex] ?? null;
                }
            } else if (detailMission) {
                closeDetail();
            } else if (sortedMissions.length > 0) {
                openDetail(selectedIndex);
            }
            return;
        }
        if (event.key === "q" || event.key === "Q") {
            event.preventDefault();
            if (!phanMode) cycleSort(-1);
            return;
        }
        if (event.key === "e" || event.key === "E") {
            event.preventDefault();
            if (!phanMode) cycleSort(1);
            return;
        }
        if (event.key === "r" || event.key === "R") {
            event.preventDefault();
            void refreshMissions();
            return;
        }
        if (event.key === "ArrowDown") {
            event.preventDefault();
            if (phanMode) {
                phanDetailMission = null;
                phanSelectedIndex = Math.min(
                    phanSelectedIndex + 1,
                    proposedMissions.length - 1,
                );
            } else {
                detailMission = null;
                if (sortedMissions.length > 0) {
                    selectedIndex = Math.min(
                        selectedIndex + 1,
                        sortedMissions.length - 1,
                    );
                }
            }
            return;
        }
        if (event.key === "ArrowUp") {
            event.preventDefault();
            if (phanMode) {
                phanDetailMission = null;
                phanSelectedIndex = Math.max(phanSelectedIndex - 1, 0);
            } else {
                detailMission = null;
                selectedIndex = Math.max(selectedIndex - 1, 0);
            }
        }
    }

    onMount(() => {
        window.addEventListener("keydown", handleKeydown);

        async function load() {
            loading = true;
            error = null;
            try {
                missionData = await invoke<MissionData>("load_missions");
            } catch (e) {
                error = String(e);
            } finally {
                loading = false;
            }
        }

        void load();

        return () => {
            window.removeEventListener("keydown", handleKeydown);
        };
    });
</script>

<section class="rm-stage">
    <div class="rm-missions-bg-poly" aria-hidden="true"></div>

    <div class="rm-missions-panel">
        <!-- Sort prompts: Q shifts right-to-left, E shifts left-to-right, center is active -->
        <header class="rm-missions-sort-bar">
            <div class="rm-sort-side-label rm-sort-side-label--prev">
                <PromptWord
                    text={SORT_OPTIONS[sortCarousel[0]].label}
                    fontSize={44}
                />
            </div>
            <button
                type="button"
                class="rm-sort-key-btn rm-sort-key-btn--prev"
                onclick={() => cycleSort(-1)}
                aria-label="Previous sort"
            >
                <KeyHint key="Q" fontSize={30} />
            </button>
            <div class="rm-sort-current-label">
                <PromptWord
                    text={`Sort by ${SORT_OPTIONS[sortCarousel[1]].label}`}
                    fontSize={54}
                />
            </div>
            <button
                type="button"
                class="rm-sort-key-btn rm-sort-key-btn--next"
                onclick={() => cycleSort(1)}
                aria-label="Next sort"
            >
                <KeyHint key="E" fontSize={30} />
            </button>
            <div class="rm-sort-side-label rm-sort-side-label--next">
                <PromptWord
                    text={SORT_OPTIONS[sortCarousel[2]].label}
                    fontSize={44}
                />
            </div>
        </header>

        <!-- Column headers -->
        <div class="rm-missions-col-headers">
            <span class="rm-col-header rm-col-status">State</span>
            <span class="rm-col-header rm-col-name">Mission Name</span>
            <span class="rm-col-header rm-col-grade">Difficulty</span>
        </div>

        <!-- Mission list -->
        <div
            class="rm-missions-scroll"
            bind:this={scrollRef}
            onscroll={updateScrollIndicator}
        >
            {#if loading}
                <p class="state-text">Loading...</p>
            {:else if error}
                <p class="state-text error">{error}</p>
            {:else if phanMode}
                {#if proposedMissions.length > 0}
                    <ul class="rm-missions-list">
                        {#each proposedMissions as mission, i (mission.id)}
                            <li
                                class="rm-mission-row"
                                class:is-selected={phanSelectedIndex === i}
                                bind:this={rowRefs[i]}
                                onclick={() => {
                                    phanSelectedIndex = i;
                                    phanDetailMission = mission;
                                }}
                                onmouseenter={() => {
                                    phanSelectedIndex = i;
                                }}
                            >
                                <img
                                    class="rm-mission-stamp"
                                    src="/ui/mission_state/proposed.png"
                                    alt="proposed"
                                />
                                <span class="rm-mission-name"
                                    >{mission.title}</span
                                >
                                <span
                                    class="rm-mission-grade"
                                    data-grade={difficultyGrade(
                                        mission.difficulty,
                                    )}
                                >
                                    {difficultyGrade(mission.difficulty)}
                                </span>
                            </li>
                        {/each}
                    </ul>
                {:else}
                    <p class="state-text">No new requests.</p>
                {/if}
            {:else if sortedMissions.length > 0}
                <ul class="rm-missions-list">
                    {#each sortedMissions as mission, i (mission.id)}
                        <li
                            class="rm-mission-row"
                            class:is-selected={selectedIndex === i}
                            class:is-completed={mission.status === "completed"}
                            class:is-archived={mission.status === "archived"}
                            bind:this={rowRefs[i]}
                            onclick={() => openDetail(i)}
                            onmouseenter={() => {
                                selectedIndex = i;
                            }}
                        >
                            <img
                                class="rm-mission-stamp"
                                src="/ui/mission_state/{mission.status ===
                                'completed'
                                    ? 'clear'
                                    : mission.status}.png"
                                alt={mission.status}
                            />

                            <span class="rm-mission-name">{mission.title}</span>

                            <span
                                class="rm-mission-grade"
                                data-grade={difficultyGrade(mission.difficulty)}
                            >
                                {difficultyGrade(mission.difficulty)}
                            </span>
                        </li>
                    {/each}
                </ul>
            {:else}
                <p class="state-text">No missions yet.</p>
            {/if}
        </div>
    </div>

    <!-- Scroll indicator -->
    <div
        class="rm-missions-scroll-indicator"
        aria-hidden="true"
        style="--thumb-ratio: {thumbRatio}; --scroll-ratio: {scrollRatio};"
    >
        <div class="rm-missions-scroll-track">
            <div class="rm-missions-scroll-thumb"></div>
        </div>
    </div>

    <!-- Detail card overlay -->
    {#if detailMission}
        <div class="rm-detail-backdrop" onclick={closeDetail}></div>
        <article class="rm-detail-card">
            <div class="rm-detail-top">
                <span
                    class="rm-detail-stamp"
                    class:stamp-active={detailMission.status === "active"}
                    class:stamp-clear={detailMission.status === "completed"}
                >
                    {statusLabel(detailMission.status)}
                </span>
                <span
                    class="rm-detail-grade"
                    data-grade={difficultyGrade(detailMission.difficulty)}
                >
                    {difficultyGrade(detailMission.difficulty)}
                </span>
            </div>
            <h2 class="rm-detail-title">{detailMission.title}</h2>
            {#if detailMission.description}
                <p class="rm-detail-desc">{detailMission.description}</p>
            {/if}
            <div class="rm-detail-meta">
                {#if detailMission.progress != null}
                    <div class="rm-detail-progress-row">
                        <div class="rm-detail-track">
                            <div
                                class="rm-detail-fill"
                                style:width="{detailMission.progress}%"
                            ></div>
                        </div>
                        <span class="rm-detail-pct"
                            >{detailMission.progress}%</span
                        >
                    </div>
                {/if}
                {#if detailMission.days_remaining != null}
                    <span
                        class="rm-detail-deadline"
                        class:is-overdue={detailMission.days_remaining < 0}
                    >
                        {detailMission.days_remaining > 0
                            ? `${detailMission.days_remaining} DAYS LEFT`
                            : detailMission.days_remaining === 0
                              ? "DUE TODAY"
                              : "OVERDUE"}
                    </span>
                {/if}
            </div>
        </article>
    {/if}

    <!-- Phan detail card overlay (when in phan mode and a mission is selected) -->
    {#if phanDetailMission}
        <div
            class="rm-detail-backdrop"
            onclick={() => {
                phanDetailMission = null;
            }}
        ></div>
        <article class="rm-detail-card">
            <div class="rm-detail-top">
                <span class="rm-detail-stamp">NEW!</span>
                <span
                    class="rm-detail-grade"
                    data-grade={difficultyGrade(phanDetailMission.difficulty)}
                >
                    {difficultyGrade(phanDetailMission.difficulty)}
                </span>
            </div>
            <h2 class="rm-detail-title">{phanDetailMission.title}</h2>
            {#if phanDetailMission.description}
                <p class="rm-detail-desc">{phanDetailMission.description}</p>
            {/if}
            <div class="rm-detail-meta">
                {#if phanDetailMission.days_remaining != null}
                    <span
                        class="rm-detail-deadline"
                        class:is-overdue={phanDetailMission.days_remaining < 0}
                    >
                        {phanDetailMission.days_remaining > 0
                            ? `${phanDetailMission.days_remaining} DAYS LEFT`
                            : phanDetailMission.days_remaining === 0
                              ? "DUE TODAY"
                              : "OVERDUE"}
                    </span>
                {/if}
            </div>
            <div class="rm-detail-actions">
                <button
                    class="rm-action-btn rm-action-accept"
                    disabled={updating}
                    onclick={() =>
                        updateMissionStatus(phanDetailMission!.id, "active")}
                    >ACCEPT</button
                >
                <button
                    class="rm-action-btn rm-action-reject"
                    disabled={updating}
                    onclick={() =>
                        updateMissionStatus(phanDetailMission!.id, "rejected")}
                    >REJECT</button
                >
            </div>
        </article>
    {/if}

    <!-- P key: toggle phan mode -->
    <button type="button" class="rm-phan-mode-btn" onclick={togglePhanMode}>
        <KeyHint key="P" fontSize={36} />
        <PromptWord text={phanMode ? "tracked" : "phansite"} fontSize={72} />
    </button>

    {#if missionProgress}
        <PhanSiteProgress
            question={missionProgress.label}
            progress={missionProgress.progress}
            placement="missions"
        />
    {/if}

    <button
        type="button"
        class="rm-back-btn rm-back-btn--missions"
        onclick={() => onBack()}
    >
        <KeyHint key="Esc" fontSize={36} />
        <PromptWord text="Back" fontSize={72} />
    </button>
</section>

<style>
    :global(.rm-stage) {
        --missions-bg-clip: polygon(
            40% 0%,
            93% 0%,
            100% 25%,
            100% 90%,
            50% 100%,
            42% 100%,
            20% 15%
        );
    }

    .rm-missions-bg-poly {
        position: absolute;
        inset: 0;
        z-index: 0;
        pointer-events: none;
        background: #000000;
        clip-path: var(--missions-bg-clip);
    }

    /* ── Panel ── */
    .rm-missions-panel {
        --missions-content-left: clamp(12rem, 16vw, 40rem);
        --missions-content-right: clamp(3rem, 6vw, 15rem);
        --missions-content-width: min(
            115rem,
            calc(
                100% - var(--missions-content-left) -
                    var(--missions-content-right)
            )
        );
        --mission-status-col: clamp(12rem, 18%, 20rem);
        --mission-grade-col: clamp(16rem, 22%, 28rem);
        --mission-grid-columns:
            var(--mission-status-col) minmax(0, 1fr)
            var(--mission-grade-col);
        position: absolute;
        top: 0;
        right: 0;
        width: 75%;
        height: 100%;
        transform-origin: bottom right;
        transform: rotate(-3deg);
        display: flex;
        flex-direction: column;
        overflow: hidden;
        background: transparent;
        z-index: 1;
    }

    /* ── Sort prompts ── */
    .rm-missions-sort-bar {
        --sort-side-slot: clamp(8rem, 10vw, 13rem);
        --sort-current-slot: clamp(18rem, 24vw, 30rem);
        flex-shrink: 0;
        display: grid;
        grid-template-columns:
            var(--sort-side-slot) auto var(--sort-current-slot) auto
            var(--sort-side-slot);
        align-items: center;
        justify-content: center;
        background: transparent;
        font-family:
            "Source Han Sans SC", "Noto Sans SC", "方正兰亭黑_GBK", Arial,
            sans-serif;
        font-weight: 600;
        padding: clamp(0.25rem, 0.4vw, 0.6rem) clamp(1rem, 1.2vw, 2rem);
        column-gap: clamp(0.2rem, 0.45vw, 0.7rem);
        transform: translate(-30rem, 2rem) rotate(-2deg);
    }

    .rm-sort-side-label,
    .rm-sort-current-label {
        display: flex;
        align-items: center;
        justify-content: center;
        min-width: 0;
        pointer-events: none;
    }

    .rm-sort-side-label--prev {
        justify-content: flex-end;
    }

    .rm-sort-side-label--next {
        justify-content: flex-start;
    }

    .rm-sort-key-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        border: none;
        padding: 0;
        background: transparent;
        cursor: pointer;
        transition: transform 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    .rm-sort-key-btn:hover {
        transform: scale(1.08) rotate(-2deg);
    }

    .rm-sort-key-btn:focus-visible {
        outline: 0.16rem solid #ffffff;
        outline-offset: 0.16rem;
    }

    .rm-sort-side-label :global(.p5-prompt-word) {
        max-width: 100%;
        height: auto;
    }

    .rm-sort-current-label :global(.p5-prompt-word) {
        max-width: 100%;
        height: auto;
    }

    /* ── Column headers ── */
    .rm-missions-col-headers {
        flex-shrink: 0;
        position: relative;
        display: grid;
        grid-template-columns: var(--mission-grid-columns);
        width: var(--missions-content-width);
        height: clamp(2.5rem, 3.2vw, 4rem);
        margin-left: var(--missions-content-left);
        padding: 0;
        background: transparent;
        font-family:
            "Source Han Sans SC", "Noto Sans SC", "方正兰亭黑_GBK", Arial,
            sans-serif;
        font-weight: 600;
    }

    .rm-col-header {
        position: relative;
        width: 100%;
        height: var(--col-h);
        display: flex;
        align-items: center;
        justify-content: center;
        box-sizing: border-box;
        padding: 0 var(--col-pad-x);
        background: #ffffff;
        color: #000000;
        font-size: var(--col-font-size);
        font-weight: 900;
        letter-spacing: 0;
        line-height: 1;
        white-space: nowrap;
        transform: rotate(var(--col-rot));
        transform-origin: center;
    }

    .rm-col-status {
        --col-h: clamp(3.5rem, 7vw, 7rem);
        --col-rot: -1deg;
        --col-font-size: clamp(1.6rem, 2.5vw, 5rem);
        --col-pad-x: clamp(0.5rem, 0.7vw, 1rem);
        top: clamp(2rem, 4vw, 4rem);
    }

    .rm-col-name {
        --col-h: clamp(3.5rem, 7vw, 7rem);
        --col-rot: -2deg;
        --col-font-size: clamp(1.6rem, 2.5vw, 5rem);
        --col-pad-x: clamp(0.7rem, 0.9vw, 1.2rem);
        top: clamp(1rem, 2vw, 2rem);
    }

    .rm-col-grade {
        --col-h: clamp(3.5rem, 7vw, 7rem);
        --col-rot: -3deg;
        --col-font-size: clamp(1.6rem, 2.5vw, 5rem);
        --col-pad-x: clamp(0.5rem, 0.7vw, 1rem);
        top: clamp(1rem, 2vw, 2rem);
        text-align: center;
    }

    /* ── Scroll area ── */
    .rm-missions-scroll {
        flex: 1;
        overflow-x: visible;
        overflow-y: auto;
        scrollbar-width: none;
    }

    .rm-missions-scroll::-webkit-scrollbar {
        display: none;
    }

    /* ── Custom scroll indicator ── */
    .rm-missions-scroll-indicator {
        position: absolute;
        top: 20vh;
        right: clamp(10rem, 20vw, 20rem);
        transform: translate(0, 0) rotate(-10deg);
        transform-origin: left top;
        z-index: 10;
        pointer-events: none;
        height: 30vh;
        width: 28px;
    }

    .rm-missions-scroll-track {
        position: relative;
        width: 100%;
        height: 100%;
        background: var(--rm-black);
        border: 4px solid var(--rm-white);
        box-sizing: border-box;
    }

    .rm-missions-scroll-thumb {
        position: absolute;
        left: 0;
        right: 0;
        height: calc(var(--thumb-ratio, 1) * 100%);
        top: calc(
            var(--scroll-ratio, 0) * (100% - var(--thumb-ratio, 1) * 100%)
        );
        background: var(--rm-white);
    }

    .rm-missions-list {
        list-style: none;
        margin: 0;
        margin-left: var(--missions-content-left);
        width: var(--missions-content-width);
        padding: 0;
        padding-bottom: 4rem;
        transform: translateY(10rem);
        display: flex;
        flex-direction: column;
        gap: 0;
    }

    /* ── Mission rows ── */
    .rm-mission-row {
        display: grid;
        grid-template-columns: var(--mission-grid-columns);
        width: 100%;
        column-gap: 0;
        align-items: center;
        height: 7rem;
        padding: 0;
        cursor: pointer;
        transition:
            color 100ms ease,
            transform 100ms ease;
        clip-path: polygon(0% 4%, 100% 0%, 100% 96%, 0% 100%);
        position: relative;
    }

    .rm-mission-row::before {
        content: "";
        position: absolute;
        inset: 0;
        left: -3rem;
        right: -5rem;
        background: transparent;
        clip-path: polygon(1% 30%, 100% 10%, 95% 100%, 3% 90%);
        pointer-events: none;
        z-index: -1;
        transition: background 100ms ease;
    }

    .rm-mission-row.is-selected {
        background: transparent;
        transform: scaleY(1.08);
        clip-path: none;
        z-index: 2;
    }

    .rm-mission-row.is-selected::before {
        background: #e5191c;
    }

    .rm-mission-row.is-completed {
        opacity: 0.55;
    }

    .rm-mission-row.is-archived {
        opacity: 0.3;
    }

    .rm-mission-row.is-selected.is-completed,
    .rm-mission-row.is-selected.is-archived {
        opacity: 1;
    }

    /* ── Status stamp ── */
    .rm-mission-stamp {
        display: block;
        width: 80%;
        height: 80%;
        object-fit: contain;
    }

    .rm-mission-row.is-completed .rm-mission-stamp {
        opacity: 0.9;
    }

    .rm-mission-row.is-selected .rm-mission-stamp {
        opacity: 1;
    }

    /* ── Mission name ── */
    .rm-mission-name {
        min-width: 0;
        font-family:
            "Source Han Sans SC", "Noto Sans SC", "方正兰亭黑_GBK", Arial,
            sans-serif;

        font-size: 3rem;
        font-weight: 1000;
        color: #ffffff;
        letter-spacing: 0.03em;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    /* ── Grade letter ── */
    .rm-mission-grade {
        text-align: center;
        font-family:
            "Source Han Sans SC", "Noto Sans SC", "方正兰亭黑_GBK", Arial,
            sans-serif;
        font-size: 8rem;
        font-weight: 900;
        color: #ffffff;
        background: none;
        padding: 0;
        clip-path: none;
        line-height: 1;
        overflow: hidden;
    }

    .rm-mission-grade[data-grade="S"] {
        color: #e5191c;
    }

    .rm-mission-grade[data-grade="--"] {
        font-size: clamp(0.9rem, 1vw, 1.5rem);
        opacity: 0.3;
    }

    .rm-mission-row.is-selected .rm-mission-grade {
        color: #ffffff;
    }

    /* ── Detail card overlay ── */
    .rm-detail-backdrop {
        position: absolute;
        inset: 0;
        z-index: 20;
        background: rgba(0, 0, 0, 0.5);
    }

    .rm-detail-card {
        position: absolute;
        top: 50%;
        left: 55%;
        transform: translate(-50%, -50%) rotate(-1.5deg);
        z-index: 21;
        width: clamp(280px, 32vw, 520px);
        background: #000000;
        border: 3px solid rgba(255, 255, 255, 0.15);
        clip-path: polygon(0% 2%, 100% 0%, 100% 98%, 0% 100%);
        display: flex;
        flex-direction: column;
        font-family:
            "Source Han Sans SC", "Noto Sans SC", "方正兰亭黑_GBK", Arial,
            sans-serif;
        font-weight: 600;
        animation: rm-detail-pop 180ms ease-out;
    }

    @keyframes rm-detail-pop {
        from {
            opacity: 0;
            transform: translate(-50%, -46%) rotate(-1.5deg) scale(0.92);
        }
        to {
            opacity: 1;
            transform: translate(-50%, -50%) rotate(-1.5deg) scale(1);
        }
    }

    .rm-detail-top {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: clamp(0.5rem, 0.6vw, 0.9rem) clamp(0.8rem, 1vw, 1.5rem);
        background: #e5191c;
        clip-path: polygon(0% 0%, 100% 0%, 100% 85%, 0% 100%);
    }

    .rm-detail-stamp {
        font-size: clamp(0.7rem, 0.75vw, 1.1rem);
        font-weight: 900;
        font-style: italic;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: #ffffff;
    }

    .rm-detail-grade {
        font-size: clamp(1.6rem, 2vw, 2.8rem);
        font-weight: 900;
        color: #000000;
        background: #ffffff;
        padding: 0 clamp(0.4rem, 0.5vw, 0.8rem);
        line-height: 1.2;
        clip-path: polygon(6% 0%, 100% 5%, 94% 100%, 0% 95%);
    }

    .rm-detail-grade[data-grade="S"] {
        color: #e5191c;
    }

    .rm-detail-grade[data-grade="--"] {
        font-size: clamp(1rem, 1.2vw, 1.8rem);
        opacity: 0.4;
    }

    .rm-detail-title {
        margin: 0;
        padding: clamp(0.6rem, 0.8vw, 1.2rem) clamp(0.8rem, 1vw, 1.5rem)
            clamp(0.3rem, 0.4vw, 0.6rem);
        font-size: clamp(1rem, 1.2vw, 1.8rem);
        font-weight: 900;
        color: #ffffff;
        letter-spacing: 0.03em;
        line-height: 1.3;
    }

    .rm-detail-desc {
        margin: 0;
        padding: 0 clamp(0.8rem, 1vw, 1.5rem) clamp(0.6rem, 0.7vw, 1rem);
        font-size: clamp(0.7rem, 0.65vw, 1rem);
        font-weight: 600;
        color: rgba(255, 255, 255, 0.6);
        line-height: 1.5;
    }

    .rm-detail-meta {
        display: flex;
        flex-direction: column;
        gap: clamp(0.3rem, 0.4vw, 0.6rem);
        padding: clamp(0.5rem, 0.6vw, 0.9rem) clamp(0.8rem, 1vw, 1.5rem)
            clamp(0.7rem, 0.8vw, 1.2rem);
        border-top: 1px solid rgba(255, 255, 255, 0.08);
        margin-top: auto;
    }

    .rm-detail-progress-row {
        display: flex;
        align-items: center;
        gap: clamp(0.4rem, 0.5vw, 0.8rem);
    }

    .rm-detail-track {
        flex: 1;
        height: clamp(6px, 0.5vw, 10px);
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.15);
        overflow: hidden;
    }

    .rm-detail-fill {
        height: 100%;
        background: #e5191c;
        transition: width 300ms ease;
    }

    .rm-detail-pct {
        font-size: clamp(0.7rem, 0.7vw, 1.1rem);
        font-weight: 800;
        color: rgba(255, 255, 255, 0.6);
        flex-shrink: 0;
    }

    .rm-detail-deadline {
        font-size: clamp(0.6rem, 0.6vw, 0.9rem);
        font-weight: 800;
        letter-spacing: 0.06em;
        color: #e5191c;
    }

    .rm-detail-deadline.is-overdue {
        color: rgba(255, 80, 80, 0.9);
    }

    /* ── Detail action buttons (proposed) ── */
    .rm-detail-actions {
        display: flex;
        gap: clamp(0.5rem, 0.6vw, 1rem);
        padding: clamp(0.5rem, 0.6vw, 0.9rem) clamp(0.8rem, 1vw, 1.5rem)
            clamp(0.7rem, 0.8vw, 1.2rem);
        border-top: 1px solid rgba(255, 255, 255, 0.08);
    }

    .rm-action-btn {
        flex: 1;
        font-family:
            "Source Han Sans SC", "Noto Sans SC", "方正兰亭黑_GBK", Arial,
            sans-serif;
        font-size: clamp(0.75rem, 0.8vw, 1.1rem);
        font-weight: 900;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        padding: clamp(0.4rem, 0.5vw, 0.7rem) 0;
        border: none;
        cursor: pointer;
        clip-path: polygon(2% 0%, 100% 4%, 98% 100%, 0% 96%);
        transition:
            opacity 120ms ease,
            transform 120ms ease;
    }

    .rm-action-btn:hover {
        transform: scale(1.03);
    }

    .rm-action-btn:disabled {
        opacity: 0.4;
        cursor: not-allowed;
        transform: none;
    }

    .rm-action-accept {
        background: #e5191c;
        color: #ffffff;
    }

    .rm-action-reject {
        background: rgba(255, 255, 255, 0.1);
        color: rgba(255, 255, 255, 0.5);
    }

    .rm-action-reject:hover {
        background: rgba(255, 255, 255, 0.15);
        color: rgba(255, 255, 255, 0.7);
    }

    .rm-back-btn--missions {
        left: auto;
        right: clamp(1.5rem, 3vw, 4rem);
    }

    /* ── P key: phan mode toggle button ── */
    .rm-phan-mode-btn {
        position: fixed;
        top: clamp(0rem, 0vh, 0rem);
        right: clamp(10rem, 18vw, 20rem);
        z-index: 10;
        display: flex;
        align-items: center;
        gap: 0;
        background: none;
        border: none;
        cursor: pointer;
        padding: 0;
        transform: rotate(-4deg);
        transition: transform 120ms ease;
    }

    .rm-phan-mode-btn:hover {
        transform: rotate(-5deg) scale(1.06);
    }

    .rm-phan-mode-btn :global(.p5-prompt-word) {
        margin-left: -1rem;
    }
</style>
