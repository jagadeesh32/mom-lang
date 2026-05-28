//! Phase 1 acceptance tests — compile a `.mom` source to a native
//! binary, execute it, and assert stdout.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use mom::build::{build, BuildOptions};

static SEQ: AtomicUsize = AtomicUsize::new(0);

fn target_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("mom-test")
}

fn unique_name(prefix: &str) -> String {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{t}-{n}")
}

fn compile_and_run(source: &str) -> String {
    let tmp = target_root();
    std::fs::create_dir_all(&tmp).unwrap();

    let stem = unique_name("snippet");
    let source_path = tmp.join(format!("{stem}.mom"));
    let bin_path = tmp.join(&stem);
    std::fs::write(&source_path, source).unwrap();

    let mut options = BuildOptions::new(source_path.clone(), bin_path.clone());
    options.cache_dir = tmp.join(format!("{stem}-cache"));
    let report = build(&options).expect("compile failed");
    assert!(report.output.exists(), "binary missing");

    // ETXTBSY guard: when many native_build tests run in parallel,
    // an unrelated test's fork() can briefly inherit a writable fd to
    // our just-written binary, making exec fail with "Text file busy".
    // It clears on its own within a handful of milliseconds. Retry a
    // few times before giving up.
    let mut attempts = 0;
    let output = loop {
        attempts += 1;
        match Command::new(&bin_path).output() {
            Ok(o) => break o,
            Err(err) if err.raw_os_error() == Some(26) && attempts < 10 => {
                std::thread::sleep(std::time::Duration::from_millis(25));
                continue;
            }
            Err(err) => panic!("failed to run produced binary: {err:?}"),
        }
    };
    assert!(
        output.status.success(),
        "binary exited non-zero: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("non-utf8 stdout")
        .replace("\r\n", "\n")
}

#[test]
fn native_hello_int() {
    let out = compile_and_run(
        r#"
        fn main() {
            print(42)
        }
        "#,
    );
    assert_eq!(out, "42\n");
}

#[test]
fn native_recursion_fib() {
    let out = compile_and_run(
        r#"
        fn fib(n: Int) -> Int {
            if n <= 1 {
                n
            } else {
                fib(n - 1) + fib(n - 2)
            }
        }

        fn main() {
            print(fib(10))
        }
        "#,
    );
    assert_eq!(out, "55\n");
}

#[test]
fn native_mutable_state() {
    let out = compile_and_run(
        r#"
        fn main() {
            let mut total = 0
            let mut i = 0
            while i < 5 {
                total = total + i
                i = i + 1
            }
            print(total)
        }
        "#,
    );
    assert_eq!(out, "10\n");
}

#[test]
fn native_for_in_range() {
    let out = compile_and_run(
        r#"
        fn main() {
            let mut total = 0
            for i in 0..6 {
                total = total + i
            }
            print(total)
        }
        "#,
    );
    assert_eq!(out, "15\n");
}

#[test]
fn native_bool_print() {
    let out = compile_and_run(
        r#"
        fn even(n: Int) -> Bool {
            n % 2 == 0
        }

        fn main() {
            print(even(4))
            print(even(7))
        }
        "#,
    );
    assert_eq!(out, "true\nfalse\n");
}

#[test]
fn native_pipeline() {
    let out = compile_and_run(
        r#"
        fn inc(x: Int) -> Int    { x + 1 }
        fn double(x: Int) -> Int { x * 2 }

        fn main() {
            let v = 20 |> inc |> double
            print(v)
        }
        "#,
    );
    assert_eq!(out, "42\n");
}

#[test]
fn build_cache_hits_on_repeat() {
    let tmp = target_root();
    std::fs::create_dir_all(&tmp).unwrap();
    let stem = unique_name("cache");
    let source_path = tmp.join(format!("{stem}.mom"));
    let bin_path = tmp.join(&stem);
    std::fs::write(
        &source_path,
        format!(r#"fn main() {{ print({}) }}"#, stem.len() as i64),
    )
    .unwrap();

    let mut options = BuildOptions::new(source_path, bin_path);
    options.cache_dir = tmp.join(format!("{stem}-cache"));
    let first = build(&options).expect("first build failed");
    assert!(!first.from_cache, "first build should be fresh");

    let second = build(&options).expect("second build failed");
    assert!(second.from_cache, "second build should hit cache");
}

#[test]
fn native_float_arithmetic_and_print() {
    let out = compile_and_run(
        r#"
        fn area(radius: Float) -> Float {
            3.14 * radius * radius
        }

        fn main() {
            print(area(2.0))         // 12.56
            print(1.5 + 0.5)         // 2 (whole-number Float prints as int)
            print(-3.25)             // -3.25
        }
        "#,
    );
    assert_eq!(out, "12.56\n2\n-3.25\n");
}

#[test]
fn native_float_comparison_returns_bool() {
    let out = compile_and_run(
        r#"
        fn main() {
            print(1.0 < 2.0)
            print(1.0 == 1.0)
            print(2.5 >= 3.0)
        }
        "#,
    );
    assert_eq!(out, "true\ntrue\nfalse\n");
}

#[test]
fn native_rejects_unsupported_constructs() {
    // Strings aren't part of the Phase 1 codegen subset.
    let tmp = target_root();
    std::fs::create_dir_all(&tmp).unwrap();
    let stem = unique_name("reject");
    let source_path = tmp.join(format!("{stem}.mom"));
    let bin_path = tmp.join(&stem);
    std::fs::write(&source_path, r#"fn main() { print("hi") }"#).unwrap();

    let mut options = BuildOptions::new(source_path, bin_path);
    options.cache_dir = tmp.join(format!("{stem}-cache"));
    let err = build(&options).expect_err("expected a codegen diagnostic");
    let msg = format!("{err}");
    assert!(
        msg.contains("String") || msg.contains("not yet"),
        "unexpected diagnostic: {msg}",
    );
}
