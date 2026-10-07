## Reply keyboard words. Icons come from ReplyCommand in src/telegram/reply.rs.

telegram-reply-status = Statut
telegram-reply-balance = Solde
telegram-reply-positions = Positions
telegram-reply-pause = Pause
telegram-reply-resume = Reprendre
telegram-reply-stop = Stop
telegram-reply-stats = Stats
telegram-reply-menu = Menu
telegram-reply-help = Aide

## Inline keyboard buttons.

telegram-button-positions = Positions
telegram-button-balance = Solde
telegram-button-stats = Stats
telegram-button-tokens = Tokens
telegram-button-pause = Pause
telegram-button-stop = Stop
telegram-button-settings = Paramètres
telegram-button-refresh = Actualiser
telegram-button-menu = Menu
telegram-button-back = Retour
telegram-button-back-to-menu = Retour au menu
telegram-button-back-to-tokens = Retour aux tokens
telegram-button-cancel = Annuler
telegram-button-close-all-positions = Clôturer toutes les positions
telegram-button-sell-percent = Vendre { $percent }{ " " }%
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = Liste noire
telegram-button-blacklist-symbol = Ajouter { $symbol } à la liste noire
telegram-button-close-position = Clôturer la position
telegram-button-confirm-close = Confirmer la clôture
telegram-button-confirm-close-all = Clôturer TOUTES les positions
telegram-button-confirm-sell = Confirmer la vente de { $percent }{ " " }%
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = CONFIRMER L'ARRÊT FORCÉ
telegram-button-confirm-buy = Acheter { $amount } { -sol }
telegram-button-notifications = Notifications
telegram-button-trading = Trading
telegram-button-entry-monitor = Moniteur d'entrée
telegram-button-exit-monitor = Moniteur de sortie
telegram-button-auto-trading = Trading automatique
telegram-button-force-stop = Arrêt forcé
telegram-button-notify-opened = Ouverture
telegram-button-notify-closed = Clôture
telegram-button-notify-partial = Partielle
telegram-button-notify-dca = DCA
telegram-button-notify-errors = Erreurs
telegram-button-details = Détails
telegram-button-position = Position
telegram-button-sell-more = Vendre plus
telegram-button-more-dca = Plus de DCA
telegram-button-history = Historique
telegram-button-status = Statut
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = Se réauthentifier
telegram-button-previous = Préc.
telegram-button-next = Suiv.
telegram-button-passed = Validés
telegram-button-rejected = Rejetés
telegram-button-new-24h = Nouveaux (24 h)
telegram-button-all-tokens = Tous les tokens
telegram-button-search-token = Rechercher un token
telegram-button-filter-stats = Stats des filtres
telegram-button-refresh-stats = Actualiser les stats
telegram-button-view-position = Voir la position
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    Commande inconnue : { $command }

    Utilisez /help pour voir les commandes disponibles.
telegram-session-expired =
    <b>Session expirée</b>

    Utilisez /login pour vous réauthentifier.
telegram-2fa-required =
    <b>2FA requise</b>

    Veuillez saisir votre code d'authentification à 6 chiffres.
telegram-account-locked =
    <b>Compte verrouillé</b>

    Trop de tentatives échouées.
    Réessayez dans { $seconds ->
        [one] { $seconds } seconde.
        [many] { $seconds } secondes.
       *[other] { $seconds } secondes.
    }
telegram-code-invalid = Veuillez saisir un code valide à 6 chiffres.
telegram-authenticated =
    <b>Authentifié{ " " }!</b>

    Vous avez désormais accès aux commandes du bot.
telegram-wrong-code =
    <b>Code incorrect</b>

    { $remaining ->
        [one] { $remaining } tentative restante.
        [many] { $remaining } tentatives restantes.
       *[other] { $remaining } tentatives restantes.
    }
telegram-auth-required =
    <b>Authentification requise</b>

    Veuillez saisir votre mot de passe pour continuer.

    <i>Tapez votre mot de passe et envoyez-le.</i>
