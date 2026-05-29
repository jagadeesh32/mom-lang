# Structs

A struct groups named fields under a single type. Structs are the primary tool for building product types in Mom.

---

## Declaring a Struct

```ebnf
struct_decl = "struct" IDENT generics? "{" struct_fields? "}"
struct_field = visibility? IDENT ":" type
```

Fields are listed inside a brace block, separated by commas or newlines. Each field has a name and a type. Trailing commas are allowed.

```mom
struct Point {
    x: Int,
    y: Int,
}
```

Mom also accepts the indented colon form:

```mom
struct Point:
    x: Int
    y: Int
```

Both forms are equivalent. Examples in this guide use the colon form.

### Visibility

Prefix the struct with `pub` to export it from its module. Individual fields can also be marked `pub`:

```mom
pub struct User:
    pub name: String
    age:      Int       // private — only visible inside this module
```

When a field is private, code outside the module cannot read or write it directly; it must go through methods.

---

## Struct Literals

Create an instance by writing the struct name followed by `{ field: value, ... }`:

```mom
let p = Point { x: 3, y: 4 }
```

Field order is **independent** of declaration order:

```mom
let p = Point { y: 4, x: 3 }   // identical result
```

All fields must be provided; there are no default values.

---

## Field Access

Use `.` to read a field:

```mom
let p = Point { x: 3, y: 4 }
print(p.x)   // 3
print(p.y)   // 4
```

---

## Field Assignment

Fields can be updated only on a **`let mut`** binding:

```mom
let mut c = Counter { n: 0 }
c.n = c.n + 1
print(c.n)   // 1
```

Assigning to a field of an immutable binding is a compile error.

---

## Nested Structs

Fields can be of any type, including other structs:

```mom
struct Rect:
    origin: Point
    width:  Int
    height: Int

let r = Rect {
    origin: Point { x: 0, y: 0 },
    width:  100,
    height: 200,
}

print(r.origin.x)   // 0
```

---

## `impl` Blocks — Methods

Define methods on a struct with an `impl` block:

```ebnf
impl_block = "impl" generics? IDENT "{" impl_method* "}"
impl_method = visibility? "async"? function
```

```mom
impl Point:
    fn shift(self, dx: Int, dy: Int) -> Point:
        Point { x: self.x + dx, y: self.y + dy }
```

### The `self` Parameter

The first parameter `self` receives the struct value. It is a **value receiver** — `self` is a copy of the instance, not a reference.

Because `self` is a plain value, mutation is done by returning a new struct:

```mom
impl Point:
    fn shift(self, dx: Int, dy: Int) -> Point:
        Point { x: self.x + dx, y: self.y + dy }

    fn manhattan(self, other: Point) -> Int:
        let dx = self.x - other.x
        let dy = self.y - other.y
        let ax = if dx < 0 { -dx } else { dx }
        let ay = if dy < 0 { -dy } else { dy }
        ax + ay
```

Call a method with dot notation:

```mom
let origin = Point { x: 0, y: 0 }
let p      = Point { x: 3, y: 4 }
print(p.manhattan(origin))   // 7
```

### Static Methods (No `self`)

Omit `self` to define a static (associated) method:

```mom
impl Point:
    fn origin() -> Point:
        Point { x: 0, y: 0 }
```

Call it as `Point.origin()` — though dispatch syntax follows the same dot-call form once the value is obtained.

---

## Method Chaining

Because methods return new values you can chain calls directly:

```mom
let p = Point { x: 0, y: 0 }
let q = p.shift(1, 2).shift(3, 4)
print(q.x)   // 4
print(q.y)   // 6
```

---

## Generic Structs

Declare type parameters in square brackets after the struct name:

```mom
struct Pair[A, B]:
    first:  A
    second: B
```

Instantiate by providing the concrete types at the literal site:

```mom
let kv = Pair[String, Int] { first: "age", second: 30 }
print(kv.first)    // age
print(kv.second)   // 30
```

Generic `impl` blocks mirror the struct header:

```mom
impl[A, B] Pair[A, B]:
    fn swap(self) -> Pair[B, A]:
        Pair[B, A] { first: self.second, second: self.first }
```

---

## Structs as Function Arguments and Return Types

Structs are passed by value like any other type:

```mom
fn midpoint(a: Point, b: Point) -> Point:
    Point { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }

fn main():
    let a = Point { x: 0, y: 0 }
    let b = Point { x: 4, y: 6 }
    let m = midpoint(a, b)
    print(m.x)   // 2
    print(m.y)   // 3
```

---

## Pattern Matching on Structs

Structs can appear as variant payloads inside enums and are matched structurally. A bare identifier in a pattern binds the whole value:

```mom
enum Shape:
    Circle(Int)       // radius
    Rect(Int, Int)    // width, height

fn area(s: Shape) -> Int:
    match s:
        Circle(r)    => r * r        // bind radius
        Rect(w, h)   => w * h        // bind width and height
```

---

## Full Worked Example

```mom
// structs.mom — struct definition, struct literal, field access, impl methods.

struct Point:
    x: Int
    y: Int

impl Point:
    fn shift(self, dx: Int, dy: Int) -> Point:
        Point { x: self.x + dx, y: self.y + dy }

    fn manhattan(self, other: Point) -> Int:
        let dx = self.x - other.x
        let dy = self.y - other.y
        let ax = if dx < 0 { -dx } else { dx }
        let ay = if dy < 0 { -dy } else { dy }
        ax + ay

fn main():
    let origin = Point { x: 0, y: 0 }
    let p      = Point { x: 3, y: 4 }
    let q      = p.shift(1, 1)
    print(p.manhattan(origin))   // 7
    print(q.x)                   // 4
    print(q.y)                   // 5
```

---

## Quick Reference

| Syntax | Meaning |
|--------|---------|
| `struct Name:` | Declare a struct |
| `pub struct Name:` | Export the struct |
| `pub field: T` | Export an individual field |
| `Name { f: v, ... }` | Struct literal |
| `s.field` | Field read |
| `s.field = expr` | Field write (requires `let mut`) |
| `impl Name:` | Attach methods |
| `fn f(self, ...)` | Instance method |
| `fn f(...)` | Static method |
| `struct Name[T]:` | Generic struct |
