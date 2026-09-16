<script lang="ts">
  import Brand from '../ui/Brand.svelte';
  import { site } from '$lib/config/site';
  const groups = [
    {
      title: 'Documentation',
      links: [
        ['Get started', '/docs/getting-started/introduction/'],
        ['Language guide', '/docs/language/variables/'],
        ['Reference', '/docs/reference/keywords/'],
        ['Standard library', '/docs/stdlib/overview/']
      ]
    },
    {
      title: 'Ecosystem',
      links: [
        ['Vut compiler', '/docs/tooling/compiler/'],
        ['VPM', '/docs/tooling/vpm/'],
        ['Community', site.community || '/community/'],
        ...(site.github ? [['GitHub', site.github]] : [])
      ]
    },
    {
      title: 'Resources',
      links: [
        ['Playground', '/playground/'],
        ['Changelog', '/changelog/'],
        ['Installation', '/docs/getting-started/installation/']
      ]
    }
  ];
</script>

<footer class="container">
  <div class="footer-main">
    <div class="identity">
      <Brand large />
      <p>Simple. Powerful. Yours.</p>
    </div>
    {#each groups as group}<div class="link-group">
        <h2>{group.title}</h2>
        {#each group.links as [label, href]}<a {href}>{label}</a>{/each}
      </div>{/each}
  </div>
  <div class="footer-bottom">
    <span>© {new Date().getFullYear()} Vut contributors</span><span
      >Open source. Built for the future.</span
    >
  </div>
</footer>

<style>
  footer {
    border-top: 1px solid var(--border);
    margin-top: 30px;
    padding-top: 48px;
  }
  .footer-main {
    display: grid;
    grid-template-columns: 1.6fr 1fr 1fr 1fr;
    gap: 30px;
    padding-bottom: 46px;
  }
  .identity p {
    font-size: 13px;
    margin-top: 18px;
  }
  h2 {
    font-size: 12px;
    font-weight: 550;
    letter-spacing: 0;
    margin-bottom: 18px;
  }
  .link-group {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .link-group a {
    font-size: 12px;
    color: var(--muted);
  }
  a:hover {
    color: var(--primary);
  }
  .footer-bottom {
    border-top: 1px solid var(--border);
    padding-block: 23px;
    display: flex;
    justify-content: space-between;
    color: var(--subtle);
    font-size: 11px;
  }
  @media (max-width: 640px) {
    .footer-main {
      grid-template-columns: 1fr 1fr;
      gap: 34px;
    }
    .footer-bottom {
      flex-direction: column;
      gap: 10px;
    }
  }
</style>
