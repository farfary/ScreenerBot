# Position details labels.

# State reasons. Ids come from POSITION_CREATED_REASON in src/positions/database/types.rs.
positions-state-reason-position-created = پوزیشن ایجاد شد


# Source: scripts/pages/positions.js

## Views, origin and toolbar

# Ids are the position status values (POSITION_STATUS_LABELS, ui/position_status.js).
positions-status-open = باز
positions-status-closed = بسته
positions-status-archived = بایگانی‌شده
positions-origin-copy = کپی
positions-origin-manual = دستی
positions-origin-wallet = کیف پول
positions-origin-copy-link =
    .title = باز کردن وظیفه کپی که این پوزیشن را باز کرده است
positions-holding-frozen = مسدودشده
    .title = اختیار مینت این حساب توکن را فریز کرده است - موجودی قابل انتقال یا فروش نیست
positions-toolbar-total = مجموع
positions-toolbar-delete-all = حذف همه
positions-search-placeholder = جستجو بر اساس نماد یا مینت...
positions-filter-origin = منشأ
positions-filter-origin-all = همه منشأها
positions-filter-origin-auto = معامله‌گر خودکار
positions-filter-origin-copy = کپی‌تریدینگ
positions-delete-all-tooltip = حذف دائمی همه پوزیشن‌های بایگانی‌شده

## Columns

positions-column-token = توکن
positions-column-archived-at = بایگانی
positions-column-entry-time = زمان ورود
positions-column-exit-time = زمان خروج
positions-column-avg-entry = میانگین ورود ({ -sol })
positions-column-avg-exit = میانگین خروج ({ -sol })
positions-column-current-price = فعلی ({ -sol })
positions-column-total-invested = کل سرمایه‌گذاری
positions-column-proceeds = عایدی
positions-column-pnl = سود و زیان
positions-column-pnl-percent = سود و زیان %
positions-column-size = حجم پوزیشن
positions-column-dca = DCA
positions-column-exits = خروج‌ها
positions-column-unrealized-pnl = سود و زیان تحقق‌نیافته
positions-column-unrealized-percent = تحقق‌نیافته %

## Cells

# Shown instead of a figure the wallet history cannot support.
positions-unknown-basis = قیمت تمام‌شده در تاریخچه این کیف پول موجود نیست (ایردراپ، معامله با قیمت دلاری یا سواپ بدون ساق { -sol })
positions-unknown-history = این دور با موجودی روی زنجیره مطابقت ندارد
positions-dca-count =
    { $count ->
        [one] { $count } DCA
       *[other] { $count } DCA
    }
positions-exit-count =
    { $count ->
        [one] { $count } خروج
       *[other] { $count } خروج
    }

## Row actions

positions-action-add =
    .title = افزودن به پوزیشن (DCA)
    .aria-label = افزودن به پوزیشن
positions-action-sell =
    .title = فروش (کامل یا جزئی بر اساس درصد)
    .aria-label = فروش پوزیشن
positions-action-sell-frozen = توسط اختیار مینت فریز شده است - این دارایی قابل فروش نیست
positions-action-remove =
    .title = حذف (بایگانی یا حذف دائمی)
    .aria-label = حذف پوزیشن
positions-action-restore =
    .title = بازگرداندن به باز/بسته
    .aria-label = بازگرداندن پوزیشن
positions-action-delete =
    .title = حذف دائمی
    .aria-label = حذف دائمی
positions-action-in-progress = در حال انجام…

## Live state of a row

positions-caption-buying = در حال خرید
# $step is the label of the current action step.
positions-caption-buying-step = در حال خرید · { $step }
positions-caption-selling = در حال فروش
positions-caption-selling-step = در حال فروش · { $step }
positions-caption-closing = در حال بستن
positions-caption-failed = ناموفق
# $error is the failure text of the action.
positions-caption-failed-detail = ناموفق · { $error }
positions-step-adding = در حال افزودن
positions-pending-buying = در حال خرید…
positions-pending-buy-failed = خرید ناموفق بود

## Messages and confirmations

