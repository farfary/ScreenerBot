## Categories

hints-category-tokens = Tokens
hints-category-positions = Positions
hints-category-filtering = Filtrage
hints-category-trader = Trader automatique
hints-category-services = Services
hints-category-wallet = Portefeuille
hints-category-wallets = Portefeuilles
hints-category-tools = Outils
hints-category-config = Config
hints-category-config-telegram = Telegram
hints-category-token-details = Détails du token
hints-category-ui = Interface

## tokens

hints-tokens-pool-service-title = Tokens du service de pools
hints-tokens-pool-service-content =
    Les tokens affichés ici ont :

    • **Passé tous les critères de filtrage** — liquidité, volume, ancienneté et contrôles de sécurité
    • **Des pools SOL valides** — pris en charge par nos décodeurs DEX (Raydium, Orca, Meteora, etc.)
    • **Un calcul de prix réussi** — prix calculés directement à partir des réserves des pools on-chain

    C'est la liste de tokens la plus fiable pour trader, car les prix proviennent des données réelles des pools et non d'API externes.

    Cliquez sur un token pour consulter ses informations détaillées et gérer son statut de liste noire.
hints-tokens-no-market-title = Sans données de marché
hints-tokens-no-market-content =
    Tokens découverts on-chain mais sans données de marché de DexScreener ou GeckoTerminal.

    Raisons fréquentes :
    • **Tokens très récents** — pas encore indexés par les agrégateurs
    • **Faible volume d'échanges** — sous les seuils des agrégateurs
    • **Paires non listées** — échangées sur des DEX non suivis par les agrégateurs

    Ces tokens peuvent avoir des pools valides et être tradés, mais n'ont pas de métriques de marché externes.
hints-tokens-all-title = Tous les tokens
hints-tokens-all-content =
    Base de données complète des tokens découverts, quel que soit leur statut de filtrage.

    Comprend :
    • Les tokens qui ont passé le filtrage
    • Les tokens rejetés
    • Les tokens sans données de marché
    • Les tokens sur liste noire

    Utilisez cette vue pour vos recherches ou pour retrouver des tokens qui ont pu être filtrés.
hints-tokens-passed-title = Filtrage validé
hints-tokens-passed-content =
    Tokens qui ont passé tous les critères de filtrage actifs.

    Les contrôles de filtrage incluent :
    • **Liquidité** — seuil minimal de liquidité en SOL
    • **Volume** — exigences de volume d'échanges sur 24 h
    • **Ancienneté du token** — durée minimale depuis la création
    • **Sécurité** — limites du score de risque Rugcheck
    • **Capitalisation** — filtres FDV/capitalisation facultatifs

    Configurez les filtres dans la page **Filtrage**.
hints-tokens-rejected-title = Tokens rejetés
hints-tokens-rejected-content =
    Tokens qui ont échoué à un ou plusieurs critères de filtrage.

    Chaque token affiche le motif précis du rejet :
    • Quel filtre a échoué
    • La valeur réelle par rapport au seuil requis
    • Le moment du contrôle

    Examinez les tokens rejetés pour affiner vos paramètres de filtrage.
hints-tokens-blacklisted-title = Tokens sur liste noire
hints-tokens-blacklisted-content =
    Tokens exclus définitivement du trading.

    Motifs de mise sur liste noire :
    • **Liste noire manuelle** — tokens que vous avez explicitement bloqués
    • **Risques de sécurité** — indicateurs de rug pull détectés
    • **Seuil de perte** — limites de perte configurées dépassées
    • **Transactions échouées** — échecs répétés de swap

    Les tokens sur liste noire n'apparaissent jamais dans les listes validées et ne sont jamais pris en compte par le trading automatique.
hints-tokens-positions-title = Tokens en position
hints-tokens-positions-content =
    Tokens actuellement détenus dans des positions ouvertes.

    Affiche les données en temps réel de vos détentions actives :
    • Prix actuel issu des réserves des pools
    • P&L latent
    • Taille de la position et prix d'entrée
    • Durée de détention

    Cliquez sur un token pour gérer la position en détail.
hints-tokens-recent-title = Découverts récemment
hints-tokens-recent-content =
    Tokens nouvellement découverts, classés par date de découverte.

    Utile pour :
    • Repérer les nouveaux lancements de tokens
    • Surveiller la liquidité fraîche
    • Saisir des opportunités d'entrée précoce

    Remarque : les nouveaux tokens peuvent manquer de données de marché complètes au départ.
