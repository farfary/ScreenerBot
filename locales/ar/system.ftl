# Results of system and configuration operations. Ids come from
# src/webserver/routes/config/operations.rs (diff) and
# src/webserver/routes/config/import_export.rs (import).

system-result-config-differs = الإعدادات في الذاكرة تختلف عن النسخة على القرص
system-result-config-matches = الإعدادات في الذاكرة تطابق النسخة على القرص

# $count is the number of imported sections, $warnings the number of problems and
# $details their text joined with commas.
system-result-config-imported =
    تم استيراد { $count ->
        [zero] { $count } قسم
        [one] { $count } قسم
        [two] { $count } قسمين
        [few] { $count } أقسام
        [many] { $count } قسمًا
       *[other] { $count } قسم
    } بنجاح
system-result-config-imported-with-warnings =
    تم استيراد { $count ->
        [zero] { $count } قسم
        [one] { $count } قسم
        [two] { $count } قسمين
        [few] { $count } أقسام
        [many] { $count } قسمًا
       *[other] { $count } قسم
    } مع { $warnings ->
        [zero] { $warnings } تحذير
        [one] { $warnings } تحذير
        [two] { $warnings } تحذيرين
        [few] { $warnings } تحذيرات
        [many] { $warnings } تحذيرًا
       *[other] { $warnings } تحذير
    }: { $details }

# The Config page (pages/config.js, config.html and pages/config/*) and the
# import/export dialog (ui/config_import_export_dialog.js). Field labels, hints,
# units and section names come from config.ftl; only the page's own text is here.

## Config page: sidebar and toolbar

system-config-search =
    .placeholder = البحث في الإعدادات...
system-config-export-title =
    .title = تصدير الإعدادات إلى ملف
system-config-import-title =
    .title = استيراد الإعدادات من ملف
system-config-reload = إعادة التحميل من القرص
system-config-reset-defaults = إعادة الضبط إلى الافتراضي
system-config-select-section = اختر قسم إعدادات
system-config-select-section-details = اختر قسم إعدادات لعرض التفاصيل.
system-config-no-metadata = لا توجد بيانات وصفية لـ <code>{ $section }</code>
system-config-technical-settings = الإعدادات التقنية
system-config-expand-title = توسيع كل قسم وكل إعداد فرعي متداخل
system-config-collapse-title = طيّ كل قسم وكل إعداد فرعي متداخل
system-config-toolbar-no-changes = لا توجد تغييرات في القسم
system-config-toolbar-section-changes =
    { $count ->
        [zero] التغييرات في القسم: <strong>{ $count }</strong>
        [one] التغييرات في القسم: <strong>{ $count }</strong>
        [two] التغييرات في القسم: <strong>{ $count }</strong>
        [few] التغييرات في القسم: <strong>{ $count }</strong>
        [many] التغييرات في القسم: <strong>{ $count }</strong>
       *[other] التغييرات في القسم: <strong>{ $count }</strong>
    }
system-config-toolbar-total-changes =
    { $count ->
        [zero] إجمالي التغييرات: <strong>{ $count }</strong>
        [one] إجمالي التغييرات: <strong>{ $count }</strong>
        [two] إجمالي التغييرات: <strong>{ $count }</strong>
        [few] إجمالي التغييرات: <strong>{ $count }</strong>
        [many] إجمالي التغييرات: <strong>{ $count }</strong>
       *[other] إجمالي التغييرات: <strong>{ $count }</strong>
    }

## Config page: state banner

system-config-loading = جارٍ تحميل الإعدادات…
system-config-refreshing = جارٍ تحديث الإعدادات…
system-config-saving-title = جارٍ حفظ التغييرات…
system-config-saving-detail = جارٍ تحديث الإعدادات
system-config-validation-issues = <strong>تم اكتشاف مشكلات في التحقق.</strong> يرجى مراجعة الحقول المميزة.

## Config page: section header and category chips

system-config-save-changes = حفظ التغييرات
system-config-saving = جارٍ الحفظ…
system-config-compare = مقارنة مع القرص
system-config-revert-section = التراجع عن تغييرات القسم
system-config-summary-critical = حرجة: { $count }
system-config-summary-performance = الأداء: { $count }
system-config-summary-pending =
    { $count ->
        [zero] { $count } تغيير معلّق
        [one] { $count } تغيير معلّق
        [two] { $count } تغييران معلّقان
        [few] { $count } تغييرات معلّقة
        [many] { $count } تغييرًا معلّقًا
       *[other] { $count } تغيير معلّق
    }
