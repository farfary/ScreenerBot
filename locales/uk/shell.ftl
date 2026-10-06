# Dashboard shell: header, ticker, notification drawer and status bar.

# Source: templates/base.html
# Document title: the page title, then the product name.
shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-brand-logo =
    .alt = { -brand }
# Version label; the number itself is passed as an argument.
shell-version = v{ $version }

## Header

shell-header-brand =
    .aria-label = Відкрити головну панелі керування
    .title = Головна панелі керування
shell-bot-card =
    .aria-label = Завантаження статусу автотрейдера
shell-bot-label = Авто
shell-bot-status-loading = ЗАВАНТАЖЕННЯ
shell-bot-today = Сьогодні
shell-explore-control =
    .aria-label = Режим огляду. Підключіть гаманець і RPC-ендпоінт, щоб увімкнути всі функції
    .title = Підключіть гаманець і RPC-ендпоінт, щоб увімкнути торгівлю, баланси та ончейн-дані в реальному часі
shell-explore-title = Режим огляду
shell-explore-detail = Гаманець і RPC не підключено
shell-explore-action = Завершити налаштування
shell-wallet-card =
    .aria-label = Вартість гаманця; відкрити «Позиції»
    .title = Вартість гаманця ({ -sol } + токени) · відкрити «Позиції»
shell-wallet-worth-label = ВАРТІСТЬ
shell-wallet-native-label = { -sol }
shell-wallet-tokens-label = ТОК.
shell-sol-price-card =
    .aria-label = Ціна { -sol } у USD — відкрити графік
    .title = Ціна { -sol } · натисніть, щоб відкрити графік
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24 год
shell-copy-card =
    .aria-label = Копітрейдинг; відкрити «Копітрейдинг»
    .title = Копітрейдинг · відкрити «Копітрейдинг»
shell-copy-label = КОПІТРЕЙДИНГ
shell-actions-more =
    .aria-label = Інші дії в заголовку
    .title = Інші дії
shell-actions-group =
    .aria-label = Дії в заголовку
shell-action-search =
    .aria-label = Пошук токенів
    .title = Пошук токенів (Ctrl/Cmd+K)
shell-action-featured =
    .aria-label = Рекомендовані токени
    .title = Рекомендовані токени
shell-action-notifications =
    .aria-label = Дії та сповіщення
    .title = Дії та сповіщення
shell-action-restart =
    .aria-label = Перезапустити застосунок
    .title = Перезапустити застосунок
shell-action-theme =
    .aria-label = Перемкнути тему
    .title = Перемкнути тему
shell-action-settings =
    .aria-label = Налаштування
    .title = Налаштування

## Ticker

shell-ticker-monitoring-segment =
    .title = Токени, які відстежує сервіс пулів
shell-ticker-monitoring = Відстеження:
shell-ticker-filtering-segment =
    .title = Токени, що пройшли або не пройшли критерії фільтрації
shell-ticker-passed = Пройшли:
shell-ticker-rejected = Відхилено:
shell-ticker-pnl-segment =
    .title = Реалізований прибуток і збиток за сьогодні
shell-ticker-pnl = Прибуток/збиток за сьогодні:
shell-ticker-rpc-segment =
    .title = RPC-виклики за хвилину та відсоток успіху
shell-ticker-rpc = RPC:
shell-ticker-rpc-rate = { $amount }/хв
shell-ticker-services-segment =
    .title = Стан працездатності фонових сервісів
shell-ticker-services-loading = Сервіси: <strong>Завантаження</strong>

## Notification drawer

shell-notification-title = Дії
shell-notification-mark-all-read =
    .title = Позначити все як прочитане
shell-notification-clear-all =
    .title = Очистити все
shell-notification-close =
    .aria-label = Закрити
shell-notification-tab-all = Усі
shell-notification-tab-active = Активні
shell-notification-tab-done = Виконані
shell-notification-tab-failed = Невдалі
shell-notification-filter-type-all = Усі типи
shell-notification-filter-type-buy = Купівля
shell-notification-filter-type-sell = Продаж
shell-notification-filter-type-open = Відкриття
shell-notification-filter-type-close = Закриття
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = Часткові
shell-notification-filter-state-all = Усі стани
shell-notification-filter-state-in-progress = Виконуються
shell-notification-filter-state-completed = Завершені
shell-notification-filter-state-failed = Невдалі
shell-notification-filter-state-cancelled = Скасовані
shell-notification-list =
    .aria-label = Сповіщення
