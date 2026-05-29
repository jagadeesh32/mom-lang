# Channels

A **channel** is a typed, thread-safe FIFO queue. Channels are the primary mechanism for passing values between concurrent tasks in Mom. They decouple senders from receivers, support optional backpressure, and form the implementation substrate that the `actor` keyword desugars into.

---

## Creating a Channel

### Bounded Channel

```mom
let ch = Channel(4)   // accepts at most 4 items before blocking
```

A bounded channel applies **backpressure**: `.send()` blocks when the queue is full, slowing producers to match consumers. Use bounded channels whenever you want explicit flow control.

### Unbounded Channel

```mom
let ch = Channel()    // no capacity limit
```

An unbounded channel never blocks on send. Use sparingly — an unbounded channel between a fast producer and a slow consumer can grow without limit.

---

## Channel API Reference

| Method | Signature | Description |
|---|---|---|
| `Channel(cap)` | `(Int) -> Channel[T]` | Create a bounded channel with the given capacity |
| `Channel()` | `() -> Channel[T]` | Create an unbounded channel |
| `.send(value)` | `(T) -> ()` | Send a value; blocks if bounded and full |
| `.recv()` | `() -> Option[T]` | Receive the next value; blocks until available; returns `None` if closed and empty |
| `.try_recv()` | `() -> Option[T]` | Non-blocking receive; returns `None` immediately if no value is ready |
| `.len()` | `() -> Int` | Current number of items in the queue |
| `.is_empty()` | `() -> Bool` | `true` if the queue has no items |
| `.capacity()` | `() -> Int` | Maximum capacity (bounded channels only) |
| `.close()` | `() -> ()` | Signal that no more values will be sent |

---

## Sending Values

`.send(value)` puts a value into the channel.

```mom
let ch = Channel(4)

ch.send(10)
ch.send(20)
ch.send(30)

print(ch.len())   // 3
```

For a bounded channel, `.send()` **blocks** if the queue is at capacity. This is intentional: it creates cooperative backpressure without explicit coordination.

---

## Receiving Values

`.recv()` returns `Option[T]`. It **blocks** until a value is available, or returns `None` if the channel is closed and empty.

```mom
match ch.recv():
    Some(v) => print(v)
    None    => print("channel closed")
```

### Non-Blocking Receive

`.try_recv()` returns immediately: `Some(v)` if a value was waiting, `None` if the queue was empty.

```mom
match ch.try_recv():
    Some(v) => print(v)
    None    => print("nothing yet")
```

Use `.try_recv()` in polling loops or when you want to do other work while waiting.

---

## Closing a Channel

`.close()` signals that no further values will be sent. Pending receivers drain the remaining items, then get `None`.

```mom
ch.send(42)
ch.close()

match ch.recv():   // Some(42)
    Some(v) => print(v)
    None    => print("empty")

match ch.recv():   // None — channel is closed and empty
    Some(v) => print(v)
    None    => print("closed")
```

> **Convention:** close from the producer side, drain from the consumer side. Closing a channel you do not own is a logic error.

---

## Full Example — Basic Send/Recv

This is the canonical `examples/channels.mom`:

```mom
fn main():
    let ch = Channel(4)

    ch.send(10)
    ch.send(20)
    ch.send(30)

    print(ch.len())     // 3

    match ch.recv():
        Some(v) => print(v)   // 10
        None    => print(-1)
    match ch.recv():
        Some(v) => print(v)   // 20
        None    => print(-1)
    match ch.recv():
        Some(v) => print(v)   // 30
        None    => print(-1)
    match ch.recv():
        Some(v) => print(v)   // -1 (channel empty, would block — shown here for illustration)
        None    => print(-1)
```

---

## Producer-Consumer Pattern

The classic pattern: one or more producers send into a shared channel; one consumer drains it.

```mom
enum Job:
    Work(Int)
    Stop

fn consumer(jobs: Channel[Job]):
    let mut running = true
    while running:
        match jobs.recv():
            Some(Work(n)) => print(n * n)   // process the job
            Some(Stop)    => running = false
            None          => running = false

fn main():
    let jobs = Channel(8)

    // producer
    jobs.send(Work(2))
    jobs.send(Work(3))
    jobs.send(Work(4))
    jobs.send(Stop)

    consumer(jobs)
    // prints: 4, 9, 16
```

---

## Multiple Producers

Several senders can share one channel. Each `.send()` is thread-safe.

