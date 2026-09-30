## Reply keyboard words.

telegram-reply-status = Статус
telegram-reply-balance = Баланс
telegram-reply-positions = Позиції
telegram-reply-pause = Пауза
telegram-reply-resume = Відновити
telegram-reply-stop = Стоп
telegram-reply-stats = Статистика
telegram-reply-menu = Меню
telegram-reply-help = Довідка

## Inline keyboard buttons.

telegram-button-positions = Позиції
telegram-button-balance = Баланс
telegram-button-stats = Статистика
telegram-button-tokens = Токени
telegram-button-pause = Пауза
telegram-button-stop = Стоп
telegram-button-settings = Налаштування
telegram-button-refresh = Оновити
telegram-button-menu = Меню
telegram-button-back = Назад
telegram-button-back-to-menu = До меню
telegram-button-back-to-tokens = До токенів
telegram-button-cancel = Скасувати
telegram-button-close-all-positions = Закрити всі позиції
telegram-button-sell-percent = Продати { $percent }%
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = У чорний список
telegram-button-blacklist-symbol = У чорний список { $symbol }
telegram-button-close-position = Закрити позицію
telegram-button-confirm-close = Підтвердити закриття
telegram-button-confirm-close-all = Закрити ВСІ позиції
telegram-button-confirm-sell = Підтвердити продаж { $percent }%
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = ПІДТВЕРДИТИ ПРИМУСОВУ ЗУПИНКУ
telegram-button-confirm-buy = Купити { $amount } { -sol }
telegram-button-notifications = Сповіщення
telegram-button-trading = Торгівля
telegram-button-entry-monitor = Монітор входу
telegram-button-exit-monitor = Монітор виходу
telegram-button-auto-trading = Автоторгівля
telegram-button-force-stop = Примусова зупинка
telegram-button-notify-opened = Відкриті
telegram-button-notify-closed = Закриті
telegram-button-notify-partial = Часткові
telegram-button-notify-dca = DCA
telegram-button-notify-errors = Помилки
telegram-button-details = Деталі
telegram-button-position = Позиція
telegram-button-sell-more = Продати ще
telegram-button-more-dca = Ще DCA
telegram-button-history = Історія
telegram-button-status = Статус
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = Автентифікуватися знову
telegram-button-previous = Назад
telegram-button-next = Далі
telegram-button-passed = Пройдені
telegram-button-rejected = Відхилені
telegram-button-new-24h = Нові (24 год)
telegram-button-all-tokens = Усі токени
telegram-button-search-token = Пошук токена
telegram-button-filter-stats = Статистика фільтрів
telegram-button-refresh-stats = Оновити статистику
telegram-button-view-position = Переглянути позицію
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    Невідома команда: { $command }

    Скористайтеся /help, щоб переглянути доступні команди.
telegram-session-expired =
    <b>Сесія закінчилася</b>

    Скористайтеся /login, щоб автентифікуватися знову.
telegram-2fa-required =
    <b>Потрібна 2FA</b>

    Введіть 6-значний код із застосунку-автентифікатора.
telegram-account-locked =
    <b>Обліковий запис заблоковано</b>

    Забагато невдалих спроб.
    Спробуйте ще раз через { $seconds ->
        [one] { $seconds } секунду.
        [few] { $seconds } секунди.
        [many] { $seconds } секунд.
       *[other] { $seconds } секунди.
    }
telegram-code-invalid = Введіть дійсний 6-значний код.
telegram-authenticated =
    <b>Автентифіковано!</b>

    Тепер вам доступні команди бота.
telegram-wrong-code =
    <b>Неправильний код</b>

    { $remaining ->
        [one] Залишилася { $remaining } спроба.
        [few] Залишилися { $remaining } спроби.
        [many] Залишилося { $remaining } спроб.
       *[other] Залишилося { $remaining } спроби.
    }
telegram-auth-required =
    <b>Потрібна автентифікація</b>

    Введіть пароль, щоб продовжити.

    <i>Напишіть пароль і надішліть його.</i>
