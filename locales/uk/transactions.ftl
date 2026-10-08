# Transaction type labels. Ids come from TransactionType::kind() in
# src/transactions/types.rs; the dashboard maps them in ui/transaction_type.js.

transactions-type-buy = Купівля
transactions-type-sell = Продаж
transactions-type-swap = Своп
transactions-type-sol-transfer = Переказ SOL
transactions-type-token-transfer = Переказ токена
transactions-type-transfer = Переказ
transactions-type-dust = Пил
transactions-type-spam = Спам
transactions-type-ata-create = Акаунт відкрито
transactions-type-ata-close = Ренту повернуто
transactions-type-ata = Токен-акаунт
transactions-type-liquidity-add = Додавання ліквідності
transactions-type-liquidity-remove = Вилучення ліквідності
transactions-type-nft = NFT
transactions-type-program = Виклик програми
transactions-type-compute = Обчислення
transactions-type-failed = Невдала
transactions-type-unknown = Не класифіковано

# A type with the payload that identifies it, as shown in the position activity feed.
transactions-type-with-detail = { $label } ({ $detail })
transactions-type-token-transfer-detail = { $label } { $mint } ({ $amount })
transactions-type-spam-detail = Спам-аірдроп ({ $mint })
transactions-type-described = { $description }

# Type filter entries whose wording differs from the type label.
transactions-filter-all = Усі типи
transactions-filter-transfer = Перекази
transactions-filter-ata = Рента й акаунти
transactions-filter-liquidity = Ліквідність
transactions-filter-program = Виклики програм

# Wallet-relative direction. Ids come from TransactionDirection in src/transactions/types.rs
# (ui/transaction_direction.js).
transactions-direction-incoming = Вхідна
transactions-direction-outgoing = Вихідна
transactions-direction-internal = Внутрішня
transactions-direction-unknown = Не класифіковано

# Chain status. Ids come from TransactionStatus in src/transactions/types.rs
# (ui/transaction_status.js); Success and Unknown label a row without a status.
transactions-status-pending = В очікуванні
transactions-status-confirmed = Підтверджена
transactions-status-finalized = Фіналізована
transactions-status-failed = Невдала
transactions-status-success = Успішна
transactions-status-unknown = Невідомо

# Ids come from AtaOperationType in src/transactions/types.rs.
transactions-ata-operation-creation = Створення
transactions-ata-operation-closure = Закриття

## Transactions page (pages/transactions.js)

transactions-toolbar-title = Історія транзакцій
transactions-search =
    .placeholder = Пошук підписів…
    .aria-label = Пошук підписів транзакцій
transactions-load-failed = Не вдалося оновити транзакції
transactions-summary-total = Усього
transactions-summary-estimate = Оцінка
transactions-summary-success = Успішні
transactions-summary-failed = Невдалі
transactions-filter-wallet = Гаманець
transactions-filter-type = Тип
transactions-filter-direction = Напрямок
transactions-filter-status = Статус
transactions-filter-all-directions = Усі напрямки
transactions-filter-all-statuses = Усі статуси
transactions-wallet-main = Основний гаманець
transactions-col-time = Час
transactions-col-signature = Підпис
transactions-col-type = Тип
transactions-col-direction = Напрямок
transactions-col-status = Статус
transactions-col-native-delta = Δ { -sol }
transactions-col-fees = Комісії ({ -sol })
transactions-col-token = Токен
transactions-col-router = Маршрутизатор
transactions-col-instructions = Інстр.

## Transaction details dialog (ui/transaction_details_dialog.js)

transactions-dialog-copy-signature =
    .title = Копіювати підпис
transactions-dialog-close =
    .title = Закрити (ESC)
transactions-dialog-tabs-label = Розділи деталей транзакції
transactions-dialog-meta-slot = Слот:
transactions-dialog-meta-fee = Комісія:
transactions-dialog-loading = Завантаження...
transactions-dialog-loading-details = Завантаження деталей транзакції...
transactions-dialog-load-failed = Не вдалося завантажити деталі транзакції
# $reason is the failure text reported by the server.
transactions-dialog-load-failed-reason = Не вдалося завантажити деталі транзакції: { $reason }
transactions-dialog-not-found = Транзакцію не знайдено
transactions-dialog-tab-overview = Огляд
transactions-dialog-tab-balances = Баланси
transactions-dialog-tab-instructions = Інструкції
transactions-dialog-tab-logs = Журнали
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = Сирі дані
transactions-dialog-unknown = Невідомо
transactions-dialog-unknown-asset = Невідомий актив
transactions-dialog-unavailable = Недоступно

