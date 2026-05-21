# mom — Engineering Roadmap

This roadmap is **phased and measurable**. Each phase ends with a
demoable artifact, a regression test suite that locks the new
capability in, and a release tag.

## Phase 0 — Stage-0 toolchain (shipped, this repository)

| Deliverable                                                | Status |
|------------------------------------------------------------|--------|
| Rust crate `mom` with lexer, parser, AST, lenient typeck, interpreter | shipped |
| CLI `mom tokens / ast / check / run / version`            | shipped |
| Example `.mom` programs                                    | shipped |
| Documentation set                                          | shipped |
| Tests covering core executable subset                      | shipped |

## Phase 1 — Native backend (shipped — Int/Bool subset)

| Deliverable                                                  | Status | Acceptance               |
|--------------------------------------------------------------|--------|--------------------------|
| HIR + MIR data structures in stage-0                         | shipped (scaffold) | `src/hir.rs`, `src/mir.rs` |
| **C-codegen backend** for Int/Bool/Unit subset               | shipped | `src/codegen.rs`        |
| Linker driver invoking system `cc`                           | shipped | `src/build.rs`          |
| Tiny C runtime (`runtime/runtime.c`)                         | shipped | `mom_print_*`, `main`   |
| Object cache (content-addressed)                             | shipped | second `mom build` is a copy |
| `mom build` / `mom build-run` / `mom emit-c` CLI             | shipped | 8 native-build tests pass |
| LLVM IR emitter (sub-phase 1.1)                              | planned | optional inkwell dep    |
| Float/String/List/Struct/Enum codegen (sub-phase 1.3)         | planned | extends `Backend` trait |
| Cross-compile matrix: `x86_64-linux`, `aarch64-linux`, `aarch64-darwin`, `x86_64-windows` | planned | green CI matrix |

## Phase 2 — Memory & ownership (shipped)

| Deliverable                                            | Status  | Acceptance                          |
|--------------------------------------------------------|---------|-------------------------------------|
| `&T`, `&mut T` in types + expressions                  | shipped | reference syntax across stage-0     |
| Region blocks (`region NAME { … }`)                    | shipped | `examples/memory.mom` runs          |
| `Box` / `Rc` / `Arc` built-in smart pointers           | shipped | constructed and printed             |
| Borrow checker for stage-0 subset (`src/borrow.rs`)    | shipped | 11 tests in `tests/memory.rs`       |
| Wired into `mom check` + `mom run`                     | shipped | borrow violations rejected pre-exec |
| Native codegen for references / regions                | sub-phase 2.1 | `&T` → `const T*`, `&mut T` → `T*` |
| Full move semantics on function arguments              | sub-phase 2.1 | per-function ownership signatures |
| Data-race-free TCP echo server                         | sub-phase 2.2 | needs Phase 3 runtime           |

## Phase 3 — Concurrency runtime (shipped at interpreter level)

| Deliverable                                            | Status  | Acceptance                          |
|--------------------------------------------------------|---------|-------------------------------------|
| `Channel(cap?)` + send/recv/try_recv/len/is_empty/close | shipped | 7 tests in `tests/concurrency.rs`  |
| `Cancel` token + signal/is_cancelled                   | shipped | cancel_token_signal                 |
| `spawn` + `await` lifecycle                            | shipped | task_spawn_await_lifecycle          |
| `sleep(ms)` builtin (no-op in bootstrap)               | shipped | sleep_is_a_typed_noop               |
| Actor pattern via struct + channels                    | shipped | `examples/actor_via_channels.mom`   |
| Multi-threaded work-stealing executor                  | sub-phase 3.1.1 | native runtime              |
| **Dedicated `actor … receive { … }` syntax sugar**     | shipped | desugars to `struct + step`        |
| Broadcast + oneshot channels                           | sub-phase 3.2 | API extensions                |
| Supervision tree built-in (`supervise … with …`)       | sub-phase 3.2 | runtime restart driver        |

## Phase 4 — Self-host (shipped — stage-1.0)

| Deliverable                                            | Status  | Acceptance                          |
|--------------------------------------------------------|---------|-------------------------------------|
| `compiler/` directory: mom-in-mom compiler             | shipped | `compiler/src/main.mom` lex+parse+emit-C |
| Stage-0 stdlib I/O: read_file/write_file/getenv/args   | shipped | feeds stage-1 source from disk      |
| `compiler/bootstrap.sh` driver                         | shipped | end-to-end stage-1 build chain      |
| stage-1.0 supported subset                             | shipped | `print`, `let [mut]`, assignment, `+`, `-`, `*` over Int |
| **stage-1.1** functions, `if/else`, `while`, `return`, Bool, comparisons, logical ops | shipped | 10 selfhost tests; `fib` / `factorial` / Bool examples |
| 10 integration tests in `tests/selfhost.rs`            | shipped | literals / let / mul / precedence / mutate / recursion / while / Bool / if-else / cross-fn calls |
| stage-1.2: structs, enums, pattern matching, lists     | sub-phase 4.2 | port the AST + visitor              |
| stage-1.3: references, regions, smart pointers         | sub-phase 4.3 | port the borrow checker             |
| stage-1.4: stage-1 compiles itself                     | sub-phase 4.4 | bit-identical fixed point           |
| Stage-2: Rust stage-0 retired                          | sub-phase 4.5 | mom binary self-distributes         |

## Phase 5 — Tooling (5.0 and 5.1 shipped on stage-0; native parity in flight)

