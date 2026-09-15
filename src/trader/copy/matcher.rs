//! Match observed activity to the copy tasks watching its subject.

use crate::wallets::watch::{WalletActivity, WatchSource};

use super::types::CopyTask;

/// Every task that watches the activity's subject, paused or not. A paused task
/// can still be watching to close what it holds; the caller decides what each
/// task acts on, and the paper and live entry prechecks refuse a paused one.
pub fn matching_tasks<'a>(activity: &WalletActivity, tasks: &'a [CopyTask]) -> Vec<&'a CopyTask> {
    tasks
        .iter()
        .filter(|task| {
            task.target_address == activity.subject
                && activity
                    .sources
                    .contains(&WatchSource::Copy { task_id: task.id })
        })
        .collect()
}
