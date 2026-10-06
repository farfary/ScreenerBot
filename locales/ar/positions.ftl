# Position details labels.

# State reasons. Ids come from POSITION_CREATED_REASON in src/positions/database/types.rs.
positions-state-reason-position-created = تم إنشاء المركز


# Source: scripts/pages/positions.js

## Views, origin and toolbar

# Ids are the position status values (POSITION_STATUS_LABELS, ui/position_status.js).
positions-status-open = مفتوح
positions-status-closed = مغلق
positions-status-archived = مؤرشف
positions-origin-copy = نسخ
positions-origin-manual = يدوي
positions-origin-wallet = محفظة
positions-origin-copy-link =
    .title = فتح مهمة النسخ التي فتحت هذا المركز
positions-holding-frozen = مجمّد
    .title = جمّدت صلاحية الإصدار حساب هذا الرمز، ولا يمكن تحويل الرصيد أو بيعه
positions-toolbar-total = الإجمالي
positions-toolbar-delete-all = حذف الكل
positions-search-placeholder = البحث بالرمز أو عنوان الإصدار...
positions-filter-origin = المصدر
positions-filter-origin-all = كل المصادر
positions-filter-origin-auto = المتداول الآلي
positions-filter-origin-copy = نسخ التداول
positions-delete-all-tooltip = حذف كل المراكز المؤرشفة نهائيًا

## Columns

positions-column-token = الرمز
positions-column-archived-at = الأرشفة
positions-column-entry-time = وقت الدخول
positions-column-exit-time = وقت الخروج
positions-column-avg-entry = متوسط الدخول ({ -sol })
positions-column-avg-exit = متوسط الخروج ({ -sol })
positions-column-current-price = الحالي ({ -sol })
positions-column-total-invested = إجمالي الاستثمار
positions-column-proceeds = العائدات
positions-column-pnl = الربح والخسارة
positions-column-pnl-percent = الربح والخسارة %
positions-column-size = الحجم
positions-column-dca = DCA
positions-column-exits = عمليات الخروج
positions-column-unrealized-pnl = الربح والخسارة غير المحققة
positions-column-unrealized-percent = غير محقق %

## Cells

# Shown instead of a figure the wallet history cannot support.
positions-unknown-basis = لا توجد تكلفة أساس في سجل هذه المحفظة (إيردروب أو تنفيذ مسعّر بالدولار أو مبادلة بلا طرف { -sol })
positions-unknown-history = هذه الجولة لا تتطابق مع الرصيد على السلسلة
positions-dca-count =
    { $count ->
        [zero] { $count } DCA
        [one] { $count } DCA
        [two] { $count } DCA
        [few] { $count } DCA
        [many] { $count } DCA
       *[other] { $count } DCA
    }
positions-exit-count =
    { $count ->
        [zero] { $count } خروج
        [one] { $count } خروج
        [two] { $count } خروجان
        [few] { $count } عمليات خروج
        [many] { $count } عملية خروج
       *[other] { $count } عملية خروج
    }

## Row actions

positions-action-add =
    .title = إضافة إلى المركز (DCA)
    .aria-label = إضافة إلى المركز
positions-action-sell =
    .title = بيع (كامل أو جزئي بنسبة %)
    .aria-label = بيع المركز
positions-action-sell-frozen = مجمّد بواسطة صلاحية الإصدار، ولا يمكن بيع هذه الحيازة
positions-action-remove =
    .title = إزالة (أرشفة أو حذف)
    .aria-label = إزالة المركز
positions-action-restore =
    .title = استعادة إلى المفتوحة/المغلقة
    .aria-label = استعادة المركز
positions-action-delete =
    .title = حذف نهائي
    .aria-label = حذف نهائي
positions-action-in-progress = قيد التنفيذ…

## Live state of a row

positions-caption-buying = شراء
# $step is the label of the current action step.
positions-caption-buying-step = شراء · { $step }
positions-caption-selling = بيع
positions-caption-selling-step = بيع · { $step }
positions-caption-closing = إغلاق
positions-caption-failed = فشل
# $error is the failure text of the action.
positions-caption-failed-detail = فشل · { $error }
positions-step-adding = إضافة
positions-pending-buying = جارٍ الشراء…
positions-pending-buy-failed = فشل الشراء

## Messages and confirmations

