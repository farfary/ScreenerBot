positions-state-reason-position-created = Позиция создана

positions-status-open = Открытые
positions-status-closed = Закрытые
positions-status-archived = В архиве
positions-origin-copy = Копирование
positions-origin-manual = Вручную
positions-origin-wallet = Кошелёк
positions-origin-copy-link =
    .title = Открыть задачу копирования, создавшую эту позицию
positions-holding-frozen = Заморожено
    .title = Токен-аккаунт заморожен владельцем полномочий mint — баланс нельзя перевести или продать
positions-toolbar-total = Всего
positions-toolbar-delete-all = Удалить все
positions-search-placeholder = Поиск по символу или минту...
positions-filter-origin = Источник
positions-filter-origin-all = Все источники
positions-filter-origin-auto = Автотрейдер
positions-filter-origin-copy = Копитрейдинг
positions-delete-all-tooltip = Удалить все архивные позиции навсегда

positions-column-token = Токен
positions-column-archived-at = В архиве
positions-column-entry-time = Время входа
positions-column-exit-time = Время выхода
positions-column-avg-entry = Сред. вход ({ -sol })
positions-column-avg-exit = Сред. выход ({ -sol })
positions-column-current-price = Текущая ({ -sol })
positions-column-total-invested = Всего вложено
positions-column-proceeds = Выручка
positions-column-pnl = P&L
positions-column-pnl-percent = P&L %
positions-column-size = Размер
positions-column-dca = DCA
positions-column-exits = Выходы
positions-column-unrealized-pnl = Нереализованный P&L
positions-column-unrealized-percent = Нереализованный %

positions-unknown-basis = В истории этого кошелька нет базовой стоимости (аирдроп, сделка с котировкой в USD или своп без части в SOL)
positions-unknown-history = Этот раунд не сходится с балансом в блокчейне
positions-dca-count =
    { $count ->
        [one] { $count } докупка
        [few] { $count } докупки
        [many] { $count } докупок
       *[other] { $count } докупки
    }
positions-exit-count =
    { $count ->
        [one] { $count } выход
        [few] { $count } выхода
        [many] { $count } выходов
       *[other] { $count } выхода
    }

positions-action-add =
    .title = Докупить (DCA)
    .aria-label = Докупить позицию
positions-action-sell =
    .title = Продать (полностью или часть в %)
    .aria-label = Продать позицию
positions-action-sell-frozen = Заморожено владельцем полномочий mint — этот актив нельзя продать
positions-action-remove =
    .title = Убрать (в архив или удалить)
    .aria-label = Убрать позицию
positions-action-restore =
    .title = Вернуть в открытые/закрытые
    .aria-label = Восстановить позицию
positions-action-delete =
    .title = Удалить навсегда
    .aria-label = Удалить навсегда
positions-action-in-progress = Выполняется…

positions-caption-buying = Покупка
positions-caption-buying-step = Покупка · { $step }
positions-caption-selling = Продажа
positions-caption-selling-step = Продажа · { $step }
positions-caption-closing = Закрытие
positions-caption-failed = Ошибка
positions-caption-failed-detail = Ошибка · { $error }
positions-step-adding = Докупка
positions-pending-buying = Покупка…
positions-pending-buy-failed = Покупка не удалась

positions-load-failed = Не удалось обновить позиции
positions-toast-not-found = Данные позиции не найдены
positions-toast-deleted = Позиция удалена
positions-toast-archived = Позиция перенесена в архив
positions-toast-restored = Позиция восстановлена
positions-action-failed = Не удалось выполнить действие
positions-delete-title = Удалить позицию навсегда
positions-delete-message = Удалить { $symbol } навсегда? Позиция и её история будут стёрты из базы данных, отменить это нельзя. Ваши транзакции и данные токена не затрагиваются.
positions-delete-confirm = Удалить навсегда
positions-delete-all-title = Удалить все архивные позиции
positions-delete-all-message =
    { $count ->
        [one] Удалить навсегда все архивные позиции (в архиве: { $count })? Это нельзя отменить. Транзакции и данные токенов не затрагиваются.
        [few] Удалить навсегда все архивные позиции (в архиве: { $count })? Это нельзя отменить. Транзакции и данные токенов не затрагиваются.
        [many] Удалить навсегда все архивные позиции (в архиве: { $count })? Это нельзя отменить. Транзакции и данные токенов не затрагиваются.
       *[other] Удалить навсегда все архивные позиции (в архиве: { $count })? Это нельзя отменить. Транзакции и данные токенов не затрагиваются.
    }
