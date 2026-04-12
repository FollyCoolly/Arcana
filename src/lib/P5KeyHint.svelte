<script lang="ts">
    let {
        key = "",
        fontSize = 36,
        fontFamily = "Bebas Neue",
    }: {
        key?: string;
        fontSize?: number;
        fontFamily?: string;
    } = $props();
</script>

<span
    class="p5-key-hint"
    style:font-size="{fontSize}px"
    style:font-family="{fontFamily}, Arial, sans-serif"
>
    {key}
</span>

<style>
    .p5-key-hint {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        background: #000000;
        color: #ffffff;
        border: 2px solid #ffffff;
        font-weight: 700;
        line-height: 1;
        width: 1.5em;
        height: 1.5em;
        padding: 0.1em 0 0 0;
        pointer-events: none;
        white-space: nowrap;
    }
</style>
