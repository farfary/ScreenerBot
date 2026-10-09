# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = وقف الخسارة
trader-exit-type-take-profit = جني الأرباح
trader-exit-type-roi = هدف العائد
trader-exit-type-roi-exit = هدف العائد
trader-exit-type-trailing-stop = وقف متحرك
trader-exit-type-time-override = تجاوز زمني
trader-exit-type-time-rule = قاعدة زمنية
trader-exit-type-manual = يدوي
trader-exit-type-manual-close = يدوي
trader-exit-type-dca = DCA
trader-exit-type-unknown = غير معروف

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = الإحصاءات
trader-tab-strategy-control = التحكم بالاستراتيجيات
trader-tab-strategies = الاستراتيجيات
trader-tab-stop-loss = وقف الخسارة
trader-tab-trailing-stop = وقف متحرك
trader-tab-roi = جني الأرباح
trader-tab-time-rules = القواعد الزمنية
trader-tab-dca = DCA
trader-tab-settings = الإعدادات

## Feature status badges and their messages

trader-feature-coming-soon = قريبًا
    .message = هذه الميزة قادمة قريبًا وغير متاحة بعد.
trader-feature-beta = تجريبي
trader-feature-disabled = معطّلة
    .message = هذه الميزة معطّلة حاليًا.

## Status bar and trading controls

trader-status-title = المتداول الآلي
trader-status-loading = جارٍ التحميل...
trader-status-running = قيد التشغيل
trader-status-stopped = متوقف
trader-status-setup-required = الإعداد مطلوب
trader-status-unavailable = أكمل إعداد المحفظة وRPC لاستخدام المتداول الآلي
trader-toggle-on = مفعّل
trader-toggle-off = معطّل
trader-toggle-unavailable = غير متاح
trader-toggle-start-failed = فشل تشغيل المتداول
trader-toggle-stop-failed = فشل إيقاف المتداول
trader-controls-title = عناصر التحكم بالتداول
trader-halt-title = التداول متوقف
trader-halt-reason-default = إيقاف قسري يدوي
trader-halt-resume = استئناف
trader-monitor-entry = مراقب الدخول
trader-monitor-exit = مراقب الخروج
trader-monitor-master-off = المتداول الآلي معطّل
trader-loss-limit-title = حد خسارة الفترة
trader-loss-limit-resume = استئناف التداول
trader-loss-limit-reset = إعادة ضبط الفترة
trader-loss-limit-off = معطّل
trader-loss-limit-none = لم يتم ضبط حد خسارة للفترة
# $hours and $minutes are formatted spans such as "2h" and "5m".
trader-loss-limit-resets-in = تتم إعادة الضبط خلال { $hours } { $minutes }
trader-loss-limit-reached = تم بلوغ الحد
trader-force-stop = إيقاف كل شيء قسريًا

## Confirmations. `.message` is the body and `.confirm` the confirming button.

trader-force-stop-confirm = إيقاف التداول قسريًا
    .message = سيؤدي ذلك إلى إيقاف كل عمليات التداول فورًا. هل تريد المتابعة؟
    .confirm = إيقاف التداول
trader-loss-limit-resume-confirm = الاستئناف بعد حد الخسارة
    .message = أوقف حد خسارة الفترة الدخولات الجديدة. الاستئناف يتيح للمتداول فتح مراكز مجددًا قبل إعادة ضبط الفترة. هل تريد المتابعة؟
trader-loss-limit-reset-confirm = إعادة ضبط فترة حد الخسارة
    .message = سيؤدي ذلك إلى مسح الخسارة المتراكمة للفترة الحالية وبدء فترة جديدة. هل تريد المتابعة؟

## Toasts

