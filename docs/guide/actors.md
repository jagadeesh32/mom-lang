# Actors

An **actor** is an isolated state machine that communicates exclusively through message passing. Actors have no shared mutable state — the only way to observe or change an actor's internals is to send it a message.

---

## The Actor Model

Mom's actor model draws from Erlang's philosophy:

- **Isolation** — each actor owns its state exclusively; no other code can touch it.
- **Message passing** — communication happens by sending typed messages into a mailbox.
- **Let it crash** — an actor that panics is restarted by its supervisor; callers are not affected.
- **Location transparency** — an `ActorRef[T]` is an opaque handle; the actor may run on any thread.

The three properties together eliminate the data races and deadlocks that plague shared-memory concurrency.

---

## Declaring an Actor

Use the `actor` keyword with a `state` block and a `receive` block.

```mom
actor Counter:
    state count: Int

    receive:
        Inc      => Counter { count: self.count + 1 }
        Dec      => Counter { count: self.count - 1 }
        Reset    => Counter { count: 0 }
        Add(n)   => Counter { count: self.count + n }
```

The compiler desugars this into:

1. A **struct** holding the state fields.
2. An **impl** with a `step(self, msg) -> Self` method that pattern-matches on `msg` and returns the next state.

The `receive` arms are pure functions of the current state — they return a new state value rather than mutating in place.

### Actor Messages as Enum Variants

Messages are always an enum. Each variant is one kind of message the actor accepts.

```mom
enum CounterMsg:
    Inc
    Dec
    Reset
    Add(Int)
```

> **Tip:** The message enum does not need to be declared separately — the `actor` block infers it from the `receive` arms. You can declare it explicitly for reuse across actors.

---

## The Step Function / Message Dispatch

The `step` method is the core of every actor. It receives the current state and one message, and returns the next state.

```mom
fn main():
    let mut c = Counter { count: 0 }
    c = c.step(Inc)        // count = 1
    c = c.step(Inc)        // count = 2
    c = c.step(Add(10))    // count = 12
    c = c.step(Dec)        // count = 11
    print(c.count)         // 11
    c = c.step(Reset)
    print(c.count)         // 0
```

In interpreter mode (Phase 3.0), you drive the step loop manually. In Phase 3.1+ the runtime wraps the step function in a message-pump loop automatically.

---

## `ActorRef[T]` — A Handle to a Running Actor

`ActorRef[T]` is an opaque, cheaply clonable reference to a live actor. It exposes only the send and ask operations — you cannot read the actor's state directly.

```mom
let cache: ActorRef[Cache] = spawn Cache::new(map: Map::new())
```

---

## Spawning an Actor

`spawn ActorName::new(…)` starts the actor's message pump on the executor and returns an `ActorRef[ActorName]`.

```mom
actor Cache:
    state map: Map[String, String]

    receive:
        Put(key, val) => Cache { map: self.map.insert(key, val) }
        Get(key)      => self   // reply handled via ask
        Clear         => Cache { map: Map::new() }

let cache: ActorRef[Cache] = spawn Cache::new(map: Map::new())
```

---

## Sending Messages — `actor <- Message`

The `<-` operator sends a message to an actor. Fire-and-forget: it enqueues the message and returns immediately.

```mom
cache <- Put("hello", "world")
cache <- Put("foo", "bar")
cache <- Clear
```

---

## `ask` — Request-Reply Pattern

`ask` sends a message and awaits a reply. The actor must include a reply channel in the message variant.

```mom
actor Cache:
    state map: Map[String, String]

    receive:
        Get(key, reply) => {
            reply.send(self.map.get(key))
            self
        }
        Put(key, val) => Cache { map: self.map.insert(key, val) }

let cache: ActorRef[Cache] = spawn Cache::new(map: Map::new())

cache <- Put("hello", "world")
let value = await cache.ask(Get("hello"))   // Some("world")
```

The `ask` call creates a temporary oneshot channel, injects it as `reply` in the message, sends the message, and awaits the channel's response.

---

## Actor Isolation Guarantees

The compiler enforces four invariants:

| Invariant | Description |
|---|---|
| State is private | The actor's `state` fields are only accessible inside its `receive` block |
| Messages move ownership | Sending a non-`Copy` value transfers it to the actor; the sender loses it |
| Single-threaded inside | Only one `receive` arm runs at a time — no internal locks needed |
| Bounded mailboxes | Actor mailboxes are bounded by default; overflow is observable, not silent |

These guarantees are enforced **at compile time**, not by convention.

---

## Supervision — Restart Policies

A supervisor monitors a set of child actors and restarts them when they fail. Mom uses Erlang-style supervision trees.

```mom
supervise:
    strategy: OneForOne
    children:
        spawn CacheActor::new()
        spawn WorkerActor::new()
        spawn LoggerActor::new()
```