telegram-login-required =
    <b>Connexion requise</b>

    Veuillez saisir votre code d'authentification à 6 chiffres :
telegram-session-activated =
    <b>Session activée</b>

    La 2FA n'est pas configurée. Votre session est maintenant active.

    <i>Astuce : activez la 2FA dans les paramètres de sécurité pour une meilleure protection.</i>

## Chat discovery.

telegram-discovery-hello = Bonjour { $name }{ " " }!
telegram-discovery-default-name = Utilisateur
telegram-discovery-detected = <b>Conversation détectée{ " " }!</b>
telegram-discovery-details =
    ID de la conversation : <code>{ $chat_id }</code>
    Type : { $chat_type }

    Rendez-vous dans le tableau de bord { -brand } et cliquez sur cette conversation pour la sélectionner.
telegram-chat-type-private = privée
telegram-chat-type-group = groupe
telegram-chat-type-supergroup = supergroupe
telegram-chat-type-channel = canal

## Menus.

telegram-menu-title =
    <b>Panneau de contrôle</b>

    Sélectionnez une option pour consulter des informations ou piloter le bot.
telegram-menu-positions-empty =
    <b>Aucune position ouverte</b>

    En attente de nouvelles opportunités...
telegram-menu-positions-title = <b>Positions ({ $count })</b>
telegram-menu-positions-hint = <i>Touchez une position pour la gérer.</i>
telegram-menu-settings =
    <b>Paramètres</b>

    Configurez les notifications et les paramètres de trading.
telegram-settings-notifications =
    <b>Paramètres de notification</b>

    Activez ou désactivez les notifications :
telegram-settings-trading =
    <b>Contrôles de trading</b>

    Activez ou désactivez les fonctions de trading :
telegram-pagination-expired = La session de pagination a expiré.

## Status commands.

telegram-status-state-stopped = <b>ARRÊTÉ</b> (arrêt forcé actif)
telegram-status-state-active = <b>ACTIF</b>
telegram-status-state-paused = <b>EN PAUSE</b>
telegram-status-on = ON
telegram-status-off = OFF
telegram-status-body =
    <b>Statut du système</b>

    <b>Système</b>
    État — { $state }
    Disponibilité — { $uptime }
    Version — v{ $version }

    <b>Trading</b>
    Entrées — { $entries }
    Sorties — { $exits }
    Positions — { $positions }
telegram-positions-empty =
    <b>Aucune position ouverte</b>

    En attente d'opportunités...
telegram-positions-title = <b>Positions ouvertes ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }{ " " }%)
telegram-positions-more = <i>+{ $count } de plus...</i>
telegram-positions-summary =
    <b>Résumé du portefeuille</b>
    Investi — { $invested } { -sol }
    P{ "&amp;" }L net — { $pnl } { -sol }
telegram-balance-body =
    <b>Solde du portefeuille</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>Statistiques du jour</b>

    Positions — { $positions }
    Investi — { $invested } { -sol }
    P{ "&amp;" }L — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } est prêt{ " " }!</b>

    Le trading est <b>activé</b>.

    Utilisez le clavier ci-dessous pour piloter le bot.
    Tapez /help pour voir les commandes disponibles.
telegram-stop-already = <b>Le trading est déjà désactivé</b>
telegram-stop-done =
    <b>Trading désactivé</b>

    Tous les moniteurs de trading (entrées { "&amp;" } sorties) sont arrêtés.
    Utilisez /pause pour arrêter uniquement les entrées.
telegram-stop-failed =
    <b>Échec de la désactivation du trading</b>

    Erreur : { $detail }
telegram-pause-done =
    <b>Moniteur d'entrée en pause</b>

    Aucune nouvelle position ne sera ouverte.
    Le moniteur de sortie continue de fonctionner.
telegram-pause-failed =
    <b>Échec de la mise en pause des entrées</b>

    Erreur : { $detail }
telegram-resume-done =
    <b>Moniteur d'entrée repris</b>

    Surveillance des signaux d'entrée en cours.
telegram-resume-failed =
    <b>Échec de la reprise des entrées</b>

    Erreur : { $detail }
