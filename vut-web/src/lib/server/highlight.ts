import { createHighlighter } from 'shiki';
import { vutGrammar } from './vut-grammar.ts';

const highlighter = createHighlighter({
  themes: ['github-dark', 'github-light'],
  langs: [vutGrammar, 'bash', 'powershell', 'toml', 'text']
});
const escape = (value: string) =>
  value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');

export async function highlight(code: string, language = 'vut') {
  const engine = await highlighter;
  const lang = engine.getLoadedLanguages().includes(language) ? language : 'text';
  return engine.codeToHtml(code, {
    lang,
    themes: { dark: 'github-dark', light: 'github-light' },
    defaultColor: 'dark'
  });
}

export async function markdownHighlighter(code: string, language: string, meta?: string) {
  const title = meta?.match(/title="([^"]+)"/)?.[1] || language || 'text';
  const label = title === language || !language ? title : `${title} · ${language}`;
  const html = `<div class="code-block"><div class="code-toolbar"><span>${escape(label)}</span><button type="button" data-copy-code aria-label="Copy code">Copy</button></div>${await highlight(code, language)}</div>`;
  return `{@html ${JSON.stringify(html)}}`;
}
