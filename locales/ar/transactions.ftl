# Transaction type labels. Ids come from TransactionType::kind() in
# src/transactions/types.rs; the dashboard maps them in ui/transaction_type.js.

transactions-type-buy = شراء
transactions-type-sell = بيع
transactions-type-swap = مبادلة
transactions-type-sol-transfer = تحويل SOL
transactions-type-token-transfer = تحويل رمز
transactions-type-transfer = تحويل
transactions-type-dust = غبار
transactions-type-spam = بريد مزعج
transactions-type-ata-create = تم فتح حساب
transactions-type-ata-close = تم استرداد الإيجار
transactions-type-ata = حساب الرمز
transactions-type-liquidity-add = إضافة سيولة
transactions-type-liquidity-remove = إزالة سيولة
transactions-type-nft = NFT
transactions-type-program = استدعاء برنامج
transactions-type-compute = حوسبة
transactions-type-failed = فشل
transactions-type-unknown = غير مصنّف

# A type with the payload that identifies it, as shown in the position activity feed.
transactions-type-with-detail = { $label } ({ $detail })
transactions-type-token-transfer-detail = { $label } { $mint } ({ $amount })
transactions-type-spam-detail = إيردروب مزعج ({ $mint })
transactions-type-described = { $description }

# Type filter entries whose wording differs from the type label.
transactions-filter-all = كل الأنواع
transactions-filter-transfer = التحويلات
transactions-filter-ata = الإيجار والحسابات
transactions-filter-liquidity = السيولة
transactions-filter-program = استدعاءات البرامج

# Wallet-relative direction. Ids come from TransactionDirection in src/transactions/types.rs
# (ui/transaction_direction.js).
transactions-direction-incoming = وارد
transactions-direction-outgoing = صادر
transactions-direction-internal = داخلي
transactions-direction-unknown = غير مصنّف

# Chain status. Ids come from TransactionStatus in src/transactions/types.rs
# (ui/transaction_status.js); Success and Unknown label a row without a status.
transactions-status-pending = قيد الانتظار
transactions-status-confirmed = مؤكدة
transactions-status-finalized = نهائية
transactions-status-failed = فشلت
transactions-status-success = نجحت
transactions-status-unknown = غير معروفة

# Ids come from AtaOperationType in src/transactions/types.rs.
transactions-ata-operation-creation = إنشاء
transactions-ata-operation-closure = إغلاق

## Transactions page (pages/transactions.js)

transactions-toolbar-title = سجل المعاملات
transactions-search =
    .placeholder = البحث في التواقيع…
    .aria-label = البحث في توقيعات المعاملات
transactions-load-failed = تعذّر تحديث المعاملات
transactions-summary-total = الإجمالي
transactions-summary-estimate = تقدير
transactions-summary-success = الناجحة
transactions-summary-failed = الفاشلة
transactions-filter-wallet = المحفظة
transactions-filter-type = النوع
transactions-filter-direction = الاتجاه
transactions-filter-status = الحالة
transactions-filter-all-directions = كل الاتجاهات
transactions-filter-all-statuses = كل الحالات
transactions-wallet-main = المحفظة الرئيسية
transactions-col-time = الوقت
transactions-col-signature = التوقيع
transactions-col-type = النوع
transactions-col-direction = الاتجاه
transactions-col-status = الحالة
transactions-col-sol-delta = Δ { -sol }
transactions-col-fees = الرسوم ({ -sol })
transactions-col-token = الرمز
transactions-col-router = الموجّه
transactions-col-instructions = التعليمات

## Transaction details dialog (ui/transaction_details_dialog.js)

transactions-dialog-copy-signature =
    .title = نسخ التوقيع
transactions-dialog-close =
    .title = إغلاق (ESC)
transactions-dialog-tabs-label = أقسام تفاصيل المعاملة
transactions-dialog-meta-slot = الفتحة:
transactions-dialog-meta-fee = الرسوم:
transactions-dialog-loading = جارٍ التحميل...
transactions-dialog-loading-details = جارٍ تحميل تفاصيل المعاملة...
transactions-dialog-load-failed = فشل تحميل تفاصيل المعاملة
# $reason is the failure text reported by the server.
transactions-dialog-load-failed-reason = فشل تحميل تفاصيل المعاملة: { $reason }
transactions-dialog-not-found = لم يتم العثور على المعاملة
transactions-dialog-tab-overview = نظرة عامة
transactions-dialog-tab-balances = الأرصدة
transactions-dialog-tab-instructions = التعليمات
transactions-dialog-tab-logs = السجلات
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = خام
transactions-dialog-unknown = غير معروف
transactions-dialog-unknown-asset = أصل غير معروف
transactions-dialog-unavailable = غير متاح

## Transaction details dialog: overview

