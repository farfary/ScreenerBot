# Filter rejection reasons. Message ids derive from the stored rejection codes
# (src/filtering/sources/rejection.rs); rows hold codes, never this text.

filtering-reject-no-decimals = لا توجد خانات عشرية في قاعدة البيانات
filtering-reject-token-too-new = الرمز حديث جدًا
filtering-reject-cooldown-filtered = تم الترشيح بسبب فترة التهدئة
filtering-reject-dex-data-missing = بيانات { -dexscreener } مفقودة
filtering-reject-gecko-data-missing = بيانات { -geckoterminal } مفقودة
filtering-reject-rug-data-missing = بيانات { -rugcheck } مفقودة
filtering-reject-onchain-numeric-symbol = رمز مختصر أرقام فقط (احتيال)
filtering-reject-onchain-empty-symbol = رمز مختصر فارغ (احتيال)
filtering-reject-onchain-suspicious-symbol = رمز مختصر مشبوه (احتيال)
filtering-reject-onchain-known-scam-authority = صلاحية احتيال معروفة
filtering-reject-onchain-immutable-with-freeze = غير قابل للتغيير + صلاحية تجميد (احتيال)
filtering-reject-onchain-high-risk-score = درجة مخاطر عالية على السلسلة
filtering-reject-dex-empty-name = اسم فارغ
filtering-reject-dex-empty-symbol = رمز مختصر فارغ
filtering-reject-dex-empty-logo = رابط الشعار فارغ
filtering-reject-dex-empty-website = رابط الموقع فارغ
filtering-reject-dex-txn-5m = معاملات 5m منخفضة
filtering-reject-dex-txn-1h = معاملات 1h منخفضة
filtering-reject-dex-zero-liq = سيولة معدومة
filtering-reject-dex-liq-low = السيولة منخفضة جدًا
filtering-reject-dex-liq-high = السيولة مرتفعة جدًا
filtering-reject-dex-mcap-low = القيمة السوقية منخفضة جدًا
filtering-reject-dex-mcap-high = القيمة السوقية مرتفعة جدًا
filtering-reject-dex-vol-low = حجم التداول منخفض جدًا
filtering-reject-dex-vol-missing = حجم التداول مفقود
filtering-reject-dex-fdv-low = FDV منخفضة جدًا
filtering-reject-dex-fdv-high = FDV مرتفعة جدًا
filtering-reject-dex-vol5m-low = حجم تداول 5m منخفض جدًا
filtering-reject-dex-vol5m-missing = حجم تداول 5m مفقود
filtering-reject-dex-vol1h-low = حجم تداول 1h منخفض جدًا
filtering-reject-dex-vol1h-missing = حجم تداول 1h مفقود
filtering-reject-dex-vol6h-low = حجم تداول 6h منخفض جدًا
filtering-reject-dex-vol6h-missing = حجم تداول 6h مفقود
filtering-reject-dex-price-change-5m-low = تغير السعر في 5m منخفض جدًا
filtering-reject-dex-price-change-5m-high = تغير السعر في 5m مرتفع جدًا
filtering-reject-dex-price-change-low = تغير السعر منخفض جدًا
filtering-reject-dex-price-change-high = تغير السعر مرتفع جدًا
filtering-reject-dex-price-change-6h-low = تغير السعر في 6h منخفض جدًا
filtering-reject-dex-price-change-6h-high = تغير السعر في 6h مرتفع جدًا
filtering-reject-dex-price-change-24h-low = تغير السعر في 24h منخفض جدًا
filtering-reject-dex-price-change-24h-high = تغير السعر في 24h مرتفع جدًا
filtering-reject-gecko-liq-low = السيولة منخفضة جدًا
filtering-reject-gecko-liq-high = السيولة مرتفعة جدًا
filtering-reject-gecko-mcap-low = القيمة السوقية منخفضة جدًا
filtering-reject-gecko-mcap-high = القيمة السوقية مرتفعة جدًا
filtering-reject-gecko-vol5m-low = حجم تداول 5m منخفض جدًا
filtering-reject-gecko-vol5m-missing = حجم تداول 5m مفقود
filtering-reject-gecko-vol1h-low = حجم تداول 1h منخفض جدًا
filtering-reject-gecko-vol1h-missing = حجم تداول 1h مفقود
filtering-reject-gecko-vol24h-low = حجم تداول 24h منخفض جدًا
filtering-reject-gecko-vol24h-missing = حجم تداول 24h مفقود
filtering-reject-gecko-price-change-5m-low = تغير السعر في 5m منخفض جدًا
filtering-reject-gecko-price-change-5m-high = تغير السعر في 5m مرتفع جدًا
filtering-reject-gecko-price-change-1h-low = تغير السعر في 1h منخفض جدًا
filtering-reject-gecko-price-change-1h-high = تغير السعر في 1h مرتفع جدًا
filtering-reject-gecko-price-change-24h-low = تغير السعر في 24h منخفض جدًا
filtering-reject-gecko-price-change-24h-high = تغير السعر في 24h مرتفع جدًا
filtering-reject-gecko-pool-count-low = عدد مجمعات السيولة منخفض جدًا
filtering-reject-gecko-pool-count-high = عدد مجمعات السيولة مرتفع جدًا
filtering-reject-gecko-pool-count-missing = عدد مجمعات السيولة مفقود
filtering-reject-gecko-reserve-low = الاحتياطي منخفض جدًا
filtering-reject-gecko-reserve-missing = الاحتياطي مفقود
filtering-reject-rug-rugged = رمز تعرض لسحب السجادة (Rug Pull)
filtering-reject-rug-score = درجة المخاطر مرتفعة جدًا
filtering-reject-rug-level-danger = مستوى مخاطر خطير
filtering-reject-rug-mint-authority = صلاحية الإصدار موجودة
filtering-reject-rug-freeze-authority = صلاحية التجميد موجودة
filtering-reject-rug-top-holder = نسبة أكبر حامل مرتفعة جدًا
filtering-reject-rug-top3-holders = نسبة أكبر 3 حاملين مرتفعة جدًا
filtering-reject-rug-min-holders = عدد الحاملين غير كافٍ
filtering-reject-rug-insider-count = عدد الحاملين المطّلعين كبير جدًا
filtering-reject-rug-insider-pct = نسبة المطّلعين مرتفعة جدًا
filtering-reject-rug-creator-pct = رصيد المنشئ مرتفع جدًا
filtering-reject-rug-transfer-fee-present = رسوم التحويل موجودة
filtering-reject-rug-transfer-fee-high = رسوم التحويل مرتفعة جدًا
filtering-reject-rug-graph-insiders = المطّلعون في الرسم البياني كُثر جدًا
filtering-reject-rug-lp-providers-low = عدد مزوّدي السيولة منخفض جدًا
filtering-reject-rug-lp-providers-missing = مزوّدو السيولة مفقودون
filtering-reject-rug-lp-lock-low = قفل السيولة منخفض جدًا
filtering-reject-rug-lp-lock-missing = قفل السيولة مفقود
filtering-reject-llm-analysis-rejected = رُفض بتحليل LLM: { $reason } (الثقة { $confidence }%، { $provider })
filtering-reject-llm-analysis-rejected-generic = رُفض بتحليل LLM
filtering-reject-unknown = { $code }

