# Update system text. Ids come from DeferReason in src/version/types.rs, the
# update check state and the /api/updates responses.

## Why a ready update has not been applied automatically

updates-defer-automatic-install-disabled = التثبيت التلقائي معطّل. التحديث جاهز وسيُطبَّق عندما تختار ذلك.
updates-defer-trading-active = هناك مركز أو صفقة أو عملية أداة نشطة، لذا تم تأجيل إعادة التشغيل. يُطبَّق التحديث تلقائيًا عندما يكون التطبيق خاملًا.
updates-defer-needs-installer = يحدّث هذا الإصدار أيضًا غلاف سطح المكتب، لذا يجب تشغيل المثبّت مرة واحدة.

## Update check failure. `cause` is the technical error text.

updates-check-failed = { $cause }

## Progress and outcome of update actions

updates-download-started = جارٍ تنزيل التحديث v{ $version }...
updates-apply-started = جارٍ تثبيت التحديث. يعيد { -brand } التشغيل ويعيد الاتصال تلقائيًا.
updates-install-opened = تم فتح مثبّت التحديث الذي تم التحقق منه. أكمل مثبّت نظام التشغيل.

# Toast shown by ui/settings/updates_tab.js after the installer is launched.
updates-installer-toast-title = تم فتح المثبّت
updates-installer-toast-message = سيُغلق { -brand } بشكل سليم الآن.

## Settings > Updates (ui/settings/updates_view.js, updates_tab.js)

updates-tab-status = الحالة
updates-tab-release-notes = ملاحظات الإصدار
updates-tab-preferences = التفضيلات
updates-tab-sections = أقسام التحديث
updates-checking-installation = جارٍ فحص هذا التثبيت...

# Status by phase. Ids come from UpdatePhase in src/version/types.rs. The detail of
# a phase that can carry backend text is the fallback shown without it; the detail
# of an available or downloading update describes the update kind instead.
updates-phase-idle-headline = جاهز للتحقق من التحديثات
updates-phase-idle-detail = الإصدار v{ $version } من { -brand } مثبّت.
updates-phase-up-to-date-headline = أنت على أحدث إصدار
updates-phase-up-to-date-detail = الإصدار v{ $version } من { -brand } هو الأحدث.
updates-phase-checking-headline = جارٍ التحقق من التحديثات
updates-phase-checking-detail = جارٍ البحث عن أحدث إصدار منشور.
updates-phase-available-headline = الإصدار { $version } متاح
updates-phase-downloading-headline = جارٍ تنزيل v{ $version }
updates-phase-verifying-headline = جارٍ التحقق من v{ $version }
updates-phase-verifying-detail = جارٍ فحص التنزيل مقابل المجموع الاختباري المنشور.
updates-phase-ready-to-apply-headline = الإصدار { $version } جاهز
updates-phase-ready-to-apply-detail = يمكن تثبيت التحديث الآن مع إعادة تشغيل قصيرة، أو تلقائيًا عند التشغيل التالي.
updates-phase-ready-to-install-headline = الإصدار { $version } جاهز
updates-phase-ready-to-install-detail = مثبّت سطح المكتب جاهز لإنهاء هذا التحديث.
updates-phase-applying-headline = جارٍ تثبيت التحديث
updates-phase-applying-detail = يعيد { -brand } التشغيل على الإصدار الجديد.
updates-phase-applied-headline = تم التحديث إلى v{ $version }
updates-phase-applied-detail = تم تثبيت التحديث. لا حاجة إلى أي إجراء آخر.
updates-phase-failed-headline = لم يكتمل التحديث
updates-phase-failed-detail = حاول التحديث مرة أخرى.
updates-phase-check-failed-headline = تعذّر التحقق من التحديثات
updates-phase-check-failed-detail = تعذّر الوصول إلى خدمة الإصدارات.
updates-status-unavailable-headline = حالة التحديث غير متاحة
updates-phase-unrecognized-detail = حالة التحديث المبلَّغ عنها غير معروفة.
updates-status-load-failed-detail = تعذّر تحميل حالة التثبيت.

