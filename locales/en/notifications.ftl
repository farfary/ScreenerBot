# Notification panel labels.

# Action types. Ids come from ActionType in src/actions/types.rs.
notifications-action-swap-buy = Buy
notifications-action-swap-sell = Sell
notifications-action-position-open = Open
notifications-action-position-close = Close
notifications-action-position-dca = DCA
notifications-action-position-partial-exit = Partial Exit
notifications-action-manual-order = Manual
notifications-action-unknown = Action

# Action steps. Ids come from ActionStepCode in src/actions/step_code.rs.
actions-step-evaluate = Evaluating
actions-step-validate = Validating
actions-step-quote = Getting Quote
actions-step-swap = Executing Swap
actions-step-verify = Verifying
actions-step-unknown = Processing
actions-step-evaluate-short = Evaluating
actions-step-validate-short = Checking
actions-step-quote-short = Quote
actions-step-swap-short = Swapping
actions-step-verify-short = Confirming
actions-step-unknown-short = Working

# Action failures. Ids come from src/actions/failure.rs and its producers. The
# technical cause travels separately as `details` and is not part of the text.
actions-failure-recorded = { $message }
actions-failure-unknown = Unknown error
actions-failure-interrupted = Interrupted by application restart
actions-failure-validation = Validation failed
actions-failure-quote = Quote failed
actions-failure-swap = Swap failed
actions-failure-trade = Trade failed
actions-failure-entry = Entry failed
actions-failure-exit = Exit failed
actions-failure-dca = DCA failed
actions-failure-verification-expired = Verification expired: the transaction never landed
actions-failure-verification-gave-up = Verification gave up
actions-failure-transaction-failed = The transaction failed on chain
actions-failure-sell-transaction-failed = The sell transaction failed on chain
actions-failure-dca-verification-failed = DCA verification failed

# Notification drawer behaviour (ui/notification_panel.js). Labels of the drawer chrome are in shell.ftl.
notifications-empty-all = No actions
notifications-empty-active = No active actions
notifications-empty-completed = No completed actions
notifications-empty-failed = No failed actions
notifications-source-auto = Auto
notifications-source-manual = Manual
notifications-state-locked = State is controlled by tab
notifications-cancelled = Cancelled
notifications-dismiss = Dismiss
notifications-dismiss-failed = Failed to dismiss notification
notifications-load-failed = Failed to load
notifications-mark-read-failed = Failed to mark notifications read
notifications-clear-title = Clear notifications
notifications-clear-message = Dismiss all notifications from this list? They remain in the Completed/Failed history.
notifications-clear-failed = Failed to clear notifications
notifications-stream-lag-title = Action stream fell behind
# $count is how many updates were skipped.
notifications-stream-lag-missed =
    Missed { $count ->
        [one] { $count } update
       *[other] { $count } updates
    } — refreshing
notifications-stream-lag-refreshing = Refreshing
notifications-sync-failed = Could not refresh actions
