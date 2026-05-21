# Maintainers

This file is the source of truth for who can merge to `main`, sign
releases, and respond to the addresses in `SECURITY.md` and
`CODE_OF_CONDUCT.md`.

| Area                                     | Maintainer        | GitHub handle    |
|------------------------------------------|-------------------|------------------|
| Compiler front end (lexer / parser / AST)| `<TBD>`           | `@<handle>`      |
| Type system & borrow checker             | `<TBD>`           | `@<handle>`      |
| Codegen & runtime                        | `<TBD>`           | `@<handle>`      |
| Phase 5 tooling (`fmt`/`lint`/`lsp`/`bench`/`prof`/`dbg`) | `<TBD>` | `@<handle>` |
| Phase 6 standard library                 | `<TBD>`           | `@<handle>`      |
| Release engineering                      | `<TBD>`           | `@<handle>`      |

When you take on an area, send a PR adding your name here. When you
step away, send a PR removing yourself and naming a successor.

## Decision making

- Routine changes: any single maintainer can merge after the CI matrix
  is green.
- RFCs (see `rfcs/`): require explicit approval from at least two
  maintainers, one of whom must own the affected area.
- Security: any single maintainer can ship a patch release without
  RFC, but must post a retrospective within a week.
