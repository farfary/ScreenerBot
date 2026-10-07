# Event display text.
events-ohlcv-default = OHLCV-Ereignis: { $subtype }
events-filtering-default = Filter-Ereignis: { $subtype }
events-trader-default = Trader-Ereignis: { $subtype }
events-rpc-default = RPC { $method } - { $action }
events-api-default = { $api } - { $action }

events-task-completed = Aufgabe „{ $name }“ abgeschlossen
events-task-failed = Aufgabe „{ $name }“ fehlgeschlagen
events-task-timed-out = Aufgabe „{ $name }“ hat das Zeitlimit überschritten

events-subtype-task-completed = Aufgabe abgeschlossen
events-subtype-task-failed = Aufgabe fehlgeschlagen
events-subtype-task-timed-out = Aufgabe: Zeitlimit überschritten

events-message-none = Keine Nachricht

events-ohlcv-cache-cleanup-failed = OHLCV-Cache konnte nicht bereinigt werden
events-ohlcv-gap-cleanup-failed = Gefüllte Lückeneinträge konnten nicht bereinigt werden
events-ohlcv-gap-fill-failed = Fehler beim Lückenfüllen für { $mint }
events-ohlcv-backfill-scheduled = Multi-Zeitrahmen-Backfill für { $mint } über { $pool } geplant
events-ohlcv-fetch-failed = OHLCV für { $mint } über { $pool } konnte nicht abgerufen werden: { $error }
events-ohlcv-gap-detection-failed = Lückenerkennung für { $mint } über { $pool } fehlgeschlagen
events-ohlcv-fetch-success = { $count } OHLCV-Punkte für { $mint } gespeichert
events-ohlcv-retention-backfill-failed = Aufbewahrungs-Backfill für { $mint } über { $pool } fehlgeschlagen
events-ohlcv-empty-fetch = Leerer OHLCV-Abruf für { $mint } über { $pool }
events-ohlcv-pool-discovery-failed = Pool-Erkennung für { $mint } fehlgeschlagen
events-ohlcv-pool-discovery-success = Pools für { $mint } erkannt
events-ohlcv-process-token-error = Fehler bei der Verarbeitung von { $mint }: { $error }
events-ohlcv-rate-limit-hit = Ratenlimit bei der Verarbeitung von { $mint } ausgelöst
events-ohlcv-pool-unavailable = Keine funktionsfähigen Pools für { $mint } verfügbar; wird zurückgestellt
events-ohlcv-token-missing = Token { $mint } fehlte während der Verarbeitung
events-monitors-stopped = Monitore für automatisiertes Trading gestoppt
events-monitors-starting = Monitore für automatisiertes Trading werden gestartet
events-entry-monitor-started = Monitor für Einstiegschancen gestartet
events-exit-monitor-started = Ausstiegs-/Positionsmonitor gestartet
events-trader-service-stopped = Trader-Dienst ordnungsgemäß gestoppt
events-trader-service-stopping = Herunterfahren des Trader-Dienstes eingeleitet
events-trader-service-started = Trader-Dienst vollständig initialisiert und aktiv
events-trader-auto-trading-error = Beim Auto-Trading ist ein Fehler aufgetreten
events-trader-trading-enabled = Trading ist aktiviert und aktiv
events-trader-trading-disabled = Trading ist in der Konfiguration deaktiviert
events-trader-service-initializing = Initialisierung des Trader-Dienstes beginnt
events-connectivity-monitoring-stopped = Konnektivitätsüberwachung gestoppt
events-connectivity-monitoring-started = Konnektivitätsüberwachung gestartet (Intervall={ $seconds } s)
events-connectivity-service-initialized = Konnektivitätsdienst mit { $count } Monitoren initialisiert
events-connectivity-critical-unhealthy = { $count } kritische(r) Endpunkt(e) fehlerhaft - Das System sollte Vorgänge pausieren
events-connectivity-endpoint-recovered = Endpunkt von { $from } auf fehlerfrei zurückgekehrt
events-position-entry-not-landed = Der Kauf von { $symbol } ist nicht on-chain angekommen; die Position wurde entfernt
events-position-fill-after-force-close = Ein Trade von { $symbol } ist on-chain angekommen, nachdem die Position zwangsgeschlossen wurde; er wurde gebucht und die Position neu berechnet
events-position-swap-unbooked = Ein on-chain bestätigter Swap von { $symbol } ist noch nicht in seiner Position gebucht; er wird erneut geprüft, bis er gebucht ist
events-position-exit-residual-unattributed = { $symbol } wurde geschlossen, während Token des Mints in der Wallet verblieben. Sie können zu Position { $blocking } gehören, deren Einstieg nicht verifiziert ist; daher wurden sie nicht verkauft und keine Position verwaltet sie

## Events page (pages/events.js, ui/event_labels.js)

events-category-swap = Swap
events-category-transaction = Transaktion
events-category-pool = Pool
events-category-position = Position
events-category-token = Token
events-category-wallet = Wallet
events-category-trader = Trader
events-category-entry = Einstieg
events-category-system = System
events-category-ohlcv = OHLCV
events-category-rpc = RPC
events-category-api = API
events-category-security = Sicherheit
events-category-connectivity = Konnektivität
events-category-filtering = Filterung
events-category-scheduled-task = Geplante Aufgabe
events-category-learner = Learner
events-category-other = Sonstige

events-loading = Ereignisse werden geladen...
events-load-failed = Ereignisse konnten nicht geladen werden
events-load-failed-description = Warten auf Antwort des Backends. Es wird automatisch erneut versucht.
events-load-error = Ereignisse konnten nicht geladen werden
events-search-placeholder = Ereignisse durchsuchen...
events-summary-total = Gesamt
events-filter-category = Kategorie
events-filter-all-categories = Alle Kategorien
events-filter-all-severities = Alle Schweregrade
events-col-time = Zeit
events-col-category = Kategorie
events-col-type = Typ
events-col-severity = Schweregrad
events-col-message = Nachricht
events-col-token = Token
events-col-details = Details
events-payload-more = +{ $count } weitere

## Event details dialog (ui/events_dialog.js)

events-dialog-title = Ereignisdetails
events-dialog-close =
    .aria-label = Dialog schließen
events-dialog-payload = Payload
events-dialog-copy = Details kopieren
events-dialog-copy-title =
    .title = Alle Ereignisdetails kopieren
events-dialog-copy-done = Kopiert!
events-dialog-copy-failed = Fehlgeschlagen
events-dialog-not-available = k. A.
events-dialog-category-event = { $category }-Ereignis
events-dialog-field-id = Ereignis-ID
events-dialog-field-severity = Schweregrad
events-dialog-field-category = Kategorie
events-dialog-field-subtype = Untertyp
events-dialog-field-mint = Token-Mint
events-dialog-field-reference = Referenz
events-dialog-field-time = Ereigniszeit
events-dialog-field-age = Alter
events-dialog-field-created = Erstellt
events-dialog-export-heading = EREIGNISDETAILS
events-dialog-export-message = NACHRICHT
events-dialog-export-payload = PAYLOAD
events-dialog-export-line = { $label }: { $value }
