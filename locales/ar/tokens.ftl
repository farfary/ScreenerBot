# Token data source status. Ids come from build_source_status in
# src/webserver/routes/tokens/source_status.rs. $label is a provider name and is
# not translated.

tokens-result-source-live = بيانات السوق المباشرة
tokens-result-source-unavailable = { $label } غير متاح — جارٍ إعادة المحاولة
tokens-result-source-not-listed = غير مدرج على { $label }
tokens-result-security-available = تقرير الأمان متاح
tokens-result-security-missing = لا يوجد تقرير من { -rugcheck }
tokens-result-chart-available = بيانات المخطط متاحة
tokens-result-chart-missing = لا توجد بيانات مخطط بعد

# Token details dialog: shared tab states (ui/token_details/state_handling.js)

tokens-state-error-title = تعذّر تحميل البيانات
tokens-state-offline = يبدو أنك غير متصل بالإنترنت.
tokens-state-request-failed = فشل الطلب بعد عدة محاولات.
tokens-state-waiting = بانتظار البيانات…

# Token details dialog: chart (ui/token_details/chart_tab.js)

tokens-chart-marker-entry = دخول
tokens-chart-level-stop-loss = وقف الخسارة
tokens-chart-level-take-profit = جني الأرباح

# Token details dialog: transactions tab (ui/token_details/transactions_tab.js)

tokens-transactions-loading = جارٍ تحميل المعاملات…
tokens-transactions-empty-title = لا توجد معاملات
tokens-transactions-empty-history = لا يتوفر سجل معاملات محفظة لهذا الرمز.
tokens-transactions-empty-data = لا تتوفر بيانات معاملات لهذا الرمز.
tokens-transactions-error-title = تعذّر تحميل المعاملات
tokens-transactions-error-message = سجل المعاملات غير متاح مؤقتًا.
tokens-transactions-activity-title = نشاط 24h
tokens-transactions-activity-subtitle = معاملات المحفظة بالساعة
tokens-transactions-metric-total = الإجمالي
tokens-transactions-metric-buys = عمليات الشراء
tokens-transactions-metric-sells = عمليات البيع
tokens-transactions-recent-title = المعاملات الأخيرة
# $count is the number of rows listed.
tokens-transactions-shown = المعروض: { $count }
tokens-transactions-column-time = الوقت
tokens-transactions-column-type = النوع
tokens-transactions-column-price = السعر ({ -sol })
tokens-transactions-column-total = الإجمالي ({ -sol })
tokens-transactions-chart-missing = مكتبة المخططات مفقودة
tokens-transactions-view-solscan = عرض المعاملة على { -solscan }

# Token details dialog: positions tab (ui/token_details/positions_tab.js)

tokens-positions-empty-title = لا يوجد مركز
tokens-positions-no-token = لم يتم تحديد رمز.
tokens-positions-empty-message = لا يوجد مركز لهذا الرمز بعد. استخدم «شراء» لفتح مركز.
tokens-positions-loading = جارٍ تحميل المركز…
tokens-positions-from-wallet-history = من سجل المحفظة
tokens-positions-frozen = مجمّد — لا يمكن بيعه
tokens-positions-no-cost-basis = بلا تكلفة أساس
tokens-positions-history-incomplete = السجل غير مكتمل
# $count is the number of DCA buys.
tokens-positions-dca-count = DCA { $count }
# $count is the number of partial exits.
tokens-positions-exit-count = الخروج { $count }
tokens-positions-fact-avg-entry = متوسط الدخول
tokens-positions-fact-current = الحالي
tokens-positions-fact-tokens = الرموز
tokens-positions-fact-opened = تاريخ الفتح
tokens-positions-fact-exit-price = سعر الخروج
tokens-positions-fact-native-received = { -sol } المستلم
tokens-positions-fact-closed-reason = سبب الإغلاق
tokens-positions-fact-target-min = الحد الأدنى لهدف الربح
tokens-positions-fact-target-max = الحد الأقصى لهدف الربح
tokens-positions-fact-highest = أعلى سعر
tokens-positions-fact-lowest = أدنى سعر
tokens-positions-section-range = الأهداف والنطاق
tokens-positions-section-market = السوق والحيازات
tokens-positions-kicker = مركز
tokens-positions-fallback-symbol = الرمز
tokens-positions-realized-pnl = الأرباح والخسائر المحققة
tokens-positions-unrealized-pnl = الأرباح والخسائر غير المحققة
tokens-positions-size = الحجم

# Token details dialog: security tab (ui/token_details/security_tab.js)
# Risk names and descriptions come from the RugCheck report and render as sent.

