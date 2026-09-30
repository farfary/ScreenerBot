notifications-action-swap-buy = شراء
notifications-action-swap-sell = بيع
notifications-action-position-open = فتح
notifications-action-position-close = إغلاق
notifications-action-position-dca = DCA
notifications-action-position-partial-exit = خروج جزئي
notifications-action-manual-order = يدوي
notifications-action-unknown = إجراء

actions-step-evaluate = جارٍ التقييم
actions-step-validate = جارٍ التحقق
actions-step-quote = جارٍ الحصول على عرض السعر
actions-step-swap = جارٍ تنفيذ المبادلة
actions-step-verify = جارٍ التحقق من التأكيد
actions-step-unknown = جارٍ المعالجة
actions-step-evaluate-short = تقييم
actions-step-validate-short = فحص
actions-step-quote-short = عرض سعر
actions-step-swap-short = مبادلة
actions-step-verify-short = تأكيد
actions-step-unknown-short = قيد العمل

actions-failure-recorded = { $message }
actions-failure-unknown = خطأ غير معروف
actions-failure-interrupted = تمت المقاطعة بسبب إعادة تشغيل التطبيق
actions-failure-validation = فشل التحقق
actions-failure-quote = فشل الحصول على عرض السعر
actions-failure-swap = فشلت المبادلة
actions-failure-trade = فشلت الصفقة
actions-failure-entry = فشل الدخول
actions-failure-exit = فشل الخروج
actions-failure-dca = فشل DCA
actions-failure-verification-expired = انتهت صلاحية التحقق: لم تصل المعاملة إلى الشبكة
actions-failure-verification-gave-up = توقف التحقق عن المحاولة
actions-failure-transaction-failed = فشلت المعاملة على السلسلة
actions-failure-sell-transaction-failed = فشلت معاملة البيع على السلسلة
actions-failure-dca-verification-failed = فشل التحقق من DCA

notifications-empty-all = لا توجد إجراءات
notifications-empty-active = لا توجد إجراءات نشطة
notifications-empty-completed = لا توجد إجراءات مكتملة
notifications-empty-failed = لا توجد إجراءات فاشلة
notifications-source-auto = تلقائي
notifications-source-manual = يدوي
notifications-state-locked = الحالة يتحكم بها التبويب
notifications-cancelled = ملغى
notifications-dismiss = تجاهل
notifications-dismiss-failed = فشل تجاهل الإشعار
notifications-load-failed = فشل التحميل
notifications-mark-read-failed = فشل تعليم الإشعارات كمقروءة
notifications-clear-title = مسح الإشعارات
notifications-clear-message = هل تريد تجاهل جميع الإشعارات في هذه القائمة؟ ستبقى في سجل المكتملة/الفاشلة.
notifications-clear-failed = فشل مسح الإشعارات
notifications-stream-lag-title = تأخر تدفق الإجراءات
notifications-stream-lag-missed =
    { $count ->
        [zero] فاتتنا { $count } تحديث — جارٍ التحديث
        [one] فاتنا تحديث ({ $count }) — جارٍ التحديث
        [two] فاتنا تحديثان ({ $count }) — جارٍ التحديث
        [few] فاتتنا { $count } تحديثات — جارٍ التحديث
        [many] فاتنا { $count } تحديثًا — جارٍ التحديث
       *[other] فاتنا { $count } تحديث — جارٍ التحديث
    }
notifications-stream-lag-refreshing = جارٍ التحديث
notifications-sync-failed = تعذّر تحديث الإجراءات
