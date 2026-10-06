wallets-type-generated = Созданный
wallets-type-imported = Импортированный
wallets-type-migrated = Перенесённый

wallets-watch-disabled-user = Приостановлено вами
wallets-watch-disabled-signature-budget = Пауза: достигнут лимит проверки (подписей: { $limit }) до того, как отслеживание нагнало историю
wallets-watch-disabled-unknown = Пауза: не удалось прочитать сохранённую причину защиты отслеживания
wallets-watch-disabled-helius-unavailable = Пауза: провайдер для кошельков с высокой активностью недоступен; курсор сохранён
wallets-watch-disabled-processing-failed = Пауза: не удалось обработать активность кошелька; курсор сохранён

wallets-watch-error-provider-unavailable = Провайдер для кошельков с высокой активностью недоступен; отслеживание приостановлено
wallets-watch-error-provider-repeated-failure = Проверки через { -helius } неоднократно завершались ошибкой; отслеживание приостановлено
wallets-watch-error-processing-repeated-failure = Обработка активности кошелька неоднократно завершалась ошибкой; отслеживание приостановлено
wallets-watch-error-position-unreadable = Отслеживанию кошелька не удалось прочитать сохранённую позицию; повторная попытка
wallets-watch-error-provider-check-failed = Проверка через провайдера для кошельков с высокой активностью не удалась; повторная попытка
wallets-watch-error-decode-failed = Не удалось декодировать транзакцию высокой активности; курсор сохранён
wallets-watch-error-processing-failed = Не удалось обработать активность кошелька; повторная попытка
wallets-watch-error-position-save-failed = Отслеживанию кошелька не удалось сохранить позицию; повторная попытка

wallets-watch-reason-user = Приостановлено вами.
wallets-watch-reason-signature-budget = У этого кошелька больше активности, чем текущее отслеживание успевает проверять.
wallets-watch-reason-helius-unavailable = Проверки через { -helius } не удались. Сохранённый прогресс не потерян.
wallets-watch-reason-processing-failed = Не удалось обработать активность кошелька. Сохранённый прогресс не потерян.

wallets-field-address = Адрес
wallets-field-name = Название кошелька
wallets-field-notes = Заметки
wallets-field-private-key = Приватный ключ
wallets-address-copy = Скопировать адрес
wallets-modal-close =
    .aria-label = Закрыть окно
wallets-this-wallet = этот кошелёк
wallets-summary-native = { -sol }
wallets-copied-address = Адрес
wallets-copied-mint = Адрес минта
wallets-copied-private-key = Приватный ключ

wallets-tab-main = Основной кошелёк
wallets-tab-secondaries = Дополнительные
wallets-tab-archive = Архив
wallets-tab-watched = Отслеживаемые
wallets-refresh-failed = Не удалось обновить кошельки
wallets-action-failed = Ошибка
wallets-toast-failed = Ошибка: { $reason }
wallets-create-busy = Создание...
wallets-create-fallback = Не удалось создать
wallets-create-done = Кошелёк «{ $name }» создан!
wallets-import-busy = Импорт...
wallets-import-failed = Не удалось импортировать
wallets-import-done = Кошелёк «{ $name }» импортирован!
wallets-archive-busy = Архивация...
wallets-archive-confirm-text = Вы уверены, что хотите архивировать <strong>{ $name }</strong>?
wallets-archive-done = Кошелёк перенесён в архив
wallets-restore-done = Кошелёк восстановлен
wallets-export-busy = Расшифровка...
wallets-export-revealed = Ключ показан — обращайтесь с ним осторожно
wallets-delete-busy = Удаление...
wallets-delete-confirm-text = Вы уверены, что хотите удалить <strong>{ $name }</strong>?
wallets-delete-done = Кошелёк удалён навсегда

wallets-add-title = Добавить кошелёк
wallets-add-tab-create = Создать новый
wallets-add-tab-import = Импортировать
wallets-create-name-input =
    .placeholder = Например, Торговый кошелёк
wallets-create-name-hint = Понятное имя, чтобы отличать этот кошелёк
wallets-create-notes-input =
    .placeholder = Необязательное описание или назначение...
