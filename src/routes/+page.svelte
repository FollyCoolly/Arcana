<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import type { UnlistenFn } from "@tauri-apps/api/event";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import Calendar from "$lib/Calendar.svelte";
    import MenuItem from "$lib/MenuItem.svelte";
    import PromptWord from "$lib/PromptWord.svelte";
    import KeyHint from "$lib/KeyHint.svelte";
    import PhanSiteProgress from "$lib/PhanSiteProgress.svelte";
    import type { LetterConfig } from "$lib/MenuItem.svelte";
    import type { StatusData } from "$lib/types/status";
    import type { AchievementData } from "$lib/types/achievement";
    import type { MainMenuMissionData } from "$lib/types/mission";

    import StatusScreen from "$lib/screens/StatusScreen.svelte";
    import AchievementsScreen from "$lib/screens/AchievementsScreen.svelte";
    import SkillsScreen from "$lib/screens/SkillsScreen.svelte";
    import ItemsScreen from "$lib/screens/ItemsScreen.svelte";
    import GalleryScreen from "$lib/screens/GalleryScreen.svelte";
    import MissionsScreen from "$lib/screens/MissionsScreen.svelte";

    type MenuScreen =
        | "main"
        | "status"
        | "achievements"
        | "skills"
        | "items"
        | "gallery"
        | "missions";
    type MenuItemId =
        | "status"
        | "skills"
        | "achievements"
        | "items"
        | "gallery"
        | "missions";

    type MenuItem = {
        id: MenuItemId;
        label: string;
        description: string;
        enabled: boolean;
    };

    const MENU_ITEMS: MenuItem[] = [
        {
            id: "status",
            label: "Status",
            description: "Body and life metrics from local JSON snapshots.",
            enabled: true,
        },
        {
            id: "skills",
            label: "Skills",
            description: "Skill tree progression linked to achievements.",
            enabled: true,
        },
        {
            id: "achievements",
            label: "Achievements",
            description: "Milestones and timeline tracking.",
            enabled: true,
        },
        {
            id: "items",
            label: "Items",
            description: "Personal inventory and purchase awareness.",
            enabled: true,
        },
        {
            id: "gallery",
            label: "Gallery",
            description: "Books, media, and games aggregation hub.",
            enabled: true,
        },
        {
            id: "missions",
            label: "Missions",
            description: "Daily and long-term mission tracking.",
            enabled: true,
        },
    ];

    const MENU_LETTER_DATA: Record<MenuItemId, LetterConfig[]> = {
        status: [
            { char: "S", size: "1.18em", yOffset: -3, rotate: -6, weight: 800 },
            {
                char: "t",
                size: "0.82em",
                yOffset: 4,
                rotate: 4,
                color: "black",
                outline: true,
            },
            { char: "A", size: "0.85em", yOffset: 1, rotate: -2 },
            {
                char: "t",
                size: "0.92em",
                yOffset: -1,
                rotate: 5,
                color: "black",
                rounded: true,
            },
            { char: "U", size: "0.7em", yOffset: 3, rotate: -4 },
            {
                char: "s",
                size: "0.78em",
                yOffset: -2,
                rotate: 6,
                color: "black",
            },
        ],
        skills: [
            { char: "S", size: "1.15em", yOffset: -2, rotate: -4, weight: 800 },
            { char: "K", size: "0.78em", yOffset: 3, rotate: 5 },
            {
                char: "i",
                size: "0.88em",
                yOffset: -1,
                rotate: -3,
                color: "black",
                rounded: true,
            },
            { char: "L", size: "1.1em", yOffset: 2, rotate: 4 },
            {
                char: "l",
                size: "0.80em",
                yOffset: -2,
                rotate: -5,
                color: "black",
                outline: true,
            },
            { char: "S", size: "0.76em", yOffset: 1, rotate: 3 },
        ],
        achievements: [
            { char: "A", size: "1.18em", yOffset: -3, rotate: -5, weight: 800 },
            {
                char: "c",
                size: "0.82em",
                yOffset: 3,
                rotate: 4,
                color: "black",
            },
            { char: "H", size: "1.0em", yOffset: -1, rotate: -3 },
            {
                char: "i",
                size: "0.88em",
                yOffset: 2,
                rotate: 5,
                color: "black",
                outline: true,
            },
            { char: "E", size: "0.82em", yOffset: -2, rotate: -2 },
            {
                char: "v",
                size: "0.95em",
                yOffset: 4,
                rotate: 3,
                color: "black",
                rounded: true,
            },
            { char: "E", size: "1.12em", yOffset: -1, rotate: -4 },
            { char: "M", size: "0.75em", yOffset: 2, rotate: 2 },
            {
                char: "e",
                size: "0.78em",
                yOffset: -3,
                rotate: -3,
                color: "black",
            },
            { char: "N", size: "1.1em", yOffset: 1, rotate: 5 },
            {
                char: "t",
                size: "0.92em",
                yOffset: -2,
                rotate: -4,
                color: "black",
                outline: true,
            },
            { char: "S", size: "0.88em", yOffset: 3, rotate: 3 },
        ],
        items: [
            { char: "I", size: "1.15em", yOffset: -2, rotate: -5, weight: 800 },
            {
                char: "t",
                size: "0.85em",
                yOffset: 3,
                rotate: 4,
                color: "black",
                outline: true,
            },
            { char: "E", size: "0.80em", yOffset: -1, rotate: -3 },
            {
                char: "m",
                size: "0.97em",
                yOffset: 2,
                rotate: 5,
                color: "black",
                rounded: true,
            },
            { char: "S", size: "1.08em", yOffset: -3, rotate: -4 },
        ],
        gallery: [
            { char: "G", size: "1.18em", yOffset: -3, rotate: -6, weight: 800 },
            {
                char: "a",
                size: "0.88em",
                yOffset: 4,
                rotate: 3,
                color: "black",
                rounded: true,
            },
            { char: "L", size: "0.78em", yOffset: -1, rotate: -4 },
            {
                char: "l",
                size: "0.76em",
                yOffset: 2,
                rotate: 5,
                color: "black",
                outline: true,
            },
            { char: "E", size: "1.1em", yOffset: -2, rotate: -3 },
            {
                char: "r",
                size: "0.93em",
                yOffset: 3,
                rotate: 4,
                color: "black",
            },
            { char: "Y", size: "1.02em", yOffset: -1, rotate: -5 },
        ],
        missions: [
            { char: "M", size: "1.18em", yOffset: -3, rotate: -6, weight: 800 },
            {
                char: "i",
                size: "0.82em",
                yOffset: 4,
                rotate: 4,
                color: "black",
                outline: true,
            },
            { char: "S", size: "1.08em", yOffset: -1, rotate: -3 },
            {
                char: "s",
                size: "0.88em",
                yOffset: 3,
                rotate: 5,
                color: "black",
                rounded: true,
            },
            { char: "I", size: "0.85em", yOffset: -2, rotate: -4 },
            {
                char: "o",
                size: "0.78em",
                yOffset: 2,
                rotate: 3,
                color: "black",
            },
            { char: "N", size: "1.1em", yOffset: -1, rotate: -5 },
            {
                char: "s",
                size: "0.76em",
                yOffset: 1,
                rotate: 4,
                color: "black",
                outline: true,
            },
        ],
    };

    const DEFAULT_FOCUS_INDEX = Math.max(
        0,
        MENU_ITEMS.findIndex((item) => item.enabled),
    );

    const MENU_QUAD_CONFIGS: { rot: number; clip: string }[] = [
        { rot: -35, clip: "polygon(12% 30%, 65% 0%, 95% 99%, 15% 80%)" },
        { rot: -27, clip: "polygon(15% 35%, 65% 2%, 97% 92%, 3% 98%)" },
        { rot: -20, clip: "polygon(15% 35%, 65% 2%, 88% 100%, 18% 88%)" },
        { rot: -8, clip: "polygon(10% 40%, 60% 0%, 95% 85%, 2% 100%)" },
        { rot: -2, clip: "polygon(10% 40%, 60% 0%, 92% 90%, 30% 90%)" },
        { rot: 2, clip: "polygon(10% 45%, 50% 0%, 100% 50%, 30% 92%)" },
    ];

    let currentScreen = $state<MenuScreen>("main");
    let focusedMenuIndex = $state(DEFAULT_FOCUS_INDEX);
    let menuFeedback = $state<string | null>(null);

    // Shared data: preloaded on mount, passed to screen components
    let statusData = $state<StatusData | null>(null);
    let achievementData = $state<AchievementData | null>(null);
    let missionMenuData = $state<MainMenuMissionData | null>(null);

    let commandRef = $state<HTMLElement | undefined>(undefined);
    let menuItemRefs = $state<(HTMLButtonElement | undefined)[]>([]);

    let menuFeedbackTimer: ReturnType<typeof setTimeout> | null = null;
    let unlistenSummonEvent: UnlistenFn | null = null;

    $effect(() => {
        const idx = focusedMenuIndex;
        const btn = menuItemRefs[idx];
        const container = commandRef;
        if (!btn || !container) return;

        const btnRect = btn.getBoundingClientRect();
        const containerRect = container.getBoundingClientRect();

        const centerX = btnRect.left + btnRect.width / 2 - containerRect.left;
        const centerY = btnRect.top + btnRect.height / 2 - containerRect.top;

        const quadW = btn.offsetWidth * 1.8;
        const quadH = btn.offsetHeight * 1.6;
        const cfg = MENU_QUAD_CONFIGS[idx] ?? {
            rot: 0,
            clip: "polygon(0% 0%, 100% 0%, 100% 100%, 0% 100%)",
        };

        container.style.setProperty("--quad-x", `${centerX - quadW / 2}px`);
        container.style.setProperty("--quad-y", `${centerY - quadH / 2}px`);
        container.style.setProperty("--quad-w", `${quadW}px`);
        container.style.setProperty("--quad-h", `${quadH}px`);
        container.style.setProperty("--quad-rot", `${cfg.rot}deg`);
        container.style.setProperty("--quad-clip", cfg.clip);
    });

    function resetToMainMenu() {
        currentScreen = "main";
        focusedMenuIndex = DEFAULT_FOCUS_INDEX;
        menuFeedback = null;
    }

    function setMenuFeedback(message: string) {
        menuFeedback = message;

        if (menuFeedbackTimer) {
            clearTimeout(menuFeedbackTimer);
        }

        menuFeedbackTimer = setTimeout(() => {
            menuFeedback = null;
            menuFeedbackTimer = null;
        }, 1600);
    }

    function isMenuItemSelectable(item: MenuItem | undefined): boolean {
        return !!item && item.enabled;
    }

    function setFocusedMenuIndex(index: number) {
        if (isMenuItemSelectable(MENU_ITEMS[index])) {
            focusedMenuIndex = index;
        }
    }

    function moveMenuFocus(offset: number) {
        const itemCount = MENU_ITEMS.length;
        if (itemCount === 0) return;

        let nextIndex = focusedMenuIndex;
        for (let i = 0; i < itemCount; i += 1) {
            nextIndex = (nextIndex + offset + itemCount) % itemCount;
            if (isMenuItemSelectable(MENU_ITEMS[nextIndex])) {
                focusedMenuIndex = nextIndex;
                return;
            }
        }
    }

    async function hideInterface() {
        resetToMainMenu();
        try {
            await getCurrentWindow().hide();
        } catch (error) {
            setMenuFeedback("Hide failed");
            console.error("Failed to hide window", error);
        }
    }

    function goBack() {
        currentScreen = "main";
    }

    async function activateMenuItem(index: number) {
        if (currentScreen !== "main") return;

        const item = MENU_ITEMS[index];
        if (!item || !isMenuItemSelectable(item)) return;

        focusedMenuIndex = index;

        // For skills, also preload achievement data
        if (item.id === "skills" && !achievementData) {
            try {
                achievementData =
                    await invoke<AchievementData>("load_achievements");
            } catch {
                // skill screen will work without it
            }
        }

        currentScreen = item.id;
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            // Screens handle their own Escape internally.
            // We only handle it on the main menu to hide the interface.
            if (currentScreen === "main") {
                event.preventDefault();
                void hideInterface();
            }
            return;
        }

        if (currentScreen !== "main") return;

        if (event.key === "ArrowDown") {
            event.preventDefault();
            moveMenuFocus(1);
            return;
        }

        if (event.key === "ArrowUp") {
            event.preventDefault();
            moveMenuFocus(-1);
            return;
        }

        if (event.key === "Enter") {
            event.preventDefault();
            void activateMenuItem(focusedMenuIndex);
        }
    }

    async function preloadStatusData() {
        try {
            statusData = await invoke<StatusData>("load_status_data");
        } catch {
            // non-critical: screen will load its own data
        }
    }

    async function preloadMissionMenuData() {
        try {
            missionMenuData = await invoke<MainMenuMissionData>(
                "load_main_menu_missions",
            );
        } catch {
            // non-critical: main menu works without mission data
        }
    }

    onMount(() => {
        const appWindow = getCurrentWindow();

        window.addEventListener("keydown", handleKeydown);

        if (!statusData) {
            void preloadStatusData();
        }
        void preloadMissionMenuData();

        appWindow
            .listen("reality://summoned", () => {
                resetToMainMenu();
            })
            .then((unlisten) => {
                unlistenSummonEvent = unlisten;
            });

        return () => {
            window.removeEventListener("keydown", handleKeydown);
            if (unlistenSummonEvent) {
                unlistenSummonEvent();
            }
            if (menuFeedbackTimer) {
                clearTimeout(menuFeedbackTimer);
            }
        };
    });
