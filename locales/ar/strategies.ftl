# Strategies page: the strategy list, the condition editor and the condition catalog.
# Condition text is addressed by the keys the schemas carry (src/strategies/conditions/catalog.rs):
#   strategies-condition-<type>                        name, with `.description`
#   strategies-condition-<type>-param-<param>          parameter name, with `.description`
#   strategies-condition-<type>-param-<param>-option-<value>
#   strategies-condition-category-<slug>
#   strategies-condition-param-timeframe / -timeframe-option-<value>   shared by every condition

## Strategy list

strategies-filter-all = الكل
strategies-filter-entry = الدخول
strategies-filter-exit = الخروج
strategies-type-entry = دخول
strategies-type-exit = خروج
strategies-list-empty-title = لا توجد استراتيجيات بعد
strategies-list-empty-hint = أنشئ أول استراتيجية لك
strategies-new = استراتيجية جديدة
strategies-import =
    .title = استيراد استراتيجية
    .aria-label = استيراد استراتيجية
strategies-item-enable =
    .title = تفعيل
strategies-item-disable =
    .title = تعطيل

# Name given to a strategy before it is saved.
strategies-new-name = استراتيجية جديدة

## Editor

strategies-editor-name =
    .placeholder = اسم الاستراتيجية
strategies-editor-dirty =
    .title = تغييرات غير محفوظة
strategies-editor-enabled =
    .aria-label = الاستراتيجية مفعّلة
    .title = الاستراتيجية مفعّلة
strategies-action-validate = تحقق
strategies-editor-empty = اختر استراتيجية لتعديلها، أو أنشئ استراتيجية جديدة
strategies-conditions-empty-title = لا توجد شروط بعد
strategies-conditions-empty-hint = استخدم «{ strategies-add-condition }» لبدء البناء
strategies-add-condition = إضافة شرط
strategies-modal-close =
    .aria-label = إغلاق
strategies-card-move-up =
    .title = نقل لأعلى
strategies-card-move-down =
    .title = نقل لأسفل
strategies-card-duplicate =
    .title = تكرار
strategies-card-delete =
    .title = حذف
# $name is the condition name.
strategies-card-delete-confirm = إزالة الشرط
    .message = إزالة «{ $name }» من هذه الاستراتيجية؟

# Card summary: one "label: value" entry per parameter.
strategies-summary-param = { $label }: { $value }
strategies-summary-none = لا توجد معاملات
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = إعداد الاستراتيجية ({ $value })
strategies-summary-period-seconds = الفترة: { $amount } ث
strategies-summary-period-minutes = الفترة: { $amount } د
strategies-summary-period-hours = الفترة: { $amount } س

# Parameter values in a card summary. $count selects the plural, $amount is the formatted number.
strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
        [zero] { $amount } ساعة
        [one] { $amount } ساعة
        [two] { $amount } ساعتان
        [few] { $amount } ساعات
        [many] { $amount } ساعة
       *[other] { $amount } ساعة
    }
strategies-value-candles =
    { $count ->
        [zero] { $amount } شمعة
        [one] { $amount } شمعة
        [two] { $amount } شمعتان
        [few] { $amount } شموع
        [many] { $amount } شمعة
       *[other] { $amount } شمعة
    }

# Text written beside a numeric input.
strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = س
strategies-unit-multiplier = ×

## Condition catalog

strategies-catalog-search =
    .placeholder = البحث في الشروط...
strategies-catalog-search-clear =
    .aria-label = مسح البحث
strategies-catalog-fold-all = طيّ الكل
strategies-catalog-unfold-all = فتح الكل
strategies-catalog-no-description = لا يوجد وصف متاح

## New strategy dialog

strategies-create-title = إنشاء استراتيجية جديدة
strategies-create-prompt = اختر نوع الاستراتيجية التي تريد إنشاءها:
strategies-create-entry-name = استراتيجية دخول
strategies-create-entry-description = حدّد شروط شراء الرمز
strategies-create-exit-name = استراتيجية خروج
strategies-create-exit-description = حدّد شروط بيع الرمز

## Delete dialog

strategies-delete-title = حذف الاستراتيجية
# $name is the strategy name.
strategies-delete-message = هل تريد حذف الاستراتيجية «{ $name }»؟ لا يمكن التراجع عن هذا الإجراء.

## Toasts. A message value is the title; `.message` is the body.

strategies-toast-fix-validation = يرجى إصلاح أخطاء التحقق قبل الحفظ
strategies-toast-enabled = تم تفعيل الاستراتيجية
    .message = تم تفعيل «{ $name }»
strategies-toast-disabled = تم تعطيل الاستراتيجية
    .message = تم تعطيل «{ $name }»
