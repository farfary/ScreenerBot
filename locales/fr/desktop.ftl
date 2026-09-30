desktop-action-ok = OK

desktop-splash-starting = Démarrage de { -brand }
desktop-splash-restarting = Redémarrage de { -brand }
desktop-splash-recovering = Récupération
desktop-splash-opening-dashboard = Ouverture du tableau de bord
desktop-splash-checking-dependencies = Vérification des dépendances
desktop-splash-installing-dependencies = Installation des dépendances système
desktop-splash-installing-dependencies-detail = { -brand } nécessite Microsoft Visual C++ Redistributable pour fonctionner.
desktop-splash-resetting-wallet = Réinitialisation des données du portefeuille
desktop-splash-resetting-wallet-detail = Les données actuelles du portefeuille sont sauvegardées avant d'être effacées.
desktop-splash-updating = Mise à jour vers v{ $version }
desktop-splash-updating-detail = Vos paramètres et vos données restent intacts.
desktop-splash-restoring = Restauration de v{ $version }
desktop-splash-restoring-detail = La mise à jour v{ $failed } n'a pas démarré ; la version précédente prend le relais.

desktop-boot-title-fallback = { -brand } n'a pas pu démarrer
desktop-boot-detail-fallback = Le noyau s'est arrêté de façon inattendue.
desktop-boot-remedy-label = Comment corriger
desktop-boot-log-file-label = Fichier journal :
desktop-boot-action-reset-wallet = Réinitialiser les données du portefeuille et redémarrer
desktop-boot-action-working = En cours...
desktop-boot-action-open-logs = Ouvrir le dossier des journaux
desktop-boot-action-copy = Copier les détails
desktop-boot-action-copied = Copié
desktop-boot-action-quit = Quitter
desktop-boot-subtitle-wallet-mismatch = Un autre portefeuille a été détecté
desktop-boot-subtitle-port-in-use = Un port réseau requis est occupé
desktop-boot-subtitle-lock-held = { -brand } est déjà en cours d'exécution
desktop-boot-subtitle-config-invalid = Problème de configuration
desktop-boot-subtitle-directory-setup = Problème de stockage
desktop-boot-subtitle-generic = Erreur de démarrage

desktop-boot-error-title = { -brand } n'a pas pu démarrer
desktop-boot-error-remedy = Ouvrez le dossier des journaux pour voir ce qui s'est passé, puis redémarrez l'application. Si le problème persiste, contactez le support à t.me/screenerbotio_support.
desktop-boot-error-default = Le noyau s'est arrêté de façon inattendue avant que le tableau de bord soit prêt.
desktop-boot-error-restore-failed = Le noyau mis à jour a échoué et la version précédente n'a pas pu être restaurée ({ $error }).
desktop-boot-error-spawn-failed = Le programme du noyau n'a pas pu être lancé ({ $error }).
desktop-boot-error-spawn-missing = Le programme du noyau n'a pas pu être lancé. Il est peut-être absent ou bloqué par un logiciel de sécurité.
desktop-boot-error-exited-running = Le noyau s'est arrêté alors que le tableau de bord était en cours d'exécution (code de sortie { $code }).
desktop-boot-error-exited-early = Le noyau s'est arrêté avant que le tableau de bord soit prêt (code de sortie { $code }).
desktop-boot-error-dashboard-load = Le chargement du tableau de bord a échoué ({ $description }, { $code }).
desktop-boot-error-renderer-gone = Le moteur d'affichage du tableau de bord s'est arrêté ({ $reason }).
desktop-boot-error-unresponsive = Le tableau de bord ne répond plus.
desktop-boot-error-url-failed = L'URL du tableau de bord n'a pas pu être chargée ({ $error }).
desktop-boot-error-relaunch-setup = Impossible de relancer le noyau après la configuration.
desktop-boot-error-relaunch-recovery = Impossible de relancer le noyau pour la récupération.
desktop-boot-error-restart-offline = Le noyau n'est pas revenu en ligne après le redémarrage.
desktop-boot-error-recovery-offline = La récupération est terminée, mais le noyau n'est pas devenu prêt.
desktop-boot-error-start-timeout = Le noyau n'a pas fini de démarrer à temps. Cela peut arriver lors d'un premier lancement lent ou si un autre programme bloque la connexion.