tokens-security-analysis-pending = تحليل { -rugcheck } قيد التنفيذ...
tokens-security-analyzing = جارٍ تحليل الأمان…
tokens-security-pulse-title = نبض الأمان
tokens-security-pending-caption = لا تزال إشارات المخاطر قيد الجمع.
tokens-security-control-title = التحكم في الرمز
tokens-security-control-meta = حالة الصلاحيات
# $time is the formatted time of the last security update.
tokens-security-updated = آخر تحديث { $time }
tokens-security-score-caption = درجة مخاطر الرمز المعيارية من 100.
tokens-security-score-label = الدرجة
tokens-security-rugged = تم سحب السجادة
tokens-security-grade-analyzing = قيد التحليل
tokens-security-grade-shielded = محمي
tokens-security-grade-safe = آمن
tokens-security-grade-caution = حذر
tokens-security-grade-vulnerable = عرضة للمخاطر
tokens-security-grade-unknown = غير معروف
tokens-security-metric-token-type = نوع الرمز
tokens-security-metric-total-holders = إجمالي الحاملين
tokens-security-metric-lp-providers = مزوّدو LP
tokens-security-metric-graph-insiders = المطّلعون في الرسم البياني
# $count is the number of insider wallets found in the holder graph.
tokens-security-insiders-detected = تم الرصد ({ $count })
tokens-security-insiders-clean = سليم
tokens-security-authority-mint = الإصدار
tokens-security-authority-freeze = التجميد
tokens-security-authority-immutable = غير قابل للتعديل
tokens-security-authority-mutable = قابل للتعديل
tokens-security-authority-revoked = ملغاة
tokens-security-authority-active = نشطة
tokens-security-holder-health-title = صحة الحاملين
tokens-security-holders-unique = فريد
tokens-security-creator-share = حصة المنشئ
tokens-security-gauge-top-10 = أكبر 10
tokens-security-concentration-unknown = غير معروف
tokens-security-concentration-critical = حرج
tokens-security-concentration-high = مرتفع
tokens-security-concentration-moderate = متوسط
tokens-security-concentration-healthy = صحي
tokens-security-transfer-title = ضريبة التحويل
tokens-security-transfer-no-fee = بلا رسوم
tokens-security-transfer-fee-percentage = نسبة الرسوم
tokens-security-transfer-max-fee = الحد الأقصى للرسوم
tokens-security-transfer-authority = صلاحية الرسوم
# $percent is the formatted transfer fee percentage.
tokens-security-transfer-note = تُفرض رسوم بنسبة { $percent } على كل عملية تحويل.
tokens-security-transfer-none = لم يتم رصد رسوم تحويل.
tokens-security-risks-title = مخاطر الأمان
tokens-security-risks-none = لم يتم اكتشاف مخاطر أمنية.
tokens-security-risk-fallback-name = إشارة أمان
tokens-security-risks-critical = حرجة: { $count }
tokens-security-risks-warnings =
    { $count ->
        [zero] { $count } تحذير
        [one] { $count } تحذير
        [two] { $count } تحذيران
        [few] { $count } تحذيرات
        [many] { $count } تحذيرًا
       *[other] { $count } تحذير
    }
tokens-security-risks-info = معلومات: { $count }
tokens-security-risks-incidents =
    { $count ->
        [zero] تم رصد { $count } حادثة
        [one] تم رصد { $count } حادثة
        [two] تم رصد { $count } حادثتين
        [few] تم رصد { $count } حوادث
        [many] تم رصد { $count } حادثةً
       *[other] تم رصد { $count } حادثة
    }
tokens-security-top-holders-title = أكبر الحاملين
# $percent is the formatted share of supply held by the top holders.
tokens-security-top-holders-concentration = تركّز { $percent }
tokens-security-insider = مطّلع

# Token details dialog: overview tab (ui/token_details/overview_tab.js)
# 5M/1H/6H/24H period codes and the chart timeframe buttons are id codes shared
# with the chart and stay as sent.

