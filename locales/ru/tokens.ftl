tokens-result-source-live = Рыночные данные в реальном времени
tokens-result-source-unavailable = { $label } недоступен — повторяем попытку
tokens-result-source-not-listed = Нет в списке { $label }
tokens-result-security-available = Отчёт безопасности доступен
tokens-result-security-missing = Нет отчёта { -rugcheck }
tokens-result-chart-available = Данные графика доступны
tokens-result-chart-missing = Данных графика пока нет

tokens-state-error-title = Не удалось загрузить данные
tokens-state-offline = Похоже, вы не в сети.
tokens-state-request-failed = Запрос не удался после нескольких попыток.
tokens-state-waiting = Ожидаем данные…

tokens-chart-marker-entry = Вход
tokens-chart-level-stop-loss = Стоп-лосс
tokens-chart-level-take-profit = Тейк-профит

tokens-transactions-loading = Загрузка транзакций…
tokens-transactions-empty-title = Нет транзакций
tokens-transactions-empty-history = Для этого токена нет истории транзакций кошелька.
tokens-transactions-empty-data = Для этого токена нет данных о транзакциях.
tokens-transactions-error-title = Не удалось загрузить транзакции
tokens-transactions-error-message = История транзакций временно недоступна.
tokens-transactions-activity-title = Активность за 24 ч
tokens-transactions-activity-subtitle = Транзакции кошелька по часам
tokens-transactions-metric-total = Всего
tokens-transactions-metric-buys = Покупки
tokens-transactions-metric-sells = Продажи
tokens-transactions-recent-title = Последние транзакции
tokens-transactions-shown = Показано: { $count }
tokens-transactions-column-time = Время
tokens-transactions-column-type = Тип
tokens-transactions-column-price = Цена ({ -sol })
tokens-transactions-column-total = Итого ({ -sol })
tokens-transactions-chart-missing = Библиотека графиков отсутствует
tokens-transactions-view-solscan = Открыть транзакцию в { -solscan }

tokens-positions-empty-title = Нет позиции
tokens-positions-no-token = Токен не выбран.
tokens-positions-empty-message = По этому токену пока нет позиции. Нажмите «Купить», чтобы открыть её.
tokens-positions-loading = Загрузка позиции…
tokens-positions-from-wallet-history = Из истории кошелька
tokens-positions-frozen = Заморожен — продать нельзя
tokens-positions-no-cost-basis = Нет базовой стоимости
tokens-positions-history-incomplete = История неполная
tokens-positions-dca-count = DCA: { $count }
tokens-positions-exit-count = Выходы: { $count }
tokens-positions-fact-avg-entry = Сред. вход
tokens-positions-fact-current = Сейчас
tokens-positions-fact-tokens = Токены
tokens-positions-fact-opened = Открыта
tokens-positions-fact-exit-price = Цена выхода
tokens-positions-fact-native-received = Получено { -sol }
tokens-positions-fact-closed-reason = Причина закрытия
tokens-positions-fact-target-min = Мин. цель прибыли
tokens-positions-fact-target-max = Макс. цель прибыли
tokens-positions-fact-highest = Наивысшая цена
tokens-positions-fact-lowest = Наименьшая цена
tokens-positions-section-range = Цели и диапазон
tokens-positions-section-market = Рынок и активы
tokens-positions-kicker = Позиция
tokens-positions-fallback-symbol = Токен
tokens-positions-realized-pnl = Реализованный P&L
tokens-positions-unrealized-pnl = Нереализованный P&L
tokens-positions-size = Размер

