# Position details labels.

positions-state-reason-position-created = Position erstellt

positions-status-open = Offen
positions-status-closed = Geschlossen
positions-status-archived = Archiviert
positions-origin-copy = Copy
positions-origin-manual = Manuell
positions-origin-wallet = Wallet
positions-origin-copy-link =
    .title = Copy-Aufgabe öffnen, die diese Position eröffnet hat
positions-holding-frozen = Eingefroren
    .title = Die Mint-Berechtigung hat dieses Token-Konto eingefroren – das Guthaben kann weder übertragen noch verkauft werden
positions-toolbar-total = Gesamt
positions-toolbar-delete-all = Alle löschen
positions-search-placeholder = Nach Symbol oder Mint suchen...
positions-filter-origin = Herkunft
positions-filter-origin-all = Alle Herkünfte
positions-filter-origin-auto = Auto-Trader
positions-filter-origin-copy = Copy-Trading
positions-delete-all-tooltip = Alle archivierten Positionen endgültig löschen

positions-column-token = Token
positions-column-archived-at = Archiviert
positions-column-entry-time = Einstiegszeit
positions-column-exit-time = Ausstiegszeit
positions-column-avg-entry = Ø Einstieg ({ -sol })
positions-column-avg-exit = Ø Ausstieg ({ -sol })
positions-column-current-price = Aktuell ({ -sol })
positions-column-total-invested = Gesamt investiert
positions-column-proceeds = Erlös
positions-column-pnl = GuV
positions-column-pnl-percent = GuV %
positions-column-size = Größe
positions-column-dca = DCA
positions-column-exits = Ausstiege
positions-column-unrealized-pnl = Unrealisierte GuV
positions-column-unrealized-percent = Unrealisiert %

positions-unknown-basis = Keine Einstandsbasis in der Historie dieser Wallet (Airdrop, in USD notierte Ausführung oder Swap ohne SOL-Seite)
positions-unknown-history = Diese Runde stimmt nicht mit dem On-Chain-Guthaben überein
positions-dca-count =
    { $count ->
        [one] { $count } DCA
       *[other] { $count } DCAs
    }
positions-exit-count =
    { $count ->
        [one] { $count } Ausstieg
       *[other] { $count } Ausstiege
    }

positions-action-add =
    .title = Zur Position hinzufügen (DCA)
    .aria-label = Zur Position hinzufügen
positions-action-sell =
    .title = Verkaufen (ganz oder Teil in %)
    .aria-label = Position verkaufen
positions-action-sell-frozen = Von der Mint-Berechtigung eingefroren – dieser Bestand kann nicht verkauft werden
positions-action-remove =
    .title = Entfernen (archivieren oder löschen)
    .aria-label = Position entfernen
positions-action-restore =
    .title = Als Offen/Geschlossen wiederherstellen
    .aria-label = Position wiederherstellen
positions-action-delete =
    .title = Endgültig löschen
    .aria-label = Endgültig löschen
positions-action-in-progress = In Bearbeitung…

positions-caption-buying = Kauf läuft
positions-caption-buying-step = Kauf läuft · { $step }
positions-caption-selling = Verkauf läuft
positions-caption-selling-step = Verkauf läuft · { $step }
positions-caption-closing = Schließen läuft
positions-caption-failed = Fehlgeschlagen
positions-caption-failed-detail = Fehlgeschlagen · { $error }
positions-step-adding = Hinzufügen
positions-pending-buying = Kauf läuft…
positions-pending-buy-failed = Kauf fehlgeschlagen

positions-load-failed = Positionen konnten nicht aktualisiert werden
positions-toast-not-found = Positionsdaten nicht gefunden
positions-toast-deleted = Position gelöscht
positions-toast-archived = Position archiviert
positions-toast-restored = Position wiederhergestellt
positions-action-failed = Aktion fehlgeschlagen
positions-delete-title = Position endgültig löschen
positions-delete-message = { $symbol } endgültig löschen? Dadurch werden die Position und ihre Historie aus der Datenbank entfernt. Das lässt sich nicht rückgängig machen. Ihre Transaktionen und Token-Daten bleiben unberührt.
positions-delete-confirm = Endgültig löschen
positions-delete-all-title = Alle archivierten Positionen löschen
positions-delete-all-message =
    { $count ->
        [one] Die archivierte Position ({ $count }) endgültig löschen? Das lässt sich nicht rückgängig machen. Transaktionen und Token-Daten bleiben unberührt.
       *[other] Alle { $count } archivierten Positionen endgültig löschen? Das lässt sich nicht rückgängig machen. Transaktionen und Token-Daten bleiben unberührt.
    }
