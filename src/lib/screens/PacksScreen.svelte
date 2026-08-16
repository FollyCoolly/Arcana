<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import KeyHint from "$lib/KeyHint.svelte";
    import PromptWord from "$lib/PromptWord.svelte";
    import { dataCommandErrorMessage } from "$lib/types/data_platform";
    import type {
        PackDashboardData,
        PackDeleteResult,
        PackEnabledState,
        PackSummary,
    } from "$lib/types/pack";

    let {
        onBack,
        onPacksChanged,
    }: {
        onBack: () => void;
        onPacksChanged?: () => void;
    } = $props();

    let packs = $state<PackSummary[]>([]);
    let loading = $state(false);
    let busyPackId = $state<string | null>(null);
    let error = $state<string | null>(null);
    let feedback = $state<string | null>(null);
    let selectedIndex = $state(0);
    let pendingDeleteId = $state<string | null>(null);
    let pendingDeleteImpact = $state<PackDeleteResult | null>(null);

    function comparePacks(left: PackSummary, right: PackSummary) {
        if (left.enabled !== right.enabled) return left.enabled ? -1 : 1;
        return left.name.localeCompare(right.name);
    }

    let sortedPacks = $derived.by(() => [...packs].sort(comparePacks));
    let selectedPack = $derived(sortedPacks[selectedIndex] ?? null);
    let enabledCount = $derived(packs.filter((pack) => pack.enabled).length);

    $effect(() => {
        if (selectedIndex >= sortedPacks.length) {
            selectedIndex = Math.max(0, sortedPacks.length - 1);
        }
    });

    async function loadPacks() {
        loading = true;
        error = null;
        const preferredId = selectedPack?.id;
        try {
            const data = await invoke<PackDashboardData>("load_pack_dashboard");
            packs = data.packs;
            if (preferredId) {
                const nextIndex = [...data.packs]
                    .sort(comparePacks)
                    .findIndex((pack) => pack.id === preferredId);
                if (nextIndex >= 0) selectedIndex = nextIndex;
            }
        } catch (cause) {
            error = dataCommandErrorMessage(
                cause,
                "Failed to load Packs from the local database.",
            );
        } finally {
            loading = false;
        }
    }

    async function togglePack(pack: PackSummary) {
        busyPackId = pack.id;
        pendingDeleteId = null;
        pendingDeleteImpact = null;
        feedback = null;
        error = null;
        try {
            const state = await invoke<PackEnabledState>("set_pack_enabled", {
                packId: pack.id,
                enabled: !pack.enabled,
            });
            feedback = `${pack.name} ${state.enabled ? "enabled" : "disabled"}.`;
            await loadPacks();
            onPacksChanged?.();
        } catch (cause) {
            error = dataCommandErrorMessage(cause, "Failed to update Pack state.");
        } finally {
            busyPackId = null;
        }
    }

    async function deletePack(pack: PackSummary) {
        if (pack.enabled) {
            error = "Disable this Pack before deleting it from the desktop UI.";
            return;
        }
        if (pendingDeleteId !== pack.id) {
            busyPackId = pack.id;
            feedback = null;
            error = null;
            try {
                pendingDeleteImpact = await invoke<PackDeleteResult>(
                    "preview_pack_deletion",
                    { packId: pack.id },
                );
                pendingDeleteId = pack.id;
                feedback = `Review the impact, then press Confirm Delete to remove ${pack.name}.`;
            } catch (cause) {
                error = dataCommandErrorMessage(
                    cause,
                    "Failed to preview Pack deletion.",
                );
            } finally {
                busyPackId = null;
            }
            return;
        }

        busyPackId = pack.id;
        feedback = null;
        error = null;
        try {
            const result = await invoke<PackDeleteResult>("delete_pack", {
                packId: pack.id,
            });
            const preserved =
                result.unresolved_record_ids.length +
                result.unresolved_achievement_state_ids.length;
            feedback = preserved
                ? `${pack.name} deleted; ${preserved} user data entr${preserved === 1 ? "y" : "ies"} preserved as unresolved.`
                : `${pack.name} deleted.`;
            pendingDeleteId = null;
            pendingDeleteImpact = null;
            await loadPacks();
            onPacksChanged?.();
        } catch (cause) {
            error = dataCommandErrorMessage(cause, "Failed to delete Pack.");
        } finally {
            busyPackId = null;
        }
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            event.preventDefault();
            if (pendingDeleteId) {
                pendingDeleteId = null;
                pendingDeleteImpact = null;
                feedback = null;
            } else {
                onBack();
            }
            return;
        }
        if (event.key === "ArrowDown" && sortedPacks.length) {
            event.preventDefault();
            selectedIndex = (selectedIndex + 1) % sortedPacks.length;
        } else if (event.key === "ArrowUp" && sortedPacks.length) {
            event.preventDefault();
            selectedIndex =
                (selectedIndex - 1 + sortedPacks.length) % sortedPacks.length;
        }
    }

    onMount(() => {
        void loadPacks();
        window.addEventListener("keydown", handleKeydown);
        return () => window.removeEventListener("keydown", handleKeydown);
    });
