# Concurrency Overview

Mom's concurrency model is **three deliberate layers** stacked on top of OS threads. Each layer can be disabled independently — kernel and embedded builds omit actors and the global scheduler; single-threaded builds omit preemption entirely.

```
┌─────────────────────────────────────────────────┐
│  Supervision trees  ←  fault tolerance           │
├─────────────────────────────────────────────────┤
│  Actors             ←  state isolation           │
├─────────────────────────────────────────────────┤
│  async tasks        ←  cooperative multitasking  │
├─────────────────────────────────────────────────┤
│  OS threads         ←  parallelism               │
└─────────────────────────────────────────────────┘
```

---

## The Concurrency Stack

| Layer | Primitive | Purpose |
|---|---|---|
| OS threads | native `pthread` / Win32 threads | true parallelism across cores |
| Async tasks | `async fn`, `await`, `spawn` | lightweight, non-blocking IO concurrency |
| Actors | `actor … receive` | isolated stateful workers, message-passing |
| Supervision | `supervise … restart` | fault tolerance, automatic restart |

You do not need all four layers in every program. A CLI tool might use only `async fn`. A game server might use channels and `spawn` without actor sugar. A distributed system uses the full stack.

---

## `async fn` — Declaring Async Functions

Prefix any function with `async` to make it return a `Future[T]`. The body may contain `await` expressions.

```mom
async fn fetch(url: String) -> Result[String, String]:
    if url == "" { Err("empty url") } else { Ok("body of " + url) }

async fn process(url: String) -> Result[Int, String]:
    let body = await fetch(url)?
    Ok(len(body))
```

- The return type is automatically wrapped: `async fn f() -> T` produces a `Future[T]`.
- `async fn` bodies run eagerly up to the first `await` suspension point.
- Tasks are **stackful** by default on hosted targets. The optimizer can lower to stackless coroutines when escape analysis proves no captured stack frames outlive the await point.

---

## `await expr` — Suspending Until a Future Completes

`await` unwraps a `Future[T]` (or `Task[T]`) into `T`, suspending the current task until the value is ready.

```mom
fn main():
    let result = await process("https://mom-lang.org")
    match result:
        Ok(n)  => print(n)
        Err(e) => print(e)
```

Rules:
- `await` can appear in any `async fn` body, or at the top level in `main`.
- `await` composes with `?`: `await expr?` propagates errors from `Result`-returning futures.
- Awaiting a non-future value is a compile error.

---

## `spawn expr` — Launching Background Tasks

`spawn` schedules an async expression onto the executor and immediately returns a `Task[T]` handle.

```mom
let task: Task[String] = spawn fetch("https://mom-lang.org")
// do other work …
let body = await task      // join: blocks until task finishes
```

The `Task[T]` handle supports:

| Operation | Description |
|---|---|
| `await task` | Block until task completes, return `T` |
| `Task::cancel(task)` | Signal cooperative cancellation |
| detach (drop handle) | Task runs to completion without joining |

### Parallel Fan-Out

```mom
async fn join_all_int(xs: [Int]) -> Int:
    let mut total = 0
    let mut i = 0
    let n = len(xs)
    while i < n:
        total = total + (await compute(xs[i]))
        i = i + 1
    total

fn main():
    let inputs = [2, 3, 4]   // squares: 4 + 9 + 16 = 29
    let total = await join_all_int(inputs)
    print(total)              // 29
```

---

## The `Cancel` Token — Cooperative Cancellation

`Cancel()` creates a lightweight cancellation token. Pass it into long-running tasks; they poll `is_cancelled()` at safe checkpoints and exit cleanly.

```mom
fn work_until(token: Cancel, max: Int) -> Int:
    let mut i = 0
    while !token.is_cancelled():
        i = i + 1
        if i >= max:
            token.signal()
    i

fn main():
    let token = Cancel()
    print(work_until(token, 5))   // 5
    print(token.is_cancelled())   // true
```

### Cancel API

| Method | Description |
|---|---|
| `Cancel()` | Create a new cancellation token |
| `.signal()` | Mark the token as cancelled |
| `.is_cancelled() -> Bool` | Check whether cancellation has been requested |

`Task::cancel(t)` signals a task's token. Resources held by the task are dropped during stack unwinding after cancellation.

```mom
async fn long_running(ctx: Cancel) -> Result[String, String]:
    while !ctx.is_cancelled():
        do_work()?
        await sleep(100)
    Err("aborted")
```

---

## `sleep(ms)` — Async Sleep

Suspend the current task for at least `ms` milliseconds.

```mom
await sleep(2000)                          // sleep 2 seconds
await timeout(5000, fetch(url))?           // error if not done in 5 s
```

