copy-skip-not-buy-swap = نشاط المحفظة لم يكن عملية شراء
copy-skip-task-disabled = المهمة متوقفة مؤقتًا
copy-skip-mode-transition-required = يجب تغيير وضع التنفيذ بشكل منفصل
copy-skip-live-confirmation-required = التنفيذ الحي يحتاج إلى تأكيد
copy-skip-unsupported-sizing-mode = وضع تحديد الحجم غير مدعوم بعد
copy-skip-self-copy = المحفظة إحدى محافظك
copy-skip-target-below-minimum = صفقة المحفظة أقل من الحد الأدنى
copy-skip-target-above-maximum = صفقة المحفظة أعلى من الحد الأقصى
copy-skip-already-bought = تم شراء هذا الرمز مسبقًا (شراء مرة واحدة)
copy-skip-blacklisted = الرمز محظور بموجب ضوابط المخاطر
copy-skip-filter-required = الرمز لم يجتز الترشيح
copy-skip-budget-exhausted = استُهلكت ميزانية المهمة
copy-skip-token-cap-reached = تم بلوغ حد الرمز الواحد
copy-skip-below-minimum-size = حجم النسخ صغير جدًا
copy-skip-invalid-sizing = حجم المهمة غير صالح
copy-skip-invalid-slippage = الانزلاق السعري للمهمة غير صالح
copy-skip-invalid-exit-policy = قواعد الخروج للمهمة غير صالحة
copy-skip-invalid-price = لا يوجد سعر سوق صالح للاستخدام
copy-skip-not-sell-swap = نشاط المحفظة لم يكن عملية بيع
copy-skip-exit-mode-disabled = تم تجاهل بيع المحفظة: المهمة تبيع وفق قواعدها الخاصة
copy-skip-force-stopped = التداول في إيقاف قسري
copy-skip-copy-position-not-found = لا يوجد مركز تملكه هذه المهمة
copy-skip-position-user-only = المركز تديره أنت
copy-skip-position-management-mismatch = المركز لم يعد يتبع عمليات بيع النسخ
copy-skip-latency-kill-switch = إيقاف مؤقت تلقائي: اكتُشفت الصفقات متأخرة جدًا
copy-skip-claim-reconciled-abandoned = أُغلق إرسال حي متوقف دون إعادة محاولة
copy-skip-stale-observation = أُعيد تشغيلها بعد توقف، وهي أقدم من أن تُنسخ
copy-skip-unknown-observation-time = الصفقة المعاد تشغيلها بلا وقت كتلة
copy-skip-entry-blocked = الدخول محظور

copy-entry-block-force-stopped = التداول في إيقاف قسري
copy-entry-block-loss-limit = حد الخسارة يمنع عمليات الدخول الجديدة
copy-entry-block-connectivity = الخدمات المطلوبة غير متاحة
copy-entry-block-position-limit = تم بلوغ حد المراكز المفتوحة
copy-entry-block-already-open = يوجد مركز مفتوح بالفعل
copy-entry-block-reentry-cooldown = فترة تهدئة إعادة دخول الرمز
copy-entry-block-open-cooldown = فترة تهدئة الدخول العامة
copy-entry-block-entry-reserved = دخول آخر قيد المعالجة
copy-entry-block-blacklisted = الرمز محظور بموجب ضوابط المخاطر
copy-entry-block-check-failed = تعذّر إكمال فحص أمان

copy-pause-user = أوقفتَه مؤقتًا
copy-pause-latency-kill-switch = إيقاف مؤقت تلقائي: وصلت الصفقات متأخرة { $average } ث في المتوسط (الحد { $threshold } ث)
copy-pause-watch-detached = إيقاف مؤقت تلقائي: لم تعد المحفظة مراقَبة
copy-pause-watch-budget-exceeded = متوقف مؤقتًا: بلغت هذه المحفظة حد فحص المراقبة (عدد التوقيعات: { $limit }) قبل أن تلحق بالركب
copy-pause-helius-unavailable = متوقف مؤقتًا: فشلت فحوصات المحفظة عبر { -helius }
copy-pause-watch-processing-failed = متوقف مؤقتًا: تعذّرت معالجة نشاط المحفظة
copy-pause-unspecified = متوقف مؤقتًا

copy-pause-short-user = بواسطتك
copy-pause-short-latency-kill-switch = بطيء جدًا
copy-pause-short-watch-detached = فُقدت المراقبة
copy-pause-short-watch-budget-exceeded = حد المراقبة
copy-pause-short-helius-unavailable = مزوّد المراقبة
copy-pause-short-watch-processing-failed = معالجة المراقبة
copy-state-paused = متوقف مؤقتًا
copy-state-paused-reason = متوقف مؤقتًا · { $reason }

copy-readiness-history = السجل التجريبي
copy-readiness-history-met =
    { $count ->
        [zero] لا جولات تجريبية مغلقة ({ $count })، المطلوب: { $needed }
        [one] جولة تجريبية مغلقة واحدة ({ $count })، المطلوب: { $needed }
        [two] جولتان تجريبيتان مغلقتان ({ $count })، المطلوب: { $needed }
        [few] { $count } جولات تجريبية مغلقة، المطلوب: { $needed }
        [many] { $count } جولة تجريبية مغلقة، المطلوب: { $needed }
       *[other] { $count } جولة تجريبية مغلقة، المطلوب: { $needed }
    }
copy-readiness-history-short = الجولات التجريبية المغلقة: { $count } من { $needed }
copy-readiness-profit = مربحة في التجريبي
copy-readiness-profit-detail =
    { $count ->
        [zero] { $realized } { -sol } محققة خلال { $count } جولة، الرابحة: { $wins }
        [one] { $realized } { -sol } محققة خلال جولة واحدة ({ $count })، الرابحة: { $wins }
        [two] { $realized } { -sol } محققة خلال جولتين ({ $count })، الرابحة: { $wins }
        [few] { $realized } { -sol } محققة خلال { $count } جولات، الرابحة: { $wins }
        [many] { $realized } { -sol } محققة خلال { $count } جولة، الرابحة: { $wins }
       *[other] { $realized } { -sol } محققة خلال { $count } جولة، الرابحة: { $wins }
    }
copy-readiness-latency = اكتشاف الصفقات في الوقت المناسب
copy-readiness-latency-detail = وصول p95 ‏{ $p95 } ث، الحد { $limit } ث
copy-readiness-latency-none = لا توجد عينات وصول بعد
copy-readiness-priced = كل الحيازات مسعّرة
copy-readiness-priced-ok = لكل حيازة تجريبية مفتوحة سعر مجمع
copy-readiness-priced-missing =
    { $count ->
        [zero] لا حيازات مفتوحة بلا سعر مجمع ({ $count })
        [one] حيازة مفتوحة واحدة بلا سعر مجمع ({ $count })
        [two] حيازتان مفتوحتان بلا سعر مجمع ({ $count })
        [few] { $count } حيازات مفتوحة بلا سعر مجمع
        [many] { $count } حيازة مفتوحة بلا سعر مجمع
       *[other] { $count } حيازة مفتوحة بلا سعر مجمع
    }
copy-readiness-runtime = التنفيذ الحي متاح
copy-readiness-runtime-ok = الإعداد وبوابات الأمان تسمح بالنسخ الحي

copy-live-block-setup-incomplete = أكمل إعداد المحفظة وRPC أولًا
copy-live-block-force-stop = الإيقاف الطارئ مفعّل
copy-live-block-copy-trading-disabled = معالجة النسخ متوقفة مؤقتًا على مستوى النظام
copy-live-block-unavailable = التنفيذ الحي غير متاح