tokens-security-analysis-pending = Идёт анализ { -rugcheck }...
tokens-security-analyzing = Анализ безопасности…
tokens-security-pulse-title = Пульс безопасности
tokens-security-pending-caption = Сигналы риска ещё собираются.
tokens-security-control-title = Контроль токена
tokens-security-control-meta = Статус полномочий
tokens-security-updated = Обновлено { $time }
tokens-security-score-caption = Нормализованная оценка риска токена из 100.
tokens-security-score-label = Оценка
tokens-security-rugged = Рагпулл
tokens-security-grade-analyzing = Анализ
tokens-security-grade-shielded = Защищён
tokens-security-grade-safe = Безопасен
tokens-security-grade-caution = Осторожно
tokens-security-grade-vulnerable = Уязвим
tokens-security-grade-unknown = Неизвестно
tokens-security-metric-token-type = Тип токена
tokens-security-metric-total-holders = Всего холдеров
tokens-security-metric-lp-providers = Поставщики LP
tokens-security-metric-graph-insiders = Инсайдеры по графу
tokens-security-insiders-detected = Обнаружены ({ $count })
tokens-security-insiders-clean = Не обнаружены
tokens-security-authority-mint = Mint
tokens-security-authority-freeze = Freeze
tokens-security-authority-immutable = Неизменяемый
tokens-security-authority-mutable = Изменяемый
tokens-security-authority-revoked = Отозваны
tokens-security-authority-active = Активны
tokens-security-holder-health-title = Состояние холдеров
tokens-security-holders-unique = уникальных
tokens-security-creator-share = Доля создателя
tokens-security-gauge-top-10 = Топ-10
tokens-security-concentration-unknown = Неизвестно
tokens-security-concentration-critical = Критическая
tokens-security-concentration-high = Высокая
tokens-security-concentration-moderate = Умеренная
tokens-security-concentration-healthy = Здоровая
tokens-security-transfer-title = Комиссия за перевод
tokens-security-transfer-no-fee = Без комиссии
tokens-security-transfer-fee-percentage = Размер комиссии
tokens-security-transfer-max-fee = Макс. сумма комиссии
tokens-security-transfer-authority = Управляющий комиссией
tokens-security-transfer-note = С каждого перевода взимается комиссия { $percent }.
tokens-security-transfer-none = Комиссий за перевод не обнаружено.
tokens-security-risks-title = Риски безопасности
tokens-security-risks-none = Рисков безопасности не обнаружено.
tokens-security-risk-fallback-name = Сигнал безопасности
tokens-security-risks-critical = Критических: { $count }
tokens-security-risks-warnings =
    { $count ->
        [one] { $count } предупреждение
        [few] { $count } предупреждения
        [many] { $count } предупреждений
       *[other] { $count } предупреждения
    }
tokens-security-risks-info = Информационных: { $count }
tokens-security-risks-incidents =
    { $count ->
        [one] Найден { $count } инцидент
        [few] Найдено { $count } инцидента
        [many] Найдено { $count } инцидентов
       *[other] Найдено { $count } инцидента
    }
tokens-security-top-holders-title = Крупнейшие холдеры
tokens-security-top-holders-concentration = Концентрация { $percent }
tokens-security-insider = Инсайдер

tokens-overview-chart-checking = Проверяем данные…
tokens-overview-banner-open = Открыть баннер токена
tokens-overview-headline-label = Основные рыночные показатели
tokens-overview-price = Цена
tokens-overview-market-cap = Капитализация
tokens-overview-liquidity = Ликвидность
tokens-overview-volume = Объём
tokens-overview-volume-24h = Объём 24 ч
tokens-overview-no-tags = Нет тегов
tokens-overview-info-title = О токене
tokens-overview-profile = Опубликованный профиль
tokens-overview-fact-mint = Минт
tokens-overview-fact-decimals = Знаков после запятой
tokens-overview-fact-age = Возраст
tokens-overview-fact-dex = DEX
tokens-overview-fact-holders = Холдеры
tokens-overview-fact-top-10 = Доля топ-10
tokens-overview-tags = Теги
tokens-overview-liquidity-title = Ликвидность и рынок
tokens-overview-fact-fdv = FDV
tokens-overview-fact-pool-native = { -sol } в пуле
tokens-overview-fact-pool-token = Токен в пуле
tokens-overview-pool = Пул
tokens-overview-pulse-title = Пульс рынка
tokens-overview-activity-title = Активность транзакций
tokens-overview-buy-share = Покупки: { $percent }
tokens-overview-buy-sell-ratio = { $ratio } П/П
tokens-overview-buys-24h = Покупки 24 ч
tokens-overview-sells-24h = Продажи 24 ч
tokens-overview-net-flow = Чистый поток
tokens-overview-total-24h = Всего за 24 ч
tokens-overview-average-24h = Сред. за 24 ч
tokens-overview-spike-5m = Всплеск 5 мин
tokens-overview-rate-per-hour = { $amount }/ч
tokens-overview-rate-per-minute = { $amount }/мин
tokens-overview-spike-factor = { $factor }×
tokens-overview-flow-counts = Покупки: { $buys } ({ $buyPercent }), продажи: { $sells } ({ $sellPercent }), всего: { $total }
tokens-overview-flow-no-data = Нет данных о транзакциях

