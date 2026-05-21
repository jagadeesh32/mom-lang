# Changelog

All notable changes to **mom** are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
adheres to [Semantic Versioning](https://semver.org/) once `1.0.0`
ships.

## [Unreleased]

### Stage-0 widening (cross-phase)

- **Phase 1 native codegen** now handles `Float` (literals, arithmetic,
  comparisons, `print`); `runtime.c` gained `mom_print_float`. New
  acceptance tests `native_float_arithmetic_and_print` +
  `native_float_comparison_returns_bool` in `tests/native_build.rs`.
- **Phase 5.1 `mom bench`** discovers `#[bench] fn name()` functions
  inside any `.mom` file under `src/` or `tests/` in addition to the
  existing file-level rules. Lexer now emits `Hash`; parser accepts
  `#[ident]` outer attributes on `fn` items; `FunctionDecl.attrs`
  carries them through the AST. New acceptance test
  `bench_discovers_pound_bench_attribute_functions_in_src`.
- **Phase 6 standard library** gains four stretch modules:
  - `std/sync.mom`  — Mutex/Atomic/Once surface over the single-thread interpreter.
  - `std/os.mom`    — env/sleep/process-info wrappers over the host shims.
  - `std/math.mom`  — gcd, lcm, pow_int, factorial, fib, Lehmer LCG RNG.
  - `std/test.mom`  — assert_eq_int / assert_true / assert_false + TestStats.
  `tests/stdlib.rs` gains four oracles and the drift-sweep covers all 14 modules.

### Phase 5.1 native parity catch-up

- `mom fmt` now uses an **AST-based pretty-printer** (`src/fmt_ast.rs`);
  the textual re-indenter is the fallback path when source doesn't
  parse. Imports normalize to the canonical `import a.b` form. New
  acceptance tests `fmt_normalizes_spacing_via_ast_printer` and
  `fmt_ast_normalizes_struct_literal_spacing`.
- `mom prof --format pprof-proto` (or `proto`) emits the canonical
  pprof protobuf wire format via an in-tree minimal proto3 encoder
  (`prof::render_pprof_proto_bytes`). New acceptance test
  `prof_pprof_proto_bytes_include_function_names_in_string_table`.
- `mom bench` accepts `#[bench] fn name(b: Bencher)`: the harness
  auto-injects a stage-0 `Bencher` struct with a `b.iter(closure)`
  loop. New acceptance test
  `bench_discovers_pound_bench_with_bencher_parameter`.
- Flaky native-build race (`ETXTBSY` on concurrent exec) fixed via
  retry loop in `tests/native_build.rs::compile_and_run`.

### Still pending — tracked as RFCs, deliberately out-of-scope here

- RFC #0001 — Stage-2 native codegen for String, List, Struct, Enum,
  Match (`rfcs/0001-stage-2-native-codegen-widening.md`).
- RFC #0002 — DWARF v5 / CodeView + real breakpoints
  (`rfcs/0002-dwarf-and-real-breakpoints.md`).
- RFC #0003 — Tier-2 platforms WASM / RISC-V / Cortex-M
  (`rfcs/0003-tier-2-platforms.md`).
- RFC #0004 — Production-user outreach
  (`rfcs/0004-production-user-outreach.md`).

### Phase 7 — Production launch artifacts (stage-0)

- Added `CONTRIBUTING.md`, `SECURITY.md`, `RELEASING.md`,
  `CODE_OF_CONDUCT.md`, and an RFC template under `rfcs/`.
- Added a GitHub Actions matrix CI workflow targeting the tier-1
  platforms in `docs/roadmap.md` (`linux × {x86_64, aarch64}`,
  `macos × {x86_64, aarch64}`, `windows × x86_64`).
- Added `scripts/install.sh` — a curl-pipe-able installer that
  picks the right release artifact for the host triple.
- Tier-2 platforms (WASM, RISC-V, Cortex-M), production-user case
  studies, and the public RFC process are deliberately tracked
  outside this repo; their acceptance is gated by external work.

## [0.6.0] — Phase 6 standard library

- `std/core.mom` — identity, min, max, clamp, abs, sign, Option /
  Result helpers.
- `std/fmt.mom`  — `repeat`, `pad_left`, `pad_right`, `join`,
  `join_ints`, `key_value`, `bracket`.
- `std/alloc.mom` — `Box`, `Rc`, `Arc`, region demo.
- `std/io.mom`   — `LineBuffer.write` / `writeln` / `flush`.
- `std/log.mom`  — `Level` enum, `Logger` with rank-gated emit.
- `std/async.mom` — `compute`, `join_all_int`, `yield_now` over the
  stage-0 synchronous executor.
- `std/actor.mom` — channel-driven mailbox + run loop.
- `std/net.mom`  — `Address`, `Request`, `Response`, route
  `dispatch`.
- `std/serde.mom` — JSON-ish encoders (bool/int/string + list
  variants + key/value).
- `std/crypto.mom` — Adler-32, polynomial rolling hash, hex
  encoders.
- `tests/stdlib.rs` — 11 acceptance tests (one per module + a
  drift-sweep that fails if a module file is added without an oracle).

## [0.5.1] — Phase 5.1 native tooling

- `mom bench` — `benches/**/*.mom` + `src/**/*_bench.mom` discovery,
  warmup + iter sampling → min/median/mean/stddev/max; `--json`.
- `mom prof`  — interpreter call-trace profiler with `text`,
  `folded` (Brendan Gregg), and pprof-JSON renderers.
- `mom dbg`   — DAP-over-stdio driver: `initialize` / `launch` /
  `threads` / `stackTrace` / `continue` / `disconnect`.
- Interpreter gained `attach_probe()` and per-call enter/exit hooks.

## [0.5.0] — Phase 5.0 tooling

- `mom fmt`, `mom lint`, `mom doc`, `mom test`, `mom new`/`mom init`,
  `mom pkg`, `mom lsp` shipped on the Rust stage-0 (see
  `docs/tooling.md`).

## [0.4.0] — Phase 4 self-host (stage-1.0)

- Stage-1 compiler emits C for the Int/Bool/Unit subset; passes the
  bit-identical fixed-point regression tests in `tests/selfhost.rs`.

## [0.3.0] — Phase 3 concurrency

- Channels, cancel tokens, async/await, actor syntax at interpreter
  level. See `docs/concurrency.md`.

## [0.2.0] — Phase 2 memory & ownership

- Move semantics, references, regions, borrow checker. See
  `docs/memory.md`.

## [0.1.0] — Phase 1 native backend

- Codegen-to-C path for the Int/Bool/Unit subset.

## [0.0.1] — Phase 0 stage-0 toolchain

- Lexer, parser, AST, interpreter, diagnostic plumbing.