hints-tokens-ohlcv-title = Gestion des données OHLCV
hints-tokens-ohlcv-content =
    Consultez et gérez les données OHLCV (bougies) stockées pour les tokens.

    Affiche :
    • **Nombre de bougies** — total des points de données stockés
    • **Progression du rattrapage** — état d'avancement par période
    • **Étendue des données** — couverture temporelle en heures
    • **Nombre de pools** — pools de liquidité suivis
    • **Statut** — surveillance active ou inactive

    Actions :
    • **Supprimer** — supprime toutes les données OHLCV d'un token
    • **Nettoyer** — supprime en masse les données des tokens inactifs

    Les données OHLCV sont conservées définitivement et jamais supprimées automatiquement.

## positions

hints-positions-overview-title = Aperçu des positions
hints-positions-overview-content =
    Vos détentions de tokens et positions de trading actuelles.

    Indicateurs clés :
    • **Prix d'entrée** — prix moyen payé (renforts DCA inclus)
    • **Prix actuel** — prix en direct issu des réserves des pools
    • **P&L** — profit/perte latent en SOL et en %
    • **Taille** — quantité totale de tokens détenus

    Cliquez sur une position pour accéder aux options de gestion détaillées.
hints-positions-dca-title = DCA (investissement programmé)
hints-positions-dca-content =
    Le DCA permet de renforcer des positions existantes à différents prix.

    Lorsqu'un DCA se déclenche :
    • Des tokens supplémentaires sont achetés
    • Le prix d'entrée est recalculé en moyenne pondérée
    • La taille de la position augmente
    • Le compteur d'entrées s'incrémente

    Configurez les règles de DCA dans les paramètres du **Trader automatique**.
hints-positions-partial-exit-title = Sortie partielle
hints-positions-partial-exit-content =
    Vendez une partie de votre position tout en conservant le reste.

    Avantages :
    • Sécuriser une partie des profits tout en restant exposé
    • Réduire la taille de la position sans la clôturer entièrement
    • Mettre en place des paliers de take profit

    Chaque sortie partielle est enregistrée séparément pour un suivi précis du P&L.
hints-positions-management-title = Gestion des positions
hints-positions-management-content =
    La gestion définit quelles automatisations peuvent agir sur une position :

    • Trader automatique : sorties de sécurité, sorties de stratégie et DCA automatique
    • Utilisateur seul : aucune action automatique
    • Tâche de copie : sorties de sécurité et ventes de copie
    • Hybride : sorties de sécurité, sorties de stratégie et ventes de copie

    Vous vendez ou renforcez vous-même. Les achats manuels sont par défaut en gestion manuelle afin que le bot ne puisse pas vendre un token que vous avez acheté volontairement. Désactivez-la pour rendre la position au trader automatique.

## filtering

hints-filtering-overview-title = Filtrage des tokens
hints-filtering-overview-content =
    Le filtrage détermine quels tokens sont éligibles au trading.

    Les tokens doivent passer **tous les critères activés** pour apparaître dans la liste validée :
    • Métriques DexScreener (liquidité, volume, etc.)
    • Métriques GeckoTerminal (capitalisation, FDV)
    • Analyse de sécurité Rugcheck
    • Filtres méta (ancienneté du token, etc.)

    Les critères désactivés sont totalement ignorés.
hints-filtering-dexscreener-title = Filtres DexScreener
hints-filtering-dexscreener-content =
    Filtres basés sur les données de marché DexScreener :

    • **Liquidité** — liquidité minimale en USD dans les pools
    • **Volume 24 h** — volume d'échanges minimal
    • **Transactions** — seuils d'activité (achats/ventes)
    • **Variation de prix** — filtres de volatilité

    Les données DexScreener sont mises à jour toutes les quelques minutes.
hints-filtering-geckoterminal-title = Filtres GeckoTerminal
hints-filtering-geckoterminal-content =
    Filtres basés sur les données de marché GeckoTerminal :

    • **Capitalisation** — capitalisation boursière minimale
    • **FDV** — limites de valorisation totalement diluée
    • **Ratio de réserves** — indicateurs de santé du pool

    GeckoTerminal dispose souvent de données pour les tokens plus récents.
