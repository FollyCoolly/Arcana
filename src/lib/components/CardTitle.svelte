<script lang="ts">
    import { buildCardTitleSvg, type CardTitleOptions } from '$lib/utils/cardTitle';

    let {
        text,
        seed = 0,
        options = {},
    }: {
        text: string;
        seed?: number;
        options?: CardTitleOptions;
    } = $props();

    const svg = $derived(buildCardTitleSvg(text, seed, options));
</script>

{@html svg}
