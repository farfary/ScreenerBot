## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = Outils
tools-category-wallet = Portefeuille
tools-category-token = Token
tools-category-single-token = Token unique
tools-category-utilities = Utilitaires
tools-sidebar-hint = Sélectionnez un outil pour commencer
tools-help-button =
    .aria-label = Afficher l'aide de cet outil
tools-help-unavailable = Aide indisponible
tools-placeholder-title = Sélectionnez un outil
tools-placeholder-subtitle = Choisissez un outil dans la barre latérale pour commencer
tools-placeholder-hint-wallets = Les outils de portefeuille aident à gérer vos portefeuilles Solana
tools-placeholder-hint-secure = Toutes les opérations sont sécurisées et réversibles lorsque c'est possible

tools-status-ready = Prêt à l'emploi
tools-status-coming = Bientôt disponible
tools-status-beta = Bêta - des bugs sont possibles
tools-status-disabled = Actuellement désactivé
tools-status-badge-coming = Bientôt
tools-status-badge-beta = Bêta
tools-toast-coming-soon = Cet outil sera bientôt disponible
tools-toast-disabled = Cet outil est actuellement désactivé
tools-setup-gate-title = Cet outil nécessite un portefeuille

## Tool names. `-title` names the tool in the navigation and the header, `-summary` is the
## navigation line, `-description` is the header line. Ids are the tool ids of the registry.

tools-tool-wallet-cleanup-title = Nettoyage du portefeuille
tools-tool-wallet-cleanup-summary = Fermer les ATA vides
tools-tool-wallet-cleanup-description = Fermez les comptes de token associés (ATA) vides pour récupérer du { -sol }
tools-tool-burn-tokens-title = Burn de tokens
tools-tool-burn-tokens-summary = Détruire des tokens définitivement
tools-tool-burn-tokens-description = Détruisez définitivement des tokens de votre portefeuille
tools-tool-token-analyzer-title = Analyseur de token
tools-tool-token-analyzer-summary = Analyse approfondie d'un token
tools-tool-token-analyzer-description = Analyse approfondie de n'importe quel token Solana avec des indicateurs multidimensionnels
tools-tool-create-token-title = Créer un token
tools-tool-create-token-summary = Déployer un nouveau token SPL
tools-tool-create-token-description = Déployez un nouveau token SPL sur Solana
tools-tool-trade-watcher-title = Surveillance de trades
tools-tool-trade-watcher-summary = Surveiller les trades et agir automatiquement
tools-tool-trade-watcher-description = Surveillez les trades d'un token et déclenchez des achats/ventes automatiques
tools-tool-token-watch-title = Surveillance des holders
tools-tool-token-watch-summary = Suivre les nouveaux holders
tools-tool-token-watch-description = Suivez et surveillez les nouveaux holders d'un token en temps réel
tools-tool-buy-multi-wallets-title = Multi-achat
tools-tool-buy-multi-wallets-summary = Coordonner des achats entre portefeuilles
tools-tool-buy-multi-wallets-description = Exécutez des ordres d'achat coordonnés sur plusieurs portefeuilles avec des montants aléatoires
tools-tool-sell-multi-wallets-title = Multi-vente
tools-tool-sell-multi-wallets-summary = Coordonner des ventes entre portefeuilles
tools-tool-sell-multi-wallets-description = Exécutez des ordres de vente coordonnés sur plusieurs portefeuilles avec consolidation du { -sol }
tools-tool-wallet-consolidation-title = Consolidation des portefeuilles
tools-tool-wallet-consolidation-nav-title = Consolidation
tools-tool-wallet-consolidation-summary = Consolider les fonds des portefeuilles
tools-tool-wallet-consolidation-description = Consolidez le { -sol } et les tokens des sous-portefeuilles vers le portefeuille principal
tools-tool-airdrop-checker-title = Vérificateur d'airdrops
tools-tool-airdrop-checker-summary = Vérifier les airdrops en attente
tools-tool-airdrop-checker-description = Recherchez les airdrops en attente et les récompenses à réclamer
tools-tool-wallet-generator-title = Générateur de portefeuilles
tools-tool-wallet-generator-summary = Générer de nouvelles paires de clés
tools-tool-wallet-generator-description = Générez de nouvelles paires de clés Solana en toute sécurité

