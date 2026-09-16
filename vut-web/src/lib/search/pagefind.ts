export interface SearchResult {
  url: string;
  excerpt: string;
  meta: { title?: string; section?: string };
  title?: string;
  sub_results?: { url: string; title: string; excerpt: string }[];
}
interface Pagefind {
  search(query: string): Promise<{ results: { data(): Promise<SearchResult> }[] }>;
}
let engine: Promise<Pagefind> | undefined;

export async function searchDocs(query: string) {
  const modulePath = '/pagefind/pagefind.js';
  engine ??= import(/* @vite-ignore */ modulePath).catch((error) => {
    engine = undefined;
    throw error;
  });
  const pagefind = await engine;
  const response = await pagefind.search(query);
  const pages = await Promise.all(response.results.slice(0, 8).map((result) => result.data()));
  return pages
    .flatMap((page) =>
      page.sub_results?.length
        ? page.sub_results.slice(0, 3).map((section) => ({ ...page, ...section }))
        : [{ ...page, title: page.meta.title }]
    )
    .slice(0, 12);
}
