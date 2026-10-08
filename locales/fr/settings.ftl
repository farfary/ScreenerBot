# The Settings dialog. Each section is named after the script that owns it.

## Shared

settings-duration-minutes =
    { $count ->
        [one] { $count } minute
        [many] { $count } minutes
       *[other] { $count } minutes
    }
settings-duration-hours =
    { $count ->
        [one] { $count } heure
        [many] { $count } heures
       *[other] { $count } heures
    }

## settings_dialog.js

settings-dialog-title = Paramètres
settings-dialog-close =
    .title = Fermer (Échap)
    .aria-label = Fermer les paramètres
settings-dialog-save = Enregistrer les modifications
settings-dialog-saving = Enregistrement…
settings-dialog-saved = Enregistré
settings-dialog-save-success = Paramètres enregistrés
settings-dialog-save-failed = Échec de l'enregistrement des paramètres
settings-dialog-update-attention = La mise à jour requiert votre attention
settings-dialog-tab-interface = Interface
settings-dialog-tab-navigation = Navigation
settings-dialog-tab-startup = Démarrage
settings-dialog-tab-hints = Astuces
settings-dialog-tab-data = Données
settings-dialog-tab-security = Sécurité
settings-dialog-tab-account = Compte
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = Connexions d'agents
settings-dialog-tab-updates = Mises à jour
settings-dialog-tab-licenses = Licences
settings-dialog-tab-about = À propos
settings-dialog-link-privacy = Politique de confidentialité
settings-dialog-link-terms = Conditions d'utilisation

## settings_dialog.js: Startup tab

settings-startup-section-title = Comportement au démarrage
settings-startup-auto-start-label = Démarrer le Trader automatiquement
settings-startup-auto-start-hint = Démarre automatiquement le Trader au lancement
settings-startup-coming-soon = Bientôt disponible
settings-startup-default-page-label = Page par défaut
settings-startup-default-page-hint = Page affichée à l'ouverture de l'application
settings-startup-page-dashboard = Tableau de bord
settings-startup-page-tokens = Tokens
settings-startup-page-positions = Positions
settings-startup-page-wallet = Portefeuille
settings-startup-page-config = Configuration
settings-startup-notifications-label = Afficher les notifications en arrière-plan
settings-startup-notifications-hint = Affiche des notifications pour les événements en arrière-plan

## settings_dialog.js: About tab

settings-about-tagline = Moteur de trading Solana natif
settings-about-link-github = { -github }
settings-about-link-docs = Documentation
settings-about-link-telegram = { -telegram }
settings-about-link-website = Site web
settings-about-credits = Conçu pour les traders Solana
settings-about-copyright = © { $year } { -brand }. Tous droits réservés.

## interface_tab.js

settings-interface-section-appearance = Apparence
settings-interface-theme-label = Thème
settings-interface-theme-hint = Choisissez votre jeu de couleurs préféré
settings-interface-theme-dark = Sombre
settings-interface-theme-light = Clair
settings-interface-language-label = Langue
settings-interface-language-hint = Langue d'affichage du tableau de bord
settings-interface-logo-shape-label = Forme des logos de token
settings-interface-logo-shape-hint = « Cercle » rogne chaque logo ; « Naturelle » préserve la silhouette propre à chaque illustration
settings-interface-logo-shape-circle = Cercle
settings-interface-logo-shape-natural = Naturelle
settings-interface-animations-label = Activer les animations
settings-interface-animations-hint = Transitions et effets fluides
settings-interface-compact-label = Mode compact
settings-interface-compact-hint = Réduit les marges pour afficher plus de contenu
settings-interface-section-data = Données et affichage
settings-interface-refresh-label = Intervalle d'actualisation
settings-interface-refresh-hint = Fréquence d'actualisation des données
settings-interface-refresh-seconds =
    { $count ->
        [one] { $count } seconde
        [many] { $count } secondes
       *[other] { $count } secondes
    }
settings-interface-refresh-minutes =
    { $count ->
        [one] { $count } minute
        [many] { $count } minutes
       *[other] { $count } minutes
    }
settings-interface-ticker-label = Afficher le bandeau de cours
settings-interface-ticker-hint = Bandeau de métriques en direct dans l'en-tête
settings-interface-page-size-label = Taille de page des tableaux
settings-interface-page-size-hint = Nombre de lignes par page de tableau par défaut
settings-interface-page-size-rows =
    { $count ->
        [one] { $count } ligne
        [many] { $count } lignes
       *[other] { $count } lignes
    }
