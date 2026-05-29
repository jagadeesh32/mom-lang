# Private Variables

Mom uses a **visibility system** to control what is accessible from outside a module or struct. By default everything is private — you must explicitly mark items `pub` to make them public.

---

## Default: Private

Any declaration without the `pub` keyword is private to its module:

```mom
// math.mom
fn helper(x: Int) -> Int:    // private — only usable inside math.mom
    x * x + 1

pub fn square(x: Int) -> Int:   // public — importable from other modules
    x * x
```

If another module imports `math`, it can only call `square`. Calling `helper` is a compile error:

```mom
import math.{square, helper}   // ERROR: 'helper' is not public
import math.{square}           // OK
```

---

## Public Fields and Structs

Both the struct itself and each individual field have independent visibility:

```mom
pub struct Config:
    pub host: String    // public field
    pub port: Int       // public field
    password: String    // private field — not accessible outside this module
```

A public struct with private fields is the standard way to enforce invariants:

```mom
// counter.mom
pub struct Counter:
    value: Int          // private — cannot be set directly from outside

pub fn new() -> Counter:
    Counter { value: 0 }

pub fn increment(c: Counter) -> Counter:
    Counter { value: c.value + 1 }

pub fn get(c: Counter) -> Int:
    c.value
```

From outside:

```mom
import counter.{Counter, new, increment, get}

fn main():
    let c = new()
    let c = increment(c)
    print(get(c))          // 1
    // c.value = 99        // ERROR: 'value' is a private field
```

---

## Private Methods

Methods in an `impl` block are private by default:

```mom
pub struct Stack:
    items: [Int]
    size: Int

impl Stack:
    fn check_bounds(self, i: Int):   // private helper
        if i < 0 or i >= self.size:
            panic("index out of bounds")

    pub fn push(self, v: Int) -> Stack:
        Stack { items: push(self.items, v), size: self.size + 1 }

    pub fn pop(self) -> Option[Int]:
        if self.size == 0:
            None
        else:
            Some(self.items[self.size - 1])
```

---

## Module-Level Private Items

Anything declared inside a `module` block without `pub` is private to that module:

```mom
module net:
    // private implementation detail
    fn raw_connect(host: String, port: Int) -> Result[Socket, String]:
        // ...

    pub fn connect(addr: String) -> Result[Connection, String]:
        // uses raw_connect internally
```

---

## Constants

Private constants are useful for implementation details:

```mom
// connection.mom
const TIMEOUT_MS: Int = 5000      // private default timeout

pub fn connect(host: String) -> Result[Socket, String]:
    connect_with_timeout(host, TIMEOUT_MS)
```

Public constants are part of the module's API:

```mom
pub const MAX_PACKET_SIZE: Int = 65535
pub const VERSION: String = "1.0"
```

---

## Visibility at a Glance

| Declaration | Default visibility | Make public with |
|---|---|---|
| `fn foo()` | Private | `pub fn foo()` |
| `struct Foo { field: T }` | Private struct + private field | `pub struct Foo { pub field: T }` |
| `enum Foo { Bar }` | Private | `pub enum Foo { Bar }` |
| `const FOO: T = ...` | Private | `pub const FOO: T = ...` |
| `impl Foo { fn method() }` | Private method | `pub fn method()` in impl |

---

## Summary

- **Nothing is public by default** — this is the safe default.
- Use `pub` explicitly on each item you want to expose.
- Struct fields are individually controllable — a `pub` struct can have private fields.
- Private fields enforce invariants: the outside world can only modify state through your public API.
