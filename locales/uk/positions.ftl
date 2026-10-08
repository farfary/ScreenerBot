# Position details labels.

# State reasons. Ids come from POSITION_CREATED_REASON in src/positions/database/types.rs.
positions-state-reason-position-created = Позицію створено


# Source: scripts/pages/positions.js

## Views, origin and toolbar

# Ids are the position status values (POSITION_STATUS_LABELS, ui/position_status.js).
positions-status-open = Відкриті
positions-status-closed = Закриті
positions-status-archived = В архіві
# Empty positions table per status (POSITION_EMPTY_LABELS)
positions-open-empty = Немає відкритих позицій
    .message = Позиція з’являється тут, коли її відкриває автотрейдер або ручна купівля.
positions-closed-empty = Немає закритих позицій
    .message = Позиція переходить сюди після повного продажу.
positions-archived-empty = Немає архівних позицій
    .message = Позиції, прибрані через «Архівувати», зберігаються тут.
positions-origin-copy = Копіювання
positions-origin-manual = Вручну
positions-origin-wallet = Гаманець
positions-origin-copy-link =
    .title = Відкрити копі-завдання, яке відкрило цю позицію
positions-holding-frozen = Заморожено
    .title = Повноваження заморожування мінта заблокували цей токен-акаунт — баланс не можна переказати чи продати
positions-toolbar-total = Усього
positions-toolbar-delete-all = Видалити все
positions-search-placeholder = Пошук за символом або мінтом...
positions-filter-origin = Джерело
positions-filter-origin-all = Усі джерела
positions-filter-origin-auto = Автотрейдер
positions-filter-origin-copy = Копітрейдинг
positions-delete-all-tooltip = Остаточно видалити всі архівні позиції

## Columns

positions-column-token = Токен
positions-column-archived-at = Архівовано
positions-column-entry-time = Час входу
positions-column-exit-time = Час виходу
positions-column-avg-entry = Сер. вхід ({ -sol })
positions-column-avg-exit = Сер. вихід ({ -sol })
positions-column-current-price = Поточна ({ -sol })
positions-column-total-invested = Усього інвестовано
positions-column-proceeds = Виручка
positions-column-pnl = Прибуток/збиток
positions-column-pnl-percent = Прибуток/збиток %
positions-column-size = Розмір
positions-column-dca = DCA
positions-column-exits = Виходи
positions-column-unrealized-pnl = Нереалізований прибуток/збиток
positions-column-unrealized-percent = Нереалізований %

## Cells

# Shown instead of a figure the wallet history cannot support.
positions-unknown-basis = Немає собівартості в історії цього гаманця (аірдроп, заповнення в USD або своп без ноги в SOL)
positions-unknown-history = Цей раунд не збігається з ончейн-балансом
positions-dca-count =
    { $count ->
        [one] { $count } DCA
        [few] { $count } DCA
        [many] { $count } DCA
       *[other] { $count } DCA
    }
positions-exit-count =
    { $count ->
        [one] { $count } вихід
        [few] { $count } виходи
        [many] { $count } виходів
       *[other] { $count } виходу
    }

## Row actions

positions-action-add =
    .title = Докупити до позиції (DCA)
    .aria-label = Докупити до позиції
positions-action-sell =
    .title = Продати (повністю або частково у %)
    .aria-label = Продати позицію
positions-action-sell-frozen = Заморожено повноваженнями мінта — цей токен не можна продати
positions-action-remove =
    .title = Прибрати (архівувати або видалити)
    .aria-label = Прибрати позицію
positions-action-restore =
    .title = Відновити у відкриті/закриті
    .aria-label = Відновити позицію
positions-action-delete =
    .title = Видалити остаточно
    .aria-label = Видалити остаточно
positions-action-in-progress = Виконується…

## Live state of a row