tokens-pools-empty-title = Нет пулов
tokens-pools-empty-message = Для этого токена не обнаружено пулов ликвидности.
tokens-pools-unknown = Неизвестно
tokens-pools-unknown-dex = Неизвестный DEX
tokens-pools-liquidity = Ликвидность
tokens-pools-volume-24h = Объём за 24 ч
tokens-pools-base-role = Роль базового токена
tokens-pools-quote-role = Роль котируемого токена
tokens-pools-canonical-title = Основной пул
tokens-pools-canonical = Основной
tokens-pools-dex = DEX
tokens-pools-summary-title = Сводка по пулам
tokens-pools-breakdown-title = Разбивка по DEX
tokens-pools-all-title = Все пулы
tokens-pools-updated = Обновлено
tokens-pools-role-base = Базовый
tokens-pools-role-quote = Котируемый
tokens-pools-role-unknown = Неизвестно
tokens-pools-reserves = Резервные аккаунты
tokens-pools-no-reserves = Нет резервных аккаунтов
tokens-pools-address-pool = Пул
    .title = Копировать пул
tokens-pools-address-base = Базовый минт
    .title = Копировать базовый минт
tokens-pools-address-quote = Котируемый минт
    .title = Копировать котируемый минт
    .title = Копировать парный минт

tokens-links-empty = Для этого токена нет официального сайта и ссылок на соцсети.
tokens-links-info-title = О токене
tokens-links-mint-address = Адрес минта
tokens-links-data-source = Источник данных
tokens-links-security = Безопасность
tokens-links-profile-title = Профиль токена
tokens-links-profile-published-title = Содержимое опубликованного профиля
tokens-links-profile-published-note = Медиа, описание и официальные ссылки — платное содержимое профиля, проверяемое перед публикацией. Оно не подтверждает право собственности или безопасность токена.
tokens-links-profile-create-note = Добавьте в публичный профиль этого токена проверенный логотип, описание проекта и официальные ссылки.
tokens-links-profile-update-hint = Обновить профиль этого токена на screenerbot.io
tokens-links-profile-create-hint = Создать профиль токена на screenerbot.io
tokens-links-profile-update = Обновить профиль
tokens-links-profile-create = Создать профиль
tokens-links-media-title = Медиафайлы
tokens-links-media-fallback-symbol = Токен
tokens-links-media-logo = Логотип
tokens-links-media-banner = Баннер
tokens-links-media-banner-alt = Баннер { $symbol }
tokens-links-media-open = Открыть изображение
tokens-links-description-title = Описание
tokens-links-explorers-title = Обозреватели и аналитика
tokens-links-websites-title = Официальные сайты
tokens-links-socials-title = Соцсети
tokens-links-explorer-solana-explorer = { -solana-explorer }
tokens-links-explorer-geckoterminal = { -geckoterminal }
tokens-links-explorer-dextools = { -dextools }
tokens-links-explorer-coingecko = { -coingecko }
tokens-links-explorer-jupiter-swap = { -jupiter } Swap
tokens-links-social-twitter = { -twitter } / { -x }
tokens-links-social-x = { -x } ({ -twitter })
tokens-links-social-telegram = { -telegram }
tokens-links-social-discord = { -discord }
tokens-links-social-medium = { -medium }
tokens-links-social-github = { -github }
tokens-links-social-youtube = { -youtube }
tokens-links-social-reddit = { -reddit }
tokens-links-social-facebook = { -facebook }
tokens-links-social-instagram = { -instagram }
tokens-links-social-linkedin = { -linkedin }
tokens-links-social-tiktok = { -tiktok }
tokens-links-social-fallback = Соцсеть

