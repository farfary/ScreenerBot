# Results of system and configuration operations. Ids come from
# src/webserver/routes/config/operations.rs (diff) and
# src/webserver/routes/config/import_export.rs (import).

system-result-config-differs = Конфігурація в пам’яті відрізняється від версії на диску
system-result-config-matches = Конфігурація в пам’яті збігається з версією на диску

# $count is the number of imported sections, $warnings the number of problems and
# $details their text joined with commas.
system-result-config-imported =
    Успішно імпортовано { $count ->
        [one] { $count } розділ
        [few] { $count } розділи
        [many] { $count } розділів
       *[other] { $count } розділу
    }
system-result-config-imported-with-warnings =
    Імпортовано { $count ->
        [one] { $count } розділ
        [few] { $count } розділи
        [many] { $count } розділів
       *[other] { $count } розділу
    } з { $warnings ->
        [one] { $warnings } попередженням
        [few] { $warnings } попередженнями
        [many] { $warnings } попередженнями
       *[other] { $warnings } попередження
    }: { $details }

# The Config page (pages/config.js, config.html and pages/config/*) and the
# import/export dialog (ui/config_import_export_dialog.js). Field labels, hints,
# units and section names come from config.ftl; only the page's own text is here.

## Config page: sidebar and toolbar

system-config-search =
    .placeholder = Пошук налаштувань...
system-config-export-title =
    .title = Експортувати конфігурацію у файл
system-config-import-title =
    .title = Імпортувати конфігурацію з файлу
system-config-reload = Перезавантажити з диска
system-config-reset-defaults = Скинути до типових
system-config-select-section = Виберіть розділ конфігурації
system-config-select-section-details = Виберіть розділ конфігурації, щоб переглянути подробиці.
system-config-no-metadata = Немає метаданих для <code>{ $section }</code>
system-config-technical-settings = Технічні налаштування
system-config-expand-title = Розгорнути кожен розділ і кожну вкладену підконфігурацію
system-config-collapse-title = Згорнути кожен розділ і кожну вкладену підконфігурацію
system-config-toolbar-no-changes = Немає змін у розділі
system-config-toolbar-section-changes =
    { $count ->
        [one] <strong>{ $count }</strong> зміна в розділі
        [few] <strong>{ $count }</strong> зміни в розділі
        [many] <strong>{ $count }</strong> змін у розділі
       *[other] <strong>{ $count }</strong> зміни в розділі
    }
system-config-toolbar-total-changes =
    { $count ->
        [one] <strong>{ $count }</strong> зміна загалом
        [few] <strong>{ $count }</strong> зміни загалом
        [many] <strong>{ $count }</strong> змін загалом
       *[other] <strong>{ $count }</strong> зміни загалом
    }

## Config page: state banner

system-config-loading = Завантаження конфігурації…
system-config-refreshing = Оновлення конфігурації…
system-config-saving-title = Збереження змін…
system-config-saving-detail = Оновлення конфігурації
system-config-validation-issues = <strong>Виявлено проблеми перевірки.</strong> Перегляньте виділені поля.

## Config page: section header and category chips

system-config-save-changes = Зберегти зміни
system-config-saving = Збереження…
system-config-compare = Порівняти з диском
system-config-revert-section = Скасувати зміни розділу
system-config-summary-critical = Критичних: { $count }
system-config-summary-performance = Щодо продуктивності: { $count }
system-config-summary-pending =
    { $count ->
        [one] { $count } зміна очікує
        [few] { $count } зміни очікують
        [many] { $count } змін очікують
       *[other] { $count } зміни очікують
    }
system-config-summary-none = Немає зведення метаданих
system-config-fields-count =
    { $count ->
        [one] { $count } поле
        [few] { $count } поля
        [many] { $count } полів
       *[other] { $count } поля
    }
# $fields is the field count above; $pending and $visible are counts.
system-config-chip-pending = { $fields } · очікують: { $pending }
system-config-chip-visible = { $visible } із { $fields }

## Config page: field rows

system-config-field-default = Типове: { $value }
system-config-field-reset = Скинути до типового
system-config-array-invalid-title = Недійсний елемент масиву
system-config-json-invalid-title = Недійсний JSON
system-config-list-separator = { ", " }
# Ids of the array-entry messages come from FieldType in src/config/metadata.rs.
# $lines is the list of offending line numbers.
system-config-array-invalid-integer =
    { $count ->
        [one] Рядок { $lines } має бути дійсним цілим числом.
        [few] Рядки { $lines } мають бути дійсними цілими числами.
        [many] Рядки { $lines } мають бути дійсними цілими числами.
       *[other] Рядки { $lines } мають бути дійсними цілими числами.
    }
