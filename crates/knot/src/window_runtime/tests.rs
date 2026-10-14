//! Teardown with a blocking task that will not finish on its own.

use std::future::Future;
use std::pin::pin;
use std::sync::{Arc, mpsc};
use std::task::{Context, Poll, Wake};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use super::WindowRuntime;
use crate::consts::WINDOW_RUNTIME_SHUTDOWN_TIMEOUT;

/// A plain `Runtime` would wait for this task forever - it blocks until the
/// test lets it go, which is after the drop. The window's runtime gives up
/// after its bound.
#[test]
fn dropping_does_not_wait_for_a_blocking_task_that_never_ends() {
    let runtime = WindowRuntime::new().expect("a runtime");
    let (release, released) = mpsc::channel::<()>();
    let (started_tx, started_rx) = mpsc::channel();
    runtime.spawn_blocking(move || {
               let _ = started_tx.send(());
               let _ = released.recv();
           });
    started_rx.recv_timeout(Duration::from_secs(10))
              .expect("the blocking task starts");

    let dropping = Instant::now();
    drop(runtime);
    let took = dropping.elapsed();

    // Let the detached task end, so the test leaves no thread behind.
    let _ = release.send(());
    assert!(took < WINDOW_RUNTIME_SHUTDOWN_TIMEOUT + Duration::from_secs(2),
            "dropping the window's runtime took {took:?}");
}

/// The bound is a ceiling, not a delay: with nothing running, teardown is
/// immediate.
#[test]
fn dropping_an_idle_runtime_is_immediate() {
    let runtime = WindowRuntime::new().expect("a runtime");

    let dropping = Instant::now();
    drop(runtime);

    assert!(dropping.elapsed() < WINDOW_RUNTIME_SHUTDOWN_TIMEOUT,
            "an idle runtime took {:?} to drop",
            dropping.elapsed());
}

/// Wakes the thread blocked in [`block_on_without_tokio`].
struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Polls `future` to completion on the calling thread with no tokio runtime
/// entered - as GPUI's executor does.
fn block_on_without_tokio<F: Future>(future: F) -> F::Output {
    let waker = Arc::new(ThreadWaker(thread::current())).into();
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        thread::park_timeout(Duration::from_millis(50));
    }
}

/// Issue #252: the settings window's adapter check awaited a
/// `tokio::process` future straight from GPUI, which has no reactor, and the
/// app crashed with "there is no reactor running". Spawned on the window's
/// runtime, the child runs there and only the join handle - which needs no
/// reactor - is awaited from outside.
#[test]
fn a_child_process_spawned_on_the_runtime_can_be_awaited_without_tokio() {
    let runtime = WindowRuntime::new().expect("a runtime");
    let child = runtime.spawn(async { tokio::process::Command::new("true").status().await });

    let status = block_on_without_tokio(child).expect("the task completes")
                                              .expect("`true` runs");

    assert!(status.success());
}
