# Filter rejection reasons. Message ids derive from the stored rejection codes
# (src/filtering/sources/rejection.rs); rows hold codes, never this text.

filtering-reject-no-decimals = Немає десяткових знаків у базі даних
filtering-reject-token-too-new = Токен занадто новий
filtering-reject-cooldown-filtered = Відфільтровано паузою
filtering-reject-dex-data-missing = Немає даних { -dexscreener }
filtering-reject-gecko-data-missing = Немає даних { -geckoterminal }
filtering-reject-rug-data-missing = Немає даних { -rugcheck }
filtering-reject-onchain-numeric-symbol = Символ лише з цифр (скам)
filtering-reject-onchain-empty-symbol = Порожній символ (скам)
filtering-reject-onchain-suspicious-symbol = Підозрілий символ (скам)
filtering-reject-onchain-known-scam-authority = Відомі повноваження скаму
filtering-reject-onchain-immutable-with-freeze = Незмінний + повноваження заморожування (скам)
filtering-reject-onchain-high-risk-score = Висока ончейн-оцінка ризику
filtering-reject-dex-empty-name = Порожня назва
filtering-reject-dex-empty-symbol = Порожній символ
filtering-reject-dex-empty-logo = Порожній URL логотипа
filtering-reject-dex-empty-website = Порожній URL вебсайту
filtering-reject-dex-txn-5m = Мало транзакцій за 5 хв
filtering-reject-dex-txn-1h = Мало транзакцій за 1 год
filtering-reject-dex-zero-liq = Нульова ліквідність
filtering-reject-dex-liq-low = Ліквідність занадто низька
filtering-reject-dex-liq-high = Ліквідність занадто висока
filtering-reject-dex-mcap-low = Ринкова капіталізація занадто низька
filtering-reject-dex-mcap-high = Ринкова капіталізація занадто висока
filtering-reject-dex-vol-low = Обсяг занадто низький
filtering-reject-dex-vol-missing = Немає обсягу
filtering-reject-dex-fdv-low = FDV занадто низька
filtering-reject-dex-fdv-high = FDV занадто висока
filtering-reject-dex-vol5m-low = Обсяг за 5 хв занадто низький
filtering-reject-dex-vol5m-missing = Немає обсягу за 5 хв
filtering-reject-dex-vol1h-low = Обсяг за 1 год занадто низький
filtering-reject-dex-vol1h-missing = Немає обсягу за 1 год
filtering-reject-dex-vol6h-low = Обсяг за 6 год занадто низький
filtering-reject-dex-vol6h-missing = Немає обсягу за 6 год
filtering-reject-dex-price-change-5m-low = Зміна ціни за 5 хв занадто мала
filtering-reject-dex-price-change-5m-high = Зміна ціни за 5 хв занадто велика
filtering-reject-dex-price-change-low = Зміна ціни занадто мала
filtering-reject-dex-price-change-high = Зміна ціни занадто велика
filtering-reject-dex-price-change-6h-low = Зміна ціни за 6 год занадто мала
filtering-reject-dex-price-change-6h-high = Зміна ціни за 6 год занадто велика
filtering-reject-dex-price-change-24h-low = Зміна ціни за 24 год занадто мала
filtering-reject-dex-price-change-24h-high = Зміна ціни за 24 год занадто велика
filtering-reject-gecko-liq-low = Ліквідність занадто низька
filtering-reject-gecko-liq-high = Ліквідність занадто висока
filtering-reject-gecko-mcap-low = Ринкова капіталізація занадто низька
filtering-reject-gecko-mcap-high = Ринкова капіталізація занадто висока
filtering-reject-gecko-vol5m-low = Обсяг за 5 хв занадто низький
filtering-reject-gecko-vol5m-missing = Немає обсягу за 5 хв
filtering-reject-gecko-vol1h-low = Обсяг за 1 год занадто низький
filtering-reject-gecko-vol1h-missing = Немає обсягу за 1 год
filtering-reject-gecko-vol24h-low = Обсяг за 24 год занадто низький
filtering-reject-gecko-vol24h-missing = Немає обсягу за 24 год
filtering-reject-gecko-price-change-5m-low = Зміна ціни за 5 хв занадто мала
filtering-reject-gecko-price-change-5m-high = Зміна ціни за 5 хв занадто велика
filtering-reject-gecko-price-change-1h-low = Зміна ціни за 1 год занадто мала
filtering-reject-gecko-price-change-1h-high = Зміна ціни за 1 год занадто велика
filtering-reject-gecko-price-change-24h-low = Зміна ціни за 24 год занадто мала
filtering-reject-gecko-price-change-24h-high = Зміна ціни за 24 год занадто велика
filtering-reject-gecko-pool-count-low = Занадто мало пулів
filtering-reject-gecko-pool-count-high = Занадто багато пулів
filtering-reject-gecko-pool-count-missing = Немає кількості пулів
filtering-reject-gecko-reserve-low = Резерв занадто малий
filtering-reject-gecko-reserve-missing = Немає резерву
filtering-reject-rug-rugged = Токен, що зазнав рагпулу
filtering-reject-rug-score = Оцінка ризику занадто висока
filtering-reject-rug-level-danger = Рівень ризику «небезпека»
filtering-reject-rug-mint-authority = Є повноваження мінта
filtering-reject-rug-freeze-authority = Є повноваження заморожування
filtering-reject-rug-top-holder = Частка найбільшого холдера занадто висока
filtering-reject-rug-top3-holders = Частка трьох найбільших холдерів занадто висока
filtering-reject-rug-min-holders = Недостатньо холдерів
filtering-reject-rug-insider-count = Занадто багато холдерів-інсайдерів
filtering-reject-rug-insider-pct = Частка інсайдерів занадто висока
filtering-reject-rug-creator-pct = Баланс творця занадто високий
filtering-reject-rug-transfer-fee-present = Є комісія за переказ
filtering-reject-rug-transfer-fee-high = Комісія за переказ занадто висока
filtering-reject-rug-graph-insiders = Інсайдерів у графі занадто багато
filtering-reject-rug-lp-providers-low = Провайдерів LP занадто мало
filtering-reject-rug-lp-providers-missing = Немає провайдерів LP
filtering-reject-rug-lp-lock-low = Блокування LP занадто мале
filtering-reject-rug-lp-lock-missing = Немає блокування LP
filtering-reject-llm-analysis-rejected = Аналіз LLM відхилив: { $reason } ({ $confidence }% впевн., { $provider })
filtering-reject-llm-analysis-rejected-generic = Аналіз LLM відхилив
filtering-reject-unknown = { $code }