hints-filtering-rugcheck-title = Filtres de sécurité
hints-filtering-rugcheck-content =
    Analyse de sécurité de Rugcheck.xyz :

    • **Score de risque** — note de risque globale (0-100)
    • **Autorité de mint** — de nouveaux tokens peuvent-ils être créés ?
    • **Autorité de gel** — les transferts peuvent-ils être gelés ?
    • **Principaux holders** — risque de concentration

    Un score de risque élevé indique davantage de signaux d'alerte potentiels.
hints-filtering-meta-title = Filtres méta
hints-filtering-meta-content =
    Critères de filtrage supplémentaires :

    • **Ancienneté du token** — durée minimale depuis la création du token
    • **Ancienneté du pool** — durée minimale depuis la création du pool
    • **Site web requis** — exiger des liens sociaux/site web
    • **Réseaux sociaux requis** — exiger Twitter/Telegram

    Ils aident à écarter les tokens très récents ou suspects.

## trader

hints-trader-overview-title = Trader automatique
hints-trader-overview-content =
    Moteur de trading automatisé qui surveille les tokens et exécute les trades.

    Composants :
    • **Moniteur d'entrée** — guette les opportunités d'achat
    • **Moniteur de sortie** — gère les ventes et les take profits
    • **Moniteur DCA** — gère les renforts de position
    • **Contrôles de risque** — limites de perte et garde-fous

    Démarrez ou arrêtez le trading depuis le panneau de contrôle.
hints-trader-entry-title = Moniteur d'entrée
hints-trader-entry-content =
    Surveille les tokens filtrés à la recherche de signaux d'entrée.

    L'évaluation d'entrée vérifie que :
    • Le token passe le filtrage actuel
    • Il n'est pas déjà en position
    • Il n'est pas sur liste noire
    • Les limites de positions ne sont pas dépassées
    • Les conditions de la stratégie sont remplies (si configurée)

    Configurez la taille d'entrée et les limites dans Config.
hints-trader-exit-title = Moniteur de sortie
hints-trader-exit-content =
    Surveille les positions ouvertes à la recherche de signaux de sortie.

    Déclencheurs de sortie :
    • **Take profit** — objectif de prix atteint
    • **Stop loss** — perte maximale dépassée
    • **Trailing stop** — le prix a reculé depuis son sommet
    • **Sortie de stratégie** — conditions personnalisées remplies
    • **Basée sur le temps** — durée de détention maximale

    Configurez les seuils dans Config.

## services

hints-services-overview-title = Services système
hints-services-overview-content =
    Services d'arrière-plan qui font fonctionner { -brand }.

    États des services :
    • **En cours** (vert) — fonctionnement normal
    • **Démarrage** (jaune) — initialisation
    • **Arrêté** (rouge) — ne fonctionne pas
    • **Erreur** (avertissement) — en échec, redémarrage automatique possible

    Les services ont des dépendances et démarrent dans l'ordre.
hints-services-health-title = Santé des services
hints-services-health-content =
    Les indicateurs de santé montrent l'état des services :

    • **Disponibilité** — temps écoulé depuis le dernier démarrage
    • **Tâches** — opérations d'arrière-plan actives
    • **Erreurs** — nombre d'erreurs récentes
    • **Métriques** — données de performance (si disponibles)

    Les services critiques affectent la capacité de trading.

## wallet

hints-wallet-overview-title = Aperçu du portefeuille
hints-wallet-overview-content =
    État de votre portefeuille Solana connecté.

    Affiche :
    • **Solde en SOL** — SOL natif pour les frais et le trading
    • **Détention de tokens** — tokens SPL avec leur valeur
    • **Variation 24 h** — évolution de la valeur du portefeuille
    • **Historique** — instantanés du solde dans le temps

    Les soldes sont actualisés chaque minute.
hints-wallet-tokens-title = Soldes de tokens
hints-wallet-tokens-content =
    Tokens SPL détenus dans votre portefeuille.

    Affiche :
    • Symbole et nom du token
    • Quantité détenue
    • Valeur actuelle en SOL/USD
    • Prix issu du pool ou des données de marché

    Les comptes de token vides peuvent être nettoyés dans les Paramètres.

## wallets

hints-wallets-main-title = Portefeuille principal
hints-wallets-main-content =
    Le portefeuille principal utilisé pour toutes les opérations de trading.

    • **Trading automatique** — les trades d'entrée/sortie sont exécutés depuis ce portefeuille
    • **Affichage du solde** — visible dans l'en-tête et le tableau de bord
    • **Détention de tokens** — tokens SPL détenus par ce portefeuille

    Changez le portefeuille principal en sélectionnant « Définir comme principal » sur n'importe quel portefeuille secondaire.
