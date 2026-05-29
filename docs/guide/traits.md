# Traits

A **trait** in mom is a named collection of method signatures that a type can implement. Traits fill the same role as interfaces in Go or Java and typeclasses in Haskell — they let you write code that is generic over any type that promises a set of behaviours, without giving up type safety.

---

## Declaring a Trait

```mom
trait Shape {
    fn area(self) -> Float
    fn perimeter(self) -> Float
}
```

The colon-block style is also accepted (used throughout the examples):

```mom
trait Shape:
    fn area(self) -> Float
    fn perimeter(self) -> Float
```

Each method signature ends with an optional semicolon. The body is **not** written here — only the signature.

### The `self` Parameter

The first parameter of every trait method is `self`. It refers to the receiver value (the struct instance the method is called on). `self` has no explicit type annotation — the compiler infers it from whichever concrete type is being implemented.

```mom
trait Display:
    fn display(self) -> String

trait Eq:
    fn eq(self, other: Self) -> Bool
```

`Self` (capital S) is an alias for the implementing type and can appear in parameter or return position inside a trait declaration.

---

## Implementing a Trait

Use `impl Trait for Type` to attach a concrete implementation to a struct:

```mom
struct Circle:
    radius: Float

struct Rect:
    w: Float
    h: Float

impl Shape for Circle:
    fn area(self) -> Float:
        3.14159 * self.radius * self.radius

    fn perimeter(self) -> Float:
        2.0 * 3.14159 * self.radius

impl Shape for Rect:
    fn area(self) -> Float:
        self.w * self.h

    fn perimeter(self) -> Float:
        2.0 * (self.w + self.h)
```

Each method in `impl` must match a signature in the trait. All methods listed in the trait **must** be implemented — the compiler (and bootstrap interpreter) will reject a partial impl.

### Grammar Reference

```ebnf
trait_decl  = "trait" IDENT generics? "{" trait_method* "}" ;
trait_method = visibility? "fn" IDENT generics? params return_type? ";"? ;

impl_block   = "impl" generics? IDENT ( "[" type_list "]" )?
               ( "for" IDENT ( "[" type_list "]" )? )?
               "{" impl_method* "}" ;
impl_method  = visibility? "async"? function ;
```

---

## Calling Trait Methods

Trait methods are called with standard **dot notation** on a value:

```mom
fn main():
    let c = Circle { radius: 2.0 }
    let r = Rect   { w: 3.0, h: 4.0 }

    print(c.area())       // 12.56636
    print(r.perimeter())  // 14.0
```

The interpreter resolves the method by looking up the impl registered for the concrete type at the call site.

---

## Trait Bounds on Generic Functions

Restrict a type parameter to only types that implement a given trait by writing the bound in square brackets after the parameter name:

```mom
fn print_area[T: Shape](shape: T):
    print(shape.area())
```

This reads: "for any type `T` that implements `Shape`, accept a value of that type."

### Multiple Bounds

Combine bounds with `+`:

```mom
fn describe[T: Display + Debug](value: T):
    print(value.display())
```

A value passed to `describe` must implement **both** `Display` and `Debug`.

### Multiple Type Parameters with Bounds

```mom
fn compare_and_show[T: Ord + Display](a: T, b: T):
    if a.eq(b):
        print("equal")
    else:
        print("not equal")
```

---

## `where` Clauses

When the inline bound list grows unwieldy, move bounds to a `where` clause placed after the parameter list:

```mom
fn merge[T](a: [T], b: [T]) -> [T]
    where T: Ord + Clone:
    // body
```

`where` is syntactically equivalent to inline bounds; prefer it for readability when there are three or more bounds or several type parameters.

```mom
fn zip_map[A, B, C](xs: [A], ys: [B], f: fn(A, B) -> C) -> [C]
    where A: Clone,
          B: Clone:
    // body
```

### Grammar Reference

```ebnf
where_clause = "where" type_bound ( "," type_bound )* ;
type_bound   = IDENT ":" IDENT ( "+" IDENT )* ;
```

---

## Default Method Implementations

> **Status — planned.** The current bootstrap interpreter does not yet support default method bodies inside trait declarations. All methods must be implemented in the `impl` block.

When default impls land, the syntax will be:

```mom
// future syntax — not yet supported
trait Greet:
    fn name(self) -> String
    fn greet(self) -> String:   // default — uses name()
        "Hello, " + self.name()
```

Types that override `greet` in their impl will shadow the default.

---

## Trait Objects and Dynamic Dispatch

> **Status — planned for native backend.** The bootstrap interpreter dispatches trait methods by looking up the concrete type at runtime, which gives an equivalent runtime behaviour, but the `dyn Trait` spelling is not yet parsed.

When `dyn Trait` lands, it will allow storing heterogeneous collections of values that implement the same trait:

```mom
// future native-backend syntax
let shapes: [dyn Shape] = [Circle { radius: 1.0 }, Rect { w: 2.0, h: 3.0 }]
for s in shapes:
    print(s.area())
```

For now you can achieve similar dynamic behaviour via enums:

```mom
enum AnyShape:
    C(Circle)
    R(Rect)

fn area_of(s: AnyShape) -> Float:
    match s:
        AnyShape.C(c) => c.area()
        AnyShape.R(r) => r.area()
```

---

## Built-in Traits

mom ships a set of standard traits that the compiler and standard library recognise. You implement them the same way as any user-defined trait.

| Trait     | Key method(s)                            | Purpose                              |
|-----------|------------------------------------------|--------------------------------------|
| `Eq`      | `fn eq(self, other: Self) -> Bool`       | Value equality (`==`)                |
| `Ord`     | `fn cmp(self, other: Self) -> Int`       | Total ordering (`<`, `>`, sort)      |
| `Hash`    | `fn hash(self) -> UInt64`                | Hashing for `Map` / `Set` keys       |
| `Clone`   | `fn clone(self) -> Self`                 | Explicit value duplication           |
| `Display` | `fn display(self) -> String`             | Human-readable formatting            |
| `Debug`   | `fn debug(self) -> String`              | Developer/debug formatting           |

### Example — Implementing `Display` and `Eq`

```mom
struct Point:
    x: Float
    y: Float

impl Display for Point:
    fn display(self) -> String:
        "(" + self.x.display() + ", " + self.y.display() + ")"

impl Eq for Point:
    fn eq(self, other: Self) -> Bool:
        self.x == other.x && self.y == other.y

fn main():
    let p = Point { x: 1.0, y: 2.0 }
    let q = Point { x: 1.0, y: 2.0 }
    print(p.display())   // (1.0, 2.0)
    print(p.eq(q))       // true
```

---

## Traits as Function Parameters

Pass a trait-bounded type parameter to accept any conforming type:

```mom
fn render[T: Display](items: [T]):
    for item in items:
        print(item.display())

fn max_item[T: Ord](xs: [T]) -> T:
    let mut best = xs[0]
    for x in xs:
        if x.cmp(best) > 0:
            best = x
    best
```

Because mom uses **monomorphisation** in the native compiler, these calls compile to type-specific code with no boxing overhead.

---

## Full Worked Example — Drawable

```mom
trait Drawable:
    fn draw(self)
    fn bounding_box(self) -> Rect

struct Circle:
    cx: Float
    cy: Float
    radius: Float

struct Rect:
    x: Float
    y: Float
    w: Float
    h: Float

impl Drawable for Circle:
    fn draw(self):
        print("Drawing circle at (" + self.cx.display() + ", " + self.cy.display() + ") r=" + self.radius.display())

    fn bounding_box(self) -> Rect:
        Rect {
            x: self.cx - self.radius,
            y: self.cy - self.radius,
            w: self.radius * 2.0,
            h: self.radius * 2.0
        }

impl Drawable for Rect:
    fn draw(self):
        print("Drawing rect " + self.w.display() + "x" + self.h.display() + " @ (" + self.x.display() + "," + self.y.display() + ")")

    fn bounding_box(self) -> Rect:
        self   // a rect's bounding box is itself

fn draw_all[T: Drawable](shapes: [T]):
    for s in shapes:
        s.draw()

fn main():
    let c = Circle { cx: 0.0, cy: 0.0, radius: 5.0 }
    let r = Rect   { x: 1.0, y: 1.0, w: 10.0, h: 4.0 }
    c.draw()
    r.draw()
    let bb = c.bounding_box()
    print(bb.w)   // 10.0
```

---

## Full Worked Example — Container Trait

```mom
trait Container[T]:
    fn add(self, value: T)
    fn count(self) -> Int
    fn get(self, index: Int) -> Option[T]

struct Stack[T]:
    items: [T]

impl Container[T] for Stack[T]:
    fn add(self, value: T):
        // push to list (interpreter uses dynamic list)
        self.items = self.items + [value]

    fn count(self) -> Int:
        self.items.len()

    fn get(self, index: Int) -> Option[T]:
        if index >= 0 && index < self.items.len():
            Option.Some(self.items[index])
        else:
            Option.None
```

---

## Summary

| Concept              | Syntax                                          |
|----------------------|-------------------------------------------------|
| Declare trait        | `trait Name { fn method(self) -> T }`           |
| Implement trait      | `impl Trait for Type { fn method(self) -> T: … }` |
| Single bound         | `fn foo[T: Trait](x: T)`                        |
| Multiple bounds      | `fn foo[T: Trait + Other](x: T)`                |
| `where` clause       | `fn foo[T](x: T) where T: Trait + Other`        |
| Call trait method    | `value.method()`                                |
| Built-in traits      | `Eq`, `Ord`, `Hash`, `Clone`, `Display`, `Debug` |