system-config-summary-none = لا يوجد ملخص للبيانات الوصفية
system-config-fields-count =
    { $count ->
        [zero] { $count } حقل
        [one] { $count } حقل
        [two] { $count } حقلان
        [few] { $count } حقول
        [many] { $count } حقلًا
       *[other] { $count } حقل
    }
# $fields is the field count above; $pending and $visible are counts.
system-config-chip-pending = { $fields } · المعلّقة: { $pending }
system-config-chip-visible = { $visible } من { $fields }

## Config page: field rows

system-config-field-unit = الوحدة: { $unit }
system-config-field-default = الافتراضي: { $value }
system-config-field-reset = إعادة إلى الافتراضي
system-config-array-invalid-title = مدخل مصفوفة غير صالح
system-config-json-invalid-title = JSON غير صالح
system-config-list-separator = { "، " }
# Ids of the array-entry messages come from FieldType in src/config/metadata.rs.
# $lines is the list of offending line numbers.
system-config-array-invalid-integer =
    { $count ->
        [zero] السطر { $lines } يجب أن يكون عددًا صحيحًا صالحًا.
        [one] السطر { $lines } يجب أن يكون عددًا صحيحًا صالحًا.
        [two] الأسطر { $lines } يجب أن تكون أعدادًا صحيحة صالحة.
        [few] الأسطر { $lines } يجب أن تكون أعدادًا صحيحة صالحة.
        [many] الأسطر { $lines } يجب أن تكون أعدادًا صحيحة صالحة.
       *[other] الأسطر { $lines } يجب أن تكون أعدادًا صحيحة صالحة.
    }
system-config-array-invalid-number =
    { $count ->
        [zero] السطر { $lines } يجب أن يكون رقمًا صالحًا.
        [one] السطر { $lines } يجب أن يكون رقمًا صالحًا.
        [two] الأسطر { $lines } يجب أن تكون أرقامًا صالحة.
        [few] الأسطر { $lines } يجب أن تكون أرقامًا صالحة.
        [many] الأسطر { $lines } يجب أن تكون أرقامًا صالحة.
       *[other] الأسطر { $lines } يجب أن تكون أرقامًا صالحة.
    }
system-config-array-invalid-boolean =
    { $count ->
        [zero] السطر { $lines } يجب أن يكون قيمة منطقية صالحة.
        [one] السطر { $lines } يجب أن يكون قيمة منطقية صالحة.
        [two] الأسطر { $lines } يجب أن تكون قيمًا منطقية صالحة.
        [few] الأسطر { $lines } يجب أن تكون قيمًا منطقية صالحة.
        [many] الأسطر { $lines } يجب أن تكون قيمًا منطقية صالحة.
       *[other] الأسطر { $lines } يجب أن تكون قيمًا منطقية صالحة.
    }
system-config-array-invalid-value =
    { $count ->
        [zero] السطر { $lines } يجب أن يكون قيمة صالحة.
        [one] السطر { $lines } يجب أن يكون قيمة صالحة.
        [two] الأسطر { $lines } يجب أن تكون قيمًا صالحة.
        [few] الأسطر { $lines } يجب أن تكون قيمًا صالحة.
        [many] الأسطر { $lines } يجب أن تكون قيمًا صالحة.
       *[other] الأسطر { $lines } يجب أن تكون قيمًا صالحة.
    }

## Config page: Telegram actions

system-config-telegram-actions = الإجراءات
system-config-telegram-test-title = اختبار الاتصال
system-config-telegram-test-description = أرسل رسالة اختبار للتحقق من عمل إعدادات { -telegram } لديك
system-config-telegram-send-test = إرسال رسالة اختبار
system-config-telegram-sending = جارٍ الإرسال...
system-config-telegram-configure-token-title = اضبط رمز البوت أولًا
system-config-telegram-configure-token-status = اضبط رمز البوت أعلاه لتفعيل الاختبار
system-config-telegram-test-sent-status = تم إرسال رسالة الاختبار بنجاح! تحقق من { -telegram }.
system-config-telegram-test-sent = تم إرسال رسالة اختبار { -telegram }
system-config-telegram-test-failed = فشل إرسال رسالة الاختبار
system-config-telegram-auth-title = مصادقة البوت
system-config-telegram-totp-title = المصادقة الثنائية (TOTP)
system-config-telegram-totp-configured = مضبوطة
system-config-telegram-totp-not-configured = غير مضبوطة
system-config-telegram-totp-active = المصادقة الثنائية مفعّلة. تتطلب جلسات { -telegram } المنتهية رمز TOTP من تطبيق المصادقة لديك.
system-config-telegram-totp-inactive = فعّل المصادقة الثنائية في إعدادات الأمان لحماية أوامر { -telegram }.
system-config-telegram-totp-note = يتشارك TOTP مع شاشة قفل لوحة التحكم. اضبطه في إعدادات الأمان.
system-config-telegram-require-2fa = طلب 2FA للأوامر
# $status is the HTTP status code.
system-config-telegram-save-rejected = تم رفض الحفظ ({ $status })
system-config-telegram-save-failed = تعذّر حفظ إعداد { -telegram }

