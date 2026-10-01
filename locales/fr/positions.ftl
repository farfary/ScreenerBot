positions-state-reason-position-created = Position créée

positions-status-open = Ouvertes
positions-status-closed = Clôturées
positions-status-archived = Archivées
positions-origin-copy = Copie
positions-origin-manual = Manuelle
positions-origin-wallet = Portefeuille
positions-origin-copy-link =
    .title = Ouvrir la tâche de copie à l'origine de cette position
positions-holding-frozen = Gelé
    .title = L'autorité de mint a gelé ce compte de token : le solde ne peut être ni transféré ni vendu
positions-toolbar-total = Total
positions-toolbar-delete-all = Tout supprimer
positions-search-placeholder = Rechercher par symbole ou mint...
positions-filter-origin = Origine
positions-filter-origin-all = Toutes les origines
positions-filter-origin-auto = Auto Trader
positions-filter-origin-copy = Copy trading
positions-delete-all-tooltip = Supprimer définitivement toutes les positions archivées

positions-column-token = Token
positions-column-archived-at = Archivée
positions-column-entry-time = Heure d'entrée
positions-column-exit-time = Heure de sortie
positions-column-avg-entry = Entrée moy. ({ -sol })
positions-column-avg-exit = Sortie moy. ({ -sol })
positions-column-current-price = Actuel ({ -sol })
positions-column-total-invested = Total investi
positions-column-proceeds = Produit
positions-column-pnl = P&L
positions-column-pnl-percent = P&L %
positions-column-size = Taille
positions-column-dca = DCA
positions-column-exits = Sorties
positions-column-unrealized-pnl = P&L latent
positions-column-unrealized-percent = Latent %

positions-unknown-basis = Aucun prix de revient dans l'historique de ce portefeuille (airdrop, exécution cotée en USD ou swap sans jambe SOL)
positions-unknown-history = Ce cycle ne concorde pas avec le solde on-chain
positions-dca-count =
    { $count ->
        [one] { $count } DCA
        [many] { $count } DCA
       *[other] { $count } DCA
    }
positions-exit-count =
    { $count ->
        [one] { $count } sortie
        [many] { $count } sorties
       *[other] { $count } sorties
    }

positions-action-add =
    .title = Renforcer la position (DCA)
    .aria-label = Renforcer la position
positions-action-sell =
    .title = Vendre (totalité ou partielle en %)
    .aria-label = Vendre la position
positions-action-sell-frozen = Gelé par l'autorité de mint : cette détention ne peut pas être vendue
positions-action-remove =
    .title = Retirer (archiver ou supprimer)
    .aria-label = Retirer la position
positions-action-restore =
    .title = Restaurer en Ouvertes/Clôturées
    .aria-label = Restaurer la position
positions-action-delete =
    .title = Supprimer définitivement
    .aria-label = Supprimer définitivement
positions-action-in-progress = En cours…

positions-caption-buying = Achat
positions-caption-buying-step = Achat · { $step }
positions-caption-selling = Vente
positions-caption-selling-step = Vente · { $step }
positions-caption-closing = Clôture
positions-caption-failed = Échec
positions-caption-failed-detail = Échec · { $error }
positions-step-adding = Renfort
positions-pending-buying = Achat…
positions-pending-buy-failed = Achat échoué

positions-load-failed = Impossible d'actualiser les positions
positions-toast-not-found = Données de la position introuvables
positions-toast-deleted = Position supprimée
positions-toast-archived = Position archivée
positions-toast-restored = Position restaurée
positions-action-failed = Échec de l'action
positions-delete-title = Supprimer définitivement la position
positions-delete-message = Supprimer définitivement { $symbol } ? Cela retire la position et son historique de la base de données et est irréversible. Vos transactions et les données du token ne sont pas affectées.
positions-delete-confirm = Supprimer définitivement
positions-delete-all-title = Supprimer toutes les positions archivées
positions-delete-all-message =
    { $count ->
        [one] Supprimer définitivement { $count } position archivée ? Cette action est irréversible. Les transactions et les données des tokens ne sont pas affectées.
        [many] Supprimer définitivement { $count } positions archivées ? Cette action est irréversible. Les transactions et les données des tokens ne sont pas affectées.
       *[other] Supprimer définitivement { $count } positions archivées ? Cette action est irréversible. Les transactions et les données des tokens ne sont pas affectées.
    }