## Transaction details dialog: overview

transactions-dialog-failed-title = Транзакція не вдалася
transactions-dialog-no-program-error = Помилку програми не надано.
transactions-dialog-story-title = Що сталося
# $router is the routing program name.
transactions-dialog-router-via = через { $router }
transactions-dialog-flow-paid = Сплачено
transactions-dialog-flow-received = Отримано
transactions-dialog-flow-from = Від
transactions-dialog-flow-to = До
transactions-dialog-flow-amount = Сума
transactions-dialog-net-wallet-change = Чиста зміна гаманця:
transactions-dialog-processed = Оброблено в Solana
transactions-dialog-execution-title = Виконання
transactions-dialog-metric-execution-price = Ціна виконання
transactions-dialog-metric-effective-received = Фактично отримано
transactions-dialog-metric-effective-spent = Фактично витрачено
transactions-dialog-metric-network-fee = Комісія мережі
transactions-dialog-metric-estimated-pnl = Оцінений прибуток/збиток
transactions-dialog-metric-net-native-change = Чиста зміна { -sol }
transactions-dialog-route-title = Маршрут і активи
transactions-dialog-route-router = Маршрутизатор
transactions-dialog-route-input-asset = Вхідний актив
transactions-dialog-route-output-asset = Вихідний актив
transactions-dialog-route-pool = Пул
transactions-dialog-route-program = Програма
transactions-dialog-tech-title = Технічні деталі
transactions-dialog-tech-summary = Підпис, слот і ресурси
transactions-dialog-tech-signature = Підпис
transactions-dialog-tech-timestamp = Позначка часу
transactions-dialog-tech-slot = Слот
transactions-dialog-tech-exact-fee = Точна комісія
transactions-dialog-tech-accounts = Акаунти
transactions-dialog-tech-instructions = Інструкції
transactions-dialog-tech-compute-units = Обчислювальні одиниці
transactions-dialog-tech-token-decimals = Десяткові знаки токена

## Transaction details dialog: balances, instructions, logs, ATA and raw tabs

transactions-dialog-balances-native-title = Зміни балансу { -sol }
transactions-dialog-balances-native-empty = Немає змін балансу { -sol }
transactions-dialog-balances-token-title = Зміни балансу токенів
transactions-dialog-balances-token-empty = Немає змін балансу токенів
transactions-dialog-balances-net-native = Чиста зміна { -sol }
transactions-dialog-balances-fee = Комісія за транзакцію
transactions-dialog-col-account = Акаунт
transactions-dialog-col-token = Токен
transactions-dialog-col-pre-balance = Баланс до
transactions-dialog-col-post-balance = Баланс після
transactions-dialog-col-change = Зміна
transactions-dialog-col-type = Тип
transactions-dialog-col-rent = Рента ({ -sol })
transactions-dialog-instructions-empty = Інструкцій не знайдено
transactions-dialog-instructions-count =
    { $count ->
        [one] { $count } інструкція
        [few] { $count } інструкції
        [many] { $count } інструкцій
       *[other] { $count } інструкції
    }
transactions-dialog-instruction-program-id = ID програми
transactions-dialog-instruction-accounts = Акаунти ({ $count })
transactions-dialog-instruction-data = Дані
transactions-dialog-logs-empty = Журнали недоступні
transactions-dialog-logs-filter = Фільтр журналів...
transactions-dialog-logs-no-match = Немає відповідних журналів
transactions-dialog-logs-count =
    { $count ->
        [one] { $count } запис журналу
        [few] { $count } записи журналу
        [many] { $count } записів журналу
       *[other] { $count } запису журналу
    }
transactions-dialog-ata-empty = У цій транзакції немає операцій ATA
transactions-dialog-ata-summary-title = Підсумок аналізу ATA
transactions-dialog-ata-creations = Створення
transactions-dialog-ata-closures = Закриття
transactions-dialog-ata-rent-spent = Витрачено ренти
transactions-dialog-ata-rent-recovered = Повернуто ренти
transactions-dialog-ata-net-rent = Чистий вплив ренти
transactions-dialog-ata-operations-title = Операції ATA ({ $count })
transactions-dialog-raw-copy = Копіювати JSON
transactions-dialog-raw-empty = Сирі дані недоступні

# Empty table (scripts/pages/transactions.js)
transactions-empty = Транзакцій поки немає
    .message = Свопи та перекази торгового гаманця з’являються тут після підтвердження в блокчейні.