positions-delete-all-message-empty = Alle archivierten Positionen endgültig löschen? Das lässt sich nicht rückgängig machen.
positions-delete-all-confirm = Alle löschen
positions-delete-all-done =
    { $count ->
        [one] { $count } archivierte Position gelöscht
       *[other] { $count } archivierte Positionen gelöscht
    }
positions-delete-all-failed = Archivierte Positionen konnten nicht gelöscht werden

positions-remove-title = Position entfernen
positions-remove-open-warning = <strong>Diese Position ist noch offen.</strong> Der Bot hält diesen Token. Beim Entfernen wird der Handelsplatz freigegeben und das Tracking beendet – es wird aber <strong>nichts</strong> verkauft. Verkaufen Sie zuerst, wenn Sie Ihre { -sol } zurückhaben möchten.
positions-remove-modes =
    .aria-label = Entfernungsmodus
positions-remove-archive = Archivieren
positions-remove-recommended = Empfohlen
positions-remove-archive-description = Blendet sie im Tab „Archiviert“ aus. Jederzeit umkehrbar – nichts wird verkauft und alle Trades bleiben gespeichert.
positions-remove-delete = Endgültig löschen
positions-remove-delete-description = Diese Position und ihre gesamte Historie aus der Datenbank löschen.
positions-remove-danger = Dadurch werden die Position und ihre Historie endgültig entfernt. <strong>Das lässt sich nicht rückgängig machen.</strong> Ihre Transaktionen und Token-Daten bleiben unberührt.
positions-remove-confirm-archive = Position archivieren

positions-management-changed = Positionsverwaltung auf { $mode } gesetzt
positions-details-load-failed = Positionsdetails konnten nicht geladen werden
positions-details-mint-label = Mint-Adresse
positions-details-management-failed = Positionsverwaltung konnte nicht aktualisiert werden
positions-details-favorite-add =
    .title = Zu Favoriten hinzufügen
    .aria-label = Zu Favoriten hinzufügen
positions-details-favorite-remove =
    .title = Aus Favoriten entfernen
    .aria-label = Aus Favoriten entfernen
positions-details-view-solscan =
    .title = Auf { -solscan } ansehen
    .aria-label = Token auf { -solscan } ansehen
positions-details-close =
    .title = Schließen (Esc)
    .aria-label = Schließen
positions-details-chart-section =
    .aria-label = Preis-Chart
positions-details-loading-chart = Chart wird geladen...
positions-details-activity-section =
    .aria-label = Aktivität
positions-details-activity-title = Aktivität
positions-details-split-handle =
    .aria-label = Größe von Chart und Aktivität ändern
positions-details-activity-pane =
    .aria-label = Aktivitätsbereich
positions-details-activity-expand =
    .title = Aktivität erweitern
    .aria-label = Aktivität erweitern
positions-details-summary-section =
    .aria-label = Positionsübersicht
positions-details-loading = Position wird geladen...

positions-management-auto-trader = Auto-Trader
positions-management-user-only = Nur Nutzer
positions-management-copy-task = Copy-Aufgabe
positions-management-hybrid = Hybrid
positions-pane-show-chart = Chart anzeigen
positions-pane-show-activity = Aktivität anzeigen
positions-pane-restore-activity = Aktivität wiederherstellen
positions-pane-expand-chart =
    .title = Chart erweitern
    .aria-label = Chart erweitern

positions-risk-low = Geringes Risiko
positions-risk-medium = Mittleres Risiko
positions-risk-high = Hohes Risiko
positions-risk-unknown = Risiko unbekannt
positions-busy-buying = Kauf läuft…
positions-busy-selling = Verkauf läuft…
positions-busy-closing = Schließen läuft…
positions-header-avg-entry = Ø Einstieg
positions-header-buy-count =
    { $count ->
        [one] { $count } Kauf
       *[other] { $count } Käufe
    }
