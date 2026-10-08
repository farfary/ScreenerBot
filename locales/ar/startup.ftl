# Fatal startup errors. Server-only: rendered by src/errors/startup.rs into the
# finished text that the Electron shell displays, never sent to the dashboard.
#
# Remedies are written for the compiled binary. Paths, ports, wallet addresses
# and error details arrive as arguments. Keep the command-line flag, the file
# names and the support handle unchanged.

## Wallet mismatch.

startup-wallet-mismatch-title = تغيّرت المحفظة
startup-wallet-mismatch-detail =
    المحفظة في إعداداتك لا تطابق المحفظة المسجّلة في السجل المحلي لهذا الحاسوب.

    المحفظة الحالية: { $current }
    المحفظة السابقة: { $stored }

    البيانات المحلية المتأثرة: { $systems }

    يحدث هذا عادةً بعد استيراد مفتاح خاص مختلف أو استعادة إعدادات مختلفة. تعود التداولات والمراكز والسجل إلى المحفظة السابقة ويجب مسحها قبل أن تبدأ المحفظة الجديدة بأمان.
startup-wallet-mismatch-systems-default = المعاملات، المراكز، سجل المحفظة
startup-wallet-mismatch-remedy =
    امسح السجل المحلي للمحفظة السابقة للمتابعة (تُنسخ قواعد بياناتك احتياطيًا تلقائيًا أولًا):

      - في التطبيق: اختر «{ $action }» أدناه.
      - من الطرفية: نفّذ  screenerbot --clean-wallet-data

    لا تتأثر الأموال على السلسلة؛ يُعاد ضبط سجل الصفقات/المراكز المحلي لهذا الحاسوب فقط. تُكتب النسخ الاحتياطية في:
      { $path }
startup-recovery-reset-wallet = إعادة ضبط بيانات المحفظة وإعادة التشغيل

## Port in use.

startup-port-in-use-title = منفذ الشبكة مشغول
startup-port-in-use-detail = منفذ لوحة التحكم { $address } قيد الاستخدام بالفعل.
startup-port-in-use-remedy = برنامج آخر يستخدم المنفذ الذي يحتاجه { -brand }. أغلق ذلك البرنامج، أو غيّر منفذ خادم الويب من الإعدادات، ثم شغّل { -brand } مرة أخرى.

## Another instance is running.

startup-lock-held-title = { -brand } قيد التشغيل بالفعل
startup-lock-held-detail = نسخة أخرى من { -brand } تعمل بالفعل على هذا الحاسوب، لذا لا يمكن تشغيل نسخة ثانية.
startup-lock-held-remedy = انتقل إلى النافذة المفتوحة بالفعل. إذا لم تجد نافذة، فأغلق أي عملية { -brand } في الخلفية وحاول مرة أخرى. إذا استمرت المشكلة بعد إعادة التشغيل، فقد يكون ملف القفل قديمًا ويمكن حذفه من مجلد البيانات (.screenerbot.lock).

## Configuration.

startup-config-invalid-title = تعذّرت قراءة الإعدادات
startup-config-parse-detail = تعذّر تحليل config.toml: { $detail }
startup-config-load-parse-detail = تعذّر تحميل الإعدادات: تعذّر تحليل config.toml: { $detail }
startup-config-parse-remedy = تعذّرت قراءة ملف الإعدادات. استعد نسخة احتياطية من مجلد البيانات، أو أعد ضبط الإعدادات إلى الافتراضية وأعد إعداد محفظتك وRPC.
startup-config-load-parse-remedy = استعد إعدادات صالحة أو أكمل الإعداد مرة أخرى.
startup-option-invalid-title = خيار بدء تشغيل غير صالح
startup-option-invalid-remedy = أحد خيارات سطر الأوامر غير صالح. شغّل { -brand } بدون هذا الخيار، أو صحّحه وحاول مرة أخرى.

## Storage upgrade.

startup-storage-upgrade-title = تعذّرت ترقية بياناتك
startup-storage-upgrade-detail =
    تعذّر على { -brand } ترقية { $database } إلى هذا الإصدار، فتوقف قبل تغييرها. لم تتغير بياناتك.

    السبب:
    { $error }
startup-storage-upgrade-remedy = انسخ التفاصيل وأرسلها مع ملف السجل إلى الدعم على t.me/screenerbotio_support. لا تعدّل قاعدة البيانات ولا تنقلها ولا تحذفها: سيفتحها { -brand } مجددًا بعد تثبيت الإصلاح.

## Generic failures.

startup-generic-title = تعذّر تشغيل { -brand }
startup-generic-remedy = راجع ملف السجل للتفاصيل ثم أعد تشغيل التطبيق. إذا استمرت المشكلة، فتواصل مع الدعم عبر t.me/screenerbotio_support.
startup-generic-detail = { $error }
startup-failure-directories = تعذّر إنشاء المجلدات المطلوبة: { $error }
startup-failure-config-load = تعذّر تحميل الإعدادات: { $error }
startup-failure-actions-init = تعذّرت تهيئة قاعدة بيانات الإجراءات: { $error }
startup-failure-actions-sync = تعذّرت مزامنة الإجراءات من قاعدة البيانات: { $error }
startup-failure-strategy-init = تعذّرت تهيئة نظام الاستراتيجيات: { $error }
startup-failure-analysis-init = تعذّرت تهيئة محرك التحليل: { $error }
startup-failure-assistant-init = تعذّرت تهيئة محرك محادثة المساعد: { $error }
startup-failure-wallets-init = تعذّرت تهيئة المحافظ: { $error }
startup-failure-wallet-validation = تعذّر التحقق من اتساق المحفظة: { $error }