tokens-dialog-tab-overview = Обзор
tokens-dialog-tab-security = Безопасность
tokens-dialog-tab-positions = Позиции
tokens-dialog-tab-pools = Пулы
tokens-dialog-tab-links = Ссылки
tokens-dialog-tab-transactions = Транзакции
tokens-dialog-sections = Разделы сведений о токене
tokens-dialog-close =
    .title = Закрыть (ESC)
    .aria-label = Закрыть сведения о токене
tokens-dialog-unknown-symbol = Неизвестно
tokens-dialog-unknown-name = Неизвестный токен
tokens-dialog-market-summary = Рыночная сводка
tokens-dialog-price-loading = Загрузка цены
tokens-dialog-unit-native = { -sol }
tokens-dialog-market-metrics = Рыночные показатели
tokens-dialog-metric-market-cap = Капитализация
tokens-dialog-metric-volume-24h = Объём за 24 ч
tokens-dialog-change-24h = Изменение за 24 часа: { $change }
tokens-dialog-buy = Купить
    .title = Купить этот токен
tokens-dialog-sell = Продать
    .title = Продать позицию
tokens-dialog-sell-unavailable = Нет открытой позиции для продажи
tokens-dialog-details = Подробности
tokens-dialog-sources = Источники
tokens-dialog-sources-status = Статус источников данных
tokens-dialog-updated-label = Обновлено
tokens-dialog-just-now = Только что
tokens-dialog-updated-at = Обновлено { $time }
tokens-dialog-updated-unavailable = Время обновления недоступно
tokens-dialog-error-title = Не удалось загрузить данные токена
tokens-dialog-waiting-token = Ожидаем данные токена…
tokens-dialog-loading-overview = Загрузка обзора…
tokens-dialog-loading-security = Загрузка данных безопасности…
tokens-dialog-loading-pools = Загрузка пулов…
tokens-dialog-loading-links = Загрузка ссылок…
tokens-dialog-chart-still-checking = Данных графика пока нет — продолжаем проверку…
tokens-dialog-no-data = Нет данных
tokens-dialog-source-token = Токен
tokens-dialog-source-market = Рынок
tokens-dialog-source-security = Безопасность
tokens-dialog-source-chart = График
tokens-dialog-status-pending = Ожидание
tokens-dialog-status-loading = Загрузка
tokens-dialog-status-ready = Готово
tokens-dialog-status-unavailable = Недоступно
tokens-dialog-status-cached = Из кэша
tokens-dialog-source-summary = { $source }: { $status }
tokens-dialog-badge-pool-price = Цена пула
tokens-dialog-badge-pool-price-hint = Цена из пула в блокчейне в реальном времени
tokens-dialog-badge-api-price = Цена API
tokens-dialog-badge-api-price-hint = Цена из кэшированных рыночных данных (API)
tokens-dialog-badge-profile = Опубликованный профиль
    .title = Платное содержимое профиля, проверенное перед публикацией; не является аудитом или подтверждением права собственности.
tokens-dialog-badge-low-risk-hint = Низкий риск по текущей оценке { -rugcheck }; это не подтверждение личности.
tokens-dialog-badge-immutable = Неизменяемый
tokens-dialog-badge-mutable = Изменяемый
tokens-dialog-badge-position = Позиция
tokens-dialog-badge-blacklisted = В чёрном списке