tokens-overview-chart-checking = جارٍ فحص البيانات…
tokens-overview-banner-open = فتح لافتة الرمز
tokens-overview-headline-label = مقاييس السوق الرئيسية
tokens-overview-price = السعر
tokens-overview-market-cap = القيمة السوقية
tokens-overview-liquidity = السيولة
tokens-overview-volume = حجم التداول
tokens-overview-volume-24h = حجم 24H
tokens-overview-no-tags = لا توجد وسوم
tokens-overview-info-title = معلومات الرمز
tokens-overview-profile = الملف المنشور
tokens-overview-fact-mint = الإصدار
tokens-overview-fact-decimals = الخانات العشرية
tokens-overview-fact-age = العمر
tokens-overview-fact-dex = DEX
tokens-overview-fact-holders = الحاملون
tokens-overview-fact-top-10 = حيازة أكبر 10
tokens-overview-tags = الوسوم
tokens-overview-liquidity-title = السيولة والسوق
tokens-overview-fact-fdv = FDV
tokens-overview-fact-pool-native = { -sol } المجمع
tokens-overview-fact-pool-token = رمز المجمع
tokens-overview-pool = مجمع السيولة
tokens-overview-pulse-title = نبض السوق
tokens-overview-activity-title = نشاط المعاملات
# $percent is the formatted share of buys among the 24h transactions.
tokens-overview-buy-share = شراء { $percent }
# $ratio is the formatted buy-to-sell ratio.
tokens-overview-buy-sell-ratio = شراء/بيع { $ratio }
tokens-overview-buys-24h = الشراء 24H
tokens-overview-sells-24h = البيع 24H
tokens-overview-net-flow = صافي التدفق
tokens-overview-total-24h = إجمالي 24H
tokens-overview-average-24h = متوسط 24H
tokens-overview-spike-5m = قفزة 5M
# $amount is the formatted average number of transactions per hour.
tokens-overview-rate-per-hour = { $amount }/ساعة
# $amount is the formatted average number of transactions per minute.
tokens-overview-rate-per-minute = { $amount }/دقيقة
# $factor is the formatted ratio of the 5-minute rate to the 1-hour rate.
tokens-overview-spike-factor = { $factor }×
# Tooltip of one activity row. Counts and percentages are formatted; "—" marks a missing value.
tokens-overview-flow-counts = الشراء: { $buys } ({ $buyPercent })، البيع: { $sells } ({ $sellPercent })، الإجمالي: { $total }
tokens-overview-flow-no-data = لا توجد بيانات معاملات

# Token details dialog: pools tab (ui/token_details/pools_links_tab.js)
# DEX names in pool data render as sent.

tokens-pools-empty-title = لا توجد مجمعات سيولة
tokens-pools-empty-message = لم يتم اكتشاف مجمعات سيولة لهذا الرمز.
tokens-pools-unknown = غير معروف
tokens-pools-unknown-dex = DEX غير معروف
tokens-pools-liquidity = السيولة
tokens-pools-volume-24h = حجم التداول (24h)
tokens-pools-base-role = دور الأساس
tokens-pools-quote-role = دور المقابل
tokens-pools-canonical-title = المجمع المرجعي
tokens-pools-canonical = مرجعي
tokens-pools-dex = DEX
tokens-pools-summary-title = ملخص المجمعات
tokens-pools-breakdown-title = التوزيع حسب DEX
tokens-pools-all-title = جميع المجمعات
tokens-pools-updated = آخر تحديث
tokens-pools-role-base = أساس
tokens-pools-role-quote = مقابل
tokens-pools-role-unknown = غير معروف
tokens-pools-reserves = حسابات الاحتياطي
tokens-pools-no-reserves = لا توجد حسابات احتياطي
tokens-pools-address-pool = المجمع
    .title = نسخ عنوان المجمع
tokens-pools-address-base = إصدار الأساس
    .title = نسخ إصدار الأساس
tokens-pools-address-quote = إصدار المقابل
    .title = نسخ إصدار المقابل
    .title = نسخ الإصدار المقترن

# Token details dialog: links tab (ui/token_details/pools_links_tab.js)

tokens-links-empty = لا تتوفر مواقع رسمية أو روابط اجتماعية لهذا الرمز.
tokens-links-info-title = معلومات الرمز
tokens-links-mint-address = عنوان الإصدار
tokens-links-data-source = مصدر البيانات
tokens-links-security = الأمان
tokens-links-profile-title = ملف الرمز
tokens-links-profile-published-title = محتوى الملف المنشور
tokens-links-profile-published-note = الوسائط والوصف والروابط الرسمية محتوى ملف مدفوع تتم مراجعته قبل النشر. لا يؤكد هذا ملكية الرمز أو سلامته.
tokens-links-profile-create-note = أضف شعارًا ووصفًا للمشروع وروابط رسمية تمت مراجعتها إلى الملف العام لهذا الرمز.
tokens-links-profile-update-hint = حدّث ملف هذا الرمز على screenerbot.io
tokens-links-profile-create-hint = أنشئ ملفًا للرمز على screenerbot.io
tokens-links-profile-update = تحديث الملف
tokens-links-profile-create = إنشاء ملف
tokens-links-media-title = الأصول الوسائطية
tokens-links-media-fallback-symbol = الرمز
tokens-links-media-logo = الشعار
tokens-links-media-banner = اللافتة
# $symbol is the token symbol.
tokens-links-media-banner-alt = لافتة { $symbol }
tokens-links-media-open = فتح الصورة
tokens-links-description-title = الوصف
tokens-links-explorers-title = المستكشفات والتحليلات
tokens-links-websites-title = المواقع الرسمية
tokens-links-socials-title = وسائل التواصل الاجتماعي
tokens-links-explorer-solana-explorer = { -solana-explorer }
tokens-links-explorer-geckoterminal = { -geckoterminal }
tokens-links-explorer-dextools = { -dextools }
tokens-links-explorer-coingecko = { -coingecko }
tokens-links-explorer-jupiter-swap = { -jupiter } Swap
tokens-links-social-twitter = { -twitter } / { -x }
tokens-links-social-x = { -x } ({ -twitter })
tokens-links-social-telegram = { -telegram }
tokens-links-social-discord = { -discord }
tokens-links-social-medium = { -medium }
tokens-links-social-github = { -github }
tokens-links-social-youtube = { -youtube }
tokens-links-social-reddit = { -reddit }
tokens-links-social-facebook = { -facebook }
tokens-links-social-instagram = { -instagram }
tokens-links-social-linkedin = { -linkedin }
tokens-links-social-tiktok = { -tiktok }
tokens-links-social-fallback = اجتماعي

