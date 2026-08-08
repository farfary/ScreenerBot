//! Match observed activity to enabled copy tasks by subject.

use crate::wallets::watch::{WalletActivity, WatchSource};

use super::types::CopyTask;

pub fn matching_tasks<'a>(activity: &WalletActivity, tasks: &'a [CopyTask]) -> Vec<&'a CopyTask> {
    tasks
        .iter()
        .filter(|task| {
            task.enabled
                && task.target_address == activity.subject
                && activity
                    .sources
                    .contains(&WatchSource::Copy { task_id: task.id })
        })
        .collect()
}
