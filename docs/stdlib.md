# mom — Standard Library Plan

The mom standard library is layered so that the minimum useful binary is
**tiny** and grows only as features are imported. Each layer has a clear
contract on allocation, blocking, and runtime dependency.

## Phase 6 status — stage-0 implementations live under `std/`

Each module is a runnable `.mom` file; `tests/stdlib.rs` enforces an
oracle per module so behaviour can't drift. The surface mirrors what
the native stage-2 will expose; only the *implementations* will be
swapped.

| Module          | Stage-0 file       | Surface (selected)                                                   |
|-----------------|--------------------|----------------------------------------------------------------------|
| `std::core`     | `std/core.mom`     | `identity`, `min`, `max`, `clamp`, `abs`, `sign`, `option_or`, `result_or` |
| `std::fmt`      | `std/fmt.mom`      | `repeat`, `pad_left`, `pad_right`, `join`, `join_ints`, `key_value`  |
| `std::alloc`    | `std/alloc.mom`    | `Box`, `Rc`, `Arc`, region demo                                       |
| `std::io`       | `std/io.mom`       | `LineBuffer.write`, `LineBuffer.writeln`, `LineBuffer.flush`         |
| `std::log`      | `std/log.mom`      | `Level` enum, `Logger.at`, `logger_for`                              |
| `std::async`    | `std/async.mom`    | `compute`, `join_all_int`, `yield_now`                               |
| `std::actor`    | `std/actor.mom`    | `CounterMsg`, `run_counter` mailbox loop                             |
| `std::net`      | `std/net.mom`      | `Address`, `Request`, `Response`, `dispatch`                         |
| `std::serde`    | `std/serde.mom`    | `encode_bool`/`int`/`string`/`int_list`/`string_list`/`kv`           |
| `std::crypto`   | `std/crypto.mom`   | `adler32`, `poly_hash`, `hex_byte`, `hex_int`                        |
| `std::sync`     | `std/sync.mom`     | `Mutex`, `Atomic`, `Once` surface (single-threaded stub)             |
| `std::os`       | `std/os.mom`       | `env_or`, `sleep_ms`, `current_process`                              |
| `std::math`     | `std/math.mom`     | `gcd`, `lcm`, `pow_int`, `factorial`, `fib`, Lehmer LCG `Rng`        |
| `std::test`     | `std/test.mom`     | `TestStats`, `assert_eq_int`, `assert_true`, `assert_false`          |

The following live in the design but **still** ship after stage-2 — the
stage-0 interpreter can't carry the semantics:

