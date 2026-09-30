format-not-available = Yok

format-yes = Evet
format-no = Hayır
format-unknown = Bilinmiyor

format-sol-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
        [one] { $amount }sn önce
       *[other] { $amount }sn önce
    }
format-ago-minute =
    { $count ->
        [one] { $amount }dk önce
       *[other] { $amount }dk önce
    }
format-ago-hour =
    { $count ->
        [one] { $amount }sa önce
       *[other] { $amount }sa önce
    }
format-ago-day =
    { $count ->
        [one] { $amount }g önce
       *[other] { $amount }g önce
    }

format-ago-span =
    { $count ->
        [one] { $amount } önce
       *[other] { $amount } önce
    }
format-just-now = az önce

format-in-second =
    { $count ->
        [one] { $amount}sn içinde
       *[other] { $amount }sn içinde
    }
format-in-minute =
    { $count ->
        [one] { $amount }dk içinde
       *[other] { $amount }dk içinde
    }
format-in-hour =
    { $count ->
        [one] { $amount }sa içinde
       *[other] { $amount }sa içinde
    }
format-in-day =
    { $count ->
        [one] { $amount }g içinde
       *[other] { $amount }g içinde
    }
format-due = vadesi geldi

format-unit-day =
    { $count ->
        [one] { $amount }g
       *[other] { $amount }g
    }
format-unit-hour =
    { $count ->
        [one] { $amount }sa
       *[other] { $amount }sa
    }
format-unit-minute =
    { $count ->
        [one] { $amount }dk
       *[other] { $amount }dk
    }
format-unit-second =
    { $count ->
        [one] { $amount }sn
       *[other] { $amount }sn
    }

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
        [one] { $amount }MB
       *[other] { $amount }MB
    }
format-memory-gb =
    { $count ->
        [one] { $amount }GB
       *[other] { $amount }GB
    }

format-under-minute = { "<1dk" }
