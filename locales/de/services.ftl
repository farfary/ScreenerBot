services-health-component-unavailable = Komponente { $component } nicht verfügbar
services-health-unavailable = Gesundheitsstatus nicht verfügbar
services-health-pools-not-running = Pool-Dienst läuft nicht
services-health-events-db-uninitialized = Ereignisdatenbank nicht initialisiert
services-health-sol-price-not-running = { -sol }-Preisdienst läuft nicht
services-health-sol-price-stale = { -sol }-Preisdaten sind veraltet (vor { $seconds } s)
services-health-sol-price-no-data = Noch keine { -sol }-Preisdaten verfügbar
services-health-telegram-discovery = Discovery-Modus
services-health-telegram-disconnected = Getrennt
services-health-wallet-watch-polling-only = Erkennung läuft nur per Polling
services-health-assistant-tasks-disabled = In der Konfiguration deaktiviert
services-health-connectivity-critical-unhealthy = Kritische Endpunkte fehlerhaft: { $endpoints }
services-health-filtering-snapshot-stale = Filter-Snapshot ist { $seconds } s alt

## Services page (pages/services.js)

services-status-healthy = Gesund
services-status-starting = Startet
services-status-degraded = Eingeschränkt
services-status-unhealthy = Fehlerhaft
services-status-stopping = Wird beendet
services-status-disabled = Deaktiviert
services-status-unknown = Unbekannt

services-name-account = Konto
services-name-assistant-scheduled-tasks = Geplante Aufgaben des Assistenten
services-name-ata-cleanup = Token-Konto-Bereinigung
services-name-connectivity = Konnektivität
services-name-copy-trading = Copy-Trading
services-name-events = Ereignisse
services-name-filtering = Filterung
services-name-llm-analysis = LLM-Analyse
services-name-ohlcv = OHLCV
services-name-pool-pricing = Pool-Preisermittlung
services-name-pools = Pools
services-name-positions = Positionen
services-name-referral = Empfehlungen
services-name-rpc-stats = RPC-Statistiken
services-name-sol-price = { -sol }-Preis
services-name-telegram = { -telegram }
services-name-tokens = Tokens
services-name-trader = Trader
services-name-transactions = Transaktionen
services-name-update-check = Update-Prüfung
services-name-wallet = Wallet
services-name-wallet-watch = Wallet-Überwachung
services-name-webserver = Webserver

services-loading = Dienste werden geladen...
services-load-failed = Dienste konnten nicht geladen werden
services-load-failed-description = Warte auf Antwort des Backends. Es wird automatisch erneut versucht.
services-refresh-failed = Dienste konnten nicht aktualisiert werden
services-search-placeholder = Dienste suchen...
services-summary-total = Gesamt
services-summary-alerts = Warnungen
services-summary-alerts-tooltip = { $degraded } eingeschränkt / { $unhealthy } fehlerhaft
services-filter-status = Status
services-filter-all-statuses = Alle Status
services-filter-all-services = Alle Dienste
services-filter-enabled-only = Nur aktivierte
services-filter-disabled-only = Nur deaktivierte
services-col-service = Dienst
services-col-health = Zustand
services-col-priority = Priorität
services-col-uptime = Laufzeit
services-col-activity = Aktivität
services-col-last-cycle = Letzter Zyklus
services-col-avg-cycle = Ø Zyklus
services-col-avg-poll = Ø Poll
services-col-cycle-rate = Zyklusrate
services-col-tasks = Aufgaben
services-col-ops = Ops/s
services-col-errors = Fehler
services-col-dependencies = Abhängigkeiten
services-dependencies-none = Keine
services-activity-busy = { $percent } ausgelastet
services-activity-polls =
    { $count ->
        [one] { $count } Poll
       *[other] { $count } Polls
    }
services-tasks-tooltip =
    { $count ->
        [one] { $count } Aufgabe
       *[other] { $count } Aufgaben
    }
    Zuletzt: { $last }
    Ø: { $avg }
    Poll: { $poll }
    Leerlauf: { $idle }
    Polls gesamt: { $polls }
services-tasks-none = Keine instrumentierten Aufgaben
