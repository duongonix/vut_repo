<script lang="ts">
  import {
    Zap,
    Box,
    BookOpen,
    Globe,
    ArrowRight,
    Terminal,
    Library,
    Network,
    Braces,
    Wrench
  } from '@lucide/svelte';
  const facts = [
    {
      icon: Zap,
      label: 'Fast by design',
      value: 'Native-focused',
      detail: 'Performance with purpose'
    },
    { icon: Box, label: 'Open source', value: 'Built in the open', detail: 'A shared foundation' },
    {
      icon: BookOpen,
      label: 'Documentation',
      value: 'Learn. Build. Explore.',
      detail: 'From first steps to reference'
    },
    {
      icon: Globe,
      label: 'Cross-platform',
      value: 'Made for everyone',
      detail: 'Windows · macOS · Linux'
    }
  ];
  const tools = [
    { icon: Terminal, name: 'Vut Compiler', href: '/docs/tooling/compiler/' },
    { icon: Box, name: 'VPM', href: '/docs/tooling/vpm/' },
    { icon: Library, name: 'Standard Library', href: '/docs/stdlib/overview/' },
    { icon: Network, name: 'Vutcon', href: '/docs/advanced/vutcon/' },
    { icon: Braces, name: 'Vutcom', href: '/docs/advanced/vutcom/' },
    { icon: Wrench, name: 'Developer Tooling', href: '/docs/tooling/vut-cli/' }
  ];
</script>

