copy-skip-not-buy-swap = Активность кошелька не была покупкой
copy-skip-task-disabled = Задача приостановлена
copy-skip-mode-transition-required = Режим исполнения нужно менять отдельно
copy-skip-live-confirmation-required = Боевое исполнение требует подтверждения
copy-skip-unsupported-sizing-mode = Режим расчёта размера пока не поддерживается
copy-skip-self-copy = Кошелёк принадлежит вам
copy-skip-target-below-minimum = Сделка кошелька меньше минимума
copy-skip-target-above-maximum = Сделка кошелька больше максимума
copy-skip-already-bought = Этот токен уже куплен (покупка один раз)
copy-skip-blacklisted = Токен заблокирован контролем рисков
copy-skip-filter-required = Токен не прошёл фильтрацию
copy-skip-budget-exhausted = Бюджет задачи исчерпан
copy-skip-token-cap-reached = Достигнут лимит на токен
copy-skip-below-minimum-size = Размер копии слишком мал
copy-skip-invalid-sizing = Некорректный расчёт размера в задаче
copy-skip-invalid-slippage = Некорректное проскальзывание в задаче
copy-skip-invalid-exit-policy = Некорректные правила выхода в задаче
copy-skip-invalid-price = Нет пригодной рыночной цены
copy-skip-not-sell-swap = Активность кошелька не была продажей
copy-skip-exit-mode-disabled = Продажа кошелька проигнорирована: задача продаёт по собственным правилам
copy-skip-force-stopped = Торговля принудительно остановлена
copy-skip-copy-position-not-found = Нет позиции, принадлежащей этой задаче
copy-skip-position-user-only = Позицией управляете вы
copy-skip-position-management-mismatch = Позиция больше не следует продажам кошелька
copy-skip-latency-kill-switch = Автопауза: сделки обнаружены слишком поздно
copy-skip-claim-reconciled-abandoned = Прерванная боевая отправка закрыта без повтора
copy-skip-stale-observation = Воспроизведено после простоя, слишком старая сделка для копирования
copy-skip-unknown-observation-time = У воспроизведённой сделки нет времени блока
copy-skip-entry-blocked = Вход заблокирован

copy-entry-block-force-stopped = Торговля принудительно остановлена
copy-entry-block-loss-limit = Лимит убытков блокирует новые входы
copy-entry-block-connectivity = Необходимые сервисы недоступны
copy-entry-block-position-limit = Достигнут лимит открытых позиций
copy-entry-block-already-open = Позиция уже открыта
copy-entry-block-reentry-cooldown = Пауза до повторного входа в токен
copy-entry-block-open-cooldown = Общая пауза между входами
copy-entry-block-entry-reserved = Обрабатывается другой вход
copy-entry-block-blacklisted = Токен заблокирован контролем рисков
copy-entry-block-check-failed = Не удалось завершить проверку безопасности

copy-pause-user = Приостановлено вами
copy-pause-latency-kill-switch = Автопауза: сделки в среднем приходили с задержкой { $average } с (лимит { $threshold } с)
copy-pause-watch-detached = Автопауза: кошелёк больше не отслеживается
copy-pause-watch-budget-exceeded = Пауза: кошелёк достиг лимита проверок наблюдения ({ $limit } подписей) раньше, чем наверстал отставание
copy-pause-helius-unavailable = Пауза: не удалось выполнить проверки кошелька через { -helius }
copy-pause-watch-processing-failed = Пауза: не удалось обработать активность кошелька
copy-pause-unspecified = Пауза

copy-pause-short-user = вами
copy-pause-short-latency-kill-switch = слишком медленно
copy-pause-short-watch-detached = наблюдение потеряно
copy-pause-short-watch-budget-exceeded = лимит наблюдения
copy-pause-short-helius-unavailable = провайдер наблюдения
copy-pause-short-watch-processing-failed = обработка наблюдения
copy-state-paused = Пауза
copy-state-paused-reason = Пауза · { $reason }

copy-readiness-history = Виртуальная история
copy-readiness-history-met =
    { $count ->
        [one] { $count } закрытый виртуальный раунд, нужно { $needed }
        [few] { $count } закрытых виртуальных раунда, нужно { $needed }
        [many] { $count } закрытых виртуальных раундов, нужно { $needed }
       *[other] { $count } закрытого виртуального раунда, нужно { $needed }
    }
copy-readiness-history-short = { $count } из { $needed } закрытых виртуальных раундов
copy-readiness-profit = Прибыльность в виртуальном режиме
copy-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } реализовано за { $count } раунд, выиграно: { $wins }
        [few] { $realized } { -sol } реализовано за { $count } раунда, выиграно: { $wins }
        [many] { $realized } { -sol } реализовано за { $count } раундов, выиграно: { $wins }
       *[other] { $realized } { -sol } реализовано за { $count } раунда, выиграно: { $wins }
    }
copy-readiness-latency = Сделки обнаруживаются вовремя
copy-readiness-latency-detail = Поступление p95: { $p95 } с, лимит { $limit } с
copy-readiness-latency-none = Замеров поступления пока нет
copy-readiness-priced = У каждого актива есть цена
copy-readiness-priced-ok = У каждого открытого виртуального актива есть цена пула
copy-readiness-priced-missing =
    { $count ->
        [one] { $count } открытый актив без цены пула
        [few] { $count } открытых актива без цены пула
        [many] { $count } открытых активов без цены пула
       *[other] { $count } открытого актива без цены пула
    }
copy-readiness-runtime = Боевое исполнение доступно
copy-readiness-runtime-ok = Настройка и защитные условия допускают боевые копии

copy-live-block-setup-incomplete = Сначала завершите настройку кошелька и RPC
copy-live-block-force-stop = Включена экстренная остановка
copy-live-block-copy-trading-disabled = Обработка копирования приостановлена глобально
copy-live-block-unavailable = Боевое исполнение недоступно

## Task state, mode and exit labels

copy-state-system-paused = Приостановлено глобально
copy-state-force-stopped = Принудительно остановлено
copy-state-entries-blocked = Входы заблокированы
copy-state-running-live = Работает
copy-state-running-paper = Работает
copy-mode-paper = Виртуальный
copy-mode-live = Боевой
copy-exit-mode-buy-only = Мои правила выхода
copy-exit-mode-mirror = Повторять продажи кошелька
copy-exit-mode-hybrid = Продажи кошелька и мои правила
copy-exit-target-sell = Продал кошелёк
copy-exit-stop-loss = Стоп-лосс
copy-exit-trailing-stop = Трейлинг-стоп
copy-exit-take-profit = Тейк-профит
copy-exit-time-override = Правило по времени
copy-exit-manual = Закрыто вручную

