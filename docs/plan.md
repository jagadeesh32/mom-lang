# mom — Master Plan (All Phases)

This document is the single source of truth for **what mom delivers, in
what order, with which acceptance criteria**. Each phase ends with a
demoable artifact, a regression test suite that locks the capability
in, and a release tag.

Status legend: ✅ shipped · 🚧 in progress · ⏳ planned

---

## Phase 0 — Stage-0 toolchain  ✅

Bootstrap front end + tree-walk interpreter, written in Rust.

| Deliverable                                                          | Status |
|----------------------------------------------------------------------|--------|
| Rust crate `mom` with lexer, parser, AST, lenient typeck, interpreter | ✅ |
| CLI `mom tokens / ast / check / run / version`                       | ✅ |
| Built-in `Option[T]`, `Result[T,E]`, `?` propagation                  | ✅ |
| Lists, indexing, ranges, `for x in …`                                 | ✅ |
| Structs, struct literals, `impl` methods, field/dot access            | ✅ |
| Variant patterns `Some(x)`, `Ok(x)`, `Err(e)`                          | ✅ |
| Modules, imports, traits, extern blocks (parsed)                       | ✅ |
| Example `.mom` programs                                                | ✅ |
| Documentation set under `docs/`                                        | ✅ |
| 14-test conformance suite (cargo test)                                 | ✅ |

**Release tag**: `v0.1`.

---

## Phase 1 — Native backend  ✅ (this release)

Compile `.mom` source to a real native executable with no interpreter
on the critical path.

| Deliverable                                                                 | Status | Acceptance |
|-----------------------------------------------------------------------------|--------|------------|
| HIR + MIR data structures (`src/hir.rs`, `src/mir.rs`)                      | ✅    | scaffolding for LLVM/borrow passes |
| **C-codegen backend** (`src/codegen.rs`) — Int / Bool / Unit subset         | ✅    | functions, control flow, recursion, `print`, pipelines |
| Tiny C runtime (`runtime/runtime.c`, `runtime/runtime.h`)                   | ✅    | `mom_print_int`, `mom_print_bool`, `mom_print_unit` |
| Linker driver (`src/build.rs`)                                              | ✅    | spawns `cc` (or `$CC`), links runtime, emits binary |
| `mom build <file.mom> -o <out>` CLI command                                  | ✅    | exits 0; produces an ELF/Mach-O/PE |
| `mom build-run` + `mom emit-c` helpers                                       | ✅    | dev-time ergonomics |
| Content-addressed build cache (`target/mom-cache/`)                          | ✅    | second build of unchanged source skips `cc` |
| Integration tests: build → execute → assert stdout                          | ✅    | 8 native-build tests in `tests/native_build.rs` |
| **Sub-phase 1.1**: LLVM IR backend behind the same `Backend` trait          | ⏳    | optional dependency on `inkwell` |
| **Sub-phase 1.2**: cross-compile `x86_64-linux`, `aarch64-linux`, `aarch64-darwin`, `x86_64-windows` | ⏳ | green CI matrix |
| **Sub-phase 1.3**: Float / String / List / Struct / Enum codegen            | ⏳    | extend `Codegen::emit_expr_value` |

**Release tag**: `v0.2` (this release) / `v0.3` for sub-phases.

What is *not* in Phase 1 (deferred to later phases by design):
- Float / String / List / struct / enum codegen (Phase 1.3)
- Generics monomorphization in native output (Phase 1.4)
- Borrow checker integration (Phase 2)
- Async runtime (Phase 3)

The C-backend choice is deliberate:
- Portable to every platform with a C compiler — no LLVM build dep.
- Trivial to audit. The emitted C is human-readable.
- Slots cleanly into the `Backend` trait so the LLVM backend can land
  without changing the front end.

---

## Phase 2 — Memory & ownership  ✅ (this release)

The bootstrap toolchain now enforces ownership rules in `check_source`
and `run_source`. The native backend still rejects references (sub-phase
2.1 lowers them to C pointers / LLVM SSA borrows).