| Deliverable                       | Status        | Notes                                              |
|-----------------------------------|---------------|----------------------------------------------------|
| `mom fmt`                         | shipped (5.0) | deterministic re-indenter; `--check` exits non-zero on drift |
| `mom lint`                        | shipped (5.0) | severity categories + `[lints]` overrides in `mom.toml` |
| `mom doc`                         | shipped (5.0) | Markdown API + doc-comment extractor               |
| `mom test`                        | shipped (5.0) | discovers `tests/**/*.mom`, `src/**/*_test.mom`    |
| `mom new` / `mom init`            | shipped (5.0) | scaffolds project layout (mom.toml + src + tests)  |
| `mom pkg`                         | shipped (5.0) | `list`/`add`/`remove`/`audit` on `[dependencies]`  |
| `mom lsp`                         | shipped (5.0) | LSP stdio: initialize + diagnostics on change      |
| `mom dbg`                         | shipped (5.1) | DAP over stdio: initialize/launch/threads/stackTrace/disconnect; DWARF v5 emission deferred to native stage-2 |
| `mom prof`                        | shipped (5.1) | interpreter call-trace profiler → text / folded-flamegraph / pprof-JSON |
| `mom bench`                       | shipped (5.1) | discovers `benches/**/*.mom` + `src/**/*_bench.mom`; warmup + iter samples → min/median/mean/stddev/max |
| registry / lockfile               | sub-phase 5.2 | follows `mom pkg` once stage-2 retires Rust stage-0 |

## Phase 6 — Standard library (✅ stage-0 shipped; native parity follows stage-2)

Each module is a runnable `.mom` file under `std/`. `tests/stdlib.rs`
locks each module's output to an oracle so behaviour can't drift.

| Layer            | Status        | Stage-0 file       |
|------------------|---------------|--------------------|
| `std::core`      | ✅ shipped    | `std/core.mom`     |
| `std::alloc`     | ✅ shipped    | `std/alloc.mom`    |
| `std::io`        | ✅ shipped    | `std/io.mom`       |
| `std::async`     | ✅ shipped    | `std/async.mom`    |
| `std::actor`     | ✅ shipped    | `std/actor.mom`    |
| `std::net`       | ✅ shipped    | `std/net.mom`      |
| `std::fmt`       | ✅ shipped    | `std/fmt.mom`      |
| `std::serde`     | ✅ shipped    | `std/serde.mom`    |
| `std::crypto`    | ✅ shipped    | `std/crypto.mom`   |
| `std::log`       | ✅ shipped    | `std/log.mom`      |
| `std::sync`      | ✅ stretch    | `std/sync.mom`     |
| `std::os`        | ✅ stretch    | `std/os.mom`       |
| `std::math`      | ✅ stretch    | `std/math.mom`     |
| `std::test`      | ✅ stretch    | `std/test.mom`     |

## Phase 7 — Production launch (🟡 launch artifacts shipped; external items remain)

| Deliverable                                     | Status                          | Acceptance                            |
|-------------------------------------------------|---------------------------------|---------------------------------------|
| `mom 1.0`                                       | ⏳ pending phase exit            | semver stability commitment           |
| Tier-1 CI matrix YAML                           | ✅ `.github/workflows/ci.yml`    | green matrix on hosted runners (external) |
| Tier-2 platforms                                | ⏳ external                      | best-effort cross builds              |
| Production users at 3+ organisations            | ⏳ external                      | case studies, public reference        |
| Community RFC process                           | ✅ `rfcs/` with template + README | rfcs/ accepts PRs                     |
| Release runbook & workflow                      | ✅ `RELEASING.md` + `release.yml`| tag → release pipeline                |
| Security disclosure policy                      | ✅ `SECURITY.md`                 | 2-day ack, 30-day fix SLA             |
| Code of Conduct                                 | ✅ `CODE_OF_CONDUCT.md`          | enforced by `MAINTAINERS.md`          |
| Contributor handbook                            | ✅ `CONTRIBUTING.md`             | onboarding + acceptance bar           |
| Installer                                       | ✅ `scripts/install.sh`          | curl-pipe-able                        |
| Changelog                                       | ✅ `CHANGELOG.md`                | Keep-a-Changelog format               |

---

## Folder structure (target)

```
.
├── Cargo.toml          # stage-0 only
├── src/                # stage-0 Rust front end + interpreter
├── compiler/           # stage-1+ mom-in-mom compiler
│   ├── syntax/
│   ├── hir/
│   ├── mir/
│   ├── borrow/
│   ├── concurrency/
│   ├── codegen_llvm/
│   ├── codegen_native/
│   └── driver/
├── runtime/            # async, actor, channel, alloc primitives
│   ├── alloc/
│   ├── async/
│   ├── actor/
│   └── ffi/
├── std/                # standard library (in mom)
│   ├── core/
│   ├── alloc/
│   ├── io/
│   ├── net/
│   ├── os/
│   └── fmt/
├── tools/              # fmt, lint, lsp, dbg, doc, pkg
├── docs/               # this directory
├── examples/           # end-user .mom samples
└── tests/              # language conformance suite
```

## Team composition (recommended)

| Role                          | Headcount (minimum) |
|-------------------------------|---------------------|
| Compiler engineers (frontend) | 2                   |
| Compiler engineers (backend)  | 2                   |
| Runtime engineers             | 2                   |
| Standard library engineers    | 2                   |
| Tooling / LSP                 | 1                   |
| Documentation / advocacy      | 1                   |
| **Total**                     | **10**              |

A 3-person team can ship through Phase 2 in 12 months but cannot
sustain Phase 3+ in parallel.

## Risk register (summary)

See [risks.md](risks.md).

## Out-of-scope (intentionally)

- A full IDE (we partner with editors via LSP/DAP).
- Cloud SaaS hosting.
- Proprietary plugin ecosystem.
- An ML framework. (mom can host one, but doesn't ship one.)
