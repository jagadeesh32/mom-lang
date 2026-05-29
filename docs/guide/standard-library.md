# Standard Library

Mom ships a standard library implemented as plain `.mom` source files under `std/`. All modules follow a **stage-0** design: the surface API is final; only the underlying implementation will be swapped when the native stage-2 compiler lands. Code written against the stage-0 API recompiles unchanged on the native backend.

---

## Layered design

```
std::core   – primitives, sum-type helpers, numeric utilities     (no allocator)
std::alloc  – Box, Rc, Arc, region demo                           (allocator)
std::io     – LineBuffer, buffered output                         (alloc)
std::fmt    – padding, joining, formatting                        (alloc)
std::log    – leveled logging                                     (alloc)
std::async  – Task wrappers, cooperative scheduler surface        (runtime)
std::actor  – channel-driven mailbox actors                       (runtime)
std::net    – Address, Request, Response, dispatcher              (runtime)
std::sync   – Mutex, Atomic, Once (single-thread stub today)      (threading)
std::os     – env, sleep, process info                            (host OS)
std::math   – integer math, Lehmer LCG RNG                        (alloc)
std::serde  – JSON-ish encoders                                   (alloc)
std::crypto – checksums, hex encoding                             (alloc)
std::test   – assertion helpers, TestStats accumulator            (alloc)
```

Every layer above `std::core` is **opt-in** via the manifest:

```toml
[dependencies.std]
features = ["alloc", "io", "async", "net"]
```

Embedded and bare-metal targets use `features = ["core"]` only.

---

## Importing modules

```mom
use std::fmt
use std::math
use std::test
```

After importing, call functions unqualified:

```mom
use std::fmt

let s = join(["a", "b", "c"], ", ")   // "a, b, c"
```

---

### `std::core`

The always-imported prelude. Every Mom program has `std::core` available without an explicit `use`. The native stage-2 compiler promotes these to language-level intrinsics where the optimizer benefits.

**Import:** automatic — no `use` required.

| Function | Signature | Description |
|---|---|---|
| `identity` | `[T](value: T) -> T` | Returns its argument unchanged. Useful as a no-op function value. |
| `min` | `(a: Int, b: Int) -> Int` | Smaller of two integers. |
| `max` | `(a: Int, b: Int) -> Int` | Larger of two integers. |
| `clamp` | `(value: Int, lo: Int, hi: Int) -> Int` | Clamps `value` into `[lo, hi]`. |
| `abs` | `(value: Int) -> Int` | Absolute value. |
| `sign` | `(value: Int) -> Int` | Returns `-1`, `0`, or `1`. |
| `option_or` | `(value: Option[Int], fallback: Int) -> Int` | Unwraps `Some(v)` or returns `fallback`. |
| `option_is_some` | `(value: Option[Int]) -> Bool` | Tests whether an `Option` is `Some`. |
| `result_or` | `(value: Result[Int, String], fallback: Int) -> Int` | Unwraps `Ok(v)` or returns `fallback`. |
| `result_map_int` | `(value: Result[Int, String], f: fn(Int) -> Int) -> Result[Int, String]` | Maps over the `Ok` branch. |

**Examples:**

```mom
print(clamp(150, 0, 100))           // 100
print(abs(-7))                      // 7
print(sign(-42))                    // -1
print(option_or(Some(5), 99))       // 5
print(option_or(None, 99))          // 99
print(result_or(Ok(42), 0))         // 42
print(result_or(Err("oops"), 0))    // 0
```

---

### `std::fmt`

String formatting helpers: padding, joining, repeating, and key/value rendering.

**Import:** `use std::fmt`

| Function | Signature | Description |
|---|---|---|
| `repeat` | `(s: String, times: Int) -> String` | Concatenates `s` with itself `times` times. |
| `pad_left` | `(s: String, width: Int, fill: String) -> String` | Right-justifies `s` in a field of `width`, padding with `fill`. |
| `pad_right` | `(s: String, width: Int, fill: String) -> String` | Left-justifies `s`, padding with `fill` on the right. |
| `join` | `(items: [String], sep: String) -> String` | Joins a string list with a separator. |
| `join_ints` | `(items: [Int], sep: String) -> String` | Joins an integer list, converting each to a string. |
| `key_value` | `(key: String, value: String) -> String` | Returns `"key: value"` formatted line. |

