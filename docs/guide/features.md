# Mom Language Features

A complete feature matrix of what Mom supports today and what is planned.

## Current Feature Status

| Feature | Interpreter | Native Build | Notes |
|---|---|---|---|
| Functions + recursion | ✅ | ✅ | |
| `let` / `let mut` bindings | ✅ | ✅ | |
| `Int`, `Bool`, `Float`, `String` | ✅ | ✅ | |
| Lists `[T]`, indexing | ✅ | ❌ | native: planned Phase 3.1 |
| `if / else` expressions | ✅ | ✅ | |
| `while`, `for i in lo..hi` | ✅ | ✅ | |
| `for x in list` | ✅ | ❌ | native: planned Phase 3.1 |
| Structs + `impl` blocks | ✅ | ✅ | |
| Struct field assignment | ✅ | ✅ | |
| Enums + `match` | ✅ | ✅ | |
| Nested sub-patterns (`Wrap(A(n))`) | ✅ | ✅ | |
| Literal sub-patterns (`Val(0)`) | ✅ | ✅ | |
| `Option[T]` / `Result[T,E]` + `?` | ✅ | ❌ | native: generic support planned |
| Traits + `impl Trait for Type` | ✅ | ❌ | native: Phase 5 |
| Generics (dynamic dispatch) | ✅ | ❌ | native: monomorphization Phase 5 |
| Channels | ✅ | ❌ | native: Phase 3.1 |
| Actors | ✅ | ❌ | native: Phase 3.2 |
| `async fn` / `await` | ✅ | ❌ | native: Phase 3.1 |
| Modules + imports | ✅ | ❌ | native: Phase 4 |
| Pipeline operator `\|>` | ✅ | ✅ | |
| Lambdas | ✅ | ❌ | native: Phase 4 |
| `block:` scoped expression | ✅ | ✅ | |
| Borrow checker | ✅ | ✅ | phase-2 lexical model |
| `&T` / `&mut T` references | ✅ | ❌ | native: Phase 2.1 |
| Regions | ✅ | ❌ | native: Phase 2.1 |
| FFI — `extern c` | ✅ | ❌ | native: Phase 4 |
| Formatter (`mom fmt`) | ✅ | — | |
| Linter (`mom lint`) | ✅ | — | |
| LSP server (`mom lsp`) | ✅ | — | |
| Package manager (`mom pkg`) | ✅ | — | |
| Doc generator (`mom doc`) | ✅ | — | |
| Test runner (`mom test`) | ✅ | — | |
| Benchmarks (`mom bench`) | ✅ | — | |
| Self-hosting compiler | ✅ | ✅ | fixed-point verified |
| Multi-threaded async runtime | 🔜 | 🔜 | Phase 3.1 |
| LLVM backend | 🔜 | 🔜 | Phase 5 |
| Full generics monomorphization | 🔜 | 🔜 | Phase 5 |

---

## Memory Safety Features

- **No null pointers** — absence is `Option[T]`, never a null reference
- **No use-after-free** — the borrow checker enforces this at compile time
- **No data races** — the compiler prevents `&mut` aliasing across threads
- **No buffer overflows** — all indexing is bounds-checked; elided when provable
- **No double-free** — each value has exactly one owner

## Concurrency Features

- **Async/await** — cooperative multitasking on a work-stealing executor
- **Channels** — bounded and unbounded typed message queues
- **Actors** — isolated state machines with typed mailboxes
- **Supervision trees** — fault-tolerant restart policies (Phase 3.2)
- **`spawn`** — lightweight task creation
- **`Cancel`** — cooperative cancellation tokens

## Type System Features

- **Full type inference** for local bindings
- **Algebraic data types** (enums as sum types)
- **Structural pattern matching** with exhaustiveness checking
- **Generics** with type-parameter bounds (`T: Ord + Clone`)
- **Trait-based polymorphism** (no inheritance)
- **`comptime`** evaluation for compile-time constants
- **`Never`** type for diverging functions
- **Type aliases** (`type Bytes = [Byte]`)

## Developer Tooling

- **`mom run`** — interpret a file instantly (no compile step)
- **`mom build`** — compile to native binary
- **`mom check`** — type-check + borrow-check without building
- **`mom fmt`** — format source in place
- **`mom lint`** — lint with configurable rules
- **`mom doc`** — emit Markdown API docs
- **`mom test`** — discover and run `#[test]` functions
- **`mom bench`** — run `#[bench]` benchmarks
- **`mom lsp`** — Language Server Protocol on stdio (IDE integration)
- **`mom dbg`** — DAP debugger on stdio
- **`mom pkg`** — package manager (`list`, `add`, `remove`, `audit`)
- **`mom new`** / **`mom init`** — project scaffolding
