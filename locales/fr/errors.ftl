errors-with-details = { $message } : { $details }

# Configuration
errors-config-save-failed = Échec de l'enregistrement de la configuration

# Authentication
errors-auth-current-password-incorrect = Le mot de passe actuel est incorrect
errors-auth-current-password-required = Le mot de passe actuel est requis pour changer de mot de passe
errors-auth-password-too-short = Le mot de passe doit comporter au moins 4 caractères
errors-auth-password-too-long = Le mot de passe doit comporter au plus 128 caractères
errors-auth-hash-failed = Échec du hachage du mot de passe
errors-auth-not-enabled = L'authentification n'est pas activée
errors-auth-no-password = Aucun mot de passe n'a été configuré
errors-auth-password-incorrect = Mot de passe incorrect
errors-auth-required = Authentification requise. Veuillez vous connecter pour accéder à cet endpoint.
errors-auth-totp-invalid = Code 2FA invalide ou expiré
errors-auth-totp-verify-failed = Échec de la vérification du code 2FA
errors-auth-totp-password-required = Un mot de passe doit être défini avant d'activer la 2FA
errors-auth-totp-uri-failed = Échec de la génération de l'URI TOTP
errors-auth-totp-qr-failed = Échec de la génération du QR code
errors-auth-totp-secret-required = Le secret est requis
errors-auth-totp-save-failed = Échec de l'enregistrement de la configuration TOTP
errors-auth-totp-code-invalid = Code de vérification invalide. Veuillez vérifier le code et réessayer.
errors-auth-totp-code-verify-failed = Échec de la vérification du code

# Request security
errors-security-invalid-local-request = La requête doit provenir du tableau de bord local
errors-security-invalid-token = Jeton de sécurité invalide
errors-security-token-required = Jeton de sécurité requis. Cet endpoint n'est accessible que depuis { -brand }.

# Lockscreen
errors-lockscreen-no-password-set = Aucun mot de passe n'a été défini
errors-lockscreen-invalid-type = Type de mot de passe invalide. Doit être « pin4 », « pin6 » ou « text »
errors-lockscreen-invalid-format = Le mot de passe ne correspond pas au type sélectionné
errors-lockscreen-no-current-password = Aucun mot de passe n'est actuellement défini
errors-lockscreen-password-incorrect = Le mot de passe est incorrect
errors-lockscreen-enable-needs-password = Impossible d'activer l'écran de verrouillage sans avoir défini un mot de passe

# Account
errors-account-signin-failed = Échec de la connexion
errors-account-signin-refused = { $reason }
errors-account-signin-unavailable = La connexion est indisponible
errors-account-browser-open-failed = Impossible d'ouvrir votre navigateur. Ouvrez votre navigateur par défaut et réessayez.
errors-account-signup-open-failed = Impossible d'ouvrir la page d'inscription. Ouvrez screenerbot.io/signup dans votre navigateur.
errors-account-credentials-required = Saisissez votre adresse e-mail et votre mot de passe.
errors-account-signout-failed = Échec de la déconnexion
errors-account-gateway-update-failed = Impossible de mettre à jour le paramètre de la passerelle

# Localization
errors-i18n-locale-not-registered = La langue n'est pas enregistrée
errors-i18n-catalog-encode-failed = Impossible d'encoder le catalogue

# System
errors-system-paths-init-failed = Impossible de créer les dossiers de l'application
errors-system-open-data-failed = Impossible d'ouvrir le dossier de données
errors-system-url-empty = L'URL ne peut pas être vide
errors-system-open-url-failed = Impossible d'ouvrir l'URL

# Initialization
errors-initialization-required = L'initialisation du bot est requise avant d'accéder à cet endpoint. Veuillez terminer l'initialisation via l'interface web.
errors-initialization-explore-unavailable = Le Mode Explorer n'a aucun portefeuille connecté. Terminez la configuration pour utiliser les portefeuilles et le copy trading.
errors-initialization-onboarding-update-failed = Échec de la mise à jour de l'état d'intégration
errors-initialization-validation-required = La vérification des identifiants est requise avant d'enregistrer la configuration
errors-initialization-encrypt-failed = Échec du chiffrement de la clé privée

