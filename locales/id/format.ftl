format-not-available = T/A

format-yes = Ya
format-no = Tidak
format-unknown = Tidak diketahui

format-native-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
       *[other] { $amount } dtk lalu
    }
format-ago-minute =
    { $count ->
       *[other] { $amount } mnt lalu
    }
format-ago-hour =
    { $count ->
       *[other] { $amount } jam lalu
    }
format-ago-day =
    { $count ->
       *[other] { $amount } hari lalu
    }

format-ago-span =
    { $count ->
       *[other] { $amount } lalu
    }
format-just-now = baru saja

format-in-second =
    { $count ->
       *[other] dalam { $amount } dtk
    }
format-in-minute =
    { $count ->
       *[other] dalam { $amount } mnt
    }
format-in-hour =
    { $count ->
       *[other] dalam { $amount } jam
    }
format-in-day =
    { $count ->
       *[other] dalam { $amount } hari
    }
format-due = jatuh tempo

format-unit-day =
    { $count ->
       *[other] { $amount } hari
    }
format-unit-hour =
    { $count ->
       *[other] { $amount } jam
    }
format-unit-minute =
    { $count ->
       *[other] { $amount } mnt
    }
format-unit-second =
    { $count ->
       *[other] { $amount } dtk
    }

format-unit-millisecond =
    { $count ->
       *[other] { $amount }ms
    }
format-unit-microsecond =
    { $count ->
       *[other] { $amount }µs
    }
format-unit-nanosecond =
    { $count ->
       *[other] { $amount }ns
    }

format-bytes-b =
    { $count ->
       *[other] { $amount } B
    }
format-bytes-kb =
    { $count ->
       *[other] { $amount } KB
    }
format-bytes-mb =
    { $count ->
       *[other] { $amount } MB
    }
format-bytes-gb =
    { $count ->
       *[other] { $amount } GB
    }

format-memory-mb =
    { $count ->
       *[other] { $amount }MB
    }
format-memory-gb =
    { $count ->
       *[other] { $amount }GB
    }

# A call rate per minute; $amount is the formatted number.
format-calls-per-minute = { $amount }/mnt

format-under-minute = { "<1 mnt" }
