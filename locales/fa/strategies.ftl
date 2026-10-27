# Strategies page: the strategy list, the condition editor and the condition catalog.
# Condition text is addressed by the keys the schemas carry (src/strategies/conditions/catalog.rs):
#   strategies-condition-<type>                        name, with `.description`
#   strategies-condition-<type>-param-<param>          parameter name, with `.description`
#   strategies-condition-<type>-param-<param>-option-<value>
#   strategies-condition-category-<slug>
#   strategies-condition-param-timeframe / -timeframe-option-<value>   shared by every condition

## Strategy list

strategies-filter-all = همه
strategies-filter-entry = ورود
strategies-filter-exit = خروج
strategies-type-entry = ورود
strategies-type-exit = خروج
strategies-list-empty-title = هنوز استراتژی‌ای وجود ندارد
strategies-list-empty-hint = اولین استراتژی خود را بسازید
strategies-new = استراتژی جدید
strategies-import =
    .title = وارد کردن استراتژی
    .aria-label = وارد کردن استراتژی
strategies-item-enable =
    .title = فعال‌سازی
strategies-item-disable =
    .title = غیرفعال‌سازی

# Name given to a strategy before it is saved.
strategies-new-name = استراتژی جدید

## Editor

strategies-editor-name =
    .placeholder = نام استراتژی
strategies-editor-dirty =
    .title = تغییرات ذخیره‌نشده
strategies-action-validate = اعتبارسنجی
strategies-editor-empty = استراتژی‌ای برای ویرایش انتخاب کنید یا استراتژی جدیدی بسازید
strategies-conditions-empty-title = هنوز شرطی وجود ندارد
strategies-conditions-empty-hint = برای شروع ساخت، «{ strategies-add-condition }» را بزنید
strategies-add-condition = افزودن شرط
strategies-modal-close =
    .aria-label = بستن
strategies-card-move-up =
    .title = انتقال به بالا
strategies-card-move-down =
    .title = انتقال به پایین
strategies-card-duplicate =
    .title = تکثیر
strategies-card-delete =
    .title = حذف
# $name is the condition name.
strategies-card-delete-confirm = حذف شرط
    .message = «{ $name }» از این استراتژی حذف شود؟

# Card summary: one "label: value" entry per parameter.
strategies-summary-param = { $label }: { $value }
strategies-summary-none = بدون پارامتر
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = تنظیم استراتژی ({ $value })
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount } ثانیه
strategies-value-duration-minutes = { $amount } دقیقه
strategies-value-duration-hours = { $amount } ساعت
strategies-value-duration-days = { $amount } روز
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = وقت محلی شما: { $time }
strategies-hint-price-impact = معامله‌ای به این اندازه قیمت را حدود { $amount }٪ جابه‌جا می‌کند

# Parameter values in a card summary. $count selects the plural, $amount is the formatted number.
strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
        [one] { $amount } ساعت
       *[other] { $amount } ساعت
    }
strategies-value-candles =
    { $count ->
        [one] { $amount } کندل
       *[other] { $amount } کندل
    }

# Text written beside a numeric input.
strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = ساعت
strategies-unit-multiplier = ×
strategies-unit-utc = UTC

## Condition catalog

strategies-catalog-search =
    .placeholder = جستجوی شرط‌ها...
    .aria-label = جست‌وجوی شرط‌ها
strategies-catalog-search-clear =
    .aria-label = پاک کردن جستجو
strategies-catalog-no-description = توضیحی موجود نیست

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = فعال
strategies-editor-timeframe = بازه زمانی پیش‌فرض
    .title = بازه زمانی کندل برای شرط‌هایی که بازه زمانی خودشان را تعیین نمی‌کنند
strategies-editor-priority = اولویت
    .title = ترتیب ارزیابی از 1 تا 100؛ عدد کوچک‌تر زودتر ارزیابی می‌شود
strategies-editor-description = توضیحات
strategies-editor-description-field =
    .placeholder = این استراتژی چه می‌کند
    .aria-label = توضیحات استراتژی
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = همه استراتژی‌های ورود
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = مدیریت
strategies-editor-entry-binding-hint = موقعیت‌هایی را مدیریت می‌کند که استراتژی‌های ورودِ انتخاب‌شده باز کرده‌اند. اگر هیچ‌کدام انتخاب نشود، همه موقعیت‌ها از جمله دستی و کپی‌شده را مدیریت می‌کند.
strategies-editor-entry-binding-empty = هنوز هیچ استراتژی ورودی وجود ندارد
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id } (حذف‌شده)
strategies-action-export = برون‌بری
strategies-action-test = آزمایش روی یک توکن
# $token is the tested token's symbol or address.
strategies-test-title = آزمایش روی { $token }
# $token is the tested token's symbol or address.
strategies-test-running = در حال آزمایش روی { $token }…
strategies-test-verdict-pass = برقرار است
strategies-test-verdict-fail = برقرار نیست
strategies-test-verdict-unknown = نامشخص

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = شرط‌هایی که باید برقرار باشند: { $mode }
strategies-group-match = باید برقرار باشد: { $mode }
strategies-match-all = همه
strategies-match-any = دست‌کم یکی
strategies-match-mode =
    .aria-label = همه یا دست‌کم یکی از شرط‌ها