positions-load-failed = به‌روزرسانی پوزیشن‌ها ممکن نشد
positions-toast-not-found = اطلاعات پوزیشن پیدا نشد
positions-toast-deleted = پوزیشن حذف شد
positions-toast-archived = پوزیشن بایگانی شد
positions-toast-restored = پوزیشن بازگردانده شد
positions-action-failed = عملیات ناموفق بود
positions-delete-title = حذف دائمی پوزیشن
# $symbol is the token symbol.
positions-delete-message = { $symbol } برای همیشه حذف شود؟ پوزیشن و تاریخچه آن از پایگاه داده پاک می‌شود و قابل بازگشت نیست. تراکنش‌ها و داده‌های توکن تغییری نمی‌کنند.
positions-delete-confirm = حذف دائمی
positions-delete-all-title = حذف همه پوزیشن‌های بایگانی‌شده
positions-delete-all-message =
    { $count ->
        [one] همه { $count } پوزیشن بایگانی‌شده برای همیشه حذف شوند؟ این کار قابل بازگشت نیست. تراکنش‌ها و داده‌های توکن تغییری نمی‌کنند.
       *[other] همه { $count } پوزیشن بایگانی‌شده برای همیشه حذف شوند؟ این کار قابل بازگشت نیست. تراکنش‌ها و داده‌های توکن تغییری نمی‌کنند.
    }
positions-delete-all-message-empty = همه پوزیشن‌های بایگانی‌شده برای همیشه حذف شوند؟ این کار قابل بازگشت نیست.
positions-delete-all-confirm = حذف همه
positions-delete-all-done =
    { $count ->
        [one] { $count } پوزیشن بایگانی‌شده حذف شد
       *[other] { $count } پوزیشن بایگانی‌شده حذف شد
    }
positions-delete-all-failed = حذف پوزیشن‌های بایگانی‌شده ناموفق بود

# Source: scripts/ui/position_remove_dialog.js

## Remove position dialog

positions-remove-title = حذف پوزیشن
# Inline markup: emphasis on the opening sentence and on "not".
positions-remove-open-warning = <strong>این پوزیشن هنوز باز است.</strong> ربات این توکن را نگه داشته است. با حذف آن، جایگاه معامله آزاد و پیگیری متوقف می‌شود، اما توکن <strong>فروخته نمی‌شود</strong>. اگر می‌خواهید { -sol } خود را پس بگیرید، ابتدا بفروشید.
positions-remove-modes =
    .aria-label = حالت حذف
positions-remove-archive = بایگانی
positions-remove-recommended = پیشنهادی
positions-remove-archive-description = انتقال به برگه بایگانی‌شده. هر زمان قابل بازگشت است؛ چیزی فروخته نمی‌شود و همه معاملات ثبت می‌مانند.
positions-remove-delete = حذف دائمی
positions-remove-delete-description = پاک کردن این پوزیشن و کل تاریخچه آن از پایگاه داده.
# Inline markup: emphasis on the irreversibility sentence.
positions-remove-danger = این کار پوزیشن و تاریخچه آن را برای همیشه حذف می‌کند. <strong>این کار قابل بازگشت نیست.</strong> تراکنش‌ها و داده‌های توکن تغییری نمی‌کنند.
positions-remove-confirm-archive = بایگانی پوزیشن

# Source: scripts/ui/position_details_dialog.js, scripts/ui/position_details/panes.js

## Position details frame

# Message shown after a management change. $mode is the label of the new mode.
positions-management-changed = مدیریت پوزیشن روی { $mode } تنظیم شد
positions-details-load-failed = بارگذاری جزئیات پوزیشن ناموفق بود
positions-details-mint-label = آدرس مینت
positions-details-management-failed = به‌روزرسانی مدیریت پوزیشن ناموفق بود
positions-details-favorite-add =
    .title = افزودن به علاقه‌مندی‌ها
    .aria-label = افزودن به علاقه‌مندی‌ها
positions-details-favorite-remove =
    .title = حذف از علاقه‌مندی‌ها
    .aria-label = حذف از علاقه‌مندی‌ها