transactions-dialog-failed-title = فشلت المعاملة
transactions-dialog-no-program-error = لم يتم تقديم خطأ من البرنامج.
transactions-dialog-story-title = ما الذي حدث
# $router is the routing program name.
transactions-dialog-router-via = عبر { $router }
transactions-dialog-flow-paid = المدفوع
transactions-dialog-flow-received = المستلَم
transactions-dialog-flow-from = من
transactions-dialog-flow-to = إلى
transactions-dialog-flow-amount = المبلغ
transactions-dialog-net-wallet-change = صافي تغير المحفظة:
transactions-dialog-processed = تمت المعالجة على Solana
transactions-dialog-execution-title = التنفيذ
transactions-dialog-metric-execution-price = سعر التنفيذ
transactions-dialog-metric-effective-received = المستلَم الفعلي
transactions-dialog-metric-effective-spent = المصروف الفعلي
transactions-dialog-metric-network-fee = رسوم الشبكة
transactions-dialog-metric-estimated-pnl = الأرباح والخسائر المقدّرة
transactions-dialog-metric-net-sol-change = صافي تغير { -sol }
transactions-dialog-route-title = المسار والأصول
transactions-dialog-route-router = الموجّه
transactions-dialog-route-input-asset = أصل الإدخال
transactions-dialog-route-output-asset = أصل الإخراج
transactions-dialog-route-pool = مجمع السيولة
transactions-dialog-route-program = البرنامج
transactions-dialog-tech-title = التفاصيل التقنية
transactions-dialog-tech-summary = التوقيع والفتحة والموارد
transactions-dialog-tech-signature = التوقيع
transactions-dialog-tech-timestamp = الطابع الزمني
transactions-dialog-tech-slot = الفتحة
transactions-dialog-tech-exact-fee = الرسوم الدقيقة
transactions-dialog-tech-accounts = الحسابات
transactions-dialog-tech-instructions = التعليمات
transactions-dialog-tech-compute-units = وحدات الحوسبة
transactions-dialog-tech-token-decimals = الخانات العشرية للرمز

## Transaction details dialog: balances, instructions, logs, ATA and raw tabs

transactions-dialog-balances-sol-title = تغيرات رصيد { -sol }
transactions-dialog-balances-sol-empty = لا توجد تغيرات في رصيد { -sol }
transactions-dialog-balances-token-title = تغيرات رصيد الرموز
transactions-dialog-balances-token-empty = لا توجد تغيرات في رصيد الرموز
transactions-dialog-balances-net-sol = صافي تغير { -sol }
transactions-dialog-balances-fee = رسوم المعاملة
transactions-dialog-col-account = الحساب
transactions-dialog-col-token = الرمز
transactions-dialog-col-mint = عنوان الإصدار
transactions-dialog-col-pre-balance = الرصيد قبل
transactions-dialog-col-post-balance = الرصيد بعد
transactions-dialog-col-change = التغير
transactions-dialog-col-type = النوع
transactions-dialog-col-rent = الإيجار ({ -sol })
transactions-dialog-instructions-empty = لم يتم العثور على تعليمات
transactions-dialog-instructions-count =
    { $count ->
        [zero] { $count } تعليمة
        [one] { $count } تعليمة
        [two] { $count } تعليمتان
        [few] { $count } تعليمات
        [many] { $count } تعليمة
       *[other] { $count } تعليمة
    }
transactions-dialog-instruction-program-id = معرّف البرنامج
transactions-dialog-instruction-accounts = الحسابات ({ $count })
transactions-dialog-instruction-data = البيانات
transactions-dialog-logs-empty = لا توجد سجلات متاحة
transactions-dialog-logs-filter = ترشيح السجلات...
transactions-dialog-logs-no-match = لا توجد سجلات مطابقة
transactions-dialog-logs-count =
    { $count ->
        [zero] { $count } سجل
        [one] { $count } سجل
        [two] { $count } سجلان
        [few] { $count } سجلات
        [many] { $count } سجلًا
       *[other] { $count } سجل
    }
transactions-dialog-ata-empty = لا توجد عمليات ATA في هذه المعاملة
transactions-dialog-ata-summary-title = ملخص تحليل ATA
transactions-dialog-ata-creations = عمليات الإنشاء
transactions-dialog-ata-closures = عمليات الإغلاق
transactions-dialog-ata-rent-spent = الإيجار المصروف
transactions-dialog-ata-rent-recovered = الإيجار المسترد
transactions-dialog-ata-net-rent = صافي أثر الإيجار
transactions-dialog-ata-operations-title = عمليات ATA ({ $count })
transactions-dialog-raw-copy = نسخ JSON
transactions-dialog-raw-empty = لا توجد بيانات خام متاحة
