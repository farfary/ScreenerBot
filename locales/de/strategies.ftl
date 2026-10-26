# Strategies page: the strategy list, the condition editor and the condition catalog.

## Strategy list

strategies-filter-all = Alle
strategies-filter-entry = Einstieg
strategies-filter-exit = Ausstieg
strategies-type-entry = Einstieg
strategies-type-exit = Ausstieg
strategies-list-empty-title = Noch keine Strategien
strategies-list-empty-hint = Erstellen Sie Ihre erste Strategie
strategies-new = Neue Strategie
strategies-import =
    .title = Strategie importieren
    .aria-label = Strategie importieren
strategies-item-enable =
    .title = Aktivieren
strategies-item-disable =
    .title = Deaktivieren

strategies-new-name = Neue Strategie

## Editor

strategies-editor-name =
    .placeholder = Strategiename
strategies-editor-dirty =
    .title = Ungespeicherte Änderungen
strategies-action-validate = Validieren
strategies-editor-empty = Wählen Sie eine Strategie zum Bearbeiten aus oder erstellen Sie eine neue
strategies-conditions-empty-title = Noch keine Bedingungen
strategies-conditions-empty-hint = Mit „{ strategies-add-condition }“ beginnen Sie den Aufbau
strategies-add-condition = Bedingung hinzufügen
strategies-modal-close =
    .aria-label = Schließen
strategies-card-move-up =
    .title = Nach oben
strategies-card-move-down =
    .title = Nach unten
strategies-card-duplicate =
    .title = Duplizieren
strategies-card-delete =
    .title = Löschen
# $name is the condition name.
strategies-card-delete-confirm = Bedingung entfernen
    .message = „{ $name }“ aus dieser Strategie entfernen?

strategies-summary-param = { $label }: { $value }
strategies-summary-none = Keine Parameter
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = Wert der Strategie ({ $value })
strategies-summary-period-seconds = Rückblick: { $amount } s
strategies-summary-period-minutes = Rückblick: { $amount } Min.
strategies-summary-period-hours = Rückblick: { $amount } Std.

strategies-value-percent = { $amount } %
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
        [one] { $amount } Stunde
       *[other] { $amount } Stunden
    }
strategies-value-candles =
    { $count ->
        [one] { $amount } Kerze
       *[other] { $amount } Kerzen
    }

strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = Std.
strategies-unit-multiplier = ×

## Condition catalog

strategies-catalog-search =
    .placeholder = Bedingungen durchsuchen...
    .aria-label = Bedingungen durchsuchen
strategies-catalog-search-clear =
    .aria-label = Suche löschen
strategies-catalog-no-description = Keine Beschreibung verfügbar

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = Aktiv
strategies-editor-timeframe = Standard-Zeitrahmen
    .title = Kerzen-Zeitrahmen für Bedingungen, die keinen eigenen festlegen
strategies-editor-priority = Priorität
    .title = Auswertungsreihenfolge von 1 bis 100; kleinere Zahlen werden zuerst ausgewertet
strategies-editor-description = Beschreibung
strategies-editor-description-field =
    .placeholder = Was diese Strategie tut
    .aria-label = Beschreibung der Strategie
strategies-action-export = Exportieren

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = Es müssen { $mode } dieser Bedingungen zutreffen
strategies-group-match = Es müssen { $mode } davon zutreffen
strategies-match-all = alle
strategies-match-any = mindestens eine
strategies-match-mode =
    .aria-label = Alle oder mindestens eine Bedingung
strategies-invert = Umkehren
    .title = Umkehren: trifft zu, wenn dies nicht zutrifft
strategies-chip-off = Aus
strategies-chip-inverted = Umgekehrt
strategies-row-switch =
    .title = Ausschalten, um es gespeichert zu lassen, aber nicht auszuwerten
    .aria-label = Von dieser Strategie ausgewertet
strategies-row-expand =
    .aria-label = Parameter ein- oder ausblenden
