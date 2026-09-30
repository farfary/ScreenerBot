## Portfolio overview

home-portfolio-title = Вартість портфеля
home-portfolio-today = сьогодні
home-stat-available = Доступні { -sol }
home-stat-holdings = Утримувані токени
home-stat-open-pnl = Відкритий P&L
home-stat-realized-today = Реалізовано сьогодні

home-holdings-token-count =
    { $count ->
        [one] { $count } токен
        [few] { $count } токени
        [many] { $count } токенів
       *[other] { $count } токена
    }
home-holdings-with-unpriced = { $tokens } · без ціни: { $count }
home-holdings-unpriced-note =
    { $count ->
        [one] { $count } утримуваний токен не має ціни й враховується в підсумку як 0
        [few] { $count } утримувані токени не мають ціни й враховуються в підсумку як 0
        [many] { $count } утримуваних токенів не мають ціни й враховуються в підсумку як 0
       *[other] { $count } утримуваного токена не має ціни й враховується в підсумку як 0
    }

## Wallet address and QR code

home-wallet-copy =
    .title = Копіювати адресу гаманця
    .aria-label = Копіювати адресу гаманця
home-wallet-qr-open =
    .title = Показати QR-код гаманця
    .aria-label = Показати QR-код гаманця
home-wallet-qr-popover =
    .aria-label = QR-код гаманця
home-wallet-qr-receive = Отримати
home-wallet-qr-assets = { -sol } і токени SPL
home-wallet-qr-close =
    .title = Закрити
    .aria-label = Закрити QR-код гаманця
home-wallet-qr-preparing = Підготовка QR-коду
home-wallet-qr-unavailable = QR-код недоступний
home-wallet-qr-image =
    .alt = QR-код адреси основного гаманця

## Performance calendar

home-calendar-title = Календар результатів
home-calendar-previous =
    .title = Попередній місяць
    .aria-label = Попередній місяць
home-calendar-next =
    .title = Наступний місяць
    .aria-label = Наступний місяць
home-calendar-month-pnl = P&L за місяць
home-calendar-trades = Угоди
home-calendar-pop-net-pnl = Чистий P&L
home-calendar-pop-win-rate = Відсоток виграшних угод
home-calendar-pop-win-rate-value = { $rate } · { $wins }В / { $losses }П
home-calendar-pop-gross-profit = Загальний прибуток
home-calendar-pop-gross-loss = Загальний збиток
home-calendar-pop-end-balance = Кінцевий баланс

## Position exposure and market pipeline

home-operations =
    .aria-label = Стан портфеля та ринку
home-exposure-title = Експозиція позицій
home-exposure-open = відкрито
home-exposure-invested = Інвестовано
home-exposure-avg-size = Середній розмір
home-exposure-avg-hold = Середній час утримання
home-exposure-best = Найкраща
home-exposure-worst = Найгірша
home-pipeline-title = Ринковий конвеєр
home-pipeline-tracked = Відстежується
home-pipeline-priced = З ціною
home-pipeline-passed = Пройшли фільтри
home-pipeline-rejected = Відхилено
home-pipeline-blacklisted = У чорному списку
home-pipeline-ohlcv = OHLCV