telegram-force-stop-confirm =
    <b>ARRÊT FORCÉ</b>

    Ceci interrompt immédiatement TOUTE l'activité de trading :
    • Aucune nouvelle entrée
    • Aucune sortie (y compris les stop loss)
    • Aucune opération DCA
telegram-force-stop-warning = <b>Il s'agit d'une action d'urgence{ " " }!</b>
telegram-force-stop-question = Êtes-vous sûr{ " " }?
telegram-force-stop-active =
    <b>ARRÊT FORCÉ ACTIVÉ</b>

    Tout le trading a été interrompu.

    Utilisez /resume_trading pour lever ce blocage.
telegram-resume-trading-not-stopped =
    <b>Le trading n'est pas en arrêt forcé</b>

    Aucune action nécessaire.
telegram-resume-trading-done =
    <b>Trading repris</b>

    Le blocage d'arrêt forcé a été levé.
    Les opérations de trading normales peuvent reprendre.

## Help.

telegram-help-title = <b>Aide { -brand }</b>
telegram-help-heading-dashboard = Tableau de bord
telegram-help-heading-market = Marché
telegram-help-heading-trading = Trading
telegram-help-heading-safety = Sécurité
telegram-help-heading-system = Système
telegram-help-commands-dashboard =
    /status — Statut du système { "&amp;" } disponibilité
    /stats — Performances du jour
    /balance — Solde du portefeuille
    /positions — Positions ouvertes
telegram-help-commands-market =
    /tokens — Explorateur de tokens
    /rejected — Tokens filtrés
telegram-help-commands-trading =
    /start — Activer le système de trading
    /stop — Désactiver le système de trading
    /pause — Suspendre les nouvelles entrées
    /resume — Reprendre les nouvelles entrées
    /menu — Menu interactif
telegram-help-commands-safety =
    /force_stop — <b>ARRÊT D'URGENCE</b>
    /resume_trading — Lever l'état d'urgence
telegram-help-commands-system =
    /update — Statut des mises à jour { "&amp;" } installation
    /login — Authentification 2FA
telegram-help-tip = <i>Astuce : touchez une commande pour l'exécuter.</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>À jour</b>

    Version v{ $version } en cours, installée automatiquement.
telegram-update-up-to-date =
    <b>À jour</b>

    Version v{ $version } en cours.
telegram-update-check-failed =
    <b>Échec de la vérification des mises à jour</b>

    { $reason }
telegram-update-unreachable = Impossible de joindre screenerbot.io.
telegram-update-installing = <b>Installation de v{ $version }</b>
telegram-update-restarting =
    { -brand } redémarre sur la nouvelle version. Le trading reprend automatiquement.
telegram-update-install-failed =
    <b>Impossible d'installer v{ $version }</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } est téléchargée</b>

    Cette version met aussi à jour l'application de bureau ; son programme d'installation doit donc être exécuté sur la machine. Ouvrez Paramètres → Mises à jour sur place.
telegram-update-downloading =
    <b>Téléchargement de v{ $version }</b>

    { $percent }{ " " }% de { $size } Mo.
telegram-update-available =
    <b>v{ $version } est disponible</b>

    { $how }
    Taille du téléchargement : { $size } Mo.

    Le téléchargement se fait automatiquement ; envoyez /update à nouveau lorsqu'elle est prête.
telegram-update-how-core = S'installe en silence avec un court redémarrage.
telegram-update-how-installer = Nécessite d'exécuter une fois le programme d'installation de bureau.

## Shared values and units. Numbers arrive formatted; only the unit words live here.

telegram-value-unknown = Inconnu
telegram-value-na = N/D
telegram-percent-value = { $percent }{ " " }%
telegram-price-native = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds }s
telegram-duration-minutes = { $minutes }min
telegram-duration-minutes-seconds = { $minutes }min { $seconds }s
telegram-duration-hours = { $hours }h
telegram-duration-hours-minutes = { $hours }h { $minutes }min
telegram-duration-days = { $days }j
telegram-duration-days-hours = { $days }j { $hours }h
telegram-pnl = { $sol } { -sol } ({ $percent }{ " " }%)
telegram-amount-native = { $amount } { -sol }
telegram-error-line = Erreur : { $detail }
telegram-ai-reasoning =
    <b>Analyse LLM</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens. Icons come from Rust.