### Restart Strategies

| Strategy | Behavior |
|---|---|
| `OneForOne` | Restart only the failed child |
| `OneForAll` | Restart all children when any one fails |
| `RestForOne` | Restart the failed child and all children started after it |

> **Phase note:** `supervise` lands in Phase 3.2. The patterns and enum names are stable; the runtime restart driver is the work-in-progress piece.

---

## Building Actors Manually with Channels

The `actor` keyword is syntactic sugar over a pattern you can build today from plain channels. This is the channel-based actor pattern from `examples/actor_via_channels.mom`, runnable in the current bootstrap interpreter.

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
    mailbox.send(Get)        // prints 42
    mailbox.send(Stop)

    let final_count = run_counter(mailbox)
    print(final_count)       // 42
```

The mapping to actor concepts:
- `Channel[CounterMsg]` is the mailbox.
- `run_counter` is the message pump / step loop.
- `count` is the actor state.
- `mailbox.send(Inc)` is `actor <- Inc`.
- `mailbox.recv()` is the dispatcher.

Switch to `actor … receive` when you want the compiler to enforce isolation and the runtime to provide restart semantics.

---

## When to Use Actors vs Channels vs Async

| Situation | Recommended |
|---|---|
| Long-lived service with internal state | `actor` |
| Simple pipeline between two tasks | `Channel` |
| Fan-out to stateless workers | `spawn` + `Channel` |
| Request-reply with isolated state | `actor` + `ask` |
| Fault isolation: restart on failure | `actor` + `supervise` |
| Short-lived computation | `async fn` + `await` |
| You need state isolation enforced by the compiler | `actor` |

Actors add overhead (a channel allocation per instance, a dispatch loop). For stateless workers or short-lived tasks, `spawn` with plain `async fn` is cheaper and simpler.

---

## Full Worked Example — Counter Actor

```mom
// examples/actor.mom — Phase 3.1 actor syntax

enum CounterMsg:
    Inc
    Dec
    Reset
    Add(Int)

actor Counter:
    state count: Int

    receive:
        Inc      => Counter { count: self.count + 1 }
        Dec      => Counter { count: self.count - 1 }
        Reset    => Counter { count: 0 }
        Add(n)   => Counter { count: self.count + n }

fn main():
    let mut c = Counter { count: 0 }
    c = c.step(Inc)
    c = c.step(Inc)
    c = c.step(Add(10))
    c = c.step(Dec)
    print(c.count)    // 11
    c = c.step(Reset)
    print(c.count)    // 0
```

---

## Full Worked Example — Cache Actor

```mom
actor Cache:
    state map: Map[String, String]

    receive:
        Put(key, val)   => Cache { map: self.map.insert(key, val) }
        Get(key, reply) => {
            reply.send(self.map.get(key))
            self
        }
        Clear           => Cache { map: Map::new() }

fn main():
    let cache: ActorRef[Cache] = spawn Cache::new(map: Map::new())

    cache <- Put("lang", "mom")
    cache <- Put("version", "3")

    let lang    = await cache.ask(Get("lang"))     // Some("mom")
    let missing = await cache.ask(Get("missing"))  // None

    match lang:
        Some(v) => print(v)
        None    => print("not found")

    cache <- Clear
```

---

## Full Worked Example — Channel-Based Actor (Runs Today)

```mom
// Runs in the bootstrap interpreter — no actor keyword needed.

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
            Some(Inc)    => { count = count + 1 }
            Some(Add(n)) => { count = count + n }
            Some(Get)    => { print(count) }
            Some(Stop)   => { running = false }
            None         => { running = false }
    count

fn main():
    let mailbox = Channel(16)
    mailbox.send(Inc)
    mailbox.send(Inc)
    mailbox.send(Add(40))
    mailbox.send(Get)        // prints 42
    mailbox.send(Stop)
    let final_count = run_counter(mailbox)
    print(final_count)       // 42
```

---

## Current Status

| Feature | Status |
|---|---|
| Channel-based actor pattern | **shipped** (runs in bootstrap interpreter) |
| `actor Name: state … receive:` syntax | **Phase 3.1** |
| `ActorRef[T]` + `spawn Actor::new(…)` | **Phase 3.1** |
| `actor <- Message` send syntax | **Phase 3.1** |
| `ask` request-reply | **Phase 3.1** |
| `supervise … restart` trees | **Phase 3.2** |
| `OneForOne` / `OneForAll` / `RestForOne` | **Phase 3.2** |

Until Phase 3.1 ships, build actors with the channel-based pattern — it uses the same concepts and is a drop-in migration target.

---

## See Also

- [Channels](channels.md) — the building block under every actor
- [Concurrency Overview](concurrency.md) — async tasks, spawn, Cancel, the full concurrency stack