strategies-invert = معکوس
    .title = معکوس: وقتی این مورد برقرار نیست درست می‌شود
strategies-chip-off = خاموش
strategies-chip-inverted = معکوس‌شده
strategies-row-switch =
    .title = برای نگه‌داشتن بدون ارزیابی خاموش کنید
    .aria-label = توسط این استراتژی ارزیابی می‌شود
strategies-row-expand =
    .aria-label = نمایش یا پنهان‌کردن پارامترها
strategies-add-group = افزودن گروه
# $count is the number of conditions in the group.
strategies-group-delete-confirm = حذف گروه
    .message =
        { $count ->
            [one] این گروه و شرط آن حذف شود؟
           *[other] این گروه و { $count } شرط آن حذف شود؟
        }

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems =
    { $count ->
        [one] { $count } مشکل برای رفع: { $first }
       *[other] { $count } مشکل برای رفع: { $first }
    }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }، { $field }: { $problem }
strategies-problem-rules-empty = دست‌کم یک شرط روشن اضافه کنید
strategies-problem-group-empty = گروه دست‌کم به یک شرط روشن نیاز دارد
strategies-problem-not-applicable = این شرط در این نوع استراتژی قابل استفاده نیست
strategies-problem-unknown-condition = نوع شرط ناشناخته: { $type }
strategies-problem-too-deep = قاعده‌ها حداکثر تا { $max } سطح می‌توانند تو در تو باشند؛ گروه معکوس یک سطح اضافه حساب می‌شود
strategies-problem-required = یک عدد وارد کنید
strategies-problem-range = باید بین { $min } و { $max } باشد
strategies-problem-invalid-option = یکی از گزینه‌های فهرست را انتخاب کنید

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = همه
strategies-catalog-categories =
    .aria-label = دسته‌ها
strategies-catalog-empty = هیچ شرطی با جست‌وجوی شما مطابقت ندارد
strategies-catalog-exit-only = فقط برای استراتژی‌های خروج
strategies-catalog-entry-only = فقط برای استراتژی‌های ورود
strategies-catalog-candles-note = { $description } فقط کندل‌ها را می‌خواند.
strategies-data-candles = کندل‌ها
strategies-data-pool = استخر
strategies-data-position = موقعیت
strategies-data-market = فعالیت بازار
strategies-data-clock = ساعت روز

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = معامله
strategies-value-trades =
    { $count ->
        [one] { $amount } معامله
       *[other] { $amount } معامله
    }

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = سطح‌های بیش از حد
    .message = قاعده‌ها حداکثر تا { $max } سطح می‌توانند تو در تو باشند
strategies-toast-priority-invalid = اولویت نامعتبر
    .message = یک عدد صحیح از 1 تا 100 وارد کنید
strategies-toast-exported = استراتژی برون‌بری شد
# $name is the strategy name.
strategies-unsaved-confirm = تغییرات ذخیره‌نشده کنار گذاشته شود؟
    .message = «{ $name }» تغییرات ذخیره‌نشده دارد.
strategies-unsaved-discard = کنار گذاشتن
strategies-import-error-json = فایل JSON معتبر نیست
strategies-import-error-shape = فایل هیچ استراتژی‌ای را توصیف نمی‌کند
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = فایل از شرطی استفاده می‌کند که در این نسخه وجود ندارد: { $type }
strategies-import-error-not-applicable = «{ $name }» در این نوع استراتژی قابل استفاده نیست

## New strategy dialog

strategies-create-title = ساخت استراتژی جدید
strategies-create-prompt = نوع استراتژی‌ای را که می‌خواهید بسازید انتخاب کنید:
strategies-create-entry-name = استراتژی ورود
strategies-create-entry-description = شرط‌های زمان خرید یک توکن را تعریف کنید
strategies-create-exit-name = استراتژی خروج
strategies-create-exit-description = شرط‌های زمان فروش یک توکن را تعریف کنید

## Delete dialog

strategies-delete-title = حذف استراتژی
# $name is the strategy name.
strategies-delete-message = استراتژی «{ $name }» حذف شود؟ این کار قابل بازگشت نیست.

## Toasts. A message value is the title; `.message` is the body.

strategies-toast-fix-validation = پیش از ذخیره، خطاهای اعتبارسنجی را برطرف کنید
strategies-toast-load-failed = بارگذاری ناموفق بود
    .message = بارگذاری استراتژی‌ها از سرور ناموفق بود
strategies-toast-load-strategy-failed = بارگذاری استراتژی ناموفق بود
strategies-toast-no-strategy = استراتژی‌ای ساخته نشده است
    .message = ابتدا دست‌کم یک شرط اضافه کنید یا برای ساخت استراتژی «استراتژی جدید» را بزنید
strategies-toast-name-required = نام لازم است
    .message = پیش از ذخیره، نام استراتژی را وارد کنید
strategies-toast-saved = استراتژی ذخیره شد
    .message = «{ $name }» با موفقیت ذخیره شد
strategies-toast-save-failed = ذخیره ناموفق بود
    .message = ذخیره استراتژی در پایگاه داده ناموفق بود
strategies-toast-no-strategy-validate = استراتژی‌ای برای اعتبارسنجی وجود ندارد
strategies-toast-no-strategy-test = استراتژی‌ای برای آزمایش نیست
strategies-toast-test-save-first = پیش از آزمایش، استراتژی را ذخیره کنید: آزمایش قوانین ذخیره‌شده را می‌خواند
strategies-toast-test-failed = آزمایش ناموفق بود
    .message = آزمایش استراتژی ممکن نشد