<section class="container">
  <div class="facts">
    {#each facts as fact}<div class="fact">
        <fact.icon size={24} strokeWidth={1.4} />
        <div>
          <p>{fact.label}</p>
          <strong>{fact.value}</strong><small>{fact.detail}</small>
        </div>
      </div>{/each}
  </div>
  <div class="ecosystem">
    <div class="orbital" aria-hidden="true">
      <div class="orbit one"></div>
      <div class="orbit two"></div>
      <div class="orbit three"></div>
      <div class="globe">
        <div class="latitude"></div>
        <div class="longitude"></div>
        <div class="longitude second"></div>
      </div>
      <i class="star a"></i><i class="star b"></i><i class="star c"></i>
    </div>
    <div>
      <p class="eyebrow">A brighter tomorrow</p>
      <h2 class="section-heading">More than a language.<br />A growing ecosystem.</h2>
      <p>
        From powerful tooling to an open community, Vut brings the pieces together — so your next
        idea has room to grow.
      </p>
      <div class="tool-links">
        {#each tools as tool}<a href={tool.href}
            ><tool.icon size={16} />{tool.name}<ArrowRight size={13} /></a
          >{/each}
      </div>
    </div>
  </div>
  <div class="final-cta">
    <p class="eyebrow">Your next idea starts here</p>
    <h2>Ready to build with Vut?</h2>
    <p>A little less ceremony. A lot more possibility.</p>
    <div>
      <a class="button primary" href="/docs/getting-started/installation/"
        >Get Started <ArrowRight size={16} /></a
      ><a class="button" href="/docs/getting-started/introduction/">Explore Documentation</a>
    </div>
  </div>
</section>

<style>
  .facts {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    border: 1px solid var(--border);
    border-radius: 13px;
    padding: 29px 18px;
    background: color-mix(in srgb, var(--surface) 50%, transparent);
  }
  .fact {
    display: flex;
    gap: 16px;
    padding: 0 22px;
    border-right: 1px solid var(--border);
  }
  .fact:last-child {
    border: 0;
  }
  .fact :global(svg) {
    color: var(--primary);
  }
  .fact p {
    font-size: 11px;
    margin-bottom: 10px;
    line-height: 1.3;
  }
  strong {
    font-size: 15px;
    font-weight: 550;
  }
  small {
    display: block;
    color: var(--subtle);
    font-size: 10px;
    margin-top: 10px;
  }
  .ecosystem {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 80px;
    align-items: center;
    padding-block: 85px;
  }
  .ecosystem p:not(.eyebrow) {
    font-size: 15px;
  }
  .tool-links {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px 25px;
    margin-top: 25px;
  }
  .tool-links a {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 12px;
    color: var(--muted);
    padding-block: 8px;
  }
  .tool-links a :global(svg:first-child) {
    color: var(--primary);
  }
  .tool-links a :global(svg:last-child) {
    margin-left: auto;
    opacity: 0.5;
  }
  .tool-links a:hover {
    color: var(--primary);
  }
  .orbital {
    position: relative;
    height: 360px;
    display: grid;
    place-items: center;
  }
  .globe {
    position: relative;
    width: 290px;
    height: 290px;
    border-radius: 50%;
    border: 1px solid #6681ff66;
    overflow: hidden;
    box-shadow:
      inset 12px 10px 40px #394bd455,
      0 0 75px #3442dc18;
    background: radial-gradient(circle at 28% 30%, #7389ff99 0.9px, transparent 1.3px) 0 0 / 6px 6px;
    mask-image: radial-gradient(circle at 32% 26%, #000 20%, #000a 50%, #0002 75%);
  }
  .latitude {
    position: absolute;
    inset: 24% -10%;
    border: 1px solid #6678f988;
    border-radius: 50%;
    transform: rotate(-24deg);
  }
  .longitude {
    position: absolute;
    inset: -6% 27%;
    border: 1px solid #6678f988;
    border-radius: 50%;
    transform: rotate(20deg);
  }
  .longitude.second {
    inset: -6% 12%;
  }
  .orbit {
    position: absolute;
    width: 440px;
    height: 160px;
    border: 1px solid #5868dd40;
    border-radius: 50%;
    transform: rotate(-30deg);
  }
  .orbit.two {
    transform: rotate(36deg);
    width: 395px;
    height: 210px;
  }
  .orbit.three {
    transform: rotate(-68deg);
    width: 380px;
    height: 225px;
  }
  .star {
    position: absolute;
    width: 4px;
    height: 4px;
    background: #bbc7ff;
    border-radius: 50%;
    box-shadow: 0 0 11px 4px #5669ff88;
  }
  .a {
    left: 20%;
    top: 30%;
  }
  .b {
    right: 17%;
    top: 52%;
  }
  .c {
    left: 52%;
    top: 4%;
  }
  .final-cta {
    text-align: center;
    padding: 60px 25px 70px;
    margin-bottom: 36px;
    border-top: 1px solid var(--border);
    background: radial-gradient(ellipse at top, #7868ff0c, transparent 70%);
  }
  .final-cta h2 {
    font-size: clamp(30px, 4vw, 42px);
    margin-bottom: 14px;
  }
  .final-cta p:not(.eyebrow) {
    font-size: 14px;
    margin-bottom: 28px;
  }
  .final-cta > div {
    display: flex;
    justify-content: center;
    gap: 14px;
  }
  @media (max-width: 1100px) {
    .facts {
      padding-inline: 0;
    }
    .fact {
      padding-inline: 16px;
      gap: 10px;
    }
    strong {
      font-size: 13px;
    }
    .ecosystem {
      gap: 40px;
    }
    .orbit {
      width: 360px;
    }
  }
  @media (max-width: 800px) {
    .facts {
      grid-template-columns: 1fr 1fr;
      gap: 28px 0;
    }
    .fact:nth-child(2) {
      border: 0;
    }
    .ecosystem {
      grid-template-columns: 1fr;
      padding-top: 55px;
      gap: 30px;
    }
    .orbital {
      height: 320px;
      overflow: hidden;
    }
  }
  @media (max-width: 480px) {
    .facts {
      grid-template-columns: 1fr;
    }
    .fact {
      border: 0;
      padding-left: 25px;
      gap: 20px;
    }
    .final-cta > div {
      flex-direction: column;
      align-items: center;
    }
    .tool-links {
      gap: 8px 16px;
    }
  }
</style>
