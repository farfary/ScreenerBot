paper-readiness-history = تاریخچه آزمایشی
paper-readiness-history-met =
    { $count ->
        [one] { $count } دور آزمایشی بسته، { $needed } لازم است
       *[other] { $count } دور آزمایشی بسته، { $needed } لازم است
    }
paper-readiness-history-short = { $count } از { $needed } دور آزمایشی بسته
paper-readiness-profit = سودآور در حالت آزمایشی
paper-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } تحقق‌یافته در { $count } دور، { $wins } برد
       *[other] { $realized } { -sol } تحقق‌یافته در { $count } دور، { $wins } برد
    }
paper-readiness-exits = خروج‌ها انجام می‌شوند
paper-readiness-exits-ok = خروج‌های آزمایشی اخیر انجام شدند
paper-readiness-exits-failing = خروج‌های آزمایشی پیاپی بدون انجام: { $count }
paper-live-block-setup-incomplete = ابتدا راه‌اندازی کیف پول و RPC را کامل کنید
paper-live-block-force-stop = توقف اضطراری فعال است