## Shared wording

copy-request-failed = Ошибка запроса
copy-keep-paused = Оставить на паузе
copy-paused-suffix = · пауза
copy-mode-paused = { $mode } · пауза
copy-task-ref = «{ $name }» ({ $mode })
copy-metric-realized-pnl = Реализованный P&L
copy-metric-unrealized-pnl = Нереализованный P&L
copy-metric-win-rate = Доля прибыльных сделок
copy-metric-budget-spent = Потрачено из бюджета
copy-metric-median-arrival = Медиана поступления
copy-metric-open-holdings = Открытые активы
copy-record-won-lost = Выиграно: { $won } · проиграно: { $lost }
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = Исполнения
copy-kind-exits = Выходы
copy-kind-skips = Пропуски
copy-kind-errors = Ошибки
copy-field-per-trade-cap = Лимит на сделку
copy-field-per-token-cap = Лимит на токен
copy-field-total-budget = Общий бюджет
copy-field-slippage = Проскальзывание
copy-rules-wallet-sells-only = Только продажи кошелька
copy-filter-copy-setting-required = Настройка копирования (обязательно)
copy-filter-copy-setting-not-required = Настройка копирования (не обязательно)
copy-count-closed-rounds =
    { $count ->
        [one] { $count } закрытый раунд
        [few] { $count } закрытых раунда
        [many] { $count } закрытых раундов
       *[other] { $count } закрытого раунда
    }
copy-count-open-holdings =
    { $count ->
        [one] { $count } открытый актив
        [few] { $count } открытых актива
        [many] { $count } открытых активов
       *[other] { $count } открытого актива
    }
copy-unrealized-partial =
    { $priced ->
        [one] { $priced } актив с ценой · { $unpriced } без цены
        [few] { $priced } актива с ценой · { $unpriced } без цены
        [many] { $priced } активов с ценой · { $unpriced } без цены
       *[other] { $priced } актива с ценой · { $unpriced } без цены
    }
copy-unrealized-unpriced =
    { $count ->
        [one] { $count } актив без цены
        [few] { $count } актива без цены
        [many] { $count } активов без цены
       *[other] { $count } актива без цены
    }
copy-range-24h = 24 ч
copy-range-7d = 7 д
copy-range-30d = 30 д
copy-range-all = Все
copy-range-label =
    .aria-label = Период

## Page strip

copy-page-title = Копитрейдинг
copy-page-beta = Бета
copy-strip-loading = Загрузка
copy-strip-unavailable = Недоступно
copy-strip-pause-all = Приостановить все
copy-strip-resume = Возобновить обработку
copy-strip-settings = Настройки
copy-strip-add-wallet = Добавить кошелёк
copy-strip-paused-globally = Приостановлено глобально · новых копий нет, выходы продолжают работать
copy-strip-force-stopped = Принудительно остановлено · ничего не копируется
copy-strip-loss-limit = Лимит убытков · новые входы заблокированы, выходы продолжают работать
copy-strip-idle-paused =
    { $count ->
        [one] Простой · на паузе: { $count } задача
        [few] Простой · на паузе: { $count } задачи
        [many] Простой · на паузе: { $count } задач
       *[other] Простой · на паузе: { $count } задачи
    }
copy-strip-idle-empty = Простой · задач пока нет
copy-strip-processing = Обработка · виртуальных: { $paper }
copy-strip-processing-live = Обработка · боевых: { $live } · виртуальных: { $paper }
copy-figures-label =
    .aria-label = Итоги копитрейдинга
copy-figure-marked-at-pool = Оценено по цене пула
copy-figure-across-tasks = По всем задачам
copy-figure-budget-lifetime = Расходы включённых задач за всё время
copy-figure-budget-none = Нет включённых задач
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
        [one] { $count } сделка
        [few] { $count } сделки
        [many] { $count } сделок
       *[other] { $count } сделки
    }
copy-figure-arrival-none = Нет замеров по включённым задачам

## Page frame

copy-load-failed = Не удалось загрузить копитрейдинг: { $error }
copy-resume-all-title = Возобновить обработку копирования
copy-resume-all-message =
    { $count ->
        [one] { $count } боевая задача будет отправлять реальные свопы, когда её кошелёк снова совершит сделку.
        [few] { $count } боевые задачи будут отправлять реальные свопы, когда их кошельки снова совершат сделки.
        [many] { $count } боевых задач будут отправлять реальные свопы, когда их кошельки снова совершат сделки.
       *[other] { $count } боевой задачи будут отправлять реальные свопы, когда их кошельки снова совершат сделки.
    }
copy-toast-resumed-all = Обработка копирования возобновлена
copy-toast-paused-all = Вся обработка копирования приостановлена
copy-toast-global-failed = Не удалось изменить состояние обработки копирования

## Onboarding

copy-onboarding-title = Копируйте кошельки, которым доверяете, сначала проверив их в виртуальном режиме
copy-onboarding-body = Каждая задача начинается в виртуальном режиме: сделки целевого кошелька симулируются по цене пула с вашим проскальзыванием и комиссиями, а ваши правила выхода работают на виртуальном портфеле. Включайте боевой режим для отдельного кошелька, когда его виртуальные результаты это заслужат.
copy-onboarding-add = Добавьте первый кошелёк
copy-onboarding-observe = Наблюдение
copy-onboarding-observe-detail = Обнаруживайте свопы кошелька, не тратя { -sol }.
copy-onboarding-evaluate = Оценка
copy-onboarding-evaluate-detail = Изучайте виртуальный P&L, долю прибыльных сделок, пропуски, скорость обнаружения и проскальзывание.
copy-onboarding-arm = Боевой режим
copy-onboarding-arm-detail = Пройдите проверки готовности, затем включите реальные свопы.

## Wallet list

copy-list-label =
    .aria-label = Копируемые кошельки