positions-delete-all-message-empty = Удалить навсегда все архивные позиции? Это нельзя отменить.
positions-delete-all-confirm = Удалить все
positions-delete-all-done =
    { $count ->
        [one] Удалена { $count } архивная позиция
        [few] Удалено { $count } архивные позиции
        [many] Удалено { $count } архивных позиций
       *[other] Удалено { $count } архивной позиции
    }
positions-delete-all-failed = Не удалось удалить архивные позиции

positions-remove-title = Убрать позицию
positions-remove-open-warning = <strong>Эта позиция всё ещё открыта.</strong> Бот держит этот токен. Если убрать позицию, освободится слот сделки и отслеживание прекратится — но токен <strong>не</strong> будет продан. Если хотите вернуть { -sol }, сначала продайте.
positions-remove-modes =
    .aria-label = Режим удаления
positions-remove-archive = В архив
positions-remove-recommended = Рекомендуется
positions-remove-archive-description = Скрыть во вкладке «В архиве». Можно вернуть в любой момент — ничего не продаётся, все сделки остаются в истории.
positions-remove-delete = Удалить навсегда
positions-remove-delete-description = Стереть эту позицию и всю её историю из базы данных.
positions-remove-danger = Позиция и её история будут удалены навсегда. <strong>Это нельзя отменить.</strong> Ваши транзакции и данные токена не затрагиваются.
positions-remove-confirm-archive = Архивировать позицию

positions-management-changed = Управление позицией: { $mode }
positions-details-load-failed = Не удалось загрузить сведения о позиции
positions-details-mint-label = Адрес минта
positions-details-management-failed = Не удалось изменить управление позицией
positions-details-favorite-add =
    .title = Добавить в избранное
    .aria-label = Добавить в избранное
positions-details-favorite-remove =
    .title = Убрать из избранного
    .aria-label = Убрать из избранного
positions-details-view-solscan =
    .title = Открыть в { -solscan }
    .aria-label = Открыть токен в { -solscan }
positions-details-close =
    .title = Закрыть (Esc)
    .aria-label = Закрыть
positions-details-chart-section =
    .aria-label = График цены
positions-details-loading-chart = Загрузка графика...
positions-details-activity-section =
    .aria-label = Активность
positions-details-activity-title = Активность
positions-details-split-handle =
    .aria-label = Изменить размер графика и активности
positions-details-activity-pane =
    .aria-label = Панель активности
positions-details-activity-expand =
    .title = Развернуть активность
    .aria-label = Развернуть активность
positions-details-summary-section =
    .aria-label = Сводка по позиции
positions-details-loading = Загрузка позиции...

positions-management-auto-trader = Автотрейдер
positions-management-user-only = Только вручную
positions-management-copy-task = Задача копирования
positions-management-hybrid = Гибридный
positions-pane-show-chart = Показать график
positions-pane-show-activity = Показать активность
positions-pane-restore-activity = Вернуть активность
positions-pane-expand-chart =
    .title = Развернуть график
    .aria-label = Развернуть график

positions-risk-low = Низкий риск
positions-risk-medium = Средний риск
positions-risk-high = Высокий риск
positions-risk-unknown = Риск неизвестен
positions-busy-buying = Идёт покупка…
positions-busy-selling = Идёт продажа…
positions-busy-closing = Идёт закрытие…
positions-header-avg-entry = Сред. вход
positions-header-buy-count =
    { $count ->
        [one] { $count } покупка
        [few] { $count } покупки
        [many] { $count } покупок
       *[other] { $count } покупки
    }