telegram-login-required =
    <b>Потрібен вхід</b>

    Введіть 6-значний код із застосунку-автентифікатора:
telegram-session-activated =
    <b>Сесію активовано</b>

    2FA не налаштовано. Ваша сесія тепер активна.

    <i>Порада: увімкніть 2FA в налаштуваннях безпеки для кращого захисту.</i>

## Chat discovery.

telegram-discovery-hello = Вітаю, { $name }!
telegram-discovery-default-name = користувачу
telegram-discovery-detected = <b>Чат виявлено!</b>
telegram-discovery-details =
    ID чату: <code>{ $chat_id }</code>
    Тип: { $chat_type }

    Перейдіть на панель керування { -brand } і натисніть на цей чат, щоб вибрати його.
telegram-chat-type-private = приватний чат
telegram-chat-type-group = група
telegram-chat-type-supergroup = супергрупа
telegram-chat-type-channel = канал

## Menus.

telegram-menu-title =
    <b>Панель керування</b>

    Виберіть опцію, щоб переглянути інформацію або керувати ботом.
telegram-menu-positions-empty =
    <b>Немає відкритих позицій</b>

    Очікування нових можливостей...
telegram-menu-positions-title = <b>Позиції ({ $count })</b>
telegram-menu-positions-hint = <i>Натисніть на позицію, щоб керувати нею.</i>
telegram-menu-settings =
    <b>Налаштування</b>

    Налаштуйте сповіщення й торгові параметри.
telegram-settings-notifications =
    <b>Налаштування сповіщень</b>

    Вмикайте й вимикайте сповіщення:
telegram-settings-trading =
    <b>Керування торгівлею</b>

    Вмикайте й вимикайте торгові функції:
telegram-pagination-expired = Сесія посторінкового перегляду закінчилася.

## Status commands.

telegram-status-state-stopped = <b>ЗУПИНЕНО</b> (активна примусова зупинка)
telegram-status-state-active = <b>АКТИВНО</b>
telegram-status-state-paused = <b>ПРИЗУПИНЕНО</b>
telegram-status-on = УВІМКНЕНО
telegram-status-off = ВИМКНЕНО
telegram-status-body =
    <b>Стан системи</b>

    <b>Система</b>
    Стан — { $state }
    Аптайм — { $uptime }
    Версія — v{ $version }

    <b>Торгівля</b>
    Входи — { $entries }
    Виходи — { $exits }
    Позиції — { $positions }
telegram-positions-empty =
    <b>Немає відкритих позицій</b>

    Очікування можливостей...
telegram-positions-title = <b>Відкриті позиції ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+ ще { $count }...</i>
telegram-positions-summary =
    <b>Підсумок портфеля</b>
    Інвестовано — { $invested } { -sol }
    Чистий P{ "&amp;" }L — { $pnl } { -sol }
telegram-balance-body =
    <b>Баланс гаманця</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>Денна статистика</b>

    Позиції — { $positions }
    Інвестовано — { $invested } { -sol }
    P{ "&amp;" }L — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } готовий!</b>

    Торгівлю <b>увімкнено</b>.

    Керуйте ботом за допомогою клавіатури нижче.
    Введіть /help, щоб переглянути доступні команди.
telegram-stop-already = <b>Торгівлю вже вимкнено</b>
telegram-stop-done =
    <b>Торгівлю вимкнено</b>

    Усі торгові монітори (входи { "&amp;" } виходи) зупинено.
    Скористайтеся /pause, щоб зупинити лише входи.
telegram-stop-failed =
    <b>Не вдалося вимкнути торгівлю</b>

    Помилка: { $detail }
telegram-pause-done =
    <b>Монітор входу призупинено</b>

    Нові позиції не відкриватимуться.
    Монітор виходу продовжує працювати.
telegram-pause-failed =
    <b>Не вдалося призупинити входи</b>

    Помилка: { $detail }
