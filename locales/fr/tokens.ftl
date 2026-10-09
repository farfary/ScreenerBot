tokens-result-source-live = Données de marché en direct
tokens-result-source-unavailable = { $label } indisponible — nouvelle tentative
tokens-result-source-not-listed = Non listé sur { $label }
tokens-result-security-available = Rapport de sécurité disponible
tokens-result-security-missing = Aucun rapport { -rugcheck }
tokens-result-chart-available = Données de graphique disponibles
tokens-result-chart-missing = Pas encore de données de graphique

tokens-state-error-title = Impossible de charger les données
tokens-state-offline = Vous semblez être hors ligne.
tokens-state-request-failed = La requête a échoué après plusieurs tentatives.
tokens-state-waiting = En attente des données…

tokens-chart-marker-entry = Entrée
tokens-chart-level-stop-loss = Stop loss
tokens-chart-level-take-profit = Take profit

tokens-transactions-loading = Chargement des transactions…
tokens-transactions-empty-title = Aucune transaction
tokens-transactions-empty-history = Aucun historique de transactions du portefeuille n'est disponible pour ce token.
tokens-transactions-empty-data = Aucune donnée de transaction n'est disponible pour ce token.
tokens-transactions-error-title = Impossible de charger les transactions
tokens-transactions-error-message = L'historique des transactions est temporairement indisponible.
tokens-transactions-activity-title = Activité sur 24 h
tokens-transactions-activity-subtitle = Transactions du portefeuille par heure
tokens-transactions-metric-total = Total
tokens-transactions-metric-buys = Achats
tokens-transactions-metric-sells = Ventes
tokens-transactions-recent-title = Transactions récentes
tokens-transactions-shown = { $count } affichées
tokens-transactions-column-time = Heure
tokens-transactions-column-type = Type
tokens-transactions-column-price = Prix ({ -sol })
tokens-transactions-column-total = Total ({ -sol })
tokens-transactions-chart-missing = Bibliothèque de graphiques manquante
tokens-transactions-view-solscan = Voir la transaction sur { -solscan }

tokens-positions-empty-title = Aucune position
tokens-positions-no-token = Aucun token sélectionné.
tokens-positions-empty-message = Aucune position sur ce token pour le moment. Utilisez Acheter pour en ouvrir une.
tokens-positions-loading = Chargement de la position…
tokens-positions-from-wallet-history = Issue de l'historique du portefeuille
tokens-positions-frozen = Gelé — ne peut pas être vendu
tokens-positions-no-cost-basis = Aucun prix de revient
tokens-positions-history-incomplete = Historique incomplet
tokens-positions-dca-count = DCA { $count }
tokens-positions-exit-count = Sorties { $count }
tokens-positions-fact-avg-entry = Entrée moy.
tokens-positions-fact-current = Actuel
tokens-positions-fact-tokens = Tokens
tokens-positions-fact-opened = Ouverture
tokens-positions-fact-exit-price = Prix de sortie
tokens-positions-fact-native-received = { -sol } reçu
tokens-positions-fact-closed-reason = Motif de clôture
tokens-positions-fact-target-min = Objectif de profit min.
tokens-positions-fact-target-max = Objectif de profit max.
tokens-positions-fact-highest = Prix le plus haut
tokens-positions-fact-lowest = Prix le plus bas
tokens-positions-section-range = Objectifs et fourchette
tokens-positions-section-market = Marché et détention
tokens-positions-kicker = Position
tokens-positions-fallback-symbol = Token
tokens-positions-realized-pnl = P&L réalisé
tokens-positions-unrealized-pnl = P&L latent
tokens-positions-size = Taille