copy-list-title = Кошельки
copy-list-compare = Сравнить
copy-list-sort-label = Сортировка кошельков
copy-list-count = Активных: { $active } · всего: { $total }
copy-sort-pnl = P&L
copy-sort-state = Состояние
copy-sort-name = Название
copy-compare-label =
    .aria-label = Сравнение кошельков

## Dialog chrome

copy-dialog-close =
    .aria-label = Закрыть
copy-editor-title-add = Добавить кошелёк
copy-editor-sub-add = Новые задачи начинаются в виртуальном режиме
copy-arm-title = Включение боевого копирования
copy-arm-sub = Реальные свопы с вашего кошелька
copy-arm-keep-paper = Остаться в виртуальном режиме
copy-arm-confirm = Включить боевой режим
copy-profile-title = Профиль кошелька
copy-profile-sub = Что этот бот видел по кошельку

## Settings dialog

copy-settings-title = Настройки копитрейдинга
copy-settings-subtitle = Общая политика для всех задач
copy-settings-filter-warning = При стандартной настройке фильтрации это отклоняет почти все токены, поэтому ничего не копируется. Оставьте выключенным, если ваши фильтры не пропускают токены, которыми торгуют ваши кошельки.
copy-settings-unit-seconds = секунд
copy-settings-unit-trades = сделок
copy-settings-unit-tasks = задач
copy-settings-unit-closed-rounds = закрытых раундов
copy-settings-save = Сохранить настройки
copy-settings-load-failed = Не удалось загрузить настройки копирования
copy-settings-saved = Настройки копитрейдинга сохранены

## Workspace

copy-tab-overview = Обзор
copy-tab-holdings = Активы
copy-tab-activity = Активность
copy-tab-rules = Правила
copy-tab-execution = Исполнение
copy-tabs-label = Разделы задачи
copy-workspace-select = Выберите кошелёк, чтобы открыть его рабочую область.
copy-workspace-loading = Загрузка задачи…
copy-workspace-load-failed = Не удалось загрузить эту задачу: { $error }

copy-state-detail-paper = Работает в виртуальном режиме · сделки симулируются, средства не тратятся
copy-state-detail-live = Работает в боевом режиме · сделки кошелька копируются реальными свопами
copy-state-detail-system-paused = Ожидание · обработка копирования приостановлена глобально, выходы продолжают работать
copy-state-detail-entries-blocked = Входы заблокированы лимитом убытков · выходы продолжают работать
copy-state-detail-force-stopped = Принудительно остановлено · ничего не копируется

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = После возобновления лимит остаётся прежним, поэтому задача снова встанет на паузу, пока сделки приходят с опозданием. Проверьте RPC-поток или увеличьте лимит поступления в настройках.
copy-paused-resume-detached = После возобновления кошелёк снова отслеживается.
copy-paused-holdings-rules =
    { $count ->
        [one] Её правила выхода по-прежнему закрывают её { $count } открытый актив.
        [few] Её правила выхода по-прежнему закрывают её { $count } открытых актива.
        [many] Её правила выхода по-прежнему закрывают её { $count } открытых активов.
       *[other] Её правила выхода по-прежнему закрывают её { $count } открытого актива.
    }
copy-paused-holdings-mirror =
    { $count ->
        [one] Продажи кошелька по-прежнему закрывают её { $count } открытый актив.
        [few] Продажи кошелька по-прежнему закрывают её { $count } открытых актива.
        [many] Продажи кошелька по-прежнему закрывают её { $count } открытых активов.
       *[other] Продажи кошелька по-прежнему закрывают её { $count } открытого актива.
    }
copy-paused-holdings-hybrid =
    { $count ->
        [one] Продажи кошелька и её правила выхода по-прежнему закрывают её { $count } открытый актив.
        [few] Продажи кошелька и её правила выхода по-прежнему закрывают её { $count } открытых актива.
        [many] Продажи кошелька и её правила выхода по-прежнему закрывают её { $count } открытых активов.
       *[other] Продажи кошелька и её правила выхода по-прежнему закрывают её { $count } открытого актива.
    }

copy-watch-state-catching-up = Наблюдение за кошельком: наверстывание. Проверка этого кошелька через { -helius }.
copy-watch-state-watching = Наблюдение за кошельком: отслеживается. Проверка этого кошелька через { -helius }.
copy-watch-last-check = Последняя проверка: { $ago }.
copy-watch-recovery-active = Наблюдение за кошельком активно
copy-watch-recovery-catching-up = Наблюдение за кошельком наверстывает отставание
copy-watch-recovery-still-paused = Задача копирования всё ещё на паузе. Возобновите копирование, когда будете готовы.
copy-watch-recovery-title = Восстановление наблюдения за кошельком
copy-watch-recovery-processing-failed = Не удалось обработать активность кошелька. Сохранённый прогресс не потерян. Повторите попытку после устранения проблемы.
copy-watch-recovery-provider-failed = Проверки через { -helius } не удались. Сохранённый прогресс не потерян. Повторите попытку, когда провайдер будет доступен.
copy-watch-recovery-budget-intro = У этого кошелька больше активности, чем текущее наблюдение успевает проверять. Выберите, как продолжить.
copy-watch-approve = Попробовать наверстать через { -helius }
copy-watch-approve-help = Продолжает с сохранённого прогресса. Может расходовать больше кредитов { -helius } и всё равно может отставать.
copy-watch-approve-unavailable = Наверстывание через { -helius } недоступно. Настройте включённый RPC-эндпоинт { -helius }, чтобы продолжить без пропуска непроверенной активности.
copy-watch-no-provider = Для этого наблюдения нет поддерживаемого провайдера наверстывания.
copy-watch-budget-label = Подписей за одну проверку
copy-watch-budget-hint = Или пропустите непроверенную активность и продолжите с текущего момента. Выберите от { $min } до { $max } подписей за проверку; чем выше лимит, тем больше может использоваться RPC-вызовов.
copy-watch-ack = Я понимаю, что пропущенная активность не будет скопирована.
copy-watch-toast-range = Выберите от { $min } до { $max } подписей за опрос с шагом { $step }
copy-watch-toast-ack = Подтвердите, что подписи с момента последней завершённой проверки будут пропущены
copy-watch-resumed = Наблюдение за кошельком возобновлено с текущего момента; задача копирования остаётся на паузе
copy-watch-resume-failed = Не удалось возобновить наблюдение за кошельком
copy-watch-retry-started = Повтор наблюдения за кошельком запущен с сохранённого прогресса; задача копирования остаётся на паузе
copy-watch-retry-failed = Не удалось повторить наблюдение за кошельком
copy-watch-approve-title = Разрешить наверстывание через { -helius } для этого кошелька
copy-watch-approve-message = { -helius } может проверять успешные транзакции Solana с сохранённого прогресса, не пропуская непроверенный интервал. Сейчас взимается 10 кредитов за каждые 100 возвращённых полных транзакций с округлением вверх, минимум 10 кредитов за запрос. Одна проверка может выполнять несколько запросов; расход и тарифы провайдера могут отличаться. Копирование остаётся на паузе, пока вы отдельно не возобновите его.
copy-watch-approve-confirm = Разрешить для этого кошелька
copy-watch-approved = Наблюдение за кошельком запущено с сохранённого прогресса; задача копирования остаётся на паузе
copy-watch-restore-failed = Не удалось восстановить наблюдение за кошельком

