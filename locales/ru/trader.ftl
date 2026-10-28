trader-tab-stats = Статистика
trader-tab-strategies = Стратегии
trader-tab-stop-loss = Стоп-лосс
trader-tab-trailing-stop = Трейлинг-стоп
trader-tab-take-profit = Тейк-профит
trader-tab-time-rules = Выход по времени
trader-tab-dca = DCA
trader-tab-settings = Настройки

trader-feature-coming-soon = Скоро
    .message = Эта функция скоро появится и пока недоступна.
trader-feature-beta = Бета
trader-feature-disabled = Отключено
    .message = Эта функция сейчас отключена.

trader-status-title = Автотрейдер
trader-status-running = Работает
trader-status-stopped = Остановлен
trader-status-setup-required = Требуется настройка
trader-setup-action = Завершить настройку
trader-toggle-start-failed = Не удалось запустить трейдер
trader-toggle-stop-failed = Не удалось остановить трейдер
trader-halt-title = Торговля остановлена
trader-halt-reason-default = Принудительная остановка вручную
trader-halt-resume = Возобновить
trader-monitor-entry = Мониторинг входов
trader-monitor-exit = Мониторинг выходов
trader-loss-limit-title = Лимит убытков за период
trader-loss-limit-resume = Возобновить торговлю
trader-loss-limit-reset = Сбросить период
trader-loss-limit-off = Выкл.
trader-loss-limit-resets-in = Сброс через { $hours } { $minutes }
trader-force-stop = Остановить всё принудительно
trader-force-stop-hint = Сразу останавливает все мониторы, задачи копирования и сделки.
trader-force-stop-idle = Сейчас не работает ничего, что остановила бы принудительная остановка.
trader-run-state-force-stopped = Принудительно остановлен
trader-run-state-waiting = Ожидание сервисов
trader-run-state-idle = Мониторинг выключен
trader-entries-label = Новые входы
trader-entries-open = Открыты
trader-entries-monitor-off = Мониторинг входов выключен
trader-entries-no-live-strategy = Нет боевой стратегии входа
trader-entries-no-paper-strategy = Нет виртуальной стратегии входа
trader-action-start = Запустить автотрейдер
trader-action-stop = Остановить автотрейдер
trader-notice-loss-limit = Достигнут лимит убытков
trader-notice-loss-limit-message = Новые боевые входы приостановлены до сброса периода. Выходы продолжают работать.
trader-notice-stats-failed = Не удалось загрузить статистику
trader-controls-group =
    .aria-label = Управление торговлей
trader-monitor-dormant = Запустится вместе с автотрейдером

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

trader-stats-window =
    .aria-label = Период статистики
trader-stats-book =
    .aria-label = Торговый портфель
trader-exposure-cash = Свободно
trader-exposure-open = В позициях
trader-exposure-unrealized = Нереализованный
trader-exposure-open-title = Позиции, занимающие слот, из лимита открытых позиций
trader-exposure-in-use = Задействовано
trader-exposure-in-use-title = { -sol }, вложенные в открытые позиции
# $amount is the formatted count.
trader-exposure-unpriced =
    { $count ->
        [one] У { $amount } открытой позиции ещё нет живой цены
        [few] У { $amount } открытых позиций ещё нет живой цены
        [many] У { $amount } открытых позиций ещё нет живой цены
       *[other] У { $amount } открытой позиции ещё нет живой цены
    }
# $date is a formatted date; $amount is a formatted SOL amount.
trader-exposure-cash-title = Виртуальный прогон с { $date } · начат с { $amount }
trader-paper-reset = Сбросить виртуальный прогон
trader-paper-reset-title = Сбросить виртуальный прогон?
trader-paper-reset-run-message = Каждая открытая виртуальная позиция, включая виртуальные активы задач копирования, закроется по текущей цене, и начнётся новый прогон с заданным стартовым балансом. Прежние прогоны останутся в истории.
trader-paper-reset-done = Начался новый виртуальный прогон
trader-paper-reset-failed = Не удалось сбросить виртуальный прогон
trader-realized-title = Реализованные результаты
trader-metric-net-pnl = Чистый P&L
trader-metric-win-rate = Доля прибыльных сделок
trader-metric-profit-factor = Профит-фактор
trader-metric-max-drawdown = Макс. просадка
trader-metric-avg-win-loss = Сред. прибыль / убыток
trader-metric-closed-trades = Закрытые сделки
trader-metric-median-hold = Медианное удержание
trader-metric-fees = Сетевые комиссии
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
trader-stats-excluded =
    { $count ->
        [one] Не учтена { $amount } закрытая сделка: её базовая стоимость неполная, поэтому её P&L неизвестен.
        [few] Не учтено закрытых сделок: { $amount }. Их базовая стоимость неполная, поэтому их P&L неизвестен.
        [many] Не учтено закрытых сделок: { $amount }. Их базовая стоимость неполная, поэтому их P&L неизвестен.
       *[other] Не учтено закрытых сделок: { $amount }. Их базовая стоимость неполная, поэтому их P&L неизвестен.
    }
