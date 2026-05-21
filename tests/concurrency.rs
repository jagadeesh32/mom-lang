//! Phase 3 concurrency primitives — interpreter-level tests.
//! The bootstrap runtime is single-threaded and cooperative; the native
//! work-stealing executor arrives in Phase 3.1.

use mom::run_source;

#[test]
fn channel_send_recv_roundtrip() {
    let out = run_source(
        r#"
        fn main() {
            let ch = Channel(4)
            ch.send(1)
            ch.send(2)
            ch.send(3)
            print(ch.len())
            match ch.recv() {
                Some(v) => print(v),
                None    => print(-1),
            }
            print(ch.len())
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, "3\n1\n2\n");
}

#[test]
fn channel_empty_recv_returns_none() {
    let out = run_source(
        r#"
        fn main() {
            let ch = Channel(2)
            match ch.recv() {
                Some(_) => print("had value"),
                None    => print("empty"),
            }
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, "empty\n");
}

#[test]
fn channel_bounded_capacity_rejects_overflow() {
    let err = run_source(
        r#"
        fn main() {
            let ch = Channel(2)
            ch.send(1)
            ch.send(2)
            ch.send(3)
        }
        "#,
    )
    .unwrap_err();
    assert!(
        err.message.contains("bounded channel at capacity"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn cancel_token_signal() {
    let out = run_source(
        r#"
        fn main() {
            let cancel = Cancel()
            print(cancel.is_cancelled())
            cancel.signal()
            print(cancel.is_cancelled())
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, "false\ntrue\n");
}

#[test]
fn task_spawn_await_lifecycle() {
    let out = run_source(
        r#"
        fn double(x: Int) -> Int { x * 2 }

        fn main() {
            let t = spawn double(21)
            let result = await t
            print(result)
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, "42\n");
}

#[test]
fn sleep_is_a_typed_noop() {
    let out = run_source(
        r#"
        fn main() {
            sleep(100)
            print(42)
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, "42\n");
}

#[test]
fn actor_sugar_desugars_to_struct_and_step_method() {
    // `actor` syntax sugar parses into a struct + a `step(self, msg)`
    // method whose body is the receive's match expression.
    let out = run_source(
        r#"
        enum Msg { Inc, Dec, Reset }

        actor Counter {
            state count: Int,

            receive {
                Inc   => Counter { count: self.count + 1 },
                Dec   => Counter { count: self.count - 1 },
                Reset => Counter { count: 0 },
            }
        }

        fn main() {
            let c0 = Counter { count: 0 }
            let c1 = c0.step(Inc)
            let c2 = c1.step(Inc)
            let c3 = c2.step(Inc)
            let c4 = c3.step(Dec)
            print(c4.count)
            let c5 = c4.step(Reset)
            print(c5.count)
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, "2\n0\n");
}

#[test]
fn channel_powers_actor_like_pattern() {
    // Library-pattern actor: a counter that consumes Inc/Get messages
    // off a channel. Phase 3.1 will offer dedicated `actor` syntax.
    let out = run_source(
        r#"
        enum Msg { Inc, Get }

        fn run(mailbox: Channel[Msg]) -> Int {
            let mut count = 0
            let mut done = false
            while !done {
                match mailbox.recv() {
                    Some(Inc) => { count = count + 1 },
                    Some(Get) => { done = true },
                    None      => { done = true },
                }
            }
            count
        }

        fn main() {
            let mailbox = Channel(8)
            mailbox.send(Inc)
            mailbox.send(Inc)
            mailbox.send(Inc)
            mailbox.send(Get)
            print(run(mailbox))
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, "3\n");
}