positions-caption-buying = Купівля
# $step is the label of the current action step.
positions-caption-buying-step = Купівля · { $step }
positions-caption-selling = Продаж
positions-caption-selling-step = Продаж · { $step }
positions-caption-closing = Закриття
positions-caption-failed = Помилка
# $error is the failure text of the action.
positions-caption-failed-detail = Помилка · { $error }
positions-step-adding = Докупівля
positions-pending-buying = Купівля…
positions-pending-buy-failed = Купівля не вдалася

## Messages and confirmations

positions-load-failed = Не вдалося оновити позиції
positions-toast-not-found = Дані позиції не знайдено
positions-toast-deleted = Позицію видалено
positions-toast-archived = Позицію архівовано
positions-toast-restored = Позицію відновлено
positions-buy-adds-to-archived = У цього токена вже є відкрита позиція в архіві. Купівля додасться до цієї позиції, і вона повернеться до відкритих позицій. Режим керування позицією зберігається.
positions-action-failed = Дія не вдалася
positions-delete-title = Остаточно видалити позицію
# $symbol is the token symbol.
positions-delete-message = Остаточно видалити { $symbol }? Позицію та її історію буде видалено з бази даних без можливості відновлення. Ваші транзакції та дані токена не зміняться.
positions-delete-confirm = Видалити остаточно
positions-delete-all-title = Видалити всі архівні позиції
positions-delete-all-message =
    { $count ->
        [one] Остаточно видалити { $count } архівну позицію? Це не можна скасувати. Транзакції та дані токенів не зміняться.
        [few] Остаточно видалити { $count } архівні позиції? Це не можна скасувати. Транзакції та дані токенів не зміняться.
        [many] Остаточно видалити { $count } архівних позицій? Це не можна скасувати. Транзакції та дані токенів не зміняться.
       *[other] Остаточно видалити { $count } архівної позиції? Це не можна скасувати. Транзакції та дані токенів не зміняться.
    }
positions-delete-all-message-empty = Остаточно видалити всі архівні позиції? Це не можна скасувати.
positions-delete-all-confirm = Видалити все
positions-delete-all-done =
    { $count ->
        [one] Видалено { $count } архівну позицію
        [few] Видалено { $count } архівні позиції
        [many] Видалено { $count } архівних позицій
       *[other] Видалено { $count } архівної позиції
    }
positions-delete-all-failed = Не вдалося видалити архівні позиції

# Source: scripts/ui/position_remove_dialog.js

## Remove position dialog

positions-remove-title = Прибрати позицію
# Inline markup: emphasis on the opening sentence and on "not".
positions-remove-open-warning = <strong>Ця позиція ще відкрита.</strong> Бот утримує цей токен. Якщо її прибрати, слот угоди звільниться, а відстеження припиниться, але токен <strong>не</strong> буде продано. Спершу продайте його, якщо хочете повернути { -sol }.
positions-remove-modes =
    .aria-label = Режим видалення
positions-remove-archive = Архівувати
positions-remove-recommended = Рекомендовано
positions-remove-archive-description = Сховати у вкладку «В архіві». Можна відновити будь-коли — нічого не продається, а всі угоди залишаються в історії.
positions-remove-delete = Видалити остаточно
positions-remove-delete-description = Стерти цю позицію та всю її історію з бази даних.
# Inline markup: emphasis on the irreversibility sentence.
positions-remove-danger = Позицію та її історію буде видалено остаточно. <strong>Це не можна скасувати.</strong> Ваші транзакції та дані токена не зміняться.
positions-remove-confirm-archive = Архівувати позицію

# Source: scripts/ui/position_details_dialog.js, scripts/ui/position_details/panes.js

## Position details frame

# Message shown after a management change. $mode is the label of the new mode.
positions-management-changed = Керування позицією: { $mode }
positions-details-load-failed = Не вдалося завантажити деталі позиції
positions-details-mint-label = Адреса мінта
positions-details-management-failed = Не вдалося змінити керування позицією
positions-details-favorite-add =
    .title = Додати до обраного
    .aria-label = Додати до обраного