copy-state-system-paused = متوقف مؤقتًا على مستوى النظام
copy-state-force-stopped = إيقاف قسري
copy-state-entries-blocked = الدخول محظور
copy-state-running-live = قيد التشغيل
copy-state-running-paper = قيد التشغيل
copy-mode-paper = تجريبي
copy-mode-live = حي
copy-exit-mode-buy-only = قواعد الخروج الخاصة بي
copy-exit-mode-mirror = محاكاة عمليات بيع المحفظة
copy-exit-mode-hybrid = عمليات بيع المحفظة وقواعدي
copy-exit-target-sell = باعت المحفظة
copy-exit-stop-loss = وقف الخسارة
copy-exit-trailing-stop = وقف متحرك
copy-exit-take-profit = جني الأرباح
copy-exit-time-override = قاعدة الوقت
copy-exit-manual = أُغلق يدويًا

copy-request-failed = فشل الطلب
copy-keep-paused = إبقاؤه متوقفًا
copy-paused-suffix = · متوقف مؤقتًا
copy-mode-paused = { $mode } · متوقف مؤقتًا
copy-task-ref = «{ $name }» ({ $mode })
copy-metric-realized-pnl = الأرباح والخسائر المحققة
copy-metric-unrealized-pnl = الأرباح والخسائر غير المحققة
copy-metric-win-rate = نسبة الربح
copy-metric-budget-spent = الميزانية المنفقة
copy-metric-median-arrival = وسيط الوصول
copy-metric-open-holdings = الحيازات المفتوحة
copy-record-won-lost = رابحة: { $won } · خاسرة: { $lost }
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = عمليات التنفيذ
copy-kind-exits = عمليات الخروج
copy-kind-skips = عمليات التخطي
copy-kind-errors = الأخطاء
copy-field-per-trade-cap = حد الصفقة الواحدة
copy-field-per-token-cap = حد الرمز الواحد
copy-field-total-budget = الميزانية الإجمالية
copy-field-slippage = الانزلاق السعري
copy-rules-wallet-sells-only = عمليات بيع المحفظة فقط
copy-filter-copy-setting-required = إعداد نسخ التداول (مطلوب)
copy-filter-copy-setting-not-required = إعداد نسخ التداول (غير مطلوب)
copy-count-closed-rounds =
    { $count ->
        [zero] { $count } جولة مغلقة
        [one] { $count } جولة مغلقة
        [two] { $count } جولتان مغلقتان
        [few] { $count } جولات مغلقة
        [many] { $count } جولة مغلقة
       *[other] { $count } جولة مغلقة
    }
copy-count-open-holdings =
    { $count ->
        [zero] { $count } حيازة مفتوحة
        [one] { $count } حيازة مفتوحة
        [two] { $count } حيازتان مفتوحتان
        [few] { $count } حيازات مفتوحة
        [many] { $count } حيازة مفتوحة
       *[other] { $count } حيازة مفتوحة
    }
copy-unrealized-partial =
    { $priced ->
        [zero] { $priced } حيازة مسعّرة · { $unpriced } بلا سعر
        [one] { $priced } حيازة مسعّرة · { $unpriced } بلا سعر
        [two] { $priced } حيازتان مسعّرتان · { $unpriced } بلا سعر
        [few] { $priced } حيازات مسعّرة · { $unpriced } بلا سعر
        [many] { $priced } حيازة مسعّرة · { $unpriced } بلا سعر
       *[other] { $priced } حيازة مسعّرة · { $unpriced } بلا سعر
    }
copy-unrealized-unpriced =
    { $count ->
        [zero] { $count } حيازة بلا سعر
        [one] { $count } حيازة بلا سعر
        [two] { $count } حيازتان بلا سعر
        [few] { $count } حيازات بلا سعر
        [many] { $count } حيازة بلا سعر
       *[other] { $count } حيازة بلا سعر
    }
copy-range-24h = 24h
copy-range-7d = 7d
copy-range-30d = 30d
copy-range-all = الكل
copy-range-label =
    .aria-label = النطاق الزمني

## Page strip

copy-page-title = نسخ التداول
copy-page-beta = تجريبي (Beta)
copy-strip-loading = جارٍ التحميل
copy-strip-unavailable = غير متاح
copy-strip-setup-required = الإعداد مطلوب · نسخ التداول يتطلب محفظة وRPC
copy-strip-pause-all = إيقاف الكل مؤقتًا
copy-strip-resume = استئناف المعالجة
copy-strip-settings = الإعدادات
copy-strip-add-wallet = إضافة محفظة
copy-strip-paused-globally = متوقف مؤقتًا على مستوى النظام · لا نسخ جديد، وعمليات الخروج تعمل
copy-strip-force-stopped = إيقاف قسري · لا يُنسخ شيء
copy-strip-loss-limit = حد الخسارة · عمليات الدخول الجديدة محظورة، وعمليات الخروج تعمل
copy-strip-idle-paused =
    { $count ->
        [zero] خامل · { $count } مهمة متوقفة مؤقتًا
        [one] خامل · { $count } مهمة متوقفة مؤقتًا
        [two] خامل · { $count } مهمتان متوقفتان مؤقتًا
        [few] خامل · { $count } مهام متوقفة مؤقتًا
        [many] خامل · { $count } مهمة متوقفة مؤقتًا
       *[other] خامل · { $count } مهمة متوقفة مؤقتًا
    }
copy-strip-idle-empty = خامل · لا مهام بعد
copy-strip-processing = قيد المعالجة · تجريبي: { $paper }
copy-strip-processing-live = قيد المعالجة · حي: { $live } · تجريبي: { $paper }
copy-figures-label =
    .aria-label = إجماليات نسخ التداول
copy-figure-marked-at-pool = مسعّر بسعر المجمع
copy-figure-across-tasks = عبر جميع المهام
copy-figure-budget-lifetime = الإنفاق التراكمي للمهام المفعّلة
copy-figure-budget-none = لا توجد مهام مفعّلة
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
        [zero] { $count } صفقة
        [one] { $count } صفقة
        [two] { $count } صفقتان
        [few] { $count } صفقات
        [many] { $count } صفقة
       *[other] { $count } صفقة
    }
copy-figure-arrival-none = لا توجد عينات من المهام المفعّلة

## Page frame

copy-load-failed = تعذّر تحميل نسخ التداول: { $error }
copy-resume-all-title = استئناف معالجة النسخ
copy-resume-all-message =
    { $count ->
        [zero] لا مهام حية ({ $count }) سترسل مبادلات حقيقية عندما تتداول محافظها مجددًا.
        [one] مهمة حية واحدة ({ $count }) سترسل مبادلات حقيقية عندما تتداول محفظتها مجددًا.
        [two] مهمتان حيتان ({ $count }) سترسلان مبادلات حقيقية عندما تتداول محفظتاهما مجددًا.
        [few] { $count } مهام حية سترسل مبادلات حقيقية عندما تتداول محافظها مجددًا.
        [many] { $count } مهمة حية سترسل مبادلات حقيقية عندما تتداول محافظها مجددًا.
       *[other] { $count } مهمة حية سترسل مبادلات حقيقية عندما تتداول محافظها مجددًا.
    }
copy-toast-resumed-all = تم استئناف معالجة النسخ
copy-toast-paused-all = تم إيقاف كل معالجة النسخ مؤقتًا
copy-toast-global-failed = تعذّر تغيير معالجة النسخ

## Onboarding

copy-onboarding-title = انسخ المحافظ التي تثق بها، واختبرها في التجريبي أولًا
copy-onboarding-body = كل مهمة تبدأ في الوضع التجريبي: تُحاكى صفقات الهدف بسعر المجمع مع الانزلاق السعري والرسوم الخاصة بك، وتعمل قواعد الخروج الخاصة بك على السجل التجريبي. فعّل التداول الحي لكل محفظة عندما تستحقه نتائجها التجريبية.
copy-onboarding-add = أضف أول محفظة لك
copy-setup-gate-title = نسخ التداول يتطلب محفظة
copy-onboarding-observe = راقب
copy-onboarding-observe-detail = اكتشف مبادلات المحفظة دون إنفاق { -sol }.
copy-onboarding-evaluate = قيّم
copy-onboarding-evaluate-detail = اقرأ الأرباح والخسائر التجريبية ونسبة الربح وعمليات التخطي وسرعة الاكتشاف والانزلاق السعري.
copy-onboarding-arm = فعّل
copy-onboarding-arm-detail = اجتز فحوصات الجاهزية ثم فعّل المبادلات الحقيقية.

