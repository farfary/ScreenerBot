format-not-available = Н/Д

format-yes = Да
format-no = Нет
format-unknown = Неизвестно

format-native-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
        [one] { $amount } с назад
        [few] { $amount } с назад
        [many] { $amount } с назад
       *[other] { $amount } с назад
    }
format-ago-minute =
    { $count ->
        [one] { $amount } мин назад
        [few] { $amount } мин назад
        [many] { $amount } мин назад
       *[other] { $amount } мин назад
    }
format-ago-hour =
    { $count ->
        [one] { $amount } ч назад
        [few] { $amount } ч назад
        [many] { $amount } ч назад
       *[other] { $amount } ч назад
    }
format-ago-day =
    { $count ->
        [one] { $amount } д назад
        [few] { $amount } д назад
        [many] { $amount } д назад
       *[other] { $amount } д назад
    }
format-ago-span =
    { $count ->
        [one] { $amount } назад
        [few] { $amount } назад
        [many] { $amount } назад
       *[other] { $amount } назад
    }
format-just-now = только что

format-in-second =
    { $count ->
        [one] через { $amount } с
        [few] через { $amount } с
        [many] через { $amount } с
       *[other] через { $amount } с
    }
format-in-minute =
    { $count ->
        [one] через { $amount } мин
        [few] через { $amount } мин
        [many] через { $amount } мин
       *[other] через { $amount } мин
    }
format-in-hour =
    { $count ->
        [one] через { $amount } ч
        [few] через { $amount } ч
        [many] через { $amount } ч
       *[other] через { $amount } ч
    }
format-in-day =
    { $count ->
        [one] через { $amount } д
        [few] через { $amount } д
        [many] через { $amount } д
       *[other] через { $amount } д
    }
format-due = пора

format-unit-day =
    { $count ->
        [one] { $amount } д
        [few] { $amount } д
        [many] { $amount } д
       *[other] { $amount } д
    }
format-unit-hour =
    { $count ->
        [one] { $amount } ч
        [few] { $amount } ч
        [many] { $amount } ч
       *[other] { $amount } ч
    }
format-unit-minute =
    { $count ->
        [one] { $amount } мин
        [few] { $amount } мин
        [many] { $amount } мин
       *[other] { $amount } мин
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

# A call rate per minute; $amount is the formatted number.
format-calls-per-minute = { $amount }/мин
format-under-minute = { "<1 мин" }