# Codes no longer emitted; they appear only in stored rows and keep their wording.
filtering-reject-dex-fdv-missing = Немає FDV
filtering-reject-dex-price-change-5m-missing = Немає зміни ціни за 5 хв
filtering-reject-dex-price-change-missing = Немає зміни ціни
filtering-reject-dex-price-change-6h-missing = Немає зміни ціни за 6 год
filtering-reject-dex-price-change-24h-missing = Немає зміни ціни за 24 год
filtering-reject-gecko-liq-missing = Немає ліквідності
filtering-reject-gecko-mcap-missing = Немає ринкової капіталізації
filtering-reject-gecko-price-change-5m-missing = Немає зміни ціни за 5 хв
filtering-reject-gecko-price-change-1h-missing = Немає зміни ціни за 1 год
filtering-reject-gecko-price-change-24h-missing = Немає зміни ціни за 24 год
filtering-reject-rug-transfer-fee-missing = Немає даних про комісію за переказ

# Rejection categories used to group reasons.
filtering-reject-category-security = Проблеми безпеки
filtering-reject-category-distribution = Розподіл між холдерами
filtering-reject-category-liquidity-lock = Проблеми блокування LP
filtering-reject-category-fees = Комісії за переказ
filtering-reject-category-liquidity = Ліквідність
filtering-reject-category-volume = Обсяг торгів
filtering-reject-category-market-cap = Ринкова капіталізація / FDV
filtering-reject-category-price-action = Рух ціни
filtering-reject-category-activity = Торгова активність
filtering-reject-category-data-quality = Відсутні дані
filtering-reject-category-timing = Часові фільтри
filtering-reject-category-market = Ринкові дані
filtering-reject-category-other = Інше

