# Trade dialog messages.

## Quote panel

# Shown in the quote panel when the quote request could not reach the core.
trade-quote-error-network = Не вдалося отримати котирування — перевірте з’єднання й спробуйте ще раз
# Fallback title when the quote request failed without a message.
trade-quote-error-title = Не вдалося отримати котирування
trade-quote-title = Попередній перегляд свопу
trade-quote-refresh =
    .aria-label = Оновити котирування
    .title = Оновити котирування
trade-quote-idle = Виберіть суму, щоб переглянути своп
trade-quote-loading = Пошук найкращого маршруту…
trade-quote-retry = Спробувати ще раз
trade-quote-pay = Ви платите
trade-quote-receive = Ви отримуєте (орієнтовно)
trade-quote-minimum = Гарантований мінімум
    .title = Найменше, що ви можете отримати після максимального проковзування. Своп буде скасовано, а не виконано за гіршою ціною.
trade-quote-impact = Вплив на ціну
trade-quote-slippage = Макс. проковзування
trade-quote-platform-fee = Комісія платформи
    .title = 0,5% — підтримує розробку. Уже враховано в котируванні вище.
trade-quote-network-fee = Комісія мережі
trade-quote-route = Маршрут
trade-quote-disclaimer = Ціни оновлюються наживо з мережі. Своп скасовується, якщо не може виконатися вище вашого гарантованого мінімуму, тож ви ніколи не отримаєте менше, ніж показано.
# Price impact below the resolution of the percentage display.
trade-quote-impact-tiny = { "<0.01%" }
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-quote-impact-warning = Вплив на ціну { $impact } перевищує ваше максимальне проковзування { $tolerance }% — такий розмір зрушує пул. Менша сума виконується ближче до ринкової ціни.

## Units

trade-unit-native = { -sol }
trade-unit-tokens = токенів

## Actions. Ids are the dialog actions: buy, sell, add.

trade-buy-title = Купити токен
trade-buy-subtitle = Введіть суму в { -sol }
trade-buy-confirm = Виконати купівлю
trade-buy-hint = Залиште порожнім для значення з конфігурації
trade-sell-title = Продати позицію
trade-sell-subtitle = Виберіть відсоток продажу
trade-sell-confirm = Виконати продаж
trade-sell-hint = Введіть значення від 1 до 100
trade-sell-input = Власний відсоток
    .placeholder = 1-100
trade-add-title = Докупити до позиції
trade-add-subtitle = DCA в наявну позицію
trade-add-confirm = Докупити
trade-add-hint = Залиште порожнім для налаштованого розміру DCA
trade-amount-input = Власна сума
    .placeholder = Введіть суму в { -sol }

## Presets

trade-presets-quick-amount = Швидка сума
trade-presets-quick-sell = Швидкий продаж
trade-presets-match-entry = Як при вході
trade-presets-fixed-amount = Фіксована сума
trade-preset-partial = Частина
trade-preset-half = Половина
trade-preset-most = Більша частина
trade-preset-full = Повний вихід
# $label is the preset's amount.
trade-preset-select =
    .aria-label = Вибрати { $label }

## Dialog chrome

trade-dialog-close =
    .aria-label = Закрити вікно
trade-input-max = МАКС
    .aria-label = Використати максимум
trade-slider =
    .aria-label = Повзунок суми
trade-context-available = Доступно
trade-context-position-size = Розмір позиції
trade-context-holdings = Утримувані токени
trade-held-badge = Утримується
    .title = У вас є відкрита позиція за цим токеном
trade-manage-title = Ручне керування
trade-manage-description = Автотрейдер не продаватиме цю позицію й не докупатиме до неї. Зніміть позначку, щоб він керував виходами.

## Slippage

trade-slippage-label = Проковзування
trade-slippage-presets =
    .aria-label = Шаблон проковзування
trade-slippage-auto = Авто
trade-slippage-custom =
    .placeholder = Власне
    .aria-label = Власне проковзування у відсотках
trade-slippage-note-auto = Авто (з налаштувань)
# $pct is the configured slippage as stored.
trade-slippage-note-auto-value = Авто ({ $pct }% з налаштувань)
# $pct is the override as typed.
trade-slippage-note-override = Перевизначено: { $pct }%
# $pct is the override as typed.
trade-slippage-warning = Високе проковзування: ви можете отримати до { $pct }% менше, ніж у котируванні.
trade-impact-warning-title = Попередження про високий вплив на ціну
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-impact-warning-text = Ця угода має вплив на ціну <strong>{ $impact }</strong>, що перевищує вашу допустиму межу проковзування <strong>{ $tolerance }%</strong>. Ви можете отримати значно менше, ніж очікувалося.
trade-impact-warning-proceed = Усе одно продовжити

