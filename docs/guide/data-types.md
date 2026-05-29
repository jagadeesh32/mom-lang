# Data Types

Mom is **statically typed** — every value has a type known at compile time. The compiler infers types for local bindings so you rarely write them explicitly.

---

## Scalar Types

### `Int` — 64-bit signed integer

The default integer type. Holds values from −9,223,372,036,854,775,808 to 9,223,372,036,854,775,807.

```mom
let age: Int = 30
let temperature = -15         // inferred as Int
let million = 1_000_000       // underscores allowed for readability
```

**Sized variants** (planned for native backend):

| Type | Bits | Range |
|---|---|---|
| `Int` | 64 | −2⁶³ to 2⁶³−1 |
| `Int32` | 32 | −2³¹ to 2³¹−1 |
| `Int16` | 16 | −32,768 to 32,767 |
| `Int8` | 8 | −128 to 127 |
| `UInt` | 64 | 0 to 2⁶⁴−1 |
| `UInt32` | 32 | 0 to 2³²−1 |
| `UInt16` | 16 | 0 to 65,535 |
| `UInt8` / `Byte` | 8 | 0 to 255 |

---

### `Float` — 64-bit IEEE-754 floating point

```mom
let pi: Float = 3.14159
let temp = -2.5               // inferred as Float
let big = 1.0e9               // scientific notation
```

**Sized variants:**

| Type | Bits | Precision |
|---|---|---|
| `Float` | 64 | ~15 significant digits |
| `Float32` | 32 | ~7 significant digits |

> A float literal must contain a decimal point or exponent. Write `1.0`, not `1`.

---

### `Bool` — Boolean

```mom
let active: Bool = true
let done = false
```

Only two values: `true` and `false`. Used in conditions, branching, and logical operations.

---

### `String` — UTF-8 text

```mom
let name: String = "Alice"
let greeting = "Hello, world!"
let empty = ""
```

Strings are **immutable UTF-8** sequences. They support:
- Concatenation with `+`
- Length via `len(s)`
- Comparison with `==`, `!=`
- Escape sequences (see the [Literals](literals.md) page)

```mom
let first = "Hello"
let second = "world"
let combined = first + ", " + second + "!"
print(combined)   // Hello, world!
```

---

### `Char` — Unicode scalar value

A single Unicode code point (U+0000 to U+10FFFF).

```mom
let letter: Char = 'A'
let emoji: Char = '😀'
```

> Single-character string literals use single quotes for `Char`.

---

### `()` — Unit

The unit type has exactly one value: `()`. It is returned by functions that produce no meaningful result (like `print`).

```mom
fn say_hello():       // return type is () by default
    print("Hi")

let result: () = say_hello()   // result is ()
```

---

## Compound Types

### Lists — `[T]`

A dynamically-sized, homogeneous sequence. All elements must be the same type.

```mom
let numbers: [Int] = [1, 2, 3, 4, 5]
let names: [String] = ["Alice", "Bob", "Carol"]
let empty: [Int] = []
```

Access by index (0-based):

```mom
let first = numbers[0]   // 1
let last  = numbers[4]   // 5
```

Operations:

```mom
let n = len(numbers)     // 5
let more = push(numbers, 6)
for x in numbers:
    print(x)
```

---

### Option — `Option[T]`

Represents a value that may or may not be present. No null pointers.

```mom
let found: Option[Int] = Some(42)
let missing: Option[Int] = None
```

Unwrap with `match` or `?`:

```mom
match found:
    Some(x) => print(x)
    None    => print("not found")
```

```mom
fn find_positive(xs: [Int]) -> Option[Int]:
    for x in xs:
        if x > 0: return Some(x)
    None
```

---

### Result — `Result[T, E]`

Represents either success (`Ok`) or failure (`Err`).

```mom
let ok:  Result[Int, String] = Ok(42)
let err: Result[Int, String] = Err("something went wrong")
```

```mom
fn divide(a: Int, b: Int) -> Result[Int, String]:
    if b == 0: Err("division by zero")
    else:      Ok(a / b)

match divide(10, 2):
    Ok(v)  => print(v)      // 5
    Err(e) => print(e)
```

---

### Structs

Named collections of named fields with fixed types:

```mom
struct Point:
    x: Int
    y: Int

struct Person:
    name: String
    age:  Int
    active: Bool
```

Instantiate with a struct literal:

```mom
let p = Point { x: 3, y: 4 }
let alice = Person { name: "Alice", age: 30, active: true }

print(p.x)         // 3
print(alice.name)  // Alice
```

---

### Enums (Sum Types)

A type that is one of several variants. Each variant can carry data.

```mom
enum Direction:
    North
    South
    East
    West

enum Shape:
    Circle(Float)           // radius
    Rectangle(Int, Int)     // width, height
    Point

let d = North
let s = Circle(5.0)
```

Use `match` to extract values:

```mom
fn area(s: Shape) -> Float:
    match s:
        Circle(r)       => 3.14159 * r * r
        Rectangle(w, h) => w * h as Float
        Point           => 0.0
```

---

### Functions as Values — `fn(T, U) -> V`

Functions are first-class values. Their type is written with `fn`:

```mom
let double: fn(Int) -> Int = fn(x: Int) => x * 2
let result = double(5)     // 10
```

---

### References — `&T` / `&mut T`

Borrowed views of a value, checked by the borrow checker. No heap allocation.

```mom
fn print_len(s: &String):
    print(len(s))

fn increment(n: &mut Int):
    n = n + 1
```

---

## Type Aliases

Give a shorter name to an existing type:

```mom
type Bytes = [Byte]
type UserId = Int
type ErrorMsg = String
```

---

## Type Summary Table

| Type | Description | Example literal |
|---|---|---|
| `Int` | 64-bit signed integer | `42`, `-7`, `1_000` |
| `Float` | 64-bit float | `3.14`, `-0.5`, `1.0e6` |
| `Bool` | Boolean | `true`, `false` |
| `String` | UTF-8 text | `"hello"` |
| `Char` | Unicode code point | `'A'`, `'😀'` |
| `()` | Unit | `()` |
| `[T]` | List of T | `[1, 2, 3]` |
| `Option[T]` | Maybe a T | `Some(x)`, `None` |
| `Result[T, E]` | Ok or Err | `Ok(x)`, `Err(e)` |
| `struct Name { … }` | Named product type | `Point { x: 0, y: 0 }` |
| `enum Name { … }` | Named sum type | `Circle(5.0)`, `None` |
| `fn(T) -> U` | Function type | `fn(x: Int) => x * 2` |
| `&T` | Immutable borrow | `&x` |
| `&mut T` | Mutable borrow | `&mut x` |
