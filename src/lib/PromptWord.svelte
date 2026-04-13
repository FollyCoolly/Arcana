<!-- Persona 5 style prompt word label -->
<script lang="ts">
    let {
        text = "",
        fontSize = 36,
        fontFamily = "p5hatty",
        fontWeight = 300,
        inverted = false,
    }: {
        text?: string;
        fontSize?: number;
        fontFamily?: string;
        fontWeight?: number;
        inverted?: boolean;
    } = $props();

    let canvas: HTMLCanvasElement | undefined = $state();

    /* ── Tuning knobs ── */
    const MAX_ANGLE = 10; // max rotation per char (degrees)
    const MAX_Y_SHIFT = 0.12; // max vertical shift as ratio of fontSize
    const SIZE_VARIANCE = 0.14; // ±font-size jitter ratio
    const OUTLINE_RATIO = 0.14; // black outline thickness (relative to fontSize)
    const STROKE_RATIO = 0.055; // white outer stroke thickness
    const PAD_RATIO = 0.3; // canvas padding
    const SPACING_RATIO = 0.02; // extra letter gap (keeps outlines merged)

    /* ── Deterministic PRNG so the layout is stable per text ── */
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

    type CharItem = {
        ch: string;
        angle: number;
        dy: number;
        sz: number;
        isSpace: boolean;
    };

    async function draw(
        c: HTMLCanvasElement,
        txt: string,
        fs: number,
        ff: string,
        fw: number,
        inv: boolean,
    ) {
        await document.fonts.load(`${fw} ${fs}px "${ff}"`);

        const ctx = c.getContext("2d")!;
        const chars = [...txt];
        if (!chars.length) {
            c.width = 0;
            c.height = 0;
            return;
        }

        /* ── Colour scheme ── */
        const colText = inv ? "#000000" : "#ffffff";
        const colOutline = inv ? "#ffffff" : "#000000";
        const colStroke = inv ? "#000000" : "#ffffff";

        const rng = mkRng(txt);
        const outline = Math.max(2, Math.round(fs * OUTLINE_RATIO));
        const stroke = Math.max(1, Math.round(fs * STROKE_RATIO));
        const pad = Math.round(fs * PAD_RATIO);
        const gap = Math.max(1, Math.round(fs * SPACING_RATIO));

        /* ── Per-character layout ── */
        const items: CharItem[] = chars.map((ch) => {
            if (/\s/.test(ch))
                return { ch, angle: 0, dy: 0, sz: fs, isSpace: true };
            const angle = (rng() * 2 - 1) * MAX_ANGLE;
            const dy = (rng() * 2 - 1) * fs * MAX_Y_SHIFT;
            const scale = 1 + (rng() * 2 - 1) * SIZE_VARIANCE;
            return {
                ch,
                angle,
                dy,
                sz: Math.round(fs * scale),
                isSpace: false,
            };
        });

        /* ── Measure character advance widths ── */
        const widths = items.map((it) => {
            if (it.isSpace) return fs * 0.3;
            ctx.font = `${fw} ${it.sz}px "${ff}"`;
            return ctx.measureText(it.ch).width;
        });

        const textW =
            widths.reduce((s, w) => s + w, 0) +
            gap * Math.max(0, chars.length - 1);
        const margin = outline + stroke;

        c.width = Math.ceil(textW + pad * 2 + margin * 2);
        c.height = Math.ceil(fs * 1.6 + pad * 2 + margin * 2);
        ctx.clearRect(0, 0, c.width, c.height);

        const x0 = pad + margin;
        const yBase = c.height * 0.5 + fs * 0.33;

        ctx.textBaseline = "alphabetic";
        ctx.lineJoin = "miter";
        ctx.miterLimit = 3;
        ctx.lineCap = "square";

        /* ── Helpers ── */
        function eachVisible(
            fn: (it: CharItem, x: number, w: number, idx: number) => void,
        ) {
            let x = x0;
            for (let i = 0; i < items.length; i++) {
                if (!items[i].isSpace) fn(items[i], x, widths[i], i);
                x += widths[i] + gap;
            }
        }

        function withRotation(
            it: CharItem,
            x: number,
            w: number,
            fn: () => void,
        ) {
            ctx.save();
            const cx = x + w / 2;
            const cy = yBase - fs * 0.33;
            ctx.translate(cx, cy);
            ctx.rotate((it.angle * Math.PI) / 180);
            ctx.translate(-cx, -cy);
            fn();
            ctx.restore();
        }

        /*
         * Three-pass rendering (bottom → top):
         *   1. Outer stroke  — widest, sits at the very back
         *   2. Outline        — follows text shape, all chars merge
         *   3. Text fill      — crisp foreground letterforms
         *
         * Normal:   white outer → black outline → white fill
         * Inverted: black outer → white outline → black fill
         */

        /* ── Pass 1 · outer stroke ── */
        ctx.strokeStyle = colStroke;
        ctx.lineWidth = outline * 2 + stroke * 2;
        eachVisible((it, x, w) =>
            withRotation(it, x, w, () => {
                ctx.font = `${fw} ${it.sz}px "${ff}"`;
                ctx.strokeText(it.ch, x, yBase + it.dy);
            }),
        );

        /* ── Pass 2 · connected outline (stroke + fill) ── */
        ctx.strokeStyle = colOutline;
        ctx.lineWidth = outline * 2;
        eachVisible((it, x, w) =>
            withRotation(it, x, w, () => {
                ctx.font = `${fw} ${it.sz}px "${ff}"`;
                ctx.strokeText(it.ch, x, yBase + it.dy);
            }),
        );
        /* Fill ensures the body is solid before the top layer */
        ctx.fillStyle = colOutline;
        eachVisible((it, x, w) =>
            withRotation(it, x, w, () => {
                ctx.font = `${fw} ${it.sz}px "${ff}"`;
                ctx.fillText(it.ch, x, yBase + it.dy);
            }),
        );

        /* ── Pass 3 · text fill ── */
        ctx.fillStyle = colText;
        eachVisible((it, x, w) =>
            withRotation(it, x, w, () => {
                ctx.font = `${fw} ${it.sz}px "${ff}"`;
                ctx.fillText(it.ch, x, yBase + it.dy);
            }),
        );
    }

    $effect(() => {
        const c = canvas;
        const t = text;
        const fs = fontSize;
        const ff = fontFamily;
        const fw = fontWeight;
        const inv = inverted;
        if (c && t) draw(c, t, fs, ff, fw, inv);
    });
</script>

<canvas bind:this={canvas} class="p5-prompt-word"></canvas>

<style>
    .p5-prompt-word {
        display: block;
        pointer-events: none;
    }
</style>
