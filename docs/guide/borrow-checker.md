# The Borrow Checker

The borrow checker is a compile-time analysis pass that enforces Mom's memory safety rules without a garbage collector. It runs after type checking and rejects programs that violate ownership or aliasing rules — giving you a precise error message before the program ever runs.

---

## What the Borrow Checker Does

Every binding in a Mom program has an **owner**. The borrow checker tracks:

- Which bindings are currently **owned** (not moved).
- Which bindings currently have **active borrows** (loans) outstanding.
- Whether those borrows are **shared** (`&T`) or **mutable** (`&mut T`).

It uses this information to enforce two core invariants at all times:

1. **No aliased mutation** — you can read from many places, or write from one place, but never both simultaneously.
2. **No dangling references** — a borrow cannot outlive the value it borrows.

The result: null dereferences, use-after-free, and data races are impossible in safe Mom code.

---

## `&T` — Immutable (Shared) Borrows

A `&T` is a read-only reference to a value. Many `&T` borrows of the same binding may coexist; none of them can modify the value.

```mom
fn main():
    let name = "alice"
    let a = &name           // shared borrow
    let b = &name           // another shared borrow — both OK
    print(a)                // "alice"
    print(b)                // "alice"
    // name is still valid; a and b are released at end of scope
```

Think of shared borrows as read locks: arbitrarily many readers are fine as long as no writer is active.

---

## `&mut T` — Mutable (Unique) Borrows

A `&mut T` is an exclusive reference that allows modification. Only one `&mut T` may exist for a given binding at a time, and no `&T` borrows may coexist with it.

```mom
fn main():
    let mut counter = 0
    block:
        let m = &mut counter    // mutable borrow
        print(m)                // can read through m
    // m goes out of scope here; the loan is released
    counter = counter + 1       // now we can mutate directly
    print(counter)              // 1
```

Think of a mutable borrow as a write lock: exclusive access, no concurrent readers.

---

## Rule 1: Multiple `&T` OR One `&mut T` — Never Both

You can have as many shared borrows as you like, **or** exactly one mutable borrow — but **never both at the same time**.

### What breaks

```mom
// BAD — double mutable borrow
let mut x = "hello"
let r1 = &mut x
let r2 = &mut x    // ERROR: second mutable borrow while r1 is live
print(r1)
```

```mom
// BAD — shared + mutable borrow
let mut y = "hi"
let s = &y
let m = &mut y     // ERROR: mutable borrow while shared borrow s is live
print(s)
```

### How to fix

Use `block:` to scope borrows so they don't overlap:

```mom
// OK — borrows in separate scopes
let mut x = "hello"
block:
    let r1 = &mut x
    print(r1)
// r1 released here

block:
    let r2 = &mut x     // fine: r1 is gone
    print(r2)
```

Or restructure so the mutation happens after all borrows are done:

```mom
let mut y = "hi"
let s = &y
print(s)        // use s here
// s released at end of scope — in the next block &mut y is legal
block:
    let m = &mut y
    print(m)
```

---

## Rule 2: Borrows Cannot Outlive the Value

A reference must not outlive the value it points into. The borrow checker enforces this lexically: a loan's lifetime is the lexical scope of the binding that holds it.

```mom
fn main():
    let r: &Int
    block:
        let x = 42
        r = &x          // ERROR: x's scope ends before r's scope
    print(r)            // r would be dangling
```

### How to fix

Move the borrow inside the scope where the value lives, or extend the value's scope to match:

```mom
fn main():
    let x = 42
    let r = &x          // r and x have the same scope — OK
    print(r)
```

---

## Use-After-Move

Passing a non-Copy value to a function, or assigning it to a new binding, **moves** it. The original binding is no longer valid.

### What breaks

```mom
// BAD — use after move
let owned = "owned"
let other = owned       // owned is moved into other
print(owned)            // ERROR: use of moved value
```

### How to fix

Use the new binding, or borrow the original if you need both:

```mom
// Fix 1: use the new binding
let owned = "owned"
let other = owned
print(other)            // OK

// Fix 2: borrow instead of move
let owned = "owned"
let r = &owned          // borrow, not move
print(r)
print(owned)            // still valid
```

---

## Double Mutable Borrow

Two `&mut` references to the same binding in the same scope is always an error.

### What breaks

```mom
let mut x = "hello"
let r1 = &mut x
let r2 = &mut x         // ERROR: x already mutably borrowed by r1
print(r1)
```

### How to fix

Scope the borrows so they do not overlap:

```mom
let mut x = "hello"
block:
    let r1 = &mut x
    print(r1)
// r1 released

let r2 = &mut x         // OK: r1 is out of scope
print(r2)
```

---

## Shared + Mutable Mix

A `&T` and a `&mut T` to the same binding cannot coexist.

### What breaks

```mom
let mut y = "hi"
let s = &y
let m = &mut y          // ERROR: y is already borrowed immutably by s
print(s)
```

### How to fix

Use the shared borrow, let it go out of scope, then take the mutable borrow:

```mom
let mut y = "hi"
block:
    let s = &y
    print(s)
// s released

let m = &mut y          // OK
print(m)
```

---

## Mutation While Borrowed

While any borrow of a binding is active, the binding may not be mutated, moved, or rebound.

### What breaks

```mom
let mut x = 10
let r = &x
x = 20                  // ERROR: cannot mutate x while r is a live borrow
print(r)
```

### How to fix

Finish using the borrow before mutating:

```mom
let mut x = 10
block:
    let r = &x
    print(r)            // 10
// r released

x = 20                  // OK
print(x)                // 20
```

---

## `&mut` on an Immutable Binding