strategies-toast-toggle-failed = فشل التبديل
    .message = فشل تحديث حالة الاستراتيجية
strategies-toast-load-failed = فشل التحميل
    .message = فشل تحميل الاستراتيجيات من الخادم
strategies-toast-load-strategy-failed = فشل تحميل الاستراتيجية
strategies-toast-no-strategy = لم يتم إنشاء استراتيجية
    .message = أضف شرطًا واحدًا على الأقل أو انقر «استراتيجية جديدة» لإنشاء استراتيجية أولًا
strategies-toast-no-conditions-save = لا توجد شروط
    .message = أضف شرطًا واحدًا على الأقل إلى الاستراتيجية قبل الحفظ
strategies-toast-name-required = الاسم مطلوب
    .message = أدخل اسم الاستراتيجية قبل الحفظ
strategies-toast-saved = تم حفظ الاستراتيجية
    .message = تم حفظ «{ $name }» بنجاح
strategies-toast-save-failed = فشل الحفظ
    .message = فشل حفظ الاستراتيجية في قاعدة البيانات
strategies-toast-no-strategy-validate = لا توجد استراتيجية للتحقق منها
strategies-toast-no-conditions-validate = لا توجد شروط
    .message = أضف شرطًا واحدًا على الأقل قبل التحقق
strategies-toast-valid = الاستراتيجية صالحة
strategies-toast-invalid = الاستراتيجية تحتوي على أخطاء
strategies-toast-validation-failed = فشل التحقق
strategies-toast-item-enabled = تم تفعيل الاستراتيجية
strategies-toast-item-disabled = تم تعطيل الاستراتيجية
strategies-toast-item-toggle-failed = فشل تبديل الاستراتيجية
strategies-toast-deleted = تم حذف الاستراتيجية
    .message = تمت إزالة «{ $name }» بنجاح
strategies-toast-delete-failed = فشل الحذف
    .message = فشل حذف الاستراتيجية من قاعدة البيانات
strategies-toast-imported = تم استيراد الاستراتيجية
strategies-toast-import-failed = فشل استيراد الاستراتيجية
strategies-toast-unknown-condition = شرط غير معروف
    .message = لم يتم العثور على نوع الشرط
strategies-toast-create-first = أنشئ استراتيجية أولًا
    .message = انقر «استراتيجية جديدة» لإنشاء استراتيجية قبل إضافة الشروط
strategies-toast-condition-added = تمت إضافة الشرط
    .message = تمت إضافة { $name } إلى الاستراتيجية


## Conditions

strategies-condition-candle-size = نمط حجم الشمعة
    .description = اكتشاف أنماط شموع محددة: جسم كبير، جسم صغير (دوجي)، ظلال طويلة
strategies-condition-candle-size-param-pattern = نوع النمط
    .description = نمط الشمعة المراد اكتشافه
strategies-condition-candle-size-param-pattern-option-large-body = جسم كبير (حركة قوية)
strategies-condition-candle-size-param-pattern-option-small-body = جسم صغير (دوجي/تردد)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = ظل علوي طويل (رفض)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = ظل سفلي طويل (دعم)
strategies-condition-candle-size-param-threshold = حد الحجم %
    .description = النسبة المئوية للحد المستخدم في اكتشاف النمط

strategies-condition-consecutive-candles = شموع متتالية
    .description = اكتشاف شموع خضراء (صاعدة) أو حمراء (هابطة) متتالية مع مرشح حد أدنى للحجم
strategies-condition-consecutive-candles-param-count = عدد الشموع
    .description = عدد الشموع المتتالية المطلوب
strategies-condition-consecutive-candles-param-direction = اتجاه الشمعة
    .description = لون/اتجاه الشموع المتتالية
strategies-condition-consecutive-candles-param-direction-option-green = خضراء (صاعدة)
strategies-condition-consecutive-candles-param-direction-option-red = حمراء (هابطة)
strategies-condition-consecutive-candles-param-minimum-change = الحد الأدنى للتغير %
    .description = الحد الأدنى لنسبة التغير في كل شمعة (لتصفية التشويش)

strategies-condition-liquidity-level = مستوى سيولة مجمع السيولة
    .description = فحص سيولة مجمع السيولة بـ { -sol } (الدخول: التأكد من كفاية السيولة، الخروج: اكتشاف سحب السيولة)
strategies-condition-liquidity-level-param-threshold = حد السيولة ({ -sol })
    .description = مستوى سيولة مجمع السيولة بـ { -sol }
strategies-condition-liquidity-level-param-comparison = المقارنة
    .description = كيفية مقارنة سيولة مجمع السيولة بالحد