</script>

<section class="packs-stage">
    <div class="slash slash-a" aria-hidden="true"></div>
    <div class="slash slash-b" aria-hidden="true"></div>

    <header class="packs-header">
        <p class="eyebrow">CONTENT LOADOUT</p>
        <h1>PACKS</h1>
        <p>{enabledCount}/{packs.length} ENABLED</p>
    </header>

    <button type="button" class="back-button" onclick={onBack}>
        <KeyHint key="Esc" fontSize={30} />
        <PromptWord text="Back" fontSize={58} />
    </button>

    {#if loading && packs.length === 0}
        <p class="state-message">Loading Packs...</p>
    {:else if error && packs.length === 0}
        <p class="state-message error">{error}</p>
    {:else if sortedPacks.length === 0}
        <p class="state-message">No Packs installed.</p>
    {:else}
        <div class="pack-layout">
            <div class="pack-list" aria-label="Installed Packs">
                {#each sortedPacks as pack, index}
                    <div
                        class="pack-row"
                        class:selected={selectedIndex === index}
                        class:disabled={!pack.enabled}
                    >
                        <button
                            type="button"
                            class="pack-select"
                            onclick={() => {
                                selectedIndex = index;
                                pendingDeleteId = null;
                                pendingDeleteImpact = null;
                            }}
                        >
                            <span class="state-mark">{pack.enabled ? "ON" : "OFF"}</span>
                            <span class="pack-name">{pack.name}</span>
                            <span class="pack-id">{pack.id}</span>
                        </button>
                    </div>
                {/each}
            </div>

            {#if selectedPack}
                <article class="pack-detail">
                    <div class="detail-heading">
                        <div>
                            <p class="detail-id">{selectedPack.id}</p>
                            <h2>{selectedPack.name}</h2>
                        </div>
                        <span class:enabled={selectedPack.enabled} class="status-stamp">
                            {selectedPack.enabled ? "ACTIVE" : "DISABLED"}
                        </span>
                    </div>

                    {#if selectedPack.parent_pack_id}
                        <p class="parent">PARENT / {selectedPack.parent_pack_id}</p>
                    {/if}

                    <div class="count-grid">
                        <div><strong>{selectedPack.record_definition_count}</strong><span>RECORDS</span></div>
                        <div><strong>{selectedPack.dimension_count}</strong><span>STATUS</span></div>
                        <div><strong>{selectedPack.achievement_count}</strong><span>ACHIEVEMENTS</span></div>
                        <div><strong>{selectedPack.skill_count}</strong><span>SKILLS</span></div>
                        <div><strong>{selectedPack.asset_count}</strong><span>ASSETS</span></div>
                    </div>

                    {#if selectedPack.tags.length}
                        <div class="tags">
                            {#each selectedPack.tags as tag}<span>#{tag}</span>{/each}
                        </div>
                    {/if}

                    <div class="actions">
                        <button
                            type="button"
                            class="toggle-button"
                            disabled={busyPackId === selectedPack.id}
                            onclick={() => void togglePack(selectedPack)}
                        >
                            {selectedPack.enabled ? "DISABLE PACK" : "ENABLE PACK"}
                        </button>
                        <button
                            type="button"
                            class="delete-button"
                            class:confirming={pendingDeleteId === selectedPack.id}
                            disabled={selectedPack.enabled || busyPackId === selectedPack.id}
                            onclick={() => void deletePack(selectedPack)}
                        >
                            {pendingDeleteId === selectedPack.id ? "CONFIRM DELETE" : "DELETE"}
                        </button>
                    </div>
                    {#if pendingDeleteId === selectedPack.id && pendingDeleteImpact}
                        <div class="delete-impact">
                            <strong>DELETE IMPACT</strong>
                            <span>{pendingDeleteImpact.child_pack_ids.length} child Packs lose their parent</span>
                            <span>{pendingDeleteImpact.unresolved_record_ids.length} Records become unresolved</span>
                            <span>{pendingDeleteImpact.unresolved_achievement_state_ids.length} Achievement states become unresolved</span>
                            <span>{pendingDeleteImpact.orphaned_status_dimension_ids.length} Status selections become unavailable</span>
                        </div>
                    {/if}
                    {#if selectedPack.enabled}
                        <p class="delete-hint">Disable the Pack before deleting it.</p>
                    {/if}
                </article>
            {/if}
        </div>
    {/if}

    <footer class="feedback-bar">
        {#if error}<span class="error">{error}</span>{:else if feedback}<span>{feedback}</span>{:else}<span>↑ ↓ Select · Use action buttons</span>{/if}
    </footer>
</section>

<style>
    .packs-stage {
        position: fixed;
        inset: 0;
        overflow: hidden;
        color: #fff;
        background:
            linear-gradient(116deg, transparent 0 54%, rgba(229, 25, 28, 0.95) 54% 61%, transparent 61%),
            radial-gradient(circle at 22% 82%, #242424 0 2px, transparent 3px) 0 0 / 18px 18px,
            #050505;
        font-family: "Source Han Sans SC", "Microsoft YaHei", sans-serif;
    }

    .slash {
        position: absolute;
        width: 55vw;
        height: 9vh;
        background: #fff;
        transform: rotate(-11deg);
        opacity: 0.08;
        pointer-events: none;
    }
    .slash-a { top: 18%; left: -8%; }
    .slash-b { bottom: 12%; right: -10%; transform: rotate(8deg); }

    .packs-header {
        position: absolute;
        top: 5vh;
        left: 6vw;
        z-index: 2;
        transform: rotate(-4deg);
    }
    .packs-header p { margin: 0; font-weight: 900; letter-spacing: 0.16em; }
    .packs-header h1 {
        margin: -0.12em 0;
        color: #e5191c;
        font-family: "p5hatty", Impact, sans-serif;
        font-size: clamp(5rem, 10vw, 10rem);
        line-height: 0.8;
        text-shadow: 0.045em 0.045em 0 #fff;
    }
    .eyebrow { color: #fff; }

    .back-button {
        position: absolute;
        right: 3vw;
        top: 3vh;
        z-index: 5;
        display: flex;
        align-items: center;
        gap: 0.4rem;
        border: 0;
        color: #fff;
        background: transparent;
        cursor: pointer;
    }

    .pack-layout {
        position: absolute;
        left: 6vw;
        right: 6vw;
        top: 24vh;
        bottom: 11vh;
        z-index: 2;
        display: grid;
        grid-template-columns: minmax(310px, 0.9fr) minmax(430px, 1.35fr);
        gap: clamp(1.5rem, 4vw, 5rem);
    }

    .pack-list {
        overflow-y: auto;
        padding: 0.8rem 1.2rem 0.8rem 0.5rem;
    }
    .pack-row { margin: 0.45rem 0; transform: skew(-7deg); }
    .pack-select {
        width: 100%;
        display: grid;
        grid-template-columns: 3.5rem 1fr;
        gap: 0.15rem 0.7rem;
        padding: 0.8rem 1rem;
        border: 2px solid #fff;
        color: #fff;
        text-align: left;
        background: #080808;
        cursor: pointer;
    }
    .pack-row.selected .pack-select { color: #000; background: #fff; box-shadow: 0.5rem 0.5rem 0 #e5191c; }
    .pack-row.disabled:not(.selected) .pack-select { color: #aaa; border-color: #555; }
    .state-mark {
        grid-row: 1 / 3;
        align-self: center;
        color: #e5191c;
        font-family: Impact, sans-serif;
        font-size: 1.45rem;
    }
    .pack-name { font-size: clamp(1.2rem, 2vw, 2rem); font-weight: 900; }
    .pack-id { font-family: monospace; font-size: 0.78rem; opacity: 0.7; }

    .pack-detail {
        position: relative;
        align-self: center;
        padding: clamp(1.5rem, 3vw, 3rem);
        color: #000;
        background: #fff;
        clip-path: polygon(2% 3%, 98% 0, 100% 94%, 4% 100%, 0 42%);
        transform: rotate(1deg);
    }
    .detail-heading { display: flex; justify-content: space-between; gap: 1rem; align-items: flex-start; }
    .detail-heading h2 { margin: 0; font-size: clamp(2rem, 4vw, 4.5rem); line-height: 0.95; }
    .detail-id, .parent { margin: 0 0 0.35rem; font-family: monospace; font-weight: 700; }
    .status-stamp { padding: 0.35rem 0.65rem; color: #fff; background: #555; font-weight: 900; transform: rotate(5deg); }
    .status-stamp.enabled { background: #e5191c; }
    .parent { margin-top: 1.2rem; color: #555; }

    .count-grid {
        display: grid;
        grid-template-columns: repeat(5, 1fr);
        gap: 0.45rem;
        margin: 2rem 0 1.5rem;
    }
    .count-grid div { display: flex; flex-direction: column; padding: 0.75rem 0.4rem; color: #fff; text-align: center; background: #000; }
    .count-grid strong { color: #e5191c; font-size: clamp(1.6rem, 3vw, 3rem); line-height: 1; }
    .count-grid span { font-size: 0.65rem; font-weight: 900; }
    .tags { display: flex; flex-wrap: wrap; gap: 0.45rem; min-height: 1.5rem; }
    .tags span { padding: 0.2rem 0.5rem; color: #fff; background: #e5191c; font-weight: 800; }

    .actions { display: flex; gap: 0.8rem; margin-top: 2rem; }
    .actions button {
        border: 3px solid #000;
        padding: 0.75rem 1.2rem;
        font: 900 1rem/1 "Source Han Sans SC", sans-serif;
        cursor: pointer;
    }
    .toggle-button { color: #fff; background: #000; }
    .toggle-button:hover { background: #e5191c; }
    .delete-button { color: #000; background: #fff; }
    .delete-button.confirming { color: #fff; background: #e5191c; }
    .actions button:disabled { cursor: not-allowed; opacity: 0.35; }
    .delete-hint { margin: 0.55rem 0 0; color: #555; font-size: 0.8rem; }
    .delete-impact {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 0.35rem 1rem;
        margin-top: 1rem;
        padding: 0.8rem 1rem;
        color: #fff;
        background: #e5191c;
        font-size: 0.78rem;
    }
    .delete-impact strong { grid-column: 1 / -1; font-size: 1rem; }

    .state-message { position: absolute; left: 50%; top: 52%; transform: translate(-50%, -50%); font-size: 1.5rem; font-weight: 900; }
    .feedback-bar {
        position: absolute;
        left: 6vw;
        right: 6vw;
        bottom: 4vh;
        z-index: 3;
        min-height: 1.5rem;
        padding: 0.55rem 1rem;
        color: #fff;
        background: #000;
        border-left: 0.65rem solid #e5191c;
        font-weight: 800;
    }
    .error { color: #ff5252; }

    @media (max-width: 900px) {
        .pack-layout { top: 21vh; grid-template-columns: 1fr; gap: 1rem; }
        .pack-list { max-height: 31vh; }
        .pack-detail { align-self: stretch; padding: 1.2rem 1.6rem; }
        .count-grid { margin: 1rem 0; }
        .packs-header h1 { font-size: clamp(4rem, 16vw, 7rem); }
    }
</style>