positions-header-exit-price = Ausstiegspreis
positions-header-closed-ago = geschlossen { $ago }
positions-header-realized-pnl = Realisierte GuV
positions-header-usd-note = USD zum heutigen { -sol }-Preis
positions-header-returned = Zurückerhalten
positions-header-of-invested = von { $amount } investiert
positions-header-price = Preis
positions-header-last-price = Letzter Preis
positions-header-pool-ago = Pool · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = Unrealisierte GuV
positions-header-pnl-last-price = GuV zum letzten Preis
positions-header-value = Wert
positions-header-last-value = Letzter Wert
positions-header-invested = { $amount } investiert
positions-header-origin-hint = So wurde diese Position eröffnet
positions-header-risk-hint = { -rugcheck }-Score – niedriger ist sicherer
positions-header-frozen = Eingefroren
    .title = Die Mint-Berechtigung hat diesen Bestand eingefroren
positions-header-managed-by = Verwaltet von
positions-header-management-select =
    .aria-label = Positionsverwaltung

positions-origin-unknown = unbekannt
positions-origin-copied-task = Kopiert · Aufgabe { $task }
positions-origin-manual-entry = Manueller Einstieg
positions-origin-wallet-entry = Wallet-Einstieg
positions-origin-auto-strategy = Auto · { $strategy }
positions-origin-auto-entry = Automatischer Einstieg

positions-pending-adding = Hinzufügen
positions-pending-adding-amount = { $amount } hinzufügen
positions-pending-selling = Verkauf
positions-pending-selling-percent = { $percent } verkaufen
positions-pending-confirming = { $label } · wird bestätigt
    .title = Gesendet und wartet auf On-Chain-Bestätigung. Die Werte werden nach der Verifizierung aktualisiert.

positions-trade-add = Hinzufügen
    .title = Zur Position hinzufügen
positions-trade-sell = Verkaufen
    .title = Teil der Position verkaufen
positions-trade-close = Position schließen
    .title = Alles verkaufen und schließen
positions-trade-token = Token-Details
    .title = Token-Details öffnen

positions-favorite-token-fallback = Token
positions-favorite-added = { $symbol } zu Favoriten hinzugefügt
positions-favorite-removed = { $symbol } aus Favoriten entfernt
positions-favorite-add-failed = Favorit konnte nicht hinzugefügt werden
positions-favorite-remove-failed = Favorit konnte nicht entfernt werden
positions-favorite-update-failed = Favoriten konnten nicht aktualisiert werden

positions-summary-position = Position
positions-summary-price-path = Preisverlauf
positions-summary-network-fees = Netzwerkgebühren
positions-summary-risk = Risiko
positions-summary-market = Markt
positions-summary-market-now = Markt aktuell
positions-summary-links = Links
positions-fact-tokens-fallback = Token
positions-fact-bought = Gekauft
positions-fact-holding = Bestand
positions-fact-sold = Verkauft
positions-fact-realized = Realisiert
positions-fact-opened = Eröffnet
positions-fact-closed = Geschlossen
positions-fact-reason = Grund
positions-fact-archived = Archiviert
positions-fact-entry = Einstieg
positions-fact-exit = Ausstieg
positions-fact-total = Gesamt
positions-fact-verified = On-Chain verifiziert
positions-fact-confirming = Wird bestätigt
positions-fact-share-of-bought = { $percent } des Gekauften
positions-fact-share-of-invested = { $percent } des Investierten
positions-fact-entry-count =
    { $count ->
        [0] 1 Einstieg
        [one] 1 Einstieg + { $count } Nachkauf
       *[other] 1 Einstieg + { $count } Nachkäufe
    }
positions-fact-partial-exits-back =
    { $count ->
        [one] { $count } Teilausstieg · { $returned } zurück
       *[other] { $count } Teilausstiege · { $returned } zurück
    }
