## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = Инструменты
tools-category-wallet = Кошелёк
tools-category-token = Токен
tools-category-single-token = Один токен
tools-category-utilities = Утилиты
tools-sidebar-hint = Выберите инструмент, чтобы начать
tools-help-button =
    .aria-label = Показать справку по этому инструменту
tools-help-unavailable = Справка недоступна
tools-placeholder-title = Выберите инструмент
tools-placeholder-subtitle = Выберите инструмент в боковой панели, чтобы начать
tools-placeholder-hint-wallets = Инструменты для кошельков помогают управлять вашими кошельками Solana
tools-placeholder-hint-secure = Все операции защищены и по возможности обратимы

tools-status-ready = Готов к использованию
tools-status-coming = Скоро
tools-status-beta = Бета — возможны ошибки
tools-status-disabled = Сейчас отключён
tools-status-badge-coming = Скоро
tools-status-badge-beta = Бета
tools-toast-coming-soon = Этот инструмент скоро появится
tools-toast-disabled = Этот инструмент сейчас отключён
tools-setup-gate-title = Для этого инструмента нужен кошелёк

## Tool names.

tools-tool-wallet-cleanup-title = Очистка кошелька
tools-tool-wallet-cleanup-summary = Закрыть пустые ATA
tools-tool-wallet-cleanup-description = Закройте пустые ассоциированные токен-аккаунты, чтобы вернуть { -sol }
tools-tool-burn-tokens-title = Сжигание токенов
tools-tool-burn-tokens-summary = Безвозвратно уничтожить токены
tools-tool-burn-tokens-description = Безвозвратно уничтожьте токены из вашего кошелька
tools-tool-token-analyzer-title = Анализатор токенов
tools-tool-token-analyzer-summary = Глубокий анализ токена
tools-tool-token-analyzer-description = Глубокий многомерный анализ любого токена Solana
tools-tool-create-token-title = Создание токена
tools-tool-create-token-summary = Выпустить новый токен SPL
tools-tool-create-token-description = Выпустите новый токен SPL в Solana
tools-tool-trade-watcher-title = Наблюдатель за сделками
tools-tool-trade-watcher-summary = Мониторинг сделок и автодействия
tools-tool-trade-watcher-description = Отслеживайте сделки с токеном и запускайте автоматические покупки и продажи
tools-tool-token-watch-title = Наблюдение за холдерами
tools-tool-token-watch-summary = Отслеживание новых холдеров токена
tools-tool-token-watch-description = Отслеживайте новых холдеров токена в реальном времени
tools-tool-buy-multi-wallets-title = Мультипокупка
tools-tool-buy-multi-wallets-summary = Согласованные покупки с нескольких кошельков
tools-tool-buy-multi-wallets-description = Выполняйте согласованные ордера на покупку с нескольких кошельков со случайными суммами
tools-tool-sell-multi-wallets-title = Мультипродажа
tools-tool-sell-multi-wallets-summary = Согласованные продажи с нескольких кошельков
tools-tool-sell-multi-wallets-description = Выполняйте согласованные ордера на продажу с нескольких кошельков с консолидацией { -sol }
tools-tool-wallet-consolidation-title = Консолидация кошельков
tools-tool-wallet-consolidation-nav-title = Консолидация
tools-tool-wallet-consolidation-summary = Консолидировать средства кошельков
tools-tool-wallet-consolidation-description = Консолидируйте { -sol } и токены с дополнительных кошельков на основной кошелёк
tools-tool-airdrop-checker-title = Проверка airdrop
tools-tool-airdrop-checker-summary = Проверить ожидающие airdrop
tools-tool-airdrop-checker-description = Проверьте ожидающие airdrop и доступные к получению награды
tools-tool-wallet-generator-title = Генератор кошельков
tools-tool-wallet-generator-summary = Создать новые пары ключей
tools-tool-wallet-generator-description = Безопасно создавайте новые пары ключей Solana

## Shared by the tools

tools-validation-mint-required = Введите адрес минта токена
tools-validation-mint-format = Недопустимый формат адреса минта токена
tools-validation-mint-invalid = Введите корректный адрес минта

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = Сведения о токене
tools-create-token-name-label = Название токена
tools-create-token-name-input =
    .placeholder = Мой токен
