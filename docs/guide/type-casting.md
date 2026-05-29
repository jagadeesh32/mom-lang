# Type Casting

Mom is strongly typed — values do not convert automatically between types. Conversions are always explicit, using built-in conversion functions. This eliminates subtle bugs caused by implicit coercions.

---

## Numeric Conversions

### `Int` ↔ `Float`

```mom
// Int to Float
let n: Int = 42
let f: Float = n as Float       // planned syntax
// or via the built-in:
let f: Float = float(n)         // 42.0

// Float to Int (truncates toward zero)
let x: Float = 3.9
let i: Int = int(x)             // 3  (not 4)

let neg: Float = -2.7
let j: Int = int(neg)           // -2  (truncates, not floors)
```

> **Caution:** converting a `Float` to `Int` truncates the fractional part silently. If you need rounding, call `round()` first.

```mom
let x = 3.9
print(int(x))           // 3  (truncate)
print(int(round(x)))    // 4  (round first, then convert)
print(int(floor(x)))    // 3  (floor first)
print(int(ceil(x)))     // 4  (ceiling first)
```

---

### `Int` ↔ `String`

```mom
// Int to String
let n = 42
let s = str(n)           // "42"
let s = to_string(n)     // "42"  (alias)

// String to Int (returns Option[Int])
let maybe = parse_int("123")     // Some(123)
let fail  = parse_int("abc")     // None
let fail2 = parse_int("12.5")    // None (not an int)

// Unwrap safely
match parse_int("99"):
    Some(v) => print(v)
    None    => print("bad input")
```

---

### `Float` ↔ `String`

```mom
let f = 3.14
let s = str(f)           // "3.14"

// String to Float (returns Option[Float])
let maybe = parse_float("2.71")   // Some(2.71)
let fail  = parse_float("hi")     // None
```

---

### `Bool` ↔ `String`

```mom
let b = true
let s = str(b)            // "true"

// String to Bool
let t = parse_bool("true")    // Some(true)
let f = parse_bool("false")   // Some(false)
let n = parse_bool("yes")     // None (only "true"/"false" are accepted)
```

---

### `Int` ↔ Numeric String Representations

```mom
let n = 255
print(hex(n))    // "ff"
print(oct(n))    // "377"
print(bin(n))    // "11111111"

// Parse hex string
let v = parse_int_radix("ff", 16)    // Some(255)
let v = parse_int_radix("377", 8)   // Some(255)
let v = parse_int_radix("11111111", 2) // Some(255)
```

---

## Character Conversions

```mom
// Char to Int (Unicode code point)
let c = 'A'
let code = ord(c)      // 65

// Int to Char
let c = chr(65)        // 'A'
let euro = chr(0x20AC) // '€'
```

---

## String ↔ List

```mom
// String to list of characters
let chars_list = chars("hello")   // ["h", "e", "l", "l", "o"]

// List of strings to joined string
let words = ["hello", "world"]
let joined = join(words, " ")      // "hello world"
let csv = join(words, ",")        // "hello,world"
```

---

## Casting Rules Summary

| From | To | Function | Notes |
|---|---|---|---|
| `Int` | `Float` | `float(n)` | exact for small integers |
| `Float` | `Int` | `int(f)` | truncates toward zero |
| `Float` | `Int` (rounded) | `int(round(f))` | round first |
| `Int` | `String` | `str(n)` | base 10 |
| `Float` | `String` | `str(f)` | decimal notation |
| `Bool` | `String` | `str(b)` | `"true"` or `"false"` |
| `String` | `Int` | `parse_int(s)` | returns `Option[Int]` |
| `String` | `Float` | `parse_float(s)` | returns `Option[Float]` |
| `String` | `Bool` | `parse_bool(s)` | returns `Option[Bool]` |
| `Char` | `Int` | `ord(c)` | Unicode code point |
| `Int` | `Char` | `chr(n)` | Unicode code point |
| `Int` | hex/oct/bin string | `hex(n)`, `oct(n)`, `bin(n)` | string representation |

---

## Implicit Coercions (What Mom Does NOT Do)

Mom will **never** automatically convert:

```mom
let n: Int = 5
let f: Float = 2.5

// This is a type error:
let sum = n + f     // ERROR: cannot add Int and Float

// This is correct:
let sum = float(n) + f   // 7.5
```

This strict no-coercion rule prevents entire classes of precision bugs (e.g., `int + float = int` truncation in C).

---

## Checked vs Unchecked

All `parse_*` functions return `Option[T]` — they never crash on bad input. Use them at boundaries where you receive external strings.

```mom
fn read_port() -> Int:
    let input = input()
    match parse_int(input):
        Some(p) =>
            if p > 0 and p <= 65535: p
            else: panic("port out of range: " + input)
        None => panic("not a number: " + input)
```
