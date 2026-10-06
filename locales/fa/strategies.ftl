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

# Card summary: up to three "label: value" entries.
strategies-summary-param = { $label }: { $value }
strategies-summary-parts =
    { $count ->
        [1] { $first }
        [2] { $first }، { $second }
       *[3] { $first }، { $second }، { $third }
    }
strategies-summary-none = بدون پارامتر
strategies-summary-period-seconds = دوره: { $amount } ثانیه
strategies-summary-period-minutes = دوره: { $amount } دقیقه
strategies-summary-period-hours = دوره: { $amount } ساعت

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

## Condition catalog

strategies-catalog-search =
    .placeholder = جستجوی شرط‌ها...
strategies-catalog-search-clear =
    .aria-label = پاک کردن جستجو
strategies-catalog-fold-all = بستن همه
strategies-catalog-unfold-all = باز کردن همه
strategies-catalog-no-description = توضیحی موجود نیست

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
strategies-toast-enabled = استراتژی فعال شد
    .message = «{ $name }» فعال شد
strategies-toast-disabled = استراتژی غیرفعال شد
    .message = «{ $name }» غیرفعال شد
strategies-toast-toggle-failed = تغییر وضعیت ناموفق بود
    .message = به‌روزرسانی وضعیت استراتژی ناموفق بود
strategies-toast-load-failed = بارگذاری ناموفق بود
    .message = بارگذاری استراتژی‌ها از سرور ناموفق بود
strategies-toast-created = استراتژی جدید
    .message =
        { $type ->
            [EXIT] استراتژی خروج جدید ساخته شد
           *[ENTRY] استراتژی ورود جدید ساخته شد
        }
strategies-toast-load-strategy-failed = بارگذاری استراتژی ناموفق بود
strategies-toast-no-strategy = استراتژی‌ای ساخته نشده است
    .message = ابتدا دست‌کم یک شرط اضافه کنید یا برای ساخت استراتژی «استراتژی جدید» را بزنید
strategies-toast-no-conditions-save = بدون شرط
    .message = پیش از ذخیره، دست‌کم یک شرط به استراتژی اضافه کنید
strategies-toast-name-required = نام لازم است
    .message = پیش از ذخیره، نام استراتژی را وارد کنید
strategies-toast-saved = استراتژی ذخیره شد
    .message = «{ $name }» با موفقیت ذخیره شد
strategies-toast-save-failed = ذخیره ناموفق بود
    .message = ذخیره استراتژی در پایگاه داده ناموفق بود
strategies-toast-no-strategy-validate = استراتژی‌ای برای اعتبارسنجی وجود ندارد
strategies-toast-no-conditions-validate = بدون شرط
    .message = پیش از اعتبارسنجی، دست‌کم یک شرط اضافه کنید
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
strategies-toast-condition-added = شرط اضافه شد
    .message = { $name } به استراتژی اضافه شد


## Conditions

strategies-condition-candle-size = الگوی اندازه کندل
    .description = تشخیص الگوهای مشخص کندل: بدنه بزرگ، بدنه کوچک (دوجی)، سایه‌های بلند
strategies-condition-candle-size-param-pattern = نوع الگو
    .description = الگوی کندلی که باید تشخیص داده شود
strategies-condition-candle-size-param-pattern-option-large-body = بدنه بزرگ (حرکت قوی)
strategies-condition-candle-size-param-pattern-option-small-body = بدنه کوچک (دوجی/تردید)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = سایه بالایی بلند (رد قیمت)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = سایه پایینی بلند (حمایت)
strategies-condition-candle-size-param-threshold = آستانه اندازه %
    .description = آستانه درصدی برای تشخیص الگو

strategies-condition-consecutive-candles = کندل‌های متوالی
    .description = تشخیص کندل‌های سبز (صعودی) یا قرمز (نزولی) متوالی با فیلتر حداقل اندازه
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
    .description = بررسی مدتی که پوزیشن نگه داشته شده است (برای استراتژی‌های خروج - خروج زمان‌محور)
strategies-condition-position-holding-time-param-hours = آستانه زمانی (ساعت)
    .description = مدت زمان بر حسب ساعت از باز شدن پوزیشن
strategies-condition-position-holding-time-param-comparison = مقایسه
    .description = نحوه مقایسه عمر پوزیشن با آستانه