strategies-add-group = Gruppe hinzufügen
# $count is the number of conditions in the group.
strategies-group-delete-confirm = Gruppe entfernen
    .message =
        { $count ->
            [one] Diese Gruppe mit ihrer Bedingung entfernen?
           *[other] Diese Gruppe mit ihren { $count } Bedingungen entfernen?
        }

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems =
    { $count ->
        [one] { $count } Problem zu beheben: { $first }
       *[other] { $count } Probleme zu beheben: { $first }
    }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }, { $field }: { $problem }
strategies-problem-rules-empty = Fügen Sie mindestens eine eingeschaltete Bedingung hinzu
strategies-problem-group-empty = Eine Gruppe braucht mindestens eine eingeschaltete Bedingung
strategies-problem-not-applicable = Diese Bedingung kann in diesem Strategietyp nicht verwendet werden
strategies-problem-unknown-condition = Unbekannter Bedingungstyp: { $type }
strategies-problem-too-deep = Regeln dürfen höchstens { $max } Ebenen tief verschachtelt sein; eine umgekehrte Gruppe zählt als eine weitere Ebene
strategies-problem-required = Geben Sie eine Zahl ein
strategies-problem-range = Muss zwischen { $min } und { $max } liegen
strategies-problem-invalid-option = Wählen Sie eine der angebotenen Optionen

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = Alle
strategies-catalog-categories =
    .aria-label = Kategorien
strategies-catalog-empty = Keine Bedingung passt zu Ihrer Suche
strategies-catalog-exit-only = Nur für Ausstiegsstrategien
strategies-catalog-entry-only = Nur für Einstiegsstrategien
strategies-data-candles = Kerzen
strategies-data-pool = Pool
strategies-data-position = Position
strategies-data-market = Marktaktivität

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = Trades
strategies-value-trades =
    { $count ->
        [one] { $amount } Trade
       *[other] { $amount } Trades
    }

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = Zu viele Ebenen
    .message = Regeln dürfen höchstens { $max } Ebenen tief verschachtelt sein
strategies-toast-priority-invalid = Ungültige Priorität
    .message = Geben Sie eine ganze Zahl von 1 bis 100 ein
strategies-toast-exported = Strategie exportiert
# $name is the strategy name.
strategies-unsaved-confirm = Ungespeicherte Änderungen verwerfen?
    .message = „{ $name }“ hat Änderungen, die nicht gespeichert sind.
strategies-unsaved-discard = Verwerfen
strategies-import-error-json = Die Datei ist kein gültiges JSON
strategies-import-error-shape = Die Datei beschreibt keine Strategie
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = Die Datei verwendet eine Bedingung, die diese Version nicht kennt: { $type }
strategies-import-error-not-applicable = { $name } kann in diesem Strategietyp nicht verwendet werden

## New strategy dialog

strategies-create-title = Neue Strategie erstellen
strategies-create-prompt = Wählen Sie den Strategietyp, den Sie erstellen möchten:
strategies-create-entry-name = Einstiegsstrategie
strategies-create-entry-description = Bedingungen festlegen, wann ein Token gekauft wird
strategies-create-exit-name = Ausstiegsstrategie
strategies-create-exit-description = Bedingungen festlegen, wann ein Token verkauft wird

## Delete dialog

strategies-delete-title = Strategie löschen
strategies-delete-message = Strategie „{ $name }“ löschen? Diese Aktion kann nicht rückgängig gemacht werden.

## Toasts. A message value is the title; `.message` is the body.

strategies-toast-fix-validation = Bitte beheben Sie die Validierungsfehler vor dem Speichern
strategies-toast-load-failed = Laden fehlgeschlagen
    .message = Strategien konnten nicht vom Server geladen werden
