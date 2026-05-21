# Contributing to mom

Welcome! mom is an open, multi-phase systems language project. Contributors
should be comfortable with Rust (the stage-0 toolchain), C (the stage-1
codegen target), and a willingness to read `.mom` source. This document
covers the day-to-day flow; deeper design context lives in `docs/`.

## Quick start

```sh
git clone https://github.com/<you>/mom
cd mom
cargo build
cargo test                       # 100+ acceptance tests, none flaky
./target/debug/mom run examples/hello.mom
```

## Where things live

- `src/`              — Rust stage-0 toolchain (lexer, parser, type
                        checker, borrow checker, interpreter, codegen,
                        every `mom <subcommand>`).
- `runtime/`          — runtime support for native builds.
- `std/`              — stage-0 standard library (each `.mom` file is a
                        runnable spec for one `std::*` module).
- `examples/`         — small `.mom` programs that double as docs.
- `tests/`            — Rust-side acceptance suites: `language.rs`,
                        `memory.rs`, `concurrency.rs`, `native_build.rs`,
                        `selfhost.rs`, `tooling.rs`, `stdlib.rs`.
- `docs/`             — design specs, plan, roadmap, risks.
- `rfcs/`             — request-for-comments for non-trivial proposals.

## The acceptance bar

Every PR must keep `cargo test` green. Phase-gated features carry their
own suites:

| Suite                  | What it locks in                                   |
|------------------------|----------------------------------------------------|
| `tests/language.rs`    | parser + typechecker + interpreter contract        |
| `tests/memory.rs`      | borrow checker correctness                         |
| `tests/concurrency.rs` | channels, cancel tokens, actor sugar               |
| `tests/native_build.rs`| codegen-to-C path, cache hits                      |
| `tests/selfhost.rs`    | stage-1 fixed-point parity                         |
| `tests/tooling.rs`     | fmt / lint / doc / test / pkg / lsp / bench / prof / dbg |
| `tests/stdlib.rs`      | one oracle per `std/*.mom` module                  |

If a regression isn't caught by an existing test, the fix PR adds the
missing test in the same commit.

## Style + tooling

- `mom fmt src/main.mom --check` is a CI gate. Use `mom fmt <file>` to
  fix locally.
- `mom lint <file.mom>` reads `[lints]` from `mom.toml` for per-crate
  severity overrides. Don't suppress lints in source — fix or escalate
  with the reviewer.
- Rust code follows `cargo fmt` defaults.

## Commit + PR conventions

- One logical change per commit; commit messages explain **why**, not
  what.
- Branch names: `phase-<N>/<short-topic>` (e.g. `phase-6/std-crypto`).
- PR description must list:
  1. The phase + sub-phase the change belongs to.
  2. The acceptance test added or updated.
  3. The user-visible delta (if any) in CLI, syntax, or stdlib API.

## Proposing language changes

Anything that changes the surface syntax, type-system semantics, or
package layout goes through the RFC process — see `rfcs/0000-template.md`.
Tooling, internal refactors, and stdlib additions ship as regular PRs.

## Security

Vulnerability reports go to the address in `SECURITY.md`. Do **not**
file public issues for security bugs.