## Shared by the tools

tools-validation-mint-required = Veuillez saisir une adresse de mint de token
tools-validation-mint-format = Format d'adresse de mint de token invalide
tools-validation-mint-invalid = Veuillez saisir une adresse de mint valide

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = Détails du token
tools-create-token-name-label = Nom du token
tools-create-token-name-input =
    .placeholder = Mon token
tools-create-token-symbol-label = Symbole
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = Décimales
tools-create-token-supply-label = Offre initiale
tools-create-token-description-label = Description
tools-create-token-description-input =
    .placeholder = Description du token...
tools-create-token-image-title = Image du token
tools-create-token-image-drop = Déposez une image ici ou cliquez pour l'importer
tools-create-token-image-hint = Recommandé : PNG 512x512
tools-create-token-action-preview = Aperçu
tools-create-token-action-create = Créer le token

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = Chargement des paramètres...
tools-holder-watch-saved = Paramètres de surveillance des holders enregistrés
tools-holder-watch-save-failed = Échec de l'enregistrement des paramètres
tools-holder-watch-save-error = Erreur lors de l'enregistrement des paramètres
tools-holder-watch-settings-title = Paramètres de surveillance des holders
tools-holder-watch-enabled-label = Activer la surveillance des holders
tools-holder-watch-interval-label = Intervalle de vérification
tools-holder-watch-interval-hint = Fréquence de vérification du nombre de holders (10-3600 s)
tools-holder-watch-max-tokens-label = Nombre max. de tokens surveillés
tools-holder-watch-max-tokens-hint = Nombre maximal de tokens surveillés simultanément
tools-holder-watch-notify-new-label = Notifier les nouveaux holders
tools-holder-watch-notify-drop-label = Notifier la baisse de holders
tools-holder-watch-min-change-label = Variation min. de holders
tools-holder-watch-min-change-hint = Variation minimale de holders pour déclencher une notification
tools-holder-watch-drop-percent-label = Seuil de baisse de holders
tools-holder-watch-drop-percent-hint = Pourcentage de baisse déclenchant une alerte
tools-holder-watch-action-save = Enregistrer les paramètres
tools-holder-watch-tokens-title = Tokens surveillés
tools-holder-watch-token-input =
    .placeholder = Saisissez l'adresse de mint du token...
tools-holder-watch-empty = Aucun token surveillé
tools-holder-watch-empty-hint = Ajoutez une adresse de mint ci-dessus pour lancer la surveillance
tools-holder-watch-coming-soon = La surveillance de tokens sera bientôt disponible

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = Analyser un token
tools-analyzer-mint-input =
    .placeholder = Collez l'adresse de mint du token...
tools-analyzer-action-analyze = Analyser
tools-analyzer-action-analyzing = Analyse en cours...
tools-analyzer-action-copy-report = Copier le rapport
tools-analyzer-loading = Analyse du token en cours...
tools-analyzer-failed = Échec de l'analyse du token
tools-analyzer-empty = Saisissez une adresse de mint de token à analyser
tools-analyzer-empty-hint = Obtenez des informations complètes sur n'importe quel token Solana
tools-analyzer-tab-overview = Aperçu
tools-analyzer-tab-security = Sécurité
tools-analyzer-tab-market = Marché
tools-analyzer-tab-liquidity = Liquidité
tools-analyzer-unknown-token = Token inconnu

tools-analyzer-favorite-add =
    .title = Ajouter aux favoris
    .aria-label = Ajouter aux favoris
tools-analyzer-favorite-already = Déjà dans les favoris
tools-analyzer-favorite-added = { $symbol } ajouté aux favoris
tools-analyzer-favorite-failed = Échec de l'ajout aux favoris
tools-analyzer-blacklist-add =
    .title = Ajouter à la liste noire
    .aria-label = Ajouter à la liste noire
tools-analyzer-blacklist-title = Ajouter le token à la liste noire
tools-analyzer-blacklist-message = Ajouter { $symbol } à la liste noire ? Ce token sera exclu du trading.
tools-analyzer-blacklist-confirm = Ajouter à la liste noire
tools-analyzer-blacklisted = Sur liste noire
tools-analyzer-blacklist-done = { $symbol } ajouté à la liste noire
tools-analyzer-blacklist-failed = Échec de l'ajout du token à la liste noire