hints-wallets-secondary-title = Portefeuilles secondaires
hints-wallets-secondary-content =
    Portefeuilles supplémentaires pour les opérations multi-portefeuilles.

    • **Trading multi-portefeuilles** — coordonnez achats/ventes entre portefeuilles
    • **Séparation du portefeuille** — organisez par stratégie ou par objectif
    • **Soldes indépendants** — chaque portefeuille a ses propres SOL/tokens

    Les portefeuilles secondaires ne sont pas utilisés par le trading automatique, sauf configuration explicite.

## tools

hints-tools-wallet-cleanup-title = Outil de nettoyage du portefeuille
hints-tools-wallet-cleanup-content =
    { "*" }*Récupérez du SOL des comptes de token vides**

    { "*" }*Que sont les ATA ?**
    Les comptes de token associés (ATA) sont des comptes Solana qui contiennent vos tokens. Chaque token avec lequel vous interagissez crée un ATA qui nécessite environ 0.002 SOL de rent.

    { "*" }*Pourquoi nettoyer les ATA vides ?**
    • Récupérer le rent (environ 0.002 SOL par ATA)
    • Les traders actifs peuvent accumuler des centaines d'ATA vides
    • 100 ATA vides = environ 0.2 SOL récupérables

    { "*" }*Fonctionnement :**
    • Analyse votre portefeuille à la recherche d'ATA à solde nul
    • Affiche le montant total de SOL récupérable
    • Ferme les comptes vides pour récupérer le rent

    { "*" }*Nettoyage automatique :**
    Lorsqu'il est activé, analyse et ferme automatiquement les ATA vides toutes les 5 minutes en arrière-plan.

    { "*" }*Important :**
    • Ne ferme que les comptes dont le solde est exactement 0
    • Les fermetures échouées sont mises en cache pour éviter les nouvelles tentatives répétées
    • Les gros portefeuilles peuvent nécessiter plusieurs passes de nettoyage
hints-tools-burn-tokens-title = Outil de burn de tokens
hints-tools-burn-tokens-content =
    { "*" }*Détruire définitivement des tokens**

    Brûler des tokens les retire définitivement de votre portefeuille et de la circulation.

    { "*" }*Ce qui se passe lors d'un burn :**
    • Les tokens sont envoyés à une adresse de burn (irrécupérable)
    • Le solde du token devient nul
    • L'ATA peut ensuite être fermé via le nettoyage du portefeuille pour récupérer environ 0.002 SOL de rent

    { "*" }*Catégories de tokens :**
    • **Positions ouvertes** - Burn impossible (trades actifs)
    • **Positions clôturées** - Reliquats de trades passés
    • **Avec valeur** - Tokens avec liquidité (envisagez plutôt de vendre)
    • **Liquidité nulle** - Tokens sans valeur (burn sans risque)

    { "*" }*Avertissement :** Cette action est **irréversible**. Les tokens brûlés ne peuvent être récupérés en aucun cas.

    { "*" }*Après le burn :** Lancez le nettoyage du portefeuille pour fermer les ATA vides et récupérer le rent en SOL.
hints-tools-wallet-generator-title = Outil de génération de portefeuilles
hints-tools-wallet-generator-content =
    { "*" }*Générez de nouvelles paires de clés Solana**

    Créez de nouveaux portefeuilles en toute sécurité sur votre appareil.

    { "*" }*Fonctions :**
    • Génère des paires de clés cryptographiquement sûres
    • Préfixe d'adresse personnalisé facultatif (p. ex. « SOL... »)
    • Export en base58 ou en tableau JSON

    { "*" }*Sécurité :**
    • Les clés sont générées localement
    • Jamais transmises sur le réseau
    • Sauvegardez toujours vos clés en lieu sûr
