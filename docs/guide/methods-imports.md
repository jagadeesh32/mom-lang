# Methods and Imports

## Methods

Methods are functions that belong to a type. They are defined in `impl` blocks and receive the type's value as their first `self` parameter.

---

### Defining Methods

```mom
struct Rectangle:
    width: Int
    height: Int

impl Rectangle:
    fn area(self) -> Int:
        self.width * self.height

    fn perimeter(self) -> Int:
        2 * (self.width + self.height)

    fn is_square(self) -> Bool:
        self.width == self.height

    fn scale(self, factor: Int) -> Rectangle:
        Rectangle { width: self.width * factor, height: self.height * factor }
```

### Calling Methods

```mom
let r = Rectangle { width: 5, height: 10 }

print(r.area())         // 50
print(r.perimeter())    // 30
print(r.is_square())    // false

let big = r.scale(3)
print(big.width)        // 15
```

---

### Method Chaining

Methods that return the same type (or a modified version) can be chained:

```mom
struct Builder:
    host: String
    port: Int
    debug: Bool

impl Builder:
    fn with_host(self, h: String) -> Builder:
        Builder { host: h, port: self.port, debug: self.debug }

    fn with_port(self, p: Int) -> Builder:
        Builder { host: self.host, port: p, debug: self.debug }

    fn with_debug(self, d: Bool) -> Builder:
        Builder { host: self.host, port: self.port, debug: d }

let config = Builder { host: "", port: 0, debug: false }
    .with_host("localhost")
    .with_port(8080)
    .with_debug(true)
```

---

### Trait Implementation Methods

When a type implements a trait, the trait methods are also callable via dot notation:

```mom
trait Display:
    fn to_string(self) -> String

struct Color:
    r: Int
    g: Int
    b: Int

impl Display for Color:
    fn to_string(self) -> String:
        "rgb(" + str(self.r) + "," + str(self.g) + "," + str(self.b) + ")"

let red = Color { r: 255, g: 0, b: 0 }
print(red.to_string())   // rgb(255,0,0)
```

---

### `pub` on Methods

Methods are private by default. Mark them `pub` to make them accessible from other modules:

```mom
impl Stack:
    fn check_bounds(self, i: Int):   // private
        if i >= self.size: panic("OOB")

    pub fn peek(self) -> Option[Int]:   // public
        if self.size == 0: None
        else: Some(self.items[self.size - 1])
```

---

## Modules

Modules group related declarations under a name.

### Declaring a Module

```mom
module geometry:
    pub struct Point:
        pub x: Float
        pub y: Float

    pub struct Circle:
        pub center: Point
        pub radius: Float

    pub fn distance(a: Point, b: Point) -> Float:
        let dx = a.x - b.x
        let dy = a.y - b.y
        sqrt(dx * dx + dy * dy)
```

### Modules in Separate Files

In a project, modules map to files:

```
src/
├── main.mom
├── geometry.mom        // contains `pub struct Point`, etc.
├── net/
│   ├── http.mom
│   └── tcp.mom
```

---

## Imports

### `import` — bring symbols into scope

```mom
import geometry.{Point, Circle, distance}
```

Now `Point`, `Circle`, and `distance` are available without the `geometry.` prefix:

```mom
let p1 = Point { x: 0.0, y: 0.0 }
let p2 = Point { x: 3.0, y: 4.0 }
print(distance(p1, p2))   // 5.0
```

### Importing all public symbols

```mom
import geometry.*
```

> Wildcard imports make it harder to know where a name comes from. Prefer explicit imports in most cases.

### Qualified access

You can always use the full qualified path without importing:

```mom
let p = geometry.Point { x: 1.0, y: 2.0 }
let d = geometry.distance(p, p)
```

### `use` — alias syntax (identical to `import`)

```mom
use std.io.{read_file, write_file}
use std.math.{gcd, lcm}
```

`use` and `import` are interchangeable keywords.

### Renaming on import

```mom
import std.crypto.{adler32 as checksum}
print(checksum("hello"))
```

---

## Standard Library Imports

```mom
import std.core.{identity, min, max, clamp}
import std.fmt.{pad_left, join}
import std.math.{gcd, factorial, fib}
import std.io.{LineBuffer}
import std.log.{Logger, logger_for}
import std.async.{compute, join_all_int}
import std.actor.{CounterMsg, run_counter}
import std.net.{Address, Request, Response, dispatch}
import std.serde.{encode_bool, encode_int, encode_string}
import std.crypto.{adler32, poly_hash, hex_byte}
import std.sync.{Mutex, Atomic}
import std.os.{env_or, sleep_ms}
import std.test.{TestStats, assert_eq_int, assert_true}
```

---

## Module Visibility Summary

| Declaration | Visible within module | Visible outside module |
|---|---|---|
| `fn foo()` | Yes | No |
| `pub fn foo()` | Yes | Yes |
| `struct Foo { field: T }` | Struct visible if `pub`; field is private | Field is not accessible |
| `pub struct Foo { pub field: T }` | Yes | Both struct and field accessible |
| `const FOO` | Yes | No |
| `pub const FOO` | Yes | Yes |

---

## Example: A Complete Module

```mom
// math_utils.mom

pub const TAU: Float = 6.28318530717958

pub fn clamp(value: Int, lo: Int, hi: Int) -> Int:
    if value < lo: lo
    elif value > hi: hi
    else: value

pub fn lerp(a: Float, b: Float, t: Float) -> Float:
    a + t * (b - a)

fn helper_square(x: Float) -> Float:   // private
    x * x

pub fn hypotenuse(a: Float, b: Float) -> Float:
    sqrt(helper_square(a) + helper_square(b))
```

Using it:

```mom
import math_utils.{clamp, lerp, hypotenuse, TAU}

fn main():
    print(clamp(150, 0, 100))       // 100
    print(lerp(0.0, 10.0, 0.5))    // 5.0
    print(hypotenuse(3.0, 4.0))    // 5.0
    print(TAU)                       // 6.28318...
```
