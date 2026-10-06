## Reply keyboard words. Icons come from ReplyCommand in src/telegram/reply.rs.

telegram-reply-status = Статус
telegram-reply-balance = Баланс
telegram-reply-positions = Позиции
telegram-reply-pause = Пауза
telegram-reply-resume = Продолжить
telegram-reply-stop = Стоп
telegram-reply-stats = Статистика
telegram-reply-menu = Меню
telegram-reply-help = Помощь

## Inline keyboard buttons.

telegram-button-positions = Позиции
telegram-button-balance = Баланс
telegram-button-stats = Статистика
telegram-button-tokens = Токены
telegram-button-pause = Пауза
telegram-button-stop = Стоп
telegram-button-settings = Настройки
telegram-button-refresh = Обновить
telegram-button-menu = Меню
telegram-button-back = Назад
telegram-button-back-to-menu = Назад в меню
telegram-button-back-to-tokens = Назад к токенам
telegram-button-cancel = Отмена
telegram-button-close-all-positions = Закрыть все позиции
telegram-button-sell-percent = Продать { $percent }%
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = В чёрный список
telegram-button-blacklist-symbol = В чёрный список: { $symbol }
telegram-button-close-position = Закрыть позицию
telegram-button-confirm-close = Подтвердить закрытие
telegram-button-confirm-close-all = Закрыть ВСЕ позиции
telegram-button-confirm-sell = Подтвердить продажу { $percent }%
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = ПОДТВЕРДИТЬ ПРИНУДИТЕЛЬНУЮ ОСТАНОВКУ
telegram-button-confirm-buy = Купить на { $amount } { -sol }
telegram-button-notifications = Уведомления
telegram-button-trading = Торговля
telegram-button-entry-monitor = Монитор входов
telegram-button-exit-monitor = Монитор выходов
telegram-button-auto-trading = Автоторговля
telegram-button-force-stop = Принудительная остановка
telegram-button-notify-opened = Открытие
telegram-button-notify-closed = Закрытие
telegram-button-notify-partial = Частичный выход
telegram-button-notify-dca = DCA
telegram-button-notify-errors = Ошибки
telegram-button-details = Подробности
telegram-button-position = Позиция
telegram-button-sell-more = Продать ещё
telegram-button-more-dca = Ещё DCA
telegram-button-history = История
telegram-button-status = Статус
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = Войти снова
telegram-button-previous = Назад
telegram-button-next = Далее
telegram-button-passed = Пройдены
telegram-button-rejected = Отклонены
telegram-button-new-24h = Новые (24 ч)
telegram-button-all-tokens = Все токены
telegram-button-search-token = Найти токен
telegram-button-filter-stats = Статистика фильтров
telegram-button-refresh-stats = Обновить статистику
telegram-button-view-position = Открыть позицию
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    Неизвестная команда: { $command }

    Список доступных команд: /help.
telegram-session-expired =
    <b>Сеанс истёк</b>

    Чтобы войти снова, используйте /login.
telegram-2fa-required =
    <b>Требуется 2FA</b>

    Введите 6-значный код из приложения-аутентификатора.
telegram-account-locked =
    <b>Аккаунт заблокирован</b>

    Слишком много неудачных попыток.
    Повторите попытку через { $seconds ->
        [one] { $seconds } секунду.
        [few] { $seconds } секунды.
        [many] { $seconds } секунд.
       *[other] { $seconds } секунды.
    }
telegram-code-invalid = Введите корректный 6-значный код.
telegram-authenticated =
    <b>Вход выполнен!</b>

    Теперь вам доступны команды бота.
telegram-wrong-code =
    <b>Неверный код</b>

    { $remaining ->
        [one] Осталась { $remaining } попытка.
        [few] Осталось { $remaining } попытки.
        [many] Осталось { $remaining } попыток.
       *[other] Осталось { $remaining } попытки.
    }