# Dashboard state
errors-ui-state-save-failed = Échec de l'enregistrement de l'état
errors-ui-state-clear-failed = Échec de la réinitialisation de l'état

# Agent control
errors-agent-config-failed = Échec de la configuration du contrôle par agent
errors-agent-invalid-parameters = Paramètres invalides
errors-agent-wallet-key-material = Le matériel de clé du portefeuille n'est pas accessible aux agents
errors-agent-store-failed = Échec du stockage du contrôle par agent
errors-agent-invalid-pairing-request = Demande d'appairage invalide
errors-agent-pairing-rejected = Identifiant d'appairage rejeté
errors-agent-disabled = Le contrôle par agent est désactivé
errors-agent-approval-not-pending = L'approbation n'est plus en attente
errors-agent-approval-not-found = Approbation introuvable
errors-agent-bridge-task-failed = Échec de la tâche de passerelle du contrôle par agent
errors-agent-task-failed = Échec de la tâche de contrôle par agent
errors-agent-pairing-not-found = Aucun appairage actif avec cet identifiant
errors-agent-permissions-update-failed = Échec de la mise à jour des permissions

# Connectivity
errors-connectivity-endpoint-not-found = Endpoint « { $endpoint } » introuvable ou non surveillé

# Copy trading
errors-copy-task-not-found = Tâche de copie introuvable
errors-copy-holding-not-found = Aucune détention simulée ouverte sur ce token
errors-copy-live-confirmation-required = L'armement du copy trading en réel nécessite une confirmation explicite
errors-copy-task-invalid = Tâche de copie invalide
errors-copy-request-rejected = Requête de copy trading rejetée
errors-copy-task-limit = Nombre maximal de tâches de copie actives atteint
errors-copy-watch-rejected = Impossible de surveiller la cible de copie
errors-copy-live-unavailable = Le copy trading en réel est indisponible
errors-copy-task-live = Mettez la tâche en réel en pause avant de la supprimer
errors-copy-task-owns-positions = La tâche de copie possède encore des positions ouvertes
errors-copy-request-failed = Échec de la requête de copy trading

# Strategies
errors-strategies-not-found = Stratégie introuvable
errors-strategies-invalid-type = Type de stratégie invalide. Doit être ENTRY ou EXIT
errors-strategies-list-failed = Échec de la récupération des stratégies
errors-strategies-get-failed = Échec de la récupération de la stratégie
errors-strategies-serialize-rules-failed = Échec de la sérialisation des règles
errors-strategies-invalid-rules-json = JSON des règles invalide
errors-strategies-already-exists = Une stratégie avec l'ID « { $id } » existe déjà
errors-strategies-validation-failed = Échec de la validation de la stratégie
errors-strategies-create-failed = Échec de la création de la stratégie
errors-strategies-update-failed = Échec de la mise à jour de la stratégie
errors-strategies-update-enabled-failed = Échec de la mise à jour de l'état d'activation de la stratégie
errors-strategies-delete-failed = Échec de la suppression de la stratégie
errors-strategies-deploy-failed = Échec du déploiement de la stratégie
errors-strategies-no-performance = Aucune donnée de performance disponible pour cette stratégie
errors-strategies-performance-failed = Échec de la récupération des statistiques de performance
errors-strategies-schemas-failed = Échec de la récupération des schémas de conditions
errors-strategies-evaluation-failed = Échec de l'évaluation de la stratégie

# Transactions
errors-transactions-own-wallet-unavailable = Le portefeuille principal n'est pas configuré
errors-transactions-invalid-subject = Le sujet de la transaction n'est pas une adresse Solana valide
errors-transactions-subject-not-watched = Le sujet de la transaction n'est pas un portefeuille surveillé
errors-transactions-watch-store-unavailable = Les portefeuilles surveillés ne sont pas disponibles