tools-analyzer-card-quick-stats = Stats rapides
tools-analyzer-card-market-summary = Résumé du marché
tools-analyzer-card-token-info = Informations sur le token
tools-analyzer-stat-holders = Holders
tools-analyzer-stat-decimals = Décimales
tools-analyzer-stat-safety-score = Score de sécurité
tools-analyzer-stat-pools = Pools
tools-analyzer-stat-volume-24h = Volume 24 h
tools-analyzer-stat-change-24h = Variation 24 h
tools-analyzer-stat-market-cap = Capitalisation
tools-analyzer-stat-liquidity = Liquidité
tools-analyzer-info-mint = Adresse de mint
tools-analyzer-info-description = Description
tools-analyzer-info-supply = Offre

tools-analyzer-security-empty = Aucune donnée de sécurité disponible
tools-analyzer-security-empty-hint = L'analyse de sécurité n'est pas disponible pour ce token
tools-analyzer-card-safety-score = Score de sécurité
tools-analyzer-score-good = Bon
tools-analyzer-score-moderate = Modéré
tools-analyzer-score-risky = Risqué
tools-analyzer-raw-score = Score de risque brut : { $score }
tools-analyzer-card-authorities = Autorités du token
tools-analyzer-authority-mint = Autorité de mint
tools-analyzer-authority-freeze = Autorité de gel
tools-analyzer-authority-transfer-fee = Frais de transfert
tools-analyzer-authority-mutable = Modifiable
tools-analyzer-authority-active = Active
tools-analyzer-authority-revoked = Révoquée
tools-analyzer-card-holder-concentration = Concentration des holders
tools-analyzer-top-holders = détenus par les 10 premiers holders
tools-analyzer-risks-title = Risques de sécurité ({ $count })
tools-analyzer-risks-title-none = Risques de sécurité
tools-analyzer-risks-none = Aucun risque de sécurité détecté

tools-analyzer-market-empty = Aucune donnée de marché disponible
tools-analyzer-market-empty-hint = Les données de marché ne sont pas disponibles pour ce token
tools-analyzer-card-price = Prix actuel
tools-analyzer-card-price-changes = Variations de prix
tools-analyzer-card-volume = Volume d'échanges
tools-analyzer-card-transactions = Transactions 24 h
tools-analyzer-card-valuation = Valorisation
tools-analyzer-stat-window-1h = 1 h
tools-analyzer-stat-window-6h = 6 h
tools-analyzer-stat-window-24h = 24 h
tools-analyzer-stat-volume-1h = Volume 1 h
tools-analyzer-stat-volume-6h = Volume 6 h
tools-analyzer-stat-fdv = Valorisation totalement diluée
tools-analyzer-txn-buys = Achats
tools-analyzer-txn-sells = Ventes

tools-analyzer-liquidity-empty = Aucune donnée de liquidité disponible
tools-analyzer-liquidity-empty-hint = Aucun pool trouvé pour ce token
tools-analyzer-card-total-liquidity = Liquidité totale
tools-analyzer-card-pools = Pools
tools-analyzer-active-pools =
    { $count ->
        [one] Pool actif
        [many] Pools actifs
       *[other] Pools actifs
    }
tools-analyzer-card-pool-details = Détails du pool
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = Liquidité ({ -sol })
tools-analyzer-pools-column-status = Statut
tools-analyzer-pool-primary = Principal