# What an available update replaces. Ids come from UpdateKind. $size is a formatted size.
updates-kind-core = تحديث النواة · { $size } · إعادة تشغيل قصيرة
updates-kind-full = تحديث سطح المكتب · { $size } · المثبّت مطلوب
updates-size-unknown = حجم غير معروف

updates-action-check-now = تحقق الآن
updates-action-check-again = تحقق مرة أخرى
updates-action-try-again = حاول مرة أخرى
updates-action-download = تنزيل التحديث
updates-action-restart = إعادة التشغيل للتحديث
updates-action-open-installer = فتح المثبّت

updates-busy-checking = جارٍ التحقق...
updates-busy-resuming = جارٍ استئناف التنزيل...
updates-busy-starting-download = جارٍ بدء التنزيل...
updates-busy-restarting = جارٍ إعادة التشغيل...
updates-busy-opening-installer = جارٍ فتح المثبّت...

updates-progress-downloading = جارٍ تنزيل التحديث
updates-progress-verifying = جارٍ التحقق من التحديث
# $done and $total are formatted sizes.
updates-progress-transferred = { $done } من { $total }
# $percent is a formatted percentage.
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }، { $percent }، { $transferred }

updates-detail-list-label = تفاصيل التثبيت
updates-detail-installed-version = الإصدار المثبّت
updates-detail-system = النظام
updates-detail-last-checked = آخر تحقق
updates-detail-never = أبدًا
updates-detail-available-version = الإصدار المتاح
updates-detail-download-size = حجم التنزيل

updates-version-installed = مثبّت
updates-version-available = متاح

updates-notes-highlights = أبرز التغييرات
updates-notes-empty-title = لا توجد ملاحظات إصدار بعد
updates-notes-empty-error = تعذّر تحميل سجل الإصدارات. تحقق من الاتصال وحاول مرة أخرى.
updates-notes-empty-none = ستظهر ملاحظات الإصدار هنا بعد نشر أول إصدار.
updates-notes-history-notice = يتم عرض ما يعرفه هذا التثبيت بالفعل، إذ تعذّر تحميل سجل الإصدارات.
updates-release-empty = لم يتم إدراج تغييرات لهذا الإصدار.
updates-release-changes =
    { $count ->
        [zero] { $count } تغيير
        [one] { $count } تغيير
        [two] { $count } تغييران
        [few] { $count } تغييرات
        [many] { $count } تغييرًا
       *[other] { $count } تغيير
    }

updates-preferences-unavailable-title = تفضيلات التحديث غير متاحة
updates-preferences-unavailable-detail = تعذّر تحميل إعدادات التحديث.
updates-preference-fallback-name = تفضيل التحديث
updates-preference-save-failed = تعذّر حفظ { $preference }

updates-request-failed = فشل الطلب
updates-check-request-failed = تعذّر التحقق من التحديثات
updates-resume-failed = تعذّر استئناف تنزيل التحديث
updates-download-failed = تعذّر بدء تنزيل التحديث
updates-apply-failed = تعذّر تثبيت التحديث
updates-install-failed = تعذّر فتح مثبّت التحديث
updates-apply-confirm-title = تثبيت v{ $version }
updates-apply-confirm-message = يعيد { -brand } التشغيل على الإصدار الجديد. يتوقف التداول لبضع ثوانٍ ثم يستأنف تلقائيًا، ولا تتأثر المراكز المفتوحة.
updates-install-confirm-title = تشغيل المثبّت
updates-install-confirm-message = يُفتح المثبّت الذي تم التحقق منه ويُغلق { -brand } بشكل سليم. أكمل المثبّت ثم أعد فتح { -brand }.

# A release version as displayed.
updates-version-number = v{ $version }

# The Home update notice, shown while a release is in play.
updates-notice-region =
    .aria-label = حالة التحديث
updates-notice-view = عرض التحديث
updates-notice-whats-new = ما الجديد
updates-notice-available-detail = اطّلع على التغييرات وثبّت التحديث من الإعدادات.
updates-notice-updated-detail = اطّلع على ما تغيّر في هذا الإصدار.
# A headless installation cannot download or install a release itself.
updates-headless-install-detail = تُحدَّث التثبيتات بدون واجهة خارج لوحة التحكم. على Linux، شغّل { "screenerbot-manager update" }.
