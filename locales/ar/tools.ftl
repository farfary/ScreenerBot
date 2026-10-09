# Tools page: the shell, the token tools, the trading tools, the wallet tools and the
# multi-wallet tools.

## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = الأدوات
tools-category-wallet = المحفظة
tools-category-token = الرمز
tools-category-single-token = رمز واحد
tools-category-utilities = أدوات مساعدة
tools-sidebar-hint = اختر أداة للبدء
tools-help-button =
    .aria-label = عرض المساعدة لهذه الأداة
tools-help-unavailable = المساعدة غير متاحة
tools-placeholder-title = اختر أداة
tools-placeholder-subtitle = اختر أداة من الشريط الجانبي للبدء
tools-placeholder-hint-wallets = تساعدك أدوات المحفظة على إدارة محافظ Solana الخاصة بك
tools-placeholder-hint-secure = جميع العمليات محمية وقابلة للتراجع حيثما أمكن

# Status dot tooltip of a tool in the navigation. Ids are the nav `data-status` values.
tools-status-ready = جاهزة للاستخدام
tools-status-coming = قريبًا
tools-status-beta = تجريبية - قد تحتوي على أخطاء
tools-status-disabled = معطّلة حاليًا
# Badge of a tool that is not available yet.
tools-status-badge-coming = قريبًا
tools-status-badge-beta = تجريبي
tools-toast-coming-soon = هذه الأداة ستتوفر قريبًا
tools-toast-disabled = هذه الأداة معطّلة حاليًا
tools-setup-gate-title = هذه الأداة تتطلب محفظة

## Tool names. `-title` names the tool in the navigation and the header, `-summary` is the
## navigation line, `-description` is the header line. Ids are the tool ids of the registry.

tools-tool-wallet-cleanup-title = تنظيف المحفظة
tools-tool-wallet-cleanup-summary = إغلاق حسابات ATA الفارغة
tools-tool-wallet-cleanup-description = إغلاق حسابات الرموز المرتبطة (ATA) الفارغة لاسترداد { -sol }
tools-tool-burn-tokens-title = حرق الرموز
tools-tool-burn-tokens-summary = إتلاف الرموز نهائيًا
tools-tool-burn-tokens-description = إتلاف الرموز من محفظتك نهائيًا
tools-tool-token-analyzer-title = محلل الرموز
tools-tool-token-analyzer-summary = تحليل معمّق للرمز
tools-tool-token-analyzer-description = تحليل معمّق لأي رمز على Solana بأبعاد متعددة
tools-tool-create-token-title = إنشاء رمز
tools-tool-create-token-summary = إصدار رمز SPL جديد
tools-tool-create-token-description = إصدار رمز SPL جديد على Solana
tools-tool-trade-watcher-title = مراقب الصفقات
tools-tool-trade-watcher-summary = مراقبة الصفقات والتنفيذ التلقائي
tools-tool-trade-watcher-description = مراقبة صفقات الرمز وتشغيل إجراءات شراء/بيع تلقائية
tools-tool-token-watch-title = مراقبة الحاملين
tools-tool-token-watch-summary = تتبع حاملي الرمز الجدد
tools-tool-token-watch-description = تتبع ومراقبة حاملي الرمز الجدد في الوقت الفعلي
tools-tool-buy-multi-wallets-title = شراء متعدد
tools-tool-buy-multi-wallets-summary = تنسيق عمليات الشراء عبر المحافظ
tools-tool-buy-multi-wallets-description = تنفيذ أوامر شراء منسقة عبر عدة محافظ بمبالغ عشوائية
tools-tool-sell-multi-wallets-title = بيع متعدد
tools-tool-sell-multi-wallets-summary = تنسيق عمليات البيع عبر المحافظ
tools-tool-sell-multi-wallets-description = تنفيذ أوامر بيع منسقة عبر عدة محافظ مع تجميع { -sol }
tools-tool-wallet-consolidation-title = تجميع المحافظ
tools-tool-wallet-consolidation-nav-title = تجميع
tools-tool-wallet-consolidation-summary = تجميع أموال المحافظ
tools-tool-wallet-consolidation-description = تجميع { -sol } والرموز من المحافظ الفرعية إلى المحفظة الرئيسية
tools-tool-airdrop-checker-title = فاحص الإيردروب
tools-tool-airdrop-checker-summary = فحص الإيردروبات المعلّقة
tools-tool-airdrop-checker-description = فحص الإيردروبات المعلّقة والمكافآت القابلة للمطالبة
tools-tool-wallet-generator-title = مولّد المحافظ
tools-tool-wallet-generator-summary = توليد أزواج مفاتيح جديدة
tools-tool-wallet-generator-description = توليد أزواج مفاتيح Solana جديدة بأمان

## Shared by the tools

