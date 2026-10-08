transactions-type-buy = Покупка
transactions-type-sell = Продажа
transactions-type-swap = Своп
transactions-type-sol-transfer = Перевод SOL
transactions-type-token-transfer = Перевод токена
transactions-type-transfer = Перевод
transactions-type-dust = Пыль
transactions-type-spam = Спам
transactions-type-ata-create = Аккаунт открыт
transactions-type-ata-close = Рента возвращена
transactions-type-ata = Токен-аккаунт
transactions-type-liquidity-add = Добавление ликвидности
transactions-type-liquidity-remove = Вывод ликвидности
transactions-type-nft = NFT
transactions-type-program = Вызов программы
transactions-type-compute = Вычисления
transactions-type-failed = Ошибка
transactions-type-unknown = Не классифицировано

transactions-type-with-detail = { $label } ({ $detail })
transactions-type-token-transfer-detail = { $label } { $mint } ({ $amount })
transactions-type-spam-detail = Спам-аирдроп ({ $mint })
transactions-type-described = { $description }

transactions-filter-all = Все типы
transactions-filter-transfer = Переводы
transactions-filter-ata = Рента и аккаунты
transactions-filter-liquidity = Ликвидность
transactions-filter-program = Вызовы программ

transactions-direction-incoming = Входящая
transactions-direction-outgoing = Исходящая
transactions-direction-internal = Внутренняя
transactions-direction-unknown = Не классифицировано

transactions-status-pending = Ожидает
transactions-status-confirmed = Подтверждена
transactions-status-finalized = Финализирована
transactions-status-failed = Ошибка
transactions-status-success = Успешно
transactions-status-unknown = Неизвестно

transactions-ata-operation-creation = Создание
transactions-ata-operation-closure = Закрытие

transactions-toolbar-title = История транзакций
transactions-search =
    .placeholder = Поиск по подписям…
    .aria-label = Поиск по подписям транзакций
transactions-load-failed = Не удалось обновить транзакции
transactions-summary-total = Всего
transactions-summary-estimate = Оценка
transactions-summary-success = Успешно
transactions-summary-failed = Ошибка
transactions-filter-wallet = Кошелёк
transactions-filter-type = Тип
transactions-filter-direction = Направление
transactions-filter-status = Статус
transactions-filter-all-directions = Все направления
transactions-filter-all-statuses = Все статусы
transactions-wallet-main = Основной кошелёк
transactions-col-time = Время
transactions-col-signature = Подпись
transactions-col-type = Тип
transactions-col-direction = Направление
transactions-col-status = Статус
transactions-col-native-delta = Δ { -sol }
transactions-col-fees = Комиссии ({ -sol })
transactions-col-token = Токен
transactions-col-router = Роутер
transactions-col-instructions = Инстр.

transactions-dialog-copy-signature =
    .title = Скопировать подпись
transactions-dialog-close =
    .title = Закрыть (ESC)
transactions-dialog-tabs-label = Разделы сведений о транзакции
transactions-dialog-meta-slot = Слот:
transactions-dialog-meta-fee = Комиссия:
transactions-dialog-loading = Загрузка...
transactions-dialog-loading-details = Загрузка сведений о транзакции...
transactions-dialog-load-failed = Не удалось загрузить сведения о транзакции
transactions-dialog-load-failed-reason = Не удалось загрузить сведения о транзакции: { $reason }
transactions-dialog-not-found = Транзакция не найдена
transactions-dialog-tab-overview = Обзор
transactions-dialog-tab-balances = Балансы
transactions-dialog-tab-instructions = Инструкции
transactions-dialog-tab-logs = Журнал
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = Исходные данные
transactions-dialog-unknown = Неизвестно
transactions-dialog-unknown-asset = Неизвестный актив
transactions-dialog-unavailable = Недоступно