| Deliverable                                          | Status | Acceptance                            |
|------------------------------------------------------|--------|---------------------------------------|
| `&T`, `&mut T` in types + expressions                | ✅    | `tests/memory.rs::function_with_reference_param_type_checks` |
| Region blocks (`region NAME { … }`)                  | ✅    | `tests/memory.rs::region_block_executes` + `examples/memory.mom` |
| `Box`, `Rc`, `Arc` built-in smart pointers           | ✅    | `tests/memory.rs::box_rc_arc_round_trip` |
| Borrow checker (`src/borrow.rs`)                     | ✅    | 11 memory tests — use-after-move, double mut-borrow, shared+mut, mutate-while-borrowed, immutable-rebinding, sequential scoped borrows |
| Wired into `mom check` and `mom run`                 | ✅    | `lib.rs` runs borrow check after typecheck |
| Lifetime elision (function signatures)               | partial | conservative; full inference in Phase 2.1 |
| Native codegen for references / regions              | ⏳ 2.1 | `&T` lowered to `const T*`, `&mut T` to `T*` |
| Per-function move/borrow signatures                  | ⏳ 2.1 | full move-on-call semantics            |
| Data-race-free TCP echo server                       | ⏳ 2.2 | needs Phase 3 runtime                 |

---

## Phase 3 — Concurrency runtime  ✅ (this release, interpreter level)

The bootstrap runtime is single-threaded and cooperative. Native
work-stealing arrives in Phase 3.1.

| Deliverable                                            | Status | Acceptance                            |
|--------------------------------------------------------|--------|---------------------------------------|
| `Channel(capacity?)` built-in + `.send` / `.recv` / `.try_recv` / `.len` / `.is_empty` / `.capacity` / `.close` | ✅ | `tests/concurrency.rs` (7 tests) |
| `Cancel()` token + `.signal` / `.is_cancelled`         | ✅    | cancel_token_signal                   |
| `spawn` + `await` lifecycle                            | ✅    | task_spawn_await_lifecycle            |
| `sleep(ms)` builtin (no-op in bootstrap)               | ✅    | sleep_is_a_typed_noop                 |
| Bounded channels enforce capacity                      | ✅    | channel_bounded_capacity_rejects_overflow |
| `Option`-returning `.recv` for empty channels          | ✅    | channel_empty_recv_returns_none       |
| Actor pattern via struct + channels                    | ✅    | `examples/actor_via_channels.mom`     |
| Multi-threaded work-stealing executor                  | ⏳ 3.1.1 | needs native compiler              |
| **Dedicated `actor … receive { … }` syntax sugar**     | ✅ 3.1 | desugars to struct + `step` method; covered by `actor_sugar_desugars_to_struct_and_step_method` test |
| Broadcast + oneshot channels                           | ⏳ 3.2 | API extensions                        |
| Supervision tree built-in (`supervise … with …`)       | ⏳ 3.2 | runtime restart driver                |

## Phase 4 — Self-host  ✅ (this release, stage-1.0)

| Deliverable                                            | Status | Acceptance                            |
|--------------------------------------------------------|--------|---------------------------------------|
| `compiler/src/main.mom` — mom-in-mom compiler          | ✅    | lex + parse + emit-C for the print/let/assign/arith subset |
| Stage-0 stdlib I/O: `read_file`, `write_file`, `args`, `getenv` | ✅ | feeds stage-1 source from disk    |
| Lexer helpers: `is_digit`, `is_alpha`, `is_alnum`, `parse_int`, `string_eq` | ✅ | keep stage-1 compact          |
| `compiler/bootstrap.sh` driver                         | ✅    | `bootstrap.sh source.mom -o bin` runs end-to-end |
| Integration tests in `tests/selfhost.rs`               | ✅    | 5 tests covering literals, let, mul, precedence, mutable assignment |
| stage-1 emits ELF binary that prints expected output   | ✅    | aarch64-linux ELF demoed              |
| stage-1.0 supported source subset                      | ✅    | `fn main() { let / let mut / NAME = EXPR / print(EXPR) }` over Int with `+`, `-`, `*` |
| **stage-1.1** — multiple functions, `if/else`, `while`, `return`, Bool, comparisons (`== != < <= > >=`), logical (`&& || !`), unary `-`, function calls | ✅ | 10 selfhost tests; compiles `fib`, `factorial`, mutual calls, Bool predicates |
| **stage-1.2** — `String` literals, `+` string concat, `len`/`str`/`println` builtins, `/`/`%` operators, `for i in lo..hi` loops | ✅ | 4 new selfhost tests; `compiler/examples/{hello,counter,sum,fibonacci,for_range}.mom` compile end-to-end via `mom selfhost` |
| stage-1.3 — Python-style `:` blocks in stage-1         | ⏳ 4.3 | indent-aware lexer in mom-in-mom       |
| stage-1.4 — structs, enums, pattern matching, lists    | ⏳ 4.4 | port the AST + visitor                |
| stage-1.5 — references, regions, `Box`/`Rc`/`Arc`      | ⏳ 4.5 | port the borrow checker               |
| stage-1.6 — stage-1 compiles itself (bit-identical FP) | ⏳ 4.6 | the headline self-host milestone      |
| stage-2 — Rust stage-0 retired                         | ⏳ 4.7 | mom binary self-distributes           |

