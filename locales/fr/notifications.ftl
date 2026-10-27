# Notification panel labels.

notifications-action-swap-buy = Achat
notifications-action-swap-sell = Vente
notifications-action-position-open = Ouverture
notifications-action-position-close = Clôture
notifications-action-position-dca = DCA
notifications-action-position-partial-exit = Sortie partielle
notifications-action-manual-order = Manuel
notifications-action-unknown = Action

actions-step-evaluate = Évaluation
actions-step-validate = Validation
actions-step-quote = Obtention de la cotation
actions-step-swap = Exécution du swap
actions-step-verify = Vérification
actions-step-unknown = Traitement
actions-step-evaluate-short = Évaluation
actions-step-validate-short = Contrôle
actions-step-quote-short = Cotation
actions-step-swap-short = Swap
actions-step-verify-short = Confirmation
actions-step-unknown-short = En cours

actions-failure-recorded = { $message }
actions-failure-unknown = Erreur inconnue
actions-failure-interrupted = Interrompu par le redémarrage de l'application
actions-failure-validation = Échec de la validation
actions-failure-quote = Échec de la cotation
actions-failure-swap = Échec du swap
actions-failure-trade = Échec du trade
actions-failure-entry = Échec de l'entrée
actions-failure-entry-withdrawn = Entrée retirée
actions-failure-exit = Échec de la sortie
actions-failure-dca = Échec du DCA
actions-failure-verification-expired = Vérification expirée : la transaction n'a jamais abouti
actions-failure-verification-gave-up = La vérification a été abandonnée
actions-failure-transaction-failed = La transaction a échoué on-chain
actions-failure-sell-transaction-failed = La transaction de vente a échoué on-chain
actions-failure-dca-verification-failed = Échec de la vérification du DCA

notifications-empty-all = Aucune action
notifications-empty-active = Aucune action active
notifications-empty-completed = Aucune action terminée
notifications-empty-failed = Aucune action échouée
notifications-source-auto = Auto
notifications-source-manual = Manuel
notifications-cancelled = Annulée
notifications-load-failed = Échec du chargement
notifications-clear-title = Effacer les notifications
notifications-clear-message = Fermer toutes les notifications de cette liste ? Elles restent dans l'historique des actions terminées et échouées.
notifications-clear-failed = Impossible d'effacer les notifications
notifications-stream-lag-title = Le flux d'actions a pris du retard
notifications-stream-lag-missed =
    { $count ->
        [one] { $count } mise à jour manquée
        [many] { $count } mises à jour manquées
       *[other] { $count } mises à jour manquées
    } — actualisation en cours
notifications-stream-lag-refreshing = Actualisation en cours
notifications-sync-failed = Impossible d'actualiser les actions

# Action rows (ui/action_row.js): day headings, the copy-trading source, the detail.
notifications-day-today = Aujourd'hui
notifications-day-yesterday = Hier
notifications-source-copy = Copy trading
notifications-steps = Étapes
notifications-detail-transaction = Transaction
notifications-detail-fill-price = Prix d'exécution
    .title = Montant en { -sol } de ce swap divisé par sa quantité de tokens
notifications-detail-fee = Frais de réseau
notifications-detail-pnl = P&L de la position
notifications-detail-route = Route
notifications-detail-duration = Durée
notifications-detail-reason = Motif
notifications-view-token = Voir le token
notifications-view-position = Voir la position
notifications-view-transaction = Voir la transaction
notifications-unrecorded = Non enregistré sur la position
notifications-unrecorded-detail = Ce swap n'a pas été comptabilisé sur la position et n'a peut-être pas abouti. Vérifiez sa transaction.
