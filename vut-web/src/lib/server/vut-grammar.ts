import type { LanguageRegistration } from 'shiki';

// Lexical highlighting only: semantic validity remains the compiler's responsibility.
export const vutGrammar: LanguageRegistration = {
  name: 'vut',
  scopeName: 'source.vut',
  displayName: 'Vut',
  patterns: [{ include: '#comments' }, { include: '#strings' }, { include: '#expressions' }],
  repository: {
    comments: {
      patterns: [
        { name: 'comment.block.vut', begin: '^\\s*##\\s*$', end: '^\\s*##\\s*$' },
        { name: 'comment.line.documentation.vut', match: '###.*$' },
        { name: 'comment.line.number-sign.vut', match: '#.*$' }
      ]
    },
    strings: {
      name: 'string.quoted.double.vut',
      begin: '"',
      end: '"',
      patterns: [
        { name: 'constant.character.escape.vut', match: '\\\\.' },
        {
          name: 'meta.interpolation.vut',
          contentName: 'source.vut',
          begin: '\\$\\(',
          end: '\\)',
          beginCaptures: { 0: { name: 'punctuation.section.interpolation.begin.vut' } },
          endCaptures: { 0: { name: 'punctuation.section.interpolation.end.vut' } },
          patterns: [{ include: '#parentheses' }, { include: '$self' }]
        },
        { name: 'variable.other.interpolation.vut', match: '\\$[A-Za-z_][A-Za-z0-9_]*' }
      ]
    },
    parentheses: {
      begin: '\\(',
      end: '\\)',
      beginCaptures: { 0: { name: 'punctuation.section.group.begin.vut' } },
      endCaptures: { 0: { name: 'punctuation.section.group.end.vut' } },
      patterns: [{ include: '#parentheses' }, { include: '$self' }]
    },
    expressions: {
      patterns: [
        { name: 'storage.type.function.vut', match: '\\bfn\\b' },
        {
          match: '\\b(data|interface|enum|type)\\s+([A-Za-z_][A-Za-z0-9_]*)',
          captures: { 1: { name: 'storage.type.vut' }, 2: { name: 'entity.name.type.vut' } }
        },
        {
          name: 'keyword.control.vut',
          match:
            '\\b(async|await|static|vut|if|elif|else|match|for|in|break|continue|return|import|as|at|extern|opaque|unsafe)\\b'
        },
        { name: 'keyword.operator.logical.vut', match: '\\b(and|or|not)\\b' },
        { name: 'constant.language.vut', match: '\\b(true|false|null)\\b' },
        {
          name: 'storage.type.vut',
          match:
            '\\b(str|int|float|bool|bytes|unit|void|dyn|list|map|result|ptr|resource|future|channel|vutcon|usize|isize|[iu](8|16|32|64)|f(32|64))\\b'
        },
        { name: 'entity.other.attribute-name.vut', match: '@[A-Za-z_][A-Za-z0-9_]*' },
        { name: 'variable.language.vut', match: '\\bself\\b' },
        { name: 'constant.other.vut', match: '\\b[A-Z][A-Z0-9_]*\\b' },
        { name: 'entity.name.type.vut', match: '\\b[A-Z][A-Za-z0-9_]*\\b' },
        { name: 'entity.name.function.vut', match: '\\b[A-Za-z_][A-Za-z0-9_]*(?=\\s*\\()' },
        { name: 'constant.numeric.vut', match: '\\b[0-9]+(?:\\.[0-9]+)?\\b' },
        { name: 'keyword.operator.vut', match: '\\.\\.=|\\.\\.|->|=>|==|!=|<=|>=|[=+*/%<>?-]' },
        { name: 'punctuation.separator.vut', match: '[@()\\[\\],.:]' },
        { name: 'variable.other.vut', match: '\\b[A-Za-z_][A-Za-z0-9_]*\\b' }
      ]
    }
  }
};