hints-tools-multi-buy-title = Outil de multi-achat
hints-tools-multi-buy-content =
    { "*" }*Coordonnez des achats sur plusieurs portefeuilles**

    Exécutez des ordres d'achat sur plusieurs sous-portefeuilles avec des montants aléatoires pour simuler une activité d'achat organique.

    { "*" }*Fonctionnement :**
    1. Crée ou réutilise des sous-portefeuilles existants
    2. Distribue du SOL du portefeuille principal vers les sous-portefeuilles
    3. Exécute les ordres d'achat avec des montants et des délais aléatoires
    4. Chaque portefeuille achète indépendamment avec des signatures uniques

    { "*" }*Paramètres des portefeuilles :**
    • **Nombre de portefeuilles** — nombre de sous-portefeuilles à utiliser (2-10)
    • **Réserve de SOL** — SOL réservé par portefeuille pour les frais (~0.015)

    { "*" }*Paramètres des montants :**
    • **SOL min./max.** — plage des montants d'achat par portefeuille
    • **Limite totale** — plafond facultatif du SOL total à dépenser

    { "*" }*Paramètres d'exécution :**
    • **Délai** — délai aléatoire entre les transactions
    • **Concurrence** — exécution en parallèle (1 = séquentielle)
    • **Slippage** — slippage maximal acceptable
    • **Routeur** — routage des swaps (Auto, Jupiter, Raydium)

    { "*" }*Important :**
    • Nécessite suffisamment de SOL dans le portefeuille principal
    • Les achats échoués sont consignés mais n'interrompent pas la session
    • Les sous-portefeuilles peuvent être réutilisés d'une session à l'autre
hints-tools-multi-sell-title = Outil de multi-vente
hints-tools-multi-sell-content =
    { "*" }*Coordonnez des ventes sur plusieurs portefeuilles**

    Vendez un token depuis tous les sous-portefeuilles qui le détiennent, avec consolidation automatique du SOL.

    { "*" }*Fonctionnement :**
    1. Analyse les soldes de tokens des sous-portefeuilles
    2. Recharge éventuellement les portefeuilles à faible SOL pour les frais
    3. Exécute les ordres de vente avec un pourcentage configurable
    4. Consolide le produit vers le portefeuille principal

    { "*" }*Paramètres de vente :**
    • **% de vente** — pourcentage de tokens à vendre (100 % par défaut)
    • **SOL min. pour les frais** — SOL minimal requis pour la transaction
    • **Recharge auto** — transfère du SOL depuis le portefeuille principal si nécessaire

    { "*" }*Actions après la vente :**
    • **Consolider le SOL** — transfère tout le SOL vers le portefeuille principal
    • **Fermer les ATA** — ferme les comptes de token pour récupérer le rent (~0.002 SOL chacun)

    { "*" }*Paramètres d'exécution :**
    • **Délai** — délai aléatoire entre les transactions
    • **Concurrence** — exécution en parallèle
    • **Slippage** — slippage maximal acceptable
    • **Routeur** — préférence de routage des swaps

    { "*" }*Conseils :**
    • L'aperçu affiche tous les portefeuilles détenant le token
    • Désélectionnez les portefeuilles dont vous ne voulez pas vendre
    • La consolidation a lieu une fois toutes les ventes terminées
hints-tools-trade-watcher-title = Outil de surveillance de trades
hints-tools-trade-watcher-content =
    { "*" }*Surveillez les trades et déclenchez des actions automatiques**

    Observez l'activité d'échange d'un token et réagissez automatiquement lorsque des trades ont lieu.

    { "*" }*Types de surveillance :**
    • **Achat sur vente** — achète automatiquement quand quelqu'un vend (saisir les creux)
    • **Vente sur achat** — vend automatiquement quand quelqu'un achète (suivre le marché)
    • **Notification seule** — recevez des alertes sans action

    { "*" }*Fonctionnement :**
    1. Saisissez une adresse de mint de token
    2. Cliquez sur « Rechercher des pools » pour trouver les pools de liquidité disponibles
    3. Sélectionnez un pool à surveiller (requis pour les actions d'achat/vente)
    4. Définissez le montant de déclenchement (taille de trade minimale pour réagir)
    5. Définissez le montant de l'action (combien de SOL acheter/vendre)
    6. Lancez la surveillance

    { "*" }*Prérequis :**
    • Adresse de mint de token valide
    • Sélection d'un pool (pour les actions d'achat/vente)
    • Solde de SOL suffisant pour les montants d'action

    { "*" }*Intégration Telegram :**
    Configurez Telegram dans Config → Telegram pour recevoir des notifications instantanées lorsque les surveillances se déclenchent.