> **Bootstrap note:** `sleep(ms)` is accepted by the interpreter but is a no-op — it returns immediately. The real timer fires in Phase 3.1 on the native work-stealing executor.

---

## Phase Roadmap — What Works Today

| Primitive | Status | Notes |
|---|---|---|
| `Channel(cap?)` | **shipped** | `.send()`, `.recv()`, `.try_recv()`, `.len()`, `.is_empty()`, `.capacity()`, `.close()` |
| `Cancel()` | **shipped** | `.signal()`, `.is_cancelled()` |
| `spawn EXPR` | **shipped** | returns `Task[T]` |
| `await EXPR` | **shipped** | unwraps `Task[T]` → `T`; bodies run synchronously in bootstrap |
| `sleep(ms)` | **shipped (no-op)** | real timer arrives in Phase 3.1 |
| `async fn` | **shipped** | syntax parsed; bodies run synchronously |
| `actor … receive` | **Phase 3.1** | desugars to `struct` + `impl … fn step(self, msg) -> Self` |
| `supervise …` | **Phase 3.2** | runtime restart driver |
| Broadcast / oneshot channels | **Phase 3.2** | API extensions |
| Native work-stealing executor | **Phase 3.1** | multi-threaded, real async IO |

The runnable demos today: `examples/channels.mom`, `examples/cancel.mom`, `examples/actor_via_channels.mom`, `examples/async_sketch.mom`.

---

## Comparison: Mom vs Other Languages

| Feature | Mom | Go | Erlang/OTP | Rust (Tokio) |
|---|---|---|---|---|
| Concurrency primitive | `async fn` + `spawn` | goroutines | processes | `async fn` + `tokio::spawn` |
| Message passing | channels + actors | channels | process mailboxes | channels (crossbeam / mpsc) |
| State isolation | actor `state` keyword enforced by compiler | manual | process heap isolation | `Arc<Mutex<T>>` by convention |
| Supervision | `supervise` trees (Phase 3.2) | manual | OTP supervisors | manual / `tokio` tasks |
| Cancellation | `Cancel` token (cooperative) | `context.Context` | process exit signals | `CancellationToken` |
| Error model | `Result[T, E]` + `?` | multiple return values | "let it crash" | `Result<T, E>` + `?` |
| Backpressure | bounded channels | buffered channels | process message queue | bounded channels |
| Shared mutable state | forbidden inside actors | allowed (with discipline) | forbidden | allowed via `Mutex`/`RwLock` |
| Runtime | opt-in, layered | always-on | always-on BEAM | always-on Tokio |

---

## When to Use Async vs Channels vs Actors

| Situation | Recommended primitive |
|---|---|
| Concurrent IO (HTTP, files, timers) | `async fn` + `await` |
| Pipeline between two tasks | `Channel` |
| Fan-out to many workers | `spawn` + `Channel` |
| Long-lived stateful service | `actor` |
| Fault-tolerant subsystem | `actor` + `supervise` |
| Short-lived parallel computation | `spawn` + `await` |
| Backpressure control | bounded `Channel(capacity)` |

**Rule of thumb:** reach for `async fn` first. Add channels when you need to decouple producers and consumers. Reach for actors when state isolation and fault recovery matter more than raw throughput.

---

## Full Worked Example — Async Fetch Pipeline

```mom
// Fetch a URL, process the body, print the length.

async fn fetch(url: String) -> Result[String, String]:
    if url == "" { Err("empty url") } else { Ok("body of " + url) }

async fn process(url: String) -> Result[Int, String]:
    let body = await fetch(url)?
    Ok(len(body))

fn main():
    let result = await process("https://mom-lang.org")
    match result:
        Ok(n)  => print(n)
        Err(e) => print(e)
```

## Full Worked Example — Parallel Computation

```mom
async fn compute(x: Int) -> Int:
    x * x

async fn sum_of_squares(xs: [Int]) -> Int:
    let mut total = 0
    let mut i = 0
    let n = len(xs)
    while i < n:
        total = total + (await compute(xs[i]))
        i = i + 1
    total

fn main():
    let result = await sum_of_squares([2, 3, 4])
    print(result)   // 29
```

## Full Worked Example — Cooperative Cancellation

```mom
fn work_until(token: Cancel, max: Int) -> Int:
    let mut i = 0
    while !token.is_cancelled():
        i = i + 1
        if i >= max:
            token.signal()
    i

fn main():
    let token = Cancel()
    let count = work_until(token, 5)
    print(count)                  // 5
    print(token.is_cancelled())   // true
```

---

## See Also

- [Channels](channels.md) — typed FIFO queues, bounded/unbounded, producer-consumer patterns
- [Actors](actors.md) — isolated state machines, supervision trees
