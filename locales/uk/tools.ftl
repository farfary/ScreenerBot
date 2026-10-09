## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = Інструменти
tools-category-wallet = Гаманець
tools-category-token = Токен
tools-category-single-token = Один токен
tools-category-utilities = Утиліти
tools-sidebar-hint = Виберіть інструмент, щоб почати
tools-help-button =
    .aria-label = Показати довідку про цей інструмент
tools-help-unavailable = Довідка недоступна
tools-placeholder-title = Виберіть інструмент
tools-placeholder-subtitle = Виберіть інструмент на бічній панелі, щоб почати
tools-placeholder-hint-wallets = Інструменти гаманця допомагають керувати вашими гаманцями Solana
tools-placeholder-hint-secure = Усі операції захищені й, де можливо, оборотні

tools-status-ready = Готово до використання
tools-status-coming = Незабаром
tools-status-beta = Бета — можливі помилки
tools-status-disabled = Наразі вимкнено
tools-status-badge-coming = Незабаром
tools-status-badge-beta = Бета
tools-toast-coming-soon = Цей інструмент з’явиться незабаром
tools-toast-disabled = Цей інструмент наразі вимкнено
tools-setup-gate-title = Для цього інструмента потрібен гаманець

## Tool names.

tools-tool-wallet-cleanup-title = Очищення гаманця
tools-tool-wallet-cleanup-summary = Закрити порожні ATA
tools-tool-wallet-cleanup-description = Закрийте порожні асоційовані токен-акаунти (ATA), щоб повернути { -sol }
tools-tool-burn-tokens-title = Спалення токенів
tools-tool-burn-tokens-summary = Безповоротно знищити токени
tools-tool-burn-tokens-description = Безповоротно знищіть токени зі свого гаманця
tools-tool-token-analyzer-title = Аналізатор токенів
tools-tool-token-analyzer-summary = Глибокий аналіз токена
tools-tool-token-analyzer-description = Глибокий багатовимірний аналіз будь-якого токена Solana
tools-tool-create-token-title = Створення токена
tools-tool-create-token-summary = Розгорнути новий токен SPL
tools-tool-create-token-description = Розгорніть новий токен SPL у Solana
tools-tool-trade-watcher-title = Trade Watcher
tools-tool-trade-watcher-summary = Моніторинг угод і автодії
tools-tool-trade-watcher-description = Стежте за угодами з токеном і запускайте автоматичні дії купівлі чи продажу
tools-tool-token-watch-title = Holder Watch
tools-tool-token-watch-summary = Відстеження нових холдерів
tools-tool-token-watch-description = Відстежуйте нових холдерів токена в реальному часі
tools-tool-buy-multi-wallets-title = Мультикупівля
tools-tool-buy-multi-wallets-summary = Скоординовані купівлі з кількох гаманців
tools-tool-buy-multi-wallets-description = Виконуйте скоординовані ордери на купівлю з кількох гаманців з випадковими сумами
tools-tool-sell-multi-wallets-title = Мультипродаж
tools-tool-sell-multi-wallets-summary = Скоординовані продажі з кількох гаманців
tools-tool-sell-multi-wallets-description = Виконуйте скоординовані ордери на продаж з кількох гаманців із консолідацією { -sol }
tools-tool-wallet-consolidation-title = Консолідація гаманців
tools-tool-wallet-consolidation-nav-title = Консолідація
tools-tool-wallet-consolidation-summary = Зібрати кошти гаманців
tools-tool-wallet-consolidation-description = Поверніть { -sol } і токени із субгаманців на основний гаманець
tools-tool-airdrop-checker-title = Перевірка аірдропів
tools-tool-airdrop-checker-summary = Перевірити очікувані аірдропи
tools-tool-airdrop-checker-description = Перевірте очікувані аірдропи та доступні для отримання винагороди
tools-tool-wallet-generator-title = Генератор гаманців
tools-tool-wallet-generator-summary = Створити нові пари ключів
tools-tool-wallet-generator-description = Безпечно створюйте нові пари ключів Solana

## Shared by the tools

tools-validation-mint-required = Введіть адресу мінта токена
tools-validation-mint-format = Недійсний формат адреси мінта токена
tools-validation-mint-invalid = Введіть дійсну адресу мінта

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = Дані токена
tools-create-token-name-label = Назва токена
tools-create-token-name-input =
    .placeholder = Мій токен
