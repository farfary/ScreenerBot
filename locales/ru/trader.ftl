trader-exit-type-stop-loss = Стоп-лосс
trader-exit-type-take-profit = Тейк-профит
trader-exit-type-roi = Цель по ROI
trader-exit-type-roi-exit = Цель по ROI
trader-exit-type-trailing-stop = Трейлинг-стоп
trader-exit-type-time-override = Выход по времени
trader-exit-type-time-rule = Правило по времени
trader-exit-type-manual = Вручную
trader-exit-type-manual-close = Вручную
trader-exit-type-dca = DCA
trader-exit-type-unknown = Неизвестно

trader-tab-stats = Статистика
trader-tab-strategy-control = Управление стратегиями
trader-tab-strategies = Стратегии
trader-tab-stop-loss = Стоп-лосс
trader-tab-trailing-stop = Трейлинг-стоп
trader-tab-roi = Тейк-профит
trader-tab-time-rules = Правила по времени
trader-tab-dca = DCA
trader-tab-settings = Настройки

trader-feature-coming-soon = Скоро
    .message = Эта функция скоро появится и пока недоступна.
trader-feature-beta = Бета
trader-feature-disabled = Отключено
    .message = Эта функция сейчас отключена.

trader-status-title = Автотрейдер
trader-status-loading = Загрузка...
trader-status-running = Работает
trader-status-stopped = Остановлен
trader-status-setup-required = Требуется настройка
trader-status-unavailable = Завершите настройку кошелька и RPC, чтобы использовать автотрейдер
trader-toggle-on = ВКЛ.
trader-toggle-off = ВЫКЛ.
trader-toggle-unavailable = НЕДОСТУПНО
trader-toggle-start-failed = Не удалось запустить трейдер
trader-toggle-stop-failed = Не удалось остановить трейдер
trader-controls-title = Управление торговлей
trader-halt-title = ТОРГОВЛЯ ОСТАНОВЛЕНА
trader-halt-reason-default = Принудительная остановка вручную
trader-halt-resume = Возобновить
trader-monitor-entry = Мониторинг входов
trader-monitor-exit = Мониторинг выходов
trader-monitor-master-off = Автотрейдер выключен
trader-loss-limit-title = Лимит убытков за период
trader-loss-limit-resume = Возобновить торговлю
trader-loss-limit-reset = Сбросить период
trader-loss-limit-off = Выкл.
trader-loss-limit-none = Лимит убытков за период не задан
trader-loss-limit-resets-in = Сброс через { $hours } { $minutes }
trader-loss-limit-reached = ЛИМИТ ДОСТИГНУТ
trader-force-stop = Остановить всё принудительно

trader-force-stop-confirm = Принудительная остановка торговли
    .message = Все торговые операции будут немедленно остановлены. Продолжить?
    .confirm = Остановить торговлю
trader-loss-limit-resume-confirm = Возобновить после лимита убытков
    .message = Лимит убытков за период остановил новые входы. Если возобновить, трейдер снова сможет открывать позиции до конца периода. Продолжить?
trader-loss-limit-reset-confirm = Сбросить период лимита убытков
    .message = Накопленный за текущий период убыток будет обнулён, начнётся новый период. Продолжить?

trader-toast-control-failed = Не удалось выполнить управление автотрейдером
trader-toast-force-stop-on = Принудительная остановка включена
trader-toast-force-stop-failed = Не удалось включить принудительную остановку
trader-toast-force-stop-cleared = Принудительная остановка снята
trader-toast-resume-failed = Не удалось возобновить торговлю
trader-toast-loss-limit-reset-failed = Не удалось сбросить лимит убытков
trader-toast-entry-monitor-failed = Не удалось переключить мониторинг входов
trader-toast-exit-monitor-failed = Не удалось переключить мониторинг выходов
trader-toast-load-failed = Ошибка загрузки
    .message = Не удалось загрузить конфигурацию трейдера
trader-toast-saved = Конфигурация сохранена
    .message = Настройки трейдера успешно применены
