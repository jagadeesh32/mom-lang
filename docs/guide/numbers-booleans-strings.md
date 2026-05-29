# Numbers, Booleans, Strings, and Control Flow

This chapter covers the four everyday value types in depth, then shows how control flow expressions interact with them.

---

## Numbers

### Integer Arithmetic

```mom
let a = 10
let b = 3

print(a + b)    // 13
print(a - b)    // 7
print(a * b)    // 30
print(a / b)    // 3   — integer division, truncates toward zero
print(a % b)    // 1   — remainder
print(-a)       // -10 — unary negation
```

### Float Arithmetic

```mom
let x = 10.0
let y = 3.0

print(x + y)    // 13.0
print(x - y)    // 7.0
print(x * y)    // 30.0
print(x / y)    // 3.3333333333333335
print(x % y)    // 1.0 — float remainder
```

### Mixing Int and Float

Int and Float cannot be mixed directly. Convert explicitly:

```mom
let n: Int = 5
let f: Float = 2.5
print(float(n) + f)    // 7.5
print(n + int(f))      // 7  (int(2.5) = 2)
```

### Integer Division vs Float Division

```mom
print(7 / 2)      // 3   (integer: truncates)
print(7.0 / 2.0)  // 3.5 (float: exact)
```

### Useful Math Functions

```mom
print(abs(-5))          // 5
print(min(3, 7))        // 3
print(max(3, 7))        // 7
print(pow(2.0, 10.0))   // 1024.0
print(sqrt(16.0))       // 4.0
print(floor(3.9))       // 3.0
print(ceil(3.1))        // 4.0
print(round(3.5))       // 4.0
```

### Number Comparisons

```mom
print(5 == 5)    // true
print(5 != 6)    // true
print(5 < 10)    // true
print(10 >= 10)  // true
print(3 > 5)     // false
```

### Number to String

```mom
let n = 42
let s = str(n)         // "42"
let f = 3.14
let sf = str(f)        // "3.14"
print("n = " + str(n)) // n = 42
```

---

## Booleans

The only two Boolean values are `true` and `false`.

### Logical Operators

```mom
print(true && true)     // true
print(true && false)    // false
print(false || true)    // true
print(false || false)   // false
print(!true)            // false
print(!false)           // true
```

### Short-Circuit Evaluation

`&&` stops at the first `false`:

```mom
fn risky() -> Bool:
    print("risky called")
    true

print(false && risky())   // prints nothing — risky() never runs
print(true && risky())    // prints "risky called", then "true"
```

`||` stops at the first `true`:

```mom
print(true || risky())    // prints nothing — risky() never runs
print(false || risky())   // prints "risky called", then "true"
```

### Boolean to String

```mom
print(str(true))    // "true"
print(str(false))   // "false"
```

### Boolean in Conditions

```mom
let authenticated = true
let is_admin = false

if authenticated && is_admin:
    print("admin panel")
elif authenticated:
    print("user panel")
else:
    print("login required")
```

---

## Strings

### Creating Strings

```mom
let hello = "Hello, world!"
let empty = ""
let multiline_hint = "line 1\nline 2\nline 3"
```

### String Length

```mom
let s = "hello"
print(len(s))    // 5
```

> `len()` returns the **byte** count. For ASCII strings, this equals the character count. For multi-byte UTF-8, use `len(chars(s))` for the character count.

### String Concatenation

```mom
let first = "Hello"
let last  = "world"
print(first + ", " + last + "!")   // Hello, world!
```

### String Comparison

```mom
print("apple" == "apple")   // true
print("apple" != "orange")  // true
print("apple" < "banana")   // true  (lexicographic)
print("z" > "a")            // true
```

### Common String Operations

```mom
let s = "  Hello, World!  "

print(upper(s))             // "  HELLO, WORLD!  "
print(lower(s))             // "  hello, world!  "
print(strip(s))             // "Hello, World!"
print(lstrip(s))            // "Hello, World!  "
print(rstrip(s))            // "  Hello, World!"

print(starts_with(s, "  Hello"))   // true
print(ends_with(s, "!  "))        // true
print(contains(s, "World"))        // true

let words = split("one,two,three", ",")
// ["one", "two", "three"]

let rejoined = join(words, " - ")
// "one - two - three"

print(replace("Hello, world!", "world", "Mom"))
// "Hello, Mom!"
```

### Extracting Substrings

```mom
let s = "Hello, world!"
print(substr(s, 7, 5))    // "world"  — start=7, length=5
print(find(s, "world"))   // 7
print(find(s, "xyz"))     // -1  (not found)
```

### Splitting and Iterating Characters

```mom
let word = "hello"
let chars_list = chars(word)
// ["h", "e", "l", "l", "o"]

for ch in chars("Mom"):
    print(ch)
// M
// o
// m
```

### String to Number

```mom
match parse_int("42"):
    Some(n) => print(n * 2)    // 84
    None    => print("not an int")

match parse_float("3.14"):
    Some(f) => print(f)
    None    => print("not a float")
```

### String Formatting

There is no format-string syntax yet (`format!` is planned). Build strings with `+` and `str()`:

```mom
let name = "Alice"
let score = 95
let msg = "Player " + name + " scored " + str(score) + " points."
print(msg)
// Player Alice scored 95 points.
```

---

## Control Flow

Control flow in Mom is mostly **expressions** — they return values.

### `if` / `elif` / `else`

```mom
// Statement form
if x > 0:
    print("positive")
elif x < 0:
    print("negative")
else:
    print("zero")

// Expression form — produces a value
let sign = if x > 0 { 1 } elif x < 0 { -1 } else { 0 }
print(sign)
```

The `then` and `else` branches must have the same type when used as an expression.

### `while`

```mom
let mut i = 0
while i < 5:
    print(i)
    i = i + 1
```

```mom
// Loop with break
let mut sum = 0
let mut n = 1
while true:
    sum = sum + n
    if sum > 100: break
    n = n + 1
print(n)   // first n where sum > 100
```

### `for` / `for-in`

```mom
// Range loop
for i in 0..5:
    print(i)    // 0 1 2 3 4

// List loop
for name in ["Alice", "Bob", "Carol"]:
    print("Hello, " + name + "!")

// With break and continue
for i in 0..10:
    if i % 2 == 0: continue    // skip evens
    if i > 7: break             // stop after 7
    print(i)                    // prints 1, 3, 5, 7
```

### `match` Expression

```mom
let n = 42
let label = match n:
    0     => "zero"
    1     => "one"
    2..10 => "small"    // range pattern (planned)
    _     => "large"    // wildcard catch-all

print(label)    // large
```

With enum data:

```mom
let maybe = Some(99)
match maybe:
    Some(v) => print("got " + str(v))
    None    => print("nothing")
```

### `return`

Exit a function early with an explicit value:

```mom
fn find_first(xs: [Int], target: Int) -> Option[Int]:
    for i in 0..len(xs):
        if xs[i] == target:
            return Some(i)
    None
```

### `break` and `continue`

```mom
for i in 0..100:
    if i == 10: break       // exit the loop entirely
    if i % 3 == 0: continue // skip to the next iteration
    print(i)
```

### Block Expressions

A `block` creates a new scope and evaluates to its final expression:

```mom
let result = block:
    let a = 10
    let b = 20
    a + b       // block value is 30
print(result)   // 30
```

This is especially useful for creating scoped borrows:

```mom
let mut x = 0
block:
    let m = &mut x
    m = m + 5
// m is out of scope; x can be used freely again
x = x + 1
print(x)   // 6
```
