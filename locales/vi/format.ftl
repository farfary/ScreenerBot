format-not-available = N/A

format-yes = Có
format-no = Không
format-unknown = Không rõ

format-sol-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
       *[other] { $amount } giây trước
    }
format-ago-minute =
    { $count ->
       *[other] { $amount } phút trước
    }
format-ago-hour =
    { $count ->
       *[other] { $amount } giờ trước
    }
format-ago-day =
    { $count ->
       *[other] { $amount } ngày trước
    }

format-ago-span =
    { $count ->
       *[other] { $amount } trước
    }
format-just-now = vừa xong

format-in-second =
    { $count ->
       *[other] sau { $amount } giây
    }
format-in-minute =
    { $count ->
       *[other] sau { $amount } phút
    }
format-in-hour =
    { $count ->
       *[other] sau { $amount } giờ
    }
format-in-day =
    { $count ->
       *[other] sau { $amount } ngày
    }
format-due = đến hạn

format-unit-day =
    { $count ->
       *[other] { $amount } ngày
    }
format-unit-hour =
    { $count ->
       *[other] { $amount } giờ
    }
format-unit-minute =
    { $count ->
       *[other] { $amount } phút
    }
format-unit-second =
    { $count ->
       *[other] { $amount } giây
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

format-under-minute = { "<1" } phút
