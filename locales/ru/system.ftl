system-result-config-differs = Конфигурация в памяти отличается от версии на диске
system-result-config-matches = Конфигурация в памяти совпадает с версией на диске

system-result-config-imported =
    Успешно импортировано: { $count ->
        [one] { $count } раздел
        [few] { $count } раздела
        [many] { $count } разделов
       *[other] { $count } раздела
    }
system-result-config-imported-with-warnings =
    Импортировано: { $count ->
        [one] { $count } раздел
        [few] { $count } раздела
        [many] { $count } разделов
       *[other] { $count } раздела
    }, с { $warnings ->
        [one] { $warnings } предупреждением
        [few] { $warnings } предупреждениями
        [many] { $warnings } предупреждениями
       *[other] { $warnings } предупреждениями
    }: { $details }

system-config-search =
    .placeholder = Поиск по настройкам...
system-config-export-title =
    .title = Экспортировать конфигурацию в файл
system-config-import-title =
    .title = Импортировать конфигурацию из файла
system-config-reload = Перезагрузить с диска
system-config-reset-defaults = Сбросить к значениям по умолчанию
system-config-select-section = Выберите раздел конфигурации
system-config-select-section-details = Выберите раздел конфигурации, чтобы посмотреть подробности.
system-config-no-metadata = Нет метаданных для <code>{ $section }</code>
system-config-technical-settings = Технические настройки
system-config-expand-title = Развернуть все разделы и все вложенные подконфигурации
system-config-collapse-title = Свернуть все разделы и все вложенные подконфигурации
system-config-toolbar-no-changes = Нет изменений в разделе
system-config-toolbar-section-changes =
    { $count ->
        [one] <strong>{ $count }</strong> изменение в разделе
        [few] <strong>{ $count }</strong> изменения в разделе
        [many] <strong>{ $count }</strong> изменений в разделе
       *[other] <strong>{ $count }</strong> изменения в разделе
    }
system-config-toolbar-total-changes =
    { $count ->
        [one] <strong>{ $count }</strong> изменение всего
        [few] <strong>{ $count }</strong> изменения всего
        [many] <strong>{ $count }</strong> изменений всего
       *[other] <strong>{ $count }</strong> изменения всего
    }

system-config-loading = Загрузка конфигурации…
system-config-refreshing = Обновление конфигурации…
system-config-saving-title = Сохранение изменений…
system-config-saving-detail = Обновление конфигурации
system-config-validation-issues = <strong>Обнаружены ошибки проверки.</strong> Проверьте выделенные поля.

system-config-save-changes = Сохранить изменения
system-config-saving = Сохранение…
system-config-compare = Сравнить с диском
system-config-revert-section = Отменить изменения раздела
system-config-summary-critical = Критичных: { $count }
system-config-summary-performance = Влияют на производительность: { $count }
system-config-summary-pending =
    { $count ->
        [one] { $count } ожидающее изменение
        [few] { $count } ожидающих изменения
        [many] { $count } ожидающих изменений
       *[other] { $count } ожидающего изменения
    }
system-config-summary-none = Нет сводки метаданных
system-config-fields-count =
    { $count ->
        [one] { $count } поле
        [few] { $count } поля
        [many] { $count } полей
       *[other] { $count } поля
    }
system-config-chip-pending = { $fields } · ожидают: { $pending }
system-config-chip-visible = { $visible } из { $fields }

system-config-field-unit = Единица: { $unit }
system-config-field-default = По умолчанию: { $value }
system-config-field-reset = Сбросить к значению по умолчанию
system-config-array-invalid-title = Недопустимый элемент массива
system-config-json-invalid-title = Недопустимый JSON
system-config-list-separator = { ", " }
system-config-array-invalid-integer =
    { $count ->
        [one] Строка { $lines } должна быть допустимым целым числом.
        [few] Строки { $lines } должны быть допустимыми целыми числами.
        [many] Строки { $lines } должны быть допустимыми целыми числами.
       *[other] Строки { $lines } должны быть допустимыми целыми числами.
    }
system-config-array-invalid-number =
    { $count ->
        [one] Строка { $lines } должна быть допустимым числом.
        [few] Строки { $lines } должны быть допустимыми числами.
        [many] Строки { $lines } должны быть допустимыми числами.
       *[other] Строки { $lines } должны быть допустимыми числами.
    }