settings-interface-auto-expand-label = Développer les catégories automatiquement
settings-interface-auto-expand-hint = Développe les catégories de configuration par défaut
settings-interface-hints-label = Afficher les astuces contextuelles
settings-interface-hints-hint = Affiche des icônes d'aide qui expliquent les fonctions du tableau de bord
settings-interface-featured-label = Afficher la rangée À la une
settings-interface-featured-hint = Affiche la rangée des tokens à la une sur les pages Accueil et Tokens
settings-interface-section-sound = Effets sonores
settings-interface-sounds-label = Activer les sons
settings-interface-sounds-hint = Repères sonores pour la navigation, les changements d'état et les résultats

## security_tab.js

settings-security-loading = Chargement des paramètres de sécurité…
settings-security-load-failed = Échec du chargement des paramètres de sécurité

settings-security-type-pin4 = Code PIN à 4 chiffres
settings-security-type-pin6 = Code PIN à 6 chiffres
settings-security-type-text = Mot de passe texte
settings-security-type-unset = Non défini

settings-security-lockscreen-title = Écran de verrouillage du tableau de bord
settings-security-lockscreen-description = Protégez votre tableau de bord avec un code PIN ou un mot de passe. L'écran de verrouillage s'affiche lorsqu'il est déclenché et exige une authentification pour continuer.
settings-security-enable-label = Activer l'écran de verrouillage
settings-security-enable-hint = Protège votre tableau de bord par mot de passe
settings-security-password-status-label = État du mot de passe
settings-security-password-current = Actuel : { $type }
settings-security-password-none = Aucun mot de passe défini
settings-security-change = Modifier
settings-security-remove = Supprimer
settings-security-set-password = Définir un mot de passe
settings-security-auto-lock-label = Verrouillage automatique après inactivité
settings-security-auto-lock-hint = Verrouille automatiquement après une période sans activité
settings-security-auto-lock-never = Jamais
settings-security-lock-blur-label = Verrouiller quand la fenêtre perd le focus
settings-security-lock-blur-hint = Verrouille automatiquement lorsque vous passez à une autre application
settings-security-quick-actions-title = Actions rapides
settings-security-lock-now-label = Verrouiller le tableau de bord maintenant
settings-security-lock-now-hint = Verrouille immédiatement le tableau de bord
settings-security-lock-now = Verrouiller maintenant
settings-security-lock-not-ready = Verrouillage impossible : écran de verrouillage non prêt
settings-security-setting-save-failed = Impossible d'enregistrer le paramètre de sécurité

## security_tab.js: two-factor authentication

settings-security-2fa-title = Authentification à deux facteurs
settings-security-2fa-description = Ajoutez un niveau de sécurité supplémentaire avec une application d'authentification (Google Authenticator, Authy, etc.)
settings-security-2fa-status-label = État de la 2FA
settings-security-2fa-status-enabled = L'authentification à deux facteurs est activée
settings-security-2fa-status-none = Non configurée
settings-security-2fa-disable = Désactiver la 2FA
settings-security-2fa-enable = Activer la 2FA

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = Fermer
settings-security-password-set-title = Définir un mot de passe
settings-security-password-change-title = Modifier le mot de passe
settings-security-password-current-label = Mot de passe actuel
settings-security-password-current-input =
    .placeholder = Saisissez le mot de passe actuel
settings-security-password-type-label = Type de mot de passe
settings-security-password-new-label = Nouveau mot de passe
settings-security-password-new-input =
    .placeholder = Saisissez le nouveau mot de passe
settings-security-password-confirm-label = Confirmer le mot de passe
settings-security-password-confirm-input =
    .placeholder = Confirmez le mot de passe
settings-security-password-update = Mettre à jour le mot de passe
settings-security-placeholder-pin4 = Saisissez le code PIN à 4 chiffres
settings-security-placeholder-pin6 = Saisissez le code PIN à 6 chiffres
settings-security-placeholder-text = Saisissez le mot de passe
settings-security-password-required = Veuillez saisir un mot de passe
settings-security-password-mismatch = Les mots de passe ne correspondent pas
settings-security-pin4-invalid = Le code PIN doit comporter exactement 4 chiffres
settings-security-pin6-invalid = Le code PIN doit comporter exactement 6 chiffres
settings-security-text-too-short = Le mot de passe doit comporter au moins 4 caractères
settings-security-password-saved = Mot de passe enregistré
settings-security-password-save-failed = Échec de l'enregistrement du mot de passe
settings-security-password-save-failed-detail = Échec de l'enregistrement du mot de passe : { $message }

## security_tab.js: remove password dialog