telegram-auth-required =
    <b>Требуется авторизация</b>

    Введите пароль, чтобы продолжить.

    <i>Напишите пароль и отправьте его.</i>
telegram-login-required =
    <b>Требуется вход</b>

    Введите 6-значный код из приложения-аутентификатора:
telegram-session-activated =
    <b>Сеанс активирован</b>

    2FA не настроена. Ваш сеанс теперь активен.

    <i>Совет: включите 2FA в настройках безопасности для лучшей защиты.</i>

## Chat discovery.

telegram-discovery-hello = Здравствуйте, { $name }!
telegram-discovery-default-name = пользователь
telegram-discovery-detected = <b>Чат обнаружен!</b>
telegram-discovery-details =
    ID чата: <code>{ $chat_id }</code>
    Тип: { $chat_type }

    Откройте дашборд { -brand } и выберите этот чат.
telegram-chat-type-private = личный
telegram-chat-type-group = группа
telegram-chat-type-supergroup = супергруппа
telegram-chat-type-channel = канал

## Menus.

telegram-menu-title =
    <b>Панель управления</b>

    Выберите пункт, чтобы посмотреть данные или управлять ботом.
telegram-menu-positions-empty =
    <b>Нет открытых позиций</b>

    Ожидаем новые возможности...
telegram-menu-positions-title = <b>Позиции ({ $count })</b>
telegram-menu-positions-hint = <i>Нажмите на позицию, чтобы управлять ею.</i>
telegram-menu-settings =
    <b>Настройки</b>

    Настройте уведомления и параметры торговли.
telegram-settings-notifications =
    <b>Настройки уведомлений</b>

    Включайте и отключайте уведомления:
telegram-settings-trading =
    <b>Управление торговлей</b>

    Включайте и отключайте функции торговли:
telegram-pagination-expired = Сеанс постраничного просмотра истёк.

## Status commands.

telegram-status-state-stopped = <b>ОСТАНОВЛЕНО</b> (принудительная остановка активна)
telegram-status-state-active = <b>АКТИВНО</b>
telegram-status-state-paused = <b>НА ПАУЗЕ</b>
telegram-status-on = Вкл.
telegram-status-off = Выкл.
telegram-status-body =
    <b>Состояние системы</b>

    <b>Система</b>
    Состояние — { $state }
    Время работы — { $uptime }
    Версия — v{ $version }

    <b>Торговля</b>
    Входы — { $entries }
    Выходы — { $exits }
    Позиции — { $positions }
telegram-positions-empty =
    <b>Нет открытых позиций</b>

    Ожидаем возможности...
telegram-positions-title = <b>Открытые позиции ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+ ещё { $count }...</i>
telegram-positions-summary =
    <b>Сводка по портфелю</b>
    Вложено — { $invested } { -sol }
    Чистый P{ "&amp;" }L — { $pnl } { -sol }
telegram-balance-body =
    <b>Баланс кошелька</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>Статистика за день</b>

    Позиции — { $positions }
    Вложено — { $invested } { -sol }
    P{ "&amp;" }L — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } готов к работе!</b>

    Торговля <b>включена</b>.

    Для управления ботом используйте клавиатуру ниже.
    Список команд: /help.
telegram-stop-already = <b>Торговля уже отключена</b>
telegram-stop-done =
    <b>Торговля отключена</b>

    Все торговые мониторы (входы { "&amp;" } выходы) остановлены.
    Чтобы остановить только входы, используйте /pause.
telegram-stop-failed =
    <b>Не удалось отключить торговлю</b>

    Ошибка: { $detail }
telegram-pause-done =
    <b>Монитор входов приостановлен</b>

    Новые позиции открываться не будут.
    Монитор выходов продолжает работать.
telegram-pause-failed =
    <b>Не удалось приостановить входы</b>

    Ошибка: { $detail }
telegram-resume-done =
    <b>Монитор входов возобновлён</b>

    Снова отслеживаем сигналы входа.