positions-load-failed = تعذّر تحديث المراكز
positions-toast-not-found = لم يتم العثور على بيانات المركز
positions-toast-deleted = تم حذف المركز
positions-toast-archived = تمت أرشفة المركز
positions-toast-restored = تمت استعادة المركز
positions-action-failed = فشل الإجراء
positions-delete-title = حذف المركز نهائيًا
# $symbol is the token symbol.
positions-delete-message = هل تريد حذف { $symbol } نهائيًا؟ سيؤدي ذلك إلى إزالة المركز وسجله من قاعدة البيانات ولا يمكن التراجع عنه. لن تتأثر معاملاتك وبيانات الرمز.
positions-delete-confirm = حذف نهائي
positions-delete-all-title = حذف كل المراكز المؤرشفة
positions-delete-all-message =
    { $count ->
        [zero] هل تريد حذف كل المراكز المؤرشفة ({ $count }) نهائيًا؟ لا يمكن التراجع عن ذلك. لن تتأثر المعاملات وبيانات الرمز.
        [one] هل تريد حذف المركز المؤرشف ({ $count }) نهائيًا؟ لا يمكن التراجع عن ذلك. لن تتأثر المعاملات وبيانات الرمز.
        [two] هل تريد حذف المركزين المؤرشفين ({ $count }) نهائيًا؟ لا يمكن التراجع عن ذلك. لن تتأثر المعاملات وبيانات الرمز.
        [few] هل تريد حذف كل المراكز المؤرشفة ({ $count }) نهائيًا؟ لا يمكن التراجع عن ذلك. لن تتأثر المعاملات وبيانات الرمز.
        [many] هل تريد حذف كل المراكز المؤرشفة ({ $count }) نهائيًا؟ لا يمكن التراجع عن ذلك. لن تتأثر المعاملات وبيانات الرمز.
       *[other] هل تريد حذف كل المراكز المؤرشفة ({ $count }) نهائيًا؟ لا يمكن التراجع عن ذلك. لن تتأثر المعاملات وبيانات الرمز.
    }
positions-delete-all-message-empty = هل تريد حذف كل المراكز المؤرشفة نهائيًا؟ لا يمكن التراجع عن ذلك.
positions-delete-all-confirm = حذف الكل
positions-delete-all-done =
    { $count ->
        [zero] تم حذف المراكز المؤرشفة: { $count }
        [one] تم حذف مركز مؤرشف: { $count }
        [two] تم حذف مركزين مؤرشفين: { $count }
        [few] تم حذف المراكز المؤرشفة: { $count }
        [many] تم حذف المراكز المؤرشفة: { $count }
       *[other] تم حذف المراكز المؤرشفة: { $count }
    }
positions-delete-all-failed = فشل حذف المراكز المؤرشفة

# Source: scripts/ui/position_remove_dialog.js

## Remove position dialog

positions-remove-title = إزالة المركز
# Inline markup: emphasis on the opening sentence and on "not".
positions-remove-open-warning = <strong>هذا المركز لا يزال مفتوحًا.</strong> البوت يحتفظ بهذا الرمز. الإزالة تُخلي خانة الصفقة وتوقف التتبع، لكنها <strong>لا</strong> تبيع. بع أولًا إذا أردت استرداد { -sol }.
positions-remove-modes =
    .aria-label = وضع الإزالة
positions-remove-archive = أرشفة
positions-remove-recommended = موصى به
positions-remove-archive-description = يُنقل إلى تبويب المؤرشفة. يمكن التراجع في أي وقت، ولا يُباع شيء وتبقى كل الصفقات مسجلة.
positions-remove-delete = حذف نهائي
positions-remove-delete-description = يمسح هذا المركز وسجله الكامل من قاعدة البيانات.
# Inline markup: emphasis on the irreversibility sentence.
positions-remove-danger = سيؤدي ذلك إلى إزالة المركز وسجله نهائيًا. <strong>لا يمكن التراجع عن ذلك.</strong> لن تتأثر معاملاتك وبيانات الرمز.
positions-remove-confirm-archive = أرشفة المركز

# Source: scripts/ui/position_details_dialog.js, scripts/ui/position_details/panes.js

## Position details frame

# Message shown after a management change. $mode is the label of the new mode.
positions-management-changed = تم تعيين إدارة المركز إلى { $mode }
positions-details-load-failed = فشل تحميل تفاصيل المركز
positions-details-mint-label = عنوان الإصدار
positions-details-management-failed = فشل تحديث إدارة المركز
positions-details-favorite-add =
    .title = إضافة إلى المفضلة
    .aria-label = إضافة إلى المفضلة
