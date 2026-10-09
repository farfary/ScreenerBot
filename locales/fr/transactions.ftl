transactions-type-buy = Achat
transactions-type-sell = Vente
transactions-type-swap = Swap
transactions-type-sol-transfer = Transfert de SOL
transactions-type-token-transfer = Transfert de token
transactions-type-transfer = Transfert
transactions-type-dust = Poussière
transactions-type-spam = Spam
transactions-type-ata-create = Compte ouvert
transactions-type-ata-close = Rent récupéré
transactions-type-ata = Compte de token
transactions-type-liquidity-add = Ajout de liquidité
transactions-type-liquidity-remove = Retrait de liquidité
transactions-type-nft = NFT
transactions-type-program = Appel de programme
transactions-type-compute = Calcul
transactions-type-failed = Échec
transactions-type-unknown = Non classée

transactions-type-with-detail = { $label } ({ $detail })
transactions-type-token-transfer-detail = { $label } { $mint } ({ $amount })
transactions-type-spam-detail = Airdrop de spam ({ $mint })
transactions-type-described = { $description }

transactions-filter-all = Tous les types
transactions-filter-transfer = Transferts
transactions-filter-ata = Rent et comptes
transactions-filter-liquidity = Liquidité
transactions-filter-program = Appels de programme

transactions-direction-tokens-in = Tokens entrants
transactions-direction-tokens-out = Tokens sortants
transactions-direction-sol-in = { -sol } entrant
transactions-direction-sol-out = { -sol } sortant
transactions-direction-internal = Interne
transactions-direction-unknown = Non classée

transactions-status-pending = En attente
transactions-status-confirmed = Confirmée
transactions-status-finalized = Finalisée
transactions-status-failed = Échec
transactions-status-success = Réussie
transactions-status-unknown = Inconnu

transactions-ata-operation-creation = Création
transactions-ata-operation-closure = Fermeture

transactions-toolbar-title = Historique des transactions
transactions-search =
    .placeholder = Rechercher des signatures…
    .aria-label = Rechercher des signatures de transaction
transactions-load-failed = Impossible d'actualiser les transactions
transactions-setup-gate-title = Les transactions nécessitent un portefeuille
transactions-summary-total = Total
transactions-summary-success = Réussies
transactions-summary-failed = Échecs
transactions-filter-wallet = Portefeuille
transactions-filter-type = Type
transactions-filter-direction = Sens
transactions-filter-status = Statut
transactions-filter-all-directions = Tous les sens
transactions-filter-all-statuses = Tous les statuts
transactions-wallet-main = Portefeuille principal
transactions-col-time = Heure
transactions-col-signature = Signature
transactions-col-type = Type
transactions-col-direction = Sens
transactions-col-status = Statut
transactions-col-native-delta = Δ { -sol }
transactions-col-fees = Frais ({ -sol })
transactions-col-token = Token
transactions-col-router = Routeur
transactions-col-instructions = Instr.

transactions-dialog-copy-signature =
    .title = Copier la signature
transactions-dialog-close =
    .title = Fermer (ÉCHAP)
transactions-dialog-tabs-label = Sections des détails de la transaction
transactions-dialog-meta-slot = Slot :
transactions-dialog-meta-fee = Frais :
transactions-dialog-loading = Chargement...
transactions-dialog-loading-details = Chargement des détails de la transaction...
transactions-dialog-load-failed = Impossible de charger les détails de la transaction
transactions-dialog-load-failed-reason = Impossible de charger les détails de la transaction : { $reason }
transactions-dialog-not-found = Transaction introuvable
transactions-dialog-tab-overview = Aperçu
transactions-dialog-tab-balances = Soldes
transactions-dialog-tab-instructions = Instructions
transactions-dialog-tab-logs = Journaux
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = Brut
transactions-dialog-unknown = Inconnu
transactions-dialog-unknown-asset = Actif inconnu
transactions-dialog-unavailable = Indisponible