## Config page: operations

system-config-saved = تم حفظ الإعدادات
system-config-save-failed = تعذّر حفظ الإعدادات
system-config-reloaded = تمت إعادة تحميل الإعدادات من القرص
system-config-reload-failed = تعذّرت إعادة تحميل الإعدادات
system-config-diff-title = فروقات الإعدادات
system-config-diff-console = تمت الكتابة في وحدة تحكم المتصفح
system-config-diff-failed = تعذّر حساب الفروقات
system-config-reset-title = إعادة ضبط الإعدادات
system-config-reset-message =
    سيؤدي ذلك إلى إعادة ضبط الإعدادات بالكامل إلى القيم الافتراضية المضمّنة. ستفقد كل الإعدادات الحالية.

    لا يمكن التراجع عن هذا الإجراء.
system-config-reset-done-title = تمت إعادة ضبط الإعدادات
system-config-reset-done-message = تمت استعادة كل الإعدادات إلى القيم الافتراضية
system-config-reset-failed = تعذّرت إعادة ضبط الإعدادات
system-config-load-failed = تعذّر تحميل الإعدادات
system-config-metadata-failed = تعذّر تحميل البيانات الوصفية للإعدادات

## Import and export dialogs: shared

system-config-dialog-close =
    .aria-label = إغلاق
system-config-select-none = إلغاء تحديد الكل
system-config-section-gui = الواجهة
system-config-changes-count =
    { $count ->
        [zero] { $count } تغيير
        [one] { $count } تغيير
        [two] { $count } تغييران
        [few] { $count } تغييرات
        [many] { $count } تغييرًا
       *[other] { $count } تغيير
    }
system-config-sections-count =
    { $count ->
        [zero] { $count } قسم
        [one] { $count } قسم
        [two] { $count } قسمان
        [few] { $count } أقسام
        [many] { $count } قسمًا
       *[other] { $count } قسم
    }

## Import and export dialogs: section descriptions. Ids are the section names of
## src/webserver/routes/config/import_export.rs.

system-config-section-hint-chains = تفعيل السلاسل ونقاط اتصال RPC وتوجيه المبادلات
system-config-section-hint-trader = قواعد التداول والأتمتة
system-config-section-hint-positions = إعدادات إدارة المراكز
system-config-section-hint-filtering = قواعد ترشيح الرموز وحدودها
system-config-section-hint-tokens = اكتشاف الرموز ومصادر البيانات
system-config-section-hint-events = إعدادات تسجيل الأحداث
system-config-section-hint-services = إعدادات خدمات الخلفية
system-config-section-hint-monitoring = إعدادات مراقبة النظام
system-config-section-hint-ohlcv = إعدادات بيانات الشموع
system-config-section-hint-gui = إعدادات لوحة التحكم والواجهة
system-config-section-hint-telegram = إعدادات بوت { -telegram }

## Export dialog

system-config-export-dialog-title = تصدير الإعدادات
system-config-export-intro = اختر أقسام الإعدادات المراد تصديرها. يمكن استيراد الملف المصدَّر لاحقًا لاستعادة الإعدادات أو مشاركتها.
system-config-export-sections = الأقسام
system-config-export-timestamp = تضمين الطابع الزمني للتصدير
system-config-sections-selected =
    { $count ->
        [zero] الأقسام المحددة: { $count }
        [one] الأقسام المحددة: { $count }
        [two] الأقسام المحددة: { $count }
        [few] الأقسام المحددة: { $count }
        [many] الأقسام المحددة: { $count }
       *[other] الأقسام المحددة: { $count }
    }