tokens-view-favorites = Избранное
tokens-view-pool = Сервис пулов
tokens-view-no-market = Нет рыночных данных
tokens-view-all = Все токены
tokens-view-passed = Пройдены
tokens-view-rejected = Отклонены
tokens-view-blacklisted = В чёрном списке
tokens-view-positions = Позиции
tokens-view-recent = Недавние
tokens-view-ohlcv = Данные OHLCV
# Empty token table per view (TOKEN_VIEW_EMPTY_LABELS)
tokens-view-pool-empty = Токенов с ценой пока нет
    .message = Токены появляются здесь, когда проходят фильтрацию и цена их пула рассчитана.
tokens-view-no-market-empty = Нет токенов без рыночных данных
    .message = Токены находятся здесь, пока источники рыночных данных их не добавили.
tokens-view-all-empty = Токены ещё не обнаружены
    .message = Здесь появляется каждый найденный токен, независимо от результата фильтрации.
tokens-view-passed-empty = Ни один токен не прошёл фильтрацию
    .message = Здесь появляются токены, прошедшие все активные фильтры. Проверьте страницу «Фильтрация», если список остаётся пустым.
tokens-view-rejected-empty = Нет отклонённых токенов
    .message = Токены, не прошедшие фильтр, появляются здесь с указанием причины.
tokens-view-blacklisted-empty = Нет токенов в чёрном списке
    .message = Здесь появляются токены, исключённые из торговли вами или проверками безопасности.
tokens-view-positions-empty = Нет токенов в позициях
    .message = Здесь появляются токены из открытых позиций.
tokens-view-recent-empty = Нет новых токенов
    .message = Недавно обнаруженные токены появляются здесь по мере нахождения.
tokens-ohlcv-empty = Данных графиков пока нет
    .message = Токены появляются здесь, когда для них начинается сбор свечей.

tokens-cell-logo-enlarge = Нажмите, чтобы увеличить
tokens-boost-title = Буст на screenerbot.io: { $boosts }
tokens-cell-action-add =
    .title = Докупить к позиции (DCA)
    .aria-label = Докупить к позиции
tokens-cell-action-sell =
    .title = Продать (полностью или часть в %)
    .aria-label = Продать токен
tokens-cell-action-buy =
    .title = Купить позицию
    .aria-label = Купить токен
tokens-cell-external-links =
    .title = Внешние ссылки
    .aria-label = Внешние ссылки

tokens-table-loading-title = Загрузка токенов…
tokens-table-loading-description = Подготавливаем выбранное представление токенов.
tokens-table-retry-hint = Переключите вкладку или повторите попытку.
tokens-filter-all = Все

tokens-favorites-load-failed-title = Не удалось загрузить избранное
tokens-favorites-load-failed-toast = Не удалось загрузить избранное
tokens-favorites-total = Всего в избранном
tokens-favorites-empty-title = Избранного пока нет
    .message = Отметьте токен звездой в любом списке, чтобы он оставался здесь.

tokens-column-token = Токен
tokens-column-status = Статус
tokens-ohlcv-delete =
    .title = Удалить данные OHLCV
    .aria-label = Удалить данные OHLCV
tokens-ohlcv-status-active = Активен
tokens-ohlcv-status-inactive = Неактивен
tokens-ohlcv-priority-critical = Критический
tokens-ohlcv-priority-high = Высокий
tokens-ohlcv-priority-medium = Средний
tokens-ohlcv-priority-low = Низкий
tokens-ohlcv-column-priority = Приоритет
tokens-ohlcv-column-backfill = Дозагрузка
tokens-ohlcv-column-data-span = Охват данных
tokens-ohlcv-column-gaps = Пропуски
tokens-ohlcv-column-pools = Пулы
tokens-ohlcv-column-last-fetch = Последняя загрузка
tokens-ohlcv-timeframe-complete = { $timeframe }: завершено
tokens-ohlcv-timeframe-pending = { $timeframe }: ожидание
tokens-ohlcv-load-failed-title = Не удалось загрузить данные OHLCV
tokens-ohlcv-load-failed-toast = Не удалось загрузить данные OHLCV
tokens-ohlcv-total = Всего токенов
tokens-ohlcv-active = Активные
tokens-ohlcv-db-size = Размер БД
tokens-ohlcv-cleanup = Очистить неактивные
tokens-ohlcv-delete-title = Удалить данные OHLCV
tokens-ohlcv-delete-token-message = Удалить все данные OHLCV для { $token }?
tokens-ohlcv-delete-done =
    Удалено: { $candles ->
        [one] { $candles } свеча
        [few] { $candles } свечи
        [many] { $candles } свечей
       *[other] { $candles } свечи
    }, { $pools ->
        [one] { $pools } пул
        [few] { $pools } пула
        [many] { $pools } пулов
       *[other] { $pools } пула
    }
