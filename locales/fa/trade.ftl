# Trade dialog messages.

## Quote panel

# Shown in the quote panel when the quote request could not reach the core.
trade-quote-error-network = دریافت قیمت پیشنهادی ممکن نشد — اتصال خود را بررسی کنید و دوباره تلاش کنید
# Fallback title when the quote request failed without a message.
trade-quote-error-title = دریافت قیمت پیشنهادی ممکن نشد
trade-quote-title = پیش‌نمایش سواپ
trade-quote-refresh =
    .aria-label = به‌روزرسانی قیمت پیشنهادی
    .title = به‌روزرسانی قیمت پیشنهادی
trade-quote-idle = برای پیش‌نمایش سواپ، مقداری را انتخاب کنید
trade-quote-loading = در حال یافتن بهترین مسیر…
trade-quote-retry = تلاش دوباره
trade-quote-pay = پرداخت شما
trade-quote-receive = دریافت شما (تخمینی)
trade-quote-minimum = حداقل تضمین‌شده
    .title = کمترین مقداری که پس از حداکثر اسلیپیج دریافت می‌کنید. سواپ به‌جای اجرا با مقدار کمتر از آن، برگشت می‌خورد.
trade-quote-impact = تأثیر قیمتی
trade-quote-slippage = حداکثر اسلیپیج
trade-quote-platform-fee = کارمزد پلتفرم
    .title = 0.5% — حمایت از توسعه. پیش‌تر در قیمت پیشنهادی بالا لحاظ شده است.
trade-quote-network-fee = کارمزد شبکه
trade-quote-route = مسیر
trade-quote-disclaimer = قیمت‌ها به‌صورت زنده از زنجیره به‌روز می‌شوند. اگر سواپ بالاتر از حداقل تضمین‌شده شما اجرا نشود، برگشت می‌خورد؛ بنابراین هرگز کمتر از مقدار نمایش‌داده‌شده دریافت نمی‌کنید.
# Price impact below the resolution of the percentage display.
trade-quote-impact-tiny = { "<0.01%" }
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-quote-impact-warning = تأثیر قیمتی { $impact } از حداکثر اسلیپیج { $tolerance }% شما بیشتر است — این مقدار استخر را جابه‌جا می‌کند. مقدار کمتر به قیمت بازار نزدیک‌تر اجرا می‌شود.

## Units

trade-unit-sol = { -sol }
trade-unit-tokens = توکن

## Actions. Ids are the dialog actions: buy, sell, add.

trade-buy-title = خرید توکن
trade-buy-subtitle = مقدار را به { -sol } وارد کنید
trade-buy-confirm = اجرای خرید
trade-buy-hint = برای مقدار پیش‌فرض تنظیمات، خالی بگذارید
trade-sell-title = فروش پوزیشن
trade-sell-subtitle = درصد فروش را انتخاب کنید
trade-sell-confirm = اجرای فروش
trade-sell-hint = مقداری بین 1 تا 100 وارد کنید
trade-sell-input = درصد دلخواه
    .placeholder = 1-100
trade-add-title = افزودن به پوزیشن
trade-add-subtitle = DCA روی پوزیشن موجود
trade-add-confirm = افزودن به پوزیشن
trade-add-hint = برای اندازه DCA تنظیم‌شده، خالی بگذارید
trade-amount-input = مقدار دلخواه
    .placeholder = مقدار { -sol } را وارد کنید

## Presets

trade-presets-quick-amount = مقدار سریع
trade-presets-quick-sell = فروش سریع
trade-presets-match-entry = هم‌اندازه ورود
trade-presets-fixed-amount = مقدار ثابت
trade-preset-partial = جزئی
trade-preset-half = نیمی
trade-preset-most = بیشتر
trade-preset-full = خروج کامل
# $label is the preset's amount.
trade-preset-select =
    .aria-label = انتخاب { $label }

## Dialog chrome

trade-dialog-close =
    .aria-label = بستن پنجره
trade-input-max = حداکثر
    .aria-label = استفاده از حداکثر
trade-slider =
    .aria-label = لغزنده مقدار
trade-context-available = موجود
trade-context-position-size = حجم پوزیشن
trade-context-holdings = دارایی
trade-held-badge = در دست
    .title = شما در این توکن پوزیشن باز دارید
trade-manage-title = مدیریت دستی
trade-manage-description = معامله‌گر خودکار این پوزیشن را نمی‌فروشد و DCA نمی‌کند. برای اینکه خروج‌ها را مدیریت کند، تیک را بردارید.

## Slippage

trade-slippage-label = اسلیپیج
trade-slippage-presets =
    .aria-label = مقادیر آماده اسلیپیج
trade-slippage-auto = خودکار
trade-slippage-custom =
    .placeholder = دلخواه
    .aria-label = درصد دلخواه اسلیپیج