strategies-toast-valid = استراتژی معتبر است
strategies-toast-invalid = استراتژی خطا دارد
strategies-toast-validation-failed = اعتبارسنجی ناموفق بود
strategies-toast-item-enabled = استراتژی فعال شد
strategies-toast-item-disabled = استراتژی غیرفعال شد
strategies-toast-item-toggle-failed = تغییر وضعیت استراتژی ناموفق بود
strategies-toast-deleted = استراتژی حذف شد
    .message = «{ $name }» با موفقیت حذف شد
strategies-toast-delete-failed = حذف ناموفق بود
    .message = حذف استراتژی از پایگاه داده ناموفق بود
strategies-toast-imported = استراتژی وارد شد
strategies-toast-import-failed = وارد کردن استراتژی ناموفق بود
strategies-toast-unknown-condition = شرط ناشناخته
    .message = نوع شرط پیدا نشد
strategies-toast-create-first = ابتدا استراتژی بسازید
    .message = پیش از افزودن شرط، برای ساخت استراتژی «استراتژی جدید» را بزنید

## Conditions

strategies-condition-candle-size = الگوی اندازه کندل
    .description = تشخیص الگوهای مشخص کندل: بدنه بزرگ، بدنه کوچک (دوجی)، سایه‌های بلند.
strategies-condition-candle-size-param-pattern = نوع الگو
    .description = الگوی کندلی که باید تشخیص داده شود
strategies-condition-candle-size-param-pattern-option-large-body = بدنه بزرگ (حرکت قوی)
strategies-condition-candle-size-param-pattern-option-small-body = بدنه کوچک (دوجی/تردید)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = سایه بالایی بلند (رد قیمت)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = سایه پایینی بلند (حمایت)
strategies-condition-candle-size-param-threshold = آستانه اندازه %
    .description = آستانه درصدی برای تشخیص الگو

strategies-condition-consecutive-candles = کندل‌های متوالی
    .description = تشخیص کندل‌های سبز (صعودی) یا قرمز (نزولی) متوالی با فیلتر حداقل اندازه.
strategies-condition-consecutive-candles-param-count = تعداد کندل
    .description = تعداد کندل‌های متوالی موردنیاز
strategies-condition-consecutive-candles-param-direction = جهت کندل
    .description = رنگ/جهت کندل‌های متوالی
strategies-condition-consecutive-candles-param-direction-option-green = سبز (صعودی)
strategies-condition-consecutive-candles-param-direction-option-red = قرمز (نزولی)
strategies-condition-consecutive-candles-param-minimum-change = حداقل تغییر %
    .description = حداقل درصد تغییر برای هر کندل (حذف نویز)

strategies-condition-liquidity-level = سطح نقدینگی استخر
    .description = بررسی نقدینگی استخر به { -sol } (ورود: اطمینان از نقدینگی کافی، خروج: تشخیص خروج نقدینگی)
strategies-condition-liquidity-level-param-threshold = آستانه نقدینگی ({ -sol })
    .description = سطح نقدینگی استخر به { -sol }
strategies-condition-liquidity-level-param-comparison = مقایسه
    .description = نحوه مقایسه نقدینگی استخر با آستانه
