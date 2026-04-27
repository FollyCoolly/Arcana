export interface CardTitleOptions {
    rotRange?: number;             // max rotation per char, degrees     (default 4)
    dyRange?: number;              // vertical jitter as fraction of fs   (default 0.14)
    scaleRange?: [number, number]; // per-char scale [min, max]           (default [0.92, 1.08])
    lean?: number;                 // global skewX degrees                (default 9)
    strokeRatio?: number;          // stroke-width / fontSize             (default 0.24)
    targetWidth?: number;          // fraction of VB_W to fill            (default 0.82)
}

const VB_W = 200;
const VB_H = 70;
const BASE_FS = 40;
// Tighter-than-natural advance width so thick strokes bleed into neighbours,
// creating one connected block rather than a row of separate letters.
const CHAR_W = 0.47;
const SPACE_W = 0.22;
const BASE_Y = VB_H * 0.50;

// Maximum structural rotation bias in degrees.
// Random jitter is added on top; the structural component dominates.
const STRUCT_MAX = 10;

function rand(i: number, salt: number, seed: number): number {
    const x = Math.sin(i * 127.1 + salt * 311.7 + seed * 17.3) * 43758.5453;
    return x - Math.floor(x);
}

/**
 * Structural rotation bias for character i out of n total.
 * shapeType (derived from seed % 3):
 *   0 = V  — monotonic ramp, left leans CCW, right leans CW
 *   1 = W  — V ramp + positive-first sine wave (two-valley oscillation)
 *   2 = M  — V ramp + negative-first sine wave (enhanced outer extremes)
 */
function structBias(i: number, n: number, shapeType: number): number {
    if (n <= 1) return 0;
    const t = i / (n - 1);                             // 0..1
    const linear = (t * 2 - 1) * STRUCT_MAX;           // V foundation

    if (shapeType === 1) {
        // W: combine reduced linear with a full-period sine oscillation
        return linear * 0.45 + Math.sin(t * Math.PI * 2) * STRUCT_MAX * 0.80;
    }
    if (shapeType === 2) {
        // M: same but negated sine — reverses the oscillation phase
        return linear * 0.45 - Math.sin(t * Math.PI * 2) * STRUCT_MAX * 0.80;
    }
    return linear; // V
}

function escapeXml(ch: string): string {
    if (ch === '&') return '&amp;';
    if (ch === '<') return '&lt;';
    if (ch === '>') return '&gt;';
    return ch;
}

export function buildCardTitleSvg(text: string, seed = 0, opts: CardTitleOptions = {}): string {
    const {
        rotRange    = 4,
        dyRange     = 0.14,
        scaleRange  = [0.92, 1.08],
        lean        = 9,
        strokeRatio = 0.24,
        targetWidth = 0.68,
    } = opts;

    const chars = text.toUpperCase().split('');
    const n = chars.length;
    const shapeType = seed % 3; // 0=V, 1=W, 2=M

    // Scale font so text fills ~targetWidth of the card width.
    const rawW = chars.reduce((s, c) => s + BASE_FS * (c === ' ' ? SPACE_W : CHAR_W), 0);
    const scale = Math.min((VB_W * targetWidth) / rawW, 1.15);
    const fs = Math.max(BASE_FS * scale, 12);

    const cws = chars.map(c => fs * (c === ' ' ? SPACE_W : CHAR_W));
    const totalW = cws.reduce((a, b) => a + b, 0);
    let curX = (VB_W - totalW) / 2;

    const texts = chars.map((ch, i) => {
        const cw = cws[i];
        const cx = curX + cw / 2;
        curX += cw;
        if (ch === ' ') return '';

        // Structural bias dominates; small random jitter adds per-char character.
        const rot = structBias(i, n, shapeType) + (rand(i, 0, seed) - 0.5) * rotRange * 2;
        const dy  = (rand(i, 1, seed) - 0.5) * fs * dyRange * 2;
        const s   = scaleRange[0] + rand(i, 2, seed) * (scaleRange[1] - scaleRange[0]);
        const afs = fs * s;
        const cy  = BASE_Y + dy;
        const sw  = (afs * strokeRatio).toFixed(2);

        return `<text x="${cx.toFixed(2)}" y="${cy.toFixed(2)}" `
            + `text-anchor="middle" dominant-baseline="central" `
            + `font-family="'方正兰亭黑_GBK', Arial, sans-serif" `
            + `font-size="${afs.toFixed(2)}" `
            + `fill="#ffffff" stroke="#000000" stroke-width="${sw}" `
            + `paint-order="stroke fill" `
            + `stroke-linejoin="miter" stroke-miterlimit="3" `
            + `transform="rotate(${rot.toFixed(2)}, ${cx.toFixed(2)}, ${cy.toFixed(2)})">`
            + `${escapeXml(ch)}</text>`;
    }).join('');

    return `<svg viewBox="0 0 ${VB_W} ${VB_H}" width="100%" height="100%" `
        + `overflow="visible" xmlns="http://www.w3.org/2000/svg">`
        + `<g transform="skewX(${lean})">${texts}</g></svg>`;
}

export function hashStr(s: string): number {
    let h = 0;
    for (let i = 0; i < s.length; i++) {
        h = (Math.imul(31, h) + s.charCodeAt(i)) | 0;
    }
    return Math.abs(h);
}