**Examples:**

```mom
use std::fmt

print(repeat("-", 5))                        // -----
print(pad_left("42", 6, " "))               //     42
print(pad_right("ok", 6, "."))              // ok....
print(join(["alpha", "beta", "gamma"], ", "))  // alpha, beta, gamma
print(join_ints([1, 2, 3], " | "))          // 1 | 2 | 3
print(key_value("host", "localhost"))        // host: localhost
```

---

### `std::math`

Integer mathematics and a deterministic pseudo-random number generator.

**Import:** `use std::math`

| Function / Type | Signature | Description |
|---|---|---|
| `gcd` | `(a: Int, b: Int) -> Int` | Greatest common divisor (Euclidean). |
| `lcm` | `(a: Int, b: Int) -> Int` | Least common multiple. |
| `pow_int` | `(base: Int, exponent: Int) -> Int` | Integer exponentiation by squaring. |
| `factorial` | `(n: Int) -> Int` | `n!` for non-negative `n`. |
| `fib` | `(n: Int) -> Int` | n-th Fibonacci number (0-indexed, iterative). |
| `rng_seeded` | `(seed: Int) -> Rng` | Creates a new `Rng` from a seed (`<= 0` normalises to 1). |
| `Rng.next` | `(self) -> Rng` | Advances the generator; returns a new `Rng`. |
| `Rng.value` | `(self) -> Int` | Raw state value of the current generator. |
| `Rng.range` | `(self, modulus: Int) -> Int` | Value in `[0, modulus)`. |

`Rng` is a **Lehmer LCG** (MINSTD parameters: multiplier 48271, modulus 2147483647). It is deterministic and suitable for simulations and tests; do not use it for cryptographic purposes.

**Examples:**

```mom
use std::math

print(gcd(54, 24))       // 6
print(lcm(4, 6))         // 12
print(pow_int(2, 10))    // 1024
print(factorial(6))      // 720
print(fib(10))           // 55

let mut r = rng_seeded(42)
r = r.next()
print(r.value())         // deterministic first value
print(r.range(100))      // value in [0, 100)
```

---

### `std::io`

Buffered output via `LineBuffer`. The stage-0 bootstrap only has `print` as a sink; `LineBuffer` accumulates lines into a single `String` so output is composed deterministically. The native stage-2 routes `flush` to the real stdout writer.

**Import:** `use std::io`

| Function / Type | Signature | Description |
|---|---|---|
| `empty_buffer` | `() -> LineBuffer` | Creates a new empty buffer. |
| `LineBuffer.write` | `(self, chunk: String) -> LineBuffer` | Appends a raw string (no newline). |
| `LineBuffer.writeln` | `(self, line: String) -> LineBuffer` | Appends a string followed by `\n`. |
| `LineBuffer.flush` | `(self) -> String` | Returns the accumulated buffer contents. |
| `newline` | `() -> String` | Returns `"\n"`. |

**Example:**

```mom
use std::io

let mut buf = empty_buffer()
buf = buf.writeln("alpha")
buf = buf.writeln("beta")
buf = buf.write("gamma")
buf = buf.write(newline())
print(buf.flush())
// alpha
// beta
// gamma
```

---

### `std::log`

Leveled, structured logging. Each log line is stamped with its level prefix. Below-threshold messages are silently dropped.

**Import:** `use std::log`

**`Level` enum:**

```mom
enum Level:
    Trace    // rank 0
    Debug    // rank 1
    Info     // rank 2
    Warn     // rank 3
    Error    // rank 4
```

| Function / Type | Signature | Description |
|---|---|---|
| `logger_for` | `(min: Level) -> Logger` | Creates a logger that emits messages at or above `min`. |
| `Logger.at` | `(self, level: Level, message: String) -> Logger` | Emits the message if `level >= min_rank`; returns the same logger. |
| `level_rank` | `(level: Level) -> Int` | Numeric rank of a level (0–4). |
| `level_label` | `(level: Level) -> String` | Short label: `"TRACE"`, `"DEBUG"`, `"INFO"`, `"WARN"`, `"ERROR"`. |

