# Unicode System

Mom is built for a Unicode world from the ground up. Source files, string values, and character literals all use UTF-8.

---

## Source File Encoding

Mom source files must be **UTF-8 encoded**. Non-ASCII characters are allowed in:

- String literals: `"こんにちは"`, `"café"`, `"🚀"`
- Character literals: `'€'`, `'日'`, `'😀'`
- Comments: `// 日本語のコメント`

Non-ASCII characters are **not** allowed in identifiers (variable names, function names, etc.) in the current version. Identifiers must be ASCII letters, digits, and underscores.

---

## The `String` Type

A `String` is an immutable, **UTF-8** encoded sequence of bytes. The internal representation is a pointer + length; strings are not null-terminated.

Key properties:

- **`len(s)` returns the number of bytes**, not the number of characters.
- For multi-byte characters (emoji, CJK, accented letters), the byte count is larger than the visual character count.
- String equality `==` and `!=` compare bytes exactly (byte-for-byte equality).

```mom
let s = "hello"
print(len(s))      // 5  (5 bytes, 5 ASCII chars)

let s2 = "café"
print(len(s2))     // 5  (c=1, a=1, f=1, é=2 bytes in UTF-8)
```

---

## The `Char` Type

A `Char` is a **Unicode scalar value** — a code point in the range U+0000 to U+10FFFF, excluding surrogates (U+D800–U+DFFF).

```mom
let a = 'A'          // U+0041 LATIN CAPITAL LETTER A
let e = 'é'          // U+00E9 LATIN SMALL LETTER E WITH ACUTE
let kanji = '日'     // U+65E5 CJK UNIFIED IDEOGRAPH
let emoji = '🎉'    // U+1F389 PARTY POPPER (4 bytes in UTF-8)
```

### Char to Integer (Code Point)

```mom
let code = ord('A')    // 65
let code = ord('é')    // 233  (0xE9)
let code = ord('🎉')  // 127881  (0x1F389)
```

### Integer to Char

```mom
let c = chr(65)        // 'A'
let c = chr(233)       // 'é'
let c = chr(0x1F389)   // '🎉'
```

---

## String Iteration

Iterating over a string character-by-character uses `chars()`, which returns a list of single-character strings:

```mom
let letters = chars("hello")
// ["h", "e", "l", "l", "o"]

for ch in chars("café"):
    print(ch)
// c
// a
// f
// é
```

---

## String Indexing

String indexing (`s[i]`) is **byte indexing**, not character indexing. Accessing a byte in the middle of a multi-byte character will give you an incorrect result. Prefer iteration with `chars()` for character-level access.

```mom
let s = "hello"
print(s[0])   // "h"  — fine for ASCII

let s2 = "café"
// s2[3] would give the first byte of "é", not the whole character
// Use chars(s2)[3] for safe character access
```

---

## Unicode Escape in String Literals

Control characters and non-printable code points can be written using `\xNN` in string literals:

```mom
let null_char = "\x00"
let tab_char  = "\x09"    // same as "\t"
let space     = "\x20"
```

For Unicode code points above U+007F, write the character directly or use a future `\u{NNNN}` syntax (planned):

```mom
let euro = "€"           // write directly  (preferred)
// let euro = "\u{20AC}" // planned syntax
```

---

## Comparison and Sorting

String comparison (`<`, `>`, `<=`, `>=`) is **byte-by-byte lexicographic order**, which for ASCII strings is alphabetical order. For non-ASCII strings, this gives code-point order, which is not locale-aware alphabetical order.

```mom
print("apple" < "banana")    // true
print("z" > "a")             // true
print("é" > "z")             // true (because é = 0xE9 > 'z' = 0x7A in UTF-8 bytes)
```

For locale-aware collation (correct alphabetical ordering in German, French, etc.), use the `std::intl` module (planned for Phase 6).

---

## Byte Operations

For low-level byte manipulation, use `[Byte]` (a list of `Byte = UInt8`) rather than `String`:

```mom
fn bytes_of(s: String) -> [Byte]:
    // built-in conversion (planned)
    s.as_bytes()

fn string_of(bs: [Byte]) -> Option[String]:
    // validates that bs is valid UTF-8
    String.from_utf8(bs)
```

---

## Common Unicode Categories

Mom provides predicates on `Char`:

```mom
is_digit('5')      // true   (0–9)
is_alpha('A')      // true   (a–z, A–Z; ASCII only in current version)
is_alnum('9')      // true   (digit or letter)
is_space(' ')      // true   (space, tab, newline)
```

---

## Emoji and Multi-Codepoint Graphemes

Some visible characters (grapheme clusters) are composed of multiple Unicode code points. For example, a flag emoji 🇺🇸 is two code points (U+1F1FA + U+1F1F8).

`chars()` iterates over individual code points, not grapheme clusters. Full grapheme-cluster segmentation is available in the `std::text` module (planned for Phase 6).

---

## UTF-8 Encoding Reference

| Code point range | Bytes | Pattern |
|---|---|---|
| U+0000–U+007F | 1 | `0xxxxxxx` |
| U+0080–U+07FF | 2 | `110xxxxx 10xxxxxx` |
| U+0800–U+FFFF | 3 | `1110xxxx 10xxxxxx 10xxxxxx` |
| U+10000–U+10FFFF | 4 | `11110xxx 10xxxxxx 10xxxxxx 10xxxxxx` |
