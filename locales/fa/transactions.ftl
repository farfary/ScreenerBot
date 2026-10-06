# Transaction type labels. Ids come from TransactionType::kind() in
# src/transactions/types.rs; the dashboard maps them in ui/transaction_type.js.

transactions-type-buy = خرید
transactions-type-sell = فروش
transactions-type-swap = سواپ
transactions-type-sol-transfer = انتقال SOL
transactions-type-token-transfer = انتقال توکن
transactions-type-transfer = انتقال
transactions-type-dust = داست
transactions-type-spam = اسپم
transactions-type-ata-create = حساب باز شد
transactions-type-ata-close = رنت بازپس گرفته شد
transactions-type-ata = حساب توکن
transactions-type-liquidity-add = افزودن نقدینگی
transactions-type-liquidity-remove = برداشت نقدینگی
transactions-type-nft = NFT
transactions-type-program = فراخوانی برنامه
transactions-type-compute = محاسبه
transactions-type-failed = ناموفق
transactions-type-unknown = طبقه‌بندی‌نشده

# A type with the payload that identifies it, as shown in the position activity feed.
transactions-type-with-detail = { $label } ({ $detail })
transactions-type-token-transfer-detail = { $label } { $mint } ({ $amount })
transactions-type-spam-detail = ایردراپ اسپم ({ $mint })
transactions-type-described = { $description }

# Type filter entries whose wording differs from the type label.
transactions-filter-all = همه انواع
transactions-filter-transfer = انتقال‌ها
transactions-filter-ata = رنت و حساب‌ها
transactions-filter-liquidity = نقدینگی
transactions-filter-program = فراخوانی برنامه‌ها

# Wallet-relative direction. Ids come from TransactionDirection in src/transactions/types.rs
# (ui/transaction_direction.js).
transactions-direction-incoming = ورودی
transactions-direction-outgoing = خروجی
transactions-direction-internal = داخلی
transactions-direction-unknown = طبقه‌بندی‌نشده

# Chain status. Ids come from TransactionStatus in src/transactions/types.rs
# (ui/transaction_status.js); Success and Unknown label a row without a status.
transactions-status-pending = در انتظار
transactions-status-confirmed = تأییدشده
transactions-status-finalized = نهایی‌شده
transactions-status-failed = ناموفق
transactions-status-success = موفق
transactions-status-unknown = نامشخص

# Ids come from AtaOperationType in src/transactions/types.rs.
transactions-ata-operation-creation = ایجاد
transactions-ata-operation-closure = بستن

## Transactions page (pages/transactions.js)

transactions-toolbar-title = تاریخچه تراکنش‌ها
transactions-search =
    .placeholder = جستجوی امضاها…
    .aria-label = جستجوی امضای تراکنش‌ها
transactions-load-failed = به‌روزرسانی تراکنش‌ها ممکن نشد
transactions-summary-total = مجموع
transactions-summary-estimate = برآورد
transactions-summary-success = موفق
transactions-summary-failed = ناموفق
transactions-filter-wallet = کیف پول
transactions-filter-type = نوع
transactions-filter-direction = جهت
transactions-filter-status = وضعیت
transactions-filter-all-directions = همه جهت‌ها
transactions-filter-all-statuses = همه وضعیت‌ها
transactions-wallet-main = کیف پول اصلی
transactions-col-time = زمان
transactions-col-signature = امضا
transactions-col-type = نوع
transactions-col-direction = جهت
transactions-col-status = وضعیت
transactions-col-native-delta = Δ { -sol }
transactions-col-fees = کارمزدها ({ -sol })
transactions-col-token = توکن
transactions-col-router = روتر
transactions-col-instructions = اینستراکشن

## Transaction details dialog (ui/transaction_details_dialog.js)

transactions-dialog-copy-signature =
    .title = کپی امضا
transactions-dialog-close =
    .title = بستن (ESC)
transactions-dialog-tabs-label = بخش‌های جزئیات تراکنش
transactions-dialog-meta-slot = اسلات:
transactions-dialog-meta-fee = کارمزد:
transactions-dialog-loading = در حال بارگذاری...
transactions-dialog-loading-details = در حال بارگذاری جزئیات تراکنش...
transactions-dialog-load-failed = بارگذاری جزئیات تراکنش ناموفق بود
# $reason is the failure text reported by the server.
transactions-dialog-load-failed-reason = بارگذاری جزئیات تراکنش ناموفق بود: { $reason }
transactions-dialog-not-found = تراکنش پیدا نشد
transactions-dialog-tab-overview = نمای کلی
transactions-dialog-tab-balances = موجودی‌ها
transactions-dialog-tab-instructions = اینستراکشن‌ها
transactions-dialog-tab-logs = لاگ‌ها
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = خام
transactions-dialog-unknown = نامشخص
transactions-dialog-unknown-asset = دارایی نامشخص
transactions-dialog-unavailable = در دسترس نیست

