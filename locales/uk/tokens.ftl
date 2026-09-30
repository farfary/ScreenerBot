tokens-result-source-live = Ринкові дані наживо
tokens-result-source-unavailable = { $label } недоступний — повторна спроба
tokens-result-source-not-listed = Немає в переліку { $label }
tokens-result-security-available = Звіт про безпеку доступний
tokens-result-security-missing = Немає звіту { -rugcheck }
tokens-result-chart-available = Дані графіка доступні
tokens-result-chart-missing = Даних графіка ще немає

tokens-state-error-title = Не вдалося завантажити дані
tokens-state-offline = Схоже, ви офлайн.
tokens-state-request-failed = Запит не вдався після кількох спроб.
tokens-state-waiting = Очікування даних…

tokens-chart-marker-entry = Вхід
tokens-chart-level-stop-loss = Стоп-лос
tokens-chart-level-take-profit = Тейк-профіт

tokens-transactions-loading = Завантаження транзакцій…
tokens-transactions-empty-title = Немає транзакцій
tokens-transactions-empty-history = Історії транзакцій гаманця для цього токена немає.
tokens-transactions-empty-data = Даних про транзакції для цього токена немає.
tokens-transactions-error-title = Не вдалося завантажити транзакції
tokens-transactions-error-message = Історія транзакцій тимчасово недоступна.
tokens-transactions-activity-title = Активність за 24 год
tokens-transactions-activity-subtitle = Транзакції гаманця по годинах
tokens-transactions-metric-total = Разом
tokens-transactions-metric-buys = Купівлі
tokens-transactions-metric-sells = Продажі
tokens-transactions-recent-title = Останні транзакції
tokens-transactions-shown = Показано: { $count }
tokens-transactions-column-time = Час
tokens-transactions-column-type = Тип
tokens-transactions-column-price = Ціна
tokens-transactions-column-total = Разом
tokens-transactions-chart-missing = Бібліотеку графіків не знайдено
tokens-transactions-view-solscan = Переглянути транзакцію в { -solscan }

tokens-positions-empty-title = Немає позиції
tokens-positions-no-token = Токен не вибрано.
tokens-positions-empty-message = Позиції за цим токеном ще немає. Натисніть «Купити», щоб відкрити її.
tokens-positions-loading = Завантаження позиції…
tokens-positions-from-wallet-history = З історії гаманця
tokens-positions-frozen = Заморожено — продати неможливо
tokens-positions-no-cost-basis = Немає собівартості
tokens-positions-history-incomplete = Історія неповна
tokens-positions-dca-count = DCA { $count }
tokens-positions-exit-count = Виходи { $count }
tokens-positions-fact-avg-entry = Сер. вхід
tokens-positions-fact-current = Поточна
tokens-positions-fact-tokens = Токени
tokens-positions-fact-opened = Відкрито
tokens-positions-fact-exit-price = Ціна виходу
tokens-positions-fact-sol-received = Отримано { -sol }
tokens-positions-fact-closed-reason = Причина закриття
tokens-positions-fact-target-min = Мін. ціль прибутку
tokens-positions-fact-target-max = Макс. ціль прибутку
tokens-positions-fact-highest = Найвища ціна
tokens-positions-fact-lowest = Найнижча ціна
tokens-positions-section-range = Цілі та діапазон
tokens-positions-section-market = Ринок і активи
tokens-positions-kicker = Позиція
tokens-positions-fallback-symbol = Токен
tokens-positions-realized-pnl = Реалізований P&L
tokens-positions-unrealized-pnl = Нереалізований P&L
tokens-positions-size = Розмір

