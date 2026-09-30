## Portfolio overview

home-portfolio-title = Valor de la cartera
home-portfolio-today = hoy
home-stat-available = { -sol } disponible
home-stat-holdings = Tenencia de tokens
home-stat-open-pnl = P&L abierto
home-stat-realized-today = Realizado hoy

home-holdings-token-count =
    { $count ->
        [one] { $count } token
        [many] { $count } tokens
       *[other] { $count } tokens
    }
home-holdings-with-unpriced = { $tokens } · { $count } sin precio
home-holdings-unpriced-note =
    { $count ->
        [one] { $count } token en tenencia no tiene precio disponible y cuenta como 0 en el total
        [many] { $count } tokens en tenencia no tienen precio disponible y cuentan como 0 en el total
       *[other] { $count } tokens en tenencia no tienen precio disponible y cuentan como 0 en el total
    }

## Wallet address and QR code

home-wallet-copy =
    .title = Copiar dirección de la billetera
    .aria-label = Copiar dirección de la billetera
home-wallet-qr-open =
    .title = Mostrar código QR de la billetera
    .aria-label = Mostrar código QR de la billetera
home-wallet-qr-popover =
    .aria-label = Código QR de la billetera
home-wallet-qr-receive = Recibir
home-wallet-qr-assets = { -sol } y tokens SPL
home-wallet-qr-close =
    .title = Cerrar
    .aria-label = Cerrar código QR de la billetera
home-wallet-qr-preparing = Preparando código QR
home-wallet-qr-unavailable = Código QR no disponible
home-wallet-qr-image =
    .alt = Código QR de la dirección de la billetera principal

## Performance calendar

home-calendar-title = Calendario de rendimiento
home-calendar-previous =
    .title = Mes anterior
    .aria-label = Mes anterior
home-calendar-next =
    .title = Mes siguiente
    .aria-label = Mes siguiente
home-calendar-month-pnl = P&L del mes
home-calendar-trades = Operaciones
home-calendar-pop-net-pnl = P&L neto
home-calendar-pop-win-rate = Tasa de aciertos
home-calendar-pop-win-rate-value = { $rate } · { $wins }G / { $losses }P
home-calendar-pop-gross-profit = Ganancia bruta
home-calendar-pop-gross-loss = Pérdida bruta
home-calendar-pop-end-balance = Saldo final

## Position exposure and market pipeline

home-operations =
    .aria-label = Estado de la cartera y del mercado
home-exposure-title = Exposición de posiciones
home-exposure-open = abiertas
home-exposure-invested = Invertido
home-exposure-avg-size = Tamaño promedio
home-exposure-avg-hold = Tenencia promedio
home-exposure-best = Mejor
home-exposure-worst = Peor
home-pipeline-title = Flujo de mercado
home-pipeline-tracked = En seguimiento
home-pipeline-priced = Con precio
home-pipeline-passed = Filtros aprobados
home-pipeline-rejected = Rechazados
home-pipeline-blacklisted = En lista negra
home-pipeline-ohlcv = OHLCV
