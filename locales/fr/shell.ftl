shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-brand-logo =
    .alt = { -brand }
shell-version = v{ $version }

shell-header-brand =
    .aria-label = Ouvrir l'accueil du tableau de bord
    .title = Accueil du tableau de bord
shell-bot-card =
    .aria-label = Chargement du statut d'Auto Trader
shell-bot-label = Auto
shell-bot-status-loading = CHARGEMENT
shell-bot-today = Aujourd'hui
shell-explore-control =
    .aria-label = Mode Explorer. Connectez un portefeuille et un endpoint RPC pour activer toutes les fonctionnalités
    .title = Connectez un portefeuille et un endpoint RPC pour activer le trading, les soldes et les données on-chain en direct
shell-explore-title = Mode Explorer
shell-explore-detail = Portefeuille et RPC non connectés
shell-explore-action = Terminer la configuration
shell-wallet-card =
    .aria-label = Valeur du portefeuille ; ouvrir les positions
    .title = Valeur du portefeuille ({ -sol } + tokens) · ouvrir les positions
shell-wallet-worth-label = VALEUR
shell-wallet-sol-label = { -sol }
shell-wallet-tokens-label = TKN
shell-sol-price-card =
    .aria-label = Prix du { -sol } en USD — ouvrir le graphique
    .title = Prix du { -sol } · cliquer pour le graphique
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24h
shell-copy-card =
    .aria-label = Copy trading ; ouvrir Copy trading
    .title = Copy trading · ouvrir Copy trading
shell-copy-label = COPIE
shell-actions-more =
    .aria-label = Plus d'actions de l'en-tête
    .title = Plus d'actions
shell-actions-group =
    .aria-label = Actions de l'en-tête
shell-action-search =
    .aria-label = Rechercher des tokens
    .title = Rechercher des tokens (Ctrl/Cmd+K)
shell-action-featured =
    .aria-label = Tokens à la une
    .title = Tokens à la une
shell-action-notifications =
    .aria-label = Actions et notifications
    .title = Actions et notifications
shell-action-restart =
    .aria-label = Redémarrer l'application
    .title = Redémarrer l'application
shell-action-theme =
    .aria-label = Changer de thème
    .title = Changer de thème
shell-action-settings =
    .aria-label = Paramètres
    .title = Paramètres

shell-ticker-monitoring-segment =
    .title = Tokens surveillés par le service de pools
shell-ticker-monitoring = Surveillés :
shell-ticker-filtering-segment =
    .title = Tokens ayant validé ou échoué aux critères de filtrage
shell-ticker-passed = Validés :
shell-ticker-rejected = Rejetés :
shell-ticker-pnl-segment =
    .title = Profits et pertes réalisés aujourd'hui
shell-ticker-pnl = P&L du jour :
shell-ticker-rpc-segment =
    .title = Appels RPC par minute et taux de réussite
shell-ticker-rpc = RPC :
shell-ticker-rpc-per-minute = /min
shell-ticker-services-segment =
    .title = État de santé des services en arrière-plan
shell-ticker-services-loading = Services : <strong>Chargement</strong>

shell-notification-title = Actions
shell-notification-mark-all-read =
    .title = Tout marquer comme lu
shell-notification-clear-all =
    .title = Tout effacer
shell-notification-close =
    .aria-label = Fermer
shell-notification-tab-all = Toutes
shell-notification-tab-active = Actives
shell-notification-tab-done = Terminées
shell-notification-tab-failed = Échouées
shell-notification-filter-type-all = Tous les types
shell-notification-filter-type-buy = Achat
shell-notification-filter-type-sell = Vente
shell-notification-filter-type-open = Ouverture
shell-notification-filter-type-close = Clôture
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = Partielle
shell-notification-filter-state-all = Tous les états
shell-notification-filter-state-in-progress = En cours
shell-notification-filter-state-completed = Terminée
shell-notification-filter-state-failed = Échouée
shell-notification-filter-state-cancelled = Annulée
shell-notification-list =
    .aria-label = Notifications