positions-details-favorite-remove =
    .title = إزالة من المفضلة
    .aria-label = إزالة من المفضلة
positions-details-view-solscan =
    .title = عرض على { -solscan }
    .aria-label = عرض الرمز على { -solscan }
positions-details-close =
    .title = إغلاق (Esc)
    .aria-label = إغلاق
positions-details-chart-section =
    .aria-label = مخطط السعر
positions-details-loading-chart = جارٍ تحميل المخطط...
positions-details-activity-section =
    .aria-label = النشاط
positions-details-activity-title = النشاط
positions-details-split-handle =
    .aria-label = تغيير حجم المخطط والنشاط
positions-details-activity-pane =
    .aria-label = لوحة النشاط
positions-details-activity-expand =
    .title = توسيع النشاط
    .aria-label = توسيع النشاط
positions-details-summary-section =
    .aria-label = ملخص المركز
positions-details-loading = جارٍ تحميل المركز...

## Management modes. Ids are the PositionManagement serde ids (src/positions/types.rs).

positions-management-auto-trader = المتداول الآلي
positions-management-user-only = المستخدم فقط
positions-management-copy-task = مهمة نسخ
positions-management-hybrid = هجين
positions-pane-show-chart = إظهار المخطط
positions-pane-show-activity = إظهار النشاط
positions-pane-restore-activity = استعادة النشاط
positions-pane-expand-chart =
    .title = توسيع المخطط
    .aria-label = توسيع المخطط

# Source: scripts/ui/position_details/header.js

## Position details header

positions-risk-low = مخاطر منخفضة
positions-risk-medium = مخاطر متوسطة
positions-risk-high = مخاطر عالية
positions-risk-unknown = مخاطر غير معروفة
positions-busy-buying = الشراء قيد التنفيذ…
positions-busy-selling = البيع قيد التنفيذ…
positions-busy-closing = الإغلاق قيد التنفيذ…
positions-header-avg-entry = متوسط الدخول
# $count is the number of buys: the entry plus each add.
positions-header-buy-count =
    { $count ->
        [zero] { $count } عملية شراء
        [one] { $count } عملية شراء
        [two] { $count } عمليتا شراء
        [few] { $count } عمليات شراء
        [many] { $count } عملية شراء
       *[other] { $count } عملية شراء
    }
positions-header-exit-price = سعر الخروج
# $ago is the elapsed time since the close, for example "3h ago".
positions-header-closed-ago = أُغلق { $ago }
positions-header-realized-pnl = الأرباح والخسائر المحققة
positions-header-usd-note = بالدولار وفق سعر { -sol } اليوم
positions-header-returned = المسترد
# $amount is the formatted SOL amount invested.
positions-header-of-invested = من أصل { $amount } مستثمرة
positions-header-price = السعر
positions-header-last-price = آخر سعر
positions-header-pool-ago = مجمع سيولة · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = الأرباح والخسائر غير المحققة
positions-header-pnl-last-price = الأرباح والخسائر عند آخر سعر
positions-header-value = القيمة
positions-header-last-value = آخر قيمة
positions-header-invested = { $amount } مستثمرة
positions-header-origin-hint = كيف فُتح هذا المركز
positions-header-risk-hint = درجة { -rugcheck }، وكلما انخفضت كان أكثر أمانًا
positions-header-frozen = مجمّد
    .title = جمّدت صلاحية الإصدار هذه الحيازة
positions-header-managed-by = تتم إدارته بواسطة
positions-header-management-select =
    .aria-label = إدارة المركز

## Entry origin shown in the header badge

positions-origin-unknown = غير معروف
# $task is the copy task id. The source wallet follows in its own element.
positions-origin-copied-task = منسوخ · المهمة { $task }
positions-origin-manual-entry = دخول يدوي
positions-origin-wallet-entry = دخول من المحفظة
# $strategy is the strategy id.
positions-origin-auto-strategy = آلي · { $strategy }
positions-origin-auto-entry = دخول آلي

## Swaps that are submitted and not yet booked

positions-pending-adding = إضافة
positions-pending-adding-amount = إضافة { $amount }
positions-pending-selling = بيع
# $percent is the formatted share of the position being sold.
positions-pending-selling-percent = بيع { $percent }
# $label is the pending swap wording.
positions-pending-confirming = { $label } · قيد التأكيد
    .title = تم الإرسال وبانتظار التأكيد على السلسلة. تتحدث الأرقام بعد التحقق.