strategies-condition-position-holding-time-param-comparison-option-greater-than = قدیمی‌تر از (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = دست‌کم (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = جدیدتر از ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = حداکثر (≤)

strategies-condition-price-breakout = شکست قیمت
    .description = تشخیص شکسته شدن مقاومت (سقف دوره) به بالا یا حمایت (کف دوره) به پایین
strategies-condition-price-breakout-param-lookback = بازه بازبینی
    .description = تعداد کندل‌ها برای یافتن سطح حمایت/مقاومت
strategies-condition-price-breakout-param-direction = جهت شکست
    .description = جهت شکست قیمت
strategies-condition-price-breakout-param-direction-option-upward = رو به بالا (شکست مقاومت)
strategies-condition-price-breakout-param-direction-option-downward = رو به پایین (شکست حمایت)
strategies-condition-price-breakout-param-confirmation = تأیید %
    .description = میزان عبور از سطح برای تأیید شکست (جلوگیری از سیگنال کاذب)

strategies-condition-price-change-percent = تغییر قیمت %
    .description = بررسی اینکه قیمت در یک بازه زمانی به اندازه آستانه درصدی تغییر کرده است یا نه
strategies-condition-price-change-percent-param-percentage = آستانه تغییر %
    .description = درصد تغییر قیمت برای فعال شدن (0.1 تا 1000%)
strategies-condition-price-change-percent-param-direction = جهت
    .description = جهت حرکت قیمت
strategies-condition-price-change-percent-param-direction-option-above = افزایش (+%)
strategies-condition-price-change-percent-param-direction-option-below = کاهش (-%)
strategies-condition-price-change-percent-param-direction-option-within = درون بازه (±%)
strategies-condition-price-change-percent-param-time-value = بازه زمانی
    .description = مقدار بازه بازبینی (1 تا 3600 برای ثانیه، 1 تا 1440 برای دقیقه، 1 تا 720 برای ساعت)
strategies-condition-price-change-percent-param-time-unit = واحد زمان
    .description = واحد زمان برای بازه بازبینی
strategies-condition-price-change-percent-param-time-unit-option-seconds = ثانیه
strategies-condition-price-change-percent-param-time-unit-option-minutes = دقیقه
strategies-condition-price-change-percent-param-time-unit-option-hours = ساعت

strategies-condition-price-to-ma = قیمت در برابر میانگین متحرک
    .description = بررسی اینکه قیمت بالاتر، پایین‌تر یا در بازه‌ای از میانگین متحرک ساده خود است
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
    .description = تشخیص جهش حجم نسبت به میانگین حجم (نشانه افزایش توجه بازار)
strategies-condition-volume-spike-param-lookback = بازه بازبینی
    .description = تعداد کندل‌ها برای محاسبه میانگین حجم
strategies-condition-volume-spike-param-multiplier = ضریب حجم
    .description = چند برابر میانگین (مثلاً 2.0 = 200% میانگین)

## Shared by every condition

strategies-condition-param-timeframe = بازه زمانی
    .description = بازه زمانی کندل برای تحلیل (در صورت تنظیم نشدن، بازه زمانی استراتژی استفاده می‌شود)
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
strategies-error-invalid-rule-tree = درخت قوانین نامعتبر است: { $reason }
strategies-error-evaluation-timeout = ارزیابی استراتژی پس از { $timeout } ms به پایان زمان مجاز رسید
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

# Indicators

strategies-error-indicator-consecutive-candles = کندل‌های متوالی
strategies-error-indicator-moving-average = میانگین متحرک
strategies-error-indicator-price-breakout = شکست قیمت
strategies-error-indicator-price-change-lookback = بازه بازبینی تغییر قیمت
strategies-error-indicator-volume-spike = جهش حجم

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = گره شاخه فاقد شرط است
strategies-error-rule-branch-node-missing-operator = گره شاخه فاقد عملگر است
strategies-error-rule-branch-node-must-have-at-least-one-child = گره شاخه باید دست‌کم یک فرزند داشته باشد
strategies-error-rule-invalid-rule-tree-structure = ساختار درخت قوانین نامعتبر است
strategies-error-rule-leaf-node-missing-condition = گره برگ فاقد شرط است
strategies-error-rule-not-operator-must-have-exactly-one-child = عملگر NOT باید دقیقاً یک فرزند داشته باشد
