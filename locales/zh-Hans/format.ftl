format-not-available = 暂无

format-yes = 是
format-no = 否
format-unknown = 未知

format-native-amount = { $amount } { -sol }

format-usd-amount = ${ $amount }

format-percent-amount = { $amount }%

format-approx = ≈ { $value }

format-ago-second =
    { $count ->
       *[other] { $amount } 秒前
    }
format-ago-minute =
    { $count ->
       *[other] { $amount } 分钟前
    }
format-ago-hour =
    { $count ->
       *[other] { $amount } 小时前
    }
format-ago-day =
    { $count ->
       *[other] { $amount } 天前
    }

format-ago-span =
    { $count ->
       *[other] { $amount }前
    }
format-just-now = 刚刚

format-in-second =
    { $count ->
       *[other] { $amount } 秒后
    }
format-in-minute =
    { $count ->
       *[other] { $amount } 分钟后
    }
format-in-hour =
    { $count ->
       *[other] { $amount } 小时后
    }
format-in-day =
    { $count ->
       *[other] { $amount } 天后
    }
format-due = 已到期

format-unit-day =
    { $count ->
       *[other] { $amount } 天
    }
format-unit-hour =
    { $count ->
       *[other] { $amount } 小时
    }
format-unit-minute =
    { $count ->
       *[other] { $amount } 分钟
    }
format-unit-second =
    { $count ->
       *[other] { $amount } 秒
    }

format-unit-millisecond =
    { $count ->
       *[other] { $amount } 毫秒
    }
format-unit-microsecond =
    { $count ->
       *[other] { $amount } 微秒
    }
format-unit-nanosecond =
    { $count ->
       *[other] { $amount } 纳秒
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

format-under-minute = { "<1" } 分钟