</script>

<main class="rm-overlay">
    <section class="rm-scene">
        {#if currentScreen === "main"}
            <div class="rm-calendar-widget">
                <Calendar />
            </div>
            <div class="rm-task-panel">
                {#if missionMenuData?.countdown && missionMenuData.countdown.days_remaining <= 99}
                    {@const cd = missionMenuData.countdown}
                    {@const labelLen = cd.label.length}
                    {@const daysStr = String(cd.days_remaining).padStart(
                        2,
                        "0",
                    )}
                    {@const boardSrc =
                        labelLen <= 2
                            ? "/ui/board/countdown_2wc.png"
                            : "/ui/board/countdown_4wc.png"}
                    <div
                        class="rm-countdown"
                        aria-label="Mission countdown"
                        data-label-len={labelLen <= 2 ? "2" : "4"}
                        style:background-image="url({boardSrc})"
                    >
                        <div class="rm-cd-mission-bg">
                            <span class="rm-cd-mission-text">{cd.short_desc}</span>
                        </div>
                        <span class="rm-cd-prefix">距离</span>
                        {#if labelLen <= 2}
                            <span class="rm-cd-label-a">{cd.label}</span>
                        {:else}
                            <span class="rm-cd-label-a"
                                >{cd.label.slice(0, 2)}</span
                            >
                            <span class="rm-cd-label-b"
                                >{cd.label.slice(2, 4)}</span
                            >
                        {/if}
                        <span class="rm-cd-middle">还剩</span>
                        <div class="rm-cd-days-bg-1" aria-hidden="true"></div>
                        <div class="rm-cd-days-bg-2" aria-hidden="true"></div>
                        <span class="rm-cd-days-1">{daysStr[0]}</span>
                        <span class="rm-cd-days-2">{daysStr[1]}</span>
                        <span class="rm-cd-suffix">日</span>
                    </div>
                {:else if statusData}
                    <div class="rm-player-info" aria-label="Player info">
                        <span class="rm-player-name">{statusData.username}</span
                        >
                        <span class="rm-player-days"
                            >Day {statusData.game_days ?? "—"}</span
                        >
                    </div>
                {/if}

                {#if missionMenuData?.hints}
                    {#each missionMenuData.hints as hint, i}
                        <div
                            class="rm-hint-board"
                            data-board={i === 0 ? "fat" : "slim"}
                            style:background-image="url(/ui/board/{i === 0
                                ? 'board_fat'
                                : 'board_slim'}.png)"
                            aria-label="Mission hint"
                        >
                            <span class="rm-hint-text">{hint.short_desc}</span>
                        </div>
                    {/each}
                {/if}
            </div>
            {#if missionMenuData?.progress}
                <PhanSiteProgress
                    question={missionMenuData.progress.label}
                    progress={missionMenuData.progress.progress}
                />
            {/if}
            <div class="rm-star-left" aria-hidden="true">
                <div class="rm-star-stack">
                    <div class="rm-star rm-star-1"></div>
                    <div class="rm-star rm-star-2"></div>
                    <div class="rm-star rm-star-3"></div>
                    <div class="rm-star rm-star-4"></div>
                    <div class="rm-star rm-star-5"></div>
                    <div class="rm-star rm-star-6"></div>
                    <div class="rm-star rm-star-7"></div>
                    <div class="rm-star rm-star-8"></div>
                </div>
                <div class="rm-star-stack rm-star-small">
                    <div class="rm-star rm-sm-1"></div>
                    <div class="rm-star rm-sm-2"></div>
                    <div class="rm-star rm-sm-3"></div>
                    <div class="rm-star rm-sm-4"></div>
                    <div class="rm-star rm-sm-5"></div>
                    <div class="rm-star rm-sm-6"></div>
                </div>
            </div>
            <div class="rm-star-right" aria-hidden="true">
                <div class="rm-star-stack">
                    <div class="rm-star rm-star-solid"></div>
                </div>
                <div class="rm-star-stack rm-star-small">
                    <div class="rm-star rm-sm-solid"></div>
                </div>
            </div>
            <div class="rm-diagonal-line" aria-hidden="true"></div>

            <div class="rm-prompt-hints">
                <button
                    type="button"
                    class="rm-prompt-hint"
                    onclick={() => void hideInterface()}
                >
                    <KeyHint key="Esc" fontSize={36} />
                    <PromptWord text="Hide" fontSize={72} />
                </button>
                <button
                    type="button"
                    class="rm-prompt-hint"
                    onclick={() => void activateMenuItem(focusedMenuIndex)}
                >
                    <KeyHint key="↵" fontSize={36} />
                    <PromptWord text="Confirm" fontSize={72} />
                </button>
            </div>
        {/if}

        {#if currentScreen === "main"}
            <aside class="rm-command" bind:this={commandRef}>
                <ul class="rm-menu">
                    {#each MENU_ITEMS as item, index}
                        <li
                            class="rm-menu-line"
                            style:position="relative"
                            style:z-index={focusedMenuIndex === index ? 10 : 0}
                        >
                            <button
                                type="button"
                                class="rm-menu-item"
                                class:is-focused={focusedMenuIndex === index}
                                class:is-disabled={!item.enabled}
                                aria-disabled={!item.enabled}
                                onclick={() => void activateMenuItem(index)}
                                onmouseenter={() => setFocusedMenuIndex(index)}
                                bind:this={menuItemRefs[index]}
                            >
                                <MenuItem
                                    letters={MENU_LETTER_DATA[item.id]}
                                    active={focusedMenuIndex === index}
                                />
                            </button>
                        </li>
                    {/each}
                </ul>
                <div class="rm-selection-quad" aria-hidden="true"></div>

                <footer class="rm-command-foot">
                    {#if menuFeedback}
                        <p class="rm-feedback">{menuFeedback}</p>
                    {/if}
                </footer>
            </aside>
        {/if}

        {#if currentScreen === "status"}
            <StatusScreen onBack={goBack} {statusData} />
        {/if}

        {#if currentScreen === "achievements"}
            <AchievementsScreen
                onBack={goBack}
                {achievementData}
                onAchievementDataLoaded={(data) => {
                    achievementData = data;
                }}
            />
        {/if}

        {#if currentScreen === "skills"}
            <SkillsScreen
                onBack={goBack}
                {achievementData}
                onAchievementDataLoaded={(data) => {
                    achievementData = data;
                }}
            />
        {/if}

        {#if currentScreen === "items"}
            <ItemsScreen onBack={goBack} />
        {/if}

        {#if currentScreen === "gallery"}
            <GalleryScreen onBack={goBack} />
        {/if}

        {#if currentScreen === "missions"}
            <MissionsScreen
                onBack={goBack}
                missionProgress={missionMenuData?.progress ?? null}
            />
        {/if}
    </section>
</main>

<style>
    :global(html),
    :global(body) {
        margin: 0;
        width: 100%;
        height: 100%;
        background: transparent;
        overflow: hidden;
    }

    .rm-overlay {
        --rm-black: #000000;
        --rm-white: #ffffff;
        --rm-red: #e5191c;
        --rm-gray: #2e2e2e;
        --rm-gold: #f5a623;
        position: relative;
        min-height: 100vh;
        color: var(--rm-white);
        background: rgba(30, 0, 0, 0.8);
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    }

    .rm-scene {
        position: relative;
        width: 100%;
        height: 100vh;
        overflow: hidden;
    }

    .rm-calendar-widget {
        position: absolute;
        top: 1.5rem;
        left: 1.5rem;
        width: clamp(250px, 14.6vw, 600px);
        z-index: 3;
        pointer-events: none;
    }

    .rm-task-panel {
        position: absolute;
        top: 2rem;
        right: 2rem;
        z-index: 3;
        display: flex;
        flex-direction: column;
        gap: 0;
        align-items: flex-end;
        pointer-events: none;
    }

    .rm-player-info {
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        gap: 0.1rem;
    }

    .rm-player-name,
    .rm-player-days {
        color: var(--rm-white);
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        -webkit-text-stroke: 0.04em var(--rm-black);
        paint-order: stroke fill;
        font-size: clamp(1.5rem, 2.1vw, 3rem);
    }

    .rm-countdown {
        width: min(57vw, 832px);
        height: clamp(10.4rem, 16.9vw, 16.9rem);
        background-repeat: no-repeat;
        background-size: 100% 100%;
        background-position: center;
        position: relative;
    }

    .rm-hint-board {
        width: min(57vw, 832px);
        height: clamp(4rem, 8vw, 8rem);
        background-repeat: no-repeat;
        background-size: 100% 100%;
        background-position: center;
        position: relative;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .rm-task-panel > * + * {
        margin-top: -1rem;
    }

    .rm-hint-board[data-board="slim"] {
        width: min(57vw, 832px);
        height: clamp(4.5rem, 7vw, 7rem);
    }

    .rm-hint-text {
        font-family:
            "方正兰亭黑_GB", "Noto Sans SC", "Microsoft YaHei", sans-serif;
        font-weight: 900;
        color: #ffffff;
        font-size: clamp(1rem, 1.8vw, 1.8rem);
        white-space: nowrap;
        line-height: 1;
        -webkit-text-stroke: 0.03em #000000;
        paint-order: stroke fill;
    }

    .rm-cd-mission-bg {
        position: absolute;
        top: 61%; /* ← 位置 */
        left: 10%; /* ← 位置 */
        width: 80%; /* ← 宽度 */
        height: 27%; /* ← 高度 */
        background: var(--rm-red);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1;
    }

    .rm-cd-mission-text {
        font-family:
            "方正兰亭黑_GB", "Noto Sans SC", "Microsoft YaHei", sans-serif;
        font-weight: 900;
        color: #000000;
        font-size: clamp(1rem, 2vw, 2rem);
        white-space: nowrap;
        line-height: 1;
    }

    .rm-cd-prefix,
    .rm-cd-middle,
    .rm-cd-label-a,
    .rm-cd-label-b,
    .rm-cd-days-1,
    .rm-cd-days-2,
    .rm-cd-suffix {
        position: absolute;
        font-family:
            "方正兰亭黑_GB", "Noto Sans SC", "Microsoft YaHei", sans-serif;
        font-weight: 900;
        color: var(--rm-red);
        letter-spacing: 0.02em;
        line-height: 1;
        white-space: nowrap;
        display: inline-block;
    }

    /* ── 2字 布局 ── */
    .rm-countdown[data-label-len="2"] .rm-cd-prefix {
        top: 8%;
        left: 30%;
        font-size: clamp(1.4rem, 2.8vw, 2.8rem);
        transform: rotate(-14deg);
        z-index: 1;
    }
    .rm-countdown[data-label-len="2"] .rm-cd-label-a {
        top: 14%;
        left: 38%;
        font-size: clamp(3rem, 6vw, 6rem);
        transform: rotate(-14deg);
        z-index: 2;
    }
    .rm-countdown[data-label-len="2"] .rm-cd-middle {
        top: 42%;
        left: 61%;
        font-size: clamp(1.2rem, 2.4vw, 2.4rem);
        font-weight: 400;
        transform: rotate(-12deg);
        z-index: 1;
    }
    .rm-countdown[data-label-len="2"] .rm-cd-days-bg-1 {
        position: absolute;
        top: 20%;
        right: 16%;
        width: 12%;
        height: 40%;
        background: var(--rm-red);
        transform: rotate(-14deg);
        z-index: 1;
    }
    .rm-countdown[data-label-len="2"] .rm-cd-days-bg-2 {
        position: absolute;
        top: 20%;
        right: 7%;
        width: 15%;
        height: 38%;
        background: var(--rm-red);
        transform: rotate(-14deg);
        z-index: 1;
    }
    .rm-countdown[data-label-len="2"] .rm-cd-days-1 {
        top: 14%;
        right: 17%;
        font-size: clamp(4rem, 8vw, 8rem);
        color: #000000;
        transform: rotate(-14deg);
        z-index: 2;
    }
    .rm-countdown[data-label-len="2"] .rm-cd-days-2 {
        top: 12%;
        right: 8%;
        font-size: clamp(4rem, 8vw, 8rem);
        color: #000000;
        transform: rotate(-14deg);
        z-index: 2;
    }
    .rm-countdown[data-label-len="2"] .rm-cd-suffix {
        top: 33%;
        right: 1%;
        font-size: clamp(1.4rem, 2.8vw, 2.8rem);
        transform: rotate(-8deg);
        z-index: 1;
    }

    /* ── 4字 布局 ── */
    .rm-countdown[data-label-len="4"] .rm-cd-prefix {
        top: 7%;
        left: 8.5%;
        font-size: clamp(1.4rem, 2.8vw, 2.8rem);
        transform: rotate(-14deg);
        z-index: 1;
    }
    .rm-countdown[data-label-len="4"] .rm-cd-label-a {
        top: 15%;
        left: 13%;
        font-size: clamp(3rem, 6vw, 6rem);
        transform: rotate(-14deg);
        z-index: 2;
    }
    .rm-countdown[data-label-len="4"] .rm-cd-label-b {
        top: 15%;
        left: 38%;
        font-size: clamp(3rem, 6vw, 6rem);
        transform: rotate(-14deg);
        z-index: 2;
    }
    .rm-countdown[data-label-len="4"] .rm-cd-middle {
        top: 43%;
        left: 60%;
        font-size: clamp(1.2rem, 2.4vw, 2.4rem);
        font-weight: 400;
        transform: rotate(-12deg);
        z-index: 1;
    }
    .rm-countdown[data-label-len="4"] .rm-cd-days-bg-1 {
        position: absolute;
        top: 20%; /* ← 位置 */
        right: 18%; /* ← 位置 */
        width: 12%; /* ← 宽度 */
        height: 40%; /* ← 高度 */
        background: var(--rm-red);
        transform: rotate(-14deg); /* ← 角度 */
        z-index: 1;
    }
    .rm-countdown[data-label-len="4"] .rm-cd-days-bg-2 {
        position: absolute;
        top: 20%; /* ← 位置 */
        right: 9%; /* ← 位置 */
        width: 15%; /* ← 宽度 */
        height: 38%; /* ← 高度 */
        background: var(--rm-red);
        transform: rotate(-14deg); /* ← 角度 */
        z-index: 1;
    }
    .rm-countdown[data-label-len="4"] .rm-cd-days-1 {
        top: 14%;
        right: 19%;
        font-size: clamp(4rem, 8vw, 8rem);
        color: #000000;
        transform: rotate(-14deg);
        z-index: 2;
    }
    .rm-countdown[data-label-len="4"] .rm-cd-days-2 {
        top: 12%;
        right: 10%;
        font-size: clamp(4rem, 8vw, 8rem);
        color: #000000;
        transform: rotate(-14deg);
        z-index: 2;
    }
    .rm-countdown[data-label-len="4"] .rm-cd-suffix {
        top: 36%;
        right: 2%;
        font-size: clamp(1.4rem, 2.8vw, 2.8rem);
        transform: rotate(-13deg);
        z-index: 1;
    }

    .rm-star-stack {
        position: absolute;
        top: 50%;
        left: 35%;
        width: 80vh;
        aspect-ratio: 1;
        transform: translate(-50%, -50%) rotate(-14deg);
        z-index: 0;
        pointer-events: none;
    }

    .rm-star {
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

    .rm-star-1 {
        background: var(--rm-white);
        transform: scale(0.92);
    }
    .rm-star-2 {
        background: var(--rm-black);
        transform: scale(0.8);
    }
    .rm-star-3 {
        background: var(--rm-white);
        transform: scale(0.68);
    }
    .rm-star-4 {
        background: var(--rm-black);
        transform: scale(0.56);
    }
    .rm-star-5 {
        background: var(--rm-white);
        transform: scale(0.44);
    }
    .rm-star-6 {
        background: var(--rm-black);
        transform: scale(0.32);
    }
    .rm-star-7 {
        background: var(--rm-white);
        transform: scale(0.2);
    }
    .rm-star-8 {
        background: var(--rm-black);
        transform: scale(0.08);
    }

    .rm-star-solid {
        background: var(--rm-black);
        transform: scale(0.92);
    }

    .rm-star-small {
        top: 62%;
        left: 35%;
        transform: translate(-50%, -50%) rotate(20deg);
    }

    .rm-sm-1 {
        background: var(--rm-white);
        transform: scale(0.5);
    }
    .rm-sm-2 {
        background: var(--rm-black);
        transform: scale(0.48);
    }
    .rm-sm-3 {
        background: var(--rm-white);
        transform: scale(0.38);
    }
    .rm-sm-4 {
        background: var(--rm-black);
        transform: scale(0.28);
    }
    .rm-sm-5 {
        background: var(--rm-white);
        transform: scale(0.18);
    }
    .rm-sm-6 {
        background: var(--rm-black);
        transform: scale(0.08);
    }

    .rm-sm-solid {
        background: var(--rm-black);
        transform: scale(0.52);
    }

    .rm-star-left {
        position: absolute;
        inset: 0;
        clip-path: polygon(0 0, 20% 0, 50% 100%, 0 100%);
        z-index: 0;
        pointer-events: none;
    }

    .rm-star-right {
        position: absolute;
        inset: 0;
        clip-path: polygon(20% 0, 100% 0, 100% 100%, 50% 100%);
        z-index: 0;
        pointer-events: none;
    }

    .rm-diagonal-line {
        position: absolute;
        inset: 0;
        clip-path: polygon(19.85% 0%, 20.15% 0%, 50.15% 100%, 49.85% 100%);
        background: var(--rm-white);
        z-index: 1;
        pointer-events: none;
    }

    .rm-command {
        position: absolute;
        left: 30%;
        top: 50%;
        width: min(75vw, 1200px);
        z-index: 2;
        transform: translateY(-50%);
    }

    .rm-menu {
        margin: 0;
        padding: 0;
        list-style: none;
    }

    .rm-menu-line {
        margin: -1rem 0;
    }

    .rm-menu-line:nth-child(1) {
        margin-left: 1.5vw;
    }
    .rm-menu-line:nth-child(2) {
        margin-left: 5vw;
    }
    .rm-menu-line:nth-child(3) {
        margin-left: 1vw;
    }
    .rm-menu-line:nth-child(4) {
        margin-left: 7.5vw;
    }
    .rm-menu-line:nth-child(5) {
        margin-left: 7vw;
    }
    .rm-menu-line:nth-child(6) {
        margin-left: 10vw;
    }

    .rm-menu-line:nth-child(1) .rm-menu-item {
        transform: rotate(-30deg);
        clip-path: polygon(0% 10%, 100% 0%, 90% 88%, 10% 96%);
    }
    .rm-menu-line:nth-child(2) .rm-menu-item {
        transform: rotate(-27deg);
        clip-path: polygon(0% 5%, 99% 10%, 96% 94%, 2% 100%);
    }
    .rm-menu-line:nth-child(3) .rm-menu-item {
        transform: rotate(-20deg);
        clip-path: polygon(2% 0%, 100% 8%, 98% 100%, 0% 90%);
    }
    .rm-menu-line:nth-child(4) .rm-menu-item {
        transform: rotate(-8deg);
        clip-path: polygon(0% 6%, 98% 0%, 100% 92%, 1% 100%);
    }
    .rm-menu-line:nth-child(5) .rm-menu-item {
        transform: rotate(-2deg);
        clip-path: polygon(1% 0%, 100% 4%, 97% 96%, 0% 100%);
    }
    .rm-menu-line:nth-child(6) .rm-menu-item {
        transform: rotate(2deg);
        clip-path: polygon(0% 8%, 99% 0%, 100% 100%, 3% 92%);
    }

    .rm-menu-item {
        width: fit-content;
        border: 0;
        padding: 1rem 4rem;
        display: flex;
        align-items: center;
        gap: 0.2rem;
        cursor: pointer;
        color: var(--rm-white);
        background: var(--rm-black);
        transition: background-color 140ms ease;
    }

    .rm-menu-item:not(.is-disabled):hover,
    .rm-menu-item.is-focused {
        background: var(--rm-red);
    }

    .rm-menu-item.is-disabled {
        cursor: default;
    }

    .rm-menu-item:focus-visible {
        outline: 0.16rem solid var(--rm-white);
        outline-offset: 0.12rem;
    }

    .rm-selection-quad {
        position: absolute;
        left: var(--quad-x);
        top: var(--quad-y);
        width: var(--quad-w);
        height: var(--quad-h);
        transform: rotate(var(--quad-rot));
        z-index: 15;
        background: var(--rm-red);
        mix-blend-mode: difference;
        clip-path: var(
            --quad-clip,
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

    .rm-command-foot {
        margin-top: 1rem;
        transform: rotate(2deg);
    }

    .rm-feedback {
        margin: 0;
        font-size: 0.8rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: var(--rm-white);
        background: var(--rm-red);
        display: inline-block;
        padding: 0.24rem 0.42rem;
    }

    /* ─── Shared styles for screen components ─── */

    :global(.rm-stage) {
        position: absolute;
        inset: 0;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        z-index: 2;
    }

    :global(.rm-back-btn) {
        position: fixed;
        bottom: clamp(1.5rem, 3vh, 3.5rem);
        left: clamp(1.5rem, 3vw, 4rem);
        z-index: 10;
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

    :global(.rm-back-btn:hover) {
        transform: rotate(2deg) scale(1.06);
    }

    :global(.rm-back-btn .p5-prompt-word) {
        margin-left: -1rem;
    }

    .rm-prompt-hints {
        position: fixed;
        bottom: clamp(1.5rem, 3vh, 3.5rem);
        left: clamp(1.5rem, 3vw, 4rem);
        z-index: 10;
        display: flex;
        flex-direction: row;
        gap: 0.5rem;
    }

    .rm-prompt-hint {
        display: flex;
        align-items: center;
        gap: 0;
        background: none;
        border: none;
        padding: 0;
        cursor: pointer;
        transition: transform 120ms ease;
    }

    .rm-prompt-hint:hover {
        transform: scale(1.06);
    }

    .rm-prompt-hint :global(.p5-prompt-word) {
        margin-left: -1rem;
    }

    :global(.state-text) {
        margin: 0.85rem 0 0;
        color: rgba(255, 255, 255, 0.7);
    }

    :global(.state-text.error) {
        color: var(--rm-red);
        font-weight: 700;
    }

    /* Tarot card styles — :global because nebula cards are created programmatically outside Svelte template */
    :global(.rm-nebula-card.rm-tarot-card) {
        cursor: pointer;
        transition: scale 160ms ease;
    }

    :global(.rm-nebula-card.rm-tarot-card:hover) {
        scale: 1.12;
        z-index: 5;
    }

    :global(.rm-tarot-card) {
        display: block;
        border: none;
        background: none;
        cursor: pointer;
        padding: 0;
        width: clamp(120px, 10vw, 200px);
        transform: rotate(var(--card-rot, 0deg));
        transition: transform 200ms ease;
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        color: var(--rm-white);
    }

    :global(.rm-tarot-card:hover) {
        transform: translateY(-6px) rotateX(4deg) rotate(var(--card-rot, 0deg));
        z-index: 5;
    }

    :global(.rm-tarot-card-inner) {
        aspect-ratio: 0.6 / 1;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        border: 2px solid rgba(255, 255, 255, 0.15);
    }

    :global(.rm-tarot-top) {
        background: #ffffff;
        color: #000000;
        padding: 3px 5px;
        display: flex;
        justify-content: space-between;
        align-items: center;
        flex-shrink: 0;
    }

    :global(.rm-tarot-level) {
        font-size: 11px;
        font-weight: 800;
        letter-spacing: 0.06em;
    }

    :global(.rm-tarot-pack) {
        font-size: 7px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        opacity: 0.5;
        text-align: right;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        max-width: 60%;
    }

    :global(.rm-tarot-art) {
        flex: 1;
        background: #000000;
        position: relative;
        overflow: hidden;
        display: flex;
        align-items: center;
        justify-content: center;
        opacity: 0.35;
    }

    :global(.rm-tarot-card--leveled .rm-tarot-art) {
        opacity: 1;
    }

    :global(.rm-nebula-image-card) {
        width: 160px;
        height: 320px;
        background: #ffffff;
        display: grid;
        grid-template-columns: 1fr 18fr 1fr;
        grid-template-rows: 1fr 32fr 7fr;
    }

    :global(.rm-nebula-title-area),
    :global(.rm-image-card-title-area) {
        grid-column: 1 / -1;
        grid-row: 3;
        overflow: hidden;
        display: flex;
        align-items: center;
        justify-content: center;
        transform: translateY(-50%);
    }

    :global(.rm-nebula-image-card img) {
        grid-column: 2;
        grid-row: 2;
        width: 100%;
        height: 100%;
        object-fit: cover;
        display: block;
        min-width: 0;
        min-height: 0;
    }

    :global(.rm-skill-image-card) {
        width: clamp(400px, 27.5vw, 625px);
        aspect-ratio: 10 / 20;
        margin-top: clamp(1rem, 3vh, 4rem);
        flex-shrink: 0;
    }

    :global(.rm-skill-image-card img) {
        width: 100%;
        height: 100%;
        object-fit: cover;
        display: block;
        min-width: 0;
        min-height: 0;
    }

    :global(.rm-tarot-star-stack) {
        position: absolute;
        width: 70%;
        aspect-ratio: 1;
    }

    :global(.rm-tarot-star) {
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

    :global(.rm-ts-1) {
        background: #ffffff;
        transform: scale(0.85);
    }
    :global(.rm-ts-2) {
        background: #000000;
        transform: scale(0.68);
    }
    :global(.rm-ts-3) {
        background: #ffffff;
        transform: scale(0.51);
    }
    :global(.rm-ts-4) {
        background: #000000;
        transform: scale(0.34);
    }
    :global(.rm-ts-5) {
        background: #ffffff;
        transform: scale(0.17);
    }

    :global(.rm-tarot-stripe) {
        position: absolute;
        top: 0;
        left: 40%;
        width: 35%;
        height: 100%;
        background: #e5191c;
        opacity: 0.35;
        transform: skewX(-20deg);
    }

    :global(.rm-tarot-card--leveled .rm-tarot-stripe) {
        opacity: 0.7;
    }

    :global(.rm-tarot-name-strip) {
        background: #e5191c;
        padding: 2px 5px;
        flex-shrink: 0;
        overflow: hidden;
    }

    :global(.rm-tarot-name) {
        display: block;
        font-size: 8px;
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: #ffffff;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    :global(.rm-tarot-bottom) {
        background: #000000;
        padding: 3px 5px;
        display: flex;
        align-items: center;
        gap: 4px;
        flex-shrink: 0;
    }

    :global(.rm-tarot-progress) {
        flex: 1;
        height: 4px;
        background: rgba(255, 255, 255, 0.15);
        overflow: hidden;
    }

    :global(.rm-tarot-progress-fill) {
        height: 100%;
        background: #e5191c;
        transition: width 300ms ease;
    }

    :global(.rm-tarot-lv) {
        font-size: 7px;
        font-weight: 700;
        letter-spacing: 0.06em;
        color: rgba(255, 255, 255, 0.6);
        flex-shrink: 0;
    }

    :global(.rm-tarot-card:not(.rm-tarot-card--leveled) .rm-tarot-card-inner) {
        border-color: rgba(255, 255, 255, 0.08);
    }

    :global(.rm-tarot-card:not(.rm-tarot-card--leveled) .rm-tarot-top) {
        background: rgba(255, 255, 255, 0.3);
    }

    :global(.rm-tarot-card:not(.rm-tarot-card--leveled) .rm-tarot-name-strip) {
        background: rgba(229, 25, 28, 0.35);
    }

    :global(.rm-tarot-card:not(.rm-tarot-card--leveled) .rm-tarot-lv) {
        color: rgba(255, 255, 255, 0.3);
    }

    :global(.rm-tarot-card--large) {
        width: clamp(400px, 27.5vw, 625px);
        margin-top: clamp(4rem, 10vh, 12rem);
        cursor: default;
        transform: none;
    }

    :global(.rm-tarot-card--large:hover) {
        transform: none;
    }

    @media (max-width: 980px) {
        .rm-command {
            position: relative;
            left: auto;
            top: auto;
            width: 100%;
            max-width: 660px;
            margin: 0.9rem auto 0;
            transform: rotate(0);
            padding: 0 0.6rem;
            box-sizing: border-box;
        }

        .rm-selection-quad {
            display: none;
        }
    }
</style>