shell-notification-empty = Aucune action pour le moment
shell-notification-loading-more = Chargement de plus d'éléments...
shell-notification-back-to-top =
    .title = Retour en haut

shell-status-bar-version = v
shell-status-bar-uptime = Actif
shell-status-bar-memory = Mém.
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/min
shell-status-bar-trading = Trading
shell-status-bar-positions = Pos.
shell-status-bar-tokens = Tokens

shell-splash-starting = Démarrage de { -brand }
shell-splash-waiting = En attente de la réponse du noyau local.
shell-splash-failed = { -brand } n'a pas pu démarrer
shell-splash-failed-detail = Consultez le fichier journal, puis redémarrez l'application.

shell-connection-connected = Noyau connecté
shell-connection-waiting = En attente du noyau…
shell-connection-retry-now = Réessayer maintenant
shell-connection-overlay-detail = Le noyau est injoignable. Le trading est suspendu ; la reprise sera automatique.
shell-connection-restored = Connexion au noyau rétablie

shell-trader-control-failed = Échec du contrôle du trader
shell-notification-button-unread = Actions et notifications, { $count } non lues
shell-restart-confirm-title = Redémarrer le bot
shell-restart-confirm-message =
    Voulez-vous vraiment redémarrer le bot ?

    Cette action va :
    • arrêter tous les services
    • redémarrer le processus
    • prendre environ 10 à 15 secondes

    Toutes les opérations en cours seront interrompues.
shell-restart-confirm-action = Redémarrer
shell-restart-progress = Redémarrage du bot
shell-restart-failed = Échec du redémarrage
shell-restart-failed-status = Échec du redémarrage : { $status }
shell-restart-helper-unavailable = L'assistant de redémarrage automatique est indisponible. Rechargez le tableau de bord dans un instant.

shell-page-title-fallback = Tableau de bord
shell-page-load-failed = Échec du chargement de la page
shell-page-offline-detail = Le noyau est actuellement injoignable. Cette page se chargera automatiquement dès le retour de la connexion.

shell-bot-state-explore = EXPLORER
shell-bot-state-halted = ARRÊTÉ
shell-bot-state-off = DÉSACTIVÉ
shell-bot-state-waiting = EN ATTENTE
shell-bot-state-idle = INACTIF
shell-bot-state-entry-paused = ENTRÉES SUSPENDUES
shell-bot-state-running = EN MARCHE
shell-bot-control-explore = Auto Trader indisponible en mode Explorer. Ouvrez la configuration du portefeuille et du RPC.
shell-bot-control-halted = L'arrêt d'urgence est actif. Ouvrez les contrôles d'Auto Trader.
shell-bot-control-off = Auto Trader est désactivé. Cliquez pour l'activer.
shell-bot-control-waiting = Auto Trader est activé et attend les services du noyau. Cliquez pour le désactiver.
shell-bot-control-idle = Auto Trader est activé, mais les deux moniteurs sont désactivés. Ouvrez les contrôles d'Auto Trader.
shell-bot-control-entry-paused = La protection contre les pertes a suspendu les entrées ; les sorties peuvent continuer. Ouvrez les contrôles d'Auto Trader.
shell-bot-control-running = Auto Trader est en marche. Cliquez pour le désactiver.

shell-wallet-card-summary = Valeur du portefeuille : { $equity } { -sol } ({ $balance } { -sol } de liquidités, { $tokens } tokens) ; ouvrir les positions
shell-copy-running-live = { $count } en réel
shell-copy-running-paper = { $count } simulées
shell-copy-value-paused = Suspendu
shell-copy-value-idle = Inactif
shell-copy-sub-active = { $active } actives sur { $total }

