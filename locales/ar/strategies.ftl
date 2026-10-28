# Strategies page: the strategy list, the condition editor and the condition catalog.
# Condition text is addressed by the keys the schemas carry (src/strategies/conditions/catalog.rs):
#   strategies-condition-<type>                        name, with `.description`
#   strategies-condition-<type>-param-<param>          parameter name, with `.description`
#   strategies-condition-<type>-param-<param>-option-<value>
#   strategies-condition-category-<slug>
#   strategies-condition-param-timeframe / -timeframe-option-<value>   shared by every condition

## Strategy list

strategies-type-entry = دخول
strategies-type-exit = خروج
strategies-list-empty-title = لا توجد استراتيجيات بعد
strategies-list-empty-hint = أنشئ أول استراتيجية لك
strategies-new = استراتيجية جديدة
strategies-import =
    .title = استيراد استراتيجية
    .aria-label = استيراد استراتيجية

# Name given to a strategy before it is saved.
strategies-new-name = استراتيجية جديدة

## Editor

strategies-editor-name =
    .placeholder = اسم الاستراتيجية
strategies-editor-dirty =
    .title = تغييرات غير محفوظة
strategies-action-validate = تحقق
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
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount } ث
strategies-value-duration-minutes = { $amount } د
strategies-value-duration-hours = { $amount } س
strategies-value-duration-days = { $amount } ي
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = وقتك المحلي: { $time }
strategies-hint-price-impact = صفقة بهذا الحجم تحرّك السعر بنحو { $amount }%

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
strategies-unit-hours = ساعة
strategies-unit-multiplier = ×
strategies-unit-utc = UTC

## Condition catalog

strategies-catalog-search =
    .placeholder = البحث في الشروط...
    .aria-label = البحث في الشروط
strategies-catalog-search-clear =
    .aria-label = مسح البحث
strategies-catalog-no-description = لا يوجد وصف متاح

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = تشغيل
strategies-editor-timeframe = الإطار الزمني الافتراضي
    .title = الإطار الزمني للشموع للشروط التي لا تحدد إطارًا خاصًا بها
strategies-editor-order = الترتيب
    .title = ترتيب التقييم من 1 إلى 100؛ الأرقام الأصغر تُقيَّم أولًا
strategies-editor-description = الوصف
strategies-editor-description-field =
    .placeholder = ما تفعله هذه الاستراتيجية
    .aria-label = وصف الاستراتيجية
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = كل استراتيجيات الدخول
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = تدير
strategies-editor-entry-binding-hint = تدير المراكز التي فتحتها استراتيجيات الدخول المحددة. إذا لم تُحدد أي استراتيجية فإنها تدير كل المراكز، بما فيها اليدوية والمنسوخة.
strategies-editor-entry-binding-empty = لا توجد استراتيجية دخول بعد
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id } (محذوفة)
strategies-action-export = تصدير
strategies-action-test = اختبار على رمز
# $token is the tested token's symbol or address.
strategies-test-title = اختبار على { $token }
# $token is the tested token's symbol or address.
strategies-test-running = جارٍ الاختبار على { $token }…
strategies-test-verdict-pass = تتحقق
strategies-test-verdict-fail = لا تتحقق
strategies-test-verdict-unknown = غير معروف

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = الشروط المطلوب تحققها: { $mode }
strategies-group-match = المطلوب تحققه: { $mode }
strategies-match-all = جميعها
strategies-match-any = واحد على الأقل
strategies-match-mode =
    .aria-label = جميع الشروط أو واحد على الأقل
strategies-invert = عكس
    .title = عكس: يتحقق عندما لا يتطابق هذا
strategies-chip-off = متوقف
strategies-chip-inverted = معكوس
strategies-row-switch =
    .title = أوقفه للاحتفاظ به محفوظًا دون تقييم
    .aria-label = تقيّمه هذه الاستراتيجية
strategies-row-expand =
    .aria-label = إظهار المعاملات أو إخفاؤها
strategies-add-group = إضافة مجموعة
# $count is the number of conditions in the group.
strategies-group-delete-confirm = إزالة المجموعة
    .message =
        { $count ->
            [zero] إزالة هذه المجموعة؟
            [one] إزالة هذه المجموعة وشرطها الوحيد؟
            [two] إزالة هذه المجموعة وشرطيها؟
            [few] إزالة هذه المجموعة وشروطها الـ { $count }؟
            [many] إزالة هذه المجموعة وشروطها الـ { $count }؟
           *[other] إزالة هذه المجموعة وشروطها الـ { $count }؟
        }

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems =
    { $count ->
        [zero] لا توجد مشكلات لإصلاحها: { $first }
        [one] مشكلة واحدة تحتاج إلى إصلاح: { $first }
        [two] مشكلتان تحتاجان إلى إصلاح: { $first }
        [few] { $count } مشكلات تحتاج إلى إصلاح: { $first }
        [many] { $count } مشكلة تحتاج إلى إصلاح: { $first }
       *[other] { $count } مشكلة تحتاج إلى إصلاح: { $first }
    }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }، { $field }: { $problem }