tools-analyzer-report-empty = Aucune analyse à copier
tools-analyzer-report-label = Rapport d'analyse
tools-analyzer-report-title = Rapport d'analyse de token
tools-analyzer-report-token = Token : { $symbol } ({ $name })
tools-analyzer-report-mint = Mint : { $mint }
tools-analyzer-report-price = Prix : { $sol }
tools-analyzer-report-price-with-usd = Prix : { $sol } ({ $usd })
tools-analyzer-report-security = Sécurité :
tools-analyzer-report-safety-score = - Score de sécurité : { $score }/100
tools-analyzer-report-mint-authority = - Autorité de mint : { $state }
tools-analyzer-report-freeze-authority = - Autorité de gel : { $state }
tools-analyzer-report-risks = - Risques : { $count }
tools-analyzer-report-market = Marché :
tools-analyzer-report-volume = - Volume 24 h : { $amount }
tools-analyzer-report-change = - Variation 24 h : { $amount }
tools-analyzer-report-market-cap = - Capitalisation : { $amount }
tools-analyzer-report-liquidity = Liquidité :
tools-analyzer-report-liquidity-total = - Total : { $amount }
tools-analyzer-report-pools = - Pools : { $count }
tools-analyzer-report-generated = Généré : { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

tools-watch-type-buy-on-sell = Achat sur vente
tools-watch-type-sell-on-buy = Vente sur achat
tools-watch-type-notify = Notifier
tools-watch-type-notify-only = Notification seule

tools-trade-watcher-setup-title = Configurer la surveillance
tools-trade-watcher-mint-label = Adresse de mint du token
tools-trade-watcher-mint-input =
    .placeholder = Saisissez l'adresse de mint du token...
tools-trade-watcher-action-search-pools = Rechercher des pools
tools-trade-watcher-pool-label = Pool sélectionné
tools-trade-watcher-pool-none = Aucun pool sélectionné
tools-trade-watcher-pool-clear =
    .title = Effacer le pool
tools-trade-watcher-pool-selected = Pool sélectionné : { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = Type de surveillance
tools-trade-watcher-type-hint = Achat sur vente : achète automatiquement quand quelqu'un vend. Vente sur achat : vend automatiquement quand quelqu'un achète.
tools-trade-watcher-trigger-label = Montant de déclenchement
tools-trade-watcher-trigger-hint = Taille minimale de trade en { -sol } pour déclencher l'action
tools-trade-watcher-action-amount-label = Montant de l'action
tools-trade-watcher-action-amount-hint = Montant à acheter/vendre lors du déclenchement
tools-trade-watcher-slippage-label = Slippage
tools-trade-watcher-slippage-hint = Slippage maximal acceptable pour les trades
tools-trade-watcher-active-title = Surveillances actives
tools-trade-watcher-empty = Aucune surveillance active
tools-trade-watcher-empty-hint = Configurez une surveillance ci-dessus puis cliquez sur « Lancer la surveillance » pour démarrer
tools-trade-watcher-action-start = Lancer la surveillance
tools-trade-watcher-action-starting = Démarrage...
tools-trade-watcher-action-stop-all = Tout arrêter
tools-trade-watcher-action-stopping = Arrêt...
tools-trade-watcher-started = Surveillance lancée pour { $token }...
tools-trade-watcher-start-failed = Échec du lancement de la surveillance
tools-trade-watcher-stopped = Surveillance arrêtée
tools-trade-watcher-stop-failed = Échec de l'arrêt de la surveillance
tools-trade-watcher-stopped-all = Toutes les surveillances arrêtées
tools-trade-watcher-stop-all-failed = Échec de l'arrêt des surveillances
tools-trade-watcher-load-failed = Échec du chargement des surveillances
tools-trade-watcher-column-token = Token
tools-trade-watcher-column-type = Type
tools-trade-watcher-column-trigger = Déclencheur
tools-trade-watcher-column-action = Action
tools-trade-watcher-column-triggered = Déclenchements
tools-trade-watcher-stop-watch =
    .title = Arrêter la surveillance

## Results returned by the tools backend. Failures are catalog text; the technical cause
## travels separately as details and is appended by the dashboard.

tools-burn-failure-native-asset = Impossible de brûler du { -sol }
tools-burn-failure-open-position = Impossible de brûler des tokens de positions ouvertes
tools-burn-failure-account-not-found = Compte de token introuvable
tools-burn-failure-zero-balance = Le solde du token est déjà nul
tools-burn-failure-transaction = Échec de la transaction
tools-burn-warning-open-position = Impossible de brûler des tokens de positions ouvertes
tools-burn-warning-closed-position = Reliquat d'une position clôturée
tools-burn-warning-worth = Vaut ~{ $amount } { -sol }
tools-multi-buy-warning-insufficient = Solde insuffisant. Requis : { $needed } { -sol }, disponible : { $have } { -sol }
tools-multi-buy-warning-over-limit = Le total de { -sol } requis ({ $needed }) dépasse la limite ({ $limit })
tools-multi-sell-warning-no-wallets = Aucun portefeuille secondaire trouvé
tools-multi-sell-warning-no-balance = Aucun portefeuille ne détient ce token
tools-multi-op-buy-failed = Échec de l'achat
tools-multi-op-sell-failed = Échec de la vente
tools-multi-op-transfer-failed = Échec du transfert
tools-multi-op-balance-failed = Échec de la récupération du solde
tools-multi-op-mint-invalid = Adresse de mint invalide
tools-multi-buy-session-failed = Échec du multi-achat
tools-multi-sell-session-failed = Échec de la multi-vente
tools-multi-session-aborted = Opération interrompue par l'utilisateur

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = Analyser le portefeuille
tools-wallet-action-scanning = Analyse en cours...
tools-wallet-scan-failed = Échec de l'analyse : { $reason }
tools-wallet-amount-approx = ~{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
        [one] Sélectionné : { $count } portefeuille
        [many] Sélectionnés : { $count } portefeuilles
       *[other] Sélectionnés : { $count } portefeuilles
    }
tools-wallet-transfer-failed = Échec du transfert : { $reason }
tools-wallet-cleanup-failed = Échec du nettoyage : { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = Résultats de l'analyse
tools-wallet-cleanup-stat-empty = ATA vides
tools-wallet-cleanup-stat-reclaimable = { -sol } récupérable
tools-wallet-cleanup-stat-failed = Échecs (en cache)
tools-wallet-cleanup-prompt = Cliquez sur « Analyser le portefeuille » pour trouver les ATA vides
tools-wallet-cleanup-prompt-hint = Tous les comptes de token de votre portefeuille seront vérifiés
tools-wallet-cleanup-action-cleanup = Tout nettoyer
tools-wallet-cleanup-action-cleaning = Nettoyage...
tools-wallet-cleanup-scanning = Analyse du portefeuille...
tools-wallet-cleanup-found =
    { $count ->
        [one] { $count } ATA vide trouvé, d'une valeur d'environ { $amount }
        [many] { $count } ATA vides trouvés, d'une valeur d'environ { $amount }
       *[other] { $count } ATA vides trouvés, d'une valeur d'environ { $amount }
    }
tools-wallet-cleanup-clean = Aucun ATA vide trouvé - le portefeuille est propre !
tools-wallet-cleanup-scan-failed = Échec de l'analyse des ATA
tools-wallet-cleanup-done =
    { $count ->
        [one] { $count } ATA nettoyé
        [many] { $count } ATA nettoyés
       *[other] { $count } ATA nettoyés
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = Burn de tokens
tools-burn-info-title = Qu'est-ce que le burn ?
tools-burn-info-body = Le burn détruit définitivement des tokens, qui deviennent irrécupérables. Après un burn, lancez le nettoyage du portefeuille pour fermer les ATA vides et récupérer environ 0.002 { -sol } de rent par token.
tools-burn-stat-total = Total des tokens
tools-burn-stat-selected = Sélectionnés
tools-burn-stat-rent = Rent récupérable
tools-burn-prompt = Cliquez sur « Analyser le portefeuille » pour trouver des tokens
tools-burn-scanning = Recherche de tokens dans le portefeuille...
tools-burn-scan-failed = Échec de l'analyse des tokens
tools-burn-empty = Aucun token trouvé dans le portefeuille
tools-burn-action-burn = Brûler la sélection ({ $count })
tools-burn-action-burning = Burn en cours...
tools-burn-cannot-burn = Burn impossible
tools-burn-no-value = Sans valeur

tools-burn-category-open-position = Positions ouvertes
tools-burn-category-has-value = Avec valeur
tools-burn-category-closed-position = Positions clôturées
tools-burn-category-zero-liquidity = Liquidité nulle
tools-burn-category-hint-open-position = Impossible de brûler des tokens de positions ouvertes
tools-burn-category-hint-has-value = Envisagez de vendre plutôt que de brûler
tools-burn-category-hint-closed-position = Reliquats de trades clôturés
tools-burn-category-hint-zero-liquidity = Burn sans risque - aucune valeur de marché

tools-burn-confirm-title = Confirmer le burn
tools-burn-confirm-message =
    { $count ->
        [one] Voulez-vous vraiment brûler <strong>{ $count }</strong> token ?
        [many] Voulez-vous vraiment brûler <strong>{ $count }</strong> tokens ?
       *[other] Voulez-vous vraiment brûler <strong>{ $count }</strong> tokens ?
    }
tools-burn-confirm-value = Valeur estimée totale : <strong>{ $amount }</strong>
tools-burn-confirm-continue = Continuer
tools-burn-final-title = Dernier avertissement
tools-burn-final-headline = Cette action est IRRÉVERSIBLE !
tools-burn-final-message =
    { $count ->
        [one] Le token suivant ({ $count }) sera définitivement détruit et ne pourra être récupéré en aucun cas.
        [many] Les { $count } tokens suivants seront définitivement détruits et ne pourront être récupérés en aucun cas.
       *[other] Les { $count } tokens suivants seront définitivement détruits et ne pourront être récupérés en aucun cas.
    }
tools-burn-final-confirm = Oui, brûler les tokens
tools-burn-toast-burned =
    { $total ->
        [one] { $successful }/{ $total } token brûlé. Lancez le nettoyage du portefeuille pour récupérer ~{ $amount }
        [many] { $successful }/{ $total } tokens brûlés. Lancez le nettoyage du portefeuille pour récupérer ~{ $amount }
       *[other] { $successful }/{ $total } tokens brûlés. Lancez le nettoyage du portefeuille pour récupérer ~{ $amount }
    }
tools-burn-toast-failed =
    { $count ->
        [one] Échec du burn pour { $count } token
        [many] Échec du burn pour { $count } tokens
       *[other] Échec du burn pour { $count } tokens
    }
tools-burn-failed = Échec du burn : { $reason }
tools-burn-failures-title =
    { $count ->
        [one] { $count } token n'a pas pu être brûlé
        [many] { $count } tokens n'ont pas pu être brûlés
       *[other] { $count } tokens n'ont pas pu être brûlés
    }
tools-burn-failure-unknown = Aucune raison n'a été signalée

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = À propos
tools-airdrop-about-body = Recherchez les airdrops en attente, les récompenses à réclamer et les allocations non réclamées sur les principaux protocoles Solana.
tools-airdrop-list-title = Airdrops disponibles
tools-airdrop-prompt = Cliquez sur « Vérifier les airdrops » pour rechercher les réclamations disponibles
tools-airdrop-action-check = Vérifier les airdrops
tools-airdrop-action-claim-all = Tout réclamer

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = Options du générateur
tools-generator-warning-title = Conservez vos clés privées en lieu sûr !
tools-generator-warning-body = Les paires de clés générées sont créées localement et jamais transmises. Sauvegardez toujours vos clés dans un endroit sécurisé.
tools-generator-count-label = Nombre de portefeuilles
tools-generator-vanity-label = Adresse personnalisée (commence par certains caractères)
tools-generator-prefix-label = Préfixe
tools-generator-prefix-input =
    .placeholder = p. ex. SOL
tools-generator-prefix-hint = Les préfixes plus longs prennent exponentiellement plus de temps à générer
tools-generator-list-title = Portefeuilles générés
tools-generator-empty = Aucun portefeuille généré pour le moment
tools-generator-action-generate = Générer
tools-generator-action-generating = Génération...
tools-generator-count-invalid = Veuillez saisir un nombre entre 1 et 10
tools-generator-no-keypairs = Aucune paire de clés retournée
tools-generator-generated =
    { $count ->
        [one] { $count } portefeuille généré
        [many] { $count } portefeuilles générés
       *[other] { $count } portefeuilles générés
    }
tools-generator-failed = Échec de la génération des portefeuilles : { $reason }
tools-generator-copy-public-key =
    .title = Copier la clé publique
tools-generator-copy-private-key =
    .title = Copier la clé privée
tools-generator-remove =
    .title = Retirer de la liste
tools-generator-reveal =
    .title = Révéler la clé privée
tools-generator-public-key-label = Clé publique :
tools-generator-private-key-label = Clé privée :
tools-generator-public-key-name = Clé publique
tools-generator-private-key-copied = Clé privée copiée
tools-generator-private-key-warning = Toute personne possédant cette clé contrôle le portefeuille
tools-generator-export-empty = Aucun portefeuille à exporter
tools-generator-exported = Portefeuilles exportés - conservez-les en lieu sûr

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = Résumé
tools-consolidation-stat-wallets = Sous-portefeuilles
tools-consolidation-stat-native = { -sol } total
tools-consolidation-stat-tokens = Types de tokens
tools-consolidation-stat-rent = Rent récupérable
tools-consolidation-wallets-title = Portefeuilles
tools-consolidation-loading-wallets = Chargement des portefeuilles...
tools-consolidation-loading-data = Chargement des données du portefeuille...
tools-consolidation-action-transfer-native = Transférer le { -sol }
tools-consolidation-action-transfer-tokens = Transférer tous les tokens
tools-consolidation-action-cleanup = Nettoyer les ATA
tools-consolidation-action-transferring = Transfert en cours...
tools-consolidation-column-name = Nom
tools-consolidation-column-native = Solde en { -sol }
tools-consolidation-column-tokens = Tokens
tools-consolidation-column-atas = ATA vides
tools-consolidation-empty = Aucun sous-portefeuille trouvé
tools-consolidation-empty-hint = Créez des sous-portefeuilles avec le multi-achat pour commencer
tools-consolidation-load-failed = Échec du chargement : { $reason }
tools-consolidation-select-prompt = Sélectionnez les portefeuilles à consolider
tools-consolidation-selection-totals =
    | { $amount } | { $tokens ->
        [one] { $tokens } token
        [many] { $tokens } tokens
       *[other] { $tokens } tokens
    } | { $atas ->
        [one] { $atas } ATA vide
        [many] { $atas } ATA vides
       *[other] { $atas } ATA vides
    }
tools-consolidation-transferred-native = { $amount } transféré vers le portefeuille principal
tools-consolidation-transferred-tokens =
    { $count ->
        [one] { $count } token transféré vers le portefeuille principal
        [many] { $count } tokens transférés vers le portefeuille principal
       *[other] { $count } tokens transférés vers le portefeuille principal
    }
tools-consolidation-cleaned =
    { $count ->
        [one] { $count } ATA fermé, { $amount } récupéré
        [many] { $count } ATA fermés, { $amount } récupéré
       *[other] { $count } ATA fermés, { $amount } récupéré
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = Token
tools-multi-mint-label = Adresse de mint du token
tools-multi-mint-input =
    .placeholder = Collez l'adresse de mint du token...
tools-multi-execution-title = Paramètres d'exécution
tools-multi-delay-min-label = Délai min.
tools-unit-native = { -sol }
tools-unit-seconds = s
tools-unit-ms = ms
tools-multi-delay-max-label = Délai max.
tools-multi-concurrency-label = Concurrence
tools-multi-concurrency-sequential = { $count } (séquentiel)
tools-multi-concurrency-parallel = { $count } en parallèle
tools-multi-slippage-label = Slippage
tools-multi-router-label = Routeur
tools-multi-router-auto = Auto (meilleure route)
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = Pool direct
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = Progression
tools-multi-progress-preparing = Préparation...
tools-multi-status-line = { $label } ({ $completed }/{ $total })
tools-multi-column-wallet = Portefeuille
tools-multi-column-route = Route
tools-multi-column-status = Statut
tools-multi-op-completed = Terminé
tools-multi-op-failed = Échec
tools-multi-action-stop = Stop
tools-multi-action-loading = Chargement...
tools-multi-start-failed = Échec du démarrage : { $reason }

tools-multi-state-pending = En attente
tools-multi-state-funding = Approvisionnement
tools-multi-state-executing = Exécution
tools-multi-state-consolidating = Consolidation
tools-multi-state-completed = Terminé
tools-multi-state-failed = Échec
tools-multi-state-aborted = Interrompu

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = Le token que vous souhaitez acheter sur plusieurs portefeuilles
tools-multi-buy-wallets-title = Paramètres des portefeuilles
tools-multi-buy-wallet-count-label = Nombre de portefeuilles
tools-multi-buy-wallet-count-option =
    { $count ->
        [one] { $count } portefeuille
        [many] { $count } portefeuilles
       *[other] { $count } portefeuilles
    }
tools-multi-buy-wallet-count-hint = Nombre de sous-portefeuilles à utiliser
tools-multi-buy-buffer-label = Réserve de { -sol } par portefeuille
tools-multi-buy-buffer-hint = Réservé aux frais (0.015 { -sol } min.)
tools-multi-buy-amounts-title = Paramètres des montants
tools-multi-buy-min-label = { -sol } min. par portefeuille
tools-multi-buy-min-hint = Montant d'achat minimal
tools-multi-buy-max-label = { -sol } max. par portefeuille
tools-multi-buy-max-hint = Montant d'achat maximal
tools-multi-buy-limit-label = Limite totale de { -sol } (facultatif)
tools-multi-buy-limit-hint = Dépense totale maximale
tools-multi-buy-preview-title = Aperçu
tools-multi-buy-preview-create = Portefeuilles à créer
tools-multi-buy-preview-amount = Montant par portefeuille
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = Total de { -sol } requis
tools-multi-buy-preview-balance = Solde principal
tools-multi-buy-action-preview = Aperçu
tools-multi-buy-action-start = Lancer le multi-achat
tools-multi-buy-executing = Exécution des achats...
tools-multi-buy-column-spent = { -sol } dépensé
tools-multi-buy-column-tokens = Tokens
tools-multi-buy-preview-failed = Échec de l'aperçu : { $reason }
tools-multi-buy-started = Multi-achat lancé
tools-multi-buy-stopped = Multi-achat arrêté
tools-multi-buy-completed = Multi-achat terminé ! { $successful }/{ $total } réussis

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = Saisissez une adresse de token pour rechercher les portefeuilles qui le détiennent
tools-multi-sell-action-scan = Analyser
tools-multi-sell-settings-title = Paramètres de vente
tools-multi-sell-percent-label = Pourcentage à vendre
tools-multi-sell-percent-hint = % de tokens à vendre par portefeuille
tools-multi-sell-min-fee-label = { -sol } min. pour les frais
tools-multi-sell-min-fee-hint = { -sol } minimum requis pour les frais de transaction
tools-multi-sell-topup-label = Recharge auto si nécessaire
tools-multi-sell-topup-hint = Transférer du { -sol } depuis le portefeuille principal si le solde du sous-portefeuille est insuffisant
tools-multi-sell-post-title = Actions après la vente
tools-multi-sell-consolidate-label = Consolider le { -sol } vers le portefeuille principal
tools-multi-sell-consolidate-hint = Transférer tout le { -sol } des sous-portefeuilles vers le portefeuille principal
tools-multi-sell-close-atas-label = Fermer les ATA du token après la vente
tools-multi-sell-close-atas-hint = Récupérer environ 0.002 { -sol } par ATA
tools-multi-sell-wallets-title = Portefeuilles détenant le token
tools-multi-sell-empty = Aucun sous-portefeuille ne détient ce token
tools-multi-sell-column-tokens = Tokens
tools-multi-sell-column-native = Solde en { -sol }
tools-multi-sell-column-topup = Recharge requise
tools-multi-sell-none-selected = Aucun portefeuille sélectionné
tools-multi-sell-select-required = Veuillez sélectionner au moins un portefeuille
tools-multi-sell-action-start = Lancer la multi-vente
tools-multi-sell-executing = Exécution des ventes...
tools-multi-sell-column-sold = Tokens vendus
tools-multi-sell-column-received = { -sol } reçu
tools-multi-sell-started = Multi-vente lancée
tools-multi-sell-stopped = Multi-vente arrêtée
tools-multi-sell-completed = Multi-vente terminée ! { $amount } reçu

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = Favoris
tools-favorites-saved = Favoris enregistrés
tools-favorites-save-current = Enregistrer l'actuel
tools-favorites-empty = Aucun favori enregistré pour le moment
tools-favorites-no-label = Sans libellé
tools-favorites-uses = { $count }x
tools-favorites-remove = Retirer
tools-favorites-loaded = Favori chargé : { $name }
tools-favorites-default-name = Config
tools-favorites-mint-required = Veuillez d'abord saisir une adresse de mint de token
tools-favorites-add-title = Ajouter un favori
tools-favorites-add-message = Saisissez un libellé pour ce favori
tools-favorites-add-placeholder = Libellé (facultatif)...
tools-favorites-saved-toast = Enregistré dans les favoris
tools-favorites-save-failed = Échec de l'enregistrement du favori
tools-favorites-remove-title = Retirer le favori
tools-favorites-remove-message = Retirer ce favori ?
tools-favorites-removed-toast = Favori retiré
tools-favorites-remove-failed = Échec du retrait du favori