telegram-resume-failed =
    <b>Не удалось возобновить входы</b>

    Ошибка: { $detail }
telegram-force-stop-confirm =
    <b>ПРИНУДИТЕЛЬНАЯ ОСТАНОВКА</b>

    Вся торговая активность будет немедленно остановлена:
    • Никаких новых входов
    • Никаких выходов (включая стоп-лоссы)
    • Никаких операций DCA
telegram-force-stop-warning = <b>Это экстренное действие!</b>
telegram-force-stop-question = Вы уверены?
telegram-force-stop-active =
    <b>ПРИНУДИТЕЛЬНАЯ ОСТАНОВКА АКТИВИРОВАНА</b>

    Вся торговля остановлена.

    Чтобы снять этот флаг, используйте /resume_trading.
telegram-resume-trading-not-stopped =
    <b>Принудительная остановка не активна</b>

    Действий не требуется.
telegram-resume-trading-done =
    <b>Торговля возобновлена</b>

    Флаг принудительной остановки снят.
    Обычная торговля может продолжаться.

## Help.

telegram-help-title = <b>Справка { -brand }</b>
telegram-help-heading-dashboard = Дашборд
telegram-help-heading-market = Рынок
telegram-help-heading-trading = Торговля
telegram-help-heading-safety = Безопасность
telegram-help-heading-system = Система
telegram-help-commands-dashboard =
    /status — состояние системы { "&amp;" } время работы
    /stats — результаты за день
    /balance — баланс кошелька
    /positions — открытые позиции
telegram-help-commands-market =
    /tokens — обозреватель токенов
    /rejected — отфильтрованные токены
telegram-help-commands-trading =
    /start — включить торговую систему
    /stop — отключить торговую систему
    /pause — приостановить новые входы
    /resume — возобновить новые входы
    /menu — интерактивное меню
telegram-help-commands-safety =
    /force_stop — <b>ЭКСТРЕННАЯ ОСТАНОВКА</b>
    /resume_trading — снять экстренный статус
telegram-help-commands-system =
    /update — статус обновлений { "&amp;" } установка
    /login — 2FA-аутентификация
telegram-help-tip = <i>Совет: нажмите на команду, чтобы выполнить её.</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>Установлена последняя версия</b>

    Работает v{ $version }, установлена автоматически.
telegram-update-up-to-date =
    <b>Установлена последняя версия</b>

    Работает v{ $version }.
telegram-update-check-failed =
    <b>Не удалось проверить обновления</b>

    { $reason }
telegram-update-unreachable = Не удалось подключиться к screenerbot.io.
telegram-update-installing = <b>Установка v{ $version }</b>
telegram-update-restarting =
    { -brand } перезапускается на новой версии. Торговля возобновится автоматически.
telegram-update-install-failed =
    <b>Не удалось установить v{ $version }</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } загружена</b>

    Этот релиз также обновляет десктопное приложение, поэтому его установщик нужно запустить на компьютере. Откройте там «Настройки» → «Обновления».
telegram-update-downloading =
    <b>Загрузка v{ $version }</b>

    { $percent }% из { $size } МБ.
telegram-update-available =
    <b>Доступна v{ $version }</b>

    { $how }
    Размер загрузки: { $size } МБ.

    Она загружается сама; отправьте /update ещё раз, когда всё будет готово.
telegram-update-how-core = Устанавливается тихо, с коротким перезапуском.
telegram-update-how-installer = Нужно один раз запустить установщик десктопного приложения.

## Shared values and units. Numbers arrive formatted; only the unit words live here.