# Token details dialog: frame, header and data sources (ui/token_details_dialog.js)

tokens-dialog-tab-overview = نظرة عامة
tokens-dialog-tab-security = الأمان
tokens-dialog-tab-positions = المراكز
tokens-dialog-tab-pools = المجمعات
tokens-dialog-tab-links = الروابط
tokens-dialog-tab-transactions = المعاملات
tokens-dialog-sections = أقسام تفاصيل الرمز
tokens-dialog-close =
    .title = إغلاق (ESC)
    .aria-label = إغلاق تفاصيل الرمز
tokens-dialog-unknown-symbol = غير معروف
tokens-dialog-unknown-name = رمز غير معروف
tokens-dialog-market-summary = ملخص السوق
tokens-dialog-price-loading = جارٍ تحميل السعر
tokens-dialog-unit-native = { -sol }
tokens-dialog-market-metrics = مقاييس السوق
tokens-dialog-metric-market-cap = القيمة السوقية
tokens-dialog-metric-volume-24h = حجم التداول (24h)
# $change is the formatted 24 hour price change.
tokens-dialog-change-24h = التغيّر خلال 24 ساعة { $change }
tokens-dialog-buy = شراء
    .title = شراء هذا الرمز
tokens-dialog-sell = بيع
    .title = بيع المركز
tokens-dialog-sell-unavailable = لا يوجد مركز مفتوح للبيع
tokens-dialog-details = التفاصيل
tokens-dialog-sources = المصادر
tokens-dialog-sources-status = حالة مصادر البيانات
tokens-dialog-updated-label = آخر تحديث
tokens-dialog-just-now = الآن
# $time is a relative or clock time.
tokens-dialog-updated-at = آخر تحديث { $time }
tokens-dialog-updated-unavailable = وقت التحديث غير متاح
tokens-dialog-error-title = تعذّر تحميل بيانات الرمز
tokens-dialog-waiting-token = بانتظار بيانات الرمز…
tokens-dialog-loading-overview = جارٍ تحميل النظرة العامة…
tokens-dialog-loading-security = جارٍ تحميل الأمان…
tokens-dialog-loading-pools = جارٍ تحميل المجمعات…
tokens-dialog-loading-links = جارٍ تحميل الروابط…
tokens-dialog-chart-still-checking = لا تتوفر بيانات مخطط بعد — لا يزال الفحص جاريًا…
tokens-dialog-no-data = لا تتوفر بيانات
tokens-dialog-source-token = الرمز
tokens-dialog-source-market = السوق
tokens-dialog-source-security = الأمان
tokens-dialog-source-chart = المخطط
tokens-dialog-status-pending = بالانتظار
tokens-dialog-status-loading = جارٍ التحميل
tokens-dialog-status-ready = جاهز
tokens-dialog-status-unavailable = غير متاح
tokens-dialog-status-cached = مخزّن مؤقتًا
# $source is a data source name and $status its state, for example "Market data: Ready".
tokens-dialog-source-summary = بيانات { $source }: { $status }
tokens-dialog-badge-pool-price = سعر المجمع
tokens-dialog-badge-pool-price-hint = السعر من مجمع السيولة الفوري على السلسلة
tokens-dialog-badge-api-price = سعر API
tokens-dialog-badge-api-price-hint = السعر من بيانات السوق المخزنة مؤقتًا (API)
tokens-dialog-badge-profile = الملف المنشور
    .title = محتوى ملف مدفوع تمت مراجعته للنشر؛ وليس تدقيقًا أو تحققًا من الملكية.
tokens-dialog-badge-low-risk-hint = مخاطر منخفضة وفق درجة { -rugcheck } الحالية؛ وليس تحققًا من الهوية.
tokens-dialog-badge-immutable = غير قابل للتعديل
tokens-dialog-badge-mutable = قابل للتعديل
tokens-dialog-badge-position = مركز
tokens-dialog-badge-blacklisted = في القائمة السوداء

# Tokens page: sub-tabs (scripts/pages/tokens/constants.js)
# Ids are the view values of /api/tokens/list.

