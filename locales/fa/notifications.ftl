notifications-action-swap-buy = خرید
notifications-action-swap-sell = فروش
notifications-action-position-open = باز کردن
notifications-action-position-close = بستن
notifications-action-position-dca = DCA
notifications-action-position-partial-exit = خروج جزئی
notifications-action-manual-order = دستی
notifications-action-unknown = اقدام

actions-step-evaluate = در حال ارزیابی
actions-step-validate = در حال اعتبارسنجی
actions-step-quote = دریافت قیمت پیشنهادی
actions-step-swap = اجرای سواپ
actions-step-verify = در حال تأیید
actions-step-unknown = در حال پردازش
actions-step-evaluate-short = ارزیابی
actions-step-validate-short = بررسی
actions-step-quote-short = قیمت
actions-step-swap-short = سواپ
actions-step-verify-short = تأیید
actions-step-unknown-short = در حال کار

actions-failure-recorded = { $message }
actions-failure-unknown = خطای ناشناخته
actions-failure-interrupted = با راه‌اندازی مجدد برنامه متوقف شد
actions-failure-validation = اعتبارسنجی ناموفق بود
actions-failure-quote = دریافت قیمت پیشنهادی ناموفق بود
actions-failure-swap = سواپ ناموفق بود
actions-failure-trade = معامله ناموفق بود
actions-failure-entry = ورود ناموفق بود
actions-failure-exit = خروج ناموفق بود
actions-failure-dca = DCA ناموفق بود
actions-failure-verification-expired = مهلت تأیید به پایان رسید: تراکنش هرگز ثبت نشد
actions-failure-verification-gave-up = تأیید متوقف شد
actions-failure-transaction-failed = تراکنش روی زنجیره ناموفق بود
actions-failure-sell-transaction-failed = تراکنش فروش روی زنجیره ناموفق بود
actions-failure-dca-verification-failed = تأیید DCA ناموفق بود

notifications-empty-all = اقدامی وجود ندارد
notifications-empty-active = اقدام فعالی وجود ندارد
notifications-empty-completed = اقدام تکمیل‌شده‌ای وجود ندارد
notifications-empty-failed = اقدام ناموفقی وجود ندارد
notifications-source-auto = خودکار
notifications-source-manual = دستی
notifications-cancelled = لغو شد
notifications-load-failed = بارگذاری ناموفق بود
notifications-clear-title = پاک‌کردن اعلان‌ها
notifications-clear-message = همه اعلان‌های این فهرست بسته شوند؟ آن‌ها در تاریخچه تکمیل‌شده/ناموفق باقی می‌مانند.
notifications-clear-failed = پاک‌کردن اعلان‌ها ناموفق بود
notifications-stream-lag-title = جریان اقدامات عقب افتاد
notifications-stream-lag-missed =
    { $count ->
        [one] { $count } به‌روزرسانی از دست رفت
       *[other] { $count } به‌روزرسانی از دست رفت
    } — در حال تازه‌سازی
notifications-stream-lag-refreshing = در حال تازه‌سازی
notifications-sync-failed = تازه‌سازی اقدامات ممکن نشد

# Action rows (ui/action_row.js): day headings, the copy-trading source, the detail.
notifications-day-today = امروز
notifications-day-yesterday = دیروز
notifications-source-copy = کپی‌تریدینگ
notifications-steps = مراحل
notifications-detail-transaction = تراکنش
notifications-detail-fill-price = قیمت اجرا
    .title = مقدار { -sol } این سواپ تقسیم بر مقدار توکن آن
notifications-detail-fee = کارمزد شبکه
notifications-detail-pnl = سود و زیان پوزیشن
notifications-detail-route = مسیر
notifications-detail-duration = مدت
notifications-detail-reason = دلیل
notifications-view-token = مشاهده توکن
notifications-view-position = مشاهده پوزیشن
notifications-view-transaction = مشاهده تراکنش
notifications-unrecorded = در پوزیشن ثبت نشده
notifications-unrecorded-detail = این سواپ در پوزیشن ثبت نشده و ممکن است انجام نشده باشد. تراکنش آن را بررسی کنید.
