# Words and units used by the dashboard formatters in core/format.js.

format-not-available = н/д

format-yes = Так
format-no = Ні
format-unknown = Невідомо

format-native-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
        [one] { $amount } с тому
        [few] { $amount } с тому
        [many] { $amount } с тому
       *[other] { $amount } с тому
    }
format-ago-minute =
    { $count ->
        [one] { $amount } хв тому
        [few] { $amount } хв тому
        [many] { $amount } хв тому
       *[other] { $amount } хв тому
    }
format-ago-hour =
    { $count ->
        [one] { $amount } год тому
        [few] { $amount } год тому
        [many] { $amount } год тому
       *[other] { $amount } год тому
    }
format-ago-day =
    { $count ->
        [one] { $amount } д тому
        [few] { $amount } д тому
        [many] { $amount } д тому
       *[other] { $amount } д тому
    }

format-ago-span =
    { $count ->
        [one] { $amount } тому
        [few] { $amount } тому
        [many] { $amount } тому
       *[other] { $amount } тому
    }
format-just-now = щойно

format-in-second =
    { $count ->
        [one] через { $amount } с
        [few] через { $amount } с
        [many] через { $amount } с
       *[other] через { $amount } с
    }
format-in-minute =
    { $count ->
        [one] через { $amount } хв
        [few] через { $amount } хв
        [many] через { $amount } хв
       *[other] через { $amount } хв
    }
format-in-hour =
    { $count ->
        [one] через { $amount } год
        [few] через { $amount } год
        [many] через { $amount } год
       *[other] через { $amount } год
    }
format-in-day =
    { $count ->
        [one] через { $amount } д
        [few] через { $amount } д
        [many] через { $amount } д
       *[other] через { $amount } д
    }
format-due = настав час

format-unit-day =
    { $count ->
        [one] { $amount } д
        [few] { $amount } д
        [many] { $amount } д
       *[other] { $amount } д
    }
format-unit-hour =
    { $count ->
        [one] { $amount } год
        [few] { $amount } год
        [many] { $amount } год
       *[other] { $amount } год
    }
format-unit-minute =
    { $count ->
        [one] { $amount } хв
        [few] { $amount } хв
        [many] { $amount } хв
       *[other] { $amount } хв
    }
format-unit-second =
    { $count ->
        [one] { $amount } с
        [few] { $amount } с
        [many] { $amount } с
       *[other] { $amount } с
    }

format-unit-millisecond =
    { $count ->
        [one] { $amount } мс
        [few] { $amount } мс
        [many] { $amount } мс
       *[other] { $amount } мс
    }
format-unit-microsecond =
    { $count ->
        [one] { $amount } мкс
        [few] { $amount } мкс
        [many] { $amount } мкс
       *[other] { $amount } мкс
    }
format-unit-nanosecond =
    { $count ->
        [one] { $amount } нс
        [few] { $amount } нс
        [many] { $amount } нс
       *[other] { $amount } нс
    }

format-bytes-b =
    { $count ->
        [one] { $amount } Б
        [few] { $amount } Б
        [many] { $amount } Б
       *[other] { $amount } Б
    }
format-bytes-kb =
    { $count ->
        [one] { $amount } КБ
        [few] { $amount } КБ
        [many] { $amount } КБ
       *[other] { $amount } КБ
    }
format-bytes-mb =
    { $count ->
        [one] { $amount } МБ
        [few] { $amount } МБ
        [many] { $amount } МБ
       *[other] { $amount } МБ
    }
format-bytes-gb =
    { $count ->
        [one] { $amount } ГБ
        [few] { $amount } ГБ
        [many] { $amount } ГБ
       *[other] { $amount } ГБ
    }

format-memory-mb =
    { $count ->
        [one] { $amount } МБ
        [few] { $amount } МБ
        [many] { $amount } МБ
       *[other] { $amount } МБ
    }
format-memory-gb =
    { $count ->
        [one] { $amount } ГБ
        [few] { $amount } ГБ
        [many] { $amount } ГБ
       *[other] { $amount } ГБ
    }

format-under-minute = { "<1 хв" }
