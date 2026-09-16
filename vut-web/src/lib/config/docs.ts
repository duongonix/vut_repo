export const docGroups = [
  {
    title: 'Getting started',
    pages: [
      ['Introduction', 'getting-started/introduction'],
      ['Installation', 'getting-started/installation'],
      ['Hello Vut', 'getting-started/hello-world'],
      ['Project structure', 'getting-started/project-structure']
    ]
  },
  {
    title: 'Language',
    pages: [
      ['Lexical structure', 'language/lexical-structure'],
      ['Variables', 'language/variables'],
      ['Constants', 'language/constants'],
      ['Types', 'language/types'],
      ['Operators', 'language/operators'],
      ['Functions', 'language/functions'],
      ['Control flow', 'language/control-flow'],
      ['Loops', 'language/loops'],
      ['Collections', 'language/collections'],
      ['Modules', 'language/modules'],
      ['Optional values', 'language/optional'],
      ['Result', 'language/result'],
      ['Interfaces', 'language/interfaces'],
      ['Error handling', 'language/error-handling']
    ]
  },
  {
    title: 'Advanced',
    pages: [
      ['Concurrency', 'advanced/concurrency'],
      ['Vutcon', 'advanced/vutcon'],
      ['Vutcom', 'advanced/vutcom'],
      ['FFI', 'advanced/ffi'],
      ['Native libraries', 'advanced/native-libraries']
    ]
  },
  {
    title: 'Tooling',
    pages: [
      ['Compiler', 'tooling/compiler'],
      ['Vut CLI', 'tooling/vut-cli'],
      ['VPM', 'tooling/vpm'],
      ['Formatter', 'tooling/formatter'],
      ['Linter', 'tooling/linter'],
      ['Language server', 'tooling/lsp']
    ]
  },
  {
    title: 'Standard library',
    pages: [
      ['Standard library', 'stdlib/overview'],
      ['Filesystem', 'stdlib/fs'],
      ['Paths', 'stdlib/path'],
      ['Operating system', 'stdlib/os'],
      ['Time', 'stdlib/time']
    ]
  },
  {
    title: 'Packages',
    pages: [
      ['Packages', 'packages/overview'],
      ['vpm.toml', 'packages/vpm-toml'],
      ['Dependencies', 'packages/dependencies'],
      ['Publishing', 'packages/publishing']
    ]
  },
  {
    title: 'Reference',
    pages: [
      ['Grammar', 'reference/grammar'],
      ['Keywords', 'reference/keywords'],
      ['Operator reference', 'reference/operators'],
      ['Built-in types', 'reference/builtin-types'],
      ['Built-in functions', 'reference/builtin-functions']
    ]
  }
];

export const docPages = docGroups.flatMap((group) =>
  group.pages.map(([title, slug]) => ({
    title,
    slug,
    section: group.title,
    href: `/docs/${slug}/`
  }))
);

export interface Heading {
  id: string;
  title: string;
  depth: number;
}
export interface Frontmatter {
  title?: string;
  description?: string;
  section?: string;
  pageType?: string;
  headings?: Heading[];
  order?: number;
}
