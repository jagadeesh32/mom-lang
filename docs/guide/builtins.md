# Built-in Functions

Mom's built-in functions are available in every program without any import. They cover I/O, type conversion, math, strings, lists, higher-order operations, and system interaction.

---

## I/O

### `print(value)`

Print a value to stdout followed by a newline.

```mom
print(42)            // 42
print(3.14)          // 3.14
print(true)          // true
print("hello")       // hello
print(())            // (unit value, prints nothing visible)
```

Works on: `Int`, `Float`, `Bool`, `String`, `()`.

> To print an `Int` as part of a string, convert it first: `print("n = " + str(n))`

### `input() -> String`

Read one line from stdin. Returns the line without the trailing newline.

```mom
let name = input()
print("Hello, " + name + "!")
```

---

## Type Conversion

### `str(value) -> String`

Convert any printable value to its string representation.

```mom
str(42)        // "42"
str(3.14)      // "3.14"
str(true)      // "true"
str(false)     // "false"
```

### `int(value) -> Int`

Convert a `Float` to `Int` by truncating toward zero. Also usable on `Bool` (`true`→1, `false`→0).

```mom
int(3.9)      // 3
int(-2.7)     // -2
int(true)     // 1
int(false)    // 0
```

### `float(value) -> Float`

Convert an `Int` to `Float`.

```mom
float(5)      // 5.0
float(-3)     // -3.0
```

### `bool(value) -> Bool`

Truthiness conversion. `0`, `0.0`, `""`, empty lists, and `None` are `false`; everything else is `true`.

```mom
bool(0)       // false
bool(1)       // true
bool("")      // false
bool("x")     // true
```

### `to_string(value) -> String`

Alias for `str`.

---

## Parsing

### `parse_int(s: String) -> Option[Int]`

Parse a decimal integer string. Returns `None` on failure.

```mom
parse_int("42")     // Some(42)
parse_int("-5")     // Some(-5)
parse_int("abc")    // None
parse_int("3.14")   // None
```

### `parse_float(s: String) -> Option[Float]`

Parse a float string. Returns `None` on failure.

```mom
parse_float("3.14")   // Some(3.14)
parse_float("abc")    // None
```

### `parse_bool(s: String) -> Option[Bool]`

Accepts `"true"` and `"false"` only.

```mom
parse_bool("true")    // Some(true)
parse_bool("false")   // Some(false)
parse_bool("yes")     // None
```

---

## Math

### `abs(x) -> T`

Absolute value. Works on `Int` and `Float`.

```mom
abs(-5)      // 5
abs(-3.14)   // 3.14
abs(7)       // 7
```

### `min(a, b) -> T` / `max(a, b) -> T`

Minimum / maximum of two comparable values.

```mom
min(3, 7)     // 3
max(3, 7)     // 7
min(1.5, 2.5) // 1.5
```

### `pow(base: Float, exp: Float) -> Float`

Exponentiation.

```mom
pow(2.0, 10.0)   // 1024.0
pow(9.0, 0.5)    // 3.0  (square root)
```

### `sqrt(x: Float) -> Float`

Square root.

```mom
sqrt(16.0)    // 4.0
sqrt(2.0)     // 1.4142135623730951
```

### `floor(x: Float) -> Float`

Round down to nearest integer value (returns Float).

```mom
floor(3.9)    // 3.0
floor(-3.1)   // -4.0
```

### `ceil(x: Float) -> Float`

Round up to nearest integer value.

```mom
ceil(3.1)     // 4.0
ceil(-3.9)    // -3.0
```

### `round(x: Float) -> Float`

Round to nearest integer (half rounds up).

```mom
round(3.5)    // 4.0
round(3.4)    // 3.0
round(-0.5)   // 0.0
```

---

## Numeric Representations

### `hex(n: Int) -> String`

Convert an integer to lowercase hexadecimal string.

```mom
hex(255)     // "ff"
hex(16)      // "10"
hex(0)       // "0"
```

### `oct(n: Int) -> String`

Convert to octal string.

```mom
oct(8)       // "10"
oct(255)     // "377"
```

### `bin(n: Int) -> String`

Convert to binary string.

```mom
bin(10)      // "1010"
bin(255)     // "11111111"
```

### `ord(c: Char) -> Int`

Unicode code point of a character.

```mom
ord('A')     // 65
ord('€')     // 8364
ord('😀')   // 128512
```

### `chr(n: Int) -> Char`

Character from Unicode code point.

```mom
chr(65)      // 'A'
chr(8364)    // '€'
```

---

## String Operations

### `len(s: String) -> Int`

Number of **bytes** in the string (not characters for non-ASCII).

```mom
len("hello")   // 5
len("")        // 0
len("café")    // 5  (é is 2 bytes in UTF-8)
```

