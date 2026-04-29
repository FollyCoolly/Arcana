<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import CollageLabel from "$lib/CollageLabel.svelte";
    import type {
        SkillData,
        SkillWithLevel,
        SkillNode,
    } from "$lib/types/skill";
    import type { AchievementData } from "$lib/types/achievement";
    import type { UiEvent } from "$lib/types/ui_event";
    import { formatGroupName } from "$lib/utils/format";
    import KeyHint from "$lib/KeyHint.svelte";
    import PromptWord from "$lib/PromptWord.svelte";

    let {
        onBack,
        achievementData,
        onAchievementDataLoaded,
    }: {
        onBack: () => void;
        achievementData: AchievementData | null;
        onAchievementDataLoaded?: (data: AchievementData) => void;
    } = $props();

    let skillLoading = $state(false);
    let skillError = $state<string | null>(null);
    let skillData = $state<SkillData | null>(null);
    let selectedIndex = $state(0);

    /** Achievement IDs that changed since last view (from ui_events) */
    let changedAchievementIds = $state<Set<string>>(new Set());

    /** Achievement currently shown in the detail modal, or null */
    let detailAchievementId = $state<string | null>(null);

    function findAchievement(id: string) {
        if (!achievementData) return null;
        for (const pack of achievementData.packs) {
            for (const ach of pack.achievements) {
                if (ach.id === id) return ach;
            }
        }
        return null;
    }

    let detailAchievement = $derived(
        detailAchievementId ? findAchievement(detailAchievementId) : null,
    );

    let detailProgress = $derived(
        detailAchievementId
            ? (achievementData?.progress[detailAchievementId] ?? null)
            : null,
    );

    function openNodeDetail(achievementId: string) {
        detailAchievementId = achievementId;
    }

    function closeNodeDetail() {
        detailAchievementId = null;
        toggleError = null;
    }

    function getDifficultyLabel(difficulty: string): string {
        return difficulty.charAt(0).toUpperCase() + difficulty.slice(1);
    }

    let toggleBusy = $state(false);
    let toggleError = $state<string | null>(null);

    /** IDs of direct prerequisites that are not yet achieved. */
    function unmetPrereqs(achievementId: string): string[] {
        const ach = findAchievement(achievementId);
        if (!ach || ach.prerequisites.length === 0) return [];
        return ach.prerequisites.filter(
            (p) => achievementData?.progress[p]?.status !== "achieved",
        );
    }

    let detailUnmetPrereqs = $derived(
        detailAchievementId ? unmetPrereqs(detailAchievementId) : [],
    );

    /** True when the toggle button should be shown:
     *  - already achieved (button locks it), OR
     *  - not achieved and all prereqs are met (button unlocks it) */
    let canShowToggle = $derived.by(() => {
        if (!detailAchievementId || !detailAchievement) return false;
        if (detailProgress?.status === "achieved") return true;
        return detailUnmetPrereqs.length === 0;
    });

    async function toggleAchievement() {
        if (!detailAchievementId || toggleBusy) return;
        toggleBusy = true;
        toggleError = null;
        const isAchieved = detailProgress?.status === "achieved";
        try {
            if (isAchieved) {
                await invoke<string>("lock_achievement", {
                    achievementId: detailAchievementId,
                });
            } else {
                await invoke<string>("set_achievement_achieved", {
                    achievementId: detailAchievementId,
                });
            }
            const fresh =
                await invoke<AchievementData>("load_achievements");
            onAchievementDataLoaded?.(fresh);
        } catch (e) {
            toggleError = typeof e === "string" ? e : "Operation failed.";
        } finally {
            toggleBusy = false;
        }
    }

    let visibleSkills = $derived(
        skillData ? skillData.skills.filter((s) => s.current_level > 0) : [],
    );

    let selectedSkill = $derived(
        visibleSkills.length > 0 ? visibleSkills[selectedIndex] : null,
    );

    let totalSkills = $derived(visibleSkills.length);

    let currentLevelTitle = $derived.by(() => {
        if (!selectedSkill) return null;
        const titles = selectedSkill.skill.level_titles;
        if (!titles || titles.length === 0 || selectedSkill.current_level === 0)
            return null;
        const idx = Math.min(selectedSkill.current_level, titles.length) - 1;
        return titles[idx] ?? null;
    });

    const ROMAN_NUMERALS = [
        "0",
        "I",
        "II",
        "III",
        "IV",
        "V",
        "VI",
        "VII",
        "VIII",
        "IX",
        "X",
    ];

    function toRomanNumeral(n: number): string {
        return ROMAN_NUMERALS[n] ?? String(n);
    }

    function getSkillProgressPercent(s: SkillWithLevel): number {
        if (s.max_points === 0) return 0;
        return (s.current_points / s.max_points) * 100;
    }

    function isNodeUnlocked(achievementId: string): boolean {
        return !!achievementData?.progress[achievementId];
    }

    function isNodeNew(achievementId: string): boolean {
        return changedAchievementIds.has(achievementId);
    }

    function getAchievementName(achievementId: string): string {
        if (!achievementData) return achievementId;
        for (const pack of achievementData.packs) {
            for (const ach of pack.achievements) {
                if (ach.id === achievementId) return ach.name;
            }
        }
        const after = achievementId.split("::")[1];
        return after ? formatGroupName(after) : achievementId;
    }

    function sortNodes(
        nodes: SkillNode[],
        data: AchievementData | null,
    ): SkillNode[] {
        if (nodes.length <= 1) return [...nodes];

        const COLS = 8;

        const nodeIds = new Set(nodes.map((n) => n.achievement_id));
        const prereqMap = new Map<string, string[]>();
        if (data) {
            for (const pack of data.packs) {
                for (const ach of pack.achievements) {
                    if (!nodeIds.has(ach.id)) continue;
                    const local = ach.prerequisites.filter((p) =>
                        nodeIds.has(p),
                    );
                    if (local.length > 0) prereqMap.set(ach.id, local);
                }
            }
        }

        function hexCol(i: number): number {
            let rem = i,
                row = 0;
            while (true) {
                const rc = row % 2 === 0 ? COLS : COLS - 1;
                if (rem < rc) return rem;
                rem -= rc;
                row++;
            }
        }

        // Step 1: sort by points ascending
        const arr = [...nodes].sort((a, b) => a.points - b.points);

        // Step 2: topo sort within each same-points window (prereqs before dependents)
        let wi = 0;
        while (wi < arr.length) {
            let wj = wi;
            while (wj < arr.length && arr[wj].points === arr[wi].points) wj++;
            if (wj - wi > 1) {
                const winIds = new Set(
                    arr.slice(wi, wj).map((n) => n.achievement_id),
                );
                const inDeg = new Map<string, number>();
                const fwd = new Map<string, string[]>();
                for (let k = wi; k < wj; k++) {
                    inDeg.set(arr[k].achievement_id, 0);
                    fwd.set(arr[k].achievement_id, []);
                }
                for (let k = wi; k < wj; k++) {
                    for (const pid of prereqMap.get(arr[k].achievement_id) ??
                        []) {
                        if (winIds.has(pid)) {
                            fwd.get(pid)!.push(arr[k].achievement_id);
                            inDeg.set(
                                arr[k].achievement_id,
                                inDeg.get(arr[k].achievement_id)! + 1,
                            );
                        }
                    }
                }
                const queue = [...inDeg.entries()]
                    .filter(([, d]) => d === 0)
                    .map(([id]) => id);
                const order: string[] = [];
                const byId = new Map(
                    arr.slice(wi, wj).map((n) => [n.achievement_id, n]),
                );
                while (queue.length > 0) {
                    const id = queue.shift()!;
                    order.push(id);
                    for (const dep of fwd.get(id) ?? []) {
                        const d = inDeg.get(dep)! - 1;
                        inDeg.set(dep, d);
                        if (d === 0) queue.push(dep);
                    }
                }
                if (order.length === wj - wi) {
                    for (let k = wi; k < wj; k++)
                        arr[k] = byId.get(order[k - wi])!;
                }
            }
            wi = wj;
        }

        // Step 3: best-effort column alignment — swap within same-points window
        // to place a node in the same column as its already-placed prerequisite
        const placed = new Map<string, number>();
        wi = 0;
        while (wi < arr.length) {
            let wj = wi;
            while (wj < arr.length && arr[wj].points === arr[wi].points) wj++;
            for (let k = wi; k < wj; k++) {
                let targetCol: number | null = null;
                for (const pid of prereqMap.get(arr[k].achievement_id) ?? []) {
                    const pp = placed.get(pid);
                    if (pp !== undefined) {
                        targetCol = hexCol(pp);
                        break;
                    }
                }
                if (targetCol !== null && hexCol(k) !== targetCol) {
                    for (let m = k + 1; m < wj; m++) {
                        if (hexCol(m) === targetCol) {
                            [arr[k], arr[m]] = [arr[m], arr[k]];
                            break;
                        }
                    }
                }
                placed.set(arr[k].achievement_id, k);
            }
            wi = wj;
        }

        return arr;
    }

    let sortedNodes = $derived(
        selectedSkill
            ? sortNodes(selectedSkill.skill.nodes, achievementData)
            : [],
    );

    /** Set of skill IDs that have newly unlocked nodes */
    let skillsWithNewNodes = $derived.by(() => {
        if (!skillData || changedAchievementIds.size === 0)
            return new Set<string>();
        const ids = new Set<string>();
        for (const s of skillData.skills) {
            if (
                s.skill.nodes.some((n) =>
                    changedAchievementIds.has(n.achievement_id),
                )
            ) {
                ids.add(s.skill.id);
            }
        }
        return ids;
    });

    function computeHexRows(nodes: SkillNode[], cols: number): SkillNode[][] {
        const rows: SkillNode[][] = [];
        let idx = 0;
        let rowIdx = 0;
        while (idx < nodes.length) {
            const rowCols = rowIdx % 2 === 0 ? cols : cols - 1;
            rows.push(nodes.slice(idx, idx + rowCols));
            idx += rowCols;
            rowIdx++;
        }
        return rows;
    }

    function navigatePrev() {
        if (totalSkills <= 1) return;
        selectedIndex = (selectedIndex - 1 + totalSkills) % totalSkills;
    }

    function navigateNext() {
        if (totalSkills <= 1) return;
        selectedIndex = (selectedIndex + 1) % totalSkills;
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            event.preventDefault();
            if (detailAchievementId) {
                closeNodeDetail();
            } else {
                onBack();
            }
        } else if (event.key === "q" || event.key === "Q") {
            if (detailAchievementId) return;
            event.preventDefault();
            navigatePrev();
        } else if (event.key === "e" || event.key === "E") {
            if (detailAchievementId) return;
            event.preventDefault();
            navigateNext();
        }
    }

    async function loadSkillData() {
        skillLoading = true;
        skillError = null;

        try {
            const [skills, events] = await Promise.all([
                invoke<SkillData>("load_skills"),
                invoke<UiEvent[]>("get_pending_events", {
                    eventType: "achievement_status_change",
                }),
            ]);
            skillData = skills;
            selectedIndex = 0;

            // Extract changed achievement IDs from consumed events
            const ids = new Set<string>();
            for (const evt of events) {
                const achId = evt.data?.achievement_id;
                if (typeof achId === "string") {
                    ids.add(achId);
                }
            }
            changedAchievementIds = ids;
        } catch (error) {
            skillError =
                typeof error === "string"
                    ? error
                    : "Failed to load skill data.";
            skillData = null;
        } finally {
            skillLoading = false;
        }
    }

    onMount(() => {
        if (!skillData && !skillLoading) {
            void loadSkillData();
        }

        window.addEventListener("keydown", handleKeydown);
        return () => {
            window.removeEventListener("keydown", handleKeydown);
        };
    });
