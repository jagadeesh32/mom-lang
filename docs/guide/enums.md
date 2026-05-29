# Enums (Sum Types)

An enum defines a type that can be exactly one of several named variants at any given time. Enums are Mom's primary tool for modeling alternatives, optional values, and errors.

---

## Declaring an Enum

```ebnf
enum_decl = "enum" IDENT generics? "{" variants? "}"
variant   = IDENT ( "(" type_list? ")" )?
```

```mom
enum Direction:
    North,
    South,
    East,
    West,
```

Trailing commas are optional. The colon form (above) and brace form (`enum Direction { ... }`) are both valid.

### Visibility

Prefix with `pub` to export:

```mom
pub enum Status:
    Active,
    Inactive,
    Suspended,
```

---

## Nullary Variants (No Data)

A variant with no parentheses carries no payload. It behaves like a named constant of the enum type:

```mom
enum Color:
    Red,
    Green,
    Blue,

let c = Color.Red
```

> Nullary variants are also used for flags, sentinels, and states.

---

## Payload Variants (Single Field)

A variant can carry one value of any type:

```mom
enum Message:
    Text(String),
    Number(Int),
    Flag(Bool),
```

Construct by calling the variant like a function:

```mom
let m = Message.Text("hello")
let n = Message.Number(42)
```

---

## Multi-Field Variants

A variant can carry more than one value. The payload is a comma-separated list of types:

```mom
enum Shape:
    Circle(Int),           // radius
    Rect(Int, Int),        // width, height
    Triangle(Int, Int, Int), // three sides
```

```mom
let s = Shape.Rect(10, 20)
```

---

## The Built-in `Option[T]`

`Option[T]` represents a value that may or may not be present. Its definition is equivalent to:

```mom
enum Option[T]:
    Some(T),
    None,
```

Use it as a return type whenever a function can legitimately return nothing:

```mom
fn first(xs: [Int]) -> Option[Int]:
    if len(xs) == 0 { None } else { Some(xs[0]) }
```

Unwrap with `match`:

```mom
match first([7, 8, 9]):
    Some(x) => print(x)   // 7
    None    => print(0)
```

---

## The Built-in `Result[T, E]`

`Result[T, E]` represents either success or failure. Its definition is equivalent to:

```mom
enum Result[T, E]:
    Ok(T),
    Err(E),
```

Functions that can fail return `Result`:

```mom
fn parse(value: Int) -> Result[Int, String]:
    if value < 0:
        Err("negative")
    else:
        Ok(value * 2)
```

The `?` operator propagates an `Err` automatically:

```mom
fn doubled_plus_one(value: Int) -> Result[Int, String]:
    let inner = parse(value)?   // returns Err early if parse fails
    Ok(inner + 1)
```

Unwrap with `match`:

```mom
match doubled_plus_one(5):
    Ok(v)  => print(v)   // 11
    Err(e) => print(e)
```

---

## Constructing Variant Values

| Situation | Syntax |
|-----------|--------|
| Nullary variant | `EnumName.VariantName` |
| Single-payload variant | `EnumName.VariantName(value)` |
| Multi-field variant | `EnumName.VariantName(v1, v2)` |
| Built-in `Option` / `Result` | `Some(x)`, `None`, `Ok(x)`, `Err(e)` |

Built-in variants (`Some`, `None`, `Ok`, `Err`) are available without qualification.

---

## Pattern Matching on Enums

Full coverage in [pattern-matching.md](./pattern-matching.md). A brief example:

```mom
fn describe(c: Color) -> String:
    match c:
        Color.Red   => "red"
        Color.Green => "green"
        Color.Blue  => "blue"

fn area(s: Shape) -> Int:
    match s:
        Shape.Circle(r)    => r * r
        Shape.Rect(w, h)   => w * h
        _                  => 0
```

The compiler requires the match to be exhaustive — every possible variant must be handled (or a wildcard `_` must appear).

---

## Nested Enum Variants

A variant's payload can itself be an enum:

```mom
enum Tree:
    Leaf(Int),
    Node(Tree, Tree),
```

Constructing:

```mom
let t = Tree.Node(Tree.Leaf(1), Tree.Leaf(2))
```

Matching recursively:

```mom
fn sum(t: Tree) -> Int:
    match t:
        Tree.Leaf(n)    => n
        Tree.Node(l, r) => sum(l) + sum(r)
```

---

## Generic Enums

Declare type parameters in square brackets:

```mom
enum Either[L, R]:
    Left(L),
    Right(R),
```

```mom
let x: Either[Int, String] = Either.Left(42)
let y: Either[Int, String] = Either.Right("hello")
```

---

## Using Enums as State Machines

Enums model states precisely because only one variant is active at a time:

```mom
enum TrafficLight:
    Red,
    Yellow,
    Green,

fn next(light: TrafficLight) -> TrafficLight:
    match light:
        TrafficLight.Red    => TrafficLight.Green
        TrafficLight.Green  => TrafficLight.Yellow
        TrafficLight.Yellow => TrafficLight.Red

fn main():
    let mut light = TrafficLight.Red
    light = next(light)   // Green
    light = next(light)   // Yellow
    light = next(light)   // Red
    print("cycled")
```

---

## `impl` Blocks on Enums

Attach methods to enums the same way as structs:

```mom
impl TrafficLight:
    fn is_stop(self) -> Bool:
        match self:
            TrafficLight.Red => true
            _                => false

    fn next(self) -> TrafficLight:
        match self:
            TrafficLight.Red    => TrafficLight.Green
            TrafficLight.Green  => TrafficLight.Yellow
            TrafficLight.Yellow => TrafficLight.Red

fn main():
    let light = TrafficLight.Red
    print(light.is_stop())         // true
    print(light.next().is_stop())  // false
```

---

## Full Worked Example

```mom
// option_result.mom — built-in Option and Result, the ? operator.

fn parse(value: Int) -> Result[Int, String]:
    if value < 0:
        Err("negative")
    else:
        Ok(value * 2)

fn doubled_plus_one(value: Int) -> Result[Int, String]:
    let inner = parse(value)?
    Ok(inner + 1)

fn first(xs: [Int]) -> Option[Int]:
    if len(xs) == 0 { None } else { Some(xs[0]) }

fn main():
    match doubled_plus_one(5):
        Ok(v)  => print(v)     // 11
        Err(e) => print(e)

    match doubled_plus_one(-1):
        Ok(v)  => print(v)
        Err(e) => print(e)     // negative

    match first([7, 8, 9]):
        Some(x) => print(x)   // 7
        None    => print(0)
```

---

## Quick Reference

| Syntax | Meaning |
|--------|---------|
| `enum Name:` | Declare an enum |
| `pub enum Name:` | Export the enum |
| `Variant,` | Nullary variant |
| `Variant(T),` | Single-payload variant |
| `Variant(T1, T2),` | Multi-field variant |
| `enum Name[T]:` | Generic enum |
| `Name.Variant` | Construct nullary |
| `Name.Variant(v)` | Construct with payload |
| `match e { V(x) => ... }` | Destructure in match |
| `expr?` | Propagate `Err` / `None` |
