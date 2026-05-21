# mom — Concurrency, Actors, and Supervision

mom's concurrency story is **three layers stacked deliberately**:

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

Each layer can be turned off. Kernel and embedded builds disable
actors and the global async scheduler. Single-threaded builds
disable preemption.

---

## 1. async / await

```mom
async fn fetch(url: String) -> Result[Body, HttpError] {
    let conn = await http.connect(url)?
    let body = await conn.read_all()?
    Ok(body)
}

fn main() {
    let body = await fetch("https://mom-lang.org")
    print(body)
}
```

- `async fn` returns a `Future[T]`.
- `await` suspends the current task until the future completes.
- Tasks are **stackful or stackless** — the runtime chooses based on
  target. Stackful by default on hosted targets for ergonomics; the
  optimizer falls back to stackless coroutines when escape analysis
  proves no captured stack frames.

## 2. spawn

```mom
let task: Task[Body] = spawn fetch(url)
…
let body = await task
```

`spawn` schedules a future onto the executor. The returned `Task[T]`
can be awaited, cancelled, joined, or detached.

## 3. Channels

```mom
let (tx, rx) = channel.bounded[Message](capacity: 64)

spawn worker(rx)

for msg in incoming {
    tx.send(msg)?
}
```

Channel kinds:

- `channel.bounded[T](capacity)` — backpressure-friendly.
- `channel.unbounded[T]()` — unlimited (use only with care).
- `channel.broadcast[T](capacity)` — multi-consumer.
- `channel.oneshot[T]()` — request/reply.

Channels do **not** require `Send`/`Sync` markers — the compiler
computes data-race safety from layout. Trying to send a non-shareable
type is a compile error.

## 4. Actors

```mom
actor Cache {
    state map: Map[String, Bytes]

    receive {
        Get(key, reply) => reply.send(map.get(key)),
        Put(key, val)   => map.insert(key, val),
        Clear           => map.clear(),
    }
}

let cache: ActorRef[Cache] = spawn Cache::new(map: Map::new())

cache <- Put("hello", b"world")
let value = await cache.ask(Get("hello"))
```

Actor invariants enforced by the compiler:

1. **State is isolated** — an actor's `state` is reachable only from
   inside its `receive` block. No external references.
2. **Messages cross by move** — the sender loses ownership of any
   non-`Copy` payload.
3. **Single-threaded inside** — only one message is processed at a time
   per actor; no internal locking is needed.
4. **Mailboxes are bounded** by default. Overflow is observable, not
   crashing.

## 5. Supervision

```mom
let policy = restart(limit: 3, window: 60.seconds, strategy: OneForOne)

supervise cache with policy

supervise group {
    spawn metrics_pump()       with restart(limit: 5)
    spawn request_handler(rx)  with restart(limit: 10)
    spawn db_pool(config)      with permanent
}
```

Strategies (from Erlang/OTP, adapted):

- `OneForOne` — restart only the failing child.
- `OneForAll` — restart every sibling when one fails.
- `RestForOne` — restart the failing child and every later sibling.
- `permanent` / `transient` / `temporary` — lifetime policy.

When a supervisor exceeds its restart budget, **it fails up** to its
own supervisor. This is the "let it crash" discipline made explicit.

## 6. Cancellation

Tasks have a **cooperative cancellation token**:

```mom
async fn long_running(ctx: Cancel) -> Result[Output, AbortError] {
    while !ctx.is_cancelled() {
        do_work()?
        await sleep(100.ms)
    }
    Err(Aborted)
}
```

`Task::cancel(t)` signals the token. Resources held by the task are
dropped during stack unwinding.

## 7. Timers, sleep, deadlines

```mom
await sleep(2.seconds)
await timeout(5.seconds, fetch(url))?      // returns Err(TimedOut) if exceeded
let deadline = now() + 100.ms
await deadline.race(fetch(url))?
```

## 8. Parallelism vs concurrency

- **async** = concurrency on a single core (no parallelism).
- **threads** = OS-thread parallelism for CPU-bound work
  (`thread.spawn { … }`).
- **work-stealing executor** is the default for `spawn`, giving both.

```mom
let result = parallel.map(items, fn(x) => expensive(x))
```

## 9. Runtime modularity

The async + actor runtime is one library, **not a kernel feature**.

```toml
# mom.toml
[features]
default      = ["std", "async", "actors"]
embedded     = ["std-core"]                     # no async, no actors
kernel       = ["std-core", "panic-abort"]      # no allocation runtime
```

This is why mom is suitable for embedded firmware, kernels, and
freestanding targets without abandoning the high-level concurrency story
on hosted builds.

## 10. Comparison

| System      | mom                  | Erlang        | Go         | Rust            |
|-------------|----------------------|---------------|------------|-----------------|
| Concurrency | actors + async       | actors        | goroutines | async + threads |
| Mailbox     | bounded, typed       | unbounded, dynamic | channels | mpsc / broadcast |
| Supervision | first-class          | first-class   | none       | community crates |
| Memory share| compile-checked      | none (copy)   | runtime races possible | borrow-checked |
| Fault model | let-it-crash + types | let-it-crash  | panics + recover | Result + panic |

---

## 11. Phase-3 status (what works today in the bootstrap interpreter)

The single-threaded bootstrap runtime ships these primitives. The
native work-stealing executor and dedicated actor sugar arrive in
sub-phases 3.1 / 3.2.

| Primitive             | Status  | API                                         |
|-----------------------|---------|---------------------------------------------|
| `Channel(cap?)`       | shipped | `.send(v)`, `.recv() -> Option`, `.try_recv()`, `.len()`, `.is_empty()`, `.capacity()`, `.close()` |
| `Cancel()`            | shipped | `.signal()`, `.is_cancelled()`              |
| `spawn EXPR`          | shipped | returns `Task[T]`                            |
| `await EXPR`          | shipped | unwraps `Task[T]` → `T`                      |
| `sleep(ms: Int)`      | shipped | no-op in the bootstrap; real timer in 3.1   |
| `actor … receive { }` | shipped (3.1) | desugars to `struct + impl Name { fn step(self, msg) -> Self { match msg { … } } }` — see `examples/actor.mom` |
| `supervise …`         | sub-phase 3.2 | runtime restart driver                    |
| Broadcast / oneshot   | sub-phase 3.2 | API extensions                             |

The runnable demos are `examples/channels.mom`, `examples/cancel.mom`,
`examples/actor_via_channels.mom`.