positions-delete-all-message-empty = Supprimer définitivement toutes les positions archivées ? Cette action est irréversible.
positions-delete-all-confirm = Tout supprimer
positions-delete-all-done =
    { $count ->
        [one] { $count } position archivée supprimée
        [many] { $count } positions archivées supprimées
       *[other] { $count } positions archivées supprimées
    }
positions-delete-all-failed = Échec de la suppression des positions archivées

positions-remove-title = Retirer la position
positions-remove-open-warning = <strong>Cette position est toujours ouverte.</strong> Le bot détient ce token. Le retirer libère l'emplacement de trade et arrête le suivi, mais ne <strong>vend pas</strong>. Vendez d'abord pour récupérer vos { -sol }.
positions-remove-modes =
    .aria-label = Mode de retrait
positions-remove-archive = Archiver
positions-remove-recommended = Recommandé
positions-remove-archive-description = La déplace dans l'onglet Archivées. Réversible à tout moment : rien n'est vendu et tous les trades restent enregistrés.
positions-remove-delete = Supprimer définitivement
positions-remove-delete-description = Efface cette position et tout son historique de la base de données.
positions-remove-danger = Cela supprime définitivement la position et son historique. <strong>Cette action est irréversible.</strong> Vos transactions et les données du token ne sont pas affectées.
positions-remove-confirm-archive = Archiver la position

positions-management-changed = Gestion de la position définie sur { $mode }
positions-details-load-failed = Échec du chargement des détails de la position
positions-details-mint-label = Adresse de mint
positions-details-management-failed = Échec de la mise à jour de la gestion de la position
positions-details-favorite-add =
    .title = Ajouter aux favoris
    .aria-label = Ajouter aux favoris
positions-details-favorite-remove =
    .title = Retirer des favoris
    .aria-label = Retirer des favoris
positions-details-view-solscan =
    .title = Voir sur { -solscan }
    .aria-label = Voir le token sur { -solscan }
positions-details-close =
    .title = Fermer (Esc)
    .aria-label = Fermer
positions-details-chart-section =
    .aria-label = Graphique des prix
positions-details-loading-chart = Chargement du graphique...
positions-details-activity-section =
    .aria-label = Activité
positions-details-activity-title = Activité
positions-details-split-handle =
    .aria-label = Redimensionner le graphique et l'activité
positions-details-activity-pane =
    .aria-label = Volet d'activité
positions-details-activity-expand =
    .title = Agrandir l'activité
    .aria-label = Agrandir l'activité
positions-details-summary-section =
    .aria-label = Résumé de la position
positions-details-loading = Chargement de la position...

positions-management-auto-trader = Auto Trader
positions-management-user-only = Utilisateur seul
positions-management-copy-task = Tâche de copie
positions-management-hybrid = Hybride
positions-pane-show-chart = Afficher le graphique
positions-pane-show-activity = Afficher l'activité
positions-pane-restore-activity = Restaurer l'activité
positions-pane-expand-chart =
    .title = Agrandir le graphique
    .aria-label = Agrandir le graphique

positions-risk-low = Risque faible
positions-risk-medium = Risque moyen
positions-risk-high = Risque élevé
positions-risk-unknown = Risque inconnu
positions-busy-buying = Achat en cours…
positions-busy-selling = Vente en cours…
positions-busy-closing = Clôture en cours…
positions-header-avg-entry = Entrée moy.
positions-header-buy-count =
    { $count ->
        [one] { $count } achat
        [many] { $count } achats
       *[other] { $count } achats
    }
