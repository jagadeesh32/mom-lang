# Mom Language History

## Origin

Mom was designed from the ground up as a modern alternative to C and C++ for systems programming, drawing lessons from two decades of language evolution. The central question was: *why do programmers still reach for C in 2025 when safer options exist?* The answer almost always came down to one of three things — build speed, binary size, or ABI compatibility. Mom addresses all three.

## Design Influences

| Influence | What Mom borrows |
|---|---|
| **Python** | Indentation-based syntax, readability, REPL-friendly feel |
| **Rust** | Ownership model, borrow checker, enums + pattern matching |
| **Go** | Simplicity, fast compilation, minimal runtime |
| **Zig** | No hidden allocations, comptime, build-system integration |
| **Erlang / Elixir** | Actors, supervision trees, "let it crash" fault model |
| **Haskell / ML** | Algebraic data types, exhaustive matching, type inference |
| **C** | ABI compatibility, native output, minimal runtime |

## Release Timeline

### v0.0.1 — Phase 0: Stage-0 Toolchain

The initial release established the stage-0 compiler written in Rust. It could lex, parse, type-check, and interpret Mom programs. No native codegen yet — programs ran in the tree-walking interpreter.

Key deliverables:
- Lexer and parser with Python-style indentation
- Type checker with inference for local bindings
- Tree-walking interpreter
- `mom run`, `mom tokens`, `mom ast`, `mom check` commands

### v0.1.0 — Phase 1: Native Backend

Introduced the C-based native code generator. Mom programs could now be compiled to native binaries via a C intermediate representation linked against `compiler/runtime.c`.

Key deliverables:
- C codegen for scalars (Int, Bool, Float)
- `mom build`, `mom build-run`, `mom emit-c` commands
- Build caching keyed on source content
- CI pipeline and cross-compilation targets

### v0.2.0 — Phase 2: Memory Safety + Python Syntax

The borrow checker shipped. Programs now have compile-time checked move semantics, borrow rules, and mutability enforcement. Syntax switched to Python-style `colon + indent` blocks (brace style still accepted).

Key deliverables:
- Phase-2 borrow checker (`src/borrow.rs`)
- Python-style indentation blocks
- `Option[T]`, `Result[T, E]` prelude types
- Channels, actors, async/await (interpreter)
- Windows ARM64 support
- Stage-1 self-hosted compiler bootstrap pipeline

### v0.3.0 — Phase 3: Native Structs, Strings, Enums

The native C backend gained the full data model. Structs, strings, enums, pattern matching (including nested sub-patterns), and field assignment all compile to native code. The build cache was hardened against stale-binary bugs.

Key deliverables:
- Strings (`const char*`) in native codegen with C re-escaping
- Structs → C `typedef struct` with literals, field access, field assignment
- Enums → C tagged unions with payload variants, nested patterns
- Recursive pattern matcher (`Wrap(A(n))`, `Val(0)` in native code)
- Build cache keyed on compiler binary identity (eliminates stale binaries)
- Borrow-checker fix: list elements and field assignments are reads, not moves
- Match-arm assignment bodies (`Some(n) => count = count + n`)
- `block:` scoped expression

## Roadmap Ahead

| Phase | Goal |
|---|---|
| Phase 3.1 | Work-stealing async executor, real timers |
| Phase 3.2 | Supervision trees, broadcast/oneshot channels |
| Phase 4 | Full self-host: stage-1 compiler compiles itself |
| Phase 5 | LLVM backend, generic monomorphization |
| Phase 6 | Full standard library, package registry |
| 1.0 | Stable API, edition system |