tools-validation-mint-required = يرجى إدخال عنوان إصدار الرمز
tools-validation-mint-invalid = يرجى إدخال عنوان إصدار صالح
tools-validation-mint-format = صيغة عنوان إصدار الرمز غير صالحة

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = تفاصيل الرمز
tools-create-token-name-label = اسم الرمز
tools-create-token-name-input =
    .placeholder = رمزي
tools-create-token-symbol-label = الرمز المختصر
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = الخانات العشرية
tools-create-token-supply-label = المعروض الأولي
tools-create-token-description-label = الوصف
tools-create-token-description-input =
    .placeholder = وصف الرمز...
tools-create-token-image-title = صورة الرمز
tools-create-token-image-drop = أفلت الصورة هنا أو انقر للرفع
tools-create-token-image-hint = الموصى به: PNG بأبعاد 512x512
tools-create-token-action-preview = معاينة
tools-create-token-action-create = إنشاء رمز

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = جارٍ تحميل الإعدادات...
tools-holder-watch-saved = تم حفظ إعدادات مراقبة الحاملين
tools-holder-watch-save-failed = تعذّر حفظ الإعدادات
tools-holder-watch-save-error = حدث خطأ أثناء حفظ الإعدادات
tools-holder-watch-settings-title = إعدادات مراقبة الحاملين
tools-holder-watch-enabled-label = تفعيل مراقبة الحاملين
tools-holder-watch-interval-label = فاصل الفحص (بالثواني)
tools-holder-watch-interval-hint = عدد مرات فحص أعداد الحاملين (10-3600 ث)
tools-holder-watch-max-tokens-label = الحد الأقصى للرموز المراقبة
tools-holder-watch-max-tokens-hint = الحد الأقصى للرموز المراقبة في وقت واحد
tools-holder-watch-notify-new-label = إشعار عند ظهور حاملين جدد
tools-holder-watch-notify-drop-label = إشعار عند انخفاض الحاملين
tools-holder-watch-min-change-label = الحد الأدنى لتغيّر الحاملين
tools-holder-watch-min-change-hint = الحد الأدنى لتغيّر الحاملين الذي يطلق الإشعار
tools-holder-watch-drop-percent-label = حد انخفاض الحاملين (%)
tools-holder-watch-drop-percent-hint = نسبة الانخفاض التي تطلق التنبيه
tools-holder-watch-action-save = حفظ الإعدادات
tools-holder-watch-tokens-title = الرموز المراقبة
tools-holder-watch-token-input =
    .placeholder = أدخل عنوان إصدار الرمز...
tools-holder-watch-empty = لا توجد رموز قيد المراقبة
tools-holder-watch-empty-hint = أضف عنوان إصدار رمز أعلاه لبدء المراقبة
tools-holder-watch-coming-soon = ميزة مراقبة الرموز ستتوفر قريبًا

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = تحليل رمز
tools-analyzer-mint-input =
    .placeholder = الصق عنوان إصدار الرمز...
tools-analyzer-action-analyze = تحليل
tools-analyzer-action-analyzing = جارٍ التحليل...
tools-analyzer-action-copy-report = نسخ التقرير
tools-analyzer-loading = جارٍ تحليل الرمز...
tools-analyzer-failed = تعذّر تحليل الرمز
tools-analyzer-empty = أدخل عنوان إصدار رمز لتحليله
tools-analyzer-empty-hint = احصل على رؤى شاملة عن أي رمز على Solana
tools-analyzer-tab-overview = نظرة عامة
tools-analyzer-tab-security = الأمان
tools-analyzer-tab-market = السوق
tools-analyzer-tab-liquidity = السيولة
tools-analyzer-unknown-token = رمز غير معروف

# Token header actions.
tools-analyzer-favorite-add =
    .title = إضافة إلى المفضلة
    .aria-label = إضافة إلى المفضلة
tools-analyzer-favorite-already = موجود في المفضلة بالفعل
# $symbol is the token symbol.
tools-analyzer-favorite-added = تمت إضافة { $symbol } إلى المفضلة
tools-analyzer-favorite-failed = تعذّرت الإضافة إلى المفضلة
tools-analyzer-blacklist-add =
    .title = إضافة إلى القائمة السوداء
    .aria-label = إضافة إلى القائمة السوداء
tools-analyzer-blacklist-title = إضافة الرمز إلى القائمة السوداء
# $symbol is the token symbol.
tools-analyzer-blacklist-message = هل تريد إضافة { $symbol } إلى القائمة السوداء؟ سيُستبعد هذا الرمز من التداول.
tools-analyzer-blacklist-confirm = إضافة إلى القائمة السوداء
tools-analyzer-blacklisted = في القائمة السوداء
# $symbol is the token symbol.
tools-analyzer-blacklist-done = تمت إضافة { $symbol } إلى القائمة السوداء
tools-analyzer-blacklist-failed = تعذّرت إضافة الرمز إلى القائمة السوداء

