<script lang="ts">
  import { Sun, Moon } from '@lucide/svelte';
  import { onMount } from 'svelte';
  let light = $state(false);
  onMount(() => {
    light = document.documentElement.dataset.theme === 'light';
  });
  function toggle() {
    light = !light;
    document.documentElement.dataset.theme = light ? 'light' : 'dark';
    try {
      localStorage.setItem('vut-theme', light ? 'light' : 'dark');
    } catch {
      /* Theme still works when storage is unavailable. */
    }
  }
</script>

<button
  class="icon-button"
  onclick={toggle}
  aria-label={light ? 'Switch to dark theme' : 'Switch to light theme'}
>
  {#if light}<Moon size={18} />{:else}<Sun size={18} />{/if}
</button>