telegram-value-unknown = Неизвестно
telegram-value-na = Н/Д
telegram-percent-value = { $percent }%
telegram-price-native = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds } с
telegram-duration-minutes = { $minutes } мин
telegram-duration-minutes-seconds = { $minutes } мин { $seconds } с
telegram-duration-hours = { $hours } ч
telegram-duration-hours-minutes = { $hours } ч { $minutes } мин
telegram-duration-days = { $days } д
telegram-duration-days-hours = { $days } д { $hours } ч
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-native = { $amount } { -sol }
telegram-error-line = Ошибка: { $detail }
telegram-ai-reasoning =
    <b>Анализ LLM</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens. Icons come from Rust.

telegram-row-entry = Вход — { $price } { -sol }
telegram-row-exit = Выход — { $price } { -sol }
telegram-row-current = Сейчас — { $price } { -sol }
telegram-row-invested = Вложено — { $amount } { -sol }
telegram-row-received = Получено — { $amount } { -sol }
telegram-row-value = Стоимость — { $amount } { -sol }
telegram-row-total = Итого — { $amount } { -sol }
telegram-row-tokens = Токены — { $tokens }
telegram-row-duration = Длительность — { $duration }
telegram-row-reason = Причина — { $reason }
telegram-row-remaining = Осталось — { $percent }%
telegram-row-pnl = P{ "&amp;" }L — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>Позиция открыта</b>
telegram-notify-opened-size = Размер — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = Цена — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>Позиция закрыта</b> — прибыль
telegram-notify-closed-title-loss = <b>Позиция закрыта</b> — убыток
telegram-notify-closed-reason-unspecified = Закрыта
telegram-notify-partial-title = <b>Частичный выход</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — продано { $percent }%
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = Добавлено — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = Сред. — { $price } { -sol }
telegram-notify-severity-critical = <b>Критическая ошибка</b>
telegram-notify-severity-error = <b>Ошибка</b>
telegram-notify-severity-warning = <b>Предупреждение</b>
telegram-notify-severity-info = <b>Информация</b>
telegram-notify-alert-title = <b>Оповещение о сделке</b>
telegram-notify-alert-token = Токен: <code>${ $symbol }</code>
telegram-notify-alert-mint = Минт: <code>{ $mint }</code>
telegram-notify-alert-bought = Действие: покупка на { $amount } { -sol }
telegram-notify-alert-sold = Действие: продажа на { $amount } { -sol }
telegram-notify-alert-wallet = Кошелёк: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (виртуальная торговля)
telegram-notify-copy-task = Задача: { $task }
telegram-notify-scheduled-completed = <b>Запланированная задача выполнена</b>
telegram-notify-scheduled-failed = <b>Запланированная задача завершилась ошибкой</b>
telegram-notify-scheduled-timed-out = <b>Время запланированной задачи истекло</b>
telegram-notify-scheduled-error = Ошибка: { $error }
telegram-notify-summary-title = <b>Сводка за день</b> — { $date }
telegram-notify-summary-performance = <b>Результаты</b>
telegram-notify-summary-trades = Сделки — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = Доля прибыльных сделок — { $percent }%
telegram-notify-summary-pnl = P{ "&amp;" }L — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = Открытые позиции — { $count }
telegram-notify-started-title = <b>{ -brand } запущен</b>
telegram-notify-started-version = <b>Версия</b> — { $version }
telegram-notify-started-mode = <b>Режим</b> — { $mode }
telegram-notify-started-ready = Готов к торговле!
telegram-notify-stopped-title = <b>{ -brand } остановлен</b>
telegram-notify-stopped-reason = <b>Причина</b> — { $reason }
telegram-notify-stopped-goodbye = До свидания! { $icon }
telegram-notify-start-mode-normal = Обычный
telegram-notify-stop-reason-graceful = Плавное завершение работы
telegram-notify-update-available =
    <b>Доступно обновление v{ $version }</b>

    { $how }
    Размер загрузки: { $size } МБ
telegram-notify-update-how-installer = Этот релиз также обновляет десктопное приложение, поэтому установщик нужно запустить один раз.
telegram-notify-update-ready =
    <b>Обновление v{ $version } готово</b>

    { $how }