## Wallet list

copy-list-label =
    .aria-label = المحافظ المنسوخة
copy-list-title = المحافظ
copy-list-compare = مقارنة
copy-list-sort-label = ترتيب المحافظ
copy-list-count = النشطة: { $active } · الإجمالي: { $total }
copy-sort-pnl = الأرباح والخسائر
copy-sort-state = الحالة
copy-sort-name = الاسم
copy-compare-label =
    .aria-label = مقارنة المحافظ

## Dialog chrome

copy-dialog-close =
    .aria-label = إغلاق
copy-editor-title-add = إضافة محفظة
copy-editor-sub-add = المهام الجديدة تبدأ في الوضع التجريبي
copy-arm-title = تفعيل النسخ الحي
copy-arm-sub = مبادلات حقيقية من محفظتك
copy-arm-keep-paper = البقاء في التجريبي
copy-arm-confirm = تفعيل التداول الحي
copy-profile-title = ملف المحفظة
copy-profile-sub = ما رآه هذا البوت من المحفظة

## Settings dialog

copy-settings-title = إعدادات نسخ التداول
copy-settings-subtitle = سياسة عامة لكل المهام
copy-settings-filter-warning = مع إعداد الترشيح الافتراضي يرفض هذا الخيار تقريبًا كل رمز، فلا يُنسخ شيء. أبقِه معطّلًا ما لم تكن مرشحاتك تُجيز الرموز التي تتداولها محافظك.
copy-settings-unit-seconds = ثانية
copy-settings-unit-trades = صفقة
copy-settings-unit-tasks = مهمة
copy-settings-unit-rounds = جولة
copy-settings-save = حفظ الإعدادات
copy-settings-load-failed = تعذّر تحميل إعدادات النسخ
copy-settings-saved = تم حفظ إعدادات نسخ التداول

## Workspace

copy-tab-overview = نظرة عامة
copy-tab-holdings = الحيازات
copy-tab-activity = النشاط
copy-tab-rules = القواعد
copy-tab-execution = التنفيذ
copy-tabs-label = عروض المهمة
copy-workspace-select = اختر محفظة لفتح مساحة عملها.
copy-workspace-loading = جارٍ تحميل المهمة…
copy-workspace-load-failed = تعذّر تحميل هذه المهمة: { $error }

copy-state-detail-paper = قيد التشغيل في التجريبي · تُحاكى الصفقات ولا يُنفق شيء
copy-state-detail-live = قيد التشغيل حيًا · تُنسخ صفقات المحفظة بمبادلات حقيقية
copy-state-detail-system-paused = في الانتظار · معالجة النسخ متوقفة مؤقتًا على مستوى النظام، وعمليات الخروج تعمل
copy-state-detail-entries-blocked = الدخول محظور بسبب حد الخسارة · عمليات الخروج تعمل
copy-state-detail-force-stopped = إيقاف قسري · لا يُنسخ شيء

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = الاستئناف يُبقي الحد نفسه، لذا يتوقف مجددًا ما دامت الصفقات تصل متأخرة. افحص تدفق RPC أو ارفع حد الوصول في الإعدادات.
copy-paused-resume-detached = الاستئناف يعيد مراقبة المحفظة.
copy-paused-holdings-rules =
    { $count ->
        [zero] قواعد الخروج الخاصة بها ما زالت تغلق حيازاتها المفتوحة ({ $count }).
        [one] قواعد الخروج الخاصة بها ما زالت تغلق حيازتها المفتوحة ({ $count }).
        [two] قواعد الخروج الخاصة بها ما زالت تغلق حيازتيها المفتوحتين ({ $count }).
        [few] قواعد الخروج الخاصة بها ما زالت تغلق حيازاتها المفتوحة ({ $count }).
        [many] قواعد الخروج الخاصة بها ما زالت تغلق حيازاتها المفتوحة ({ $count }).
       *[other] قواعد الخروج الخاصة بها ما زالت تغلق حيازاتها المفتوحة ({ $count }).
    }
copy-paused-holdings-mirror =
    { $count ->
        [zero] عمليات بيع المحفظة ما زالت تغلق حيازاتها المفتوحة ({ $count }).
        [one] عمليات بيع المحفظة ما زالت تغلق حيازتها المفتوحة ({ $count }).
        [two] عمليات بيع المحفظة ما زالت تغلق حيازتيها المفتوحتين ({ $count }).
        [few] عمليات بيع المحفظة ما زالت تغلق حيازاتها المفتوحة ({ $count }).
        [many] عمليات بيع المحفظة ما زالت تغلق حيازاتها المفتوحة ({ $count }).
       *[other] عمليات بيع المحفظة ما زالت تغلق حيازاتها المفتوحة ({ $count }).
    }
copy-paused-holdings-hybrid =
    { $count ->
        [zero] عمليات بيع المحفظة وقواعد الخروج ما زالت تغلق حيازاتها المفتوحة ({ $count }).
        [one] عمليات بيع المحفظة وقواعد الخروج ما زالت تغلق حيازتها المفتوحة ({ $count }).
        [two] عمليات بيع المحفظة وقواعد الخروج ما زالت تغلق حيازتيها المفتوحتين ({ $count }).
        [few] عمليات بيع المحفظة وقواعد الخروج ما زالت تغلق حيازاتها المفتوحة ({ $count }).
        [many] عمليات بيع المحفظة وقواعد الخروج ما زالت تغلق حيازاتها المفتوحة ({ $count }).
       *[other] عمليات بيع المحفظة وقواعد الخروج ما زالت تغلق حيازاتها المفتوحة ({ $count }).
    }