strategies-condition-liquidity-level-param-comparison-option-greater-than = بزرگ‌تر از (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = بزرگ‌تر یا مساوی (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = کوچک‌تر از ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = کوچک‌تر یا مساوی (≤)

strategies-condition-position-holding-time = مدت نگهداری پوزیشن
    .description = بررسی مدتی که پوزیشن نگه داشته شده است
strategies-condition-position-holding-time-param-hours = آستانه زمانی (ساعت)
    .description = مدت زمان بر حسب ساعت از باز شدن پوزیشن
strategies-condition-position-holding-time-param-comparison = مقایسه
    .description = نحوه مقایسه عمر پوزیشن با آستانه
strategies-condition-position-holding-time-param-comparison-option-greater-than = قدیمی‌تر از (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = دست‌کم (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = جدیدتر از ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = حداکثر (≤)

strategies-condition-price-breakout = شکست قیمت
    .description = تشخیص شکسته شدن مقاومت (سقف دوره) به بالا یا حمایت (کف دوره) به پایین.
strategies-condition-price-breakout-param-lookback = بازه بازبینی
    .description = تعداد کندل‌ها برای یافتن سطح حمایت/مقاومت
strategies-condition-price-breakout-param-direction = جهت شکست
    .description = جهت شکست قیمت
strategies-condition-price-breakout-param-direction-option-upward = رو به بالا (شکست مقاومت)
strategies-condition-price-breakout-param-direction-option-downward = رو به پایین (شکست حمایت)
strategies-condition-price-breakout-param-confirmation = تأیید %
    .description = میزان عبور از سطح برای تأیید شکست (جلوگیری از سیگنال کاذب)

strategies-condition-price-change-percent = تغییر قیمت %
    .description = بررسی اینکه قیمت در یک بازه زمانی به اندازه آستانه درصدی تغییر کرده است یا نه.
strategies-condition-price-change-percent-param-percentage = آستانه تغییر %
    .description = درصد تغییر قیمت برای فعال شدن (0.1 تا 1000%)
strategies-condition-price-change-percent-param-direction = جهت
    .description = جهت حرکت قیمت
strategies-condition-price-change-percent-param-direction-option-above = افزایش (+%)
strategies-condition-price-change-percent-param-direction-option-below = کاهش (-%)
strategies-condition-price-change-percent-param-direction-option-within = درون بازه (±%)
strategies-condition-price-change-percent-param-time-value = بازه بازبینی
    .description = تا چه زمانی به عقب تغییر سنجیده می‌شود، با واحدی که داخل فیلد انتخاب می‌شود (1 تا 3600 ثانیه، 1 تا 1440 دقیقه، 1 تا 720 ساعت)
strategies-condition-price-change-percent-param-time-unit = واحد زمان
    .description = واحد زمان برای بازه بازبینی
strategies-condition-price-change-percent-param-time-unit-option-seconds = ثانیه
strategies-condition-price-change-percent-param-time-unit-option-minutes = دقیقه
strategies-condition-price-change-percent-param-time-unit-option-hours = ساعت

strategies-condition-price-to-ma = قیمت در برابر میانگین متحرک
    .description = بررسی اینکه قیمت بالاتر، پایین‌تر یا در بازه‌ای از میانگین متحرک ساده خود است.
strategies-condition-price-to-ma-param-period = دوره MA
    .description = تعداد کندل‌ها برای محاسبه میانگین متحرک
strategies-condition-price-to-ma-param-position = موقعیت
    .description = موقعیت قیمت نسبت به MA
strategies-condition-price-to-ma-param-position-option-above = بالای MA
strategies-condition-price-to-ma-param-position-option-below = پایین MA
strategies-condition-price-to-ma-param-position-option-within = درون بازه
strategies-condition-price-to-ma-param-distance = فاصله %
    .description = حداقل فاصله از MA (برای بالا/پایین) یا حداکثر بازه (برای درون بازه)

strategies-condition-volume-spike = جهش حجم
    .description = تشخیص جهش حجم نسبت به میانگین حجم (نشانه افزایش توجه بازار).
strategies-condition-volume-spike-param-lookback = بازه بازبینی
    .description = تعداد کندل‌ها برای محاسبه میانگین حجم
strategies-condition-volume-spike-param-multiplier = ضریب حجم
    .description = چند برابر میانگین (مثلاً 2.0 = 200% میانگین)

strategies-condition-rsi = RSI (شاخص قدرت نسبی)
    .description = وقتی فعال می‌شود که RSI، با هموارسازی وایلدر روی کندل‌های بسته‌شده، بالاتر یا پایین‌تر از یک آستانه باشد.
strategies-condition-rsi-param-period = دوره RSI
    .description = تعداد کندل‌هایی که RSI روی آن‌ها محاسبه می‌شود؛ شرط به یک کندل بیشتر از این مقدار نیاز دارد
strategies-condition-rsi-param-comparison = مقایسه
    .description = RSI باید پایین‌تر از آستانه باشد یا بالاتر
strategies-condition-rsi-param-comparison-option-below = پایین‌تر (اشباع فروش)
strategies-condition-rsi-param-comparison-option-above = بالاتر (اشباع خرید)
strategies-condition-rsi-param-threshold = آستانه RSI
    .description = سطح RSI از 1 تا 99 (مقادیر رایج 30 برای اشباع فروش و 70 برای اشباع خرید است)

strategies-condition-ma-cross = تقاطع میانگین‌های متحرک
    .description = وقتی فعال می‌شود که یک میانگین متحرک سریع در چند کندل اخیر میانگین کند را قطع کند.
strategies-condition-ma-cross-param-ma-type = نوع میانگین
    .description = میانگین‌های نمایی به کندل‌های اخیر وزن بیشتری می‌دهند؛ میانگین‌های ساده همه کندل‌ها را یکسان وزن می‌کنند
strategies-condition-ma-cross-param-ma-type-option-ema = نمایی (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = ساده (SMA)
strategies-condition-ma-cross-param-fast-period = دوره سریع
    .description = تعداد کندل‌ها در میانگین سریع
strategies-condition-ma-cross-param-slow-period = دوره کند
    .description = تعداد کندل‌ها در میانگین کند؛ باید از دوره سریع بزرگ‌تر باشد
strategies-condition-ma-cross-param-direction = جهت تقاطع
    .description = میانگین سریع در چه جهتی میانگین کند را قطع می‌کند
strategies-condition-ma-cross-param-direction-option-bullish = صعودی (سریع به بالا قطع می‌کند)
strategies-condition-ma-cross-param-direction-option-bearish = نزولی (سریع به پایین قطع می‌کند)
strategies-condition-ma-cross-param-within-candles = تقاطع در آخرین
    .description = تقاطع می‌تواند در چند کندل آخر باشد؛ 1 یعنی فقط جدیدترین کندل

strategies-condition-position-profit = سود پوزیشن
    .description = وقتی فعال می‌شود که سود تحقق‌نیافته پوزیشن باز بالاتر یا پایین‌تر از درصدی از قیمت ورود باشد، با قیمت استخر سنجیده می‌شود.
strategies-condition-position-profit-param-comparison = مقایسه
    .description = سود باید دست‌کم برابر آستانه یا حداکثر برابر آن باشد
strategies-condition-position-profit-param-comparison-option-above = دست‌کم (≥)
strategies-condition-position-profit-param-comparison-option-below = حداکثر (≤)
strategies-condition-position-profit-param-threshold-pct = آستانه سود %
    .description = سود تحقق‌نیافته به‌صورت درصدی از قیمت ورود؛ مقادیر منفی زیان هستند

strategies-condition-drawdown-from-peak = افت از اوج
    .description = وقتی فعال می‌شود که قیمت استخر به‌اندازه یک درصد از بالاترین قیمت دیده‌شده از آخرین خرید افت کرده باشد، به شرط آنکه آن اوج به‌اندازه کافی بالای قیمت ورود بوده باشد.
strategies-condition-drawdown-from-peak-param-drop-pct = افت از اوج %
    .description = قیمت فعلی چقدر باید پایین‌تر از اوج باشد
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = فعال‌سازی پس از سود %
    .description = اوج باید دست‌کم این‌قدر بالاتر از قیمت ورود بوده باشد تا شرط بتواند فعال شود؛ 0 یعنی همیشه فعال

strategies-condition-buy-pressure = فشار خرید
    .description = وقتی فعال می‌شود که خریدها دست‌کم سهم معینی از تراکنش‌های یک بازه زمانی باشند و آن بازه تراکنش کافی داشته باشد. تعداد خرید و فروش توکن را می‌خواند.
strategies-condition-buy-pressure-param-window = بازه زمانی
    .description = دوره‌ای که تعداد خرید و فروش پوشش می‌دهد
strategies-condition-buy-pressure-param-window-option-5m = 5 دقیقه
strategies-condition-buy-pressure-param-window-option-1h = 1 ساعت
strategies-condition-buy-pressure-param-window-option-6h = 6 ساعت
strategies-condition-buy-pressure-param-window-option-24h = 24 ساعت
strategies-condition-buy-pressure-param-min-buy-share-pct = حداقل سهم خرید %
    .description = خریدها به‌صورت درصدی از همه تراکنش‌های بازه
strategies-condition-buy-pressure-param-min-transactions = حداقل تراکنش
    .description = بازه‌هایی با تراکنش کمتر هرگز شرط را فعال نمی‌کنند

strategies-condition-transaction-velocity = سرعت تراکنش
    .description = وقتی فعال می‌شود که نرخ تراکنش یک بازه اخیر دست‌کم یا حداکثر ضریبی از نرخ بازه بلندتری باشد که آن را در بر دارد. تعداد خرید و فروش توکن را می‌خواند.
strategies-condition-transaction-velocity-param-window = بازه‌ها
    .description = بازه اخیر و بازه بلندتری که با آن مقایسه می‌شود
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = ۵ دقیقه در برابر ۱ ساعت
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = ۱ ساعت در برابر ۶ ساعت
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = ۶ ساعت در برابر ۲۴ ساعت
strategies-condition-transaction-velocity-param-comparison = مقایسه
    .description = آیا نرخ اخیر باید دست‌کم یا حداکثر برابر نسبت باشد
strategies-condition-transaction-velocity-param-comparison-option-above = دست‌کم (≥)
strategies-condition-transaction-velocity-param-comparison-option-below = حداکثر (≤)
strategies-condition-transaction-velocity-param-ratio = نسبت نرخ
    .description = تراکنش در ثانیه بازه اخیر تقسیم بر تراکنش در ثانیه بازه بلند
strategies-condition-transaction-velocity-param-min-transactions = حداقل تراکنش
    .description = بازه‌های اخیر با تراکنش کمتر هرگز شرط را فعال نمی‌کنند

strategies-condition-pullback-from-high = اصلاح از سقف
    .description = وقتی فعال می‌شود که قیمت زنده بین کمترین و بیشترین افت زیر بالاترین سقف کندل‌های اخیر باشد.
strategies-condition-pullback-from-high-param-lookback = بازه بازبینی
    .description = تعداد کندل‌هایی که بالاترین سقف از آن‌ها گرفته می‌شود
strategies-condition-pullback-from-high-param-min-drop-pct = کمترین افت ٪
    .description = قیمت باید دست‌کم این اندازه زیر سقف باشد
strategies-condition-pullback-from-high-param-max-drop-pct = بیشترین افت ٪
    .description = قیمت باید حداکثر این اندازه زیر سقف باشد؛ باید از کمترین افت بزرگ‌تر باشد

strategies-condition-profit-giveback = بازپس‌دهی سود
    .description = وقتی فعال می‌شود که پوزیشن بخشی از بهترین سود تحقق‌نیافته‌اش از آخرین خرید را پس داده باشد، پس از آنکه آن سود به سطحی رسیده باشد.
strategies-condition-profit-giveback-param-arm-after-profit-pct = فعال پس از سود ٪
    .description = بهترین سود باید پیش از فعال شدن شرط به این درصد از قیمت ورود رسیده باشد
strategies-condition-profit-giveback-param-giveback-pct = بازپس‌دهی ٪
    .description = سهم بازپس‌داده‌شده از بهترین سود؛ ۱۰۰٪ یعنی قیمت به قیمت ورود برگشته است

strategies-condition-token-age = عمر توکن
    .description = وقتی فعال می‌شود که زمان سپری‌شده از ساخت درون‌زنجیره‌ای توکن بیشتر یا کمتر از یک مدت باشد. توکنی که زمان ساختش معلوم نیست شرط را نامعلوم می‌گذارد.
strategies-condition-token-age-param-comparison = مقایسه
    .description = آیا توکن باید قدیمی‌تر یا جدیدتر از مدت باشد
strategies-condition-token-age-param-comparison-option-older-than = قدیمی‌تر از (≥)
strategies-condition-token-age-param-comparison-option-younger-than = جدیدتر از ({ "<" })
strategies-condition-token-age-param-time-value = عمر
    .description = زمان از ساخت توکن، با واحدی که درون فیلد انتخاب می‌شود (۱ تا ۱۰۰۰۰)
strategies-condition-token-age-param-time-unit = واحد زمان
    .description = واحد عمر
strategies-condition-token-age-param-time-unit-option-minutes = دقیقه
strategies-condition-token-age-param-time-unit-option-hours = ساعت
strategies-condition-token-age-param-time-unit-option-days = روز

strategies-condition-vwap = VWAP
    .description = وقتی فعال می‌شود که قیمت زنده دست‌کم به اندازه یک فاصله بالاتر یا پایین‌تر از میانگین قیمت وزن‌دار با حجم کندل‌های اخیر باشد.
strategies-condition-vwap-param-lookback = بازه بازبینی
    .description = تعداد کندل‌هایی که میانگین وزن‌دار با حجم در بر می‌گیرد
strategies-condition-vwap-param-position = موقعیت
    .description = موقعیت قیمت نسبت به VWAP
strategies-condition-vwap-param-position-option-above = بالای VWAP
strategies-condition-vwap-param-position-option-below = پایین VWAP
strategies-condition-vwap-param-distance-pct = فاصله ٪
    .description = کمترین فاصله از VWAP؛ صفر یعنی هر فاصله‌ای در سمت انتخاب‌شده

strategies-condition-volatility-range = دامنه نوسان
    .description = وقتی فعال می‌شود که میانگین دامنه واقعی کندل‌های اخیر، به‌صورت درصدی از قیمت زنده، بالاتر یا پایین‌تر از یک آستانه باشد. دو شرط از این نوع در یک گروه یک بازه می‌سازند.
strategies-condition-volatility-range-param-period = دوره ATR
    .description = تعداد کندل‌هایی که میانگین دامنه واقعی روی آن‌ها هموار می‌شود؛ شرط یک کندل بیشتر لازم دارد
strategies-condition-volatility-range-param-comparison = مقایسه
    .description = آیا دامنه باید بالاتر یا پایین‌تر از آستانه باشد
strategies-condition-volatility-range-param-comparison-option-above = بالاتر
strategies-condition-volatility-range-param-comparison-option-below = پایین‌تر
strategies-condition-volatility-range-param-threshold-pct = آستانه دامنه ٪
    .description = میانگین دامنه واقعی به‌صورت درصدی از قیمت زنده

strategies-condition-bollinger-bands = باندهای بولینگر
    .description = وقتی فعال می‌شود که قیمت زنده زیر باند پایین یا بالای باند بالا باشد، یا باندها به هم فشرده شوند. باندها چند انحراف معیار دورتر از میانگین قیمت بسته شدن قرار دارند.
strategies-condition-bollinger-bands-param-period = دوره
    .description = تعداد کندل‌هایی که میانگین و انحراف معیار در بر می‌گیرند
strategies-condition-bollinger-bands-param-std-dev = انحراف‌های معیار
    .description = هر باند چند انحراف معیار از میانگین فاصله دارد
strategies-condition-bollinger-bands-param-signal = سیگنال
    .description = آنچه باندها باید نشان دهند
strategies-condition-bollinger-bands-param-signal-option-below-lower = زیر باند پایین
strategies-condition-bollinger-bands-param-signal-option-above-upper = بالای باند بالا
strategies-condition-bollinger-bands-param-signal-option-squeeze = فشردگی
strategies-condition-bollinger-bands-param-squeeze-width-pct = پهنای فشردگی ٪
    .description = برای فشردگی: بیشترین فاصله میان باندها، به‌صورت درصدی از میانگین

strategies-condition-liquidity-to-size = نقدینگی نسبت به اندازه
    .description = وقتی فعال می‌شود که ذخایر استخر دست‌کم یا کمتر از ضریبی از اندازه درگیر باشد: اندازه معامله در استراتژی ورود، ارزش باقی‌مانده پوزیشن در استراتژی خروج.
strategies-condition-liquidity-to-size-param-comparison = مقایسه
    .description = آیا ذخایر باید دست‌کم یا کمتر از ضریب باشد
strategies-condition-liquidity-to-size-param-comparison-option-at-least = دست‌کم (≥)
strategies-condition-liquidity-to-size-param-comparison-option-below = کمتر از ({ "<" })
strategies-condition-liquidity-to-size-param-multiple = ضریب نقدینگی
    .description = ذخایر استخر تقسیم بر اندازه درگیر؛ ضریب N قیمت را حدود 1/(N+1) جابه‌جا می‌کند

strategies-condition-position-adds = افزوده‌های پوزیشن
    .description = وقتی فعال می‌شود که تعداد خریدهای افزوده پس از ورود اول، یا فروش‌های جزئی، دست‌کم یا حداکثر یک عدد باشد.
strategies-condition-position-adds-param-subject = مورد شمارش
    .description = کدام اقدام‌ها شمرده می‌شوند
strategies-condition-position-adds-param-subject-option-adds = خریدهای افزوده
strategies-condition-position-adds-param-subject-option-partial-sells = فروش‌های جزئی
strategies-condition-position-adds-param-comparison = مقایسه
    .description = آیا شمارش باید دست‌کم یا حداکثر برابر عدد باشد
strategies-condition-position-adds-param-comparison-option-at-least = دست‌کم (≥)
strategies-condition-position-adds-param-comparison-option-at-most = حداکثر (≤)
strategies-condition-position-adds-param-count = تعداد
    .description = تعداد اقدام‌ها

strategies-condition-time-of-day = ساعت روز
    .description = وقتی فعال می‌شود که ساعت کنونی درون بازه‌ای از ساعت‌های UTC باشد. شروعی پس از پایان از نیمه‌شب می‌گذرد.
strategies-condition-time-of-day-param-start-hour = ساعت شروع (UTC)
    .description = نخستین ساعت بازه، از ۰ تا ۲۳
strategies-condition-time-of-day-param-end-hour = ساعت پایان (UTC)
    .description = ساعتی که بازه در آن پایان می‌یابد، از ۱ تا ۲۴؛ خود آن ساعت بیرون از بازه است

strategies-condition-liquidity-change-since-entry = تغییر نقدینگی از ورود
    .description = وقتی فعال می‌شود که ذخایر استخر از زمان باز شدن پوزیشن دست‌کم به اندازه یک درصد کاهش یا افزایش یافته باشد. پوزیشنی که نقدینگی ورود ثبت‌شده ندارد شرط را نامعلوم می‌گذارد.
strategies-condition-liquidity-change-since-entry-param-comparison = مقایسه
    .description = آیا ذخایر باید کاهش یا افزایش یافته باشد
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = کاهش به اندازه
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = افزایش به اندازه
strategies-condition-liquidity-change-since-entry-param-change-pct = تغییر ٪
    .description = تغییر ذخایر استخر از زمان ورود

## Shared by every condition

strategies-condition-param-timeframe = بازه زمانی
    .description = اندازه کندل: طول هر کندلی که شرط می‌خواند (در صورت تنظیم نشدن، بازه زمانی استراتژی)
strategies-condition-timeframe-option-1m = 1 دقیقه
strategies-condition-timeframe-option-5m = 5 دقیقه
strategies-condition-timeframe-option-15m = 15 دقیقه
strategies-condition-timeframe-option-1h = 1 ساعت
strategies-condition-timeframe-option-4h = 4 ساعت
strategies-condition-timeframe-option-12h = 12 ساعت
strategies-condition-timeframe-option-1d = 1 روز

## Condition categories

strategies-condition-category-price-analysis = تحلیل قیمت
strategies-condition-category-candle-patterns = الگوهای کندلی
strategies-condition-category-technical-indicators = اندیکاتورهای تکنیکال
strategies-condition-category-market-context = وضعیت بازار
strategies-condition-category-position-performance = پوزیشن و عملکرد
strategies-condition-category-volume-analysis = تحلیل حجم

## Validation errors
# Each validation error is a `UiText`; the tokens below name what the message refers to.

strategies-error-missing-parameter = پارامتر { $field } وجود ندارد
strategies-error-parameter-type = پارامتر { $field } باید { $expected } باشد
strategies-error-invalid-value = «{ $value }» یک { $field } معتبر نیست
strategies-error-missing-data = { $data } در دسترس نیست
strategies-error-no-candle-data = بازه زمانی { $timeframe } داده کندل ندارد
strategies-error-insufficient-history = تاریخچه برای { $indicator } کافی نیست: { $available } ثانیه موجود، { $required } ثانیه لازم
strategies-error-insufficient-candles = کندل‌ها برای { $indicator } کافی نیستند: { $available } موجود، { $required } لازم
strategies-error-stale-candle-data = داده کندل بازه { $timeframe } قدیمی است: عمر { $age } ثانیه از { $max } ثانیه بیشتر است
strategies-error-stale-market-data = تعداد تراکنش‌های بازه { $window } قدیمی است: عمر { $age } ثانیه از { $max } ثانیه بیشتر است
strategies-error-unusable-data = مقدار «{ $value }» برای { $field } قابل استفاده نیست
strategies-error-invalid-rule-tree = درخت قوانین نامعتبر است: { $reason }
strategies-error-evaluation-timeout = ارزیابی استراتژی پس از { $timeout } ms به پایان زمان مجاز رسید
strategies-error-condition-not-applicable = { $condition } در استراتژی‌های نوع { $strategy } قابل استفاده نیست
strategies-error-invalid-rules = خواندن قوانین ممکن نشد: { $reason }

# Parameter names

strategies-error-field-average-volume = میانگین حجم
strategies-error-field-candle-open = قیمت باز شدن کندل
strategies-error-field-comparison = مقایسه
strategies-error-field-condition-type = نوع شرط
strategies-error-field-confirmation = تأیید
strategies-error-field-count = تعداد
strategies-error-field-current-price = قیمت فعلی
strategies-error-field-direction = جهت
strategies-error-field-distance = فاصله
strategies-error-field-hours = ساعت
strategies-error-field-lookback = بازه بازبینی
strategies-error-field-minimum-change = حداقل تغییر
strategies-error-field-multiplier = ضریب
strategies-error-field-pattern = الگو
strategies-error-field-percentage = درصد
strategies-error-field-period = دوره
strategies-error-field-position = موقعیت
strategies-error-field-threshold = آستانه
strategies-error-field-time-unit = واحد زمان
strategies-error-field-time-value = مقدار زمان
strategies-error-field-timeframe = بازه زمانی
strategies-error-field-arm-after-profit = فعال‌سازی پس از سود
strategies-error-field-drop-percentage = درصد افت
strategies-error-field-entry-price = قیمت ورود
strategies-error-field-fast-period = دوره سریع
strategies-error-field-minimum-buy-share = حداقل سهم خرید
strategies-error-field-minimum-transactions = حداقل تراکنش
strategies-error-field-moving-average-type = نوع میانگین
strategies-error-field-moving-average = میانگین متحرک
strategies-error-field-reference-candle-close = قیمت بسته شدن کندل مرجع
strategies-error-field-peak-price = قیمت اوج
strategies-error-field-slow-period = دوره کند
strategies-error-field-window = بازه
strategies-error-field-within-candles = تعداد کندل تقاطع
strategies-error-field-change = تغییر
strategies-error-field-end-hour = ساعت پایان
strategies-error-field-entry-liquidity = نقدینگی ورود
strategies-error-field-giveback = بازپس‌دهی
strategies-error-field-highest-high = بالاترین سقف
strategies-error-field-long-window-transactions = تراکنش‌های بازه بلند
strategies-error-field-maximum-drop = بیشترین افت
strategies-error-field-minimum-drop = کمترین افت
strategies-error-field-multiple = ضریب
strategies-error-field-pool-liquidity = نقدینگی استخر
strategies-error-field-ratio = نسبت
strategies-error-field-remaining-position-value = ارزش باقی‌مانده پوزیشن
strategies-error-field-signal = سیگنال
strategies-error-field-squeeze-width = پهنای فشردگی
strategies-error-field-standard-deviations = انحراف‌های معیار
strategies-error-field-start-hour = ساعت شروع
strategies-error-field-subject = مورد شمارش
strategies-error-field-token-creation-time = زمان ساخت توکن
strategies-error-field-total-volume = حجم کل
strategies-error-field-trade-size = اندازه معامله
strategies-error-field-vwap = VWAP

# Expected parameter types

strategies-error-expected-boolean = مقدار منطقی
strategies-error-expected-number = عدد
strategies-error-expected-string = متن

# Missing context data

strategies-error-data-current-price = قیمت فعلی
strategies-error-data-liquidity-data = داده نقدینگی
strategies-error-data-market-data = داده بازار
strategies-error-data-ohlcv-data = داده OHLCV
strategies-error-data-position-data = داده پوزیشن
strategies-error-data-peak-price = قیمت اوج
strategies-error-data-transaction-counts = تعداد تراکنش‌ها
strategies-error-data-entry-liquidity = نقدینگی ورود
strategies-error-data-entry-pool-liquidity = نقدینگی استخر ورود
strategies-error-data-real-pool-reserves = نقدینگی واقعی استخر
strategies-error-data-remaining-position-value = ارزش باقی‌مانده پوزیشن
strategies-error-data-token-creation-time = زمان ساخت توکن
strategies-error-data-trade-size = اندازه معامله

# Indicators

strategies-error-indicator-consecutive-candles = کندل‌های متوالی
strategies-error-indicator-moving-average = میانگین متحرک
strategies-error-indicator-price-breakout = شکست قیمت
strategies-error-indicator-price-change-lookback = بازه بازبینی تغییر قیمت
strategies-error-indicator-volume-spike = جهش حجم
strategies-error-indicator-moving-average-cross = تقاطع میانگین‌های متحرک
strategies-error-indicator-rsi = RSI
strategies-error-indicator-bollinger-bands = باندهای بولینگر
strategies-error-indicator-pullback-from-high = اصلاح از سقف
strategies-error-indicator-volatility-range = دامنه نوسان
strategies-error-indicator-vwap = VWAP

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = گره شاخه فاقد شرط است
strategies-error-rule-branch-node-missing-operator = گره شاخه فاقد عملگر است
strategies-error-rule-branch-node-must-have-at-least-one-child = گره شاخه باید دست‌کم یک فرزند داشته باشد
strategies-error-rule-invalid-rule-tree-structure = ساختار درخت قوانین نامعتبر است
strategies-error-rule-leaf-node-missing-condition = گره برگ فاقد شرط است
strategies-error-rule-not-operator-must-have-exactly-one-child = عملگر NOT باید دقیقاً یک فرزند داشته باشد
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = هر گروه دست‌کم به یک شرط روشن نیاز دارد
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = عملگر NOT نمی‌تواند فرزند خاموش داشته باشد
strategies-error-rule-rules-nest-deeper-than-four-levels = قوانین نمی‌توانند بیش از چهار سطح تودرتو باشند
strategies-error-rule-the-root-group-cannot-be-switched-off = گروه ریشه را نمی‌توان خاموش کرد
