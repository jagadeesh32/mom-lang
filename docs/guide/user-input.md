# User Input

Mom provides several ways to read input from the user, from command-line arguments to interactive stdin reads.

---

## Reading a Line from stdin — `input()`

`input()` reads a single line from standard input and returns it as a `String` (without the trailing newline):

```mom
fn main():
    print("Enter your name: ")
    let name = input()
    print("Hello, " + name + "!")
```

Run:

```
Enter your name: Alice
Hello, Alice!
```

### Reading a number

`input()` always returns `String`. Use `parse_int` or `parse_float` to convert:

```mom
fn main():
    print("Enter a number: ")
    let raw = input()
    match parse_int(raw):
        Some(n) => print("You entered: " + str(n * 2))
        None    => print("That is not a number.")
```

### Reading multiple values

Call `input()` multiple times:

```mom
fn main():
    print("First name: ")
    let first = input()
    print("Last name: ")
    let last = input()
    print("Hello, " + first + " " + last + "!")
```

---

## Command-Line Arguments — `args()`

`args()` returns a `[String]` containing all command-line arguments, **including** the program name at index 0.

```mom
fn main():
    let argv = args()
    print("Program: " + argv[0])
    for i in 1..len(argv):
        print("Arg " + str(i) + ": " + argv[i])
```

Run:

```bash
mom run myprog.mom hello world 42
```

Output:

```
Program: myprog.mom
Arg 1: hello
Arg 2: world
Arg 3: 42
```

### Parsing arguments

```mom
fn main():
    let argv = args()
    if len(argv) < 2:
        print("usage: greet <name>")
        return
    let name = argv[1]
    print("Hello, " + name + "!")
```

### Requiring a numeric argument

```mom
fn main():
    let argv = args()
    if len(argv) < 2:
        panic("usage: count <n>")
    match parse_int(argv[1]):
        None    => panic("argument must be an integer")
        Some(n) =>
            for i in 0..n:
                print(i)
```

---

## Environment Variables — `getenv(name)`

`getenv(name)` returns the value of an environment variable as `Option[String]`. Returns `None` if the variable is not set.

```mom
fn main():
    match getenv("HOME"):
        Some(home) => print("Home directory: " + home)
        None       => print("HOME is not set")

    match getenv("PORT"):
        Some(raw) =>
            match parse_int(raw):
                Some(port) => print("Serving on port " + str(port))
                None       => panic("PORT is not a number: " + raw)
        None => print("No PORT set, using default 8080")
```

---

## Reading with a Prompt

A common pattern is to print a prompt then immediately read:

```mom
fn prompt(msg: String) -> String:
    print(msg)
    input()

fn main():
    let city = prompt("Enter city: ")
    let country = prompt("Enter country: ")
    print(city + ", " + country)
```

---

## Reading a Float

```mom
fn read_float(prompt_msg: String) -> Float:
    print(prompt_msg)
    let raw = input()
    match parse_float(raw):
        Some(f) => f
        None    => panic("expected a number, got: " + raw)

fn main():
    let temperature = read_float("Temperature in Celsius: ")
    let fahrenheit = temperature * 9.0 / 5.0 + 32.0
    print(str(fahrenheit) + "°F")
```

---

## Reading Until a Sentinel Value

```mom
fn main():
    let mut lines = []
    while true:
        let line = input()
        if line == "quit": break
        lines = push(lines, line)
    print("You entered " + str(len(lines)) + " lines.")
```

---

## Input Validation Pattern

A reusable pattern for validated input:

```mom
fn read_int_in_range(msg: String, lo: Int, hi: Int) -> Int:
    while true:
        print(msg)
        let raw = input()
        match parse_int(raw):
            None    => print("Please enter an integer.")
            Some(n) =>
                if n >= lo and n <= hi: return n
                print("Value must be between " + str(lo) + " and " + str(hi) + ".")
    0   // unreachable; satisfies the return type

fn main():
    let age = read_int_in_range("Enter your age (0-120): ", 0, 120)
    print("Age: " + str(age))
```

---

## Summary of Input Functions

| Function | Returns | Description |
|---|---|---|
| `input()` | `String` | Read one line from stdin, stripping newline |
| `args()` | `[String]` | All command-line arguments including argv[0] |
| `getenv(name)` | `Option[String]` | Environment variable, or `None` |