# $ratio is a formatted ratio.
trader-stats-payoff = Payoff { $ratio }
trader-stats-streak-win =
    { $count ->
        [one] Последняя сделка прибыльная
        [few] { $amount } прибыльные подряд
        [many] { $amount } прибыльных подряд
       *[other] { $amount } прибыльной подряд
    }
trader-stats-streak-loss =
    { $count ->
        [one] Последняя сделка убыточная
        [few] { $amount } убыточные подряд
        [many] { $amount } убыточных подряд
       *[other] { $amount } убыточной подряд
    }
# $wins and $losses are formatted durations.
trader-stats-hold-split = Прибыльные { $wins } · убыточные { $losses }
trader-stats-fees-basis = Учтены в чистом P&L
trader-stats-loading = Загрузка статистики...
trader-open-strategies = Открыть стратегии

trader-daily-title = P&L по дням
trader-daily-subtitle = Реализованный { -sol } за день с накопленным итогом
trader-daily-chart = Реализованная прибыль и убыток по дням в { -sol }
trader-weekly-title = P&L по неделям
trader-weekly-subtitle = Реализованный { -sol } за неделю ISO с накопленным итогом
trader-weekly-chart = Реализованная прибыль и убыток по неделям в { -sol }
# $date is a formatted date; $pnl a signed SOL amount.
trader-daily-bar =
    { $count ->
        [one] { $date } · { $pnl } · { $amount } сделка
        [few] { $date } · { $pnl } · { $amount } сделки
        [many] { $date } · { $pnl } · { $amount } сделок
       *[other] { $date } · { $pnl } · { $amount } сделки
    }
trader-weekly-bar =
    { $count ->
        [one] Неделя с { $date } · { $pnl } · { $amount } сделка
        [few] Неделя с { $date } · { $pnl } · { $amount } сделки
        [many] Неделя с { $date } · { $pnl } · { $amount } сделок
       *[other] Неделя с { $date } · { $pnl } · { $amount } сделки
    }
trader-extreme-best = Лучшая сделка
trader-extreme-worst = Худшая сделка
trader-extreme-runs = Больше всего подряд
trader-outcome-title = Итоги сделок
trader-outcome-subtitle = Закрытые сделки по доходности, в процентах
# $value, $from and $to are formatted signed percentages.
trader-outcome-below = Ниже { $value }
trader-outcome-range = От { $from } до { $to }
trader-outcome-above = { $value } и выше
# $range is one of the three messages above; $pnl a signed SOL amount.
trader-outcome-bucket =
    { $count ->
        [one] { $range } · { $amount } сделка · { $pnl }
        [few] { $range } · { $amount } сделки · { $pnl }
        [many] { $range } · { $amount } сделок · { $pnl }
       *[other] { $range } · { $amount } сделки · { $pnl }
    }

trader-stats-empty-day = Нет закрытых сделок за последние 24 часа
trader-stats-empty-all = Закрытых сделок пока нет
trader-stats-empty-hint = Закрытые сделки появятся здесь после продажи позиции. Запустите автотрейдер или выберите более длинный период.
trader-stats-empty-days =
    { $count ->
        [one] Нет закрытых сделок за последний { $amount } день
        [few] Нет закрытых сделок за последние { $amount } дня
        [many] Нет закрытых сделок за последние { $amount } дней
       *[other] Нет закрытых сделок за последние { $amount } дня
    }
trader-exits-title = Причины выхода
trader-exits-subtitle = Чем закончились закрытые сделки и что принёс каждый выход
trader-sources-title = Источники входа
trader-sources-subtitle = Что открыло закрытые сделки
trader-recent-title = Последние закрытия
trader-recent-subtitle = Последние закрытые сделки в этом периоде
trader-column-token = Токен
trader-column-exit = Выход
trader-column-source = Источник
trader-column-trades = Сделки
trader-column-win-rate = Доля прибыльных
trader-column-avg-return = Ср. доходность
trader-column-net-pnl = Чистый P&L
trader-column-held = Удержание
trader-column-closed = Закрыта
trader-column-pnl = P&L
trader-column-return = Доходность

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
trader-stop-loss-impact = Выход при падении на { $threshold }% от входа
trader-stop-loss-hold-immediate = Сразу
trader-stop-loss-hold-delay = задержка { $span }
trader-stop-loss-price-falls = Цена падает
trader-stop-loss-threshold-reached = Порог достигнут
trader-stop-loss-sell-part = Продать { $part }
trader-stop-loss-part-of-position = Часть позиции
trader-stop-loss-sell-rest = Продать остаток
trader-stop-loss-still-below = Всё ещё ниже стопа
trader-stop-loss-summary = Убыток ограничен: <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Примечание:</strong> стоп-лосс защищает от более крупных убытков, выходя из позиции раньше

