# Trade dialog messages.

## Quote panel

trade-quote-error-network = Angebot konnte nicht abgerufen werden – prüfen Sie Ihre Verbindung und versuchen Sie es erneut
trade-quote-error-title = Angebot konnte nicht abgerufen werden
trade-quote-title = Swap-Vorschau
trade-quote-refresh =
    .aria-label = Angebot aktualisieren
    .title = Angebot aktualisieren
trade-quote-idle = Wählen Sie einen Betrag, um Ihren Swap in der Vorschau zu sehen
trade-quote-loading = Beste Route wird gesucht…
trade-quote-retry = Erneut versuchen
trade-quote-pay = Sie zahlen
trade-quote-receive = Sie erhalten (geschätzt)
trade-quote-minimum = Garantiertes Minimum
    .title = Der geringste Betrag, den Sie nach maximaler Slippage erhalten können. Der Swap wird rückgängig gemacht, statt darunter ausgeführt zu werden.
trade-quote-impact = Preisauswirkung
trade-quote-slippage = Max. Slippage
trade-quote-platform-fee = Plattformgebühr
    .title = 0,5 % – unterstützt die Entwicklung. Bereits im obigen Angebot enthalten.
trade-quote-network-fee = Netzwerkgebühr
trade-quote-route = Route
trade-quote-disclaimer = Die Preise werden live von der Chain aktualisiert. Der Swap wird rückgängig gemacht, wenn er nicht über Ihrem garantierten Minimum ausgeführt werden kann, sodass Sie nie weniger als angezeigt erhalten.
trade-quote-impact-tiny = { "<0.01%" }
trade-quote-impact-warning = Die Preisauswirkung von { $impact } liegt über Ihrer maximalen Slippage von { $tolerance } % – diese Größe bewegt den Pool. Ein kleinerer Betrag wird näher am Marktpreis ausgeführt.

## Units

trade-unit-native = { -sol }
trade-unit-tokens = Token

## Actions. Ids are the dialog actions: buy, sell, add.

trade-buy-title = Token kaufen
trade-buy-subtitle = Betrag in { -sol } eingeben
trade-buy-confirm = Kauf ausführen
trade-buy-hint = Leer lassen für den Konfigurationsstandard
trade-sell-title = Position verkaufen
trade-sell-subtitle = Verkaufsanteil wählen
trade-sell-confirm = Verkauf ausführen
trade-sell-hint = Wert zwischen 1 und 100 eingeben
trade-sell-input = Eigener Prozentsatz
    .placeholder = 1-100
trade-add-title = Zur Position hinzufügen
trade-add-subtitle = DCA in bestehende Position
trade-add-confirm = Position aufstocken
trade-add-hint = Leer lassen für die konfigurierte DCA-Größe
trade-amount-input = Eigener Betrag
    .placeholder = { -sol }-Betrag eingeben

## Presets

trade-presets-quick-amount = Schnellbetrag
trade-presets-quick-sell = Schnellverkauf
trade-presets-match-entry = Wie Einstieg
trade-presets-fixed-amount = Fester Betrag
trade-preset-partial = Teil
trade-preset-half = Hälfte
trade-preset-most = Großteil
trade-preset-full = Voller Ausstieg
trade-preset-select =
    .aria-label = { $label } auswählen

## Dialog chrome

trade-dialog-close =
    .aria-label = Dialog schließen
trade-input-max = MAX
    .aria-label = Maximum verwenden
trade-slider =
    .aria-label = Betragsregler
trade-context-available = Verfügbar
trade-context-position-size = Positionsgröße
trade-context-holdings = Bestand
trade-held-badge = Gehalten
    .title = Sie halten eine offene Position in diesem Token
trade-manage-title = Manuelle Verwaltung
trade-manage-description = Der Auto-Trader verkauft oder stockt diese Position nicht auf. Deaktivieren, damit er die Ausstiege verwaltet.

## Slippage

trade-slippage-label = Slippage
trade-slippage-presets =
    .aria-label = Slippage-Voreinstellung
trade-slippage-auto = Auto
trade-slippage-custom =
    .placeholder = Eigene
    .aria-label = Eigene Slippage in Prozent
