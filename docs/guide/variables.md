# Variables

Variables in Mom are **immutable by default**. This makes programs easier to reason about and eliminates a large class of accidental mutations. Mutability is always explicitly marked.

---

## Immutable Bindings — `let`

```mom
let x = 42
let name = "Alice"
let pi = 3.14159
```

Once bound, the value cannot be changed:

```mom
let x = 10
x = 20        // ERROR: cannot assign to immutable binding 'x'
```

### With a type annotation

Type annotations are optional for locals (the compiler infers them) but can be written explicitly:

```mom
let count: Int = 0
let flag: Bool = true
let label: String = "active"
```

---

## Mutable Bindings — `let mut`

Add `mut` to allow reassignment:

```mom
let mut counter = 0
counter = counter + 1
counter = counter + 1
print(counter)    // 2
```

```mom
let mut name = "Alice"
name = "Bob"
print(name)       // Bob
```

Mutability is a property of the **binding**, not the value. You can rebind a `let mut` to a completely different value.

---

## Constants — `const`

Constants are evaluated at compile time. They must have a type annotation and a value that the compiler can compute without running the program.

```mom
const MAX_SIZE: Int = 1024
const PI: Float = 3.14159265358979
const GREETING: String = "Hello"
const IS_DEBUG: Bool = false
```

Constants can be used anywhere a value of their type is expected:

```mom
fn buffer_ok(n: Int) -> Bool:
    n <= MAX_SIZE
```

**Differences from `let`:**
- Always available (no scope restriction if top-level)
- Never mutable — `const mut` does not exist
- Evaluated at compile time
- Naming convention: `SCREAMING_SNAKE_CASE`

---

## Shadowing

A new `let` with the same name *shadows* the previous binding in the same scope. The old binding is gone for the rest of the scope:

```mom
let x = 5
let x = x * 2   // shadows the previous x; x is now 10
let x = x + 1   // shadows again; x is now 11
print(x)         // 11
```

Shadowing is distinct from mutation: each new binding can have a different type:

```mom
let value = 42
let value = to_string(value)   // now value: String = "42"
print(value)                   // "42"
```

---

## Scope

Variables are scoped to the block they are declared in:

```mom
fn main():
    let outer = 1
    if true:
        let inner = 2
        print(outer)   // ok — outer is visible here
        print(inner)   // ok
    // inner is out of scope here
    print(outer)       // ok
```

Block expressions also create a scope:

```mom
let result = block:
    let temp = compute_something()
    temp * 2
// temp is not accessible here
print(result)
```

---

## Multiple Assignment

Mom does not support destructuring in `let` yet (planned for a future phase). Use named fields or pattern matching on structs/tuples instead.

---

## Variable Naming Rules

- Must start with a letter or underscore: `my_var`, `_unused`
- May contain letters, digits, and underscores: `count_2`, `is_valid`
- Case-sensitive: `value` ≠ `Value`
- Convention: `snake_case` for variables and functions

These are **reserved keywords** and cannot be used as variable names:

```
actor   and     as      async   await   break   comptime
const   continue defer  elif    else    enum    extern
false   fn      for     from    if      impl    import
in      let     match   module  mut     None    not
or      pub     receive region  return  self    spawn
struct  supervise trait  true    type    unsafe  use
where   while
```

---

## Type Inference

The compiler infers the type of every `let` binding from its initializer. You rarely need to write type annotations for local variables:

```mom
let x = 42              // inferred as Int
let y = 3.14            // inferred as Float
let flag = true         // inferred as Bool
let name = "Alice"      // inferred as String
let items = [1, 2, 3]  // inferred as [Int]
```

Type annotations are required for function parameters and return types, and for top-level `const` declarations.

---

## Printing Variables

```mom
let score = 95
print(score)                     // 95 (Int prints directly)
print("Score: " + str(score))    // Score: 95
```
