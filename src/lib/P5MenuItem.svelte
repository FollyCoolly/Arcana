<script lang="ts">
  export type LetterConfig = {
    char: string;
    size?: string;
    yOffset?: number;
    rotate?: number;
    weight?: number;
    color?: 'white' | 'black';
    outline?: boolean;
    rounded?: boolean;
    special?: 'star-dot' | 'star-counter';
  };

  let { letters, active = false }: {
    letters: LetterConfig[];
    active?: boolean;
  } = $props();
</script>

<span class="p5m" class:is-active={active}>
  {#each letters as l}
    <span
      class="p5m-char"
      class:p5m-black={l.color === 'black' && !l.outline}
      class:p5m-outline={l.color === 'black' && l.outline}
      class:p5m-rounded={l.color === 'black' && !l.outline && l.rounded}
      data-special={l.special ?? null}
      style:font-size={l.size ?? '1em'}
      style:font-weight={l.weight ?? 700}
      style:transform="translateY({l.yOffset ?? 0}px) rotate({l.rotate ?? 0}deg)"
    >{l.char}</span>
  {/each}
</span>

<style>
  .p5m {
    display: inline-flex !important;
    align-items: baseline;
    font-family: "p5hatty", "Orbitron", Arial, sans-serif;
    font-size: clamp(3.5rem, 11vw, 6.5rem);
    letter-spacing: 0.02em;
    white-space: nowrap;
  }

  .p5m-char {
    display: inline-block;
    position: relative;
    line-height: 1;
    margin: 0 0.06em;
    transform-origin: center bottom;
    color: #fff;
    transition: color 140ms ease;
  }

  /* ── Black variant: black text with white rectangle behind ── */
  .p5m-char.p5m-black {
    color: #000;
  }

  .p5m-char.p5m-black::before {
    content: '';
    position: absolute;
    top: 0.15em;
    bottom: 0.3em;
    left: -0.04em;
    right: -0.04em;
    background: #fff;
    z-index: -1;
    transform: scale(1.1);
    transition: background 140ms ease;
  }

  .p5m-char.p5m-black.p5m-rounded::before {
    border-radius: 0.08em;
  }

  /* ── Outline variant: black text with white stroke ── */
  .p5m-char.p5m-outline {
    color: #000;
    -webkit-text-stroke: 15px #fff;
    paint-order: stroke fill;
  }

  /* ── Active state: white letters → bright red ── */
  .p5m.is-active .p5m-char:not(.p5m-black):not(.p5m-outline) {
    color: #ff003c;
  }

  /* Rectangle turns red when active */
  .p5m.is-active .p5m-char.p5m-black::before {
    background: #ff003c;
  }

  /* Outline stroke turns red when active */
  .p5m.is-active .p5m-char.p5m-outline {
    -webkit-text-stroke-color: #ff003c;
  }

  /* ── star-dot: replace the dot on "i" with a ★ ── */
  .p5m-char[data-special="star-dot"] {
    overflow: visible;
  }

  .p5m-char[data-special="star-dot"]::after {
    content: "★";
    position: absolute;
    top: -0.08em;
    left: 50%;
    transform: translateX(-50%) rotate(-12deg) scale(0.55);
    font-size: 1em;
    line-height: 1;
    color: inherit;
    -webkit-text-stroke: 0;
    pointer-events: none;
  }

  /* ── star-counter: fill enclosed counter of letter with a star matching bg ── */
  /* Only use on letters with enclosed counters: A B D O P Q R */
  .p5m-char[data-special="star-counter"] {
    overflow: visible;
  }

  .p5m-char[data-special="star-counter"]::after {
    content: "★";
    position: absolute;
    bottom: 0.18em;
    left: 50%;
    transform: translateX(-50%) rotate(8deg);
    font-size: 0.48em;
    line-height: 1;
    /* Star color matches the menu item background to create hollow illusion */
    color: var(--rm-black, #000);
    -webkit-text-stroke: 0;
    pointer-events: none;
    transition: color 140ms ease;
  }

  /* When active, bg is red so counter star must become red to keep illusion */
  .p5m.is-active .p5m-char[data-special="star-counter"]::after {
    color: var(--rm-red, #80001a);
  }
</style>