hints-tools-wallet-consolidation-title = Outil de consolidation des portefeuilles
hints-tools-wallet-consolidation-content =
    { "*" }*Gérez et consolidez les fonds des sous-portefeuilles**

    Consultez tous les sous-portefeuilles et consolidez le SOL, les tokens et le rent des ATA vers votre portefeuille principal.

    { "*" }*Le résumé affiche :**
    • **Sous-portefeuilles** — nombre total de sous-portefeuilles créés
    • **SOL total** — solde SOL cumulé de tous les sous-portefeuilles
    • **Types de tokens** — nombre de tokens différents détenus
    • **Rent récupérable** — SOL bloqué dans des ATA vides

    { "*" }*Actions :**
    • **Transférer le SOL** — déplace tout le SOL des portefeuilles sélectionnés vers le principal
    • **Transférer les tokens** — déplace tous les tokens vers le portefeuille principal
    • **Nettoyer les ATA** — ferme les comptes de token vides pour récupérer le rent

    { "*" }*Informations du tableau :**
    • Case à cocher pour sélectionner les portefeuilles pour les opérations groupées
    • Nom, adresse, solde SOL, nombre de tokens, ATA vides
    • Les portefeuilles vides sont atténués pour les repérer facilement

    { "*" }*Conseils :**
    • À utiliser après une multi-vente pour récupérer le SOL restant
    • Nettoyez régulièrement les ATA pour récupérer le rent
    • Les portefeuilles vides peuvent être réutilisés pour de futures opérations

## config

hints-config-overview-title = Configuration
hints-config-overview-content =
    Paramètres globaux de { -brand }.

    Catégories :
    • **Trader** — règles d'entrée/sortie, dimensionnement des positions
    • **Filtrage** — seuils des filtres de tokens
    • **Swaps** — paramètres de routage et de slippage
    • **RPC** — configuration des nœuds
    • **Services** — paramètres des services d'arrière-plan

    Les modifications prennent effet immédiatement (rechargement à chaud).
hints-config-telegram-title = Notifications Telegram
hints-config-telegram-content =
    { "*" }*Recevez des alertes de trading instantanées via Telegram**

    Soyez informé des trades, des positions et des événements importants directement dans Telegram.

    { "*" }*Étapes de configuration :**

    1. **Créer un bot :**
       • Ouvrez Telegram et écrivez à @BotFather
       • Envoyez /newbot et suivez les instructions
       • Copiez le token du bot (de la forme : 123456:ABC-DEF...)

    2. **Obtenir votre ID de conversation :**
       • Écrivez à @userinfobot ou @getidsbot
       • Copiez l'ID numérique qu'il renvoie

    3. **Configurer dans { -brand } :**
       • Activez l'interrupteur des notifications
       • Collez le token du bot et l'ID de conversation
       • Cliquez sur « Tester la connexion » pour vérifier

    { "*" }*Ce que vous recevrez :**
    • Confirmations d'exécution des trades
    • Mises à jour des positions (entrée/sortie)
    • Alertes de surveillance de trades
    • Notifications d'erreur

    { "*" }*Confidentialité :**
    Les messages sont envoyés directement de { -brand } à votre bot Telegram — aucun serveur tiers n'est impliqué.
hints-config-telegram-password-title = Mot de passe d'authentification du bot
hints-config-telegram-password-content =
    { "*" }*Sécurisez votre bot Telegram par mot de passe**

    Lorsque vous interagissez avec votre bot Telegram { -brand }, vous devrez vous authentifier avec ce mot de passe avant d'exécuter des commandes sensibles.

    { "*" }*Pourquoi définir un mot de passe ?**
    • Empêche les utilisateurs non autorisés de contrôler votre bot
    • Requis pour exécuter des commandes de trading via Telegram
    • Doit comporter au moins 8 caractères

    { "*" }*Fonctionnement :**
    1. Définissez un mot de passe ici, dans le tableau de bord
    2. Lorsque vous envoyez une commande de trading à votre bot, il demande une authentification
    3. Saisissez votre mot de passe pour vérifier votre identité
    4. Activez éventuellement la 2FA pour plus de sécurité

    { "*" }*Remarque :** Le mot de passe est stocké sous forme de hachage SHA256 sécurisé — nous ne stockons jamais le texte en clair.