tokens-view-favorites = المفضلة
tokens-view-pool = خدمة المجمعات
tokens-view-no-market = بلا بيانات سوق
tokens-view-all = جميع الرموز
tokens-view-passed = الناجحة
tokens-view-rejected = المرفوضة
tokens-view-blacklisted = القائمة السوداء
tokens-view-positions = المراكز
tokens-view-recent = الأحدث
tokens-view-ohlcv = بيانات OHLCV
# Empty token table per view (TOKEN_VIEW_EMPTY_LABELS)
tokens-view-pool-empty = لا توجد رموز مسعّرة بعد
    .message = تظهر الرموز هنا بعد اجتيازها التصفية وحساب سعر مجمّعها.
tokens-view-no-market-empty = لا توجد رموز بلا بيانات سوق
    .message = تبقى الرموز هنا ما دامت مصادر بيانات السوق لم تُدرجها بعد.
tokens-view-all-empty = لم تُكتشف رموز بعد
    .message = يظهر هنا كل رمز يعثر عليه الاكتشاف، أيًّا كانت نتيجة تصفيته.
tokens-view-passed-empty = لم يجتز أي رمز التصفية
    .message = تظهر هنا الرموز التي تجتاز كل المرشحات المفعّلة. راجع صفحة التصفية إذا بقيت القائمة فارغة.
tokens-view-rejected-empty = لا توجد رموز مرفوضة
    .message = تظهر هنا الرموز التي تفشل في أحد المرشحات مع سبب الرفض.
tokens-view-blacklisted-empty = لا توجد رموز في القائمة السوداء
    .message = تظهر هنا الرموز المستبعدة من التداول، بواسطتك أو بواسطة فحوص الأمان.
tokens-view-positions-empty = لا توجد رموز في مراكز
    .message = تظهر هنا الرموز المحتفظ بها في مراكز مفتوحة.
tokens-view-recent-empty = لا توجد رموز حديثة
    .message = تظهر هنا الرموز المكتشفة حديثًا فور العثور عليها.
tokens-ohlcv-empty = لا توجد بيانات رسم بياني بعد
    .message = تظهر الرموز هنا عندما يبدأ جمع شموعها.

# Tokens page: token cell (scripts/pages/tokens/formatters.js)

tokens-cell-logo-enlarge = انقر للتكبير
# $boosts is the formatted active boost count.
tokens-boost-title = التعزيزات على screenerbot.io: { $boosts }
tokens-cell-action-add =
    .title = إضافة إلى المركز (DCA)
    .aria-label = إضافة إلى المركز
tokens-cell-action-sell =
    .title = بيع (كامل أو جزئي بنسبة %)
    .aria-label = بيع الرمز
tokens-cell-action-buy =
    .title = شراء مركز
    .aria-label = شراء الرمز
tokens-cell-external-links =
    .title = روابط خارجية
    .aria-label = روابط خارجية

# Tokens page: table states shared by the token lists (scripts/pages/tokens/*.js)

tokens-table-loading-title = جارٍ تحميل الرموز…
tokens-table-loading-description = جارٍ تحضير عرض الرموز المحدد.
tokens-table-retry-hint = بدّل التبويب أو حاول مرة أخرى.
tokens-filter-all = الكل

# Tokens page: favorites (scripts/pages/tokens/favorites.js)

tokens-favorites-load-failed-title = تعذّر تحميل المفضلة
tokens-favorites-load-failed-toast = تعذّر تحميل المفضلة
tokens-favorites-total = إجمالي المفضلة
tokens-favorites-empty-title = لا توجد مفضلة بعد
    .message = ضع نجمة على رمز في أي قائمة لإبقائه هنا.

# Tokens page: OHLCV data view (scripts/pages/tokens/ohlcv.js)
# Status ids come from /api/ohlcv/tokens; priority ids are Priority::as_str in src/ohlcvs/types.rs.

tokens-column-token = الرمز
tokens-column-status = الحالة
tokens-ohlcv-delete =
    .title = حذف بيانات OHLCV
    .aria-label = حذف بيانات OHLCV
