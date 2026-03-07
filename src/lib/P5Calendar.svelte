<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  const BASE_W = 835;
  const BASE_H = 653;

  // Date state
  let day = $state(0);
  let monthNum = $state(0);
  let monthKey = $state(0);
  let weekday = $state("");
  let todIndex = $state(1);

  // Weather state
  let weatherIcon = $state("");
  let weatherFrame = $state(0);
  let weatherInterval: ReturnType<typeof setInterval> | null = null;

  function getTimeOfDay(hour: number): number {
    if (hour < 1) return 1;
    if (hour < 7) return 2;
    if (hour < 11) return 3;
    if (hour < 14) return 4;
    if (hour < 19) return 5;
    if (hour < 21) return 6;
    return 1;
  }

  function getWeatherFrameSet(dayNum: number): number {
    // day >= 10 uses icons0/1/2, day < 10 uses icons3/4/5
    return dayNum >= 10 ? 0 : 3;
  }

  function updateDate() {
    const now = new Date();
    day = now.getDate();
    monthNum = now.getMonth() + 1;
    // If day < 10, month key is just the month number (1-12)
    // If day >= 10, month key is month * 100 (100-1200) for shifted position
    monthKey = day < 10 ? monthNum : monthNum * 100;
    weekday = now.toLocaleDateString("en-US", { weekday: "long" });
    todIndex = getTimeOfDay(now.getHours());
  }

  async function fetchWeather() {
    try {
      const result = await invoke<{ icon: string }>("get_weather");
      weatherIcon = result.icon;
    } catch {
      weatherIcon = "";
    }
  }

  onMount(() => {
    updateDate();
    fetchWeather();

    // Animate weather frames: cycle through 3 frames every 300ms
    weatherInterval = setInterval(() => {
      weatherFrame = (weatherFrame + 1) % 3;
    }, 300);

    return () => {
      if (weatherInterval) clearInterval(weatherInterval);
    };
  });

  let frameBase = $derived(getWeatherFrameSet(day));
  let currentWeatherFolder = $derived(`icons${frameBase + weatherFrame}`);
</script>

<div class="p5cal">
  {#if day > 0}
    <!-- Layer 1: Day Bottom (white background) -->
    <img
      src="/ui/calendar/day/{day}Bottom.png"
      alt=""
      class="p5cal-layer"

    />

    <!-- Layer 2: Month Bottom -->
    <img
      src="/ui/calendar/month/{monthKey}Bottom.png"
      alt=""
      class="p5cal-layer"

    />

    <!-- Layer 3: Weekday Bottom -->
    <img
      src="/ui/calendar/week/{weekday}Bottom.png"
      alt=""
      class="p5cal-layer"

    />

    <!-- Layer 4: Time of Day -->
    <img
      src="/ui/calendar/tod/{todIndex}.png"
      alt=""
      class="p5cal-layer"

    />

    <!-- Layer 5: Weather (animated) -->
    {#if weatherIcon}
      <img
        src="/ui/calendar/weather/{currentWeatherFolder}/{weatherIcon}.png"
        alt=""
        class="p5cal-layer"
  
      />
    {/if}

    <!-- Layer 6: Day foreground (black) -->
    <img
      src="/ui/calendar/day/{day}.png"
      alt=""
      class="p5cal-layer"

    />

    <!-- Layer 7: Month foreground -->
    <img
      src="/ui/calendar/month/{monthKey}.png"
      alt=""
      class="p5cal-layer"

    />

    <!-- Layer 8: Weekday foreground -->
    <img
      src="/ui/calendar/week/{weekday}.png"
      alt=""
      class="p5cal-layer"

    />

    <!-- Layer 9: Day Top (colored text) -->
    <img
      src="/ui/calendar/day/{day}Top.png"
      alt=""
      class="p5cal-layer"

    />

    <!-- Layer 10: Month Top -->
    <img
      src="/ui/calendar/month/{monthKey}Top.png"
      alt=""
      class="p5cal-layer"

    />

    <!-- Layer 11: Weekday Top -->
    <img
      src="/ui/calendar/week/{weekday}Top.png"
      alt=""
      class="p5cal-layer"

    />
  {/if}
</div>

<style>
  .p5cal {
    position: relative;
    width: 100%;
    aspect-ratio: 835 / 653;
    overflow: hidden;
  }

  .p5cal-layer {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: auto;
    pointer-events: none;
    user-select: none;
  }
</style>