trade-slippage-note-auto = خودکار (از تنظیمات)
# $pct is the configured slippage as stored.
trade-slippage-note-auto-value = خودکار ({ $pct }% از تنظیمات)
# $pct is the override as typed.
trade-slippage-note-override = جایگزین: { $pct }%
# $pct is the override as typed.
trade-slippage-warning = اسلیپیج بالا: ممکن است تا { $pct }% کمتر از قیمت پیشنهادی دریافت کنید.
trade-impact-warning-title = هشدار تأثیر قیمتی بالا
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-impact-warning-text = تأثیر قیمتی این معامله <strong>{ $impact }</strong> است که از تحمل اسلیپیج <strong>{ $tolerance }%</strong> شما بیشتر است. ممکن است به‌طور قابل‌توجهی کمتر از انتظار دریافت کنید.
trade-impact-warning-proceed = با این حال ادامه بده

## Validation and verification

trade-error-invalid-number = عدد نامعتبر
trade-error-percentage-range = درصد باید بین 1 تا 100 باشد
trade-error-amount-positive = مقدار باید بزرگ‌تر از 0 باشد
trade-error-amount-minimum = حداقل: 0.001 { -sol }
# $needed is a formatted SOL amount, $reserve the fee headroom in SOL and $balance the formatted balance.
trade-error-insufficient = موجودی کافی نیست (نیاز: { $needed } به‌علاوه { $reserve } برای کارمزد، موجودی: { $balance })
trade-error-position-closed = این پوزیشن دیگر باز نیست.
trade-error-verify-failed = بررسی موجودی توکن ممکن نشد
trade-error-position-missing = پوزیشن پیدا نشد - ممکن است بسته شده باشد
# $expected and $current are formatted token amounts.
trade-error-balance-changed = موجودی توکن تغییر کرده است. مورد انتظار { $expected }، اکنون { $current }. لطفاً به‌روزرسانی کنید.
trade-error-verify-network = خطای شبکه در بررسی موجودی

## Quick trade

trade-quick-buy-title = خرید سریع
trade-quick-sell-title = فروش سریع
trade-quick-subtitle = آدرس مینت توکن را وارد کنید
trade-quick-mint-label = آدرس مینت توکن را وارد کنید
trade-quick-mint-input =
    .placeholder = آدرس مینت را وارد کنید یا بر اساس نماد جستجو کنید...
trade-quick-paste =
    .aria-label = چسباندن از کلیپ‌بورد
trade-quick-recent = اخیر:
trade-quick-fetching = در حال دریافت اطلاعات توکن...
trade-quick-continue = ادامه
trade-quick-token-not-found = توکن پیدا نشد
trade-quick-token-failed = دریافت توکن ناموفق بود
trade-quick-token-not-in-database = توکن در پایگاه داده پیدا نشد
trade-quick-token-info-failed = دریافت اطلاعات توکن ناموفق بود
trade-quick-no-position = پوزیشنی برای این توکن پیدا نشد
trade-quick-no-holdings = پوزیشن توکنی باقی‌مانده ندارد
trade-quick-position-failed = دریافت داده پوزیشن ناموفق بود

## Manual trade toasts

trade-toast-no-mint = آدرس مینتی در دسترس نیست
trade-toast-open-failed = باز کردن پنجره معامله ممکن نشد
trade-toast-pending-buy = خرید هنوز در حال انجام است
trade-toast-pending-add = افزودن هنوز در حال انجام است
trade-toast-pending-sell = فروش هنوز در حال انجام است
trade-toast-pending-message = مرورگر دیگر منتظر نمی‌ماند؛ نتیجه را در ردیف پوزیشن دنبال کنید
trade-toast-failed-buy = خرید ناموفق بود
trade-toast-failed-add = افزودن به پوزیشن ناموفق بود
trade-toast-failed-sell = فروش ناموفق بود

# Trade and close reasons. Ids are the Debug names of TradeReason
# (src/trader/types.rs) and the reasons written by src/positions.
trade-reason-strategy-signal = سیگنال استراتژی
trade-reason-manual-entry = ورود دستی
trade-reason-force-buy = خرید اجباری
trade-reason-copy-buy = خرید کپی
trade-reason-dca-scheduled = DCA زمان‌بندی‌شده
trade-reason-take-profit = حد سود
trade-reason-stop-loss = حد ضرر
trade-reason-trailing-stop = حد ضرر متحرک
trade-reason-time-override = بازنویسی زمانی
trade-reason-strategy-exit = خروج استراتژی
trade-reason-llm-analysis-exit = خروج تحلیل LLM
trade-reason-manual-exit = خروج دستی
trade-reason-risk-management = مدیریت ریسک
trade-reason-blacklisted = در فهرست سیاه
trade-reason-force-sell = فروش اجباری
trade-reason-copy-sell = فروش کپی
trade-reason-closed-externally = بسته‌شده از بیرون
trade-reason-wallet-history = تاریخچه کیف پول
trade-reason-exit-retry-pending = تلاش دوباره خروج در انتظار
trade-reason-synthetic-exit-permanent-failure = شکست دائمی خروج مصنوعی
# $reason is the label of the base reason. Applies to a closed_reason that
# carries the pending-verification suffix.
trade-reason-pending-verification = { $reason } (در انتظار بررسی)
# $note is the operator text of a force close.
trade-reason-force-closed = بسته‌شده به‌صورت اجباری: { $note }
# $reason is a stored closed_reason that has no label; it is shown as stored.
trade-reason-stored = { $reason }

# Toast shown when a quick-trade shortcut runs without a token selected (ui/quick_trade_shortcuts.js).
trade-quick-no-token = توکنی انتخاب نشده است