tokens-security-analysis-pending = Analyse { -rugcheck } en cours...
tokens-security-analyzing = Analyse de la sécurité…
tokens-security-pulse-title = Pouls de sécurité
tokens-security-pending-caption = Les signaux de risque sont encore en cours de collecte.
tokens-security-control-title = Contrôle du token
tokens-security-control-meta = Statut des autorités
tokens-security-updated = Mis à jour { $time }
tokens-security-score-caption = Score de risque normalisé du token sur 100.
tokens-security-score-label = Score
tokens-security-rugged = Rug pull avéré
tokens-security-grade-analyzing = Analyse
tokens-security-grade-shielded = Protégé
tokens-security-grade-safe = Sûr
tokens-security-grade-caution = Prudence
tokens-security-grade-vulnerable = Vulnérable
tokens-security-grade-unknown = Inconnu
tokens-security-metric-token-type = Type de token
tokens-security-metric-total-holders = Total des holders
tokens-security-metric-lp-providers = Fournisseurs de LP
tokens-security-metric-graph-insiders = Initiés du graphe
tokens-security-insiders-detected = Détectés ({ $count })
tokens-security-insiders-clean = Aucun
tokens-security-authority-mint = Mint
tokens-security-authority-freeze = Gel
tokens-security-authority-immutable = Immuable
tokens-security-authority-mutable = Modifiable
tokens-security-authority-revoked = Révoquée
tokens-security-authority-active = Active
tokens-security-holder-health-title = Santé des holders
tokens-security-holders-unique = uniques
tokens-security-creator-share = Part du créateur
tokens-security-gauge-top-10 = Top 10
tokens-security-concentration-unknown = Inconnue
tokens-security-concentration-critical = Critique
tokens-security-concentration-high = Élevée
tokens-security-concentration-moderate = Modérée
tokens-security-concentration-healthy = Saine
tokens-security-transfer-title = Taxe de transfert
tokens-security-transfer-no-fee = Sans frais
tokens-security-transfer-fee-percentage = Pourcentage des frais
tokens-security-transfer-max-fee = Montant max. des frais
tokens-security-transfer-authority = Autorité des frais
tokens-security-transfer-note = Des frais de { $percent } sont prélevés à chaque transfert.
tokens-security-transfer-none = Aucun frais de transfert détecté.
tokens-security-risks-title = Risques de sécurité
tokens-security-risks-none = Aucun risque de sécurité détecté.
tokens-security-risk-fallback-name = Signal de sécurité
tokens-security-risks-critical = { $count } critique(s)
tokens-security-risks-warnings =
    { $count ->
        [one] { $count } avertissement
        [many] { $count } avertissements
       *[other] { $count } avertissements
    }
tokens-security-risks-info = { $count } info(s)
tokens-security-risks-incidents =
    { $count ->
        [one] { $count } incident trouvé
        [many] { $count } incidents trouvés
       *[other] { $count } incidents trouvés
    }
tokens-security-top-holders-title = Principaux holders
tokens-security-top-holders-concentration = concentration de { $percent }
tokens-security-insider = Initié

tokens-overview-chart-checking = Vérification des données…
tokens-overview-banner-open = Ouvrir la bannière du token
tokens-overview-headline-label = Principales métriques de marché
tokens-overview-price = Prix
tokens-overview-market-cap = Capitalisation
tokens-overview-liquidity = Liquidité
tokens-overview-volume = Volume
tokens-overview-no-tags = Aucun tag
tokens-overview-info-title = Infos du token
tokens-overview-profile = Profil publié
tokens-overview-fact-mint = Mint
tokens-overview-fact-decimals = Décimales
tokens-overview-fact-age = Âge
tokens-overview-fact-dex = DEX
tokens-overview-fact-holders = Holders
tokens-overview-fact-top-10 = Top 10 détenu
tokens-overview-tags = Tags
tokens-overview-liquidity-title = Liquidité et marché
tokens-overview-fact-fdv = FDV
tokens-overview-fact-pool-native = { -sol } du pool
tokens-overview-fact-pool-token = Token du pool
tokens-overview-pool = Pool
tokens-overview-pulse-title = Pouls du marché
tokens-overview-activity-title = Activité des transactions
tokens-overview-buy-share = { $percent } d'achats
tokens-overview-buy-sell-ratio = { $ratio } A/V
tokens-overview-buys-24h = Achats 24H
tokens-overview-sells-24h = Ventes 24H
tokens-overview-net-flow = Flux net
tokens-overview-total-24h = Total 24H
tokens-overview-average-24h = Moy. 24H
tokens-overview-spike-5m = Pic 5M
tokens-overview-rate-per-hour = { $amount }/h
tokens-overview-rate-per-minute = { $amount }/min
tokens-overview-spike-factor = { $factor }×
tokens-overview-flow-counts = Achats : { $buys } ({ $buyPercent }), Ventes : { $sells } ({ $sellPercent }), Total : { $total }
tokens-overview-flow-no-data = Aucune donnée de transaction