positions-details-view-solscan =
    .title = مشاهده در { -solscan }
    .aria-label = مشاهده توکن در { -solscan }
positions-details-close =
    .title = بستن (Esc)
    .aria-label = بستن
positions-details-chart-section =
    .aria-label = نمودار قیمت
positions-details-loading-chart = در حال بارگذاری نمودار...
positions-details-activity-section =
    .aria-label = فعالیت
positions-details-activity-title = فعالیت
positions-details-split-handle =
    .aria-label = تغییر اندازه نمودار و فعالیت
positions-details-activity-pane =
    .aria-label = بخش فعالیت
positions-details-activity-expand =
    .title = بزرگ کردن فعالیت
    .aria-label = بزرگ کردن فعالیت
positions-details-summary-section =
    .aria-label = خلاصه پوزیشن
positions-details-loading = در حال بارگذاری پوزیشن...

## Management modes. Ids are the PositionManagement serde ids (src/positions/types.rs).

positions-management-auto-trader = معامله‌گر خودکار
positions-management-user-only = فقط کاربر
positions-management-copy-task = وظیفه کپی
positions-management-hybrid = ترکیبی
positions-pane-show-chart = نمایش نمودار
positions-pane-show-activity = نمایش فعالیت
positions-pane-restore-activity = بازگرداندن فعالیت
positions-pane-expand-chart =
    .title = بزرگ کردن نمودار
    .aria-label = بزرگ کردن نمودار

# Source: scripts/ui/position_details/header.js

## Position details header

positions-risk-low = ریسک کم
positions-risk-medium = ریسک متوسط
positions-risk-high = ریسک بالا
positions-risk-unknown = ریسک نامشخص
positions-busy-buying = خرید در حال انجام…
positions-busy-selling = فروش در حال انجام…
positions-busy-closing = بستن در حال انجام…
positions-header-avg-entry = میانگین ورود
# $count is the number of buys: the entry plus each add.
positions-header-buy-count =
    { $count ->
        [one] { $count } خرید
       *[other] { $count } خرید
    }
positions-header-exit-price = قیمت خروج
# $ago is the elapsed time since the close, for example "3h ago".
positions-header-closed-ago = بسته‌شده، { $ago }
positions-header-realized-pnl = سود و زیان تحقق‌یافته
positions-header-usd-note = دلار به قیمت امروز { -sol }
positions-header-returned = بازگشتی
# $amount is the formatted SOL amount invested.
positions-header-of-invested = از { $amount } سرمایه‌گذاری
positions-header-price = قیمت
positions-header-last-price = آخرین قیمت
positions-header-pool-ago = استخر · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = سود و زیان تحقق‌نیافته
positions-header-pnl-last-price = سود و زیان به آخرین قیمت
positions-header-value = ارزش
positions-header-last-value = آخرین ارزش
positions-header-invested = { $amount } سرمایه‌گذاری
positions-header-origin-hint = نحوه باز شدن این پوزیشن
positions-header-risk-hint = امتیاز { -rugcheck } — هرچه کمتر، امن‌تر
positions-header-frozen = مسدودشده
    .title = اختیار مینت این دارایی را فریز کرده است
positions-header-managed-by = مدیریت‌شده توسط
positions-header-management-select =
    .aria-label = مدیریت پوزیشن

## Entry origin shown in the header badge

positions-origin-unknown = نامشخص
# $task is the copy task id. The source wallet follows in its own element.
positions-origin-copied-task = کپی‌شده · وظیفه { $task }
positions-origin-manual-entry = ورود دستی
positions-origin-wallet-entry = ورود از کیف پول
# $strategy is the strategy id.
positions-origin-auto-strategy = خودکار · { $strategy }
positions-origin-auto-entry = ورود خودکار

## Swaps that are submitted and not yet booked

positions-pending-adding = در حال افزودن
positions-pending-adding-amount = در حال افزودن { $amount }
positions-pending-selling = در حال فروش
# $percent is the formatted share of the position being sold.
positions-pending-selling-percent = در حال فروش { $percent }
# $label is the pending swap wording.
positions-pending-confirming = { $label } · در انتظار تأیید
    .title = ارسال شده و در انتظار تأیید روی زنجیره است. پس از تأیید، ارقام به‌روز می‌شوند.

