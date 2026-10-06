# Trade dialog messages.

## Quote panel

# Shown in the quote panel when the quote request could not reach the core.
trade-quote-error-network = تعذّر جلب عرض السعر، تحقق من اتصالك وحاول مرة أخرى
# Fallback title when the quote request failed without a message.
trade-quote-error-title = تعذّر جلب عرض السعر
trade-quote-title = معاينة المبادلة
trade-quote-refresh =
    .aria-label = تحديث عرض السعر
    .title = تحديث عرض السعر
trade-quote-idle = اختر مبلغًا لمعاينة المبادلة
trade-quote-loading = جارٍ البحث عن أفضل مسار…
trade-quote-retry = حاول مرة أخرى
trade-quote-pay = أنت تدفع
trade-quote-receive = أنت تستلم (تقديري)
trade-quote-minimum = الحد الأدنى المضمون
    .title = أقل ما يمكنك استلامه بعد أقصى انزلاق سعري. تُلغى المبادلة بدلًا من التنفيذ دون هذا الحد.
trade-quote-impact = التأثير على السعر
trade-quote-slippage = أقصى انزلاق سعري
trade-quote-platform-fee = رسوم المنصة
    .title = 0.5% لدعم التطوير. مضمّنة بالفعل في عرض السعر أعلاه.
trade-quote-network-fee = رسوم الشبكة
trade-quote-route = المسار
trade-quote-disclaimer = تتحدث الأسعار مباشرة من السلسلة. تُلغى المبادلة إذا تعذّر تنفيذها فوق حدك الأدنى المضمون، فلن تستلم أقل مما هو معروض.
# Price impact below the resolution of the percentage display.
trade-quote-impact-tiny = { "<0.01%" }
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-quote-impact-warning = التأثير على السعر { $impact } أعلى من أقصى انزلاق سعري لديك ({ $tolerance }%)، وهذا الحجم يحرّك مجمع السيولة. مبلغ أصغر يُنفَّذ أقرب إلى سعر السوق.

## Units

trade-unit-native = { -sol }
trade-unit-tokens = رموز

## Actions. Ids are the dialog actions: buy, sell, add.

trade-buy-title = شراء رمز
trade-buy-subtitle = أدخل المبلغ بـ { -sol }
trade-buy-confirm = تنفيذ الشراء
trade-buy-hint = اتركه فارغًا لاستخدام الافتراضي من الإعدادات
trade-sell-title = بيع المركز
trade-sell-subtitle = اختر نسبة البيع
trade-sell-confirm = تنفيذ البيع
trade-sell-hint = أدخل قيمة بين 1-100
trade-sell-input = نسبة مخصصة
    .placeholder = 1-100
trade-add-title = إضافة إلى المركز
trade-add-subtitle = DCA في المركز الحالي
trade-add-confirm = إضافة إلى المركز
trade-add-hint = اتركه فارغًا لاستخدام حجم DCA المضبوط
trade-amount-input = مبلغ مخصص
    .placeholder = أدخل مبلغ { -sol }

## Presets

trade-presets-quick-amount = مبلغ سريع
trade-presets-quick-sell = بيع سريع
trade-presets-match-entry = مطابقة الدخول
trade-presets-fixed-amount = مبلغ ثابت
trade-preset-partial = جزئي
trade-preset-half = النصف
trade-preset-most = الأغلب
trade-preset-full = خروج كامل
# $label is the preset's amount.
trade-preset-select =
    .aria-label = اختيار { $label }

## Dialog chrome

trade-dialog-close =
    .aria-label = إغلاق النافذة
trade-input-max = الأقصى
    .aria-label = استخدام الحد الأقصى
trade-slider =
    .aria-label = شريط تمرير المبلغ
trade-context-available = المتاح
trade-context-position-size = حجم المركز
trade-context-holdings = الحيازات
trade-held-badge = محتفظ به
    .title = لديك مركز مفتوح في هذا الرمز
trade-manage-title = إدارة يدوية
trade-manage-description = لن يبيع المتداول الآلي هذا المركز ولن ينفّذ DCA عليه. ألغِ التحديد ليتولى إدارة الخروج.

## Slippage

trade-slippage-label = الانزلاق السعري
trade-slippage-presets =
    .aria-label = إعداد الانزلاق السعري المسبق
trade-slippage-auto = تلقائي
trade-slippage-custom =
    .placeholder = مخصص
    .aria-label = نسبة الانزلاق السعري المخصصة
trade-slippage-note-auto = تلقائي (من الإعدادات)
# $pct is the configured slippage as stored.
trade-slippage-note-auto-value = تلقائي ({ $pct }% من الإعدادات)
# $pct is the override as typed.
trade-slippage-note-override = تجاوز: { $pct }%
# $pct is the override as typed.
trade-slippage-warning = انزلاق سعري مرتفع: قد تستلم أقل من المعروض بما يصل إلى { $pct }%.
trade-impact-warning-title = تحذير من تأثير مرتفع على السعر
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-impact-warning-text = تأثير هذه الصفقة على السعر <strong>{ $impact }</strong>، وهو يتجاوز تحمّلك للانزلاق السعري البالغ <strong>{ $tolerance }%</strong>. قد تستلم أقل بكثير مما هو متوقع.
trade-impact-warning-proceed = المتابعة على أي حال