copy-action-pause = Приостановить
copy-action-resume = Возобновить
copy-action-resume-copy = Возобновить копирование
copy-action-resume-from-now = Возобновить с текущего момента
copy-action-retry-watch = Повторить наблюдение за кошельком
copy-action-return-paper = Вернуть в виртуальный режим
copy-action-edit-rules = Изменить правила
copy-action-clone = Клонировать
copy-action-profile = Профиль кошелька
copy-resume-live-title = Возобновление боевого копирования
copy-resume-live-message = «{ $name }» будет отправлять реальные свопы с вашего кошелька, когда этот кошелёк снова совершит сделку.
copy-resume-live-confirm = Возобновить боевой режим
copy-task-resumed = Задача возобновлена
copy-task-paused = Задача приостановлена
copy-task-state-failed = Не удалось изменить состояние задачи
copy-return-paper-message = Новые копии задачи «{ $name }» снова будут симулироваться без расхода { -sol }.
copy-return-paper-cancel = Оставить боевой режим
copy-task-returned-paper = Задача возвращена в виртуальный режим
copy-mode-change-failed = Не удалось изменить режим исполнения
copy-delete-title = Удаление задачи копирования
copy-delete-message = Удалить «{ $name }»? Её решения и виртуальные результаты будут удалены, а кошелёк перестанет отслеживаться для этой задачи.
copy-delete-confirm = Удалить задачу
copy-delete-cancel = Оставить задачу
copy-task-deleted = Задача копирования удалена
copy-task-delete-failed = Не удалось удалить задачу копирования

## Overview tab

copy-overview-results = Результаты
copy-analytics-load-failed = Не удалось загрузить аналитику: { $error }
copy-analytics-loading = Загрузка аналитики…
copy-exit-bucket =
    { $count ->
        [one] { $count } продажа · { $pnl }
        [few] { $count } продажи · { $pnl }
        [many] { $count } продаж · { $pnl }
       *[other] { $count } продажи · { $pnl }
    }
copy-overview-average-win = Средний выигрыш
copy-overview-average-loss = Средний убыток { $amount }
copy-overview-profit-factor = Профит-фактор
copy-overview-profit-factor-note = Валовая прибыль ÷ валовой убыток
copy-overview-average-hold = Среднее время удержания
copy-overview-average-hold-note = От входа до выхода
copy-overview-best-round = Лучший раунд
copy-overview-worst-round = Худший { $amount }
copy-overview-curve-title = Накопленный P&L
copy-overview-exits-title = Продажи по типам выхода
copy-overview-skips-title = Почему сделки были пропущены
copy-book-title-live = Боевой портфель
copy-book-title-paper = Виртуальный портфель
copy-book-all-time = За всё время
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
        [one] покупка
        [few] покупки
        [many] покупок
       *[other] покупки
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
        [one] выход по вашим правилам
        [few] выхода по вашим правилам
        [many] выходов по вашим правилам
       *[other] выхода по вашим правилам
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
        [one] продажа кошелька
        [few] продажи кошелька
        [many] продаж кошелька
       *[other] продажи кошелька
    }
copy-book-manual-closes = <strong>{ $count }</strong> закрыто вручную
copy-book-skipped = <strong>{ $count }</strong> пропущено
copy-book-failed = <strong>{ $count }</strong> с ошибкой
copy-book-closed = { $count } закрыто
copy-book-budget-note = { $mode }: потрачено из { $total } · осталось { $remaining }
copy-check-passed = пройдена
copy-check-not-passed = не пройдена
copy-readiness-title = Перед боевым режимом
copy-readiness-live-note = Эта задача торгует в боевом режиме. Верните её в виртуальный режим в заголовке выше.
copy-readiness-all-pass = Все проверки пройдены.
copy-readiness-needs-review = Включение боевого режима требует явной проверки того, что ещё не готово.
copy-readiness-arm = Проверить и включить боевой режим

## Rules tab and review