telegram-resume-done =
    <b>Монітор входу відновлено</b>

    Тепер стежить за сигналами входу.
telegram-resume-failed =
    <b>Не вдалося відновити входи</b>

    Помилка: { $detail }
telegram-force-stop-confirm =
    <b>ПРИМУСОВА ЗУПИНКА</b>

    Це негайно зупинить ВСЮ торгову активність:
    • Жодних нових входів
    • Жодних виходів (зокрема стоп-лосів)
    • Жодних операцій DCA
telegram-force-stop-warning = <b>Це екстрена дія!</b>
telegram-force-stop-question = Ви впевнені?
telegram-force-stop-active =
    <b>ПРИМУСОВУ ЗУПИНКУ АКТИВОВАНО</b>

    Усю торгівлю зупинено.

    Скористайтеся /resume_trading, щоб зняти цей прапорець.
telegram-resume-trading-not-stopped =
    <b>Торгівлю не зупинено примусово</b>

    Жодних дій не потрібно.
telegram-resume-trading-done =
    <b>Торгівлю відновлено</b>

    Прапорець примусової зупинки знято.
    Звичайні торгові операції тепер можуть відновитися.

## Help.

telegram-help-title = <b>Довідка { -brand }</b>
telegram-help-heading-dashboard = Панель керування
telegram-help-heading-market = Ринок
telegram-help-heading-trading = Торгівля
telegram-help-heading-safety = Безпека
telegram-help-heading-system = Система
telegram-help-commands-dashboard =
    /status — стан системи { "&amp;" } аптайм
    /stats — денна продуктивність
    /balance — баланс гаманця
    /positions — відкриті позиції
telegram-help-commands-market =
    /tokens — оглядач токенів
    /rejected — відфільтровані токени
telegram-help-commands-trading =
    /start — увімкнути торгову систему
    /stop — вимкнути торгову систему
    /pause — призупинити нові входи
    /resume — відновити нові входи
    /menu — інтерактивне меню
telegram-help-commands-safety =
    /force_stop — <b>ЕКСТРЕНА ЗУПИНКА</b>
    /resume_trading — зняти екстрений статус
telegram-help-commands-system =
    /update — стан оновлення { "&amp;" } встановлення
    /login — автентифікація 2FA
telegram-help-tip = <i>Порада: натисніть на команду, щоб виконати її.</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>Оновлень немає</b>

    Працює v{ $version }, встановлена автоматично.
telegram-update-up-to-date =
    <b>Оновлень немає</b>

    Працює v{ $version }.
telegram-update-check-failed =
    <b>Не вдалося перевірити оновлення</b>

    { $reason }
telegram-update-unreachable = Не вдалося зв’язатися з screenerbot.io.
telegram-update-installing = <b>Встановлення v{ $version }</b>
telegram-update-restarting =
    { -brand } перезапускається на нову версію. Торгівля відновиться автоматично.
telegram-update-install-failed =
    <b>Не вдалося встановити v{ $version }</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } завантажено</b>

    Цей випуск також оновлює десктопний застосунок, тож його інсталятор потрібно запустити на пристрої. Відкрийте там Налаштування → Оновлення.
telegram-update-downloading =
    <b>Завантаження v{ $version }</b>

    { $percent }% із { $size } МБ.
telegram-update-available =
    <b>Доступна v{ $version }</b>

    { $how }
    Розмір завантаження: { $size } МБ.

    Завантаження відбувається саме; надішліть /update ще раз, коли воно буде готове.
telegram-update-how-core = Встановлюється непомітно з коротким перезапуском.
telegram-update-how-installer = Потрібно один раз запустити десктопний інсталятор.

## Shared values and units.