# Wallet
errors-wallet-unavailable = Le portefeuille principal n'est pas disponible
errors-wallet-changed = Le portefeuille principal a changé ; actualisez et réessayez
errors-wallet-qr-failed = Impossible de générer le QR code du portefeuille

# Updates
errors-updates-none-available = Aucune mise à jour à télécharger
errors-updates-version-changed = La mise à jour disponible a changé ; vérifiez de nouveau les mises à jour
errors-updates-check-failed = Échec de la vérification des mises à jour
errors-updates-download-failed = Impossible de démarrer le téléchargement de la mise à jour
errors-updates-history-unavailable = L'historique des versions est indisponible
errors-updates-apply-failed = Impossible d'appliquer la mise à jour
errors-updates-install-failed = Impossible d'ouvrir le programme d'installation de la mise à jour

# Telegram
errors-telegram-settings-update-failed = Échec de la mise à jour des paramètres
errors-telegram-disabled = { -telegram } n'est pas activé
errors-telegram-not-configured = Token du bot ou ID de conversation non configuré
errors-telegram-send-failed = Échec de l'envoi du message
errors-telegram-notifier-failed = Échec de la création du notificateur
errors-telegram-token-required = Le token du bot doit d'abord être configuré
errors-telegram-discovery-failed = Échec du démarrage de la découverte
errors-telegram-chat-select-failed = Échec de la sélection de la conversation

# Assistant chat
errors-chat-message-empty = Le message ne peut pas être vide
errors-chat-message-too-long = Le message dépasse la longueur maximale de 10 000 caractères
errors-chat-database-unavailable = Base de données du chat non initialisée
errors-chat-session-not-found = Session de chat { $id } introuvable
errors-chat-session-validate-failed = Échec de la validation de la session
errors-chat-engine-unavailable = Moteur de chat non initialisé
errors-chat-process-failed = Échec du traitement du message de chat
errors-chat-stream-serialize-failed = Échec de la sérialisation de l'événement de chat
errors-chat-sessions-list-failed = Échec de la récupération des sessions de chat
errors-chat-session-create-failed = Échec de la création de la session de chat
errors-chat-session-get-failed = Échec de la récupération de la session de chat
errors-chat-messages-get-failed = Échec de la récupération des messages du chat
errors-chat-session-delete-failed = Échec de la suppression de la session de chat
errors-chat-messages-load-failed = Échec de la récupération des messages
errors-chat-summarize-empty = Impossible de résumer une session de chat vide
errors-chat-provider-invalid = Fournisseur invalide : { $provider }
errors-chat-summary-save-failed = Échec de l'enregistrement du résumé
errors-chat-title-empty-session = Impossible de générer un titre pour une session de chat vide
errors-chat-no-user-message = Aucun message utilisateur trouvé dans la session
errors-chat-title-save-failed = Échec de la mise à jour du titre de la session
errors-chat-confirmation-save-failed = Échec de l'enregistrement de la réponse de confirmation
errors-chat-confirmation-failed = Échec du traitement de la confirmation
errors-chat-summary-failed = Échec de la génération du résumé