**Example:**

```mom
use std::log

let mut log = logger_for(Info)
log = log.at(Trace, "ignored")        // below threshold, no output
log = log.at(Debug, "ignored")        // below threshold, no output
log = log.at(Info,  "starting up")    // INFO  starting up
log = log.at(Warn,  "watch out")      // WARN  watch out
log = log.at(Error, "exploded")       // ERROR exploded
```

Because `Logger` is a struct, re-assignment (`log = log.at(...)`) is required to chain calls.

---

### `std::async`

Cooperative async surface. The bootstrap interpreter runs `async` bodies synchronously and treats `await` as a typed no-op. The surface stays stable for the multi-thread scheduler in stage-2.

**Import:** `use std::async`

| Function | Signature | Description |
|---|---|---|
| `compute` | `async (x: Int) -> Int` | Squares `x` asynchronously. Example task primitive. |
| `join_all_int` | `async (xs: [Int]) -> Int` | Awaits `compute` for each element and sums results. |
| `yield_now` | `() -> Bool` | Cooperative yield point. No-op in stage-0; always returns `true`. |

**Example:**

```mom
use std::async

let inputs = [2, 3, 4]            // 4 + 9 + 16 = 29
let total = await join_all_int(inputs)
print(total)     // 29
print(yield_now())   // true
```

---

### `std::actor`

Channel-driven mailbox actors. The canonical pattern for stateful concurrent components. The native stage-2 generalises this with `actor` syntax and supervised restart-on-failure semantics.

**Import:** `use std::actor`

**`CounterMsg` enum:**

```mom
enum CounterMsg:
    Inc
    Add(Int)
    Get
    Stop
```

| Function | Signature | Description |
|---|---|---|
| `run_counter` | `(mailbox: Channel[CounterMsg]) -> Int` | Drains messages from `mailbox` until `Stop` or `None`; returns final count. |

**Example:**

```mom
use std::actor

let mailbox = Channel(16)
mailbox.send(Inc)
mailbox.send(Inc)
mailbox.send(Add(40))
mailbox.send(Get)      // prints 42
mailbox.send(Stop)
let final_count = run_counter(mailbox)
print(final_count)     // 42
```

---

### `std::net`

HTTP-ish surface types and a route dispatcher. The native stage-2 wires `bind` + `accept` to the real socket layer; today the types are stable stubs.

**Import:** `use std::net`

| Function / Type | Signature | Description |
|---|---|---|
| `address` | `(host: String, port: Int) -> Address` | Constructs an `Address`. |
| `pretty` | `(addr: Address) -> String` | Formats as `"host:port"`. |
| `ok` | `(body: String) -> Response` | HTTP 200 response. |
| `not_found` | `() -> Response` | HTTP 404 response. |
| `dispatch` | `(req: Request) -> Response` | Routes `req.path` to a handler; returns `not_found()` for unknown paths. |
| `Address` | `struct { host: String, port: Int }` | Network address. |
| `Request` | `struct { method: String, path: String }` | Incoming request. |
| `Response` | `struct { status: Int, body: String }` | Outgoing response. |

**Example:**

```mom
use std::net

let addr = address("127.0.0.1", 8080)
print(pretty(addr))   // 127.0.0.1:8080

let r = dispatch(Request { method: "GET", path: "/health" })
print(r.status)   // 200
print(r.body)     // ok
```

---

### `std::serde`

Pocket JSON-ish encoder. Decoding is deferred to stage-2 once the native lexer is reachable from `.mom` code.

**Import:** `use std::serde`

| Function | Signature | Description |
|---|---|---|
| `encode_bool` | `(value: Bool) -> String` | `"true"` or `"false"`. |
| `encode_int` | `(value: Int) -> String` | Decimal string representation. |
| `encode_string` | `(value: String) -> String` | JSON-quoted string: `"\"hello\""`. |
| `encode_int_list` | `(values: [Int]) -> String` | JSON array of integers: `"[1,2,3]"`. |
| `encode_string_list` | `(values: [String]) -> String` | JSON array of strings. |
| `encode_kv` | `(key: String, value: String) -> String` | `"key":value` pair (value already encoded). |