hints-config-telegram-totp-title = Authentification à deux facteurs (2FA)
hints-config-telegram-totp-content =
    { "*" }*Ajoutez une couche de sécurité supplémentaire avec la 2FA TOTP**

    L'authentification à deux facteurs utilise des mots de passe à usage unique basés sur le temps (TOTP) générés par des applications comme Google Authenticator, Authy ou 1Password.

    { "*" }*Pourquoi activer la 2FA ?**
    • Même si quelqu'un connaît votre mot de passe, il ne peut pas accéder à votre bot sans le code
    • Les codes à 6 chiffres changent toutes les 30 secondes
    • Fonctionne hors ligne une fois configurée

    { "*" }*Procédure de configuration :**
    1. Cliquez sur « Activer la 2FA » et saisissez votre mot de passe
    2. Scannez le QR code avec votre application d'authentification
    3. Saisissez le code à 6 chiffres pour valider la configuration

    { "*" }*Applications compatibles :**
    • Google Authenticator
    • Authy
    • 1Password
    • Microsoft Authenticator
    • Toute application compatible TOTP

    { "*" }*Important :** Conservez votre clé secrète en lieu sûr. Si vous perdez l'accès à votre application d'authentification, vous devrez désactiver la 2FA depuis ce tableau de bord.

## token_details

hints-token-details-chart-title = Graphique des prix (OHLCV)
hints-token-details-chart-content =
    { "*" }*Important :** Ce graphique affiche des **données OHLCV en cache** pour l'évaluation des stratégies, *et non* le prix d'exécution en direct.

    { "*" }*Pourquoi des données en cache ?**
    • **Objectif :** utilisées par les stratégies automatisées et les indicateurs (p. ex. RSI, MM).
    • **Fraîcheur :** les mises à jour dépendent de la priorité du token (positions ouvertes = mises à jour plus rapides).
    • **Source :** agrégées depuis DexScreener/GeckoTerminal, et non directement depuis le RPC on-chain.

    { "*" }*La réalité du prix sur les DEX :**
    En DeFi, les tokens s'échangent sur **plusieurs pools** (Raydium, Orca, Meteora). Chaque pool a un prix propre selon la profondeur de liquidité et les trades récents.
    • **Prix du graphique :** une moyenne/agrégation entre les marchés.
    • **Prix du swap :** le taux précis obtenu via la meilleure route au moment exact du trade.

    { "*" }Attendez-vous à de petits écarts entre ce graphique et votre prix d'exécution final.*

    { "*" }*Statut :** « En attente de données » signifie que des workers d'arrière-plan récupèrent de nouvelles bougies.
hints-token-details-token-info-title = Informations sur le token
hints-token-details-token-info-content =
    Métadonnées de base du token issues de sources on-chain et de marché.

        • **Mint** — adresse unique du token sur Solana (cliquez pour copier)
        • **Décimales** — précision du token (généralement 6-9)
        • **Âge** — temps écoulé depuis la création du pool/token principal
        • **DEX** — place de marché principale de ce token
        • **Holders** — portefeuilles uniques détenant le token
        • **Top 10** — % détenu par les 10 principaux portefeuilles

        Un nombre de holders élevé et une concentration plus faible indiquent généralement une répartition plus saine.
