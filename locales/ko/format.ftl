format-not-available = 해당 없음

format-yes = 예
format-no = 아니요
format-unknown = 알 수 없음

format-native-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
       *[other] { $amount }초 전
    }
format-ago-minute =
    { $count ->
       *[other] { $amount }분 전
    }
format-ago-hour =
    { $count ->
       *[other] { $amount }시간 전
    }
format-ago-day =
    { $count ->
       *[other] { $amount }일 전
    }

format-ago-span =
    { $count ->
       *[other] { $amount } 전
    }
format-just-now = 방금

format-in-second =
    { $count ->
       *[other] { $amount }초 후
    }
format-in-minute =
    { $count ->
       *[other] { $amount }분 후
    }
format-in-hour =
    { $count ->
       *[other] { $amount }시간 후
    }
format-in-day =
    { $count ->
       *[other] { $amount }일 후
    }
format-due = 기한 도래

format-unit-day =
    { $count ->
       *[other] { $amount }일
    }
format-unit-hour =
    { $count ->
       *[other] { $amount }시간
    }
format-unit-minute =
    { $count ->
       *[other] { $amount }분
    }
format-unit-second =
    { $count ->
       *[other] { $amount }초
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
format-calls-per-minute = { $amount }/분

format-under-minute = { "<1분" }