# Overview tab.
tools-analyzer-card-quick-stats = إحصاءات سريعة
tools-analyzer-card-market-summary = ملخص السوق
tools-analyzer-card-token-info = معلومات الرمز
tools-analyzer-stat-holders = الحاملون
tools-analyzer-stat-decimals = الخانات العشرية
tools-analyzer-stat-safety-score = درجة الأمان
tools-analyzer-stat-pools = مجمعات السيولة
tools-analyzer-stat-volume-24h = حجم التداول (24h)
tools-analyzer-stat-change-24h = التغيّر (24h)
tools-analyzer-stat-market-cap = القيمة السوقية
tools-analyzer-stat-liquidity = السيولة
tools-analyzer-info-mint = عنوان الإصدار
tools-analyzer-info-description = الوصف
tools-analyzer-info-supply = المعروض

# Security tab.
tools-analyzer-security-empty = لا تتوفر بيانات أمان
tools-analyzer-security-empty-hint = تحليل الأمان غير متاح لهذا الرمز
tools-analyzer-card-safety-score = درجة الأمان
tools-analyzer-score-good = جيدة
tools-analyzer-score-moderate = متوسطة
tools-analyzer-score-risky = محفوفة بالمخاطر
# $score is the formatted raw risk score.
tools-analyzer-raw-score = درجة المخاطر الخام: { $score }
tools-analyzer-card-authorities = صلاحيات الرمز
tools-analyzer-authority-mint = صلاحية الإصدار
tools-analyzer-authority-freeze = صلاحية التجميد
tools-analyzer-authority-transfer-fee = رسوم التحويل
tools-analyzer-authority-mutable = قابل للتعديل
tools-analyzer-authority-active = نشطة
tools-analyzer-authority-revoked = ملغاة
tools-analyzer-card-holder-concentration = تركّز الحاملين
tools-analyzer-top-holders = يملكها أكبر 10 حاملين
# $count is the formatted number of risks.
tools-analyzer-risks-title = مخاطر الأمان ({ $count })
tools-analyzer-risks-title-none = مخاطر الأمان
tools-analyzer-risks-none = لم يتم اكتشاف مخاطر أمنية

# Market tab.
tools-analyzer-market-empty = لا تتوفر بيانات السوق
tools-analyzer-market-empty-hint = بيانات السوق غير متاحة لهذا الرمز
tools-analyzer-card-price = السعر الحالي
tools-analyzer-card-price-changes = تغيّرات السعر
tools-analyzer-card-volume = حجم التداول
tools-analyzer-card-transactions = معاملات 24h
tools-analyzer-card-valuation = التقييم
tools-analyzer-stat-window-1h = 1h
tools-analyzer-stat-window-6h = 6h
tools-analyzer-stat-window-24h = 24h
tools-analyzer-stat-volume-1h = حجم التداول (1h)
tools-analyzer-stat-volume-6h = حجم التداول (6h)
tools-analyzer-stat-fdv = القيمة المخففة بالكامل
tools-analyzer-txn-buys = عمليات الشراء
tools-analyzer-txn-sells = عمليات البيع

# Liquidity tab.
tools-analyzer-liquidity-empty = لا تتوفر بيانات السيولة
tools-analyzer-liquidity-empty-hint = لم يتم العثور على مجمعات سيولة لهذا الرمز
tools-analyzer-card-total-liquidity = إجمالي السيولة
tools-analyzer-card-pools = مجمعات السيولة
tools-analyzer-active-pools =
    { $count ->
        [zero] مجمعات السيولة النشطة
        [one] مجمع السيولة النشط
        [two] مجمعا السيولة النشطان
        [few] مجمعات السيولة النشطة
        [many] مجمعات السيولة النشطة
       *[other] مجمعات السيولة النشطة
    }
tools-analyzer-card-pool-details = تفاصيل مجمع السيولة
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = السيولة ({ -sol })
tools-analyzer-pools-column-status = الحالة
tools-analyzer-pool-primary = رئيسي