## Transaction details dialog: overview

transactions-dialog-failed-title = تراکنش ناموفق بود
transactions-dialog-no-program-error = خطای برنامه‌ای ارائه نشده است.
transactions-dialog-story-title = چه اتفاقی افتاد
# $router is the routing program name.
transactions-dialog-router-via = از طریق { $router }
transactions-dialog-flow-paid = پرداخت‌شده
transactions-dialog-flow-received = دریافت‌شده
transactions-dialog-flow-from = از
transactions-dialog-flow-to = به
transactions-dialog-flow-amount = مقدار
transactions-dialog-net-wallet-change = تغییر خالص کیف پول:
transactions-dialog-processed = پردازش‌شده روی Solana
transactions-dialog-execution-title = اجرا
transactions-dialog-metric-execution-price = قیمت اجرا
transactions-dialog-metric-effective-received = دریافت مؤثر
transactions-dialog-metric-effective-spent = پرداخت مؤثر
transactions-dialog-metric-network-fee = کارمزد شبکه
transactions-dialog-metric-estimated-pnl = سود و زیان تخمینی
transactions-dialog-metric-net-native-change = تغییر خالص { -sol }
transactions-dialog-route-title = مسیر و دارایی‌ها
transactions-dialog-route-router = روتر
transactions-dialog-route-input-asset = دارایی ورودی
transactions-dialog-route-output-asset = دارایی خروجی
transactions-dialog-route-pool = استخر
transactions-dialog-route-program = برنامه
transactions-dialog-tech-title = جزئیات فنی
transactions-dialog-tech-summary = امضا، اسلات و منابع
transactions-dialog-tech-signature = امضا
transactions-dialog-tech-timestamp = مهر زمانی
transactions-dialog-tech-slot = اسلات
transactions-dialog-tech-exact-fee = کارمزد دقیق
transactions-dialog-tech-accounts = حساب‌ها
transactions-dialog-tech-instructions = اینستراکشن‌ها
transactions-dialog-tech-compute-units = واحد محاسباتی
transactions-dialog-tech-token-decimals = اعشار توکن

## Transaction details dialog: balances, instructions, logs, ATA and raw tabs

transactions-dialog-balances-native-title = تغییرات موجودی { -sol }
transactions-dialog-balances-native-empty = تغییری در موجودی { -sol } وجود ندارد
transactions-dialog-balances-token-title = تغییرات موجودی توکن
transactions-dialog-balances-token-empty = تغییری در موجودی توکن وجود ندارد
transactions-dialog-balances-net-native = تغییر خالص { -sol }
transactions-dialog-balances-fee = کارمزد تراکنش
transactions-dialog-col-account = حساب
transactions-dialog-col-token = توکن
transactions-dialog-col-mint = آدرس مینت
transactions-dialog-col-pre-balance = موجودی قبل
transactions-dialog-col-post-balance = موجودی بعد
transactions-dialog-col-change = تغییر
transactions-dialog-col-type = نوع
transactions-dialog-col-rent = رنت ({ -sol })
transactions-dialog-instructions-empty = اینستراکشنی پیدا نشد
transactions-dialog-instructions-count =
    { $count ->
        [one] { $count } اینستراکشن
       *[other] { $count } اینستراکشن
    }
transactions-dialog-instruction-program-id = شناسه برنامه
transactions-dialog-instruction-accounts = حساب‌ها ({ $count })
transactions-dialog-instruction-data = داده
transactions-dialog-logs-empty = لاگی موجود نیست
transactions-dialog-logs-filter = فیلتر لاگ‌ها...
transactions-dialog-logs-no-match = لاگ مطابقی پیدا نشد
transactions-dialog-logs-count =
    { $count ->
        [one] { $count } لاگ
       *[other] { $count } لاگ
    }
transactions-dialog-ata-empty = این تراکنش عملیات ATA ندارد
transactions-dialog-ata-summary-title = خلاصه تحلیل ATA
transactions-dialog-ata-creations = ایجادها
transactions-dialog-ata-closures = بستن‌ها
transactions-dialog-ata-rent-spent = رنت مصرف‌شده
transactions-dialog-ata-rent-recovered = رنت بازیابی‌شده
transactions-dialog-ata-net-rent = تأثیر خالص رنت
transactions-dialog-ata-operations-title = عملیات ATA ({ $count })
transactions-dialog-raw-copy = کپی JSON
transactions-dialog-raw-empty = داده خامی موجود نیست
