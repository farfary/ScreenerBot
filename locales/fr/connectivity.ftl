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