tools-create-token-symbol-label = Символ
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = Десяткові знаки
tools-create-token-supply-label = Початкова пропозиція
tools-create-token-description-label = Опис
tools-create-token-description-input =
    .placeholder = Опис токена...
tools-create-token-image-title = Зображення токена
tools-create-token-image-drop = Перетягніть зображення сюди або натисніть, щоб завантажити
tools-create-token-image-hint = Рекомендовано: PNG 512x512
tools-create-token-action-preview = Попередній перегляд
tools-create-token-action-create = Створити токен

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = Завантаження налаштувань...
tools-holder-watch-saved = Налаштування Holder Watch збережено
tools-holder-watch-save-failed = Не вдалося зберегти налаштування
tools-holder-watch-save-error = Помилка збереження налаштувань
tools-holder-watch-settings-title = Налаштування Holder Watch
tools-holder-watch-enabled-label = Увімкнути стеження за холдерами
tools-holder-watch-interval-label = Інтервал перевірки
tools-holder-watch-interval-hint = Як часто перевіряти кількість холдерів (10–3600 с)
tools-holder-watch-max-tokens-label = Макс. відстежуваних токенів
tools-holder-watch-max-tokens-hint = Максимальна кількість токенів для одночасного відстеження
tools-holder-watch-notify-new-label = Сповіщати про нових холдерів
tools-holder-watch-notify-drop-label = Сповіщати про зменшення кількості холдерів
tools-holder-watch-min-change-label = Мін. зміна кількості холдерів
tools-holder-watch-min-change-hint = Мінімальна зміна кількості холдерів для сповіщення
tools-holder-watch-drop-percent-label = Поріг зменшення кількості холдерів
tools-holder-watch-drop-percent-hint = Відсоток зменшення для спрацювання сповіщення
tools-holder-watch-action-save = Зберегти налаштування
tools-holder-watch-tokens-title = Відстежувані токени
tools-holder-watch-token-input =
    .placeholder = Введіть адресу мінта токена...
tools-holder-watch-empty = Жодного токена не відстежується
tools-holder-watch-empty-hint = Додайте адресу мінта токена вище, щоб почати стеження
tools-holder-watch-coming-soon = Відстеження токенів з’явиться незабаром

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = Аналіз токена
tools-analyzer-mint-input =
    .placeholder = Вставте адресу мінта токена...
tools-analyzer-action-analyze = Аналізувати
tools-analyzer-action-analyzing = Аналіз...
tools-analyzer-action-copy-report = Копіювати звіт
tools-analyzer-loading = Аналіз токена...
tools-analyzer-failed = Не вдалося проаналізувати токен
tools-analyzer-empty = Введіть адресу мінта токена для аналізу
tools-analyzer-empty-hint = Отримайте вичерпну аналітику щодо будь-якого токена Solana
tools-analyzer-tab-overview = Огляд
tools-analyzer-tab-security = Безпека
tools-analyzer-tab-market = Ринок
tools-analyzer-tab-liquidity = Ліквідність
tools-analyzer-unknown-token = Невідомий токен

tools-analyzer-favorite-add =
    .title = Додати до обраного
    .aria-label = Додати до обраного
tools-analyzer-favorite-already = Уже в обраному
tools-analyzer-favorite-added = { $symbol } додано до обраного
tools-analyzer-favorite-failed = Не вдалося додати до обраного
tools-analyzer-blacklist-add =
    .title = Додати до чорного списку
    .aria-label = Додати до чорного списку
tools-analyzer-blacklist-title = Занести токен до чорного списку
tools-analyzer-blacklist-message = Занести { $symbol } до чорного списку? Цей токен буде виключено з торгівлі.
tools-analyzer-blacklist-confirm = Занести до чорного списку
tools-analyzer-blacklisted = У чорному списку
tools-analyzer-blacklist-done = { $symbol } занесено до чорного списку
tools-analyzer-blacklist-failed = Не вдалося занести токен до чорного списку

