services-health-component-unavailable = Composant { $component } indisponible
services-health-unavailable = Statut de santé indisponible
services-health-pools-not-running = Le service de pools n'est pas en cours d'exécution
services-health-events-db-uninitialized = Base de données des événements non initialisée
services-health-sol-price-not-running = Le service de prix du { -sol } n'est pas en cours d'exécution
services-health-sol-price-stale = Les données de prix du { -sol } sont obsolètes ({ $seconds } s)
services-health-sol-price-no-data = Aucune donnée de prix du { -sol } disponible pour le moment
services-health-telegram-discovery = Mode découverte
services-health-telegram-disconnected = Déconnecté
services-health-wallet-watch-polling-only = Détection par interrogation uniquement
services-health-assistant-tasks-disabled = Désactivé dans la configuration
services-health-connectivity-critical-unhealthy = Endpoints critiques défaillants : { $endpoints }
services-health-filtering-snapshot-stale = L'instantané du filtrage date de { $seconds } s

## Services page (pages/services.js)

services-status-healthy = Sain
services-status-starting = Démarrage
services-status-degraded = Dégradé
services-status-unhealthy = Défaillant
services-status-stopping = Arrêt
services-status-disabled = Désactivé
services-status-unknown = Inconnu

services-name-account = Compte
services-name-assistant-scheduled-tasks = Tâches planifiées de l'Assistant
services-name-ata-cleanup = Nettoyage des comptes de token
services-name-connectivity = Connectivité
services-name-copy-trading = Copy trading
services-name-events = Événements
services-name-filtering = Filtrage
services-name-llm-analysis = Analyse LLM
services-name-ohlcv = OHLCV
services-name-pool-pricing = Tarification des pools
services-name-pools = Pools
services-name-positions = Positions
services-name-referral = Parrainage
services-name-rpc-stats = Statistiques RPC
services-name-sol-price = Prix du { -sol }
services-name-telegram = { -telegram }
services-name-tokens = Tokens
services-name-trader = Trader
services-name-transactions = Transactions
services-name-update-check = Vérification des mises à jour
services-name-wallet = Portefeuille
services-name-wallet-watch = Surveillance de portefeuille
services-name-webserver = Serveur web

services-loading = Chargement des services...
services-load-failed = Échec du chargement des services
services-load-failed-description = En attente de la réponse du backend. Nouvelle tentative automatique.
services-refresh-failed = Impossible d'actualiser les services
services-search-placeholder = Rechercher des services...
services-summary-total = Total
services-summary-alerts = Alertes
services-summary-alerts-tooltip = { $degraded } dégradés / { $unhealthy } défaillants
services-filter-status = Statut
services-filter-all-statuses = Tous les statuts
services-filter-all-services = Tous les services
services-filter-enabled-only = Activés uniquement
services-filter-disabled-only = Désactivés uniquement
services-col-service = Service
services-col-health = Santé
services-col-priority = Priorité
services-col-uptime = Disponibilité
services-col-activity = Activité
services-col-last-cycle = Dernier cycle
services-col-avg-cycle = Cycle moy.
services-col-avg-poll = Interrog. moy.
services-col-cycle-rate = Fréquence des cycles
services-col-tasks = Tâches
services-col-ops = Ops/s
services-col-errors = Erreurs
services-col-dependencies = Dépendances
services-dependencies-none = Aucune
services-activity-busy = { $percent } occupé
services-activity-polls =
    { $count ->
        [one] { $count } interrogation
        [many] { $count } interrogations
       *[other] { $count } interrogations
    }
services-tasks-tooltip =
    { $count ->
        [one] { $count } tâche
        [many] { $count } tâches
       *[other] { $count } tâches
    }
    Dernière : { $last }
    Moy. : { $avg }
    Interrogation : { $poll }
    Inactivité : { $idle }
    Total des interrogations : { $polls }
services-tasks-none = Aucune tâche instrumentée

# Empty table (scripts/pages/services.js)
services-empty = Aucun service en cours
    .message = Les services apparaissent ici une fois démarrés par le bot.