tools-create-token-symbol-label = Символ
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = Знаков после запятой
tools-create-token-supply-label = Начальное предложение
tools-create-token-description-label = Описание
tools-create-token-description-input =
    .placeholder = Описание токена...
tools-create-token-image-title = Изображение токена
tools-create-token-image-drop = Перетащите изображение сюда или нажмите для загрузки
tools-create-token-image-hint = Рекомендуется: PNG 512x512
tools-create-token-action-preview = Предпросмотр
tools-create-token-action-create = Создать токен

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = Загрузка настроек...
tools-holder-watch-saved = Настройки наблюдения за холдерами сохранены
tools-holder-watch-save-failed = Не удалось сохранить настройки
tools-holder-watch-save-error = Ошибка при сохранении настроек
tools-holder-watch-settings-title = Настройки наблюдения за холдерами
tools-holder-watch-enabled-label = Включить наблюдение за холдерами
tools-holder-watch-interval-label = Интервал проверки
tools-holder-watch-interval-hint = Как часто проверять число холдеров (10–3600 с)
tools-holder-watch-max-tokens-label = Макс. отслеживаемых токенов
tools-holder-watch-max-tokens-hint = Максимальное число токенов, отслеживаемых одновременно
tools-holder-watch-notify-new-label = Уведомлять о новых холдерах
tools-holder-watch-notify-drop-label = Уведомлять об уходе холдеров
tools-holder-watch-min-change-label = Мин. изменение числа холдеров
tools-holder-watch-min-change-hint = Минимальное изменение числа холдеров для отправки уведомления
tools-holder-watch-drop-percent-label = Порог ухода холдеров
tools-holder-watch-drop-percent-hint = Процент снижения, при котором срабатывает оповещение
tools-holder-watch-action-save = Сохранить настройки
tools-holder-watch-tokens-title = Отслеживаемые токены
tools-holder-watch-token-input =
    .placeholder = Введите адрес минта токена...
tools-holder-watch-empty = Нет отслеживаемых токенов
tools-holder-watch-empty-hint = Добавьте адрес минта токена выше, чтобы начать наблюдение
tools-holder-watch-coming-soon = Наблюдение за токенами скоро появится

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = Анализ токена
tools-analyzer-mint-input =
    .placeholder = Вставьте адрес минта токена...
tools-analyzer-action-analyze = Анализировать
tools-analyzer-action-analyzing = Анализ...
tools-analyzer-action-copy-report = Копировать отчёт
tools-analyzer-loading = Анализируем токен...
tools-analyzer-failed = Не удалось проанализировать токен
tools-analyzer-empty = Введите адрес минта токена для анализа
tools-analyzer-empty-hint = Получите подробную аналитику по любому токену Solana
tools-analyzer-tab-overview = Обзор
tools-analyzer-tab-security = Безопасность
tools-analyzer-tab-market = Рынок
tools-analyzer-tab-liquidity = Ликвидность
tools-analyzer-unknown-token = Неизвестный токен

tools-analyzer-favorite-add =
    .title = Добавить в избранное
    .aria-label = Добавить в избранное
tools-analyzer-favorite-already = Уже в избранном
tools-analyzer-favorite-added = { $symbol } добавлен в избранное
tools-analyzer-favorite-failed = Не удалось добавить в избранное
tools-analyzer-blacklist-add =
    .title = Добавить в чёрный список
    .aria-label = Добавить в чёрный список
tools-analyzer-blacklist-title = Добавить токен в чёрный список
tools-analyzer-blacklist-message = Добавить { $symbol } в чёрный список? Этот токен будет исключён из торговли.
tools-analyzer-blacklist-confirm = В чёрный список
tools-analyzer-blacklisted = В чёрном списке
tools-analyzer-blacklist-done = { $symbol } добавлен в чёрный список
tools-analyzer-blacklist-failed = Не удалось добавить токен в чёрный список

tools-analyzer-card-quick-stats = Краткая статистика
tools-analyzer-card-market-summary = Рыночная сводка
tools-analyzer-card-token-info = Информация о токене
tools-analyzer-stat-holders = Холдеры
tools-analyzer-stat-decimals = Знаков после запятой
tools-analyzer-stat-safety-score = Оценка безопасности
tools-analyzer-stat-pools = Пулы
tools-analyzer-stat-volume-24h = Объём за 24 ч
tools-analyzer-stat-change-24h = Изменение за 24 ч
tools-analyzer-stat-market-cap = Капитализация
tools-analyzer-stat-liquidity = Ликвидность
tools-analyzer-info-mint = Адрес минта
tools-analyzer-info-description = Описание
tools-analyzer-info-supply = Предложение