tokens-ohlcv-delete-failed = Не удалось удалить данные OHLCV
tokens-ohlcv-cleanup-title = Удалить неактивные токены
tokens-ohlcv-cleanup-message = Удалить неактивные токены старше указанного числа часов
tokens-ohlcv-cleanup-placeholder = Часы...
tokens-ohlcv-cleanup-invalid = Введите положительное число
tokens-ohlcv-cleanup-done =
    Очищено: { $count ->
        [one] { $count } неактивный токен
        [few] { $count } неактивных токена
        [many] { $count } неактивных токенов
       *[other] { $count } неактивного токена
    }
tokens-ohlcv-cleanup-failed = Не удалось очистить данные OHLCV

tokens-summary-total = Всего
tokens-summary-pool-priced = С ценой пула
tokens-summary-positions = Позиции
tokens-summary-blacklisted = В чёрном списке
tokens-search-placeholder = Поиск по символу или минту...
tokens-table-waiting-title = Токены ещё загружаются...
tokens-table-waiting-description = Ожидаем ответа от бэкенда. Попытка повторится автоматически.
tokens-load-failed-toast = Не удалось загрузить токены
tokens-row-data-missing = Данные токена не найдены
tokens-column-price-sol = Цена ({ -sol })
tokens-column-liquidity = Ликвидность
tokens-column-volume-24h = Объём 24 ч
tokens-column-fdv = FDV
tokens-column-market-cap = Капит.
tokens-column-change-1h = 1 ч
tokens-column-change-24h = 24 ч
tokens-column-txns-5m = Транз. 5 мин
tokens-column-txns-1h = Транз. 1 ч
tokens-column-txns-6h = Транз. 6 ч
tokens-column-txns-24h = Транз. 24 ч
tokens-column-risk-score = Оценка риска
tokens-column-reject-reason = Причина отклонения
tokens-column-blacklist-reason = Причина занесения в чёрный список
tokens-column-updated = Обновлено
tokens-column-birth = Создан
tokens-column-first-seen = Впервые замечен
tokens-badge-price = Цена
tokens-badge-ohlcv = OHLCV
tokens-badge-position = Позиция
tokens-badge-blacklisted = В чёрном списке
tokens-badge-blacklisted-title = Токен в чёрном списке
tokens-badge-blacklisted-reasons = В чёрном списке: { $reasons }
tokens-links-menu-copy-mint = Копировать минт
tokens-links-copy-failed = Не удалось скопировать минт
tokens-lightbox-token-age = Возраст токена

tokens-search-placeholder-dialog = Поиск по названию, символу или минту...
tokens-search-input-label = Поиск токенов
tokens-search-results-label = Результаты поиска
tokens-search-tip-nav = навигация
tokens-search-tip-open = открыть
tokens-search-tip-close = закрыть
tokens-search-failed = Не удалось выполнить поиск
tokens-search-error = Ошибка: { $message }
tokens-search-clear =
    .title = Очистить поиск
    .aria-label = Очистить поиск
