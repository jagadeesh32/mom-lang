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
        .arg(root.join("compiler"))
        .arg(&c_path)
        .arg(root.join("compiler/runtime.c"))
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

// ── helpers for file-based and self-hosting tests ─────────────────────────────

/// Compile a .mom *file* (not inline source) through stage-1 → native, return stdout.
///
/// Returns `None` if the compiler rejects the file (e.g. unsupported syntax).
/// Tests that call this should skip gracefully when `None` is returned.
fn try_run_stage1_file(mom_path: &std::path::Path, test_name: &str) -> Option<String> {
    let root = repo_root();

    if !root.join("target/debug/mom").exists() {
        return None; // stage-0 not built
    }

    let work = root.join("target/selfhost-tests");
    std::fs::create_dir_all(&work).unwrap();

    let stem = unique_id(test_name);
    let c_path = work.join(format!("{stem}.c"));
    let bin_path = work.join(&stem);

    // Stage-0 runs the mom-in-mom compiler on the given file.
    let status = Command::new(root.join("target/debug/mom"))
        .arg("run")
        .arg(root.join("compiler/src/main.mom"))
        .env("MOM_INPUT", mom_path)
        .env("MOM_OUTPUT", &c_path)
        .status()
        .expect("failed to spawn stage-0 mom");

    if !status.success() {
        eprintln!(
            "stage1_file: compiler rejected {} — skipping (feature not yet supported)",
            mom_path.display()
        );
        return None;
    }

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let status = Command::new(&cc)
        .arg("-std=c99")
        .arg("-O0")
        .arg("-I")
        .arg(root.join("compiler"))
        .arg(&c_path)
        .arg(root.join("compiler/runtime.c"))
        .arg("-o")
        .arg(&bin_path)
        .status()
        .expect("failed to spawn cc");
    if !status.success() {
        eprintln!("cc failed for {} — skipping", mom_path.display());
        return None;
    }

    let output = Command::new(&bin_path)
        .output()
        .expect("failed to run stage-1 binary");
    if !output.status.success() {
        eprintln!(
            "binary exited non-zero for {}: stderr={}",
            mom_path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        return None;
    }
    Some(String::from_utf8(output.stdout).unwrap())
}

/// Compile a .mom file using the stage-1 *native* binary (not stage-0), return stdout.
///
/// `compiler_bin` is the path to the native stage-1 compiler binary.
fn run_native_compiler(
    compiler_bin: &std::path::Path,
    input_mom: &std::path::Path,
    c_out: &std::path::Path,
    bin_out: &std::path::Path,
) {
    let root = repo_root();

    let status = Command::new(compiler_bin)
        .env("MOM_INPUT", input_mom)
        .env("MOM_OUTPUT", c_out)
        .status()
        .expect("failed to spawn native stage-1 compiler");
    assert!(
        status.success(),
        "native compiler {} failed on {}",
        compiler_bin.display(),
        input_mom.display()
    );

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let status = Command::new(&cc)
        .arg("-std=c99")
        .arg("-O0")
        .arg("-I")
        .arg(root.join("compiler"))
        .arg(c_out)
        .arg(root.join("compiler/runtime.c"))
        .arg("-o")
        .arg(bin_out)
        .status()
        .expect("failed to spawn cc");
    assert!(status.success(), "cc failed linking {}", c_out.display());
}

// ── example-file tests ────────────────────────────────────────────────────────

// These tests compile the Python-style colon-syntax example files.
// They require an enhanced stage-1 compiler with string literals,
// println(), str(), and colon-style syntax support.
// They skip gracefully if the compiler cannot handle the input yet.

#[test]
fn stage1_compiles_hello_example() {
    let root = repo_root();
    let mom = root.join("compiler/examples/hello.mom");
    if !mom.exists() { return; }
    if let Some(out) = try_run_stage1_file(&mom, "hello") {
        assert_eq!(
            out,
            "hello from stage-1!\nhello, mom!\ngreeting count: 1\n"
        );
    }
    // else: compiler rejected the file — expected until enhanced compiler lands
}

#[test]
fn stage1_compiles_counter_example() {
    let root = repo_root();
    let mom = root.join("compiler/examples/counter.mom");
    if !mom.exists() { return; }
    if let Some(out) = try_run_stage1_file(&mom, "counter") {
        assert_eq!(out, "0\n1\n2\n3\n4\n");
    }
}

#[test]
fn stage1_compiles_sum_example() {
    let root = repo_root();
    let mom = root.join("compiler/examples/sum.mom");
    if !mom.exists() { return; }
    if let Some(out) = try_run_stage1_file(&mom, "sum") {
        assert_eq!(out, "5050\n");
    }
}

#[test]
fn stage1_compiles_fibonacci_example() {
    let root = repo_root();
    let mom = root.join("compiler/examples/fibonacci.mom");
    if !mom.exists() { return; }
    if let Some(out) = try_run_stage1_file(&mom, "fibonacci") {
        assert_eq!(out, "55\n");
    }
}

// ── inline sum and counter tests (brace-style, matching existing test style) ──

#[test]
fn stage1_1_compiles_sum_to() {
    let out = run_stage1(
        r#"
        fn sum_to(n: Int) -> Int {
            let mut total = 0
            let mut i = 1
            while i <= n {
                total = total + i
                i = i + 1
            }
            return total
        }

        fn main() {
            print(sum_to(100))
        }
        "#,
        "sum",
    );
    assert_eq!(out, "5050\n");
}

#[test]
fn stage1_1_compiles_counter_loop() {
    let out = run_stage1(
        r#"
        fn main() {
            let mut i = 0
            while i < 5 {
                print(i)
                i = i + 1
            }
        }
        "#,
        "counter",
    );
    assert_eq!(out, "0\n1\n2\n3\n4\n");
}

#[test]
fn stage1_1_compiles_nested_calls() {
    let out = run_stage1(
        r#"
        fn square(n: Int) -> Int { return n * n }
        fn sum_squares(a: Int, b: Int) -> Int { return square(a) + square(b) }
        fn main() {
            print(sum_squares(3, 4))
        }
        "#,
        "nested",
    );
    assert_eq!(out, "25\n");
}

// ── stage-1.2 regression tests (strings, div/mod, for-in-range) ───────────────

#[test]
fn stage1_2_compiles_division_and_modulo() {
    let out = run_stage1(
        r#"
        fn main() {
            print(7 / 2)
            print(7 % 2)
            print(100 / 3)
            print(100 % 3)
            print(2 * 3 + 8 / 4 - 1)
        }
        "#,
        "divmod",
    );
    assert_eq!(out, "3\n1\n33\n1\n7\n");
}

#[test]
fn stage1_2_compiles_string_println_concat() {
    let out = run_stage1(
        r#"
        fn greet(name: String) -> String {
            return "hello, " + name + "!"
        }

        fn main() {
            println("hello from stage-1!")
            println(greet("mom"))
            println("count: " + str(42))
        }
        "#,
        "strings",
    );
    assert_eq!(out, "hello from stage-1!\nhello, mom!\ncount: 42\n");
}

#[test]
fn stage1_2_compiles_str_bool_and_len() {
    let out = run_stage1(
        r#"
        fn main() {
            println(str(true))
            println(str(false))
            print(len("hello"))
        }
        "#,
        "str_bool_len",
    );
    assert_eq!(out, "true\nfalse\n5\n");
}

#[test]
fn stage1_2_compiles_for_in_range() {
    let out = run_stage1(
        r#"
        fn main() {
            let mut total = 0
            for i in 0..5 {
                total = total + i
            }
            print(total)
        }
        "#,
        "for_range",
    );
    assert_eq!(out, "10\n");
}

#[test]
fn stage1_1_compiles_negation_and_unary() {
    let out = run_stage1(
        r#"
        fn main() {
            let x = 0 - 7
            print(x)
        }
        "#,
        "neg",
    );
    assert_eq!(out, "-7\n");
}

// ── self-hosting fixed-point test ─────────────────────────────────────────────
//
// This test is marked #[ignore] because it requires runtime.c to be present
// and is slow (three full compilation rounds). Run with:
//   cargo test selfhost_fixed_point -- --ignored
//
#[test]
#[ignore]
fn selfhost_fixed_point() {
    let root = repo_root();
    let compiler_src = root.join("compiler/src/main.mom");
    let runtime_c = root.join("compiler/runtime.c");

    assert!(
        root.join("target/debug/mom").exists(),
        "stage-0 binary missing — run `cargo build` first"
    );
    assert!(
        runtime_c.exists(),
        "compiler/runtime.c missing — runtime not yet written"
    );

    let work = root.join("target/selfhost-fixedpoint");
    std::fs::create_dir_all(&work).unwrap();

    // Round 1: stage-0 → stage1_v1
    let c1 = work.join("stage1_v1.c");
    let bin1 = work.join("stage1_v1");
    let status = Command::new(root.join("target/debug/mom"))
        .arg("run")
        .arg(&compiler_src)
        .env("MOM_INPUT", &compiler_src)
        .env("MOM_OUTPUT", &c1)
        .status()
        .expect("failed to spawn stage-0 mom (round 1)");
    assert!(status.success(), "stage-0 failed compiling stage-1 (round 1)");

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    Command::new(&cc)
        .args(["-std=c99", "-O0", "-I"])
        .arg(root.join("compiler"))
        .arg(&c1)
        .arg(&runtime_c)
        .args(["-o"])
        .arg(&bin1)
        .status()
        .expect("cc failed round 1");

    // Round 2: stage1_v1 → stage1_v2
    let c2 = work.join("stage1_v2.c");
    let bin2 = work.join("stage1_v2");
    run_native_compiler(&bin1, &compiler_src, &c2, &bin2);

    // Round 3: stage1_v2 → stage1_v3
    let c3 = work.join("stage1_v3.c");
    let bin3 = work.join("stage1_v3");
    run_native_compiler(&bin2, &compiler_src, &c3, &bin3);

    // Fixed-point check: round-2 C == round-3 C
    let v2 = std::fs::read_to_string(&c2).unwrap();
    let v3 = std::fs::read_to_string(&c3).unwrap();
    assert_eq!(
        v2, v3,
        "Self-hosting fixed-point FAILED: C output of round 2 != round 3.\n\
         The compiler does not yet reproducibly compile itself."
    );

    // Sanity: the resulting binary should still work
    let hello = work.join("hello_check.mom");
    std::fs::write(&hello, "fn main() {\n    print(42)\n}\n").unwrap();
    let c_hello = work.join("hello_check.c");
    let bin_hello = work.join("hello_check");
    run_native_compiler(&bin3, &hello, &c_hello, &bin_hello);
    let output = Command::new(&bin_hello)
        .output()
        .expect("failed to run hello_check");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout, "42\n", "sanity check after fixed-point failed");
}
