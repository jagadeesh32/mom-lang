# Operators

This page documents every operator in Mom, with precedence, associativity, and examples.

---

## Precedence Table

Operators are listed from **lowest precedence** (evaluated last) to **highest** (evaluated first).

| Level | Operator(s) | Associativity | Description |
|---|---|---|---|
| 1 | `\|\|` | left | Logical OR |
| 2 | `&&` | left | Logical AND |
| 3 | `\|>` | left | Pipeline (thread left into right) |
| 3 | `..` | left | Range |
| 4 | `==` `!=` | left | Equality / inequality |
| 5 | `<` `<=` `>` `>=` | left | Comparison |
| 6 | `+` `-` | left | Addition / subtraction |
| 7 | `*` `/` `%` | left | Multiplication / division / remainder |
| 8 | `!` (unary) | right | Logical NOT |
| 8 | `-` (unary) | right | Arithmetic negation |
| 8 | `spawn` (prefix) | right | Spawn a task |
| 8 | `await` (prefix) | right | Await a future |
| 9 | `()` `.` `[]` `?` | left | Call, field access, index, try |

Parentheses `( )` override precedence at any level.

---

## Arithmetic Operators

All arithmetic operators work on `Int` and `Float`. Mixed `Int`/`Float` arithmetic is a **type error** — convert explicitly.

| Operator | Name | Example | Result |
|---|---|---|---|
| `+` | Addition | `3 + 4` | `7` |
| `-` | Subtraction | `10 - 3` | `7` |
| `*` | Multiplication | `6 * 7` | `42` |
| `/` | Division | `17 / 5` | `3` (integer truncation) |
| `%` | Remainder (modulo) | `17 % 5` | `2` |
| `-x` | Unary negation | `-5` | `-5` |

### Integer division

`/` on `Int` truncates toward zero:

```mom
print(17 / 5)     // 3
print(-17 / 5)    // -3  (not -4)
```

### Float division

`/` on `Float` is standard IEEE-754 division:

```mom
print(17.0 / 5.0)   // 3.4
print(1.0 / 0.0)    // Inf  (positive infinity)
```

### Modulo

`%` gives the remainder. The sign matches the **dividend** (the left operand):

```mom
print(17 % 5)     //  2
print(-17 % 5)    // -2
```

---

## Comparison Operators

Return `Bool`. Work on `Int`, `Float`, `String`, `Bool`, `Char`, and any type that implements `Ord`.

| Operator | Meaning | Example |
|---|---|---|
| `==` | Equal | `x == 5` |
| `!=` | Not equal | `x != 0` |
| `<` | Less than | `a < b` |
| `<=` | Less than or equal | `a <= b` |
| `>` | Greater than | `a > b` |
| `>=` | Greater than or equal | `a >= b` |

```mom
print(3 == 3)     // true
print(3 != 4)     // true
print(5 > 10)     // false
print("abc" < "abd")   // true  (lexicographic)
```

---

## Logical Operators

Operate on `Bool`. Both `&&` and `||` **short-circuit**: the right operand is not evaluated if the result is already determined by the left.

| Operator | Name | Example | Behavior |
|---|---|---|---|
| `&&` | Logical AND | `a && b` | `true` only if both are `true`; evaluates `b` only if `a` is `true` |
| `\|\|` | Logical OR | `a \|\| b` | `true` if either is `true`; evaluates `b` only if `a` is `false` |
| `!x` | Logical NOT | `!flag` | `true` if `flag` is `false`, and vice versa |
| `not x` | Logical NOT (keyword) | `not flag` | same as `!flag` |
| `and` | Logical AND (keyword) | `a and b` | same as `&&` |
| `or` | Logical OR (keyword) | `a or b` | same as `\|\|` |

```mom
let x = 5
print(x > 0 && x < 10)    // true
print(x == 0 || x > 3)    // true
print(not (x == 5))        // false
```

---

## Assignment Operator

`=` assigns a new value to a `let mut` binding:

```mom
let mut x = 0
x = x + 1      // assignment
x = 100        // reassignment
```

Assignment to an immutable binding is a compile error:

```mom
let y = 5
y = 10    // ERROR: cannot assign to immutable binding 'y'
```

### Field Assignment

```mom
struct Counter { n: Int }
let mut c = Counter { n: 0 }
c.n = c.n + 1
```

> There are no compound assignment operators (`+=`, `-=`, etc.) in the current version.

---

## Pipeline Operator `|>`

Threads a value through a sequence of function calls, left to right. Avoids deeply nested calls.

```mom
// Without pipeline:
let result = compile(validate(parse(input)))

// With pipeline:
let result = input |> parse |> validate |> compile
```

With arguments, the pipelined value is passed as the **first argument**:

```mom
let result = items |> filter(fn(x) => x > 0) |> map(fn(x) => x * 2)
```

---

## Range Operator `..`

Creates a range `lo..hi` representing the half-open interval `[lo, hi)`:

```mom
for i in 0..5:
    print(i)    // prints 0, 1, 2, 3, 4  (not 5)
```

Ranges can also be stored and passed:

```mom
let r = 1..10
```

---

## Try Operator `?`

Propagates errors out of a `Result[T, E]` or `Option[T]`. Must be used inside a function that returns `Result` or `Option`.

```mom
fn read_config() -> Result[Config, String]:
    let text = read_file("config.toml")?    // returns Err early if read fails
    let cfg = parse_toml(text)?             // returns Err early if parse fails
    Ok(cfg)
```

For `Option[T]`:
```mom
fn first_positive(xs: [Int]) -> Option[Int]:
    let first = xs.first()?    // returns None early if list is empty
    if first > 0 { Some(first) } else { None }
```

---

## Indexing Operator `[]`

Access an element of a list by 0-based index. Out-of-bounds indexing panics at runtime.

```mom
let xs = [10, 20, 30]
print(xs[0])    // 10
print(xs[2])    // 30
// xs[3]        // panic: index 3 out of bounds for list of length 3
```

---

## Field Access Operator `.`

Access a field of a struct or call a method:

```mom
struct Point { x: Int, y: Int }
let p = Point { x: 3, y: 4 }
print(p.x)    // 3
print(p.y)    // 4
```

Method call:

```mom
let s = "hello"
print(s.len())       // method call (if len is a method)
print(len(s))        // equivalent built-in call
```

---

## String Concatenation

`+` on strings produces a new concatenated string:

```mom
let a = "Hello"
let b = "world"
print(a + ", " + b + "!")    // Hello, world!
```

---

## Operator Overloading

Operator overloading is not supported in the current version. All operators have fixed behavior for built-in types.

---

## Full Worked Example

```mom
fn main():
    let a = 15
    let b = 4

    // Arithmetic
    print(a + b)     // 19
    print(a - b)     // 11
    print(a * b)     // 60
    print(a / b)     // 3   (integer truncation)
    print(a % b)     // 3   (remainder)

    // Comparison
    print(a > b)     // true
    print(a == b)    // false

    // Logical
    print(a > 10 && b < 10)    // true
    print(a < 10 || b < 10)    // true
    print(!false)              // true

    // Unary
    print(-a)        // -15

    // Pipeline
    let doubled = a |> fn(x: Int) => x * 2
    print(doubled)   // 30

    // Range in a for loop
    for i in 0..4:
        print(i)     // 0, 1, 2, 3
```