positions-details-favorite-remove =
    .title = Прибрати з обраного
    .aria-label = Прибрати з обраного
positions-details-view-solscan =
    .title = Переглянути в { -solscan }
    .aria-label = Переглянути токен в { -solscan }
positions-details-close =
    .title = Закрити (Esc)
    .aria-label = Закрити
positions-details-chart-section =
    .aria-label = Графік ціни
positions-details-loading-chart = Завантаження графіка...
positions-details-activity-section =
    .aria-label = Активність
positions-details-activity-title = Активність
positions-details-split-handle =
    .aria-label = Змінити розмір графіка й активності
positions-details-activity-pane =
    .aria-label = Панель активності
positions-details-activity-expand =
    .title = Розгорнути активність
    .aria-label = Розгорнути активність
positions-details-summary-section =
    .aria-label = Підсумок позиції
positions-details-loading = Завантаження позиції...

## Management modes. Ids are the PositionManagement serde ids (src/positions/types.rs).

positions-management-auto-trader = Автотрейдер
positions-management-user-only = Лише користувач
positions-management-copy-task = Копі-завдання
positions-management-hybrid = Гібридне
positions-pane-show-chart = Показати графік
positions-pane-show-activity = Показати активність
positions-pane-restore-activity = Відновити активність
positions-pane-expand-chart =
    .title = Розгорнути графік
    .aria-label = Розгорнути графік

# Source: scripts/ui/position_details/header.js

## Position details header

positions-risk-low = Низький ризик
positions-risk-medium = Середній ризик
positions-risk-high = Високий ризик
positions-risk-unknown = Ризик невідомий
positions-busy-buying = Триває купівля…
positions-busy-selling = Триває продаж…
positions-busy-closing = Триває закриття…
positions-header-avg-entry = Сер. вхід
# $count is the number of buys: the entry plus each add.
positions-header-buy-count =
    { $count ->
        [one] { $count } купівля
        [few] { $count } купівлі
        [many] { $count } купівель
       *[other] { $count } купівлі
    }
positions-header-exit-price = Ціна виходу
# $ago is the elapsed time since the close, for example "3h ago".
positions-header-closed-ago = закрито { $ago }
positions-header-realized-pnl = Реалізований прибуток/збиток
positions-header-usd-note = USD за сьогоднішнім курсом { -sol }
positions-header-returned = Повернуто
# $amount is the formatted SOL amount invested.
positions-header-of-invested = з інвестованих { $amount }
positions-header-price = Ціна
positions-header-last-price = Остання ціна
positions-header-pool-ago = пул · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = Нереалізований прибуток/збиток
positions-header-pnl-last-price = Прибуток/збиток за останньою ціною
positions-header-value = Вартість
positions-header-last-value = Остання вартість
positions-header-invested = інвестовано { $amount }
positions-header-origin-hint = Як було відкрито цю позицію
positions-header-risk-hint = Оцінка { -rugcheck } — чим нижча, тим безпечніше
positions-header-frozen = Заморожено
    .title = Повноваження мінта заморозили цей токен
positions-header-managed-by = Керує
positions-header-management-select =
    .aria-label = Керування позицією

## Entry origin shown in the header badge

positions-origin-unknown = невідомо
# $task is the copy task id. The source wallet follows in its own element.
positions-origin-copied-task = Копіювання · завдання { $task }
positions-origin-manual-entry = Вхід вручну
positions-origin-wallet-entry = Вхід із гаманця
# $strategy is the strategy id.
positions-origin-auto-strategy = Авто · { $strategy }
positions-origin-auto-entry = Автовхід

## Swaps that are submitted and not yet booked

positions-pending-adding = Докупівля
positions-pending-adding-amount = Докупівля { $amount }
positions-pending-selling = Продаж
# $percent is the formatted share of the position being sold.
positions-pending-selling-percent = Продаж { $percent }
# $label is the pending swap wording.
positions-pending-confirming = { $label } · підтвердження
    .title = Надіслано, очікує ончейн-підтвердження. Показники оновляться після перевірки.