# Assistant automation
errors-automation-database-unavailable = Base de données non initialisée
errors-automation-tasks-list-failed = Échec de la récupération des tâches
errors-automation-name-empty = Le nom de la tâche ne peut pas être vide
errors-automation-instruction-empty = La consigne de la tâche ne peut pas être vide
errors-automation-schedule-type-invalid = schedule_type invalide. Doit être : interval, daily ou weekly
errors-automation-schedule-value-invalid = schedule_value invalide
errors-automation-task-create-failed = Échec de la création de la tâche
errors-automation-task-not-found = Tâche introuvable
errors-automation-task-get-failed = Échec de la récupération de la tâche
errors-automation-schedule-invalid = Planification invalide
errors-automation-tool-permissions-invalid = tool_permissions doit être « full » ou « readonly »
errors-automation-priority-invalid = priority doit être « low », « medium » ou « high »
errors-automation-task-update-failed = Échec de la mise à jour de la tâche
errors-automation-task-running-delete = Impossible de supprimer la tâche pendant son exécution
errors-automation-task-delete-failed = Échec de la suppression de la tâche
errors-automation-task-toggle-failed = Échec du basculement de la tâche
errors-automation-task-disabled = Impossible d'exécuter une tâche désactivée
errors-automation-task-already-running = La tâche est déjà en cours d'exécution
errors-automation-runs-list-failed = Échec de la récupération des exécutions
errors-automation-recent-runs-failed = Échec de la récupération des exécutions récentes
errors-automation-run-not-found = Exécution introuvable
errors-automation-run-get-failed = Échec de la récupération de l'exécution
errors-automation-stats-failed = Échec de la récupération des statistiques

# LLM providers
errors-llm-config-update-failed = Échec de la mise à jour de la configuration LLM
errors-llm-provider-unknown = Fournisseur inconnu : { $provider }
errors-llm-manager-unavailable = Gestionnaire LLM non initialisé
errors-llm-provider-disabled = Le fournisseur « { $provider } » n'est pas configuré ou est désactivé
errors-llm-provider-config-update-failed = Échec de la mise à jour de la configuration du fournisseur
errors-llm-provider-test-failed = Échec du test du fournisseur
errors-llm-provider-refused = { $reason }

# LLM analysis
errors-llm-analysis-config-update-failed = Échec de la mise à jour de la configuration de l'analyse
errors-llm-analysis-unavailable = Moteur d'analyse non initialisé
errors-llm-analysis-disabled = Les fonctions LLM sont désactivées. Activez d'abord [llm].
errors-llm-analysis-priority-invalid = Priorité invalide : « { $priority } ». Utilisez « high », « medium » ou « low ».
errors-llm-analysis-evaluation-failed = Échec de l'analyse du modèle
errors-llm-analysis-instructions-list-failed = Échec de la récupération des consignes
errors-llm-analysis-instruction-not-found = Consigne { $id } introuvable
errors-llm-analysis-instruction-get-failed = Échec de la récupération de la consigne
errors-llm-analysis-instruction-created-retrieve-failed = Échec de la récupération de la consigne créée
errors-llm-analysis-instruction-create-failed = Échec de la création de la consigne
errors-llm-analysis-instruction-updated-retrieve-failed = Échec de la récupération de la consigne mise à jour
errors-llm-analysis-instruction-update-failed = Échec de la mise à jour de la consigne
errors-llm-analysis-instruction-delete-failed = Échec de la suppression de la consigne
errors-llm-analysis-instructions-reorder-failed = Échec du réordonnancement des consignes
errors-llm-analysis-decisions-list-failed = Échec de la récupération de l'historique des décisions
errors-llm-analysis-decision-not-found = Décision { $id } introuvable
errors-llm-analysis-decision-get-failed = Échec de la récupération de la décision

