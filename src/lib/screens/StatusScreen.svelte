<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import RadarChart from "$lib/components/RadarChart.svelte";
    import StatusDetailView from "$lib/screens/StatusDetailView.svelte";
    import type { DimensionData, StatusData } from "$lib/types/status";
    import { dataCommandErrorMessage } from "$lib/types/data_platform";
    import KeyHint from "$lib/KeyHint.svelte";
    import PromptWord from "$lib/PromptWord.svelte";

    let {
        onBack,
        statusData: initialStatusData,
        onStatusDataLoaded,
    }: {
        onBack: () => void;
        statusData: StatusData | null;
        onStatusDataLoaded?: (data: StatusData) => void;
    } = $props();

    let loading = $state(false);
    let errorMessage = $state<string | null>(null);
    let statusData = $state<StatusData | null>(initialStatusData);
    let view = $state<"radar" | "detail" | "configure">("radar");
    let selectedDimensionId = $state<string | null>(null);
    let updatingPosition = $state<number | null>(null);

    let selectedDimensions = $derived.by<DimensionData[]>(() =>
        (statusData?.dimensions ?? [])
            .filter((dimension) => dimension.selected_position !== undefined)
            .sort(
                (left, right) =>
                    (left.selected_position ?? 0) -
                    (right.selected_position ?? 0),
            ),
    );

    function errorText(error: unknown): string {
        return dataCommandErrorMessage(
            error,
            "Failed to load Status data from the local database.",
        );
    }

    function handleDimensionSelect(id: string) {
        selectedDimensionId = id;
        view = "detail";
    }

    function handleDetailBack() {
        view = "radar";
        selectedDimensionId = null;
    }

    async function loadStatusData() {
        loading = true;
        errorMessage = null;

        try {
            statusData = await invoke<StatusData>("load_status_dashboard");
            onStatusDataLoaded?.(statusData);
        } catch (error) {
            errorMessage = errorText(error);
            statusData = null;
        } finally {
            loading = false;
        }
    }

    async function updateSelection(position: number, dimensionId: string) {
        updatingPosition = position;
        errorMessage = null;
        try {
            if (dimensionId) {
                await invoke("select_status_dimension", {
                    position,
                    dimensionId,
                });
            } else {
                await invoke("clear_status_dimension", { position });
            }
            await loadStatusData();
        } catch (error) {
            errorMessage = errorText(error);
        } finally {
            updatingPosition = null;
        }
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            event.preventDefault();
            if (view !== "radar") {
                handleDetailBack();
            } else {
                onBack();
            }
        }
    }

    onMount(() => {
        if (!statusData && !loading) {
            void loadStatusData();
        }

        window.addEventListener("keydown", handleKeydown);
        return () => {
            window.removeEventListener("keydown", handleKeydown);
        };
    });
</script>