system-config-array-invalid-number =
    { $count ->
        [one] Рядок { $lines } має бути дійсним числом.
        [few] Рядки { $lines } мають бути дійсними числами.
        [many] Рядки { $lines } мають бути дійсними числами.
       *[other] Рядки { $lines } мають бути дійсними числами.
    }
system-config-array-invalid-boolean =
    { $count ->
        [one] Рядок { $lines } має бути дійсним логічним значенням.
        [few] Рядки { $lines } мають бути дійсними логічними значеннями.
        [many] Рядки { $lines } мають бути дійсними логічними значеннями.
       *[other] Рядки { $lines } мають бути дійсними логічними значеннями.
    }
system-config-array-invalid-value =
    { $count ->
        [one] Рядок { $lines } має бути дійсним значенням.
        [few] Рядки { $lines } мають бути дійсними значеннями.
        [many] Рядки { $lines } мають бути дійсними значеннями.
       *[other] Рядки { $lines } мають бути дійсними значеннями.
    }

## Config page: Telegram actions

system-config-telegram-actions = Дії
system-config-telegram-test-title = Перевірити з’єднання
system-config-telegram-test-description = Надішліть тестове повідомлення, щоб переконатися, що конфігурація { -telegram } працює
system-config-telegram-send-test = Надіслати тестове повідомлення
system-config-telegram-sending = Надсилання...
system-config-telegram-configure-token-title = Спершу налаштуйте токен бота
system-config-telegram-configure-token-status = Налаштуйте токен бота вище, щоб увімкнути тестування
system-config-telegram-test-sent-status = Тестове повідомлення успішно надіслано! Перевірте свій { -telegram }.
system-config-telegram-test-sent = Тестове повідомлення { -telegram } надіслано
system-config-telegram-test-failed = Не вдалося надіслати тестове повідомлення
system-config-telegram-auth-title = Автентифікація бота
system-config-telegram-totp-title = Двофакторна автентифікація (TOTP)
system-config-telegram-totp-configured = Налаштовано
system-config-telegram-totp-not-configured = Не налаштовано
system-config-telegram-totp-active = Двофакторну автентифікацію активовано. Для сесій { -telegram } із простроченим терміном потрібен код TOTP із вашого застосунку-автентифікатора.
system-config-telegram-totp-inactive = Увімкніть двофакторну автентифікацію в налаштуваннях безпеки, щоб захистити команди { -telegram }.
system-config-telegram-totp-note = TOTP спільний із екраном блокування панелі керування. Налаштуйте його в налаштуваннях безпеки.
system-config-telegram-require-2fa = Вимагати 2FA для команд
# $status is the HTTP status code.
system-config-telegram-save-rejected = Збереження відхилено ({ $status })
system-config-telegram-save-failed = Не вдалося зберегти налаштування { -telegram }

## Config page: operations

system-config-saved = Конфігурацію збережено
system-config-save-failed = Не вдалося зберегти конфігурацію
system-config-reloaded = Конфігурацію перезавантажено з диска
system-config-reload-failed = Не вдалося перезавантажити конфігурацію
system-config-diff-title = Відмінності конфігурації
system-config-diff-console = Виведено в консоль браузера
system-config-diff-failed = Не вдалося обчислити відмінності
system-config-reset-title = Скинути конфігурацію
system-config-reset-message =
    Це скине всю конфігурацію до вбудованих типових значень. Усі поточні налаштування буде втрачено.

    Цю дію не можна скасувати.
system-config-reset-done-title = Конфігурацію скинуто
system-config-reset-done-message = Усі налаштування відновлено до типових значень
system-config-reset-failed = Не вдалося скинути конфігурацію
system-config-load-failed = Не вдалося завантажити конфігурацію
system-config-metadata-failed = Не вдалося завантажити метадані конфігурації

## Import and export dialogs: shared

system-config-dialog-close =
    .aria-label = Закрити
system-config-select-none = Зняти вибір
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
        [one] { $count } зміна
        [few] { $count } зміни
        [many] { $count } змін
       *[other] { $count } зміни
    }
system-config-sections-count =
    { $count ->
        [one] { $count } розділ
        [few] { $count } розділи
        [many] { $count } розділів
       *[other] { $count } розділу
    }

## Import and export dialogs: section descriptions. Ids are the section names of
## src/webserver/routes/config/import_export.rs.