# Copied report. Each line is one message; values arrive already formatted.
tools-analyzer-report-empty = لا يوجد تحليل لنسخه
tools-analyzer-report-label = تقرير التحليل
tools-analyzer-report-title = تقرير تحليل الرمز
tools-analyzer-report-token = الرمز: { $symbol } ({ $name })
tools-analyzer-report-mint = الإصدار: { $mint }
# $sol is the price with the SOL unit.
tools-analyzer-report-price = السعر: { $sol }
tools-analyzer-report-price-with-usd = السعر: { $sol } ({ $usd })
tools-analyzer-report-security = الأمان:
tools-analyzer-report-safety-score = - درجة الأمان: { $score }/100
tools-analyzer-report-mint-authority = - صلاحية الإصدار: { $state }
tools-analyzer-report-freeze-authority = - صلاحية التجميد: { $state }
tools-analyzer-report-risks = - المخاطر: { $count }
tools-analyzer-report-market = السوق:
tools-analyzer-report-volume = - حجم التداول (24h): { $amount }
tools-analyzer-report-change = - التغيّر (24h): { $amount }
tools-analyzer-report-market-cap = - القيمة السوقية: { $amount }
tools-analyzer-report-liquidity = السيولة:
tools-analyzer-report-liquidity-total = - الإجمالي: { $amount }
tools-analyzer-report-pools = - مجمعات السيولة: { $count }
# $time is the formatted time the analysis was fetched.
tools-analyzer-report-generated = تاريخ الإنشاء: { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

# Ids are the watch types. `notify` is the short badge of the notify-only type.
tools-watch-type-buy-on-sell = شراء عند البيع
tools-watch-type-sell-on-buy = بيع عند الشراء
tools-watch-type-notify = إشعار
tools-watch-type-notify-only = إشعار فقط

tools-trade-watcher-setup-title = إعداد المراقبة
tools-trade-watcher-mint-label = عنوان إصدار الرمز
tools-trade-watcher-mint-input =
    .placeholder = أدخل عنوان إصدار الرمز...
tools-trade-watcher-action-search-pools = البحث عن مجمعات السيولة
tools-trade-watcher-pool-label = مجمع السيولة المحدد
tools-trade-watcher-pool-none = لم يتم تحديد مجمع سيولة
tools-trade-watcher-pool-clear =
    .title = مسح مجمع السيولة
# $dex is the DEX name, $base and $quote are the pair symbols.
tools-trade-watcher-pool-selected = مجمع السيولة المحدد: { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = نوع المراقبة
tools-trade-watcher-type-hint = شراء عند البيع: شراء تلقائي عندما يبيع أحدهم. بيع عند الشراء: بيع تلقائي عندما يشتري أحدهم.
tools-trade-watcher-trigger-label = مبلغ التشغيل ({ -sol })
tools-trade-watcher-trigger-hint = الحد الأدنى لحجم الصفقة بـ{ -sol } الذي يشغّل الإجراء
tools-trade-watcher-action-amount-label = مبلغ الإجراء ({ -sol })
tools-trade-watcher-action-amount-hint = المبلغ المراد شراؤه/بيعه عند التشغيل
tools-trade-watcher-slippage-label = الانزلاق السعري (%)
tools-trade-watcher-slippage-hint = أقصى انزلاق سعري مقبول للصفقات
tools-trade-watcher-active-title = المراقبات النشطة
tools-trade-watcher-empty = لا توجد مراقبات نشطة
tools-trade-watcher-empty-hint = اضبط مراقبة أعلاه ثم انقر «بدء المراقبة» لبدء الرصد
tools-trade-watcher-action-start = بدء المراقبة
tools-trade-watcher-action-starting = جارٍ البدء...
tools-trade-watcher-action-stop-all = إيقاف الكل
tools-trade-watcher-action-stopping = جارٍ الإيقاف...
# $token is the token symbol or the start of its mint address.
tools-trade-watcher-started = بدأت المراقبة للرمز { $token }...
tools-trade-watcher-start-failed = تعذّر بدء المراقبة
tools-trade-watcher-stopped = تم إيقاف المراقبة
tools-trade-watcher-stop-failed = تعذّر إيقاف المراقبة
tools-trade-watcher-stopped-all = تم إيقاف جميع المراقبات
tools-trade-watcher-stop-all-failed = تعذّر إيقاف المراقبات
tools-trade-watcher-load-failed = تعذّر تحميل المراقبات
tools-trade-watcher-column-token = الرمز
tools-trade-watcher-column-type = النوع
tools-trade-watcher-column-trigger = التشغيل
tools-trade-watcher-column-action = الإجراء
tools-trade-watcher-column-triggered = تم التشغيل
tools-trade-watcher-stop-watch =
    .title = إيقاف المراقبة

## Results returned by the tools backend. Failures are catalog text; the technical cause
## travels separately as details and is appended by the dashboard.

tools-burn-failure-native-asset = لا يمكن حرق { -sol }
tools-burn-failure-open-position = لا يمكن حرق رموز من مراكز مفتوحة
tools-burn-failure-account-not-found = لم يتم العثور على حساب الرمز
tools-burn-failure-zero-balance = رصيد الرمز صفر بالفعل
tools-burn-failure-transaction = فشلت المعاملة
tools-burn-warning-open-position = لا يمكن حرق رموز من مراكز مفتوحة
tools-burn-warning-closed-position = بقايا من مركز مغلق
# $amount is the token value in SOL with six decimals.
tools-burn-warning-worth = بقيمة ~{ $amount } { -sol }
# $needed and $have are SOL amounts with four decimals.
tools-multi-buy-warning-insufficient = الرصيد غير كافٍ. المطلوب { $needed } { -sol }، والمتوفر { $have } { -sol }
# $needed and $limit are SOL amounts with four decimals.
tools-multi-buy-warning-over-limit = إجمالي { -sol } المطلوب ({ $needed }) يتجاوز الحد ({ $limit })
tools-multi-sell-warning-no-wallets = لم يتم العثور على محافظ ثانوية
tools-multi-sell-warning-no-balance = لا توجد محافظ تحتوي على رصيد من الرمز
tools-multi-op-buy-failed = فشل الشراء
tools-multi-op-sell-failed = فشل البيع
tools-multi-op-transfer-failed = فشل التحويل
tools-multi-op-balance-failed = تعذّر جلب الرصيد
tools-multi-op-mint-invalid = عنوان الإصدار غير صالح
tools-multi-buy-session-failed = فشل الشراء المتعدد
tools-multi-sell-session-failed = فشل البيع المتعدد
tools-multi-session-aborted = أوقف المستخدم العملية

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = فحص المحفظة
tools-wallet-action-scanning = جارٍ الفحص...
# $reason is the technical cause of the failure.
tools-wallet-scan-failed = فشل الفحص: { $reason }
# $amount is an amount with its unit.
tools-wallet-amount-approx = ~{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
        [zero] المحدد: { $count } محفظة
        [one] المحدد: { $count } محفظة
        [two] المحدد: { $count } محفظتان
        [few] المحدد: { $count } محافظ
        [many] المحدد: { $count } محفظةً
       *[other] المحدد: { $count } محفظة
    }
tools-wallet-transfer-failed = فشل التحويل: { $reason }
tools-wallet-cleanup-failed = فشل التنظيف: { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = نتائج الفحص
tools-wallet-cleanup-stat-empty = حسابات ATA الفارغة
tools-wallet-cleanup-stat-reclaimable = { -sol } القابل للاسترداد
tools-wallet-cleanup-stat-failed = الفاشلة (مخزنة مؤقتًا)
tools-wallet-cleanup-prompt = انقر «فحص المحفظة» للعثور على حسابات ATA الفارغة
tools-wallet-cleanup-prompt-hint = سيؤدي هذا إلى فحص جميع حسابات الرموز في محفظتك
tools-wallet-cleanup-action-cleanup = تنظيف الكل
tools-wallet-cleanup-action-cleaning = جارٍ التنظيف...
tools-wallet-cleanup-scanning = جارٍ فحص المحفظة...
# $amount is the reclaimable rent with its unit.
tools-wallet-cleanup-found = حسابات ATA الفارغة التي تم العثور عليها: { $count }، بقيمة ~{ $amount }
tools-wallet-cleanup-clean = لم يتم العثور على حسابات ATA فارغة - المحفظة نظيفة
tools-wallet-cleanup-scan-failed = تعذّر فحص حسابات ATA
tools-wallet-cleanup-done = حسابات ATA التي تم تنظيفها: { $count }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = حرق الرموز
tools-burn-info-title = ما هو الحرق؟
tools-burn-info-body = يؤدي الحرق إلى إتلاف الرموز نهائيًا بحيث يستحيل استرجاعها. بعد الحرق، شغّل تنظيف المحفظة لإغلاق حسابات ATA الفارغة واسترداد إيجار يقارب 0.002 { -sol } لكل رمز.
tools-burn-stat-total = إجمالي الرموز
tools-burn-stat-selected = المحدد
tools-burn-stat-rent = الإيجار القابل للاسترداد
tools-burn-prompt = انقر «فحص المحفظة» للعثور على الرموز
tools-burn-scanning = جارٍ فحص المحفظة بحثًا عن الرموز...
tools-burn-scan-failed = تعذّر فحص الرموز
tools-burn-empty = لم يتم العثور على رموز في المحفظة
# $count is the number of selected tokens.
tools-burn-action-burn = حرق المحدد ({ $count })
tools-burn-action-burning = جارٍ الحرق...
tools-burn-cannot-burn = لا يمكن الحرق
tools-burn-no-value = بلا قيمة

# Category titles and descriptions. Ids are the token categories of the scan.
tools-burn-category-open-position = المراكز المفتوحة
tools-burn-category-has-value = ذات قيمة
tools-burn-category-closed-position = المراكز المغلقة
tools-burn-category-zero-liquidity = بلا سيولة
tools-burn-category-hint-open-position = لا يمكن حرق رموز من مراكز مفتوحة
tools-burn-category-hint-has-value = فكّر في البيع بدلًا من الحرق
tools-burn-category-hint-closed-position = بقايا من صفقات مغلقة
tools-burn-category-hint-zero-liquidity = آمنة للحرق - لا قيمة سوقية لها

tools-burn-confirm-title = تأكيد الحرق
tools-burn-confirm-message =
    { $count ->
        [zero] هل أنت متأكد من رغبتك في حرق <strong>{ $count }</strong> رمز؟
        [one] هل أنت متأكد من رغبتك في حرق <strong>{ $count }</strong> رمز؟
        [two] هل أنت متأكد من رغبتك في حرق <strong>{ $count }</strong> رمزين؟
        [few] هل أنت متأكد من رغبتك في حرق <strong>{ $count }</strong> رموز؟
        [many] هل أنت متأكد من رغبتك في حرق <strong>{ $count }</strong> رمزًا؟
       *[other] هل أنت متأكد من رغبتك في حرق <strong>{ $count }</strong> رمز؟
    }
# $amount is the estimated value with its unit.
tools-burn-confirm-value = إجمالي القيمة التقديرية: <strong>{ $amount }</strong>
tools-burn-confirm-continue = متابعة
tools-burn-final-title = تحذير أخير
tools-burn-final-headline = هذا الإجراء لا رجعة فيه!
tools-burn-final-message =
    { $count ->
        [zero] سيتم إتلاف الرموز التالية ({ $count }) نهائيًا ولا يمكن استرجاعها بأي حال من الأحوال.
        [one] سيتم إتلاف الرمز التالي ({ $count }) نهائيًا ولا يمكن استرجاعه بأي حال من الأحوال.
        [two] سيتم إتلاف الرمزين التاليين ({ $count }) نهائيًا ولا يمكن استرجاعهما بأي حال من الأحوال.
        [few] سيتم إتلاف الرموز التالية ({ $count }) نهائيًا ولا يمكن استرجاعها بأي حال من الأحوال.
        [many] سيتم إتلاف الرموز التالية ({ $count }) نهائيًا ولا يمكن استرجاعها بأي حال من الأحوال.
       *[other] سيتم إتلاف الرموز التالية ({ $count }) نهائيًا ولا يمكن استرجاعها بأي حال من الأحوال.
    }
tools-burn-final-confirm = نعم، حرق الرموز
# $successful and $total count tokens; $amount is the reclaimable rent with its unit.
tools-burn-toast-burned = تم حرق { $successful }/{ $total } من الرموز. شغّل تنظيف المحفظة لاسترداد ~{ $amount }
tools-burn-toast-failed =
    { $count ->
        [zero] تعذّر حرق { $count } رمز
        [one] تعذّر حرق { $count } رمز
        [two] تعذّر حرق { $count } رمزين
        [few] تعذّر حرق { $count } رموز
        [many] تعذّر حرق { $count } رمزًا
       *[other] تعذّر حرق { $count } رمز
    }
tools-burn-failed = فشل الحرق: { $reason }
tools-burn-failures-title =
    { $count ->
        [zero] تعذّر حرق { $count } رمز
        [one] تعذّر حرق { $count } رمز
        [two] تعذّر حرق { $count } رمزين
        [few] تعذّر حرق { $count } رموز
        [many] تعذّر حرق { $count } رمزًا
       *[other] تعذّر حرق { $count } رمز
    }
tools-burn-failure-unknown = لم يتم الإبلاغ عن سبب

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = حول
tools-airdrop-about-body = افحص الإيردروبات المعلّقة والمكافآت القابلة للمطالبة والمخصصات غير المطالَب بها عبر بروتوكولات Solana الشائعة.
tools-airdrop-list-title = الإيردروبات المتاحة
tools-airdrop-prompt = انقر «فحص الإيردروبات» للبحث عن المطالبات المتاحة
tools-airdrop-action-check = فحص الإيردروبات
tools-airdrop-action-claim-all = مطالبة بالكل

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = خيارات التوليد
tools-generator-warning-title = احفظ مفاتيحك الخاصة بأمان!
tools-generator-warning-body = تُنشأ أزواج المفاتيح محليًا ولا تُرسل إلى أي جهة. احرص دائمًا على نسخها احتياطيًا في مكان آمن.
tools-generator-count-label = عدد المحافظ
tools-generator-vanity-label = عنوان مخصص (يبدأ بأحرف محددة)
tools-generator-prefix-label = البادئة
tools-generator-prefix-input =
    .placeholder = مثال: SOL
tools-generator-prefix-hint = تستغرق البادئات الأطول وقتًا أطول للتوليد بشكل أسّي
tools-generator-list-title = المحافظ المولّدة
tools-generator-empty = لم يتم توليد أي محافظ بعد
tools-generator-action-generate = توليد
tools-generator-action-generating = جارٍ التوليد...
tools-generator-count-invalid = يرجى إدخال رقم بين 1 و10
tools-generator-no-keypairs = لم يتم إرجاع أي أزواج مفاتيح
tools-generator-generated =
    { $count ->
        [zero] تم توليد { $count } محفظة
        [one] تم توليد { $count } محفظة
        [two] تم توليد { $count } محفظتين
        [few] تم توليد { $count } محافظ
        [many] تم توليد { $count } محفظةً
       *[other] تم توليد { $count } محفظة
    }
tools-generator-failed = تعذّر توليد المحافظ: { $reason }
tools-generator-copy-public-key =
    .title = نسخ المفتاح العام
tools-generator-copy-private-key =
    .title = نسخ المفتاح الخاص
tools-generator-remove =
    .title = إزالة من القائمة
tools-generator-reveal =
    .title = إظهار المفتاح الخاص
tools-generator-public-key-label = المفتاح العام:
tools-generator-private-key-label = المفتاح الخاص:
# Names the copied value in the shared copied toast.
tools-generator-public-key-name = المفتاح العام
tools-generator-private-key-copied = تم نسخ المفتاح الخاص
tools-generator-private-key-warning = أي شخص يملك هذا المفتاح يتحكم في المحفظة
tools-generator-export-empty = لا توجد محافظ للتصدير
tools-generator-exported = تم تصدير المحافظ - احفظها بأمان

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = الملخص
tools-consolidation-stat-wallets = المحافظ الفرعية
tools-consolidation-stat-native = إجمالي { -sol }
tools-consolidation-stat-tokens = أنواع الرموز
tools-consolidation-stat-rent = الإيجار القابل للاسترداد
tools-consolidation-wallets-title = المحافظ
tools-consolidation-loading-wallets = جارٍ تحميل المحافظ...
tools-consolidation-loading-data = جارٍ تحميل بيانات المحفظة...
tools-consolidation-action-transfer-native = تحويل { -sol }
tools-consolidation-action-transfer-tokens = تحويل كل الرموز
tools-consolidation-action-cleanup = تنظيف حسابات ATA
tools-consolidation-action-transferring = جارٍ التحويل...
tools-consolidation-column-name = الاسم
tools-consolidation-column-native = رصيد { -sol }
tools-consolidation-column-tokens = الرموز
tools-consolidation-column-atas = حسابات ATA الفارغة
tools-consolidation-empty = لم يتم العثور على محافظ فرعية
tools-consolidation-empty-hint = أنشئ محافظ فرعية باستخدام الشراء المتعدد للبدء
tools-consolidation-load-failed = تعذّر التحميل: { $reason }
tools-consolidation-select-prompt = اختر المحافظ المراد تجميعها
# $amount is the selected balance with its unit.
tools-consolidation-selection-totals = | { $amount } | الرموز: { $tokens } | حسابات ATA الفارغة: { $atas }
# $amount is the transferred balance with its unit.
tools-consolidation-transferred-native = تم تحويل { $amount } إلى المحفظة الرئيسية
tools-consolidation-transferred-tokens = الرموز المحوّلة إلى المحفظة الرئيسية: { $count }
# $amount is the reclaimed rent with its unit.
tools-consolidation-cleaned = تم إغلاق حسابات ATA: { $count }، والمسترد: { $amount }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = الرمز
tools-multi-mint-label = عنوان إصدار الرمز
tools-multi-mint-input =
    .placeholder = الصق عنوان إصدار الرمز...
tools-multi-execution-title = إعدادات التنفيذ
tools-multi-delay-min-label = الحد الأدنى للتأخير (ms)
tools-multi-delay-max-label = الحد الأقصى للتأخير (ms)
tools-multi-concurrency-label = التزامن
tools-multi-concurrency-sequential = { $count } (تسلسلي)
tools-multi-concurrency-parallel = { $count } بالتوازي
tools-multi-slippage-label = الانزلاق السعري (%)
tools-multi-router-label = الموجّه
tools-multi-router-auto = تلقائي (أفضل مسار)
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = مجمع سيولة مباشر
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = التقدم
tools-multi-progress-preparing = جارٍ التحضير...
# $label is the session state, $completed and $total count wallet operations.
tools-multi-status-line = { $label } ({ $completed }/{ $total })
tools-multi-column-wallet = المحفظة
tools-multi-column-route = المسار
tools-multi-column-status = الحالة
tools-multi-op-completed = مكتملة
tools-multi-op-failed = فاشلة
tools-multi-action-stop = إيقاف
tools-multi-action-loading = جارٍ التحميل...
# $reason is the technical cause of the failure.
tools-multi-start-failed = تعذّر البدء: { $reason }

# Session states. Ids are the states of a multi-wallet session.
tools-multi-state-pending = قيد الانتظار
tools-multi-state-funding = جارٍ التمويل
tools-multi-state-executing = قيد التنفيذ
tools-multi-state-consolidating = جارٍ التجميع
tools-multi-state-completed = مكتملة
tools-multi-state-failed = فاشلة
tools-multi-state-aborted = ملغاة

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = الرمز الذي تريد شراءه عبر عدة محافظ
tools-multi-buy-wallets-title = إعدادات المحافظ
tools-multi-buy-wallet-count-label = عدد المحافظ
tools-multi-buy-wallet-count-option =
    { $count ->
        [zero] { $count } محفظة
        [one] { $count } محفظة
        [two] { $count } محفظتان
        [few] { $count } محافظ
        [many] { $count } محفظةً
       *[other] { $count } محفظة
    }
tools-multi-buy-wallet-count-hint = عدد المحافظ الفرعية المراد استخدامها
tools-multi-buy-buffer-label = هامش { -sol } لكل محفظة
tools-multi-buy-buffer-hint = مخصص للرسوم (الحد الأدنى 0.015 { -sol })
tools-multi-buy-amounts-title = إعدادات المبالغ
tools-multi-buy-min-label = الحد الأدنى من { -sol } لكل محفظة
tools-multi-buy-min-hint = الحد الأدنى لمبلغ الشراء
tools-multi-buy-max-label = الحد الأقصى من { -sol } لكل محفظة
tools-multi-buy-max-hint = الحد الأقصى لمبلغ الشراء
tools-multi-buy-limit-label = حد إجمالي { -sol } (اختياري)
tools-multi-buy-limit-hint = الحد الأقصى للإنفاق الإجمالي
tools-multi-buy-preview-title = معاينة
tools-multi-buy-preview-create = المحافظ المراد إنشاؤها
tools-multi-buy-preview-amount = المبلغ لكل محفظة
# $min and $max are amounts with their unit.
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = إجمالي { -sol } المطلوب
tools-multi-buy-preview-balance = رصيد المحفظة الرئيسية
tools-multi-buy-action-preview = معاينة
tools-multi-buy-action-start = بدء الشراء المتعدد
tools-multi-buy-executing = جارٍ تنفيذ عمليات الشراء...
tools-multi-buy-column-spent = { -sol } المنفق
tools-multi-buy-column-tokens = الرموز
tools-multi-buy-preview-failed = فشلت المعاينة: { $reason }
tools-multi-buy-started = بدأ الشراء المتعدد
tools-multi-buy-stopped = توقف الشراء المتعدد
tools-multi-buy-completed = اكتمل الشراء المتعدد! نجح { $successful }/{ $total }

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = أدخل عنوان رمز لفحص المحافظ التي تحتفظ به
tools-multi-sell-action-scan = فحص
tools-multi-sell-settings-title = إعدادات البيع
tools-multi-sell-percent-label = نسبة البيع
tools-multi-sell-percent-hint = نسبة الرموز المراد بيعها لكل محفظة (%)
tools-multi-sell-min-fee-label = الحد الأدنى من { -sol } للرسوم
tools-multi-sell-min-fee-hint = الحد الأدنى من { -sol } اللازم لرسوم المعاملة
tools-multi-sell-topup-label = تعبئة تلقائية عند الحاجة
tools-multi-sell-topup-hint = تحويل { -sol } من المحفظة الرئيسية إذا كان رصيد المحفظة الفرعية غير كافٍ
tools-multi-sell-post-title = إجراءات ما بعد البيع
tools-multi-sell-consolidate-label = تجميع { -sol } في المحفظة الرئيسية
tools-multi-sell-consolidate-hint = تحويل كل { -sol } من المحافظ الفرعية إلى المحفظة الرئيسية
tools-multi-sell-close-atas-label = إغلاق حسابات ATA للرمز بعد البيع
tools-multi-sell-close-atas-hint = استرداد نحو 0.002 { -sol } لكل حساب ATA
tools-multi-sell-wallets-title = المحافظ التي تحتوي على الرمز
tools-multi-sell-empty = لا توجد محافظ فرعية تحتفظ بهذا الرمز
tools-multi-sell-column-tokens = الرموز
tools-multi-sell-column-native = رصيد { -sol }
tools-multi-sell-column-topup = تحتاج تعبئة
tools-multi-sell-none-selected = لم يتم تحديد أي محافظ
tools-multi-sell-select-required = يرجى تحديد محفظة واحدة على الأقل
tools-multi-sell-action-start = بدء البيع المتعدد
tools-multi-sell-executing = جارٍ تنفيذ عمليات البيع...
tools-multi-sell-column-sold = الرموز المباعة
tools-multi-sell-column-received = { -sol } المستلم
tools-multi-sell-started = بدأ البيع المتعدد
tools-multi-sell-stopped = توقف البيع المتعدد
# $amount is the received amount with its unit.
tools-multi-sell-completed = اكتمل البيع المتعدد! تم استلام { $amount }

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = المفضلة
tools-favorites-saved = المفضلات المحفوظة
tools-favorites-save-current = حفظ الحالي
tools-favorites-empty = لا توجد مفضلات محفوظة بعد
tools-favorites-no-label = بلا تسمية
# $count is how many times the favorite was used.
tools-favorites-uses = { $count }×
tools-favorites-remove = إزالة
# $name is the favorite's label or symbol.
tools-favorites-loaded = تم تحميل المفضلة: { $name }
tools-favorites-default-name = الإعداد
tools-favorites-mint-required = يرجى إدخال عنوان إصدار الرمز أولًا
tools-favorites-add-title = إضافة إلى المفضلة
tools-favorites-add-message = أدخل تسمية لهذه المفضلة
tools-favorites-add-placeholder = التسمية (اختياري)...
tools-favorites-saved-toast = تم الحفظ في المفضلة
tools-favorites-save-failed = تعذّر حفظ المفضلة
tools-favorites-remove-title = إزالة المفضلة
tools-favorites-remove-message = هل تريد إزالة هذه المفضلة؟
tools-favorites-removed-toast = تمت إزالة المفضلة
tools-favorites-remove-failed = تعذّرت إزالة المفضلة
