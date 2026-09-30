# Event display text. Default ids come from src/events/recorders.rs; task ids
# from ScheduledTaskOutcome in src/events/display_text.rs. Arguments are data
# (subtype codes, method and API names, task names) and are not translated.
events-ohlcv-default = OHLCV event: { $subtype }
events-filtering-default = Filtering event: { $subtype }
events-trader-default = Trader event: { $subtype }
events-rpc-default = RPC { $method } - { $action }
events-api-default = { $api } - { $action }

# $name is the user-chosen scheduled task name.
events-task-completed = Task '{ $name }' completed
events-task-failed = Task '{ $name }' failed
events-task-timed-out = Task '{ $name }' timed out

# Event type column labels for the stable scheduled-task subtype codes.
events-subtype-task-completed = Task completed
events-subtype-task-failed = Task failed
events-subtype-task-timed-out = Task timed out

# Shown when an event carries no display text.
events-message-none = No message
