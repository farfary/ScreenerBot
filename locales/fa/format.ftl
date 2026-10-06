format-not-available = ندارد

format-yes = بله
format-no = خیر
format-unknown = نامشخص

format-native-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
        [one] { $amount } ثانیه پیش
       *[other] { $amount } ثانیه پیش
    }
format-ago-minute =
    { $count ->
        [one] { $amount } دقیقه پیش
       *[other] { $amount } دقیقه پیش
    }
format-ago-hour =
    { $count ->
        [one] { $amount } ساعت پیش
       *[other] { $amount } ساعت پیش
    }
format-ago-day =
    { $count ->
        [one] { $amount } روز پیش
       *[other] { $amount } روز پیش
    }

format-ago-span =
    { $count ->
        [one] { $amount } پیش
       *[other] { $amount } پیش
    }
format-just-now = هم‌اکنون

format-in-second =
    { $count ->
        [one] { $amount } ثانیه دیگر
       *[other] { $amount } ثانیه دیگر
    }
format-in-minute =
    { $count ->
        [one] { $amount } دقیقه دیگر
       *[other] { $amount } دقیقه دیگر
    }
format-in-hour =
    { $count ->
        [one] { $amount } ساعت دیگر
       *[other] { $amount } ساعت دیگر
    }
format-in-day =
    { $count ->
        [one] { $amount } روز دیگر
       *[other] { $amount } روز دیگر
    }
format-due = سررسید

format-unit-second =
    { $count ->
        [one] { $amount } ثانیه
       *[other] { $amount } ثانیه
    }
format-unit-minute =
    { $count ->
        [one] { $amount } دقیقه
       *[other] { $amount } دقیقه
    }
format-unit-hour =
    { $count ->
        [one] { $amount } ساعت
       *[other] { $amount } ساعت
    }
format-unit-day =
    { $count ->
        [one] { $amount } روز
       *[other] { $amount } روز
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

format-under-minute = { "<1 دقیقه" }