positions-header-exit-price = Prix de sortie
positions-header-closed-ago = clôturée { $ago }
positions-header-realized-pnl = P&L réalisé
positions-header-usd-note = USD au cours actuel du { -sol }
positions-header-returned = Récupéré
positions-header-of-invested = sur { $amount } investis
positions-header-price = Prix
positions-header-last-price = Dernier prix
positions-header-pool-ago = pool · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = P&L latent
positions-header-pnl-last-price = P&L au dernier prix
positions-header-value = Valeur
positions-header-last-value = Dernière valeur
positions-header-invested = { $amount } investis
positions-header-origin-hint = Comment cette position a été ouverte
positions-header-risk-hint = Score { -rugcheck } : plus il est bas, plus c'est sûr
positions-header-frozen = Gelé
    .title = L'autorité de mint a gelé cette détention
positions-header-managed-by = Gérée par
positions-header-management-select =
    .aria-label = Gestion de la position

positions-origin-unknown = inconnue
positions-origin-copied-task = Copiée · tâche { $task }
positions-origin-manual-entry = Entrée manuelle
positions-origin-wallet-entry = Entrée du portefeuille
positions-origin-auto-strategy = Auto · { $strategy }
positions-origin-auto-entry = Entrée auto

positions-pending-adding = Renfort
positions-pending-adding-amount = Renfort de { $amount }
positions-pending-selling = Vente
positions-pending-selling-percent = Vente de { $percent }
positions-pending-confirming = { $label } · confirmation
    .title = Envoyée, en attente de confirmation on-chain. Les chiffres se mettent à jour une fois la transaction vérifiée.

positions-trade-add = Renforcer
    .title = Renforcer la position
positions-trade-sell = Vendre
    .title = Vendre une partie de la position
positions-trade-close = Clôturer la position
    .title = Tout vendre et clôturer
positions-trade-token = Détails du token
    .title = Ouvrir les détails du token

positions-favorite-token-fallback = Token
positions-favorite-added = { $symbol } ajouté aux favoris
positions-favorite-removed = { $symbol } retiré des favoris
positions-favorite-add-failed = Échec de l'ajout du favori
positions-favorite-remove-failed = Échec du retrait du favori
positions-favorite-update-failed = Échec de la mise à jour des favoris

positions-summary-position = Position
positions-summary-price-path = Trajectoire du prix
positions-summary-network-fees = Frais de réseau
positions-summary-risk = Risque
positions-summary-market = Marché
positions-summary-market-now = Marché actuel
positions-summary-links = Liens
positions-fact-tokens-fallback = tokens
positions-fact-bought = Acheté
positions-fact-holding = Détention
positions-fact-sold = Vendu
positions-fact-realized = Réalisé
positions-fact-opened = Ouverte
positions-fact-closed = Clôturée
positions-fact-reason = Motif
positions-fact-archived = Archivée
positions-fact-entry = Entrée
positions-fact-exit = Sortie
positions-fact-total = Total
positions-fact-verified = Vérifié on-chain
positions-fact-confirming = Confirmation
positions-fact-share-of-bought = { $percent } de l'acheté
positions-fact-share-of-invested = { $percent } de l'investi
positions-fact-entry-count =
    { $count ->
        [0] 1 entrée
        [one] 1 entrée + { $count } renfort
        [many] 1 entrée + { $count } renforts
       *[other] 1 entrée + { $count } renforts
    }
positions-fact-partial-exits-back =
    { $count ->
        [one] { $count } sortie partielle · { $returned } récupérés
        [many] { $count } sorties partielles · { $returned } récupérés
       *[other] { $count } sorties partielles · { $returned } récupérés
    }