## Trade controls

positions-trade-add = افزودن
    .title = افزودن به پوزیشن
positions-trade-sell = فروش
    .title = فروش بخشی از پوزیشن
positions-trade-close = بستن پوزیشن
    .title = فروش کامل و بستن
positions-trade-token = جزئیات توکن
    .title = باز کردن جزئیات توکن

## Favorites

positions-favorite-token-fallback = توکن
# $symbol is the token symbol.
positions-favorite-added = { $symbol } به علاقه‌مندی‌ها اضافه شد
positions-favorite-removed = { $symbol } از علاقه‌مندی‌ها حذف شد
positions-favorite-add-failed = افزودن به علاقه‌مندی‌ها ناموفق بود
positions-favorite-remove-failed = حذف از علاقه‌مندی‌ها ناموفق بود
positions-favorite-update-failed = به‌روزرسانی علاقه‌مندی‌ها ناموفق بود

# Source: scripts/ui/position_details/summary.js

## Summary rail

positions-summary-position = پوزیشن
positions-summary-price-path = مسیر قیمت
positions-summary-network-fees = کارمزد شبکه
positions-summary-risk = ریسک
positions-summary-market = بازار
positions-summary-market-now = بازار اکنون
positions-summary-links = پیوندها
positions-fact-tokens-fallback = توکن
positions-fact-bought = خریداری‌شده
positions-fact-holding = دارایی
positions-fact-sold = فروخته‌شده
positions-fact-realized = تحقق‌یافته
positions-fact-opened = باز شده
positions-fact-closed = بسته شده
positions-fact-reason = دلیل
positions-fact-archived = بایگانی
positions-fact-entry = ورود
positions-fact-exit = خروج
positions-fact-total = مجموع
positions-fact-verified = تأییدشده روی زنجیره
positions-fact-confirming = در انتظار تأیید
# $percent is the formatted share, for example "12.5%".
positions-fact-share-of-bought = { $percent } از خرید
positions-fact-share-of-invested = { $percent } از سرمایه‌گذاری
# $count is the number of adds after the entry.
positions-fact-entry-count =
    { $count ->
        [0] 1 ورود
        [one] 1 ورود + { $count } افزودن
       *[other] 1 ورود + { $count } افزودن
    }
# $count is the number of partial exits, $returned the formatted SOL amount.
positions-fact-partial-exits-back =
    { $count ->
        [one] { $count } خروج جزئی · { $returned } بازگشت
       *[other] { $count } خروج جزئی · { $returned } بازگشت
    }
# $age is the elapsed time of the hold.
positions-fact-held = نگهداری‌شده، { $age }
# $percent is the signed change against the entry price.
positions-fact-vs-entry = { $percent } نسبت به ورود
positions-fact-exit-vs-peak = خروج نسبت به اوج
positions-fact-now-vs-peak = اکنون نسبت به اوج
positions-fact-entry-range = بازه ورود
positions-range-low = کف
positions-range-peak = اوج
positions-range-now = اکنون
positions-range-label-exit = قیمت ورود و خروج میان کف و اوج
positions-range-label-now = قیمت ورود و قیمت اکنون میان کف و اوج
positions-fact-mint-authority = اختیار مینت
positions-fact-freeze-authority = اختیار فریز
positions-fact-active = فعال
positions-fact-pool = استخر
# $amount is the formatted liquidity in SOL.
positions-fact-pool-liquidity = نقدینگی { $amount } { -sol }
positions-fact-market-cap = ارزش بازار
# $value is the formatted fully diluted valuation in USD.
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = نقدینگی
positions-fact-volume-24h = حجم 24h
positions-fact-price-change = تغییر قیمت
positions-change-period-1h = 1h
positions-change-period-24h = 24h
positions-fact-holders = هولدرها
positions-link-website = وب‌سایت
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

# Source: scripts/ui/position_details/activity.js, scripts/ui/position_details/activity_event.js