# Codes no longer emitted; they appear only in stored rows and keep their wording.
filtering-reject-dex-fdv-missing = FDV مفقودة
filtering-reject-dex-price-change-5m-missing = تغير السعر في 5m مفقود
filtering-reject-dex-price-change-missing = تغير السعر مفقود
filtering-reject-dex-price-change-6h-missing = تغير السعر في 6h مفقود
filtering-reject-dex-price-change-24h-missing = تغير السعر في 24h مفقود
filtering-reject-gecko-liq-missing = السيولة مفقودة
filtering-reject-gecko-mcap-missing = القيمة السوقية مفقودة
filtering-reject-gecko-price-change-5m-missing = تغير السعر في 5m مفقود
filtering-reject-gecko-price-change-1h-missing = تغير السعر في 1h مفقود
filtering-reject-gecko-price-change-24h-missing = تغير السعر في 24h مفقود
filtering-reject-rug-transfer-fee-missing = بيانات رسوم التحويل مفقودة

# Rejection categories used to group reasons.
filtering-reject-category-security = مشكلات الأمان
filtering-reject-category-distribution = توزيع الحاملين
filtering-reject-category-liquidity-lock = مشكلات قفل السيولة
filtering-reject-category-fees = رسوم التحويل
filtering-reject-category-liquidity = السيولة
filtering-reject-category-volume = حجم التداول
filtering-reject-category-market-cap = القيمة السوقية/FDV
filtering-reject-category-price-action = حركة السعر
filtering-reject-category-activity = نشاط التداول
filtering-reject-category-data-quality = بيانات مفقودة
filtering-reject-category-timing = مرشحات التوقيت
filtering-reject-category-market = بيانات السوق
filtering-reject-category-other = أخرى