## Trade controls

positions-trade-add = إضافة
    .title = إضافة إلى المركز
positions-trade-sell = بيع
    .title = بيع جزء من المركز
positions-trade-close = إغلاق المركز
    .title = بيع الكل وإغلاق المركز
positions-trade-token = تفاصيل الرمز
    .title = فتح تفاصيل الرمز

## Favorites

positions-favorite-token-fallback = الرمز
# $symbol is the token symbol.
positions-favorite-added = تمت إضافة { $symbol } إلى المفضلة
positions-favorite-removed = تمت إزالة { $symbol } من المفضلة
positions-favorite-add-failed = فشلت إضافة المفضلة
positions-favorite-remove-failed = فشلت إزالة المفضلة
positions-favorite-update-failed = فشل تحديث المفضلة

# Source: scripts/ui/position_details/summary.js

## Summary rail

positions-summary-position = المركز
positions-summary-price-path = مسار السعر
positions-summary-network-fees = رسوم الشبكة
positions-summary-risk = المخاطر
positions-summary-market = السوق
positions-summary-market-now = السوق الآن
positions-summary-links = الروابط
positions-fact-tokens-fallback = رموز
positions-fact-bought = المشترى
positions-fact-holding = الحيازة
positions-fact-sold = المباع
positions-fact-realized = المحقق
positions-fact-opened = الفتح
positions-fact-closed = الإغلاق
positions-fact-reason = السبب
positions-fact-archived = الأرشفة
positions-fact-entry = الدخول
positions-fact-exit = الخروج
positions-fact-total = الإجمالي
positions-fact-verified = تم التحقق على السلسلة
positions-fact-confirming = قيد التأكيد
# $percent is the formatted share, for example "12.5%".
positions-fact-share-of-bought = { $percent } من المشترى
positions-fact-share-of-invested = { $percent } من المستثمر
# $count is the number of adds after the entry.
positions-fact-entry-count =
    { $count ->
        [0] دخول واحد
        [zero] دخول واحد + { $count } إضافة
        [one] دخول واحد + { $count } إضافة
        [two] دخول واحد + { $count } إضافتان
        [few] دخول واحد + { $count } إضافات
        [many] دخول واحد + { $count } إضافة
       *[other] دخول واحد + { $count } إضافة
    }
# $count is the number of partial exits, $returned the formatted SOL amount.
positions-fact-partial-exits-back =
    { $count ->
        [zero] خروج جزئي: { $count } · المسترد { $returned }
        [one] خروج جزئي: { $count } · المسترد { $returned }
        [two] خروجان جزئيان: { $count } · المسترد { $returned }
        [few] خروج جزئي: { $count } · المسترد { $returned }
        [many] خروج جزئي: { $count } · المسترد { $returned }
       *[other] خروج جزئي: { $count } · المسترد { $returned }
    }
# $age is the elapsed time of the hold.
positions-fact-held = احتفاظ { $age }
# $percent is the signed change against the entry price.
positions-fact-vs-entry = { $percent } مقابل الدخول
positions-fact-exit-vs-peak = الخروج مقابل القمة
positions-fact-now-vs-peak = الآن مقابل القمة
positions-fact-entry-range = نطاق الدخول
positions-range-low = الأدنى
positions-range-peak = القمة
positions-range-now = الآن
positions-range-label-exit = سعر الدخول والخروج بين الأدنى والقمة
positions-range-label-now = سعر الدخول والسعر الحالي بين الأدنى والقمة
positions-fact-mint-authority = صلاحية الإصدار
positions-fact-freeze-authority = صلاحية التجميد
positions-fact-active = نشطة
positions-fact-pool = مجمع سيولة
# $amount is the formatted liquidity in SOL.
positions-fact-pool-liquidity = سيولة { $amount } { -sol }
positions-fact-market-cap = القيمة السوقية
# $value is the formatted fully diluted valuation in USD.
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = السيولة
positions-fact-volume-24h = حجم التداول 24h
positions-fact-price-change = تغير السعر
positions-change-period-1h = 1h
positions-change-period-24h = 24h
positions-fact-holders = الحاملون
positions-link-website = الموقع الإلكتروني
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

# Source: scripts/ui/position_details/activity.js, scripts/ui/position_details/activity_event.js

## Activity

