use mom::{lex_source, parse_source, run_source};

#[test]
fn lexes_keywords_and_literals() {
    let tokens = lex_source(r#"fn main() { print("ok") }"#).unwrap();
    assert!(tokens.len() > 6);
}

#[test]
fn parses_function_program() {
    let program = parse_source(
        r#"
        fn add(a: Int, b: Int) -> Int {
            a + b
        }
        "#,
    )
    .unwrap();
    assert_eq!(program.items.len(), 1);
}

#[test]
fn runs_recursion() {
    let output = run_source(
        r#"
        fn fib(n: Int) -> Int {
            if n <= 1 {
                n
            } else {
                fib(n - 1) + fib(n - 2)
            }
        }

        fn main() {
            print(fib(8))
        }
        "#,
    )
    .unwrap();
    assert_eq!(output, "21\n");
}

#[test]
fn enforces_mutability() {
    let err = run_source(
        r#"
        fn main() {
            let x = 1
            x = 2
        }
        "#,
    )
    .unwrap_err();
    assert!(err.message.contains("immutable"));
}

#[test]
fn runs_match_and_pipeline() {
    let output = run_source(
        r#"
        fn inc(x: Int) -> Int { x + 1 }
        fn label(x: Int) -> String {
            match x {
                0 => "zero",
                5 => "five",
                _ => "many",
            }
        }

        fn main() {
            let value = 4 |> inc
            print(label(value))
        }
        "#,
    )
    .unwrap();
    assert_eq!(output, "five\n");
}

#[test]
fn runs_lambdas() {
    let output = run_source(
        r#"
        fn main() {
            let square = fn(x: Int) => x * x
            print(square(9))
        }
        "#,
    )
    .unwrap();
    assert_eq!(output, "81\n");
}

#[test]
fn runs_lists_and_indexing() {
    let output = run_source(
        r#"
        fn main() {
            let xs = [10, 20, 30]
            print(xs[1])
            print(len(xs))
        }
        "#,
    )
    .unwrap();
    assert_eq!(output, "20\n3\n");
}

#[test]
fn runs_for_in_range() {
    let output = run_source(
        r#"
        fn main() {
            let mut total = 0
            for i in 0..5 {
                total = total + i
            }
            print(total)
        }
        "#,
    )
    .unwrap();
    assert_eq!(output, "10\n");
}

#[test]
fn runs_for_in_list() {
    let output = run_source(
        r#"
        fn main() {
            let mut total = 0
            for x in [1, 2, 3, 4] {
                total = total + x
            }
            print(total)
        }
        "#,
    )
    .unwrap();
    assert_eq!(output, "10\n");
}

#[test]
fn runs_option_variants() {
    let output = run_source(
        r#"
        fn first(xs: [Int]) -> Option[Int] {
            if len(xs) == 0 {
                None
            } else {
                Some(xs[0])
            }
        }

        fn main() {
            match first([7, 8, 9]) {
                Some(x) => print(x),
                None => print(0),
            }
        }
        "#,
    )
    .unwrap();
    assert_eq!(output, "7\n");
}

#[test]
fn runs_result_try_operator() {
    let output = run_source(
        r#"
        fn parse(value: Int) -> Result[Int, String] {
            if value < 0 {
                Err("negative")
            } else {
                Ok(value * 2)
            }
        }

        fn doubled(value: Int) -> Result[Int, String] {
            let inner = parse(value)?
            Ok(inner + 1)
        }

        fn main() {
            match doubled(5) {
                Ok(v) => print(v),
                Err(e) => print(e),
            }
            match doubled(-1) {
                Ok(v) => print(v),
                Err(e) => print(e),
            }
        }
        "#,
    )
    .unwrap();
    assert_eq!(output, "11\nnegative\n");
}

#[test]
fn runs_structs_and_field_access() {
    let output = run_source(
        r#"
        struct Point { x: Int, y: Int }

        fn main() {
            let p = Point { x: 3, y: 4 }
            print(p.x + p.y)
        }
        "#,
    )
    .unwrap();
    assert_eq!(output, "7\n");
}

#[test]
fn runs_impl_methods() {
    let output = run_source(
        r#"
        struct Counter { value: Int }

        impl Counter {
            fn inc(self) -> Int {
                self.value + 1
            }
        }

        fn main() {
            let c = Counter { value: 41 }
            print(c.inc())
        }
        "#,
    )
    .unwrap();
    assert_eq!(output, "42\n");
}

#[test]
fn parses_enterprise_constructs() {
    // These constructs should at least parse cleanly even when they are
    // not yet executable in the bootstrap interpreter.
    let source = r#"
        module net {
            pub struct Address { host: String, port: Int }
        }

        import net.{Address}

        trait Greet {
            fn hello(self) -> String
        }

        async fn fetch(url: String) -> Result[String, String] {
            Ok(url)
        }

        extern c "m" {
            fn cos(x: Float) -> Float
        }

        fn main() {
            print("ok")
        }
    "#;
    let program = parse_source(source).unwrap();
    assert!(program.items.len() >= 5);
}