system-config-exporting = جارٍ التصدير...
system-config-export-invalid-response = استجابة غير صالحة من الخادم
system-config-exported-title = تم تصدير الإعدادات
system-config-exported-message =
    { $count ->
        [zero] تم تصدير { $count } قسم
        [one] تم تصدير { $count } قسم
        [two] تم تصدير { $count } قسمين
        [few] تم تصدير { $count } أقسام
        [many] تم تصدير { $count } قسمًا
       *[other] تم تصدير { $count } قسم
    }
system-config-export-failed-title = فشل التصدير
system-config-export-failed = فشل تصدير الإعدادات

## Import dialog

system-config-import-dialog-title = استيراد الإعدادات
system-config-import-upload-intro = ارفع ملف إعدادات مُصدَّرًا سابقًا. ستتمكن من معاينة الأقسام واختيار ما تريد استيراده.
system-config-import-dropzone-title = أفلت ملف الإعدادات هنا
system-config-import-dropzone-hint = أو انقر للتصفح
system-config-import-analyzing = جارٍ تحليل الإعدادات...
system-config-import-preview = معاينة
system-config-import-preview-intro = راجع أقسام الإعدادات أدناه، ثم اختر الأقسام المراد استيرادها.
system-config-import-sections = الأقسام في الملف
system-config-import-select-valid = تحديد كل الصالح
system-config-import-merge-label = الدمج مع الموجود
system-config-import-merge-hint = تحديث الحقول الموجودة في الملف فقط. عند عدم التحديد يتم استبدال الأقسام بالكامل.
system-config-import-save-label = الحفظ على القرص
system-config-import-save-hint = حفظ التغييرات في config.toml بعد الاستيراد
system-config-import-selected = استيراد المحدد
system-config-import-warnings =
    { $count ->
        [zero] التحذيرات: { $count }
        [one] التحذيرات: { $count }
        [two] التحذيرات: { $count }
        [few] التحذيرات: { $count }
        [many] التحذيرات: { $count }
       *[other] التحذيرات: { $count }
    }
# $section is a section name from the file, $field a dotted setting path, $detail the
# technical reason a section failed to parse.
system-config-import-warning-unknown-section = سيتم تجاهل القسم غير المعروف «{ $section }»
system-config-import-warning-sensitive-field = قد يؤدي استيراد { $field } إلى استبدال إعدادات المصادقة
system-config-import-section-error = { $detail }
# $sections and $changes are the counts above, already worded.
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = غير موجود في الملف
system-config-import-status-invalid = إعدادات غير صالحة
system-config-import-status-unchanged = لا توجد تغييرات
system-config-import-not-included = غير مضمَّن في الملف
system-config-import-show-changes = إظهار التغييرات
system-config-import-hide-changes = إخفاء التغييرات
system-config-import-value-current = القيمة الحالية
system-config-import-value-new = القيمة الجديدة
system-config-import-more-changes =
    { $count ->
        [zero] +{ $count } تغيير إضافي
        [one] +{ $count } تغيير إضافي
        [two] +{ $count } تغييران إضافيان
        [few] +{ $count } تغييرات إضافية
        [many] +{ $count } تغييرًا إضافيًا
       *[other] +{ $count } تغيير إضافي
    }
system-config-import-value-items =
    { "[" }{ $count ->
        [zero] { $count } عنصر
        [one] { $count } عنصر
        [two] { $count } عنصران
        [few] { $count } عناصر
        [many] { $count } عنصرًا
       *[other] { $count } عنصر
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
        [zero] { $count } مفتاح
        [one] { $count } مفتاح
        [two] { $count } مفتاحان
        [few] { $count } مفاتيح
        [many] { $count } مفتاحًا
       *[other] { $count } مفتاح
    }{ "}" }
system-config-importing = جارٍ الاستيراد...
system-config-import-failed = فشل الاستيراد
system-config-import-invalid-file-title = ملف غير صالح
system-config-import-invalid-file = فشل تحليل ملف الإعدادات
system-config-imported-title = تم استيراد الإعدادات
system-config-imported-message =
    { $count ->
        [zero] تم استيراد { $count } قسم
        [one] تم استيراد { $count } قسم
        [two] تم استيراد { $count } قسمين
        [few] تم استيراد { $count } أقسام
        [many] تم استيراد { $count } قسمًا
       *[other] تم استيراد { $count } قسم
    }
system-config-import-failed-title = فشل الاستيراد
system-config-import-failed-message = فشل استيراد الإعدادات
