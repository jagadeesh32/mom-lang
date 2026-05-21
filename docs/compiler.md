# mom — Compiler Architecture

The mom compiler is a **multi-stage native compiler** with a small,
auditable core and pluggable backends. This document describes the
intended architecture of the native compiler. The current
implementation in `/src/` is the bootstrap front end + interpreter that
prepares for it.

```
┌────────────┐  ┌────────────┐  ┌─────────────┐  ┌────────────┐  ┌──────────────┐
│  Source    │→ │  Frontend  │→ │  Middle-end │→ │  Backend   │→ │  Linker      │
│  *.mom     │  │  parse,    │  │  HIR, MIR,  │  │  LLVM IR + │  │  native      │
│            │  │  resolve,  │  │  borrow,    │  │  custom    │  │  binary      │
│            │  │  typeck    │  │  monomorph. │  │  codegen   │  │              │
└────────────┘  └────────────┘  └─────────────┘  └────────────┘  └──────────────┘
```

---

## 1. Frontend

### 1.1 Lexer
- Single-pass character-by-character.
- Handles UTF-8, escape sequences, comments, raw strings.
- Numeric literals: integer (with `_`), float, hex/binary/octal
  prefixes (`0x`, `0b`, `0o`).
- Reports lexical errors with line/column spans (already implemented in
  the bootstrap toolchain).

### 1.2 Parser
- Recursive-descent for statements and items; Pratt for expressions.
- Produces a **lossless syntax tree** (CST) and an AST.
  - CST is what `mom fmt` and `mom lsp` consume.
  - AST is what later phases consume.
- Error recovery: a missing `)` does not poison the rest of the file.

### 1.3 Module resolution
- Walks `src/` to build the module tree.
- Resolves `import` / `use` to actual `Item` references.
- Cycle detection.

### 1.4 Macro expansion / comptime
- mom has **no text macros**.
- `comptime fn` runs at compile time over compile-time values, with the
  same syntax and type system as ordinary mom — no separate DSL.
- The comptime evaluator is the same interpreter the bootstrap toolchain
  ships, hardened against unbounded loops.

### 1.5 Name resolution
- Single pass over the AST.
- Resolves identifiers, paths, traits, methods.
- Produces a `DefId` table consumed by everything downstream.

### 1.6 Type checking + inference
- Bidirectional inference.
- Trait selection and method resolution.
- Borrow-related lifetimes inferred from signatures.
- Generic parameters bound but not yet specialised.

---

## 2. Middle-end

### 2.1 HIR — High-level IR
- Desugared AST: `for` → `Iterator`, `?` → match-on-Result, `await`
  → state machine, pipelines → call.
- Single statically-typed form. Generic parameters preserved.

### 2.2 Trait monomorphization
- For each generic instantiation reachable from the program roots,
  emit a concrete copy.
- Hash-consed so identical instantiations share code.

### 2.3 Borrow / region check
- Operates on HIR.
- Verifies the ownership / borrow / region rules described in
  [memory.md](memory.md).
- Produces a "drop schedule" — when each owned value is destructed.

### 2.4 Concurrency check
- Verifies that values crossing actor / thread boundaries are sendable.
- Verifies no `&mut` aliasing across concurrent operations.

### 2.5 MIR — Mid-level IR
- Three-address code, SSA, basic blocks.
- The optimization sweet spot:
  - inlining (small functions, generics, closures)
  - constant propagation, dead-code elimination
  - escape analysis → stack promotion
  - bounds-check elimination on monotonic indices
  - drop elaboration → explicit destructor calls
  - loop strength reduction

---

## 3. Backend

mom ships **two backends** behind a common interface:

### 3.1 LLVM backend (default)
- Lowers MIR to LLVM IR.
- Reuses LLVM's optimizer (`-O0 .. -O3`, LTO, ThinLTO).
- Yields top-tier code quality across every architecture LLVM supports.

### 3.2 Fast backend (debug / hot iter)
- Custom direct-to-machine code generator for `x86-64`, `aarch64`,
  `riscv64`, `wasm32`.
- Skips most optimization in exchange for **10×** faster codegen.
- Used by `mom build` (debug) and the LSP for hover-time IR display.
- Inspired by Cranelift and Zig's self-hosted backend.

The build system chooses the backend automatically:

| Profile             | Backend     |
|---------------------|-------------|
| `mom build`         | fast        |
| `mom build --release` | LLVM       |
| `mom test`          | fast        |
| `mom run`           | fast        |
| `mom bench`         | LLVM        |

## 4. Linker

- Wraps `lld` (default) but supports the system linker.
- Static linking by default.
- Generates symbol maps, GNU build IDs, debug info splits.

---

## 5. Incremental compilation

The compiler is built around a **query engine** (a la rust-analyzer /
salsa). Every derived fact (token stream, parse tree, type, MIR
function, …) is a memoized query keyed by its inputs. A file edit
invalidates only the queries that transitively depend on the edited
file.

Practical effect:

- Edit a comment → no rebuild.
- Edit a function body → re-typecheck and re-codegen only that function.
- Edit a type signature → re-typecheck dependents; codegen only
  changed monomorphizations.

## 6. Caching layers

- **Process-local cache** — query memoization.
- **Disk cache** — content-addressed by source hash + flags. Survives
  `mom clean` (it's safe to share between projects).
- **Distributed cache** — optional remote object cache shared by a
  team or CI fleet.

## 7. Output artifacts

| Artifact                  | Notes                          |
|---------------------------|--------------------------------|
| `*.mom.ir`                | textual MIR for debugging      |
| `*.ll` (LLVM IR)          | only with `--emit=llvm-ir`     |
| `*.o`                     | per-module object              |
| binary / staticlib / cdylib | final product                 |
| `mom.lock`                | dependency lockfile            |
| `*.mombuild.json`         | build metadata (reproducible)  |
| DWARF / PDB               | debug info, optionally split   |

## 8. Error reporting

- Rust-style snippets with caret underlines and explanations.
- Error codes (`E0123`) link to a docs page with a worked example.
- Suggestions inline when the compiler is confident (typo → did-you-mean).
- LSP shares the same diagnostic IDs.

## 9. Bootstrap status — May 2026

The repository's `src/` directory contains the **stage-0** front end
in Rust. It implements lexer, parser, AST, lenient type checker, and
a tree-walking interpreter. The remaining stages (HIR, MIR, borrow,
LLVM/native backends) ship in subsequent releases per the
[roadmap](roadmap.md).