copy-watch-state-catching-up = مراقبة المحفظة: جارٍ اللحاق. يجري الفحص لهذه المحفظة عبر { -helius }.
copy-watch-state-watching = مراقبة المحفظة: قيد المراقبة. يجري الفحص لهذه المحفظة عبر { -helius }.
copy-watch-last-check = آخر فحص { $ago }.
copy-watch-recovery-active = مراقبة المحفظة نشطة
copy-watch-recovery-catching-up = مراقبة المحفظة تلحق بالركب
copy-watch-recovery-still-paused = مهمة النسخ ما زالت متوقفة مؤقتًا. استأنف النسخ عندما تكون مستعدًا.
copy-watch-recovery-title = استعادة مراقبة المحفظة
copy-watch-recovery-processing-failed = تعذّرت معالجة نشاط المحفظة. التقدم المحفوظ باقٍ. أعد المحاولة بعد حل المشكلة.
copy-watch-recovery-provider-failed = فشلت فحوصات { -helius }. التقدم المحفوظ باقٍ. أعد المحاولة عندما يتوفر المزوّد.
copy-watch-recovery-budget-intro = لدى هذه المحفظة نشاط أكثر مما تستطيع مراقبتها الحالية فحصه. اختر كيف تتابع.
copy-watch-approve = محاولة اللحاق باستخدام { -helius }
copy-watch-approve-help = يتابع من التقدم المحفوظ. قد يستهلك رصيد { -helius } أكثر وقد يتأخر رغم ذلك.
copy-watch-approve-unavailable = اللحاق عبر { -helius } غير متاح. اضبط نقطة اتصال RPC مفعّلة من { -helius } للمتابعة دون تخطي النشاط غير المفحوص.
copy-watch-no-provider = لا يوجد مزوّد لحاق مدعوم لهذه المراقبة.
copy-watch-budget-label = التوقيعات المفحوصة في كل فحص
copy-watch-budget-hint = أو تخطَّ النشاط غير المفحوص واستأنف من الآن. اختر عدد التوقيعات لكل فحص من { $min } إلى { $max }؛ الحد الأعلى قد يستخدم مزيدًا من استدعاءات RPC.
copy-watch-ack = أفهم أن النشاط الفائت لن يُنسخ.
copy-watch-toast-range = اختر عدد التوقيعات لكل استعلام بين { $min } و{ $max } بخطوات قدرها { $step }
copy-watch-toast-ack = أقرّ بأن التوقيعات منذ آخر فحص مكتمل ستُتخطى
copy-watch-resumed = استؤنفت مراقبة المحفظة من الآن؛ مهمة النسخ ما زالت متوقفة مؤقتًا
copy-watch-resume-failed = تعذّر استئناف مراقبة المحفظة
copy-watch-retry-started = بدأت إعادة محاولة مراقبة المحفظة من التقدم المحفوظ؛ مهمة النسخ ما زالت متوقفة مؤقتًا
copy-watch-retry-failed = تعذّرت إعادة محاولة مراقبة المحفظة
copy-watch-approve-title = السماح باللحاق عبر { -helius } لهذه المحفظة
copy-watch-approve-message = يمكن لـ { -helius } فحص معاملات Solana الناجحة من التقدم المحفوظ دون تخطي الفترة غير المفحوصة. يحتسب حاليًا 10 أرصدة لكل 100 معاملة كاملة مُعادة، بالتقريب للأعلى، بحد أدنى 10 أرصدة لكل طلب. قد يجري الفحص الواحد عدة طلبات؛ وقد يختلف الاستهلاك وتسعير المزوّد. يبقى النسخ متوقفًا مؤقتًا حتى تستأنفه بشكل منفصل.
copy-watch-approve-confirm = السماح لهذه المحفظة
copy-watch-approved = بدأت مراقبة المحفظة من التقدم المحفوظ؛ مهمة النسخ ما زالت متوقفة مؤقتًا
copy-watch-restore-failed = تعذّرت استعادة مراقبة المحفظة

copy-action-pause = إيقاف مؤقت
copy-action-resume = استئناف
copy-action-resume-copy = استئناف النسخ
copy-action-resume-from-now = الاستئناف من الآن
copy-action-retry-watch = إعادة محاولة مراقبة المحفظة
copy-action-return-paper = العودة إلى التجريبي
copy-action-edit-rules = تعديل القواعد
copy-action-clone = استنساخ
copy-action-profile = ملف المحفظة
copy-resume-live-title = استئناف النسخ الحي
copy-resume-live-message = «{ $name }» سترسل مبادلات حقيقية من محفظتك عندما تتداول هذه المحفظة مجددًا.
copy-resume-live-confirm = استئناف التداول الحي
copy-task-resumed = تم استئناف المهمة
copy-task-paused = تم إيقاف المهمة مؤقتًا
copy-task-state-failed = تعذّر تغيير حالة المهمة
copy-return-paper-message = النسخ الجديد بواسطة «{ $name }» سيُحاكى مجددًا دون إنفاق { -sol }.
copy-return-paper-cancel = البقاء في الحي
copy-task-returned-paper = عادت المهمة إلى التجريبي
copy-mode-change-failed = تعذّر تغيير وضع التنفيذ
copy-delete-title = حذف مهمة النسخ
copy-delete-message = هل تريد حذف «{ $name }»؟ ستُزال قراراتها ونتائجها التجريبية ولن تُراقَب المحفظة لهذه المهمة بعد الآن.
copy-delete-confirm = حذف المهمة
copy-delete-cancel = إبقاء المهمة
copy-task-deleted = تم حذف مهمة النسخ
copy-task-delete-failed = تعذّر حذف مهمة النسخ

## Overview tab

copy-overview-results = النتائج
copy-analytics-load-failed = تعذّر تحميل التحليلات: { $error }
copy-analytics-loading = جارٍ تحميل التحليلات…
copy-exit-bucket =
    { $count ->
        [zero] { $count } عملية بيع · { $pnl }
        [one] { $count } عملية بيع · { $pnl }
        [two] { $count } عمليتا بيع · { $pnl }
        [few] { $count } عمليات بيع · { $pnl }
        [many] { $count } عملية بيع · { $pnl }
       *[other] { $count } عملية بيع · { $pnl }
    }
copy-overview-average-win = متوسط الربح
copy-overview-average-loss = متوسط الخسارة { $amount }
copy-overview-profit-factor = عامل الربح
copy-overview-profit-factor-note = إجمالي الأرباح ÷ إجمالي الخسائر
copy-overview-average-hold = متوسط مدة الاحتفاظ
copy-overview-average-hold-note = من الدخول إلى الخروج
copy-overview-best-round = أفضل جولة
copy-overview-worst-round = الأسوأ { $amount }
copy-overview-curve-title = الأرباح والخسائر التراكمية
copy-overview-exits-title = عمليات البيع حسب الخروج
copy-overview-skips-title = أسباب تخطي الصفقات
copy-book-title-live = السجل الحي
copy-book-title-paper = السجل التجريبي
copy-book-all-time = كل الأوقات
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
        [zero] عملية شراء
        [one] عملية شراء
        [two] عمليتا شراء
        [few] عمليات شراء
        [many] عملية شراء
       *[other] عملية شراء
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
        [zero] خروج بقواعدك
        [one] خروج بقواعدك
        [two] خروجان بقواعدك
        [few] عمليات خروج بقواعدك
        [many] خروجًا بقواعدك
       *[other] خروج بقواعدك
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
        [zero] عملية بيع للمحفظة
        [one] عملية بيع للمحفظة
        [two] عمليتا بيع للمحفظة
        [few] عمليات بيع للمحفظة
        [many] عملية بيع للمحفظة
       *[other] عملية بيع للمحفظة
    }
copy-book-manual-closes = <strong>{ $count }</strong> أُغلقت يدويًا
copy-book-skipped = <strong>{ $count }</strong> تم تخطيها
copy-book-failed = <strong>{ $count }</strong> فشلت
copy-book-closed = { $count } أُغلقت
copy-book-budget-note = إنفاق { $mode } من { $total } · المتبقي { $remaining }
copy-check-passed = ناجح
copy-check-not-passed = غير ناجح
copy-readiness-title = قبل التحول إلى الحي
copy-readiness-live-note = هذه المهمة تتداول حيًا. أعدها إلى التجريبي من الترويسة أعلاه.
copy-readiness-all-pass = كل الفحوصات ناجحة.
copy-readiness-needs-review = التفعيل يتطلب مراجعة صريحة لما هو غير جاهز.
copy-readiness-arm = مراجعة وتفعيل التداول الحي

## Rules tab and review

copy-rules-title = القواعد السارية
copy-rules-size-ratio = { $pct } من صفقة المحفظة
copy-rules-size-fixed = { $amount } لكل نسخة
copy-rules-target-any = أي حجم
copy-rules-target-min = { $amount } على الأقل
copy-rules-target-max = { $amount } على الأكثر
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = تجاوز المهمة · المتداول { $value }
copy-rules-source-default = افتراضي المتداول
copy-rules-not-used = غير مستخدمة: عمليات بيع المحفظة هي التي تحدد
copy-rules-col-rule = القاعدة
copy-rules-col-applies = التطبيق
copy-rules-col-source = المصدر
copy-rules-budget-note = أُنفق { $spent } في { $mode } · المتبقي { $remaining }
copy-rules-token-copies =
    { $count ->
        [zero] نحو { $count } نسخة كاملة لرمز واحد
        [one] نحو نسخة كاملة واحدة ({ $count }) لرمز واحد
        [two] نحو نسختين كاملتين ({ $count }) لرمز واحد
        [few] نحو { $count } نسخ كاملة لرمز واحد
        [many] نحو { $count } نسخة كاملة لرمز واحد
       *[other] نحو { $count } نسخة كاملة لرمز واحد
    }