wallets-create-submit = Создать кошелёк
wallets-import-warning-title = Предупреждение о безопасности
wallets-import-warning-body = Импортируйте приватные ключи только из надёжных источников. Ключ будет зашифрован и надёжно сохранён на этом устройстве.
wallets-import-name-input =
    .placeholder = Например, Мой кошелёк
wallets-import-key-input =
    .placeholder = Строка Base58 или массив JSON [1,2,3,...]
wallets-import-key-toggle =
    .aria-label = Показать или скрыть приватный ключ
wallets-import-key-hint = Поддерживается ключ в кодировке base58 или в виде массива байтов
wallets-import-notes-input =
    .placeholder = Необязательное описание...
wallets-import-submit = Импортировать кошелёк

wallets-watch-add-title = Отслеживать кошелёк
wallets-watch-add-address = Адрес кошелька
wallets-watch-add-address-input =
    .placeholder = Адрес Solana
wallets-watch-add-address-hint = Записывает ончейн-активность кошелька и отправляет уведомления о сделках через ваши настройки { -telegram }.
wallets-watch-add-label = Метка
wallets-watch-add-label-input =
    .placeholder = Необязательное имя
wallets-watch-add-submit = Добавить отслеживание

wallets-watch-budget-title-options = Параметры отслеживания кошелька
wallets-watch-budget-title-restore = Восстановить отслеживание кошелька
wallets-watch-budget-close =
    .aria-label = Закрыть
wallets-watch-budget-label-signatures = Подписей за одну проверку
wallets-watch-budget-label-transactions = Успешных полных транзакций за одну проверку
wallets-watch-budget-hint-signatures = Текущий лимит: { $limit }. Выберите от 500 до 5 000 подписей за проверку с шагом 100.
wallets-watch-budget-hint-transactions = Текущий лимит: { $limit }. Выберите от 500 до 5 000 успешных транзакций за проверку с шагом 100.
wallets-watch-budget-error-range = Выберите от 500 до 5 000 записей за проверку с шагом 100.
wallets-watch-budget-error-ack = Подтвердите, что подписи с момента последней завершённой проверки будут пропущены.
wallets-watch-budget-save-failed = Не удалось сохранить лимит отслеживания.
wallets-watch-budget-save = Сохранить лимит
wallets-watch-budget-resume = Продолжить с текущего момента
wallets-watch-budget-resume-notice = Этот кошелёк достиг лимита проверки, не успев нагнать историю. Продолжение с текущего момента начнёт с последней активности кошелька; активность с момента последней завершённой проверки скопирована не будет.
wallets-watch-budget-resume-tasks = Задачи копирования остаются на паузе, пока вы не возобновите каждую из них в разделе «Копитрейдинг».
wallets-watch-budget-resume-ack = Я понимаю, что пропущенная активность не будет скопирована.
wallets-watch-budget-resumed = Отслеживание продолжено с текущего момента кошелька
wallets-watch-budget-updated = Лимит отслеживания кошелька обновлён
wallets-watch-helius-allow = Разрешить нагонять историю через { -helius } при необходимости
wallets-watch-helius-try = Попробовать нагнать историю через { -helius }
wallets-watch-helius-stop = Отключить нагон истории через { -helius } для этого кошелька
wallets-watch-helius-description-approved = Нагон истории через { -helius } разрешён для этого кошелька. Если отключить его, вернутся стандартные проверки, которые на активном кошельке могут отставать.
wallets-watch-helius-description-available = { -helius } может проверять успешные транзакции Solana с сохранённой позиции, не пропуская непроверенный интервал. Это может расходовать больше кредитов провайдера, и отставание всё равно возможно.
wallets-watch-helius-description-unavailable = Нагон истории через { -helius } недоступен. Чтобы его использовать, настройте включённый RPC-эндпоинт { -helius }.
wallets-watch-helius-description-unsupported = Для этого отслеживания нет поддерживаемого провайдера нагона истории. Если отслеживание достигнет лимита, можно продолжить с текущего момента.
wallets-watch-helius-allow-title = Разрешить нагон истории через { -helius } для этого кошелька
wallets-watch-helius-allow-message = { -helius } может проверять успешные транзакции Solana с сохранённой позиции, не пропуская непроверенный интервал. Сейчас списывается 10 кредитов за каждые 100 возвращённых полных транзакций с округлением вверх, минимум 10 кредитов за запрос. Одна проверка может делать несколько запросов; расход и тарифы провайдера могут отличаться. Задачи копирования остаются на паузе, пока вы не возобновите их отдельно.
wallets-watch-helius-allow-confirm = Разрешить для этого кошелька
wallets-watch-helius-stop-message = Этот кошелёк вернётся к стандартным проверкам. Активный кошелёк может снова достичь лимита отслеживания и встать на паузу. Другие кошельки и ваша RPC-конфигурация { -helius } не меняются.
wallets-watch-helius-stop-confirm = Отключить для этого кошелька
wallets-watch-helius-stop-keep = Оставить разрешённым
wallets-watch-helius-restored = Отслеживание восстановлено с сохранённого прогресса; задачи копирования остаются на паузе
wallets-watch-helius-allowed = Нагон истории через { -helius } разрешён для этого кошелька при необходимости
wallets-watch-helius-stopped = Нагон истории через { -helius } отключён для этого кошелька
wallets-watch-helius-update-failed = Не удалось обновить настройку нагона истории кошелька