tokens-security-analysis-pending = Триває аналіз { -rugcheck }...
tokens-security-analyzing = Аналіз безпеки…
tokens-security-pulse-title = Пульс безпеки
tokens-security-pending-caption = Сигнали ризику ще збираються.
tokens-security-control-title = Контроль токена
tokens-security-control-meta = Стан повноважень
tokens-security-updated = Оновлено { $time }
tokens-security-score-caption = Нормалізована оцінка ризику токена зі 100.
tokens-security-score-label = Оцінка
tokens-security-rugged = Рагпул
tokens-security-grade-analyzing = Аналіз
tokens-security-grade-shielded = Захищений
tokens-security-grade-safe = Безпечний
tokens-security-grade-caution = Обережно
tokens-security-grade-vulnerable = Вразливий
tokens-security-grade-unknown = Невідомо
tokens-security-metric-token-type = Тип токена
tokens-security-metric-total-holders = Усього холдерів
tokens-security-metric-lp-providers = Провайдери LP
tokens-security-metric-graph-insiders = Інсайдери графа
tokens-security-insiders-detected = Виявлено ({ $count })
tokens-security-insiders-clean = Немає
tokens-security-authority-mint = Мінт
tokens-security-authority-freeze = Заморожування
tokens-security-authority-immutable = Незмінний
tokens-security-authority-mutable = Змінюваний
tokens-security-authority-revoked = Відкликано
tokens-security-authority-active = Активне
tokens-security-holder-health-title = Стан холдерів
tokens-security-holders-unique = унікальних
tokens-security-creator-share = Частка творця
tokens-security-gauge-top-10 = Топ-10
tokens-security-concentration-unknown = Невідомо
tokens-security-concentration-critical = Критична
tokens-security-concentration-high = Висока
tokens-security-concentration-moderate = Помірна
tokens-security-concentration-healthy = Здорова
tokens-security-transfer-title = Податок на переказ
tokens-security-transfer-no-fee = Без комісії
tokens-security-transfer-fee-percentage = Відсоток комісії
tokens-security-transfer-max-fee = Макс. сума комісії
tokens-security-transfer-authority = Повноваження комісії
tokens-security-transfer-note = З кожного переказу стягується комісія { $percent }.
tokens-security-transfer-none = Комісій за переказ не виявлено.
tokens-security-risks-title = Ризики безпеки
tokens-security-risks-none = Ризиків безпеки не виявлено.
tokens-security-risk-fallback-name = Сигнал безпеки
tokens-security-risks-critical = Критичних: { $count }
tokens-security-risks-warnings =
    { $count ->
        [one] { $count } попередження
        [few] { $count } попередження
        [many] { $count } попереджень
       *[other] { $count } попередження
    }
tokens-security-risks-info = Інформаційних: { $count }
tokens-security-risks-incidents =
    { $count ->
        [one] Знайдено { $count } інцидент
        [few] Знайдено { $count } інциденти
        [many] Знайдено { $count } інцидентів
       *[other] Знайдено { $count } інциденту
    }
tokens-security-top-holders-title = Найбільші холдери
tokens-security-top-holders-concentration = Концентрація { $percent }
tokens-security-insider = Інсайдер

tokens-overview-chart-checking = Перевірка даних…
tokens-overview-banner-open = Відкрити банер токена
tokens-overview-headline-label = Ключові ринкові показники
tokens-overview-price = Ціна
tokens-overview-market-cap = Ринкова капіталізація
tokens-overview-liquidity = Ліквідність
tokens-overview-volume = Обсяг
tokens-overview-volume-24h = Обсяг 24 год
tokens-overview-no-tags = Без тегів
tokens-overview-info-title = Інформація про токен
tokens-overview-profile = Опублікований профіль
tokens-overview-fact-mint = Мінт
tokens-overview-fact-decimals = Десяткові знаки
tokens-overview-fact-age = Вік
tokens-overview-fact-dex = DEX
tokens-overview-fact-holders = Холдери
tokens-overview-fact-top-10 = Топ-10 утримують
tokens-overview-tags = Теги
tokens-overview-liquidity-title = Ліквідність і ринок
tokens-overview-fact-fdv = FDV
tokens-overview-fact-pool-sol = Pool { -sol }
tokens-overview-fact-pool-token = Pool Token
tokens-overview-pool = Пул
tokens-overview-pulse-title = Пульс ринку
tokens-overview-activity-title = Активність транзакцій
tokens-overview-buy-share = Купівлі: { $percent }
tokens-overview-buy-sell-ratio = К/П { $ratio }
tokens-overview-buys-24h = Купівлі 24 год
tokens-overview-sells-24h = Продажі 24 год
tokens-overview-net-flow = Чистий потік
tokens-overview-total-24h = Разом 24 год
tokens-overview-average-24h = Сер. 24 год
tokens-overview-spike-5m = Сплеск 5 хв
tokens-overview-rate-per-hour = { $amount }/год
tokens-overview-rate-per-minute = { $amount }/хв
tokens-overview-spike-factor = { $factor }×
tokens-overview-flow-counts = Купівлі: { $buys } ({ $buyPercent }), продажі: { $sells } ({ $sellPercent }), разом: { $total }
tokens-overview-flow-no-data = Немає даних про транзакції

