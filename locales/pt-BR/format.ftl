format-not-available = N/D

format-yes = Sim
format-no = Não
format-unknown = Desconhecido

format-native-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
        [one] há { $amount }s
        [many] há { $amount }s
       *[other] há { $amount }s
    }
format-ago-minute =
    { $count ->
        [one] há { $amount }min
        [many] há { $amount }min
       *[other] há { $amount }min
    }
format-ago-hour =
    { $count ->
        [one] há { $amount }h
        [many] há { $amount }h
       *[other] há { $amount }h
    }
format-ago-day =
    { $count ->
        [one] há { $amount }d
        [many] há { $amount }d
       *[other] há { $amount }d
    }

format-ago-span =
    { $count ->
        [one] há { $amount }
        [many] há { $amount }
       *[other] há { $amount }
    }
format-just-now = agora mesmo

format-in-second =
    { $count ->
        [one] em { $amount }s
        [many] em { $amount }s
       *[other] em { $amount }s
    }
format-in-minute =
    { $count ->
        [one] em { $amount }min
        [many] em { $amount }min
       *[other] em { $amount }min
    }
format-in-hour =
    { $count ->
        [one] em { $amount }h
        [many] em { $amount }h
       *[other] em { $amount }h
    }
format-in-day =
    { $count ->
        [one] em { $amount }d
        [many] em { $amount }d
       *[other] em { $amount }d
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