trader-toast-save-failed = Ошибка сохранения
    .message = Не удалось сохранить конфигурацию трейдера
trader-toast-feature-enabled = Функция включена
trader-toast-feature-disabled = Функция отключена
trader-toast-feature-applied = Настройка автотрейдера применена
trader-toast-strategy-enabled = Стратегия включена
    .message = Стратегия активна
trader-toast-strategy-disabled = Стратегия отключена
    .message = Стратегия неактивна
trader-toast-strategy-failed = Ошибка обновления
    .message = Не удалось обновить статус стратегии

trader-stats-window =
    .aria-label = Период статистики
trader-stats-window-day = 24 ч
trader-stats-window-week = 7 д
trader-stats-window-month = 30 д
trader-realized-title = Реализованные результаты
trader-metric-net-pnl = Чистый P&L
trader-metric-win-rate = Доля прибыльных сделок
trader-metric-profit-factor = Профит-фактор
trader-metric-max-drawdown = Макс. просадка
trader-metric-capital = Капитал в работе
trader-metric-avg-win-loss = Сред. прибыль / убыток
trader-metric-closed-trades = Закрытые сделки
trader-metric-median-hold = Медианное удержание
trader-stats-empty = В этом периоде нет закрытых сделок
trader-stats-won-lost = выиграно { $won } · проиграно { $lost }
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
        [one] { $amount } прибыльная
        [few] { $amount } прибыльные
        [many] { $amount } прибыльных
       *[other] { $amount } прибыльной
    }
trader-stats-losses =
    { $count ->
        [one] { $amount } убыточная
        [few] { $amount } убыточные
        [many] { $amount } убыточных
       *[other] { $amount } убыточной
    }
trader-stats-expected = ожидаемо { $amount } за сделку
trader-stats-profit-factor-basis = Валовая прибыль ÷ валовой убыток
trader-stats-drawdown-basis = Наибольшее реализованное падение от пика до минимума
trader-stats-slots =
    { $count ->
        [one] Слоты позиций: занято { $used } из { $max }
        [few] Слоты позиций: занято { $used } из { $max }
        [many] Слоты позиций: занято { $used } из { $max }
       *[other] Слоты позиций: занято { $used } из { $max }
    }
trader-stats-avg-basis = Средний результат прибыльной и убыточной сделки
trader-stats-closed =
    { $count ->
        [one] закрыта { $amount } позиция
        [few] закрыто { $amount } позиции
        [many] закрыто { $amount } позиций
       *[other] закрыто { $amount } позиции
    }
trader-stats-hold-average = в среднем { $span }
trader-stats-excluded =
    { $count ->
        [one] Исключён { $amount } закрытый раунд — нет полной базовой стоимости, поэтому нельзя честно посчитать P&L.
        [few] Исключены { $amount } закрытых раунда — нет полной базовой стоимости, поэтому нельзя честно посчитать P&L.
        [many] Исключены { $amount } закрытых раундов — нет полной базовой стоимости, поэтому нельзя честно посчитать P&L.
       *[other] Исключены { $amount } закрытого раунда — нет полной базовой стоимости, поэтому нельзя честно посчитать P&L.
    }

trader-daily-title = P&L по дням
trader-daily-subtitle = Реализованный { -sol } за день с накопленным итогом
trader-daily-loading = Загрузка P&L по дням...
trader-daily-chart = Реализованная прибыль и убыток по дням в { -sol }
trader-extreme-best = Лучшая сделка
trader-extreme-worst = Худшая сделка

trader-exit-title = Разбивка стратегий выхода
trader-exit-subtitle = Как закрывались позиции и что вернул каждый выход
trader-exit-loading = Загрузка данных о выходах...
trader-exit-empty-day = Нет закрытых сделок за последние 24 часа
trader-exit-empty-days =
    { $count ->
        [one] Нет закрытых сделок за последний { $amount } день
        [few] Нет закрытых сделок за последние { $amount } дня
        [many] Нет закрытых сделок за последние { $amount } дней
       *[other] Нет закрытых сделок за последние { $amount } дня
    }