**Example:**

```mom
use std::serde

print(encode_bool(true))                       // true
print(encode_int(-7))                          // -7
print(encode_string("hello"))                  // "hello"
print(encode_int_list([1, 2, 3]))              // [1,2,3]
print(encode_kv("name", encode_string("mom"))) // "name":"mom"
```

---

### `std::crypto`

Checksums and hex encoding. Stage-0 uses only `+`, `*`, `/`, and `%` (no bitwise operators); the full cryptographic suite (FNV-1a, SipHash, SHA-256) arrives with the native stage-2 bitwise operators.

**Import:** `use std::crypto`

| Function | Signature | Description |
|---|---|---|
| `adler32` | `(bytes: [Int]) -> Int` | Mark Adler's 1995 checksum of a byte array. |
| `poly_hash` | `(bytes: [Int]) -> Int` | Polynomial rolling hash (prime 131, modulus 1e9+7). |
| `hex_byte` | `(byte: Int) -> String` | 2-character lowercase hex of a 0–255 byte. |
| `hex_int` | `(value: Int) -> String` | 8-character zero-padded lowercase hex of a 32-bit integer. |

**Example:**

```mom
use std::crypto

// "abc" → [97, 98, 99]; Adler-32 is 0x024d0127 = 38600999
print(adler32([97, 98, 99]))   // 38600999
print(adler32([]))             // 1  (empty input)
print(hex_byte(255))           // ff
print(hex_byte(171))           // ab
print(hex_int(305419896))      // 12345678
```

---

### `std::sync`

Mutual-exclusion primitives. Stage-0 is a single-threaded stub: `Mutex.lock` counts acquisitions but never blocks; `Atomic` operations are sequentially consistent no-ops; `Once.call` runs only the first call. The surface is identical to what stage-2 will enforce under real concurrent load.

**Import:** `use std::sync`

| Function / Type | Signature | Description |
|---|---|---|
| `new_mutex` | `(initial: Int) -> Mutex` | Creates a `Mutex` holding `initial`. |
| `Mutex.lock` | `(self) -> Mutex` | Acquires the lock (increments `locks` counter in stage-0). |
| `Mutex.set` | `(self, value: Int) -> Mutex` | Updates the protected value. |
| `Mutex.get` | `(self) -> Int` | Reads the protected value. |
| `new_atomic` | `(initial: Int) -> Atomic` | Creates an `Atomic` integer. |
| `Atomic.swap` | `(self, value: Int) -> Atomic` | Atomically replaces the value. |
| `Atomic.fetch_add` | `(self, delta: Int) -> Atomic` | Atomically adds `delta`. |
| `Atomic.get` | `(self) -> Int` | Reads the current value. |
| `new_once` | `() -> Once` | Creates a `Once` cell. |
| `Once.call` | `(self, value: Int) -> Once` | Sets the value on the first call; subsequent calls are ignored. |

**Example:**

```mom
use std::sync

let mut m = new_mutex(10)
m = m.lock()
m = m.set(42)
print(m.get())     // 42
print(m.locks)     // 1

let mut a = new_atomic(7)
a = a.fetch_add(3)
a = a.swap(100)
print(a.get())     // 100

let mut o = new_once()
o = o.call(5)
o = o.call(99)    // ignored
print(o.value)     // 5
```

---

### `std::os`

Operating-system wrappers. Stage-0 uses the interpreter's host shims; the native stage-2 replaces them with real `libc` / `clock_gettime` calls.

**Import:** `use std::os`

| Function / Type | Signature | Description |
|---|---|---|
| `env_or` | `(name: String, fallback: String) -> String` | Returns env var `name`, or `fallback` if unset. |
| `sleep_ms` | `(duration_ms: Int) -> Int` | Sleeps for `duration_ms` milliseconds; returns `duration_ms`. No-op in stage-0. |
| `current_process` | `() -> ProcessInfo` | Returns a `ProcessInfo` with `pid` and `threads` fields. |
| `ProcessInfo` | `struct { pid: Int, threads: Int }` | Snapshot of the current process. |

