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
    import { formatGroupName } from "$lib/utils/format";
    import KeyHint from "$lib/KeyHint.svelte";
    import PromptWord from "$lib/PromptWord.svelte";

    let {
        onBack,
        achievementData,
    }: {
        onBack: () => void;
        achievementData: AchievementData | null;
    } = $props();

    let skillLoading = $state(false);
    let skillError = $state<string | null>(null);
    let skillData = $state<SkillData | null>(null);
    let selectedIndex = $state(0);

    let selectedSkill = $derived(
        skillData && skillData.skills.length > 0
            ? skillData.skills[selectedIndex]
            : null,
    );

    let totalSkills = $derived(skillData ? skillData.skills.length : 0);

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
            onBack();
        } else if (event.key === "q" || event.key === "Q") {
            event.preventDefault();
            navigatePrev();
        } else if (event.key === "e" || event.key === "E") {
            event.preventDefault();
            navigateNext();
        }
    }

    async function loadSkillData() {
        skillLoading = true;
        skillError = null;

        try {
            skillData = await invoke<SkillData>("load_skills");
            selectedIndex = 0;
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

                <div
                    class="rm-tarot-card rm-tarot-card--large"
                    class:rm-tarot-card--leveled={selectedSkill.current_level >
                        0}
                >
                    <div class="rm-tarot-card-inner">
                        <div class="rm-tarot-top">
                            <span class="rm-tarot-level"
                                >{toRomanNumeral(
                                    selectedSkill.current_level,
                                )}</span
                            >
                            <span class="rm-tarot-pack"
                                >{selectedSkill.pack_name}</span
                            >
                        </div>
                        <div class="rm-tarot-art">
                            <div class="rm-tarot-star-stack">
                                <div class="rm-tarot-star rm-ts-1"></div>
                                <div class="rm-tarot-star rm-ts-2"></div>
                                <div class="rm-tarot-star rm-ts-3"></div>
                                <div class="rm-tarot-star rm-ts-4"></div>
                                <div class="rm-tarot-star rm-ts-5"></div>
                            </div>
                            <div class="rm-tarot-stripe"></div>
                        </div>
                        <div class="rm-tarot-name-strip">
                            <span class="rm-tarot-name"
                                >{selectedSkill.skill.name}</span
                            >
                        </div>
                        <div class="rm-tarot-bottom">
                            <div class="rm-tarot-progress">
                                <div
                                    class="rm-tarot-progress-fill"
                                    style:width="{getSkillProgressPercent(
                                        selectedSkill,
                                    )}%"
                                ></div>
                            </div>
                            <span class="rm-tarot-lv"
                                >LV {selectedSkill.current_level}</span
                            >
                        </div>
                    </div>
                </div>

                {#if selectedSkill.skill.description}
                    <p class="rm-skill-description">
                        {selectedSkill.skill.description}
                    </p>
                {/if}
            </div>

            <div class="rm-skill-detail-right">
                <div class="rm-skill-node-grid" style="--cols: 8">
                    {#each computeHexRows(selectedSkill.skill.nodes, 8) as row, rowIdx}
                        <div
                            class="rm-hex-row"
                            class:rm-hex-row--odd={rowIdx % 2 === 1}
                        >
                            {#each row as node}
                                {@const unlocked = isNodeUnlocked(
                                    node.achievement_id,
                                )}
                                <div
                                    class="rm-hex-border"
                                    class:rm-hex-border--unlocked={unlocked}
                                >
                                    <div
                                        class="rm-skill-node-hex"
                                        class:rm-skill-node-hex--unlocked={unlocked}
                                    >
                                        <span class="rm-node-name"
                                            >{getAchievementName(
                                                node.achievement_id,
                                            )}</span
                                        >
                                        <span class="rm-node-points"
                                            >{node.points} pt</span
                                        >
                                    </div>
                                </div>
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
        gap: clamp(0.8rem, 1vw, 1.5rem);
        overflow-y: auto;
        padding: clamp(0.5rem, 0.8vw, 1.2rem) clamp(1rem, 2vw, 3rem) 0;
    }

    .rm-skill-description {
        margin: 0;
        width: clamp(400px, 27.5vw, 625px);
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
        --hex-w: clamp(96px, 7.8vw, 216px);
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
        transition: background 150ms ease;
    }

    .rm-hex-border--unlocked {
        background: var(--rm-black);
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
        background: var(--rm-gold, #f5a623);
        color: var(--rm-black);
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
</style>