tokens-pools-empty-title = Aucun pool
tokens-pools-empty-message = Aucun pool de liquidité n'a été détecté pour ce token.
tokens-pools-unknown = Inconnu
tokens-pools-unknown-dex = DEX inconnu
tokens-pools-liquidity = Liquidité
tokens-pools-volume-24h = Volume 24 h
tokens-pools-base-role = Rôle de base
tokens-pools-quote-role = Rôle de cotation
tokens-pools-canonical = Canonique
tokens-pools-summary-title = Résumé des pools
tokens-pools-breakdown-title = Répartition par DEX
tokens-pools-all-title = Tous les pools
tokens-pools-updated = Mis à jour
tokens-pools-role-base = Base
tokens-pools-role-quote = Cotation
tokens-pools-role-unknown = Inconnu
tokens-pools-reserves = Comptes de réserve
tokens-pools-no-reserves = Aucun compte de réserve
tokens-pools-address-pool = Pool
    .title = Copier le pool
tokens-pools-address-base = Mint de base
    .title = Copier le mint de base
tokens-pools-address-quote = Mint de cotation
    .title = Copier le mint de cotation
    .title = Copier le mint apparié

tokens-links-empty = Aucun site web officiel ni lien de réseau social n'est disponible pour ce token.
tokens-links-info-title = Infos du token
tokens-links-mint-address = Adresse de mint
tokens-links-data-source = Source des données
tokens-links-security = Sécurité
tokens-links-profile-title = Profil du token
tokens-links-profile-published-title = Contenu du profil publié
tokens-links-profile-published-note = Les médias, la description et les liens officiels sont un contenu de profil payant, examiné avant publication. Cela ne vérifie ni la propriété ni la sécurité du token.
tokens-links-profile-create-note = Ajoutez un logo vérifié, une description du projet et des liens officiels au profil public de ce token.
tokens-links-profile-update-hint = Mettre à jour le profil de ce token sur screenerbot.io
tokens-links-profile-create-hint = Créer un profil de token sur screenerbot.io
tokens-links-profile-update = Mettre à jour le profil
tokens-links-profile-create = Créer le profil
tokens-links-media-title = Ressources médias
tokens-links-media-fallback-symbol = Token
tokens-links-media-logo = Logo
tokens-links-media-banner = Bannière
tokens-links-media-banner-alt = Bannière { $symbol }
tokens-links-media-open = Ouvrir l'image
tokens-links-description-title = Description
tokens-links-explorers-title = Explorateurs et analyses
tokens-links-websites-title = Sites web officiels
tokens-links-socials-title = Réseaux sociaux
tokens-links-explorer-solana-explorer = { -solana-explorer }
tokens-links-explorer-geckoterminal = { -geckoterminal }
tokens-links-explorer-dextools = { -dextools }
tokens-links-explorer-coingecko = { -coingecko }
tokens-links-explorer-jupiter-swap = { -jupiter } Swap
tokens-links-social-twitter = { -twitter } / { -x }
tokens-links-social-x = { -x } ({ -twitter })
tokens-links-social-telegram = { -telegram }
tokens-links-social-discord = { -discord }
tokens-links-social-medium = { -medium }
tokens-links-social-github = { -github }
tokens-links-social-youtube = { -youtube }
tokens-links-social-reddit = { -reddit }
tokens-links-social-facebook = { -facebook }
tokens-links-social-instagram = { -instagram }
tokens-links-social-linkedin = { -linkedin }
tokens-links-social-tiktok = { -tiktok }
tokens-links-social-fallback = Réseau social