tools-analyzer-card-quick-stats = Коротка статистика
tools-analyzer-card-market-summary = Підсумок ринку
tools-analyzer-card-token-info = Інформація про токен
tools-analyzer-stat-holders = Холдери
tools-analyzer-stat-decimals = Десяткові знаки
tools-analyzer-stat-safety-score = Оцінка безпеки
tools-analyzer-stat-pools = Пули
tools-analyzer-stat-volume-24h = Обсяг за 24 год
tools-analyzer-stat-change-24h = Зміна за 24 год
tools-analyzer-stat-market-cap = Ринкова капіталізація
tools-analyzer-stat-liquidity = Ліквідність
tools-analyzer-info-mint = Адреса мінта
tools-analyzer-info-description = Опис
tools-analyzer-info-supply = Пропозиція

tools-analyzer-security-empty = Даних про безпеку немає
tools-analyzer-security-empty-hint = Аналіз безпеки для цього токена недоступний
tools-analyzer-card-safety-score = Оцінка безпеки
tools-analyzer-score-good = Добре
tools-analyzer-score-moderate = Помірно
tools-analyzer-score-risky = Ризиковано
tools-analyzer-raw-score = Необроблена оцінка ризику: { $score }
tools-analyzer-card-authorities = Повноваження токена
tools-analyzer-authority-mint = Повноваження мінта
tools-analyzer-authority-freeze = Повноваження заморожування
tools-analyzer-authority-transfer-fee = Комісія за переказ
tools-analyzer-authority-mutable = Змінюваний
tools-analyzer-authority-active = Активне
tools-analyzer-authority-revoked = Відкликано
tools-analyzer-card-holder-concentration = Концентрація холдерів
tools-analyzer-top-holders = у топ-10 холдерів
tools-analyzer-risks-title = Ризики безпеки ({ $count })
tools-analyzer-risks-title-none = Ризики безпеки
tools-analyzer-risks-none = Ризиків безпеки не виявлено

tools-analyzer-market-empty = Ринкових даних немає
tools-analyzer-market-empty-hint = Ринкові дані для цього токена недоступні
tools-analyzer-card-price = Поточна ціна
tools-analyzer-card-price-changes = Зміни ціни
tools-analyzer-card-volume = Обсяг торгів
tools-analyzer-card-transactions = Транзакції за 24 год
tools-analyzer-card-valuation = Оцінка вартості
tools-analyzer-stat-window-1h = 1 год
tools-analyzer-stat-window-6h = 6 год
tools-analyzer-stat-window-24h = 24 год
tools-analyzer-stat-volume-1h = Обсяг за 1 год
tools-analyzer-stat-volume-6h = Обсяг за 6 год
tools-analyzer-stat-fdv = Повністю розбавлена вартість
tools-analyzer-txn-buys = Купівлі
tools-analyzer-txn-sells = Продажі

tools-analyzer-liquidity-empty = Даних про ліквідність немає
tools-analyzer-liquidity-empty-hint = Для цього токена пулів не знайдено
tools-analyzer-card-total-liquidity = Загальна ліквідність
tools-analyzer-card-pools = Пули
tools-analyzer-active-pools =
    { $count ->
        [one] Активний пул
        [few] Активні пули
        [many] Активних пулів
       *[other] Активного пулу
    }
tools-analyzer-card-pool-details = Дані пулу
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = Ліквідність ({ -sol })
tools-analyzer-pools-column-status = Статус
tools-analyzer-pool-primary = Основний