## Activity

positions-activity-load-failed = بارگذاری فعالیت ممکن نشد
positions-activity-loading = در حال بارگذاری فعالیت...
positions-activity-empty = هنوز هیچ رویدادی برای این توکن در این کیف پول ثبت نشده است
positions-activity-filter-empty = فعالیتی با این فیلتر مطابقت ندارد
positions-activity-round-count =
    { $count ->
        [one] { $count } دور
       *[other] { $count } دور
    }
positions-activity-event-count =
    { $count ->
        [one] { $count } رویداد
       *[other] { $count } رویداد
    }
positions-activity-pending-count = در انتظار: { $count }
positions-activity-failed-count = ناموفق: { $count }
positions-filter-all = همه
positions-filter-trades = معاملات
positions-filter-buys = خریدها
positions-filter-sells = فروش‌ها
positions-filter-wallet = کیف پول
positions-filter-issues = مشکلات
positions-activity-filters =
    .aria-label = فیلتر فعالیت
positions-activity-totals =
    .aria-label = همه دورهای این توکن
positions-activity-realized-all = تحقق‌یافته، همه دورها
positions-activity-invested = سرمایه‌گذاری
positions-activity-returned = بازگشتی
# $when is the formatted open time of a round that has not closed.
positions-activity-opened = باز شده، { $when }
# $index is the 1-based number of the round.
positions-activity-round-title = پوزیشن { $index }
positions-activity-this-position = این پوزیشن
positions-activity-dates-unavailable = تاریخ‌ها در دسترس نیستند
positions-activity-wallet-title = تراکنش‌های کیف پول
# $range is the date range, $count the number of events.
positions-activity-outside =
    { $count ->
        [one] خارج از هر پوزیشن · { $range } · { $count } رویداد
       *[other] خارج از هر پوزیشن · { $range } · { $count } رویداد
    }
positions-details-signature-label = امضا

## State history milestones. Ids are the PositionState names (src/positions/database/types.rs).

positions-state-open = پوزیشن باز
positions-state-closing = پوزیشن در حال بسته شدن
positions-state-closed = پوزیشن بسته
positions-state-exit-pending = خروج پوزیشن در انتظار
positions-state-exit-failed = خروج پوزیشن ناموفق
positions-state-phantom = پوزیشن فانتوم
positions-state-reconciling = پوزیشن در حال تطبیق

## Activity events