shell-ticker-services-healthy = Services : <strong>Sains</strong>
shell-ticker-services-issues =
    { $count ->
        [one] Services : <strong>{ $count } problème</strong>
        [many] Services : <strong>{ $count } problèmes</strong>
       *[other] Services : <strong>{ $count } problèmes</strong>
    }

shell-agent-request-title = Demande d'un agent
shell-agent-request-client-fallback = Un agent associé
shell-agent-request-message = { $client } souhaite exécuter « { $tool } » dans { -brand }. Cette demande { $expiry }.
shell-agent-request-message-arguments = { $client } souhaite exécuter « { $tool } » dans { -brand }. Arguments : { $summary }. Cette demande { $expiry }.
shell-agent-request-expires-minutes = expire dans { $minutes }min
shell-agent-request-expires-seconds = expire dans { $seconds }s
shell-agent-request-approve = Approuver
shell-agent-request-deny = Refuser

shell-toast-copied = { $label } copié
shell-toast-copy-failed = Échec de la copie
shell-toast-still-running = Toujours en cours — consultez le centre de notifications
shell-toast-dismiss =
    .aria-label = Fermer
shell-confirm-title = Confirmer l'action
shell-confirm-message = Êtes-vous sûr ?
shell-address-open-solscan = — ouvrir dans { -solscan }
shell-address-copy = Copier l'adresse

shell-assistant-label = Assistant
shell-assistant-dialog =
    .aria-label = Assistant

shell-status-bar-trading-active = Actif
shell-status-bar-trading-inactive = Inactif

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title } annulé
shell-action-swap-buy-live = Achat
shell-action-swap-buy-done = Acheté
shell-action-swap-buy-failed = Achat échoué
shell-action-swap-sell-live = Vente
shell-action-swap-sell-done = Vendu
shell-action-swap-sell-failed = Vente échouée
shell-action-position-open-live = Ouverture de la position
shell-action-position-open-done = Ouverte
shell-action-position-open-failed = Ouverture échouée
shell-action-position-close-live = Clôture de la position
shell-action-position-close-done = Clôturée
shell-action-position-close-failed = Clôture échouée
shell-action-position-dca-live = Renfort de la position
shell-action-position-dca-done = Renforcée
shell-action-position-dca-failed = Renfort échoué
shell-action-partial-exit-live = Sortie partielle
shell-action-partial-exit-done = Sortie partielle
shell-action-partial-exit-failed = Sortie partielle échouée
shell-action-manual-order-live = Envoi de l'ordre
shell-action-manual-order-done = Ordre envoyé
shell-action-manual-order-failed = Ordre échoué
shell-action-trade-live = Trade
shell-action-trade-done = Trade terminé
shell-action-trade-failed = Trade échoué
shell-action-via-router = { $action } via { $router }
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = en évitant { $venue }
shell-action-cost-guard-avoiding-cost = en évitant { $venue } · { $cost }
shell-action-cost-guard-avoiding-unnamed = en évitant une plateforme
shell-action-cost-guard-avoiding-unnamed-cost = en évitant une plateforme · { $cost }
shell-action-cost-guard-avoided = { $outcome } · { $cost } de loyer évités sur { $venue }
shell-action-cost-guard-avoided-unnamed = { $outcome } · { $cost } de loyer de plateforme évités
shell-action-exit-full = Sortie totale
shell-action-exit-percent = Sortie de { $percent }

shell-exit-title = Fermer { -brand } ?
shell-exit-description = Choisissez comment fermer l'application
shell-exit-minimize = Réduire dans la zone de notification
shell-exit-minimize-detail = Continuer en arrière-plan
shell-exit-quit = Quitter l'application
shell-exit-quit-detail = Fermer complètement et arrêter tous les services

shell-lightbox-save =
    .title = Enregistrer l'image
shell-lightbox-close =
    .title = Fermer (ÉCHAP)

shell-theme-light = Clair
shell-theme-dark = Sombre
shell-theme-switch-to-light = Passer au thème clair
shell-theme-switch-to-dark = Passer au thème sombre