tools-analyzer-report-empty = Немає аналізу для копіювання
tools-analyzer-report-label = Звіт аналізу
tools-analyzer-report-title = Звіт аналізу токена
tools-analyzer-report-token = Токен: { $symbol } ({ $name })
tools-analyzer-report-mint = Мінт: { $mint }
tools-analyzer-report-price = Ціна: { $sol }
tools-analyzer-report-price-with-usd = Ціна: { $sol } ({ $usd })
tools-analyzer-report-security = Безпека:
tools-analyzer-report-safety-score = - Оцінка безпеки: { $score }/100
tools-analyzer-report-mint-authority = - Повноваження мінта: { $state }
tools-analyzer-report-freeze-authority = - Повноваження заморожування: { $state }
tools-analyzer-report-risks = - Ризики: { $count }
tools-analyzer-report-market = Ринок:
tools-analyzer-report-volume = - Обсяг за 24 год: { $amount }
tools-analyzer-report-change = - Зміна за 24 год: { $amount }
tools-analyzer-report-market-cap = - Ринкова капіталізація: { $amount }
tools-analyzer-report-liquidity = Ліквідність:
tools-analyzer-report-liquidity-total = - Разом: { $amount }
tools-analyzer-report-pools = - Пули: { $count }
tools-analyzer-report-generated = Створено: { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

tools-watch-type-buy-on-sell = Купівля на продаж
tools-watch-type-sell-on-buy = Продаж на купівлю
tools-watch-type-notify = Сповіщення
tools-watch-type-notify-only = Лише сповіщення

tools-trade-watcher-setup-title = Налаштування стеження
tools-trade-watcher-mint-label = Адреса мінта токена
tools-trade-watcher-mint-input =
    .placeholder = Введіть адресу мінта токена...
tools-trade-watcher-action-search-pools = Шукати пули
tools-trade-watcher-pool-label = Вибраний пул
tools-trade-watcher-pool-none = Пул не вибрано
tools-trade-watcher-pool-clear =
    .title = Скинути пул
tools-trade-watcher-pool-selected = Вибраний пул: { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = Тип стеження
tools-trade-watcher-type-hint = Купівля на продаж: автоматично купувати, коли хтось продає. Продаж на купівлю: автоматично продавати, коли хтось купує.
tools-trade-watcher-trigger-label = Сума спрацювання
tools-trade-watcher-trigger-hint = Мінімальний розмір угоди в { -sol } для запуску дії
tools-trade-watcher-action-amount-label = Сума дії
tools-trade-watcher-action-amount-hint = Сума купівлі чи продажу після спрацювання
tools-trade-watcher-slippage-label = Проковзування
tools-trade-watcher-slippage-hint = Максимально допустиме проковзування для угод
tools-trade-watcher-active-title = Активні стеження
tools-trade-watcher-empty = Активних стежень немає
tools-trade-watcher-empty-hint = Налаштуйте стеження вище й натисніть «Почати стеження», щоб розпочати моніторинг
tools-trade-watcher-action-start = Почати стеження
tools-trade-watcher-action-starting = Запуск...
tools-trade-watcher-action-stop-all = Зупинити все
tools-trade-watcher-action-stopping = Зупинка...
tools-trade-watcher-started = Стеження за { $token } запущено...
tools-trade-watcher-start-failed = Не вдалося запустити стеження
tools-trade-watcher-stopped = Стеження зупинено
tools-trade-watcher-stop-failed = Не вдалося зупинити стеження
tools-trade-watcher-stopped-all = Усі стеження зупинено
tools-trade-watcher-stop-all-failed = Не вдалося зупинити стеження
tools-trade-watcher-load-failed = Не вдалося завантажити стеження
tools-trade-watcher-column-token = Токен
tools-trade-watcher-column-type = Тип
tools-trade-watcher-column-trigger = Спрацювання ({ -sol })
tools-trade-watcher-column-action = Дія ({ -sol })
tools-trade-watcher-column-triggered = Спрацювало
tools-trade-watcher-stop-watch =
    .title = Зупинити стеження

## Results returned by the tools backend.

tools-burn-failure-native-asset = Не можна спалити { -sol }
tools-burn-failure-open-position = Не можна спалювати токени з відкритих позицій
tools-burn-failure-account-not-found = Токен-акаунт не знайдено
tools-burn-failure-zero-balance = Баланс токена вже нульовий
tools-burn-failure-transaction = Транзакція не вдалася
tools-burn-warning-open-position = Не можна спалювати токени з відкритих позицій
tools-burn-warning-closed-position = Залишок від закритої позиції
tools-burn-warning-worth = Вартість ~{ $amount } { -sol }
tools-multi-buy-warning-insufficient = Недостатньо коштів. Потрібно { $needed } { -sol }, доступно { $have } { -sol }
tools-multi-buy-warning-over-limit = Потрібна загальна сума { -sol } ({ $needed }) перевищує ліміт ({ $limit })
tools-multi-sell-warning-no-wallets = Додаткових гаманців не знайдено
tools-multi-sell-warning-no-balance = Жоден гаманець не має балансу токена
tools-multi-op-buy-failed = Не вдалося купити
tools-multi-op-sell-failed = Не вдалося продати
tools-multi-op-transfer-failed = Не вдалося виконати переказ
tools-multi-op-balance-failed = Не вдалося отримати баланс
tools-multi-op-mint-invalid = Недійсна адреса мінта
tools-multi-buy-session-failed = Мультикупівля не вдалася
tools-multi-sell-session-failed = Мультипродаж не вдався
tools-multi-session-aborted = Операцію перервав користувач

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = Сканувати гаманець
tools-wallet-action-scanning = Сканування...
tools-wallet-scan-failed = Помилка сканування: { $reason }
tools-wallet-amount-approx = ~{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
        [one] Вибрано: { $count } гаманець
        [few] Вибрано: { $count } гаманці
        [many] Вибрано: { $count } гаманців
       *[other] Вибрано: { $count } гаманця
    }
tools-wallet-transfer-failed = Помилка переказу: { $reason }
tools-wallet-cleanup-failed = Помилка очищення: { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = Результати сканування
tools-wallet-cleanup-stat-empty = Порожні ATA
tools-wallet-cleanup-stat-reclaimable = { -sol } до повернення
tools-wallet-cleanup-stat-failed = З помилкою (кеш)
tools-wallet-cleanup-prompt = Натисніть «Сканувати гаманець», щоб знайти порожні ATA
tools-wallet-cleanup-prompt-hint = Буде перевірено всі токен-акаунти у вашому гаманці
tools-wallet-cleanup-action-cleanup = Очистити все
tools-wallet-cleanup-action-cleaning = Очищення...
tools-wallet-cleanup-scanning = Сканування гаманця...
tools-wallet-cleanup-found =
    { $count ->
        [one] Знайдено { $count } порожній ATA вартістю ~{ $amount }
        [few] Знайдено { $count } порожні ATA вартістю ~{ $amount }
        [many] Знайдено { $count } порожніх ATA вартістю ~{ $amount }
       *[other] Знайдено { $count } порожнього ATA вартістю ~{ $amount }
    }
tools-wallet-cleanup-clean = Порожніх ATA не знайдено — гаманець чистий!
tools-wallet-cleanup-scan-failed = Не вдалося просканувати ATA
tools-wallet-cleanup-done =
    { $count ->
        [one] Очищено { $count } ATA
        [few] Очищено { $count } ATA
        [many] Очищено { $count } ATA
       *[other] Очищено { $count } ATA
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = Спалення токенів
tools-burn-info-title = Що таке спалення?
tools-burn-info-body = Спалення безповоротно знищує токени, і відновити їх неможливо. Після спалення запустіть «Очищення гаманця», щоб закрити порожні ATA й повернути ~0.002 { -sol } ренти за кожен токен.
tools-burn-stat-total = Усього токенів
tools-burn-stat-selected = Вибрано
tools-burn-stat-rent = Рента до повернення
tools-burn-prompt = Натисніть «Сканувати гаманець», щоб знайти токени
tools-burn-scanning = Сканування гаманця на токени...
tools-burn-scan-failed = Не вдалося просканувати токени
tools-burn-empty = У гаманці токенів не знайдено
tools-burn-action-burn = Спалити вибрані ({ $count })
tools-burn-action-burning = Спалення...
tools-burn-cannot-burn = Спалити не можна
tools-burn-no-value = Без вартості

tools-burn-category-open-position = Відкриті позиції
tools-burn-category-has-value = Мають вартість
tools-burn-category-closed-position = Закриті позиції
tools-burn-category-zero-liquidity = Нульова ліквідність
tools-burn-category-hint-open-position = Не можна спалювати токени з відкритих позицій
tools-burn-category-hint-has-value = Краще продайте, ніж спалюйте
tools-burn-category-hint-closed-position = Залишки від закритих угод
tools-burn-category-hint-zero-liquidity = Спалювати безпечно — ринкової вартості немає

tools-burn-confirm-title = Підтвердження спалення
tools-burn-confirm-message =
    { $count ->
        [one] Справді спалити <strong>{ $count }</strong> токен?
        [few] Справді спалити <strong>{ $count }</strong> токени?
        [many] Справді спалити <strong>{ $count }</strong> токенів?
       *[other] Справді спалити <strong>{ $count }</strong> токена?
    }
tools-burn-confirm-value = Загальна орієнтовна вартість: <strong>{ $amount }</strong>
tools-burn-confirm-continue = Продовжити
tools-burn-final-title = Остаточне попередження
tools-burn-final-headline = Цю дію НЕМОЖЛИВО скасувати!
tools-burn-final-message =
    { $count ->
        [one] Наведений нижче токен ({ $count }) буде безповоротно знищено, і відновити його не можна за жодних обставин.
        [few] Наведені нижче токени ({ $count }) буде безповоротно знищено, і відновити їх не можна за жодних обставин.
        [many] Наведені нижче токени ({ $count }) буде безповоротно знищено, і відновити їх не можна за жодних обставин.
       *[other] Наведені нижче токени ({ $count }) буде безповоротно знищено, і відновити їх не можна за жодних обставин.
    }
tools-burn-final-confirm = Так, спалити токени
tools-burn-toast-burned =
    { $total ->
        [one] Спалено { $successful }/{ $total } токен. Запустіть «Очищення гаманця», щоб повернути ~{ $amount }
        [few] Спалено { $successful }/{ $total } токени. Запустіть «Очищення гаманця», щоб повернути ~{ $amount }
        [many] Спалено { $successful }/{ $total } токенів. Запустіть «Очищення гаманця», щоб повернути ~{ $amount }
       *[other] Спалено { $successful }/{ $total } токена. Запустіть «Очищення гаманця», щоб повернути ~{ $amount }
    }
tools-burn-toast-failed =
    { $count ->
        [one] Не вдалося спалити { $count } токен
        [few] Не вдалося спалити { $count } токени
        [many] Не вдалося спалити { $count } токенів
       *[other] Не вдалося спалити { $count } токена
    }
tools-burn-failed = Помилка спалення: { $reason }
tools-burn-failures-title =
    { $count ->
        [one] Не вдалося спалити { $count } токен
        [few] Не вдалося спалити { $count } токени
        [many] Не вдалося спалити { $count } токенів
       *[other] Не вдалося спалити { $count } токена
    }
tools-burn-failure-unknown = Причину не повідомлено

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = Про інструмент
tools-airdrop-about-body = Перевіряйте очікувані аірдропи, доступні винагороди й неотримані розподіли в популярних протоколах Solana.
tools-airdrop-list-title = Доступні аірдропи
tools-airdrop-prompt = Натисніть «Перевірити аірдропи», щоб знайти доступні виплати
tools-airdrop-action-check = Перевірити аірдропи
tools-airdrop-action-claim-all = Отримати все

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = Параметри генератора
tools-generator-warning-title = Надійно зберігайте свої приватні ключі!
tools-generator-warning-body = Пари ключів створюються локально й ніколи не передаються. Обов’язково зробіть резервну копію ключів у безпечному місці.
tools-generator-count-label = Кількість гаманців
tools-generator-vanity-label = Красива адреса (починається із заданих символів)
tools-generator-prefix-label = Префікс
tools-generator-prefix-input =
    .placeholder = напр., SOL
tools-generator-prefix-hint = Довші префікси генеруються експоненційно довше
tools-generator-list-title = Створені гаманці
tools-generator-empty = Гаманці ще не створено
tools-generator-action-generate = Створити
tools-generator-action-generating = Створення...
tools-generator-count-invalid = Введіть число від 1 до 10
tools-generator-no-keypairs = Пар ключів не повернуто
tools-generator-generated =
    { $count ->
        [one] Створено { $count } гаманець
        [few] Створено { $count } гаманці
        [many] Створено { $count } гаманців
       *[other] Створено { $count } гаманця
    }
tools-generator-failed = Не вдалося створити гаманці: { $reason }
tools-generator-copy-public-key =
    .title = Копіювати публічний ключ
tools-generator-copy-private-key =
    .title = Копіювати приватний ключ
tools-generator-remove =
    .title = Видалити зі списку
tools-generator-reveal =
    .title = Показати приватний ключ
tools-generator-public-key-label = Публічний ключ:
tools-generator-private-key-label = Приватний ключ:
tools-generator-public-key-name = Публічний ключ
tools-generator-private-key-copied = Приватний ключ скопійовано
tools-generator-private-key-warning = Будь-хто з цим ключем контролює гаманець
tools-generator-export-empty = Немає гаманців для експорту
tools-generator-exported = Гаманці експортовано — зберігайте надійно

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = Підсумок
tools-consolidation-stat-wallets = Субгаманці
tools-consolidation-stat-native = Усього { -sol }
tools-consolidation-stat-tokens = Типи токенів
tools-consolidation-stat-rent = Рента до повернення
tools-consolidation-wallets-title = Гаманці
tools-consolidation-loading-wallets = Завантаження гаманців...
tools-consolidation-loading-data = Завантаження даних гаманця...
tools-consolidation-action-transfer-native = Перевести { -sol }
tools-consolidation-action-transfer-tokens = Перевести всі токени
tools-consolidation-action-cleanup = Очистити ATA
tools-consolidation-action-transferring = Переказ...
tools-consolidation-column-name = Назва
tools-consolidation-column-native = Баланс ({ -sol })
tools-consolidation-column-tokens = Токени
tools-consolidation-column-atas = Порожні ATA
tools-consolidation-empty = Субгаманців не знайдено
tools-consolidation-empty-hint = Створіть субгаманці через мультикупівлю, щоб почати
tools-consolidation-load-failed = Не вдалося завантажити: { $reason }
tools-consolidation-select-prompt = Виберіть гаманці для консолідації
tools-consolidation-selection-totals =
    | { $amount } | { $tokens ->
        [one] { $tokens } токен
        [few] { $tokens } токени
        [many] { $tokens } токенів
       *[other] { $tokens } токена
    } | { $atas ->
        [one] { $atas } порожній ATA
        [few] { $atas } порожні ATA
        [many] { $atas } порожніх ATA
       *[other] { $atas } порожнього ATA
    }
tools-consolidation-transferred-native = { $amount } переведено на основний гаманець
tools-consolidation-transferred-tokens =
    { $count ->
        [one] На основний гаманець переведено { $count } токен
        [few] На основний гаманець переведено { $count } токени
        [many] На основний гаманець переведено { $count } токенів
       *[other] На основний гаманець переведено { $count } токена
    }
tools-consolidation-cleaned =
    { $count ->
        [one] Закрито { $count } ATA, повернуто { $amount }
        [few] Закрито { $count } ATA, повернуто { $amount }
        [many] Закрито { $count } ATA, повернуто { $amount }
       *[other] Закрито { $count } ATA, повернуто { $amount }
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = Токен
tools-multi-mint-label = Адреса мінта токена
tools-multi-mint-input =
    .placeholder = Вставте адресу мінта токена...
tools-multi-execution-title = Параметри виконання
tools-multi-delay-min-label = Мін. затримка
tools-unit-native = { -sol }
tools-unit-seconds = с
tools-unit-ms = мс
tools-multi-delay-max-label = Макс. затримка
tools-multi-concurrency-label = Паралельність
tools-multi-concurrency-sequential = { $count } (послідовно)
tools-multi-concurrency-parallel = { $count } паралельно
tools-multi-slippage-label = Проковзування
tools-multi-router-label = Маршрутизатор
tools-multi-router-auto = Авто (найкращий маршрут)
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = Прямий пул
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = Хід виконання
tools-multi-progress-preparing = Підготовка...
tools-multi-status-line = { $label } ({ $completed }/{ $total })
tools-multi-column-wallet = Гаманець
tools-multi-column-route = Маршрут
tools-multi-column-status = Статус
tools-multi-op-completed = Завершено
tools-multi-op-failed = Помилка
tools-multi-action-stop = Зупинити
tools-multi-action-loading = Завантаження...
tools-multi-start-failed = Не вдалося запустити: { $reason }

tools-multi-state-pending = Очікування
tools-multi-state-funding = Поповнення
tools-multi-state-executing = Виконання
tools-multi-state-consolidating = Консолідація
tools-multi-state-completed = Завершено
tools-multi-state-failed = Помилка
tools-multi-state-aborted = Перервано

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = Токен, який ви хочете купити з кількох гаманців
tools-multi-buy-wallets-title = Параметри гаманців
tools-multi-buy-wallet-count-label = Кількість гаманців
tools-multi-buy-wallet-count-option =
    { $count ->
        [one] { $count } гаманець
        [few] { $count } гаманці
        [many] { $count } гаманців
       *[other] { $count } гаманця
    }
tools-multi-buy-wallet-count-hint = Кількість субгаманців для використання
tools-multi-buy-buffer-label = Запас { -sol } на гаманець
tools-multi-buy-buffer-hint = Резерв на комісії (мін. 0.015 { -sol })
tools-multi-buy-amounts-title = Параметри сум
tools-multi-buy-min-label = Мін. { -sol } на гаманець
tools-multi-buy-min-hint = Мінімальна сума купівлі
tools-multi-buy-max-label = Макс. { -sol } на гаманець
tools-multi-buy-max-hint = Максимальна сума купівлі
tools-multi-buy-limit-label = Загальний ліміт { -sol } (необов’язково)
tools-multi-buy-limit-hint = Максимальна загальна сума витрат
tools-multi-buy-preview-title = Попередній перегляд
tools-multi-buy-preview-create = Гаманців для створення
tools-multi-buy-preview-amount = Сума на гаманець
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = Потрібно { -sol } загалом
tools-multi-buy-preview-balance = Баланс основного гаманця
tools-multi-buy-action-preview = Попередній перегляд
tools-multi-buy-action-start = Почати мультикупівлю
tools-multi-buy-executing = Виконання купівель...
tools-multi-buy-column-spent = Витрачено ({ -sol })
tools-multi-buy-column-tokens = Токени
tools-multi-buy-preview-failed = Помилка попереднього перегляду: { $reason }
tools-multi-buy-started = Мультикупівлю розпочато
tools-multi-buy-stopped = Мультикупівлю зупинено
tools-multi-buy-completed = Мультикупівлю завершено! Успішно: { $successful }/{ $total }

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = Введіть адресу токена, щоб знайти гаманці, які ним володіють
tools-multi-sell-action-scan = Сканувати
tools-multi-sell-settings-title = Параметри продажу
tools-multi-sell-percent-label = Відсоток продажу
tools-multi-sell-percent-hint = % токенів для продажу з кожного гаманця
tools-multi-sell-min-fee-label = Мін. { -sol } на комісію
tools-multi-sell-min-fee-hint = Мінімум { -sol } для комісії транзакції
tools-multi-sell-topup-label = Автопоповнення за потреби
tools-multi-sell-topup-hint = Переказувати { -sol } з основного гаманця, якщо на субгаманці недостатньо коштів
tools-multi-sell-post-title = Дії після продажу
tools-multi-sell-consolidate-label = Консолідувати { -sol } на основний гаманець
tools-multi-sell-consolidate-hint = Повернути весь { -sol } із субгаманців на основний гаманець
tools-multi-sell-close-atas-label = Закривати ATA токена після продажу
tools-multi-sell-close-atas-hint = Повернення ~0.002 { -sol } за кожен ATA
tools-multi-sell-wallets-title = Гаманці з токеном
tools-multi-sell-empty = Жоден субгаманець не володіє цим токеном
tools-multi-sell-column-tokens = Токени
tools-multi-sell-column-native = Баланс ({ -sol })
tools-multi-sell-column-topup = Потрібне поповнення
tools-multi-sell-none-selected = Гаманці не вибрано
tools-multi-sell-select-required = Виберіть принаймні один гаманець
tools-multi-sell-action-start = Почати мультипродаж
tools-multi-sell-executing = Виконання продажів...
tools-multi-sell-column-sold = Продано токенів
tools-multi-sell-column-received = Отримано ({ -sol })
tools-multi-sell-started = Мультипродаж розпочато
tools-multi-sell-stopped = Мультипродаж зупинено
tools-multi-sell-completed = Мультипродаж завершено! Отримано { $amount }

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = Обране
tools-favorites-saved = Збережене обране
tools-favorites-save-current = Зберегти поточне
tools-favorites-empty = Обраного ще немає
tools-favorites-no-label = Без позначки
tools-favorites-uses = { $count }x
tools-favorites-remove = Видалити
tools-favorites-loaded = Завантажено з обраного: { $name }
tools-favorites-default-name = Конфігурація
tools-favorites-mint-required = Спершу введіть адресу мінта токена
tools-favorites-add-title = Додати до обраного
tools-favorites-add-message = Введіть позначку для цього запису
tools-favorites-add-placeholder = Позначка (необов’язково)...
tools-favorites-saved-toast = Збережено в обране
tools-favorites-save-failed = Не вдалося зберегти в обране
tools-favorites-remove-title = Видалити з обраного
tools-favorites-remove-message = Видалити цей запис з обраного?
tools-favorites-removed-toast = Запис видалено з обраного
tools-favorites-remove-failed = Не вдалося видалити запис з обраного