positions-header-exit-price = Цена выхода
positions-header-closed-ago = закрыта: { $ago }
positions-header-realized-pnl = Реализованный P&L
positions-header-usd-note = USD по сегодняшнему курсу { -sol }
positions-header-returned = Возвращено
positions-header-of-invested = из { $amount } вложенных
positions-header-price = Цена
positions-header-last-price = Последняя цена
positions-header-pool-ago = пул · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = Нереализованный P&L
positions-header-pnl-last-price = P&L по последней цене
positions-header-value = Стоимость
positions-header-last-value = Последняя стоимость
positions-header-invested = вложено: { $amount }
positions-header-origin-hint = Как была открыта эта позиция
positions-header-risk-hint = Оценка { -rugcheck } — чем ниже, тем безопаснее
positions-header-frozen = Заморожено
    .title = Владелец полномочий mint заморозил этот актив
positions-header-managed-by = Управление
positions-header-management-select =
    .aria-label = Управление позицией

positions-origin-unknown = неизвестно
positions-origin-copied-task = Копирование · задача { $task }
positions-origin-manual-entry = Вход вручную
positions-origin-wallet-entry = Вход из кошелька
positions-origin-auto-strategy = Авто · { $strategy }
positions-origin-auto-entry = Автовход

positions-pending-adding = Докупка
positions-pending-adding-amount = Докупка: { $amount }
positions-pending-selling = Продажа
positions-pending-selling-percent = Продажа: { $percent }
positions-pending-confirming = { $label } · подтверждается
    .title = Отправлено, ожидает подтверждения в блокчейне. Показатели обновятся после проверки.

positions-trade-add = Докупить
    .title = Докупить позицию
positions-trade-sell = Продать
    .title = Продать часть позиции
positions-trade-close = Закрыть позицию
    .title = Продать всё и закрыть
positions-trade-token = О токене
    .title = Открыть сведения о токене

positions-favorite-token-fallback = Токен
positions-favorite-added = { $symbol } добавлен в избранное
positions-favorite-removed = { $symbol } убран из избранного
positions-favorite-add-failed = Не удалось добавить в избранное
positions-favorite-remove-failed = Не удалось убрать из избранного
positions-favorite-update-failed = Не удалось обновить избранное

positions-summary-position = Позиция
positions-summary-price-path = Путь цены
positions-summary-network-fees = Комиссии сети
positions-summary-risk = Риск
positions-summary-market = Рынок
positions-summary-market-now = Рынок сейчас
positions-summary-links = Ссылки
positions-fact-tokens-fallback = токенов
positions-fact-bought = Куплено
positions-fact-holding = В позиции
positions-fact-sold = Продано
positions-fact-realized = Реализовано
positions-fact-opened = Открыта
positions-fact-closed = Закрыта
positions-fact-reason = Причина
positions-fact-archived = В архиве
positions-fact-entry = Вход
positions-fact-exit = Выход
positions-fact-total = Итого
positions-fact-verified = Проверено в блокчейне
positions-fact-confirming = Подтверждается
positions-fact-share-of-bought = { $percent } от купленного
positions-fact-share-of-invested = { $percent } от вложенного
positions-fact-entry-count =
    { $count ->
        [0] 1 вход
        [one] 1 вход + { $count } докупка
        [few] 1 вход + { $count } докупки
        [many] 1 вход + { $count } докупок
       *[other] 1 вход + { $count } докупки
    }
positions-fact-partial-exits-back =
    { $count ->
        [one] { $count } частичный выход · возвращено { $returned }
        [few] { $count } частичных выхода · возвращено { $returned }
        [many] { $count } частичных выходов · возвращено { $returned }
       *[other] { $count } частичного выхода · возвращено { $returned }
    }
