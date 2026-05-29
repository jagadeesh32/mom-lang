# Memory and Ownership

Mom's headline guarantee:

> Safe programs cannot suffer from **null dereferences, use-after-free, double-free, data races, or out-of-bounds memory access** — and they do not pay for a tracing garbage collector to get those guarantees.

This is achieved through a **regions + owned references + actor isolation** model inspired by Rust but trading some expressive power for faster learning and cleaner ergonomics.

---

## The Memory Model at a Glance

Mom gives you three clearly separated allocation domains. The compiler always knows which domain a value belongs to, and the rules for each domain are enforced statically.

| Domain | Lifetime | Typical use |
|--------|----------|-------------|
| **Stack** | scope-bound | local variables, scratch values |
| **Region** | request / arena / task-bound | per-request HTTP buffers, parse trees |
| **Heap** | reference-counted or owned | long-lived collections, shared caches |

The choice is **explicit** at the call site that constructs the value (`Box(v)`, `Rc(v)`, `Arc(v)`, `region r: ...`) or implicit when stack-only is sufficient (the default).

---

## Ownership

Every value has **exactly one owner** at any moment. The owner is responsible for cleanup; the compiler emits the destructor automatically at the end of the owner's scope.

```mom
fn take(s: String):
    print(s)    // s is moved in, dropped on return

fn main():
    let name = "hello"
    take(name)
    // name is gone here; using it is a compile error
```

Key rules:

1. One value, one owner — always.
2. When the owner goes out of scope, the value is freed.
3. There is no manual `free`. No `delete`. No `gc.collect()`.

---

## Move Semantics

Passing a **non-Copy** value to a function, or assigning it to a new binding, **moves** it. After the move the original binding is no longer valid.

```mom
fn main():
    let s = "owned string"
    let t = s               // s is moved into t
    print(t)                // OK
    // print(s)             // ERROR: use after move
```

This prevents double-free: if ownership was transferred, only the new owner frees the value.

---

## Copy Types

Some types are cheap to duplicate. Mom copies them implicitly rather than moving them:

