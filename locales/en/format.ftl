# Words and units used by the dashboard formatters in core/format.js.
# $count selects the plural form; $amount is the already formatted number and
# is the only text shown for the quantity.

# Fallback shown when a value has no display form.
format-not-available = N/A

format-yes = Yes
format-no = No
format-unknown = Unknown

# Amount of SOL, for example "0.1500 SOL".
format-sol-amount = { $amount } { -sol }

# Time elapsed since a moment.
format-ago-second =
    { $count ->
        [one] { $amount }s ago
       *[other] { $amount }s ago
    }
format-ago-minute =
    { $count ->
        [one] { $amount }m ago
       *[other] { $amount }m ago
    }
format-ago-hour =
    { $count ->
        [one] { $amount }h ago
       *[other] { $amount }h ago
    }
format-ago-day =
    { $count ->
        [one] { $amount }d ago
       *[other] { $amount }d ago
    }

# Time remaining until a moment.
format-in-second =
    { $count ->
        [one] in { $amount }s
       *[other] in { $amount }s
    }
format-in-minute =
    { $count ->
        [one] in { $amount }m
       *[other] in { $amount }m
    }
format-in-hour =
    { $count ->
        [one] in { $amount }h
       *[other] in { $amount }h
    }
format-in-day =
    { $count ->
        [one] in { $amount }d
       *[other] in { $amount }d
    }
# A scheduled moment that has already been reached.
format-due = due

# Compact time units, for uptimes and spans.
format-unit-day =
    { $count ->
        [one] { $amount }d
       *[other] { $amount }d
    }
format-unit-hour =
    { $count ->
        [one] { $amount }h
       *[other] { $amount }h
    }
format-unit-minute =
    { $count ->
        [one] { $amount }m
       *[other] { $amount }m
    }
format-unit-second =
    { $count ->
        [one] { $amount }s
       *[other] { $amount }s
    }

# Sub-second duration units.
format-unit-millisecond =
    { $count ->
        [one] { $amount }ms
       *[other] { $amount }ms
    }
format-unit-microsecond =
    { $count ->
        [one] { $amount }µs
       *[other] { $amount }µs
    }
format-unit-nanosecond =
    { $count ->
        [one] { $amount }ns
       *[other] { $amount }ns
    }

# Data sizes.
format-bytes-b =
    { $count ->
        [one] { $amount } B
       *[other] { $amount } B
    }
format-bytes-kb =
    { $count ->
        [one] { $amount } KB
       *[other] { $amount } KB
    }
format-bytes-mb =
    { $count ->
        [one] { $amount } MB
       *[other] { $amount } MB
    }
format-bytes-gb =
    { $count ->
        [one] { $amount } GB
       *[other] { $amount } GB
    }
