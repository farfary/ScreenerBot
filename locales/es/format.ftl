format-not-available = N/D

format-yes = Sí
format-no = No
format-unknown = Desconocido

format-native-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
        [one] hace { $amount } s
        [many] hace { $amount } s
       *[other] hace { $amount } s
    }
format-ago-minute =
    { $count ->
        [one] hace { $amount } min
        [many] hace { $amount } min
       *[other] hace { $amount } min
    }
format-ago-hour =
    { $count ->
        [one] hace { $amount } h
        [many] hace { $amount } h
       *[other] hace { $amount } h
    }
format-ago-day =
    { $count ->
        [one] hace { $amount } d
        [many] hace { $amount } d
       *[other] hace { $amount } d
    }
format-ago-span =
    { $count ->
        [one] hace { $amount }
        [many] hace { $amount }
       *[other] hace { $amount }
    }
format-just-now = ahora mismo

format-in-second =
    { $count ->
        [one] en { $amount } s
        [many] en { $amount } s
       *[other] en { $amount } s
    }
format-in-minute =
    { $count ->
        [one] en { $amount } min
        [many] en { $amount } min
       *[other] en { $amount } min
    }
format-in-hour =
    { $count ->
        [one] en { $amount } h
        [many] en { $amount } h
       *[other] en { $amount } h
    }
format-in-day =
    { $count ->
        [one] en { $amount } d
        [many] en { $amount } d
       *[other] en { $amount } d
    }
format-due = vencido

format-unit-day =
    { $count ->
        [one] { $amount }d
        [many] { $amount }d
       *[other] { $amount }d
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
        [one] { $amount } B
        [many] { $amount } B
       *[other] { $amount } B
    }
format-bytes-kb =
    { $count ->
        [one] { $amount } KB
        [many] { $amount } KB
       *[other] { $amount } KB
    }
format-bytes-mb =
    { $count ->
        [one] { $amount } MB
        [many] { $amount } MB
       *[other] { $amount } MB
    }
format-bytes-gb =
    { $count ->
        [one] { $amount } GB
        [many] { $amount } GB
       *[other] { $amount } GB
    }
format-memory-mb =
    { $count ->
        [one] { $amount }MB
        [many] { $amount }MB
       *[other] { $amount }MB
    }
format-memory-gb =
    { $count ->
        [one] { $amount }GB
        [many] { $amount }GB
       *[other] { $amount }GB
    }

# A call rate per minute; $amount is the formatted number.
format-calls-per-minute = { $amount }/min
format-under-minute = { "<1min" }