trader-toast-control-failed = فشل التحكم بالمتداول الآلي
trader-toast-force-stop-on = تم تفعيل الإيقاف القسري
trader-toast-force-stop-failed = تعذّر تفعيل الإيقاف القسري
trader-toast-force-stop-cleared = تم إلغاء الإيقاف القسري
trader-toast-resume-failed = تعذّر استئناف التداول
trader-toast-loss-limit-reset-failed = تعذّرت إعادة ضبط حد الخسارة
trader-toast-entry-monitor-failed = تعذّر تبديل مراقب الدخول
trader-toast-exit-monitor-failed = تعذّر تبديل مراقب الخروج
trader-toast-load-failed = فشل التحميل
    .message = فشل تحميل إعدادات المتداول
trader-toast-saved = تم حفظ الإعدادات
    .message = تم تطبيق إعدادات المتداول بنجاح
trader-toast-save-failed = فشل الحفظ
    .message = فشل حفظ إعدادات المتداول
trader-toast-feature-enabled = تم تفعيل الميزة
trader-toast-feature-disabled = تم تعطيل الميزة
trader-toast-feature-applied = تم تطبيق إعداد المتداول الآلي
trader-toast-strategy-enabled = تم تفعيل الاستراتيجية
    .message = الاستراتيجية نشطة
trader-toast-strategy-disabled = تم تعطيل الاستراتيجية
    .message = الاستراتيجية غير نشطة
trader-toast-strategy-failed = فشل التحديث
    .message = فشل تحديث حالة الاستراتيجية

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = نافذة الإحصاءات
trader-stats-window-day = 24H
trader-stats-window-week = 7D
trader-stats-window-month = 30D
trader-realized-title = الأداء المحقق
trader-metric-net-pnl = صافي الأرباح والخسائر
trader-metric-win-rate = نسبة الربح
trader-metric-profit-factor = عامل الربح
trader-metric-max-drawdown = أقصى تراجع
trader-metric-capital = رأس المال العامل
trader-metric-avg-win-loss = متوسط الربح / الخسارة
trader-metric-closed-trades = الصفقات المغلقة
trader-metric-median-hold = وسيط مدة الاحتفاظ
trader-stats-empty = لا توجد صفقات مغلقة في هذه النافذة
# $won and $lost are formatted SOL amounts.
trader-stats-won-lost = ربح { $won } · خسارة { $lost }
# $wins and $losses are the plural messages below.
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
        [zero] { $amount } ربح
        [one] { $amount } ربح
        [two] { $amount } ربحان
        [few] { $amount } أرباح
        [many] { $amount } ربحًا
       *[other] { $amount } ربح
    }
trader-stats-losses =
    { $count ->
        [zero] { $amount } خسارة
        [one] { $amount } خسارة
        [two] { $amount } خسارتان
        [few] { $amount } خسائر
        [many] { $amount } خسارة
       *[other] { $amount } خسارة
    }
# $amount is a formatted SOL amount.
trader-stats-expected = المتوقع لكل صفقة: { $amount }
trader-stats-profit-factor-basis = إجمالي الربح ÷ إجمالي الخسارة
trader-stats-drawdown-basis = أعمق تراجع محقق من القمة إلى القاع
# $count is the position limit and selects the plural.
trader-stats-slots =
    { $count ->
        [zero] خانات المراكز المستخدمة: { $used } من { $max }
        [one] خانات المراكز المستخدمة: { $used } من { $max }
        [two] خانات المراكز المستخدمة: { $used } من { $max }
        [few] خانات المراكز المستخدمة: { $used } من { $max }
        [many] خانات المراكز المستخدمة: { $used } من { $max }
       *[other] خانات المراكز المستخدمة: { $used } من { $max }
    }
trader-stats-avg-basis = متوسط نتيجة الصفقة الرابحة مقابل الخاسرة
trader-stats-closed =
    { $count ->
        [zero] مراكز مغلقة: { $amount }
        [one] مراكز مغلقة: { $amount }
        [two] مراكز مغلقة: { $amount }
        [few] مراكز مغلقة: { $amount }
        [many] مراكز مغلقة: { $amount }
       *[other] مراكز مغلقة: { $amount }
    }