trader-exit-share =
    { $count ->
        [one] { $amount } сделка · { $share } выходов
        [few] { $amount } сделки · { $share } выходов
        [many] { $amount } сделок · { $share } выходов
       *[other] { $amount } сделки · { $share } выходов
    }
trader-exit-average = в среднем { $value }

trader-impact-label = Влияние:
trader-current-label = Сейчас:
trader-readable-label = В привычном виде:
trader-example-how-it-works = Как это работает
trader-step-entry = Вход
trader-step-initial-position = Начальная позиция
trader-step-auto-exit = Автовыход
trader-step-exit = Выход
trader-step-full-exit = Полный выход из позиции
trader-value-percent = { $value }%
trader-example-profit = +{ $value }% прибыли

trader-stop-loss-title = Стоп-лосс
trader-stop-loss-subtitle = Автоматически выходить из позиции, когда убыток превышает ваш порог
trader-stop-loss-threshold-badge = Лимит убытков
trader-stop-loss-hold-badge = Необязательная задержка
trader-stop-loss-impact = Выход при падении на { $threshold }% от входа
trader-stop-loss-hold-immediate = Сразу
trader-stop-loss-hold-delay = задержка { $span }
trader-stop-loss-price-falls = Цена падает
trader-stop-loss-threshold-reached = Порог достигнут
trader-stop-loss-partial = Частичные выходы разрешены
trader-stop-loss-summary = Убыток ограничен: <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Примечание:</strong> стоп-лосс защищает от более крупных убытков, выходя из позиции раньше

trader-trailing-title = Трейлинг-стоп
trader-trailing-subtitle = Автоматически защищать прибыль, следуя за ценой по мере её роста
trader-trailing-activation-badge = Когда включать
trader-trailing-distance-badge = Запас прочности
trader-trailing-activation-impact = Слежение начинается при +{ $value }% прибыли
trader-trailing-distance-impact = Выход при -{ $value }% от пика
trader-trailing-activation = Активация
trader-trailing-peak = Пик
trader-trailing-final = итог +{ $value }%
trader-trailing-summary-protected = Прибыль защищена: <strong>{ $value }</strong>
trader-trailing-summary-avoided = Избегнут убыток от пика: <strong>{ $value }</strong>

trader-roi-title = Тейк-профит
trader-roi-subtitle = Автоматически выходить из всей позиции, когда прибыль достигает цели
trader-roi-target-badge = Одна цель
trader-roi-impact = Выход при +{ $target }% прибыли
trader-roi-example-title = Пример сценария
trader-roi-initial-buy = Первая покупка
trader-roi-target-hit = Цель достигнута
trader-roi-full-position = Вся позиция
trader-roi-sold = 100% продано
trader-roi-summary = Зафиксирована прибыль <strong>+{ $target }%</strong>

trader-time-title = Выход по времени
trader-time-subtitle = Автоматически выходить из позиций по истечении максимального времени удержания, если убыток превышает порог
trader-time-hold-badge = Триггер по времени
trader-time-loss-badge = Условие по убытку
trader-time-unit-seconds = секунды
trader-time-unit-minutes = минуты
trader-time-unit-hours = часы
trader-time-unit-days = дни
trader-time-conversion-default = 168 ч = 7 д
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
        [one] { $amount } секунда
        [few] { $amount } секунды
        [many] { $amount } секунд
       *[other] { $amount } секунды
    }
trader-duration-minutes =
    { $count ->
        [one] { $amount } минута
        [few] { $amount } минуты
        [many] { $amount } минут
       *[other] { $amount } минуты
    }
trader-duration-hours =
    { $count ->
        [one] { $amount } час
        [few] { $amount } часа
        [many] { $amount } часов
       *[other] { $amount } часа
    }
trader-duration-days =
    { $count ->
        [one] { $amount } день
        [few] { $amount } дня
        [many] { $amount } дней
       *[other] { $amount } дня
    }
