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
