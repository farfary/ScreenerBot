# Dashboard shell: header, ticker, notification drawer and status bar.

shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-version = v{ $version }

## Header

shell-header-brand =
    .aria-label = Dashboard-Startseite öffnen
    .title = Dashboard-Startseite
shell-bot-card =
    .aria-label = Status des Auto-Traders wird geladen
shell-bot-label = Auto
shell-bot-status-loading = LÄDT
shell-bot-today = Heute
shell-explore-control =
    .aria-label = Explore Mode. Verbinden Sie eine Wallet und einen RPC-Endpunkt, um alle Funktionen zu aktivieren
    .title = Verbinden Sie eine Wallet und einen RPC-Endpunkt, um Trading, Guthaben und Live-On-Chain-Daten zu aktivieren
shell-explore-title = Explore Mode
shell-explore-detail = Wallet und RPC nicht verbunden
shell-explore-action = Einrichtung abschließen
shell-setup-gate-detail = Der Explore Mode läuft ohne Wallet und RPC. Schließen Sie die Einrichtung ab, um beides zu verbinden.
shell-wallet-card =
    .aria-label = Wallet-Wert; Positionen öffnen
    .title = Wallet-Wert ({ -sol } + Token) · Positionen öffnen
shell-wallet-worth-label = WERT
shell-wallet-native-label = { -sol }
shell-wallet-tokens-label = TKN
shell-sol-price-card =
    .aria-label = { -sol }-Preis in USD — Chart öffnen
    .title = { -sol }-Preis · Klicken für Chart
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24 Std.
shell-copy-card =
    .aria-label = Copy-Trading; Copy-Trading öffnen
    .title = Copy-Trading · Copy-Trading öffnen
shell-copy-label = COPY-TRADING
shell-actions-more =
    .aria-label = Weitere Kopfzeilenaktionen
    .title = Weitere Aktionen
shell-actions-group =
    .aria-label = Kopfzeilenaktionen
shell-action-search =
    .aria-label = Token suchen
    .title = Token suchen (Strg/Cmd+K)
shell-action-featured =
    .aria-label = Hervorgehobene Token
    .title = Hervorgehobene Token
shell-action-notifications =
    .aria-label = Aktionen und Benachrichtigungen
    .title = Aktionen und Benachrichtigungen
shell-action-restart =
    .aria-label = App neu starten
    .title = App neu starten
shell-action-theme =
    .aria-label = Design umschalten
    .title = Design umschalten
shell-action-settings =
    .aria-label = Einstellungen
    .title = Einstellungen
shell-tabs-scroll-start =
    .aria-label = Vorherige Tabs anzeigen
    .title = Vorherige Tabs anzeigen
shell-tabs-scroll-end =
    .aria-label = Weitere Tabs anzeigen
    .title = Weitere Tabs anzeigen
shell-nav-more = Mehr
shell-ticker-scroll-start =
    .aria-label = Vorherige Kennzahlen anzeigen
    .title = Vorherige Kennzahlen anzeigen
shell-ticker-scroll-end =
    .aria-label = Weitere Kennzahlen anzeigen
    .title = Weitere Kennzahlen anzeigen

## Ticker

shell-ticker-monitoring-segment =
    .title = Vom Pool-Dienst überwachte Token
shell-ticker-monitoring = Überwacht:
shell-ticker-filtering-segment =
    .title = Token, die die Filterkriterien bestanden bzw. nicht bestanden haben
shell-ticker-passed = Bestanden:
shell-ticker-rejected = Abgelehnt:
shell-ticker-pnl-segment =
    .title = Heutiger realisierter Gewinn und Verlust
shell-ticker-pnl = GuV heute:
shell-ticker-rpc-segment =
    .title = RPC-Aufrufe pro Minute und Erfolgsquote
shell-ticker-rpc = RPC:
shell-ticker-rpc-rate = { $amount }/Min.
shell-ticker-services-segment =
    .title = Zustand der Hintergrunddienste
shell-ticker-services-loading = Dienste: <strong>Lädt</strong>

## Notification drawer

shell-notification-title = Aktionen
shell-notification-mark-all-read =
    .title = Alle als gelesen markieren
shell-notification-clear-all =
    .title = Alle löschen
shell-notification-close =
    .aria-label = Schließen
shell-notification-tab-all = Alle
shell-notification-tab-active = Aktiv
shell-notification-tab-done = Erledigt
shell-notification-tab-failed = Fehlgeschlagen
shell-notification-filter-type-all = Alle Typen
shell-notification-filter-type-buy = Kauf
shell-notification-filter-type-sell = Verkauf
shell-notification-filter-type-open = Öffnen
shell-notification-filter-type-close = Schließen
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = Teilweise
shell-notification-filter-clear =
    .title = Filter zurücksetzen
    .aria-label = Filter zurücksetzen
