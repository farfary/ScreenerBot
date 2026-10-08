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
strategies-summary-period-seconds = Zeitraum: { $amount } s
strategies-summary-period-minutes = Zeitraum: { $amount } Min.
strategies-summary-period-hours = Zeitraum: { $amount } Std.

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
strategies-catalog-search-clear =
    .aria-label = Suche löschen
strategies-catalog-fold-all = Alle einklappen
strategies-catalog-unfold-all = Alle ausklappen
strategies-catalog-no-description = Keine Beschreibung verfügbar

## New strategy dialog

strategies-create-title = Neue Strategie erstellen
strategies-create-prompt = Wählen Sie den Strategietyp, den Sie erstellen möchten:
strategies-create-entry-name = Einstiegsstrategie
strategies-create-entry-description = Bedingungen festlegen, wann ein Token GEKAUFT wird
strategies-create-exit-name = Ausstiegsstrategie
strategies-create-exit-description = Bedingungen festlegen, wann ein Token VERKAUFT wird

## Delete dialog

strategies-delete-title = Strategie löschen
strategies-delete-message = Strategie „{ $name }“ löschen? Diese Aktion kann nicht rückgängig gemacht werden.

## Toasts. A message value is the title; `.message` is the body.

strategies-toast-fix-validation = Bitte beheben Sie die Validierungsfehler vor dem Speichern
strategies-toast-enabled = Strategie aktiviert
    .message = „{ $name }“ aktiviert
strategies-toast-disabled = Strategie deaktiviert
    .message = „{ $name }“ deaktiviert
strategies-toast-toggle-failed = Umschalten fehlgeschlagen
    .message = Strategiestatus konnte nicht aktualisiert werden
strategies-toast-load-failed = Laden fehlgeschlagen
    .message = Strategien konnten nicht vom Server geladen werden
strategies-toast-load-strategy-failed = Strategie konnte nicht geladen werden
strategies-toast-no-strategy = Keine Strategie erstellt
    .message = Fügen Sie mindestens eine Bedingung hinzu oder klicken Sie auf „Neue Strategie“, um zuerst eine Strategie zu erstellen
strategies-toast-no-conditions-save = Keine Bedingungen
    .message = Fügen Sie der Strategie vor dem Speichern mindestens eine Bedingung hinzu
strategies-toast-name-required = Name erforderlich
    .message = Geben Sie vor dem Speichern einen Strategienamen ein
strategies-toast-saved = Strategie gespeichert
    .message = „{ $name }“ erfolgreich gespeichert
strategies-toast-save-failed = Speichern fehlgeschlagen
    .message = Strategie konnte nicht in der Datenbank gespeichert werden
strategies-toast-no-strategy-validate = Keine Strategie zum Validieren
strategies-toast-no-conditions-validate = Keine Bedingungen
    .message = Fügen Sie vor dem Validieren mindestens eine Bedingung hinzu
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
strategies-toast-condition-added = Bedingung hinzugefügt
    .message = { $name } zur Strategie hinzugefügt

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
strategies-condition-price-change-percent-param-time-value = Zeitraum
    .description = Wert des Rückblickzeitraums (1–3600 für Sekunden, 1–1440 für Minuten, 1–720 für Stunden)
strategies-condition-price-change-percent-param-time-unit = Zeiteinheit
    .description = Zeiteinheit für den Rückblickzeitraum
strategies-condition-price-change-percent-param-time-unit-option-seconds = Sekunden
strategies-condition-price-change-percent-param-time-unit-option-minutes = Minuten
strategies-condition-price-change-percent-param-time-unit-option-hours = Stunden

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

## Shared by every condition

strategies-condition-param-timeframe = Zeitrahmen
    .description = Zu analysierender Kerzen-Zeitrahmen (Standard ist der Strategie-Zeitrahmen, wenn nicht gesetzt)
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

strategies-error-expected-boolean = ein Boolean
strategies-error-expected-number = eine Zahl
strategies-error-expected-string = ein String

strategies-error-data-current-price = Aktueller Preis
strategies-error-data-liquidity-data = Liquiditätsdaten
strategies-error-data-market-data = Marktdaten
strategies-error-data-ohlcv-data = OHLCV-Daten
strategies-error-data-position-data = Positionsdaten

strategies-error-indicator-consecutive-candles = aufeinanderfolgende Kerzen
strategies-error-indicator-moving-average = gleitenden Durchschnitt
strategies-error-indicator-price-breakout = Preisausbruch
strategies-error-indicator-price-change-lookback = Preisänderungs-Rückblick
strategies-error-indicator-volume-spike = Volumenspitze

strategies-error-rule-branch-node-missing-conditions = Verzweigungsknoten ohne Bedingungen
strategies-error-rule-branch-node-missing-operator = Verzweigungsknoten ohne Operator
strategies-error-rule-branch-node-must-have-at-least-one-child = Verzweigungsknoten muss mindestens einen Unterknoten haben
strategies-error-rule-invalid-rule-tree-structure = Ungültige Regelbaumstruktur
strategies-error-rule-leaf-node-missing-condition = Blattknoten ohne Bedingung
strategies-error-rule-not-operator-must-have-exactly-one-child = Der NOT-Operator muss genau einen Unterknoten haben