system-config-array-invalid-boolean =
    { $count ->
        [one] Строка { $lines } должна быть допустимым логическим значением.
        [few] Строки { $lines } должны быть допустимыми логическими значениями.
        [many] Строки { $lines } должны быть допустимыми логическими значениями.
       *[other] Строки { $lines } должны быть допустимыми логическими значениями.
    }
system-config-array-invalid-value =
    { $count ->
        [one] Строка { $lines } должна быть допустимым значением.
        [few] Строки { $lines } должны быть допустимыми значениями.
        [many] Строки { $lines } должны быть допустимыми значениями.
       *[other] Строки { $lines } должны быть допустимыми значениями.
    }

system-config-telegram-actions = Действия
system-config-telegram-test-title = Проверить соединение
system-config-telegram-test-description = Отправьте тестовое сообщение, чтобы убедиться, что конфигурация { -telegram } работает
system-config-telegram-send-test = Отправить тестовое сообщение
system-config-telegram-sending = Отправка...
system-config-telegram-configure-token-title = Сначала настройте токен бота
system-config-telegram-configure-token-status = Настройте токен бота выше, чтобы включить проверку
system-config-telegram-test-sent-status = Тестовое сообщение отправлено! Проверьте { -telegram }.
system-config-telegram-test-sent = Тестовое сообщение { -telegram } отправлено
system-config-telegram-test-failed = Не удалось отправить тестовое сообщение
system-config-telegram-auth-title = Аутентификация бота
system-config-telegram-totp-title = Двухфакторная аутентификация (TOTP)
system-config-telegram-totp-configured = Настроена
system-config-telegram-totp-not-configured = Не настроена
system-config-telegram-totp-active = Двухфакторная аутентификация активна. Для истёкших сессий { -telegram } нужен TOTP-код из приложения-аутентификатора.
system-config-telegram-totp-inactive = Включите двухфакторную аутентификацию в настройках безопасности, чтобы защитить команды { -telegram }.
system-config-telegram-totp-note = TOTP общий с экраном блокировки дашборда. Настройте его в настройках безопасности.
system-config-telegram-require-2fa = Требовать 2FA для команд
system-config-telegram-save-rejected = Сохранение отклонено ({ $status })
system-config-telegram-save-failed = Не удалось сохранить настройку { -telegram }

system-config-saved = Конфигурация сохранена
system-config-save-failed = Не удалось сохранить конфигурацию
system-config-reloaded = Конфигурация перезагружена с диска
system-config-reload-failed = Не удалось перезагрузить конфигурацию
system-config-diff-title = Различия конфигурации
system-config-diff-console = Выведено в консоль браузера
system-config-diff-failed = Не удалось вычислить различия
system-config-reset-title = Сброс конфигурации
system-config-reset-message =
    Вся конфигурация будет сброшена к встроенным значениям по умолчанию. Все текущие настройки будут потеряны.

    Это действие нельзя отменить.
system-config-reset-done-title = Конфигурация сброшена
system-config-reset-done-message = Для всех настроек восстановлены значения по умолчанию
system-config-reset-failed = Не удалось сбросить конфигурацию
system-config-load-failed = Не удалось загрузить конфигурацию
system-config-metadata-failed = Не удалось загрузить метаданные конфигурации

system-config-dialog-close =
    .aria-label = Закрыть
system-config-select-none = Снять выбор
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
        [one] { $count } изменение
        [few] { $count } изменения
        [many] { $count } изменений
       *[other] { $count } изменения
    }
system-config-sections-count =
    { $count ->
        [one] { $count } раздел
        [few] { $count } раздела
        [many] { $count } разделов
       *[other] { $count } раздела
    }

system-config-section-hint-chains = Включение блокчейнов, RPC-эндпоинты и маршрутизация свопов
system-config-section-hint-trader = Правила торговли и автоматизация
system-config-section-hint-positions = Настройки управления позициями
system-config-section-hint-filtering = Правила и пороги фильтрации токенов
system-config-section-hint-tokens = Обнаружение токенов и источники данных
system-config-section-hint-events = Настройки записи событий
system-config-section-hint-services = Настройки фоновых сервисов
system-config-section-hint-monitoring = Конфигурация мониторинга системы
system-config-section-hint-ohlcv = Настройки данных свечей
system-config-section-hint-gui = Настройки дашборда и интерфейса
system-config-section-hint-telegram = Конфигурация бота { -telegram }

