format-not-available = k. A.

format-yes = Ja
format-no = Nein
format-unknown = Unbekannt

format-native-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount } %

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
        [one] vor { $amount } s
       *[other] vor { $amount } s
    }
format-ago-minute =
    { $count ->
        [one] vor { $amount } Min.
       *[other] vor { $amount } Min.
    }
format-ago-hour =
    { $count ->
        [one] vor { $amount } Std.
       *[other] vor { $amount } Std.
    }
format-ago-day =
    { $count ->
        [one] vor { $amount } T.
       *[other] vor { $amount } T.
    }

format-ago-span =
    { $count ->
        [one] vor { $amount }
       *[other] vor { $amount }
    }
format-just-now = gerade eben

format-in-second =
    { $count ->
        [one] in { $amount } s
       *[other] in { $amount } s
    }
format-in-minute =
    { $count ->
        [one] in { $amount } Min.
       *[other] in { $amount } Min.
    }
format-in-hour =
    { $count ->
        [one] in { $amount } Std.
       *[other] in { $amount } Std.
    }
format-in-day =
    { $count ->
        [one] in { $amount } T.
       *[other] in { $amount } T.
    }
format-due = fällig

format-unit-day =
    { $count ->
        [one] { $amount } T.
       *[other] { $amount } T.
    }
format-unit-hour =
    { $count ->
        [one] { $amount } Std.
       *[other] { $amount } Std.
    }
format-unit-minute =
    { $count ->
        [one] { $amount } Min.
       *[other] { $amount } Min.
    }
format-unit-second =
    { $count ->
        [one] { $amount } s
       *[other] { $amount } s
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

# A call rate per minute; $amount is the formatted number.
format-calls-per-minute = { $amount }/Min.

format-under-minute = { "<1 Min." }