# Filtering page: sub-tabs, sources, status, analytics, explorer and configuration.

## Sub-tabs and sources. Source ids are FilterSource::as_str plus the `meta` settings tab.

filtering-tab-status = الحالة
filtering-tab-analytics = التحليلات
filtering-tab-explorer = المستكشف
filtering-source-core = النواة
filtering-source-onchain = على السلسلة
filtering-source-dexscreener = { -dexscreener }
filtering-source-geckoterminal = { -geckoterminal }
filtering-source-rugcheck = { -rugcheck }
filtering-source-llm-analysis = تحليل LLM

## Time range

filtering-range-1h = 1H
filtering-range-6h = 6H
filtering-range-24h = 24H
filtering-range-7d = 7D
filtering-range-all = الكل
filtering-range-all-time = كل الأوقات
filtering-range-custom = مخصص
filtering-range-now = الآن
# $start and $end are formatted moments, or the open-ended markers.
filtering-range-span = { $start } ← { $end }
# $min and $max are the two ends of a value range.
filtering-range-bounds = { $min } – { $max }

## Footer status line

filtering-footer-saving = جارٍ حفظ التغييرات...
filtering-footer-refreshing = جارٍ تحديث اللقطة...
filtering-footer-unsaved = توجد تغييرات غير محفوظة
# $time is a relative time such as "5m ago".
filtering-footer-last-saved = آخر حفظ { $time }
filtering-footer-in-sync = الإعدادات متزامنة

## Info bar and status metrics

filtering-info-total = الإجمالي:
filtering-info-priced = المسعّرة:
filtering-info-passed = الناجحة:
filtering-info-positions = المراكز:
filtering-info-blacklisted = في القائمة السوداء:
filtering-info-cache = الذاكرة المؤقتة:
# A count followed by its share of the total, e.g. "120 (4.0%)".
filtering-count-share = { $count } ({ $share })
filtering-refresh-building = جارٍ البناء…
filtering-refresh-never = أبدًا

filtering-status-loading = جارٍ تحميل الإحصاءات...
filtering-status-total = إجمالي الرموز
filtering-status-total-detail = في ذاكرة الترشيح المؤقتة
filtering-status-total-detail-building = اللقطة قيد البناء، وتظهر الأعداد في التحديث التالي
filtering-status-priced = ذات سعر
filtering-status-priced-detail = { $share } لها تسعير
filtering-status-passed = اجتازت المرشحات
filtering-status-passed-detail = { $share } ناجحة
filtering-status-positions = المراكز المفتوحة
filtering-status-positions-detail = الصفقات النشطة
filtering-status-blacklisted = في القائمة السوداء
filtering-status-blacklisted-detail = رموز مُبلَّغ عنها
filtering-status-ohlcv = مع OHLCV
filtering-status-ohlcv-detail = بيانات تاريخية
filtering-status-refresh = آخر تحديث
filtering-status-refresh-building = اللقطة الأولى قيد الإنشاء
filtering-status-refresh-none = لا يوجد تحديث بعد
filtering-status-no-rejections = لا توجد بيانات رفض متاحة

## Analytics

filtering-analytics-loading = جارٍ تحميل التحليلات لـ { $range }…
filtering-analytics-scanned = إجمالي الرموز الممسوحة
# $time is a relative time such as "5m ago".
filtering-analytics-updated = تم التحديث { $time }
filtering-analytics-passed = الرموز الناجحة
filtering-analytics-pass-rate = نسبة النجاح <strong>{ $share }</strong>
filtering-analytics-rejected = الرموز المرفوضة
filtering-analytics-rejection-rate = نسبة الرفض <strong>{ $share }</strong>
filtering-analytics-by-category = الرفض حسب الفئة
filtering-analytics-by-source = الرفض حسب المصدر
filtering-analytics-no-category = لا توجد بيانات فئات
filtering-analytics-no-source = لا توجد بيانات مصادر
filtering-analytics-top-reasons = أهم أسباب الرفض
filtering-analytics-no-data = لا توجد بيانات متاحة
filtering-analytics-column-reason = السبب
filtering-analytics-column-category = الفئة
filtering-analytics-column-count = العدد
filtering-analytics-column-share = %
filtering-analytics-column-impact = الأثر
# $amount is the formatted count, $count selects the plural.
filtering-tokens-count =
    { $count ->
        [zero] { $amount } رمز
        [one] { $amount } رمز
        [two] { $amount } رمزان
        [few] { $amount } رموز
        [many] { $amount } رمزًا
       *[other] { $amount } رمز
    }