trade-slippage-note-auto = Auto (aus den Einstellungen)
trade-slippage-note-auto-value = Auto ({ $pct } % aus den Einstellungen)
trade-slippage-note-override = Überschreibung: { $pct } %
trade-slippage-warning = Hohe Slippage: Sie erhalten möglicherweise bis zu { $pct } % weniger als angeboten.
trade-impact-warning-title = Warnung: hohe Preisauswirkung
trade-impact-warning-text = Dieser Trade hat eine Preisauswirkung von <strong>{ $impact }</strong>, die Ihre Slippage-Toleranz von <strong>{ $tolerance } %</strong> überschreitet. Sie erhalten möglicherweise deutlich weniger als erwartet.
trade-impact-warning-proceed = Trotzdem fortfahren

## Validation and verification

trade-error-invalid-number = Ungültige Zahl
trade-error-percentage-range = Der Prozentsatz muss zwischen 1 und 100 liegen
trade-error-amount-positive = Der Betrag muss größer als 0 sein
trade-error-amount-minimum = Minimum: 0,001 { -sol }
trade-error-insufficient = Unzureichendes Guthaben (benötigt { $needed } plus { $reserve } für Gebühren, vorhanden { $balance })
trade-error-position-closed = Diese Position ist nicht mehr offen.
trade-error-verify-failed = Token-Guthaben konnte nicht verifiziert werden
trade-error-position-missing = Position nicht gefunden – möglicherweise wurde sie geschlossen
trade-error-balance-changed = Token-Guthaben hat sich geändert. Erwartet { $expected }, jetzt { $current }. Bitte aktualisieren.
trade-error-verify-network = Netzwerkfehler bei der Guthabenprüfung

## Quick trade

trade-quick-buy-title = Schnellkauf
trade-quick-sell-title = Schnellverkauf
trade-quick-subtitle = Token-Mint-Adresse eingeben
trade-quick-mint-label = Token-Mint-Adresse eingeben
trade-quick-mint-input =
    .placeholder = Mint-Adresse eingeben oder nach Symbol suchen...
trade-quick-paste =
    .aria-label = Aus der Zwischenablage einfügen
trade-quick-recent = Zuletzt:
trade-quick-fetching = Token-Infos werden abgerufen...
trade-quick-continue = Weiter
trade-quick-token-not-found = Token nicht gefunden
trade-quick-token-failed = Token konnte nicht abgerufen werden
trade-quick-token-not-in-database = Token nicht in der Datenbank gefunden
trade-quick-token-info-failed = Token-Infos konnten nicht abgerufen werden
trade-quick-no-position = Keine Position für diesen Token gefunden
trade-quick-no-holdings = Die Position hat keine verbleibenden Token
trade-quick-position-failed = Positionsdaten konnten nicht abgerufen werden

## Manual trade toasts

trade-toast-no-mint = Keine Mint-Adresse verfügbar
trade-toast-open-failed = Trade-Dialog konnte nicht geöffnet werden
trade-toast-pending-buy = Kauf läuft noch
trade-toast-pending-add = Aufstockung läuft noch
trade-toast-pending-sell = Verkauf läuft noch
trade-toast-pending-message = Der Browser hat nicht weiter gewartet; das Ergebnis sehen Sie in der Positionszeile
trade-toast-failed-buy = Kauf fehlgeschlagen
trade-toast-failed-add = Aufstockung der Position fehlgeschlagen
trade-toast-failed-sell = Verkauf fehlgeschlagen

trade-reason-strategy-signal = Strategiesignal
trade-reason-manual-entry = Manueller Einstieg
trade-reason-force-buy = Erzwungener Kauf
trade-reason-copy-buy = Copy-Kauf
trade-reason-dca-scheduled = DCA geplant
trade-reason-take-profit = Take Profit
trade-reason-stop-loss = Stop-Loss
trade-reason-trailing-stop = Trailing-Stop
trade-reason-time-override = Zeit-Override
trade-reason-strategy-exit = Strategie-Ausstieg
trade-reason-llm-analysis-exit = LLM-Analyse-Ausstieg
trade-reason-manual-exit = Manueller Ausstieg
trade-reason-risk-management = Risikomanagement
trade-reason-blacklisted = Auf der Blacklist
trade-reason-force-sell = Erzwungener Verkauf
trade-reason-copy-sell = Copy-Verkauf
trade-reason-closed-externally = Extern geschlossen
trade-reason-wallet-history = Wallet-Historie
trade-reason-exit-retry-pending = Ausstiegs-Wiederholung ausstehend
trade-reason-synthetic-exit-permanent-failure = Synthetischer Ausstieg: dauerhafter Fehler
trade-reason-pending-verification = { $reason } (Verifizierung ausstehend)
trade-reason-force-closed = Zwangsgeschlossen: { $note }
trade-reason-stored = { $reason }

trade-quick-no-token = Kein Token ausgewählt