tokens-dialog-tab-overview = Aperçu
tokens-dialog-tab-security = Sécurité
tokens-dialog-tab-positions = Positions
tokens-dialog-tab-pools = Pools
tokens-dialog-tab-links = Liens
tokens-dialog-tab-transactions = Txs
tokens-dialog-sections = Sections des détails du token
tokens-dialog-close =
    .title = Fermer (ÉCHAP)
    .aria-label = Fermer les détails du token
tokens-dialog-unknown-symbol = Inconnu
tokens-dialog-unknown-name = Token inconnu
tokens-dialog-market-summary = Résumé du marché
tokens-dialog-price-loading = Chargement du prix
tokens-dialog-unit-native = { -sol }
tokens-dialog-market-metrics = Métriques de marché
tokens-dialog-metric-market-cap = Capitalisation
tokens-dialog-metric-volume-24h = Volume 24 h
tokens-dialog-change-24h = Variation sur 24 heures { $change }
tokens-dialog-buy = Acheter
    .title = Acheter ce token
tokens-dialog-sell = Vendre
    .title = Vendre la position
tokens-dialog-sell-unavailable = Aucune position ouverte à vendre
tokens-dialog-details = Détails
tokens-dialog-sources = Sources
tokens-dialog-sources-status = Statut des sources de données
tokens-dialog-updated-label = Mis à jour
tokens-dialog-just-now = À l'instant
tokens-dialog-updated-at = Mis à jour { $time }
tokens-dialog-updated-unavailable = Heure de mise à jour indisponible
tokens-dialog-error-title = Impossible de charger les données du token
tokens-dialog-waiting-token = En attente des données du token…
tokens-dialog-loading-overview = Chargement de l'aperçu…
tokens-dialog-loading-security = Chargement de la sécurité…
tokens-dialog-loading-pools = Chargement des pools…
tokens-dialog-loading-links = Chargement des liens…
tokens-dialog-chart-still-checking = Aucune donnée de graphique disponible pour le moment — vérification en cours…
tokens-dialog-no-data = Aucune donnée disponible
tokens-dialog-source-token = Token
tokens-dialog-source-market = Marché
tokens-dialog-source-security = Sécurité
tokens-dialog-source-chart = Graphique
tokens-dialog-status-pending = En attente
tokens-dialog-status-loading = Chargement
tokens-dialog-status-ready = Prêt
tokens-dialog-status-unavailable = Indisponible
tokens-dialog-status-cached = En cache
tokens-dialog-source-summary = Données { $source } : { $status }
tokens-dialog-badge-pool-price = Prix du pool
tokens-dialog-badge-pool-price-hint = Prix issu du pool on-chain en temps réel
tokens-dialog-badge-api-price = Prix API
tokens-dialog-badge-api-price-hint = Prix issu des données de marché en cache (API)
tokens-dialog-badge-profile = Profil publié
    .title = Contenu de profil payant examiné avant publication ; ni un audit ni une vérification de propriété.
tokens-dialog-badge-low-risk-hint = Risque faible selon le score { -rugcheck } actuel ; ne constitue pas une vérification d'identité.
tokens-dialog-badge-immutable = Immuable
tokens-dialog-badge-mutable = Modifiable
tokens-dialog-badge-position = Position
tokens-dialog-badge-blacklisted = Sur liste noire

tokens-view-favorites = Favoris
tokens-view-pool = Service de pools
tokens-view-no-market = Sans données de marché
tokens-view-all = Tous les tokens
tokens-view-passed = Validés
tokens-view-rejected = Rejetés
tokens-view-blacklisted = Sur liste noire
tokens-view-positions = Positions
tokens-view-recent = Récents
tokens-view-ohlcv = Données OHLCV
# Empty token table per view (TOKEN_VIEW_EMPTY_LABELS)
tokens-view-pool-empty = Aucun token valorisé pour l’instant
    .message = Les tokens apparaissent ici une fois le filtrage passé et le prix de leur pool calculé.
