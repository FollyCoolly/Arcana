<!-- Persona 5 calling card style: tilted letters with random rotation, black bases, red accents, white glow -->
<script lang="ts">
    let { text = '', fontSize = 60, fontFamily = 'p5hatty' }: {
        text?: string;
        fontSize?: number;
        fontFamily?: string;
    } = $props();

    let canvas: HTMLCanvasElement | undefined = $state();

    const COLORS = { RED: '#E5191C', WHITE: '#FDFDFD', BLACK: '#0F0F0F' };
    const MAX_ANGLE = 10;
    const GUTTER = 5;
    const PADDING = 20;
    const BORDER_SCALE = 1.4;
    const BG_SCALE = 1.2;
    const GLOW_SIZE = 6;

    function randomOp() { return Math.floor(Math.random() * 10) % 2 ? 1 : -1; }

    function canvasRotate(ctx: CanvasRenderingContext2D, angle: number, x: number, y: number) {
        ctx.translate(x, y);
        ctx.rotate(Math.PI * angle / 180);
        ctx.translate(-x, -y);
    }

    function getCharMetrics(char: string, size: number, family: string) {
        const tmp = document.createElement('canvas');
        tmp.width = 1; tmp.height = 1;
        const ctx = tmp.getContext('2d')!;
        ctx.font = `bold ${size}px ${family}`;
        const m = ctx.measureText(char);
        return {
            width: m.actualBoundingBoxLeft + m.actualBoundingBoxRight,
            height: m.actualBoundingBoxAscent + m.actualBoundingBoxDescent,
            leftOffset: m.actualBoundingBoxLeft,
            ascentOffset: m.actualBoundingBoxAscent,
        };
    }

    function parseChars(input: string) {
        const result: { char: string; isRed: boolean; isFirst: boolean; isSpace: boolean }[] = [];
        const upper = input.toUpperCase();
        let i = 0;
        let isFirst = true;

        while (i < upper.length) {
            const ch = upper[i];
            if (ch === '*' && i + 2 < upper.length && upper[i + 2] === '*') {
                result.push({ char: upper[i + 1], isRed: true, isFirst, isSpace: false });
                isFirst = false;
                i += 3;
            } else if (/^\s$/.test(ch)) {
                result.push({ char: '', isRed: false, isFirst: false, isSpace: true });
                i++;
            } else {
                result.push({ char: ch, isRed: false, isFirst, isSpace: false });
                isFirst = false;
                i++;
            }
        }

        // 随机加红（每5字内最多一个，不覆盖手动指定的）
        for (let j = 1; j < result.length; j += 5) {
            for (let k = j; k < j + 4 && k < result.length; ++k) {
                if (!result[k].isSpace && !result[k].isFirst && !result[k].isRed) {
                    if (Math.random() > 0.6) { result[k].isRed = true; break; }
                }
            }
        }

        return result;
    }

    async function draw(c: HTMLCanvasElement, t: string, fs: number, ff: string) {
        await document.fonts.load(`bold ${fs}px ${ff}`);

        const ctx = c.getContext('2d')!;
        const parsed = parseChars(t);

        const layouts = parsed.map(({ char, isRed, isFirst, isSpace }) => {
            if (isSpace) {
                return { char: '', isFirst: false, isRed: false, isSpace: true, charFontSize: 0, angle: 0, metrics: { width: 0, height: 0, leftOffset: 0, ascentOffset: 0 }, outerW: GUTTER * 2, outerH: 0 };
            }
            const angle = -(Math.round(Math.random() * 10) % MAX_ANGLE);
            const scale = isFirst ? 1.1 : 1 - Math.floor(Math.random() * 10) % 3 / 10;
            const finalAngle = isFirst ? angle : angle * randomOp();
            const charFontSize = fs * scale;
            const metrics = getCharMetrics(char, charFontSize, ff);
            const rad = Math.abs(finalAngle) * Math.PI / 180;
            const sin = Math.abs(Math.sin(rad)), cos = Math.abs(Math.cos(rad));
            const rotateW = Math.ceil(metrics.width * cos) + Math.ceil(metrics.height * sin);
            const rotateH = Math.ceil(metrics.height * cos) + Math.ceil(metrics.width * sin);
            const outerScale = isFirst ? BORDER_SCALE : BG_SCALE;
            return { char, isFirst, isRed, isSpace: false, charFontSize, angle: finalAngle, metrics, outerW: rotateW * outerScale, outerH: rotateH * outerScale };
        });

        const canvasWidth = PADDING * 2 + layouts.reduce((sum, l) => sum + l.outerW + GUTTER, 0);
        const canvasHeight = PADDING * 2 + Math.max(0, ...layouts.map(l => l.outerH));

        c.width = canvasWidth;
        c.height = canvasHeight;
        ctx.clearRect(0, 0, canvasWidth, canvasHeight);

        let drawOffset = PADDING;

        for (const l of layouts) {
            if (l.isSpace) { drawOffset += GUTTER * 2; continue; }

            const { char, isFirst, isRed, charFontSize, angle, metrics, outerW, outerH } = l;
            ctx.save();

            const rotateX = drawOffset + outerW / 2;
            const boxTop = (canvasHeight - outerH) / 2;
            const rotateY = boxTop + outerH / 2;
            const textX = drawOffset + (outerW - metrics.width) / 2 + metrics.leftOffset;
            const textY = boxTop + (outerH - metrics.height) / 2 + metrics.ascentOffset;

            if (isFirst) {
                canvasRotate(ctx, angle - 5, rotateX, rotateY);
                ctx.fillStyle = COLORS.BLACK;
                ctx.fillRect(drawOffset, boxTop, outerW, outerH);

                canvasRotate(ctx, 3, rotateX, rotateY);
                const bgW = outerW * 0.85, bgH = outerH * 0.85;
                ctx.fillStyle = COLORS.RED;
                ctx.fillRect(drawOffset + (outerW - bgW) / 2, boxTop + (outerH - bgH) / 2, bgW, bgH);

                canvasRotate(ctx, 2, rotateX, rotateY);
                ctx.fillStyle = COLORS.WHITE;
            } else {
                canvasRotate(ctx, angle + 1, rotateX, rotateY);
                ctx.fillStyle = COLORS.BLACK;
                ctx.fillRect(drawOffset, boxTop, outerW, outerH);

                canvasRotate(ctx, -1, rotateX, rotateY);
                ctx.fillStyle = isRed ? COLORS.RED : COLORS.WHITE;
            }

            ctx.font = `bold ${charFontSize}px ${ff}`;
            ctx.textBaseline = 'alphabetic';
            ctx.fillText(char, textX, textY);
            ctx.restore();
            drawOffset += outerW + GUTTER;
        }

        // 白色光晕轮廓
        const imageData = ctx.getImageData(0, 0, canvasWidth, canvasHeight);
        const glowData = ctx.createImageData(canvasWidth, canvasHeight);
        const half = Math.floor(GLOW_SIZE / 2);

        for (let i = half; i < imageData.height - half; ++i) {
            for (let j = half; j < imageData.width - half; ++j) {
                const idx = (i * imageData.width + j) * 4;
                if (!imageData.data[idx + 3]) continue;
                const a = imageData.data[idx + 3];
                for (let dx = i - GLOW_SIZE + 1; dx < i + GLOW_SIZE; ++dx) {
                    for (let dy = j - GLOW_SIZE + 1; dy < j + GLOW_SIZE; ++dy) {
                        const nIdx = (dx * imageData.width + dy) * 4;
                        glowData.data[nIdx] = 255;
                        glowData.data[nIdx + 1] = 255;
                        glowData.data[nIdx + 2] = 255;
                        glowData.data[nIdx + 3] = Math.min(255, glowData.data[nIdx + 3] + a / 4);
                    }
                }
            }
        }

        const glowCanvas = document.createElement('canvas');
        glowCanvas.width = canvasWidth;
        glowCanvas.height = canvasHeight;
        glowCanvas.getContext('2d')!.putImageData(glowData, 0, 0);

        ctx.save();
        ctx.globalCompositeOperation = 'destination-over';
        ctx.drawImage(glowCanvas, 0, 0);
        ctx.restore();
    }

    $effect(() => {
        const c = canvas;
        const t = text;
        const fs = fontSize;
        const ff = fontFamily;
        if (c && t) {
            draw(c, t, fs, ff);
        }
    });
</script>

<canvas bind:this={canvas}></canvas>