positions-activity-load-failed = تعذّر تحميل النشاط
positions-activity-loading = جارٍ تحميل النشاط...
positions-activity-empty = لم يحدث شيء لهذا الرمز في هذه المحفظة بعد
positions-activity-filter-empty = لا يوجد نشاط يطابق هذا المرشح
positions-activity-round-count =
    { $count ->
        [zero] { $count } جولة
        [one] { $count } جولة
        [two] { $count } جولتان
        [few] { $count } جولات
        [many] { $count } جولة
       *[other] { $count } جولة
    }
positions-activity-event-count =
    { $count ->
        [zero] { $count } حدث
        [one] { $count } حدث
        [two] { $count } حدثان
        [few] { $count } أحداث
        [many] { $count } حدثًا
       *[other] { $count } حدث
    }
positions-activity-pending-count = قيد الانتظار: { $count }
positions-activity-failed-count = فاشلة: { $count }
positions-filter-all = الكل
positions-filter-trades = الصفقات
positions-filter-buys = عمليات الشراء
positions-filter-sells = عمليات البيع
positions-filter-wallet = المحفظة
positions-filter-issues = المشكلات
positions-activity-filters =
    .aria-label = تصفية النشاط
positions-activity-totals =
    .aria-label = كل الجولات على هذا الرمز
positions-activity-realized-all = المحقق، كل الجولات
positions-activity-invested = المستثمر
positions-activity-returned = المسترد
# $when is the formatted open time of a round that has not closed.
positions-activity-opened = فُتح { $when }
# $index is the 1-based number of the round.
positions-activity-round-title = المركز { $index }
positions-activity-this-position = هذا المركز
positions-activity-dates-unavailable = التواريخ غير متاحة
positions-activity-wallet-title = معاملات المحفظة
# $range is the date range, $count the number of events.
positions-activity-outside =
    { $count ->
        [zero] خارج أي مركز · { $range } · { $count } حدث
        [one] خارج أي مركز · { $range } · { $count } حدث
        [two] خارج أي مركز · { $range } · { $count } حدثان
        [few] خارج أي مركز · { $range } · { $count } أحداث
        [many] خارج أي مركز · { $range } · { $count } حدثًا
       *[other] خارج أي مركز · { $range } · { $count } حدث
    }
positions-details-signature-label = التوقيع

## State history milestones. Ids are the PositionState names (src/positions/database/types.rs).

positions-state-open = المركز مفتوح
positions-state-closing = المركز قيد الإغلاق
positions-state-closed = المركز مغلق
positions-state-exit-pending = خروج المركز قيد الانتظار
positions-state-exit-failed = فشل خروج المركز
positions-state-phantom = المركز وهمي
positions-state-reconciling = المركز قيد المطابقة

## Activity events

