## Portfolio overview

home-portfolio-title = Portfoliowert
home-portfolio-today = heute
home-stat-available = Verfügbare { -sol }
home-stat-holdings = Token-Bestand
home-stat-open-pnl = Offene GuV
home-stat-realized-today = Realisiert heute

home-holdings-token-count =
    { $count ->
        [one] { $count } Token
       *[other] { $count } Tokens
    }
home-holdings-with-unpriced = { $tokens } · { $count } ohne Preis
home-holdings-unpriced-note =
    { $count ->
        [one] Für { $count } gehaltenen Token ist kein Preis verfügbar; er zählt in der Summe als 0
       *[other] Für { $count } gehaltene Tokens ist kein Preis verfügbar; sie zählen in der Summe als 0
    }

## Wallet address and QR code

home-wallet-copy =
    .title = Wallet-Adresse kopieren
    .aria-label = Wallet-Adresse kopieren
home-wallet-qr-open =
    .title = Wallet-QR-Code anzeigen
    .aria-label = Wallet-QR-Code anzeigen
home-wallet-qr-popover =
    .aria-label = Wallet-QR-Code
home-wallet-qr-receive = Empfangen
home-wallet-qr-assets = { -sol } und SPL-Tokens
home-wallet-qr-close =
    .title = Schließen
    .aria-label = Wallet-QR-Code schließen
home-wallet-qr-preparing = QR-Code wird vorbereitet
home-wallet-qr-unavailable = QR-Code nicht verfügbar
home-wallet-qr-image =
    .alt = QR-Code der Adresse des Haupt-Wallets

## Performance calendar

home-calendar-title = Performance-Kalender
home-calendar-previous =
    .title = Vorheriger Monat
    .aria-label = Vorheriger Monat
home-calendar-next =
    .title = Nächster Monat
    .aria-label = Nächster Monat
home-calendar-month-pnl = Monats-GuV
home-calendar-trades = Trades
home-calendar-pop-net-pnl = Netto-GuV
home-calendar-pop-win-rate = Gewinnquote
home-calendar-pop-win-rate-value = { $rate } · { $wins } G / { $losses } V
home-calendar-pop-gross-profit = Bruttogewinn
home-calendar-pop-gross-loss = Bruttoverlust
home-calendar-pop-end-balance = Endsaldo

## Position exposure and market pipeline

home-operations =
    .aria-label = Portfolio- und Marktstatus
home-exposure-title = Positions-Exposure
home-exposure-open = offen
home-exposure-invested = Investiert
home-exposure-avg-size = Durchschnittsgröße
home-exposure-avg-hold = Durchschnittliche Haltedauer
home-exposure-best = Beste
home-exposure-worst = Schlechteste
home-pipeline-title = Markt-Pipeline
home-pipeline-tracked = Beobachtet
home-pipeline-priced = Mit Preis
home-pipeline-passed = Filter bestanden
home-pipeline-not-passed = Nicht bestanden (alle beobachteten)
home-pipeline-blacklisted = Auf der Blacklist
home-pipeline-ohlcv = OHLCV