### `upper(s: String) -> String` / `lower(s: String) -> String`

Convert case (ASCII letters only in current version).

```mom
upper("hello")   // "HELLO"
lower("WORLD")   // "world"
```

### `strip(s: String) -> String`

Remove leading and trailing whitespace.

```mom
strip("  hello  ")    // "hello"
strip("\nhello\n")    // "hello"
```

### `lstrip(s) / rstrip(s)`

Strip only left (leading) or right (trailing) whitespace.

### `starts_with(s: String, prefix: String) -> Bool`

```mom
starts_with("hello world", "hello")   // true
starts_with("hello world", "world")   // false
```

### `ends_with(s: String, suffix: String) -> Bool`

```mom
ends_with("hello.mom", ".mom")   // true
```

### `contains(s: String, sub: String) -> Bool`

```mom
contains("hello world", "world")   // true
contains("hello world", "xyz")     // false
```

### `find(s: String, sub: String) -> Int`

Index of first occurrence, or `-1` if not found.

```mom
find("hello world", "world")   // 6
find("hello world", "xyz")     // -1
```

### `replace(s: String, old: String, new: String) -> String`

Replace the first occurrence.

```mom
replace("hello world", "world", "Mom")   // "hello Mom"
```

### `substr(s: String, start: Int, length: Int) -> String`

Extract a substring.

```mom
substr("hello world", 6, 5)   // "world"
```

### `split(s: String, delim: String) -> [String]`

Split into a list.

```mom
split("a,b,c", ",")        // ["a", "b", "c"]
split("hello", "")         // ["h", "e", "l", "l", "o"]
split("  ", " ")           // ["", "", ""]
```

### `join(parts: [String], delim: String) -> String`

Join a list of strings.

```mom
join(["a", "b", "c"], ",")   // "a,b,c"
join(["a", "b", "c"], " ")   // "a b c"
join([], ",")                 // ""
```

### `chars(s: String) -> [String]`

List of single-character strings.

```mom
chars("hello")   // ["h", "e", "l", "l", "o"]
```

---

## List Operations

### `len(xs: [T]) -> Int`

Number of elements.

```mom
len([1, 2, 3])   // 3
len([])          // 0
```

### `push(xs: [T], v: T) -> [T]`

Return a new list with `v` appended (does not mutate).

```mom
let xs = [1, 2, 3]
let ys = push(xs, 4)
// ys = [1, 2, 3, 4],  xs unchanged
```

### `pop(xs: [T]) -> (T, [T])`

Return the last element and the rest of the list.

```mom
let (last, rest) = pop([1, 2, 3])
// last = 3, rest = [1, 2]
```

### `reverse(xs: [T]) -> [T]`

Return a reversed copy.

```mom
reverse([1, 2, 3])   // [3, 2, 1]
```

### `sort(xs: [T]) -> [T]`

Return a sorted copy (ascending).

```mom
sort([3, 1, 4, 1, 5])   // [1, 1, 3, 4, 5]
sort(["banana", "apple", "cherry"])   // ["apple", "banana", "cherry"]
```

### `sum(xs: [Int]) -> Int`

Sum all integers.

```mom
sum([1, 2, 3, 4, 5])   // 15
sum([])                  // 0
```

### `any(xs: [Bool]) -> Bool`

True if at least one element is `true`.

```mom
any([false, true, false])   // true
any([false, false])          // false
```

### `all(xs: [Bool]) -> Bool`

True if all elements are `true`.

```mom
all([true, true, true])    // true
all([true, false, true])   // false
```

---

## Higher-Order Functions

### `map(f: fn(T) -> U, xs: [T]) -> [U]`

Apply a function to each element.

```mom
map(fn(x: Int) => x * 2, [1, 2, 3])       // [2, 4, 6]
map(fn(x: Int) => str(x), [1, 2, 3])      // ["1", "2", "3"]
```

### `filter(f: fn(T) -> Bool, xs: [T]) -> [T]`

Keep elements where `f` returns `true`.

```mom
filter(fn(x: Int) => x > 2, [1, 2, 3, 4])   // [3, 4]
filter(fn(s: String) => len(s) > 3, ["hi", "hello", "world"])   // ["hello", "world"]
```

### `reduce(f: fn(T, T) -> T, xs: [T]) -> T`

Fold the list left. The list must be non-empty.

```mom
reduce(fn(a: Int, b: Int) => a + b, [1, 2, 3, 4])   // 10
reduce(fn(a: Int, b: Int) => max(a, b), [3, 1, 4, 1, 5])   // 5
```

### `enumerate(xs: [T]) -> [(Int, T)]`

List of `(index, value)` pairs.

```mom
enumerate(["a", "b", "c"])
// [(0, "a"), (1, "b"), (2, "c")]
```

