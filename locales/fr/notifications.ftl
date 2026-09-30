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
notifications-state-locked = L'état est déterminé par l'onglet
notifications-cancelled = Annulée
notifications-dismiss = Fermer
notifications-dismiss-failed = Impossible de fermer la notification
notifications-load-failed = Échec du chargement
notifications-mark-read-failed = Impossible de marquer les notifications comme lues
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
