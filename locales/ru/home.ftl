## Portfolio overview

home-portfolio-title = Стоимость портфеля
home-portfolio-change-label = Сегодня
home-trend-range = С 00:00 UTC
home-portfolio-change-incomparable =
    .title = Цены некоторых токенов отсутствовали на начало дня или отсутствуют сейчас, поэтому изменение за сегодня не показано.
home-stat-available = Доступно { -sol }
home-stat-holdings = Токены в кошельке
home-stat-open-pnl = P&L открытых позиций
home-stat-realized-today = Реализовано сегодня

home-holdings-token-count =
    { $count ->
        [one] { $count } токен
        [few] { $count } токена
        [many] { $count } токенов
       *[other] { $count } токена
    }
home-holdings-with-unpriced = { $tokens } · без цены: { $count }
home-holdings-unpriced-note =
    { $count ->
        [one] { $count } токен в кошельке без цены учитывается в сумме как 0
        [few] { $count } токена в кошельке без цены учитываются в сумме как 0
        [many] { $count } токенов в кошельке без цены учитываются в сумме как 0
       *[other] { $count } токена в кошельке без цены учитываются в сумме как 0
    }

## Wallet address and QR code

home-wallet-copy =
    .title = Копировать адрес кошелька
    .aria-label = Копировать адрес кошелька
home-wallet-qr-open =
    .title = Показать QR-код кошелька
    .aria-label = Показать QR-код кошелька
home-wallet-qr-popover =
    .aria-label = QR-код кошелька
home-wallet-qr-receive = Получить
home-wallet-qr-assets = { -sol } и токены SPL
home-wallet-qr-close =
    .title = Закрыть
    .aria-label = Закрыть QR-код кошелька
home-wallet-qr-preparing = Подготовка QR-кода
home-wallet-qr-unavailable = QR-код недоступен
home-wallet-qr-image =
    .alt = QR-код адреса основного кошелька

## Performance calendar

home-calendar-title = Календарь результатов
home-calendar-previous =
    .title = Предыдущий месяц
    .aria-label = Предыдущий месяц
home-calendar-next =
    .title = Следующий месяц
    .aria-label = Следующий месяц
home-calendar-month-pnl = P&L за месяц
home-calendar-trades = Сделки
home-calendar-win-days = Дни прибыли
home-calendar-loss-days = Дни убытка
home-calendar-best-day = Лучший день
home-calendar-week = Неделя
home-calendar-day-label = { $date }, { $pnl }
home-calendar-pop-net-pnl = Чистый P&L
home-calendar-pop-win-rate = Доля прибыльных сделок
home-calendar-pop-win-rate-value = { $rate } · { $wins } приб. / { $losses } убыт.
home-calendar-pop-gross-profit = Валовая прибыль
home-calendar-pop-gross-loss = Валовой убыток
home-calendar-pop-end-balance = Конечный баланс

## Position exposure and market pipeline

home-operations =
    .aria-label = Состояние портфеля и рынка
home-exposure-title = Объём позиций
home-exposure-open = открыто
home-exposure-invested = Вложено
home-exposure-value = Текущая стоимость
home-exposure-position = Позиция
home-exposure-held = Время удержания
home-exposure-empty = Нет открытых позиций
home-exposure-avg-size = Средний размер
home-exposure-avg-hold = Среднее время удержания
home-exposure-best = Лучшая
home-exposure-worst = Худшая
home-pipeline-title = Рыночный конвейер
home-pipeline-tracked = В базе
home-pipeline-monitored = Отслеживается
home-pipeline-passed = Прошли фильтры
home-pipeline-rejected = Отклонены
home-pipeline-blacklisted = В чёрном списке
home-pipeline-live-price = Живая цена
home-pipeline-ohlcv = OHLCV