# $span is a formatted duration.
trader-stats-hold-average = المتوسط { $span }
trader-stats-excluded =
    { $count ->
        [zero] جولات مغلقة مستبعدة: { $amount }. لا توجد تكلفة أساس كاملة، لذا لا توجد أرباح وخسائر موثوقة.
        [one] جولات مغلقة مستبعدة: { $amount }. لا توجد تكلفة أساس كاملة، لذا لا توجد أرباح وخسائر موثوقة.
        [two] جولات مغلقة مستبعدة: { $amount }. لا توجد تكلفة أساس كاملة، لذا لا توجد أرباح وخسائر موثوقة.
        [few] جولات مغلقة مستبعدة: { $amount }. لا توجد تكلفة أساس كاملة، لذا لا توجد أرباح وخسائر موثوقة.
        [many] جولات مغلقة مستبعدة: { $amount }. لا توجد تكلفة أساس كاملة، لذا لا توجد أرباح وخسائر موثوقة.
       *[other] جولات مغلقة مستبعدة: { $amount }. لا توجد تكلفة أساس كاملة، لذا لا توجد أرباح وخسائر موثوقة.
    }

## Stats: daily P&L and extremes

trader-daily-title = الأرباح والخسائر اليومية
trader-daily-subtitle = { -sol } المحققة يوميًا مع الإجمالي التراكمي
trader-daily-loading = جارٍ تحميل الأرباح والخسائر اليومية...
trader-daily-chart = الأرباح والخسائر اليومية المحققة بـ { -sol }
trader-extreme-best = أفضل صفقة
trader-extreme-worst = أسوأ صفقة

## Stats: exit breakdown

trader-exit-title = تفصيل استراتيجيات الخروج
trader-exit-subtitle = كيف أُغلقت المراكز وما عاد به كل خروج
trader-exit-loading = جارٍ تحميل بيانات الخروج...
trader-exit-empty-day = لا توجد صفقات مغلقة خلال آخر 24 ساعة
trader-exit-empty-days =
    { $count ->
        [zero] لا توجد صفقات مغلقة خلال آخر { $amount } يوم
        [one] لا توجد صفقات مغلقة خلال آخر { $amount } يوم
        [two] لا توجد صفقات مغلقة خلال آخر { $amount } يومين
        [few] لا توجد صفقات مغلقة خلال آخر { $amount } أيام
        [many] لا توجد صفقات مغلقة خلال آخر { $amount } يومًا
       *[other] لا توجد صفقات مغلقة خلال آخر { $amount } يومًا
    }
# $share is a formatted percentage of all exits.
trader-exit-share =
    { $count ->
        [zero] { $amount } صفقة · { $share } من عمليات الخروج
        [one] { $amount } صفقة · { $share } من عمليات الخروج
        [two] { $amount } صفقتان · { $share } من عمليات الخروج
        [few] { $amount } صفقات · { $share } من عمليات الخروج
        [many] { $amount } صفقة · { $share } من عمليات الخروج
       *[other] { $amount } صفقة · { $share } من عمليات الخروج
    }
# $value is a formatted average percentage.
trader-exit-average = المتوسط { $value }

## Shared example vocabulary

trader-impact-label = الأثر:
trader-current-label = الحالي:
trader-readable-label = بصيغة مقروءة:
trader-example-how-it-works = طريقة العمل
trader-step-entry = دخول
trader-step-initial-position = المركز الأولي
trader-step-auto-exit = خروج آلي
trader-step-exit = خروج
trader-step-full-exit = خروج كامل من المركز
# $value is a percentage without its sign, as typed.
trader-value-percent = { $value }%
# $value is a percentage such as "20.0", shown after a plus sign.
trader-example-profit = ربح +{ $value }%

## Stop loss