| Copy type | Notes |
|-----------|-------|
| `Int` | machine integer |
| `Float` | floating-point number |
| `Bool` | boolean |
| `Char` | Unicode scalar value |
| `Unit` | the empty type `()` |
| `&T`, `&mut T` | borrow tokens (source binding's loan state governs) |

```mom
fn add_one(n: Int) -> Int:
    n + 1

fn main():
    let x = 10
    let y = add_one(x)  // x is copied, not moved
    print(x)            // still valid: 10
    print(y)            // 11
```

Structs can opt into `Copy` semantics with `derive(Copy)` if all their fields are `Copy`.

---

## Non-Copy Types: Moved by Default

Structs, strings, lists, and other heap-backed types are **non-Copy**. Assigning or passing them moves ownership:

```mom
struct Point:
    x: Int
    y: Int

fn main():
    let p = Point { x: 1, y: 2 }
    let q = p               // p moved into q
    print(q.x)              // OK
    // print(p.x)           // ERROR: use after move
```

To share without moving, use a borrow (`&Point`) — see the [Borrow Checker](borrow-checker.md) guide.

---

## Heap Allocation: `Box[T]`, `Rc[T]`, `Arc[T]`

When a value needs to outlive its creating scope, or when you need shared or thread-safe access, allocate on the heap.

### `Box[T]` — Unique Heap Ownership

A `Box` owns its value on the heap. Moving the `Box` moves ownership. Dropped when the `Box` goes out of scope.

```mom
fn main():
    let boxed = Box(99)
    print(boxed)            // 99
    // boxed is freed here
```

Use `Box` for:
- Heap-allocating a large value to avoid stack pressure.
- Recursive types (a struct that contains itself must use `Box`).
- Returning heap-allocated values from functions.

### `Rc[T]` — Reference-Counted Shared Ownership (Single-Threaded)

`Rc` allows multiple owners within **one thread**. The value is freed when the last `Rc` clone is dropped.

```mom
fn main():
    let shared = Rc("greeting")
    let also   = shared         // clone of the Rc; both point to "greeting"
    print(shared)
    print(also)
    // freed when both go out of scope
```

`Rc` has no synchronization overhead, but cannot be sent across thread boundaries.

### `Arc[T]` — Atomically Reference-Counted (Multi-Threaded)

`Arc` is like `Rc` but uses an atomic reference count safe for multi-threaded sharing.

```mom
fn main():
    let atomic = Arc(true)
    print(atomic)
    // can be cloned and sent to other actors/threads
```

Use `Arc` when a value must be shared across actor or thread boundaries.

**Choosing the right heap type:**

| Need | Use |
|------|-----|
| One owner, heap-allocated | `Box[T]` |
| Multiple owners, one thread | `Rc[T]` |
| Multiple owners, multiple threads | `Arc[T]` |

---

## Stack Allocation (the Default)

Local variables live on the stack with no allocation overhead. They are freed automatically when their scope ends.

```mom
fn compute() -> Int:
    let x = 10          // stack-allocated
    let y = 20          // stack-allocated
    x + y               // returned by value (copied if Int)
// x and y are freed here
```

Prefer stack allocation whenever the value fits and does not need to outlive the function.

---

## Regions: Arena Allocation

Regions are Mom's ergonomic answer to request-scoped or arena-scoped allocations. Instead of individually freeing many related values, you allocate everything into a region and free the whole arena at once in O(1).

```mom
fn handle(req: Request) -> Response:
    let label = region r:
        let tag = "request:GET /healthz"
        tag                         // the region's value is its tail expression
    print(label)
    // every allocation made inside `region r:` is freed here, all at once
```

The `region r:` block is a **bump allocator**. Every `r.alloc(...)` call is a pointer increment. Freeing the region resets the pointer — O(1) regardless of how many objects were allocated.

The compiler performs **escape analysis** to ensure that no pointer to region-allocated memory outlives the region.

```mom
fn process(req: Request) -> Response:
    region r:
        let headers = r.alloc(parse_headers(req.bytes))
        let body    = r.alloc(parse_body(req.bytes))
        respond(headers, body)
    // headers and body are freed here
```

**When to use a region:**

- Per-request HTTP buffers.
- Parse trees that are consumed and discarded in one pass.
- Any group of allocations with the same lifetime that are cheaper to free together.

---

## References: Borrowing Without Moving

You can lend a value without transferring ownership using references. References never own the value; they only borrow it.

```mom
fn read_only(buf: &[Int]) -> Int:
    len(buf)                // borrows buf, does not move it

fn main():
    let n = 21
    let a = &n              // shared borrow
    let b = &n              // another shared borrow — both OK
    print(a)
    print(b)
    // n is still owned here
```

See the [Borrow Checker](borrow-checker.md) guide for the full rules on `&T` and `&mut T`.

---

## The `unsafe` Block

The safety rules can be suspended inside an `unsafe` block. This is an explicit, auditable opt-out:

```mom
unsafe:
    // raw pointer arithmetic, FFI calls, unchecked indexing
    let ptr = raw_ptr(buf)
    ptr_write(ptr, 0xFF)
```

`unsafe` is for:
- FFI (calling C libraries).
- Operating-system-level code.
- Performance-critical paths where the programmer can prove safety manually.

`unsafe` does **not** disable the borrow checker or the type checker. It only unlocks a small set of additional operations (raw pointers, unchecked casts, FFI). Keep `unsafe` blocks small and document why they are correct.

---

## What Mom Prevents

| Error class | How Mom prevents it |
|-------------|---------------------|
| Null dereferences | No `null` exists. Absence is `Option[T]`. |
| Use-after-free | Ownership transfer means only the current owner can access a value. |
| Double-free | One owner, one destructor. |
| Data races | `&mut` aliasing is forbidden across all threads; shared mutable state requires `Atomic[T]`, `Mutex[T]`, or `Channel[T]`. |
| Buffer overflows | All indexing is bounds-checked; `unsafe` can opt out with explicit proof of correctness. |
| Memory leaks | Destructors are emitted automatically; regions free everything at once. |

---

## Comparison to Rust

Mom's ownership model is directly inspired by Rust. The key differences are ergonomic, not conceptual:

| Concern | Rust | Mom |
|---------|------|-----|
| Borrow checker | Full | Full (simpler lifetime elision) |
| Lifetime syntax | Required at API boundaries | Mostly inferred; explicit only where ambiguous |
| Regions / arenas | External crates | Built-in (`region { … }`) |
| `Pin`, GATs | Yes | Deferred — re-evaluated after MVP |
| `Send`/`Sync` markers | Explicit `derive` | Implicit; computed from data layout |
| GC option | No | No (opt-in `gc { … }` region on roadmap for graph workloads) |

If you already know Rust, Mom will feel familiar. The mental model is the same: ownership, moves, borrows, lifetimes. The syntax and error messages are designed to have a shorter learning curve.

---

## Worked Examples

### Multiple Shared Borrows

```mom
fn main():
    let n = 21
    let a = &n          // shared borrow
    let b = &n          // another shared borrow — both legal
    print(a)            // 21
    print(b)            // 21
```

### Region Allocation

```mom
fn main():
    let label = region r:
        let tag = "request:GET /healthz"
        tag             // tag is the region's result value
    print(label)        // "request:GET /healthz"
    // the region and all its allocations are freed here
```

### Heap Smart Pointers

```mom
fn main():
    // Unique heap ownership
    let boxed  = Box(99)
    print(boxed)            // 99

    // Reference-counted single-threaded sharing
    let shared = Rc("greeting")
    print(shared)           // "greeting"

    // Atomically reference-counted cross-thread sharing
    let atomic = Arc(true)
    print(atomic)           // true
```

### Ownership and Move

```mom
fn consume(s: String):
    print(s)                // prints, then s is dropped

fn main():
    let greeting = "hello"
    consume(greeting)       // greeting is moved
    // print(greeting)      // compile error: use after move
```

---

## What You Give Up

These trade-offs are deliberate:

- **Self-referential structures** need `Rc[RefCell[T]]`-style patterns.
- **Zero-copy parsing across functions** can require a lifetime annotation.
- **Cyclic graphs** use `Arena` or `WeakRef` — plain references are not cyclic.

The alternative — a garbage collector — costs more on the workloads Mom targets: kernels, real-time systems, high-frequency networking, AI inference. The ownership model makes those costs predictable and zero at runtime.