positions-fact-held = gehalten { $age }
positions-fact-vs-entry = { $percent } ggü. Einstieg
positions-fact-exit-vs-peak = Ausstieg ggü. Hoch
positions-fact-now-vs-peak = Aktuell ggü. Hoch
positions-fact-entry-range = Einstiegsbereich
positions-range-low = Tief
positions-range-peak = Hoch
positions-range-now = Aktuell
positions-range-label-exit = Einstiegs- und Ausstiegspreis zwischen Tief und Hoch
positions-range-label-now = Einstiegs- und aktueller Preis zwischen Tief und Hoch
positions-fact-mint-authority = Mint-Berechtigung
positions-fact-freeze-authority = Freeze-Berechtigung
positions-fact-active = Aktiv
positions-fact-pool = Pool
positions-fact-pool-liquidity = { $amount } { -sol } Liquidität
positions-fact-market-cap = Marktkapitalisierung
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = Liquidität
positions-fact-volume-24h = Volumen 24 Std.
positions-fact-price-change = Preisänderung
positions-change-period-1h = 1 Std.
positions-change-period-24h = 24 Std.
positions-fact-holders = Holder
positions-link-website = Website
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

positions-activity-load-failed = Aktivität konnte nicht geladen werden
positions-activity-loading = Aktivität wird geladen...
positions-activity-empty = Mit diesem Token ist in dieser Wallet noch nichts passiert
positions-activity-filter-empty = Keine Aktivität entspricht diesem Filter
positions-activity-round-count =
    { $count ->
        [one] { $count } Runde
       *[other] { $count } Runden
    }
positions-activity-event-count =
    { $count ->
        [one] { $count } Ereignis
       *[other] { $count } Ereignisse
    }
positions-activity-pending-count = { $count } ausstehend
positions-activity-failed-count = { $count } fehlgeschlagen
positions-filter-all = Alle
positions-filter-trades = Trades
positions-filter-buys = Käufe
positions-filter-sells = Verkäufe
positions-filter-wallet = Wallet
positions-filter-issues = Probleme
positions-activity-filters =
    .aria-label = Aktivität filtern
positions-activity-totals =
    .aria-label = Alle Runden für diesen Token
positions-activity-realized-all = Realisiert, alle Runden
positions-activity-invested = Investiert
positions-activity-returned = Zurückerhalten
positions-activity-opened = Eröffnet { $when }
positions-activity-round-title = Position { $index }
positions-activity-this-position = Diese Position
positions-activity-dates-unavailable = Daten nicht verfügbar
positions-activity-wallet-title = Wallet-Transaktionen
positions-activity-outside =
    { $count ->
        [one] Außerhalb jeder Position · { $range } · { $count } Ereignis
       *[other] Außerhalb jeder Position · { $range } · { $count } Ereignisse
    }
positions-details-signature-label = Signatur

positions-state-open = Position offen
positions-state-closing = Position wird geschlossen
positions-state-closed = Position geschlossen
positions-state-exit-pending = Positionsausstieg ausstehend
positions-state-exit-failed = Positionsausstieg fehlgeschlagen
positions-state-phantom = Position Phantom
positions-state-reconciling = Position wird abgeglichen