trader-stop-loss-title = وقف الخسارة
trader-stop-loss-subtitle = الخروج تلقائيًا من المركز عندما تتجاوز خسارته الحد الذي تحدده
# $threshold is the threshold as typed.
trader-stop-loss-impact = الخروج عند الهبوط بنسبة { $threshold }% عن الدخول
trader-stop-loss-hold-immediate = فوري
# $span is a formatted duration.
trader-stop-loss-hold-delay = تأخير { $span }
trader-stop-loss-price-falls = هبوط السعر
trader-stop-loss-threshold-reached = تم بلوغ الحد
trader-stop-loss-partial = الخروج الجزئي مسموح
# $loss is the loss percentage with its sign.
trader-stop-loss-summary = تم حصر الخسارة عند <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>ملاحظة:</strong> يحميك وقف الخسارة من الخسائر الأكبر عبر الخروج المبكر

## Trailing stop

trader-trailing-title = وقف متحرك
trader-trailing-subtitle = حماية الأرباح تلقائيًا بتتبع السعر أثناء ارتفاعه
# $value is the activation percentage as typed.
trader-trailing-activation-impact = يبدأ التتبع عند ربح +{ $value }%
# $value is the trail distance percentage as typed.
trader-trailing-distance-impact = الخروج عند -{ $value }% من القمة
trader-trailing-activation = التفعيل
trader-trailing-peak = القمة
# $value is a formatted percentage.
trader-trailing-final = +{ $value }% نهائي
# $value is a formatted percentage.
trader-trailing-summary-protected = تمت حماية ربح <strong>{ $value }</strong>
# $value is a formatted percentage.
trader-trailing-summary-avoided = تم تفادي خسارة <strong>{ $value }</strong> من القمة

## Take profit

trader-roi-title = جني الأرباح
trader-roi-subtitle = الخروج تلقائيًا من المركز بالكامل عندما يبلغ الربح هدفك
# $target is the target percentage as typed.
trader-roi-impact = الخروج عند ربح +{ $target }%
trader-roi-example-title = سيناريو توضيحي
trader-roi-initial-buy = الشراء الأولي
trader-roi-target-hit = بلوغ الهدف
trader-roi-full-position = المركز كاملًا
trader-roi-sold = تم بيع 100%
# $target is the target percentage as typed.
trader-roi-summary = تم تثبيت ربح <strong>+{ $target }%</strong>

## Time-based exit

trader-time-title = الخروج الزمني
trader-time-subtitle = الخروج تلقائيًا من المراكز بعد أقصى مدة احتفاظ إذا تجاوزت الخسارة الحد
trader-time-unit-seconds = ثوانٍ
trader-time-unit-minutes = دقائق
trader-time-unit-hours = ساعات
trader-time-unit-days = أيام
# Shown before the configured duration loads.
trader-time-conversion-default = 168 ساعة = 7 أيام
# $duration and $readable are formatted durations.
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
        [zero] { $amount } ثانية
        [one] { $amount } ثانية
        [two] { $amount } ثانيتان
        [few] { $amount } ثوانٍ
        [many] { $amount } ثانية
       *[other] { $amount } ثانية
    }
trader-duration-minutes =
    { $count ->
        [zero] { $amount } دقيقة
        [one] { $amount } دقيقة
        [two] { $amount } دقيقتان
        [few] { $amount } دقائق
        [many] { $amount } دقيقة
       *[other] { $amount } دقيقة
    }
trader-duration-hours =
    { $count ->
        [zero] { $amount } ساعة
        [one] { $amount } ساعة
        [two] { $amount } ساعتان
        [few] { $amount } ساعات
        [many] { $amount } ساعة
       *[other] { $amount } ساعة
    }
trader-duration-days =
    { $count ->
        [zero] { $amount } يومًا
        [one] { $amount } يوم
        [two] { $amount } يومان
        [few] { $amount } أيام
        [many] { $amount } يومًا
       *[other] { $amount } يومًا
    }