### `zip(xs: [T], ys: [U]) -> [(T, U)]`

Pair up elements from two lists (stops at the shorter list).

```mom
zip([1, 2, 3], ["a", "b", "c"])
// [(1, "a"), (2, "b"), (3, "c")]
```

### `range(start: Int, end: Int) -> [Int]`

Generate a list of integers from `start` to `end-1`.

```mom
range(0, 5)   // [0, 1, 2, 3, 4]
range(3, 7)   // [3, 4, 5, 6]
```

---

## Type Predicates

```mom
is_int(42)          // true
is_int(3.14)        // false
is_float(3.14)      // true
is_string("hi")     // true
is_bool(true)       // true
is_list([1, 2])     // true
is_none(None)       // true
type_of(42)         // "Int"
type_of("hi")       // "String"
```

---

## Character Predicates

```mom
is_digit('5')    // true
is_digit('a')    // false
is_alpha('A')    // true
is_alpha('5')    // false
is_alnum('a')    // true
is_alnum('5')    // true
is_alnum('!')    // false
```

---

## System Functions

### `args() -> [String]`

All command-line arguments (index 0 is the program path).

### `getenv(name: String) -> Option[String]`

Read an environment variable.

```mom
match getenv("HOME"):
    Some(h) => print(h)
    None    => print("not set")
```

### `sleep(ms: Int)`

Pause execution for `ms` milliseconds.

```mom
sleep(1000)   // sleep 1 second
```

### `exit(code: Int)`

Exit the process with the given code. Code 0 is success, anything else is failure.

```mom
if error_occurred:
    print("Fatal error")
    exit(1)
```

### `panic(msg: String)`

Abort the program with an error message and non-zero exit code.

```mom
panic("unreachable state reached")
```

### `assert(condition: Bool)`

Panic if `condition` is false. Used for invariant checks in debug builds.

```mom
assert(index >= 0)
assert(len(list) > 0)
```

---

## File I/O

### `read_file(path: String) -> Result[String, String]`

Read an entire file as a string.

```mom
match read_file("data.txt"):
    Ok(content) => print(content)
    Err(e)      => print("Error: " + e)
```

### `write_file(path: String, content: String) -> Result[(), String]`

Write a string to a file (creates or overwrites).

```mom
match write_file("output.txt", "hello\n"):
    Ok(())  => print("written")
    Err(e)  => print("Error: " + e)
```

---

## Quick Reference

| Function | Args | Returns | Description |
|---|---|---|---|
| `print` | `value` | `()` | Print with newline |
| `input` | — | `String` | Read line from stdin |
| `str` | `T` | `String` | Any value → string |
| `int` | `Float`/`Bool` | `Int` | Truncate to int |
| `float` | `Int` | `Float` | Int to float |
| `parse_int` | `String` | `Option[Int]` | Parse decimal |
| `parse_float` | `String` | `Option[Float]` | Parse float |
| `abs` | `T` | `T` | Absolute value |
| `min/max` | `T, T` | `T` | Min/max of two |
| `pow` | `Float, Float` | `Float` | Exponentiate |
| `sqrt` | `Float` | `Float` | Square root |
| `floor/ceil/round` | `Float` | `Float` | Rounding |
| `len` | `String`/`[T]` | `Int` | Length |
| `push` | `[T], T` | `[T]` | Append to list |
| `sort/reverse` | `[T]` | `[T]` | Sort/reverse copy |
| `sum` | `[Int]` | `Int` | Sum of list |
| `any/all` | `[Bool]` | `Bool` | Boolean aggregation |
| `map` | `fn, [T]` | `[U]` | Transform list |
| `filter` | `fn, [T]` | `[T]` | Filter list |
| `reduce` | `fn, [T]` | `T` | Fold left |
| `split` | `String, String` | `[String]` | Split string |
| `join` | `[String], String` | `String` | Join strings |
| `contains` | `String, String` | `Bool` | Substring check |
| `find` | `String, String` | `Int` | Find index |
| `replace` | `String, String, String` | `String` | Replace first |
| `upper/lower/strip` | `String` | `String` | Case/trim |
| `chars` | `String` | `[String]` | Character list |
| `ord` | `Char` | `Int` | Code point |
| `chr` | `Int` | `Char` | Code point → char |
| `hex/oct/bin` | `Int` | `String` | Numeric string |
| `args` | — | `[String]` | CLI arguments |
| `getenv` | `String` | `Option[String]` | Env variable |
| `sleep` | `Int` | `()` | Sleep ms |
| `exit` | `Int` | `Never` | Exit process |
| `panic` | `String` | `Never` | Abort with message |
| `assert` | `Bool` | `()` | Assert invariant |
| `read_file` | `String` | `Result[String, String]` | Read file |
| `write_file` | `String, String` | `Result[(), String]` | Write file |
