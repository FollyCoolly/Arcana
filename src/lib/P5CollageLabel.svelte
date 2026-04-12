<script lang="ts">
    let {
        text,
        level = null,
        title = null,
    }: {
        text: string;
        level?: number | null;
        title?: string | null;
    } = $props();

    /** Deterministic PRNG — same text always yields same layout */
    function mkRng(s: string) {
        let h = 0;
        for (let i = 0; i < s.length; i++)
            h = ((h << 5) - h + s.charCodeAt(i)) | 0;
        return () => {
            h |= 0;
            h = (h + 0x6d2b79f5) | 0;
            let t = Math.imul(h ^ (h >>> 15), 1 | h);
            t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
            return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
        };
    }

    type Fragment = {
        chars: string;
        rotate: number;
        yShift: number;
        gap: number;
        padL: number;
        padR: number;
        inverted: boolean;
    };

    function buildFragments(word: string): Fragment[] {
        const rng = mkRng(word);
        const upper = word.toUpperCase();
        const len = upper.length;
        if (len === 0) return [];

        // Fragment count based on word length
        let count: number;
        if (len <= 3) count = 2;
        else if (len <= 6) count = rng() < 0.5 ? 2 : 3;
        else count = rng() < 0.4 ? 3 : 4;
        count = Math.min(count, len);

        const frags: Fragment[] = [];
        let pos = 0;

        for (let f = 0; f < count; f++) {
            const remaining = len - pos;
            const remainingFrags = count - f;
            const avg = remaining / remainingFrags;
            const fragLen =
                f === count - 1
                    ? remaining
                    : Math.max(1, Math.round(avg + (rng() - 0.5) * 1.5));

            frags.push({
                chars: upper.slice(pos, pos + fragLen),
                rotate: (rng() * 2 - 1) * 6,
                yShift: (rng() * 2 - 1) * 0.07,
                // Slight overlap between adjacent fragments for collage feel
                gap: f === 0 ? 0 : -0.05 + rng() * 0.04,
                // Asymmetric horizontal padding per fragment
                padL: 0.06 + rng() * 0.06,
                padR: 0.06 + rng() * 0.06,
                inverted: false,
            });
            pos += fragLen;
        }

        // Decide how many fragments to invert (~40%), then pick which ones
        const invertCount = Math.round(count * 0.4);
        if (invertCount > 0) {
            // Assign a random score to each fragment, pick the lowest N
            const scores = frags.map(() => rng());
            const ranked = scores
                .map((s, i) => ({ s, i }))
                .sort((a, b) => a.s - b.s);
            for (let k = 0; k < invertCount; k++) {
                frags[ranked[k].i].inverted = true;
            }
        }

        return frags;
    }

    const fragments = $derived(buildFragments(text));
</script>

<span class="p5cl">
    <span class="p5cl-frags">
        {#each fragments as frag}
            <span
                class="p5cl-frag"
                class:p5cl-inv={frag.inverted}
                style:transform="rotate({frag.rotate}deg) translateY({frag.yShift}em)"
                style:margin-left="{frag.gap}em"
                style:padding-left="{frag.padL}em"
                style:padding-right="{frag.padR}em"
            >
                {frag.chars}
            </span>
        {/each}
        {#if level !== null && level !== undefined}
            <span class="p5cl-badge">{level}</span>
        {/if}
    </span>
    {#if title}
        <span class="p5cl-title">{title}</span>
    {/if}
</span>

<style>
    /* ── Container ── */
    .p5cl {
        display: inline-flex;
        flex-direction: column;
        align-items: center;
        gap: 0.12em;
    }

    /* ── Fragment row ── */
    .p5cl-frags {
        display: inline-flex;
        align-items: center;
        white-space: nowrap;
    }

    /* ── Individual fragment block: gold bg, black text ── */
    .p5cl-frag {
        display: inline-block;
        background: var(--rm-gold, #f5a623);
        color: var(--rm-black, #000);
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-weight: 800;
        font-size: 1em;
        line-height: 1;
        padding-top: 0.06em;
        padding-bottom: 0.12em;
        transform-origin: center center;
        /* Slight shadow to lift blocks off the background */
        box-shadow: 0.04em 0.06em 0 rgba(0, 0, 0, 0.35);
    }

    /* ── Inverted variant: gold text on black bg, gold outer wrap ── */
    .p5cl-frag.p5cl-inv {
        background: var(--rm-black, #000);
        color: var(--rm-gold, #f5a623);
        /* Gold outline wrap around the black block */
        box-shadow:
            0 0 0 0.07em var(--rm-gold, #f5a623),
            0.04em 0.06em 0 rgba(0, 0, 0, 0.35);
    }

    /* ── Level badge: small gold square, bottom-right, slightly below text ── */
    .p5cl-badge {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        background: var(--rm-gold, #f5a623);
        color: var(--rm-black, #000);
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-weight: 800;
        font-size: 0.58em;
        min-width: 1.3em;
        padding: 0.08em 0.18em 0.12em;
        line-height: 1;
        align-self: flex-end;
        transform: rotate(-5deg) translateY(0.25em);
        margin-left: 0.06em;
        box-shadow: 0.04em 0.06em 0 rgba(0, 0, 0, 0.35);
    }

    /* ── Rank title below fragments ── */
    .p5cl-title {
        font-family: "p5hatty", "Orbitron", Arial, sans-serif;
        font-size: 0.5em;
        font-weight: 700;
        color: var(--rm-white, #fff);
        line-height: 1;
        letter-spacing: 0.03em;
    }
</style>