# $value is the loss percentage as typed, without its sign.
trader-time-loss-impact = الخروج عند الهبوط بنسبة { $value }% أو أكثر بعد مدة الاحتفاظ
# $day is the day number of the example.
trader-time-day = اليوم { $day }
trader-time-position-opened = تم فتح المركز
trader-time-limit = الحد الزمني
trader-time-hold-reached = تم بلوغ مدة الاحتفاظ
trader-time-loss-met = تم بلوغ حد الخسارة
trader-time-note = <strong>ملاحظة:</strong> لن يتم الخروج من المراكز الرابحة أو ذات الخسائر الأصغر
trader-time-positions-title = حالة المراكز الحالية
trader-time-positions-loading = جارٍ تحميل المراكز...
trader-time-positions-empty = لا توجد مراكز مفتوحة
trader-time-positions-token = الرمز
trader-time-positions-hold = مدة الاحتفاظ
trader-time-positions-roi = العائد

## Strategy control

trader-strategy-entry-title = استراتيجيات الدخول
trader-strategy-entry-subtitle = إشارات يمكنها فتح مركز جديد.
trader-strategy-exit-title = استراتيجيات الخروج
trader-strategy-exit-subtitle = إشارات يمكنها إغلاق مركز مفتوح أو حمايته.
trader-strategy-active-unknown = -- نشطة
trader-strategy-active = { $enabled }/{ $total } نشطة
trader-strategy-loading = جارٍ تحميل الاستراتيجيات...
trader-strategy-load-failed = تعذّر تحميل الاستراتيجيات
trader-strategy-empty = لا توجد استراتيجيات معرّفة
trader-strategy-no-description = لا يوجد وصف.
trader-strategy-unnamed = استراتيجية بلا اسم
trader-strategy-priority-auto = تلقائي
trader-strategy-priority = الأولوية { $priority }

## Dollar-cost averaging

trader-dca-title = متوسط التكلفة (DCA)
trader-dca-subtitle = الإضافة تلقائيًا إلى المراكز الخاسرة لخفض متوسط سعر الدخول
trader-dca-example-title = مثال DCA
trader-dca-example = 0.01 { -sol } مبدئيًا ← DCA #1: 0.005 { -sol } @ -10% ← DCA #2: 0.005 { -sol } @ -10% إضافية
trader-dca-info-title = معلومات استراتيجية DCA
trader-dca-info-subtitle = اعتبارات مهمة عند التداول بـ DCA
trader-dca-how-title = طريقة عمل DCA
trader-dca-how-trigger = <strong>المشغّل:</strong> ينخفض المركز دون حد DCA (مثل -10%)
trader-dca-how-action = <strong>الإجراء:</strong> إضافة المزيد من { -sol } لخفض تكلفة الأساس المتوسطة
trader-dca-how-repeat = <strong>التكرار:</strong> يمكن تنفيذ DCA عدة مرات بحسب الحد الأقصى للعدد
trader-dca-risk-title = تحذيرات المخاطر
trader-dca-risk-exposure = <strong>زيادة التعرض:</strong> يزيد DCA إجمالي رأس المال المعرّض للخطر في كل مركز
trader-dca-risk-knife = <strong>السكين الساقط:</strong> لن يفيد DCA إذا واصل الرمز اتجاهه الهابط
trader-dca-risk-cooldown = <strong>فترة التهدئة:</strong> استخدم فترة التهدئة لتجنب دخولات DCA المتتالية بسرعة

## General settings

trader-sizing-title = حجم المركز
trader-sizing-subtitle = التحكم بمقدار الاستثمار في كل مركز
trader-timing-title = التوقيت وفترات التهدئة
trader-timing-subtitle = التحكم بالتوقيت بين العمليات
trader-timing-close-cooldown = فترة تهدئة إغلاق المركز
trader-timing-close-cooldown-hint = الدقائق التي يجب انتظارها قبل إعادة فتح الرمز نفسه
trader-timing-concurrency = تزامن فحص الدخول
trader-timing-concurrency-hint = عدد الرموز المفحوصة في وقت واحد (الأعلى أسرع لكنه يستهلك معالجًا أكثر)
trader-timing-unit-minutes = دقيقة
trader-timing-unit-tokens = رمز