positions-fact-held = удержание: { $age }
positions-fact-vs-entry = { $percent } к входу
positions-fact-exit-vs-peak = Выход к пику
positions-fact-now-vs-peak = Сейчас к пику
positions-fact-entry-range = Диапазон входа
positions-range-low = Минимум
positions-range-peak = Пик
positions-range-now = Сейчас
positions-range-label-exit = Цена входа и выхода между минимумом и пиком
positions-range-label-now = Цена входа и текущая цена между минимумом и пиком
positions-fact-mint-authority = Полномочия mint
positions-fact-freeze-authority = Полномочия freeze
positions-fact-active = Активны
positions-fact-pool = Пул
positions-fact-pool-liquidity = ликвидность { $amount } { -sol }
positions-fact-market-cap = Капитализация
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = Ликвидность
positions-fact-volume-24h = Объём за 24 ч
positions-fact-price-change = Изменение цены
positions-change-period-1h = 1 ч
positions-change-period-24h = 24 ч
positions-fact-holders = Холдеры
positions-link-website = Сайт
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

positions-activity-load-failed = Не удалось загрузить активность
positions-activity-loading = Загрузка активности...
positions-activity-empty = С этим токеном в этом кошельке пока ничего не происходило
positions-activity-filter-empty = Нет активности, подходящей под этот фильтр
positions-activity-round-count =
    { $count ->
        [one] { $count } раунд
        [few] { $count } раунда
        [many] { $count } раундов
       *[other] { $count } раунда
    }
positions-activity-event-count =
    { $count ->
        [one] { $count } событие
        [few] { $count } события
        [many] { $count } событий
       *[other] { $count } события
    }
positions-activity-pending-count = В ожидании: { $count }
positions-activity-failed-count = С ошибкой: { $count }
positions-filter-all = Все
positions-filter-trades = Сделки
positions-filter-buys = Покупки
positions-filter-sells = Продажи
positions-filter-wallet = Кошелёк
positions-filter-issues = Проблемы
positions-activity-filters =
    .aria-label = Фильтр активности
positions-activity-totals =
    .aria-label = Все раунды по этому токену
positions-activity-realized-all = Реализовано, все раунды
positions-activity-invested = Вложено
positions-activity-returned = Возвращено
positions-activity-opened = Открыта { $when }
positions-activity-round-title = Позиция { $index }
positions-activity-this-position = Эта позиция
positions-activity-dates-unavailable = Даты недоступны
positions-activity-wallet-title = Транзакции кошелька
positions-activity-outside =
    { $count ->
        [one] Вне позиций · { $range } · { $count } событие
        [few] Вне позиций · { $range } · { $count } события
        [many] Вне позиций · { $range } · { $count } событий
       *[other] Вне позиций · { $range } · { $count } события
    }
positions-details-signature-label = Подпись

positions-state-open = Позиция открыта
positions-state-closing = Позиция закрывается
positions-state-closed = Позиция закрыта
positions-state-exit-pending = Выход из позиции ожидает
positions-state-exit-failed = Выход из позиции не удался
positions-state-phantom = Фантомная позиция
positions-state-reconciling = Позиция сверяется