shell-notification-filter-state-all = Alle Status
shell-notification-filter-state-in-progress = In Bearbeitung
shell-notification-filter-state-completed = Abgeschlossen
shell-notification-filter-state-failed = Fehlgeschlagen
shell-notification-filter-state-cancelled = Abgebrochen
shell-notification-list =
    .aria-label = Benachrichtigungen
shell-notification-empty = Noch keine Aktionen
shell-notification-loading-more = Weitere werden geladen...
shell-notification-back-to-top =
    .title = Nach oben

## Status bar

shell-status-bar-version = v
shell-status-bar-uptime = Laufzeit
shell-status-bar-memory = RAM
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/Min.
shell-status-bar-trading = Trading
shell-status-bar-positions = Pos.
shell-status-bar-tokens = Token

## Splash

shell-splash-starting = { -brand } wird gestartet
shell-splash-waiting = Warten auf Antwort des lokalen Kerns.
shell-splash-failed = { -brand } konnte nicht gestartet werden
shell-splash-failed-detail = Prüfen Sie die Protokolldatei und starten Sie die App neu.

## Connection state

shell-connection-connected = Kern verbunden
shell-connection-waiting = Warten auf Kern…
shell-connection-retry-now = Jetzt erneut versuchen
shell-connection-overlay-detail = Der Kern ist nicht erreichbar. Das Trading ist pausiert; die Verbindung wird automatisch wiederhergestellt.
shell-connection-restored = Verbindung zum Kern wiederhergestellt

shell-trader-control-failed = Trader-Steuerung fehlgeschlagen
shell-notification-button-unread = Aktionen und Benachrichtigungen, { $count } ungelesen
shell-restart-confirm-title = Bot neu starten
shell-restart-confirm-message =
    Möchten Sie den Bot wirklich neu starten?

    Dabei werden:
    • alle Dienste gestoppt
    • der Prozess neu gestartet
    • ca. 10–15 Sekunden benötigt

    Alle aktiven Vorgänge werden unterbrochen.
shell-restart-confirm-action = Neu starten
shell-restart-progress = Bot wird neu gestartet
shell-restart-failed = Neustart fehlgeschlagen
shell-restart-failed-status = Neustart fehlgeschlagen: { $status }
shell-restart-helper-unavailable = Der automatische Neustart-Helfer ist nicht verfügbar. Laden Sie das Dashboard in Kürze neu.

shell-page-title-fallback = Dashboard
shell-page-load-failed = Seite konnte nicht geladen werden
shell-page-offline-detail = Der Kern ist derzeit nicht erreichbar. Diese Seite wird automatisch geladen, sobald die Verbindung wieder besteht.

## Auto Trader card

shell-bot-state-explore = EXPLORE
shell-bot-state-halted = ANGEHALTEN
shell-bot-state-off = AUS
shell-bot-state-waiting = WARTET
shell-bot-state-idle = LEERLAUF
shell-bot-state-entry-paused = EINSTIEG PAUSIERT
shell-bot-state-running = LÄUFT
shell-bot-control-explore = Der Auto-Trader ist im Explore Mode nicht verfügbar. Wallet- und RPC-Einrichtung öffnen.
shell-bot-control-halted = Der Notstopp ist aktiv. Auto-Trader-Steuerung öffnen.
shell-bot-control-off = Der Auto-Trader ist aus. Zum Aktivieren klicken.
shell-bot-control-waiting = Der Auto-Trader ist aktiviert und wartet auf die Kerndienste. Zum Deaktivieren klicken.
shell-bot-control-idle = Der Auto-Trader ist aktiviert, aber beide Monitore sind aus. Auto-Trader-Steuerung öffnen.
shell-bot-control-entry-paused = Der Verlustschutz hat Einstiege pausiert; Ausstiege laufen weiter. Auto-Trader-Steuerung öffnen.
shell-bot-control-running = Der Auto-Trader läuft. Zum Deaktivieren klicken.

## Wallet and copy cards

shell-wallet-card-summary = Wallet-Wert: { $equity } { -sol } ({ $balance } { -sol } Guthaben, { $tokens } Token); Positionen öffnen
shell-copy-running-live = { $count } live
shell-copy-running-paper = { $count } Paper
shell-copy-value-paused = Pausiert
shell-copy-value-idle = Leerlauf
shell-copy-sub-active = { $active } von { $total } aktiv

