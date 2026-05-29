# Changelog

All notable changes to **mom** are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
adheres to [Semantic Versioning](https://semver.org/) once `1.0.0`
ships.

## [Unreleased]

## [0.3.0] — 2026-05-29 — native structs, strings, enums & cache integrity

### Stage-1.3 native backend widening (stage-0 C codegen)

The native `mom build` C backend grew from a scalar-only subset to cover
the core data model. All items below are validated end-to-end (compile →
link → run) and against the interpreter:

- **String literals** lower to C `const char*` (with correct re-escaping
  of `\n`, `\t`, `"`, `\\`, control chars) and print via `mom_print_str`.
- **Structs** lower to plain C structs: struct literals
  (`Point { x: 1, y: 2 }`, fields matched by name regardless of order),
  field access (`o.inner.v`), and field assignment (`c.n = c.n + 1`).
- **Enums + `match`** lower to C tagged unions, supporting payload-carrying
  and nullary variants, value-form and statement-form `match`, and
  variant constructors.
- **Nested & literal sub-patterns** inside enum patterns
  (`Wrap(A(n))`, `Val(0)`) lower to short-circuiting tag/value guards via
  a recursive pattern matcher.
- Struct and enum types may reference one another in any declaration
  order; types are emitted in source order so value-embedded types are
  complete at first use.

### Surface language (stage-0)

- **Match-arm assignment bodies**: `Some(Add(n)) => count = count + n` now
  parses (assignment lowered to a single-statement block expression),
  unblocking state-machine-style `match`.
- **`block:` scoped expression**: a `block` opens a fresh lexical scope
  (so a `&mut` borrow taken inside it ends at the block boundary) and
  evaluates to its tail expression.

### Bug fixes (stage-0)

- **Build cache integrity**: the native build cache key now incorporates
  the running compiler binary's identity (length + mtime, version
  fallback). Previously a rebuilt compiler with new codegen could serve a
  stale binary from `target/mom-cache/` for an unchanged source.
- **Borrow checker false moves**: values used inside a list literal
  (`[a, identity(a)]`) and the base of a field/index assignment
  (`c.n = ...`) are now treated as reads, not moves — matching the
  relaxation already applied to struct-literal fields and call arguments.
  Genuine use-after-move via rebinding is still rejected.

### Stage-1.2 mom-in-mom compiler widening

- **String type** in stage-1: string literals, `String`/`Str` type name,
  `+` concatenation between two strings (lowered to `mom_strcat_alloc`),
  `len(s)` (lowered to `mom_str_len_raw`), `str(int)` /`str(bool)`
  conversions (`mom_str_from_int` / `mom_str_from_bool`), and `println(s)`
  (lowered to `mom_print_str`). New runtime symbols live in
  `compiler/runtime.h` / `compiler/runtime.c`.
- **Division and modulo** in stage-1: `/` and `%` tokens, `EDiv`/`EMod`
  expression nodes, codegen at the same precedence as `*`.
- **For-in-range loops** in stage-1: `for i in lo..hi { ... }` lowers to a
  C `for` loop bound to `int64_t`.
- **`mom selfhost`** CLI subcommand orchestrates the stage-1 pipeline
  end-to-end (interpret stage-1 → emit C → link with runtime →
  optionally run); see `mom help`. Flags: `-o OUT`, `--run`,
  `--emit-c PATH`.
- **Stage-1.2 acceptance tests** in `tests/selfhost.rs`:
  `stage1_2_compiles_division_and_modulo`,
  `stage1_2_compiles_string_println_concat`,
  `stage1_2_compiles_str_bool_and_len`,
  `stage1_2_compiles_for_in_range`.

### Bug fixes (stage-0)

- **Brace-style block expressions** now parse correctly when used as
  match-arm bodies (e.g. `Some(Inc) => { count = count + 1 }`). The
  parser previously committed to a dict literal as soon as it saw `{`,
  yielding "expected ':' in dict literal" for any block-shaped arm.
  Disambiguation now looks at the next two tokens: `IDENT :` /
  `STRING :` → dict; anything else → block expression.
- **Bounded channels** now reject `.send(v)` when the queue is at
  capacity, raising `bounded channel at capacity (N)`. Previously the
  capacity argument was retained but never enforced.
- **`channel_powers_actor_like_pattern`**,
  **`channel_bounded_capacity_rejects_overflow`**,
  **`allows_sequential_mut_borrows`**, and
  **`std_actor_runs_to_completion_with_expected_oracle`** all pass now
  as a result of the two fixes above.

### Still out of scope (deferred to stage-1.3+)

- Python-style INDENT/DEDENT in stage-1 — requires an indent-aware
  lexer; current stage-1 accepts the brace form only. The
  `compiler/examples/*.mom` files were rewritten in brace style so the
  bootstrap pipeline runs against the same files the surface compiler
  ingests.
- Self-host fixed point (stage-1.4): stage-1 still lacks structs,
  enums, lists, and pattern matching, which are used by
  `compiler/src/main.mom` itself. RFC #0001 tracks the codegen
  widening required to close this gap.


## [0.2.0] — Python-style syntax + Windows ARM

### Added

- **Python-style indentation syntax**: blocks are now opened with `:` and
  delimited by INDENT/DEDENT tokens instead of `{ }`. Applies to `fn`,
  `if`, `else`, `elif`, `while`, `for`, and `match` bodies.
- **Dual-mode parsing**: the classic `{ }` brace syntax is still accepted
  so existing code continues to compile without changes.
- **Comprehensive standard library** — 60+ built-in functions covering
  arithmetic, string manipulation, list operations, I/O, and type
  conversion.
- **Dict data type**: key-value map literal `{"key": value}` with
  `dict_get`, `dict_set`, `dict_keys`, `dict_values`, `dict_contains`.
- **Python keywords**: `and`, `or`, `not` as aliases for `&&`, `||`, `!`;
  `elif` as an alias for `else if`; `none` as the unit/null sentinel;
  `True` / `False` as aliases for `true` / `false`.
- **Windows ARM build targets**: `aarch64-pc-windows-msvc` added to both
  the CI matrix and the release pipeline; cross-compiled via `cross`.
- **Duck typing improvements**: method dispatch now falls back to
  structural matching when a concrete trait bound is absent.

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

## [0.2.0-pre] — Phase 2 memory & ownership

- Move semantics, references, regions, borrow checker. See
  `docs/memory.md`.

## [0.1.0] — Phase 1 native backend

- Codegen-to-C path for the Int/Bool/Unit subset.

## [0.0.1] — Phase 0 stage-0 toolchain

- Lexer, parser, AST, interpreter, diagnostic plumbing.
