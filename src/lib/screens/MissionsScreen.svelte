<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import CallingCardText from "$lib/CallingCardText.svelte";
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

    // Phan-Site panel state
    let phanSiteOpen = $state(false);
    let phanSelectedIndex = $state(0);
    let phanDetailMission = $state<MissionResponse | null>(null);

    type SortOption = { key: string; label: string };
    const SORT_OPTIONS: SortOption[] = [
        { key: "newest", label: "NEW" },
        { key: "status", label: "STATE" },
        { key: "progress", label: "DIFFICULTY" },
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
                    (a, b) => (b.progress ?? 0) - (a.progress ?? 0),
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

    // Auto-scroll selected row into view
    $effect(() => {
        const el = rowRefs[selectedIndex];
        if (el) el.scrollIntoView({ block: "nearest", behavior: "smooth" });
    });

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

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            event.preventDefault();
            if (phanDetailMission) {
                phanDetailMission = null;
            } else if (phanSiteOpen) {
                phanSiteOpen = false;
            } else if (detailMission) {
                closeDetail();
            } else {
                onBack();
            }
            return;
        }
        if (event.key === "Enter") {
            event.preventDefault();
            if (detailMission) {
                closeDetail();
            } else if (sortedMissions.length > 0) {
                openDetail(selectedIndex);
            }
            return;
        }
        if (event.key === "q" || event.key === "Q") {
            event.preventDefault();
            cycleSort(-1);
            return;
        }
        if (event.key === "e" || event.key === "E") {
            event.preventDefault();
            cycleSort(1);
            return;
        }
        if (event.key === "r" || event.key === "R") {
            event.preventDefault();
            void refreshMissions();
            return;
        }
        if (event.key === "ArrowDown") {
            event.preventDefault();
            detailMission = null;
            if (sortedMissions.length > 0) {
                selectedIndex = Math.min(
                    selectedIndex + 1,
                    sortedMissions.length - 1,
                );
            }
            return;
        }
        if (event.key === "ArrowUp") {
            event.preventDefault();
            detailMission = null;
            selectedIndex = Math.max(selectedIndex - 1, 0);
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
    <div class="rm-missions-panel">
        <!-- Sort carousel: Q shifts right-to-left, E shifts left-to-right, center is active -->
        <header class="rm-missions-sort-bar">
            <button
                class="rm-sort-key-hint"
                onclick={() => cycleSort(-1)}
                aria-label="Previous sort">Q</button
            >
            <div class="rm-sort-carousel">
                <button
                    class="rm-sort-item rm-sort-side"
                    onclick={() => cycleSort(-1)}
                >
                    {SORT_OPTIONS[sortCarousel[0]].label}
                </button>
                <span class="rm-sort-item rm-sort-center">
                    {SORT_OPTIONS[sortCarousel[1]].label}
                </span>
                <button
                    class="rm-sort-item rm-sort-side"
                    onclick={() => cycleSort(1)}
                >
                    {SORT_OPTIONS[sortCarousel[2]].label}
                </button>
            </div>
            <button
                class="rm-sort-key-hint"
                onclick={() => cycleSort(1)}
                aria-label="Next sort">E</button
            >
        </header>

        <!-- Column headers -->
        <div class="rm-missions-col-headers">
            <span class="rm-col-header rm-col-status">STATUS</span>
            <span class="rm-col-header rm-col-name">MISSION</span>
            <span class="rm-col-header rm-col-grade">DIFFICULTY</span>
        </div>

        <!-- Mission list -->
        <div class="rm-missions-scroll">
            {#if loading}
                <p class="state-text">Loading...</p>
            {:else if error}
                <p class="state-text error">{error}</p>
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
                            <span
                                class="rm-mission-stamp"
                                class:stamp-active={mission.status === "active"}
                                class:stamp-clear={mission.status ===
                                    "completed"}
                                class:stamp-archived={mission.status ===
                                    "archived"}
                            >
                                {statusLabel(mission.status)}
                            </span>

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

    <!-- Phan-Site button (top-left) -->
    <button
        class="rm-phansite-btn"
        class:has-new={proposedMissions.length > 0}
        onclick={() => {
            phanSiteOpen = true;
            phanSelectedIndex = 0;
            phanDetailMission = null;
        }}
    >
        <div class="rm-phansite-phone-icon">
            <div class="rm-phansite-phone-screen">
                {#if proposedMissions.length > 0}
                    <span class="rm-phansite-badge"
                        >{proposedMissions.length}</span
                    >
                {/if}
            </div>
        </div>
        <div class="rm-phansite-label">
            <CallingCardText text="PHAN SiTE" fontSize={28} />
        </div>
    </button>

    <!-- Phan-Site phone panel overlay -->
    {#if phanSiteOpen}
        <div
            class="rm-phan-backdrop"
            onclick={() => {
                phanSiteOpen = false;
                phanDetailMission = null;
            }}
        ></div>
        <div class="rm-phan-phone">
            <!-- Phone notch / header -->
            <div class="rm-phan-phone-header">
                <div class="rm-phan-phone-notch"></div>
                <span class="rm-phan-phone-title">PHAN-SITE</span>
                <span class="rm-phan-phone-subtitle">New Requests</span>
            </div>

            <!-- Request list -->
            <div class="rm-phan-phone-body">
                {#if proposedMissions.length === 0}
                    <div class="rm-phan-empty">
                        <span class="rm-phan-empty-icon">--</span>
                        <span class="rm-phan-empty-text">No new requests.</span>
                    </div>
                {:else}
                    <ul class="rm-phan-list">
                        {#each proposedMissions as mission, i (mission.id)}
                            <li
                                class="rm-phan-item"
                                class:is-selected={phanSelectedIndex === i}
                                onclick={() => {
                                    phanSelectedIndex = i;
                                    phanDetailMission = mission;
                                }}
                                onmouseenter={() => {
                                    phanSelectedIndex = i;
                                }}
                            >
                                <span
                                    class="rm-phan-item-tier"
                                    data-tier={mission.progress != null
                                        ? difficultyGrade(mission.difficulty)
                                        : "--"}
                                >
                                    {#if mission.deadline && mission.days_remaining != null}
                                        {mission.days_remaining}d
                                    {:else}
                                        --
                                    {/if}
                                </span>
                                <span class="rm-phan-item-title"
                                    >{mission.title}</span
                                >
                            </li>
                        {/each}
                    </ul>
                {/if}
            </div>

            <!-- Phone home bar -->
            <div class="rm-phan-phone-footer">
                <div class="rm-phan-home-bar"></div>
            </div>
        </div>

        <!-- Phan-Site detail card (slides over phone) -->
        {#if phanDetailMission}
            <div class="rm-phan-detail">
                <button
                    class="rm-phan-detail-back"
                    onclick={() => {
                        phanDetailMission = null;
                    }}>BACK</button
                >
                <h2 class="rm-phan-detail-title">{phanDetailMission.title}</h2>
                {#if phanDetailMission.description}
                    <p class="rm-phan-detail-desc">
                        {phanDetailMission.description}
                    </p>
                {/if}
                <div class="rm-phan-detail-meta">
                    {#if phanDetailMission.days_remaining != null}
                        <span class="rm-phan-detail-deadline">
                            {phanDetailMission.days_remaining > 0
                                ? `${phanDetailMission.days_remaining} DAYS`
                                : phanDetailMission.days_remaining === 0
                                  ? "TODAY"
                                  : "OVERDUE"}
                        </span>
                    {/if}
                </div>
                <div class="rm-phan-detail-actions">
                    <button
                        class="rm-action-btn rm-action-accept"
                        disabled={updating}
                        onclick={() =>
                            updateMissionStatus(
                                phanDetailMission!.id,
                                "active",
                            )}>ACCEPT</button
                    >
                    <button
                        class="rm-action-btn rm-action-reject"
                        disabled={updating}
                        onclick={() =>
                            updateMissionStatus(
                                phanDetailMission!.id,
                                "rejected",
                            )}>REJECT</button
                    >
                </div>
            </div>
        {/if}
    {/if}

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
    /* ── Panel ── */
    .rm-missions-panel {
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
        background: rgba(0, 0, 0, 0.85);
        border-left: 3px solid rgba(255, 255, 255, 0.12);
    }

    /* ── Sort carousel ── */
    .rm-missions-sort-bar {
        flex-shrink: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        background: #000000;
        border-bottom: 2px solid rgba(255, 255, 255, 0.12);
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        padding: clamp(0.4rem, 0.5vw, 0.7rem) 0;
        gap: 0;
    }

    .rm-sort-carousel {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: clamp(0.8rem, 1.2vw, 2rem);
        flex: 1;
        overflow: hidden;
    }

    .rm-sort-item {
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        white-space: nowrap;
        transition:
            font-size 180ms ease,
            color 180ms ease,
            transform 180ms ease,
            opacity 180ms ease;
    }

    .rm-sort-center {
        font-size: clamp(1.1rem, 1.5vw, 2rem);
        color: #e5191c;
        transform: scale(1);
        padding: clamp(0.15rem, 0.2vw, 0.3rem) clamp(0.6rem, 0.8vw, 1.2rem);
        background: rgba(229, 25, 28, 0.12);
        clip-path: polygon(3% 0%, 100% 5%, 97% 100%, 0% 95%);
    }

    .rm-sort-side {
        font-size: clamp(0.65rem, 0.7vw, 1rem);
        color: rgba(255, 255, 255, 0.3);
        border: none;
        background: none;
        cursor: pointer;
        padding: 0;
    }

    .rm-sort-side:hover {
        color: rgba(255, 255, 255, 0.6);
    }

    .rm-sort-key-hint {
        flex-shrink: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        width: clamp(1.6rem, 2vw, 2.4rem);
        height: clamp(1.6rem, 2vw, 2.4rem);
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(0.6rem, 0.6vw, 0.9rem);
        font-weight: 800;
        color: rgba(255, 255, 255, 0.35);
        background: rgba(255, 255, 255, 0.06);
        border: 1px solid rgba(255, 255, 255, 0.1);
        cursor: pointer;
        margin: 0 clamp(0.3rem, 0.4vw, 0.6rem);
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    .rm-sort-key-hint:hover {
        background: rgba(255, 255, 255, 0.12);
        color: rgba(255, 255, 255, 0.6);
    }

    /* ── Column headers ── */
    .rm-missions-col-headers {
        flex-shrink: 0;
        display: flex;
        align-items: center;
        padding: clamp(0.3rem, 0.4vw, 0.6rem) clamp(1rem, 1.2vw, 2rem);
        border-bottom: 2px solid rgba(255, 255, 255, 0.15);
        background: rgba(255, 255, 255, 0.03);
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    }

    .rm-col-header {
        font-size: clamp(0.55rem, 0.5vw, 0.8rem);
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.12em;
        color: rgba(255, 255, 255, 0.35);
    }

    .rm-col-status {
        width: clamp(5rem, 8vw, 9rem);
        flex-shrink: 0;
    }

    .rm-col-name {
        flex: 1;
    }

    .rm-col-grade {
        width: clamp(3rem, 5vw, 6rem);
        flex-shrink: 0;
        text-align: center;
    }

    /* ── Scroll area ── */
    .rm-missions-scroll {
        flex: 1;
        overflow-y: auto;
        padding: 0 0 4rem 0;
    }

    .rm-missions-list {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 0;
    }

    /* ── Mission rows ── */
    .rm-mission-row {
        display: flex;
        align-items: center;
        padding: clamp(0.6rem, 0.8vw, 1.2rem) clamp(1rem, 1.2vw, 2rem);
        border-bottom: 1px solid rgba(255, 255, 255, 0.06);
        cursor: pointer;
        transition:
            background 100ms ease,
            transform 100ms ease;
        clip-path: polygon(0% 4%, 100% 0%, 100% 96%, 0% 100%);
        position: relative;
    }

    .rm-mission-row:hover {
        background: rgba(255, 255, 255, 0.06);
    }

    .rm-mission-row.is-selected {
        background: #e5191c;
        transform: scaleY(1.08);
        clip-path: polygon(0% 0%, 100% 3%, 100% 97%, 0% 100%);
        z-index: 2;
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
        width: clamp(5rem, 8vw, 9rem);
        flex-shrink: 0;
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(0.7rem, 0.75vw, 1.1rem);
        font-weight: 900;
        font-style: italic;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: rgba(255, 255, 255, 0.3);
    }

    .rm-mission-stamp.stamp-active {
        color: #e5191c;
        transform: rotate(-2deg);
        text-shadow: 1px 1px 0 rgba(0, 0, 0, 0.5);
    }

    .rm-mission-stamp.stamp-clear {
        color: rgba(255, 255, 255, 0.7);
        transform: rotate(-1deg);
    }

    .rm-mission-row.is-selected .rm-mission-stamp {
        color: #ffffff;
    }

    /* ── Mission name ── */
    .rm-mission-name {
        flex: 1;
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(0.85rem, 0.9vw, 1.4rem);
        font-weight: 800;
        color: #ffffff;
        letter-spacing: 0.03em;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    /* ── Grade letter ── */
    .rm-mission-grade {
        width: clamp(3rem, 5vw, 6rem);
        flex-shrink: 0;
        text-align: center;
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(1.4rem, 1.8vw, 2.5rem);
        font-weight: 900;
        color: #000000;
        background: #ffffff;
        padding: clamp(0.1rem, 0.15vw, 0.25rem) 0;
        clip-path: polygon(5% 0%, 100% 4%, 95% 100%, 0% 96%);
        line-height: 1.2;
    }

    .rm-mission-grade[data-grade="S"] {
        color: #e5191c;
    }

    .rm-mission-grade[data-grade="D"] {
        opacity: 0.5;
    }

    .rm-mission-grade[data-grade="--"] {
        font-size: clamp(0.9rem, 1vw, 1.5rem);
        opacity: 0.3;
    }

    .rm-mission-row.is-selected .rm-mission-grade {
        background: #ffffff;
        color: #e5191c;
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
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
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
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
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

    /* ── Phan-Site button ── */
    .rm-phansite-btn {
        position: absolute;
        left: clamp(1rem, 2vw, 2.5rem);
        top: clamp(1rem, 2vw, 2.5rem);
        z-index: 5;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: clamp(0.3rem, 0.4vw, 0.6rem);
        background: none;
        border: none;
        cursor: pointer;
        transition: transform 150ms ease;
    }

    .rm-phansite-btn:hover {
        transform: scale(1.06) rotate(-2deg);
    }

    .rm-phansite-phone-icon {
        width: clamp(2.8rem, 3.5vw, 4.5rem);
        height: clamp(4.5rem, 5.5vw, 7rem);
        background: #1a1a1a;
        border: 2px solid rgba(255, 255, 255, 0.2);
        border-radius: clamp(6px, 0.5vw, 10px);
        display: flex;
        align-items: center;
        justify-content: center;
        position: relative;
    }

    .rm-phansite-phone-screen {
        width: 80%;
        height: 75%;
        background: #111111;
        border-radius: 2px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .rm-phansite-badge {
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(1rem, 1.2vw, 1.6rem);
        font-weight: 900;
        color: #e5191c;
        animation: rm-badge-pulse 1.5s ease-in-out infinite;
    }

    @keyframes rm-badge-pulse {
        0%,
        100% {
            opacity: 1;
            transform: scale(1);
        }
        50% {
            opacity: 0.7;
            transform: scale(0.9);
        }
    }

    .rm-phansite-btn.has-new .rm-phansite-phone-icon {
        border-color: #e5191c;
        box-shadow: 0 0 12px rgba(229, 25, 28, 0.4);
    }

    .rm-phansite-label {
        pointer-events: none;
    }

    .rm-back-btn--missions {
        left: auto;
        right: clamp(1.5rem, 3vw, 4rem);
    }

    /* ── Phan-Site phone panel ── */
    .rm-phan-backdrop {
        position: absolute;
        inset: 0;
        z-index: 30;
        background: rgba(0, 0, 0, 0.6);
    }

    .rm-phan-phone {
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        z-index: 31;
        width: clamp(260px, 22vw, 380px);
        height: clamp(440px, 52vh, 640px);
        background: #0a0a0a;
        border: 3px solid rgba(255, 255, 255, 0.15);
        border-radius: clamp(16px, 1.5vw, 28px);
        display: flex;
        flex-direction: column;
        overflow: hidden;
        box-shadow:
            0 0 40px rgba(0, 0, 0, 0.8),
            0 0 8px rgba(229, 25, 28, 0.15);
        animation: rm-phone-slide 250ms ease-out;
    }

    @keyframes rm-phone-slide {
        from {
            opacity: 0;
            transform: translate(-50%, -44%) scale(0.9);
        }
        to {
            opacity: 1;
            transform: translate(-50%, -50%) scale(1);
        }
    }

    .rm-phan-phone-header {
        flex-shrink: 0;
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: clamp(0.8rem, 1vw, 1.4rem) clamp(0.8rem, 1vw, 1.2rem)
            clamp(0.4rem, 0.5vw, 0.7rem);
        background: #e5191c;
        position: relative;
    }

    .rm-phan-phone-notch {
        width: clamp(3rem, 4vw, 5rem);
        height: 4px;
        background: rgba(0, 0, 0, 0.4);
        border-radius: 2px;
        margin-bottom: clamp(0.4rem, 0.5vw, 0.7rem);
    }

    .rm-phan-phone-title {
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(1.1rem, 1.3vw, 1.8rem);
        font-weight: 900;
        color: #ffffff;
        letter-spacing: 0.1em;
        text-transform: uppercase;
    }

    .rm-phan-phone-subtitle {
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(0.5rem, 0.5vw, 0.7rem);
        font-weight: 700;
        color: rgba(255, 255, 255, 0.6);
        letter-spacing: 0.15em;
        text-transform: uppercase;
        margin-top: 2px;
    }

    .rm-phan-phone-body {
        flex: 1;
        overflow-y: auto;
        padding: clamp(0.4rem, 0.5vw, 0.8rem) 0;
    }

    .rm-phan-empty {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        height: 100%;
        gap: 0.5rem;
    }

    .rm-phan-empty-icon {
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(2rem, 2.5vw, 3rem);
        color: rgba(255, 255, 255, 0.1);
        font-weight: 900;
    }

    .rm-phan-empty-text {
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(0.65rem, 0.6vw, 0.9rem);
        color: rgba(255, 255, 255, 0.25);
        font-weight: 700;
    }

    /* ── Phan-Site list items ── */
    .rm-phan-list {
        list-style: none;
        margin: 0;
        padding: 0;
    }

    .rm-phan-item {
        display: flex;
        align-items: center;
        gap: clamp(0.5rem, 0.6vw, 0.8rem);
        padding: clamp(0.6rem, 0.7vw, 1rem) clamp(0.8rem, 1vw, 1.2rem);
        cursor: pointer;
        transition: background 100ms ease;
        border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    }

    .rm-phan-item:hover {
        background: rgba(255, 255, 255, 0.04);
    }

    .rm-phan-item.is-selected {
        background: #e5191c;
    }

    .rm-phan-item-tier {
        flex-shrink: 0;
        width: clamp(2rem, 2.5vw, 3rem);
        text-align: center;
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(0.6rem, 0.6vw, 0.85rem);
        font-weight: 800;
        color: rgba(255, 255, 255, 0.4);
    }

    .rm-phan-item.is-selected .rm-phan-item-tier {
        color: rgba(255, 255, 255, 0.8);
    }

    .rm-phan-item-title {
        flex: 1;
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(0.7rem, 0.7vw, 1rem);
        font-weight: 800;
        color: #ffffff;
        letter-spacing: 0.02em;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    /* ── Phan-Site detail card ── */
    .rm-phan-detail {
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        z-index: 32;
        width: clamp(240px, 20vw, 350px);
        background: #0a0a0a;
        border: 2px solid rgba(229, 25, 28, 0.4);
        border-radius: clamp(12px, 1vw, 20px);
        padding: clamp(0.8rem, 1vw, 1.4rem);
        display: flex;
        flex-direction: column;
        gap: clamp(0.5rem, 0.6vw, 0.8rem);
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        animation: rm-phone-slide 200ms ease-out;
        box-shadow: 0 0 30px rgba(0, 0, 0, 0.9);
    }

    .rm-phan-detail-back {
        align-self: flex-start;
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(0.55rem, 0.5vw, 0.75rem);
        font-weight: 800;
        color: rgba(255, 255, 255, 0.4);
        background: none;
        border: none;
        cursor: pointer;
        padding: 0;
        letter-spacing: 0.08em;
    }

    .rm-phan-detail-back:hover {
        color: rgba(255, 255, 255, 0.7);
    }

    .rm-phan-detail-title {
        margin: 0;
        font-size: clamp(0.95rem, 1.1vw, 1.5rem);
        font-weight: 900;
        color: #ffffff;
        letter-spacing: 0.02em;
        line-height: 1.3;
    }

    .rm-phan-detail-desc {
        margin: 0;
        font-size: clamp(0.6rem, 0.55vw, 0.85rem);
        font-weight: 600;
        color: rgba(255, 255, 255, 0.55);
        line-height: 1.6;
    }

    .rm-phan-detail-meta {
        display: flex;
        gap: 0.5rem;
    }

    .rm-phan-detail-deadline {
        font-size: clamp(0.55rem, 0.55vw, 0.8rem);
        font-weight: 800;
        color: #e5191c;
        letter-spacing: 0.06em;
    }

    .rm-phan-detail-actions {
        display: flex;
        gap: clamp(0.4rem, 0.5vw, 0.8rem);
        margin-top: clamp(0.3rem, 0.4vw, 0.5rem);
    }

    .rm-phan-phone-footer {
        flex-shrink: 0;
        display: flex;
        justify-content: center;
        padding: clamp(0.4rem, 0.5vw, 0.7rem);
    }

    .rm-phan-home-bar {
        width: clamp(3rem, 4vw, 5rem);
        height: 4px;
        background: rgba(255, 255, 255, 0.15);
        border-radius: 2px;
    }

</style>