tokens-view-no-market-empty = Aucun token sans données de marché
    .message = Les tokens apparaissent ici tant que les sources de données de marché ne les ont pas encore listés.
tokens-view-all-empty = Aucun token découvert pour l’instant
    .message = Chaque token trouvé par la découverte apparaît ici, quel que soit son résultat de filtrage.
tokens-view-passed-empty = Aucun token n’a passé le filtrage
    .message = Les tokens qui passent tous les filtres actifs apparaissent ici. Consultez la page Filtrage si cette liste reste vide.
tokens-view-rejected-empty = Aucun token rejeté
    .message = Les tokens qui échouent à un filtre apparaissent ici avec la raison.
tokens-view-blacklisted-empty = Aucun token sur liste noire
    .message = Les tokens exclus du trading, par vous ou par les contrôles de sécurité, apparaissent ici.
tokens-view-positions-empty = Aucun token en position
    .message = Les tokens détenus dans des positions ouvertes apparaissent ici.
tokens-view-recent-empty = Aucun token récent
    .message = Les tokens nouvellement découverts apparaissent ici au fur et à mesure.
tokens-ohlcv-empty = Aucune donnée de graphique pour l’instant
    .message = Les tokens apparaissent ici dès que leurs bougies sont collectées.

tokens-cell-logo-enlarge = Cliquez pour agrandir
tokens-boost-title = Boosté { $boosts } fois sur screenerbot.io
tokens-cell-action-add =
    .title = Renforcer la position (DCA)
    .aria-label = Renforcer la position
tokens-cell-action-sell =
    .title = Vendre (totalité ou % partiel)
    .aria-label = Vendre le token
tokens-cell-action-buy =
    .title = Acheter une position
    .aria-label = Acheter le token
tokens-cell-external-links =
    .title = Liens externes
    .aria-label = Liens externes

tokens-table-loading-title = Chargement des tokens…
tokens-table-loading-description = Préparation de la vue de tokens sélectionnée.
tokens-table-retry-hint = Changez d'onglet ou réessayez.
tokens-filter-all = Tous

tokens-favorites-load-failed-title = Impossible de charger les favoris
tokens-favorites-load-failed-toast = Impossible de charger les favoris
tokens-favorites-total = Total des favoris
tokens-favorites-empty-title = Aucun favori pour le moment
    .message = Ajoutez une étoile à un token dans n’importe quelle liste pour le garder ici.

tokens-column-token = Token
tokens-column-status = Statut
tokens-ohlcv-delete =
    .title = Supprimer les données OHLCV
    .aria-label = Supprimer les données OHLCV
tokens-ohlcv-status-active = Actif
tokens-ohlcv-status-inactive = Inactif
tokens-ohlcv-priority-critical = Critique
tokens-ohlcv-priority-high = Haute
tokens-ohlcv-priority-medium = Moyenne
tokens-ohlcv-priority-low = Basse
tokens-ohlcv-column-priority = Priorité
tokens-ohlcv-column-backfill = Rattrapage
tokens-ohlcv-column-data-span = Étendue des données
tokens-ohlcv-column-gaps = Lacunes
tokens-ohlcv-column-pools = Pools
tokens-ohlcv-column-last-fetch = Dernière récupération
tokens-ohlcv-timeframe-complete = { $timeframe } : terminé
tokens-ohlcv-timeframe-pending = { $timeframe } : en attente
tokens-ohlcv-load-failed-title = Impossible de charger les données OHLCV
tokens-ohlcv-load-failed-toast = Impossible de charger les données OHLCV
tokens-ohlcv-total = Total des tokens
tokens-ohlcv-active = Actifs
tokens-ohlcv-db-size = Taille de la base
tokens-ohlcv-cleanup = Nettoyer les inactifs
tokens-ohlcv-delete-title = Supprimer les données OHLCV
tokens-ohlcv-delete-token-message = Supprimer toutes les données OHLCV de { $token } ?
tokens-ohlcv-delete-done =
    Supprimé : { $candles ->
        [one] { $candles } bougie
        [many] { $candles } bougies
       *[other] { $candles } bougies
    }, { $pools ->
        [one] { $pools } pool
        [many] { $pools } pools
       *[other] { $pools } pools
    }