system-config-export-dialog-title = Экспорт конфигурации
system-config-export-intro = Выберите разделы конфигурации для экспорта. Экспортированный файл можно позже импортировать, чтобы восстановить настройки или поделиться ими.
system-config-export-sections = Разделы
system-config-export-timestamp = Добавить метку времени экспорта
system-config-sections-selected =
    { $count ->
        [one] Выбран { $count } раздел
        [few] Выбрано { $count } раздела
        [many] Выбрано { $count } разделов
       *[other] Выбрано { $count } раздела
    }
system-config-exporting = Экспорт...
system-config-export-invalid-response = Недопустимый ответ сервера
system-config-exported-title = Конфигурация экспортирована
system-config-exported-message =
    { $count ->
        [one] Экспортирован { $count } раздел
        [few] Экспортировано { $count } раздела
        [many] Экспортировано { $count } разделов
       *[other] Экспортировано { $count } раздела
    }
system-config-export-failed-title = Ошибка экспорта
system-config-export-failed = Не удалось экспортировать конфигурацию

system-config-import-dialog-title = Импорт конфигурации
system-config-import-upload-intro = Загрузите ранее экспортированный файл конфигурации. Вы сможете просмотреть его и выбрать, какие разделы импортировать.
system-config-import-dropzone-title = Перетащите файл конфигурации сюда
system-config-import-dropzone-hint = или нажмите, чтобы выбрать
system-config-import-analyzing = Анализ конфигурации...
system-config-import-preview = Предпросмотр
system-config-import-preview-intro = Просмотрите разделы конфигурации ниже. Выберите, какие из них импортировать.
system-config-import-sections = Разделы в файле
system-config-import-select-valid = Выбрать все корректные
system-config-import-merge-label = Объединить с текущими
system-config-import-merge-hint = Обновляются только поля, которые есть в файле. Если не отмечено — разделы заменяются целиком.
system-config-import-save-label = Сохранить на диск
system-config-import-save-hint = Записать изменения в config.toml после импорта
system-config-import-selected = Импортировать выбранное
system-config-import-warnings =
    { $count ->
        [one] { $count } предупреждение
        [few] { $count } предупреждения
        [many] { $count } предупреждений
       *[other] { $count } предупреждения
    }
system-config-import-warning-unknown-section = Неизвестный раздел «{ $section }» будет пропущен
system-config-import-warning-sensitive-field = Импорт { $field } может перезаписать настройки аутентификации
system-config-import-section-error = { $detail }
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = Нет в файле
system-config-import-status-invalid = Недопустимая конфигурация
system-config-import-status-unchanged = Без изменений
system-config-import-not-included = Не включено в файл
system-config-import-show-changes = Показать изменения
system-config-import-hide-changes = Скрыть изменения
system-config-import-value-current = Текущее значение
system-config-import-value-new = Новое значение
system-config-import-more-changes =
    { $count ->
        [one] ещё +{ $count } изменение
        [few] ещё +{ $count } изменения
        [many] ещё +{ $count } изменений
       *[other] ещё +{ $count } изменения
    }
system-config-import-value-items =
    { "[" }{ $count ->
        [one] { $count } элемент
        [few] { $count } элемента
        [many] { $count } элементов
       *[other] { $count } элемента
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
        [one] { $count } ключ
        [few] { $count } ключа
        [many] { $count } ключей
       *[other] { $count } ключа
    }{ "}" }
system-config-importing = Импорт...
system-config-import-failed = Не удалось импортировать
system-config-import-invalid-file-title = Недопустимый файл
system-config-import-invalid-file = Не удалось разобрать файл конфигурации
system-config-imported-title = Конфигурация импортирована
system-config-imported-message =
    { $count ->
        [one] Импортирован { $count } раздел
        [few] Импортировано { $count } раздела
        [many] Импортировано { $count } разделов
       *[other] Импортировано { $count } раздела
    }
system-config-import-failed-title = Ошибка импорта
system-config-import-failed-message = Не удалось импортировать конфигурацию
