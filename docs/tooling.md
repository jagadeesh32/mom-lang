# mom — Tooling

The `mom` CLI is the single entry point for every developer interaction.
There are no third-party "official" tools — the toolchain ships
together so that versions cannot drift.

```
mom build       # compile
mom run         # build + run
mom test        # unit/integration/property tests
mom bench       # benchmarks
mom fmt         # formatter
mom lint        # linter / static analyzer
mom check       # type check only (CI gate)
mom doc         # API docs generator
mom pkg         # package manager (add, upgrade, audit, publish)
mom lsp         # language server (stdio-based)
mom dbg         # debugger driver (DAP-compatible)
mom prof        # profiler harness
mom new / init  # scaffolding
```

## Phase 5 status — what ships today

The Rust stage-0 toolchain in this repo implements every developer
command — including `dbg`, `prof`, and `bench`, which landed in
Phase 5.1. Each implementation is dependency-free and lives under
`src/`:

| Command                | Module                | Notes |
|------------------------|-----------------------|-------|
| `mom fmt`              | `src/fmt.rs`          | Deterministic re-indenter; idempotent; `--check` for CI |
| `mom lint`             | `src/lint.rs`         | Severity categories driven by `[lints]` in `mom.toml` |
| `mom doc`              | `src/doc.rs`          | Markdown API generator from `pub` items + leading `//` comments |
| `mom test`             | `src/test_runner.rs`  | Discovers `tests/**/*.mom` and `src/**/*_test.mom` |
| `mom new` / `mom init` | `src/scaffold.rs`     | `mom.toml` + `src/main.mom` + `tests/smoke_test.mom` + `.gitignore` |
| `mom pkg`              | `src/pkg.rs`          | `list`/`add`/`remove`/`audit` on `[dependencies]` |
| `mom lsp`              | `src/lsp.rs`          | stdio JSON-RPC; `initialize` + `publishDiagnostics` via `check_source` |
| `mom bench`            | `src/bench.rs`        | Discovers `benches/**/*.mom` + `src/**/*_bench.mom`; warmup + iter sampling → min/median/mean/stddev/max; `--json` for CI |
| `mom prof`             | `src/prof.rs`         | Interpreter call-trace profiler; renders `text`, folded flamegraph, pprof-JSON |
| `mom dbg`              | `src/dbg.rs`          | DAP-over-stdio driver: `initialize` / `launch` / `threads` / `stackTrace` / `continue` / `disconnect` → `output` + `terminated` + `exited` events |
| Manifest reader        | `src/manifest.rs`     | Minimal TOML subset: `[section]`, strings, ints, bools |

All wire through `src/main.rs`. See `tests/tooling.rs` for the
acceptance contract (23 acceptance tests covering all of the above).

What is *deliberately* deferred from Phase 5.1 to the native stage-2:

- DWARF v5 / CodeView source-line emission, real breakpoints, async-aware
  stack unwinding inside `mom dbg`.
- Off-CPU and heap sampling, allocator hooks for `mom prof`.
- `#[bench]` attribute and the `Bencher` API for `mom bench` (the
  file-level convention is the stage-0 stand-in).
- Lockfile + registry for `mom pkg`.

---

## 1. Formatter — `mom fmt`

- **No options.** The format is decided by the compiler team; no
  bikeshed flags, no `.editorconfig`-fights. Pre-commit just runs
  `mom fmt`.
- Operates on the **AST**, not on text — comments are preserved in their
  semantic position, not their literal column.
- Sub-second on a 100 kLOC project (parallel, cache-aware).
- Deterministic; idempotent.

Sample:

```mom
fn   handle( req:Request   )->Response{ let body=req.body; Response{status:200, body}}
```

becomes:

```mom
fn handle(req: Request) -> Response {
    let body = req.body
    Response { status: 200, body }
}
```

## 2. Linter — `mom lint`

Built into the compiler frontend. Diagnostic categories:

- **`correctness`** — bugs (always-on, default-deny).
- **`suspicious`** — likely-wrong patterns (default-warn).
- **`performance`** — suboptimal patterns (default-allow).
- **`style`** — opinions (default-allow).
- **`unsafe-audit`** — every `unsafe` block lists why (default-warn).

Configurable per crate via `mom.toml`:

```toml
[lints]
default                = "warn"
correctness.shadowing  = "deny"
performance.allocation = "warn"
style.naming           = "allow"
```

## 3. Language server — `mom lsp`

- Speaks LSP over stdio.
- Reuses the compiler's incremental query engine — completions and
  errors update at typing speed.
- Features:
  - completions (type-aware, fuzzy)
  - hover with types and rustdoc-style summaries
  - go-to-definition, find-references, rename
  - code actions (fix, extract, inline)
  - inlay hints (parameter names, inferred types)
  - workspace symbol search
  - debugger entry points

## 4. Debugger — `mom dbg`

- Emits **DWARF v5** on hosted targets, **CodeView** on Windows.
- Provides Debug Adapter Protocol (DAP) so VS Code, Helix, Zed, and
  JetBrains plug in directly.
- Pretty-printers for `Option`, `Result`, `Vec`, `String`, `HashMap`,
  `Box`, `Rc`, `Arc`, `Future`, `Task`, custom `Debug` impls.
- Async-aware stack traces — `await` points show the awaiting task,
  not just the executor frame.

## 5. Profiler — `mom prof`

```sh
mom prof cpu  -- ./target/release/server --port 8080
mom prof heap -- ./target/release/server
mom prof flame                                  # render results
```

- CPU sampling profiler with off-CPU support.
- Heap profiler (`bytehound`-style live + allocation graph).
- Allocator tracking, async task lifetimes.
- Output → flamegraphs, pprof, OTLP.

## 6. Test framework — `mom test`

```mom
#[test]
fn it_adds() {
    assert_eq(add(2, 3), 5)
}

#[prop]
fn list_concat_length(a: [Int], b: [Int]) {
    assert_eq(len(a ++ b), len(a) + len(b))
}

#[bench]
fn sort_1m(b: &mut Bencher) {
    b.iter(fn() { sort(make_input(1_000_000)) })
}
```

- Unit tests (`#[test]`).
- Property tests (`#[prop]`, with shrinking).
- Benchmarks (`#[bench]`, statistically rigorous).
- Snapshots, fuzzing, doctests.
- Output: TAP, JUnit, JSON.
- Parallel by default; deterministic seeding.

## 7. Docs — `mom doc`

- Generates HTML and Markdown from declarations + `///` comments.
- Type-aware: every symbol page links its inputs, outputs, generic
  bounds, related types.
- Embedded example runnable: code blocks in docs are compiled and
  tested by `mom test`.
- Search index is precomputed; static hosting is one upload.

## 8. Coverage

```sh
mom test --coverage
mom test --coverage --format lcov > coverage.lcov
```

- Line + branch coverage.
- Integrates with `mom prof`'s sampler — same backend, same UI.

## 9. Fuzzing

```mom
#[fuzz]
fn decode_never_panics(data: [Byte]) {
    let _ = decode(data)
}
```

- libFuzzer + AFL++ backends.
- Stored corpus + minimization built in.
- `mom fuzz --duration 60s`.

## 10. Editor integration

| Editor   | Status                                  |
|----------|-----------------------------------------|
| VS Code  | extension shipping the LSP + DAP        |
| Neovim   | built-in via `nvim-lspconfig` + DAP     |
| Zed      | native LSP/DAP                          |
| Helix    | native LSP/DAP                          |
| JetBrains| plugin (IDE + LSP fallback)              |
| Emacs    | `lsp-mode` / `eglot` config             |
| Sublime  | LSP-mom package                          |

All editors get the **same** features because they all consume the
same `mom lsp`.