tokens-ohlcv-delete-failed = Échec de la suppression des données OHLCV
tokens-ohlcv-cleanup-title = Supprimer les tokens inactifs
tokens-ohlcv-cleanup-message = Supprimer les tokens inactifs plus anciens que le nombre d'heures indiqué
tokens-ohlcv-cleanup-placeholder = Heures...
tokens-ohlcv-cleanup-invalid = Veuillez saisir un nombre positif
tokens-ohlcv-cleanup-done =
    { $count ->
        [one] { $count } token inactif nettoyé
        [many] { $count } tokens inactifs nettoyés
       *[other] { $count } tokens inactifs nettoyés
    }
tokens-ohlcv-cleanup-failed = Échec du nettoyage des données OHLCV

tokens-summary-total = Total
tokens-summary-pool-priced = Avec prix de pool
tokens-summary-positions = Positions
tokens-summary-blacklisted = Sur liste noire
tokens-search-placeholder = Rechercher par symbole ou mint...
tokens-table-waiting-title = Chargement des tokens toujours en cours...
tokens-table-waiting-description = En attente de la réponse du backend. Nouvelle tentative automatique.
tokens-load-failed-toast = Impossible de charger les tokens
tokens-row-data-missing = Données du token introuvables
tokens-column-price-sol = Prix ({ -sol })
tokens-column-liquidity = Liquidité
tokens-column-volume-24h = Vol. 24 h
tokens-column-fdv = FDV
tokens-column-market-cap = Cap.
tokens-column-change-1h = 1 h
tokens-column-change-24h = 24 h
tokens-column-txns-5m = Txs 5min
tokens-column-txns-1h = Txs 1 h
tokens-column-txns-6h = Txs 6 h
tokens-column-txns-24h = Txs 24 h
tokens-column-risk-score = Score de risque
tokens-column-reject-reason = Motif de rejet
tokens-column-blacklist-reason = Motif de liste noire
tokens-column-updated = Mis à jour
tokens-column-birth = Création
tokens-column-first-seen = Première détection
tokens-badge-price = Prix
tokens-badge-ohlcv = OHLCV
tokens-badge-position = Position
tokens-badge-blacklisted = Sur liste noire
tokens-badge-blacklisted-title = Token sur liste noire
tokens-badge-blacklisted-reasons = Sur liste noire : { $reasons }
tokens-links-menu-copy-mint = Copier le mint
tokens-links-copy-failed = Échec de la copie du mint
tokens-lightbox-token-age = Âge du token

tokens-search-placeholder-dialog = Rechercher un nom, un symbole ou un mint...
tokens-search-input-label = Rechercher des tokens
tokens-search-results-label = Résultats de recherche
tokens-search-tip-nav = naviguer
tokens-search-tip-open = ouvrir
tokens-search-tip-close = fermer
tokens-search-failed = Échec de la recherche
tokens-search-error = Erreur : { $message }
tokens-search-clear =
    .title = Effacer la recherche
    .aria-label = Effacer la recherche