wallets-export-title = Экспорт приватного ключа
wallets-export-warning-title = Критическое предупреждение о безопасности
wallets-export-warning-body = Никому не передавайте свой приватный ключ. Любой, у кого есть этот ключ, может похитить все средства с этого кошелька.
wallets-export-key-label = Приватный ключ (Base58)
wallets-export-copy =
    .title = Скопировать в буфер обмена
    .aria-label = Скопировать в буфер обмена
wallets-export-reveal = Показать ключ

wallets-archive-title = Архивировать кошелёк
wallets-archive-note = Архивные кошельки не участвуют ни в каких операциях, но их можно восстановить в любой момент.
wallets-archive-confirm = Да, архивировать
wallets-delete-title = Удалить кошелёк
wallets-delete-warning-title = Это действие нельзя отменить!
wallets-delete-warning-body = Кошелёк и его зашифрованный приватный ключ будут навсегда удалены с этого устройства.
wallets-delete-confirm = Да, удалить

wallets-bulk-import-title = Импорт кошельков
wallets-bulk-import-submit = Импортировать кошельки
wallets-bulk-step-upload = Загрузка файла
wallets-bulk-step-map = Сопоставление столбцов
wallets-bulk-step-results = Результаты
wallets-bulk-import-file-warning-body = Импортируйте файлы только из надёжных источников. Приватные ключи будут зашифрованы и надёжно сохранены на этом устройстве.
wallets-bulk-drop-title = Перетащите файл сюда
wallets-bulk-drop-subtitle = или нажмите, чтобы выбрать
wallets-bulk-drop-formats = Поддерживаются CSV и Excel (.xlsx, .xls)
wallets-bulk-file-remove =
    .aria-label = Удалить файл
wallets-bulk-map-subtitle = Сопоставьте столбцы файла с полями кошелька
wallets-bulk-preview-title = Предпросмотр (первые 5 строк)
wallets-bulk-summary-valid = Корректных: <strong>{ $count }</strong>
wallets-bulk-summary-invalid = Некорректных: <strong>{ $count }</strong>
wallets-bulk-summary-duplicate =
    { $count ->
        [one] <strong>{ $count }</strong> дубликат
        [few] <strong>{ $count }</strong> дубликата
        [many] <strong>{ $count }</strong> дубликатов
       *[other] <strong>{ $count }</strong> дубликата
    }