tokens-pools-empty-title = Немає пулів
tokens-pools-empty-message = Для цього токена не виявлено пулів ліквідності.
tokens-pools-unknown = Невідомо
tokens-pools-unknown-dex = Невідомий DEX
tokens-pools-total = Усього пулів
tokens-pools-liquidity = Ліквідність
tokens-pools-volume-24h = Обсяг за 24 год
tokens-pools-base-role = Базова роль
tokens-pools-quote-role = Котирувальна роль
tokens-pools-canonical-title = Канонічний пул
tokens-pools-canonical = Канонічний
tokens-pools-dex = DEX
tokens-pools-summary-title = Підсумок пулів
tokens-pools-breakdown-title = Розподіл за DEX
tokens-pools-all-title = Усі пули
tokens-pools-updated = Оновлено
tokens-pools-role-base = Базовий
tokens-pools-role-quote = Котирувальний
tokens-pools-role-unknown = Невідомо
tokens-pools-reserves = Резервні акаунти
tokens-pools-no-reserves = Немає резервних акаунтів
tokens-pools-address-copy = Копіювати адресу
tokens-pools-address-pool = Пул
    .title = Копіювати пул
tokens-pools-address-base = Базовий мінт
    .title = Копіювати базовий мінт
tokens-pools-address-quote = Котирувальний мінт
    .title = Копіювати котирувальний мінт
tokens-pools-address-paired = Парний мінт
    .title = Копіювати парний мінт

tokens-links-empty = Для цього токена немає офіційного вебсайту чи посилань на соцмережі.
tokens-links-info-title = Інформація про токен
tokens-links-mint-address = Адреса мінта
tokens-links-data-source = Джерело даних
tokens-links-security = Безпека
tokens-links-profile-title = Профіль токена
tokens-links-profile-published-title = Опублікований вміст профілю
tokens-links-profile-published-note = Медіа, опис і офіційні посилання — це платний вміст профілю, який перевіряється перед публікацією. Це не підтверджує право власності чи безпеку токена.
tokens-links-profile-create-note = Додайте перевірений логотип, опис проєкту й офіційні посилання до публічного профілю цього токена.
tokens-links-profile-update-hint = Оновіть профіль цього токена на screenerbot.io
tokens-links-profile-create-hint = Створіть профіль токена на screenerbot.io
tokens-links-profile-update = Оновити профіль
tokens-links-profile-create = Створити профіль
tokens-links-media-title = Медіафайли
tokens-links-media-fallback-symbol = Токен
tokens-links-media-logo = Логотип
tokens-links-media-banner = Банер
tokens-links-media-banner-alt = Банер { $symbol }
tokens-links-media-open = Відкрити зображення
tokens-links-description-title = Опис
tokens-links-explorers-title = Оглядачі й аналітика
tokens-links-websites-title = Офіційні вебсайти
tokens-links-socials-title = Соцмережі
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
tokens-links-social-fallback = Соцмережа

tokens-dialog-tab-overview = Огляд
tokens-dialog-tab-security = Безпека
tokens-dialog-tab-positions = Позиції
tokens-dialog-tab-pools = Пули
tokens-dialog-tab-links = Посилання
tokens-dialog-tab-transactions = Транз.
tokens-dialog-sections = Розділи даних токена
tokens-dialog-close =
    .title = Закрити (ESC)
    .aria-label = Закрити дані токена
tokens-dialog-unknown-symbol = Невідомо
tokens-dialog-unknown-name = Невідомий токен
tokens-dialog-market-summary = Підсумок ринку
tokens-dialog-price-loading = Завантаження ціни
tokens-dialog-unit-sol = { -sol }
tokens-dialog-market-metrics = Ринкові показники
tokens-dialog-metric-market-cap = Ринкова капіталізація
tokens-dialog-metric-volume-24h = Обсяг за 24 год
tokens-dialog-change-24h = Зміна за 24 години { $change }
tokens-dialog-buy = Купити
    .title = Купити цей токен
tokens-dialog-sell = Продати
    .title = Продати позицію