tools-analyzer-security-empty = Нет данных о безопасности
tools-analyzer-security-empty-hint = Анализ безопасности для этого токена недоступен
tools-analyzer-card-safety-score = Оценка безопасности
tools-analyzer-score-good = Хорошая
tools-analyzer-score-moderate = Умеренная
tools-analyzer-score-risky = Рискованная
tools-analyzer-raw-score = Исходная оценка риска: { $score }
tools-analyzer-card-authorities = Полномочия токена
tools-analyzer-authority-mint = Mint-полномочия
tools-analyzer-authority-freeze = Freeze-полномочия
tools-analyzer-authority-transfer-fee = Комиссия за перевод
tools-analyzer-authority-mutable = Изменяемый
tools-analyzer-authority-active = Активны
tools-analyzer-authority-revoked = Отозваны
tools-analyzer-card-holder-concentration = Концентрация холдеров
tools-analyzer-top-holders = у 10 крупнейших холдеров
tools-analyzer-risks-title = Риски безопасности ({ $count })
tools-analyzer-risks-title-none = Риски безопасности
tools-analyzer-risks-none = Рисков безопасности не обнаружено

tools-analyzer-market-empty = Нет рыночных данных
tools-analyzer-market-empty-hint = Рыночные данные для этого токена недоступны
tools-analyzer-card-price = Текущая цена
tools-analyzer-card-price-changes = Изменения цены
tools-analyzer-card-volume = Объём торгов
tools-analyzer-card-transactions = Транзакции за 24 ч
tools-analyzer-card-valuation = Оценка стоимости
tools-analyzer-stat-window-1h = 1 ч
tools-analyzer-stat-window-6h = 6 ч
tools-analyzer-stat-window-24h = 24 ч
tools-analyzer-stat-volume-1h = Объём за 1 ч
tools-analyzer-stat-volume-6h = Объём за 6 ч
tools-analyzer-stat-fdv = Полностью разводнённая стоимость
tools-analyzer-txn-buys = Покупки
tools-analyzer-txn-sells = Продажи

tools-analyzer-liquidity-empty = Нет данных о ликвидности
tools-analyzer-liquidity-empty-hint = Для этого токена не найдено пулов
tools-analyzer-card-total-liquidity = Общая ликвидность
tools-analyzer-card-pools = Пулы
tools-analyzer-active-pools =
    { $count ->
        [one] Активный пул
        [few] Активных пула
        [many] Активных пулов
       *[other] Активного пула
    }
tools-analyzer-card-pool-details = Сведения о пулах
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = Ликвидность ({ -sol })
tools-analyzer-pools-column-status = Статус
tools-analyzer-pool-primary = Основной

