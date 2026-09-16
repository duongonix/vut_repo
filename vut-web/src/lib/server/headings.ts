import type { Root, Element, RootContent } from 'hast';
import type { VFile } from 'vfile';
import type { Heading } from '../config/docs.ts';
import type { Plugin } from 'unified';
import rehypeSlug from 'rehype-slug';
import rehypeAutolinkHeadings from 'rehype-autolink-headings';

export const docsHeadings: Plugin = function () {
  const slug = rehypeSlug();
  const collect = collectHeadings();
  const links = rehypeAutolinkHeadings({
    behavior: 'append',
    properties: {
      className: ['heading-anchor'],
      ariaLabel: 'Copy link to section',
      title: 'Copy link to section'
    },
    content: { type: 'text', value: '#' }
  });
  return (tree, file) => {
    slug(tree as Root);
    collect(tree as Root, file);
    links(tree as Root);
  };
};

function textOf(node: RootContent): string {
  if (node.type === 'text') return node.value;
  return 'children' in node ? node.children.map(textOf).join('') : '';
}

// Enrich SveltePress's frontmatter with the IDs assigned by rehype-slug.
export function collectHeadings() {
  return (tree: Root, file: VFile) => {
    const headings: Heading[] = [];
    function walk(nodes: RootContent[]) {
      for (const node of nodes) {
        if (node.type !== 'element') continue;
        const element = node as Element;
        if (/^h[23]$/.test(element.tagName)) {
          headings.push({
            id: String(element.properties.id),
            title: textOf(element),
            depth: Number(element.tagName[1])
          });
        }
        walk(element.children);
      }
    }
    walk(tree.children);
    file.data.headings = headings;
  };
}