positions-event-kind-entry = دخول
positions-event-kind-dca = إضافة
positions-event-kind-partial-exit = خروج جزئي
positions-event-kind-exit = خروج
positions-event-kind-buy = شراء من المحفظة
positions-event-kind-sell = بيع من المحفظة
positions-event-kind-transfer = تحويل
positions-event-kind-ata = حساب الرمز
positions-event-kind-other = معاملة
positions-event-state-pending = قيد الانتظار
positions-event-state-failed = فشل
positions-event-state-synthetic = اصطناعي
# $error is the failure text reported by the chain.
positions-chain-status-failed-detail = فشل: { $error }
positions-event-tokens-fallback = رموز
# In the descriptions below $amount is the token amount with its symbol, $sol the SOL amount
# and $percent the share of the position sold.
positions-event-entry-submitted = تم إرسال شراء بمقدار { $amount }
positions-event-entry-for = تم شراء { $amount } مقابل { $sol }
positions-event-entry = تم شراء { $amount }
positions-event-dca-submitted = تم إرسال إضافة بمقدار { $amount }
positions-event-dca-for = تمت إضافة { $amount } مقابل { $sol }
positions-event-dca = تمت إضافة { $amount }
positions-event-partial-exit-submitted-percent = تم إرسال خروج جزئي بنسبة { $percent } بمقدار { $amount }
positions-event-partial-exit-submitted = تم إرسال خروج جزئي بمقدار { $amount }
positions-event-sold-percent-for = تم بيع { $amount } ({ $percent }) مقابل { $sol }
positions-event-sold-percent = تم بيع { $amount } ({ $percent })
positions-event-sold-for = تم بيع { $amount } مقابل { $sol }
positions-event-sold = تم بيع { $amount }
positions-event-exit-submitted = تم إرسال خروج كامل من المركز
positions-event-exit-for = أُغلق المركز ببيع { $amount } مقابل { $sol }
positions-event-exit-closed = تم إغلاق المركز
positions-event-wallet-bought = اشترت المحفظة { $amount } في مكان آخر
positions-event-wallet-sold = باعت المحفظة { $amount } في مكان آخر
positions-event-received = تم استلام { $amount }
positions-event-sent = تم إرسال { $amount }
positions-event-transferred = تم تحويل { $amount }
positions-event-ata = نشاط حساب الرمز
positions-event-wallet-transaction = معاملة في المحفظة تخص { $amount }
# $price is the formatted price per token in SOL.
positions-event-price-per-token = { $price } { -sol } / رمز
# $amount is the signed SOL change of the wallet.
positions-event-wallet-change = تغير المحفظة { $amount }
positions-event-after-title = المركز بعد هذا الحدث
positions-event-capital-invested = رأس المال المستثمر
positions-event-average-entry = متوسط الدخول
positions-event-transfers-title = تحويلات الرمز
positions-event-transfer-amount = المبلغ
positions-event-transfer-mint = الإصدار
positions-event-transfer-from = من
positions-event-transfer-to = إلى
positions-event-no-signature = لا يوجد توقيع على السلسلة
positions-event-click-to-copy = انقر للنسخ
positions-event-solscan = { -solscan }
positions-event-token-amount = كمية الرمز
positions-event-trade-price = سعر الصفقة
positions-event-native-amount = مبلغ { -sol }
positions-event-cost-basis = تكلفة الأساس
positions-event-usd-value = القيمة بالدولار
positions-event-network-fee = رسوم الشبكة
positions-event-router = موجّه
positions-event-slot = الفتحة
positions-event-chain-status = حالة السلسلة
positions-event-transaction-type = نوع المعاملة
positions-event-direction = الاتجاه
positions-event-wallet-native-change = تغير { -sol } في المحفظة
positions-event-instructions = التعليمات
positions-event-compute-units = وحدات الحوسبة
positions-event-accounts = الحسابات
positions-event-record-id = معرّف السجل
positions-event-time-unavailable = الوقت غير متاح
positions-event-details = التفاصيل
positions-event-hide-details = إخفاء التفاصيل

# Source: scripts/ui/position_details/chart.js

## Position chart

positions-chart-type-candles = شموع
positions-chart-type-line = خطي
positions-chart-type-area = مساحي
positions-chart-type-group =
    .aria-label = نوع المخطط
positions-chart-overlays-group =
    .aria-label = طبقات المخطط
positions-chart-ema = EMA
    .title = المتوسطات المتحركة الأسية، 9 و21
positions-chart-fit = ملاءمة
    .title = إطار عمر هذا المركز
positions-chart-timeframes-group =
    .aria-label = الإطار الزمني
positions-chart-pane-group =
    .aria-label = لوحة المخطط
positions-chart-unavailable = محرك المخطط غير متاح
positions-chart-collecting = جارٍ جمع بيانات المخطط…
positions-chart-no-data = لا توجد بيانات مخطط لهذا الرمز بعد
positions-chart-avg-entry = متوسط الدخول
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = متوسط الدخول
positions-chart-legend-avg-entry-off-scale = متوسط الدخول (خارج المقياس)
positions-chart-dropped-events =
    { $count ->
        [zero] { $count } حدث بلا شمعة في هذا الإطار الزمني
        [one] { $count } حدث بلا شمعة في هذا الإطار الزمني
        [two] { $count } حدثان بلا شمعة في هذا الإطار الزمني
        [few] { $count } أحداث بلا شمعة في هذا الإطار الزمني
        [many] { $count } حدثًا بلا شمعة في هذا الإطار الزمني
       *[other] { $count } حدث بلا شمعة في هذا الإطار الزمني
    }
positions-chart-level = المستوى
# $label names the reference level, $price is its formatted price.
positions-chart-level-above = { $label } { $price } فوق هذا العرض
positions-chart-level-below = { $label } { $price } أسفل هذا العرض
positions-chart-scale-hint = اسحب محور السعر لتصغير المقياس حتى يصل إليه
positions-chart-pnl-at-bar = الأرباح والخسائر @ الشمعة
positions-chart-click-to-locate = انقر لتحديد الموقع