telegram-notify-update-ready-silent = Отправьте /update, чтобы применить его сейчас, или оно установится при следующем запуске { -brand }.
telegram-notify-update-ready-installer = Откройте «Настройки» → «Обновления», чтобы запустить установщик.
telegram-notify-update-applying =
    <b>Установка v{ $version }</b>

    Бэкенд перезапускается; торговля возобновится автоматически.
telegram-notify-new-tokens =
    <b>Оповещение фильтрации</b>

    { $count ->
        [one] Найден { $count } новый токен, подходящий под ваши критерии.
        [few] Найдено { $count } новых токена, подходящих под ваши критерии.
        [many] Найдено { $count } новых токенов, подходящих под ваши критерии.
       *[other] Найдено { $count } нового токена, подходящего под ваши критерии.
    }
telegram-notify-crash =
    <b>Сбой бота!</b>

    <b>Место:</b> <code>{ $location }</code>
    <b>Ошибка:</b> <code>{ $error }</code>
telegram-notify-crash-restart = Перезапустите бота.

## Filter results page.

telegram-filter-results-title = <b>Результаты фильтрации</b> ({ $count })
telegram-filter-results-empty = <i>Токены не найдены.</i>
telegram-filter-results-page = <i>Страница { $page } из { $total }</i>

## Position screens.

telegram-position-not-found = Позиция не найдена
telegram-position-no-positions = Нет позиций для закрытия
telegram-position-history-empty =
    <b>История сделок</b>

    Закрытых позиций пока нет.
telegram-position-history-title = <b>Последние сделки</b>
telegram-position-history-more = <i>Ещё сделок: { $count }...</i>
telegram-position-confirm-hint = <i>Подтвердите в течение 30 с, чтобы выполнить.</i>
telegram-position-confirm-close-title = <b>Закрыть позицию?</b>
telegram-position-confirm-close-selling = Продаём токенов: { $tokens }
telegram-position-confirm-close-estimated = Ожидаемо — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>Подтвердите в течение 30 секунд</i>
telegram-position-confirm-sell =
    <b>Подтвердите продажу</b>

    Токен — { $symbol }
    Доля — { $percent }%
    Токены — { $tokens }
telegram-position-confirm-dca =
    <b>Подтвердите докупку</b>

    Токен — { $symbol }
    Докупка — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>Закрыть все позиции?</b>

    Количество — { $count }
telegram-position-confirm-close-all-hint =
    <i>Все открытые позиции будут проданы по рынку.
    Подтвердите в течение 30 с.</i>
telegram-position-confirm-force-stop =
    <b>ПРИНУДИТЕЛЬНАЯ ОСТАНОВКА</b>

    Вся торговля будет немедленно остановлена:
    • Никаких новых входов
    • Никаких выходов
    • Никакого DCA
telegram-position-confirm-force-stop-warning = <b>Это экстренное действие.</b>
telegram-position-confirm-blacklist =
    <b>Добавить токен в чёрный список?</b>

    Токен — { $symbol }
    Минт — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>Позиция будет закрыта, а будущие входы в этот токен — запрещены.</i>
telegram-position-selling = Продаём { $percent }% { $symbol }...
telegram-position-sell-done =
    <b>Продажа выполнена</b>

    Токен — { $symbol }
    Продано — { $percent }%
    Получено — { $amount } { -sol }
telegram-position-sell-failed = <b>Не удалось продать</b>
telegram-position-adding = Докупаем { $symbol } на { $amount } { -sol }...
telegram-position-dca-done =
    <b>DCA выполнен</b>

    Токен — { $symbol }
    Добавлено — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA не выполнен</b>
telegram-position-closing-all = Закрываем все позиции...
telegram-position-close-all-done =
    <b>Закрытие всех позиций завершено</b>

    Закрыто — { $closed }
    Не удалось — { $failed }
