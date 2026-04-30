<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

  // Cancel Windows display scaling so CSS viewport equals the monitor's
  // physical pixel count — regardless of whether the user set Windows scale
  // to 100% / 125% / 150%. setZoom(1/DPR) tells WebView2 to re-evaluate the
  // viewport at that factor, unlike CSS `zoom` which leaves fixed/vh/vw
  // elements anchored to the real viewport.
  function isWindows() {
    const platform = navigator.platform || '';
    const userAgent = navigator.userAgent || '';
    return /Win/.test(platform) || /Windows/.test(userAgent);
  }

  async function applyZoom() {
    if (!isWindows()) return;
    const win = getCurrentWebviewWindow();
    await win.setZoom(1 / window.devicePixelRatio);
  }

  onMount(() => {
    applyZoom();

    if (!isWindows()) return;

    const win = getCurrentWebviewWindow();
    const unlistenPromise = win.onScaleChanged(() => {
      applyZoom();
    });

    return () => {
      unlistenPromise.then((fn) => fn());
    };
  });

  let { children } = $props();
</script>

{@render children()}