copy-rules-sizing = تحديد الحجم
copy-rules-copy-size = حجم النسخ
copy-rules-entry-filters = مرشحات الدخول
copy-rules-target-size = حجم صفقة المحفظة
copy-rules-repeat-buys = عمليات الشراء المتكررة
copy-rules-repeat-first-only = أول عملية شراء لكل رمز فقط
copy-rules-repeat-every = كل عملية شراء، حتى حد الرمز الواحد
copy-rules-filter-pass = اجتياز الترشيح
copy-rules-filter-required = مطلوب
copy-rules-filter-not-required = غير مطلوب
copy-rules-filter-task-override = تجاوز المهمة
copy-rules-exits = الخروج
copy-rules-exits-inactive = تُباع الحيازات فقط عندما تبيع المحفظة؛ القواعد أدناه لا تعمل في هذا الوضع.

## Exit rules

copy-rule-status = الحالة
copy-rule-on = مفعّل
copy-rule-off = معطّل
copy-rule-unit-seconds = ثانية
copy-rule-unit-minutes = دقيقة
copy-rule-stop-loss-threshold = البيع عند خسارة قدرها
copy-rule-stop-loss-min-hold = ليس قبل الاحتفاظ لمدة
copy-rule-no-minimum = بلا حد أدنى
copy-rule-partial-exits = الخروج الجزئي
copy-rule-partial-allowed = مسموح
copy-rule-partial-full-only = خروج كامل فقط
copy-rule-partial-size = حجم الخروج الجزئي
copy-rule-trailing-activation = يُفعَّل عند ربح قدره
copy-rule-trailing-distance = البيع عند الهبوط عن الذروة بمقدار
copy-rule-take-profit-target = البيع عند ربح قدره
copy-rule-time-duration = يفحص بعد الاحتفاظ لمدة
copy-rule-time-threshold = يبيع عندما تكون الأرباح والخسائر عند هذا الحد أو أقل
copy-preset-inherit = افتراضيات المتداول
copy-preset-conservative = متحفظ
copy-preset-balanced = متوازن
copy-preset-aggressive = هجومي
copy-preset-custom = مخصص
copy-validate-stop-loss = يجب أن يكون وقف الخسارة أعلى من 0% وبحد أقصى 100%.
copy-validate-partial-size = يجب أن يكون حجم الخروج الجزئي بين 0% و100%.
copy-validate-min-hold = يجب أن يكون الحد الأدنى للاحتفاظ عددًا صحيحًا من الثواني.
copy-validate-trailing-activation = يجب أن يكون تفعيل الوقف المتحرك أعلى من 0% وبحد أقصى 100%.
copy-validate-trailing-distance = يجب أن تكون مسافة الوقف المتحرك أعلى من 0% وبحد أقصى 100%.
copy-validate-take-profit = يجب أن يكون جني الأرباح أعلى من 0%.
copy-validate-time-duration = قاعدة الوقت تحتاج إلى مدة أكبر من صفر.
copy-validate-time-threshold = حد قاعدة الوقت هو خسارة: استخدم 0% أو رقمًا سالبًا.
copy-warning-mirror = عمليات بيع المحفظة وحدها تغلق الحيازات: لا وقف خسارة يحميها، والرمز الذي لا تبيعه المحفظة يبقى محتفَظًا به.
copy-warning-no-rules = لا توجد قاعدة خروج مفعّلة وعمليات بيع المحفظة متجاهلة: لن تُباع الحيازات أبدًا.
copy-warning-no-stop-loss = لا يوجد وقف خسارة: الرمز الهابط يبقى محتفَظًا به حتى تبيعه قاعدة أخرى أو المحفظة.
copy-warning-stop-delay = وقف الخسارة ينتظر { $hold } بعد كل عملية شراء: الرمز الذي يهبط أسرع يُغلق بعيدًا عن { $threshold }.
copy-warning-take-profit-cost = جني الأرباح عند { $target } لا يغطي تكلفة البيع (انزلاق سعري { $slippage } ورسوم مبادلة { $fee })، لذا يغلق الجولات بخسارة.
copy-warning-trailing-distance = مسافة الوقف المتحرك تساوي ربح تفعيله أو تزيد عليه، لذا قد يبيع الوقف المفعّل دون سعر الدخول.

## Execution tab

copy-execution-title = جودة التنفيذ
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = أي
copy-execution-limit-on =
    { $count ->
        [zero] يتوقف مؤقتًا فوق { $limit } في المتوسط خلال { $count } صفقة
        [one] يتوقف مؤقتًا فوق { $limit } في المتوسط خلال صفقة واحدة ({ $count })
        [two] يتوقف مؤقتًا فوق { $limit } في المتوسط خلال صفقتين ({ $count })
        [few] يتوقف مؤقتًا فوق { $limit } في المتوسط خلال { $count } صفقات
        [many] يتوقف مؤقتًا فوق { $limit } في المتوسط خلال { $count } صفقة
       *[other] يتوقف مؤقتًا فوق { $limit } في المتوسط خلال { $count } صفقة
    }
copy-execution-limit-off = مفتاح الإيقاف الطارئ معطّل
copy-execution-arrival-samples =
    { $count ->
        [zero] { $count } صفقة رُصدت لحظة حدوثها
        [one] { $count } صفقة رُصدت لحظة حدوثها
        [two] { $count } صفقتان رُصدتا لحظة حدوثهما
        [few] { $count } صفقات رُصدت لحظة حدوثها
        [many] { $count } صفقة رُصدت لحظة حدوثها
       *[other] { $count } صفقة رُصدت لحظة حدوثها
    }
copy-execution-p95 = وصول p95
copy-execution-median-slippage = وسيط الانزلاق السعري
copy-execution-slippage-samples =
    { $count ->
        [zero] { $count } عملية تنفيذ مقاسة
        [one] { $count } عملية تنفيذ مقاسة
        [two] { $count } عمليتا تنفيذ مقاستان
        [few] { $count } عمليات تنفيذ مقاسة
        [many] { $count } عملية تنفيذ مقاسة
       *[other] { $count } عملية تنفيذ مقاسة
    }
copy-execution-worst-slippage = أسوأ انزلاق سعري
copy-execution-average-slippage = المتوسط { $amount }
copy-execution-delay-title = تأخر الاكتشاف
copy-execution-delay-note = الزمن من كتلة المحفظة إلى رؤية هذا البوت للصفقة. تُستثنى عمليات إعادة التشغيل بعد التوقف.
copy-execution-delay-limit = الأعمدة التي تتجاوز حد الوصول { $limit } تظهر باللون الكهرماني.
copy-execution-fastest = الأسرع
copy-execution-average = المتوسط
copy-execution-slowest = الأبطأ
copy-execution-fill-title = التنفيذ مقارنةً بالمحفظة
copy-execution-fill-note = القيمة الموجبة تعني أسوأ من المحفظة: دفعت أكثر في الشراء، أو استلمت أقل في بيع محاكى. التنفيذ التجريبي لرمز بلا سعر مجمع يُسعَّر بصفقة المحفظة نفسها، فلا يقيس شيئًا ويُستبعد.
copy-execution-samples = العينات
copy-execution-median = الوسيط
copy-execution-worst = الأسوأ
copy-execution-decisions = القرارات في النطاق

## Compare view

copy-compare-title = مقارنة المحافظ
copy-compare-back = العودة إلى المحفظة
copy-compare-load-failed = تعذّر تحميل المقارنة: { $error }
copy-compare-loading = جارٍ تحميل المقارنة…
copy-compare-empty = لا توجد مهام للمقارنة.
copy-compare-empty-message = أضف مهمة نسخ لمقارنة نتائجها بالمهام الأخرى.
copy-compare-curve-title = الأرباح والخسائر المحققة التراكمية
copy-table-wallet = المحفظة
copy-table-mode = الوضع
copy-table-rounds = الجولات
copy-table-realized = المحقق
copy-table-profit-factor = عامل الربح
copy-table-average-hold = متوسط الاحتفاظ
copy-table-median-slippage = وسيط الانزلاق السعري