desktop-tray-tooltip = { -brand } - Bot de trading Solana
desktop-tray-show = Afficher { -brand }
desktop-tray-open-dashboard = Ouvrir le tableau de bord
desktop-tray-quit = Quitter { -brand }

desktop-menu-open-data-folder = Ouvrir le dossier des données
desktop-menu-open-logs-folder = Ouvrir le dossier des journaux
desktop-menu-documentation = Documentation
desktop-menu-telegram-support = Support { -telegram }
desktop-menu-check-updates = Rechercher des mises à jour...

desktop-menu-file = Fichier
desktop-menu-edit = Édition
desktop-menu-view = Affichage
desktop-menu-window = Fenêtre
desktop-menu-help = Aide
desktop-menu-reset-zoom = Réinitialiser le zoom
desktop-menu-zoom-in = Zoom avant
desktop-menu-zoom-out = Zoom arrière
desktop-menu-keyboard-shortcuts = Raccourcis clavier
desktop-menu-telegram-channel = Canal { -telegram }
desktop-menu-telegram-community = Communauté { -telegram }
desktop-menu-follow-x = Suivre sur { -x } ({ -twitter })
desktop-menu-visit-website = Visiter le site web
desktop-menu-about = À propos de { -brand }

desktop-about-title = À propos de { -brand }
desktop-about-message = { -brand }
desktop-about-detail =
    Version { $version }

    Bot avancé de gestion de portefeuilles et de trading automatique sur Solana.

    https://screenerbot.io

    © 2024-2026 { -brand }

desktop-shortcuts-title = Raccourcis clavier
desktop-shortcuts-message = Raccourcis clavier de { -brand }
desktop-shortcuts-body-mac =
    Raccourcis clavier :

    Contrôles de la fenêtre :
      Cmd+M          Réduire
      Cmd+W          Fermer la fenêtre
      Cmd+Q          Quitter
      Cmd+Ctrl+F     Basculer le plein écran

    Zoom :
      Cmd++          Zoom avant
      Cmd+-          Zoom arrière
      Cmd+0          Réinitialiser le zoom

    Navigation :
      Cmd+R          Recharger le tableau de bord
      Cmd+Shift+D    Ouvrir le dossier des données

    Autres :
      F1             Ouvrir la documentation
      Cmd+Alt+I      Basculer les outils de développement
desktop-shortcuts-body-other =
    Raccourcis clavier :

    Contrôles de la fenêtre :
      Alt+F4         Quitter
      F11            Basculer le plein écran

    Zoom :
      Ctrl++         Zoom avant
      Ctrl+-         Zoom arrière
      Ctrl+0         Réinitialiser le zoom

    Navigation :
      Ctrl+R         Recharger le tableau de bord
      Ctrl+Shift+D   Ouvrir le dossier des données

    Autres :
      F1             Ouvrir la documentation
      Ctrl+Shift+I   Basculer les outils de développement

desktop-close-title = Fermer { -brand }
desktop-close-message = Que souhaitez-vous faire ?
desktop-close-detail = { -brand } peut continuer à s'exécuter en arrière-plan. Le bot de trading continuera à surveiller et à trader lorsqu'il est réduit dans la zone de notification.
desktop-close-minimize = Réduire dans la zone de notification
desktop-close-quit = Quitter complètement
desktop-close-cancel = Annuler

desktop-vcredist-missing-title = Dépendance manquante
desktop-vcredist-missing-message = Visual C++ Redistributable est manquant
desktop-vcredist-missing-detail = { -brand } nécessite Microsoft Visual C++ Redistributable pour fonctionner. Voulez-vous l'installer maintenant ?
desktop-vcredist-install = Installer et corriger
desktop-vcredist-exit = Quitter
desktop-vcredist-not-found-title = Programme d'installation introuvable
desktop-vcredist-not-found-message = Impossible de localiser { $name } correctement.
desktop-vcredist-done-title = Installation terminée
desktop-vcredist-done-message = Dépendances installées avec succès.
desktop-vcredist-done-detail = { -brand } va maintenant démarrer.
desktop-vcredist-failed-title = Échec de l'installation
desktop-vcredist-failed-message = Veuillez installer Visual C++ Redistributable manuellement.