tokens-ohlcv-status-active = نشط
tokens-ohlcv-status-inactive = غير نشط
tokens-ohlcv-priority-critical = حرجة
tokens-ohlcv-priority-high = عالية
tokens-ohlcv-priority-medium = متوسطة
tokens-ohlcv-priority-low = منخفضة
tokens-ohlcv-column-priority = الأولوية
tokens-ohlcv-column-backfill = ملء البيانات السابقة
tokens-ohlcv-column-data-span = امتداد البيانات
tokens-ohlcv-column-gaps = الفجوات
tokens-ohlcv-column-pools = المجمعات
tokens-ohlcv-column-last-fetch = آخر جلب
# $timeframe is a timeframe code such as 1h.
tokens-ohlcv-timeframe-complete = { $timeframe }: مكتمل
tokens-ohlcv-timeframe-pending = { $timeframe }: قيد الانتظار
tokens-ohlcv-load-failed-title = تعذّر تحميل بيانات OHLCV
tokens-ohlcv-load-failed-toast = تعذّر تحميل بيانات OHLCV
tokens-ohlcv-total = إجمالي الرموز
tokens-ohlcv-active = نشطة
tokens-ohlcv-db-size = حجم قاعدة البيانات
tokens-ohlcv-cleanup = تنظيف غير النشطة
tokens-ohlcv-delete-title = حذف بيانات OHLCV
# $token is the token symbol.
tokens-ohlcv-delete-token-message = هل تريد حذف جميع بيانات OHLCV للرمز { $token }؟
tokens-ohlcv-delete-done = تم الحذف — الشموع: { $candles }، المجمعات: { $pools }
tokens-ohlcv-delete-failed = تعذّر حذف بيانات OHLCV
tokens-ohlcv-cleanup-title = حذف الرموز غير النشطة
tokens-ohlcv-cleanup-message = حذف الرموز غير النشطة الأقدم من عدد الساعات المحدد
tokens-ohlcv-cleanup-placeholder = الساعات...
tokens-ohlcv-cleanup-invalid = يرجى إدخال رقم موجب
tokens-ohlcv-cleanup-done =
    { $count ->
        [zero] تم تنظيف { $count } رمز غير نشط
        [one] تم تنظيف { $count } رمز غير نشط
        [two] تم تنظيف { $count } رمزين غير نشطين
        [few] تم تنظيف { $count } رموز غير نشطة
        [many] تم تنظيف { $count } رمزًا غير نشط
       *[other] تم تنظيف { $count } رمز غير نشط
    }
tokens-ohlcv-cleanup-failed = تعذّر تنظيف بيانات OHLCV

# Tokens page: token lists (scripts/pages/tokens.js)
# The list statuses shown in the Status column come from row flags, not ids.

tokens-summary-total = الإجمالي
tokens-summary-pool-priced = بسعر مجمع السيولة
tokens-summary-positions = المراكز
tokens-summary-blacklisted = في القائمة السوداء
tokens-search-placeholder = ابحث بالرمز المختصر أو الإصدار...
tokens-table-waiting-title = لا يزال تحميل الرموز جاريًا...
tokens-table-waiting-description = بانتظار استجابة الخلفية. ستتم إعادة المحاولة تلقائيًا.
tokens-load-failed-toast = تعذّر تحميل الرموز
tokens-row-data-missing = لم يتم العثور على بيانات الرمز
tokens-column-price-sol = السعر ({ -sol })
tokens-column-liquidity = السيولة
tokens-column-volume-24h = حجم 24h
tokens-column-fdv = FDV
tokens-column-market-cap = القيمة السوقية
tokens-column-change-1h = 1h
tokens-column-change-24h = 24h
tokens-column-txns-5m = معاملات 5m
tokens-column-txns-1h = معاملات 1h
tokens-column-txns-6h = معاملات 6h
tokens-column-txns-24h = معاملات 24h
tokens-column-risk-score = درجة المخاطر
tokens-column-reject-reason = سبب الرفض
tokens-column-blacklist-reason = سبب الإدراج في القائمة السوداء
tokens-column-updated = آخر تحديث
tokens-column-birth = الميلاد
tokens-column-first-seen = أول ظهور
tokens-badge-price = السعر
tokens-badge-ohlcv = OHLCV
tokens-badge-position = مركز
tokens-badge-blacklisted = في القائمة السوداء
tokens-badge-blacklisted-title = رمز في القائمة السوداء
# $reasons is the list of blacklist categories, reasons and details.
tokens-badge-blacklisted-reasons = في القائمة السوداء: { $reasons }
tokens-links-menu-copy-mint = نسخ الإصدار
tokens-links-copy-failed = تعذّر نسخ الإصدار
tokens-lightbox-token-age = عمر الرمز

# Global search dialog (scripts/ui/search_dialog.js)

tokens-search-placeholder-dialog = ابحث بالاسم أو الرمز المختصر أو الإصدار...
tokens-search-input-label = البحث في الرموز
tokens-search-results-label = نتائج البحث
tokens-search-tip-nav = تنقّل
tokens-search-tip-open = فتح
tokens-search-tip-close = إغلاق
tokens-search-failed = فشل البحث
# $message is the failure text.
tokens-search-error = خطأ: { $message }
tokens-search-clear =
    .title = مسح البحث
    .aria-label = مسح البحث
tokens-search-recent = الأخيرة
tokens-search-recent-label = عمليات البحث الأخيرة
tokens-search-lists-label = قوائم الرموز
tokens-search-tab-trending = الرائجة
tokens-search-kinds = الاسم · الرمز · عنوان السك
tokens-search-empty-trending = تظهر الرموز الرائجة بعد أن يسعّر البوت مجمّعاته الأولى.
tokens-search-empty-positions = لا توجد مراكز مفتوحة حاليًا.
tokens-search-empty-favorites = ضع نجمة على رمز ليظهر هنا في بحثك التالي.
tokens-search-empty-boosted = لا يوجد رمز معزَّز حاليًا.
tokens-search-list-failed = تعذّر تحميل هذه القائمة.
tokens-search-searching = جارٍ البحث في الأسواق…
# $count is the number of tokens found.
tokens-search-result-count =
    { $count ->
        [zero] { $count } نتيجة
        [one] نتيجة واحدة
        [two] نتيجتان
        [few] { $count } نتائج
        [many] { $count } نتيجة
       *[other] { $count } نتيجة
    }
