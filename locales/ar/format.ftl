# Words and units used by the dashboard formatters in core/format.js.

format-not-available = غير متاح

format-yes = نعم
format-no = لا
format-unknown = غير معروف

format-native-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
    [zero] منذ { $amount } ثانية
    [one] منذ { $amount } ثانية
    [two] منذ { $amount } ثانيتين
    [few] منذ { $amount } ثوانٍ
    [many] منذ { $amount } ثانية
   *[other] منذ { $amount } ثانية
    }
format-ago-minute =
    { $count ->
    [zero] منذ { $amount } دقيقة
    [one] منذ { $amount } دقيقة
    [two] منذ { $amount } دقيقتين
    [few] منذ { $amount } دقائق
    [many] منذ { $amount } دقيقة
   *[other] منذ { $amount } دقيقة
    }
format-ago-hour =
    { $count ->
    [zero] منذ { $amount } ساعة
    [one] منذ { $amount } ساعة
    [two] منذ { $amount } ساعتين
    [few] منذ { $amount } ساعات
    [many] منذ { $amount } ساعة
   *[other] منذ { $amount } ساعة
    }
format-ago-day =
    { $count ->
    [zero] منذ { $amount } يومًا
    [one] منذ { $amount } يوم
    [two] منذ { $amount } يومين
    [few] منذ { $amount } أيام
    [many] منذ { $amount } يومًا
   *[other] منذ { $amount } يومًا
    }
format-ago-span =
    { $count ->
    [zero] منذ { $amount }
    [one] منذ { $amount }
    [two] منذ { $amount }
    [few] منذ { $amount }
    [many] منذ { $amount }
   *[other] منذ { $amount }
    }
format-just-now = الآن

format-in-second =
    { $count ->
    [zero] خلال { $amount } ثانية
    [one] خلال { $amount } ثانية
    [two] خلال { $amount } ثانيتين
    [few] خلال { $amount } ثوانٍ
    [many] خلال { $amount } ثانية
   *[other] خلال { $amount } ثانية
    }
format-in-minute =
    { $count ->
    [zero] خلال { $amount } دقيقة
    [one] خلال { $amount } دقيقة
    [two] خلال { $amount } دقيقتين
    [few] خلال { $amount } دقائق
    [many] خلال { $amount } دقيقة
   *[other] خلال { $amount } دقيقة
    }
format-in-hour =
    { $count ->
    [zero] خلال { $amount } ساعة
    [one] خلال { $amount } ساعة
    [two] خلال { $amount } ساعتين
    [few] خلال { $amount } ساعات
    [many] خلال { $amount } ساعة
   *[other] خلال { $amount } ساعة
    }
format-in-day =
    { $count ->
    [zero] خلال { $amount } يومًا
    [one] خلال { $amount } يوم
    [two] خلال { $amount } يومين
    [few] خلال { $amount } أيام
    [many] خلال { $amount } يومًا
   *[other] خلال { $amount } يومًا
    }
format-due = مستحق

format-unit-day =
    { $count ->
    [zero] { $amount } يومًا
    [one] { $amount } يوم
    [two] { $amount } يومان
    [few] { $amount } أيام
    [many] { $amount } يومًا
   *[other] { $amount } يومًا
    }
format-unit-hour =
    { $count ->
    [zero] { $amount } ساعة
    [one] { $amount } ساعة
    [two] { $amount } ساعتان
    [few] { $amount } ساعات
    [many] { $amount } ساعة
   *[other] { $amount } ساعة
    }
format-unit-minute =
    { $count ->
    [zero] { $amount } دقيقة
    [one] { $amount } دقيقة
    [two] { $amount } دقيقتان
    [few] { $amount } دقائق
    [many] { $amount } دقيقة
   *[other] { $amount } دقيقة
    }
format-unit-second =
    { $count ->
    [zero] { $amount } ثانية
    [one] { $amount } ثانية
    [two] { $amount } ثانيتان
    [few] { $amount } ثوانٍ
    [many] { $amount } ثانية
   *[other] { $amount } ثانية
    }
format-unit-millisecond =
    { $count ->
    [zero] { $amount } ms
    [one] { $amount } ms
    [two] { $amount } ms
    [few] { $amount } ms
    [many] { $amount } ms
   *[other] { $amount } ms
    }
format-unit-microsecond =
    { $count ->
    [zero] { $amount } µs
    [one] { $amount } µs
    [two] { $amount } µs
    [few] { $amount } µs
    [many] { $amount } µs
   *[other] { $amount } µs
    }
format-unit-nanosecond =
    { $count ->
    [zero] { $amount } ns
    [one] { $amount } ns
    [two] { $amount } ns
    [few] { $amount } ns
    [many] { $amount } ns
   *[other] { $amount } ns
    }
format-bytes-b =
    { $count ->
    [zero] { $amount } B
    [one] { $amount } B
    [two] { $amount } B
    [few] { $amount } B
    [many] { $amount } B
   *[other] { $amount } B
    }
format-bytes-kb =
    { $count ->
    [zero] { $amount } KB
    [one] { $amount } KB
    [two] { $amount } KB
    [few] { $amount } KB
    [many] { $amount } KB
   *[other] { $amount } KB
    }
format-bytes-mb =
    { $count ->
    [zero] { $amount } MB
    [one] { $amount } MB
    [two] { $amount } MB
    [few] { $amount } MB
    [many] { $amount } MB
   *[other] { $amount } MB
    }
format-bytes-gb =
    { $count ->
    [zero] { $amount } GB
    [one] { $amount } GB
    [two] { $amount } GB
    [few] { $amount } GB
    [many] { $amount } GB
   *[other] { $amount } GB
    }
format-memory-mb =
    { $count ->
    [zero] { $amount } MB
    [one] { $amount } MB
    [two] { $amount } MB
    [few] { $amount } MB
    [many] { $amount } MB
   *[other] { $amount } MB
    }
format-memory-gb =
    { $count ->
    [zero] { $amount } GB
    [one] { $amount } GB
    [two] { $amount } GB
    [few] { $amount } GB
    [many] { $amount } GB
   *[other] { $amount } GB
    }

# A call rate per minute; $amount is the formatted number.
format-calls-per-minute = { $amount }/د
format-under-minute = { "<1 دقيقة" }