---

## Phase 5 — Tooling  ✅ (5.0 + 5.1 shipped on the Rust stage-0)

Phase 5.0 shipped the deterministic, dependency-free Rust-side
implementation of every developer-facing subcommand so the toolchain
is usable end-to-end *today*. Phase 5.1 has now landed on stage-0:
the bench harness, profiler, and DAP debugger driver are all live.
Native parity (DWARF v5 / CodeView emission, off-CPU sampling,
`#[bench]` attribute) follows when the native stage-2 retires the
Rust front end.

| Deliverable | Status | Notes |
|-------------|--------|-------|
| `mom fmt`   | ✅ 5.0 | Deterministic re-indenter; idempotent on Int/Bool/Unit/Struct/Enum corpora; AST-based printer pending native parity |
| `mom lint`  | ✅ 5.0 | `correctness`/`suspicious`/`performance`/`style`/`unsafe-audit` categories; per-crate `[lints]` overrides in `mom.toml` |
| `mom doc`   | ✅ 5.0 | Markdown API generator that pairs `pub` items with their leading `//` comment block |
| `mom test`  | ✅ 5.0 | Discovery walker (`tests/**/*.mom`, `src/**/*_test.mom`) + interpreter harness |
| `mom new`/`mom init` | ✅ 5.0 | Scaffolds `mom.toml` + `src/main.mom` + `tests/smoke_test.mom` + `.gitignore` |
| `mom pkg`   | ✅ 5.0 | `list`/`add`/`remove`/`audit` over `[dependencies]`; registry/lockfile in 5.2 |
| `mom lsp`   | ✅ 5.0 | LSP over stdio: `initialize` / `didOpen` / `didChange` → `publishDiagnostics` |
| `mom dbg`   | ✅ 5.1 | DAP over stdio: `initialize` / `launch` / `threads` / `stackTrace` / `continue` / `disconnect` → `output`/`terminated`/`exited` events. DWARF emission + breakpoints follow native stage-2 |
| `mom prof`  | ✅ 5.1 | Interpreter call-trace profiler with per-fn calls/self/total ns; renderers for `text`, Brendan-Gregg `folded` flamegraph, and pprof-JSON |
| `mom bench` | ✅ 5.1 | Discovers `benches/**/*.mom` + `src/**/*_bench.mom`; warmup + iter sampling → min/median/mean/stddev/max; `--json` for CI |

---

## Phase 6 — Standard library  ✅ (stage-0 shipped under `std/`)

Each module is a runnable `.mom` file under `std/`; `tests/stdlib.rs`
enforces an oracle per module. Native stage-2 will swap each one for
its real implementation behind the same public surface.

| Layer            | Status        | Stage-0 file       | Notes                                                |
|------------------|---------------|--------------------|------------------------------------------------------|
| `std::core`      | ✅ shipped    | `std/core.mom`     | identity, min, max, clamp, abs, sign, Option/Result helpers |
| `std::alloc`     | ✅ shipped    | `std/alloc.mom`    | `Box`/`Rc`/`Arc` + region demo                        |
| `std::io`        | ✅ shipped    | `std/io.mom`       | `LineBuffer.write`/`writeln`/`flush`; real stdout in stage-2 |
| `std::async`     | ✅ shipped    | `std/async.mom`    | `compute`, `join_all_int`, `yield_now` over sync executor |
| `std::actor`     | ✅ shipped    | `std/actor.mom`    | channel-driven mailbox + run loop                     |
| `std::net`       | ✅ shipped    | `std/net.mom`      | Address/Request/Response + `dispatch`                 |
| `std::fmt`       | ✅ shipped    | `std/fmt.mom`      | repeat, pad_left/right, join, key_value               |
| `std::serde`     | ✅ shipped    | `std/serde.mom`    | JSON-ish encoders; decoder lands with native lexer    |
| `std::crypto`    | ✅ shipped    | `std/crypto.mom`   | Adler-32, polynomial rolling hash, hex encoders. FNV/SHA-256 land once bitwise ops do |
| `std::log`       | ✅ shipped    | `std/log.mom`      | Level enum, Logger struct with rank-gated emit        |
| `std::sync`      | ✅ stretch    | `std/sync.mom`     | Mutex/Atomic/Once surface; real contention arrives with the multi-thread scheduler |
| `std::os`        | ✅ stretch    | `std/os.mom`       | env/sleep/process-info wrappers over the host shims    |
| `std::math`      | ✅ stretch    | `std/math.mom`     | gcd, lcm, pow_int, factorial, fib, Lehmer LCG RNG      |
| `std::test`      | ✅ stretch    | `std/test.mom`     | assert_eq_int / assert_true / assert_false + TestStats |

