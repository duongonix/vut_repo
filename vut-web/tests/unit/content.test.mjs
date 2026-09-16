import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFile, readdir } from 'node:fs/promises';
import { docPages } from '../../src/lib/config/docs.ts';

test('every navigation entry resolves to Markdown with matching frontmatter and real internal links', async () => {
  assert.equal(new Set(docPages.map((page) => page.slug)).size, docPages.length);
  const routes = new Set([
    ...docPages.map((page) => page.href),
    '/',
    '/playground/',
    '/community/',
    '/changelog/'
  ]);
  for (const page of docPages) {
    const source = await readFile(
      new URL(`../../src/routes/docs/${page.slug}/+page.md`, import.meta.url),
      'utf8'
    );
    assert.match(source, /^---\r?\n/);
    assert.ok(source.includes(`title: ${page.title}`), page.slug);
    assert.ok(source.includes(`section: ${page.section}`), page.slug);
    assert.match(source, /\ndescription: .+/);
    for (const [, href] of source.matchAll(/\]\((\/[^)#]*)(?:#[^)]*)?\)/g)) {
      assert.ok(routes.has(href), `${page.slug}: broken link ${href}`);
    }
  }
  const files = await readdir(new URL('../../src/routes/docs/', import.meta.url), {
    recursive: true
  });
  assert.equal(
    files.filter((file) => file.endsWith('+page.md')).length,
    docPages.length,
    'Orphan Markdown page'
  );
});