## Charts

copy-chart-curve-label = الأرباح والخسائر التراكمية { $amount } { -sol }
copy-chart-compare-label = الأرباح والخسائر التراكمية حسب المهمة
copy-chart-empty-curve = لا توجد جولات مغلقة في هذا النطاق بعد.
copy-chart-empty-bars = لا شيء مسجل في هذا النطاق.
copy-chart-empty-histogram = لا توجد عينات وصول في هذا النطاق.
copy-chart-empty-compare = لا توجد جولات مغلقة للمقارنة في هذا النطاق.
copy-chart-histogram-title = { $count } من { $total }

## Wallet profile

copy-profile-copy = نسخ هذه المحفظة
copy-profile-copy-other = النسخ بقواعد أخرى
copy-profile-loading = جارٍ تحميل ملف المحفظة…
copy-profile-watch-title = المراقبة
copy-profile-watched = مراقَبة
copy-profile-watch-resume-hint = استئناف المهمة يعيد مراقبتها
copy-profile-watch-add-hint = إضافة مهمة تبدأ مراقبتها
copy-profile-stream = التدفق
copy-profile-subscribed = مشترَك
copy-profile-not-subscribed = غير مشترَك
copy-profile-sources =
    { $count ->
        [zero] { $count } مصدر
        [one] { $count } مصدر
        [two] { $count } مصدران
        [few] { $count } مصادر
        [many] { $count } مصدرًا
       *[other] { $count } مصدر
    }
copy-profile-last-activity = آخر نشاط
copy-profile-last-error = آخر خطأ
copy-profile-own-wallet = هذه إحدى محافظك؛ نسخها مرفوض.
copy-profile-observed-title = الصفقات المرصودة
copy-profile-observed-none = لا توجد صفقات من هذه المحفظة في هذا البوت بعد. المهمة التجريبية ترصدها دون إنفاق { -sol }.
copy-profile-swaps-seen = المبادلات المرصودة
copy-profile-swaps-seen-note = مبادلات المحفظة المتمايزة عبر مهامك
copy-profile-buys-sells = الشراء / البيع
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = الرموز المتداولة
copy-profile-first-seen = أول ظهور
copy-profile-last-seen = آخر ظهور
copy-profile-tasks-title = مهامك على هذه المحفظة
copy-table-task = المهمة

## Arm live dialog

copy-arm-acks-left =
    { $count ->
        [zero] لا إقرارات متبقية للتأشير ({ $count })
        [one] إقرار واحد متبقٍ للتأشير ({ $count })
        [two] إقراران متبقيان للتأشير ({ $count })
        [few] { $count } إقرارات متبقية للتأشير
        [many] { $count } إقرارًا متبقيًا للتأشير
       *[other] { $count } إقرار متبقٍ للتأشير
    }
copy-arm-readiness-title = الجاهزية من السجل التجريبي
copy-arm-exposure-title = الانكشاف
copy-arm-per-copy = لكل نسخة
copy-arm-budget-left-value = { $left } من { $total } { -sol }
copy-arm-budget-left = الميزانية الحية المتبقية
copy-arm-budget-left-note = الإنفاق التجريبي يُحتسب بشكل منفصل ولا يستهلكها
copy-arm-exits = الخروج
copy-arm-stop-note = ليس قبل الاحتفاظ لمدة { $hold }: الهبوط الأسرع يغلق بسعر أدنى
copy-arm-shared = هذه المحفظة تنسخها أيضًا: { $tasks }. كل مهمة تنسخ صفقاتها بميزانيتها الخاصة.
copy-arm-unavailable = التنفيذ الحي غير متاح الآن؛ راجع آخر فحص.
copy-arm-ack-real-native = { -sol } حقيقي: يمكن لهذه المهمة إنفاق ما يصل إلى { $budget } { -sol } من محفظتك، وبحد أقصى { $trade } { -sol } لكل نسخة.
copy-arm-ack-fees = النسخ الحي يدفع رسوم شبكة وانزلاقًا سعريًا حقيقيين؛ والنتائج التجريبية لا تضمن النتائج الحية.
copy-arm-ack-unready = بعض فحوصات الجاهزية لم تنجح. فعّل هذه المهمة رغم ذلك.
copy-arm-lead = «{ $name }» ستنسخ صفقات هذه المحفظة بمبادلات حقيقية من محفظتك.
copy-arm-confirmation-missing = تعذّر تحميل التأكيد الحي
copy-arm-armed = تم تفعيل النسخ الحي
copy-arm-failed = تعذّر تفعيل النسخ الحي

## Holdings tab

copy-holdings-title = الحيازات
copy-holdings-view-label = عرض الحيازات
copy-holdings-view-open = المفتوحة ({ $count })
copy-holdings-view-closed = الجولات المغلقة ({ $count })
copy-holdings-reset = إعادة تعيين السجل التجريبي
copy-holdings-live-note = النسخ الحي مراكز حقيقية.
copy-holdings-open-positions = المراكز المفتوحة
copy-holdings-token-details = فتح تفاصيل الرمز
copy-holdings-opened = فُتح { $time }
copy-holdings-no-pool-price = لا يوجد سعر مجمع
copy-holdings-close = إغلاق
copy-holdings-write-off = شطب
copy-holdings-activity = النشاط
copy-holdings-no-exit-rule = لا توجد قاعدة خروج
copy-holdings-watch-stop = وقف { $level }
copy-holdings-watch-stop-until = وقف { $level } خلال { $span }
copy-holdings-watch-take = جني { $level }
copy-holdings-watch-trail = متحرك { $level }
copy-holdings-watch-trail-arms = يُفعَّل المتحرك { $level }
copy-holdings-watch-time = الوقت ≤ { $level }
copy-holdings-watch-time-until = الوقت ≤ { $level } خلال { $span }
copy-holdings-watch-wallet-sells = بيع المحفظة
copy-holdings-empty = لا توجد حيازات تجريبية مفتوحة. عمليات الشراء المنسوخة من المحفظة تظهر هنا.
copy-holdings-col-token = الرمز
copy-holdings-col-cost = التكلفة
copy-holdings-col-entry = الدخول
copy-holdings-col-mark = السعر الحالي
copy-holdings-col-peak = الذروة
copy-holdings-col-pnl = الأرباح والخسائر
copy-holdings-col-exit-rules = قواعد الخروج
copy-holdings-col-held = مدة الاحتفاظ
copy-holdings-col-actions = الإجراءات
copy-holdings-col-invested = المستثمر
copy-holdings-col-proceeds = العائد
copy-holdings-col-exit = الخروج
copy-holdings-col-closed = الإغلاق
copy-holdings-price-note = الأسعار بوحدة { -sol } لكل رمز. سعر الدخول يشمل انزلاق الشراء ورسومه؛ والذروة ومستويات الخروج نسبة إليه، لذا تُفتح الحيازة وذروتها دون سعر الدخول. مرّر المؤشر فوق أي منها لرؤية سعر مجمعها.
copy-holdings-paused-rules = متوقف مؤقتًا: لا نسخ جديد. قواعد الخروج الخاصة بك ما زالت تغلق هذه الحيازات.
copy-holdings-paused-mirror = متوقف مؤقتًا: لا نسخ جديد. عمليات بيع المحفظة ما زالت تغلق هذه الحيازات.
copy-holdings-paused-hybrid = متوقف مؤقتًا: لا نسخ جديد. عمليات بيع المحفظة وقواعد الخروج الخاصة بك ما زالت تغلق هذه الحيازات.
copy-holdings-closed-load-failed = تعذّر تحميل الجولات المغلقة: { $error }
copy-holdings-closed-loading = جارٍ تحميل الجولات المغلقة…
copy-holdings-closed-empty = لا توجد جولات مغلقة بعد.
copy-holdings-closed-latest = أحدث { $shown } من أصل { $total } من الجولات.
copy-holdings-close-title = إغلاق حيازة تجريبية
copy-holdings-close-message = بيع { $token } في السجل التجريبي بسعر المجمع ({ $price }) مع الانزلاق السعري والرسوم الخاصة بالمهمة.
copy-holdings-close-confirm = إغلاق الحيازة
copy-holdings-write-off-title = شطب حيازة تجريبية
copy-holdings-write-off-message = { $token } بلا سعر مجمع للبيع عنده. الشطب يغلقها عند الصفر ويسجل تكلفتها ({ $cost }) كخسارة.
copy-holdings-keep = إبقاء
copy-holdings-written-off = تم شطب { $token }
copy-holdings-closed = تم إغلاق { $token }
copy-holdings-written-off-detail = أُغلقت بعائد صفري
copy-holdings-sold-at = بيعت بسعر { $price }
copy-holdings-close-failed = تعذّر إغلاق الحيازة
copy-holdings-reset-message = ابدأ «{ $name }» من جديد: ستُزال حيازاتها التجريبية وإنفاقها وعمليات التنفيذ والخروج والتخطي. تبقى القواعد والمحفظة.
copy-holdings-reset-cancel = إبقاء السجل
copy-holdings-reset-done = تمت إعادة تعيين السجل التجريبي
copy-holdings-reset-detail =
    { $count ->
        [zero] لم تُزل قرارات ({ $count })
        [one] أُزيل قرار واحد ({ $count })
        [two] أُزيل قراران ({ $count })
        [few] أُزيلت { $count } قرارات
        [many] أُزيل { $count } قرارًا
       *[other] أُزيل { $count } قرار
    }
