<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import KeyHint from "$lib/KeyHint.svelte";
    import PhanSiteProgress from "$lib/PhanSiteProgress.svelte";
    import PromptWord from "$lib/PromptWord.svelte";
    import type {
        Mission,
        DashboardMissionSlot,
        MissionDashboardData,
        MissionMenuDashboardData,
        MissionSuggestion,
    } from "$lib/types/mission";
    import { dataCommandErrorMessage } from "$lib/types/data_platform";

    let {
        onBack,
        missionMenuData = null,
        onMissionMenuDataLoaded,
    }: {
        onBack: () => void;
        missionMenuData?: MissionMenuDashboardData | null;
        onMissionMenuDataLoaded?: (data: MissionMenuDashboardData) => void;
    } = $props();

    let loading = $state(false);
    let error = $state<string | null>(null);
    let missionData = $state<MissionDashboardData | null>(null);
    let sortIndex = $state(0);
    let selectedIndex = $state(0);
    let detailMission = $state<Mission | null>(null);
    let rowRefs = $state<(HTMLElement | undefined)[]>([]);
    let scrollRef = $state<HTMLElement | undefined>(undefined);
    let scrollRatio = $state(0);
    let thumbRatio = $state(1);

    // Phan-Site mode state
    let phanMode = $state(false);
    let phanSelectedIndex = $state(0);
    let phanDetailMission = $state<MissionSuggestion | null>(null);

    type SortOption = { key: string; label: string };
    const SORT_OPTIONS: SortOption[] = [
        { key: "newest", label: "Pubtime" },
        { key: "status", label: "State" },
        { key: "difficulty", label: "Difficulty" },
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
    };

    let proposedMissions = $derived(
        [...(missionData?.suggestions ?? [])].sort((a, b) =>
            b.generated_at.localeCompare(a.generated_at),
        ),
    );

    let sortedMissions = $derived.by(() => {
        if (!missionData) return [];
        const list = [...missionData.missions];
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
            case "difficulty":
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
        actionError = null;
    }

    function closeDetail() {
        detailMission = null;
        actionError = null;
    }

    function difficultyGrade(difficulty?: string): string {
        return difficulty ?? "--";
    }

    function statusLabel(status: string): string {
        switch (status) {
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
    let actionError = $state<string | null>(null);

    const DASHBOARD_SLOTS: {
        slot: DashboardMissionSlot;
        label: string;
    }[] = [
        { slot: "countdown", label: "Countdown" },
        { slot: "progress", label: "Progress" },
        { slot: "hint_1", label: "Hint 1" },
        { slot: "hint_2", label: "Hint 2" },
    ];

    async function refreshAfterMutation() {
        const [dashboard, menuDashboard] = await Promise.all([
            invoke<MissionDashboardData>("load_mission_dashboard"),
            invoke<MissionMenuDashboardData>("load_mission_menu_dashboard"),
        ]);
        missionData = dashboard;
        onMissionMenuDataLoaded?.(menuDashboard);
        detailMission = null;
        phanDetailMission = null;
    }

    async function runMissionCommand(
        command: "complete_mission" | "archive_mission",
        missionId: string,
    ) {
        updating = true;
        actionError = null;
        try {
            await invoke(command, { missionId });
            await refreshAfterMutation();
        } catch (e) {
            actionError = dataCommandErrorMessage(
                e,
                "Failed to update Mission.",
            );
        } finally {
            updating = false;
        }
    }

    async function runSuggestionCommand(
        command:
            | "accept_mission_suggestion"
            | "reject_mission_suggestion",
        suggestionId: string,
    ) {
        updating = true;
        actionError = null;
        try {
            await invoke(command, { suggestionId });
            await refreshAfterMutation();
        } catch (e) {
            actionError = dataCommandErrorMessage(
                e,
                "Failed to update Mission suggestion.",
            );
        } finally {
            updating = false;
        }
    }

    function isDashboardSlotSelected(
        slot: DashboardMissionSlot,
        missionId: string,
    ): boolean {
        return missionMenuData?.selections[slot]?.mission_id === missionId;
    }

    async function toggleDashboardSlot(
        slot: DashboardMissionSlot,
        mission: Mission,
    ) {
        updating = true;
        actionError = null;
        try {
            if (isDashboardSlotSelected(slot, mission.id)) {
                await invoke("clear_mission_dashboard_slot", { slot });
            } else {
                await invoke("select_mission_dashboard_slot", {
                    slot,
                    missionId: mission.id,
                    label: null,
                });
            }
            const menuDashboard = await invoke<MissionMenuDashboardData>(
                "load_mission_menu_dashboard",
            );
            onMissionMenuDataLoaded?.(menuDashboard);
        } catch (e) {
            actionError = dataCommandErrorMessage(
                e,
                "Failed to update Dashboard selection.",
            );
        } finally {
            updating = false;
        }
    }

    async function refreshMissions() {
        loading = true;
        error = null;
        try {
            const [dashboard, menuDashboard] = await Promise.all([
                invoke<MissionDashboardData>("load_mission_dashboard"),
                invoke<MissionMenuDashboardData>(
                    "load_mission_menu_dashboard",
                ),
            ]);
            missionData = dashboard;
            onMissionMenuDataLoaded?.(menuDashboard);
        } catch (e) {
            error = dataCommandErrorMessage(e, "Failed to load Missions.");
        } finally {
            loading = false;
        }
    }

    function togglePhanMode() {
        phanMode = !phanMode;
        phanSelectedIndex = 0;
        phanDetailMission = null;
        detailMission = null;
        actionError = null;
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
                if (proposedMissions.length > 0) {
                    phanSelectedIndex = Math.min(
                        phanSelectedIndex + 1,
                        proposedMissions.length - 1,
                    );
                }
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
                missionData = await invoke<MissionDashboardData>(
                    "load_mission_dashboard",
                );
            } catch (e) {
                error = dataCommandErrorMessage(
                    e,
                    "Failed to load Missions.",
                );
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

<section class="arcana-stage">
    <div class="arcana-missions-bg-poly" aria-hidden="true"></div>

    <div class="arcana-missions-panel">
        <!-- Sort prompts: Q shifts right-to-left, E shifts left-to-right, center is active -->
        <header class="arcana-missions-sort-bar">
            <div class="arcana-sort-side-label arcana-sort-side-label--prev">
                <PromptWord
                    text={SORT_OPTIONS[sortCarousel[0]].label}
                    fontSize={44}
                />
            </div>
            <button
                type="button"
                class="arcana-sort-key-btn arcana-sort-key-btn--prev"
                onclick={() => cycleSort(-1)}
                aria-label="Previous sort"
            >
                <KeyHint key="Q" fontSize={30} />
            </button>
            <div class="arcana-sort-current-label">
                <PromptWord
                    text={`Sort by ${SORT_OPTIONS[sortCarousel[1]].label}`}
                    fontSize={54}
                />
            </div>
            <button
                type="button"
                class="arcana-sort-key-btn arcana-sort-key-btn--next"
                onclick={() => cycleSort(1)}
                aria-label="Next sort"
            >
                <KeyHint key="E" fontSize={30} />
            </button>
            <div class="arcana-sort-side-label arcana-sort-side-label--next">
                <PromptWord
                    text={SORT_OPTIONS[sortCarousel[2]].label}
                    fontSize={44}
                />
            </div>
        </header>

        <!-- Column headers -->
        <div class="arcana-missions-col-headers">
            <span class="arcana-col-header arcana-col-status">State</span>
            <span class="arcana-col-header arcana-col-name">Mission Name</span>
            <span class="arcana-col-header arcana-col-grade">Difficulty</span>
        </div>

        <!-- Mission list -->
        <div
            class="arcana-missions-scroll"
            bind:this={scrollRef}
            onscroll={updateScrollIndicator}
        >
            {#if loading}
                <p class="state-text">Loading...</p>
            {:else if error}
                <p class="state-text error">{error}</p>
            {:else if phanMode}
                {#if proposedMissions.length > 0}
                    <div class="arcana-missions-list">
                        {#each proposedMissions as mission, i (mission.id)}
                            <button
                                type="button"
                                class="arcana-mission-row"
                                class:is-selected={phanSelectedIndex === i}
                                bind:this={rowRefs[i]}
                                onclick={() => {
                                    phanSelectedIndex = i;
                                    phanDetailMission = mission;
                                    actionError = null;
                                }}
                                onmouseenter={() => {
                                    phanSelectedIndex = i;
                                }}
                                onkeydown={(event) => {
                                    if (
                                        event.key === "Enter" ||
                                        event.key === " "
                                    ) {
                                        phanSelectedIndex = i;
                                        phanDetailMission = mission;
                                        actionError = null;
                                    }
                                }}
                            >
                                <img
                                    class="arcana-mission-stamp"
                                    src="/ui/mission_state/proposed.png"
                                    alt="proposed"
                                />
                                <span class="arcana-mission-name"
                                    >{mission.title}</span
                                >
                                <span
                                    class="arcana-mission-grade"
                                    data-grade={difficultyGrade(
                                        mission.difficulty,
                                    )}
                                >
                                    {difficultyGrade(mission.difficulty)}
                                </span>
                            </button>
                        {/each}
                    </div>
                {:else}
                    <p class="state-text">No new requests.</p>
                {/if}
            {:else if sortedMissions.length > 0}
                <div class="arcana-missions-list">
                    {#each sortedMissions as mission, i (mission.id)}
                        <button
                            type="button"
                            class="arcana-mission-row"
                            class:is-selected={selectedIndex === i}
                            class:is-completed={mission.status === "completed"}
                            class:is-archived={mission.status === "archived"}
                            bind:this={rowRefs[i]}
                            onclick={() => openDetail(i)}
                            onmouseenter={() => {
                                selectedIndex = i;
                            }}
                            onkeydown={(event) => {
                                if (
                                    event.key === "Enter" ||
                                    event.key === " "
                                )
                                    openDetail(i);
                            }}
                        >
                            <img
                                class="arcana-mission-stamp"
                                src="/ui/mission_state/{mission.status ===
                                'completed'
                                    ? 'clear'
                                    : mission.status}.png"
                                alt={mission.status}
                            />

                            <span class="arcana-mission-name">{mission.title}</span>

                            <span
                                class="arcana-mission-grade"
                                data-grade={difficultyGrade(mission.difficulty)}
                            >
                                {difficultyGrade(mission.difficulty)}
                            </span>
                        </button>
                    {/each}
                </div>
            {:else}
                <p class="state-text">No missions yet.</p>
            {/if}
        </div>
    </div>

    <!-- Scroll indicator -->
    <div
        class="arcana-missions-scroll-indicator"
        aria-hidden="true"
        style="--thumb-ratio: {thumbRatio}; --scroll-ratio: {scrollRatio};"
    >
        <div class="arcana-missions-scroll-track">
            <div class="arcana-missions-scroll-thumb"></div>
        </div>
    </div>

    <!-- Detail card overlay -->
    {#if detailMission}
        <button
            type="button"
            class="arcana-detail-backdrop"
            aria-label="Close Mission details"
            onclick={closeDetail}
        ></button>
        <article class="arcana-detail-card">
            <div class="arcana-detail-top">
                <span
                    class="arcana-detail-stamp"
                    class:stamp-active={detailMission.status === "active"}
                    class:stamp-clear={detailMission.status === "completed"}
                >
                    {statusLabel(detailMission.status)}
                </span>
                <span
                    class="arcana-detail-grade"
                    data-grade={difficultyGrade(detailMission.difficulty)}
                >
                    {difficultyGrade(detailMission.difficulty)}
                </span>
            </div>
            <h2 class="arcana-detail-title">{detailMission.title}</h2>
            {#if detailMission.description}
                <p class="arcana-detail-desc">{detailMission.description}</p>
            {/if}
            <div class="arcana-detail-meta">
                {#if detailMission.progress != null}
                    <div class="arcana-detail-progress-row">
                        <div class="arcana-detail-track">
                            <div
                                class="arcana-detail-fill"
                                style:width="{detailMission.progress}%"
                            ></div>
                        </div>
                        <span class="arcana-detail-pct"
                            >{detailMission.progress}%</span
                        >
                    </div>
                {/if}
                {#if detailMission.days_remaining != null}
                    <span
                        class="arcana-detail-deadline"
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
            <div class="arcana-dashboard-slots">
                <span class="arcana-dashboard-slots-label">Main menu</span>
                <div class="arcana-dashboard-slot-buttons">
                    {#each DASHBOARD_SLOTS as item (item.slot)}
                        {@const selected = isDashboardSlotSelected(
                            item.slot,
                            detailMission.id,
                        )}
                        {@const unavailable =
                            !selected &&
                            (detailMission.status !== "active" ||
                                (item.slot === "countdown" &&
                                    !detailMission.deadline))}
                        <button
                            type="button"
                            class="arcana-dashboard-slot-btn"
                            class:is-selected={selected}
                            disabled={updating || unavailable}
                            title={item.slot === "countdown" &&
                            !detailMission.deadline
                                ? "Countdown requires a deadline"
                                : item.label}
                            onclick={() =>
                                toggleDashboardSlot(
                                    item.slot,
                                    detailMission!,
                                )}
                        >
                            {selected ? "✓ " : ""}{item.label}
                        </button>
                    {/each}
                </div>
            </div>
            {#if detailMission.status !== "archived"}
                <div class="arcana-detail-actions">
                    {#if detailMission.status === "active"}
                        <button
                            class="arcana-action-btn arcana-action-accept"
                            disabled={updating}
                            onclick={() =>
                                runMissionCommand(
                                    "complete_mission",
                                    detailMission!.id,
                                )}>COMPLETE</button
                        >
                    {/if}
                    <button
                        class="arcana-action-btn arcana-action-reject"
                        disabled={updating}
                        onclick={() =>
                            runMissionCommand(
                                "archive_mission",
                                detailMission!.id,
                            )}>ARCHIVE</button
                    >
                </div>
            {/if}
            {#if actionError}
                <p class="arcana-detail-action-error">{actionError}</p>
            {/if}
        </article>
    {/if}

    <!-- Phan detail card overlay (when in phan mode and a mission is selected) -->
    {#if phanDetailMission}
        <button
            type="button"
            class="arcana-detail-backdrop"
            aria-label="Close Mission suggestion"
            onclick={() => {
                phanDetailMission = null;
                actionError = null;
            }}
        ></button>
        <article class="arcana-detail-card">
            <div class="arcana-detail-top">
                <span class="arcana-detail-stamp">NEW!</span>
                <span
                    class="arcana-detail-grade"
                    data-grade={difficultyGrade(phanDetailMission.difficulty)}
                >
                    {difficultyGrade(phanDetailMission.difficulty)}
                </span>
            </div>
            <h2 class="arcana-detail-title">{phanDetailMission.title}</h2>
            {#if phanDetailMission.description}
                <p class="arcana-detail-desc">{phanDetailMission.description}</p>
            {/if}
            {#if phanDetailMission.reason}
                <p class="arcana-detail-reason">
                    <strong>WHY THIS MISSION</strong>
                    {phanDetailMission.reason}
                </p>
            {/if}
            <div class="arcana-detail-meta">
                {#if phanDetailMission.days_remaining != null}
                    <span
                        class="arcana-detail-deadline"
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
            <div class="arcana-detail-actions">
                <button
                    class="arcana-action-btn arcana-action-accept"
                    disabled={updating}
                    onclick={() =>
                        runSuggestionCommand(
                            "accept_mission_suggestion",
                            phanDetailMission!.id,
                        )}
                    >ACCEPT</button
                >
                <button
                    class="arcana-action-btn arcana-action-reject"
                    disabled={updating}
                    onclick={() =>
                        runSuggestionCommand(
                            "reject_mission_suggestion",
                            phanDetailMission!.id,
                        )}
                    >REJECT</button
                >
            </div>
            {#if actionError}
                <p class="arcana-detail-action-error">{actionError}</p>
            {/if}
        </article>
    {/if}

    <!-- P key: toggle phan mode -->
    <button type="button" class="arcana-phan-mode-btn" onclick={togglePhanMode}>
        <KeyHint key="P" fontSize={36} />
        <PromptWord text={phanMode ? "tracked" : "phansite"} fontSize={72} />
    </button>

    {#if missionMenuData?.progress}
        <PhanSiteProgress
            question={missionMenuData.progress.label}
            progress={missionMenuData.progress.progress}
            placement="missions"
        />
    {/if}

    <button
        type="button"
        class="arcana-back-btn arcana-back-btn--missions"
        onclick={() => onBack()}
    >
        <KeyHint key="Esc" fontSize={36} />
        <PromptWord text="Back" fontSize={72} />
    </button>
</section>

<style>
    :global(.arcana-stage) {
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

    .arcana-missions-bg-poly {
        position: absolute;
        inset: 0;
        z-index: 0;
        pointer-events: none;
        background: #000000;
        clip-path: var(--missions-bg-clip);
    }

    /* ── Panel ── */
    .arcana-missions-panel {
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
        --mission-grid-columns: var(--mission-status-col) minmax(0, 1fr)
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
    .arcana-missions-sort-bar {
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

    .arcana-sort-side-label,
    .arcana-sort-current-label {
        display: flex;
        align-items: center;
        justify-content: center;
        min-width: 0;
        pointer-events: none;
    }

    .arcana-sort-side-label--prev {
        justify-content: flex-end;
    }

    .arcana-sort-side-label--next {
        justify-content: flex-start;
    }

    .arcana-sort-key-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        border: none;
        padding: 0;
        background: transparent;
        cursor: pointer;
        transition: transform 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    .arcana-sort-key-btn:hover {
        transform: scale(1.08) rotate(-2deg);
    }

    .arcana-sort-key-btn:focus-visible {
        outline: 0.16rem solid #ffffff;
        outline-offset: 0.16rem;
    }

    .arcana-sort-side-label :global(.p5-prompt-word) {
        max-width: 100%;
        height: auto;
    }

    .arcana-sort-current-label :global(.p5-prompt-word) {
        max-width: 100%;
        height: auto;
    }

    /* ── Column headers ── */
    .arcana-missions-col-headers {
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

    .arcana-col-header {
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

    .arcana-col-status {
        --col-h: clamp(3.5rem, 7vw, 7rem);
        --col-rot: -1deg;
        --col-font-size: clamp(1.6rem, 2.5vw, 5rem);
        --col-pad-x: clamp(0.5rem, 0.7vw, 1rem);
        top: clamp(2rem, 4vw, 4rem);
    }

    .arcana-col-name {
        --col-h: clamp(3.5rem, 7vw, 7rem);
        --col-rot: -2deg;
        --col-font-size: clamp(1.6rem, 2.5vw, 5rem);
        --col-pad-x: clamp(0.7rem, 0.9vw, 1.2rem);
        top: clamp(1rem, 2vw, 2rem);
    }

    .arcana-col-grade {
        --col-h: clamp(3.5rem, 7vw, 7rem);
        --col-rot: -3deg;
        --col-font-size: clamp(1.6rem, 2.5vw, 5rem);
        --col-pad-x: clamp(0.5rem, 0.7vw, 1rem);
        top: clamp(1rem, 2vw, 2rem);
        text-align: center;
    }

    /* ── Scroll area ── */
    .arcana-missions-scroll {
        flex: 1;
        overflow-x: visible;
        overflow-y: auto;
        scrollbar-width: none;
    }

    .arcana-missions-scroll::-webkit-scrollbar {
        display: none;
    }

    /* ── Custom scroll indicator ── */
    .arcana-missions-scroll-indicator {
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

    .arcana-missions-scroll-track {
        position: relative;
        width: 100%;
        height: 100%;
        background: var(--arcana-black);
        border: 4px solid var(--arcana-white);
        box-sizing: border-box;
    }

    .arcana-missions-scroll-thumb {
        position: absolute;
        left: 0;
        right: 0;
        height: calc(var(--thumb-ratio, 1) * 100%);
        top: calc(
            var(--scroll-ratio, 0) * (100% - var(--thumb-ratio, 1) * 100%)
        );
        background: var(--arcana-white);
    }

    .arcana-missions-list {
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
    .arcana-mission-row {
        display: grid;
        grid-template-columns: var(--mission-grid-columns);
        width: 100%;
        column-gap: 0;
        align-items: center;
        height: 7rem;
        padding: 0;
        border: 0;
        color: inherit;
        background: transparent;
        font: inherit;
        text-align: left;
        cursor: pointer;
        transition:
            color 100ms ease,
            transform 100ms ease;
        clip-path: polygon(0% 4%, 100% 0%, 100% 96%, 0% 100%);
        position: relative;
    }

    .arcana-mission-row::before {
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

    .arcana-mission-row.is-selected {
        background: transparent;
        transform: scaleY(1.08);
        clip-path: none;
        z-index: 2;
    }

    .arcana-mission-row.is-selected::before {
        background: #e5191c;
    }

    .arcana-mission-row.is-completed {
        opacity: 0.55;
    }

    .arcana-mission-row.is-archived {
        opacity: 0.3;
    }

    .arcana-mission-row.is-selected.is-completed,
    .arcana-mission-row.is-selected.is-archived {
        opacity: 1;
    }

    /* ── Status stamp ── */
    .arcana-mission-stamp {
        display: block;
        width: 80%;
        height: 80%;
        object-fit: contain;
    }

    .arcana-mission-row.is-completed .arcana-mission-stamp {
        opacity: 0.9;
    }

    .arcana-mission-row.is-selected .arcana-mission-stamp {
        opacity: 1;
    }

    /* ── Mission name ── */
    .arcana-mission-name {
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
    .arcana-mission-grade {
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

    .arcana-mission-grade[data-grade="S"] {
        color: #e5191c;
    }

    .arcana-mission-grade[data-grade="--"] {
        font-size: clamp(0.9rem, 1vw, 1.5rem);
        opacity: 0.3;
    }

    .arcana-mission-row.is-selected .arcana-mission-grade {
        color: #ffffff;
    }

    /* ── Detail card overlay ── */
    .arcana-detail-backdrop {
        position: absolute;
        inset: 0;
        z-index: 20;
        padding: 0;
        border: 0;
        background: rgba(0, 0, 0, 0.5);
    }

    .arcana-detail-card {
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
        animation: arcana-detail-pop 180ms ease-out;
    }

    @keyframes arcana-detail-pop {
        from {
            opacity: 0;
            transform: translate(-50%, -46%) rotate(-1.5deg) scale(0.92);
        }
        to {
            opacity: 1;
            transform: translate(-50%, -50%) rotate(-1.5deg) scale(1);
        }
    }

    .arcana-detail-top {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: clamp(0.5rem, 0.6vw, 0.9rem) clamp(0.8rem, 1vw, 1.5rem);
        background: #e5191c;
        clip-path: polygon(0% 0%, 100% 0%, 100% 85%, 0% 100%);
    }

    .arcana-detail-stamp {
        font-size: clamp(0.7rem, 0.75vw, 1.1rem);
        font-weight: 900;
        font-style: italic;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: #ffffff;
    }

    .arcana-detail-grade {
        font-size: clamp(1.6rem, 2vw, 2.8rem);
        font-weight: 900;
        color: #000000;
        background: #ffffff;
        padding: 0 clamp(0.4rem, 0.5vw, 0.8rem);
        line-height: 1.2;
        clip-path: polygon(6% 0%, 100% 5%, 94% 100%, 0% 95%);
    }

    .arcana-detail-grade[data-grade="S"] {
        color: #e5191c;
    }

    .arcana-detail-grade[data-grade="--"] {
        font-size: clamp(1rem, 1.2vw, 1.8rem);
        opacity: 0.4;
    }

    .arcana-detail-title {
        margin: 0;
        padding: clamp(0.6rem, 0.8vw, 1.2rem) clamp(0.8rem, 1vw, 1.5rem)
            clamp(0.3rem, 0.4vw, 0.6rem);
        font-size: clamp(1rem, 1.2vw, 1.8rem);
        font-weight: 900;
        color: #ffffff;
        letter-spacing: 0.03em;
        line-height: 1.3;
    }

    .arcana-detail-desc {
        margin: 0;
        padding: 0 clamp(0.8rem, 1vw, 1.5rem) clamp(0.6rem, 0.7vw, 1rem);
        font-size: clamp(0.7rem, 0.65vw, 1rem);
        font-weight: 600;
        color: rgba(255, 255, 255, 0.6);
        line-height: 1.5;
        white-space: pre-line;
    }

    .arcana-detail-meta {
        display: flex;
        flex-direction: column;
        gap: clamp(0.3rem, 0.4vw, 0.6rem);
        padding: clamp(0.5rem, 0.6vw, 0.9rem) clamp(0.8rem, 1vw, 1.5rem)
            clamp(0.7rem, 0.8vw, 1.2rem);
        border-top: 1px solid rgba(255, 255, 255, 0.08);
        margin-top: auto;
    }

    .arcana-detail-progress-row {
        display: flex;
        align-items: center;
        gap: clamp(0.4rem, 0.5vw, 0.8rem);
    }

    .arcana-detail-track {
        flex: 1;
        height: clamp(6px, 0.5vw, 10px);
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.15);
        overflow: hidden;
    }

    .arcana-detail-fill {
        height: 100%;
        background: #e5191c;
        transition: width 300ms ease;
    }

    .arcana-detail-pct {
        font-size: clamp(0.7rem, 0.7vw, 1.1rem);
        font-weight: 800;
        color: rgba(255, 255, 255, 0.6);
        flex-shrink: 0;
    }

    .arcana-detail-deadline {
        font-size: clamp(0.6rem, 0.6vw, 0.9rem);
        font-weight: 800;
        letter-spacing: 0.06em;
        color: #e5191c;
    }

    .arcana-detail-deadline.is-overdue {
        color: rgba(255, 80, 80, 0.9);
    }

    .arcana-detail-reason {
        margin: 0 clamp(0.8rem, 1vw, 1.5rem) clamp(0.6rem, 0.7vw, 1rem);
        padding: clamp(0.5rem, 0.6vw, 0.9rem);
        color: rgba(255, 255, 255, 0.72);
        background: rgba(255, 255, 255, 0.06);
        border-left: 3px solid #e5191c;
        font-size: clamp(0.68rem, 0.65vw, 1rem);
        line-height: 1.45;
    }

    .arcana-detail-reason strong {
        display: block;
        margin-bottom: 0.2rem;
        color: #e5191c;
        font-size: 0.82em;
        letter-spacing: 0.08em;
    }

    .arcana-dashboard-slots {
        padding: clamp(0.45rem, 0.55vw, 0.8rem) clamp(0.8rem, 1vw, 1.5rem);
        border-top: 1px solid rgba(255, 255, 255, 0.08);
    }

    .arcana-dashboard-slots-label {
        display: block;
        margin-bottom: 0.35rem;
        color: rgba(255, 255, 255, 0.5);
        font-size: clamp(0.58rem, 0.58vw, 0.86rem);
        font-weight: 800;
        letter-spacing: 0.08em;
        text-transform: uppercase;
    }

    .arcana-dashboard-slot-buttons {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 0.3rem;
    }

    .arcana-dashboard-slot-btn {
        border: 1px solid rgba(255, 255, 255, 0.18);
        padding: 0.3rem 0.45rem;
        color: rgba(255, 255, 255, 0.65);
        background: rgba(255, 255, 255, 0.06);
        font-size: clamp(0.58rem, 0.6vw, 0.88rem);
        font-weight: 800;
        text-transform: uppercase;
        cursor: pointer;
    }

    .arcana-dashboard-slot-btn.is-selected {
        border-color: #e5191c;
        color: #ffffff;
        background: rgba(229, 25, 28, 0.55);
    }

    .arcana-dashboard-slot-btn:disabled {
        opacity: 0.3;
        cursor: not-allowed;
    }

    /* ── Detail action buttons ── */
    .arcana-detail-actions {
        display: flex;
        gap: clamp(0.5rem, 0.6vw, 1rem);
        padding: clamp(0.5rem, 0.6vw, 0.9rem) clamp(0.8rem, 1vw, 1.5rem)
            clamp(0.7rem, 0.8vw, 1.2rem);
        border-top: 1px solid rgba(255, 255, 255, 0.08);
    }

    .arcana-action-btn {
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

    .arcana-action-btn:hover {
        transform: scale(1.03);
    }

    .arcana-action-btn:disabled {
        opacity: 0.4;
        cursor: not-allowed;
        transform: none;
    }

    .arcana-action-accept {
        background: #e5191c;
        color: #ffffff;
    }

    .arcana-action-reject {
        background: rgba(255, 255, 255, 0.1);
        color: rgba(255, 255, 255, 0.5);
    }

    .arcana-action-reject:hover {
        background: rgba(255, 255, 255, 0.15);
        color: rgba(255, 255, 255, 0.7);
    }

    .arcana-detail-action-error {
        margin: 0;
        padding: 0 clamp(0.8rem, 1vw, 1.5rem) clamp(0.7rem, 0.8vw, 1.2rem);
        color: #ff6b6b;
        font-size: clamp(0.65rem, 0.65vw, 0.95rem);
        font-weight: 700;
    }

    .arcana-back-btn--missions {
        left: auto;
        right: clamp(1.5rem, 3vw, 4rem);
    }

    /* ── P key: phan mode toggle button ── */
    .arcana-phan-mode-btn {
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

    .arcana-phan-mode-btn:hover {
        transform: rotate(-5deg) scale(1.06);
    }

    .arcana-phan-mode-btn :global(.p5-prompt-word) {
        margin-left: -1rem;
    }
</style>