tokens-search-order = الأقرب تطابقًا أولًا، ثم حجم التداول خلال 24h
tokens-search-metric-mc = ق.س
    .title = القيمة السوقية
tokens-search-metric-fdv = FDV
    .title = القيمة المخففة بالكامل
tokens-search-metric-liq = سيولة
    .title = السيولة
tokens-search-metric-vol = حجم
    .title = حجم التداول خلال 24h
tokens-search-more =
    .title = إجراءات أخرى
    .aria-label = إجراءات أخرى
# $query is the text the user typed.
tokens-search-no-match = لا يوجد رمز يطابق «{ $query }».

# Featured dialog (scripts/ui/featured_dialog.js)
# Category and source ids are those of CATEGORIES; provider names are terms.

tokens-featured-category-boosted = معزَّزة
tokens-featured-category-jupiter-organic = الأعلى نشاطًا عضويًا في { -jupiter }
tokens-featured-category-jupiter-traded = الأكثر تداولًا في { -jupiter }
tokens-featured-category-dexscreener-trending = الرائجة في { -dexscreener }
tokens-featured-source-jupiter = { -jupiter }
tokens-featured-source-dexscreener = { -dexscreener }
tokens-featured-note-boosted = مروَّج لها من فرقها
tokens-featured-security-risky = محفوفة بالمخاطر
tokens-featured-load-failed = تعذّر تحميل المميّزة
# $message is the failure text.
tokens-featured-network-error = خطأ في الشبكة: { $message }
tokens-featured-title = المميّزة
tokens-featured-subtitle = الرموز المعزَّزة أولًا، ثم الرائجة عبر Solana
tokens-featured-boost = عزّز رمزًا
tokens-featured-close =
    .title = إغلاق (ESC)
tokens-featured-loading = جارٍ تحميل المميّزة والرائجة...
tokens-featured-error-hint = تحقق من الاتصال أو حاول مرة أخرى
tokens-featured-empty = لا توجد رموز متاحة حاليًا
tokens-featured-count =
    { $count ->
        [zero] { $count } رمز
        [one] { $count } رمز
        [two] { $count } رمزان
        [few] { $count } رموز
        [many] { $count } رمزًا
       *[other] { $count } رمز
    }
tokens-featured-stat-market-cap = القيمة السوقية
tokens-featured-stat-liquidity = السيولة
tokens-featured-stat-volume = حجم 24H
tokens-featured-stat-holders = الحاملون
tokens-featured-stat-txns = معاملات 24H
# $symbol is the token symbol.
tokens-featured-buy = شراء
    .title = شراء { $symbol }
# $score is the normalized security score out of 100, where higher is safer.
tokens-featured-security-score = درجة الأمان: { $score }/100
tokens-featured-social-website = الموقع
tokens-featured-social-twitter = { -twitter }

# Featured row (scripts/ui/featured_row.js)

tokens-featured-row-view-all = الكل
    .title = فتح عرض المميّزة الكامل
tokens-featured-row-scroll-start =
    .aria-label = عرض الرموز السابقة
tokens-featured-row-scroll-end =
    .aria-label = عرض المزيد من الرموز
tokens-featured-row-empty = لا توجد رموز مميّزة
# $name and $symbol identify the token; $boosts is the formatted active boost count.
tokens-featured-row-title = { $name } ({ $symbol })
tokens-featured-row-boosted-title = { $name } ({ $symbol }) — التعزيزات: { $boosts }

# Pool selector dialog (scripts/ui/pool_selector.js)

tokens-pool-selector-title = اختيار مجمع السيولة
tokens-pool-selector-loading = جارٍ تحميل المجمعات...
tokens-pool-selector-empty = لم يتم العثور على مجمعات سيولة لهذا الرمز
# $message is the failure text.
tokens-pool-selector-load-failed = تعذّر تحميل المجمعات: { $message }
tokens-pool-selector-count =
    { $count ->
        [zero] تم العثور على { $count } مجمع سيولة
        [one] تم العثور على { $count } مجمع سيولة
        [two] تم العثور على { $count } مجمعي سيولة
        [few] تم العثور على { $count } مجمعات سيولة
        [many] تم العثور على { $count } مجمع سيولة
       *[other] تم العثور على { $count } مجمع سيولة
    }
# $amount is the formatted pool liquidity in USD.
tokens-pool-selector-liquidity = { $amount } سيولة
    .title = السيولة