tokens-search-recent = Недавние
tokens-search-recent-label = Недавние поиски
tokens-search-lists-label = Списки токенов
tokens-search-tab-trending = В тренде
tokens-search-kinds = Название · символ · минт
tokens-search-empty-trending = Токены в тренде появятся, когда бот рассчитает цены первых пулов.
tokens-search-empty-positions = Сейчас нет открытых позиций.
tokens-search-empty-favorites = Отметьте токен звёздочкой, и он будет ждать здесь при следующем поиске.
tokens-search-empty-boosted = Сейчас нет токенов с бустом.
tokens-search-list-failed = Не удалось загрузить этот список.
tokens-search-searching = Поиск по рынкам…
# $count is the number of tokens found.
tokens-search-result-count =
    { $count ->
        [one] { $count } результат
        [few] { $count } результата
        [many] { $count } результатов
       *[other] { $count } результата
    }
tokens-search-order = Сначала лучшие совпадения, затем объём за 24 ч
tokens-search-metric-mc = Кап.
    .title = Рыночная капитализация
tokens-search-metric-fdv = FDV
    .title = Полностью разводнённая оценка
tokens-search-metric-liq = Ликв.
    .title = Ликвидность
tokens-search-metric-vol = Объём
    .title = Объём за 24 ч
tokens-search-more =
    .title = Другие действия
    .aria-label = Другие действия
# $query is the text the user typed.
tokens-search-no-match = Нет токенов, соответствующих «{ $query }».

tokens-featured-category-boosted = С бустом
tokens-featured-category-jupiter-organic = { -jupiter }: топ органики
tokens-featured-category-jupiter-traded = { -jupiter }: топ по торгам
tokens-featured-category-dexscreener-trending = { -dexscreener }: в тренде
tokens-featured-source-jupiter = { -jupiter }
tokens-featured-source-dexscreener = { -dexscreener }
tokens-featured-note-boosted = Продвигаются своими командами
tokens-featured-security-risky = Рискованный
tokens-featured-load-failed = Не удалось загрузить рекомендуемые
tokens-featured-network-error = Ошибка сети: { $message }
tokens-featured-title = Рекомендуемые
tokens-featured-subtitle = Сначала токены с бустом, затем тренды по всей Solana
tokens-featured-boost = Буст токена
tokens-featured-close =
    .title = Закрыть (ESC)
tokens-featured-loading = Загрузка рекомендуемых и трендовых...
tokens-featured-error-hint = Проверьте подключение или повторите попытку
tokens-featured-empty = Сейчас нет доступных токенов
tokens-featured-count =
    { $count ->
        [one] { $count } токен
        [few] { $count } токена
        [many] { $count } токенов
       *[other] { $count } токена
    }
tokens-featured-stat-market-cap = Капитализация
tokens-featured-stat-liquidity = Ликвидность
tokens-featured-stat-volume = Объём 24 ч
tokens-featured-stat-holders = Холдеры
tokens-featured-stat-txns = Транз. 24 ч
tokens-featured-buy = Купить
    .title = Купить { $symbol }
tokens-featured-security-score = Оценка безопасности: { $score }/100
tokens-featured-social-website = Сайт
tokens-featured-social-twitter = { -twitter }

tokens-featured-row-view-all = Все
    .title = Открыть полный список рекомендуемых
tokens-featured-row-scroll-start =
    .aria-label = Показать предыдущие токены
tokens-featured-row-scroll-end =
    .aria-label = Показать ещё токены
tokens-featured-row-empty = Нет рекомендуемых токенов
tokens-featured-row-title = { $name } ({ $symbol })
tokens-featured-row-boosted-title = { $name } ({ $symbol }) — буст: { $boosts }

tokens-pool-selector-title = Выберите пул
tokens-pool-selector-loading = Загрузка пулов...
tokens-pool-selector-empty = Для этого токена пулов не найдено
tokens-pool-selector-load-failed = Не удалось загрузить пулы: { $message }
tokens-pool-selector-count =
    { $count ->
        [one] Найден { $count } пул
        [few] Найдено { $count } пула
        [many] Найдено { $count } пулов
       *[other] Найдено { $count } пула
    }
tokens-pool-selector-liquidity = { $amount } ликв.
    .title = Ликвидность