wallets-bulk-done = Готово
wallets-bulk-file-invalid = Недопустимый тип файла. Используйте файлы CSV или Excel.
wallets-bulk-preview-busy = Обработка...
wallets-bulk-preview-fallback = Не удалось обработать файл
wallets-bulk-preview-failed = Не удалось обработать файл: { $reason }
wallets-bulk-column-select = -- Выберите столбец --
wallets-bulk-preview-empty = В файле нет строк с данными
wallets-bulk-preview-status = Статус
wallets-bulk-status-valid = Корректный
wallets-bulk-status-duplicate = Дубликат
wallets-bulk-status-invalid = Некорректный
wallets-bulk-import-busy = Импорт...
wallets-bulk-import-toast =
    { $count ->
        [one] Импортирован { $count } кошелёк
        [few] Импортировано { $count } кошелька
        [many] Импортировано { $count } кошельков
       *[other] Импортировано { $count } кошелька
    }
wallets-bulk-import-error = Не удалось импортировать: { $reason }
wallets-bulk-result-success-title = Импорт выполнен
wallets-bulk-result-success-detail =
    { $count ->
        [one] Успешно импортирован { $count } кошелёк
        [few] Успешно импортированы все { $count } кошелька
        [many] Успешно импортированы все { $count } кошельков
       *[other] Успешно импортированы все { $count } кошелька
    }
wallets-bulk-result-partial-title = Частичный успех
wallets-bulk-result-partial-detail = Импортировано: { $imported }, с ошибкой: { $failed }
wallets-bulk-result-failed-title = Не удалось импортировать
wallets-bulk-result-failed-detail =
    { $count ->
        [one] Не удалось импортировать { $count } кошелёк
        [few] Не удалось импортировать все { $count } кошелька
        [many] Не удалось импортировать все { $count } кошельков
       *[other] Не удалось импортировать все { $count } кошелька
    }
wallets-bulk-result-imported = Импортировано
wallets-bulk-result-failed = Ошибка

wallets-bulk-export-title = Экспорт кошельков
wallets-bulk-export-format = Формат
wallets-bulk-export-format-csv = CSV (.csv)
wallets-bulk-export-format-xlsx = Excel (.xlsx)
wallets-bulk-export-include-archived = Включить архивные кошельки
wallets-bulk-export-safe-title = Безопасный экспорт
wallets-bulk-export-safe-body = Экспортируются только адреса кошельков и метаданные. Приватные ключи не включаются.
wallets-bulk-export-safe-submit = Экспортировать адреса
wallets-bulk-export-or = или
wallets-bulk-export-danger-title = Опасный экспорт
wallets-bulk-export-danger-body = Приватные ключи будут включены в экспорт. Любой, у кого есть этот файл, может похитить ваши средства.
wallets-bulk-export-danger-submit = Экспорт с приватными ключами
wallets-bulk-export-busy = Экспорт...
wallets-bulk-export-done = Кошельки экспортированы в { $filename }
wallets-bulk-export-fallback = Не удалось экспортировать
wallets-bulk-export-error = Не удалось экспортировать: { $reason }
wallets-bulk-confirm-title = Подтвердите опасный экспорт
wallets-bulk-confirm-warning =
    { $count ->
        [one] Вы собираетесь экспортировать <strong>{ $count }</strong> приватный ключ. Это крайне опасно!
        [few] Вы собираетесь экспортировать <strong>{ $count }</strong> приватных ключа. Это крайне опасно!
        [many] Вы собираетесь экспортировать <strong>{ $count }</strong> приватных ключей. Это крайне опасно!
       *[other] Вы собираетесь экспортировать <strong>{ $count }</strong> приватного ключа. Это крайне опасно!
    }
wallets-bulk-confirm-risk-steal = Любой, у кого есть этот файл, может похитить все средства
wallets-bulk-confirm-risk-share = Никому не передавайте этот файл
wallets-bulk-confirm-risk-delete = Сразу после использования удалите файл
wallets-bulk-confirm-prompt = Введите фразу ниже для подтверждения
wallets-bulk-confirm-submit = Экспортировать ключи

wallets-holdings-col-token = Токен
wallets-holdings-col-balance = Баланс
wallets-holdings-col-value = Стоимость ({ -sol })
wallets-holdings-col-type = Тип
wallets-holdings-col-decimals = Знаков после запятой
wallets-holdings-col-mint = Минт
wallets-holdings-empty-title = Нет токенов
wallets-holdings-empty-message = Здесь появятся токены, которые хранятся в этом кошельке.
wallets-holdings-no-main = Нет основного кошелька
wallets-holdings-main-tag = Основной
wallets-holdings-main-title = Основной кошелёк
wallets-holdings-tokens = Токены
wallets-holdings-last-used = Последнее использование
wallets-holdings-never = Никогда
wallets-holdings-search =
    .placeholder = Поиск по символу или минту...