## Trade controls

positions-trade-add = Докупити
    .title = Докупити до позиції
positions-trade-sell = Продати
    .title = Продати частину позиції
positions-trade-close = Закрити позицію
    .title = Продати все й закрити
positions-trade-token = Деталі токена
    .title = Відкрити деталі токена

## Favorites

positions-favorite-token-fallback = Токен
# $symbol is the token symbol.
positions-favorite-added = { $symbol } додано до обраного
positions-favorite-removed = { $symbol } прибрано з обраного
positions-favorite-add-failed = Не вдалося додати до обраного
positions-favorite-remove-failed = Не вдалося прибрати з обраного
positions-favorite-update-failed = Не вдалося оновити обране

# Source: scripts/ui/position_details/summary.js

## Summary rail

positions-summary-position = Позиція
positions-summary-price-path = Шлях ціни
positions-summary-network-fees = Комісії мережі
positions-summary-risk = Ризик
positions-summary-market = Ринок
positions-summary-market-now = Ринок зараз
positions-summary-links = Посилання
positions-fact-tokens-fallback = токенів
positions-fact-bought = Куплено
positions-fact-holding = Утримується
positions-fact-sold = Продано
positions-fact-realized = Реалізовано
positions-fact-opened = Відкрито
positions-fact-closed = Закрито
positions-fact-reason = Причина
positions-fact-archived = Архівовано
positions-fact-entry = Вхід
positions-fact-exit = Вихід
positions-fact-total = Усього
positions-fact-verified = Перевірено в мережі
positions-fact-confirming = Підтвердження
# $percent is the formatted share, for example "12.5%".
positions-fact-share-of-bought = { $percent } від купленого
positions-fact-share-of-invested = { $percent } від інвестованого
# $count is the number of adds after the entry.
positions-fact-entry-count =
    { $count ->
        [0] 1 вхід
        [one] 1 вхід + { $count } докупівля
        [few] 1 вхід + { $count } докупівлі
        [many] 1 вхід + { $count } докупівель
       *[other] 1 вхід + { $count } докупівлі
    }
# $count is the number of partial exits, $returned the formatted SOL amount.
positions-fact-partial-exits-back =
    { $count ->
        [one] { $count } частковий вихід · повернуто { $returned }
        [few] { $count } часткові виходи · повернуто { $returned }
        [many] { $count } часткових виходів · повернуто { $returned }
       *[other] { $count } часткового виходу · повернуто { $returned }
    }
# $age is the elapsed time of the hold.
positions-fact-held = утримується { $age }
# $percent is the signed change against the entry price.
positions-fact-vs-entry = { $percent } до входу
positions-fact-exit-vs-peak = Вихід до піку
positions-fact-now-vs-peak = Зараз до піку
positions-fact-entry-range = Діапазон входу
positions-range-low = Мінімум
positions-range-peak = Пік
positions-range-now = Зараз
positions-range-label-exit = Ціна входу та виходу між мінімумом і піком
positions-range-label-now = Ціна входу та поточна ціна між мінімумом і піком
positions-fact-mint-authority = Повноваження мінта
positions-fact-freeze-authority = Повноваження заморожування
positions-fact-active = Активні
positions-fact-pool = Пул
# $amount is the formatted liquidity in SOL.
positions-fact-pool-liquidity = ліквідність { $amount } { -sol }
positions-fact-market-cap = Ринкова капіталізація
# $value is the formatted fully diluted valuation in USD.
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = Ліквідність
positions-fact-volume-24h = Обсяг за 24 год
positions-fact-price-change = Зміна ціни
positions-change-period-1h = 1 год
positions-change-period-24h = 24 год
positions-fact-holders = Холдери
positions-link-website = Вебсайт
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

# Source: scripts/ui/position_details/activity.js, scripts/ui/position_details/activity_event.js

