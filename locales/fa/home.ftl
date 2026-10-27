## Portfolio overview

home-portfolio-title = ارزش پورتفوی
home-portfolio-change-label = امروز
home-wallet-unread = موجودی در دسترس نیست
home-trend-range = از ساعت ⁨00:00 UTC⁩
home-portfolio-change-incomparable =
    .title = قیمت برخی توکن‌ها در ابتدای روز در دسترس نبود یا اکنون در دسترس نیست، به همین دلیل تغییر امروز نمایش داده نمی‌شود.
home-stat-available = { -sol } در دسترس
home-stat-holdings = توکن‌های در اختیار
home-stat-open-pnl = سود و زیان باز
home-stat-realized-today = تحقق‌یافته امروز

home-holdings-token-count =
    { $count ->
        [one] { $count } توکن
       *[other] { $count } توکن
    }
home-holdings-with-unpriced = { $tokens } · بدون قیمت: { $count }
home-holdings-unpriced-note =
    { $count ->
        [one] قیمت { $count } توکن در اختیار در دسترس نیست و در مجموع صفر حساب می‌شود
       *[other] قیمت { $count } توکن در اختیار در دسترس نیست و در مجموع صفر حساب می‌شود
    }

## Wallet address and QR code

home-wallet-copy =
    .title = کپی آدرس کیف پول
    .aria-label = کپی آدرس کیف پول
home-wallet-qr-open =
    .title = نمایش کد QR کیف پول
    .aria-label = نمایش کد QR کیف پول
home-wallet-qr-popover =
    .aria-label = کد QR کیف پول
home-wallet-qr-receive = دریافت
home-wallet-qr-assets = { -sol } و توکن‌های SPL
home-wallet-qr-close =
    .title = بستن
    .aria-label = بستن کد QR کیف پول
home-wallet-qr-preparing = در حال آماده‌سازی کد QR
home-wallet-qr-unavailable = کد QR در دسترس نیست
home-wallet-qr-image =
    .alt = کد QR آدرس کیف پول اصلی

## Performance calendar

home-calendar-title = تقویم عملکرد
home-calendar-previous =
    .title = ماه قبل
    .aria-label = ماه قبل
home-calendar-next =
    .title = ماه بعد
    .aria-label = ماه بعد
home-calendar-month-pnl = سود و زیان ماه
home-calendar-trades = معاملات
home-calendar-win-days = روزهای سود
home-calendar-loss-days = روزهای زیان
home-calendar-best-day = بهترین روز
home-calendar-week = هفته
home-calendar-day-label = { $date }، { $pnl }
home-calendar-pop-net-pnl = سود و زیان خالص
home-calendar-pop-win-rate = نرخ برد
home-calendar-pop-win-rate-value = { $rate } · { $wins }W / { $losses }L
home-calendar-pop-gross-profit = سود ناخالص
home-calendar-pop-gross-loss = زیان ناخالص
home-calendar-pop-end-balance = موجودی پایانی

## Auto Trader and Copy Trading status

home-status =
    .aria-label = وضعیت خودکارسازی
home-status-trader-title = ترید خودکار
home-status-copy-title = کپی‌تریدینگ
home-status-running = در حال اجرا
home-status-stopped = متوقف
home-status-setup-required = نیازمند راه‌اندازی
home-status-open-positions = پوزیشن‌های باز
home-status-capacity = { $used } از { $limit }
home-status-entries-today = ورودهای امروز
home-status-exits-today = خروج‌های امروز
home-status-followed-wallets = کیف پول‌های دنبال‌شده
home-status-active-tasks = وظایف فعال
home-status-last-copy = آخرین معامله کپی‌شده
home-status-trader-unavailable = برای استفاده از ترید خودکار، راه‌اندازی کیف پول و RPC را کامل کنید
home-status-copy-unavailable = برای استفاده از کپی‌تریدینگ، راه‌اندازی کیف پول و RPC را کامل کنید
home-status-setup = تکمیل راه‌اندازی
home-status-trader-open = باز کردن ترید خودکار
home-status-copy-open = باز کردن کپی‌تریدینگ

## Position exposure and market pipeline

home-operations =
    .aria-label = وضعیت پورتفوی و بازار
home-exposure-title = میزان پوزیشن‌ها
home-exposure-open = باز
home-exposure-invested = سرمایه‌گذاری‌شده
home-exposure-value = ارزش فعلی
home-exposure-position = پوزیشن
home-exposure-held = مدت نگهداری
home-exposure-empty = پوزیشن بازی وجود ندارد
home-exposure-avg-size = میانگین حجم
home-exposure-avg-hold = میانگین مدت نگهداری
home-exposure-best = بهترین
home-exposure-worst = بدترین
home-pipeline-title = خط لوله بازار
home-pipeline-tracked = ردیابی‌شده
home-pipeline-monitored = در حال پایش
home-pipeline-passed = تأییدشده در فیلترها
home-pipeline-rejected = ردشده
home-pipeline-blacklisted = در فهرست سیاه
home-pipeline-live-price = قیمت زنده
home-pipeline-ohlcv = OHLCV
