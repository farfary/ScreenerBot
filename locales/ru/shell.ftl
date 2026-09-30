shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-brand-logo =
    .alt = { -brand }
shell-version = v{ $version }

shell-header-brand =
    .aria-label = Открыть главную страницу дашборда
    .title = Главная дашборда
shell-bot-card =
    .aria-label = Загрузка статуса автотрейдера
shell-bot-label = Авто
shell-bot-status-loading = ЗАГРУЗКА
shell-bot-today = Сегодня
shell-explore-control =
    .aria-label = Режим обзора. Подключите кошелёк и RPC-эндпоинт, чтобы включить все функции
    .title = Подключите кошелёк и RPC-эндпоинт, чтобы включить торговлю, балансы и ончейн-данные в реальном времени
shell-explore-title = Режим обзора
shell-explore-detail = Кошелёк и RPC не подключены
shell-explore-action = Завершить настройку
shell-wallet-card =
    .aria-label = Стоимость кошелька; открыть позиции
    .title = Стоимость кошелька ({ -sol } + токены) · открыть позиции
shell-wallet-worth-label = ИТОГО
shell-wallet-sol-label = { -sol }
shell-wallet-tokens-label = ТКН
shell-sol-price-card =
    .aria-label = Цена { -sol } в USD — открыть график
    .title = Цена { -sol } · нажмите, чтобы открыть график
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24 ч
shell-copy-card =
    .aria-label = Копитрейдинг; открыть раздел «Копитрейдинг»
    .title = Копитрейдинг · открыть раздел «Копитрейдинг»
shell-copy-label = КОПИ
shell-actions-more =
    .aria-label = Ещё действия в шапке
    .title = Ещё действия
shell-actions-group =
    .aria-label = Действия в шапке
shell-action-search =
    .aria-label = Поиск токенов
    .title = Поиск токенов (Ctrl/Cmd+K)
shell-action-featured =
    .aria-label = Избранные токены
    .title = Избранные токены
shell-action-notifications =
    .aria-label = Действия и уведомления
    .title = Действия и уведомления
shell-action-restart =
    .aria-label = Перезапустить приложение
    .title = Перезапустить приложение
shell-action-theme =
    .aria-label = Сменить тему
    .title = Сменить тему
shell-action-settings =
    .aria-label = Настройки
    .title = Настройки

shell-ticker-monitoring-segment =
    .title = Токены, отслеживаемые сервисом пулов
shell-ticker-monitoring = Отслеживается:
shell-ticker-filtering-segment =
    .title = Токены, прошедшие или не прошедшие критерии фильтрации
shell-ticker-passed = Прошли:
shell-ticker-rejected = Отклонены:
shell-ticker-pnl-segment =
    .title = Реализованная прибыль и убыток за сегодня
shell-ticker-pnl = P&L за сегодня:
shell-ticker-rpc-segment =
    .title = RPC-вызовы в минуту и доля успешных
shell-ticker-rpc = RPC:
shell-ticker-rpc-per-minute = /мин
shell-ticker-services-segment =
    .title = Состояние фоновых сервисов
shell-ticker-services-loading = Сервисы: <strong>загрузка</strong>

shell-notification-title = Действия
shell-notification-mark-all-read =
    .title = Отметить все как прочитанные
shell-notification-clear-all =
    .title = Очистить все
shell-notification-close =
    .aria-label = Закрыть
shell-notification-tab-all = Все
shell-notification-tab-active = Активные
shell-notification-tab-done = Выполненные
shell-notification-tab-failed = С ошибкой
shell-notification-filter-type-all = Все типы
shell-notification-filter-type-buy = Покупка
shell-notification-filter-type-sell = Продажа
shell-notification-filter-type-open = Открытие
shell-notification-filter-type-close = Закрытие
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = Частичный
shell-notification-filter-state-all = Все состояния
shell-notification-filter-state-in-progress = Выполняются
shell-notification-filter-state-completed = Завершены
shell-notification-filter-state-failed = С ошибкой
shell-notification-filter-state-cancelled = Отменены
shell-notification-list =
    .aria-label = Уведомления
shell-notification-empty = Действий пока нет
shell-notification-loading-more = Загрузка...
shell-notification-back-to-top =
    .title = Наверх

shell-status-bar-version = v
shell-status-bar-uptime = Работа
shell-status-bar-memory = Пам.
shell-status-bar-rpc = RPC
shell-status-bar-trading = Торговля
shell-status-bar-positions = Поз.
shell-status-bar-tokens = Токены

shell-splash-starting = Запуск { -brand }
shell-splash-waiting = Ожидание ответа локального ядра.
shell-splash-failed = { -brand } не удалось запустить
shell-splash-failed-detail = Проверьте файл журнала и перезапустите приложение.

shell-connection-connected = Ядро подключено
shell-connection-waiting = Ожидание ядра…
shell-connection-retry-now = Повторить сейчас
shell-connection-overlay-detail = Ядро недоступно. Торговля приостановлена; соединение восстановится автоматически.
shell-connection-restored = Соединение с ядром восстановлено

shell-trader-control-failed = Не удалось управлять трейдером
shell-notification-button-unread = Действия и уведомления, непрочитанных: { $count }
shell-restart-confirm-title = Перезапустить бота
shell-restart-confirm-message =
    Вы уверены, что хотите перезапустить бота?

    Это приведёт к следующему:
    • Все сервисы будут остановлены
    • Процесс будет перезапущен
    • Займёт ~10-15 секунд

    Все активные операции будут прерваны.
