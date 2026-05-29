# Functions

Functions are the primary building block of Mom programs. They are first-class values and can be passed, returned, and stored just like any other value.

---

## Declaring a Function

```mom
fn add(a: Int, b: Int) -> Int:
    a + b
```

**Syntax:**

```
fn <name>(<params>) -> <return_type>:
    <body>
```

- **Parameters** — `name: Type` pairs separated by commas
- **Return type** — after `->`. Omit for functions returning `()`
- **Body** — the last expression is the implicit return value

---

## Calling a Function

```mom
let result = add(3, 5)
print(result)    // 8
```

---

## Implicit Return

The last expression in the function body is the return value — no `return` keyword needed:

```mom
fn square(x: Int) -> Int:
    x * x           // implicit return

fn greet(name: String) -> String:
    "Hello, " + name + "!"   // implicit return
```

---

## Explicit `return`

Use `return` to exit early:

```mom
fn first_positive(xs: [Int]) -> Option[Int]:
    for x in xs:
        if x > 0: return Some(x)
    None

fn safe_div(a: Int, b: Int) -> Result[Int, String]:
    if b == 0:
        return Err("division by zero")
    Ok(a / b)
```

---

## Functions Returning Nothing (Unit)

Omit `->` for functions that produce no value (implicitly return `()`):

```mom
fn say_hello():
    print("Hello!")

fn print_range(lo: Int, hi: Int):
    for i in lo..hi:
        print(i)
```

---

## Recursive Functions

Functions can call themselves:

```mom
fn fib(n: Int) -> Int:
    if n <= 1: n
    else: fib(n - 1) + fib(n - 2)

fn factorial(n: Int) -> Int:
    if n <= 1: 1
    else: n * factorial(n - 1)

print(fib(10))        // 55
print(factorial(10))  // 3628800
```

---

## Generic Functions

Type parameters in square brackets allow a function to work on any type:

```mom
fn identity[T](value: T) -> T:
    value

fn first[T](xs: [T]) -> Option[T]:
    if len(xs) == 0: None
    else: Some(xs[0])

print(identity(42))       // 42
print(identity("hello"))  // hello
```

### With type bounds

Restrict `T` to types that implement a trait:

```mom
fn max_of[T: Ord](a: T, b: T) -> T:
    if a > b: a else: b
```

---

## Lambdas (Anonymous Functions)

```mom
// Inline lambda
let double = fn(x: Int) -> Int => x * 2

// Lambda with inferred types
let square = fn(x: Int) => x * x

// Multi-statement lambda uses a block body
let complex = fn(x: Int) => block:
    let doubled = x * 2
    doubled + 1
```

### Passing lambdas to functions

```mom
fn apply(f: fn(Int) -> Int, x: Int) -> Int:
    f(x)

print(apply(fn(x: Int) => x + 10, 5))   // 15
```

### Higher-order functions with lambdas

```mom
let numbers = [1, 2, 3, 4, 5]

let doubled  = map(fn(x: Int) => x * 2, numbers)
// [2, 4, 6, 8, 10]

let evens    = filter(fn(x: Int) => x % 2 == 0, numbers)
// [2, 4]

let total    = reduce(fn(acc: Int, x: Int) => acc + x, numbers)
// 15
```

---

## Methods (Functions on Types)

Functions defined inside an `impl` block become methods. They receive `self` as the first parameter:

```mom
struct Circle:
    radius: Float

impl Circle:
    fn area(self) -> Float:
        3.14159 * self.radius * self.radius

    fn perimeter(self) -> Float:
        2.0 * 3.14159 * self.radius

    fn scale(self, factor: Float) -> Circle:
        Circle { radius: self.radius * factor }
```

Call methods with dot notation:

```mom
let c = Circle { radius: 5.0 }
print(c.area())         // 78.53975
print(c.perimeter())    // 31.4159
let bigger = c.scale(2.0)
print(bigger.radius)    // 10.0
```

---

## `self` — The Receiver

`self` is the first parameter by convention for methods. It receives a copy of the struct value. Methods that return a modified struct return a new value (immutable update pattern):

```mom
impl Counter:
    fn increment(self) -> Counter:
        Counter { value: self.value + 1 }

    fn reset(self) -> Counter:
        Counter { value: 0 }
```

---

## Trait Methods (Polymorphism)

```mom
trait Drawable:
    fn draw(self)
    fn area(self) -> Float

struct Circle:
    radius: Float

impl Drawable for Circle:
    fn draw(self):
        print("Circle(r=" + str(self.radius) + ")")

    fn area(self) -> Float:
        3.14159 * self.radius * self.radius

struct Rectangle:
    width: Float
    height: Float

impl Drawable for Rectangle:
    fn draw(self):
        print("Rectangle(" + str(self.width) + "x" + str(self.height) + ")")

    fn area(self) -> Float:
        self.width * self.height
```

---

## Async Functions

```mom
async fn fetch(url: String) -> Result[String, String]:
    let conn = await http.connect(url)?
    let body = await conn.read_all()?
    Ok(body)

async fn main():
    match await fetch("https://example.com"):
        Ok(body) => print(body)
        Err(e)   => print("Error: " + e)
```

---

## Functions as Values

Functions have the type `fn(T, U, ...) -> V`. They can be stored, passed, and returned:

```mom
let ops: [fn(Int, Int) -> Int] = [
    fn(a: Int, b: Int) => a + b,
    fn(a: Int, b: Int) => a - b,
    fn(a: Int, b: Int) => a * b,
]

for op in ops:
    print(op(10, 3))
// 13, 7, 30
```

### Returning a function

```mom
fn make_adder(n: Int) -> fn(Int) -> Int:
    fn(x: Int) => x + n

let add5 = make_adder(5)
print(add5(10))   // 15
print(add5(20))   // 25
```

---

## Overloading

Mom does **not** support function overloading. Each function name is unique in its scope. Use trait methods or differently named functions instead.

---

## Summary

| Pattern | Example |
|---|---|
| Basic function | `fn f(x: Int) -> Int: x * 2` |
| No return value | `fn greet(): print("Hi")` |
| Explicit return | `return Some(x)` |
| Generic | `fn id[T](x: T) -> T: x` |
| Lambda | `fn(x: Int) => x + 1` |
| Method | `impl T: fn method(self) -> ...` |
| Async | `async fn f() -> Result[T, E]` |
| Trait method | `impl Trait for T: fn method(self)` |
| Function value | `let f: fn(Int) -> Int = ...` |