## Explorer

filtering-explorer-top-reasons = أهم الأسباب
filtering-explorer-recent = حالات الرفض الأخيرة
filtering-explorer-none = لا توجد بيانات
filtering-explorer-none-recent = لا شيء حديث
filtering-explorer-search =
    .placeholder = البحث في الأسباب...
filtering-explorer-overview = نظرة عامة
filtering-explorer-no-match = لا توجد أسباب مطابقة
filtering-explorer-column-token = الرمز
filtering-explorer-column-source = المصدر
filtering-explorer-column-time = الوقت
filtering-explorer-page = الصفحة { $page }
filtering-explorer-no-results = لا توجد نتائج
filtering-explorer-empty = لم يتم العثور على رموز
filtering-explorer-empty-filtered = لم يتم العثور على رموز تطابق المرشح
filtering-explorer-load-failed = فشل تحميل الرموز

## Configuration panels

filtering-config-loading = جارٍ تحميل الإعدادات…
# $query is the text typed in the filter box.
filtering-config-no-match = لا يوجد معامل يطابق «{ $query }»
filtering-config-no-parameters = هذا المصدر لا يعرض أي معاملات
# $source is the source name.
filtering-source-off = ترشيح { $source } معطّل، ولا يتم تقييم هذه المعاملات.
filtering-toolbar-filter =
    .placeholder = ترشيح المعاملات
    .aria-label = ترشيح المعاملات
filtering-toolbar-clear =
    .aria-label = مسح المرشح
# $count selects the plural, $amount is the number shown.
filtering-parameter-count =
    { $count ->
        [zero] { $amount } معامل
        [one] { $amount } معامل
        [two] { $amount } معاملان
        [few] { $amount } معاملات
        [many] { $amount } معاملًا
       *[other] { $amount } معامل
    }
# $count is the total and selects the plural.
filtering-parameter-count-filtered =
    { $count ->
        [zero] المعاملات: { $visible } من { $total }
        [one] المعاملات: { $visible } من { $total }
        [two] المعاملات: { $visible } من { $total }
        [few] المعاملات: { $visible } من { $total }
        [many] المعاملات: { $visible } من { $total }
       *[other] المعاملات: { $visible } من { $total }
    }
filtering-group-enable =
    .aria-label = تفعيل فحوصات { $group }
filtering-field-min = الأدنى
filtering-field-max = الأقصى
# $label is the parameter name.
filtering-field-min-aria =
    .aria-label = الحد الأدنى لـ { $label }
filtering-field-max-aria =
    .aria-label = الحد الأقصى لـ { $label }
# $default is the shipped value, $label the parameter name.
filtering-field-reset =
    .title = إعادة إلى الافتراضي ({ $default })
    .aria-label = إعادة { $label } إلى الافتراضي

## Toasts. A message value is the title; `.message` is the body.

filtering-toast-saved = تم حفظ الإعدادات
    .message = تم حفظ إعدادات الترشيح وتحديث اللقطة
filtering-toast-save-failed = فشل الحفظ
    .message = فشل حفظ إعدادات الترشيح
filtering-toast-reset = تمت إعادة ضبط التغييرات
    .message = تمت استعادة الإعدادات إلى آخر حالة محفوظة
filtering-toast-refresh-failed = فشل التحديث
    .message = فشل تحديث لقطة الترشيح
filtering-toast-exported = تم تصدير الإعدادات
    .message = تم حفظ إعدادات الترشيح في ملف
filtering-toast-imported = تم استيراد الإعدادات
    .message = تم تحميل إعدادات الترشيح من الملف
filtering-toast-import-failed = فشل الاستيراد
    .message = فشل استيراد الإعدادات، صيغة الملف غير صالحة
filtering-toast-load-failed = فشل التحميل
    .message = فشل تحميل إعدادات الترشيح
filtering-toast-range-missing = يرجى تحديد تاريخي البداية والنهاية
filtering-toast-range-order = يجب أن يسبق وقت البداية وقت النهاية
filtering-toast-range-future = لا يمكن أن يكون وقت النهاية في المستقبل
