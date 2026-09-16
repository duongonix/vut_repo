<script lang="ts">
  import { site } from '$lib/config/site';
  let { large = false } = $props<{ large?: boolean }>();
</script>

<a class:large class="brand" href="/" aria-label="Vut home">
  {#if site.logo}<img src={site.logo} alt="" width="32" height="32" />{/if}
  <span>vut<span class="dot">.</span></span>
</a>

<style>
  .brand {
    display: inline-flex;
    gap: 9px;
    align-items: center;
    font-size: 32px;
    font-weight: 740;
    line-height: 1;
    letter-spacing: -0.065em;
  }
  .dot {
    color: var(--primary);
  }
  .large {
    font-size: 40px;
  }
  img {
    object-fit: contain;
  }
</style>