strategies-condition-liquidity-level-param-comparison-option-greater-than = أكبر من (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = أكبر من أو يساوي (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = أصغر من ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = أصغر من أو يساوي (≤)

strategies-condition-position-holding-time = مدة الاحتفاظ بالمركز
    .description = فحص المدة التي احتُفظ فيها بالمركز (لاستراتيجيات الخروج، أي الخروج الزمني)
strategies-condition-position-holding-time-param-hours = الحد الزمني (ساعات)
    .description = المدة بالساعات منذ فتح المركز
strategies-condition-position-holding-time-param-comparison = المقارنة
    .description = كيفية مقارنة عمر المركز بالحد
strategies-condition-position-holding-time-param-comparison-option-greater-than = أقدم من (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = على الأقل (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = أحدث من ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = على الأكثر (≤)

strategies-condition-price-breakout = اختراق السعر
    .description = اكتشاف اختراق السعر لأعلى المقاومة (أعلى سعر في الفترة) أو لأسفل الدعم (أدنى سعر في الفترة)
strategies-condition-price-breakout-param-lookback = فترة الرجوع
    .description = عدد الشموع لتحديد مستوى الدعم/المقاومة
strategies-condition-price-breakout-param-direction = اتجاه الاختراق
    .description = اتجاه الاختراق
strategies-condition-price-breakout-param-direction-option-upward = صعودي (كسر المقاومة)
strategies-condition-price-breakout-param-direction-option-downward = هبوطي (كسر الدعم)
strategies-condition-price-breakout-param-confirmation = التأكيد %
    .description = مدى تجاوز المستوى لتأكيد الاختراق (لتجنب الإشارات الخاطئة)

strategies-condition-price-change-percent = تغير السعر %
    .description = فحص ما إذا تغير السعر بنسبة محددة خلال فترة زمنية
strategies-condition-price-change-percent-param-percentage = حد التغير %
    .description = نسبة تغير السعر المطلوبة للتشغيل (0.1-1000%)
strategies-condition-price-change-percent-param-direction = الاتجاه
    .description = اتجاه حركة السعر
strategies-condition-price-change-percent-param-direction-option-above = ارتفاع (+%)
strategies-condition-price-change-percent-param-direction-option-below = انخفاض (-%)
strategies-condition-price-change-percent-param-direction-option-within = ضمن النطاق (±%)
strategies-condition-price-change-percent-param-time-value = الفترة الزمنية
    .description = قيمة فترة الرجوع (1-3600 للثواني، 1-1440 للدقائق، 1-720 للساعات)
strategies-condition-price-change-percent-param-time-unit = وحدة الوقت
    .description = وحدة الوقت لفترة الرجوع
strategies-condition-price-change-percent-param-time-unit-option-seconds = ثوانٍ
strategies-condition-price-change-percent-param-time-unit-option-minutes = دقائق
strategies-condition-price-change-percent-param-time-unit-option-hours = ساعات

strategies-condition-price-to-ma = السعر مقابل المتوسط المتحرك
    .description = فحص ما إذا كان السعر أعلى من المتوسط المتحرك البسيط أو أدنى منه أو ضمن نطاقه
strategies-condition-price-to-ma-param-period = فترة المتوسط المتحرك
    .description = عدد الشموع المستخدمة في حساب المتوسط المتحرك
strategies-condition-price-to-ma-param-position = الموضع
    .description = موضع السعر بالنسبة إلى المتوسط المتحرك
strategies-condition-price-to-ma-param-position-option-above = فوق المتوسط المتحرك
strategies-condition-price-to-ma-param-position-option-below = تحت المتوسط المتحرك
strategies-condition-price-to-ma-param-position-option-within = ضمن النطاق
strategies-condition-price-to-ma-param-distance = المسافة %
    .description = الحد الأدنى للمسافة عن المتوسط المتحرك (لحالتي فوق/تحت) أو أقصى نطاق (لحالة ضمن)

strategies-condition-volume-spike = طفرة حجم التداول
    .description = اكتشاف طفرات حجم التداول مقارنة بالمتوسط (تدل على زيادة الاهتمام)
strategies-condition-volume-spike-param-lookback = فترة الرجوع
    .description = عدد الشموع لحساب متوسط حجم التداول
strategies-condition-volume-spike-param-multiplier = مضاعف حجم التداول
    .description = كم مرة فوق المتوسط (مثال: 2.0 = 200% من المتوسط)

## Shared by every condition

strategies-condition-param-timeframe = الإطار الزمني
    .description = الإطار الزمني للشموع المراد تحليلها (يُستخدم إطار الاستراتيجية افتراضيًا إذا لم يُحدد)
strategies-condition-timeframe-option-1m = دقيقة واحدة
strategies-condition-timeframe-option-5m = 5 دقائق
strategies-condition-timeframe-option-15m = 15 دقيقة
strategies-condition-timeframe-option-1h = ساعة واحدة
strategies-condition-timeframe-option-4h = 4 ساعات
strategies-condition-timeframe-option-12h = 12 ساعة
strategies-condition-timeframe-option-1d = يوم واحد

## Condition categories

strategies-condition-category-price-analysis = تحليل السعر
strategies-condition-category-candle-patterns = أنماط الشموع
strategies-condition-category-technical-indicators = المؤشرات الفنية
strategies-condition-category-market-context = سياق السوق
strategies-condition-category-position-performance = المركز والأداء
strategies-condition-category-volume-analysis = تحليل حجم التداول

## Validation errors
# Each validation error is a `UiText`; the tokens below name what the message refers to.

strategies-error-missing-parameter = المعامل «{ $field }» مفقود
strategies-error-parameter-type = يجب أن يكون المعامل «{ $field }» من النوع { $expected }
strategies-error-invalid-value = «{ $value }» ليست قيمة صالحة للمعامل «{ $field }»
strategies-error-missing-data = { $data }: غير متاح
strategies-error-no-candle-data = لا توجد بيانات شموع للإطار الزمني { $timeframe }
strategies-error-insufficient-history = السجل غير كافٍ لـ { $indicator }: المتاح { $available } ث والمطلوب { $required } ث
strategies-error-insufficient-candles = الشموع غير كافية لـ { $indicator }: المتاح { $available } والمطلوب { $required }
strategies-error-stale-candle-data = بيانات شموع الإطار الزمني { $timeframe } قديمة: عمرها { $age } ث يتجاوز { $max } ث
strategies-error-invalid-rule-tree = شجرة قواعد غير صالحة: { $reason }
strategies-error-evaluation-timeout = انتهت مهلة تقييم الاستراتيجية بعد { $timeout } ms
strategies-error-invalid-rules = تعذّرت قراءة القواعد: { $reason }

# Parameter names

strategies-error-field-average-volume = متوسط حجم التداول
strategies-error-field-candle-open = افتتاح الشمعة
strategies-error-field-comparison = المقارنة
strategies-error-field-condition-type = نوع الشرط
strategies-error-field-confirmation = التأكيد
strategies-error-field-count = العدد
strategies-error-field-current-price = السعر الحالي
strategies-error-field-direction = الاتجاه
strategies-error-field-distance = المسافة
strategies-error-field-hours = الساعات
strategies-error-field-lookback = فترة الرجوع
strategies-error-field-minimum-change = الحد الأدنى للتغير
strategies-error-field-multiplier = المضاعف
strategies-error-field-pattern = النمط
strategies-error-field-percentage = النسبة المئوية
strategies-error-field-period = الفترة
strategies-error-field-position = الموضع
strategies-error-field-threshold = الحد
strategies-error-field-time-unit = وحدة الوقت
strategies-error-field-time-value = قيمة الوقت
strategies-error-field-timeframe = الإطار الزمني

# Expected parameter types

strategies-error-expected-boolean = منطقي (Boolean)
strategies-error-expected-number = رقمي
strategies-error-expected-string = نصي

# Missing context data

strategies-error-data-current-price = السعر الحالي
strategies-error-data-liquidity-data = بيانات السيولة
strategies-error-data-market-data = بيانات السوق
strategies-error-data-ohlcv-data = بيانات OHLCV
strategies-error-data-position-data = بيانات المركز

# Indicators

strategies-error-indicator-consecutive-candles = الشموع المتتالية
strategies-error-indicator-moving-average = المتوسط المتحرك
strategies-error-indicator-price-breakout = اختراق السعر
strategies-error-indicator-price-change-lookback = فترة رجوع تغير السعر
strategies-error-indicator-volume-spike = طفرة حجم التداول

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = عقدة الفرع تفتقر إلى الشروط
strategies-error-rule-branch-node-missing-operator = عقدة الفرع تفتقر إلى العامل
strategies-error-rule-branch-node-must-have-at-least-one-child = يجب أن تحتوي عقدة الفرع على عقدة فرعية واحدة على الأقل
strategies-error-rule-invalid-rule-tree-structure = بنية شجرة القواعد غير صالحة
strategies-error-rule-leaf-node-missing-condition = عقدة الورقة تفتقر إلى شرط
strategies-error-rule-not-operator-must-have-exactly-one-child = يجب أن يكون للعامل NOT عقدة فرعية واحدة بالضبط
