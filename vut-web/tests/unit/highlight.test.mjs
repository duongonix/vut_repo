import assert from 'node:assert/strict';
import { after, test } from 'node:test';
import { createHighlighter } from 'shiki';
import { vutGrammar } from '../../src/lib/server/vut-grammar.ts';
import { highlight, markdownHighlighter } from '../../src/lib/server/highlight.ts';

const engine = await createHighlighter({ themes: ['github-dark'], langs: [vutGrammar] });
after(() => engine.dispose());
const tokenize = (code) =>
  engine
    .codeToTokens(code, {
      lang: 'vut',
      theme: 'github-dark',
      includeExplanation: true
    })
    .tokens.flat();
const scopesFor = (tokens, text) =>
  tokens
    .filter((token) => token.content.includes(text))
    .flatMap((token) => token.explanation ?? [])
    .filter((part) => part.content.includes(text))
    .flatMap((part) => part.scopes.map((scope) => scope.scopeName));

test('Vut grammar covers control flow, functions, types, constants, and attributes', () => {
  const tokens = tokenize(
    'async fn calculate(value: u16) -> result[int, str]:\n  for i in 0..=10:\n    if i > 1 and true:\n      return ok(MAX_SIZE)\n@repr(C)\ndata Point:\n  x: f64'
  );
  for (const word of ['async', 'for', 'if', 'return']) {
    assert.ok(scopesFor(tokens, word).includes('keyword.control.vut'), word);
  }
  assert.ok(scopesFor(tokens, 'calculate').includes('entity.name.function.vut'));
  assert.ok(scopesFor(tokens, 'u16').includes('storage.type.vut'));
  assert.ok(scopesFor(tokens, 'MAX_SIZE').includes('constant.other.vut'));
  assert.ok(scopesFor(tokens, '@repr').includes('entity.other.attribute-name.vut'));
  assert.ok(scopesFor(tokens, 'Point').includes('entity.name.type.vut'));
});

test('nested interpolation returns to string scope after balanced calls and nested strings', () => {
  const code = 'out("answer $(square((2 + 3)) + size("x")) tail $name \\$")';
  const tokens = tokenize(code);
  assert.equal(tokens.map((token) => token.content).join(''), code);
  assert.ok(scopesFor(tokens, 'square').includes('entity.name.function.vut'));
  const tailScopes = scopesFor(tokens, 'tail');
  assert.ok(tailScopes.includes('string.quoted.double.vut'));
  assert.ok(!tailScopes.includes('meta.interpolation.vut'));
  assert.ok(scopesFor(tokens, '$name').includes('variable.other.interpolation.vut'));
  assert.ok(scopesFor(tokens, '\\$').includes('constant.character.escape.vut'));
});

test('multiline comments end before ordinary code', () => {
  const tokens = tokenize('##\nfn hidden()\n##\nfn visible():\n  out("ok")');
  assert.ok(scopesFor(tokens, 'hidden').includes('comment.block.vut'));
  assert.ok(scopesFor(tokens, 'visible').includes('entity.name.function.vut'));
});

test('current concurrency and ownership types highlight without historical keywords', () => {
  const tokens = tokenize(
    'static fn Scope.make() -> channel[int]:\n  count: usize = 1\n  handle: resource[Scope]\n  task: future[unit]\ncomposition children vutcom'
  );
  assert.ok(scopesFor(tokens, 'static').includes('keyword.control.vut'));
  for (const name of ['channel', 'usize', 'resource', 'future', 'unit']) {
    assert.ok(scopesFor(tokens, name).includes('storage.type.vut'), name);
  }
  for (const name of ['composition', 'children', 'vutcom']) {
    assert.ok(!scopesFor(tokens, name).includes('keyword.control.vut'), name);
    assert.ok(!scopesFor(tokens, name).includes('storage.type.vut'), name);
  }
});

test('unknown languages fall back to escaped text and filename metadata cannot inject markup', async () => {
  const fallback = await highlight('<script>alert(1)</script>', 'not-a-language');
  assert.match(fallback, /(?:&lt;|&#x3[Cc];)script(?:&gt;|>)/);
  assert.ok(!fallback.includes('<script>'));
  const html = await markdownHighlighter('fn main():', 'vut', 'title="<img src=x>"');
  assert.match(html, /&lt;img src=x&gt;/);
  assert.match(html, /data-copy-code/);
  assert.match(html, /--shiki-light/);
});