shell-restart-confirm-action = Перезапустить
shell-restart-progress = Перезапуск бота
shell-restart-failed = Не удалось перезапустить
shell-restart-failed-status = Не удалось перезапустить: { $status }
shell-restart-helper-unavailable = Помощник автоматического перезапуска недоступен. Скоро перезагрузите дашборд.

shell-page-title-fallback = Дашборд
shell-page-load-failed = Не удалось загрузить страницу
shell-page-offline-detail = Ядро сейчас недоступно. Страница загрузится автоматически, когда соединение восстановится.

shell-bot-state-explore = ОБЗОР
shell-bot-state-halted = ОСТАНОВЛЕН
shell-bot-state-off = ВЫКЛ.
shell-bot-state-waiting = ОЖИДАНИЕ
shell-bot-state-idle = ПРОСТОЙ
shell-bot-state-entry-paused = ВХОДЫ НА ПАУЗЕ
shell-bot-state-running = РАБОТАЕТ
shell-bot-control-explore = Автотрейдер недоступен в режиме обзора. Откройте настройку кошелька и RPC.
shell-bot-control-halted = Включена экстренная остановка. Откройте управление автотрейдером.
shell-bot-control-off = Автотрейдер выключен. Нажмите, чтобы включить.
shell-bot-control-waiting = Автотрейдер включён и ждёт основные сервисы. Нажмите, чтобы отключить.
shell-bot-control-idle = Автотрейдер включён, но оба монитора выключены. Откройте управление автотрейдером.
shell-bot-control-entry-paused = Защита от убытков приостановила входы; выходы продолжают работать. Откройте управление автотрейдером.
shell-bot-control-running = Автотрейдер работает. Нажмите, чтобы отключить.

shell-wallet-card-summary = Стоимость кошелька: { $equity } { -sol } (свободно { $balance } { -sol }, токенов: { $tokens }); открыть позиции
shell-copy-running-live = Боевых: { $count }
shell-copy-running-paper = Виртуальных: { $count }
shell-copy-value-paused = На паузе
shell-copy-value-idle = Простой
shell-copy-sub-active = активно: { $active } из { $total }

shell-ticker-services-healthy = Сервисы: <strong>в порядке</strong>
shell-ticker-services-issues =
    { $count ->
        [one] Сервисы: <strong>{ $count } проблема</strong>
        [few] Сервисы: <strong>{ $count } проблемы</strong>
        [many] Сервисы: <strong>{ $count } проблем</strong>
       *[other] Сервисы: <strong>{ $count } проблемы</strong>
    }

shell-agent-request-title = Запрос агента
shell-agent-request-client-fallback = Подключённый агент
shell-agent-request-message = { $client } хочет выполнить «{ $tool }» в { -brand }. Запрос { $expiry }.
shell-agent-request-message-arguments = { $client } хочет выполнить «{ $tool }» в { -brand }. Аргументы: { $summary }. Запрос { $expiry }.
shell-agent-request-expires-minutes = истекает через { $minutes } мин
shell-agent-request-expires-seconds = истекает через { $seconds } с
shell-agent-request-approve = Разрешить
shell-agent-request-deny = Отклонить

shell-toast-copied = { $label } скопировано
shell-toast-copy-failed = Не удалось скопировать
shell-toast-still-running = Всё ещё выполняется — проверьте центр уведомлений
shell-toast-dismiss =
    .aria-label = Закрыть
shell-confirm-title = Подтвердите действие
shell-confirm-message = Вы уверены?
shell-address-open-solscan = — открыть в { -solscan }
shell-address-copy = Скопировать адрес

shell-assistant-label = Ассистент
shell-assistant-dialog =
    .aria-label = Ассистент

shell-status-bar-trading-active = Активна
shell-status-bar-trading-inactive = Неактивна

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title }: отменено
shell-action-swap-buy-live = Покупка
shell-action-swap-buy-done = Куплено
shell-action-swap-buy-failed = Покупка не удалась
shell-action-swap-sell-live = Продажа
shell-action-swap-sell-done = Продано
shell-action-swap-sell-failed = Продажа не удалась
shell-action-position-open-live = Открытие позиции
shell-action-position-open-done = Открыта
shell-action-position-open-failed = Не удалось открыть
shell-action-position-close-live = Закрытие позиции
shell-action-position-close-done = Закрыта
shell-action-position-close-failed = Не удалось закрыть
shell-action-position-dca-live = Докупка позиции
shell-action-position-dca-done = Докуплено
shell-action-position-dca-failed = Докупка не удалась
shell-action-partial-exit-live = Частичный выход
shell-action-partial-exit-done = Частичный выход
shell-action-partial-exit-failed = Частичный выход не удался
shell-action-manual-order-live = Размещение ордера
shell-action-manual-order-done = Ордер размещён
shell-action-manual-order-failed = Не удалось разместить ордер
shell-action-trade-live = Сделка
shell-action-trade-done = Сделка выполнена
shell-action-trade-failed = Сделка не удалась

shell-exit-title = Закрыть { -brand }?
shell-exit-description = Выберите, как закрыть приложение
shell-exit-minimize = Свернуть в трей
shell-exit-minimize-detail = Продолжить работу в фоне
shell-exit-quit = Выйти из приложения
shell-exit-quit-detail = Закрыть полностью и остановить все сервисы

shell-lightbox-save =
    .title = Сохранить изображение
shell-lightbox-close =
    .title = Закрыть (ESC)

shell-theme-light = Светлая
shell-theme-dark = Тёмная
shell-theme-switch-to-light = Переключить на светлую тему
shell-theme-switch-to-dark = Переключить на тёмную тему