telegram-position-blacklisted =
    <b>Токен в чёрном списке</b>

    Токен — { $symbol }
    Статус — закрыт { "&amp;" } в чёрном списке

## Token screens.

telegram-token-not-found = Токен не найден
telegram-token-not-found-prefix = Токен не найден. Попробуйте искать по более длинному префиксу.
telegram-token-stats-failed = Не удалось получить статистику: { $detail }
telegram-token-list-failed = Не удалось получить токены: { $detail }
telegram-token-list-empty = В разделе <b>{ $view }</b> токенов нет.
telegram-token-view-passed = Прошли фильтр
telegram-token-view-rejected = Отклонены
telegram-token-view-recent = Недавно добавлены
telegram-token-view-all = Все токены
telegram-token-list-title = <b>{ $name }</b> (стр. { $page }/{ $total })
telegram-token-list-stats = Ликв.: { $liquidity } • Цена: { $price }
telegram-token-list-hint = <i>Нажмите /token_ID, чтобы открыть подробности</i>
telegram-token-explorer =
    <b>Обозреватель рынка</b>

    <b>Обзор</b>
    Прошли фильтр — { $passed }
    Отклонены — { $rejected }
    Активные цены — { $priced }
    Всего обнаружено — { $total }

    <i>Выберите категорию для просмотра:</i>
telegram-token-filter-title = <b>Анализ фильтров</b>
telegram-token-filter-distribution = <b>Распределение</b>
telegram-token-filter-passed = Пройдены — { $count } ({ $percent }%)
telegram-token-filter-rejected = Отклонены — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = В чёрном списке — { $count }
telegram-token-filter-coverage = <b>Охват</b>
telegram-token-filter-priced = С ценой пула — { $count }
telegram-token-filter-open = Открытые позиции — { $count }
telegram-token-filter-total = Всего обнаружено — { $count }
telegram-token-filter-updated = <b>Последнее обновление</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>Автообновление каждые { $interval }</i>
telegram-token-detail-active = <b>Активная позиция</b>
telegram-token-detail-price = Цена — { $price } { -sol }
telegram-token-detail-liquidity = Ликвидность — { $value }
telegram-token-detail-volume = Объём за 24 ч — { $value }
telegram-token-detail-change = Изменение за 24 ч — { $value }
telegram-token-detail-risk = Оценка риска: { $score }/100
telegram-token-detail-risk-unknown = Оценка риска: неизвестно
telegram-token-detail-action = <i>Выберите действие:</i>
telegram-token-search =
    <b>Поиск по рынку</b>

    Введите символ или адрес минта для поиска:

    <i>Пример: /token_BONK или /token_So11111</i>
telegram-token-confirm-buy =
    <b>Подтвердите прямую покупку</b>

    Токен — ${ $symbol }
    Минт — <code>{ $mint }</code>
    Сумма — { $amount } { -sol }

    <i>Подтвердите в течение 30 с, чтобы выполнить.</i>
telegram-token-confirm-blacklist =
    <b>Добавить токен в чёрный список?</b>

    Токен — ${ $symbol }
    Минт — <code>{ $mint }</code>

    <i>Токен больше не сможет проходить фильтры.</i>
telegram-token-blacklisted =
    <b>Токен в чёрном списке</b>

    Токен — ${ $symbol }
    Статус — добавлен в чёрный список
telegram-token-blacklist-failed = <b>Не удалось добавить в чёрный список</b>
telegram-token-buy-processing =
    <b>Выполняем покупку...</b>

    Токен — ${ $symbol }
    Сумма — { $amount } { -sol }
telegram-token-buy-done =
    <b>Покупка выполнена</b>

    Токен — ${ $symbol }
    Сумма — { $amount } { -sol }

    <i>Подробности смотрите в /positions</i>
telegram-token-buy-failed =
    <b>Не удалось купить</b>

    Токен — ${ $symbol }
    Ошибка — { $detail }