transactions-dialog-failed-title = Транзакция не выполнена
transactions-dialog-no-program-error = Ошибка программы не указана.
transactions-dialog-story-title = Что произошло
transactions-dialog-router-via = через { $router }
transactions-dialog-flow-paid = Оплачено
transactions-dialog-flow-received = Получено
transactions-dialog-flow-from = Откуда
transactions-dialog-flow-to = Куда
transactions-dialog-flow-amount = Сумма
transactions-dialog-net-wallet-change = Итоговое изменение кошелька:
transactions-dialog-processed = Обработана в Solana
transactions-dialog-execution-title = Исполнение
transactions-dialog-metric-execution-price = Цена исполнения
transactions-dialog-metric-effective-received = Фактически получено
transactions-dialog-metric-effective-spent = Фактически потрачено
transactions-dialog-metric-network-fee = Комиссия сети
transactions-dialog-metric-estimated-pnl = Оценочный P&L
transactions-dialog-metric-net-native-change = Итоговое изменение { -sol }
transactions-dialog-route-title = Маршрут и активы
transactions-dialog-route-router = Роутер
transactions-dialog-route-input-asset = Входной актив
transactions-dialog-route-output-asset = Выходной актив
transactions-dialog-route-pool = Пул
transactions-dialog-route-program = Программа
transactions-dialog-tech-title = Технические сведения
transactions-dialog-tech-summary = Подпись, слот и ресурсы
transactions-dialog-tech-signature = Подпись
transactions-dialog-tech-timestamp = Метка времени
transactions-dialog-tech-slot = Слот
transactions-dialog-tech-exact-fee = Точная комиссия
transactions-dialog-tech-accounts = Аккаунты
transactions-dialog-tech-instructions = Инструкции
transactions-dialog-tech-compute-units = Вычислительные единицы
transactions-dialog-tech-token-decimals = Десятичные знаки токена

transactions-dialog-balances-native-title = Изменения баланса { -sol }
transactions-dialog-balances-native-empty = Нет изменений баланса { -sol }
transactions-dialog-balances-token-title = Изменения баланса токенов
transactions-dialog-balances-token-empty = Нет изменений баланса токенов
transactions-dialog-balances-net-native = Итоговое изменение { -sol }
transactions-dialog-balances-fee = Комиссия транзакции
transactions-dialog-col-account = Аккаунт
transactions-dialog-col-token = Токен
transactions-dialog-col-pre-balance = Баланс до
transactions-dialog-col-post-balance = Баланс после
transactions-dialog-col-change = Изменение
transactions-dialog-col-type = Тип
transactions-dialog-col-rent = Рента ({ -sol })
transactions-dialog-instructions-empty = Инструкции не найдены
transactions-dialog-instructions-count =
    { $count ->
        [one] { $count } инструкция
        [few] { $count } инструкции
        [many] { $count } инструкций
       *[other] { $count } инструкции
    }
transactions-dialog-instruction-program-id = ID программы
transactions-dialog-instruction-accounts = Аккаунты ({ $count })
transactions-dialog-instruction-data = Данные
transactions-dialog-logs-empty = Журнал недоступен
transactions-dialog-logs-filter = Фильтр журнала...
transactions-dialog-logs-no-match = Нет подходящих записей журнала
transactions-dialog-logs-count =
    { $count ->
        [one] { $count } запись журнала
        [few] { $count } записи журнала
        [many] { $count } записей журнала
       *[other] { $count } записи журнала
    }
transactions-dialog-ata-empty = В этой транзакции нет операций с ATA
transactions-dialog-ata-summary-title = Сводка анализа ATA
transactions-dialog-ata-creations = Создания
transactions-dialog-ata-closures = Закрытия
transactions-dialog-ata-rent-spent = Потрачено на ренту
transactions-dialog-ata-rent-recovered = Возвращено ренты
transactions-dialog-ata-net-rent = Итоговое влияние ренты
transactions-dialog-ata-operations-title = Операции с ATA ({ $count })
transactions-dialog-raw-copy = Скопировать JSON
transactions-dialog-raw-empty = Исходные данные недоступны

# Empty table (scripts/pages/transactions.js)
transactions-empty = Транзакций пока нет
    .message = Свопы и переводы торгового кошелька появляются здесь после подтверждения в блокчейне.