</script>

<section class="rm-stage">
    <!-- Bottom-left key hints -->
    <div class="rm-skills-hints">
        <button
            type="button"
            class="rm-back-btn rm-back-btn--inline"
            onclick={() => onBack()}
        >
            <KeyHint key="Esc" fontSize={36} />
            <PromptWord text="Back" fontSize={72} />
        </button>

        {#if totalSkills > 1}
            <div class="rm-nav-hint-group">
                <button
                    type="button"
                    class="rm-nav-hint-btn"
                    onclick={() => navigatePrev()}
                >
                    <KeyHint key="Q" fontSize={36} />
                    <PromptWord text="Prev" fontSize={72} />
                </button>

                <button
                    type="button"
                    class="rm-nav-hint-btn"
                    onclick={() => navigateNext()}
                >
                    <KeyHint key="E" fontSize={36} />
                    <PromptWord text="Next" fontSize={72} />
                </button>
            </div>
        {/if}
    </div>

    {#if skillLoading}
        <p class="state-text" style="padding: 2rem;">Loading skills...</p>
    {:else if skillError}
        <p class="state-text error" style="padding: 2rem;">{skillError}</p>
    {:else if skillData && selectedSkill}
        <div class="rm-skill-detail">
            <div class="rm-skill-detail-left">
                <div class="rm-skill-detail-header">
                    <CollageLabel text={selectedSkill.skill.name} />
                    <span class="rm-skill-level-badge">
                        <span
                            class="rm-skill-lv-frag"
                            style:transform="rotate(-3deg)">Lv.</span
                        >
                        <span
                            class="rm-skill-lv-frag rm-skill-lv-inv"
                            style:transform="rotate(4deg)"
                            >{selectedSkill.current_level >=
                            selectedSkill.skill.max_level
                                ? "MAX"
                                : selectedSkill.current_level}</span
                        >
                    </span>
                    {#if currentLevelTitle}
                        <CollageLabel text={currentLevelTitle} />
                    {/if}
                </div>

                <div class="rm-skill-image-card">
                    <img
                        src={selectedSkill.skill.card_image ??
                            "/card_examples/fool.png"}
                        alt={selectedSkill.skill.name}
                    />
                </div>

                {#if selectedSkill.skill.description}
                    <p class="rm-skill-description">
                        {selectedSkill.skill.description}
                    </p>
                {/if}
            </div>

            <div class="rm-skill-detail-right">
                <div class="rm-skill-node-grid" style="--cols: 8">
                    {#each computeHexRows(sortedNodes, 8) as row, rowIdx}
                        <div
                            class="rm-hex-row"
                            class:rm-hex-row--odd={rowIdx % 2 === 1}
                        >
                            {#each row as node}
                                {@const unlocked = isNodeUnlocked(
                                    node.achievement_id,
                                )}
                                {@const isNew =
                                    unlocked && isNodeNew(node.achievement_id)}
                                <button
                                    type="button"
                                    class="rm-hex-border"
                                    class:rm-hex-border--unlocked={unlocked}
                                    class:rm-hex-border--new={isNew}
                                    onclick={() =>
                                        openNodeDetail(node.achievement_id)}
                                    aria-label={getAchievementName(
                                        node.achievement_id,
                                    )}
                                >
                                    <span
                                        class="rm-skill-node-hex"
                                        class:rm-skill-node-hex--unlocked={unlocked}
                                        class:rm-skill-node-hex--new={isNew}
                                    >
                                        <span class="rm-node-name"
                                            >{getAchievementName(
                                                node.achievement_id,
                                            )}</span
                                        >
                                        <span class="rm-node-points"
                                            >{node.points} pt</span
                                        >
                                    </span>
                                </button>
                            {/each}
                        </div>
                    {/each}
                </div>
            </div>
        </div>
    {:else}
        <p class="state-text" style="padding: 2rem;">
            No skills available yet.
        </p>
    {/if}

    {#if detailAchievementId}
        {@const ach = detailAchievement}
        {@const prog = detailProgress}
        {@const unlocked = !!prog}
        <div
            class="rm-node-modal-backdrop"
            role="presentation"
            onclick={closeNodeDetail}
        >
            <div
                class="rm-node-modal"
                role="dialog"
                aria-modal="true"
                aria-label="Achievement detail"
                tabindex="-1"
                onclick={(e) => e.stopPropagation()}
                onkeydown={(e) => e.stopPropagation()}
            >
                <header class="rm-node-modal-header">
                    <span class="rm-node-modal-status-icon"
                        >{unlocked ? "✓" : "○"}</span
                    >
                    <span class="rm-node-modal-title"
                        >{ach?.name ??
                            getAchievementName(detailAchievementId)}</span
                    >
                    {#if ach}
                        <span
                            class="rm-difficulty rm-difficulty--{ach.difficulty}"
                            >{getDifficultyLabel(ach.difficulty)}</span
                        >
                    {/if}
                </header>

                {#if ach}
                    <p class="rm-node-modal-desc">{ach.description}</p>
                {:else}
                    <p class="rm-node-modal-desc rm-node-modal-missing">
                        Achievement metadata not found.
                    </p>
                {/if}

                <dl class="rm-node-modal-meta">
                    <div class="rm-node-modal-meta-row">
                        <dt>Status</dt>
                        <dd>
                            {#if unlocked && prog?.status === "achieved"}
                                <span class="rm-node-modal-badge rm-unlocked"
                                    >Achieved</span
                                >
                            {:else if unlocked && prog?.status === "tracked"}
                                <span class="rm-node-modal-badge rm-tracked"
                                    >Tracked</span
                                >
                            {:else}
                                <span class="rm-node-modal-badge rm-locked"
                                    >Locked</span
                                >
                            {/if}
                        </dd>
                    </div>

                    {#if prog?.achieved_at}
                        <div class="rm-node-modal-meta-row">
                            <dt>Achieved</dt>
                            <dd>{prog.achieved_at}</dd>
                        </div>
                    {/if}

                    {#if prog?.tracked_at}
                        <div class="rm-node-modal-meta-row">
                            <dt>Tracked</dt>
                            <dd>{prog.tracked_at}</dd>
                        </div>
                    {/if}

                    {#if ach && ach.tags.length > 0}
                        <div class="rm-node-modal-meta-row">
                            <dt>Tags</dt>
                            <dd>
                                <div class="rm-node-modal-tags">
                                    {#each ach.tags as tag}
                                        <span class="rm-node-modal-tag"
                                            >{tag}</span
                                        >
                                    {/each}
                                </div>
                            </dd>
                        </div>
                    {/if}

                    {#if ach && ach.prerequisites.length > 0}
                        <div class="rm-node-modal-meta-row">
                            <dt>Prereqs</dt>
                            <dd>
                                <div class="rm-node-modal-tags">
                                    {#each ach.prerequisites as prereq}
                                        <span class="rm-node-modal-tag"
                                            >{prereq
                                                .split("::")[1]
                                                ?.replace(/_/g, " ") ??
                                                prereq}</span
                                        >
                                    {/each}
                                </div>
                            </dd>
                        </div>
                    {/if}
                </dl>

                {#if canShowToggle}
                    <div class="rm-node-modal-actions">
                        <button
                            type="button"
                            class="rm-node-modal-action"
                            class:rm-node-modal-action--lock={unlocked &&
                                prog?.status === "achieved"}
                            disabled={toggleBusy}
                            onclick={toggleAchievement}
                        >
                            {#if toggleBusy}
                                …
                            {:else if unlocked && prog?.status === "achieved"}
                                Lock
                            {:else}
                                Unlock
                            {/if}
                        </button>
                        {#if toggleError}
                            <span class="rm-node-modal-action-error"
                                >{toggleError}</span
                            >
                        {/if}
                    </div>
                {/if}

                {#if prog?.note}
                    <p class="rm-node-modal-note">{prog.note}</p>
                {/if}

                {#if prog?.progress_detail && prog.progress_detail.length > 0}
                    <div class="rm-node-modal-progress">
                        <div class="rm-node-modal-progress-label">Progress</div>
                        <ul class="rm-node-modal-progress-list">
                            {#each prog.progress_detail as line}
                                <li>{line}</li>
                            {/each}
                        </ul>
                    </div>
                {/if}
            </div>
        </div>
    {/if}
</section>

<style>
    /* ── Bottom-left hints container ── */
    .rm-skills-hints {
        position: fixed;
        bottom: clamp(1.5rem, 3vh, 3.5rem);
        left: clamp(1.5rem, 3vw, 4rem);
        z-index: 10;
        display: flex;
        align-items: flex-end;
        gap: clamp(1.5rem, 2vw, 3rem);
    }

    /* Override the global .rm-back-btn positioning so it flows inline */
    .rm-back-btn--inline {
        position: static;
        display: flex;
        align-items: center;
        gap: 0;
        background: none;
        border: none;
        cursor: pointer;
        padding: 0;
        transform: rotate(2deg);
        transition: transform 120ms ease;
    }
    .rm-back-btn--inline:hover {
        transform: rotate(2deg) scale(1.06);
    }
    .rm-back-btn--inline :global(.p5-prompt-word) {
        margin-left: -1rem;
    }

    .rm-nav-hint-group {
        display: flex;
        align-items: center;
        gap: clamp(0.6rem, 1vw, 1.5rem);
    }

    .rm-nav-hint-btn {
        display: flex;
        align-items: center;
        gap: 0;
        background: none;
        border: none;
        cursor: pointer;
        padding: 0;
        transform: rotate(-1deg);
        transition: transform 120ms ease;
    }
    .rm-nav-hint-btn:hover {
        transform: rotate(-1deg) scale(1.06);
    }
    .rm-nav-hint-btn :global(.p5-prompt-word) {
        margin-left: -1rem;
    }

    /* ── Skill detail layout ── */
    .rm-skill-detail {
        flex: 1;
        display: grid;
        grid-template-columns: 1fr 2fr;
        gap: clamp(1.5rem, 2vw, 3rem);
        overflow: hidden;
        height: 100%;
        padding: clamp(1.5rem, 2.5vh, 4rem) clamp(2rem, 3vw, 5rem)
            clamp(6rem, 10vh, 10rem);
        box-sizing: border-box;
    }

    .rm-skill-detail-left {
        display: flex;
        flex-direction: column;
        align-items: center;
        overflow-y: auto;
        padding: clamp(0.5rem, 0.8vw, 1.2rem) clamp(1rem, 2vw, 3rem) 0;
    }

    .rm-skill-image-card {
        line-height: 0;
    }

    .rm-skill-description {
        margin: 2rem 0 0 0;
        width: 40rem;
        font-size: clamp(1.5rem, 1.4vw, 2.3rem);
        color: rgba(255, 255, 255, 0.55);
        line-height: 1.6;
    }

    .rm-skill-detail-right {
        overflow-y: auto;
        padding: clamp(0.5rem, 0.8vw, 1.2rem) clamp(2rem, 4vw, 8rem) 0
            clamp(0.3rem, 0.5vw, 0.8rem);
    }

    .rm-skill-detail-header {
        display: flex;
        align-items: center;
        gap: clamp(0.7rem, 1.2vw, 1.8rem);
        margin-bottom: clamp(0.6rem, 1vw, 1.5rem);
        font-size: clamp(2.16rem, 2.43vw, 3.78rem);
        flex-wrap: wrap;
        --rm-gold: #ffffff;
    }

    .rm-skill-level-badge {
        display: inline-flex;
        align-items: center;
        white-space: nowrap;
        gap: -0.05em;
    }

    .rm-skill-lv-frag {
        display: inline-block;
        background: var(--rm-gold, #f5a623);
        color: var(--rm-black, #000);
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-weight: 800;
        font-size: 1em;
        line-height: 1;
        padding: 0.06em 0.08em 0.12em;
        transform-origin: center center;
        box-shadow: 0.04em 0.06em 0 rgba(0, 0, 0, 0.35);
    }

    .rm-skill-lv-frag.rm-skill-lv-inv {
        background: var(--rm-black, #000);
        color: var(--rm-gold, #f5a623);
        box-shadow:
            0 0 0 0.07em var(--rm-gold, #f5a623),
            0.04em 0.06em 0 rgba(0, 0, 0, 0.35);
        margin-left: -0.03em;
    }

    .rm-skill-node-grid {
        --hex-w: 13.5rem;
        --hex-h: calc(var(--hex-w) * 1.1547);
        --cols: 8;
        display: flex;
        flex-wrap: wrap;
        align-content: flex-start;
        width: calc(var(--hex-w) * var(--cols) + var(--hex-w) / 2);
        padding-bottom: calc(var(--hex-h) * 0.25);
    }

    .rm-hex-row {
        display: flex;
        width: 100%;
    }

    .rm-hex-row:not(:first-child) {
        margin-top: calc(var(--hex-h) * -0.25);
    }

    .rm-hex-row--odd {
        padding-left: calc(var(--hex-w) / 2);
    }

    .rm-hex-border {
        width: var(--hex-w);
        height: var(--hex-h);
        clip-path: polygon(
            50% 0%,
            100% 25%,
            100% 75%,
            50% 100%,
            0% 75%,
            0% 25%
        );
        background: var(--rm-white);
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
        border: none;
        padding: 0;
        font: inherit;
        color: inherit;
        cursor: pointer;
        transition:
            background 150ms ease,
            transform 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    .rm-hex-border:hover {
        transform: scale(1.06);
    }

    .rm-hex-border:focus-visible {
        outline: none;
    }

    .rm-hex-border:focus-visible .rm-skill-node-hex {
        background: var(--rm-gold, #f5a623);
        color: var(--rm-black);
    }

    .rm-skill-node-hex {
        width: calc(100% - 10px);
        height: calc(100% - 10px);
        clip-path: polygon(
            50% 0%,
            100% 25%,
            100% 75%,
            50% 100%,
            0% 75%,
            0% 25%
        );
        background: var(--rm-black);
        color: var(--rm-white);
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: clamp(0.1rem, 0.2vw, 0.3rem);
        padding: clamp(0.4rem, 0.5vw, 0.8rem) clamp(0.8rem, 1vw, 1.4rem);
        transition:
            background 150ms ease,
            color 150ms ease;
    }

    .rm-skill-node-hex--unlocked {
        background: #e0093b;
        color: var(--rm-white);
    }

    .rm-node-name {
        font-size: clamp(0.78rem, 1.02vw, 1.38rem);
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        text-align: center;
        line-height: 1.2;
        overflow: hidden;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        line-clamp: 2;
        -webkit-box-orient: vertical;
    }

    .rm-node-points {
        font-size: clamp(0.72rem, 0.9vw, 1.26rem);
        font-weight: 800;
        opacity: 0.7;
    }

    .rm-skill-node-hex--unlocked .rm-node-points {
        opacity: 1;
    }

    /* ── Node detail modal ── */
    .rm-node-modal-backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.72);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 100;
        padding: 2rem;
        animation: rm-node-modal-fade 180ms cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    @keyframes rm-node-modal-fade {
        from {
            opacity: 0;
        }
        to {
            opacity: 1;
        }
    }

    .rm-node-modal {
        position: relative;
        background: var(--rm-black);
        color: var(--rm-white);
        width: min(920px, 94vw);
        max-height: 88vh;
        overflow-y: auto;
        padding: clamp(2rem, 2.8vw, 3.6rem) clamp(2.2rem, 3.2vw, 4.2rem)
            clamp(2.2rem, 3vw, 3.8rem);
        clip-path: polygon(0% 2%, 98% 0%, 100% 96%, 2% 100%);
        transform: rotate(-0.6deg);
        border: 30px solid var(--rm-white);
        animation: rm-node-modal-pop 260ms cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    @keyframes rm-node-modal-pop {
        from {
            opacity: 0;
            transform: rotate(-0.6deg) scale(0.94);
        }
        to {
            opacity: 1;
            transform: rotate(-0.6deg) scale(1);
        }
    }

    .rm-node-modal-header {
        display: flex;
        align-items: center;
        gap: clamp(0.4rem, 0.6vw, 0.9rem);
        background: var(--rm-white);
        color: var(--rm-black);
        padding: clamp(0.45rem, 0.6vw, 0.9rem) clamp(0.9rem, 1.2vw, 1.8rem);
        margin: 0 0 clamp(0.8rem, 1.2vw, 1.6rem);
        clip-path: polygon(0% 0%, 100% 0%, 98% 100%, 1% 100%);
    }

    .rm-node-modal-status-icon {
        font-size: clamp(1rem, 1vw, 1.5rem);
        font-weight: 800;
        color: var(--rm-red);
        flex-shrink: 0;
    }

    .rm-node-modal-title {
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(1.3rem, 1.5vw, 2.2rem);
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        line-height: 1.2;
        flex: 1;
    }

    .rm-node-modal-desc {
        margin: 0 0 clamp(1rem, 1.4vw, 1.8rem);
        padding: 0 clamp(0.4rem, 0.5vw, 0.8rem);
        font-size: clamp(1.05rem, 1.05vw, 1.5rem);
        color: rgba(255, 255, 255, 0.78);
        line-height: 1.55;
    }

    .rm-node-modal-missing {
        color: rgba(255, 255, 255, 0.4);
        font-style: italic;
    }

    .rm-node-modal-meta {
        margin: 0 0 clamp(0.6rem, 1vw, 1.2rem);
        padding: 0 clamp(0.4rem, 0.5vw, 0.8rem);
        display: flex;
        flex-direction: column;
        gap: clamp(0.35rem, 0.5vw, 0.7rem);
    }

    .rm-node-modal-meta-row {
        display: grid;
        grid-template-columns: clamp(70px, 7vw, 110px) 1fr;
        align-items: center;
        gap: clamp(0.6rem, 0.9vw, 1.2rem);
    }

    .rm-node-modal-meta-row dt {
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(0.9rem, 0.9vw, 1.25rem);
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: rgba(255, 255, 255, 0.45);
    }

    .rm-node-modal-meta-row dd {
        margin: 0;
        font-size: clamp(1rem, 1vw, 1.35rem);
        color: var(--rm-white);
    }

    .rm-node-modal-badge {
        display: inline-block;
        padding: 0.15em 0.6em;
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(0.7rem, 0.7vw, 1rem);
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        clip-path: polygon(4% 0%, 100% 0%, 96% 100%, 0% 100%);
    }

    .rm-node-modal-badge.rm-unlocked {
        background: var(--rm-red);
        color: var(--rm-white);
    }

    .rm-node-modal-badge.rm-tracked {
        background: var(--rm-gold, #f5a623);
        color: var(--rm-black);
    }

    .rm-node-modal-badge.rm-locked {
        background: var(--rm-gray, #2e2e2e);
        color: var(--rm-white);
    }

    .rm-node-modal-tags {
        display: flex;
        flex-wrap: wrap;
        gap: clamp(0.2rem, 0.3vw, 0.45rem);
    }

    .rm-node-modal-tag {
        font-size: clamp(0.65rem, 0.6vw, 0.9rem);
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: rgba(255, 255, 255, 0.55);
        border: 1px solid rgba(255, 255, 255, 0.25);
        padding: 0.12rem 0.5rem;
    }

    .rm-node-modal-note {
        margin: clamp(0.4rem, 0.6vw, 0.8rem) clamp(0.4rem, 0.5vw, 0.8rem)
            clamp(0.8rem, 1vw, 1.2rem);
        padding: clamp(0.5rem, 0.7vw, 0.9rem) clamp(0.8rem, 1vw, 1.4rem);
        font-size: clamp(0.78rem, 0.78vw, 1.1rem);
        font-style: italic;
        color: rgba(255, 255, 255, 0.7);
        background: rgba(255, 255, 255, 0.06);
        border-left: 3px solid var(--rm-gold, #f5a623);
    }

    .rm-node-modal-progress {
        margin-top: clamp(0.6rem, 0.9vw, 1.1rem);
        padding: 0 clamp(0.4rem, 0.5vw, 0.8rem);
    }

    .rm-node-modal-progress-label {
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(0.7rem, 0.7vw, 1rem);
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: rgba(255, 255, 255, 0.45);
        margin-bottom: clamp(0.3rem, 0.4vw, 0.5rem);
    }

    .rm-node-modal-progress-list {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: clamp(0.2rem, 0.3vw, 0.4rem);
    }

    .rm-node-modal-progress-list li {
        position: relative;
        padding-left: clamp(0.9rem, 1vw, 1.3rem);
        font-size: clamp(0.78rem, 0.78vw, 1.1rem);
        color: rgba(255, 255, 255, 0.75);
        line-height: 1.5;
    }

    .rm-node-modal-progress-list li::before {
        content: "▶";
        position: absolute;
        left: 0;
        top: 0.2em;
        font-size: 0.6em;
        color: var(--rm-red);
    }

    .rm-difficulty {
        font-size: clamp(0.65rem, 0.65vw, 1rem);
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

    /* ── Unlock / Lock action ── */
    .rm-node-modal-actions {
        display: flex;
        align-items: center;
        gap: clamp(0.6rem, 0.9vw, 1.2rem);
        margin: clamp(0.4rem, 0.6vw, 0.9rem)
            clamp(0.4rem, 0.5vw, 0.8rem)
            clamp(0.8rem, 1.2vw, 1.6rem);
    }

    .rm-node-modal-action {
        background: var(--rm-red);
        color: var(--rm-white);
        border: none;
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: clamp(1rem, 1vw, 1.5rem);
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        padding: clamp(0.5rem, 0.7vw, 1rem)
            clamp(1.4rem, 1.8vw, 2.6rem);
        cursor: pointer;
        clip-path: polygon(4% 0%, 100% 0%, 96% 100%, 0% 100%);
        transform: rotate(-1deg);
        transition:
            transform 120ms cubic-bezier(0.2, 0.8, 0.2, 1),
            background 120ms ease;
    }

    .rm-node-modal-action:hover:not(:disabled) {
        transform: rotate(-1deg) scale(1.05);
    }

    .rm-node-modal-action:disabled {
        opacity: 0.5;
        cursor: progress;
    }

    .rm-node-modal-action--lock {
        background: var(--rm-white);
        color: var(--rm-black);
    }

    .rm-node-modal-action-error {
        font-size: clamp(0.78rem, 0.78vw, 1.1rem);
        color: var(--rm-red);
        font-weight: 700;
    }
</style>