# Wallets
errors-wallets-list-failed = Échec de la récupération des portefeuilles
errors-wallets-name-empty = Le nom du portefeuille ne peut pas être vide
errors-wallets-create-failed = Échec de la création du portefeuille
errors-wallets-key-empty = La clé privée ne peut pas être vide
errors-wallets-already-exists = Le portefeuille existe déjà
errors-wallets-key-invalid = Format de clé privée invalide
errors-wallets-import-failed = Échec de l'import du portefeuille
errors-wallets-summary-failed = Échec de la récupération du résumé des portefeuilles
errors-wallets-no-main-wallet = Aucun portefeuille principal configuré
errors-wallets-main-get-failed = Échec de la récupération du portefeuille principal
errors-wallets-not-found = Portefeuille introuvable
errors-wallets-get-failed = Échec de la récupération du portefeuille
errors-wallets-update-failed = Échec de la mise à jour du portefeuille
errors-wallets-delete-failed = Échec de la suppression du portefeuille
errors-wallets-export-failed = Échec de l'export du portefeuille
errors-wallets-set-main-failed = Échec de la définition du portefeuille principal
errors-wallets-archive-failed = Échec de l'archivage du portefeuille
errors-wallets-restore-failed = Échec de la restauration du portefeuille
errors-wallets-export-format-unsupported = Seul le format CSV est actuellement pris en charge
errors-wallets-export-confirmation-required = Vous devez confirmer en fournissant : « { $confirmation } »
errors-wallets-export-no-ids = Aucun ID de portefeuille fourni
errors-wallets-export-bulk-failed = Échec de l'export des portefeuilles
errors-wallets-export-no-match = Aucun portefeuille ne correspond aux ID fournis
errors-wallets-import-file-too-large = Le fichier dépasse la taille maximale de { $megabytes } Mo
errors-wallets-import-read-failed = Échec de la lecture du fichier importé
errors-wallets-import-no-file = Aucun fichier importé. Utilisez le champ « file » du formulaire multipart
errors-wallets-import-encoding-invalid = Le fichier CSV doit être encodé en UTF-8
errors-wallets-import-csv-parse-failed = Échec de l'analyse du fichier CSV
errors-wallets-import-excel-parse-failed = Échec de l'analyse du fichier Excel
errors-wallets-import-format-unsupported = Format de fichier non pris en charge. Utilisez .csv, .xlsx ou .xls
errors-wallets-import-file-empty = Le fichier ne contient aucune ligne de données
errors-wallets-import-existing-check-failed = Échec de la vérification des portefeuilles existants
errors-wallets-import-mapping-invalid = Colonnes obligatoires manquantes : { $columns }
errors-wallets-import-session-not-found = Session d'import introuvable ou expirée. Veuillez importer de nouveau le fichier
errors-wallets-import-no-valid-rows = Aucune ligne valide à importer

# Wallet watching
errors-wallet-watch-list-failed = Échec de la récupération des cibles de surveillance
errors-wallet-watch-address-empty = L'adresse ne peut pas être vide
errors-wallet-watch-add-failed = Échec de l'ajout de la cible de surveillance
errors-wallet-watch-remove-failed = Échec du retrait de la cible de surveillance
errors-wallet-watch-update-failed = Échec de la mise à jour de la cible de surveillance
errors-wallet-watch-budget-failed = Impossible de mettre à jour le budget de surveillance
errors-wallet-watch-resume-failed = Impossible de reprendre la surveillance
errors-wallet-watch-approval-failed = Impossible de mettre à jour l'approbation { -helius }
errors-wallet-watch-status-failed = Échec de la récupération du statut de surveillance

