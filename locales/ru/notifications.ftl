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
notifications-state-locked = Состояние определяется вкладкой
notifications-cancelled = Отменено
notifications-dismiss = Закрыть
notifications-dismiss-failed = Не удалось закрыть уведомление
notifications-load-failed = Не удалось загрузить
notifications-mark-read-failed = Не удалось отметить уведомления прочитанными
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