## Activity

positions-activity-load-failed = Не вдалося завантажити активність
positions-activity-loading = Завантаження активності...
positions-activity-empty = З цим токеном у цьому гаманці ще нічого не відбувалося
positions-activity-filter-empty = Немає активності, що відповідає фільтру
positions-activity-round-count =
    { $count ->
        [one] { $count } раунд
        [few] { $count } раунди
        [many] { $count } раундів
       *[other] { $count } раунду
    }
positions-activity-event-count =
    { $count ->
        [one] { $count } подія
        [few] { $count } події
        [many] { $count } подій
       *[other] { $count } події
    }
positions-activity-pending-count = В очікуванні: { $count }
positions-activity-failed-count = Невдало: { $count }
positions-filter-all = Усі
positions-filter-trades = Угоди
positions-filter-buys = Купівлі
positions-filter-sells = Продажі
positions-filter-wallet = Гаманець
positions-filter-issues = Проблеми
positions-activity-filters =
    .aria-label = Фільтр активності
positions-activity-totals =
    .aria-label = Усі раунди за цим токеном
positions-activity-realized-all = Реалізовано, усі раунди
positions-activity-invested = Інвестовано
positions-activity-returned = Повернуто
# $when is the formatted open time of a round that has not closed.
positions-activity-opened = Відкрито { $when }
# $index is the 1-based number of the round.
positions-activity-round-title = Позиція { $index }
positions-activity-this-position = Ця позиція
positions-activity-dates-unavailable = Дати недоступні
positions-activity-wallet-title = Транзакції гаманця
# $range is the date range, $count the number of events.
positions-activity-outside =
    { $count ->
        [one] Поза позиціями · { $range } · { $count } подія
        [few] Поза позиціями · { $range } · { $count } події
        [many] Поза позиціями · { $range } · { $count } подій
       *[other] Поза позиціями · { $range } · { $count } події
    }
positions-details-signature-label = Підпис

## State history milestones. Ids are the PositionState names (src/positions/database/types.rs).

positions-state-open = Позиція відкрита
positions-state-closing = Позиція закривається
positions-state-closed = Позиція закрита
positions-state-exit-pending = Вихід із позиції очікує
positions-state-exit-failed = Вихід із позиції не вдався
positions-state-phantom = Фантомна позиція
positions-state-reconciling = Позиція звіряється

## Activity events