positions-event-kind-entry = Einstieg
positions-event-kind-dca = Nachkauf
positions-event-kind-partial-exit = Teilausstieg
positions-event-kind-exit = Ausstieg
positions-event-kind-buy = Wallet-Kauf
positions-event-kind-sell = Wallet-Verkauf
positions-event-kind-transfer = Übertragung
positions-event-kind-ata = Token-Konto
positions-event-kind-other = Transaktion
positions-event-state-pending = Ausstehend
positions-event-state-failed = Fehlgeschlagen
positions-event-state-synthetic = Synthetisch
positions-chain-status-failed-detail = Fehlgeschlagen: { $error }
positions-event-tokens-fallback = Token
positions-event-entry-submitted = Kauf über { $amount } gesendet
positions-event-entry-for = { $amount } für { $sol } gekauft
positions-event-entry = { $amount } gekauft
positions-event-dca-submitted = Nachkauf über { $amount } gesendet
positions-event-dca-for = { $amount } für { $sol } nachgekauft
positions-event-dca = { $amount } nachgekauft
positions-event-partial-exit-submitted-percent = Teilausstieg von { $percent } über { $amount } gesendet
positions-event-partial-exit-submitted = Teilausstieg über { $amount } gesendet
positions-event-sold-percent-for = { $amount } ({ $percent }) für { $sol } verkauft
positions-event-sold-percent = { $amount } ({ $percent }) verkauft
positions-event-sold-for = { $amount } für { $sol } verkauft
positions-event-sold = { $amount } verkauft
positions-event-exit-submitted = Vollständigen Positionsausstieg gesendet
positions-event-exit-for = Geschlossen, { $amount } für { $sol } verkauft
positions-event-exit-closed = Position geschlossen
positions-event-wallet-bought = Wallet hat { $amount } anderweitig gekauft
positions-event-wallet-sold = Wallet hat { $amount } anderweitig verkauft
positions-event-received = { $amount } erhalten
positions-event-sent = { $amount } gesendet
positions-event-transferred = { $amount } übertragen
positions-event-ata = Token-Konto-Aktivität
positions-event-wallet-transaction = Wallet-Transaktion mit { $amount }
positions-event-price-per-token = { $price } { -sol } / Token
positions-event-wallet-change = { $amount } Wallet-Änderung
positions-event-after-title = Position nach diesem Ereignis
positions-event-capital-invested = Investiertes Kapital
positions-event-average-entry = Durchschnittlicher Einstieg
positions-event-transfers-title = Token-Übertragungen
positions-event-transfer-amount = Menge
positions-event-transfer-mint = Mint
positions-event-transfer-from = Von
positions-event-transfer-to = An
positions-event-no-signature = Keine On-Chain-Signatur
positions-event-click-to-copy = Zum Kopieren klicken
positions-event-solscan = { -solscan }
positions-event-token-amount = Token-Menge
positions-event-trade-price = Trade-Preis
positions-event-sol-amount = { -sol }-Menge
positions-event-cost-basis = Einstandsbasis
positions-event-usd-value = USD-Wert
positions-event-network-fee = Netzwerkgebühr
positions-event-router = Router
positions-event-slot = Slot
positions-event-chain-status = Chain-Status
positions-event-transaction-type = Transaktionstyp
positions-event-direction = Richtung
positions-event-wallet-sol-change = Wallet-{ -sol }-Änderung
positions-event-instructions = Instruktionen
positions-event-compute-units = Compute Units
positions-event-accounts = Konten
positions-event-record-id = Datensatz-ID
positions-event-time-unavailable = Zeit nicht verfügbar
positions-event-details = Details
positions-event-hide-details = Details ausblenden

positions-chart-type-candles = Kerzen
positions-chart-type-line = Linie
positions-chart-type-area = Fläche
positions-chart-type-group =
    .aria-label = Chart-Typ
positions-chart-overlays-group =
    .aria-label = Chart-Overlays
positions-chart-ema = EMA
    .title = Exponentielle gleitende Durchschnitte, 9 und 21
positions-chart-fit = Anpassen
    .title = Lebensdauer dieser Position einpassen
positions-chart-timeframes-group =
    .aria-label = Zeitrahmen
positions-chart-pane-group =
    .aria-label = Chart-Bereich
positions-chart-unavailable = Chart-Engine nicht verfügbar
positions-chart-collecting = Chart-Daten werden gesammelt…
positions-chart-no-data = Für diesen Token gibt es noch keine Chart-Daten
positions-chart-avg-entry = Ø Einstieg
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = Ø Einstieg
positions-chart-legend-avg-entry-off-scale = Ø Einstieg (außerhalb der Skala)
positions-chart-dropped-events =
    { $count ->
        [one] { $count } Ereignis ohne Kerze in diesem Zeitrahmen
       *[other] { $count } Ereignisse ohne Kerze in diesem Zeitrahmen
    }
positions-chart-level = Level
positions-chart-level-above = { $label } { $price } liegt über dieser Ansicht
positions-chart-level-below = { $label } { $price } liegt unter dieser Ansicht
positions-chart-scale-hint = Preisachse ziehen, um dorthin zu skalieren
positions-chart-pnl-at-bar = GuV @ Balken
positions-chart-click-to-locate = Zum Auffinden klicken