copy-holdings-reset-failed = تعذّرت إعادة تعيين السجل التجريبي

## Activity tab

copy-activity-title = النشاط
copy-activity-filter-label = مرشح النشاط
copy-filter-all = الكل
copy-outcome-paper-filled = شراء تجريبي
copy-outcome-live-submitted = أُرسل شراء حي
copy-outcome-live-confirmed = تأكد شراء حي
copy-outcome-live-failed = فشل شراء حي
copy-outcome-paper-sell-observed = بيع تجريبي · باعت المحفظة
copy-outcome-live-sell-submitted = أُرسل بيع حي
copy-outcome-live-sell-failed = فشل بيع حي
copy-outcome-skipped = تم التخطي
copy-activity-decision = القرار
copy-activity-paper-exit = خروج تجريبي · { $rule }
copy-activity-filled = { $input } بسعر { $price } · اشترت المحفظة { $target }
copy-activity-filled-slippage = { $input } بسعر { $price } · اشترت المحفظة { $target } · الانزلاق السعري { $slippage }
copy-activity-filled-unpriced = { $input } بسعر { $price } · اشترت المحفظة { $target } · سُعّر بصفقة المحفظة، لا يوجد سعر مجمع
copy-activity-live-sized = { $sized } · اشترت المحفظة { $target }
copy-activity-sell-nothing = باعت المحفظة { $amount } · لا شيء محتفَظ به للبيع
copy-activity-written-off = شُطب عند الصفر: لا يوجد سعر مجمع
copy-activity-sold = الرموز: { $tokens } مقابل { $proceeds } بسعر { $price }
copy-activity-full-close = إغلاق كامل
copy-activity-partial-exit = خروج { $pct }
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = الحد الأدنى { $amount }
copy-activity-skip-maximum = الحد الأقصى { $value }
copy-activity-skip-stale = تأخر { $arrival }، الحد { $limit }
copy-activity-skip-latency = المتوسط { $average }، الحد { $limit }
copy-activity-arrival-replayed = أُعيد تشغيلها بعد { $span } من الكتلة
copy-activity-arrival-seen = رُصدت بعد { $span } من الكتلة
copy-activity-link-wallet-tx = معاملة المحفظة
copy-activity-link-own-tx = معاملتك
copy-activity-only-token = هذا الرمز فقط
copy-activity-skipped-group = تم التخطي ×{ $count }
copy-activity-group-detail =
    { $tokens ->
        [zero] { $tokens } رمز · منذ { $since }
        [one] { $tokens } رمز · منذ { $since }
        [two] { $tokens } رمزان · منذ { $since }
        [few] { $tokens } رموز · منذ { $since }
        [many] { $tokens } رمزًا · منذ { $since }
       *[other] { $tokens } رمز · منذ { $since }
    }
copy-activity-mint-filter =
    .placeholder = عنوان إصدار الرمز
    .aria-label = التصفية حسب عنوان إصدار الرمز
copy-activity-clear = مسح
copy-activity-load-failed = تعذّر تحميل النشاط: { $error }
copy-activity-loading = جارٍ تحميل النشاط…
copy-activity-no-match = لا شيء يطابق هذا المرشح.
copy-activity-empty = لا توجد قرارات بعد. عمليات التنفيذ والخروج والتخطي تظهر هنا مع تداول المحفظة.
copy-activity-load-older = تحميل الأقدم
copy-activity-start = بداية السجل
copy-activity-older-failed = تعذّر تحميل النشاط الأقدم

## Task editor