telegram-row-entry = Entrée — { $price } { -sol }
telegram-row-exit = Sortie — { $price } { -sol }
telegram-row-current = Actuel — { $price } { -sol }
telegram-row-invested = Investi — { $amount } { -sol }
telegram-row-received = Reçu — { $amount } { -sol }
telegram-row-value = Valeur — { $amount } { -sol }
telegram-row-total = Total — { $amount } { -sol }
telegram-row-tokens = Tokens — { $tokens }
telegram-row-duration = Durée — { $duration }
telegram-row-reason = Motif — { $reason }
telegram-row-remaining = Restant — { $percent }{ " " }%
telegram-row-pnl = P{ "&amp;" }L — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>Position ouverte</b>
telegram-notify-opened-size = Taille — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = Prix — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>Position clôturée</b> — Profit
telegram-notify-closed-title-loss = <b>Position clôturée</b> — Perte
telegram-notify-closed-reason-unspecified = Clôturée
telegram-notify-partial-title = <b>Sortie partielle</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — { $percent }{ " " }% vendu
telegram-notify-dca-title = <b>DCA n° { $count }</b>
telegram-notify-dca-added = Ajouté — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = Moy. — { $price } { -sol }
telegram-notify-unbooked-title = <b>Swap non comptabilisé</b>
telegram-notify-unbooked-body = Un swap confirmé on-chain n'est pas encore dans sa position. Il est revérifié jusqu'à sa comptabilisation.
telegram-notify-unbooked-signature = Transaction : <code>{ $signature }</code>
telegram-notify-severity-critical = <b>Erreur critique</b>
telegram-notify-severity-error = <b>Erreur</b>
telegram-notify-severity-warning = <b>Avertissement</b>
telegram-notify-severity-info = <b>Info</b>
telegram-notify-alert-title = <b>Alerte de trade</b>
telegram-notify-alert-token = Token : <code>${ $symbol }</code>
telegram-notify-alert-mint = Mint : <code>{ $mint }</code>
telegram-notify-alert-bought = Action : achat de { $amount } { -sol }
telegram-notify-alert-sold = Action : vente de { $amount } { -sol }
telegram-notify-alert-wallet = Portefeuille : <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (simulé)
telegram-notify-copy-task = Tâche : { $task }
telegram-notify-scheduled-completed = <b>Tâche planifiée terminée</b>
telegram-notify-scheduled-failed = <b>Échec de la tâche planifiée</b>
telegram-notify-scheduled-timed-out = <b>Délai de la tâche planifiée dépassé</b>
telegram-notify-scheduled-error = Erreur : { $error }
telegram-notify-summary-title = <b>Résumé du jour</b> — { $date }
telegram-notify-summary-performance = <b>Performance</b>
telegram-notify-summary-trades = Trades — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = Taux de réussite — { $percent }{ " " }%
telegram-notify-summary-pnl = P{ "&amp;" }L — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = Positions ouvertes — { $count }
telegram-notify-started-title = <b>{ -brand } démarré</b>
telegram-notify-started-version = <b>Version</b> — { $version }
telegram-notify-started-mode = <b>Mode</b> — { $mode }
telegram-notify-started-ready = Prêt pour le trading{ " " }!
telegram-notify-stopped-title = <b>{ -brand } arrêté</b>
telegram-notify-stopped-reason = <b>Motif</b> — { $reason }
telegram-notify-stopped-goodbye = À bientôt{ " " }! { $icon }
telegram-notify-start-mode-normal = Normal
telegram-notify-stop-reason-graceful = Arrêt propre
telegram-notify-update-available =
    <b>Mise à jour v{ $version } disponible</b>

    { $how }
    Taille du téléchargement : { $size } Mo
