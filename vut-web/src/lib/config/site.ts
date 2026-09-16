import * as publicEnvironment from '$env/static/public';

const env = publicEnvironment as Record<string, string | undefined>;

export const site = {
  name: 'Vut',
  description:
    'A modern, fast, and expressive programming language. Simple to learn, powerful to build.',
  // Supply the public origin and official links before deployment.
  origin: env.PUBLIC_SITE_URL?.replace(/\/$/, '') || '',
  github: env.PUBLIC_GITHUB_URL || '',
  community: env.PUBLIC_COMMUNITY_URL || '',
  editBase: env.PUBLIC_EDIT_BASE_URL?.replace(/\/$/, '') || '',
  logo: env.PUBLIC_LOGO_URL || '',
  versions: [{ label: 'v0.1', href: '/docs/getting-started/introduction/' }]
};

export const navigation = [
  { label: 'Docs', href: '/docs/getting-started/introduction/' },
  { label: 'Guide', href: '/docs/getting-started/installation/' },
  { label: 'Reference', href: '/docs/reference/keywords/' },
  { label: 'Playground', href: '/playground/' },
  { label: 'Community', href: site.community || '/community/' }
];
