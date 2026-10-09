home-portfolio-title = Valor do portfólio
home-portfolio-today = hoje
home-stat-available = { -sol } disponível
home-stat-holdings = Tokens em carteira
home-stat-open-pnl = P&L aberto
home-stat-realized-today = Realizado hoje

home-holdings-token-count =
    { $count ->
        [one] { $count } token
        [many] { $count } tokens
       *[other] { $count } tokens
    }
home-holdings-with-unpriced = { $tokens } · { $count } sem preço
home-holdings-unpriced-note =
    { $count ->
        [one] { $count } token em carteira não tem preço disponível e conta como 0 no total
        [many] { $count } tokens em carteira não têm preço disponível e contam como 0 no total
       *[other] { $count } tokens em carteira não têm preço disponível e contam como 0 no total
    }

home-wallet-copy =
    .title = Copiar endereço da carteira
    .aria-label = Copiar endereço da carteira
home-wallet-qr-open =
    .title = Mostrar QR code da carteira
    .aria-label = Mostrar QR code da carteira
home-wallet-qr-popover =
    .aria-label = QR code da carteira
home-wallet-qr-receive = Receber
home-wallet-qr-assets = { -sol } e tokens SPL
home-wallet-qr-close =
    .title = Fechar
    .aria-label = Fechar QR code da carteira
home-wallet-qr-preparing = Preparando o QR code
home-wallet-qr-unavailable = QR code indisponível
home-wallet-qr-image =
    .alt = QR code do endereço da carteira principal

home-calendar-title = Calendário de desempenho
home-calendar-previous =
    .title = Mês anterior
    .aria-label = Mês anterior
home-calendar-next =
    .title = Próximo mês
    .aria-label = Próximo mês
home-calendar-month-pnl = P&L do mês
home-calendar-trades = Trades
home-calendar-pop-net-pnl = P&L líquido
home-calendar-pop-win-rate = Taxa de acerto
home-calendar-pop-win-rate-value = { $rate } · { $wins }G / { $losses }P
home-calendar-pop-gross-profit = Lucro bruto
home-calendar-pop-gross-loss = Prejuízo bruto
home-calendar-pop-end-balance = Saldo final

home-operations =
    .aria-label = Status do portfólio e do mercado
home-exposure-title = Exposição das posições
home-exposure-open = abertas
home-exposure-invested = Investido
home-exposure-avg-size = Tamanho médio
home-exposure-avg-hold = Tempo médio de posse
home-exposure-best = Melhor
home-exposure-worst = Pior
home-pipeline-title = Pipeline de mercado
home-pipeline-tracked = Monitorados
home-pipeline-priced = Com preço
home-pipeline-passed = Aprovados nos filtros
home-pipeline-not-passed = Não aprovados (todos monitorados)
home-pipeline-blacklisted = Na lista negra
home-pipeline-ohlcv = OHLCV