wallets-holdings-export = Экспорт ключа
wallets-holdings-export-tooltip = Экспортировать приватный ключ этого кошелька
wallets-list-col-name = Название
wallets-list-col-balance = Баланс ({ -sol })
wallets-list-col-type = Тип
wallets-list-col-created = Создан
wallets-list-col-actions = Действия
wallets-list-action-export = Экспортировать приватный ключ
wallets-list-action-archive = Архивировать кошелёк
wallets-list-action-restore = Восстановить кошелёк
wallets-list-action-delete = Удалить навсегда
wallets-list-count = Кошельки
wallets-list-search =
    .placeholder = Поиск по названию или адресу...
wallets-list-loading-title = Загрузка кошельков…
wallets-list-loading-description = Подготовка выбранного списка кошельков.
wallets-secondaries-empty-title = Нет дополнительных кошельков
wallets-secondaries-empty-message = Создайте дополнительные кошельки, чтобы распределить торговую активность по нескольким аккаунтам.
wallets-secondaries-add = Добавить кошелёк
wallets-archive-empty-title = Нет архивных кошельков
wallets-archive-empty-message = Кошельки, которые вы архивируете, будут надёжно храниться здесь.

wallets-watched-col-wallet = Кошелёк
wallets-watched-col-status = Статус
wallets-watched-col-progress = Сохранённый прогресс
wallets-watched-col-last-check = Последняя проверка
wallets-watched-unlabelled = Кошелёк без метки
wallets-watched-generic-name = кошелёк
wallets-watched-not-synced = Ещё не синхронизирован
wallets-watched-not-checked = Ещё не проверялся
wallets-watched-action-copy = Копировать
    .title = Открыть этот кошелёк в разделе «Копитрейдинг»
wallets-watched-action-restore = Восстановить отслеживание
wallets-watched-action-options = Параметры отслеживания
wallets-watched-action-retry = Повторить отслеживание
wallets-watched-action-pause = Пауза
wallets-watched-action-enable = Включить
wallets-watched-action-remove =
    .title = Удалить
    .aria-label = Удалить { $name }
wallets-watch-state-paused = На паузе
wallets-watch-state-catching-up = Нагоняет
wallets-watch-state-watching = Отслеживается
wallets-watch-state-streaming = Потоковое
wallets-watch-state-polling = Опрос
wallets-watched-detail-helius = Проверка этого кошелька идёт через { -helius }.
wallets-watched-empty-title = Нет отслеживаемых адресов
wallets-watched-empty-message = Используйте «Отслеживать кошелёк», чтобы записывать ончейн-активность публичного кошелька.
wallets-watched-count = Отслеживаемые
wallets-watched-search =
    .placeholder = Поиск по отслеживаемым кошелькам...
wallets-watched-add = Отслеживать кошелёк
wallets-watched-refresh = Обновить отслеживаемые кошельки
wallets-watched-loading-title = Загрузка отслеживаемых кошельков...
wallets-watched-loading-description = Получение целей наблюдения.
wallets-watched-load-error-title = Не удалось загрузить отслеживаемые адреса
wallets-watched-load-error-description = Нажмите «Обновить», чтобы повторить.
wallets-watched-address-invalid = Введите корректный адрес кошелька Solana.
wallets-watched-added = Отслеживание кошелька добавлено
wallets-watched-duplicate = Этот кошелёк уже отслеживается.
wallets-watched-add-failed = Не удалось добавить отслеживание кошелька.
wallets-watched-retried = Отслеживание кошелька восстановлено с сохранённым курсором
wallets-watched-paused = Отслеживание кошелька приостановлено
wallets-watched-enabled = Отслеживание кошелька включено
wallets-watched-removed = Отслеживание кошелька удалено
wallets-watched-update-failed = Не удалось обновить отслеживание кошелька
