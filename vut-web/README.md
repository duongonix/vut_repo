# Vut web

The Vut language website and documentation, built with SvelteKit, SveltePress,
Svelte 5, TypeScript, Shiki, and Pagefind. All application code and generated
output live inside this directory. No compiler or extension changes are needed.

## Run locally

Use Node.js 24 and the existing npm lockfile:

```sh
npm ci
npm run dev
```

The asset task runs automatically before development and production builds.
It optimizes the existing `hero-source.png` into `static/images/hero.webp` and
creates the 1200×630 Open Graph image. It does not generate or redesign a logo.

## Production and search

```sh
npm run build
npm run preview
```

Deploy the contents of `build/` to a static host. Serve directory `index.html`
files and configure missing URLs to use `404.html` **with HTTP status 404**.
The site assumes deployment at the origin root, not a path prefix.

Pagefind builds its local search index from prerendered documentation. Use the
production preview to test search; the development server does not create a
fresh search index. Search failures expose a usable documentation fallback.
Search loads on demand and sends no queries to an external service.

## Public configuration

Copy `.env.example` to `.env` and set real values before building. SvelteKit's
public environment variables are compiled into the static site; rebuild after
changing them. Never put secrets in a `PUBLIC_` variable.

| Variable               | Purpose                                                                                                          |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `PUBLIC_SITE_URL`      | Actual public origin, such as your HTTPS domain, without a path. Enables absolute canonical and Open Graph URLs. |
| `PUBLIC_GITHUB_URL`    | Official project repository. Empty hides the external link.                                                      |
| `PUBLIC_COMMUNITY_URL` | Official community destination. Empty keeps the local community page.                                            |
| `PUBLIC_EDIT_BASE_URL` | GitHub edit URL ending in `/vut-web/src/routes`, without a trailing slash.                                       |
| `PUBLIC_LOGO_URL`      | Official logo URL, or a local asset path from `static/`.                                                         |

No official logo asset, production domain, public installer, or verified release
history was present in this workspace. The site therefore uses a text wordmark,
omits unconfigured canonical URLs, and does not invent external links or release
statistics. Configure these values before public launch. Do not use the design
mockup's community statistics as real metrics.

## Documentation authoring

Pages live in `src/routes/docs/<section>/<slug>/+page.md`. Required frontmatter:

```yaml
---
title: Page title
description: A concise, accurate description.
section: Language
---
```

Register the page in `src/lib/config/docs.ts`; this configuration controls the
sidebar and previous/next order. `order` is optional content metadata and does
not override that configuration. The content test catches missing and orphaned
pages. Rehype assigns heading IDs, collects the TOC, and appends deep links.

Use fenced `vut` blocks for Vut examples. An optional `title="main.vut"` fence
metadata field supplies a filename. Unknown languages fall back to escaped plain
text. Shiki runs on the server/build side, not in the browser bundle.

Reusable components are available for callouts, API signatures, and content tabs.
Keep their text/data in Markdown. For tabbed content, pass a data array declared
in the Markdown script to `ContentTabs`; do not embed Svelte snippet blocks
directly in Markdown. `Tabs` supplies keyboard behavior and unique ARIA IDs.

The source of truth is `../specs/`, especially `../specs/vut-web/`. Documentation
distinguishes specified behavior from distributed-toolchain availability. The
stdlib release reference, publishing workflow, and optional narrowing details
remain explicitly marked where the source specifications do not establish a
verified release API.

## Tests

```sh
npm run check
npm run test:unit
npm run build
npx playwright install chromium
npm run test:e2e
```

Or run `npm test` after installing the browser. E2E starts and stops its own
production-preview server on port 4173. To use an already installed browser,
set `PLAYWRIGHT_CHANNEL=msedge` or another Playwright-supported channel in your
shell. Test outputs stay in `test-results/` and `playwright-report/`.

The suite checks content links, Vut grammar (including nested interpolation),
safe code escaping, desktop/mobile layout, search, keyboard interaction,
clipboard, themes, docs navigation, playground behavior, and axe accessibility.
Screenshots are saved for visual review. Automated accessibility checks do not
replace manual keyboard and screen-reader testing.

```sh
npm run format:check
```

## Deliberate runtime boundary

The playground edits, resets, copies, and downloads examples. Run remains
disabled with an explanation because no browser compiler/WASM runtime is
provided. Expected output in documentation is labeled as an example, never as
an executed result. Running the native compiler belongs outside this website.