strategies-toast-load-strategy-failed = Strategie konnte nicht geladen werden
strategies-toast-no-strategy = Keine Strategie erstellt
    .message = Fügen Sie mindestens eine Bedingung hinzu oder klicken Sie auf „Neue Strategie“, um zuerst eine Strategie zu erstellen
strategies-toast-name-required = Name erforderlich
    .message = Geben Sie vor dem Speichern einen Strategienamen ein
strategies-toast-saved = Strategie gespeichert
    .message = „{ $name }“ erfolgreich gespeichert
strategies-toast-save-failed = Speichern fehlgeschlagen
    .message = Strategie konnte nicht in der Datenbank gespeichert werden
strategies-toast-no-strategy-validate = Keine Strategie zum Validieren
strategies-toast-valid = Strategie ist gültig
strategies-toast-invalid = Strategie enthält Fehler
strategies-toast-validation-failed = Validierung fehlgeschlagen
strategies-toast-item-enabled = Strategie aktiviert
strategies-toast-item-disabled = Strategie deaktiviert
strategies-toast-item-toggle-failed = Strategie konnte nicht umgeschaltet werden
strategies-toast-deleted = Strategie gelöscht
    .message = „{ $name }“ erfolgreich entfernt
strategies-toast-delete-failed = Löschen fehlgeschlagen
    .message = Strategie konnte nicht aus der Datenbank gelöscht werden
strategies-toast-imported = Strategie importiert
strategies-toast-import-failed = Strategie konnte nicht importiert werden
strategies-toast-unknown-condition = Unbekannte Bedingung
    .message = Bedingungstyp nicht gefunden
strategies-toast-create-first = Zuerst Strategie erstellen
    .message = Klicken Sie auf „Neue Strategie“, um eine Strategie zu erstellen, bevor Sie Bedingungen hinzufügen

## Conditions

strategies-condition-candle-size = Kerzengrößenmuster
    .description = Bestimmte Kerzenmuster erkennen: großer Körper, kleiner Körper (Doji), lange Dochte
strategies-condition-candle-size-param-pattern = Mustertyp
    .description = Zu erkennendes Kerzenmuster
strategies-condition-candle-size-param-pattern-option-large-body = Großer Körper (starke Bewegung)
strategies-condition-candle-size-param-pattern-option-small-body = Kleiner Körper (Doji/Unentschlossenheit)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Langer oberer Docht (Ablehnung)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Langer unterer Docht (Unterstützung)
strategies-condition-candle-size-param-threshold = Größenschwelle %
    .description = Prozentuale Schwelle für die Mustererkennung

strategies-condition-consecutive-candles = Aufeinanderfolgende Kerzen
    .description = Aufeinanderfolgende grüne (bullische) oder rote (bärische) Kerzen mit Mindestgrößenfilter erkennen
strategies-condition-consecutive-candles-param-count = Kerzenanzahl
    .description = Anzahl der erforderlichen aufeinanderfolgenden Kerzen
strategies-condition-consecutive-candles-param-direction = Kerzenrichtung
    .description = Farbe/Richtung der aufeinanderfolgenden Kerzen
strategies-condition-consecutive-candles-param-direction-option-green = Grün (bullisch)
strategies-condition-consecutive-candles-param-direction-option-red = Rot (bärisch)
strategies-condition-consecutive-candles-param-minimum-change = Mindeständerung %
    .description = Mindeständerung in % für jede Kerze (filtert Rauschen)

strategies-condition-liquidity-level = Pool-Liquiditätsniveau
    .description = Pool-Liquidität in { -sol } prüfen (Einstieg: ausreichende Liquidität sicherstellen, Ausstieg: Liquiditätsabzug erkennen)
strategies-condition-liquidity-level-param-threshold = Liquiditätsschwelle ({ -sol })
    .description = Pool-Liquiditätsniveau in { -sol }
strategies-condition-liquidity-level-param-comparison = Vergleich
    .description = Wie die Pool-Liquidität mit der Schwelle verglichen wird
