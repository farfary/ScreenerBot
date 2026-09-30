# Words and units used by the dashboard formatters in core/format.js.

format-not-available = लागू नहीं

format-yes = हाँ
format-no = नहीं
format-unknown = अज्ञात

format-sol-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
        [one] { $amount } सेकंड पहले
       *[other] { $amount } सेकंड पहले
    }
format-ago-minute =
    { $count ->
        [one] { $amount } मिनट पहले
       *[other] { $amount } मिनट पहले
    }
format-ago-hour =
    { $count ->
        [one] { $amount } घंटे पहले
       *[other] { $amount } घंटे पहले
    }
format-ago-day =
    { $count ->
        [one] { $amount } दिन पहले
       *[other] { $amount } दिन पहले
    }
format-ago-span =
    { $count ->
        [one] { $amount } पहले
       *[other] { $amount } पहले
    }
format-just-now = अभी-अभी

format-in-second =
    { $count ->
        [one] { $amount } सेकंड में
       *[other] { $amount } सेकंड में
    }
format-in-minute =
    { $count ->
        [one] { $amount } मिनट में
       *[other] { $amount } मिनट में
    }
format-in-hour =
    { $count ->
        [one] { $amount } घंटे में
       *[other] { $amount } घंटे में
    }
format-in-day =
    { $count ->
        [one] { $amount } दिन में
       *[other] { $amount } दिन में
    }
format-due = देय

format-unit-day =
    { $count ->
        [one] { $amount } दिन
       *[other] { $amount } दिन
    }
format-unit-hour =
    { $count ->
        [one] { $amount } घंटा
       *[other] { $amount } घंटे
    }
format-unit-minute =
    { $count ->
        [one] { $amount } मिनट
       *[other] { $amount } मिनट
    }
format-unit-second =
    { $count ->
        [one] { $amount } सेकंड
       *[other] { $amount } सेकंड
    }

format-unit-millisecond =
    { $count ->
        [one] { $amount } ms
       *[other] { $amount } ms
    }
format-unit-microsecond =
    { $count ->
        [one] { $amount } µs
       *[other] { $amount } µs
    }
format-unit-nanosecond =
    { $count ->
        [one] { $amount } ns
       *[other] { $amount } ns
    }

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

format-memory-mb =
    { $count ->
        [one] { $amount } MB
       *[other] { $amount } MB
    }
format-memory-gb =
    { $count ->
        [one] { $amount } GB
       *[other] { $amount } GB
    }

format-under-minute = { "<1 मिनट" }
