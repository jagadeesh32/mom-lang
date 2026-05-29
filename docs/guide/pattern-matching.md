# Pattern Matching

`match` tests a value against a sequence of patterns and executes the first arm that matches. The compiler requires every possible case to be covered — an incomplete match is a compile error.

---

## Syntax

```ebnf
match_expr   = "match" expression "{" match_arm ( "," match_arm )* ","? "}"
match_arm    = pattern "=>" expression

pattern      = "_"
             | IDENT ( "(" pattern_list? ")" )?
             | INT | FLOAT | STRING
             | "-" ( INT | FLOAT )
             | "true" | "false"
             | "(" ")"
pattern_list = pattern ( "," pattern )* ","?
```

Arms are separated by commas (or newlines in the indented form). The last comma is optional.

---

## Pattern Kinds

### Wildcard `_`

Matches any value and discards it. Used as the default/catch-all arm:

```mom
fn classify(code: Int) -> String:
    match code:
        200 => "ok"
        404 => "missing"
        _   => "error"
```

`_` never binds — the matched value is unreachable in the arm body.

---

### Named Binding

A bare identifier (not a known variant name) binds the whole scrutinee to that name:

```mom
fn describe(n: Int) -> String:
    match n:
        0   => "zero"
        1   => "one"
        val => "other"    // val is bound to n
```

The bound name is in scope for the arm's expression body.

---

### Nullary Variant Pattern

A variant name with no parentheses matches that variant:

```mom
enum Color: Red, Green, Blue,

fn label(c: Color) -> String:
    match c:
        Color.Red   => "red"
        Color.Green => "green"
        Color.Blue  => "blue"
```

For built-in variants `None`, `Ok`, `Err`, `Some` the qualification is optional.

---

### Variant with Single Payload — `Some(x)`, `Circle(r)`

Wrap a sub-pattern in parentheses to destructure the payload:

```mom
fn double_if_some(opt: Option[Int]) -> Int:
    match opt:
        Some(n) => n * 2
        None    => 0
```

`n` is bound to the inner value when the `Some` arm fires.

---

### Multi-Field Variant — `Two(a, b)`, `Rect(w, h)`

List sub-patterns separated by commas inside the parentheses:

```mom
enum Shape: Circle(Int), Rect(Int, Int),

fn area(s: Shape) -> Int:
    match s:
        Shape.Circle(r)  => r * r
        Shape.Rect(w, h) => w * h
```

Each sub-pattern binds independently.

---

### Nested Patterns — `Wrap(A(n))`, `Some(Circle(r))`

Sub-patterns can themselves be patterns, to arbitrary depth:

```mom
enum Inner: A(Int), B(Int),
enum Outer: Wrap(Inner),

fn unwrap_a(o: Outer) -> Int:
    match o:
        Outer.Wrap(Inner.A(n)) => n
        _                      => 0
```

Matching `Option[Shape]`:

```mom
fn circle_area(opt: Option[Shape]) -> Int:
    match opt:
        Some(Shape.Circle(r)) => r * r
        _                     => 0
```

---

### Literal Patterns

#### Integer

```mom
match code:
    200 => "ok"
    404 => "missing"
    500 => "server error"
    _   => "unknown"
```

#### Negative Integer

Use `-` before the literal:

```mom
match n:
    -1  => "minus one"
    0   => "zero"
    _   => "positive"
```

#### Float

```mom
match ratio:
    0.0 => "empty"
    1.0 => "full"
    _   => "partial"
```

#### Bool

```mom
match flag:
    true  => "on"
    false => "off"
```

#### String

```mom
match cmd:
    "quit" => "bye"
    "help" => show_help()
    _      => "unknown command"
```

#### Inside a Variant — `Val(0)`, `Val(true)`

Literal patterns compose inside variant patterns:

```mom
enum Event: Key(Int), Flag(Bool),

match event:
    Event.Key(0)     => "enter"
    Event.Key(27)    => "escape"
    Event.Flag(true) => "on"
    _                => "other"
```

---

### Unit Pattern `()`

Matches the unit type `()`:

```mom
fn handle(result: Result[(), String]) -> String:
    match result:
        Ok(()) => "done"
        Err(e) => e
```

---

## Exhaustiveness

The compiler checks that every possible value is covered. A non-exhaustive match is a compile error:

```mom
// ERROR — Blue is not handled
fn label(c: Color) -> String:
    match c:
        Color.Red   => "red"
        Color.Green => "green"
        // Color.Blue missing!
```

Fix by adding the missing arm or a wildcard:

```mom
fn label(c: Color) -> String:
    match c:
        Color.Red   => "red"
        Color.Green => "green"
        _           => "other"
```

---

## Wildcard Catch-All `_`

`_` as the final arm covers every remaining case without binding:

```mom
match status:
    200 => "ok"
    _   => "not ok"
```

---

## Named Catch-All

A bare identifier as the final arm covers every remaining case and binds the value:

```mom
match status:
    200  => "ok"
    code => "unexpected: " + code
```

---

## Match as an Expression

`match` produces a value. All arms must evaluate to the same type:

```mom
let label = match status:
    200 => "ok"
    404 => "not found"
    _   => "error"
print(label)
```

---

## Match as a Statement

When the arm bodies perform side effects and return `()`, the match is used as a statement:

```mom
match event:
    Event.Key(13) => print("enter pressed")
    Event.Key(27) => print("escape pressed")
    _             => ()
```

---

## Match Arm Bodies with Assignment

An arm body can be an assignment expression. This is how match is used to drive mutable state:

```mom
let mut count = 0

match opt:
    Some(n) => count = count + n
    None    => ()

print(count)
```

---

## Matching `Option[T]`

```mom
fn first(xs: [Int]) -> Option[Int]:
    if len(xs) == 0 { None } else { Some(xs[0]) }

fn main():
    match first([7, 8, 9]):
        Some(x) => print(x)   // 7
        None    => print(0)

    match first([]):
        Some(x) => print(x)
        None    => print(0)   // 0
```

---

## Matching `Result[T, E]`

```mom
fn parse(value: Int) -> Result[Int, String]:
    if value < 0 { Err("negative") } else { Ok(value * 2) }

fn main():
    match parse(5):
        Ok(v)  => print(v)    // 10
        Err(e) => print(e)

    match parse(-3):
        Ok(v)  => print(v)
        Err(e) => print(e)    // negative
```

---

## Pipeline with Match

`match` is an expression, so it composes in pipelines:

```mom
fn double(n: Int) -> Int: n * 2

fn main():
    let result = parse(5)
    let label = match result:
        Ok(v)  => v |> double
        Err(_) => -1
    print(label)   // 20
```

---

## Practical Examples

### State Machine

```mom
enum TrafficLight: Red, Yellow, Green,

fn next(light: TrafficLight) -> TrafficLight:
    match light:
        TrafficLight.Red    => TrafficLight.Green
        TrafficLight.Green  => TrafficLight.Yellow
        TrafficLight.Yellow => TrafficLight.Red

fn main():
    let mut light = TrafficLight.Red
    light = next(light)
    light = next(light)
    light = next(light)
    // back to Red
```

### Command Parsing

```mom
enum Cmd:
    Quit,
    Move(Int, Int),
    Print(String),

fn run(cmd: Cmd):
    match cmd:
        Cmd.Quit          => print("quitting")
        Cmd.Move(x, y)    => print("move to " + x + "," + y)
        Cmd.Print(msg)    => print(msg)

fn main():
    run(Cmd.Move(10, 20))
    run(Cmd.Print("hello"))
    run(Cmd.Quit)
```

### Error Dispatch

```mom
enum AppError:
    NotFound(String),
    Unauthorized,
    Internal(Int),

fn handle(e: AppError):
    match e:
        AppError.NotFound(path)  => print("404: " + path)
        AppError.Unauthorized    => print("401")
        AppError.Internal(code)  => print("500 code " + code)

fn main():
    handle(AppError.NotFound("/api/user"))
    handle(AppError.Unauthorized)
    handle(AppError.Internal(9001))
```

---

## Pattern Summary

| Pattern | Syntax | Binds |
|---------|--------|-------|
| Wildcard | `_` | nothing |
| Named binding | `name` | whole value as `name` |
| Nullary variant | `Enum.Variant` | nothing |
| Single-payload variant | `Enum.Variant(p)` | sub-pattern `p` |
| Multi-field variant | `Enum.Variant(p1, p2)` | each sub-pattern |
| Nested | `Outer.V(Inner.A(n))` | deepest bindings |
| Integer literal | `42` | nothing |
| Negative literal | `-5` | nothing |
| Float literal | `3.14` | nothing |
| Bool literal | `true` / `false` | nothing |
| String literal | `"text"` | nothing |
| Unit | `()` | nothing |
| Literal inside variant | `Variant(0)` | nothing |
| Wildcard catch-all | `_` (final arm) | nothing |
| Named catch-all | `x` (final arm) | value as `x` |