strategies-condition-liquidity-level-param-comparison-option-greater-than = Größer als (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = Größer oder gleich (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = Kleiner als ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = Kleiner oder gleich (≤)

strategies-condition-position-holding-time = Haltedauer der Position
    .description = Prüfen, wie lange eine Position gehalten wird (für Ausstiegsstrategien – zeitbasierte Ausstiege)
strategies-condition-position-holding-time-param-hours = Zeitschwelle (Stunden)
    .description = Dauer in Stunden seit Eröffnung der Position
strategies-condition-position-holding-time-param-comparison = Vergleich
    .description = Wie das Positionsalter mit der Schwelle verglichen wird
strategies-condition-position-holding-time-param-comparison-option-greater-than = Älter als (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = Mindestens (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Jünger als ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = Höchstens (≤)

strategies-condition-price-breakout = Preisausbruch
    .description = Erkennen, wenn der Preis über den Widerstand (Periodenhoch) oder unter die Unterstützung (Periodentief) ausbricht
strategies-condition-price-breakout-param-lookback = Rückblickzeitraum
    .description = Anzahl der Kerzen zur Bestimmung des Unterstützungs-/Widerstandsniveaus
strategies-condition-price-breakout-param-direction = Ausbruchsrichtung
    .description = Richtung des Ausbruchs
strategies-condition-price-breakout-param-direction-option-upward = Aufwärts (Widerstandsbruch)
strategies-condition-price-breakout-param-direction-option-downward = Abwärts (Unterstützungsbruch)
strategies-condition-price-breakout-param-confirmation = Bestätigung %
    .description = Wie weit über das Niveau hinaus der Ausbruch bestätigt wird (vermeidet Fehlsignale)

strategies-condition-price-change-percent = Preisänderung %
    .description = Prüfen, ob sich der Preis innerhalb eines Zeitraums um eine prozentuale Schwelle geändert hat
strategies-condition-price-change-percent-param-percentage = Änderungsschwelle %
    .description = Prozentuale Preisänderung, die auslöst (0,1–1000 %)
strategies-condition-price-change-percent-param-direction = Richtung
    .description = Richtung der Preisbewegung
strategies-condition-price-change-percent-param-direction-option-above = Gewinn (+%)
strategies-condition-price-change-percent-param-direction-option-below = Verlust (-%)
strategies-condition-price-change-percent-param-direction-option-within = Innerhalb der Spanne (±%)
strategies-condition-price-change-percent-param-time-value = Rückblick
    .description = Wie weit zurück die Änderung gemessen wird, in der im Feld gewählten Einheit (1–3600 s, 1–1440 Min., 1–720 Std.)
strategies-condition-price-change-percent-param-time-unit = Zeiteinheit
    .description = Zeiteinheit für den Rückblickzeitraum
strategies-condition-price-change-percent-param-time-unit-option-seconds = s
strategies-condition-price-change-percent-param-time-unit-option-minutes = Min.
strategies-condition-price-change-percent-param-time-unit-option-hours = Std.

strategies-condition-price-to-ma = Preis vs. gleitender Durchschnitt
    .description = Prüfen, ob der Preis über, unter oder innerhalb der Spanne seines einfachen gleitenden Durchschnitts liegt
strategies-condition-price-to-ma-param-period = MA-Periode
    .description = Anzahl der Kerzen für die Berechnung des gleitenden Durchschnitts
strategies-condition-price-to-ma-param-position = Position
    .description = Preisposition relativ zum MA
strategies-condition-price-to-ma-param-position-option-above = Über MA
strategies-condition-price-to-ma-param-position-option-below = Unter MA
strategies-condition-price-to-ma-param-position-option-within = Innerhalb der Spanne
strategies-condition-price-to-ma-param-distance = Abstand %
    .description = Mindestabstand zum MA (für ÜBER/UNTER) oder maximale Spanne (für INNERHALB)

strategies-condition-volume-spike = Volumenspitze
    .description = Volumenspitzen im Vergleich zum Durchschnittsvolumen erkennen (zeigt gestiegenes Interesse an)
strategies-condition-volume-spike-param-lookback = Rückblickzeitraum
    .description = Anzahl der Kerzen zur Berechnung des Durchschnittsvolumens
strategies-condition-volume-spike-param-multiplier = Volumenmultiplikator
    .description = Wievielfaches des Durchschnitts (z. B. 2,0 = 200 % des Durchschnitts)

strategies-condition-rsi = RSI (Relative-Stärke-Index)
    .description = Wird ausgelöst, wenn der RSI der Schlusskurse der Kerzen, nach Wilder geglättet, unter oder über einer Schwelle liegt. Liest nur Kerzen.
strategies-condition-rsi-param-period = RSI-Periode
    .description = Anzahl der Kerzen, über die der RSI berechnet wird; die Bedingung braucht eine Kerze mehr als diesen Wert
strategies-condition-rsi-param-comparison = Vergleich
    .description = Ob der RSI unter oder über der Schwelle liegen muss
strategies-condition-rsi-param-comparison-option-below = Unter (überverkauft)
strategies-condition-rsi-param-comparison-option-above = Über (überkauft)
strategies-condition-rsi-param-threshold = RSI-Schwelle
    .description = RSI-Wert von 1 bis 99 (üblich sind 30 für überverkauft und 70 für überkauft)

strategies-condition-ma-cross = Kreuzung gleitender Durchschnitte
    .description = Wird ausgelöst, wenn ein schneller gleitender Durchschnitt einen langsamen innerhalb der letzten Kerzen kreuzt. Liest nur Kerzen.
strategies-condition-ma-cross-param-ma-type = Durchschnittstyp
    .description = Exponentielle Durchschnitte gewichten neuere Kerzen stärker, einfache gewichten alle Kerzen gleich
strategies-condition-ma-cross-param-ma-type-option-ema = Exponentiell (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = Einfach (SMA)
strategies-condition-ma-cross-param-fast-period = Schnelle Periode
    .description = Anzahl der Kerzen im schnellen Durchschnitt
strategies-condition-ma-cross-param-slow-period = Langsame Periode
    .description = Anzahl der Kerzen im langsamen Durchschnitt; muss größer als die schnelle Periode sein
strategies-condition-ma-cross-param-direction = Kreuzungsrichtung
    .description = In welche Richtung der schnelle den langsamen Durchschnitt kreuzt
strategies-condition-ma-cross-param-direction-option-bullish = Bullisch (schnell kreuzt nach oben)
strategies-condition-ma-cross-param-direction-option-bearish = Bärisch (schnell kreuzt nach unten)
strategies-condition-ma-cross-param-within-candles = Kreuzung innerhalb
    .description = Wie viele der letzten Kerzen die Kreuzung enthalten dürfen; 1 bedeutet nur die letzte Kerze

strategies-condition-position-profit = Positionsgewinn
    .description = Wird ausgelöst, wenn der nicht realisierte Gewinn der offenen Position, gemessen am Poolpreis gegenüber dem Einstiegspreis, über oder unter einem Prozentwert liegt. Nur für Ausstiegsstrategien.
strategies-condition-position-profit-param-comparison = Vergleich
    .description = Ob der Gewinn mindestens oder höchstens der Schwelle entsprechen muss
strategies-condition-position-profit-param-comparison-option-above = Mindestens (≥)
strategies-condition-position-profit-param-comparison-option-below = Höchstens (≤)
strategies-condition-position-profit-param-threshold-pct = Gewinnschwelle %
    .description = Nicht realisierter Gewinn in Prozent des Einstiegspreises; negative Werte sind Verluste

strategies-condition-drawdown-from-peak = Rückgang vom Höchststand
    .description = Wird ausgelöst, wenn der Poolpreis um einen Prozentwert unter den höchsten Preis seit dem Einstieg gefallen ist und dieser Höchststand weit genug über dem Einstiegspreis lag. Liest nur Poolpreise. Nur für Ausstiegsstrategien.
strategies-condition-drawdown-from-peak-param-drop-pct = Rückgang vom Höchststand %
    .description = Wie weit der aktuelle Preis unter dem Höchststand liegen muss
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = Aktivierung nach Gewinn %
    .description = Der Höchststand muss mindestens so weit über dem Einstiegspreis gelegen haben, bevor die Bedingung auslösen kann; 0 bedeutet immer aktiv

strategies-condition-buy-pressure = Kaufdruck
    .description = Wird ausgelöst, wenn Käufe mindestens einen Anteil der Transaktionen in einem Zeitfenster ausmachen und das Fenster genug Transaktionen hat. Liest die Kauf- und Verkaufszahlen des Tokens.
strategies-condition-buy-pressure-param-window = Zeitfenster
    .description = Zeitraum, den die Kauf- und Verkaufszahlen abdecken
strategies-condition-buy-pressure-param-window-option-5m = 5 Minuten
strategies-condition-buy-pressure-param-window-option-1h = 1 Stunde
strategies-condition-buy-pressure-param-window-option-6h = 6 Stunden
strategies-condition-buy-pressure-param-window-option-24h = 24 Stunden
strategies-condition-buy-pressure-param-min-buy-share-pct = Mindestkaufanteil %
    .description = Käufe in Prozent aller Transaktionen im Fenster
strategies-condition-buy-pressure-param-min-transactions = Mindestanzahl Transaktionen
    .description = Fenster mit weniger Transaktionen lösen die Bedingung nie aus

## Shared by every condition

strategies-condition-param-timeframe = Zeitrahmen
    .description = Kerzengröße: die Dauer jeder Kerze, die die Bedingung liest (Strategie-Zeitrahmen, wenn nicht gesetzt)
strategies-condition-timeframe-option-1m = 1 Minute
strategies-condition-timeframe-option-5m = 5 Minuten
strategies-condition-timeframe-option-15m = 15 Minuten
strategies-condition-timeframe-option-1h = 1 Stunde
strategies-condition-timeframe-option-4h = 4 Stunden
strategies-condition-timeframe-option-12h = 12 Stunden
strategies-condition-timeframe-option-1d = 1 Tag

## Condition categories

strategies-condition-category-price-analysis = Preisanalyse
strategies-condition-category-candle-patterns = Kerzenmuster
strategies-condition-category-technical-indicators = Technische Indikatoren
strategies-condition-category-market-context = Marktkontext
strategies-condition-category-position-performance = Position und Performance
strategies-condition-category-volume-analysis = Volumenanalyse

## Validation errors

strategies-error-missing-parameter = Der Parameter „{ $field }“ fehlt
strategies-error-parameter-type = Der Parameter „{ $field }“ muss { $expected } sein
strategies-error-invalid-value = „{ $value }“ ist kein gültiger Wert für { $field }
strategies-error-missing-data = { $data } ist nicht verfügbar
strategies-error-no-candle-data = Der Zeitrahmen { $timeframe } hat keine Kerzendaten
strategies-error-insufficient-history = Nicht genug Historie für { $indicator }: { $available } s verfügbar, { $required } s erforderlich
strategies-error-insufficient-candles = Nicht genug Kerzen für { $indicator }: { $available } vorhanden, { $required } erforderlich
strategies-error-stale-candle-data = Die Kerzendaten für { $timeframe } sind veraltet: Ihr Alter von { $age } s überschreitet { $max } s
strategies-error-invalid-rule-tree = Ungültiger Regelbaum: { $reason }
strategies-error-evaluation-timeout = Strategieauswertung nach { $timeout } ms abgebrochen (Zeitüberschreitung)
strategies-error-condition-not-applicable = { $condition } kann nicht in Strategien vom Typ { $strategy } verwendet werden
strategies-error-invalid-rules = Die Regeln konnten nicht gelesen werden: { $reason }

strategies-error-field-average-volume = Durchschnittsvolumen
strategies-error-field-candle-open = Kerzeneröffnung
strategies-error-field-comparison = Vergleich
strategies-error-field-condition-type = Bedingungstyp
strategies-error-field-confirmation = Bestätigung
strategies-error-field-count = Anzahl
strategies-error-field-current-price = aktueller Preis
strategies-error-field-direction = Richtung
strategies-error-field-distance = Abstand
strategies-error-field-hours = Stunden
strategies-error-field-lookback = Rückblick
strategies-error-field-minimum-change = Mindeständerung
strategies-error-field-multiplier = Multiplikator
strategies-error-field-pattern = Muster
strategies-error-field-percentage = Prozentsatz
strategies-error-field-period = Periode
strategies-error-field-position = Position
strategies-error-field-threshold = Schwelle
strategies-error-field-time-unit = Zeiteinheit
strategies-error-field-time-value = Zeitwert
strategies-error-field-timeframe = Zeitrahmen
strategies-error-field-arm-after-profit = Aktivierung nach Gewinn
strategies-error-field-drop-percentage = Rückgangsprozent
strategies-error-field-entry-price = Einstiegspreis
strategies-error-field-fast-period = Schnelle Periode
strategies-error-field-minimum-buy-share = Mindestkaufanteil
strategies-error-field-minimum-transactions = Mindestanzahl Transaktionen
strategies-error-field-moving-average-type = Durchschnittstyp
strategies-error-field-peak-price = Höchstpreis
strategies-error-field-slow-period = Langsame Periode
strategies-error-field-window = Zeitfenster
strategies-error-field-within-candles = Kerzenzahl der Kreuzung

strategies-error-expected-boolean = ein Boolean
strategies-error-expected-number = eine Zahl
strategies-error-expected-string = ein String

strategies-error-data-current-price = Aktueller Preis
strategies-error-data-liquidity-data = Liquiditätsdaten
strategies-error-data-market-data = Marktdaten
strategies-error-data-ohlcv-data = OHLCV-Daten
strategies-error-data-position-data = Positionsdaten
strategies-error-data-peak-price = Höchstpreis
strategies-error-data-transaction-counts = Transaktionszahlen

strategies-error-indicator-consecutive-candles = aufeinanderfolgende Kerzen
strategies-error-indicator-moving-average = gleitenden Durchschnitt
strategies-error-indicator-price-breakout = Preisausbruch
strategies-error-indicator-price-change-lookback = Preisänderungs-Rückblick
strategies-error-indicator-volume-spike = Volumenspitze
strategies-error-indicator-moving-average-cross = Kreuzung gleitender Durchschnitte
strategies-error-indicator-rsi = RSI

strategies-error-rule-branch-node-missing-conditions = Verzweigungsknoten ohne Bedingungen
strategies-error-rule-branch-node-missing-operator = Verzweigungsknoten ohne Operator
strategies-error-rule-branch-node-must-have-at-least-one-child = Verzweigungsknoten muss mindestens einen Unterknoten haben
strategies-error-rule-invalid-rule-tree-structure = Ungültige Regelbaumstruktur
strategies-error-rule-leaf-node-missing-condition = Blattknoten ohne Bedingung
strategies-error-rule-not-operator-must-have-exactly-one-child = Der NOT-Operator muss genau einen Unterknoten haben
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = Eine Gruppe braucht mindestens eine eingeschaltete Bedingung
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = Der NOT-Operator darf keinen ausgeschalteten Unterknoten haben
strategies-error-rule-rules-nest-deeper-than-four-levels = Regeln dürfen höchstens vier Ebenen tief verschachtelt sein
strategies-error-rule-the-root-group-cannot-be-switched-off = Die oberste Gruppe kann nicht ausgeschaltet werden