# Tools
errors-tools-wallet-failed = Échec de la récupération du portefeuille
errors-tools-wallet-address-failed = Échec de la récupération de l'adresse du portefeuille
errors-tools-accounts-scan-failed = Échec de l'analyse des comptes
errors-tools-token-accounts-scan-failed = Échec de l'analyse des comptes de token
errors-tools-token-accounts-get-failed = Échec de la récupération des comptes de token
errors-tools-cleanup-failed = Échec du nettoyage
errors-tools-cache-clear-failed = Échec du vidage du cache
errors-tools-no-tokens = Aucun token sélectionné pour le burn
errors-tools-burn-failed = Échec du burn des tokens
errors-tools-favorites-list-failed = Échec de la récupération des favoris
errors-tools-favorite-type-invalid = Type d'outil invalide. Doit être l'un des suivants : { $types }
errors-tools-favorite-add-failed = Échec de l'ajout du favori
errors-tools-favorite-not-found = Favori introuvable
errors-tools-favorite-update-failed = Échec de la mise à jour du favori
errors-tools-favorite-delete-failed = Échec de la suppression du favori
errors-tools-favorite-use-failed = Échec de la mise à jour du compteur d'utilisation
errors-tools-pool-search-failed = Échec de la recherche de pools pour le token { $mint }
errors-tools-watched-list-failed = Échec de la récupération des tokens surveillés
errors-tools-watched-add-failed = Échec de l'ajout du token surveillé
errors-tools-watched-delete-failed = Échec de la suppression du token surveillé
errors-tools-mint-invalid = Adresse de mint de token invalide
errors-tools-wallets-get-failed = Échec de la récupération des portefeuilles
errors-tools-balance-failed = Échec de la récupération du solde du portefeuille
errors-tools-session-active = Une autre opération multi-portefeuilles est déjà en cours
errors-tools-config-invalid = Configuration de l'outil invalide : { $reason }
errors-tools-config-rejected = Configuration de l'outil invalide
errors-tools-consolidate-failed = Échec de la consolidation des portefeuilles
errors-tools-ata-cleanup-failed = Échec du nettoyage des ATA
errors-tools-routers-unavailable = Les routeurs de swap ne sont pas encore prêts
errors-tools-router-disabled-chain-settings = { $router } est désactivé dans Paramètres > Chaînes
errors-tools-router-unknown = Routeur de swap inconnu « { $router } »
errors-tools-session-type-mismatch = La session est de type { $actual } et non { $expected }
errors-tools-session-not-found = Session introuvable
errors-tools-session-complete = La session est déjà terminée

# Configuration import and reload
errors-config-reload-failed = Échec du rechargement de la configuration
errors-config-reset-failed = Échec de la réinitialisation de la configuration
errors-config-disk-parse-failed = Échec de l'analyse de la configuration sur disque
errors-config-disk-read-failed = Échec de la lecture de la configuration sur disque
errors-config-update-failed = Échec de la mise à jour de la configuration
errors-config-import-not-object = La configuration doit être un objet JSON
errors-config-import-no-sections = Aucune section valide à importer
errors-config-import-validation-failed = Échec de la validation de la configuration. Aucune modification n'a été appliquée.
errors-config-import-commit-failed = Échec de l'application des modifications de configuration
errors-config-import-failed = Échec de l'import de la configuration

# Filtering
errors-filtering-analytics-failed = Échec de la récupération des analyses
errors-filtering-refresh-failed = Échec de la reconstruction de l'instantané du filtrage
errors-filtering-rejection-stats-failed = Échec de la récupération des statistiques de rejet
errors-filtering-rejected-tokens-failed = Échec de la récupération des tokens rejetés
errors-filtering-csv-header-failed = Échec de l'écriture de l'en-tête CSV
errors-filtering-csv-record-failed = Échec de l'écriture d'un enregistrement CSV
errors-filtering-csv-finalize-failed = Échec de la finalisation du CSV
errors-filtering-export-response-failed = Échec de la construction de la réponse

# OHLCV
errors-ohlcv-fetch-failed = Échec de la récupération des données OHLCV
errors-ohlcv-pools-failed = Échec de la récupération des pools
errors-ohlcv-gaps-failed = Échec de la récupération des lacunes
errors-ohlcv-refresh-failed = Échec de l'actualisation
errors-ohlcv-monitor-start-failed = Échec du démarrage de la surveillance
errors-ohlcv-monitor-stop-failed = Échec de l'arrêt de la surveillance
errors-ohlcv-activity-failed = Échec de l'enregistrement de l'activité
errors-ohlcv-list-failed = Échec de la récupération des tokens OHLCV
errors-ohlcv-delete-failed = Échec de la suppression des données du token
errors-ohlcv-clear-failed = Échec du vidage du cache OHLCV
errors-ohlcv-cleanup-failed = Échec du nettoyage des tokens inactifs

