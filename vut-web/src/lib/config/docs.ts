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
      ['Data and methods', 'language/data'],
      ['Enums and patterns', 'language/enums'],
      ['Generics', 'language/generics'],
      ['Functions as values', 'language/closures'],
      ['Control flow', 'language/control-flow'],
      ['Loops', 'language/loops'],
      ['Collections', 'language/collections'],
      ['Lists and arrays', 'language/lists-arrays'],
      ['Maps', 'language/maps'],
      ['Strings and bytes', 'language/strings-bytes'],
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
      ['Channels', 'advanced/channels'],
      ['Receiver functions', 'advanced/receivers'],
      ['Memory and ownership', 'advanced/memory'],
      ['Attributes', 'advanced/attributes'],
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
      ['Input and output', 'stdlib/io'],
      ['Filesystem', 'stdlib/fs'],
      ['Paths', 'stdlib/path'],
      ['Operating system', 'stdlib/os'],
      ['Time', 'stdlib/time'],
      ['Environment', 'stdlib/env'],
      ['Processes', 'stdlib/process'],
      ['HTTP', 'stdlib/http'],
      ['JSON', 'stdlib/json']
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
      ['Built-in functions', 'reference/builtin-functions'],
      ['Source compatibility', 'reference/source-compatibility']
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