positions-event-kind-entry = Вход
positions-event-kind-dca = Докупка
positions-event-kind-partial-exit = Частичный выход
positions-event-kind-exit = Выход
positions-event-kind-buy = Покупка в кошельке
positions-event-kind-sell = Продажа в кошельке
positions-event-kind-transfer = Перевод
positions-event-kind-ata = Токен-аккаунт
positions-event-kind-other = Транзакция
positions-event-state-pending = Ожидает
positions-event-state-failed = Ошибка
positions-event-state-synthetic = Синтетическое
positions-chain-status-failed-detail = Ошибка: { $error }
positions-event-tokens-fallback = токенов
positions-event-entry-submitted = Отправлена покупка: { $amount }
positions-event-entry-for = Куплено { $amount } за { $sol }
positions-event-entry = Куплено: { $amount }
positions-event-dca-submitted = Отправлена докупка: { $amount }
positions-event-dca-for = Докуплено { $amount } за { $sol }
positions-event-dca = Докуплено: { $amount }
positions-event-partial-exit-submitted-percent = Отправлен частичный выход ({ $percent }): { $amount }
positions-event-partial-exit-submitted = Отправлен частичный выход: { $amount }
positions-event-sold-percent-for = Продано { $amount } ({ $percent }) за { $sol }
positions-event-sold-percent = Продано { $amount } ({ $percent })
positions-event-sold-for = Продано { $amount } за { $sol }
positions-event-sold = Продано: { $amount }
positions-event-exit-submitted = Отправлен полный выход из позиции
positions-event-exit-for = Закрыто: продано { $amount } за { $sol }
positions-event-exit-closed = Позиция закрыта
positions-event-wallet-bought = Кошелёк купил { $amount } в другом месте
positions-event-wallet-sold = Кошелёк продал { $amount } в другом месте
positions-event-received = Получено: { $amount }
positions-event-sent = Отправлено: { $amount }
positions-event-transferred = Переведено: { $amount }
positions-event-ata = Активность токен-аккаунта
positions-event-wallet-transaction = Транзакция кошелька с { $amount }
positions-event-price-per-token = { $price } { -sol } / токен
positions-event-wallet-change = изменение кошелька: { $amount }
positions-event-after-title = Позиция после этого события
positions-event-capital-invested = Вложенный капитал
positions-event-average-entry = Средний вход
positions-event-transfers-title = Переводы токенов
positions-event-transfer-amount = Сумма
positions-event-transfer-mint = Минт
positions-event-transfer-from = Откуда
positions-event-transfer-to = Куда
positions-event-no-signature = Нет подписи в блокчейне
positions-event-click-to-copy = Нажмите, чтобы скопировать
positions-event-solscan = { -solscan }
positions-event-token-amount = Количество токенов
positions-event-trade-price = Цена сделки
positions-event-native-amount = Сумма в { -sol }
positions-event-cost-basis = Базовая стоимость
positions-event-usd-value = Стоимость в USD
positions-event-network-fee = Комиссия сети
positions-event-router = Роутер
positions-event-slot = Слот
positions-event-chain-status = Статус в блокчейне
positions-event-transaction-type = Тип транзакции
positions-event-direction = Направление
positions-event-wallet-native-change = Изменение { -sol } в кошельке
positions-event-instructions = Инструкции
positions-event-compute-units = Вычислительные единицы
positions-event-accounts = Аккаунты
positions-event-record-id = ID записи
positions-event-time-unavailable = Время недоступно
positions-event-details = Подробнее
positions-event-hide-details = Скрыть подробности

positions-chart-type-candles = Свечи
positions-chart-type-line = Линия
positions-chart-type-area = Область
positions-chart-type-group =
    .aria-label = Тип графика
positions-chart-overlays-group =
    .aria-label = Наложения на график
positions-chart-ema = EMA
    .title = Экспоненциальные скользящие средние, 9 и 21
positions-chart-fit = Вписать
    .title = Показать весь период жизни позиции
positions-chart-timeframes-group =
    .aria-label = Таймфрейм
positions-chart-pane-group =
    .aria-label = Панель графика
positions-chart-unavailable = Движок графика недоступен
positions-chart-collecting = Сбор данных графика…
positions-chart-no-data = Данных графика по этому токену пока нет
positions-chart-avg-entry = Сред. вход
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = Сред. вход
positions-chart-legend-avg-entry-off-scale = Сред. вход (вне шкалы)
positions-chart-dropped-events =
    { $count ->
        [one] { $count } событие без свечи на этом таймфрейме
        [few] { $count } события без свечи на этом таймфрейме
        [many] { $count } событий без свечи на этом таймфрейме
       *[other] { $count } события без свечи на этом таймфрейме
    }
positions-chart-level = Уровень
positions-chart-level-above = { $label } { $price } выше этого диапазона
positions-chart-level-below = { $label } { $price } ниже этого диапазона
positions-chart-scale-hint = Потяните ценовую ось, чтобы приблизиться к уровню
positions-chart-pnl-at-bar = P&L на баре
positions-chart-click-to-locate = Нажмите, чтобы найти