# Filtering page: sub-tabs, sources, status, analytics, explorer and configuration.

## Sub-tabs and sources. Source ids are FilterSource::as_str plus the `meta` settings tab.

filtering-tab-status = Статус
filtering-tab-analytics = Аналітика
filtering-tab-explorer = Оглядач
filtering-source-core = Ядро
filtering-source-onchain = Ончейн
filtering-source-dexscreener = { -dexscreener }
filtering-source-geckoterminal = { -geckoterminal }
filtering-source-rugcheck = { -rugcheck }
filtering-source-llm-analysis = Аналіз LLM

## Time range

filtering-range-1h = 1 год
filtering-range-6h = 6 год
filtering-range-24h = 24 год
filtering-range-7d = 7 д
filtering-range-all = Усі
filtering-range-all-time = За весь час
filtering-range-custom = Власний
filtering-range-now = Зараз
# $start and $end are formatted moments, or the open-ended markers.
filtering-range-span = { $start } → { $end }
# $min and $max are the two ends of a value range.
filtering-range-bounds = { $min } – { $max }

## Footer status line

filtering-footer-saving = Збереження змін...
filtering-footer-refreshing = Оновлення знімка...
filtering-footer-unsaved = Є незбережені зміни
# $time is a relative time such as "5m ago".
filtering-footer-last-saved = Останнє збереження: { $time }
filtering-footer-in-sync = Конфігурацію синхронізовано

## Info bar and status metrics

filtering-info-total = Усього
filtering-info-priced = З ціною
filtering-info-passed = Пройшли
filtering-info-positions = Позиції
filtering-info-blacklisted = У чорному списку
filtering-info-cache = Кеш
# A count followed by its share of the total, e.g. "120 (4.0%)".
filtering-count-share = { $count } ({ $share })
filtering-refresh-building = Побудова…
filtering-refresh-never = Ніколи

filtering-status-loading = Завантаження статистики...
filtering-status-total = Усього токенів
filtering-status-total-detail = У кеші фільтрації
filtering-status-total-detail-building = Знімок будується — підрахунки з’являться після наступного оновлення
filtering-status-priced = З ціною
filtering-status-priced-detail = { $share } мають ціну
filtering-status-passed = Пройшли фільтри
filtering-status-passed-detail = { $share } пройшли
filtering-status-positions = Відкриті позиції
filtering-status-positions-detail = Активні угоди
filtering-status-blacklisted = У чорному списку
filtering-status-blacklisted-detail = Токени з позначкою
filtering-status-ohlcv = З OHLCV
filtering-status-ohlcv-detail = Історичні дані
filtering-status-refresh = Останнє оновлення
filtering-status-refresh-building = Триває перший знімок
filtering-status-refresh-none = Оновлень ще не було
filtering-status-no-rejections = Даних про відхилення немає

## Analytics

filtering-analytics-loading = Завантаження аналітики за період: { $range }…
filtering-analytics-scanned = Усього проскановано
# $time is a relative time such as "5m ago".
filtering-analytics-updated = Оновлено { $time }
filtering-analytics-passed = Токени, що пройшли
filtering-analytics-pass-rate = Частка пройдених: <strong>{ $share }</strong>
filtering-analytics-rejected = Відхилені токени
filtering-analytics-rejection-rate = Частка відхилених: <strong>{ $share }</strong>
filtering-analytics-by-category = Відхилення за категоріями
filtering-analytics-by-source = Відхилення за джерелами
filtering-analytics-no-category = Немає даних за категоріями
filtering-analytics-no-source = Немає даних за джерелами
filtering-analytics-top-reasons = Найчастіші причини відхилення
filtering-analytics-no-data = Даних немає
filtering-analytics-column-reason = Причина
filtering-analytics-column-category = Категорія
filtering-analytics-column-count = Кількість
filtering-analytics-column-share = %
filtering-analytics-column-impact = Вплив
# $amount is the formatted count, $count selects the plural.
filtering-tokens-count =
    { $count ->
        [one] { $amount } токен
        [few] { $amount } токени
        [many] { $amount } токенів
       *[other] { $amount } токена
    }