## Validation and verification

trade-error-invalid-number = رقم غير صالح
trade-error-percentage-range = يجب أن تكون النسبة بين 1 و100
trade-error-amount-positive = يجب أن يكون المبلغ أكبر من 0
trade-error-amount-minimum = الحد الأدنى: 0.001 { -sol }
# $needed is a formatted SOL amount, $reserve the fee headroom in SOL and $balance the formatted balance.
trade-error-insufficient = الرصيد غير كافٍ (المطلوب { $needed } إضافة إلى { $reserve } للرسوم، والمتوفر { $balance })
trade-error-position-closed = هذا المركز لم يعد مفتوحًا.
trade-error-verify-failed = تعذّر التحقق من رصيد الرمز
trade-error-position-missing = لم يتم العثور على المركز، وربما أُغلق
# $expected and $current are formatted token amounts.
trade-error-balance-changed = تغير رصيد الرمز. المتوقع { $expected } والحالي { $current }. يرجى التحديث.
trade-error-verify-network = خطأ في الشبكة أثناء التحقق من الرصيد

## Quick trade

trade-quick-buy-title = شراء سريع
trade-quick-sell-title = بيع سريع
trade-quick-subtitle = أدخل عنوان إصدار الرمز
trade-quick-mint-label = أدخل عنوان إصدار الرمز
trade-quick-mint-input =
    .placeholder = أدخل عنوان الإصدار أو ابحث بالرمز المختصر...
trade-quick-paste =
    .aria-label = لصق من الحافظة
trade-quick-recent = الأحدث:
trade-quick-fetching = جارٍ جلب معلومات الرمز...
trade-quick-continue = متابعة
trade-quick-token-not-found = لم يتم العثور على الرمز
trade-quick-token-failed = فشل جلب الرمز
trade-quick-token-not-in-database = لم يتم العثور على الرمز في قاعدة البيانات
trade-quick-token-info-failed = فشل جلب معلومات الرمز
trade-quick-no-position = لم يتم العثور على مركز لهذا الرمز
trade-quick-no-holdings = لا توجد رموز متبقية في المركز
trade-quick-position-failed = فشل جلب بيانات المركز

## Manual trade toasts

trade-toast-no-mint = لا يتوفر عنوان إصدار
trade-toast-open-failed = تعذّر فتح نافذة الصفقة
trade-toast-pending-buy = الشراء لا يزال قيد التنفيذ
trade-toast-pending-add = الإضافة لا تزال قيد التنفيذ
trade-toast-pending-sell = البيع لا يزال قيد التنفيذ
trade-toast-pending-message = توقف المتصفح عن الانتظار، تابع صف المركز لمعرفة النتيجة
trade-toast-failed-buy = فشل الشراء
trade-toast-failed-add = فشلت الإضافة إلى المركز
trade-toast-failed-sell = فشل البيع

# Trade and close reasons. Ids are the Debug names of TradeReason
# (src/trader/types.rs) and the reasons written by src/positions.
trade-reason-strategy-signal = إشارة استراتيجية
trade-reason-manual-entry = دخول يدوي
trade-reason-force-buy = شراء قسري
trade-reason-copy-buy = شراء بالنسخ
trade-reason-dca-scheduled = DCA مجدول
trade-reason-take-profit = جني الأرباح
trade-reason-stop-loss = وقف الخسارة
trade-reason-trailing-stop = وقف متحرك
trade-reason-time-override = تجاوز زمني
trade-reason-strategy-exit = خروج الاستراتيجية
trade-reason-llm-analysis-exit = خروج بتحليل LLM
trade-reason-manual-exit = خروج يدوي
trade-reason-risk-management = إدارة المخاطر
trade-reason-blacklisted = في القائمة السوداء
trade-reason-force-sell = بيع قسري
trade-reason-copy-sell = بيع بالنسخ
trade-reason-closed-externally = أُغلق خارجيًا
trade-reason-wallet-history = سجل المحفظة
trade-reason-exit-retry-pending = إعادة محاولة الخروج معلّقة
trade-reason-synthetic-exit-permanent-failure = فشل دائم في الخروج الاصطناعي
# $reason is the label of the base reason. Applies to a closed_reason that
# carries the pending-verification suffix.
trade-reason-pending-verification = { $reason } (بانتظار التحقق)
# $note is the operator text of a force close.
trade-reason-force-closed = إغلاق قسري: { $note }
# $reason is a stored closed_reason that has no label; it is shown as stored.
trade-reason-stored = { $reason }

# Toast shown when a quick-trade shortcut runs without a token selected (ui/quick_trade_shortcuts.js).
trade-quick-no-token = لم يتم اختيار رمز
