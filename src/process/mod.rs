//! Process-level concerns: the single-instance lock, profiling, the shutdown flag, the panic hook and restart.

pub mod lock;
pub mod panic_hook;
pub mod profiling;
pub mod restart;
pub mod shutdown;