## Explorer

filtering-explorer-top-reasons = Найчастіші причини
filtering-explorer-recent = Останні відхилення
filtering-explorer-none = Немає даних
filtering-explorer-none-recent = Немає нещодавніх
filtering-explorer-search =
    .placeholder = Пошук причин...
filtering-explorer-overview = Огляд
filtering-explorer-no-match = Немає відповідних причин
filtering-explorer-column-token = Токен
filtering-explorer-column-source = Джерело
filtering-explorer-column-time = Час
filtering-explorer-page = Сторінка { $page }
filtering-explorer-no-results = Немає результатів
filtering-explorer-empty = Токенів не знайдено
filtering-explorer-empty-filtered = Токенів, що відповідають фільтру, не знайдено
filtering-explorer-load-failed = Не вдалося завантажити токени

## Configuration panels

filtering-config-loading = Завантаження конфігурації…
# $query is the text typed in the filter box.
filtering-config-no-match = Жоден параметр не відповідає «{ $query }»
filtering-config-no-parameters = Це джерело не має параметрів
# $source is the source name.
filtering-source-off = Фільтрацію { $source } вимкнено — ці параметри не оцінюються.
filtering-toolbar-filter =
    .placeholder = Фільтр параметрів
    .aria-label = Фільтр параметрів
filtering-toolbar-clear =
    .aria-label = Очистити фільтр
# $count selects the plural, $amount is the number shown.
filtering-parameter-count =
    { $count ->
        [one] { $amount } параметр
        [few] { $amount } параметри
        [many] { $amount } параметрів
       *[other] { $amount } параметра
    }
# $count is the total and selects the plural.
filtering-parameter-count-filtered =
    { $count ->
        [one] Параметрів: { $visible } із { $total }
        [few] Параметрів: { $visible } із { $total }
        [many] Параметрів: { $visible } із { $total }
       *[other] Параметрів: { $visible } із { $total }
    }
filtering-group-enable =
    .aria-label = Увімкнути перевірки: { $group }
filtering-field-min = Мін.
filtering-field-max = Макс.
# $label is the parameter name.
filtering-field-min-aria =
    .aria-label = Мінімум: { $label }
filtering-field-max-aria =
    .aria-label = Максимум: { $label }
# $default is the shipped value, $label the parameter name.
filtering-field-reset =
    .title = Скинути до типового ({ $default })
    .aria-label = Скинути «{ $label }» до типового значення

## Toasts. A message value is the title; `.message` is the body.

filtering-toast-saved = Конфігурацію збережено
    .message = Налаштування фільтрації збережено, знімок оновлено
filtering-toast-save-failed = Помилка збереження
    .message = Не вдалося зберегти конфігурацію фільтрації
filtering-toast-reset = Зміни скинуто
    .message = Конфігурацію відновлено до останнього збереженого стану
filtering-toast-refresh-failed = Помилка оновлення
    .message = Не вдалося оновити знімок фільтрації
filtering-toast-exported = Конфігурацію експортовано
    .message = Налаштування фільтрації збережено у файл
filtering-toast-imported = Конфігурацію імпортовано
    .message = Налаштування фільтрації завантажено з файлу
filtering-toast-import-failed = Помилка імпорту
    .message = Не вдалося імпортувати конфігурацію — недійсний формат файлу
filtering-toast-load-failed = Помилка завантаження
    .message = Не вдалося завантажити конфігурацію фільтрації
filtering-toast-range-missing = Виберіть початкову й кінцеву дати
filtering-toast-range-order = Час початку має бути раніше за час завершення
filtering-toast-range-future = Час завершення не може бути в майбутньому
