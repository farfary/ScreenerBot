# Home page.

## Portfolio overview

home-portfolio-title = Valeur du portefeuille
home-portfolio-today = aujourd'hui
home-stat-available = { -sol } disponibles
home-stat-holdings = Tokens détenus
home-stat-open-pnl = P&L ouvert
home-stat-realized-today = Réalisé aujourd'hui

home-holdings-token-count =
    { $count ->
        [one] { $count } token
        [many] { $count } tokens
       *[other] { $count } tokens
    }
home-holdings-with-unpriced = { $tokens } · { $count } sans prix
home-holdings-unpriced-note =
    { $count ->
        [one] { $count } token détenu n'a aucun prix disponible et compte pour 0 dans le total
        [many] { $count } tokens détenus n'ont aucun prix disponible et comptent pour 0 dans le total
       *[other] { $count } tokens détenus n'ont aucun prix disponible et comptent pour 0 dans le total
    }

## Wallet address and QR code

home-wallet-copy =
    .title = Copier l'adresse du portefeuille
    .aria-label = Copier l'adresse du portefeuille
home-wallet-qr-open =
    .title = Afficher le QR code du portefeuille
    .aria-label = Afficher le QR code du portefeuille
home-wallet-qr-popover =
    .aria-label = QR code du portefeuille
home-wallet-qr-receive = Recevoir
home-wallet-qr-assets = { -sol } et tokens SPL
home-wallet-qr-close =
    .title = Fermer
    .aria-label = Fermer le QR code du portefeuille
home-wallet-qr-preparing = Préparation du QR code
home-wallet-qr-unavailable = QR code indisponible
home-wallet-qr-image =
    .alt = QR code de l'adresse du portefeuille principal

## Performance calendar

home-calendar-title = Calendrier des performances
home-calendar-previous =
    .title = Mois précédent
    .aria-label = Mois précédent
home-calendar-next =
    .title = Mois suivant
    .aria-label = Mois suivant
home-calendar-month-pnl = P&L du mois
home-calendar-trades = Trades
home-calendar-pop-net-pnl = P&L net
home-calendar-pop-win-rate = Taux de réussite
home-calendar-pop-win-rate-value = { $rate } · { $wins }G / { $losses }P
home-calendar-pop-gross-profit = Profit brut
home-calendar-pop-gross-loss = Perte brute
home-calendar-pop-end-balance = Solde final

## Position exposure and market pipeline

home-operations =
    .aria-label = État du portefeuille et du marché
home-exposure-title = Exposition des positions
home-exposure-open = ouvertes
home-exposure-invested = Investi
home-exposure-avg-size = Taille moyenne
home-exposure-avg-hold = Détention moyenne
home-exposure-best = Meilleure
home-exposure-worst = Pire
home-pipeline-title = Pipeline de marché
home-pipeline-tracked = Suivis
home-pipeline-priced = Valorisés
home-pipeline-passed = Filtres validés
home-pipeline-not-passed = Non validés (tous suivis)
home-pipeline-blacklisted = En liste noire
home-pipeline-ohlcv = OHLCV
