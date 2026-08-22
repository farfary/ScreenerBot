//! The process-wide shutdown flag.

use std::sync::atomic::{AtomicBool, Ordering};

/// Process-wide "the app is shutting down" flag.
///
/// Lives in the library, not in `main`, because the modules that need to *read*
/// it are all library modules: once shutdown begins, in-flight work is
/// abandoned on purpose, and a component that fails because a peer has already
/// torn down its channel is reporting an expected consequence of exiting, not a
/// fault. Without a library-visible flag those components had no way to tell the
/// two apart, so they logged teardown as failure — the pool analyzer filed a
/// warning for every fetch request it handed to the already-closed fetcher
/// channel, burying the real shutdown sequence.
static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

/// Whether shutdown has been requested. Use this to downgrade or suppress a
/// diagnostic that is only meaningful while the app is running — never to skip
/// cleanup work.
pub fn is_shutdown_requested() -> bool {
    SHUTDOWN_FLAG.load(Ordering::SeqCst)
}

/// Request application shutdown.
pub fn request_shutdown() {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
}