- `std::sync` contention semantics (today's `Mutex` is a single-thread stub).
- `std::os` real syscall bindings (today's surface is a wrapper on
  the interpreter's host shims).
- `std::math` Float intrinsics (sin/cos/log/exp), big integers, and a
  real CSPRNG — these need bitwise ops the lexer doesn't accept yet.

---


```
std::core   – primitives, traits, sum types, slices, panics            (no allocator)
std::alloc  – Box, Rc, Arc, Vec, String, HashMap                       (allocator)
std::io     – files, stdio, sockets, pipes, async wrappers             (alloc + runtime)
std::sync  – Mutex, RwLock, Atomic, channel                            (threading)
std::async – Future, Task, executor, sleep, timeout                    (runtime)
std::actor – Actor, Supervisor, restart policies                       (runtime)
std::net   – TCP, UDP, TLS, HTTP, gRPC, DNS                            (runtime)
std::os    – process, fs, env, signals, time                           (host OS)
std::fmt   – formatting, Display, Debug, write!                        (alloc)
std::math  – Float ops, RNG, big integers                              (alloc?)
std::serde – derive(Serialize/Deserialize), JSON, CBOR, MsgPack        (alloc)
std::test  – test runner, property, benchmark harness                  (alloc)
std::crypto – hashing, AEAD, x509, signatures                          (alloc)
std::log    – structured logging, tracing spans                        (alloc)
```

Every layer above `std::core` is **opt-in** via the manifest:

```toml
[dependencies.std]
features = ["alloc", "io", "async", "net"]
```

Embedded and kernel targets ship with `features = ["core"]` only.

---

## `std::core`

| Item | Notes |
|------|-------|
| Primitives | `Int`, `Float`, `Bool`, `String`, `Char`, `()` |
| `Option[T]`, `Result[T, E]` | prelude, with `?` propagation |
| `Iterator` trait | `next`, `map`, `filter`, `fold`, `collect` |
| `Ord`, `Eq`, `Hash`, `Clone`, `Copy`, `Display`, `Debug` traits | — |
| Slices `[T]`, `[T; N]`, ranges | — |
| `Box[T]` (only if `alloc` is enabled) | — |
| `panic`, `assert`, `unreachable` | abort-only by default |
| `comptime` reflection on layouts and types | constrained surface |

## `std::alloc`

- Pluggable allocator interface (`Allocator` trait).
- `GlobalAllocator` is system-malloc by default, mimalloc on hosted
  targets when linked in.
- `Region`, `Bump`, `Pool`, `Slab` allocators.
- Containers: `Vec[T]`, `String`, `HashMap[K, V]`, `BTreeMap[K, V]`,
  `Set[T]`, `Deque[T]`, `Rc[T]`, `Arc[T]`.

## `std::io`

- `Read`, `Write`, `Seek` traits; sync + async variants.
- `File`, `Stdin`, `Stdout`, `Stderr`.
- `BufReader`, `BufWriter`.
- Path manipulation (`std::path::Path`).
- Memory-mapped files (`Mmap`).

## `std::async`

- `Future[T]`, `Task[T]`, `Cancel`.
- Executors: `current_thread`, `multi_thread`, `single_thread_io`.
- Timers: `sleep`, `interval`, `deadline`, `timeout`.
- Composition: `join`, `select`, `race`, `try_join`.

## `std::actor`

- `Actor` trait, `receive` macro-free DSL (compiler-recognized).
- `ActorRef[T]`, `Supervisor`, restart policies.
- Backpressure-aware mailboxes.

## `std::net`

- TCP, UDP, Unix sockets, IPC.
- TLS via system providers (rustls-equivalent in mom).
- HTTP/1.1, HTTP/2, HTTP/3 client + server.
- gRPC client + server, with codegen from `.proto`.
- DNS resolver, both blocking and async.

## `std::os`

- `process::exec`, `Command`, exit codes.
- Filesystem: open, create, mkdir, walk, watch.
- Environment variables, args, signals.
- Time: `Instant`, `Duration`, `SystemTime`.

## `std::fmt`

- `write!`, `format!` macros (recognized by the compiler).
- `Display`, `Debug`, `LowerHex`, `Binary`, …
- Locale-neutral; international formatting via `std::intl`.

## `std::serde`

- `derive(Serialize)`, `derive(Deserialize)`.
- Formats: JSON, CBOR, MsgPack, TOML, YAML.
- Zero-copy borrowed deserialization for slices.

## `std::test`

- `#[test]` and `#[bench]` attributes.
- Property-based: `prop fn foo(xs: [Int]) { … }`.
- Snapshot, fuzz, integration, doc tests.
- Output formats: TAP, JUnit, JSON for CI.

## `std::crypto`

- SHA-2, SHA-3, BLAKE3.
- AES-GCM, ChaCha20-Poly1305.
- Ed25519, X25519, ECDSA.
- X.509 verification with system trust store.

## `std::log`

- Structured key-value records.
- Spans (open-telemetry compatible).
- Pluggable exporters: stdout, file, OTLP, syslog.

---

## Versioning policy

- The standard library is versioned **separately** from the compiler.
- Each compiler version pins a default `std` version, but the user can
  override via `mom.toml`.
- Breaking changes in `std` only happen at language editions (currently
  `2026`).

## Maintenance cadence

- Security fixes within 7 days.
- New `std` features piggy-back on the compiler release cycle (every
  6 weeks, target).
- Each public symbol carries `since` and `stable` markers in docs.

## What is *not* in the standard library

- Database drivers (live in the registry).
- GUI toolkits (live in the registry).
- Game engines (live in the registry).
- Heavy ML frameworks (mom's role here is the runtime, not the model
  zoo).

This is deliberate. The standard library is the part the compiler team
maintains forever; community libraries iterate faster.
