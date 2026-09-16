<script lang="ts">
  import { Zap, Box, Layers, Users } from '@lucide/svelte';
  const features = [
    {
      icon: Zap,
      title: 'Fast by design',
      text: 'Built with native performance in mind, from the ground up.',
      href: '/docs/tooling/compiler/'
    },
    {
      icon: Box,
      title: 'Simple & clean',
      text: 'A concise and expressive syntax that just makes sense.',
      href: '/docs/language/variables/'
    },
    {
      icon: Layers,
      title: 'Modern tooling',
      text: 'Everything you need for a smooth development experience.',
      href: '/docs/tooling/vpm/'
    },
    {
      icon: Users,
      title: 'Open and community-driven',
      text: 'An open-source ecosystem for a brighter future.',
      href: '/community/'
    }
  ];
</script>

<section class="features container" aria-label="Why build with Vut">
  {#each features as feature}<a class="feature" href={feature.href}
      ><feature.icon size={32} strokeWidth={1.4} />
      <h2>{feature.title}</h2>
      <p>{feature.text}</p></a
    >{/each}
</section>

<style>
  .features {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 18px;
  }
  .feature {
    border: 1px solid var(--border);
    border-radius: 14px;
    background: linear-gradient(
      135deg,
      color-mix(in srgb, var(--surface) 80%, transparent),
      color-mix(in srgb, var(--secondary) 75%, transparent)
    );
    padding: 30px 27px 26px;
    transition:
      transform 0.18s,
      border-color 0.18s;
  }
  .feature:hover {
    border-color: var(--border-hover);
    transform: translateY(-2px);
  }
  .feature :global(svg) {
    color: var(--primary);
    filter: drop-shadow(0 0 8px #7868ff44);
    margin-bottom: 19px;
  }
  h2 {
    font-size: 16px;
    font-weight: 550;
    letter-spacing: -0.025em;
    margin-bottom: 9px;
  }
  p {
    font-size: 13px;
    line-height: 1.6;
    margin-bottom: 0;
  }
  @media (max-width: 1100px) {
    .feature {
      padding: 24px 20px;
    }
    h2 {
      font-size: 14px;
    }
  }
  @media (max-width: 800px) {
    .features {
      grid-template-columns: 1fr 1fr;
    }
  }
  @media (max-width: 480px) {
    .features {
      grid-template-columns: 1fr;
      gap: 12px;
    }
    .feature {
      padding: 22px;
    }
    .feature :global(svg) {
      float: left;
      margin: 4px 19px 22px 0;
    }
    h2 {
      font-size: 16px;
    }
  }
</style>