transactions-dialog-failed-title = Échec de la transaction
transactions-dialog-no-program-error = Aucune erreur de programme n'a été fournie.
transactions-dialog-story-title = Ce qui s'est passé
transactions-dialog-router-via = via { $router }
transactions-dialog-flow-paid = Payé
transactions-dialog-flow-received = Reçu
transactions-dialog-flow-from = De
transactions-dialog-flow-to = Vers
transactions-dialog-flow-amount = Montant
transactions-dialog-net-wallet-change = Variation nette du portefeuille :
transactions-dialog-processed = Traitée sur Solana
transactions-dialog-execution-title = Exécution
transactions-dialog-metric-execution-price = Prix d'exécution
transactions-dialog-metric-effective-received = Reçu effectif
transactions-dialog-metric-effective-spent = Dépensé effectif
transactions-dialog-metric-network-fee = Frais de réseau
transactions-dialog-metric-estimated-pnl = P&L estimé
transactions-dialog-metric-net-native-change = Variation nette de { -sol }
transactions-dialog-route-title = Route et actifs
transactions-dialog-route-router = Routeur
transactions-dialog-route-input-asset = Actif d'entrée
transactions-dialog-route-output-asset = Actif de sortie
transactions-dialog-route-pool = Pool
transactions-dialog-route-program = Programme
transactions-dialog-tech-title = Détails techniques
transactions-dialog-tech-summary = Signature, slot et ressources
transactions-dialog-tech-signature = Signature
transactions-dialog-tech-timestamp = Horodatage
transactions-dialog-tech-slot = Slot
transactions-dialog-tech-exact-fee = Frais exacts
transactions-dialog-tech-accounts = Comptes
transactions-dialog-tech-instructions = Instructions
transactions-dialog-tech-compute-units = Unités de calcul
transactions-dialog-tech-token-decimals = Décimales du token

transactions-dialog-balances-native-title = Variations de solde en { -sol }
transactions-dialog-balances-native-empty = Aucune variation de solde en { -sol }
transactions-dialog-balances-token-title = Variations de solde en tokens
transactions-dialog-balances-token-empty = Aucune variation de solde en tokens
transactions-dialog-balances-net-native = Variation nette de { -sol }
transactions-dialog-balances-fee = Frais de transaction
transactions-dialog-col-account = Compte
transactions-dialog-col-token = Token
transactions-dialog-col-pre-balance = Solde avant
transactions-dialog-col-post-balance = Solde après
transactions-dialog-col-change = Variation
transactions-dialog-col-type = Type
transactions-dialog-col-rent = Rent ({ -sol })
transactions-dialog-instructions-empty = Aucune instruction trouvée
transactions-dialog-instructions-count =
    { $count ->
        [one] { $count } instruction
        [many] { $count } instructions
       *[other] { $count } instructions
    }
transactions-dialog-instruction-program-id = ID du programme
transactions-dialog-instruction-accounts = Comptes ({ $count })
transactions-dialog-instruction-data = Données
transactions-dialog-logs-empty = Aucun journal disponible
transactions-dialog-logs-filter = Filtrer les journaux...
transactions-dialog-logs-no-match = Aucun journal correspondant
transactions-dialog-logs-count =
    { $count ->
        [one] { $count } journal
        [many] { $count } journaux
       *[other] { $count } journaux
    }
transactions-dialog-ata-empty = Aucune opération ATA dans cette transaction
transactions-dialog-ata-summary-title = Résumé de l'analyse ATA
transactions-dialog-ata-creations = Créations
transactions-dialog-ata-closures = Fermetures
transactions-dialog-ata-rent-spent = Rent dépensé
transactions-dialog-ata-rent-recovered = Rent récupéré
transactions-dialog-ata-net-rent = Impact net du rent
transactions-dialog-ata-operations-title = Opérations ATA ({ $count })
transactions-dialog-raw-copy = Copier le JSON
transactions-dialog-raw-empty = Aucune donnée brute disponible

# Empty table (scripts/pages/transactions.js)
transactions-empty = Aucune transaction pour l’instant
    .message = Les swaps et transferts du portefeuille de trading apparaissent ici une fois confirmés on-chain.
