# Connectivity endpoint health text. Probe ids come from ProbeFailure in
# src/connectivity/types.rs; the technical values are not translated.
connectivity-probe-client-setup-failed = Échec de la création du client : { $detail }
connectivity-probe-http-status = HTTP { $status }
connectivity-probe-timeout = Délai dépassé après { $seconds } s
connectivity-probe-request-failed = La requête a échoué : { $detail }
connectivity-probe-rpc-none = Aucun fournisseur RPC configuré
connectivity-probe-rpc-partial = { $healthy }/{ $total } fournisseurs RPC opérationnels. Défaillants : { $unhealthy }
connectivity-probe-rpc-all-unreachable = Les { $total } fournisseurs RPC sont injoignables : { $unhealthy }
connectivity-probe-internet-dns-failed = La vérification DNS a échoué mais HTTP fonctionne : { $detail }
connectivity-probe-internet-all-failed = Les vérifications DNS et HTTP ont échoué. DNS : { $dns }. HTTP : { $http }
connectivity-probe-unknown = Erreur inconnue

connectivity-endpoint-not-checked = Pas encore vérifié

connectivity-service-checking = Connexion au service { -brand }
    .detail = Le trading démarre dès que la connexion est confirmée.
connectivity-service-ready = Service { -brand } connecté
    .detail = Les lectures du wallet et les trades passent par screenerbot.io.
connectivity-service-signed-out = Trading en pause · Déconnecté
    .detail = Connectez-vous pour reprendre. Les positions ouvertes ne sont pas vendues pendant la pause.
connectivity-service-reauthorize = Trading en pause · Nouvelle connexion requise
    .detail = La connexion de cet appareil ne permet pas encore de lire le wallet. Reconnectez-vous pour reprendre.
connectivity-service-busy = Trading en pause · Service occupé, nouvel essai
    .detail = Le trading reprend automatiquement. Pour éviter les pauses, utilisez votre propre RPC.
connectivity-service-daily-limit = Trading en pause · Limite quotidienne atteinte
    .detail = Votre compte a atteint la limite du jour pour le service { -brand }. Le trading reprend à 00:00 UTC, ou tout de suite avec votre propre RPC.
connectivity-service-unreachable = Trading en pause · screenerbot.io injoignable
    .detail = Le trading reprend au retour de la connexion. Vérifiez votre connexion internet.
connectivity-service-version = Trading en pause · Mise à jour requise
    .detail = Cette version n'est plus prise en charge par le service { -brand }. Passez à { $minimum } ou plus récent.