trader-trailing-title = Трейлинг-стоп
trader-trailing-subtitle = Автоматически защищать прибыль, следуя за ценой по мере её роста
trader-trailing-activation-impact = Слежение начинается при +{ $value }% прибыли
trader-trailing-distance-impact = Выход при -{ $value }% от пика
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = Зафиксированный минимум при срабатывании: { $value }
trader-trailing-locked-minimum-warning = При активации стоп оказывается на цене входа или ниже; он продаёт только в прибыли. Включите стоп в безубыток, чтобы задать нижнюю границу.
trader-trailing-activation = Активация
trader-trailing-peak = Пик
trader-trailing-final = итог +{ $value }%
trader-trailing-summary-protected = Прибыль защищена: <strong>{ $value }</strong>
trader-trailing-summary-avoided = Избегнут убыток от пика: <strong>{ $value }</strong>
# $step is the band's position in the list, from 1.
trader-trailing-band = Полоса { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-profit-label = Пиковая прибыль полосы { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-distance-label = Отступ полосы { $step }
trader-trailing-band-profit-caption = С пиковой прибыли
trader-trailing-band-distance-caption = Отступ
trader-trailing-add-band = Добавить полосу
# $step is the band's position in the list, from 1.
trader-trailing-remove-band = Удалить полосу { $step }
# $min and $max are the configured bounds of a band's peak profit.
trader-trailing-band-error-profit-range = Пиковая прибыль каждой полосы должна быть от { $min }% до { $max }%
# $min and $max are the configured bounds of a band's distance.
trader-trailing-band-error-distance-range = Отступ каждой полосы должен быть от { $min }% до { $max }%
trader-trailing-band-error-order = Пиковая прибыль каждой полосы должна быть выше активации и предыдущей полосы
trader-trailing-band-error-distance = Отступ каждой полосы должен быть меньше её пиковой прибыли

## Break-even stop

trader-break-even-title = Стоп в безубыток
trader-break-even-subtitle = Когда позиция ушла в прибыль, продать всё, пока она не стала убыточной
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = Как только лучшая цена с последней покупки достигнет { $trigger }, откат до { $offset } продаст всё.

trader-take-profit-title = Тейк-профит
trader-take-profit-subtitle = Продаёт позицию ступенями по мере достижения каждого уровня прибыли
trader-take-profit-initial-buy = Первая покупка
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = Ступень { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = Уровень прибыли ступени { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = Доля продажи ступени { $step }
trader-take-profit-profit-caption = Уровень прибыли
trader-take-profit-sell-caption = Доля продажи
trader-take-profit-add-step = Добавить ступень
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = Удалить ступень { $step }
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = Каждый уровень прибыли должен быть от { $min }% до { $max }%
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = Каждая доля должна быть от { $min }% до { $max }%
trader-take-profit-error-order = У каждой ступени уровень прибыли должен быть выше, чем у предыдущей
# $max is the largest total share, the whole position.
trader-take-profit-error-total = Все ступени вместе могут продать не более { $max }%
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = { $runner }% позиции остаётся открытыми как раннер
trader-take-profit-runner-none = Последняя ступень продаёт остаток позиции
trader-take-profit-runner-warning = Когда трейлинг-стоп, стоп в безубыток, оба выхода по времени и все стратегии выхода выключены, раннер закрывается только стоп-лоссом или вручную
trader-take-profit-info = Доли считаются от исходной позиции. Если цена проходит несколько уровней сразу, за одну проверку продаётся одна ступень.
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = При +{ $profit }% продать { $share }%
trader-take-profit-example-runner = Раннер
trader-take-profit-example-runner-detail = Остаётся на другие правила выхода
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = Продаёт <strong>{ $sold }%</strong> к +{ $profit }% прибыли

trader-time-loss-title = Закрывать убыточные позиции
trader-time-subtitle = Автоматически выходить из позиций по истечении максимального времени удержания, если убыток превышает порог
trader-time-flat-title = Закрывать позиции без движения
trader-time-flat-subtitle = Освобождает капитал из позиций, которые не растут и не падают
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = Позиция, удерживаемая { $span } или дольше с P&L в пределах ±{ $band }, закрывается.
trader-time-unit-seconds = секунды
trader-time-unit-minutes = минуты
trader-time-unit-hours = часы
trader-time-unit-days = дни
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
trader-time-loss-hint = Убыток, при котором выход по времени продаёт после истечения срока удержания (40 = выход при -40%)
trader-time-day = День { $day }
trader-time-position-opened = Позиция открыта
trader-time-limit = Лимит времени
trader-time-hold-reached = Период удержания истёк
trader-time-loss-met = Порог убытка достигнут
trader-time-note = <strong>Примечание:</strong> позиции в плюсе или с меньшим убытком закрыты НЕ будут
trader-time-positions-title = Состояние текущих позиций
trader-time-positions-loading = Загрузка позиций...
trader-time-positions-empty = Нет открытых позиций
trader-time-positions-empty-message = Здесь отображаются открытые позиции со временем удержания и ROI относительно лимита времени.
trader-time-positions-token = Токен
trader-time-positions-hold = Время удержания
trader-time-positions-roi = ROI
trader-time-positions-rule = Выход по времени
trader-time-status-off = Выходы по времени выключены
trader-time-status-none = Сейчас не действует ни один выход по времени
trader-time-status-not-managed = Не управляется автотрейдером
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = В убытке { $pnl }; закроется через { $remaining }
trader-time-status-losing-due = В убытке { $pnl }; закроется при следующей проверке
# $held and $remaining are formatted durations.
trader-time-status-flat = Без движения { $held }; закроется через { $remaining }
trader-time-status-flat-due = Без движения { $held }; закроется при следующей проверке

trader-dca-title = Усреднение (DCA)
trader-dca-subtitle = Автоматически докупать убыточные позиции, чтобы снизить среднюю цену входа
trader-dca-measure-average = Средний вход
trader-dca-measure-first-entry = Первый вход
trader-dca-step = Докупка { $step }
trader-dca-drop-label = Падение цены для докупки { $step }
trader-dca-size-label = Размер докупки { $step }, кратно первой покупке
trader-dca-drop-caption = Падение цены
trader-dca-size-caption = Покупка × первая покупка
trader-dca-add-step = Добавить уровень
trader-dca-remove-step = Удалить докупку { $step }
trader-dca-step-amount = = { $amount }
trader-dca-step-below-minimum = Меньше минимальной сделки { $minimum }: эта докупка не отправляется
trader-dca-error-drop-range = Каждое падение должно быть от { $min }% до { $max }%
trader-dca-error-size-range = Каждый размер должен быть от { $min }× до { $max }× первой покупки
trader-dca-error-order = При измерении от первого входа каждой докупке нужно падение глубже предыдущего
trader-dca-cooldown-impact = Не менее { $span } между дополнительными входами
trader-dca-min-hold-none = Первая докупка может последовать сразу после входа
trader-dca-min-hold-impact = Первая докупка ждёт не менее { $span } после входа
trader-dca-capacity = Максимум, который может держать эта позиция: { $amount } ({ $multiple }× первая покупка)
trader-dca-info-title = О стратегии DCA
trader-dca-info-subtitle = Важные особенности торговли с DCA
trader-dca-how-title = Как работает DCA
trader-dca-how-trigger = <strong>Триггер:</strong> цена опускается ниже следующего уровня, считая от среднего или первого входа
trader-dca-how-action = <strong>Действие:</strong> докупить ещё { -sol }, чтобы снизить среднюю базовую стоимость
trader-dca-how-repeat = <strong>Лестница:</strong> каждая докупка переходит к следующему уровню; когда все уровни использованы, докупки прекращаются
trader-dca-how-pause = <strong>Паузы:</strong> докупок нет, пока предыдущая ещё проводится или пока лимит убытков блокирует новые входы
trader-dca-risk-title = Предупреждения о рисках
trader-dca-risk-exposure = <strong>Рост экспозиции:</strong> DCA увеличивает общий капитал под риском в каждой позиции
trader-dca-risk-knife = <strong>Падающий нож:</strong> DCA не поможет, если токен продолжит падать
trader-dca-risk-cooldown = <strong>Пауза:</strong> используйте паузу, чтобы избежать серии быстрых докупок

trader-sizing-title = Размер позиции
trader-sizing-subtitle = Сколько вкладывать в одну позицию
trader-timing-title = Тайминг и паузы
trader-timing-subtitle = Управление временем между операциями
trader-timing-close-cooldown = Пауза после закрытия позиции
trader-timing-close-cooldown-hint = Сколько минут ждать перед повторным открытием позиции по тому же токену
trader-timing-concurrency = Параллельность проверки входов
trader-timing-concurrency-hint = Сколько токенов проверять одновременно (больше — быстрее, но выше нагрузка на процессор)
trader-timing-unit-minutes = мин
trader-timing-unit-tokens = токенов

## Service mode notice

trader-service-notice-title = Работает через сервис { -brand }
trader-service-notice-message = Цены обновляются раз в несколько секунд, поэтому быстрые выходы, например узкие трейлинг-стопы, срабатывают позже, чем со своим RPC.