## Validation and verification

trade-error-invalid-number = Недійсне число
trade-error-percentage-range = Відсоток має бути від 1 до 100
trade-error-amount-positive = Сума має бути більшою за 0
trade-error-amount-minimum = Мінімум: 0,001 { -sol }
# $needed is a formatted SOL amount, $reserve the fee headroom in SOL and $balance the formatted balance.
trade-error-insufficient = Недостатньо коштів (потрібно { $needed } плюс { $reserve } на комісії, є { $balance })
trade-error-position-closed = Ця позиція більше не відкрита.
trade-error-verify-failed = Не вдалося перевірити баланс токена
trade-error-position-missing = Позицію не знайдено — можливо, її закрито
# $expected and $current are formatted token amounts.
trade-error-balance-changed = Баланс токена змінився. Очікувалося { $expected }, зараз { $current }. Оновіть, будь ласка.
trade-error-verify-network = Помилка мережі під час перевірки балансу

## Quick trade

trade-quick-buy-title = Швидка купівля
trade-quick-sell-title = Швидкий продаж
trade-quick-subtitle = Введіть адресу мінта токена
trade-quick-mint-label = Введіть адресу мінта токена
trade-quick-mint-input =
    .placeholder = Введіть адресу мінта або шукайте за символом...
trade-quick-paste =
    .aria-label = Вставити з буфера обміну
trade-quick-recent = Нещодавні:
trade-quick-fetching = Отримання даних токена...
trade-quick-continue = Продовжити
trade-quick-token-not-found = Токен не знайдено
trade-quick-token-failed = Не вдалося отримати токен
trade-quick-token-not-in-database = Токен не знайдено в базі даних
trade-quick-token-info-failed = Не вдалося отримати дані токена
trade-quick-no-position = Для цього токена позицію не знайдено
trade-quick-no-holdings = У позиції не залишилося токенів
trade-quick-position-failed = Не вдалося отримати дані позиції

## Manual trade toasts

trade-toast-no-mint = Адреса мінта недоступна
trade-toast-open-failed = Не вдалося відкрити вікно угоди
trade-toast-pending-buy = Купівля ще виконується
trade-toast-pending-add = Докупівля ще виконується
trade-toast-pending-sell = Продаж ще виконується
trade-toast-pending-message = Браузер перестав чекати; результат дивіться в рядку позиції
trade-toast-failed-buy = Купівля не вдалася
trade-toast-failed-add = Докупівля до позиції не вдалася
trade-toast-failed-sell = Продаж не вдався

# Trade and close reasons. Ids are the Debug names of TradeReason
# (src/trader/types.rs) and the reasons written by src/positions.
trade-reason-strategy-signal = Сигнал стратегії
trade-reason-manual-entry = Вхід вручну
trade-reason-force-buy = Примусова купівля
trade-reason-copy-buy = Купівля копіюванням
trade-reason-dca-scheduled = Запланований DCA
trade-reason-take-profit = Тейк-профіт
trade-reason-stop-loss = Стоп-лос
trade-reason-trailing-stop = Трейлінг-стоп
trade-reason-time-override = Часове перевизначення
trade-reason-strategy-exit = Вихід за стратегією
trade-reason-llm-analysis-exit = Вихід за аналізом LLM
trade-reason-manual-exit = Вихід вручну
trade-reason-risk-management = Управління ризиками
trade-reason-blacklisted = У чорному списку
trade-reason-force-sell = Примусовий продаж
trade-reason-copy-sell = Продаж копіюванням
trade-reason-closed-externally = Закрито ззовні
trade-reason-wallet-history = Історія гаманця
trade-reason-exit-retry-pending = Очікує повтору виходу
trade-reason-synthetic-exit-permanent-failure = Постійна помилка синтетичного виходу
# $reason is the label of the base reason. Applies to a closed_reason that
# carries the pending-verification suffix.
trade-reason-pending-verification = { $reason } (очікує перевірки)
# $note is the operator text of a force close.
trade-reason-force-closed = Примусово закрито: { $note }
# $reason is a stored closed_reason that has no label; it is shown as stored.
trade-reason-stored = { $reason }

# Toast shown when a quick-trade shortcut runs without a token selected (ui/quick_trade_shortcuts.js).
trade-quick-no-token = Токен не вибрано
