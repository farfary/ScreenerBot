format-not-available = 該当なし

format-yes = はい
format-no = いいえ
format-unknown = 不明

format-native-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
       *[other] { $amount }秒前
    }
format-ago-minute =
    { $count ->
       *[other] { $amount }分前
    }
format-ago-hour =
    { $count ->
       *[other] { $amount }時間前
    }
format-ago-day =
    { $count ->
       *[other] { $amount }日前
    }

format-ago-span =
    { $count ->
       *[other] { $amount }前
    }
format-just-now = たった今

format-in-second =
    { $count ->
       *[other] { $amount }秒後
    }
format-in-minute =
    { $count ->
       *[other] { $amount }分後
    }
format-in-hour =
    { $count ->
       *[other] { $amount }時間後
    }
format-in-day =
    { $count ->
       *[other] { $amount }日後
    }
format-due = 期限到来

format-unit-day =
    { $count ->
       *[other] { $amount }日
    }
format-unit-hour =
    { $count ->
       *[other] { $amount }時間
    }
format-unit-minute =
    { $count ->
       *[other] { $amount }分
    }
format-unit-second =
    { $count ->
       *[other] { $amount }秒
    }

format-unit-millisecond =
    { $count ->
       *[other] { $amount }ミリ秒
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

format-under-minute = { "<1分" }
