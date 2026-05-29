# Mom Language Overview

**Mom** is a modern, safe, fast, self-hosted systems programming language that compiles to native binaries via C. It combines the readability of Python, the performance of C, the memory safety of Rust, and the concurrency model of Erlang — while staying small enough to understand in a weekend.

```mom
fn fib(n: Int) -> Int:
    if n <= 1: n
    else: fib(n - 1) + fib(n - 2)

fn main():
    for i in 0..10:
        print(fib(i))
```

---

## Core Identity

| Property | Value |
|---|---|
| Paradigm | Multi-paradigm: imperative, functional, concurrent |
| Typing | Static, strong, inferred |
| Memory | Ownership + borrows + regions (no GC by default) |
| Concurrency | Async/await + actors + channels |
| Native output | Binary via C backend (LLVM planned) |
| Self-hosting | Yes — the compiler is written in Mom |
| Runtime | Minimal C runtime (`compiler/runtime.c`), no VM |

---

## Design Goals

1. **Simple syntax** — Python-style indentation; no header files, no build macros, no angle-bracket soup.
2. **Safe by default** — no null, no use-after-free, no data races in safe code.
3. **Fast** — C-level throughput; the compiler itself boots in milliseconds.
4. **Explicit** — mutability, ownership, and concurrency are always visible at the call site.
5. **Self-hosting** — the compiler, standard library, and tooling are written in Mom.
6. **Ergonomic** — one way to do most things; comprehensive built-in tooling.

---

## What Mom Is Not

- Not a scripting language (no dynamic dispatch or eval)
- Not garbage-collected (no GC pauses unless you opt into `gc { }` regions)
- Not a Rust replacement (simpler ownership model, different trade-offs)
- Not a managed runtime language (no JVM, no .NET, no Node)

---

## Hello, World

```mom
fn main():
    print("Hello, world!")
```

Run it:

```bash
mom run hello.mom
```

Or compile to a native binary:

```bash
mom build hello.mom -o hello
./hello
```

---

## Repository Layout

```
src/          Stage-0 compiler (Rust) — lexer, parser, type checker,
              borrow checker, interpreter, C-codegen, LSP, CLI
compiler/     Stage-1 compiler (Mom) — the compiler written in itself
std/          Standard library (.mom files)
tests/        Integration test suites
examples/     Sample programs
docs/         Language specification and design documents
```

---

## Versioning

Mom follows Semantic Versioning. Pre-1.0 releases may break the API between minor versions with explicit migration notes in the changelog.

| Version | Status |
|---|---|
| 0.1.0 | Phase 1 native backend |
| 0.2.0 | Python-style syntax + Windows ARM |
| **0.3.0** | **Native structs, strings, enums, cache integrity** |
| 1.0.0 | Full self-host + LLVM backend (planned) |