copy-step-wallet = المحفظة
copy-step-sizing = تحديد الحجم
copy-step-entry = مرشحات الدخول
copy-step-exits = الخروج
copy-step-review = المراجعة
copy-editor-title-edit = تعديل { $name }
copy-editor-title-clone = استنساخ { $name }
copy-editor-sub-edit = مهمة { $mode } · تُطبَّق التغييرات على قراراتها التالية
copy-editor-sub-clone = القواعد نفسها، وسجل تجريبي فارغ، وتبدأ في الوضع التجريبي
copy-editor-save-edit = حفظ التغييرات
copy-editor-save-clone = إنشاء نسخة مستنسخة
copy-editor-save-create = إنشاء مهمة تجريبية
copy-editor-clone-suffix = (نسخة)
copy-editor-discard-edit = تجاهل التغييرات
copy-editor-discard-create = تجاهل هذه المهمة
copy-editor-discard-edit-message = تغييراتك على «{ $name }» غير محفوظة.
copy-editor-discard-create-message = المحفظة والقواعد المدخلة حتى الآن غير محفوظة.
copy-editor-discard-confirm = تجاهل
copy-editor-keep-editing = متابعة التعديل
copy-editor-toast-updated = تم تحديث المهمة
copy-editor-toast-clone = تم إنشاء النسخة المستنسخة
copy-editor-toast-created = تم إنشاء المهمة التجريبية
copy-unit-native = { -sol }
copy-editor-any = أي
copy-editor-duplicate = تنسخها بالفعل: { $tasks }. هذه المهمة تنسخ الصفقات نفسها، بقواعدها وميزانيتها الخاصة.
copy-editor-wallet = المحفظة
copy-editor-wallet-identity = محفظة المهمة هي هويتها. لنسخ محفظة أخرى بهذه القواعد، استنسخ المهمة.
copy-editor-address-label = عنوان المحفظة
copy-editor-address-placeholder = عنوان محفظة Solana
copy-editor-address-help-clone = القواعد نفسها مع سجل تجريبي فارغ. أبقِ هذه المحفظة لاختبار قواعد أخرى عليها، أو أدخل محفظة أخرى.
copy-editor-address-help-create = المحفظة التي تنسخ هذه المهمة عمليات شرائها (وبيعها إذا اخترت ذلك).
copy-editor-name-label = الاسم <em>اختياري</em>
copy-editor-name-placeholder = مثال: المتبدّل السريع
copy-editor-enabled-title = معالجة صفقات المحفظة
copy-editor-enabled-help = التعطيل يُبقي المهمة متوقفة مؤقتًا حتى تستأنفها.
copy-editor-note-live = هذه المهمة حية: التغييرات تُطبَّق على نسخها الحقيقي التالي.
copy-editor-note-paper = المهام تعمل في التجريبي حتى تفعّلها: تُحاكى الصفقات بسعر المجمع ولا يُنفق شيء.
copy-editor-copy-size = حجم النسخ
copy-editor-sizing-fixed = مبلغ ثابت
copy-editor-sizing-ratio = حصة من صفقة المحفظة
copy-editor-amount-fixed = المبلغ لكل نسخة
copy-editor-amount-ratio = الحصة من كل صفقة
copy-editor-amount-help-fixed = يُنفق على كل عملية شراء منسوخة، بحد أدنى { $minimum }.
copy-editor-amount-help-ratio = من عملية شراء المحفظة نفسها، حتى حد الصفقة الواحدة.
copy-editor-help-trade-cap = لا تنفق أي نسخة واحدة أكثر من ذلك.
copy-editor-help-token-cap = إجمالي المنفق على رمز واحد.
copy-editor-help-budget = كل ما يمكن لهذه المهمة إنفاقه طوال عمرها؛ يحتسب التجريبي والحي إنفاق كل منهما بشكل مستقل.
copy-editor-preview-title = تكلفة النسخة الواحدة
copy-editor-preview-empty = أدخل الحجم لرؤية تكلفة النسخة الواحدة.
copy-editor-preview-example = المحفظة تشتري { $target } ← تنسخ أنت <strong>{ $copy }</strong>
copy-editor-preview-once = يأخذ الرمز الواحد نسخة واحدة فقط بحجم { $size }، إذ يُشترى كل رمز مرة واحدة
copy-editor-preview-token-cap =
    { $count ->
        [zero] يأخذ الرمز الواحد { $count } نسخة بحجم { $size } كحد أقصى
        [one] يأخذ الرمز الواحد نسخة واحدة ({ $count }) بحجم { $size } كحد أقصى
        [two] يأخذ الرمز الواحد نسختين ({ $count }) بحجم { $size } كحد أقصى
        [few] يأخذ الرمز الواحد { $count } نسخ بحجم { $size } كحد أقصى
        [many] يأخذ الرمز الواحد { $count } نسخة بحجم { $size } كحد أقصى
       *[other] يأخذ الرمز الواحد { $count } نسخة بحجم { $size } كحد أقصى
    }
copy-editor-preview-summary-exact = { $perToken }؛ تغطي الميزانية نحو { $count } منها. تُضاف رسوم الشبكة والأولوية.
copy-editor-preview-summary-minimum = { $perToken }؛ تغطي الميزانية { $count } منها على الأقل. تُضاف رسوم الشبكة والأولوية.
copy-editor-target-min = أصغر صفقة محفظة تُنسخ
copy-editor-target-min-help = تجاهل عمليات شراء المحفظة الأصغر. اتركه فارغًا لعدم وجود حد أدنى.
copy-editor-target-max = أكبر صفقة محفظة تُنسخ
copy-editor-target-max-help = تجاهل عمليات شراء المحفظة الأكبر. اتركه فارغًا لعدم وجود حد أقصى.
copy-editor-buy-once-title = شراء كل رمز مرة واحدة
copy-editor-buy-once-help = انسخ أول عملية شراء للمحفظة لرمز ما فقط؛ وتُتخطى عمليات شرائها اللاحقة له.
copy-editor-filter-require = مطلوب
copy-editor-filter-skip = غير مطلوب
copy-editor-filter-help = اشترط أن يجتاز الرمز مسار الترشيح الخاص بك قبل نسخه.
copy-editor-filter-warning = مع إعداد الترشيح الافتراضي يفشل تقريبًا كل رمز، فالمهمة التي تشترط الاجتياز لا تنسخ شيئًا. اشترطه فقط عندما تُجيز مرشحاتك الرموز التي تتداولها هذه المحفظة.
copy-editor-exit-both = كلاهما
copy-editor-exit-help-buy-only = قواعدك أدناه تبيع كل حيازة؛ وتُتجاهل عمليات بيع المحفظة.
copy-editor-exit-help-hybrid = أيهما يأتي أولًا: أن تبيع المحفظة، أو أن تنطلق إحدى قواعدك.
copy-editor-exit-help-mirror = تُباع الحيازات فقط عندما تبيع المحفظة. قواعد الخروج الخاصة بك لا تعمل.
copy-editor-who-sells = من يبيع
copy-editor-preset = إعداد مسبق
copy-editor-preset-help = الإعداد المسبق يملأ كل القواعد أدناه؛ عدّل أيًّا منها بعد ذلك.
copy-editor-mirror-note = هذه القواعد لا تعمل ما دامت عمليات بيع المحفظة هي التي تحدد. تُطبَّق إذا انتقلت إلى { $mine } أو { $both }.
copy-editor-rule-inherit = افتراضي المتداول
copy-editor-inherit-value = افتراضي المتداول ({ $value })
copy-editor-rule-aria = إعداد { $rule }
copy-editor-rule-empty-uses = الفارغ يستخدم افتراضي المتداول: { $value }
copy-editor-rule-follows = يتبع المتداول: { $summary }
copy-editor-rule-follows-plain = يتبع إعداد المتداول.
copy-editor-rule-follows-own = يتبع مفتاح المتداول بقيم هذه المهمة: { $summary }
copy-editor-rule-off-note = معطّل لهذه المهمة، مهما استخدم المتداول.
copy-task-unnamed = مهمة بلا اسم
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = تعالج الصفقات بعد الحفظ
copy-editor-review-paused = تُحفظ متوقفة مؤقتًا
copy-editor-error-address = أدخل عنوان محفظة Solana صالحًا.
copy-editor-error-sizing = يجب أن تكون كل قيم الحجم أكبر من صفر.
copy-editor-error-min-copy = يجب ألا تقل النسخة عن { $minimum }: ارفع المبلغ لكل نسخة.
copy-editor-error-min-cap = يجب ألا تقل النسخة عن { $minimum }: ارفع حد الصفقة الواحدة.
copy-editor-error-trade-cap = لا يمكن أن يتجاوز حد الصفقة الواحدة حد الرمز الواحد.
copy-editor-error-token-cap = لا يمكن أن يتجاوز حد الرمز الواحد الميزانية الإجمالية.
copy-editor-error-slippage = يجب أن يكون الانزلاق السعري بين { $min } و{ $max }.
copy-editor-error-target-limits = يجب أن تكون حدود صفقة المحفظة صفرًا أو أكثر.
copy-editor-error-target-order = لا يمكن أن تتجاوز أصغر صفقة محفظة أكبرَها.

## Copy notices

copy-notice-task-unnamed = المهمة #{ $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = شراء نسخ تجريبي
copy-notice-title-paper-sell = بيع نسخ تجريبي
copy-notice-title-paper-closed = أُغلقت حيازة تجريبية
copy-notice-title-paper-exit = خروج تجريبي: { $rule }
copy-notice-title-live-buy-submitted = أُرسل شراء نسخ حي
copy-notice-title-live-buy-confirmed = تأكد شراء نسخ حي
copy-notice-title-live-buy-failed = فشل شراء نسخ حي
copy-notice-title-live-sell-submitted = أُرسل بيع نسخ حي
copy-notice-title-live-sell-failed = فشل بيع نسخ حي
copy-notice-title-auto-paused = أُوقفت مهمة النسخ مؤقتًا تلقائيًا
copy-notice-detail-bought = اشتُري بمبلغ { $amount } { -sol }
copy-notice-detail-sold = بيع بمبلغ { $amount } { -sol }
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = { $percent }% من الحيازة
copy-notice-detail-full-close = إغلاق كامل
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = فشلت المبادلة