## Ticker services state

shell-ticker-services-healthy = Dienste: <strong>Fehlerfrei</strong>
shell-ticker-services-issues =
    { $count ->
        [one] Dienste: <strong>{ $count } Problem</strong>
       *[other] Dienste: <strong>{ $count } Probleme</strong>
    }

## Agent approval prompt

shell-agent-request-title = Agent-Anfrage
shell-agent-request-client-fallback = Ein gekoppelter Agent
shell-agent-request-message = { $client } möchte „{ $tool }“ in { -brand } ausführen. Diese Anfrage { $expiry }.
shell-agent-request-message-arguments = { $client } möchte „{ $tool }“ in { -brand } ausführen. Argumente: { $summary }. Diese Anfrage { $expiry }.
shell-agent-request-expires-minutes = läuft in { $minutes } Min. ab
shell-agent-request-expires-seconds = läuft in { $seconds } s ab
shell-agent-request-approve = Genehmigen
shell-agent-request-deny = Ablehnen

## Toasts, dialogs and shared widgets

shell-toast-copied = { $label } kopiert
shell-toast-copy-failed = Kopieren fehlgeschlagen
shell-toast-still-running = Läuft noch — siehe Benachrichtigungscenter
shell-toast-dismiss =
    .aria-label = Schließen
shell-confirm-title = Aktion bestätigen
shell-confirm-message = Sind Sie sicher?

shell-assistant-label = Assistent
shell-assistant-dialog =
    .aria-label = Assistent

shell-status-bar-trading-active = Aktiv
shell-status-bar-trading-inactive = Inaktiv

## Action toasts

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title } abgebrochen
shell-action-swap-buy-live = Kauf läuft
shell-action-swap-buy-done = Gekauft
shell-action-swap-buy-failed = Kauf fehlgeschlagen
shell-action-swap-sell-live = Verkauf läuft
shell-action-swap-sell-done = Verkauft
shell-action-swap-sell-failed = Verkauf fehlgeschlagen
shell-action-position-open-live = Position wird eröffnet
shell-action-position-open-done = Eröffnet
shell-action-position-open-failed = Eröffnung fehlgeschlagen
shell-action-position-close-live = Position wird geschlossen
shell-action-position-close-done = Geschlossen
shell-action-position-close-failed = Schließen fehlgeschlagen
shell-action-position-dca-live = Position wird aufgestockt
shell-action-position-dca-done = Aufgestockt
shell-action-position-dca-failed = Aufstockung fehlgeschlagen
shell-action-partial-exit-live = Teilausstieg
shell-action-partial-exit-done = Teilausstieg
shell-action-partial-exit-failed = Teilausstieg fehlgeschlagen
shell-action-manual-order-live = Order wird aufgegeben
shell-action-manual-order-done = Order aufgegeben
shell-action-manual-order-failed = Order fehlgeschlagen
shell-action-trade-live = Trade
shell-action-trade-done = Trade abgeschlossen
shell-action-trade-failed = Trade fehlgeschlagen
shell-action-via-router = { $action } über { $router }
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = umgeht { $venue }
shell-action-cost-guard-avoiding-cost = umgeht { $venue } · { $cost }
shell-action-cost-guard-avoiding-unnamed = umgeht einen Handelsplatz
shell-action-cost-guard-avoiding-unnamed-cost = umgeht einen Handelsplatz · { $cost }
shell-action-cost-guard-avoided = { $outcome } · { $cost } Kontomiete bei { $venue } vermieden
shell-action-cost-guard-avoided-unnamed = { $outcome } · { $cost } Kontomiete bei einem Handelsplatz vermieden
shell-action-exit-full = Vollständiger Ausstieg
shell-action-exit-percent = Ausstieg von { $percent }

## Exit dialog (ui/exit_dialog.js)

shell-exit-title = { -brand } schließen?
shell-exit-description = Wählen Sie, wie die Anwendung geschlossen werden soll
shell-exit-minimize = In den Tray minimieren
shell-exit-minimize-detail = Im Hintergrund weiterlaufen
shell-exit-quit = App beenden
shell-exit-quit-detail = Vollständig schließen und alle Dienste stoppen

## Image lightbox (ui/image_lightbox.js)

shell-lightbox-save =
    .title = Bild speichern
shell-lightbox-close =
    .title = Schließen (ESC)

## Theme control (scripts/theme.js)

shell-theme-light = Hell
shell-theme-dark = Dunkel
shell-theme-switch-to-light = Zum hellen Design wechseln
shell-theme-switch-to-dark = Zum dunklen Design wechseln