system-config-section-hint-chains = Увімкнення блокчейнів, RPC-ендпоінти та маршрутизація свопів
system-config-section-hint-trader = Торгові правила й автоматизація
system-config-section-hint-positions = Налаштування керування позиціями
system-config-section-hint-filtering = Правила та пороги фільтрації токенів
system-config-section-hint-tokens = Пошук токенів і джерела даних
system-config-section-hint-events = Налаштування запису подій
system-config-section-hint-services = Налаштування фонових сервісів
system-config-section-hint-monitoring = Конфігурація моніторингу системи
system-config-section-hint-ohlcv = Налаштування свічкових даних
system-config-section-hint-gui = Налаштування панелі керування та інтерфейсу
system-config-section-hint-telegram = Конфігурація бота { -telegram }

## Export dialog

system-config-export-dialog-title = Експорт конфігурації
system-config-export-intro = Виберіть, які розділи конфігурації експортувати. Експортований файл можна пізніше імпортувати, щоб відновити або поділитися налаштуваннями.
system-config-export-sections = Розділи
system-config-export-timestamp = Додати позначку часу експорту
system-config-sections-selected =
    { $count ->
        [one] Вибрано { $count } розділ
        [few] Вибрано { $count } розділи
        [many] Вибрано { $count } розділів
       *[other] Вибрано { $count } розділу
    }
system-config-exporting = Експорт...
system-config-export-invalid-response = Недійсна відповідь сервера
system-config-exported-title = Конфігурацію експортовано
system-config-exported-message =
    { $count ->
        [one] Експортовано { $count } розділ
        [few] Експортовано { $count } розділи
        [many] Експортовано { $count } розділів
       *[other] Експортовано { $count } розділу
    }
system-config-export-failed-title = Помилка експорту
system-config-export-failed = Не вдалося експортувати конфігурацію

## Import dialog

system-config-import-dialog-title = Імпорт конфігурації
system-config-import-upload-intro = Завантажте раніше експортований файл конфігурації. Ви зможете переглянути його та вибрати, які розділи імпортувати.
system-config-import-dropzone-title = Перетягніть файл конфігурації сюди
system-config-import-dropzone-hint = або натисніть, щоб вибрати
system-config-import-analyzing = Аналіз конфігурації...
system-config-import-preview = Попередній перегляд
system-config-import-preview-intro = Перегляньте розділи конфігурації нижче. Виберіть, які розділи імпортувати.
system-config-import-sections = Розділи у файлі
system-config-import-select-valid = Вибрати всі дійсні
system-config-import-merge-label = Об’єднати з наявними
system-config-import-merge-hint = Оновлювати лише поля, що є у файлі. Якщо не позначено — розділи замінюються повністю.
system-config-import-save-label = Зберегти на диск
system-config-import-save-hint = Записати зміни до config.toml після імпорту
system-config-import-selected = Імпортувати вибране
system-config-import-warnings =
    { $count ->
        [one] { $count } попередження
        [few] { $count } попередження
        [many] { $count } попереджень
       *[other] { $count } попередження
    }
# $section is a section name from the file, $field a dotted setting path, $detail the
# technical reason a section failed to parse.
system-config-import-warning-unknown-section = Невідомий розділ «{ $section }» буде проігноровано
system-config-import-warning-sensitive-field = Імпорт { $field } може перезаписати налаштування автентифікації
system-config-import-section-error = { $detail }
# $sections and $changes are the counts above, already worded.
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = Немає у файлі
system-config-import-status-invalid = Недійсна конфігурація
system-config-import-status-unchanged = Без змін
system-config-import-not-included = Не включено до файлу
system-config-import-show-changes = Показати зміни
system-config-import-hide-changes = Сховати зміни
system-config-import-value-current = Поточне значення
system-config-import-value-new = Нове значення
system-config-import-more-changes =
    { $count ->
        [one] ще +{ $count } зміна
        [few] ще +{ $count } зміни
        [many] ще +{ $count } змін
       *[other] ще +{ $count } зміни
    }
system-config-import-value-items =
    { "[" }{ $count ->
        [one] { $count } елемент
        [few] { $count } елементи
        [many] { $count } елементів
       *[other] { $count } елемента
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
        [one] { $count } ключ
        [few] { $count } ключі
        [many] { $count } ключів
       *[other] { $count } ключа
    }{ "}" }
system-config-importing = Імпорт...
system-config-import-failed = Помилка імпорту
system-config-import-invalid-file-title = Недійсний файл
system-config-import-invalid-file = Не вдалося розібрати файл конфігурації
system-config-imported-title = Конфігурацію імпортовано
system-config-imported-message =
    { $count ->
        [one] Імпортовано { $count } розділ
        [few] Імпортовано { $count } розділи
        [many] Імпортовано { $count } розділів
       *[other] Імпортовано { $count } розділу
    }
system-config-import-failed-title = Помилка імпорту
system-config-import-failed-message = Не вдалося імпортувати конфігурацію