Taking a mutable reference to a binding declared without `mut` is an error. The compiler gives a helpful hint.

### What breaks

```mom
let x = 5
let m = &mut x          // ERROR: cannot borrow x as mutable; declare it `let mut`
```

### How to fix

Declare the binding as `let mut`:

```mom
let mut x = 5
let m = &mut x          // OK
```

---

## The `block:` Expression for Scoping Borrows

`block:` creates a new lexical scope. Borrows introduced inside a `block:` are released when the block ends, making it the primary tool for managing borrow lifetimes.

```mom
fn main():
    let mut counter = 0

    block:
        let m = &mut counter    // mutable borrow begins
        print(m)
    // m goes out of scope; mutable borrow released

    counter = counter + 1       // now safe to mutate directly
    print(counter)              // 1
```

Sequential `block:` regions can each take their own `&mut`:

```mom
let mut x = "hello"
block:
    let r1 = &mut x
    print(r1)
// r1 released

block:
    let r2 = &mut x     // legal: r1 is gone
    print(r2)
```

---

## Transient Borrows

Borrows that appear only inside an expression (not bound to a name) are **transient**: they are checked and released immediately after the expression is evaluated.

```mom
fn len_of(s: &String) -> Int: len(s)

fn main():
    let name = "alice"
    print(len_of(&name))    // &name is a transient borrow — released right here
    print(name)             // still valid
```

---

## What the Phase-2 Borrow Checker Catches Today

The current bootstrap borrow checker (`src/borrow.rs`) is a conservative, lexical-scope-based analysis (pre-NLL style). It rejects the following at compile time:

| Error | Example |
|-------|---------|
| **Use after move** | `let y = x; print(x)` where `x` is not Copy |
| **Double mutable borrow** | `let a = &mut x; let b = &mut x` in the same scope |
| **Shared + mutable mix** | `let r = &x; let m = &mut x` in the same scope |
| **Mutation while borrowed** | `x = …` while any borrow of `x` is alive |
| **`&mut` of immutable** | `let x = …; let m = &mut x` — hint: declare `let mut` |
| **Re-binding moved values** | rebinding a moved non-`mut` binding |

Loans are released at the end of the **lexical scope** of the binding that introduced them. Two sequential scopes can each hold a `&mut` to the same value because by the time the second begins, the first is gone.

---

## What the Borrow Checker Does NOT Yet Catch (Phase 2.1)

The bootstrap checker is conservative. Some valid programs are rejected because the checker reasons at the lexical-scope level, not at the data-flow level. These limitations will be lifted in Phase 2.1 and the native compiler (Phase 4 self-host):

- **Non-lexical lifetimes (NLL)** — a borrow that is last used partway through a scope should be released there, not at scope end. The current checker keeps it live until scope end, which can cause false "borrow still active" errors.
- **Per-function move-on-call semantics** — function arguments are currently treated as reads. The checker does not yet model moves across function call boundaries precisely.
- **Cross-module ownership tracking** — ownership analysis is currently intra-module only.
- **Region escape detection** — a reference into a region cannot yet be statically proven not to escape the region; this relies on manual discipline.

---

## Fix Patterns for Common Borrow Errors

| Error | Fix |
|-------|-----|
| Use after move | Use the new binding, or borrow with `&` instead |
| Double `&mut` | Wrap first borrow in `block:` so it ends before the second begins |
| `&` and `&mut` together | Use `&` in one `block:`, then `&mut` in the next |
| Mutate while borrowed | Finish using the borrow (let its `block:` end), then mutate |
| `&mut` on immutable | Add `mut` to the `let` declaration |

---

## Full Worked Example

The following is `examples/borrow.mom`, which demonstrates all the rules:

```mom
// borrow.mom — Phase 2 borrow checker in action.
//
// The borrow checker catches these errors at compile time:
//   * use of a moved value
//   * two `&mut` of the same binding in scope
//   * `&` and `&mut` of the same binding in scope
//   * mutating a binding while it is borrowed
//   * `&mut` of an immutable binding
//
// Uncomment any of the BAD blocks to see the borrow checker reject
// the program with a precise diagnostic.

fn main():
    // OK — multiple shared borrows.
    let name = "alice"
    let a = &name
    let b = &name
    print(a)
    print(b)

    // OK — mutable borrow inside its own scope.
    let mut counter = 0
    block:
        let m = &mut counter
        print(m)
    counter = counter + 1
    print(counter)

    // BAD — double mutable borrow.
    // let mut x = "hello"
    // let r1 = &mut x
    // let r2 = &mut x
    // print(r1)

    // BAD — shared + mutable borrow.
    // let mut y = "hi"
    // let s = &y
    // let m = &mut y
    // print(s)

    // BAD — use after move.
    // let owned = "owned"
    // let other = owned
    // print(owned)
```

Uncomment any of the `BAD` blocks to see the borrow checker emit a precise diagnostic. Each error names the binding, the conflicting loan, and the relevant source location.

---

## Summary of Rules

| Rule | Wording |
|------|---------|
| **Exclusive mutation** | While `&mut x` is live, no other borrow of `x` is allowed. |
| **No mutable aliases** | While any `&x` is live, `&mut x` is not allowed. |
| **No mutation under borrow** | While any borrow of `x` is live, `x` may not be mutated, moved, or rebound. |
| **Borrows don't outlive owners** | A loan's lifetime is bounded by the lexical scope of its binding. |
| **Moves are final** | Reading a non-Copy binding after it is moved is an error. |
| **Mutability is opt-in** | A binding without `mut` cannot be mutably borrowed. |

These rules are checked at compile time. There are no runtime borrow counters, no reference counting overhead, and no garbage collection pauses.
