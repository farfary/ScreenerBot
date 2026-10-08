// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The messages down an error's cause chain, so a diagnosis shows the failure that started it and not only the outermost wrapper.

/// Each message from `error` down its `source()` chain. A level whose message is
/// already part of the level above it (a wrapper that prints its source, or a
/// transparent one) is skipped, so every cause appears once.
pub fn source_chain(error: &(dyn std::error::Error + 'static)) -> Vec<String> {
    let mut messages: Vec<String> = Vec::new();
    let mut current = Some(error);
    while let Some(level) = current {
        let message = level.to_string();
        let repeated = messages
            .last()
            .is_some_and(|previous| previous.contains(&message));
        if !repeated && !message.is_empty() {
            messages.push(message);
        }
        current = level.source();
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    enum Inner {
        #[error("positions schema migration failed: unrecognized column positions.flag")]
        Refused,
    }

    #[derive(Debug, thiserror::Error)]
    enum Middle {
        #[error(transparent)]
        Positions(#[from] Inner),
    }

    #[derive(Debug, thiserror::Error)]
    enum Outer {
        #[error("service lifecycle failed")]
        Core {
            #[source]
            source: Middle,
        },
        #[error("start failed: {source}")]
        Repeating {
            #[source]
            source: Inner,
        },
    }

    #[test]
    fn the_root_cause_follows_its_wrappers_once() {
        let error = Outer::Core {
            source: Middle::from(Inner::Refused),
        };
        assert_eq!(
            source_chain(&error),
            vec![
                "service lifecycle failed".to_owned(),
                "positions schema migration failed: unrecognized column positions.flag".to_owned(),
            ]
        );
    }

    #[test]
    fn a_wrapper_that_prints_its_source_is_not_repeated() {
        let error = Outer::Repeating {
            source: Inner::Refused,
        };
        assert_eq!(
            source_chain(&error),
            vec![
                "start failed: positions schema migration failed: unrecognized column positions.flag"
                    .to_owned()
            ]
        );
    }
}