telegram-notify-update-how-installer = Cette version met aussi à jour l'application de bureau ; son programme d'installation doit donc être exécuté une fois.
telegram-notify-update-ready =
    <b>Mise à jour v{ $version } prête</b>

    { $how }
telegram-notify-update-ready-silent = Envoyez /update pour l'appliquer maintenant, ou elle s'installera au prochain démarrage de { -brand }.
telegram-notify-update-ready-installer = Ouvrez Paramètres → Mises à jour pour exécuter le programme d'installation.
telegram-notify-update-applying =
    <b>Installation de v{ $version }</b>

    Le backend redémarre ; le trading reprend automatiquement.
telegram-notify-new-tokens =
    <b>Alerte de filtrage</b>

    { $count ->
        [one] { $count } nouveau token correspondant à vos critères a été trouvé.
        [many] { $count } nouveaux tokens correspondant à vos critères ont été trouvés.
       *[other] { $count } nouveaux tokens correspondant à vos critères ont été trouvés.
    }
telegram-notify-crash =
    <b>Le bot a planté{ " " }!</b>

    <b>Emplacement :</b> <code>{ $location }</code>
    <b>Erreur :</b> <code>{ $error }</code>
telegram-notify-crash-restart = Veuillez redémarrer le bot.

## Filter results page.

telegram-filter-results-title = <b>Résultats du filtrage</b> ({ $count })
telegram-filter-results-empty = <i>Aucun token trouvé.</i>
telegram-filter-results-page = <i>Page { $page } sur { $total }</i>

## Position screens.

telegram-position-not-found = Position introuvable
telegram-position-no-positions = Aucune position à clôturer
telegram-position-history-empty =
    <b>Historique des trades</b>

    Aucune position clôturée pour le moment.
telegram-position-history-title = <b>Trades récents</b>
telegram-position-history-more =
    <i>+{ $count } { $count ->
        [one] trade de plus
        [many] trades de plus
       *[other] trades de plus
    }...</i>
telegram-position-confirm-hint = <i>Confirmez sous 30 s pour exécuter.</i>
telegram-position-confirm-close-title = <b>Clôturer la position{ " " }?</b>
telegram-position-confirm-close-selling = Vente de { $tokens } tokens
telegram-position-confirm-close-estimated = Estimé — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>Confirmez sous 30 secondes</i>
telegram-position-confirm-sell =
    <b>Confirmer la vente</b>

    Token — { $symbol }
    Montant — { $percent }{ " " }%
    Tokens — { $tokens }
telegram-position-confirm-dca =
    <b>Confirmer le renfort</b>

    Token — { $symbol }
    Ajout — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>Clôturer toutes les positions{ " " }?</b>

    Nombre — { $count }
telegram-position-confirm-close-all-hint =
    <i>Toutes les positions ouvertes seront vendues au marché.
    Confirmez sous 30 s.</i>
telegram-position-confirm-force-stop =
    <b>ARRÊT FORCÉ</b>

    Ceci interrompt immédiatement TOUT le trading :
    • Aucune nouvelle entrée
    • Aucune sortie
    • Aucun DCA
telegram-position-confirm-force-stop-warning = <b>Il s'agit d'une action d'urgence.</b>
telegram-position-confirm-blacklist =
    <b>Ajouter le token à la liste noire{ " " }?</b>

    Token — { $symbol }
    Mint — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>La position sera clôturée et toute future entrée sera bloquée.</i>
telegram-position-selling = Vente de { $percent }{ " " }% de { $symbol }...
telegram-position-sell-done =
    <b>Vente exécutée</b>

    Token — { $symbol }
    Vendu — { $percent }{ " " }%
    Reçu — { $amount } { -sol }
telegram-position-sell-failed = <b>Échec de la vente</b>
telegram-position-adding = Ajout de { $amount } { -sol } à { $symbol }...
telegram-position-dca-done =
    <b>DCA exécuté</b>

    Token — { $symbol }
    Ajouté — { $amount } { -sol }