# $amount is the formatted 24 hour pool volume in USD.
tokens-pool-selector-volume = { $amount } 24h
    .title = حجم التداول (24h)

# Token identity chips and address rows (scripts/ui/token_identity.js)

tokens-identity-unknown-asset = أصل غير معروف
tokens-identity-copy-address =
    .title = نسخ العنوان
    .aria-label = نسخ العنوان
tokens-identity-copy-signature =
    .title = نسخ التوقيع
    .aria-label = نسخ التوقيع

tokens-rugcheck-risk-single-holder-ownership = استحواذ حامل واحد
    .description = يمتلك حامل واحد جزءًا كبيرًا من معروض الرمز.
tokens-rugcheck-risk-low-liquidity = سيولة منخفضة
    .description = يحتوي مجمع الرمز على سيولة قليلة.
tokens-rugcheck-risk-few-lp-providers = قلة مزوّدي LP
    .description = عدد قليل فقط من المستخدمين يوفّر السيولة.
tokens-rugcheck-risk-high-holder-concentration = تركّز مرتفع للحاملين
    .description = يمتلك أكبر 10 حاملين أكثر من 50% من معروض الرمز.
tokens-rugcheck-risk-top-10-holders-high-ownership = ملكية مرتفعة لأكبر 10 حاملين
    .description = يمتلك أكبر 10 حاملين أكثر من 70% من معروض الرمز.
tokens-rugcheck-risk-high-ownership = ملكية مرتفعة
    .description = يمتلك كبار الحاملين أكثر من 80% من معروض الرمز.
tokens-rugcheck-risk-creator-rug-history = سجل المنشئ في سحب السجادة
    .description = للمنشئ سجل في تنفيذ عمليات سحب السجادة على الرموز.
tokens-rugcheck-risk-large-lp-unlocked = جزء كبير من LP غير مقفل
    .description = جزء كبير من رموز LP غير مقفل، ما يسمح للمالك بسحب السيولة في أي وقت.
tokens-rugcheck-risk-mutable-metadata = بيانات وصفية قابلة للتعديل
    .description = يستطيع المالك تعديل البيانات الوصفية للرمز.
tokens-rugcheck-risk-few-holders = قلة الحاملين
    .description = عدد المحافظ التي تحمل الرمز قليل.
tokens-rugcheck-risk-copycat-token = رمز مقلِّد
    .description = يستخدم هذا الرمز الرمز الاختصاري لرمز موثّق.
tokens-rugcheck-risk-fee-config-enabled = إعداد الرسوم مفعّل
    .description = يستطيع المالك تغيير الرسوم في أي وقت.
tokens-rugcheck-risk-high-holder-correlation = ارتباط مرتفع بين الحاملين
    .description = يمتلك كبار الحاملين كميات متقاربة من المعروض.
tokens-rugcheck-risk-freeze-authority-enabled = صلاحية التجميد مفعّلة
    .description = يمكن تجميد الرموز ومنع تداولها.
tokens-rugcheck-risk-mint-authority-enabled = صلاحية الإصدار مفعّلة
    .description = يستطيع المالك إصدار مزيد من الرموز.
tokens-rugcheck-risk-missing-file-metadata = ملف البيانات الوصفية مفقود
    .description = لا يوجد ملف بيانات وصفية مرتبط بهذا الرمز.
tokens-rugcheck-risk-high-market-cap-per-holder = قيمة سوقية مرتفعة لكل حامل
    .description = القيمة السوقية مرتفعة جدًا مقارنةً بعدد الحاملين.
tokens-rugcheck-risk-symbol-mismatch = عدم تطابق الرمز الاختصاري
    .description = لا يطابق الرمز الاختصاري للرمز ملف بياناته الوصفية.
tokens-rugcheck-risk-name-mismatch = عدم تطابق الاسم
    .description = لا يطابق اسم الرمز ملف بياناته الوصفية.
tokens-rugcheck-risk-permanent-control-enabled = التحكم الدائم مفعّل
    .description = يستطيع منشئ الرمز التحكم في جميع الرموز بشكل دائم.
tokens-rugcheck-risk-missing-metadata = البيانات الوصفية مفقودة
    .description = لم يُعثر على بيانات وصفية لهذا الرمز.
tokens-rugcheck-risk-lp-unlock-soon = فتح قفل LP قريبًا
    .description = سيُفتح قفل رموز LP قريبًا، ما يسمح للمالك بسحب السيولة.
tokens-rugcheck-risk-lp-vault-unlocked = خزنة LP غير مقفلة
    .description = يمكن استرداد رموز LP الموجودة في الخزنة.
tokens-rugcheck-risk-mint-authority-locked = صلاحية الإصدار مقفلة
    .description = إصدار رموز جديدة مقفل.
tokens-rugcheck-risk-high-transfer-fee = رسوم تحويل مرتفعة
    .description = تُفرض ضريبة مرتفعة على كل تحويل لهذا الرمز.
