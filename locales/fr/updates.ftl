updates-defer-automatic-install-disabled = L'installation automatique est désactivée. La mise à jour est prête et sera appliquée quand vous le déciderez.
updates-defer-trading-active = Une position, un trade ou une opération d'outil est en cours ; le redémarrage est donc différé. La mise à jour s'applique automatiquement lorsque l'application est inactive.
updates-defer-needs-installer = Cette version met aussi à jour le shell de bureau ; le programme d'installation doit donc être exécuté une fois.

updates-check-failed = { $cause }
updates-check-failed-legacy = { $cause }

updates-download-started = Téléchargement de la mise à jour v{ $version }...
updates-apply-started = Installation de la mise à jour. { -brand } redémarre et se reconnecte automatiquement.
updates-install-opened = Programme d'installation vérifié ouvert. Terminez l'installation dans le programme du système d'exploitation.

updates-installer-toast-title = Programme d'installation ouvert
updates-installer-toast-message = { -brand } va maintenant se fermer proprement.

updates-tab-status = Statut
updates-tab-release-notes = Notes de version
updates-tab-preferences = Préférences
updates-tab-sections = Sections des mises à jour
updates-checking-installation = Vérification de cette installation...

updates-phase-idle-headline = Prêt à rechercher des mises à jour
updates-phase-idle-detail = { -brand } v{ $version } est installé.
updates-phase-up-to-date-headline = Vous êtes à jour
updates-phase-up-to-date-detail = { -brand } v{ $version } est la dernière version.
updates-phase-checking-headline = Recherche de mises à jour
updates-phase-checking-detail = Recherche de la dernière version publiée.
updates-phase-available-headline = La version { $version } est disponible
updates-phase-downloading-headline = Téléchargement de v{ $version }
updates-phase-verifying-headline = Vérification de v{ $version }
updates-phase-verifying-detail = Comparaison du téléchargement avec son checksum publié.
updates-phase-ready-to-apply-headline = La version { $version } est prête
updates-phase-ready-to-apply-detail = La mise à jour peut être installée maintenant avec un court redémarrage, ou automatiquement au prochain démarrage.
updates-phase-ready-to-install-headline = La version { $version } est prête
updates-phase-ready-to-install-detail = Le programme d'installation de bureau est prêt à terminer cette mise à jour.
updates-phase-applying-headline = Installation de la mise à jour
updates-phase-applying-detail = { -brand } redémarre sur la nouvelle version.
updates-phase-applied-headline = Mis à jour vers v{ $version }
updates-phase-applied-detail = La mise à jour a été installée. Rien d'autre n'est nécessaire.
updates-phase-failed-headline = La mise à jour ne s'est pas terminée
updates-phase-failed-detail = Réessayez la mise à jour.
updates-phase-check-failed-headline = Impossible de rechercher des mises à jour
updates-phase-check-failed-detail = Le service de versions est injoignable.
updates-status-unavailable-headline = Le statut de la mise à jour est indisponible
updates-phase-unrecognized-detail = L'état de mise à jour signalé n'est pas reconnu.
updates-status-load-failed-detail = Le statut de l'installation n'a pas pu être chargé.

updates-kind-core = Mise à jour du noyau · { $size } · court redémarrage
updates-kind-full = Mise à jour de bureau · { $size } · installateur requis
updates-size-unknown = taille inconnue

updates-action-check-now = Rechercher maintenant
updates-action-check-again = Rechercher à nouveau
updates-action-try-again = Réessayer
updates-action-download = Télécharger la mise à jour
updates-action-restart = Redémarrer pour mettre à jour
updates-action-open-installer = Ouvrir le programme d'installation

updates-busy-checking = Recherche...
updates-busy-resuming = Reprise du téléchargement...
updates-busy-starting-download = Démarrage du téléchargement...
updates-busy-restarting = Redémarrage...
updates-busy-opening-installer = Ouverture du programme d'installation...

updates-progress-downloading = Téléchargement de la mise à jour
updates-progress-verifying = Vérification de la mise à jour
updates-progress-transferred = { $done } sur { $total }
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }, { $percent }, { $transferred }

updates-detail-list-label = Détails de l'installation
updates-detail-installed-version = Version installée
updates-detail-system = Système
updates-detail-last-checked = Dernière vérification
updates-detail-never = Jamais
updates-detail-available-version = Version disponible
updates-detail-download-size = Taille du téléchargement

updates-version-installed = Installée
updates-version-available = Disponible

updates-notes-highlights = Points forts
updates-notes-empty-title = Aucune note de version pour le moment
updates-notes-empty-error = L'historique des versions n'a pas pu être chargé. Vérifiez la connexion et réessayez.
updates-notes-empty-none = Les notes de version apparaîtront ici dès qu'une version aura été publiée.
updates-notes-history-notice = Affichage de ce que cette installation connaît déjà : l'historique des versions n'a pas pu être chargé.
updates-release-empty = Aucun changement n'a été listé pour cette version.
updates-release-changes =
    { $count ->
        [one] { $count } changement
        [many] { $count } changements
       *[other] { $count } changements
    }

updates-preferences-unavailable-title = Les préférences de mise à jour sont indisponibles
updates-preferences-unavailable-detail = La configuration des mises à jour n'a pas pu être chargée.
updates-preference-fallback-name = préférence de mise à jour
updates-preference-save-failed = Impossible d'enregistrer { $preference }

updates-request-failed = Échec de la requête
updates-check-request-failed = Impossible de rechercher des mises à jour
updates-resume-failed = Impossible de reprendre le téléchargement de la mise à jour
updates-download-failed = Impossible de démarrer le téléchargement de la mise à jour
updates-apply-failed = Impossible d'installer la mise à jour
updates-install-failed = Impossible d'ouvrir le programme d'installation de la mise à jour
updates-apply-confirm-title = Installer v{ $version }
updates-apply-confirm-message = { -brand } redémarre sur la nouvelle version. Le trading s'arrête quelques secondes puis reprend automatiquement ; les positions ouvertes ne sont pas touchées.
updates-install-confirm-title = Lancer le programme d'installation
updates-install-confirm-message = Le programme d'installation vérifié s'ouvre et { -brand } se ferme proprement. Terminez l'installation, puis rouvrez { -brand }.

updates-version-number = v{ $version }
