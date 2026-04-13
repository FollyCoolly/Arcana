<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import RadarChart from "$lib/components/RadarChart.svelte";
    import StatusDetailView from "$lib/screens/StatusDetailView.svelte";
    import type { StatusData } from "$lib/types/status";
    import KeyHint from "$lib/KeyHint.svelte";
    import PromptWord from "$lib/PromptWord.svelte";

    let {
        onBack,
        statusData: initialStatusData,
    }: { onBack: () => void; statusData: StatusData | null } = $props();

    let loading = $state(false);
    let errorMessage = $state<string | null>(null);
    let statusData = $state<StatusData | null>(initialStatusData);
    let view = $state<"radar" | "detail">("radar");
    let selectedDimensionId = $state<string | null>(null);

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
            statusData = await invoke<StatusData>("load_status_data");
        } catch (error) {
            errorMessage =
                typeof error === "string"
                    ? error
                    : "Failed to load status data. Check data files in /data.";
            statusData = null;
        } finally {
            loading = false;
        }
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            event.preventDefault();
            if (view === "detail") {
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
    <img src="/ui/Status.png" alt="Status" class="rm-status-title-img" />

    <button
        type="button"
        class="rm-back-btn"
        onclick={() => {
            if (view === "detail") {
                handleDetailBack();
            } else {
                onBack();
            }
        }}
    >
        <KeyHint key="Esc" fontSize={36} />
        <PromptWord text="Back" fontSize={72} />
    </button>

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
            <div class="rm-radar-stage">
                <RadarChart
                    dimensions={statusData.dimensions.filter((d) => d.enabled)}
                    onSelect={handleDimensionSelect}
                />
            </div>
        {:else}
            <StatusDetailView
                {statusData}
                {selectedDimensionId}
                onBack={handleDetailBack}
            />
        {/if}
    {:else}
        <div class="rm-stage-inner">
            <p class="state-text">Status data is not available yet.</p>
        </div>
    {/if}
</section>

<style>
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
</style>