strategies-problem-rules-empty = أضف شرطًا واحدًا مفعّلًا على الأقل
strategies-problem-group-empty = تحتاج المجموعة إلى شرط مفعّل واحد على الأقل
strategies-problem-not-applicable = لا يمكن استخدام هذا الشرط في هذا النوع من الاستراتيجيات
strategies-problem-unknown-condition = نوع شرط غير معروف: { $type }
strategies-problem-too-deep = يمكن تداخل القواعد حتى { $max } مستويات كحد أقصى؛ وتُحتسب المجموعة المعكوسة مستوى إضافيًا
strategies-problem-required = أدخل رقمًا
strategies-problem-range = يجب أن تكون القيمة بين { $min } و{ $max }
strategies-problem-invalid-option = اختر أحد الخيارات المعروضة

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = الكل
strategies-catalog-categories =
    .aria-label = الفئات
strategies-catalog-empty = لا يوجد شرط يطابق بحثك
strategies-catalog-exit-only = لاستراتيجيات الخروج فقط
strategies-catalog-entry-only = لاستراتيجيات الدخول فقط
strategies-catalog-candles-note = { $description } يقرأ الشموع فقط.
strategies-data-candles = الشموع
strategies-data-pool = المجمّع
strategies-data-position = المركز
strategies-data-market = نشاط السوق
strategies-data-clock = وقت اليوم

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = صفقة
strategies-value-trades =
    { $count ->
        [zero] { $amount } صفقة
        [one] { $amount } صفقة
        [two] { $amount } صفقتان
        [few] { $amount } صفقات
        [many] { $amount } صفقة
       *[other] { $amount } صفقة
    }

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = مستويات كثيرة جدًا
    .message = يمكن تداخل القواعد حتى { $max } مستويات كحد أقصى
strategies-toast-priority-invalid = أولوية غير صالحة
    .message = أدخل عددًا صحيحًا من 1 إلى 100
strategies-toast-exported = تم تصدير الاستراتيجية
# $name is the strategy name.
strategies-unsaved-confirm = تجاهل التغييرات غير المحفوظة؟
    .message = لدى «{ $name }» تغييرات غير محفوظة.
strategies-unsaved-discard = تجاهل
strategies-import-error-json = الملف ليس JSON صالحًا
# $min and $max are the bounds of the evaluation order.
strategies-import-error-priority = يجب أن تكون الأولوية في الملف عددًا صحيحًا من { $min } إلى { $max }
strategies-import-error-shape = الملف لا يصف استراتيجية
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = يستخدم الملف شرطًا غير موجود في هذا الإصدار: { $type }
strategies-import-error-not-applicable = لا يمكن استخدام «{ $name }» في هذا النوع من الاستراتيجيات

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
strategies-toast-load-failed = فشل التحميل
    .message = فشل تحميل الاستراتيجيات من الخادم
strategies-toast-load-strategy-failed = فشل تحميل الاستراتيجية
strategies-toast-no-strategy = لم يتم إنشاء استراتيجية
    .message = أضف شرطًا واحدًا على الأقل أو انقر «استراتيجية جديدة» لإنشاء استراتيجية أولًا
strategies-toast-name-required = الاسم مطلوب
    .message = أدخل اسم الاستراتيجية قبل الحفظ
strategies-toast-saved = تم حفظ الاستراتيجية
    .message = تم حفظ «{ $name }» بنجاح
strategies-toast-save-failed = فشل الحفظ
    .message = فشل حفظ الاستراتيجية في قاعدة البيانات
strategies-toast-no-strategy-validate = لا توجد استراتيجية للتحقق منها
strategies-toast-no-strategy-test = لا توجد استراتيجية لاختبارها
strategies-toast-test-save-first = احفظ الاستراتيجية قبل اختبارها: يقرأ الاختبار القواعد المحفوظة
strategies-toast-test-failed = فشل الاختبار
    .message = تعذّر اختبار الاستراتيجية
strategies-toast-valid = الاستراتيجية صالحة
strategies-toast-invalid = الاستراتيجية تحتوي على أخطاء
strategies-toast-validation-failed = فشل التحقق
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

## Conditions

strategies-condition-candle-size = نمط حجم الشمعة
    .description = اكتشاف أنماط شموع محددة: جسم كبير، جسم صغير (دوجي)، ظلال طويلة.
strategies-condition-candle-size-param-pattern = نوع النمط
    .description = نمط الشمعة المراد اكتشافه
strategies-condition-candle-size-param-pattern-option-large-body = جسم كبير (حركة قوية)
strategies-condition-candle-size-param-pattern-option-small-body = جسم صغير (دوجي/تردد)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = ظل علوي طويل (رفض)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = ظل سفلي طويل (دعم)
strategies-condition-candle-size-param-threshold = حد الحجم %
    .description = النسبة المئوية للحد المستخدم في اكتشاف النمط

strategies-condition-consecutive-candles = شموع متتالية
    .description = اكتشاف شموع خضراء (صاعدة) أو حمراء (هابطة) متتالية مع مرشح حد أدنى للحجم.
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
    .description = فحص المدة التي احتُفظ فيها بالمركز
strategies-condition-position-holding-time-param-hours = الحد الزمني (ساعات)
    .description = المدة بالساعات منذ فتح المركز
strategies-condition-position-holding-time-param-comparison = المقارنة
    .description = كيفية مقارنة عمر المركز بالحد
