import { highlight } from '$lib/server/highlight';
import { hello, examples } from '$lib/content/examples';

export async function load() {
  return {
    heroCode: await highlight(hello),
    examples: await Promise.all(
      examples.map(async (example) => ({ ...example, html: await highlight(example.code) }))
    )
  };
}
