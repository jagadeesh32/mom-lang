//! Phase 4 self-host integration tests.
//!
//! Verify that the stage-1 mom-in-mom compiler (`compiler/src/main.mom`)
//! can be driven by the stage-0 interpreter to produce real C source
//! that links into a real native binary with expected output.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static SEQ: AtomicUsize = AtomicUsize::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn unique_id(prefix: &str) -> String {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{n}-{}", std::process::id())
}

fn run_stage1(source: &str, source_name: &str) -> String {
    let root = repo_root();

    // Ensure stage-0 is built.
    assert!(
        root.join("target/debug/mom").exists(),
        "stage-0 binary missing — run `cargo build` first"
    );

    let work = root.join("target/selfhost-tests");
    std::fs::create_dir_all(&work).unwrap();

    let stem = unique_id(source_name);
    let mom_path = work.join(format!("{stem}.mom"));
    let c_path = work.join(format!("{stem}.c"));
    let bin_path = work.join(&stem);
    std::fs::write(&mom_path, source).unwrap();

    // Stage-0 runs the mom-in-mom compiler.
    let status = Command::new(root.join("target/debug/mom"))
        .arg("run")
        .arg(root.join("compiler/src/main.mom"))
        .env("MOM_INPUT", &mom_path)
        .env("MOM_OUTPUT", &c_path)
        .status()
        .expect("failed to spawn stage-0 mom");
    assert!(status.success(), "stage-1 compiler exited non-zero");

    // System cc links the generated C with the runtime.
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let status = Command::new(&cc)
        .arg("-std=c99")
        .arg("-O0")
        .arg("-I")
        .arg(root.join("runtime"))
        .arg(&c_path)
        .arg(root.join("runtime/runtime.c"))
        .arg("-o")
        .arg(&bin_path)
        .status()
        .expect("failed to spawn cc");
    assert!(status.success(), "cc failed for stage-1 output");

    // Execute and capture stdout.
    let output = Command::new(&bin_path)
        .output()
        .expect("failed to run stage-1 binary");
    assert!(
        output.status.success(),
        "stage-1 binary exited non-zero: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn stage1_compiles_print_int_literal() {
    let out = run_stage1(
        r#"
        fn main() {
            print(42)
        }
        "#,
        "lit",
    );
    assert_eq!(out, "42\n");
}

#[test]
fn stage1_compiles_let_and_print() {
    let out = run_stage1(
        r#"
        fn main() {
            let x = 100
            print(x)
        }
        "#,
        "let",
    );
    assert_eq!(out, "100\n");
}

#[test]
fn stage1_compiles_binary_arithmetic() {
    let out = run_stage1(
        r#"
        fn main() {
            let a = 6
            let b = 7
            print(a * b)
        }
        "#,
        "mul",
    );
    assert_eq!(out, "42\n");
}

#[test]
fn stage1_compiles_precedence() {
    let out = run_stage1(
        r#"
        fn main() {
            print(1 + 2 * 3)
        }
        "#,
        "prec",
    );
    assert_eq!(out, "7\n");
}

#[test]
fn stage1_compiles_assignment() {
    let out = run_stage1(
        r#"
        fn main() {
            let mut x = 10
            x = x + 5
            x = x * 2
            print(x)
        }
        "#,
        "assign",
    );
    assert_eq!(out, "30\n");
}

#[test]
fn stage1_1_compiles_recursive_function() {
    let out = run_stage1(
        r#"
        fn fib(n: Int) -> Int {
            if n <= 1 {
                return n
            }
            return fib(n - 1) + fib(n - 2)
        }

        fn main() {
            print(fib(10))
        }
        "#,
        "fib",
    );
    assert_eq!(out, "55\n");
}

#[test]
fn stage1_1_compiles_while_loop_and_mutation() {
    let out = run_stage1(
        r#"
        fn factorial(n: Int) -> Int {
            let mut result = 1
            let mut i = 1
            while i <= n {
                result = result * i
                i = i + 1
            }
            return result
        }

        fn main() {
            print(factorial(6))
        }
        "#,
        "fact",
    );
    assert_eq!(out, "720\n");
}

#[test]
fn stage1_1_compiles_bool_and_comparisons() {
    let out = run_stage1(
        r#"
        fn between(lo: Int, x: Int, hi: Int) -> Bool {
            return lo <= x && x <= hi
        }

        fn main() {
            print(between(1, 5, 10))
            print(between(1, 99, 10))
            print(!false)
        }
        "#,
        "bool",
    );
    assert_eq!(out, "true\nfalse\ntrue\n");
}

#[test]
fn stage1_1_compiles_if_else_branching() {
    let out = run_stage1(
        r#"
        fn classify(n: Int) -> Int {
            if n < 0 {
                return 0 - 1
            } else {
                if n == 0 {
                    return 0
                } else {
                    return 1
                }
            }
        }

        fn main() {
            print(classify(0 - 5))
            print(classify(0))
            print(classify(42))
        }
        "#,
        "ifelse",
    );
    assert_eq!(out, "-1\n0\n1\n");
}

#[test]
fn stage1_1_compiles_calls_between_user_functions() {
    let out = run_stage1(
        r#"
        fn double(x: Int) -> Int { return x * 2 }
        fn inc(x: Int) -> Int    { return x + 1 }
        fn main() {
            print(inc(double(20)))
        }
        "#,
        "calls",
    );
    assert_eq!(out, "41\n");
}