telegram-value-unknown = Невідомо
telegram-value-na = н/д
telegram-percent-value = { $percent }%
telegram-price-sol = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds } с
telegram-duration-minutes = { $minutes } хв
telegram-duration-minutes-seconds = { $minutes } хв { $seconds } с
telegram-duration-hours = { $hours } год
telegram-duration-hours-minutes = { $hours } год { $minutes } хв
telegram-duration-days = { $days } д
telegram-duration-days-hours = { $days } д { $hours } год
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-sol = { $amount } { -sol }
telegram-error-line = Помилка: { $detail }
telegram-ai-reasoning =
    <b>Аналіз LLM</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens.

telegram-row-entry = Вхід — { $price } { -sol }
telegram-row-exit = Вихід — { $price } { -sol }
telegram-row-current = Поточна — { $price } { -sol }
telegram-row-invested = Інвестовано — { $amount } { -sol }
telegram-row-received = Отримано — { $amount } { -sol }
telegram-row-value = Вартість — { $amount } { -sol }
telegram-row-total = Разом — { $amount } { -sol }
telegram-row-tokens = Токени — { $tokens }
telegram-row-duration = Тривалість — { $duration }
telegram-row-reason = Причина — { $reason }
telegram-row-remaining = Залишок — { $percent }%
telegram-row-pnl = P{ "&amp;" }L — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>Позицію відкрито</b>
telegram-notify-opened-size = Розмір — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = Ціна — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>Позицію закрито</b> — прибуток
telegram-notify-closed-title-loss = <b>Позицію закрито</b> — збиток
telegram-notify-closed-reason-unspecified = Закрито
telegram-notify-partial-title = <b>Частковий вихід</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — продано { $percent }%
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = Додано — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = Середня — { $price } { -sol }
telegram-notify-severity-critical = <b>Критична помилка</b>
telegram-notify-severity-error = <b>Помилка</b>
telegram-notify-severity-warning = <b>Попередження</b>
telegram-notify-severity-info = <b>Інформація</b>
telegram-notify-alert-title = <b>Торгове сповіщення</b>
telegram-notify-alert-token = Токен: <code>${ $symbol }</code>
telegram-notify-alert-mint = Мінт: <code>{ $mint }</code>
telegram-notify-alert-bought = Дія: куплено { $amount } { -sol }
telegram-notify-alert-sold = Дія: продано { $amount } { -sol }
telegram-notify-alert-wallet = Гаманець: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (віртуально)
telegram-notify-copy-task = Завдання: { $task }
telegram-notify-scheduled-completed = <b>Заплановане завдання виконано</b>
telegram-notify-scheduled-failed = <b>Заплановане завдання не вдалося</b>
telegram-notify-scheduled-timed-out = <b>Для запланованого завдання вичерпано час</b>
telegram-notify-scheduled-error = Помилка: { $error }
telegram-notify-summary-title = <b>Підсумок дня</b> — { $date }
telegram-notify-summary-performance = <b>Продуктивність</b>
telegram-notify-summary-trades = Угоди — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = Відсоток виграшних угод — { $percent }%
telegram-notify-summary-pnl = P{ "&amp;" }L — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = Відкриті позиції — { $count }
telegram-notify-started-title = <b>{ -brand } запущено</b>
telegram-notify-started-version = <b>Версія</b> — { $version }
telegram-notify-started-mode = <b>Режим</b> — { $mode }
telegram-notify-started-ready = Готовий до торгівлі!
telegram-notify-stopped-title = <b>{ -brand } зупинено</b>
telegram-notify-stopped-reason = <b>Причина</b> — { $reason }
telegram-notify-stopped-goodbye = До побачення! { $icon }
telegram-notify-start-mode-normal = Звичайний
telegram-notify-stop-reason-graceful = Штатне завершення роботи
telegram-notify-update-available =
    <b>Доступне оновлення v{ $version }</b>

    { $how }
    Розмір завантаження: { $size } МБ
telegram-notify-update-how-installer = Цей випуск також оновлює десктопний застосунок, тож його інсталятор потрібно запустити один раз.
telegram-notify-update-ready =
    <b>Оновлення v{ $version } готове</b>

    { $how }