tools-analyzer-report-empty = Нет анализа для копирования
tools-analyzer-report-label = Отчёт об анализе
tools-analyzer-report-title = Отчёт об анализе токена
tools-analyzer-report-token = Токен: { $symbol } ({ $name })
tools-analyzer-report-mint = Минт: { $mint }
tools-analyzer-report-price = Цена: { $sol }
tools-analyzer-report-price-with-usd = Цена: { $sol } ({ $usd })
tools-analyzer-report-security = Безопасность:
tools-analyzer-report-safety-score = - Оценка безопасности: { $score }/100
tools-analyzer-report-mint-authority = - Mint-полномочия: { $state }
tools-analyzer-report-freeze-authority = - Freeze-полномочия: { $state }
tools-analyzer-report-risks = - Риски: { $count }
tools-analyzer-report-market = Рынок:
tools-analyzer-report-volume = - Объём за 24 ч: { $amount }
tools-analyzer-report-change = - Изменение за 24 ч: { $amount }
tools-analyzer-report-market-cap = - Капитализация: { $amount }
tools-analyzer-report-liquidity = Ликвидность:
tools-analyzer-report-liquidity-total = - Всего: { $amount }
tools-analyzer-report-pools = - Пулы: { $count }
tools-analyzer-report-generated = Создано: { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

tools-watch-type-buy-on-sell = Покупка при продаже
tools-watch-type-sell-on-buy = Продажа при покупке
tools-watch-type-notify = Уведомление
tools-watch-type-notify-only = Только уведомление

tools-trade-watcher-setup-title = Настройка наблюдения
tools-trade-watcher-mint-label = Адрес минта токена
tools-trade-watcher-mint-input =
    .placeholder = Введите адрес минта токена...
tools-trade-watcher-action-search-pools = Найти пулы
tools-trade-watcher-pool-label = Выбранный пул
tools-trade-watcher-pool-none = Пул не выбран
tools-trade-watcher-pool-clear =
    .title = Сбросить пул
tools-trade-watcher-pool-selected = Выбранный пул: { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = Тип наблюдения
tools-trade-watcher-type-hint = Покупка при продаже: автоматически покупать, когда кто-то продаёт. Продажа при покупке: автоматически продавать, когда кто-то покупает.
tools-trade-watcher-trigger-label = Сумма срабатывания
tools-trade-watcher-trigger-hint = Минимальный размер сделки в { -sol } для запуска действия
tools-trade-watcher-action-amount-label = Сумма действия
tools-trade-watcher-action-amount-hint = Сумма покупки/продажи при срабатывании
tools-trade-watcher-slippage-label = Проскальзывание
tools-trade-watcher-slippage-hint = Максимально допустимое проскальзывание для сделок
tools-trade-watcher-active-title = Активные наблюдения
tools-trade-watcher-empty = Нет активных наблюдений
tools-trade-watcher-empty-hint = Настройте наблюдение выше и нажмите «Запустить наблюдение», чтобы начать мониторинг
tools-trade-watcher-action-start = Запустить наблюдение
tools-trade-watcher-action-starting = Запуск...
tools-trade-watcher-action-stop-all = Остановить все
tools-trade-watcher-action-stopping = Остановка...
tools-trade-watcher-started = Наблюдение за { $token } запущено...
tools-trade-watcher-start-failed = Не удалось запустить наблюдение
tools-trade-watcher-stopped = Наблюдение остановлено
tools-trade-watcher-stop-failed = Не удалось остановить наблюдение
tools-trade-watcher-stopped-all = Все наблюдения остановлены
tools-trade-watcher-stop-all-failed = Не удалось остановить наблюдения
tools-trade-watcher-load-failed = Не удалось загрузить наблюдения
tools-trade-watcher-column-token = Токен
tools-trade-watcher-column-type = Тип
tools-trade-watcher-column-trigger = Срабатывание
tools-trade-watcher-column-action = Действие
tools-trade-watcher-column-triggered = Сработало
tools-trade-watcher-stop-watch =
    .title = Остановить наблюдение

## Results returned by the tools backend.

tools-burn-failure-native-asset = Нельзя сжечь { -sol }
tools-burn-failure-open-position = Нельзя сжигать токены из открытых позиций
tools-burn-failure-account-not-found = Токен-аккаунт не найден
tools-burn-failure-zero-balance = Баланс токена уже равен нулю
tools-burn-failure-transaction = Транзакция не выполнена
tools-burn-warning-open-position = Нельзя сжигать токены из открытых позиций
tools-burn-warning-closed-position = Остаток от закрытой позиции
tools-burn-warning-worth = Стоит ~{ $amount } { -sol }
tools-multi-buy-warning-insufficient = Недостаточно средств. Нужно { $needed } { -sol }, есть { $have } { -sol }
tools-multi-buy-warning-over-limit = Необходимая общая сумма { -sol } ({ $needed }) превышает лимит ({ $limit })
tools-multi-sell-warning-no-wallets = Второстепенные кошельки не найдены
tools-multi-sell-warning-no-balance = Ни на одном кошельке нет баланса токена
tools-multi-op-buy-failed = Не удалось купить
tools-multi-op-sell-failed = Не удалось продать
tools-multi-op-transfer-failed = Не удалось выполнить перевод
tools-multi-op-balance-failed = Не удалось получить баланс
tools-multi-op-mint-invalid = Недопустимый адрес минта
tools-multi-buy-session-failed = Мультипокупка не удалась
tools-multi-sell-session-failed = Мультипродажа не удалась
tools-multi-session-aborted = Операция прервана пользователем

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = Сканировать кошелёк
tools-wallet-action-scanning = Сканирование...
tools-wallet-scan-failed = Не удалось выполнить сканирование: { $reason }
tools-wallet-amount-approx = ~{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
        [one] Выбран: { $count } кошелёк
        [few] Выбрано: { $count } кошелька
        [many] Выбрано: { $count } кошельков
       *[other] Выбрано: { $count } кошелька
    }
tools-wallet-transfer-failed = Не удалось выполнить перевод: { $reason }
tools-wallet-cleanup-failed = Не удалось выполнить очистку: { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = Результаты сканирования
tools-wallet-cleanup-stat-empty = Пустые ATA
tools-wallet-cleanup-stat-reclaimable = { -sol } к возврату
tools-wallet-cleanup-stat-failed = Не удалось (в кэше)
tools-wallet-cleanup-prompt = Нажмите «Сканировать кошелёк», чтобы найти пустые ATA
tools-wallet-cleanup-prompt-hint = Будут проверены все токен-аккаунты вашего кошелька
tools-wallet-cleanup-action-cleanup = Очистить все
tools-wallet-cleanup-action-cleaning = Очистка...
tools-wallet-cleanup-scanning = Сканируем кошелёк...
tools-wallet-cleanup-found =
    { $count ->
        [one] Найден { $count } пустой ATA на ~{ $amount }
        [few] Найдено { $count } пустых ATA на ~{ $amount }
        [many] Найдено { $count } пустых ATA на ~{ $amount }
       *[other] Найдено { $count } пустых ATA на ~{ $amount }
    }
tools-wallet-cleanup-clean = Пустых ATA не найдено — кошелёк чист!
tools-wallet-cleanup-scan-failed = Не удалось просканировать ATA
tools-wallet-cleanup-done =
    { $count ->
        [one] Очищен { $count } ATA
        [few] Очищено { $count } ATA
        [many] Очищено { $count } ATA
       *[other] Очищено { $count } ATA
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = Сжигание токенов
tools-burn-info-title = Что такое сжигание?
tools-burn-info-body = Сжигание навсегда уничтожает токены, и вернуть их невозможно. После сжигания запустите очистку кошелька, чтобы закрыть пустые ATA и вернуть ~0,002 { -sol } ренты за каждый токен.
tools-burn-stat-total = Всего токенов
tools-burn-stat-selected = Выбрано
tools-burn-stat-rent = Рента к возврату
tools-burn-prompt = Нажмите «Сканировать кошелёк», чтобы найти токены
tools-burn-scanning = Ищем токены в кошельке...
tools-burn-scan-failed = Не удалось просканировать токены
tools-burn-empty = В кошельке не найдено токенов
tools-burn-action-burn = Сжечь выбранные ({ $count })
tools-burn-action-burning = Сжигание...
tools-burn-cannot-burn = Сжечь нельзя
tools-burn-no-value = Без стоимости

tools-burn-category-open-position = Открытые позиции
tools-burn-category-has-value = Есть стоимость
tools-burn-category-closed-position = Закрытые позиции
tools-burn-category-zero-liquidity = Нулевая ликвидность
tools-burn-category-hint-open-position = Нельзя сжигать токены из открытых позиций
tools-burn-category-hint-has-value = Подумайте о продаже вместо сжигания
tools-burn-category-hint-closed-position = Остатки от закрытых сделок
tools-burn-category-hint-zero-liquidity = Сжигать безопасно — рыночной стоимости нет

tools-burn-confirm-title = Подтвердите сжигание
tools-burn-confirm-message =
    { $count ->
        [one] Вы уверены, что хотите сжечь <strong>{ $count }</strong> токен?
        [few] Вы уверены, что хотите сжечь <strong>{ $count }</strong> токена?
        [many] Вы уверены, что хотите сжечь <strong>{ $count }</strong> токенов?
       *[other] Вы уверены, что хотите сжечь <strong>{ $count }</strong> токена?
    }
tools-burn-confirm-value = Общая оценочная стоимость: <strong>{ $amount }</strong>
tools-burn-confirm-continue = Продолжить
tools-burn-final-title = Последнее предупреждение
tools-burn-final-headline = Это действие НЕОБРАТИМО!
tools-burn-final-message =
    { $count ->
        [one] Следующий токен ({ $count }) будет безвозвратно уничтожен и не подлежит восстановлению ни при каких обстоятельствах.
        [few] Следующие токены ({ $count }) будут безвозвратно уничтожены и не подлежат восстановлению ни при каких обстоятельствах.
        [many] Следующие токены ({ $count }) будут безвозвратно уничтожены и не подлежат восстановлению ни при каких обстоятельствах.
       *[other] Следующие токены ({ $count }) будут безвозвратно уничтожены и не подлежат восстановлению ни при каких обстоятельствах.
    }
tools-burn-final-confirm = Да, сжечь токены
tools-burn-toast-burned =
    { $total ->
        [one] Сожжено токенов: { $successful }/{ $total }. Запустите очистку кошелька, чтобы вернуть ~{ $amount }
        [few] Сожжено токенов: { $successful }/{ $total }. Запустите очистку кошелька, чтобы вернуть ~{ $amount }
        [many] Сожжено токенов: { $successful }/{ $total }. Запустите очистку кошелька, чтобы вернуть ~{ $amount }
       *[other] Сожжено токенов: { $successful }/{ $total }. Запустите очистку кошелька, чтобы вернуть ~{ $amount }
    }
tools-burn-toast-failed =
    { $count ->
        [one] Не удалось сжечь { $count } токен
        [few] Не удалось сжечь { $count } токена
        [many] Не удалось сжечь { $count } токенов
       *[other] Не удалось сжечь { $count } токена
    }
tools-burn-failed = Не удалось сжечь: { $reason }
tools-burn-failures-title =
    { $count ->
        [one] Не удалось сжечь { $count } токен
        [few] Не удалось сжечь { $count } токена
        [many] Не удалось сжечь { $count } токенов
       *[other] Не удалось сжечь { $count } токена
    }
tools-burn-failure-unknown = Причина не указана

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = О инструменте
tools-airdrop-about-body = Проверяйте ожидающие airdrop, доступные к получению награды и неполученные распределения в популярных протоколах Solana.
tools-airdrop-list-title = Доступные airdrop
tools-airdrop-prompt = Нажмите «Проверить airdrop», чтобы найти доступные для получения
tools-airdrop-action-check = Проверить airdrop
tools-airdrop-action-claim-all = Получить все

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = Параметры генератора
tools-generator-warning-title = Храните приватные ключи в безопасности!
tools-generator-warning-body = Пары ключей создаются локально и никогда не передаются. Всегда делайте резервную копию ключей в надёжном месте.
tools-generator-count-label = Число кошельков
tools-generator-vanity-label = Красивый адрес (начинается с заданных символов)
tools-generator-prefix-label = Префикс
tools-generator-prefix-input =
    .placeholder = напр., SOL
tools-generator-prefix-hint = Чем длиннее префикс, тем экспоненциально дольше генерация
tools-generator-list-title = Созданные кошельки
tools-generator-empty = Кошельки ещё не созданы
tools-generator-action-generate = Создать
tools-generator-action-generating = Создание...
tools-generator-count-invalid = Введите число от 1 до 10
tools-generator-no-keypairs = Пары ключей не получены
tools-generator-generated =
    { $count ->
        [one] Создан { $count } кошелёк
        [few] Создано { $count } кошелька
        [many] Создано { $count } кошельков
       *[other] Создано { $count } кошелька
    }
tools-generator-failed = Не удалось создать кошельки: { $reason }
tools-generator-copy-public-key =
    .title = Копировать публичный ключ
tools-generator-copy-private-key =
    .title = Копировать приватный ключ
tools-generator-remove =
    .title = Убрать из списка
tools-generator-reveal =
    .title = Показать приватный ключ
tools-generator-public-key-label = Публичный ключ:
tools-generator-private-key-label = Приватный ключ:
tools-generator-public-key-name = Публичный ключ
tools-generator-private-key-copied = Приватный ключ скопирован
tools-generator-private-key-warning = Любой, у кого есть этот ключ, управляет кошельком
tools-generator-export-empty = Нет кошельков для экспорта
tools-generator-exported = Кошельки экспортированы — храните их в безопасности

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = Сводка
tools-consolidation-stat-wallets = Дополнительные кошельки
tools-consolidation-stat-native = Всего { -sol }
tools-consolidation-stat-tokens = Типы токенов
tools-consolidation-stat-rent = Возвращаемая рента
tools-consolidation-wallets-title = Кошельки
tools-consolidation-loading-wallets = Загрузка кошельков...
tools-consolidation-loading-data = Загрузка данных кошелька...
tools-consolidation-action-transfer-native = Перевести { -sol }
tools-consolidation-action-transfer-tokens = Перевести все токены
tools-consolidation-action-cleanup = Очистить ATA
tools-consolidation-action-transferring = Перевод...
tools-consolidation-column-name = Название
tools-consolidation-column-native = Баланс { -sol }
tools-consolidation-column-tokens = Токены
tools-consolidation-column-atas = Пустые ATA
tools-consolidation-empty = Дополнительные кошельки не найдены
tools-consolidation-empty-hint = Создайте дополнительные кошельки через мультипокупку, чтобы начать
tools-consolidation-load-failed = Не удалось загрузить: { $reason }
tools-consolidation-select-prompt = Выберите кошельки для консолидации
tools-consolidation-selection-totals =
    | { $amount } | { $tokens ->
        [one] { $tokens } токен
        [few] { $tokens } токена
        [many] { $tokens } токенов
       *[other] { $tokens } токена
    } | { $atas ->
        [one] { $atas } пустой ATA
        [few] { $atas } пустых ATA
        [many] { $atas } пустых ATA
       *[other] { $atas } пустых ATA
    }
tools-consolidation-transferred-native = Переведено на основной кошелёк: { $amount }
tools-consolidation-transferred-tokens =
    { $count ->
        [one] На основной кошелёк переведён { $count } токен
        [few] На основной кошелёк переведено { $count } токена
        [many] На основной кошелёк переведено { $count } токенов
       *[other] На основной кошелёк переведено { $count } токена
    }
tools-consolidation-cleaned =
    { $count ->
        [one] Закрыт { $count } ATA, возвращено { $amount }
        [few] Закрыто { $count } ATA, возвращено { $amount }
        [many] Закрыто { $count } ATA, возвращено { $amount }
       *[other] Закрыто { $count } ATA, возвращено { $amount }
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = Токен
tools-multi-mint-label = Адрес минта токена
tools-multi-mint-input =
    .placeholder = Вставьте адрес минта токена...
tools-multi-execution-title = Настройки выполнения
tools-multi-delay-min-label = Мин. задержка
tools-unit-native = { -sol }
tools-unit-seconds = с
tools-unit-ms = мс
tools-multi-delay-max-label = Макс. задержка
tools-multi-concurrency-label = Параллелизм
tools-multi-concurrency-sequential = { $count } (последовательно)
tools-multi-concurrency-parallel = { $count } параллельно
tools-multi-slippage-label = Проскальзывание
tools-multi-router-label = Роутер
tools-multi-router-auto = Авто (лучший маршрут)
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = Прямой пул
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = Прогресс
tools-multi-progress-preparing = Подготовка...
tools-multi-status-line = { $label } ({ $completed }/{ $total })
tools-multi-column-wallet = Кошелёк
tools-multi-column-route = Маршрут
tools-multi-column-status = Статус
tools-multi-op-completed = Выполнено
tools-multi-op-failed = Не удалось
tools-multi-action-stop = Стоп
tools-multi-action-loading = Загрузка...
tools-multi-start-failed = Не удалось запустить: { $reason }

tools-multi-state-pending = Ожидание
tools-multi-state-funding = Пополнение
tools-multi-state-executing = Выполнение
tools-multi-state-consolidating = Консолидация
tools-multi-state-completed = Завершено
tools-multi-state-failed = Не удалось
tools-multi-state-aborted = Прервано

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = Токен, который вы хотите купить с нескольких кошельков
tools-multi-buy-wallets-title = Настройки кошельков
tools-multi-buy-wallet-count-label = Число кошельков
tools-multi-buy-wallet-count-option =
    { $count ->
        [one] { $count } кошелёк
        [few] { $count } кошелька
        [many] { $count } кошельков
       *[other] { $count } кошелька
    }
tools-multi-buy-wallet-count-hint = Сколько дополнительных кошельков использовать
tools-multi-buy-buffer-label = Запас { -sol } на кошелёк
tools-multi-buy-buffer-hint = Резерв на комиссии (мин. 0,015 { -sol })
tools-multi-buy-amounts-title = Настройки сумм
tools-multi-buy-min-label = Мин. { -sol } на кошелёк
tools-multi-buy-min-hint = Минимальная сумма покупки
tools-multi-buy-max-label = Макс. { -sol } на кошелёк
tools-multi-buy-max-hint = Максимальная сумма покупки
tools-multi-buy-limit-label = Общий лимит { -sol } (необязательно)
tools-multi-buy-limit-hint = Максимальные общие траты
tools-multi-buy-preview-title = Предпросмотр
tools-multi-buy-preview-create = Кошельков к созданию
tools-multi-buy-preview-amount = Сумма на кошелёк
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = Всего нужно { -sol }
tools-multi-buy-preview-balance = Баланс основного кошелька
tools-multi-buy-action-preview = Предпросмотр
tools-multi-buy-action-start = Запустить мультипокупку
tools-multi-buy-executing = Выполняем покупки...
tools-multi-buy-column-spent = Потрачено { -sol }
tools-multi-buy-column-tokens = Токены
tools-multi-buy-preview-failed = Не удалось выполнить предпросмотр: { $reason }
tools-multi-buy-started = Мультипокупка запущена
tools-multi-buy-stopped = Мультипокупка остановлена
tools-multi-buy-completed = Мультипокупка завершена! Успешно: { $successful }/{ $total }

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = Введите адрес токена, чтобы найти кошельки, в которых он есть
tools-multi-sell-action-scan = Сканировать
tools-multi-sell-settings-title = Настройки продажи
tools-multi-sell-percent-label = Процент продажи
tools-multi-sell-percent-hint = % токенов для продажи на каждом кошельке
tools-multi-sell-min-fee-label = Мин. { -sol } на комиссию
tools-multi-sell-min-fee-hint = Минимум { -sol }, необходимый для комиссии транзакции
tools-multi-sell-topup-label = Автопополнение при необходимости
tools-multi-sell-topup-hint = Переводить { -sol } с основного кошелька, если на дополнительном недостаточно средств
tools-multi-sell-post-title = Действия после продажи
tools-multi-sell-consolidate-label = Консолидировать { -sol } на основной кошелёк
tools-multi-sell-consolidate-hint = Перевести весь { -sol } с дополнительных кошельков обратно на основной
tools-multi-sell-close-atas-label = Закрывать ATA токена после продажи
tools-multi-sell-close-atas-hint = Возвращает ~0,002 { -sol } за каждый ATA
tools-multi-sell-wallets-title = Кошельки с токеном
tools-multi-sell-empty = Ни на одном дополнительном кошельке нет этого токена
tools-multi-sell-column-tokens = Токены
tools-multi-sell-column-native = Баланс { -sol }
tools-multi-sell-column-topup = Нужно пополнение
tools-multi-sell-none-selected = Кошельки не выбраны
tools-multi-sell-select-required = Выберите хотя бы один кошелёк
tools-multi-sell-action-start = Запустить мультипродажу
tools-multi-sell-executing = Выполняем продажи...
tools-multi-sell-column-sold = Продано токенов
tools-multi-sell-column-received = Получено { -sol }
tools-multi-sell-started = Мультипродажа запущена
tools-multi-sell-stopped = Мультипродажа остановлена
tools-multi-sell-completed = Мультипродажа завершена! Получено: { $amount }

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = Избранное
tools-favorites-saved = Сохранённое избранное
tools-favorites-save-current = Сохранить текущее
tools-favorites-empty = Избранного пока нет
tools-favorites-no-label = Без метки
tools-favorites-uses = { $count }×
tools-favorites-remove = Удалить
tools-favorites-loaded = Загружено избранное: { $name }
tools-favorites-default-name = Конфигурация
tools-favorites-mint-required = Сначала введите адрес минта токена
tools-favorites-add-title = Добавить в избранное
tools-favorites-add-message = Введите метку для этого избранного
tools-favorites-add-placeholder = Метка (необязательно)...
tools-favorites-saved-toast = Сохранено в избранное
tools-favorites-save-failed = Не удалось сохранить избранное
tools-favorites-remove-title = Удалить из избранного
tools-favorites-remove-message = Удалить это избранное?
tools-favorites-removed-toast = Избранное удалено
tools-favorites-remove-failed = Не удалось удалить избранное