strategies-condition-position-holding-time-param-comparison-option-greater-than = أقدم من (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = على الأقل (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = أحدث من ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = على الأكثر (≤)

strategies-condition-price-breakout = اختراق السعر
    .description = اكتشاف اختراق السعر لأعلى المقاومة (أعلى سعر في الفترة) أو لأسفل الدعم (أدنى سعر في الفترة).
strategies-condition-price-breakout-param-lookback = فترة الرجوع
    .description = عدد الشموع لتحديد مستوى الدعم/المقاومة
strategies-condition-price-breakout-param-direction = اتجاه الاختراق
    .description = اتجاه الاختراق
strategies-condition-price-breakout-param-direction-option-upward = صعودي (كسر المقاومة)
strategies-condition-price-breakout-param-direction-option-downward = هبوطي (كسر الدعم)
strategies-condition-price-breakout-param-confirmation = التأكيد %
    .description = مدى تجاوز المستوى لتأكيد الاختراق (لتجنب الإشارات الخاطئة)

strategies-condition-price-change-percent = تغير السعر %
    .description = فحص ما إذا تغير السعر بنسبة محددة خلال فترة زمنية.
strategies-condition-price-change-percent-param-percentage = حد التغير %
    .description = نسبة تغير السعر المطلوبة للتشغيل (0.1-1000%)
strategies-condition-price-change-percent-param-direction = الاتجاه
    .description = اتجاه حركة السعر
strategies-condition-price-change-percent-param-direction-option-above = ارتفاع (+%)
strategies-condition-price-change-percent-param-direction-option-below = انخفاض (-%)
strategies-condition-price-change-percent-param-direction-option-within = ضمن النطاق (±%)
strategies-condition-price-change-percent-param-time-value = فترة الرجوع
    .description = مدى الرجوع لقياس التغير، بالوحدة المختارة داخل الحقل (1-3600 ثانية، 1-1440 دقيقة، 1-720 ساعة)
strategies-condition-price-change-percent-param-time-unit = وحدة الوقت
    .description = وحدة الوقت لفترة الرجوع
strategies-condition-price-change-percent-param-time-unit-option-seconds = ثانية
strategies-condition-price-change-percent-param-time-unit-option-minutes = دقيقة
strategies-condition-price-change-percent-param-time-unit-option-hours = ساعة

strategies-condition-price-to-ma = السعر مقابل المتوسط المتحرك
    .description = فحص ما إذا كان السعر أعلى من المتوسط المتحرك البسيط أو أدنى منه أو ضمن نطاقه.
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
    .description = اكتشاف طفرات حجم التداول مقارنة بالمتوسط (تدل على زيادة الاهتمام).
strategies-condition-volume-spike-param-lookback = فترة الرجوع
    .description = عدد الشموع لحساب متوسط حجم التداول
strategies-condition-volume-spike-param-multiplier = مضاعف حجم التداول
    .description = كم مرة فوق المتوسط (مثال: 2.0 = 200% من المتوسط)

strategies-condition-rsi = RSI (مؤشر القوة النسبية)
    .description = يعمل عندما يكون مؤشر RSI، بتنعيم وايلدر على الشموع المغلقة، أعلى من حد معيّن أو أقل منه.
strategies-condition-rsi-param-period = فترة RSI
    .description = عدد الشموع التي يُحسب عليها المؤشر؛ يحتاج الشرط إلى شمعة أكثر من هذه القيمة
strategies-condition-rsi-param-comparison = المقارنة
    .description = هل يجب أن يكون المؤشر أقل من الحد أم أعلى منه
strategies-condition-rsi-param-comparison-option-below = أقل من (تشبع بيعي)
strategies-condition-rsi-param-comparison-option-above = أعلى من (تشبع شرائي)
strategies-condition-rsi-param-threshold = حد RSI
    .description = مستوى المؤشر من 1 إلى 99 (القيمتان 30 و70 هما المستويان المعتادان للتشبع البيعي والشرائي)

strategies-condition-ma-cross = تقاطع المتوسطات المتحركة
    .description = يعمل عندما يقطع متوسط متحرك سريع متوسطًا بطيئًا خلال آخر عدة شموع.
strategies-condition-ma-cross-param-ma-type = نوع المتوسط
    .description = المتوسطات الأسية تعطي الشموع الحديثة وزنًا أكبر، والبسيطة تعطي كل الشموع وزنًا متساويًا
strategies-condition-ma-cross-param-ma-type-option-ema = أسّي (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = بسيط (SMA)
strategies-condition-ma-cross-param-fast-period = الفترة السريعة
    .description = عدد الشموع في المتوسط السريع
strategies-condition-ma-cross-param-slow-period = الفترة البطيئة
    .description = عدد الشموع في المتوسط البطيء؛ يجب أن يكون أكبر من الفترة السريعة
strategies-condition-ma-cross-param-direction = اتجاه التقاطع
    .description = في أي اتجاه يقطع المتوسط السريع المتوسط البطيء
strategies-condition-ma-cross-param-direction-option-bullish = صاعد (السريع يقطع للأعلى)
strategies-condition-ma-cross-param-direction-option-bearish = هابط (السريع يقطع للأسفل)
strategies-condition-ma-cross-param-within-candles = التقاطع خلال آخر
    .description = كم من آخر الشموع يمكن أن يحتوي التقاطع؛ القيمة 1 تعني أحدث شمعة فقط

strategies-condition-position-profit = ربح المركز
    .description = يعمل عندما يكون الربح غير المحقق للمركز المفتوح أعلى أو أقل من نسبة مئوية من سعر الدخول، مقيسًا بسعر مجمع السيولة.
strategies-condition-position-profit-param-comparison = المقارنة
    .description = هل يجب أن يكون الربح مساويًا للحد أو أكبر منه، أم مساويًا له أو أقل منه
strategies-condition-position-profit-param-comparison-option-above = على الأقل (≥)
strategies-condition-position-profit-param-comparison-option-below = على الأكثر (≤)
strategies-condition-position-profit-param-threshold-pct = حد الربح %
    .description = الربح غير المحقق كنسبة مئوية من سعر الدخول؛ القيم السالبة خسائر

strategies-condition-drawdown-from-peak = التراجع عن القمة
    .description = يعمل عندما ينخفض سعر مجمع السيولة بنسبة مئوية عن أعلى سعر شوهد منذ آخر شراء، بعد أن كانت تلك القمة أعلى من سعر الدخول بما يكفي.
strategies-condition-drawdown-from-peak-param-drop-pct = التراجع عن القمة %
    .description = كم يجب أن يكون السعر الحالي أدنى من القمة
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = التفعيل بعد ربح %
    .description = يجب أن تكون القمة قد ارتفعت عن سعر الدخول بهذه النسبة على الأقل قبل أن يعمل الشرط؛ القيمة 0 تعني أنه مفعّل دائمًا

strategies-condition-buy-pressure = ضغط الشراء
    .description = يعمل عندما تشكّل عمليات الشراء نسبة معينة على الأقل من معاملات نافذة زمنية وتحتوي النافذة على معاملات كافية. يقرأ أعداد الشراء والبيع للعملة.
strategies-condition-buy-pressure-param-window = النافذة الزمنية
    .description = الفترة التي تغطيها أعداد الشراء والبيع
strategies-condition-buy-pressure-param-window-option-5m = 5 دقائق
strategies-condition-buy-pressure-param-window-option-1h = ساعة واحدة
strategies-condition-buy-pressure-param-window-option-6h = 6 ساعات
strategies-condition-buy-pressure-param-window-option-24h = 24 ساعة
strategies-condition-buy-pressure-param-min-buy-share-pct = الحد الأدنى لحصة الشراء %
    .description = عمليات الشراء كنسبة مئوية من كل معاملات النافذة
strategies-condition-buy-pressure-param-min-transactions = الحد الأدنى للمعاملات
    .description = النوافذ ذات المعاملات الأقل لا تُفعّل الشرط أبدًا

strategies-condition-transaction-velocity = سرعة المعاملات
    .description = تتحقق عندما يكون معدل المعاملات في نافذة حديثة على الأقل أو على الأكثر مضاعفًا لمعدل النافذة الأطول التي تحتويها. تقرأ أعداد الشراء والبيع للرمز.
strategies-condition-transaction-velocity-param-window = النوافذ
    .description = النافذة الحديثة والنافذة الأطول التي تُقارن بها
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = 5 دقائق مقابل ساعة
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = ساعة مقابل 6 ساعات
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = 6 ساعات مقابل 24 ساعة
strategies-condition-transaction-velocity-param-comparison = المقارنة
    .description = هل يجب أن يكون المعدل الحديث على الأقل أو على الأكثر النسبة
strategies-condition-transaction-velocity-param-comparison-option-above = على الأقل (≥)
strategies-condition-transaction-velocity-param-comparison-option-below = على الأكثر (≤)
strategies-condition-transaction-velocity-param-ratio = نسبة المعدل
    .description = المعاملات في الثانية في النافذة الحديثة مقسومة على المعاملات في الثانية في النافذة الطويلة
strategies-condition-transaction-velocity-param-min-transactions = الحد الأدنى للمعاملات
    .description = النوافذ الحديثة ذات المعاملات الأقل لا تحقق الشرط أبدًا

strategies-condition-pullback-from-high = التراجع من القمة
    .description = تتحقق عندما يقع السعر الحي بين هبوط أدنى وهبوط أقصى تحت أعلى قمة لآخر الشموع.
strategies-condition-pullback-from-high-param-lookback = فترة الرجوع
    .description = عدد الشموع التي تؤخذ منها أعلى قمة
strategies-condition-pullback-from-high-param-min-drop-pct = الهبوط الأدنى %
    .description = يجب أن يكون السعر تحت القمة بهذا القدر على الأقل
strategies-condition-pullback-from-high-param-max-drop-pct = الهبوط الأقصى %
    .description = يجب أن يكون السعر تحت القمة بهذا القدر على الأكثر؛ يجب أن يكون أكبر من الهبوط الأدنى

strategies-condition-profit-giveback = التنازل عن الربح
    .description = تتحقق عندما يكون المركز قد تنازل عن حصة من أفضل ربح غير محقق منذ آخر شراء، بعد أن بلغ ذلك الربح مستوى معينًا.
strategies-condition-profit-giveback-param-arm-after-profit-pct = التفعيل بعد ربح %
    .description = يجب أن يبلغ أفضل ربح هذه النسبة من سعر الدخول قبل أن يمكن للشرط أن يتحقق
strategies-condition-profit-giveback-param-giveback-pct = التنازل %
    .description = حصة أفضل ربح التي تم التنازل عنها؛ 100% تعني أن السعر عاد إلى سعر الدخول

strategies-condition-token-age = عمر الرمز
    .description = تتحقق عندما تكون المدة منذ إنشاء الرمز على السلسلة أطول أو أقصر من مدة محددة. الرمز ذو وقت الإنشاء المجهول يترك الشرط غير محسوم.
strategies-condition-token-age-param-comparison = المقارنة
    .description = هل يجب أن يكون الرمز أقدم أو أحدث من المدة
strategies-condition-token-age-param-comparison-option-older-than = أقدم من (≥)
strategies-condition-token-age-param-comparison-option-younger-than = أحدث من ({ "<" })
strategies-condition-token-age-param-time-value = العمر
    .description = الوقت منذ إنشاء الرمز، بالوحدة المختارة داخل الحقل (1-10000)
strategies-condition-token-age-param-time-unit = وحدة الوقت
    .description = وحدة العمر
strategies-condition-token-age-param-time-unit-option-minutes = دقيقة
strategies-condition-token-age-param-time-unit-option-hours = ساعة
strategies-condition-token-age-param-time-unit-option-days = يوم

strategies-condition-vwap = VWAP
    .description = تتحقق عندما يكون السعر الحي فوق أو تحت متوسط السعر المرجح بالحجم لآخر الشموع بمسافة على الأقل.
strategies-condition-vwap-param-lookback = فترة الرجوع
    .description = عدد الشموع التي يغطيها المتوسط المرجح بالحجم
strategies-condition-vwap-param-position = الموضع
    .description = موضع السعر بالنسبة إلى VWAP
strategies-condition-vwap-param-position-option-above = فوق VWAP
strategies-condition-vwap-param-position-option-below = تحت VWAP
strategies-condition-vwap-param-distance-pct = المسافة %
    .description = أدنى مسافة عن VWAP؛ 0 تعني أي مسافة في الجهة المختارة

strategies-condition-volatility-range = نطاق التقلب
    .description = تتحقق عندما يكون متوسط المدى الحقيقي لآخر الشموع، كنسبة من السعر الحي، فوق أو تحت حد معين. اثنان منها في مجموعة يكوّنان نطاقًا.
strategies-condition-volatility-range-param-period = فترة ATR
    .description = عدد الشموع التي يُنعَّم عليها متوسط المدى الحقيقي؛ يحتاج الشرط إلى شمعة إضافية
strategies-condition-volatility-range-param-comparison = المقارنة
    .description = هل يجب أن يكون المدى فوق أو تحت الحد
strategies-condition-volatility-range-param-comparison-option-above = فوق
strategies-condition-volatility-range-param-comparison-option-below = تحت
strategies-condition-volatility-range-param-threshold-pct = حد المدى %
    .description = متوسط المدى الحقيقي كنسبة من السعر الحي

strategies-condition-bollinger-bands = نطاقات بولينجر
    .description = تتحقق عندما يكون السعر الحي تحت النطاق السفلي أو فوق النطاق العلوي، أو عندما تنضغط النطاقات. تقع النطاقات على بعد عدد من الانحرافات المعيارية حول متوسط الإغلاق.
strategies-condition-bollinger-bands-param-period = الفترة
    .description = عدد الشموع التي يغطيها المتوسط والانحراف المعياري
strategies-condition-bollinger-bands-param-std-dev = الانحرافات المعيارية
    .description = كم انحرافًا معياريًا يبعد كل نطاق عن المتوسط
strategies-condition-bollinger-bands-param-signal = الإشارة
    .description = ما يجب أن تظهره النطاقات
strategies-condition-bollinger-bands-param-signal-option-below-lower = تحت النطاق السفلي
strategies-condition-bollinger-bands-param-signal-option-above-upper = فوق النطاق العلوي
strategies-condition-bollinger-bands-param-signal-option-squeeze = انضغاط
strategies-condition-bollinger-bands-param-squeeze-width-pct = عرض الانضغاط %
    .description = للانضغاط: أكبر مسافة بين النطاقين، كنسبة من المتوسط

strategies-condition-liquidity-to-size = السيولة مقابل الحجم
    .description = تتحقق عندما تكون احتياطيات المجمع على الأقل أو أقل من مضاعف للحجم المعني: حجم الصفقة في استراتيجية الدخول، والقيمة المتبقية للمركز في استراتيجية الخروج.
strategies-condition-liquidity-to-size-param-comparison = المقارنة
    .description = هل يجب أن تكون الاحتياطيات على الأقل أو أقل من المضاعف
strategies-condition-liquidity-to-size-param-comparison-option-at-least = على الأقل (≥)
strategies-condition-liquidity-to-size-param-comparison-option-below = أقل من ({ "<" })
strategies-condition-liquidity-to-size-param-multiple = مضاعف السيولة
    .description = احتياطيات المجمع مقسومة على الحجم المعني؛ المضاعف N يحرّك السعر بنحو 1/(N+1)

strategies-condition-position-adds = إضافات المركز
    .description = تتحقق عندما يكون عدد المشتريات المضافة بعد الدخول الأول، أو عمليات البيع الجزئي، على الأقل أو على الأكثر عددًا معينًا.
strategies-condition-position-adds-param-subject = المعدود
    .description = الإجراءات التي تُعد
strategies-condition-position-adds-param-subject-option-adds = المشتريات المضافة
strategies-condition-position-adds-param-subject-option-partial-sells = عمليات البيع الجزئي
strategies-condition-position-adds-param-comparison = المقارنة
    .description = هل يجب أن يكون العدد على الأقل أو على الأكثر الرقم
strategies-condition-position-adds-param-comparison-option-at-least = على الأقل (≥)
strategies-condition-position-adds-param-comparison-option-at-most = على الأكثر (≤)
strategies-condition-position-adds-param-count = العدد
    .description = عدد الإجراءات

strategies-condition-time-of-day = وقت اليوم
    .description = تتحقق عندما تقع الساعة الحالية ضمن نافذة من ساعات UTC. البداية بعد النهاية تمتد عبر منتصف الليل.
strategies-condition-time-of-day-param-start-hour = ساعة البداية (UTC)
    .description = أول ساعة في النافذة، من 0 إلى 23
strategies-condition-time-of-day-param-end-hour = ساعة النهاية (UTC)
    .description = الساعة التي تنتهي عندها النافذة، من 1 إلى 24؛ تلك الساعة نفسها خارج النافذة

strategies-condition-liquidity-change-since-entry = تغير السيولة منذ الدخول
    .description = تتحقق عندما تنخفض احتياطيات المجمع أو ترتفع بنسبة مئوية على الأقل منذ فتح المركز. المركز الذي لا توجد له سيولة دخول مسجلة يترك الشرط غير محسوم.
strategies-condition-liquidity-change-since-entry-param-comparison = المقارنة
    .description = هل يجب أن تكون الاحتياطيات قد انخفضت أو ارتفعت
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = انخفضت بمقدار
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = ارتفعت بمقدار
strategies-condition-liquidity-change-since-entry-param-change-pct = التغير %
    .description = تغير احتياطيات المجمع منذ الدخول

## Shared by every condition

strategies-condition-param-timeframe = الإطار الزمني
    .description = حجم الشمعة: مدة كل شمعة يقرؤها الشرط (إطار الاستراتيجية إذا لم يُحدد)
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
strategies-error-stale-market-data = أعداد المعاملات للنافذة { $window } قديمة: عمرها { $age } ث يتجاوز { $max } ث
strategies-error-unusable-data = لا يمكن استخدام قيمة «{ $field }» «{ $value }»
strategies-error-invalid-rule-tree = شجرة قواعد غير صالحة: { $reason }
strategies-error-evaluation-timeout = انتهت مهلة تقييم الاستراتيجية بعد { $timeout } ms
strategies-error-condition-not-applicable = لا يمكن استخدام { $condition } في استراتيجيات من نوع { $strategy }
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
strategies-error-field-arm-after-profit = التفعيل بعد الربح
strategies-error-field-drop-percentage = نسبة التراجع
strategies-error-field-entry-price = سعر الدخول
strategies-error-field-fast-period = الفترة السريعة
strategies-error-field-minimum-buy-share = الحد الأدنى لحصة الشراء
strategies-error-field-minimum-transactions = الحد الأدنى للمعاملات
strategies-error-field-moving-average-type = نوع المتوسط
strategies-error-field-moving-average = المتوسط المتحرك
strategies-error-field-reference-candle-close = إغلاق الشمعة المرجعية
strategies-error-field-peak-price = سعر القمة
strategies-error-field-slow-period = الفترة البطيئة
strategies-error-field-window = النافذة
strategies-error-field-within-candles = عدد شموع التقاطع
strategies-error-field-change = التغير
strategies-error-field-end-hour = ساعة النهاية
strategies-error-field-entry-liquidity = سيولة الدخول
strategies-error-field-giveback = التنازل
strategies-error-field-highest-high = أعلى قمة
strategies-error-field-long-window-transactions = معاملات النافذة الطويلة
strategies-error-field-maximum-drop = الهبوط الأقصى
strategies-error-field-minimum-drop = الهبوط الأدنى
strategies-error-field-multiple = المضاعف
strategies-error-field-pool-liquidity = سيولة المجمع
strategies-error-field-ratio = النسبة
strategies-error-field-remaining-position-value = القيمة المتبقية للمركز
strategies-error-field-signal = الإشارة
strategies-error-field-squeeze-width = عرض الانضغاط
strategies-error-field-standard-deviations = الانحرافات المعيارية
strategies-error-field-start-hour = ساعة البداية
strategies-error-field-subject = المعدود
strategies-error-field-token-creation-time = وقت إنشاء الرمز
strategies-error-field-total-volume = الحجم الكلي
strategies-error-field-trade-size = حجم الصفقة
strategies-error-field-vwap = VWAP

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
strategies-error-data-peak-price = سعر القمة
strategies-error-data-transaction-counts = أعداد المعاملات
strategies-error-data-entry-liquidity = سيولة الدخول
strategies-error-data-entry-pool-liquidity = سيولة مجمع الدخول
strategies-error-data-real-pool-reserves = السيولة الفعلية للمجمع
strategies-error-data-remaining-position-value = القيمة المتبقية للمركز
strategies-error-data-token-creation-time = وقت إنشاء الرمز
strategies-error-data-trade-size = حجم الصفقة

# Indicators

strategies-error-indicator-consecutive-candles = الشموع المتتالية
strategies-error-indicator-moving-average = المتوسط المتحرك
strategies-error-indicator-price-breakout = اختراق السعر
strategies-error-indicator-price-change-lookback = فترة رجوع تغير السعر
strategies-error-indicator-volume-spike = طفرة حجم التداول
strategies-error-indicator-moving-average-cross = تقاطع المتوسطات المتحركة
strategies-error-indicator-rsi = RSI
strategies-error-indicator-bollinger-bands = نطاقات بولينجر
strategies-error-indicator-pullback-from-high = التراجع من القمة
strategies-error-indicator-volatility-range = نطاق التقلب
strategies-error-indicator-vwap = VWAP

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = عقدة الفرع تفتقر إلى الشروط
strategies-error-rule-branch-node-missing-operator = عقدة الفرع تفتقر إلى العامل
strategies-error-rule-branch-node-must-have-at-least-one-child = يجب أن تحتوي عقدة الفرع على عقدة فرعية واحدة على الأقل
strategies-error-rule-invalid-rule-tree-structure = بنية شجرة القواعد غير صالحة
strategies-error-rule-leaf-node-missing-condition = عقدة الورقة تفتقر إلى شرط
strategies-error-rule-not-operator-must-have-exactly-one-child = يجب أن يكون للعامل NOT عقدة فرعية واحدة بالضبط
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = تحتاج المجموعة إلى شرط واحد مفعّل على الأقل
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = لا يمكن أن يكون للعامل NOT عقدة فرعية معطّلة
strategies-error-rule-rules-nest-deeper-than-four-levels = لا يمكن تداخل القواعد لأكثر من أربعة مستويات
strategies-error-rule-the-root-group-cannot-be-switched-off = لا يمكن تعطيل المجموعة الجذرية

strategies-readiness-history-since-met =
    { $count ->
        [zero] لا جولات تجريبية مغلقة ({ $count }) منذ تغيير القواعد في { $changed }، المطلوب: { $needed }
        [one] جولة تجريبية مغلقة واحدة ({ $count }) منذ تغيير القواعد في { $changed }، المطلوب: { $needed }
        [two] جولتان تجريبيتان مغلقتان ({ $count }) منذ تغيير القواعد في { $changed }، المطلوب: { $needed }
        [few] { $count } جولات تجريبية مغلقة منذ تغيير القواعد في { $changed }، المطلوب: { $needed }
        [many] { $count } جولة تجريبية مغلقة منذ تغيير القواعد في { $changed }، المطلوب: { $needed }
       *[other] { $count } جولة تجريبية مغلقة منذ تغيير القواعد في { $changed }، المطلوب: { $needed }
    }
strategies-readiness-history-since-short = الجولات التجريبية المغلقة منذ تغيير القواعد في { $changed }: { $count } من { $needed }
strategies-readiness-priced = كل المراكز مسعّرة
strategies-readiness-priced-ok = لكل مركز تجريبي مفتوح سعر مجمع
strategies-readiness-priced-missing =
    { $count ->
        [zero] لا مراكز تجريبية مفتوحة بلا سعر مجمع ({ $count })
        [one] مركز تجريبي مفتوح واحد بلا سعر مجمع ({ $count })
        [two] مركزان تجريبيان مفتوحان بلا سعر مجمع ({ $count })
        [few] { $count } مراكز تجريبية مفتوحة بلا سعر مجمع
        [many] { $count } مركزًا تجريبيًا مفتوحًا بلا سعر مجمع
       *[other] { $count } مركز تجريبي مفتوح بلا سعر مجمع
    }
strategies-readiness-runtime = التداول الحي متاح
strategies-readiness-runtime-ok = الإعداد وبوابات الأمان تسمح بالصفقات الحية

## Roster: the strategy list, one view per kind. Status and notice ids come from
## src/strategies/overview.rs.

strategies-back = كل الاستراتيجيات
strategies-view-kind = نوع الاستراتيجية
strategies-view-all = الكل
strategies-search-placeholder = البحث في الاستراتيجيات...
strategies-unnamed = استراتيجية بلا اسم
strategies-col-on = تشغيل
strategies-col-strategy = الاستراتيجية
strategies-col-rank = الرتبة
    .title = رتبة التقييم بين استراتيجيات الدخول المفعّلة في الوضع نفسه، من أدنى ترتيب صعودًا؛ أول استراتيجية تعطي إشارة تأخذ الرمز
strategies-col-mode = الوضع
strategies-col-status = الحالة
strategies-col-open = مفتوحة
strategies-col-watching = قيد المتابعة
strategies-col-kind = النوع
strategies-col-rounds = الجولات
strategies-col-closed = مغلقة
strategies-col-win-rate = نسبة الربح
strategies-col-realized = المحقق ({ -sol })
strategies-col-manages = تدير
strategies-col-last-signal = آخر إشارة
strategies-status-running = تعمل
strategies-status-not-running = لا تعمل
strategies-status-off = متوقفة
strategies-status-setup-required = يلزم الإعداد
strategies-status-live-paused = الدخول الحي متوقف مؤقتًا
strategies-status-paper-cash = الرصيد التجريبي لا يكفي
strategies-status-slots-full = المراكز ممتلئة
strategies-status-entry-monitor-off = مراقب الدخول متوقف
strategies-status-exit-monitor-off = مراقب الخروج متوقف
strategies-notice-setup-required = يبدأ التداول التجريبي بعد الإعداد
strategies-notice-force-stopped = الإيقاف الطارئ مفعّل
strategies-notice-trader-off = Auto Trader متوقف
strategies-notice-waiting = بانتظار الخدمات الأساسية
strategies-notice-monitors-off = مراقبا الدخول والخروج متوقفان
strategies-notice-start = تشغيل Auto Trader
# $closed and $needed count closed paper rounds; $check is the title of a readiness check.
strategies-readiness-progress = الجولات { $closed }/{ $needed }
strategies-readiness-ready = جاهزة للتداول الحي
strategies-readiness-failing = فحص لم يتحقق: { $check }
# $book is the trading mode of the other book; $count its open positions.
strategies-open-other-book =
    { $count ->
        [zero] { $book }: { $count } مركز
        [one] { $book }: { $count } مركز
        [two] { $book }: { $count } مركزان
        [few] { $book }: { $count } مراكز
        [many] { $book }: { $count } مركزًا
       *[other] { $book }: { $count } مركز
    }
strategies-manages-all = كل استراتيجيات الدخول
# $first is the first managed entry strategy; $more the count of the others.
strategies-manages-some =
    { $more ->
        [zero] { $first } و{ $more } أخرى
        [one] { $first } و{ $more } أخرى
        [two] { $first } و{ $more } أخرى
        [few] { $first } و{ $more } أخرى
        [many] { $first } و{ $more } أخرى
       *[other] { $first } و{ $more } أخرى
    }
strategies-exit-before = تُفحص أولًا
strategies-exit-hint = تُفحص قواعد الخروج هذه في كل مركز قبل أي استراتيجية خروج
strategies-action-edit = تعديل { $name }
strategies-action-delete = حذف { $name }
strategies-roster-stale = آخر تحديث

## Saving new rules: the server answers whether a save changed the rules a strategy trades on.

strategies-live-rules-confirm = تغيير قواعد «{ $name }» الحية
    .message = تتداول القواعد الجديدة بـ { -sol } حقيقية من الفحص التالي، ويبدأ الدليل التجريبي لهذه الاستراتيجية من جديد من هذا الحفظ.
strategies-live-rules-save = حفظ والتداول الحي
strategies-toast-evidence-restarted = تم حفظ الاستراتيجية
    .message = تغيّرت قواعد «{ $name }»؛ يبدأ دليلها التجريبي من جديد الآن

## Trading mode: arming a paper strategy live and returning it to paper.

strategies-mode-arm = تفعيل التداول الحي
strategies-mode-return-paper = العودة إلى التجريبي
strategies-mode-arm-title = تفعيل «{ $name }» للتداول الحي
strategies-mode-arm-lead = الصفقات الجديدة لـ«{ $name }» ستُنفَّذ بمبادلات حقيقية من محفظة التداول الخاصة بك.
strategies-mode-arm-lead-exit = ستبيع «{ $name }» المراكز الحية بمبادلات حقيقية من محفظة التداول عند إشارتها.
strategies-mode-arm-readiness = الجاهزية من السجل التجريبي
strategies-mode-arm-open-paper = المراكز التجريبية المفتوحة تبقى تجريبية وتُغلق تجريبيًا.
strategies-mode-arm-ack-real-native = الصفقات الجديدة تنفق { -sol } حقيقية من محفظة التداول الخاصة بك.
strategies-mode-arm-ack-real-native-exit = تُباع المراكز الحية التي تديرها بمبادلات حقيقية.
strategies-mode-arm-ack-fees = المبادلات الحية تدفع رسوم شبكة وانزلاقًا سعريًا حقيقيين؛ والنتائج التجريبية لا تضمن النتائج الحية.
strategies-mode-arm-ack-unready = بعض فحوصات الجاهزية لم تنجح. فعّل هذه الاستراتيجية رغم ذلك.
strategies-mode-arm-unavailable = التداول الحي غير متاح الآن؛ راجع آخر فحص.
strategies-mode-return-title = إعادة «{ $name }» إلى التجريبي
strategies-mode-return-message = الصفقات الجديدة تُفتح في السجل التجريبي دون إنفاق { -sol }. المراكز الحية المفتوحة تبقى حية وتُغلق حيًا.
strategies-mode-return-message-exit = تتوقف عن تقييم المراكز الحية؛ وتبقى لها قواعد الخروج الخاصة بها.
strategies-mode-readiness-failed = تعذّرت قراءة جاهزية الاستراتيجية
strategies-mode-toast-armed = «{ $name }» يتداول الآن حيًا
strategies-mode-toast-paper = «{ $name }» يتداول الآن تجريبيًا
strategies-mode-toast-failed = تعذّر تغيير وضع التداول