settings-security-remove-title = Supprimer le mot de passe
settings-security-remove-description = Saisissez votre mot de passe actuel pour retirer la protection de l'écran de verrouillage.
settings-security-remove-confirm = Supprimer le mot de passe
settings-security-current-required = Veuillez saisir votre mot de passe actuel
settings-security-password-removed = Mot de passe supprimé
settings-security-password-remove-failed = Échec de la suppression du mot de passe
settings-security-password-remove-failed-detail = Échec de la suppression du mot de passe : { $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = Activer l'authentification à deux facteurs
settings-security-2fa-password-prompt = Saisissez votre mot de passe pour continuer :
settings-security-2fa-password-input =
    .placeholder = Saisissez le mot de passe
settings-security-2fa-continue = Continuer
settings-security-2fa-manual-code = Code de saisie manuelle :
settings-security-2fa-qr =
    .alt = QR code TOTP
settings-security-2fa-code-prompt = Saisissez le code à 6 chiffres de votre application d'authentification :
settings-security-2fa-verify-enable = Vérifier et activer
settings-security-2fa-password-required = Veuillez saisir votre mot de passe
settings-security-2fa-setup-failed = Échec de la configuration de la 2FA
settings-security-2fa-code-invalid-length = Veuillez saisir un code à 6 chiffres
settings-security-2fa-code-invalid = Code invalide
settings-security-2fa-enabled = Authentification à deux facteurs activée
settings-security-2fa-verify-failed = Échec de la vérification du code
settings-security-2fa-disable-title = Désactiver l'authentification à deux facteurs
settings-security-2fa-disable-prompt = Saisissez votre mot de passe pour désactiver la 2FA :
settings-security-2fa-disable-failed = Échec de la désactivation de la 2FA
settings-security-2fa-disabled = Authentification à deux facteurs désactivée

## agent_connections_tab.js

settings-agent-category-analysis = Analyse
settings-agent-category-portfolio = Portefeuille
settings-agent-category-trading = Trading
settings-agent-category-config = Configuration
settings-agent-category-system = Système
settings-agent-category-analysis-description = Analyse des tokens, données de marché et contrôles de sécurité.
settings-agent-category-portfolio-description = Positions ouvertes, soldes et P&L.
settings-agent-category-trading-description = Achat, vente et clôture de positions avec des fonds réels.
settings-agent-category-config-description = Tous les paramètres du bot, y compris les endpoints RPC. Jamais les clés des portefeuilles.
settings-agent-category-system-description = État, événements et arrêt d'urgence.
settings-agent-category-analysis-inline = l'analyse
settings-agent-category-portfolio-inline = le portefeuille
settings-agent-category-trading-inline = le trading
settings-agent-category-config-inline = la configuration
settings-agent-category-system-inline = le système

settings-agent-level-allow = Autoriser
settings-agent-level-ask-user = Demander
settings-agent-level-deny = Désactivé
settings-agent-level-allow-hint = S'exécute immédiatement.
settings-agent-level-ask-user-hint = Attend votre approbation dans l'application.
settings-agent-level-deny-hint = Refusé et masqué pour l'agent.

settings-agent-preset-full = Accès complet
settings-agent-preset-ask = Demander d'abord
settings-agent-preset-read = Lecture seule
settings-agent-preset-full-description = Tout s'exécute sans demander. Les clés des portefeuilles restent inaccessibles.
settings-agent-preset-ask-description = Chaque action attend votre approbation dans l'application.
settings-agent-preset-read-description = Lecture de l'analyse et du portefeuille. Rien ne peut être modifié.
settings-agent-preset-custom = Personnalisé
settings-agent-preset-group =
    .aria-label = Préréglage d'autorisations
settings-agent-permission-group = Autorisation : { $category }

settings-agent-summary-asks-only = Limité — demande pour { $asking }
settings-agent-summary-off-only = Limité — pas de { $off }
settings-agent-summary-asks-and-off = Limité — demande pour { $asking } ; pas de { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = MCP stdio générique

settings-agent-note-placeholder = Remplacez /absolute/path/to/screenerbot par le chemin absolu de votre binaire { -brand } — l'application en cours d'exécution n'a pas pu représenter le chemin de son exécutable sur ce système.
settings-agent-note-data-dir = Si vous exécutez { -brand } avec un répertoire de données non standard, définissez aussi SCREENERBOT_DATA_DIR sur le client (un autre indicateur -e / --env, ou une entrée env) avec le même chemin.
settings-agent-note-codex-run = Exécutez la commande, ou ajoutez le bloc TOML à ~/.codex/config.toml ($CODEX_HOME/config.toml). Redémarrez ensuite { -codex }.
settings-agent-note-codex-get = `codex mcp get screenerbot` masque le secret dans sa sortie.
settings-agent-note-claude-code = { -claude } Code : exécutez la commande, puis redémarrez { -claude } Code. `claude mcp get screenerbot` affichera l'environnement configuré, secret compris.
settings-agent-note-claude-desktop = { -claude } Desktop : fusionnez le JSON dans claude_desktop_config.json sous `mcpServers` et redémarrez l'application.
settings-agent-note-openclaw = Exécutez la commande, puis utilisez `openclaw mcp doctor screenerbot --probe` pour vérifier que le serveur stdio enregistré démarre et expose des outils.
settings-agent-note-hermes = Ajoutez ceci sous `mcp_servers` dans le fichier de configuration de { -hermes }, puis redémarrez { -hermes }.
settings-agent-note-generic = Tout client MCP compatible stdio : exécutez cette commande avec ces arguments et cet environnement, là où le client conserve sa liste de serveurs.
settings-agent-block-codex-command = { -codex } CLI — commande de terminal
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (secours)
settings-agent-block-claude-command = { -claude } Code — commande de terminal
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — commande de terminal
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = Client MCP stdio générique

settings-agent-name-required = Saisissez un nom pour cette connexion.
settings-agent-name-too-long = Le nom doit comporter { $max } caractères au maximum.
settings-agent-name-control-characters = Le nom ne doit pas contenir de caractères de contrôle.

settings-agent-title = Connexions d'agents
settings-agent-description = Connectez { -claude }, { -codex }, { -hermes }, { -openclaw } ou tout client MCP stdio. { -brand } doit rester en cours d'exécution. Chaque connexion a ses propres autorisations : accès complet par défaut, limité par connexion quand vous le souhaitez. Aucune connexion ne peut jamais lire ni modifier la clé de votre portefeuille.
settings-agent-name-label = Nom de la connexion
settings-agent-name-hint = Affiché dans la liste ci-dessous pour distinguer les connexions.
settings-agent-name-input =
    .placeholder = Agent de code du portable
settings-agent-client-label = Client
settings-agent-client-hint = Détermine la configuration affichée après la création de la connexion.
settings-agent-permissions-label = Autorisations
settings-agent-permissions-hint = Une nouvelle connexion peut tout faire. Limitez n'importe quelle catégorie maintenant, ou plus tard depuis la liste ci-dessous — les clés des portefeuilles ne sont jamais accessibles dans les deux cas.
settings-agent-create = Créer la connexion
settings-agent-issued-group =
    .aria-label = Identifiants de la nouvelle connexion
settings-agent-issued-warning = Copiez le secret maintenant. Il n'est affiché qu'une fois et ne peut pas être récupéré — révoquez et recréez la connexion si vous le perdez. { -brand } ne conserve qu'un vérificateur à sens unique ; votre client MCP stocke le secret en clair dans sa propre configuration.
settings-agent-issued-client-id = ID client
settings-agent-issued-secret = Secret à usage unique
settings-agent-setup-for = Configuration pour
settings-agent-done = Terminé
settings-agent-list-title = Connexions
settings-agent-loading = Chargement des connexions…
settings-agent-active-count = { $count } actives
settings-agent-empty = Aucune connexion pour l'instant. Créez-en une ci-dessus pour appairer un client.
settings-agent-empty-active = Aucune connexion active.
settings-agent-revoked-title = Connexions révoquées
settings-agent-created = Créée { $time }
settings-agent-last-used = Dernière utilisation { $time }
settings-agent-never-used = Jamais utilisée
settings-agent-permissions-edit = Autorisations
settings-agent-revoke = Révoquer
settings-agent-permissions-save = Enregistrer les autorisations

settings-agent-load-failed = Échec du chargement des connexions d'agents
settings-agent-list-failed = Impossible de charger les connexions
settings-agent-create-failed = Impossible de créer la connexion.
settings-agent-unreachable-create = Impossible de joindre { -brand } pour créer la connexion.
settings-agent-permissions-update-failed = Impossible de mettre à jour les autorisations
settings-agent-permissions-updated = Autorisations mises à jour
settings-agent-permissions-updated-detail = S'applique à la prochaine requête de la connexion.
settings-agent-unreachable-save = Impossible de joindre { -brand } pour enregistrer
settings-agent-revoke-title = Révoquer la connexion
settings-agent-revoke-message = Révoquer « { $label } » ? Le client cesse de fonctionner dès sa prochaine requête et ne peut pas être rétabli.
settings-agent-revoke-fallback-name = cette connexion
settings-agent-revoke-failed = Impossible de révoquer la connexion
settings-agent-unreachable-revoke = Impossible de joindre { -brand } pour révoquer

## telegram_tab.js

settings-telegram-loading = Chargement des paramètres { -telegram }…
settings-telegram-load-failed = Échec du chargement des paramètres { -telegram }
settings-telegram-unknown = Inconnu
settings-telegram-session-active = Active : { $duration }
settings-telegram-sessions-empty = Aucune session active
settings-telegram-session-revoke = Révoquer

settings-telegram-connection-title = Connexion
settings-telegram-connection-description = Connectez votre bot { -telegram } pour recevoir des notifications et contrôler { -brand } à distance.
settings-telegram-enable-label = Activer { -telegram }
settings-telegram-enable-hint = Active l'intégration du bot { -telegram }
settings-telegram-token-label = Token du bot
settings-telegram-token-saved = Token enregistré
settings-telegram-token-help = À obtenir auprès de @BotFather sur { -telegram }
settings-telegram-token-input-saved =
    .placeholder = Token enregistré (saisissez-en un nouveau pour le changer)
settings-telegram-token-input =
    .placeholder = Saisissez le token du bot
settings-telegram-token-toggle =
    .title = Afficher/Masquer
settings-telegram-chat-label = ID du chat
settings-telegram-chat-connected = Connecté au chat :
settings-telegram-chat-discover-hint = Découvrez automatiquement votre ID de chat
settings-telegram-chat-change =
    .title = Modifier
settings-telegram-chat-discover = Découvrir l'ID du chat
settings-telegram-discovery-step-add = Ajoutez votre bot à un groupe { -telegram }, ou démarrez une conversation privée avec lui
settings-telegram-discovery-step-privacy = Pour les groupes : vérifiez @BotFather → /mybots → [votre bot] → Bot Settings → Group Privacy
settings-telegram-discovery-privacy = <strong>Mode confidentialité désactivé :</strong> le bot reçoit tous les messages du groupe<br/><strong>Mode confidentialité activé :</strong> le bot ne reçoit les messages que lorsqu'il est mentionné avec @
settings-telegram-discovery-step-send = Envoyez un message quelconque (ou mentionnez votre bot avec @ si le mode confidentialité est activé)
settings-telegram-discovery-listening = En attente de messages…
settings-telegram-discovery-select = Sélectionner
settings-telegram-chat-id-label = ID :
settings-telegram-language-label = Langue des messages
settings-telegram-language-hint = Langue des messages et des boutons du bot { -telegram }
settings-telegram-language-follow-app = Suivre la langue de l'application
settings-telegram-test-label = Tester la connexion
settings-telegram-test-hint = Envoie un message de test pour vérifier la configuration
settings-telegram-test-send = Envoyer un test
settings-telegram-test-sending = Envoi…

settings-telegram-chat-type-private = privé
settings-telegram-chat-type-group = groupe
settings-telegram-chat-type-supergroup = supergroupe
settings-telegram-chat-type-channel = canal

settings-telegram-auth-title = Authentification des commandes
settings-telegram-auth-description = Les commandes { -telegram } utilisent la même 2FA que l'écran de verrouillage du tableau de bord.
settings-telegram-auth-protected = Protégé
settings-telegram-auth-disabled = Désactivé
settings-telegram-auth-not-configured = Non configuré
settings-telegram-auth-error = Erreur
settings-telegram-auth-protected-note = Les commandes sont protégées par la 2FA de l'écran de verrouillage. À l'expiration des sessions, les utilisateurs doivent fournir leur code d'authentification via la commande <code>/login</code>.
settings-telegram-auth-disabled-note = La 2FA de l'écran de verrouillage est configurée mais désactivée pour { -telegram }. Activez « Exiger la 2FA pour les commandes » ci-dessus pour protéger les commandes { -telegram }.
settings-telegram-auth-missing-note = La 2FA de l'écran de verrouillage n'est pas configurée. Sans 2FA, les sessions expirées se réactivent automatiquement sans vérification.
settings-telegram-auth-managed-in = La 2FA se gère dans
settings-telegram-auth-configure-in = Configurez la 2FA dans
settings-telegram-auth-configure-suffix = pour exiger une vérification pour les commandes { -telegram }.
settings-telegram-security-link = Paramètres de sécurité
settings-telegram-timeout-title = Expiration de session
settings-telegram-timeout-description = Durée pendant laquelle une session authentifiée reste active
settings-telegram-sessions-title = Sessions actives

settings-telegram-notifications-title = Paramètres de notification
settings-telegram-notifications-description = Choisissez les événements qui déclenchent des notifications { -telegram }.
settings-telegram-notify-opened-label = Position ouverte
settings-telegram-notify-opened-hint = Notifie lorsqu'une nouvelle position est ouverte
settings-telegram-notify-closed-label = Position clôturée
settings-telegram-notify-closed-hint = Notifie lorsqu'une position est clôturée
settings-telegram-notify-partial-label = Sortie partielle
settings-telegram-notify-partial-hint = Notifie les sorties partielles de position
settings-telegram-notify-dca-label = DCA exécuté
settings-telegram-notify-dca-hint = Notifie lorsque des ordres DCA sont exécutés
settings-telegram-notify-errors-label = Erreurs
settings-telegram-notify-errors-hint = Notifie les erreurs et les échecs
settings-telegram-notify-startup-label = Démarrage/Arrêt
settings-telegram-notify-startup-hint = Notifie lorsque le bot démarre ou s'arrête
settings-telegram-notify-filtering-label = Alertes de filtrage
settings-telegram-notify-filtering-hint = Notifie lorsque de nouveaux tokens passent les critères de filtrage
settings-telegram-notify-trades-label = Alertes de trades
settings-telegram-notify-trades-hint = Notifie les trades importants sur les tokens surveillés
settings-telegram-notify-daily-label = Résumé quotidien
settings-telegram-notify-daily-hint = Recevez un résumé quotidien de l'activité de trading et du P&L

settings-telegram-features-title = Fonctionnalités
settings-telegram-features-description = Configurez les capacités du bot { -telegram }.
settings-telegram-commands-label = Activer les commandes
settings-telegram-commands-hint = Permet de contrôler le bot via les commandes { -telegram }
settings-telegram-require-2fa-label = Exiger la 2FA pour les commandes
settings-telegram-require-2fa-hint = À l'expiration des sessions, exige un code 2FA pour les réactiver. Utilise la 2FA de l'écran de verrouillage.
settings-telegram-inline-label = Boutons d'action intégrés
settings-telegram-inline-hint = Affiche des boutons d'action dans les messages de notification

settings-telegram-setting-save-failed = Impossible d'enregistrer le paramètre { -telegram }
settings-telegram-discovery-start-failed = Impossible de démarrer la découverte
settings-telegram-chat-selected = Chat sélectionné
settings-telegram-chat-select-failed = Impossible de sélectionner le chat
settings-telegram-test-sent = Message de test envoyé
settings-telegram-test-failed = Échec du message de test
settings-telegram-session-revoked = Session révoquée
settings-telegram-session-revoke-failed = Impossible de révoquer la session

## licenses_tab.js

settings-licenses-title = Licences open source
settings-licenses-subtitle = { -brand } est construit avec les logiciels open source suivants
settings-licenses-footer = Les textes complets des licences sont disponibles dans le dépôt du projet et dans le code source de chaque dépendance.
settings-licenses-category-framework = Framework applicatif
settings-licenses-category-solana = Blockchain Solana
settings-licenses-category-data = Données et stockage
settings-licenses-category-networking = Réseau
settings-licenses-category-cryptography = Cryptographie et encodage
settings-licenses-category-assets = Ressources d'interface
settings-licenses-desc-electron = Framework d'application de bureau
settings-licenses-desc-tokio = Runtime asynchrone pour Rust
settings-licenses-desc-axum = Framework de serveur web
settings-licenses-desc-tower = Abstractions de services
settings-licenses-desc-hyper = Implémentation HTTP
settings-licenses-desc-solana-sdk = Cœur du SDK Solana
settings-licenses-desc-solana-client = Client RPC
settings-licenses-desc-solana-program = Bibliothèque de programmes
settings-licenses-desc-spl-token = Programme SPL Token
settings-licenses-desc-spl-token-2022 = Extensions Token-2022
settings-licenses-desc-spl-associated-token-account = Comptes de token associés
settings-licenses-desc-sqlite = Moteur de base de données embarqué
settings-licenses-desc-rusqlite = Liaisons SQLite pour Rust
settings-licenses-desc-r2d2 = Pool de connexions à la base de données
settings-licenses-desc-serde = Framework de sérialisation
settings-licenses-desc-toml = Analyse de la configuration
settings-licenses-desc-reqwest = Client HTTP
settings-licenses-desc-tokio-tungstenite = Client WebSocket
settings-licenses-desc-rustls = Implémentation TLS
settings-licenses-desc-blake3 = Fonction de hachage
settings-licenses-desc-sha-2 = Hachage SHA-256/512
settings-licenses-desc-bs58 = Encodage Base58
settings-licenses-desc-base64 = Encodage Base64
settings-licenses-desc-lucide-icons = Bibliothèque de polices d'icônes
settings-licenses-desc-inter = Police d'interface
settings-licenses-desc-jetbrains-mono = Police à chasse fixe
settings-licenses-desc-orbitron = Police d'affichage
settings-licenses-desc-vazirmatn = Police pour l'arabe et le persan
settings-licenses-desc-noto-sans-devanagari = Police pour le devanagari
settings-licenses-desc-noto-sans-sc = Police pour le chinois simplifié
settings-licenses-desc-pretendard = Police pour le coréen
settings-licenses-desc-pretendard-jp = Police pour le japonais

## hints_tab.js

settings-hints-title = Astuces contextuelles
settings-hints-description = Les astuces contextuelles sont les icônes d'aide qui expliquent les fonctions du tableau de bord. Passez en revue chaque astuce ci-dessous et rétablissez celles que vous avez masquées avec « Ne plus afficher » — une par une ou toutes ensemble.
settings-hints-hidden-label = Astuces masquées
settings-hints-hidden-summary = { $hidden } astuces sur { $total } sont actuellement masquées.
settings-hints-restore-all = Rétablir toutes les astuces
settings-hints-toggle-shown =
    .title = Afficher cette astuce
settings-hints-toggle-shown-title = Affichée
settings-hints-toggle-hidden-title = Masquée — activez pour afficher
settings-hints-restore-title = Rétablir toutes les astuces
settings-hints-restore-message = Afficher de nouveau toutes les astuces contextuelles, y compris celles que vous avez masquées ?
settings-hints-restore-confirm = Tout rétablir
settings-hints-restored = Toutes les astuces rétablies

## account_tab.js

settings-account-title = Compte { -brand }
settings-account-description = Gratuit et facultatif. { -brand } trade, découvre et trace des graphiques sans compte — il le fait simplement avec les fournisseurs publics. Le panneau ci-dessous liste ce que la connexion apporte en plus.
settings-account-data-title = Données { -brand }
settings-account-data-description = Nous exploitons un service de données de marché partagé sur screenerbot.io : bougies mutualisées sur sept périodes, registre de pools résolu, rapports de sécurité en cache et identité des tokens normalisée. Il existe pour que chaque installation ne soit pas limitée séparément par les fournisseurs publics, et son utilisation exige un compte afin que ce coût partagé soit attribué.
settings-account-data-fallback = Lorsqu'il est indisponible, { -brand } bascule automatiquement vers les fournisseurs publics. Rien ne s'arrête ; les graphiques se remplissent plus lentement et ont moins d'historique.
settings-account-gateway-title = Envoi des transactions
settings-account-gateway-description = Lorsque vous êtes connecté, { -brand } peut diffuser vos swaps via screenerbot.io plutôt que via votre propre RPC. Votre bot construit et signe toujours chaque transaction sur cette machine — le serveur ne fait que la relayer et ne peut pas modifier une transaction signée sans invalider sa signature.
settings-account-gateway-label = Utiliser le RPC { -brand } pour envoyer les transactions
settings-account-gateway-hint = Envoi uniquement. Les données de prix proviennent toujours de votre propre RPC — l'interrogation des pools est bien trop lourde pour un endpoint partagé, elle ne lui est donc jamais envoyée.
settings-account-manage-title = Gérer votre compte
settings-account-manage-description = Votre mot de passe, votre adresse e-mail, vos appareils connectés et vos paiements de parrainage se gèrent sur le site web. Y révoquer un appareil le déconnecte partout, y compris ici.
settings-account-open-dashboard = Ouvrir votre tableau de bord

## navigation_tab.js

settings-navigation-title = Onglets de navigation
settings-navigation-hint = Faites glisser les éléments pour les réorganiser. Basculez leur visibilité avec l'interrupteur.
settings-navigation-note = Les modifications s'appliquent après l'enregistrement. Actualisez la page pour voir les changements dans la barre de navigation.
settings-navigation-drag-handle =
    .title = Faire glisser pour réorganiser
settings-navigation-defaults-failed = Impossible de charger la navigation par défaut
settings-navigation-reset = Navigation rétablie par défaut

## data_tab.js

settings-data-storage-title = Stockage des bases de données
settings-data-storage-description = Vue d'ensemble de toutes les bases de données qui stockent vos données de trading, vos positions et votre historique.
settings-data-stats-loading = Chargement des statistiques des bases de données…
settings-data-stats-load-failed = Échec du chargement des statistiques des bases de données
settings-data-total-storage = Stockage total des bases de données
settings-data-db-tokens = Tokens
settings-data-db-transactions = Transactions
settings-data-db-positions = Positions
settings-data-db-events = Événements
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = Portefeuille
settings-data-db-pools = Pools
settings-data-db-strategies = Stratégies
settings-data-db-actions = Actions
settings-data-directory-label = Répertoire de données
settings-data-directory-copied = Répertoire de données
settings-data-config-path-copied = Chemin de configuration
settings-data-path-unavailable = Indisponible
settings-data-path-copy-title = Cliquez pour copier le chemin
settings-data-path-copy-failed = Échec de la copie du chemin

settings-data-config-title = Gestion de la configuration
settings-data-config-description = Exportez, importez et gérez la configuration de votre bot. Faites des sauvegardes avant tout changement important.
settings-data-config-export = Exporter la configuration
settings-data-config-import = Importer la configuration
settings-data-config-reset = Rétablir les valeurs par défaut
settings-data-config-location-label = Emplacement de la configuration
settings-data-config-fetch-failed = Échec de la récupération de la configuration
settings-data-config-exported = Configuration exportée
settings-data-config-export-failed = Échec de l'export de la configuration : { $message }
settings-data-config-import-title = Importer la configuration
settings-data-config-import-message = Importer cette configuration ? Les paramètres actuels seront écrasés. Les identifiants du portefeuille seront conservés.
settings-data-config-imported = Configuration importée. Certains changements peuvent nécessiter un redémarrage.
settings-data-config-import-failed = Échec de l'import de la configuration : { $message }
settings-data-config-reset-title = Réinitialiser la configuration
settings-data-config-reset-message = Rétablir tous les paramètres par défaut ? Les identifiants de votre portefeuille seront conservés, mais tous les autres paramètres seront réinitialisés.
settings-data-config-reset-done = Configuration rétablie par défaut
settings-data-config-reset-failed = Échec de la réinitialisation de la configuration : { $message }
settings-data-unknown-error = Erreur inconnue

settings-data-cleanup-title = Nettoyage des données
settings-data-cleanup-description = Libérez de l'espace disque en supprimant les données anciennes ou inutilisées. Ces actions sont irréversibles.
settings-data-ohlcv-cleanup-label = Nettoyage des données OHLCV
settings-data-ohlcv-cleanup-hint = Supprime les données de bougies des tokens inactifs depuis la durée indiquée.
settings-data-cleanup-hours-unit = heures
settings-data-cleanup-ohlcv = Nettoyer l'OHLCV
settings-data-cleanup-running = Nettoyage…
settings-data-cleanup-hours-invalid = Valeur d'heures invalide
settings-data-cleanup-confirm-title = Supprimer les données OHLCV
settings-data-cleanup-confirm-message =
    Supprimer les données OHLCV des tokens inactifs depuis plus de { $hours ->
        [one] { $hours } heure
        [many] { $hours } heures
       *[other] { $hours } heures
    } ?
settings-data-cleanup-done =
    { $count ->
        [one] { $count } token inactif nettoyé
        [many] { $count } tokens inactifs nettoyés
       *[other] { $count } tokens inactifs nettoyés
    }
settings-data-cleanup-failed = Échec du nettoyage
settings-data-cleanup-failed-detail = Échec du nettoyage : { $message }

settings-data-cache-clear-label = Vider tout le cache OHLCV
settings-data-cache-clear-hint = Efface toutes les bougies en cache et récupère à nouveau chaque token suivi depuis zéro. À utiliser si les graphiques semblent incorrects ou après une mise à jour de la logique des données.
settings-data-cache-clear = Vider le cache OHLCV
settings-data-cache-clearing = Vidage…
settings-data-cache-confirm-title = Vider tout le cache OHLCV
settings-data-cache-confirm-message = Effacer toutes les bougies en cache pour chaque token ? Les tokens suivis récupéreront leur historique depuis zéro. Cette action est irréversible.
settings-data-candles-count =
    { $count ->
        [one] { $count } bougie
        [many] { $count } bougies
       *[other] { $count } bougies
    }
settings-data-tokens-count =
    { $count ->
        [one] { $count } token
        [many] { $count } tokens
       *[other] { $count } tokens
    }
settings-data-cache-cleared = { $candles } effacées sur { $tokens } ; nouvelle récupération en cours
settings-data-cache-clear-failed = Échec du vidage du cache OHLCV
settings-data-cache-clear-failed-detail = Échec du vidage du cache OHLCV : { $message }

settings-data-ui-cache-label = Cache d'état de l'interface
settings-data-ui-cache-hint = Efface les préférences de tableaux, les états de filtres et les réglages de vues enregistrés.
settings-data-ui-cache-clear = Vider le cache de l'interface
settings-data-ui-cache-confirm-title = Vider l'état de l'interface
settings-data-ui-cache-confirm-message = Effacer toutes les préférences d'interface enregistrées ? Cela réinitialisera les colonnes des tableaux, les filtres et les réglages de vues.
settings-data-ui-cache-cleared =
    { $count ->
        [one] { $count } réglage d'interface en cache effacé
        [many] { $count } réglages d'interface en cache effacés
       *[other] { $count } réglages d'interface en cache effacés
    }

settings-data-folder-label = Ouvrir le dossier de données
settings-data-folder-hint = Ouvre dans votre gestionnaire de fichiers le dossier contenant toutes les données de { -brand }.
settings-data-folder-open = Ouvrir le dossier
settings-data-folder-open-failed = Impossible d'ouvrir le dossier de données