tokens-dialog-sell-unavailable = Немає відкритої позиції для продажу
tokens-dialog-details = Деталі
tokens-dialog-sources = Джерела
tokens-dialog-sources-status = Стан джерел даних
tokens-dialog-updated-label = Оновлено
tokens-dialog-just-now = Щойно
tokens-dialog-updated-at = Оновлено { $time }
tokens-dialog-updated-unavailable = Час оновлення недоступний
tokens-dialog-error-title = Не вдалося завантажити дані токена
tokens-dialog-waiting-token = Очікування даних токена…
tokens-dialog-loading-overview = Завантаження огляду…
tokens-dialog-loading-security = Завантаження безпеки…
tokens-dialog-loading-pools = Завантаження пулів…
tokens-dialog-loading-links = Завантаження посилань…
tokens-dialog-chart-still-checking = Даних графіка ще немає — триває перевірка…
tokens-dialog-no-data = Даних немає
tokens-dialog-source-token = Токен
tokens-dialog-source-market = Ринок
tokens-dialog-source-security = Безпека
tokens-dialog-source-chart = Графік
tokens-dialog-status-pending = Очікування
tokens-dialog-status-loading = Завантаження
tokens-dialog-status-ready = Готово
tokens-dialog-status-unavailable = Недоступно
tokens-dialog-status-cached = Із кешу
tokens-dialog-source-summary = { $source }: { $status }
tokens-dialog-badge-pool-price = Ціна пулу
tokens-dialog-badge-pool-price-hint = Ціна з ончейн-пулу в реальному часі
tokens-dialog-badge-api-price = Ціна API
tokens-dialog-badge-api-price-hint = Ціна з кешованих ринкових даних (API)
tokens-dialog-badge-profile = Опублікований профіль
    .title = Платний вміст профілю, перевірений перед публікацією; це не аудит і не підтвердження права власності.
tokens-dialog-badge-low-risk-hint = Низький ризик за поточною оцінкою { -rugcheck }; це не підтвердження особи.
tokens-dialog-badge-immutable = Незмінний
tokens-dialog-badge-mutable = Змінюваний
tokens-dialog-badge-auth = Повноваження:
tokens-dialog-badge-update-authority = Повноваження оновлення:
tokens-dialog-badge-position = Позиція
tokens-dialog-badge-blacklisted = У чорному списку

tokens-view-favorites = Обране
tokens-view-pool = Сервіс пулів
tokens-view-no-market = Без ринкових даних
tokens-view-all = Усі токени
tokens-view-passed = Пройдені
tokens-view-rejected = Відхилені
tokens-view-blacklisted = Чорний список
tokens-view-positions = Позиції
tokens-view-recent = Нещодавні
tokens-view-ohlcv = Дані OHLCV

tokens-cell-logo-enlarge = Натисніть, щоб збільшити
tokens-boost-title = Бустів на screenerbot.io: { $boosts }
tokens-cell-action-add =
    .title = Докупити в позицію (DCA)
    .aria-label = Докупити в позицію
tokens-cell-action-sell =
    .title = Продати (повністю або частково у %)
    .aria-label = Продати токен
tokens-cell-action-buy =
    .title = Купити позицію
    .aria-label = Купити токен
tokens-cell-external-links =
    .title = Зовнішні посилання
    .aria-label = Зовнішні посилання

tokens-table-loading-title = Завантаження токенів…
tokens-table-loading-description = Підготовка вибраного вигляду токенів.
tokens-table-retry-hint = Перемкніть вкладку або спробуйте ще раз.
tokens-filter-all = Усі

tokens-favorites-load-failed-title = Не вдалося завантажити обране
tokens-favorites-load-failed-toast = Не вдалося завантажити обране
tokens-favorites-total = Усього в обраному
tokens-favorites-empty-title = В обраному ще нічого немає
tokens-favorites-empty-description = Скористайтеся пошуком ({ $shortcut }), щоб знайти токени й додати їх до обраного.

tokens-column-token = Токен
tokens-column-status = Статус
tokens-ohlcv-delete =
    .title = Видалити дані OHLCV
    .aria-label = Видалити дані OHLCV