copy-rules-title = Действующие правила
copy-rules-size-ratio = { $pct } от сделки кошелька
copy-rules-size-fixed = { $amount } на копию
copy-rules-target-any = Любой размер
copy-rules-target-min = От { $amount }
copy-rules-target-max = До { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = Переопределено в задаче · у трейдера { $value }
copy-rules-source-default = По умолчанию трейдера
copy-rules-not-used = Не используется: решают продажи кошелька
copy-rules-col-rule = Правило
copy-rules-col-applies = Значение
copy-rules-col-source = Источник
copy-rules-budget-note = Потрачено в режиме «{ $mode }»: { $spent } · осталось { $remaining }
copy-rules-token-copies =
    { $count ->
        [one] Примерно { $count } полная копия одного токена
        [few] Примерно { $count } полные копии одного токена
        [many] Примерно { $count } полных копий одного токена
       *[other] Примерно { $count } полной копии одного токена
    }
copy-rules-sizing = Расчёт размера
copy-rules-copy-size = Размер копии
copy-rules-entry-filters = Фильтры входа
copy-rules-target-size = Размер сделки кошелька
copy-rules-repeat-buys = Повторные покупки
copy-rules-repeat-first-only = Только первая покупка каждого токена
copy-rules-repeat-every = Каждая покупка, до лимита на токен
copy-rules-filter-pass = Прохождение фильтрации
copy-rules-filter-required = Обязательно
copy-rules-filter-not-required = Не требуется
copy-rules-filter-task-override = Переопределено в задаче
copy-rules-exits = Выходы
copy-rules-exits-inactive = Активы продаются только тогда, когда продаёт кошелёк; правила ниже в этом режиме не работают.

## Exit rules

copy-rule-status = Статус
copy-rule-on = Вкл.
copy-rule-off = Выкл.
copy-rule-unit-seconds = секунд
copy-rule-unit-minutes = минут
copy-rule-stop-loss-threshold = Продаёт при убытке
copy-rule-stop-loss-min-hold = Не раньше, чем через
copy-rule-no-minimum = Без минимума
copy-rule-partial-exits = Частичные выходы
copy-rule-partial-allowed = Разрешены
copy-rule-partial-full-only = Только полный выход
copy-rule-partial-size = Размер частичного выхода
copy-rule-trailing-activation = Включается при росте на
copy-rule-trailing-distance = Продаёт при падении от пика на
copy-rule-take-profit-target = Продаёт при росте на
copy-rule-time-duration = Проверка после удержания
copy-rule-time-threshold = Продаёт, пока P&L не выше
copy-preset-inherit = По умолчанию трейдера
copy-preset-conservative = Консервативный
copy-preset-balanced = Сбалансированный
copy-preset-aggressive = Агрессивный
copy-preset-custom = Свой
copy-validate-stop-loss = Стоп-лосс должен быть больше 0% и не больше 100%.
copy-validate-partial-size = Размер частичного выхода должен быть от 0% до 100%.
copy-validate-min-hold = Минимальное время удержания должно быть целым числом секунд.
copy-validate-trailing-activation = Порог включения трейлинга должен быть больше 0% и не больше 100%.
copy-validate-trailing-distance = Дистанция трейлинга должна быть больше 0% и не больше 100%.
copy-validate-take-profit = Тейк-профит должен быть больше 0%.
copy-validate-time-duration = Для правила по времени нужна длительность больше нуля.
copy-validate-time-threshold = Порог правила по времени — это убыток: укажите 0% или отрицательное число.
copy-warning-mirror = Активы закрывают только продажи кошелька: стоп-лосс их не защищает, а токен, который кошелёк никогда не продаёт, остаётся в активах.
copy-warning-no-rules = Ни одно правило выхода не включено, а продажи кошелька игнорируются: активы никогда не продаются.
copy-warning-no-stop-loss = Стоп-лосс не действует: падающий токен удерживается, пока его не продаст другое правило или кошелёк.
copy-warning-stop-delay = Стоп-лосс ждёт { $hold } после каждой покупки: токен, который падает быстрее, закрывается значительно ниже { $threshold }.
copy-warning-take-profit-cost = Тейк-профит на уровне { $target } не покрывает продажу (проскальзывание { $slippage } и комиссия свопа { $fee }), поэтому раунды закрываются в убыток.
copy-warning-trailing-distance = Дистанция трейлинга не меньше порога его включения, поэтому включённый трейлинг может продать ниже цены входа.

## Execution tab

copy-execution-title = Качество исполнения
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = Любое
copy-execution-limit-on =
    { $count ->
        [one] Пауза, если в среднем выше { $limit } за { $count } сделку
        [few] Пауза, если в среднем выше { $limit } за { $count } сделки
        [many] Пауза, если в среднем выше { $limit } за { $count } сделок
       *[other] Пауза, если в среднем выше { $limit } за { $count } сделки
    }
copy-execution-limit-off = Аварийный выключатель выкл.
copy-execution-arrival-samples =
    { $count ->
        [one] { $count } сделка замечена в момент совершения
        [few] { $count } сделки замечены в момент совершения
        [many] { $count } сделок замечено в момент совершения
       *[other] { $count } сделки замечены в момент совершения
    }
copy-execution-p95 = Поступление p95
copy-execution-median-slippage = Медианное проскальзывание
copy-execution-slippage-samples =
    { $count ->
        [one] { $count } измеренное исполнение
        [few] { $count } измеренных исполнения
        [many] { $count } измеренных исполнений
       *[other] { $count } измеренного исполнения
    }
copy-execution-worst-slippage = Худшее проскальзывание
copy-execution-average-slippage = Среднее { $amount }
copy-execution-delay-title = Задержка обнаружения
copy-execution-delay-note = Время от блока кошелька до момента, когда этот бот увидел сделку. Воспроизведения после простоя не учитываются.
copy-execution-delay-limit = Столбцы выше лимита поступления { $limit } выделены жёлтым.
copy-execution-fastest = Самая быстрая
copy-execution-average = Средняя
copy-execution-slowest = Самая медленная
copy-execution-fill-title = Исполнение относительно кошелька
copy-execution-fill-note = Положительное значение означает хуже, чем у кошелька: при покупке заплачено больше, при зеркальной продаже получено меньше. Виртуальное исполнение токена без цены пула оценивается по сделке самого кошелька, поэтому ничего не измеряет и не учитывается.
copy-execution-samples = Замеры
copy-execution-median = Медиана
copy-execution-worst = Худшее
copy-execution-decisions = Решения за период

## Compare view

copy-compare-title = Сравнение кошельков
copy-compare-back = Назад к кошельку
copy-compare-load-failed = Не удалось загрузить сравнение: { $error }
copy-compare-loading = Загрузка сравнения…
copy-compare-empty = Нет задач для сравнения.
copy-compare-curve-title = Накопленный реализованный P&L
copy-table-wallet = Кошелёк
copy-table-mode = Режим
copy-table-rounds = Раунды
copy-table-realized = Реализовано
copy-table-profit-factor = Профит-фактор
copy-table-average-hold = Ср. удержание
copy-table-median-slippage = Медианное проскальзывание

## Charts

copy-chart-curve-label = Накопленный P&L { $amount } { -sol }
copy-chart-compare-label = Накопленный P&L по задачам
copy-chart-empty-curve = В этом периоде закрытых раундов пока нет.
copy-chart-empty-bars = В этом периоде ничего не записано.
copy-chart-empty-histogram = В этом периоде нет замеров поступления.
copy-chart-empty-compare = В этом периоде нет закрытых раундов для сравнения.
copy-chart-histogram-title = { $count } из { $total }

## Wallet profile

copy-profile-copy = Копировать этот кошелёк
copy-profile-copy-other = Копировать с другими правилами
copy-profile-loading = Загрузка профиля кошелька…
copy-profile-watch-title = Наблюдение
copy-profile-watched = Отслеживается
copy-profile-watch-resume-hint = Возобновление задачи снова включает наблюдение
copy-profile-watch-add-hint = Добавление задачи запускает наблюдение
copy-profile-stream = Поток
copy-profile-subscribed = Подписка активна
copy-profile-not-subscribed = Нет подписки
copy-profile-sources =
    { $count ->
        [one] { $count } источник
        [few] { $count } источника
        [many] { $count } источников
       *[other] { $count } источника
    }
copy-profile-last-activity = Последняя активность
copy-profile-last-error = Последняя ошибка
copy-profile-own-wallet = Это один из ваших кошельков; копирование отклонено.
copy-profile-observed-title = Замеченные сделки
copy-profile-observed-none = В этом боте пока нет сделок этого кошелька. Виртуальная задача наблюдает за ним, не тратя { -sol }.
copy-profile-swaps-seen = Замечено свопов
copy-profile-swaps-seen-note = Уникальные свопы кошелька по всем вашим задачам
copy-profile-buys-sells = Покупки / продажи
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = Токенов в торговле
copy-profile-first-seen = Впервые замечен
copy-profile-last-seen = Последний раз замечен
copy-profile-tasks-title = Ваши задачи по этому кошельку
copy-table-task = Задача

## Arm live dialog

copy-arm-acks-left =
    { $count ->
        [one] Остаётся отметить { $count } подтверждение
        [few] Остаётся отметить { $count } подтверждения
        [many] Остаётся отметить { $count } подтверждений
       *[other] Остаётся отметить { $count } подтверждения
    }
copy-arm-readiness-title = Готовность по виртуальному портфелю
copy-arm-exposure-title = Экспозиция
copy-arm-per-copy = На одну копию
copy-arm-budget-left-value = { $left } из { $total } { -sol }
copy-arm-budget-left = Остаток боевого бюджета
copy-arm-budget-left-note = Виртуальные расходы учитываются отдельно и его не расходуют
copy-arm-exits = Выходы
copy-arm-stop-note = Не раньше, чем через { $hold } удержания: при более быстром падении позиция закроется ниже
copy-arm-shared = Этот кошелёк также копируют: { $tasks }. Каждая задача копирует его сделки на собственный бюджет.
copy-arm-unavailable = Боевое исполнение сейчас недоступно; см. последнюю проверку.
copy-arm-ack-real-native = Реальные { -sol }: эта задача может потратить с вашего кошелька до { $budget } { -sol }, не более { $trade } { -sol } на одну копию.
copy-arm-ack-fees = Боевые копии оплачивают реальные сетевые комиссии и проскальзывание; виртуальные результаты не гарантируют боевых.
copy-arm-ack-unready = Часть проверок готовности не пройдена. Всё равно включить боевой режим для этой задачи.
copy-arm-lead = «{ $name }» будет копировать сделки этого кошелька реальными свопами с вашего кошелька.
copy-arm-confirmation-missing = Не удалось загрузить боевое подтверждение
copy-arm-armed = Боевое копирование включено
copy-arm-failed = Не удалось включить боевое копирование

## Holdings tab

copy-holdings-title = Активы
copy-holdings-view-label = Вид активов
copy-holdings-view-open = Открытые ({ $count })
copy-holdings-view-closed = Закрытые раунды ({ $count })
copy-holdings-reset = Сбросить виртуальный портфель
copy-holdings-live-note = Боевые копии — это реальные позиции.
copy-holdings-open-positions = Открытые позиции
copy-holdings-token-details = Открыть сведения о токене
copy-holdings-opened = Открыто { $time }
copy-holdings-no-pool-price = Нет цены пула
copy-holdings-close = Закрыть
copy-holdings-write-off = Списать
copy-holdings-activity = Активность
copy-holdings-no-exit-rule = Нет правила выхода
copy-holdings-watch-stop = Стоп { $level }
copy-holdings-watch-stop-until = Стоп { $level } через { $span }
copy-holdings-watch-take = Тейк { $level }
copy-holdings-watch-trail = Трейлинг { $level }
copy-holdings-watch-trail-arms = Трейлинг включается { $level }
copy-holdings-watch-time = Время ≤ { $level }
copy-holdings-watch-time-until = Время ≤ { $level } через { $span }
copy-holdings-watch-wallet-sells = Продажи кошелька
copy-holdings-empty = Открытых виртуальных активов нет. Покупки, скопированные с кошелька, появятся здесь.
copy-holdings-col-token = Токен
copy-holdings-col-cost = Стоимость
copy-holdings-col-entry = Вход
copy-holdings-col-mark = Оценка
copy-holdings-col-peak = Пик
copy-holdings-col-pnl = P&L
copy-holdings-col-exit-rules = Правила выхода
copy-holdings-col-held = Удержание
copy-holdings-col-actions = Действия
copy-holdings-col-invested = Вложено
copy-holdings-col-proceeds = Выручка
copy-holdings-col-exit = Выход
copy-holdings-col-closed = Закрыто
copy-holdings-price-note = Цены указаны в { -sol } за токен. Цена входа включает проскальзывание и комиссии покупки; пик и уровни выхода отсчитываются от неё, поэтому актив открывается с пиком ниже входа. Наведите курсор, чтобы увидеть цену пула.
copy-holdings-paused-rules = Пауза: новых копий нет. Ваши правила выхода по-прежнему закрывают эти активы.
copy-holdings-paused-mirror = Пауза: новых копий нет. Продажи кошелька по-прежнему закрывают эти активы.
copy-holdings-paused-hybrid = Пауза: новых копий нет. Продажи кошелька и ваши правила выхода по-прежнему закрывают эти активы.
copy-holdings-closed-load-failed = Не удалось загрузить закрытые раунды: { $error }
copy-holdings-closed-loading = Загрузка закрытых раундов…
copy-holdings-closed-empty = Закрытых раундов пока нет.
copy-holdings-closed-latest = Показаны последние: { $shown } из { $total }.
copy-holdings-close-title = Закрытие виртуального актива
copy-holdings-close-message = Продать { $token } в виртуальном портфеле по цене пула ({ $price }) с учётом проскальзывания и комиссий задачи.
copy-holdings-close-confirm = Закрыть актив
copy-holdings-write-off-title = Списание виртуального актива
copy-holdings-write-off-message = У { $token } нет цены пула для продажи. Списание закрывает актив по нулевой цене и записывает его стоимость { $cost } как убыток.
copy-holdings-keep = Оставить
copy-holdings-written-off = { $token } списан
copy-holdings-closed = { $token } закрыт
copy-holdings-written-off-detail = Закрыт с нулевой выручкой
copy-holdings-sold-at = Продан по { $price }
copy-holdings-close-failed = Не удалось закрыть актив
copy-holdings-reset-message = Начать «{ $name }» заново: её виртуальные активы, расходы, исполнения, выходы и пропуски будут удалены. Правила и кошелёк останутся.
copy-holdings-reset-cancel = Сохранить историю
copy-holdings-reset-done = Виртуальный портфель сброшен
copy-holdings-reset-detail =
    { $count ->
        [one] Удалено { $count } решение
        [few] Удалено { $count } решения
        [many] Удалено { $count } решений
       *[other] Удалено { $count } решения
    }
copy-holdings-reset-failed = Не удалось сбросить виртуальный портфель

## Activity tab

copy-activity-title = Активность
copy-activity-filter-label = Фильтр активности
copy-filter-all = Все
copy-outcome-paper-filled = Виртуальная покупка
copy-outcome-live-submitted = Боевая покупка отправлена
copy-outcome-live-confirmed = Боевая покупка подтверждена
copy-outcome-live-failed = Боевая покупка не удалась
copy-outcome-paper-sell-observed = Виртуальная продажа · продал кошелёк
copy-outcome-live-sell-submitted = Боевая продажа отправлена
copy-outcome-live-sell-failed = Боевая продажа не удалась
copy-outcome-skipped = Пропущено
copy-activity-decision = Решение
copy-activity-paper-exit = Виртуальный выход · { $rule }
copy-activity-filled = { $input } по цене { $price } · кошелёк купил { $target }
copy-activity-filled-slippage = { $input } по цене { $price } · кошелёк купил { $target } · проскальзывание { $slippage }
copy-activity-filled-unpriced = { $input } по цене { $price } · оценено по сделке кошелька, цены пула нет
copy-activity-live-sized = { $sized } · кошелёк купил { $target }
copy-activity-sell-nothing = Кошелёк продал { $amount } · нечего продавать
copy-activity-written-off = Списано по нулевой цене: нет цены пула
copy-activity-sold = Продано токенов: { $tokens } за { $proceeds } по { $price }
copy-activity-full-close = Полное закрытие
copy-activity-partial-exit = Выход на { $pct }
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = минимум { $amount }
copy-activity-skip-maximum = максимум { $value }
copy-activity-skip-stale = опоздание { $arrival }, лимит { $limit }
copy-activity-skip-latency = в среднем { $average }, лимит { $limit }
copy-activity-arrival-replayed = Воспроизведено через { $span } после блока
copy-activity-arrival-seen = Замечено через { $span } после блока
copy-activity-link-wallet-tx = Транзакция кошелька
copy-activity-link-own-tx = Ваша транзакция
copy-activity-only-token = Только этот токен
copy-activity-skipped-group = Пропущено ×{ $count }
copy-activity-group-detail =
    { $tokens ->
        [one] { $tokens } токен · с { $since }
        [few] { $tokens } токена · с { $since }
        [many] { $tokens } токенов · с { $since }
       *[other] { $tokens } токена · с { $since }
    }
copy-activity-mint-filter =
    .placeholder = Минт токена
    .aria-label = Фильтр по минту токена
copy-activity-clear = Очистить
copy-activity-load-failed = Не удалось загрузить активность: { $error }
copy-activity-loading = Загрузка активности…
copy-activity-no-match = По этому фильтру ничего не найдено.
copy-activity-empty = Решений пока нет. Исполнения, выходы и пропуски появятся здесь по мере сделок кошелька.
copy-activity-load-older = Загрузить более ранние
copy-activity-start = Начало истории
copy-activity-older-failed = Не удалось загрузить более раннюю активность

## Task editor

copy-step-wallet = Кошелёк
copy-step-sizing = Размер
copy-step-entry = Фильтры входа
copy-step-exits = Выходы
copy-step-review = Проверка
copy-editor-title-edit = Изменение: { $name }
copy-editor-title-clone = Клонирование: { $name }
copy-editor-sub-edit = Задача ({ $mode }) · изменения применяются к её следующим решениям
copy-editor-sub-clone = Те же правила, пустой виртуальный портфель, старт в виртуальном режиме
copy-editor-save-edit = Сохранить изменения
copy-editor-save-clone = Создать клон
copy-editor-save-create = Создать виртуальную задачу
copy-editor-clone-suffix = (копия)
copy-editor-discard-edit = Отменить изменения
copy-editor-discard-create = Отменить эту задачу
copy-editor-discard-edit-message = Ваши изменения задачи «{ $name }» не сохранены.
copy-editor-discard-create-message = Введённые кошелёк и правила не сохранены.
copy-editor-discard-confirm = Отменить
copy-editor-keep-editing = Продолжить редактирование
copy-editor-toast-updated = Задача обновлена
copy-editor-toast-clone = Клон создан
copy-editor-toast-created = Виртуальная задача создана
copy-unit-native = { -sol }
copy-editor-any = Любой
copy-editor-duplicate = Уже копируется: { $tasks }. Эта задача снова копирует те же сделки со своими правилами и бюджетом.
copy-editor-wallet = Кошелёк
copy-editor-wallet-identity = Кошелёк задачи — её идентификатор. Чтобы копировать другой кошелёк с этими правилами, клонируйте задачу.
copy-editor-address-label = Адрес кошелька
copy-editor-address-placeholder = Адрес кошелька Solana
copy-editor-address-help-clone = Те же правила с пустым виртуальным портфелем. Оставьте этот кошелёк, чтобы проверить на нём другие правила, или введите другой.
copy-editor-address-help-create = Кошелёк, покупки (и при выборе продажи) которого копирует эта задача.
copy-editor-name-label = Название <em>необязательно</em>
copy-editor-name-placeholder = например, Быстрый ротатор
copy-editor-enabled-title = Обрабатывать сделки кошелька
copy-editor-enabled-help = Если выключено, задача остаётся на паузе, пока вы её не возобновите.
copy-editor-note-live = Эта задача работает в боевом режиме: изменения применяются к её следующим реальным копиям.
copy-editor-note-paper = Задачи работают в виртуальном режиме, пока вы не включите боевой: сделки симулируются по цене пула, средства не тратятся.
copy-editor-copy-size = Размер копии
copy-editor-sizing-fixed = Фиксированная сумма
copy-editor-sizing-ratio = Доля от сделки кошелька
copy-editor-amount-fixed = Сумма на копию
copy-editor-amount-ratio = Доля от каждой сделки
copy-editor-amount-help-fixed = Тратится на каждую скопированную покупку, не менее { $minimum }.
copy-editor-amount-help-ratio = От собственной покупки кошелька, но не больше лимита на сделку.
copy-editor-help-trade-cap = Ни одна копия не потратит больше.
copy-editor-help-token-cap = Общая сумма, потраченная на один токен.
copy-editor-help-budget = Всё, что задача может потратить за время жизни; виртуальный и боевой режимы считают свои расходы отдельно.
copy-editor-preview-title = Сколько стоит копия
copy-editor-preview-empty = Укажите размер, чтобы увидеть стоимость копии.
copy-editor-preview-example = Кошелёк покупает на { $target } → вы копируете на <strong>{ $copy }</strong>
copy-editor-preview-once = На один токен приходится одна копия размером { $size }, так как каждый токен покупается один раз
copy-editor-preview-token-cap =
    { $count ->
        [one] На один токен приходится не более { $count } копии размером { $size }
        [few] На один токен приходится не более { $count } копий размером { $size }
        [many] На один токен приходится не более { $count } копий размером { $size }
       *[other] На один токен приходится не более { $count } копии размером { $size }
    }
copy-editor-preview-summary-exact = { $perToken }; копий в рамках бюджета — примерно { $count }. Сетевые и приоритетные комиссии не входят.
copy-editor-preview-summary-minimum = { $perToken }; копий в рамках бюджета — не менее { $count }. Сетевые и приоритетные комиссии не входят.
copy-editor-target-min = Минимальная копируемая сделка кошелька
copy-editor-target-min-help = Игнорировать меньшие покупки кошелька. Оставьте пустым, если минимума нет.
copy-editor-target-max = Максимальная копируемая сделка кошелька
copy-editor-target-max-help = Игнорировать более крупные покупки кошелька. Оставьте пустым, если максимума нет.
copy-editor-buy-once-title = Покупать каждый токен один раз
copy-editor-buy-once-help = Копировать только первую покупку токена кошельком; последующие его покупки пропускаются.
copy-editor-filter-require = Требовать
copy-editor-filter-skip = Не требовать
copy-editor-filter-help = Требовать, чтобы токен прошёл вашу фильтрацию перед копированием.
copy-editor-filter-warning = При стандартной настройке фильтрации почти каждый токен не проходит, поэтому задача с обязательной проверкой ничего не копирует. Включайте требование, только если ваши фильтры пропускают токены, которыми торгует этот кошелёк.
copy-editor-exit-both = Оба
copy-editor-exit-help-buy-only = Ваши правила ниже продают каждый актив; продажи кошелька игнорируются.
copy-editor-exit-help-hybrid = Что наступит раньше: продажа кошелька или срабатывание одного из ваших правил.
copy-editor-exit-help-mirror = Активы продаются только тогда, когда продаёт кошелёк. Ваши правила выхода не работают.
copy-editor-who-sells = Кто продаёт
copy-editor-preset = Пресет
copy-editor-preset-help = Пресет заполняет все правила ниже; затем любое из них можно изменить.
copy-editor-mirror-note = Эти правила не работают, пока решают продажи кошелька. Они применяются, если переключиться на «{ $mine }» или «{ $both }».
copy-editor-rule-inherit = По умолчанию трейдера
copy-editor-inherit-value = По умолчанию трейдера ({ $value })
copy-editor-rule-aria = Настройка: { $rule }
copy-editor-rule-empty-uses = Пустое значение использует настройку трейдера по умолчанию: { $value }
copy-editor-rule-follows = Следует настройке трейдера: { $summary }
copy-editor-rule-follows-plain = Следует настройке трейдера.
copy-editor-rule-off-note = Выключено для этой задачи, независимо от настроек трейдера.
copy-editor-unnamed = Задача без названия
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = будет обрабатывать сделки после сохранения
copy-editor-review-paused = будет сохранена на паузе
copy-editor-error-address = Введите корректный адрес кошелька Solana.
copy-editor-error-sizing = Все значения размера должны быть больше нуля.
copy-editor-error-min-copy = Копия должна быть не менее { $minimum }: увеличьте сумму на копию.
copy-editor-error-min-cap = Копия должна быть не менее { $minimum }: увеличьте лимит на сделку.
copy-editor-error-trade-cap = Лимит на сделку не может превышать лимит на токен.
copy-editor-error-token-cap = Лимит на токен не может превышать общий бюджет.
copy-editor-error-slippage = Проскальзывание должно быть от { $min } до { $max }.
copy-editor-error-target-limits = Лимиты сделок кошелька должны быть не меньше нуля.
copy-editor-error-target-order = Минимальная сделка кошелька не может превышать максимальную.

## Copy notices

copy-notice-task-unnamed = Задача №{ $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = Виртуальная копия покупки
copy-notice-title-paper-sell = Виртуальная копия продажи
copy-notice-title-paper-closed = Виртуальный актив закрыт
copy-notice-title-paper-exit = Виртуальный выход: { $rule }
copy-notice-title-live-buy-submitted = Боевая покупка отправлена
copy-notice-title-live-buy-confirmed = Боевая покупка подтверждена
copy-notice-title-live-buy-failed = Боевая покупка не удалась
copy-notice-title-live-sell-submitted = Боевая продажа отправлена
copy-notice-title-live-sell-failed = Боевая продажа не удалась
copy-notice-title-auto-paused = Задача копирования автоматически приостановлена
copy-notice-detail-bought = Куплено на { $amount } { -sol }
copy-notice-detail-sold = Продано на { $amount } { -sol }
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = { $percent }% актива
copy-notice-detail-full-close = Полное закрытие
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = Ошибка свопа
