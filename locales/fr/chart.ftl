# Chart vocabulary shared by the position details and token details charts.

chart-data = Données
chart-ohlc-open = O
chart-ohlc-high = H
chart-ohlc-low = B
chart-ohlc-close = C
chart-loading = Chargement des données du graphique…
chart-waiting = En attente des données du graphique…
chart-marker-dca = DCA { $index }
chart-marker-exit-numbered = Sortie { $index }
chart-candles = Bougies

chart-status-none = Aucune donnée de graphique pour l'instant
chart-status-ready = Données prêtes
chart-status-partial = Collecte de l'historique…
chart-status-collecting = Récupération des données…
chart-status-aria = Données du graphique : { $summary }
chart-status-last-candle = Dernière nouvelle bougie
chart-status-checked = vérifié { $ago }
chart-status-checking = vérification…
chart-status-not-checked = non vérifié
chart-status-updated = mis à jour { $ago }
chart-status-no-candles = aucune bougie pour l'instant
chart-status-column-timeframe = UT
chart-status-column-new = Nouv.
chart-status-total-monitoring =
    { $count ->
        [one] { $count } bougie · suivi actif
        [many] { $count } bougies · suivi actif
       *[other] { $count } bougies · suivi actif
    }
chart-status-total-idle =
    { $count ->
        [one] { $count } bougie · inactif
        [many] { $count } bougies · inactif
       *[other] { $count } bougies · inactif
    }
