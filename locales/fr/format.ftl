# Words and units used by the dashboard formatters in core/format.js.

format-not-available = N/D

format-yes = Oui
format-no = Non
format-unknown = Inconnu

format-native-amount = { $amount } { -sol }

format-usd-amount = { $amount } $

format-percent-amount = { $amount } %

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
        [one] il y a { $amount } s
        [many] il y a { $amount } s
       *[other] il y a { $amount } s
    }
format-ago-minute =
    { $count ->
        [one] il y a { $amount } min
        [many] il y a { $amount } min
       *[other] il y a { $amount } min
    }
format-ago-hour =
    { $count ->
        [one] il y a { $amount } h
        [many] il y a { $amount } h
       *[other] il y a { $amount } h
    }
format-ago-day =
    { $count ->
        [one] il y a { $amount } j
        [many] il y a { $amount } j
       *[other] il y a { $amount } j
    }

format-ago-span =
    { $count ->
        [one] il y a { $amount }
        [many] il y a { $amount }
       *[other] il y a { $amount }
    }
format-just-now = à l'instant

format-in-second =
    { $count ->
        [one] dans { $amount } s
        [many] dans { $amount } s
       *[other] dans { $amount } s
    }
format-in-minute =
    { $count ->
        [one] dans { $amount } min
        [many] dans { $amount } min
       *[other] dans { $amount } min
    }
format-in-hour =
    { $count ->
        [one] dans { $amount } h
        [many] dans { $amount } h
       *[other] dans { $amount } h
    }
format-in-day =
    { $count ->
        [one] dans { $amount } j
        [many] dans { $amount } j
       *[other] dans { $amount } j
    }
format-due = échu

format-unit-day =
    { $count ->
        [one] { $amount }j
        [many] { $amount }j
       *[other] { $amount }j
    }
format-unit-hour =
    { $count ->
        [one] { $amount }h
        [many] { $amount }h
       *[other] { $amount }h
    }
format-unit-minute =
    { $count ->
        [one] { $amount }min
        [many] { $amount }min
       *[other] { $amount }min
    }
format-unit-second =
    { $count ->
        [one] { $amount }s
        [many] { $amount }s
       *[other] { $amount }s
    }

format-unit-millisecond =
    { $count ->
        [one] { $amount }ms
        [many] { $amount }ms
       *[other] { $amount }ms
    }
format-unit-microsecond =
    { $count ->
        [one] { $amount }µs
        [many] { $amount }µs
       *[other] { $amount }µs
    }
format-unit-nanosecond =
    { $count ->
        [one] { $amount }ns
        [many] { $amount }ns
       *[other] { $amount }ns
    }

format-bytes-b =
    { $count ->
        [one] { $amount } o
        [many] { $amount } o
       *[other] { $amount } o
    }
format-bytes-kb =
    { $count ->
        [one] { $amount } Ko
        [many] { $amount } Ko
       *[other] { $amount } Ko
    }
format-bytes-mb =
    { $count ->
        [one] { $amount } Mo
        [many] { $amount } Mo
       *[other] { $amount } Mo
    }
format-bytes-gb =
    { $count ->
        [one] { $amount } Go
        [many] { $amount } Go
       *[other] { $amount } Go
    }

format-memory-mb =
    { $count ->
        [one] { $amount }Mo
        [many] { $amount }Mo
       *[other] { $amount }Mo
    }
format-memory-gb =
    { $count ->
        [one] { $amount }Go
        [many] { $amount }Go
       *[other] { $amount }Go
    }

format-under-minute = { "<1min" }