trader-time-loss-impact = Выход, если после периода удержания убыток составляет { $value }% или больше
trader-time-day = День { $day }
trader-time-position-opened = Позиция открыта
trader-time-limit = Лимит времени
trader-time-hold-reached = Период удержания истёк
trader-time-loss-met = Порог убытка достигнут
trader-time-note = <strong>Примечание:</strong> позиции в плюсе или с меньшим убытком закрыты НЕ будут
trader-time-positions-title = Состояние текущих позиций
trader-time-positions-loading = Загрузка позиций...
trader-time-positions-empty = Нет открытых позиций
trader-time-positions-hold = Время удержания:
trader-time-positions-roi = ROI:

trader-strategy-entry-title = Стратегии входа
trader-strategy-entry-subtitle = Сигналы, которые могут открыть новую позицию.
trader-strategy-exit-title = Стратегии выхода
trader-strategy-exit-subtitle = Сигналы, которые могут закрыть или защитить открытую позицию.
trader-strategy-active-unknown = -- активно
trader-strategy-active = активно: { $enabled }/{ $total }
trader-strategy-loading = Загрузка стратегий...
trader-strategy-load-failed = Не удалось загрузить стратегии
trader-strategy-empty = Стратегии не заданы
trader-strategy-no-description = Описание не указано.
trader-strategy-unnamed = Стратегия без названия
trader-strategy-type-unknown = Стратегия
trader-strategy-priority-auto = Авто
trader-strategy-priority = Приоритет { $priority }

trader-dca-title = Усреднение (DCA)
trader-dca-subtitle = Автоматически докупать убыточные позиции, чтобы снизить среднюю цену входа
trader-dca-threshold-badge = Триггер входа
trader-dca-example-title = Пример DCA
trader-dca-example = 0.01 { -sol } начальная → DCA №1: 0.005 { -sol } при -10% → DCA №2: 0.005 { -sol } ещё при -10%
trader-dca-info-title = О стратегии DCA
trader-dca-info-subtitle = Важные особенности торговли с DCA
trader-dca-how-title = Как работает DCA
trader-dca-how-trigger = <strong>Триггер:</strong> позиция падает ниже порога DCA (например, -10%)
trader-dca-how-action = <strong>Действие:</strong> докупить ещё { -sol }, чтобы снизить среднюю базовую стоимость
trader-dca-how-repeat = <strong>Повтор:</strong> DCA можно выполнять несколько раз, вплоть до максимального числа
trader-dca-risk-title = Предупреждения о рисках
trader-dca-risk-exposure = <strong>Рост экспозиции:</strong> DCA увеличивает общий капитал под риском в каждой позиции
trader-dca-risk-knife = <strong>Падающий нож:</strong> DCA не поможет, если токен продолжит падать
trader-dca-risk-cooldown = <strong>Пауза:</strong> используйте паузу, чтобы избежать серии быстрых докупок

trader-sizing-title = Размер позиции
trader-sizing-subtitle = Сколько вкладывать в одну позицию
trader-sizing-positions-badge = Контроль риска
trader-sizing-trade-size-badge = На позицию
trader-timing-title = Тайминг и паузы
trader-timing-subtitle = Управление временем между операциями
trader-timing-close-cooldown = Пауза после закрытия позиции
trader-timing-close-cooldown-hint = Сколько минут ждать перед повторным открытием позиции по тому же токену
trader-timing-concurrency = Параллельность проверки входов
trader-timing-concurrency-hint = Сколько токенов проверять одновременно (больше — быстрее, но выше нагрузка на процессор)
trader-timing-unit-minutes = мин
trader-timing-unit-tokens = токенов
trader-timing-intervals = Интервалы мониторинга
trader-timing-intervals-badge = Только чтение
trader-timing-intervals-hint = Задаются в коде (через интерфейс не меняются)
trader-timing-intervals-value = <strong>Мониторинг входов:</strong> 30 с | <strong>Мониторинг выходов:</strong> 5 с