# Trader and manual trading
errors-trade-already-running = Le Trader est déjà en cours d'exécution
errors-trade-already-stopped = Le Trader est déjà arrêté
errors-trade-config-update-failed = Échec de la mise à jour de la configuration du Trader
errors-trade-trader-unavailable = Terminez la configuration du portefeuille et du RPC avant d'utiliser le trader automatique
errors-trade-force-stop-active = L'arrêt d'urgence est actif ; levez-le d'abord
errors-trade-template-not-found = Aucun modèle de Trader nommé { $template }
errors-trade-manual-force-stopped = Le trading manuel est désactivé tant que l'arrêt d'urgence est actif
errors-trade-core-services-not-ready = Les services essentiels ne sont pas prêts pour le trading : { $pending }
errors-trade-mint-invalid = Adresse de mint de token invalide { $mint }
errors-trade-blacklisted = Le token { $mint } est sur liste noire
errors-trade-slippage-invalid = Le slippage de { $slippage }{ " " }% doit être dans l'intervalle ]0, { $maximum }]
errors-trade-percentage-invalid = Le pourcentage de vente { $percentage } doit être dans l'intervalle ]0, 100]
errors-trade-record-failed = Impossible d'enregistrer le trade manuel
errors-trade-task-cancelled = Le trade manuel s'est arrêté avant de répondre car l'application se ferme ; la position en indique le résultat
errors-trade-no-open-position = Aucune position ouverte pour le token { $mint }
errors-trade-size-invalid = Taille de trade invalide { $amount } { -sol }
errors-trade-management-invalid = Gestion de position invalide { $management }
errors-trade-strategy-evaluation-failed = Échec de l'évaluation de la stratégie pour le token { $mint }
errors-trade-token-data-missing = Données du token indisponibles pour { $mint }
errors-trade-endpoints-unhealthy = Aucun endpoint sain disponible
errors-trade-dependency-failed = Échec de la dépendance { $dependency }
errors-trade-storage-failed = La requête de trade n'a pas pu être menée à bien
errors-trade-manual-failed = Échec du trade manuel
errors-trade-manual-refused = { $reason }
errors-trade-swap-too-large = Aucune route de swap n’a pu construire une transaction assez petite pour être envoyée
    .hint = La meilleure route nécessitait plus de comptes qu’une transaction ne peut en contenir : rien n’a été envoyé ni dépensé. Réessayez dans un instant pour obtenir une autre route, ou activez un autre routeur de swap.
errors-trade-wallet-not-configured = Portefeuille non configuré
errors-trade-amount-sol-invalid = amount_sol est requis pour un achat et doit être positif
errors-trade-no-tokens-in-wallet = Aucun token trouvé dans le portefeuille pour cette position. Le solde du token est de 0 ; la position ne peut pas être clôturée par swap.
errors-trade-percentage-range = Le pourcentage doit être dans l'intervalle ]0, 100]
errors-trade-amount-tokens-invalid = amount_tokens doit être positif
errors-trade-sell-amount-zero = Le montant de vente calculé est nul

# Swap quotes. The message is the dialog headline; `.hint` is what the user can do.
errors-trade-quote-registry-unavailable = Le routage des swaps n'est pas encore prêt
    .hint = Le service de swap est encore en cours de démarrage. Attendez que les services soient prêts, puis réessayez.
errors-trade-quote-no-routers-enabled = Aucun fournisseur de swap n'est activé
    .hint = Activez au moins un routeur de swap dans les paramètres du Trader, puis réessayez.
errors-trade-quote-not-tradable = Ce token n'est pas négociable pour le moment
    .hint = Aucune liquidité ni route de swap n'est disponible. Le token n'est peut-être pas lancé, est abandonné ou n'a pas de pool. Réessayez plus tard ou choisissez un autre token.
errors-trade-quote-no-route = Aucune route de swap disponible
    .hint = Aucun fournisseur n'a pu router ce trade pour le montant demandé. Essayez un montant plus faible, ou réessayez dans un instant.
errors-trade-quote-rate-limited = Les fournisseurs de swap limitent nos requêtes
    .hint = Les fournisseurs de swap limitent le débit des requêtes. Patientez quelques secondes et réessayez.