positions-event-kind-entry = Вхід
positions-event-kind-dca = Докупівля
positions-event-kind-partial-exit = Частковий вихід
positions-event-kind-exit = Вихід
positions-event-kind-buy = Купівля гаманця
positions-event-kind-sell = Продаж гаманця
positions-event-kind-transfer = Переказ
positions-event-kind-ata = Токен-акаунт
positions-event-kind-other = Транзакція
positions-event-state-pending = В очікуванні
positions-event-state-failed = Невдало
positions-event-state-synthetic = Синтетична
# $error is the failure text reported by the chain.
positions-chain-status-failed-detail = Невдало: { $error }
positions-event-tokens-fallback = токенів
# In the descriptions below $amount is the token amount with its symbol, $sol the SOL amount
# and $percent the share of the position sold.
positions-event-entry-submitted = Надіслано купівлю на { $amount }
positions-event-entry-for = Куплено { $amount } за { $sol }
positions-event-entry = Куплено { $amount }
positions-event-dca-submitted = Надіслано докупівлю на { $amount }
positions-event-dca-for = Докуплено { $amount } за { $sol }
positions-event-dca = Докуплено { $amount }
positions-event-partial-exit-submitted-percent = Надіслано частковий вихід ({ $percent }) на { $amount }
positions-event-partial-exit-submitted = Надіслано частковий вихід на { $amount }
positions-event-sold-percent-for = Продано { $amount } ({ $percent }) за { $sol }
positions-event-sold-percent = Продано { $amount } ({ $percent })
positions-event-sold-for = Продано { $amount } за { $sol }
positions-event-sold = Продано { $amount }
positions-event-exit-submitted = Надіслано повний вихід із позиції
positions-event-exit-for = Закрито: продано { $amount } за { $sol }
positions-event-exit-closed = Позицію закрито
positions-event-wallet-bought = Гаманець купив { $amount } деінде
positions-event-wallet-sold = Гаманець продав { $amount } деінде
positions-event-received = Отримано { $amount }
positions-event-sent = Надіслано { $amount }
positions-event-transferred = Переказано { $amount }
positions-event-ata = Активність токен-акаунта
positions-event-wallet-transaction = Транзакція гаманця з { $amount }
# $price is the formatted price per token in SOL.
positions-event-price-per-token = { $price } { -sol } / токен
# $amount is the signed SOL change of the wallet.
positions-event-wallet-change = зміна гаманця: { $amount }
positions-event-after-title = Позиція після цієї події
positions-event-capital-invested = Інвестований капітал
positions-event-average-entry = Середній вхід
positions-event-transfers-title = Перекази токенів
positions-event-transfer-amount = Сума
positions-event-transfer-mint = Мінт
positions-event-transfer-from = Від
positions-event-transfer-to = До
positions-event-no-signature = Немає ончейн-підпису
positions-event-click-to-copy = Натисніть, щоб скопіювати
positions-event-solscan = { -solscan }
positions-event-token-amount = Кількість токенів
positions-event-trade-price = Ціна угоди
positions-event-native-amount = Сума в { -sol }
positions-event-cost-basis = Собівартість
positions-event-usd-value = Вартість у USD
positions-event-network-fee = Комісія мережі
positions-event-router = Маршрутизатор
positions-event-slot = Слот
positions-event-chain-status = Статус у мережі
positions-event-transaction-type = Тип транзакції
positions-event-direction = Напрямок
positions-event-wallet-native-change = Зміна { -sol } гаманця
positions-event-instructions = Інструкції
positions-event-compute-units = Обчислювальні одиниці
positions-event-accounts = Акаунти
positions-event-record-id = ID запису
positions-event-time-unavailable = Час недоступний
positions-event-details = Деталі
positions-event-hide-details = Сховати деталі

# Source: scripts/ui/position_details/chart.js

## Position chart

positions-chart-type-candles = Свічки
positions-chart-type-line = Лінія
positions-chart-type-area = Область
positions-chart-type-group =
    .aria-label = Тип графіка
positions-chart-overlays-group =
    .aria-label = Накладання графіка
positions-chart-ema = EMA
    .title = Експоненційні ковзні середні, 9 і 21
positions-chart-fit = Вмістити
    .title = Показати весь життєвий цикл цієї позиції
positions-chart-timeframes-group =
    .aria-label = Таймфрейм
positions-chart-pane-group =
    .aria-label = Панель графіка
positions-chart-unavailable = Рушій графіків недоступний
positions-chart-collecting = Збір даних графіка…
positions-chart-no-data = Даних графіка для цього токена ще немає
positions-chart-avg-entry = Сер. вхід
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = Сер. вхід
positions-chart-legend-avg-entry-off-scale = Сер. вхід (поза шкалою)
positions-chart-dropped-events =
    { $count ->
        [one] { $count } подія без свічки на цьому таймфреймі
        [few] { $count } події без свічки на цьому таймфреймі
        [many] { $count } подій без свічки на цьому таймфреймі
       *[other] { $count } події без свічки на цьому таймфреймі
    }
positions-chart-level = Рівень
# $label names the reference level, $price is its formatted price.
positions-chart-level-above = { $label } { $price } вище цього вікна
positions-chart-level-below = { $label } { $price } нижче цього вікна
positions-chart-scale-hint = Потягніть цінову вісь, щоб зменшити масштаб до нього
positions-chart-pnl-at-bar = Прибуток/збиток на свічці
positions-chart-click-to-locate = Натисніть, щоб знайти