positions-fact-held = détenue { $age }
positions-fact-vs-entry = { $percent } vs entrée
positions-fact-exit-vs-peak = Sortie vs pic
positions-fact-now-vs-peak = Actuel vs pic
positions-fact-entry-range = Fourchette d'entrée
positions-range-low = Plus bas
positions-range-peak = Pic
positions-range-now = Actuel
positions-range-label-exit = Prix d'entrée et de sortie entre le plus bas et le pic
positions-range-label-now = Prix d'entrée et prix actuel entre le plus bas et le pic
positions-fact-mint-authority = Autorité de mint
positions-fact-freeze-authority = Autorité de gel
positions-fact-active = Active
positions-fact-pool = Pool
positions-fact-pool-liquidity = { $amount } { -sol } de liquidité
positions-fact-market-cap = Capitalisation
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = Liquidité
positions-fact-volume-24h = Volume 24h
positions-fact-price-change = Variation du prix
positions-change-period-1h = 1h
positions-change-period-24h = 24h
positions-fact-holders = Holders
positions-link-website = Site web
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

positions-activity-load-failed = Impossible de charger l'activité
positions-activity-loading = Chargement de l'activité...
positions-activity-empty = Il ne s'est encore rien passé pour ce token dans ce portefeuille
positions-activity-filter-empty = Aucune activité ne correspond à ce filtre
positions-activity-round-count =
    { $count ->
        [one] { $count } cycle
        [many] { $count } cycles
       *[other] { $count } cycles
    }
positions-activity-event-count =
    { $count ->
        [one] { $count } événement
        [many] { $count } événements
       *[other] { $count } événements
    }
positions-activity-pending-count = { $count } en attente
positions-activity-failed-count = { $count } en échec
positions-filter-all = Tout
positions-filter-trades = Trades
positions-filter-buys = Achats
positions-filter-sells = Ventes
positions-filter-wallet = Portefeuille
positions-filter-issues = Problèmes
positions-activity-filters =
    .aria-label = Filtrer l'activité
positions-activity-totals =
    .aria-label = Tous les cycles de ce token
positions-activity-realized-all = Réalisé, tous cycles
positions-activity-invested = Investi
positions-activity-returned = Récupéré
positions-activity-opened = Ouverte { $when }
positions-activity-round-title = Position { $index }
positions-activity-this-position = Cette position
positions-activity-dates-unavailable = Dates indisponibles
positions-activity-wallet-title = Transactions du portefeuille
positions-activity-outside =
    { $count ->
        [one] Hors position · { $range } · { $count } événement
        [many] Hors position · { $range } · { $count } événements
       *[other] Hors position · { $range } · { $count } événements
    }
positions-details-signature-label = Signature

positions-state-open = Position ouverte
positions-state-closing = Position en cours de clôture
positions-state-closed = Position clôturée
positions-state-exit-pending = Sortie de position en attente
positions-state-exit-failed = Sortie de position échouée
positions-state-phantom = Position fantôme
positions-state-reconciling = Position en rapprochement

