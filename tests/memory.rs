//! Phase 2 borrow checker + memory primitive tests.

use mom::{check_source, run_source};

fn expect_err(source: &str, fragment: &str) {
    let err = run_source(source).expect_err("expected an error");
    let msg = err.message.to_lowercase();
    assert!(
        msg.contains(&fragment.to_lowercase()),
        "expected error containing '{fragment}', got: '{}'",
        err.message
    );
}

#[test]
fn rejects_double_mutable_borrow() {
    expect_err(
        r#"
        fn main() {
            let mut x = "hello"
            let a = &mut x
            let b = &mut x
            print(a)
        }
        "#,
        "already borrowed mutably",
    );
}

#[test]
fn rejects_shared_and_mut_borrow() {
    expect_err(
        r#"
        fn main() {
            let mut x = "hello"
            let r = &x
            let m = &mut x
            print(r)
        }
        "#,
        "mutably while it has shared",
    );
}

#[test]
fn rejects_mut_borrow_of_immutable_binding() {
    expect_err(
        r#"
        fn main() {
            let x = "hello"
            let r = &mut x
            print(r)
        }
        "#,
        "immutable binding",
    );
}

#[test]
fn rejects_assign_while_borrowed() {
    expect_err(
        r#"
        fn main() {
            let mut x = "a"
            let r = &x
            x = "b"
            print(r)
        }
        "#,
        "borrowed",
    );
}

#[test]
fn rejects_use_after_move() {
    expect_err(
        r#"
        fn main() {
            let xs = "owned string"
            let ys = xs
            print(xs)
        }
        "#,
        "moved",
    );
}

#[test]
fn allows_multiple_shared_borrows() {
    let output = run_source(
        r#"
        fn main() {
            let x = "hello"
            let a = &x
            let b = &x
            print(a)
            print(b)
        }
        "#,
    )
    .expect("multiple shared borrows are legal");
    assert_eq!(output, "hello\nhello\n");
}

#[test]
fn allows_sequential_mut_borrows() {
    // Borrow checker is lexical; each block scopes the loan.
    let output = run_source(
        r#"
        fn main() {
            let mut x = "hello"
            { let a = &mut x; print(a) }
            { let b = &mut x; print(b) }
        }
        "#,
    )
    .expect("sequential scoped mutable borrows must be allowed");
    assert_eq!(output, "hello\nhello\n");
}

#[test]
fn copy_types_are_not_moved() {
    let output = run_source(
        r#"
        fn main() {
            let x = 42
            let y = x
            print(x)
            print(y)
        }
        "#,
    )
    .expect("Int is Copy; reusing x after let y = x must be legal");
    assert_eq!(output, "42\n42\n");
}

#[test]
fn region_block_executes() {
    let output = run_source(
        r#"
        fn main() {
            let value = region r {
                let buf = "scratch"
                buf
            }
            print(value)
        }
        "#,
    )
    .expect("region block must run");
    assert_eq!(output, "scratch\n");
}

#[test]
fn box_rc_arc_round_trip() {
    let output = run_source(
        r#"
        fn main() {
            let a = Box(7)
            let b = Rc("shared")
            let c = Arc(true)
            print(a)
            print(b)
            print(c)
        }
        "#,
    )
    .expect("Box/Rc/Arc must construct and print");
    assert_eq!(output, "Box(7)\nRc(shared)\nArc(true)\n");
}

#[test]
fn function_with_reference_param_type_checks() {
    let report = check_source(
        r#"
        fn len_str(s: &String) -> Int { 0 }

        fn main() {
            let s = "abc"
            print(len_str(&s))
        }
        "#,
    )
    .expect("reference parameter types must check");
    assert!(report.functions.contains_key("len_str"));
}

#[test]
fn list_literal_does_not_move_elements() {
    // Regression: a value used inside a list literal and again afterward
    // must not be reported as moved. List elements are treated as reads,
    // consistent with struct-literal fields and call arguments.
    check_source(
        r#"
        fn identity(value: Int) -> Int { value }

        fn main() {
            let a = 5
            let pair = [a, identity(a)]
            print(pair[0])
            print(a)
        }
        "#,
    )
    .expect("reusing a list element afterward must not be a move error");
}

#[test]
fn field_assignment_does_not_move_base() {
    // Regression: assigning to a struct field must not move the whole
    // binding, so it can be read/assigned again afterward.
    check_source(
        r#"
        struct Counter { n: Int }

        fn main() {
            let mut c = Counter { n: 0 }
            c.n = c.n + 5
            c.n = c.n + 3
            print(c.n)
        }
        "#,
    )
    .expect("field assignment must not trigger a move error");
}

#[test]
fn rejects_use_after_move_into_binding() {
    // Soundness guard: rebinding a non-Copy value still moves it.
    expect_err(
        r#"
        struct Buf { n: Int }

        fn main() {
            let a = Buf { n: 1 }
            let b = a
            print(a.n)
        }
        "#,
        "use of moved value",
    );
}