<section class="rm-stage">
    {#if view === "radar"}
        <div class="rm-status-stars-br" aria-hidden="true">
            <div class="rm-star-group rm-sg-1">
                <div class="rm-sstar rm-sstar-1"></div>
                <div class="rm-sstar rm-sstar-2"></div>
                <div class="rm-sstar rm-sstar-3"></div>
                <div class="rm-sstar rm-sstar-4"></div>
                <div class="rm-sstar rm-sstar-5"></div>
                <div class="rm-sstar rm-sstar-6"></div>
                <div class="rm-sstar rm-sstar-7"></div>
                <div class="rm-sstar rm-sstar-8"></div>
            </div>
            <div class="rm-star-group rm-sg-2">
                <div class="rm-sstar rm-sstar-1"></div>
                <div class="rm-sstar rm-sstar-2"></div>
                <div class="rm-sstar rm-sstar-3"></div>
                <div class="rm-sstar rm-sstar-4"></div>
                <div class="rm-sstar rm-sstar-5"></div>
                <div class="rm-sstar rm-sstar-6"></div>
                <div class="rm-sstar rm-sstar-7"></div>
                <div class="rm-sstar rm-sstar-8"></div>
            </div>
            <div class="rm-star-group rm-sg-3">
                <div class="rm-sstar rm-sstar-1"></div>
                <div class="rm-sstar rm-sstar-2"></div>
                <div class="rm-sstar rm-sstar-3"></div>
                <div class="rm-sstar rm-sstar-4"></div>
                <div class="rm-sstar rm-sstar-5"></div>
                <div class="rm-sstar rm-sstar-6"></div>
                <div class="rm-sstar rm-sstar-7"></div>
                <div class="rm-sstar rm-sstar-8"></div>
            </div>
            <div class="rm-star-group rm-sg-4">
                <div class="rm-sstar rm-sstar-1"></div>
                <div class="rm-sstar rm-sstar-2"></div>
                <div class="rm-sstar rm-sstar-3"></div>
                <div class="rm-sstar rm-sstar-4"></div>
                <div class="rm-sstar rm-sstar-5"></div>
                <div class="rm-sstar rm-sstar-6"></div>
                <div class="rm-sstar rm-sstar-7"></div>
                <div class="rm-sstar rm-sstar-8"></div>
            </div>
            <div class="rm-star-group rm-sg-5">
                <div class="rm-sstar rm-sstar-1"></div>
                <div class="rm-sstar rm-sstar-2"></div>
                <div class="rm-sstar rm-sstar-3"></div>
                <div class="rm-sstar rm-sstar-4"></div>
                <div class="rm-sstar rm-sstar-5"></div>
                <div class="rm-sstar rm-sstar-6"></div>
                <div class="rm-sstar rm-sstar-7"></div>
                <div class="rm-sstar rm-sstar-8"></div>
            </div>
        </div>
        <div class="rm-status-bg" aria-hidden="true"></div>
    {/if}
    <img src="/ui/Status.png" alt="Status" class="rm-status-title-img" />

    <button
        type="button"
        class="rm-back-btn"
        onclick={() => {
            if (view !== "radar") {
                handleDetailBack();
            } else {
                onBack();
            }
        }}
    >
        <KeyHint key="Esc" fontSize={36} />
        <PromptWord text="Back" fontSize={72} />
    </button>

    {#if statusData && view === "radar"}
        <div class="rm-player-panel" aria-label="Player info">
            <div
                class="rm-hint-board"
                style:background-image="url(/ui/board/board_fat.png)"
            >
                <span class="rm-hint-text">用户：{statusData.username}</span>
            </div>
            {#if statusData.game_days !== null}
                <div
                    class="rm-hint-board rm-hint-board--slim"
                    style:background-image="url(/ui/board/board_slim.png)"
                >
                    <span class="rm-hint-text">游戏天数：{statusData.game_days}</span>
                </div>
            {/if}
        </div>
    {/if}

    {#if loading}
        <div class="rm-stage-inner">
            <p class="state-text">Loading status data...</p>
        </div>
    {:else if errorMessage}
        <div class="rm-stage-inner">
            <p class="state-text error">{errorMessage}</p>
        </div>
    {:else if statusData}
        {#if view === "radar"}
            {#if selectedDimensions.length > 0}
                <div class="rm-radar-stage">
                    <RadarChart
                        dimensions={selectedDimensions}
                        onSelect={handleDimensionSelect}
                    />
                </div>
            {:else}
                <div class="rm-stage-inner">
                    <p class="state-text">No Status dimensions selected.</p>
                </div>
            {/if}
            <button
                type="button"
                class="rm-configure-btn"
                onclick={() => {
                    view = "configure";
                }}
            >
                Configure dimensions
            </button>
        {:else if view === "detail"}
            <StatusDetailView
                {statusData}
                {selectedDimensionId}
                onBack={handleDetailBack}
            />
        {:else}
            <div class="rm-configure-panel">
                <h2>Displayed dimensions</h2>
                <p>Choose up to five dimensions for the Status radar.</p>
                <div class="rm-slot-list">
                    {#each Array.from({ length: 5 }, (_, position) => position) as position}
                        {@const selected = statusData.dimensions.find(
                            (dimension) =>
                                dimension.selected_position === position,
                        )}
                        <label class="rm-slot-row">
                            <span>Slot {position + 1}</span>
                            <select
                                value={selected?.id ?? ""}
                                disabled={updatingPosition !== null}
                                onchange={(event) =>
                                    void updateSelection(
                                        position,
                                        event.currentTarget.value,
                                    )}
                            >
                                <option value="">Not selected</option>
                                {#each statusData.dimensions as dimension}
                                    <option value={dimension.id}>
                                        {dimension.name} · {dimension.pack_id}
                                    </option>
                                {/each}
                            </select>
                        </label>
                    {/each}
                </div>
            </div>
        {/if}
    {:else}
        <div class="rm-stage-inner">
            <p class="state-text">Status data is not available yet.</p>
        </div>
    {/if}
</section>

<style>
    .rm-player-panel {
        position: fixed;
        top: 2rem;
        left: 2rem;
        z-index: 10;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        pointer-events: none;
    }

    .rm-hint-board {
        width: 32rem;
        height: 8rem;
        background-repeat: no-repeat;
        background-size: 100% 100%;
        background-position: center;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .rm-hint-board--slim {
        height: 7rem;
        margin-top: -1rem;
    }

    .rm-hint-text {
        font-family:
            "Source Han Sans SC", "Noto Sans SC", "方正兰亭黑_GBK", "Microsoft YaHei", sans-serif;
        font-weight: 900;
        color: #ffffff;
        font-size: clamp(1rem, 1.8vw, 1.8rem);
        white-space: nowrap;
        line-height: 1;
        -webkit-text-stroke: 0.03em #000000;
        paint-order: stroke fill;
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

    .rm-stage-inner {
        flex: 1;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .rm-status-bg {
        position: absolute;
        inset: 0;
        background: var(--rm-black, #000);
        clip-path: polygon(50% 0%, 100% 0%, 100% 40%, 80% 100%, 13% 100%);
        z-index: 0;
        pointer-events: none;
    }

    /* ── Bottom-right stacked star groups decoration ── */
    .rm-status-stars-br {
        position: absolute;
        inset: 0;
        z-index: 0;
        pointer-events: none;
        overflow: hidden;
    }

    .rm-star-group {
        position: absolute;
        aspect-ratio: 1;
        pointer-events: none;
    }

    .rm-sstar {
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

    .rm-sstar-1 {
        background: #444444;
        transform: scale(0.92);
    }
    .rm-sstar-2 {
        background: var(--rm-black, #000);
        transform: scale(0.8);
    }
    .rm-sstar-3 {
        background: #444444;
        transform: scale(0.68);
    }
    .rm-sstar-4 {
        background: var(--rm-black, #000);
        transform: scale(0.56);
    }
    .rm-sstar-5 {
        background: #444444;
        transform: scale(0.44);
    }
    .rm-sstar-6 {
        background: var(--rm-black, #000);
        transform: scale(0.32);
    }
    .rm-sstar-7 {
        background: #444444;
        transform: scale(0.2);
    }
    .rm-sstar-8 {
        background: var(--rm-black, #000);
        transform: scale(0.08);
    }

    .rm-sg-1 {
        width: 90vh;
        top: 5%;
        right: -22%;
        transform: rotate(12deg);
    }

    .rm-sg-2 {
        width: 90vh;
        top: 20%;
        right: -20%;
        transform: rotate(-22deg);
    }

    .rm-sg-3 {
        width: 90vh;
        top: 32%;
        right: -15%;
        transform: rotate(0deg);
    }

    .rm-sg-4 {
        width: 90vh;
        top: 55%;
        right: -10%;
        transform: rotate(-8deg);
    }

    .rm-sg-5 {
        width: 90vh;
        top: 45%;
        right: -20%;
        transform: rotate(28deg);
    }

    .rm-radar-stage {
        flex: 1;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: clamp(1rem, 2vw, 3rem);
        padding-left: 8%;
    }

    .rm-configure-btn {
        position: fixed;
        right: clamp(2rem, 4vw, 6rem);
        bottom: clamp(2rem, 4vh, 5rem);
        z-index: 12;
        border: 0.2rem solid var(--rm-white, #fff);
        background: var(--rm-black, #000);
        color: var(--rm-white, #fff);
        padding: 0.65em 1em;
        font: 800 clamp(1rem, 1.4vw, 2rem) "p5hatty", "Orbitron", sans-serif;
        text-transform: uppercase;
        cursor: pointer;
        transform: rotate(-1deg);
    }

    .rm-configure-panel {
        position: relative;
        z-index: 2;
        width: min(58rem, 72vw);
        margin: clamp(8rem, 16vh, 15rem) auto 0;
        padding: clamp(1.5rem, 3vw, 3.5rem);
        background: var(--rm-black, #000);
        border: 0.35rem solid var(--rm-white, #fff);
        transform: rotate(-0.4deg);
    }

    .rm-configure-panel h2,
    .rm-configure-panel p {
        margin-top: 0;
    }

    .rm-configure-panel h2 {
        font-size: clamp(2rem, 3vw, 4rem);
        text-transform: uppercase;
    }

    .rm-configure-panel p {
        font-size: clamp(1rem, 1.3vw, 1.8rem);
    }

    .rm-slot-list {
        display: grid;
        gap: clamp(0.6rem, 1vh, 1rem);
    }

    .rm-slot-row {
        display: grid;
        grid-template-columns: minmax(7rem, 0.35fr) 1fr;
        align-items: center;
        gap: 1rem;
        font-size: clamp(1rem, 1.35vw, 1.9rem);
        font-weight: 800;
        text-transform: uppercase;
    }

    .rm-slot-row select {
        width: 100%;
        padding: 0.55em 0.7em;
        border: 0.2rem solid var(--rm-white, #fff);
        border-radius: 0;
        background: var(--rm-white, #fff);
        color: var(--rm-black, #000);
        font: 700 clamp(0.9rem, 1.1vw, 1.5rem) "p5hatty", "Orbitron", sans-serif;
    }
</style>