telegram-notify-update-ready-silent = Надішліть /update, щоб застосувати його зараз, або воно встановиться під час наступного запуску { -brand }.
telegram-notify-update-ready-installer = Відкрийте Налаштування → Оновлення, щоб запустити інсталятор.
telegram-notify-update-applying =
    <b>Встановлення v{ $version }</b>

    Бекенд перезапускається; торгівля відновиться автоматично.
telegram-notify-new-tokens =
    <b>Сповіщення фільтрації</b>

    { $count ->
        [one] Знайдено { $count } новий токен, що відповідає вашим критеріям.
        [few] Знайдено { $count } нові токени, що відповідають вашим критеріям.
        [many] Знайдено { $count } нових токенів, що відповідають вашим критеріям.
       *[other] Знайдено { $count } нового токена, що відповідає вашим критеріям.
    }
telegram-notify-crash =
    <b>Бот аварійно завершив роботу!</b>

    <b>Місце:</b> <code>{ $location }</code>
    <b>Помилка:</b> <code>{ $error }</code>
telegram-notify-crash-restart = Перезапустіть бота.

## Filter results page.

telegram-filter-results-title = <b>Результати фільтрації</b> ({ $count })
telegram-filter-results-empty = <i>Токенів не знайдено.</i>
telegram-filter-results-page = <i>Сторінка { $page } з { $total }</i>

## Position screens.

telegram-position-not-found = Позицію не знайдено
telegram-position-no-positions = Немає позицій для закриття
telegram-position-history-empty =
    <b>Історія угод</b>

    Закритих позицій ще немає.
telegram-position-history-title = <b>Останні угоди</b>
telegram-position-history-more = <i>+ ще угод: { $count }...</i>
telegram-position-confirm-hint = <i>Підтвердьте протягом 30 с, щоб виконати.</i>
telegram-position-confirm-close-title = <b>Закрити позицію?</b>
telegram-position-confirm-close-selling = Продаж токенів: { $tokens }
telegram-position-confirm-close-estimated = Орієнтовно — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>Підтвердьте протягом 30 секунд</i>
telegram-position-confirm-sell =
    <b>Підтвердьте продаж</b>

    Токен — { $symbol }
    Сума — { $percent }%
    Токени — { $tokens }
telegram-position-confirm-dca =
    <b>Підтвердьте докупівлю</b>

    Токен — { $symbol }
    Додати — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>Закрити всі позиції?</b>

    Кількість — { $count }
telegram-position-confirm-close-all-hint =
    <i>Усі відкриті позиції буде продано за ринковою ціною.
    Підтвердьте протягом 30 с.</i>
telegram-position-confirm-force-stop =
    <b>ПРИМУСОВА ЗУПИНКА</b>

    Це негайно зупинить ВСЮ торгівлю:
    • Жодних нових входів
    • Жодних виходів
    • Жодного DCA
telegram-position-confirm-force-stop-warning = <b>Це екстрена дія.</b>
telegram-position-confirm-blacklist =
    <b>Занести токен до чорного списку?</b>

    Токен — { $symbol }
    Мінт — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>Позицію буде закрито, а майбутні входи заблоковано.</i>
telegram-position-selling = Продаж { $percent }% { $symbol }...
telegram-position-sell-done =
    <b>Продаж виконано</b>

    Токен — { $symbol }
    Продано — { $percent }%
    Отримано — { $amount } { -sol }
telegram-position-sell-failed = <b>Продаж не вдався</b>
telegram-position-adding = Докупівля { $amount } { -sol } до { $symbol }...
telegram-position-dca-done =
    <b>DCA виконано</b>

    Токен — { $symbol }
    Додано — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA не вдалося</b>
telegram-position-closing-all = Закриття всіх позицій...
telegram-position-close-all-done =
    <b>Закриття всіх позицій завершено</b>

    Закрито — { $closed }
    Помилки — { $failed }
