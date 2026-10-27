notifications-action-swap-buy = Покупка
notifications-action-swap-sell = Продажа
notifications-action-position-open = Открытие
notifications-action-position-close = Закрытие
notifications-action-position-dca = DCA
notifications-action-position-partial-exit = Частичный выход
notifications-action-manual-order = Вручную
notifications-action-unknown = Действие

actions-step-evaluate = Оценка
actions-step-validate = Проверка
actions-step-quote = Получение котировки
actions-step-swap = Выполнение свопа
actions-step-verify = Верификация
actions-step-unknown = Обработка
actions-step-evaluate-short = Оценка
actions-step-validate-short = Проверка
actions-step-quote-short = Котировка
actions-step-swap-short = Своп
actions-step-verify-short = Подтверждение
actions-step-unknown-short = В работе

actions-failure-recorded = { $message }
actions-failure-unknown = Неизвестная ошибка
actions-failure-interrupted = Прервано перезапуском приложения
actions-failure-validation = Проверка не пройдена
actions-failure-quote = Не удалось получить котировку
actions-failure-swap = Ошибка свопа
actions-failure-trade = Ошибка сделки
actions-failure-entry = Ошибка входа
actions-failure-entry-withdrawn = Вход отозван
actions-failure-exit = Ошибка выхода
actions-failure-dca = Ошибка DCA
actions-failure-verification-expired = Время верификации истекло: транзакция так и не была записана в блокчейн
actions-failure-verification-gave-up = Верификация прекращена
actions-failure-transaction-failed = Транзакция завершилась ошибкой в блокчейне
actions-failure-sell-transaction-failed = Транзакция продажи завершилась ошибкой в блокчейне
actions-failure-dca-verification-failed = Верификация DCA не пройдена

notifications-empty-all = Нет действий
notifications-empty-active = Нет активных действий
notifications-empty-completed = Нет завершённых действий
notifications-empty-failed = Нет неудачных действий
notifications-source-auto = Авто
notifications-source-manual = Вручную
notifications-cancelled = Отменено
notifications-load-failed = Не удалось загрузить
notifications-clear-title = Очистить уведомления
notifications-clear-message = Убрать все уведомления из этого списка? Они останутся в истории завершённых и неудачных действий.
notifications-clear-failed = Не удалось очистить уведомления
notifications-stream-lag-title = Поток действий отстал
notifications-stream-lag-missed =
    Пропущено { $count ->
        [one] { $count } обновление
        [few] { $count } обновления
        [many] { $count } обновлений
       *[other] { $count } обновления
    } — идёт обновление
notifications-stream-lag-refreshing = Обновление
notifications-sync-failed = Не удалось обновить действия

# Action rows (ui/action_row.js): day headings, the copy-trading source, the detail.
notifications-day-today = Сегодня
notifications-day-yesterday = Вчера
notifications-source-copy = Копитрейдинг
notifications-steps = Шаги
notifications-detail-transaction = Транзакция
notifications-detail-fill-price = Цена исполнения
    .title = Сумма в { -sol } этого свопа, делённая на количество токенов
notifications-detail-fee = Комиссия сети
notifications-detail-pnl = P&L позиции
notifications-detail-route = Маршрут
notifications-detail-duration = Длительность
notifications-detail-reason = Причина
notifications-view-token = Показать токен
notifications-view-position = Показать позицию
notifications-view-transaction = Показать транзакцию
notifications-unrecorded = Не учтено в позиции
notifications-unrecorded-detail = Этот своп не учтён в позиции и, возможно, не прошёл. Проверьте его транзакцию.
