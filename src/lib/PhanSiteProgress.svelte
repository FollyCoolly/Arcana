<script lang="ts">
    type Props = {
        question: string;
        progress: number;
    };

    let { question, progress }: Props = $props();

    const clamped = $derived(Math.max(0, Math.min(100, progress)));
    const intPart = $derived(Math.floor(clamped).toString());
    const decimalPart = $derived(
        (clamped - Math.floor(clamped)).toFixed(1).slice(1),
    );
    const isHigh = $derived(clamped >= 70);

    type Segment = { text: string; emphasis: boolean };
    const segments = $derived<Segment[]>(
        question
            .split(/(\*\*[^*]+\*\*)/g)
            .filter((part) => part.length > 0)
            .map((part) =>
                part.startsWith("**") && part.endsWith("**")
                    ? { text: part.slice(2, -2), emphasis: true }
                    : { text: part, emphasis: false },
            ),
    );
</script>

<div class="phansite" aria-label="Mission progress">
    <div class="phansite-bolts" aria-hidden="true">
        <img src="/ui/phansite/bolt.png" alt="" class="bolt bolt-a" />
        <img src="/ui/phansite/bolt.png" alt="" class="bolt bolt-b" />
    </div>

    <div class="phansite-header">
        <div class="phansite-triangle" aria-hidden="true"></div>
        <img src="/ui/phansite/tag.png" alt="Phan-Site" class="phansite-tag" />
        <span class="phansite-question"
            >{#each segments as seg}{#if seg.emphasis}<span
                        class="phansite-emphasis">{seg.text}</span
                    >{:else}{seg.text}{/if}{/each}</span
        >
    </div>

    <div class="phansite-bar-row">
        <div class="phansite-bar">
            <img
                src="/ui/phansite/q_icon.png"
                alt=""
                class="phansite-q"
                aria-hidden="true"
            />
            <svg
                class="phansite-bar-svg"
                viewBox="0 0 795 133"
                preserveAspectRatio="none"
                aria-hidden="true"
            >
                <defs>
                    <!-- Outer white frame as a ring (even-odd fill rule) -->
                    <clipPath
                        id="phansite-inner-clip"
                        clipPathUnits="userSpaceOnUse"
                    >
                        <polygon points="24,66 751,29 745,99 22,119" />
                    </clipPath>
                </defs>
                <!-- White frame only: outer polygon with inner hole -->
                <path
                    fill="var(--rm-white)"
                    fill-rule="evenodd"
                    d="M 0,43 L 795,0 L 775,125 L 0,133 Z
                       M 16,57 L 757,20 L 751,109 L 14,128 Z"
                />
                <!-- Fill (white < 70%, red >= 70%) clipped to inset inner quad -->
                <rect
                    x="0"
                    y="0"
                    width={795 * (clamped / 100)}
                    height="133"
                    fill={isHigh ? "var(--rm-red)" : "var(--rm-white)"}
                    clip-path="url(#phansite-inner-clip)"
                    class="phansite-fill"
                />
            </svg>
            <img
                src="/ui/phansite/yes.png"
                alt=""
                class="phansite-yes"
                aria-hidden="true"
            />
        </div>

        <span class="phansite-percent" class:is-high={isHigh}>
            {intPart}<span class="phansite-percent-small">{decimalPart}%</span>
        </span>
    </div>
</div>

<style>
    .phansite {
        position: absolute;
        bottom: clamp(3rem, 5vh, 6rem);
        right: clamp(-2rem, -3vw, -3rem);
        z-index: 3;
        pointer-events: none;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 0.35rem;
        transform: rotate(-1deg);
    }

    .phansite-bolts {
        position: absolute;
        top: -5rem;
        left: 13.5rem;
        display: flex;
        gap: 1.5rem;
        pointer-events: none;
    }

    .bolt {
        height: clamp(2.2rem, 3vw, 3rem);
        width: auto;
        display: block;
    }

    .bolt-b {
        transform: scaleX(2) rotate(20deg);
    }

    .bolt-a {
        transform: rotate(-20deg);
    }

    .phansite-header {
        position: relative;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        white-space: nowrap;
    }

    .phansite-q {
        height: clamp(5.4rem, 7vw, 7rem);
        width: auto;
        display: block;
        position: absolute;
        left: 2rem;
        top: -1.5rem;
        transform: translate(-50%, -50%) rotate(8deg);
        z-index: 4;
    }

    .phansite-triangle {
        position: absolute;
        width: 4.4rem;
        height: 2rem;
        background: var(--rm-white);
        clip-path: polygon(50% 0%, 100% 100%, 0% 100%);
        top: -1.3rem;
        left: 12.5rem;
        z-index: 2;
        transform: rotate(130deg) scale(0.9);
    }

    .phansite-tag {
        height: clamp(3.8rem, 5vw, 5rem);
        width: auto;
        display: block;
        margin-left: 6rem;
        transform: translateY(-0.5rem) rotate(-10deg);
    }

    .phansite-question {
        color: var(--rm-white);
        font-family: "方正兰亭黑_GB", Arial, sans-serif;
        font-weight: 700;
        font-size: clamp(1.1rem, 1.5vw, 1.5rem);
        letter-spacing: 0.01em;
        -webkit-text-stroke: 0.5em var(--rm-black);
        paint-order: stroke fill;
        text-align: left;
        display: inline-block;
        transform: translate(-13rem, 3.5rem) rotate(-5deg);
        transform-origin: left center;
        position: relative;
        z-index: 10;
    }

    .phansite-emphasis {
        color: var(--rm-red);
        font-size: 1.3em;
        font-weight: 900;
        letter-spacing: 0.02em;
        margin: 0 0.08em;
    }

    .phansite-bar-row {
        display: flex;
        align-items: center;
        gap: 0.4rem;
    }

    .phansite-bar {
        position: relative;
        width: clamp(300px, 33vw, 510px);
        aspect-ratio: 795 / 133;
        display: block;
    }

    .phansite-bar-svg {
        width: 100%;
        height: 100%;
        display: block;
        overflow: visible;
    }

    .phansite-fill {
        transition: width 400ms cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    .phansite-yes {
        position: absolute;
        right: -1.5%;
        bottom: 6%;
        height: 40%;
        width: auto;
        z-index: 2;
    }

    .phansite-percent {
        color: var(--rm-white);
        font-family: "方正兰亭黑_GB", Arial, sans-serif;
        font-weight: 900;
        font-style: italic;
        font-size: clamp(4.8rem, 7.2vw, 7.2rem);
        line-height: 1;
        letter-spacing: 0.01em;
        -webkit-text-stroke: 0.03em var(--rm-black);
        paint-order: stroke fill;
        transition: color 200ms ease;
        margin-top: -1rem;
        transform: scaleX(0.7);
        transform-origin: left center;
    }

    .phansite-percent.is-high {
        color: var(--rm-red);
    }

    .phansite-percent-small {
        font-size: 0.7em;
        margin-left: 0.02em;
    }
</style>
