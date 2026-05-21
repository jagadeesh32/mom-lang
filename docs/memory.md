# mom — Memory Model & Safety

mom's headline guarantee:

> Safe programs cannot suffer from **null dereferences, use-after-free,
> double-free, data races, or out-of-bounds memory access** — and they
> do not pay for a tracing garbage collector to get those guarantees.

This is achieved through a deliberately simpler **regions + borrowed
references + actor isolation** model. Rust's borrow checker inspired
this, but mom trades some expressive power for a faster learning curve
and cleaner API ergonomics.

## 1. Three allocation domains

mom programs split allocation into three deliberately distinct domains.
The compiler always knows which domain a value belongs to.

| Domain | Lifetime | Typical use |
|--------|----------|-------------|
| **Stack**  | scope-bound | local variables, scratch values |
| **Region** | request/arena/task-bound | per-request HTTP buffers, parse trees |
| **Heap**   | reference-counted or owned | long-lived collections, shared caches |

The choice is **explicit** at the call site that constructs the value
(`Box::new`, `Rc::new`, `Region.alloc`) or implicit when stack-only is
sufficient (the default).

## 2. Ownership

- Each value has exactly one owner at any moment.
- Passing a value to a function moves ownership unless it's a `Copy`
  type (primitives, small structs marked `derive(Copy)`).
- The owner is responsible for cleanup; the compiler emits the
  destructor automatically at the end of the owner's scope.

```mom
fn take(s: String) { print(s) }      // s is moved in, dropped on return

fn main() {
    let name = "hello".to_string()
    take(name)
    // name is gone here; using it would be a compile error
}
```

## 3. Borrowing

Two reference kinds:

- `&T` — **shared, immutable** borrow. Many may co-exist.
- `&mut T` — **unique, mutable** borrow. Only one at a time, and no
  shared borrows may co-exist.

```mom
fn read_only(buf: &[Byte]) -> Int { len(buf) }
fn write_into(buf: &mut [Byte]) { buf[0] = 0 }
```

Borrow rules are checked at compile time. There are no runtime borrow
counters.

### Lifetime inference

In simple cases, lifetimes are inferred:

```mom
fn first_word(s: &String) -> &String { … }
```

When two references could outlive different scopes, an explicit
lifetime parameter is required (rare in practice):

```mom
fn pick<'a>(a: &'a String, b: &'a String) -> &'a String { … }
```

The native compiler will use the standard "input → output" elision
rules borrowed from Rust.

## 4. Regions

Regions are mom's signature ergonomic move. Instead of arguing with the
borrow checker for request-scoped buffers, drop everything into a region:

```mom
fn handle(req: Request) -> Response {
    region r {
        let header = r.alloc(parse_headers(req.bytes))
        let body   = r.alloc(parse_body(req.bytes))
        respond(header, body)
    }
    // every allocation made in `r` is freed here, all at once
}
```

The region's allocator is a bump allocator; freeing is O(1). The
compiler ensures that no pointer to region-allocated memory escapes
the region (escape analysis).

## 5. No `null`, no implicit failure

There is no `null` value, no `nil`, no `None` returned from random APIs.
Absence is modelled by `Option[T]` and the compiler refuses to treat
a non-optional `T` as if it might be missing.

```mom
let user: User = lookup(id)?      // Result-propagating, never null
let maybe: Option[User] = …       // explicit optional
```

## 6. Data race freedom

A data race in mom is **a compile error**. The rules:

- A value crossing an actor boundary is **moved** by default. The
  sender no longer owns it.
- Shared, mutable, multi-threaded access requires `Atomic[T]`,
  `Mutex[T]`, `RwLock[T]`, or `Channel[T]`.
- The borrow checker enforces "no `&mut` aliasing" across all threads,
  not just within a function.

See [concurrency.md](concurrency.md).

## 7. Out-of-bounds prevention

- All indexing is bounds-checked at runtime by default.
- The native compiler elides checks it can prove safe (loop induction
  variables, range-bounded indices).
- `unsafe` lets the programmer skip checks where the proof is manual.

## 8. The `unsafe` boundary

```mom
unsafe {
    let raw = ffi.malloc(1024)        // raw pointer arithmetic
    *raw.offset(7) = 0x42              // dereference of raw pointer
    ffi.free(raw)
}
```

Inside `unsafe`, the programmer accepts responsibility for the
guarantees the checker normally provides. Outside `unsafe`, soundness
is enforced. `unsafe` is the **audit boundary**: code reviews and tools
focus their attention there.

## 9. Comparison to Rust

| Concern | Rust | mom |
|---------|------|-----|
| Borrow checker  | full      | full (with simpler elision rules)  |
| Lifetime syntax | required at API boundaries | mostly inferred; explicit only where ambiguous |
| Regions/arenas  | external crates | built-in (`region { … }`)     |
| `Pin`, GATs     | yes       | deferred — re-evaluated after MVP   |
| `Send`/`Sync` markers | yes | implicit; computed from data layout |
| GC option       | no        | no (an opt-in `gc { … }` region is on the roadmap for the AI/graph workloads that demand it) |

## 10. What you give up

- Some self-referential data structures need explicit
  `Rc[RefCell[T]]`-style patterns.
- Zero-copy parsing across functions can require a lifetime annotation.
- Cyclic graphs use `Arena` or `WeakRef` — they are not idiomatic plain
  references.

These trade-offs are explicit because the alternative — a GC — costs
more on the workloads mom targets (kernels, real-time systems,
high-frequency networking, AI inference).

## 11. What the Phase-2 borrow checker catches today

The bootstrap borrow checker (`src/borrow.rs`) is conservative,
lexical-scope based (pre-NLL style). It runs after type checking and
rejects the following at compile time:

- **Use after move** — `let y = x; print(x)` where `x`'s type is not
  Copy.
- **Double mutable borrow** — `let a = &mut x; let b = &mut x` in the
  same scope.
- **Shared + mutable mix** — `let r = &x; let m = &mut x` in the same
  scope.
- **Mutation while borrowed** — `x = …` (or `&mut x`) while any borrow
  of `x` is alive.
- **`&mut` of immutable** — `let x = …; let m = &mut x` rejects with
  a "declare it `let mut`" hint.
- **Re-binding moved values** — only allowed if the binding is `mut`.

Loans are released at the end of the lexical scope of the binding that
introduced them. Sequential `{ let m = &mut x; … } { let n = &mut x; … }`
is therefore legal.

What the bootstrap checker **does not yet do** (Phase 2.1):

- Non-lexical lifetimes (NLL) — once-used borrows freed at their last
  use rather than at end of scope.
- Per-function move-on-call semantics. Function arguments are currently
  treated as reads.
- Cross-module ownership tracking.
- Region escape detection — a reference into a region cannot yet be
  proven not to escape.

The native compiler (Phase 4 self-host) lifts these limitations.
