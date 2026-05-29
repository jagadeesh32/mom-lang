# Literals

A literal is a value written directly in source code. Mom supports integer, float, boolean, string, character, list, and unit literals.

---

## Integer Literals

Decimal integers. Underscores may be inserted anywhere for readability:

```mom
42
-17
0
1_000_000
9_223_372_036_854_775_807    // Int max
```

**Type:** inferred as `Int` (64-bit signed) unless annotated otherwise.

> Negative integer literals are parsed as a unary minus followed by a positive literal, so `-5` is an expression, not a single token. This is transparent to you in practice.

---

## Float Literals

Floating-point numbers must contain a decimal point or an exponent:

```mom
3.14
-2.5
0.0
1.0
1.0e9       // 1,000,000,000.0
2.5e-3      // 0.0025
1.23E+10
```

**Type:** inferred as `Float` (64-bit IEEE-754) unless annotated `Float32`.

> `1` is an `Int`. `1.0` is a `Float`. You cannot write a float without at least one digit after the decimal point: `1.` is not valid.

---

## Boolean Literals

```mom
true
false
```

**Type:** `Bool`.

---

## String Literals

Enclosed in double quotes `"..."`. The lexer processes escape sequences and stores the decoded string internally.

```mom
"Hello, world!"
"tab\there"
"line one\nline two"
"quote: \""
"backslash: \\"
""              // empty string
```

### Escape Sequences

| Sequence | Meaning | Unicode |
|---|---|---|
| `\n` | Newline (line feed) | U+000A |
| `\r` | Carriage return | U+000D |
| `\t` | Horizontal tab | U+0009 |
| `\\` | Backslash | U+005C |
| `\"` | Double quote | U+0022 |
| `\0` | Null byte | U+0000 |
| `\xNN` | Byte value (hex) | 0x00–0xFF |

```mom
let path = "C:\\Users\\Alice\\Documents"
let json = "{\"key\": \"value\"}"
let multiline_hint = "line1\nline2\nline3"
```

**Type:** `String`.

---

## Character Literals

Single Unicode code points in single quotes:

```mom
'A'
'z'
'0'
'\n'        // newline character
'\t'        // tab character
'\\'        // backslash character
'\''        // single quote
'€'         // U+20AC EURO SIGN
'😀'        // emoji, U+1F600
```

Character literals support the same escape sequences as string literals.

**Type:** `Char`.

---

## List Literals

Enclosed in square brackets, comma-separated:

```mom
[1, 2, 3]
["alpha", "beta", "gamma"]
[true, false, true]
[]                         // empty list; type must be inferable from context
[1, 2, 3,]                 // trailing comma is allowed
```

**Type:** `[T]` where `T` is inferred from the elements.

Nested lists:

```mom
let matrix = [[1, 2], [3, 4], [5, 6]]
print(matrix[0][1])   // 2
```

---

## Unit Literal

The only value of the unit type `()`:

```mom
let nothing: () = ()
```

Functions that return nothing implicitly return `()`.

---

## Struct Literals

Named fields in braces. Fields can be in any order:

```mom
struct Point { x: Int, y: Int }

let p = Point { x: 3, y: 4 }
let q = Point { y: 10, x: 5 }   // order doesn't matter
```

---

## Enum / Variant Literals

Variants are constructed like function calls (for variants with data) or bare names (for variants without data):

```mom
enum Color { Red, Green(Int), Blue(Int, Int) }

let c1 = Red                // nullary variant — no parentheses
let c2 = Green(128)         // one field
let c3 = Blue(0, 255)       // two fields
let maybe: Option[Int] = Some(42)
let nothing: Option[Int] = None
```

---

## Range Literals

Ranges are created with the `..` operator. They are used primarily in `for` loops:

```mom
for i in 0..10:          // i goes from 0 to 9 (exclusive upper bound)
    print(i)

for i in 1..=10:         // i goes from 1 to 10 (inclusive upper bound) — planned
    print(i)
```

The range `lo..hi` is equivalent to the half-open interval `[lo, hi)`.

---

## None Literal

`None` is both a keyword and a value of type `Option[T]` for any `T`:

```mom
let x: Option[Int] = None
```

In pattern matching, `None` is both a pattern and a value:

```mom
match x:
    Some(v) => print(v)
    None    => print("empty")
```

---

## Summary

| Literal | Type | Examples |
|---|---|---|
| `42`, `-7`, `1_000` | `Int` | decimal integer |
| `3.14`, `1.0e9`, `-0.5` | `Float` | decimal float |
| `true`, `false` | `Bool` | boolean |
| `"hello"`, `"a\nb"` | `String` | UTF-8 string |
| `'A'`, `'\n'`, `'€'` | `Char` | Unicode char |
| `[1, 2, 3]`, `[]` | `[T]` | list |
| `()` | `()` | unit |
| `Point { x: 1, y: 2 }` | struct type | struct literal |
| `Some(5)`, `Ok(1)`, `Red` | enum variant | variant literal |
| `0..10` | range | range expression |
| `None` | `Option[T]` | absence |
