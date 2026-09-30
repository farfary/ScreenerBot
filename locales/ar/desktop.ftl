# Text owned by the Electron shell: tray, application menu, native dialogs, the
# splash and the boot-error screen. Server-only: read from the packaged catalogs
# by electron/src/l10n.js, never sent to the dashboard. Fatal startup errors
# arrive already rendered from startup.ftl; only their chrome lives here.
#
# Menu roles (Edit, Window, Quit and the like) are not listed: the operating
# system localizes them.

## Actions shared by dialogs.

desktop-action-ok = موافق

## Splash and loading status.

desktop-splash-starting = جارٍ تشغيل { -brand }
desktop-splash-restarting = جارٍ إعادة تشغيل { -brand }
desktop-splash-recovering = جارٍ الاسترداد
desktop-splash-opening-dashboard = جارٍ فتح لوحة التحكم
desktop-splash-checking-dependencies = جارٍ فحص التبعيات
desktop-splash-installing-dependencies = جارٍ تثبيت تبعيات النظام
desktop-splash-installing-dependencies-detail = يحتاج { -brand } إلى Microsoft Visual C++ Redistributable ليعمل.
desktop-splash-resetting-wallet = جارٍ إعادة ضبط بيانات المحفظة
desktop-splash-resetting-wallet-detail = يتم نسخ بيانات المحفظة الحالية احتياطيًا قبل مسحها.
desktop-splash-updating = جارٍ التحديث إلى v{ $version }
desktop-splash-updating-detail = تبقى إعداداتك وبياناتك كما هي تمامًا.
desktop-splash-restoring = جارٍ استعادة v{ $version }
desktop-splash-restoring-detail = لم يبدأ التحديث v{ $failed }، لذا تتولى النسخة السابقة التشغيل.

## Boot-error screen: headings, actions and per-code subtitles.

desktop-boot-title-fallback = تعذّر تشغيل { -brand }
desktop-boot-detail-fallback = توقف الخادم الخلفي بشكل غير متوقع.
desktop-boot-remedy-label = طريقة الإصلاح
desktop-boot-log-file-label = ملف السجل:
desktop-boot-action-reset-wallet = إعادة ضبط بيانات المحفظة وإعادة التشغيل
desktop-boot-action-working = جارٍ العمل...
desktop-boot-action-open-logs = فتح مجلد السجلات
desktop-boot-action-copy = نسخ التفاصيل
desktop-boot-action-copied = تم النسخ
desktop-boot-action-quit = خروج
desktop-boot-subtitle-wallet-mismatch = تم اكتشاف محفظة مختلفة
desktop-boot-subtitle-port-in-use = منفذ شبكة مطلوب مشغول
desktop-boot-subtitle-lock-held = { -brand } قيد التشغيل بالفعل
desktop-boot-subtitle-config-invalid = مشكلة في الإعدادات
desktop-boot-subtitle-directory-setup = مشكلة في التخزين
desktop-boot-subtitle-generic = خطأ في بدء التشغيل

## Boot errors raised by the shell itself (the backend never reported one).

desktop-boot-error-title = تعذّر تشغيل { -brand }
desktop-boot-error-remedy = افتح مجلد السجلات لمعرفة ما حدث، ثم أعد تشغيل التطبيق. إذا استمرت المشكلة، تواصل مع الدعم عبر t.me/screenerbotio_support.
desktop-boot-error-default = توقف الخادم الخلفي بشكل غير متوقع قبل أن تصبح لوحة التحكم جاهزة.
desktop-boot-error-restore-failed = فشل الخادم الخلفي المحدَّث وتعذّرت استعادة النسخة السابقة ({ $error }).
desktop-boot-error-spawn-failed = تعذّر تشغيل برنامج الخادم الخلفي ({ $error }).
desktop-boot-error-spawn-missing = تعذّر تشغيل برنامج الخادم الخلفي. قد يكون مفقودًا أو محظورًا بواسطة برنامج أمان.
desktop-boot-error-exited-running = توقف الخادم الخلفي أثناء تشغيل لوحة التحكم (رمز الخروج { $code }).
desktop-boot-error-exited-early = توقف الخادم الخلفي قبل أن تصبح لوحة التحكم جاهزة (رمز الخروج { $code }).
desktop-boot-error-dashboard-load = فشل تحميل لوحة التحكم ({ $description }، { $code }).
desktop-boot-error-renderer-gone = توقف عارض لوحة التحكم ({ $reason }).
desktop-boot-error-unresponsive = توقفت لوحة التحكم عن الاستجابة.
desktop-boot-error-url-failed = تعذّر تحميل رابط لوحة التحكم ({ $error }).
desktop-boot-error-relaunch-setup = تعذّرت إعادة تشغيل الخادم الخلفي بعد الإعداد.
desktop-boot-error-relaunch-recovery = تعذّرت إعادة تشغيل الخادم الخلفي للاسترداد.
desktop-boot-error-restart-offline = لم يعد الخادم الخلفي إلى العمل بعد إعادة التشغيل.
desktop-boot-error-recovery-offline = انتهى الاسترداد لكن الخادم الخلفي لم يصبح جاهزًا.
desktop-boot-error-start-timeout = لم يكتمل تشغيل الخادم الخلفي في الوقت المناسب. قد يحدث ذلك في أول تشغيل بطيء أو إذا كان برنامج آخر يحظر الاتصال.