tokens-pool-selector-volume = { $amount } за 24 ч
    .title = Объём за 24 ч

tokens-identity-unknown-asset = Неизвестный актив
tokens-identity-copy-address =
    .title = Копировать адрес
    .aria-label = Копировать адрес
tokens-identity-copy-signature =
    .title = Копировать подпись
    .aria-label = Копировать подпись

tokens-rugcheck-risk-single-holder-ownership = Один крупный холдер
    .description = Один холдер владеет большой долей предложения токена.
tokens-rugcheck-risk-low-liquidity = Низкая ликвидность
    .description = В пуле токена мало ликвидности.
tokens-rugcheck-risk-few-lp-providers = Мало поставщиков LP
    .description = Ликвидность предоставляют лишь несколько пользователей.
tokens-rugcheck-risk-high-holder-concentration = Высокая концентрация холдеров
    .description = Топ-10 холдеров владеют более чем 50 % предложения токена.
tokens-rugcheck-risk-top-10-holders-high-ownership = Большая доля топ-10 холдеров
    .description = Топ-10 холдеров владеют более чем 70 % предложения токена.
tokens-rugcheck-risk-high-ownership = Высокая доля владения
    .description = Крупнейшие холдеры владеют более чем 80 % предложения токена.
tokens-rugcheck-risk-creator-rug-history = Создатель с историей рагпуллов
    .description = Создатель уже устраивал рагпуллы с токенами.
tokens-rugcheck-risk-large-lp-unlocked = Большая часть LP разблокирована
    .description = Большая часть LP-токенов разблокирована, и владелец может вывести ликвидность в любой момент.
tokens-rugcheck-risk-mutable-metadata = Изменяемые метаданные
    .description = Владелец может изменить метаданные токена.
tokens-rugcheck-risk-few-holders = Мало холдеров
    .description = Токен держит немного кошельков.
tokens-rugcheck-risk-copycat-token = Токен-подражатель
    .description = Этот токен использует символ верифицированного токена.
tokens-rugcheck-risk-fee-config-enabled = Настраиваемые комиссии
    .description = Владелец может изменить комиссии в любой момент.
tokens-rugcheck-risk-high-holder-correlation = Высокая корреляция холдеров
    .description = Крупнейшие холдеры держат схожие объёмы предложения.
tokens-rugcheck-risk-freeze-authority-enabled = Полномочия freeze активны
    .description = Токены могут быть заморожены и исключены из торговли.
tokens-rugcheck-risk-mint-authority-enabled = Полномочия mint активны
    .description = Владелец может выпустить больше токенов.
tokens-rugcheck-risk-missing-file-metadata = Нет файла метаданных
    .description = С этим токеном не связан файл метаданных.
tokens-rugcheck-risk-high-market-cap-per-holder = Высокая капитализация на холдера
    .description = Капитализация очень высока относительно числа холдеров.
tokens-rugcheck-risk-symbol-mismatch = Несовпадение символа
    .description = Символ токена не совпадает с его файлом метаданных.
tokens-rugcheck-risk-name-mismatch = Несовпадение названия
    .description = Название токена не совпадает с его файлом метаданных.
tokens-rugcheck-risk-permanent-control-enabled = Постоянный контроль включён
    .description = Создатель токена может навсегда контролировать все токены.
tokens-rugcheck-risk-missing-metadata = Нет метаданных
    .description = Метаданные для этого токена не найдены.
tokens-rugcheck-risk-lp-unlock-soon = Скорая разблокировка LP
    .description = LP-токены скоро разблокируются, и владелец сможет вывести ликвидность.
tokens-rugcheck-risk-lp-vault-unlocked = Хранилище LP разблокировано
    .description = LP-токены из хранилища можно вернуть.
tokens-rugcheck-risk-mint-authority-locked = Полномочия mint заблокированы
    .description = Выпуск новых токенов заблокирован.
tokens-rugcheck-risk-high-transfer-fee = Высокая комиссия за перевод
    .description = С каждого перевода этого токена взимается высокий налог.