---

## Phase 7 — Production launch  🟡 (launch artifacts shipped; external items outside-chat)

The file-based pieces of the launch are in the repo. The
inherently-external items (production users, public RFC traffic,
multi-platform CI runs against real hosted runners, tier-2 hardware)
must be driven outside this repo and are flagged below.

| Deliverable                                                 | Status                          | Acceptance                       |
|-------------------------------------------------------------|---------------------------------|----------------------------------|
| `mom 1.0` semver commitment                                 | ⏳ pending phase exit            | semver stability commitment      |
| Tier-1 platforms: Linux, macOS, Windows × x86_64 + aarch64  | ✅ CI matrix YAML shipped (`.github/workflows/ci.yml`) | green matrix on hosted runners (external) |
| Tier-2: WASM, RISC-V, embedded ARM Cortex-M                 | ⏳ best-effort                   | nightly cross builds (external)  |
| Production users at 3+ organisations                        | ⏳ external                      | case studies, public reference   |
| Community RFC process                                       | ✅ `rfcs/0000-template.md` + `rfcs/README.md` | rfcs/ accepts PRs                |
| Release runbook                                             | ✅ `RELEASING.md`                | tag → release workflow shipped (`.github/workflows/release.yml`) |
| Security disclosure policy                                  | ✅ `SECURITY.md`                 | 2-day ack, 30-day fix SLA        |
| Code of Conduct                                             | ✅ `CODE_OF_CONDUCT.md`          | enforced by `MAINTAINERS.md`     |
| Contributor handbook                                        | ✅ `CONTRIBUTING.md`             | onboarding + acceptance bar      |
| Installer                                                   | ✅ `scripts/install.sh`          | curl-pipe-able, signs to `$PREFIX/bin` |
| Changelog                                                   | ✅ `CHANGELOG.md`                | Keep-a-Changelog format          |

---

## Engineering rhythm

- **Release cadence**: every 6 weeks until 1.0; then every quarter.
- **Cut policy**: feature-complete branch freezes 1 week before tag.
- **Reproducibility CI**: every release tag verified with a fresh
  stage-2 self-build that produces bit-identical artifacts.
- **Security**: 7-day SLA on critical advisories; signed binaries.
- **RFC process**: anything affecting the language surface goes through
  an RFC PR with a 2-week comment window.

---

## What stays out of scope (permanent)

- A full IDE — we partner with editors via LSP/DAP.
- A managed cloud / SaaS hosting layer.
- A proprietary plugin marketplace.
- A bundled ML framework. mom can host one; it doesn't ship one.
- Garbage collection as a default. Opt-in `gc { … }` regions are on
  the roadmap for AI/graph workloads, but the default is regions +
  borrow + actor isolation.

---

## Where each piece lives in the repo (target)

```
.
├── Cargo.toml          # stage-0 only; retired in Phase 4
├── src/                # stage-0 Rust front end + interpreter + C-codegen + driver
│   ├── lexer.rs
│   ├── parser.rs
│   ├── ast.rs
│   ├── token.rs
│   ├── diagnostic.rs
│   ├── typechecker.rs
│   ├── interpreter.rs
│   ├── hir.rs          # Phase 1 scaffolding
│   ├── mir.rs          # Phase 1 scaffolding
│   ├── codegen.rs      # Phase 1 C-backend
│   ├── build.rs        # Phase 1 driver
│   ├── main.rs
│   └── lib.rs
├── runtime/            # Phase 1 C runtime; later mom-native
│   ├── runtime.c
│   └── runtime.h
├── compiler/           # Phase 4 mom-in-mom compiler
├── std/                # Phase 6 standard library, in mom
├── tools/              # Phase 5 fmt, lint, lsp, dbg, doc, pkg
├── docs/               # this directory
├── examples/           # end-user .mom samples
└── tests/              # language conformance suite
```

## Where each phase is tracked

- Issue labels: `phase-1`, `phase-2`, …
- Project board columns: Backlog · Triaged · In Progress · Review · Done.
- Each phase has a tracking issue collecting all dependent issues and
  the acceptance test that demonstrates "Phase N is done".