hints-token-details-liquidity-title = Liquidité et données de marché
hints-token-details-liquidity-content =
    Métriques de marché du pool SOL à la plus forte liquidité.

        • **FDV** — prix × offre totale (prix de l'agrégateur)
        • **Liquidité** — valeur en USD des réserves du pool
        • **SOL du pool** / **Token du pool** — réserves en direct qui déterminent le prix du pool

        { "*" }*Pourquoi c'est important :**
        • Plus de liquidité = moins de slippage
        • Les pools peu profonds peuvent bouger sur de petits trades
        • Les réserves du pool déterminent directement le prix d'exécution du swap

        Les données sont actualisées périodiquement depuis DexScreener/GeckoTerminal, complétées par des lectures de pools on-chain.
hints-token-details-market-pulse-title = Pouls du marché
hints-token-details-market-pulse-content =
    Le mouvement des prix et le volume d'échanges en USD partagent la même chronologie **5 min / 1 h / 6 h / 24 h**, ce qui permet de comparer directement le momentum et la participation.

    { "*" }*Interprétation :**
    • **Prix** — variation en pourcentage issue des agrégateurs, et non le prix d'exécution du pool en direct.
    • **Volume élevé** — intérêt plus fort, découverte de prix plus efficace et sorties plus faciles.
    • **Volume faible** — slippage plus important, spreads plus larges et sorties importantes plus difficiles.
    • **Volume élevé + faible liquidité** — volatilité et risque d'exécution accrus.

    Les données de marché sont agrégées sur les principaux DEX via DexScreener/GeckoTerminal ; la variation de prix peut donc différer du prix actuel du pool on-chain.
hints-token-details-activity-title = Activité des transactions (nombres)
hints-token-details-activity-content =
    Analyse le **nombre de trades** (achats vs ventes) sur plusieurs périodes. Cela révèle l'intention des traders, quelle que soit la taille des trades.

    { "*" }*Détail des métriques :**
    • **Périodes :** fenêtres de 5 min, 1 h, 6 h, 24 h.
    • **Barres :** ratio visuel entre le nombre d'achats (vert) et de ventes (rouge).
    • **Rythme :** trades par minute (p. ex. « 12.5/min »). Un rythme élevé = activité virale.
    • **Nombres :** nombre exact d'achats/ventes et leur part en pourcentage.

    { "*" }*Métriques de synthèse :**
    • **% d'achats 24 h :** >50 % est haussier (plus d'acheteurs), { "<" }50 % est baissier (plus de vendeurs).
    • **Flux net :** total des achats moins les ventes. Positif = accumulation.
    • **Pic 5 min :** à quel point les échanges sont plus rapides *en ce moment* que la moyenne sur 1 h.
      • **>1.0x :** intérêt en accélération.
      • **>3.0x :** percée virale ou épisode de panique.
      • **{ "<" }1.0x :** refroidissement.

    { "*" }*Conseil de stratégie :** Un « % d'achats » élevé associé à un « facteur de pic » élevé signale souvent une bonne entrée sur cassure.
hints-token-details-security-title = Analyse de sécurité
hints-token-details-security-content =
    Évaluation des risques par Rugcheck.xyz et analyse on-chain.

    { "*" }*Score de sécurité (0-100) :**
    Un score élevé indique un token plus sûr. Les facteurs incluent :
    • Permissions d'autorité (mint/gel)
    • Concentration des holders
    • Statut de verrouillage des LP
    • Schémas de risque connus

    { "*" }*Principaux indicateurs de risque :**
    • **Autorité de mint** — peut créer de nouveaux tokens (risque d'inflation)
    • **Autorité de gel** — peut geler les comptes de token
    • **% du principal holder** — risque de concentration
    • **Fournisseurs de LP** — nombre de fournisseurs de liquidité

    Vérifiez toujours la sécurité avant de trader des montants importants.
hints-token-details-pools-title = Pools de liquidité
hints-token-details-pools-content =
    Tous les pools de liquidité découverts pour ce token.

    { "*" }*Pourquoi plusieurs pools comptent :**
    • Chaque pool a une liquidité et un prix différents
    • Les routeurs de swap trouvent la meilleure route entre les pools
    • Le prix peut varier de 1 à 5 % entre les pools

    { "*" }*Informations sur le pool :**
    • **DEX** — quelle plateforme héberge le pool
    • **Liquidité** — valeur en USD des réserves du pool
    • **Volume** — activité d'échange récente
    • **Prix** — prix actuel du pool

    Le service de pools calcule les prix à partir de la paire SOL à la plus forte liquidité.

## ui

hints-ui-featured-title = À la une
hints-ui-featured-content =
    Les tokens boostés d'abord, puis les projets tendance de Jupiter et DexScreener.

    { "*" }*Ce que vous verrez :**
    • Les tokens boostés — leurs équipes ont payé pour les promouvoir — épinglés en tête, marqués en or
    • Les tokens tendance des tableaux de découverte ensuite
    • Cliquez sur un token pour ouvrir sa fiche complète

    { "*" }*Booster un token :**
    Un boost achète de la visibilité, jamais une recommandation. Les lignes boostées sont marquées en or
    partout où elles apparaissent, y compris dans votre tableau de tokens, afin que vous sachiez toujours
    lesquelles sont lesquelles. Boostez un token sur
    { "*" }*screenerbot.io/boost**.

    { "*" }*Désactiver la ligne :**
    Masquez-la dans **Paramètres → Interface → Afficher la ligne À la une**. L'action de l'en-tête ouvre
    toujours la vue complète À la une.

## Hint popover chrome (ui/hint_popover.js)

hints-trigger =
    .aria-label = Aide : { $title }
hints-popover-close =
    .aria-label = Fermer
hints-popover-learn-more = En savoir plus
hints-popover-dismiss = Ne plus afficher