shell-notification-empty = Дій ще немає
shell-notification-loading-more = Завантаження ще...
shell-notification-back-to-top =
    .title = Нагору

## Status bar

shell-status-bar-version = v
shell-status-bar-uptime = Аптайм
shell-status-bar-memory = Пам.
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/хв
shell-status-bar-trading = Торгівля
shell-status-bar-positions = Поз.
shell-status-bar-tokens = Токени

# Source: templates/pages/splash.html, scripts/core/splash.js

## Splash

shell-splash-starting = Запуск { -brand }
shell-splash-waiting = Очікування відповіді локального ядра.
shell-splash-failed = Не вдалося запустити { -brand }
shell-splash-failed-detail = Перевірте файл журналу, потім перезапустіть застосунок.

# Source: scripts/core/header.js, scripts/core/connectivity_watcher.js, scripts/core/router.js

## Connection state

shell-connection-connected = Ядро підключено
shell-connection-waiting = Очікування ядра…
shell-connection-retry-now = Повторити зараз
shell-connection-overlay-detail = Ядро недоступне. Торгівлю призупинено; з’єднання відновиться автоматично.
shell-connection-restored = З’єднання з ядром відновлено

# Source: scripts/core/header.js
shell-trader-control-failed = Не вдалося керувати трейдером
shell-notification-button-unread = Дії та сповіщення, непрочитаних: { $count }
shell-restart-confirm-title = Перезапустити бота
shell-restart-confirm-message =
    Ви впевнені, що хочете перезапустити бота?

    Це:
    • зупинить усі сервіси
    • перезапустить процес
    • триватиме ~10–15 с

    Усі активні операції буде перервано.
shell-restart-confirm-action = Перезапустити
shell-restart-progress = Перезапуск бота
shell-restart-failed = Не вдалося перезапустити
shell-restart-failed-status = Не вдалося перезапустити: { $status }
shell-restart-helper-unavailable = Помічник автоматичного перезапуску недоступний. Незабаром перезавантажте панель керування.

# Source: scripts/core/router.js
shell-page-title-fallback = Панель керування
shell-page-load-failed = Не вдалося завантажити сторінку
shell-page-offline-detail = Ядро зараз недоступне. Сторінка завантажиться автоматично, щойно з’єднання відновиться.

# Source: scripts/core/header_metrics.js

## Auto Trader card

shell-bot-state-explore = ОГЛЯД
shell-bot-state-halted = ЗУПИНЕНО
shell-bot-state-off = ВИМКНЕНО
shell-bot-state-waiting = ОЧІКУВАННЯ
shell-bot-state-idle = ПРОСТІЙ
shell-bot-state-entry-paused = ВХОДИ НА ПАУЗІ
shell-bot-state-running = ПРАЦЮЄ
shell-bot-control-explore = Автотрейдер недоступний у режимі огляду. Відкрийте налаштування гаманця та RPC.
shell-bot-control-halted = Екстрену зупинку активовано. Відкрийте керування автотрейдером.
shell-bot-control-off = Автотрейдер вимкнено. Натисніть, щоб увімкнути.
shell-bot-control-waiting = Автотрейдер увімкнено, він очікує на основні сервіси. Натисніть, щоб вимкнути.
shell-bot-control-idle = Автотрейдер увімкнено, але обидва монітори вимкнено. Відкрийте керування автотрейдером.
shell-bot-control-entry-paused = Захист від збитків призупинив входи; виходи можуть тривати. Відкрийте керування автотрейдером.
shell-bot-control-running = Автотрейдер працює. Натисніть, щоб вимкнути.

## Wallet and copy cards

shell-wallet-card-summary = Вартість гаманця: { $equity } { -sol } ({ $balance } { -sol } вільних, токенів: { $tokens }); відкрити «Позиції»
shell-copy-running-live = Реальних: { $count }
shell-copy-running-paper = Віртуальних: { $count }
shell-copy-value-paused = Призупинено
shell-copy-value-idle = Простій
shell-copy-sub-active = Активні: { $active } із { $total }

## Ticker services state

shell-ticker-services-healthy = Сервіси: <strong>Справні</strong>
shell-ticker-services-issues =
    { $count ->
        [one] Сервіси: <strong>{ $count } проблема</strong>
        [few] Сервіси: <strong>{ $count } проблеми</strong>
        [many] Сервіси: <strong>{ $count } проблем</strong>
       *[other] Сервіси: <strong>{ $count } проблеми</strong>
    }