telegram-position-dca-failed = <b>Échec du DCA</b>
telegram-position-closing-all = Clôture de toutes les positions...
telegram-position-close-all-done =
    <b>Clôture globale terminée</b>

    Clôturées — { $closed }
    Échecs — { $failed }
telegram-position-blacklisted =
    <b>Token ajouté à la liste noire</b>

    Token — { $symbol }
    Statut — Clôturé { "&amp;" } sur liste noire

## Token screens.

telegram-token-not-found = Token introuvable
telegram-token-not-found-prefix = Token introuvable. Essayez avec un préfixe plus long.
telegram-token-stats-failed = Échec de la récupération des stats : { $detail }
telegram-token-list-failed = Échec de la récupération des tokens : { $detail }
telegram-token-list-empty = Aucun token trouvé dans la vue <b>{ $view }</b>.
telegram-token-view-passed = Filtre validé
telegram-token-view-rejected = Rejetés
telegram-token-view-recent = Ajoutés récemment
telegram-token-view-all = Tous les tokens
telegram-token-list-title = <b>{ $name }</b> (page { $page }/{ $total })
telegram-token-list-stats = Liq. : { $liquidity } • Prix : { $price }
telegram-token-list-hint = <i>Touchez /token_ID pour voir les détails</i>
telegram-token-explorer =
    <b>Explorateur de marché</b>

    <b>Aperçu</b>
    Filtre validé — { $passed }
    Rejetés — { $rejected }
    Prix actifs — { $priced }
    Total découvert — { $total }

    <i>Sélectionnez une catégorie à parcourir :</i>
telegram-token-filter-title = <b>Analyse des filtres</b>
telegram-token-filter-distribution = <b>Répartition</b>
telegram-token-filter-passed = Validés — { $count } ({ $percent }{ " " }%)
telegram-token-filter-rejected = Rejetés — { $count } ({ $percent }{ " " }%)
telegram-token-filter-blacklisted = Sur liste noire — { $count }
telegram-token-filter-coverage = <b>Couverture</b>
telegram-token-filter-priced = Avec prix de pool — { $count }
telegram-token-filter-open = Positions ouvertes — { $count }
telegram-token-filter-total = Total découvert — { $count }
telegram-token-filter-updated = <b>Dernière mise à jour</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>Actualisation automatique toutes les { $interval }</i>
telegram-token-detail-active = <b>Position active</b>
telegram-token-detail-price = Prix — { $price } { -sol }
telegram-token-detail-liquidity = Liquidité — { $value }
telegram-token-detail-volume = Volume 24 h — { $value }
telegram-token-detail-change = Variation 24 h — { $value }
telegram-token-detail-risk = Évaluation du risque : { $score }/100
telegram-token-detail-risk-unknown = Évaluation du risque : inconnue
telegram-token-detail-action = <i>Sélectionnez une action :</i>
telegram-token-search =
    <b>Rechercher sur le marché</b>

    Saisissez un symbole ou une adresse de mint :

    <i>Exemple : /token_BONK ou /token_So11111</i>
telegram-token-confirm-buy =
    <b>Confirmer l'achat direct</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>
    Montant — { $amount } { -sol }

    <i>Confirmez sous 30 s pour exécuter.</i>
telegram-token-confirm-blacklist =
    <b>Ajouter le token à la liste noire{ " " }?</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>

    <i>Ce token ne pourra plus satisfaire les filtres.</i>
telegram-token-blacklisted =
    <b>Token ajouté à la liste noire</b>

    Token — ${ $symbol }
    Statut — Ajouté à la liste noire
telegram-token-blacklist-failed = <b>Échec de l'ajout à la liste noire</b>
telegram-token-buy-processing =
    <b>Traitement de l'achat...</b>

    Token — ${ $symbol }
    Montant — { $amount } { -sol }
telegram-token-buy-done =
    <b>Achat réussi</b>

    Token — ${ $symbol }
    Montant — { $amount } { -sol }

    <i>Consultez les détails dans /positions</i>
telegram-token-buy-failed =
    <b>Échec de l'achat</b>

    Token — ${ $symbol }
    Erreur — { $detail }