tokens-ohlcv-status-active = Активний
tokens-ohlcv-status-inactive = Неактивний
tokens-ohlcv-priority-critical = Критичний
tokens-ohlcv-priority-high = Високий
tokens-ohlcv-priority-medium = Середній
tokens-ohlcv-priority-low = Низький
tokens-ohlcv-column-priority = Пріоритет
tokens-ohlcv-column-backfill = Дозавантаження історії
tokens-ohlcv-column-data-span = Охоплення даних
tokens-ohlcv-column-gaps = Розриви
tokens-ohlcv-column-pools = Пули
tokens-ohlcv-column-last-fetch = Останнє отримання
tokens-ohlcv-timeframe-complete = { $timeframe }: завершено
tokens-ohlcv-timeframe-pending = { $timeframe }: очікує
tokens-ohlcv-load-failed-title = Не вдалося завантажити дані OHLCV
tokens-ohlcv-load-failed-toast = Не вдалося завантажити дані OHLCV
tokens-ohlcv-total = Усього токенів
tokens-ohlcv-active = Активні
tokens-ohlcv-db-size = Розмір БД
tokens-ohlcv-cleanup = Очистити неактивні
tokens-ohlcv-delete-title = Видалення даних OHLCV
tokens-ohlcv-delete-message = Видалити всі дані OHLCV для { $mint }...?
tokens-ohlcv-delete-done =
    Видалено: { $candles ->
        [one] { $candles } свічку
        [few] { $candles } свічки
        [many] { $candles } свічок
       *[other] { $candles } свічки
    }, { $pools ->
        [one] { $pools } пул
        [few] { $pools } пули
        [many] { $pools } пулів
       *[other] { $pools } пулу
    }
tokens-ohlcv-delete-failed = Не вдалося видалити дані OHLCV
tokens-ohlcv-cleanup-title = Видалення неактивних токенів
tokens-ohlcv-cleanup-message = Видалити неактивні токени, старші за вказану кількість годин
tokens-ohlcv-cleanup-placeholder = Години...
tokens-ohlcv-cleanup-invalid = Введіть додатне число
tokens-ohlcv-cleanup-done =
    Очищено { $count ->
        [one] { $count } неактивний токен
        [few] { $count } неактивні токени
        [many] { $count } неактивних токенів
       *[other] { $count } неактивного токена
    }
tokens-ohlcv-cleanup-failed = Не вдалося очистити дані OHLCV

tokens-summary-total = Усього
tokens-summary-priced = З ціною
tokens-summary-positions = Позиції
tokens-summary-blacklisted = У чорному списку
tokens-search-placeholder = Пошук за символом або мінтом...
tokens-table-waiting-title = Токени ще завантажуються...
tokens-table-waiting-description = Очікування відповіді від бекенду. Ми повторимо спробу автоматично.
tokens-load-failed-toast = Не вдалося завантажити токени
tokens-row-data-missing = Дані токена не знайдено
tokens-column-price-sol = Ціна ({ -sol })
tokens-column-liquidity = Ліквідність
tokens-column-volume-24h = Обсяг 24 год
tokens-column-fdv = FDV
tokens-column-market-cap = Рин. кап.
tokens-column-change-1h = 1 год
tokens-column-change-24h = 24 год
tokens-column-txns-5m = Транз. 5 хв
tokens-column-txns-1h = Транз. 1 год
tokens-column-txns-6h = Транз. 6 год
tokens-column-txns-24h = Транз. 24 год
tokens-column-risk-score = Оцінка ризику
tokens-column-reject-reason = Причина відхилення
tokens-column-blacklist-reason = Причина чорного списку
tokens-column-updated = Оновлено
tokens-column-birth = Створення
tokens-column-first-seen = Уперше помічено
tokens-badge-price = Ціна
tokens-badge-ohlcv = OHLCV
tokens-badge-position = Позиція
tokens-badge-blacklisted = У чорному списку
tokens-badge-blacklisted-title = Токен у чорному списку
tokens-badge-blacklisted-reasons = У чорному списку: { $reasons }
tokens-links-menu-copy-mint = Копіювати мінт
tokens-links-copy-failed = Не вдалося скопіювати мінт
tokens-lightbox-token-age = Вік токена