errors-trade-quote-timeout = La requête de cotation a expiré
    .hint = Les fournisseurs de swap n'ont pas répondu à temps. Vérifiez votre connexion et réessayez.
errors-trade-quote-router-rejected = La cotation a été refusée
    .hint = Un fournisseur a renvoyé une cotation qui a échoué à nos contrôles de sécurité et a été écartée. Réessayez pour en obtenir une nouvelle.
errors-trade-quote-not-offered-exact-out = Aucun routeur de swap activé ne cote un montant de sortie exact
    .hint = Les routeurs de swap activés ne calculent une transaction qu'à partir du montant dépensé. Saisissez le montant à dépenser ou activez un autre routeur de swap.
errors-trade-quote-not-offered-unsupported-venue = Aucun routeur de swap activé ne négocie dans le pool de ce token
    .hint = Ce token s'échange sur une plateforme que les routeurs de swap activés ne prennent pas encore en charge. Activez un autre routeur de swap, puis réessayez.
errors-trade-quote-unavailable = Impossible d'obtenir une cotation
    .hint = Les fournisseurs de swap n'ont pas pu coter ce trade. Réessayez dans un instant.

# Positions
errors-positions-not-found = Position introuvable
errors-positions-already-closed = La position est déjà clôturée
errors-positions-force-close-failed = Échec de la clôture forcée de la position
errors-positions-already-archived = La position est déjà archivée
errors-positions-unverified-entry-archive = L'achat de cette position n'est pas encore confirmé. Archivez-la après sa confirmation.
errors-positions-not-archived = La position n'est pas archivée
errors-positions-archive-failed = Échec de l'archivage de la position
errors-positions-unarchive-failed = Échec du désarchivage de la position
errors-positions-unarchive-duplicate-open = Ce token a plus d’une position ouverte ({ $positions }), et une seule peut être active. Fermez ou archivez d’abord les autres.
errors-positions-management-invalid = La gestion par copie nécessite une position issue d'une copie
errors-positions-management-failed = Échec de la mise à jour de la gestion de la position
errors-positions-delete-failed = Échec de la suppression de la position
errors-positions-bulk-delete-failed = Échec de la suppression des positions archivées
errors-positions-detail-failed = Échec du chargement des détails de la position
errors-positions-resolve-failed = Échec de la résolution de la position
errors-positions-wrapped-sol-activity = Le SOL wrappé n'a pas d'activité de token

# Tokens
errors-tokens-database-unavailable = Base de données des tokens indisponible
errors-tokens-blacklist-failed = Échec de l'ajout du token à la liste noire
errors-tokens-blacklist-internal = Erreur interne lors de l'ajout à la liste noire
errors-tokens-unblacklist-failed = Échec du retrait de la liste noire
errors-tokens-unblacklist-internal = Erreur interne lors du retrait de la liste noire
errors-tokens-blacklist-status-failed = Échec de la vérification du statut de liste noire
errors-tokens-blacklist-status-internal = Erreur interne lors de la vérification du statut de liste noire
errors-tokens-favorites-fetch-failed = Échec de la récupération des favoris
errors-tokens-favorite-add-failed = Échec de l'ajout du favori
errors-tokens-favorite-remove-failed = Échec du retrait du favori
errors-tokens-favorite-update-failed = Échec de la mise à jour du favori
errors-tokens-detail-not-found = Token introuvable dans la base de données ou les sources externes
errors-tokens-fetch-failed = Échec de la récupération du token
errors-tokens-refresh-all-failed = Toutes les sources de données ont échoué
errors-tokens-refresh-failed = Échec de l'actualisation du token
errors-tokens-search-query-required = Le paramètre de recherche « q » est requis
errors-tokens-search-failed = Échec de la recherche de tokens

# Actions and services
errors-actions-not-found = Action { $id } introuvable
errors-services-not-found = Service « { $name } » introuvable