tokens-search-recent = Récents
tokens-search-recent-label = Recherches récentes
tokens-search-lists-label = Listes de tokens
tokens-search-tab-trending = Tendance
tokens-search-kinds = Nom · symbole · mint
tokens-search-empty-trending = Les tokens tendance apparaissent dès que le bot a coté ses premiers pools.
tokens-search-empty-positions = Aucune position ouverte pour le moment.
tokens-search-empty-favorites = Ajoutez une étoile à un token et il vous attendra ici à la prochaine recherche.
tokens-search-empty-boosted = Aucun token n’est boosté pour le moment.
tokens-search-list-failed = Impossible de charger cette liste.
tokens-search-searching = Recherche sur les marchés…
# $count is the number of tokens found.
tokens-search-result-count =
    { $count ->
        [one] { $count } résultat
        [many] { $count } résultats
       *[other] { $count } résultats
    }
tokens-search-order = Meilleure correspondance d’abord, puis volume 24h
tokens-search-metric-mc = MC
    .title = Capitalisation
tokens-search-metric-fdv = FDV
    .title = Valorisation entièrement diluée
tokens-search-metric-liq = Liq
    .title = Liquidité
tokens-search-metric-vol = Vol
    .title = Volume 24h
tokens-search-more =
    .title = Plus d’actions
    .aria-label = Plus d’actions
# $query is the text the user typed.
tokens-search-no-match = Aucun token ne correspond à « { $query } ».

tokens-featured-category-boosted = Boostés
tokens-featured-category-jupiter-organic = { -jupiter } Top organique
tokens-featured-category-jupiter-traded = { -jupiter } Top échangés
tokens-featured-category-dexscreener-trending = { -dexscreener } Tendances
tokens-featured-source-jupiter = { -jupiter }
tokens-featured-source-dexscreener = { -dexscreener }
tokens-featured-note-boosted = Promus par leurs équipes
tokens-featured-security-risky = Risqué
tokens-featured-load-failed = Échec du chargement des tokens à la une
tokens-featured-network-error = Erreur réseau : { $message }
tokens-featured-title = À la une
tokens-featured-subtitle = Les tokens boostés d'abord, puis les tendances sur Solana
tokens-featured-boost = Booster un token
tokens-featured-close =
    .title = Fermer (ÉCHAP)
tokens-featured-loading = Chargement des tokens à la une et tendance...
tokens-featured-error-hint = Vérifiez la connexion ou réessayez
tokens-featured-empty = Aucun token disponible pour le moment
tokens-featured-count =
    { $count ->
        [one] { $count } token
        [many] { $count } tokens
       *[other] { $count } tokens
    }
tokens-featured-stat-market-cap = Capitalisation
tokens-featured-stat-liquidity = Liquidité
tokens-featured-stat-volume = Vol. 24H
tokens-featured-stat-holders = Holders
tokens-featured-stat-txns = Txs 24H
tokens-featured-buy = Acheter
    .title = Acheter { $symbol }
tokens-featured-security-score = Score de sécurité : { $score }/100
tokens-featured-social-website = Site web
tokens-featured-social-twitter = { -twitter }

tokens-featured-row-view-all = Tous
    .title = Ouvrir la vue À la une complète
tokens-featured-row-scroll-start =
    .aria-label = Afficher les tokens précédents
tokens-featured-row-scroll-end =
    .aria-label = Afficher plus de tokens
tokens-featured-row-empty = Aucun token à la une
tokens-featured-row-title = { $name } ({ $symbol })
tokens-featured-row-boosted-title = { $name } ({ $symbol }) — boosté { $boosts }

tokens-pool-selector-title = Sélectionner un pool
tokens-pool-selector-loading = Chargement des pools...
tokens-pool-selector-empty = Aucun pool trouvé pour ce token
tokens-pool-selector-load-failed = Échec du chargement des pools : { $message }
tokens-pool-selector-count =
    { $count ->
        [one] { $count } pool trouvé
        [many] { $count } pools trouvés
       *[other] { $count } pools trouvés
    }
tokens-pool-selector-liquidity = { $amount } de liq.
    .title = Liquidité
tokens-pool-selector-volume = { $amount } sur 24 h
    .title = Volume 24 h

tokens-identity-unknown-asset = Actif inconnu
tokens-identity-copy-address =
    .title = Copier l'adresse
    .aria-label = Copier l'adresse
