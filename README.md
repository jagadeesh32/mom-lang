# mom

**mom** is a modern, self-hosted systems programming language designed for
enterprise-scale software: operating systems, distributed systems, AI
infrastructure, networking stacks, and high-performance backend services.

Source files use the `.mom` extension. The compiler binary is `mom`.

mom combines:

| Influence  | Property                                                     |
|------------|--------------------------------------------------------------|
| Python     | readable, low-ceremony syntax                                 |
| Rust       | memory safety, sum types, traits, exhaustive pattern matching |
| Erlang/OTP | actors, message passing, supervised fault isolation           |
| Zig        | extremely fast incremental compilation, no hidden allocations |
| Go         | lightweight tasks, easy concurrency, simple build flow        |
| C / C++    | first-class FFI, predictable performance, no managed runtime  |

The language goal is **simple to learn, safe by default, fast at runtime,
fast to compile, and able to compile itself**.

---

## Status

This repository hosts the **Rust-based bootstrap toolchain** for mom — the
first stage in a self-hosting plan that ends with mom compiling itself
through a native LLVM and custom-codegen backend.

What works today:

- Lexer with line/block comments, string escapes, integer and float literals
- Recursive-descent + Pratt parser covering the full enterprise surface syntax
- AST for: functions (with generics, `pub`, `async`), structs, enums,
  constants, modules, imports, traits, `impl` blocks, `extern` blocks,
  lambdas, pattern matching, pipelines, lists, ranges, `?` propagation,
  `spawn`/`await`, for-in loops, method calls, field access, indexing
- Type checker for primitive types, function signatures, variants, lists,
  ranges, and lenient handling of generics/methods
- Tree-walk interpreter (the bootstrap runtime) for executable subsets
- **Phase 1 native backend**: `mom build` lowers the Int/Bool/Unit subset
  to portable C99, links via the system `cc`, and produces a real native
  executable. Content-addressed build cache skips `cc` on unchanged input.
- HIR + MIR data structure scaffolds in place for the LLVM backend
  (Phase 1.1).
- **Phase 2 memory model**: `&T`, `&mut T`, `region NAME { … }`, built-in
  `Box`/`Rc`/`Arc`, and a borrow checker that catches use-after-move,
  double-mut-borrow, shared+mut, and mutate-while-borrowed at compile time.
- **Phase 3 concurrency runtime** (interpreter level): built-in `Channel`,
  `Cancel`, `spawn`/`await`, `sleep`. Native work-stealing executor in 3.1.
- **Phase 4 self-host (stage-1.0)**: a mom-in-mom compiler in
  `compiler/src/main.mom` that takes a real `.mom` file, emits C99, links
  via `cc`, and produces a native binary. Stage-0 interprets it.
  `compiler/bootstrap.sh` drives the whole chain.
- **Phase 5.0 tooling**: the developer-facing surface is wired end-to-end —
  `mom fmt` (deterministic re-indenter, `--check` mode), `mom lint`
  (severity-tunable categories driven by `[lints]` in `mom.toml`),
  `mom doc` (Markdown API generator), `mom test` (discovers and runs
  `tests/**/*.mom`), `mom new` / `mom init` (project scaffolding),
  `mom pkg` (`list`/`add`/`remove`/`audit` over `[dependencies]`), and
  `mom lsp` (LSP-over-stdio with diagnostics). `mom dbg`, `mom prof`,
  and `mom bench` are stubs that land in Phase 5.1.
- Recognized but deferred to later sub-phases: `extern` FFI execution,
  multi-threaded async, dedicated `actor … receive { … }` syntax,
  supervision tree runtime, full generics monomorphization in native
  output, and self-hosted stage-2.

The complete plan, including the LLVM IR backend and self-hosting,
is in [`docs/plan.md`](docs/plan.md) and [`docs/roadmap.md`](docs/roadmap.md).

---

## Quick Start

```sh
cargo run -- run    examples/hello.mom        # interpret
cargo run -- check  examples/fib.mom          # type-check + borrow-check
cargo run -- ast    examples/pipeline.mom     # show AST
cargo run -- tokens examples/lists.mom        # show tokens

# Compile to a real native binary via the Rust-hosted C backend:
cargo run -- build     examples/fib.mom -o /tmp/fib
/tmp/fib                                      # → 55

# Compile and immediately execute:
cargo run -- build-run examples/state.mom     # → 5

# Inspect the generated C source:
cargo run -- emit-c    examples/fib.mom

# Phase 4: the stage-1 mom-in-mom compiler.
# Stage-0 interprets compiler/src/main.mom, which emits C, then cc links it.
./compiler/bootstrap.sh compiler/examples/answer.mom
./target/stage1/answer                        # → 42
```

After building once you can use the binary directly:

```sh
./target/debug/mom run examples/hello.mom
./target/debug/mom build examples/fib.mom -o ./fib && ./fib
```

---

## A Taste of mom

```mom
module geometry {
    pub struct Point { x: Float, y: Float }

    impl Point {
        fn distance(self, other: Point) -> Float {
            let dx = self.x - other.x
            let dy = self.y - other.y
            (dx * dx + dy * dy)
        }
    }
}

import geometry.{Point}

fn parse(value: Int) -> Result[Int, String] {
    if value < 0 { Err("negative") } else { Ok(value * 2) }
}

fn main() {
    let p = Point { x: 0.0, y: 0.0 }
    let q = Point { x: 3.0, y: 4.0 }
    print(p.distance(q))

    let xs = [1, 2, 3, 4, 5]
    let mut total = 0
    for x in xs {
        total = total + x
    }
    print(total)

    match parse(21) {
        Ok(v)  => print(v),
        Err(e) => print(e),
    }
}
```

Immutable `let` bindings are the default. Mutation is opt-in with `let mut`.
Errors flow through `Result[T, E]` with `?` for early-return propagation.
There are no `null` values — absence is modelled by `Option[T]`.

---

## CLI Commands

```sh
# Compiler frontend
mom tokens    <file.mom>                  # show token stream
mom ast       <file.mom>                  # print parsed AST
mom check     <file.mom>                  # run type checker
mom run       <file.mom>                  # run via bootstrap interpreter
mom build     <file.mom> [-o OUT] [--release]   # compile native binary
mom build-run <file.mom> [-o OUT] [--release]   # compile then execute
mom emit-c    <file.mom>                  # show generated C source

# Phase 5 tooling
mom fmt       <file.mom> [--check]        # reindent in place; --check exits 1 if dirty
mom lint      <file.mom>                  # apply mom.toml [lints] config
mom doc       <file.mom>                  # emit Markdown API reference
mom test      [dir]                       # discover + run *.mom tests
mom new       <dir>                       # scaffold a new mom project
mom init      [dir]                       # scaffold in an existing directory
mom pkg       list|add|remove|audit       # manage [dependencies] in mom.toml
mom lsp                                   # LSP server on stdio

mom version                               # print compiler version
mom help                                  # full help
```

---

## Documentation Index

| Topic                              | File                                |
|------------------------------------|-------------------------------------|
| Language philosophy and goals      | [docs/philosophy.md](docs/philosophy.md) |
| Surface language design            | [docs/design.md](docs/design.md)        |
| Formal grammar (EBNF)              | [docs/grammar.ebnf](docs/grammar.ebnf)  |
| Type system                        | [docs/types.md](docs/types.md)          |
| Memory model & safety              | [docs/memory.md](docs/memory.md)        |
| Concurrency, actors, supervision   | [docs/concurrency.md](docs/concurrency.md) |
| C and C++ interoperability         | [docs/interop.md](docs/interop.md)      |
| Build system & package manager     | [docs/build_system.md](docs/build_system.md) |
| Standard library plan              | [docs/stdlib.md](docs/stdlib.md)        |
| Tooling: fmt, lint, LSP, debugger  | [docs/tooling.md](docs/tooling.md)      |
| Compiler architecture              | [docs/compiler.md](docs/compiler.md)    |
| Bootstrapping & self-hosting plan  | [docs/bootstrap.md](docs/bootstrap.md)  |
| Engineering roadmap                | [docs/roadmap.md](docs/roadmap.md)      |
| Risks & mitigations                | [docs/risks.md](docs/risks.md)          |

---

## Repository Layout

```
.
├── Cargo.toml              # Rust workspace for the bootstrap toolchain
├── src/                    # Rust sources for lexer, parser, AST, types,
│                           # borrow checker, interpreter, codegen, and
│                           # every `mom <subcommand>`
├── runtime/                # native runtime support
├── compiler/               # native compiler scaffolding
├── std/                    # Phase 6 standard library (stage-0 .mom files)
├── tests/                  # Rust acceptance suites: language, memory,
│                           # concurrency, native_build, selfhost, tooling,
│                           # stdlib
├── examples/               # Sample .mom programs
├── docs/                   # Language design, grammar, plan, roadmap
├── rfcs/                   # RFC template + accepted proposals
├── scripts/                # install.sh and other ops scripts
├── .github/workflows/      # CI matrix + release pipeline
├── CHANGELOG.md            # Keep-a-Changelog history
├── CONTRIBUTING.md         # contributor handbook
├── SECURITY.md             # vulnerability reporting policy
├── RELEASING.md            # release runbook
├── CODE_OF_CONDUCT.md      # community standards
├── MAINTAINERS.md          # area owners
└── README.md
```

---

## License

Apache-2.0.