tokens-search-placeholder-dialog = Пошук за назвою, символом або мінтом...
tokens-search-input-label = Пошук токенів
tokens-search-results-label = Результати пошуку
tokens-search-hint = Введіть назву чи символ токена або вставте мінт
tokens-search-no-matches = Збігів немає — спробуйте інший запит
tokens-search-tip-nav = навігація
tokens-search-tip-open = відкрити
tokens-search-tip-close = закрити
tokens-search-failed = Пошук не вдався
tokens-search-error = Помилка: { $message }
tokens-search-action-favorite =
    .title = Додати до обраного
    .aria-label = Додати до обраного
tokens-search-action-blacklist =
    .title = Додати до чорного списку
    .aria-label = Додати до чорного списку
tokens-search-no-mint = У токена немає адреси мінта
tokens-search-open-failed = Не вдалося відкрити дані токена
tokens-search-copy-failed = Не вдалося скопіювати в буфер обміну
tokens-search-favorite-added = { $symbol } додано до обраного
tokens-search-favorite-already = Уже в обраному
tokens-search-favorite-failed = Не вдалося додати до обраного
tokens-search-blacklist-message = Занести { $symbol } до чорного списку? Цей токен буде виключено з торгівлі.
tokens-search-blacklist-done = { $symbol } занесено до чорного списку
tokens-search-blacklisted = У чорному списку
tokens-search-blacklist-failed = Не вдалося занести токен до чорного списку

tokens-featured-category-boosted = З бустом
tokens-featured-category-jupiter-organic = { -jupiter } Top Organic
tokens-featured-category-jupiter-traded = { -jupiter } Top Traded
tokens-featured-category-dexscreener-trending = { -dexscreener } Trending
tokens-featured-source-jupiter = { -jupiter }
tokens-featured-source-dexscreener = { -dexscreener }
tokens-featured-note-boosted = Просуваються своїми командами
tokens-featured-security-risky = Ризиковано
tokens-featured-load-failed = Не вдалося завантажити рекомендоване
tokens-featured-network-error = Помилка мережі: { $message }
tokens-featured-title = Рекомендоване
tokens-featured-subtitle = Спочатку токени з бустом, далі трендові в Solana
tokens-featured-boost = Забустити токен
tokens-featured-close =
    .title = Закрити (ESC)
tokens-featured-loading = Завантаження рекомендованих і трендових...
tokens-featured-error-hint = Перевірте з’єднання або спробуйте ще раз
tokens-featured-empty = Наразі немає доступних токенів
tokens-featured-count =
    { $count ->
        [one] { $count } токен
        [few] { $count } токени
        [many] { $count } токенів
       *[other] { $count } токена
    }
tokens-featured-stat-market-cap = Ринкова капіталізація
tokens-featured-stat-liquidity = Ліквідність
tokens-featured-stat-volume = Обсяг 24 год
tokens-featured-stat-holders = Холдери
tokens-featured-stat-txns = Транз. 24 год
tokens-featured-buy = Купити
    .title = Купити { $symbol }
tokens-featured-security-score = Оцінка безпеки: { $score }/100
tokens-featured-social-website = Вебсайт
tokens-featured-social-twitter = { -twitter }

tokens-featured-row-view-all = Усі
    .title = Відкрити повний вигляд рекомендованого
tokens-featured-row-scroll-start =
    .aria-label = Показати попередні токени
tokens-featured-row-scroll-end =
    .aria-label = Показати більше токенів
tokens-featured-row-empty = Немає рекомендованих токенів
tokens-featured-row-title = { $name } ({ $symbol })
tokens-featured-row-boosted-title = { $name } ({ $symbol }) — бустів: { $boosts }

tokens-pool-selector-title = Вибір пулу
tokens-pool-selector-loading = Завантаження пулів...
tokens-pool-selector-empty = Для цього токена пулів не знайдено
tokens-pool-selector-load-failed = Не вдалося завантажити пули: { $message }
tokens-pool-selector-count =
    { $count ->
        [one] Знайдено { $count } пул
        [few] Знайдено { $count } пули
        [many] Знайдено { $count } пулів
       *[other] Знайдено { $count } пулу
    }
tokens-pool-selector-liquidity = { $amount } ліквід.
    .title = Ліквідність
tokens-pool-selector-volume = { $amount } за 24 год
    .title = Обсяг за 24 год

tokens-identity-unknown-asset = Невідомий актив
tokens-identity-copy-address =
    .title = Копіювати адресу
    .aria-label = Копіювати адресу
tokens-identity-copy-signature =
    .title = Копіювати підпис
    .aria-label = Копіювати підпис