telegram-position-blacklisted =
    <b>Токен у чорному списку</b>

    Токен — { $symbol }
    Статус — закрито { "&amp;" } у чорному списку

## Token screens.

telegram-token-not-found = Токен не знайдено
telegram-token-not-found-prefix = Токен не знайдено. Спробуйте пошук за довшим префіксом.
telegram-token-stats-failed = Не вдалося отримати статистику: { $detail }
telegram-token-list-failed = Не вдалося отримати токени: { $detail }
telegram-token-list-empty = У вигляді <b>{ $view }</b> токенів не знайдено.
telegram-token-view-passed = Пройшли фільтр
telegram-token-view-rejected = Відхилені
telegram-token-view-recent = Нещодавно додані
telegram-token-view-all = Усі токени
telegram-token-list-title = <b>{ $name }</b> (сторінка { $page }/{ $total })
telegram-token-list-stats = Ліквід.: { $liquidity } • Ціна: { $price }
telegram-token-list-hint = <i>Натисніть /token_ID, щоб переглянути деталі</i>
telegram-token-explorer =
    <b>Оглядач ринку</b>

    <b>Огляд</b>
    Пройшли фільтр — { $passed }
    Відхилені — { $rejected }
    Активні ціни — { $priced }
    Усього виявлено — { $total }

    <i>Виберіть категорію для перегляду:</i>
telegram-token-filter-title = <b>Аналіз фільтрів</b>
telegram-token-filter-distribution = <b>Розподіл</b>
telegram-token-filter-passed = Пройшли — { $count } ({ $percent }%)
telegram-token-filter-rejected = Відхилені — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = У чорному списку — { $count }
telegram-token-filter-coverage = <b>Охоплення</b>
telegram-token-filter-priced = З ціною пулу — { $count }
telegram-token-filter-open = Відкриті позиції — { $count }
telegram-token-filter-total = Усього виявлено — { $count }
telegram-token-filter-updated = <b>Останнє оновлення</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>Автооновлення кожні { $interval }</i>
telegram-token-detail-active = <b>Активна позиція</b>
telegram-token-detail-price = Ціна — { $price } { -sol }
telegram-token-detail-liquidity = Ліквідність — { $value }
telegram-token-detail-volume = Обсяг за 24 год — { $value }
telegram-token-detail-change = Зміна за 24 год — { $value }
telegram-token-detail-risk = Оцінка ризику: { $score }/100
telegram-token-detail-risk-unknown = Оцінка ризику: невідомо
telegram-token-detail-action = <i>Виберіть дію:</i>
telegram-token-search =
    <b>Пошук на ринку</b>

    Введіть символ або адресу мінта для пошуку:

    <i>Приклад: /token_BONK або /token_So11111</i>
telegram-token-confirm-buy =
    <b>Підтвердіть пряму купівлю</b>

    Токен — ${ $symbol }
    Мінт — <code>{ $mint }</code>
    Сума — { $amount } { -sol }

    <i>Підтвердьте протягом 30 с, щоб виконати.</i>
telegram-token-confirm-blacklist =
    <b>Занести токен до чорного списку?</b>

    Токен — ${ $symbol }
    Мінт — <code>{ $mint }</code>

    <i>Цей токен більше не проходитиме фільтри.</i>
telegram-token-blacklisted =
    <b>Токен у чорному списку</b>

    Токен — ${ $symbol }
    Статус — додано до чорного списку
telegram-token-blacklist-failed = <b>Не вдалося занести до чорного списку</b>
telegram-token-buy-processing =
    <b>Обробка купівлі...</b>

    Токен — ${ $symbol }
    Сума — { $amount } { -sol }
telegram-token-buy-done =
    <b>Купівля успішна</b>

    Токен — ${ $symbol }
    Сума — { $amount } { -sol }

    <i>Докладніше в /positions</i>
telegram-token-buy-failed =
    <b>Купівля не вдалася</b>

    Токен — ${ $symbol }
    Помилка — { $detail }
