<script lang="ts">
  import { ArrowRight, ChevronRight } from '@lucide/svelte';
  import CodeWindow from '../code/CodeWindow.svelte';
  import { site } from '$lib/config/site';
  let { html }: { html: string } = $props();
</script>

<section class="hero">
  <div class="landscape" aria-hidden="true"></div>
  <div class="hero-inner container">
    <div class="copy">
      <a class="version" href="/changelog/"
        ><span>{site.versions[0].label}</span><span class="badge-copy"
          >A new era for simple and powerful programming</span
        ><ChevronRight size={13} /></a
      >
      <h1>Build a brighter<br /><span>tomorrow</span> with Vut.</h1>
      <p>
        Vut is a modern, fast, and expressive programming language designed for real-world
        development. Simple to learn, powerful to build, and made for everyone.
      </p>
      <div class="buttons">
        <a href="/docs/getting-started/installation/" class="button primary"
          >Get Started <ArrowRight size={16} /></a
        ><a href="/docs/getting-started/introduction/" class="button">Read the Docs</a>
      </div>
      <div class="notes">
        <span>Open source</span><b>·</b><span>Modern tooling</span><b>·</b><span
          >Built for the future</span
        >
      </div>
    </div>
    <div class="visual">
      <div class="hero-mark" aria-hidden="true">
        {#if site.logo}<img src={site.logo} alt="" />{:else}<span
            >vut<span class="period">.</span></span
          >{/if}
        <div class="tagline">SIMPLE<br />POWERFUL<br />YOURS</div>
      </div>
      <CodeWindow {html} run />
    </div>
  </div>
</section>

<style>
  .hero {
    position: relative;
    isolation: isolate;
    padding: 72px 0 65px;
    overflow: hidden;
  }
  .landscape {
    position: absolute;
    inset: -70px 0 -90px 20%;
    z-index: -1;
    background: url('/images/hero.webp') center 55% / cover no-repeat;
    opacity: 0.83;
    mask-image:
      linear-gradient(to right, transparent, #000 25%),
      linear-gradient(to bottom, transparent, #000 15%, #000 72%, transparent);
    mask-composite: intersect;
  }
  .hero-inner {
    display: grid;
    grid-template-columns: minmax(0, 1.06fr) minmax(0, 1fr);
    align-items: end;
    gap: 24px;
  }
  .copy {
    padding-bottom: 20px;
    min-width: 0;
  }
  .version {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    background: #171d2a80;
    border: 1px solid #ffffff08;
    border-radius: 20px;
    padding: 5px 10px 5px 5px;
    font-size: 10px;
    color: #a5adbb;
    margin-bottom: 27px;
  }
  .version > span:first-child {
    color: #f5f7fb;
    border-radius: 14px;
    padding: 3px 7px;
    background: #273145;
  }
  h1 {
    font-size: clamp(39px, 4.3vw, 62px);
    letter-spacing: -0.048em;
    font-weight: 650;
    line-height: 1.08;
    margin-bottom: 24px;
    white-space: nowrap;
  }
  h1 > span {
    background: linear-gradient(110deg, #6292ff, #9463fa);
    color: transparent;
    background-clip: text;
  }
  p {
    max-width: 520px;
    font-size: 17px;
    line-height: 1.7;
    margin-bottom: 29px;
  }
  .buttons {
    display: flex;
    gap: 16px;
  }
  .notes {
    display: flex;
    gap: 12px;
    margin-top: 24px;
    font-size: 11px;
    color: var(--subtle);
  }
  .notes b {
    font-weight: 400;
  }
  .visual {
    padding-left: 10px;
    min-width: 0;
  }
  .hero-mark {
    display: flex;
    align-items: center;
    justify-content: space-around;
    height: 245px;
    margin-bottom: 7px;
  }
  .hero-mark > span {
    font-size: 140px;
    font-weight: 760;
    letter-spacing: -0.09em;
    line-height: 1;
    color: #8190ff;
    text-shadow: 0 4px 50px #555bff55;
    transform: rotate(-8deg);
  }
  .period {
    color: #aa7bff;
  }
  .hero-mark img {
    width: 165px;
    height: 165px;
    object-fit: contain;
  }
  .tagline {
    font-size: 12px;
    line-height: 2.4;
    letter-spacing: 0.55em;
    color: #bac4e4;
  }
  :global([data-theme='light']) .landscape {
    opacity: 0.14;
  }
  :global([data-theme='light']) .tagline {
    color: #4f5671;
  }
  :global([data-theme='light']) .version {
    background: var(--surface);
    border-color: var(--border);
    color: var(--muted);
  }
  @media (min-width: 1500px) {
    .hero {
      padding-top: 92px;
      padding-bottom: 80px;
    }
  }
  @media (max-width: 1050px) {
    h1 {
      font-size: 43px;
    }
    p {
      font-size: 15px;
    }
    .hero-mark > span {
      font-size: 110px;
    }
    .hero-mark {
      height: 200px;
    }
    .badge-copy {
      max-width: 280px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
  }
  @media (max-width: 800px) {
    .hero {
      padding-top: 45px;
    }
    .hero-inner {
      grid-template-columns: 1fr;
      gap: 25px;
    }
    h1 {
      font-size: clamp(38px, 7vw, 56px);
    }
    .copy {
      padding-bottom: 0;
    }
    p {
      max-width: 590px;
      font-size: 16px;
    }
    .hero-mark {
      display: none;
    }
    .visual {
      padding: 0;
    }
    .landscape {
      inset: 0 -180px 0 0;
      opacity: 0.45;
    }
  }
  @media (max-width: 430px) {
    h1 {
      font-size: clamp(29px, 8.3vw, 36px);
    }
    .version {
      max-width: 100%;
    }
    .badge-copy {
      min-width: 0;
      max-width: 240px;
      font-size: 9px;
    }
    .notes {
      flex-wrap: wrap;
      gap: 8px;
      font-size: 9px;
    }
    .buttons {
      flex-wrap: wrap;
      gap: 10px;
    }
    .button {
      padding-inline: 18px;
      font-size: 12px;
    }
  }
</style>