```mom
enum Task:
    Item(Int)
    Done

fn producer(ch: Channel[Task], start: Int, end: Int):
    let mut i = start
    while i <= end:
        ch.send(Item(i))
        i = i + 1
    ch.send(Done)

fn main():
    let ch = Channel(32)

    // Two producers (in a native runtime each would run on its own task/thread)
    producer(ch, 1, 3)   // sends Item(1), Item(2), Item(3), Done
    producer(ch, 4, 5)   // sends Item(4), Item(5), Done

    // Consumer drains until two Done signals
    let mut done_count = 0
    while done_count < 2:
        match ch.recv():
            Some(Item(n)) => print(n)
            Some(Done)    => done_count = done_count + 1
            None          => done_count = 2
```

---

## Channel as a Mailbox (Basis for Actors)

The `actor` keyword desugars into a struct of state, a typed `Channel` mailbox, and a recv-and-dispatch loop. You can build this pattern directly today using plain channels.

This is exactly what `examples/actor_via_channels.mom` demonstrates:

```mom
enum CounterMsg:
    Inc
    Add(Int)
    Get
    Stop

fn run_counter(mailbox: Channel[CounterMsg]) -> Int:
    let mut count = 0
    let mut running = true

    while running:
        match mailbox.recv():
            Some(Inc)      => count = count + 1
            Some(Add(n))   => count = count + n
            Some(Get)      => print(count)
            Some(Stop)     => running = false
            None           => running = false
    count

fn main():
    let mailbox = Channel(16)

    mailbox.send(Inc)
    mailbox.send(Inc)
    mailbox.send(Add(40))
    mailbox.send(Get)    // prints 42
    mailbox.send(Stop)

    let final_count = run_counter(mailbox)
    print(final_count)   // 42
```

The actor-via-channels pattern is portable and runs in the bootstrap interpreter today. Switch to the `actor` keyword (Phase 3.1) when you want compiler-enforced state isolation and supervisor restart semantics.

---

## Bounded Channels for Backpressure

Use a small bounded capacity to apply natural flow control between a fast producer and a slow consumer.

```mom
// Capacity 2: producer blocks after two unsent items,
// preventing it from racing too far ahead.
let pipeline = Channel(2)

// producer runs until consumer drains each batch
pipeline.send(Work(1))
pipeline.send(Work(2))
// would block here if consumer has not read yet (in concurrent execution)

match pipeline.recv():
    Some(Work(n)) => process(n)
    _             => ()
```

Guidelines:
- Set capacity to **1–4x** the expected processing burst size.
- A capacity of `1` gives the tightest coupling (rendezvous-like).
- A capacity of `0` is not supported — use a rendezvous channel abstraction (Phase 3.2).

---

## Error Handling on Send/Recv

In the current bootstrap interpreter, `.send()` on a closed channel is a runtime error. In the native runtime (Phase 3.1+) it will return `Result[(), ChannelError]`.

Pattern for safe receive loops today:

```mom
let mut running = true
while running:
    match ch.recv():
        Some(value) => handle(value)
        None        => running = false   // channel closed or empty after close
```

Pattern for safe non-blocking polling:

```mom
match ch.try_recv():
    Some(value) => handle(value)
    None        => do_other_work()
```

---

## Channel vs Other Primitives

| Need | Use |
|---|---|
| Pass a value between two tasks | `Channel` |
| One sender, one receiver, one value | oneshot channel (Phase 3.2) |
| Multiple receivers for the same value | broadcast channel (Phase 3.2) |
| Long-lived stateful worker | `actor` (backed by a channel mailbox) |
| Cooperative flow control | bounded `Channel(n)` |

---

## Current Status

Channels are **fully shipped** in the bootstrap interpreter.

| Feature | Status |
|---|---|
| `Channel(cap)` bounded | shipped |
| `Channel()` unbounded | shipped |
| `.send()`, `.recv()`, `.try_recv()` | shipped |
| `.len()`, `.is_empty()`, `.capacity()` | shipped |
| `.close()` | shipped |
| Broadcast channel | Phase 3.2 |
| Oneshot channel | Phase 3.2 |
| Native concurrent sends (multi-thread) | Phase 3.1 |

---

## See Also

- [Concurrency Overview](concurrency.md) — async tasks, spawn, Cancel
- [Actors](actors.md) — channels as actor mailboxes, supervision