positions-event-kind-entry = ورود
positions-event-kind-dca = افزودن
positions-event-kind-partial-exit = خروج جزئی
positions-event-kind-exit = خروج
positions-event-kind-buy = خرید کیف پول
positions-event-kind-sell = فروش کیف پول
positions-event-kind-transfer = انتقال
positions-event-kind-ata = حساب توکن
positions-event-kind-other = تراکنش
positions-event-state-pending = در انتظار
positions-event-state-failed = ناموفق
positions-event-state-synthetic = مصنوعی
# $error is the failure text reported by the chain.
positions-chain-status-failed-detail = ناموفق: { $error }
positions-event-tokens-fallback = توکن
# In the descriptions below $amount is the token amount with its symbol, $sol the SOL amount
# and $percent the share of the position sold.
positions-event-entry-submitted = خرید { $amount } ارسال شد
positions-event-entry-for = { $amount } با { $sol } خریداری شد
positions-event-entry = { $amount } خریداری شد
positions-event-dca-submitted = افزودن { $amount } ارسال شد
positions-event-dca-for = { $amount } با { $sol } اضافه شد
positions-event-dca = { $amount } اضافه شد
positions-event-partial-exit-submitted-percent = خروج جزئی { $percent } برای { $amount } ارسال شد
positions-event-partial-exit-submitted = خروج جزئی برای { $amount } ارسال شد
positions-event-sold-percent-for = { $amount } ({ $percent }) با { $sol } فروخته شد
positions-event-sold-percent = { $amount } ({ $percent }) فروخته شد
positions-event-sold-for = { $amount } با { $sol } فروخته شد
positions-event-sold = { $amount } فروخته شد
positions-event-exit-submitted = خروج کامل از پوزیشن ارسال شد
positions-event-exit-for = با فروش { $amount } به مبلغ { $sol } بسته شد
positions-event-exit-closed = پوزیشن بسته شد
positions-event-wallet-bought = کیف پول { $amount } را جای دیگر خرید
positions-event-wallet-sold = کیف پول { $amount } را جای دیگر فروخت
positions-event-received = { $amount } دریافت شد
positions-event-sent = { $amount } ارسال شد
positions-event-transferred = { $amount } منتقل شد
positions-event-ata = فعالیت حساب توکن
positions-event-wallet-transaction = تراکنش کیف پول مربوط به { $amount }
# $price is the formatted price per token in SOL.
positions-event-price-per-token = { $price } { -sol } / توکن
# $amount is the signed SOL change of the wallet.
positions-event-wallet-change = تغییر کیف پول: { $amount }
positions-event-after-title = پوزیشن پس از این رویداد
positions-event-capital-invested = سرمایه‌گذاری‌شده
positions-event-average-entry = میانگین ورود
positions-event-transfers-title = انتقال‌های توکن
positions-event-transfer-amount = مقدار
positions-event-transfer-mint = مینت
positions-event-transfer-from = از
positions-event-transfer-to = به
positions-event-no-signature = بدون امضای روی زنجیره
positions-event-click-to-copy = برای کپی کلیک کنید
positions-event-solscan = { -solscan }
positions-event-token-amount = مقدار توکن
positions-event-trade-price = قیمت معامله
positions-event-native-amount = مقدار { -sol }
positions-event-cost-basis = قیمت تمام‌شده
positions-event-usd-value = ارزش دلاری
positions-event-network-fee = کارمزد شبکه
positions-event-router = روتر
positions-event-slot = اسلات
positions-event-chain-status = وضعیت زنجیره
positions-event-transaction-type = نوع تراکنش
positions-event-direction = جهت
positions-event-wallet-native-change = تغییر { -sol } کیف پول
positions-event-instructions = اینستراکشن‌ها
positions-event-compute-units = واحد محاسباتی
positions-event-accounts = حساب‌ها
positions-event-record-id = شناسه رکورد
positions-event-time-unavailable = زمان در دسترس نیست
positions-event-details = جزئیات
positions-event-hide-details = پنهان کردن جزئیات

# Source: scripts/ui/position_details/chart.js

## Position chart

positions-chart-type-candles = کندل
positions-chart-type-line = خطی
positions-chart-type-area = ناحیه‌ای
positions-chart-type-group =
    .aria-label = نوع نمودار
positions-chart-overlays-group =
    .aria-label = لایه‌های نمودار
positions-chart-ema = EMA
    .title = میانگین‌های متحرک نمایی، 9 و 21
positions-chart-fit = تطبیق
    .title = نمایش کل مدت این پوزیشن
positions-chart-timeframes-group =
    .aria-label = بازه زمانی
positions-chart-pane-group =
    .aria-label = بخش نمودار
positions-chart-unavailable = موتور نمودار در دسترس نیست
positions-chart-collecting = در حال جمع‌آوری داده‌های نمودار…
positions-chart-no-data = هنوز داده‌ای برای نمودار این توکن وجود ندارد
positions-chart-avg-entry = میانگین ورود
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = میانگین ورود
positions-chart-legend-avg-entry-off-scale = میانگین ورود (خارج از مقیاس)
positions-chart-dropped-events =
    { $count ->
        [one] { $count } رویداد بدون کندل در این بازه زمانی
       *[other] { $count } رویداد بدون کندل در این بازه زمانی
    }
positions-chart-level = سطح
# $label names the reference level, $price is its formatted price.
positions-chart-level-above = { $label } { $price } بالاتر از این نما است
positions-chart-level-below = { $label } { $price } پایین‌تر از این نما است
positions-chart-scale-hint = برای مقیاس‌دهی تا آن سطح، محور قیمت را بکشید
positions-chart-pnl-at-bar = سود و زیان @ کندل
positions-chart-click-to-locate = برای یافتن کلیک کنید