**Example:**

```mom
use std::os

let path = env_or("PATH", "<unset>")
print(len(path) > 0)    // true

print(sleep_ms(0))      // 0

let info = current_process()
print(info.pid)         // 1  (stage-0 constant)
print(info.threads)     // 1
```

---

### `std::alloc`

Heap allocation primitives: `Box` (unique ownership), `Rc` (reference-counted shared), `Arc` (atomic reference-counted shared). In the stage-0 interpreter all three share the same cell; the native stage-2 distinguishes them at the type-system level. Region blocks are also demonstrated here.

**Import:** `use std::alloc`

| Construct | Description |
|---|---|
| `Box(value)` | Unique heap allocation. Prints its interior value. |
| `Rc(value)` | Shared heap allocation (single-threaded). |
| `Arc(value)` | Shared heap allocation (thread-safe in stage-2). |
| `region r { … }` | Scoped allocation region; values are freed at the closing brace. |

**Example:**

```mom
use std::alloc

let boxed  = Box(99)
let shared = Rc(7)
let atomic = Arc(true)
print(boxed)    // 99
print(shared)   // 7
print(atomic)   // true

let label = region r:
    "request:GET /healthz"
print(label)    // request:GET /healthz
```

---

### `std::test`

In-program assertion helpers. The `mom test` driver discovers test files; this module supplies the assertion vocabulary used inside those files. See the [Testing guide](testing.md) for the full workflow.

**Import:** `use std::test`

| Function / Type | Signature | Description |
|---|---|---|
| `new_stats` | `() -> TestStats` | Creates a zeroed `TestStats` accumulator. |
| `TestStats.pass` | `(self) -> TestStats` | Increments the passed counter. |
| `TestStats.fail` | `(self) -> TestStats` | Increments the failed counter. |
| `TestStats.summary` | `(self) -> String` | Returns `"passed=N failed=M"`. |
| `assert_eq_int` | `(stats: TestStats, actual: Int, expected: Int, label: String) -> TestStats` | Passes if `actual == expected`; prints `ok` or `FAIL`. |
| `assert_true` | `(stats: TestStats, condition: Bool, label: String) -> TestStats` | Passes if `condition` is `true`. |
| `assert_false` | `(stats: TestStats, condition: Bool, label: String) -> TestStats` | Passes if `condition` is `false`. |

**Example:**

```mom
use std::test

let mut stats = new_stats()
stats = assert_eq_int(stats, 2 + 2, 4, "addition")
stats = assert_true(stats, len([1, 2, 3]) > 0, "non-empty list")
stats = assert_false(stats, false, "false is false")
print(stats.summary())   // passed=3 failed=0
```

---

## Stage status summary

| Module | Stage-0 file | Status | Notes |
|---|---|---|---|
| `std::core` | `std/core.mom` | Full | Promoted to intrinsics in stage-2 |
| `std::fmt` | `std/fmt.mom` | Full | `Display` trait in stage-2 |
| `std::math` | `std/math.mom` | Partial | Float ops and CSPRNG deferred to stage-2 |
| `std::io` | `std/io.mom` | Stub | Real stdout in stage-2 |
| `std::log` | `std/log.mom` | Full | Structured fields in stage-2 |
| `std::async` | `std/async.mom` | Stub | Single-thread; real scheduler in stage-2 |
| `std::actor` | `std/actor.mom` | Full | `actor` sugar in stage-2 |
| `std::net` | `std/net.mom` | Stub | Real sockets in stage-2 |
| `std::serde` | `std/serde.mom` | Partial | Decode deferred to stage-2 |
| `std::crypto` | `std/crypto.mom` | Partial | Bitwise ops (FNV, SHA-256) deferred |
| `std::sync` | `std/sync.mom` | Stub | Single-thread; real locks in stage-2 |
| `std::os` | `std/os.mom` | Stub | Real syscalls in stage-2 |
| `std::alloc` | `std/alloc.mom` | Partial | Type-level distinction in stage-2 |
| `std::test` | `std/test.mom` | Full | `#[prop]` in stage-2 |