tokens-identity-copy-signature =
    .title = Copier la signature
    .aria-label = Copier la signature

tokens-rugcheck-risk-single-holder-ownership = Un holder dominant
    .description = Un seul holder détient une grande partie de l'offre du token.
tokens-rugcheck-risk-low-liquidity = Liquidité faible
    .description = Le pool du token contient peu de liquidité.
tokens-rugcheck-risk-few-lp-providers = Peu de fournisseurs de LP
    .description = Seuls quelques utilisateurs fournissent de la liquidité.
tokens-rugcheck-risk-high-holder-concentration = Forte concentration des holders
    .description = Les 10 premiers holders détiennent plus de 50 % de l'offre du token.
tokens-rugcheck-risk-top-10-holders-high-ownership = Forte détention du top 10
    .description = Les 10 premiers holders détiennent plus de 70 % de l'offre du token.
tokens-rugcheck-risk-high-ownership = Détention élevée
    .description = Les principaux holders détiennent plus de 80 % de l'offre du token.
tokens-rugcheck-risk-creator-rug-history = Historique de rug pull du créateur
    .description = Le créateur a déjà fait des rug pulls sur des tokens.
tokens-rugcheck-risk-large-lp-unlocked = Grande part de LP déverrouillée
    .description = Une grande partie des tokens LP est déverrouillée, ce qui permet au propriétaire de retirer la liquidité à tout moment.
tokens-rugcheck-risk-mutable-metadata = Métadonnées modifiables
    .description = Le propriétaire peut modifier les métadonnées du token.
tokens-rugcheck-risk-few-holders = Peu de holders
    .description = Peu de portefeuilles détiennent le token.
tokens-rugcheck-risk-copycat-token = Token imitateur
    .description = Ce token utilise le symbole d'un token vérifié.
tokens-rugcheck-risk-fee-config-enabled = Frais configurables
    .description = Le propriétaire peut modifier les frais à tout moment.
tokens-rugcheck-risk-high-holder-correlation = Forte corrélation des holders
    .description = Les principaux holders détiennent des quantités similaires de l'offre.
tokens-rugcheck-risk-freeze-authority-enabled = Autorité de gel active
    .description = Les tokens peuvent être gelés et exclus des échanges.
tokens-rugcheck-risk-mint-authority-enabled = Autorité de mint active
    .description = Le propriétaire peut émettre davantage de tokens.
tokens-rugcheck-risk-missing-file-metadata = Fichier de métadonnées manquant
    .description = Aucun fichier de métadonnées n'est associé à ce token.
tokens-rugcheck-risk-high-market-cap-per-holder = Capitalisation élevée par holder
    .description = La capitalisation est très élevée par rapport au nombre de holders.
tokens-rugcheck-risk-symbol-mismatch = Symbole incohérent
    .description = Le symbole du token ne correspond pas à son fichier de métadonnées.
tokens-rugcheck-risk-name-mismatch = Nom incohérent
    .description = Le nom du token ne correspond pas à son fichier de métadonnées.
tokens-rugcheck-risk-permanent-control-enabled = Contrôle permanent activé
    .description = Le créateur du token peut contrôler tous les tokens de façon permanente.
tokens-rugcheck-risk-missing-metadata = Métadonnées manquantes
    .description = Aucune métadonnée n'a été trouvée pour ce token.
tokens-rugcheck-risk-lp-unlock-soon = Déverrouillage de LP imminent
    .description = Les tokens LP seront bientôt déverrouillés, ce qui permettra au propriétaire de retirer la liquidité.
tokens-rugcheck-risk-lp-vault-unlocked = Coffre LP déverrouillé
    .description = Les tokens LP du coffre peuvent être récupérés.
tokens-rugcheck-risk-mint-authority-locked = Autorité de mint verrouillée
    .description = L'émission de nouveaux tokens est verrouillée.
tokens-rugcheck-risk-high-transfer-fee = Frais de transfert élevés
    .description = Chaque transfert de ce token est soumis à une taxe élevée.