positions-event-kind-entry = Entrée
positions-event-kind-dca = Renfort
positions-event-kind-partial-exit = Sortie partielle
positions-event-kind-exit = Sortie
positions-event-kind-buy = Achat du portefeuille
positions-event-kind-sell = Vente du portefeuille
positions-event-kind-transfer = Transfert
positions-event-kind-ata = Compte de token
positions-event-kind-other = Transaction
positions-event-state-pending = En attente
positions-event-state-failed = Échec
positions-event-state-synthetic = Synthétique
positions-chain-status-failed-detail = Échec : { $error }
positions-event-tokens-fallback = tokens
positions-event-entry-submitted = Achat envoyé pour { $amount }
positions-event-entry-for = { $amount } achetés pour { $sol }
positions-event-entry = { $amount } achetés
positions-event-dca-submitted = Renfort envoyé pour { $amount }
positions-event-dca-for = { $amount } ajoutés pour { $sol }
positions-event-dca = { $amount } ajoutés
positions-event-partial-exit-submitted-percent = Sortie partielle de { $percent } envoyée pour { $amount }
positions-event-partial-exit-submitted = Sortie partielle envoyée pour { $amount }
positions-event-sold-percent-for = { $amount } vendus ({ $percent }) pour { $sol }
positions-event-sold-percent = { $amount } vendus ({ $percent })
positions-event-sold-for = { $amount } vendus pour { $sol }
positions-event-sold = { $amount } vendus
positions-event-exit-submitted = Sortie complète de la position envoyée
positions-event-exit-for = Clôturée avec { $amount } vendus pour { $sol }
positions-event-exit-closed = Position clôturée
positions-event-wallet-bought = Le portefeuille a acheté { $amount } ailleurs
positions-event-wallet-sold = Le portefeuille a vendu { $amount } ailleurs
positions-event-received = { $amount } reçus
positions-event-sent = { $amount } envoyés
positions-event-transferred = { $amount } transférés
positions-event-ata = Activité du compte de token
positions-event-wallet-transaction = Transaction du portefeuille impliquant { $amount }
positions-event-price-per-token = { $price } { -sol } / token
positions-event-wallet-change = { $amount } de variation du portefeuille
positions-event-after-title = Position après cet événement
positions-event-capital-invested = Capital investi
positions-event-average-entry = Entrée moyenne
positions-event-transfers-title = Transferts de tokens
positions-event-transfer-amount = Montant
positions-event-transfer-mint = Mint
positions-event-transfer-from = De
positions-event-transfer-to = Vers
positions-event-no-signature = Aucune signature on-chain
positions-event-click-to-copy = Cliquer pour copier
positions-event-solscan = { -solscan }
positions-event-token-amount = Montant en tokens
positions-event-trade-price = Prix du trade
positions-event-sol-amount = Montant en { -sol }
positions-event-cost-basis = Prix de revient
positions-event-usd-value = Valeur en USD
positions-event-network-fee = Frais de réseau
positions-event-router = Routeur
positions-event-slot = Slot
positions-event-chain-status = Statut on-chain
positions-event-transaction-type = Type de transaction
positions-event-direction = Sens
positions-event-wallet-sol-change = Variation du { -sol } du portefeuille
positions-event-instructions = Instructions
positions-event-compute-units = Unités de calcul
positions-event-accounts = Comptes
positions-event-record-id = ID de l'enregistrement
positions-event-time-unavailable = Heure indisponible
positions-event-details = Détails
positions-event-hide-details = Masquer les détails

positions-chart-type-candles = Bougies
positions-chart-type-line = Ligne
positions-chart-type-area = Aire
positions-chart-type-group =
    .aria-label = Type de graphique
positions-chart-overlays-group =
    .aria-label = Surcouches du graphique
positions-chart-ema = EMA
    .title = Moyennes mobiles exponentielles, 9 et 21
positions-chart-fit = Ajuster
    .title = Cadrer la durée de vie de cette position
positions-chart-timeframes-group =
    .aria-label = Période
positions-chart-pane-group =
    .aria-label = Volet du graphique
positions-chart-unavailable = Moteur de graphique indisponible
positions-chart-collecting = Collecte des données du graphique…
positions-chart-no-data = Pas encore de données de graphique pour ce token
positions-chart-avg-entry = Entrée moy.
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = Entrée moy.
positions-chart-legend-avg-entry-off-scale = Entrée moy. (hors échelle)
positions-chart-dropped-events =
    { $count ->
        [one] { $count } événement sans bougie sur cette période
        [many] { $count } événements sans bougie sur cette période
       *[other] { $count } événements sans bougie sur cette période
    }
positions-chart-level = Niveau
positions-chart-level-above = { $label } { $price } est au-dessus de cette vue
positions-chart-level-below = { $label } { $price } est en dessous de cette vue
positions-chart-scale-hint = Faites glisser l'axe des prix pour l'inclure dans l'échelle
positions-chart-pnl-at-bar = P&L à la barre
positions-chart-click-to-locate = Cliquer pour localiser