# Source: scripts/core/agent_approvals.js

## Agent approval prompt

shell-agent-request-title = Запит від агента
shell-agent-request-client-fallback = Підключений агент
shell-agent-request-message = { $client } хоче виконати «{ $tool }» у { -brand }. Цей запит { $expiry }.
shell-agent-request-message-arguments = { $client } хоче виконати «{ $tool }» у { -brand }. Аргументи: { $summary }. Цей запит { $expiry }.
shell-agent-request-expires-minutes = втратить чинність через { $minutes } хв
shell-agent-request-expires-seconds = втратить чинність через { $seconds } с
shell-agent-request-approve = Схвалити
shell-agent-request-deny = Відхилити

# Source: scripts/core/utils.js, scripts/core/toast.js, scripts/ui/toast.js, scripts/ui/confirmation_dialog.js

## Toasts, dialogs and shared widgets

shell-toast-copied = { $label } скопійовано
shell-toast-copy-failed = Не вдалося скопіювати
shell-toast-still-running = Ще виконується — перевірте центр сповіщень
shell-toast-dismiss =
    .aria-label = Закрити
shell-confirm-title = Підтвердьте дію
shell-confirm-message = Ви впевнені?
shell-address-open-solscan = — відкрити в { -solscan }
shell-address-copy = Копіювати адресу

# Source: scripts/core/global_chat.js
shell-assistant-label = Асистент
shell-assistant-dialog =
    .aria-label = Асистент

# Source: scripts/core/status_bar.js
shell-status-bar-trading-active = Активна
shell-status-bar-trading-inactive = Неактивна

# Source: scripts/core/action_toasts.js

## Action toasts

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title }: скасовано
shell-action-swap-buy-live = Купівля
shell-action-swap-buy-done = Куплено
shell-action-swap-buy-failed = Купівля не вдалася
shell-action-swap-sell-live = Продаж
shell-action-swap-sell-done = Продано
shell-action-swap-sell-failed = Продаж не вдався
shell-action-position-open-live = Відкриття позиції
shell-action-position-open-done = Відкрито
shell-action-position-open-failed = Відкриття не вдалося
shell-action-position-close-live = Закриття позиції
shell-action-position-close-done = Закрито
shell-action-position-close-failed = Закриття не вдалося
shell-action-position-dca-live = Докупівля до позиції
shell-action-position-dca-done = Докуплено
shell-action-position-dca-failed = Докупівля не вдалася
shell-action-partial-exit-live = Частковий вихід
shell-action-partial-exit-done = Частковий вихід
shell-action-partial-exit-failed = Частковий вихід не вдався
shell-action-manual-order-live = Розміщення ордера
shell-action-manual-order-done = Ордер розміщено
shell-action-manual-order-failed = Ордер не вдався
shell-action-trade-live = Угода
shell-action-trade-done = Угоду виконано
shell-action-trade-failed = Угода не вдалася
shell-action-via-router = { $action } через { $router }
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = в обхід { $venue }
shell-action-cost-guard-avoiding-cost = в обхід { $venue } · { $cost }
shell-action-cost-guard-avoiding-unnamed = в обхід майданчика
shell-action-cost-guard-avoiding-unnamed-cost = в обхід майданчика · { $cost }
shell-action-cost-guard-avoided = { $outcome } · заощаджено { $cost } ренти на { $venue }
shell-action-cost-guard-avoided-unnamed = { $outcome } · заощаджено { $cost } ренти майданчика
shell-action-exit-full = Повний вихід
shell-action-exit-percent = Вихід { $percent }

## Exit dialog (ui/exit_dialog.js)

shell-exit-title = Закрити { -brand }?
shell-exit-description = Виберіть, як закрити застосунок
shell-exit-minimize = Згорнути в трей
shell-exit-minimize-detail = Продовжити роботу у фоновому режимі
shell-exit-quit = Вийти із застосунку
shell-exit-quit-detail = Закрити повністю й зупинити всі сервіси

## Image lightbox (ui/image_lightbox.js)

shell-lightbox-save =
    .title = Зберегти зображення
shell-lightbox-close =
    .title = Закрити (ESC)

## Theme control (scripts/theme.js)

shell-theme-light = Світла
shell-theme-dark = Темна
shell-theme-switch-to-light = Перемкнути на світлу тему
shell-theme-switch-to-dark = Перемкнути на темну тему