## System tray.

desktop-tray-tooltip = { -brand } - بوت تداول Solana
desktop-tray-show = إظهار { -brand }
desktop-tray-open-dashboard = فتح لوحة التحكم
desktop-tray-quit = إنهاء { -brand }

## Menu items shared by the tray and the application menu.

desktop-menu-open-data-folder = فتح مجلد البيانات
desktop-menu-open-logs-folder = فتح مجلد السجلات
desktop-menu-documentation = الوثائق
desktop-menu-telegram-support = دعم { -telegram }
desktop-menu-check-updates = التحقق من التحديثات...

## Application menu.

desktop-menu-file = ملف
desktop-menu-edit = تحرير
desktop-menu-view = عرض
desktop-menu-window = نافذة
desktop-menu-help = مساعدة
desktop-menu-reset-zoom = إعادة ضبط التكبير
desktop-menu-zoom-in = تكبير
desktop-menu-zoom-out = تصغير
desktop-menu-keyboard-shortcuts = اختصارات لوحة المفاتيح
desktop-menu-telegram-channel = قناة { -telegram }
desktop-menu-telegram-community = مجتمع { -telegram }
desktop-menu-follow-x = تابعنا على { -x } ({ -twitter })
desktop-menu-visit-website = زيارة الموقع الإلكتروني
desktop-menu-about = حول { -brand }

## About dialog.

desktop-about-title = حول { -brand }
desktop-about-message = { -brand }
desktop-about-detail =
    الإصدار { $version }

    إدارة محافظ Solana المتقدمة وبوت تداول آلي.

    https://screenerbot.io

    © 2024-2026 { -brand }

## Keyboard shortcuts dialog. Key names stay as typed on the keyboard.

desktop-shortcuts-title = اختصارات لوحة المفاتيح
desktop-shortcuts-message = اختصارات لوحة مفاتيح { -brand }
desktop-shortcuts-body-mac =
    اختصارات لوحة المفاتيح:

    التحكم بالنافذة:
      Cmd+M          تصغير
      Cmd+W          إغلاق النافذة
      Cmd+Q          إنهاء
      Cmd+Ctrl+F     تبديل ملء الشاشة

    التكبير:
      Cmd++          تكبير
      Cmd+-          تصغير
      Cmd+0          إعادة ضبط التكبير

    التنقل:
      Cmd+R          إعادة تحميل لوحة التحكم
      Cmd+Shift+D    فتح مجلد البيانات

    أخرى:
      F1             فتح الوثائق
      Cmd+Alt+I      تبديل أدوات المطوّر
desktop-shortcuts-body-other =
    اختصارات لوحة المفاتيح:

    التحكم بالنافذة:
      Alt+F4         إنهاء
      F11            تبديل ملء الشاشة

    التكبير:
      Ctrl++         تكبير
      Ctrl+-         تصغير
      Ctrl+0         إعادة ضبط التكبير

    التنقل:
      Ctrl+R         إعادة تحميل لوحة التحكم
      Ctrl+Shift+D   فتح مجلد البيانات

    أخرى:
      F1             فتح الوثائق
      Ctrl+Shift+I   تبديل أدوات المطوّر

## Close confirmation (Windows and Linux).

desktop-close-title = إغلاق { -brand }
desktop-close-message = ماذا تريد أن تفعل؟
desktop-close-detail = يمكن أن يستمر { -brand } في العمل في الخلفية. سيواصل بوت التداول المراقبة والتداول أثناء تصغيره إلى علبة النظام.
desktop-close-minimize = تصغير إلى علبة النظام
desktop-close-quit = إنهاء كامل
desktop-close-cancel = إلغاء

## Visual C++ Redistributable (Windows).

desktop-vcredist-missing-title = تبعية مفقودة
desktop-vcredist-missing-message = Visual C++ Redistributable مفقود
desktop-vcredist-missing-detail = يتطلب { -brand } وجود Microsoft Visual C++ Redistributable ليعمل. هل تريد تثبيته الآن؟
desktop-vcredist-install = تثبيت وإصلاح
desktop-vcredist-exit = خروج
desktop-vcredist-not-found-title = لم يتم العثور على المثبّت
desktop-vcredist-not-found-message = تعذّر تحديد موقع { $name } بشكل صحيح.
desktop-vcredist-done-title = اكتمل التثبيت
desktop-vcredist-done-message = تم تثبيت التبعيات بنجاح.
desktop-vcredist-done-detail = سيبدأ { -brand } الآن.
desktop-vcredist-failed-title = فشل التثبيت
desktop-vcredist-failed-message = يرجى تثبيت Visual C++ Redistributable يدويًا.
