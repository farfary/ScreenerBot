copy-skip-not-buy-swap = Wallet-Aktivität war kein Kauf
copy-skip-task-disabled = Aufgabe ist pausiert
copy-skip-mode-transition-required = Ausführungsmodus muss separat geändert werden
copy-skip-live-confirmation-required = Live-Ausführung erfordert Bestätigung
copy-skip-unsupported-sizing-mode = Dimensionierungsmodus wird noch nicht unterstützt
copy-skip-self-copy = Die Wallet gehört zu Ihren eigenen
copy-skip-target-below-minimum = Wallet-Trade unter dem Minimum
copy-skip-target-above-maximum = Wallet-Trade über dem Maximum
copy-skip-already-bought = Token bereits gekauft (nur einmal kaufen)
copy-skip-blacklisted = Token durch Risikokontrollen gesperrt
copy-skip-filter-required = Token hat die Filterung nicht bestanden
copy-skip-budget-exhausted = Aufgabenbudget aufgebraucht
copy-skip-token-cap-reached = Limit pro Token erreicht
copy-skip-below-minimum-size = Kopiergröße zu klein
copy-skip-invalid-sizing = Dimensionierung der Aufgabe ungültig
copy-skip-invalid-slippage = Slippage der Aufgabe ungültig
copy-skip-invalid-exit-policy = Ausstiegsregeln der Aufgabe ungültig
copy-skip-invalid-price = Kein verwendbarer Marktpreis
copy-skip-not-sell-swap = Wallet-Aktivität war kein Verkauf
copy-skip-exit-mode-disabled = Wallet-Verkauf ignoriert: Die Aufgabe verkauft nach eigenen Regeln
copy-skip-force-stopped = Trading ist per Sofortstopp angehalten
copy-skip-copy-position-not-found = Keine Position dieser Aufgabe vorhanden
copy-skip-position-user-only = Position wird von Ihnen verwaltet
copy-skip-position-management-mismatch = Position folgt den Copy-Verkäufen nicht mehr
copy-skip-latency-kill-switch = Automatisch pausiert: Trades zu spät erkannt
copy-skip-claim-reconciled-abandoned = Unterbrochene Live-Übermittlung ohne Wiederholung geschlossen
copy-skip-stale-observation = Nach Ausfallzeit nachgeholt, zu alt zum Kopieren
copy-skip-unknown-observation-time = Nachgeholter Trade hat keine Blockzeit
copy-skip-entry-blocked = Einstieg blockiert

copy-entry-block-force-stopped = Trading ist per Sofortstopp angehalten
copy-entry-block-loss-limit = Verlustlimit blockiert neue Einstiege
copy-entry-block-connectivity = Erforderliche Dienste sind nicht verfügbar
copy-entry-block-position-limit = Limit offener Positionen erreicht
copy-entry-block-already-open = Eine Position ist bereits offen
copy-entry-block-reentry-cooldown = Abkühlphase für erneuten Token-Einstieg
copy-entry-block-open-cooldown = Globale Abkühlphase für Einstiege
copy-entry-block-entry-reserved = Ein anderer Einstieg wird verarbeitet
copy-entry-block-blacklisted = Token durch Risikokontrollen gesperrt
copy-entry-block-check-failed = Eine Sicherheitsprüfung konnte nicht abgeschlossen werden

copy-pause-user = Von Ihnen pausiert
copy-pause-latency-kill-switch = Automatisch pausiert: Trades kamen durchschnittlich { $average } s zu spät an (Limit { $threshold } s)
copy-pause-watch-detached = Automatisch pausiert: Die Wallet wird nicht mehr beobachtet
copy-pause-watch-budget-exceeded = Pausiert: Diese Wallet hat ihr Limit von { $limit } Signaturen pro Beobachtungsprüfung erreicht, bevor sie aufgeholt hat
copy-pause-helius-unavailable = Pausiert: Wallet-Prüfungen über Helius sind fehlgeschlagen
copy-pause-watch-processing-failed = Pausiert: Wallet-Aktivität konnte nicht verarbeitet werden
copy-pause-unspecified = Pausiert

copy-pause-short-user = durch Sie
copy-pause-short-latency-kill-switch = zu langsam
copy-pause-short-watch-detached = Beobachtung verloren
copy-pause-short-watch-budget-exceeded = Beobachtungslimit
copy-pause-short-helius-unavailable = Beobachtungsanbieter
copy-pause-short-watch-processing-failed = Beobachtungsverarbeitung
copy-state-paused = Pausiert
copy-state-paused-reason = Pausiert · { $reason }

copy-readiness-history = Paper-Verlauf
copy-readiness-history-met =
    { $count ->
        [one] { $count } geschlossene Paper-Runde, { $needed } erforderlich
       *[other] { $count } geschlossene Paper-Runden, { $needed } erforderlich
    }
copy-readiness-history-short = { $count } von { $needed } geschlossenen Paper-Runden
copy-readiness-profit = Im Paper-Modus profitabel
copy-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } realisiert in { $count } Runde, { $wins } gewonnen
       *[other] { $realized } { -sol } realisiert in { $count } Runden, { $wins } gewonnen
    }
copy-readiness-latency = Trades rechtzeitig erkannt
copy-readiness-latency-detail = p95-Ankunft { $p95 } s, Limit { $limit } s
copy-readiness-latency-none = Noch keine Ankunftsstichproben
copy-readiness-priced = Jeder Bestand mit Preis
copy-readiness-priced-ok = Jeder offene Paper-Bestand hat einen Pool-Preis
copy-readiness-priced-missing =
    { $count ->
        [one] { $count } offener Bestand ohne Pool-Preis
       *[other] { $count } offene Bestände ohne Pool-Preis
    }
copy-readiness-runtime = Live-Ausführung verfügbar
copy-readiness-runtime-ok = Einrichtung und Sicherheitsschranken erlauben Live-Kopien

copy-live-block-setup-incomplete = Schließen Sie zuerst die Wallet- und RPC-Einrichtung ab
copy-live-block-force-stop = Der Not-Stopp ist aktiv
copy-live-block-copy-trading-disabled = Copy-Verarbeitung ist global pausiert
copy-live-block-unavailable = Live-Ausführung ist nicht verfügbar

## Task state, mode and exit labels.

copy-state-system-paused = Global pausiert
copy-state-force-stopped = Sofort gestoppt
copy-state-entries-blocked = Einstiege blockiert
copy-state-running-live = Läuft
copy-state-running-paper = Läuft
copy-mode-paper = Paper
copy-mode-live = Live
copy-exit-mode-buy-only = Meine Ausstiegsregeln
copy-exit-mode-mirror = Wallet-Verkäufe spiegeln
copy-exit-mode-hybrid = Wallet-Verkäufe und meine Regeln
copy-exit-target-sell = Wallet hat verkauft
copy-exit-stop-loss = Stop-Loss
copy-exit-trailing-stop = Trailing-Stop
copy-exit-take-profit = Take Profit
copy-exit-time-override = Zeitregel
copy-exit-manual = Manuell geschlossen

## Shared wording

copy-request-failed = Anfrage fehlgeschlagen
copy-keep-paused = Pausiert lassen
copy-paused-suffix = · pausiert
copy-mode-paused = { $mode } · pausiert
copy-task-ref = „{ $name }“ ({ $mode })
copy-metric-realized-pnl = Realisierte GuV
copy-metric-unrealized-pnl = Unrealisierte GuV
copy-metric-win-rate = Gewinnquote
copy-metric-budget-spent = Budget verbraucht
copy-metric-median-arrival = Mediane Ankunft
copy-metric-open-holdings = Offene Bestände
copy-record-won-lost = { $won } gewonnen · { $lost } verloren
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = Ausführungen
copy-kind-exits = Ausstiege
copy-kind-skips = Übersprungen
copy-kind-errors = Fehler
copy-field-per-trade-cap = Limit pro Trade
copy-field-per-token-cap = Limit pro Token
copy-field-total-budget = Gesamtbudget
copy-field-slippage = Slippage
copy-rules-wallet-sells-only = Nur Wallet-Verkäufe
copy-filter-copy-setting-required = Copy-Einstellung (erforderlich)
copy-filter-copy-setting-not-required = Copy-Einstellung (nicht erforderlich)
copy-count-closed-rounds =
    { $count ->
        [one] { $count } geschlossene Runde
       *[other] { $count } geschlossene Runden
    }
copy-count-open-holdings =
    { $count ->
        [one] { $count } offener Bestand
       *[other] { $count } offene Bestände
    }
copy-unrealized-partial =
    { $priced ->
        [one] { $priced } Bestand mit Preis · { $unpriced } ohne Preis
       *[other] { $priced } Bestände mit Preis · { $unpriced } ohne Preis
    }
copy-unrealized-unpriced =
    { $count ->
        [one] { $count } Bestand ohne Preis
       *[other] { $count } Bestände ohne Preis
    }
copy-range-24h = 24 Std.
copy-range-7d = 7 T.
copy-range-30d = 30 T.
copy-range-all = Alle
copy-range-label =
    .aria-label = Zeitraum

## Page strip

copy-page-title = Copy-Trading
copy-page-beta = Beta
copy-strip-loading = Wird geladen
copy-strip-unavailable = Nicht verfügbar
copy-strip-pause-all = Alle pausieren
copy-strip-resume = Verarbeitung fortsetzen
copy-strip-settings = Einstellungen
copy-strip-add-wallet = Wallet hinzufügen
copy-strip-paused-globally = Global pausiert · keine neuen Kopien, Ausstiege laufen weiter
copy-strip-force-stopped = Sofort gestoppt · nichts wird kopiert
copy-strip-loss-limit = Verlustlimit · neue Einstiege blockiert, Ausstiege laufen weiter
copy-strip-idle-paused =
    { $count ->
        [one] Inaktiv · { $count } Aufgabe pausiert
       *[other] Inaktiv · { $count } Aufgaben pausiert
    }
copy-strip-idle-empty = Inaktiv · noch keine Aufgaben
copy-strip-processing = Verarbeitung · { $paper } Paper
copy-strip-processing-live = Verarbeitung · { $live } Live · { $paper } Paper
copy-figures-label =
    .aria-label = Copy-Trading-Summen
copy-figure-marked-at-pool = Bewertet zum Pool-Preis
copy-figure-across-tasks = Über alle Aufgaben
copy-figure-budget-lifetime = Gesamtausgaben aktivierter Aufgaben
copy-figure-budget-none = Keine aktivierten Aufgaben
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
        [one] { $count } Trade
       *[other] { $count } Trades
    }
copy-figure-arrival-none = Keine Stichproben aktivierter Aufgaben

## Page frame

copy-load-failed = Copy-Trading konnte nicht geladen werden: { $error }
copy-resume-all-title = Copy-Verarbeitung fortsetzen
copy-resume-all-message =
    { $count ->
        [one] { $count } Live-Aufgabe übermittelt echte Swaps, sobald ihre Wallet wieder handelt.
       *[other] { $count } Live-Aufgaben übermitteln echte Swaps, sobald ihre Wallets wieder handeln.
    }
copy-toast-resumed-all = Copy-Verarbeitung fortgesetzt
copy-toast-paused-all = Gesamte Copy-Verarbeitung pausiert
copy-toast-global-failed = Copy-Verarbeitung konnte nicht geändert werden

## Onboarding

copy-onboarding-title = Kopieren Sie Wallets, denen Sie vertrauen – und testen Sie sie zuerst im Paper-Modus
copy-onboarding-body = Jede Aufgabe startet im Paper-Modus: Trades der Ziel-Wallet werden zum Pool-Preis mit Ihrer Slippage und Ihren Gebühren simuliert, und Ihre Ausstiegsregeln laufen auf dem Paper-Buch. Schalten Sie Live pro Wallet scharf, sobald ihre Paper-Ergebnisse es rechtfertigen.
copy-onboarding-add = Erste Wallet hinzufügen
copy-onboarding-observe = Beobachten
copy-onboarding-observe-detail = Erkennt die Swaps der Wallet, ohne { -sol } auszugeben.
copy-onboarding-evaluate = Auswerten
copy-onboarding-evaluate-detail = Paper-GuV, Gewinnquote, übersprungene Trades, Erkennungsgeschwindigkeit und Slippage prüfen.
copy-onboarding-arm = Scharfschalten
copy-onboarding-arm-detail = Bereitschaftsprüfungen bestehen, dann echte Swaps aktivieren.

## Wallet list

copy-list-label =
    .aria-label = Kopierte Wallets
copy-list-title = Wallets
copy-list-compare = Vergleichen
copy-list-sort-label = Wallets sortieren
copy-list-count = { $active } aktiv · { $total } gesamt
copy-sort-pnl = GuV
copy-sort-state = Status
copy-sort-name = Name
copy-compare-label =
    .aria-label = Wallets vergleichen

## Dialog chrome

copy-dialog-close =
    .aria-label = Schließen
copy-editor-title-add = Wallet hinzufügen
copy-editor-sub-add = Neue Aufgaben starten im Paper-Modus
copy-arm-title = Live-Kopieren scharfschalten
copy-arm-sub = Echte Swaps aus Ihrer Wallet
copy-arm-keep-paper = Paper beibehalten
copy-arm-confirm = Live scharfschalten
copy-profile-title = Wallet-Profil
copy-profile-sub = Was dieser Bot von der Wallet gesehen hat

## Settings dialog

copy-settings-title = Copy-Trading-Einstellungen
copy-settings-subtitle = Globale Richtlinie für alle Aufgaben
copy-settings-filter-warning = Mit der Standard-Filterung lehnt dies fast jeden Token ab, sodass nichts kopiert wird. Lassen Sie es ausgeschaltet, es sei denn, Ihre Filter lassen die Tokens durch, die Ihre Wallets handeln.
copy-settings-unit-seconds = Sekunden
copy-settings-unit-trades = Trades
copy-settings-unit-tasks = Aufgaben
copy-settings-unit-closed-rounds = geschlossene Runden
copy-settings-save = Einstellungen speichern
copy-settings-load-failed = Copy-Einstellungen konnten nicht geladen werden
copy-settings-saved = Copy-Trading-Einstellungen gespeichert

## Workspace

copy-tab-overview = Übersicht
copy-tab-holdings = Bestände
copy-tab-activity = Aktivität
copy-tab-rules = Regeln
copy-tab-execution = Ausführung
copy-tabs-label = Aufgabenansichten
copy-workspace-select = Wählen Sie eine Wallet, um ihren Arbeitsbereich zu öffnen.
copy-workspace-loading = Aufgabe wird geladen…
copy-workspace-load-failed = Diese Aufgabe konnte nicht geladen werden: { $error }

copy-state-detail-paper = Läuft im Paper-Modus · Trades werden simuliert, nichts wird ausgegeben
copy-state-detail-live = Läuft live · Wallet-Trades werden mit echten Swaps kopiert
copy-state-detail-system-paused = Wartet · Copy-Verarbeitung ist global pausiert, Ausstiege laufen weiter
copy-state-detail-entries-blocked = Einstiege durch das Verlustlimit blockiert · Ausstiege laufen weiter
copy-state-detail-force-stopped = Sofort gestoppt · nichts wird kopiert

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = Beim Fortsetzen bleibt dasselbe Limit bestehen, sodass die Aufgabe erneut pausiert, solange Trades noch zu spät ankommen. Prüfen Sie den RPC-Stream oder erhöhen Sie das Ankunftslimit in den Einstellungen.
copy-paused-resume-detached = Beim Fortsetzen wird die Wallet wieder beobachtet.
copy-paused-holdings-rules =
    { $count ->
        [one] Ihre Ausstiegsregeln schließen weiterhin den offenen Bestand ({ $count }).
       *[other] Ihre Ausstiegsregeln schließen weiterhin die offenen Bestände ({ $count }).
    }
copy-paused-holdings-mirror =
    { $count ->
        [one] Die Verkäufe der Wallet schließen weiterhin den offenen Bestand ({ $count }).
       *[other] Die Verkäufe der Wallet schließen weiterhin die offenen Bestände ({ $count }).
    }
copy-paused-holdings-hybrid =
    { $count ->
        [one] Die Verkäufe der Wallet und Ihre Ausstiegsregeln schließen weiterhin den offenen Bestand ({ $count }).
       *[other] Die Verkäufe der Wallet und Ihre Ausstiegsregeln schließen weiterhin die offenen Bestände ({ $count }).
    }

copy-watch-state-catching-up = Wallet-Beobachtung: Holt auf. Die Prüfung dieser Wallet erfolgt über { -helius }.
copy-watch-state-watching = Wallet-Beobachtung: Aktiv. Die Prüfung dieser Wallet erfolgt über { -helius }.
copy-watch-last-check = Letzte Prüfung { $ago }.
copy-watch-recovery-active = Wallet-Beobachtung aktiv
copy-watch-recovery-catching-up = Wallet-Beobachtung holt auf
copy-watch-recovery-still-paused = Die Copy-Aufgabe ist weiterhin pausiert. Setzen Sie das Kopieren fort, wenn Sie bereit sind.
copy-watch-recovery-title = Wallet-Beobachtung wiederherstellen
copy-watch-recovery-processing-failed = Wallet-Aktivität konnte nicht verarbeitet werden. Der gespeicherte Fortschritt bleibt erhalten. Versuchen Sie es erneut, sobald das Problem behoben ist.
copy-watch-recovery-provider-failed = Prüfungen über { -helius } sind fehlgeschlagen. Der gespeicherte Fortschritt bleibt erhalten. Versuchen Sie es erneut, sobald der Anbieter verfügbar ist.
copy-watch-recovery-budget-intro = Diese Wallet hat mehr Aktivität, als die aktuelle Beobachtung prüfen kann. Wählen Sie, wie es weitergehen soll.
copy-watch-approve = Aufholen über { -helius } versuchen
copy-watch-approve-help = Setzt beim gespeicherten Fortschritt fort. Kann mehr { -helius }-Credits verbrauchen und dennoch zurückfallen.
copy-watch-approve-unavailable = Das Aufholen über { -helius } ist nicht verfügbar. Konfigurieren Sie einen aktivierten { -helius }-RPC-Endpunkt, um fortzufahren, ohne ungeprüfte Aktivität zu überspringen.
copy-watch-no-provider = Für diese Beobachtung wird kein Anbieter zum Aufholen unterstützt.
copy-watch-budget-label = Geprüfte Signaturen pro Prüfung
copy-watch-budget-hint = Oder ungeprüfte Aktivität überspringen und ab jetzt fortsetzen. Wählen Sie { $min }–{ $max } Signaturen pro Prüfung; ein höheres Limit kann mehr RPC-Aufrufe verbrauchen.
copy-watch-ack = Mir ist bewusst, dass verpasste Aktivität nicht kopiert wird.
copy-watch-toast-range = Wählen Sie zwischen { $min } und { $max } Signaturen pro Abfrage in Schritten von { $step } Signaturen
copy-watch-toast-ack = Bestätigen Sie, dass Signaturen seit der letzten abgeschlossenen Prüfung übersprungen werden
copy-watch-resumed = Wallet-Beobachtung ab jetzt fortgesetzt; Copy-Aufgabe bleibt pausiert
copy-watch-resume-failed = Wallet-Beobachtung konnte nicht fortgesetzt werden
copy-watch-retry-started = Wallet-Beobachtung wird ab dem gespeicherten Fortschritt erneut versucht; Copy-Aufgabe bleibt pausiert
copy-watch-retry-failed = Wallet-Beobachtung konnte nicht erneut versucht werden
copy-watch-approve-title = Aufholen über { -helius } für diese Wallet erlauben
copy-watch-approve-message = { -helius } kann erfolgreiche Solana-Transaktionen ab dem gespeicherten Fortschritt prüfen, ohne das ungeprüfte Intervall zu überspringen. Derzeit werden 10 Credits pro 100 zurückgegebene vollständige Transaktionen berechnet (aufgerundet), mit mindestens 10 Credits pro Anfrage. Eine Prüfung kann mehrere Anfragen auslösen; Nutzung und Preise des Anbieters können variieren. Das Kopieren bleibt pausiert, bis Sie es separat fortsetzen.
copy-watch-approve-confirm = Für diese Wallet erlauben
copy-watch-approved = Wallet-Beobachtung ab dem gespeicherten Fortschritt gestartet; Copy-Aufgabe bleibt pausiert
copy-watch-restore-failed = Wallet-Beobachtung konnte nicht wiederhergestellt werden

copy-action-pause = Pausieren
copy-action-resume = Fortsetzen
copy-action-resume-copy = Kopieren fortsetzen
copy-action-resume-from-now = Ab jetzt fortsetzen
copy-action-retry-watch = Wallet-Beobachtung wiederholen
copy-action-return-paper = Zurück zu Paper
copy-action-edit-rules = Regeln bearbeiten
copy-action-clone = Klonen
copy-action-profile = Wallet-Profil
copy-resume-live-title = Live-Kopieren fortsetzen
copy-resume-live-message = „{ $name }“ übermittelt echte Swaps aus Ihrer Wallet, sobald diese Wallet wieder handelt.
copy-resume-live-confirm = Live fortsetzen
copy-task-resumed = Aufgabe fortgesetzt
copy-task-paused = Aufgabe pausiert
copy-task-state-failed = Aufgabenstatus konnte nicht geändert werden
copy-return-paper-message = Neue Kopien von „{ $name }“ werden wieder simuliert, ohne { -sol } auszugeben.
copy-return-paper-cancel = Live beibehalten
copy-task-returned-paper = Aufgabe zurück im Paper-Modus
copy-mode-change-failed = Ausführungsmodus konnte nicht geändert werden
copy-delete-title = Copy-Aufgabe löschen
copy-delete-message = „{ $name }“ löschen? Ihre Entscheidungen und Paper-Ergebnisse werden entfernt, und die Wallet wird für diese Aufgabe nicht mehr beobachtet.
copy-delete-confirm = Aufgabe löschen
copy-delete-cancel = Aufgabe behalten
copy-task-deleted = Copy-Aufgabe gelöscht
copy-task-delete-failed = Copy-Aufgabe konnte nicht gelöscht werden

## Overview tab

copy-overview-results = Ergebnisse
copy-analytics-load-failed = Analysen konnten nicht geladen werden: { $error }
copy-analytics-loading = Analysen werden geladen…
copy-exit-bucket =
    { $count ->
        [one] { $count } Verkauf · { $pnl }
       *[other] { $count } Verkäufe · { $pnl }
    }
copy-overview-average-win = Durchschnittsgewinn
copy-overview-average-loss = Durchschnittsverlust { $amount }
copy-overview-profit-factor = Profitfaktor
copy-overview-profit-factor-note = Bruttogewinne ÷ Bruttoverluste
copy-overview-average-hold = Durchschnittliche Haltedauer
copy-overview-average-hold-note = Einstieg bis Ausstieg
copy-overview-best-round = Beste Runde
copy-overview-worst-round = Schlechteste { $amount }
copy-overview-curve-title = Kumulierte GuV
copy-overview-exits-title = Verkäufe nach Ausstieg
copy-overview-skips-title = Warum Trades übersprungen wurden
copy-book-title-live = Live-Buch
copy-book-title-paper = Paper-Buch
copy-book-all-time = Gesamtzeitraum
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
        [one] Kauf
       *[other] Käufe
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
        [one] Ausstieg nach Ihren Regeln
       *[other] Ausstiege nach Ihren Regeln
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
        [one] Wallet-Verkauf
       *[other] Wallet-Verkäufe
    }
copy-book-manual-closes = <strong>{ $count }</strong> manuell geschlossen
copy-book-skipped = <strong>{ $count }</strong> übersprungen
copy-book-failed = <strong>{ $count }</strong> fehlgeschlagen
copy-book-closed = { $count } geschlossen
copy-book-budget-note = { $mode }-Ausgaben von { $total } · { $remaining } übrig
copy-check-passed = bestanden
copy-check-not-passed = nicht bestanden
copy-readiness-title = Vor dem Live-Gang
copy-readiness-live-note = Diese Aufgabe handelt live. Setzen Sie sie in der Kopfzeile oben auf Paper zurück.
copy-readiness-all-pass = Alle Prüfungen bestanden.
copy-readiness-needs-review = Das Scharfschalten erfordert eine ausdrückliche Prüfung dessen, was nicht bereit ist.
copy-readiness-arm = Prüfen und live scharfschalten

## Rules tab and review

copy-rules-title = Geltende Regeln
copy-rules-size-ratio = { $pct } des Wallet-Trades
copy-rules-size-fixed = { $amount } pro Kopie
copy-rules-target-any = Beliebige Größe
copy-rules-target-min = Mindestens { $amount }
copy-rules-target-max = Höchstens { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = Aufgaben-Überschreibung · Trader { $value }
copy-rules-source-default = Trader-Standard
copy-rules-not-used = Nicht verwendet: Die Verkäufe der Wallet entscheiden
copy-rules-col-rule = Regel
copy-rules-col-applies = Gilt
copy-rules-col-source = Quelle
copy-rules-budget-note = { $spent } ausgegeben im { $mode }-Modus · { $remaining } übrig
copy-rules-token-copies =
    { $count ->
        [one] Etwa { $count } volle Kopie eines Tokens
       *[other] Etwa { $count } volle Kopien eines Tokens
    }
copy-rules-sizing = Dimensionierung
copy-rules-copy-size = Kopiergröße
copy-rules-entry-filters = Einstiegsfilter
copy-rules-target-size = Größe des Wallet-Trades
copy-rules-repeat-buys = Wiederholte Käufe
copy-rules-repeat-first-only = Nur der erste Kauf jedes Tokens
copy-rules-repeat-every = Jeder Kauf, bis zum Limit pro Token
copy-rules-filter-pass = Filterung bestanden
copy-rules-filter-required = Erforderlich
copy-rules-filter-not-required = Nicht erforderlich
copy-rules-filter-task-override = Aufgaben-Überschreibung
copy-rules-exits = Ausstiege
copy-rules-exits-inactive = Bestände werden nur verkauft, wenn die Wallet verkauft; die Regeln unten laufen in diesem Modus nicht.

## Exit rules

copy-rule-status = Status
copy-rule-on = An
copy-rule-off = Aus
copy-rule-unit-seconds = Sekunden
copy-rule-unit-minutes = Minuten
copy-rule-stop-loss-threshold = Verkauft bei einem Verlust von
copy-rule-stop-loss-min-hold = Nicht vor einer Haltedauer von
copy-rule-no-minimum = Kein Minimum
copy-rule-partial-exits = Teilausstiege
copy-rule-partial-allowed = Erlaubt
copy-rule-partial-full-only = Nur vollständiger Ausstieg
copy-rule-partial-size = Größe des Teilausstiegs
copy-rule-trailing-activation = Aktiviert sich bei einem Gewinn von
copy-rule-trailing-distance = Verkauft unterhalb des Höchststands um
copy-rule-take-profit-target = Verkauft bei einem Gewinn von
copy-rule-time-duration = Prüft nach einer Haltedauer von
copy-rule-time-threshold = Verkauft, solange die GuV höchstens beträgt
copy-preset-inherit = Trader-Standards
copy-preset-conservative = Konservativ
copy-preset-balanced = Ausgewogen
copy-preset-aggressive = Aggressiv
copy-preset-custom = Benutzerdefiniert
copy-validate-stop-loss = Der Stop-Loss muss über 0 % und höchstens bei 100 % liegen.
copy-validate-partial-size = Die Größe des Teilausstiegs muss zwischen 0 % und 100 % liegen.
copy-validate-min-hold = Die Mindesthaltedauer muss eine ganze Zahl von Sekunden sein.
copy-validate-trailing-activation = Die Trailing-Aktivierung muss über 0 % und höchstens bei 100 % liegen.
copy-validate-trailing-distance = Der Trailing-Abstand muss über 0 % und höchstens bei 100 % liegen.
copy-validate-take-profit = Take Profit muss über 0 % liegen.
copy-validate-time-duration = Die Zeitregel benötigt eine Dauer größer als null.
copy-validate-time-threshold = Der Schwellenwert der Zeitregel ist ein Verlust: Verwenden Sie 0 % oder eine negative Zahl.
copy-warning-mirror = Nur die Verkäufe der Wallet schließen Bestände: Kein Stop-Loss schützt sie, und ein Token, den die Wallet nie verkauft, bleibt im Bestand.
copy-warning-no-rules = Keine Ausstiegsregel ist aktiv und Wallet-Verkäufe werden ignoriert: Bestände werden nie verkauft.
copy-warning-no-stop-loss = Kein Stop-Loss aktiv: Ein fallender Token wird gehalten, bis eine andere Regel oder die Wallet verkauft.
copy-warning-stop-delay = Der Stop-Loss wartet nach jedem Kauf { $hold }: Ein Token, der schneller fällt, wird deutlich unter { $threshold } geschlossen.
copy-warning-take-profit-cost = Take Profit bei { $target } deckt den Verkauf nicht ab ({ $slippage } Slippage und { $fee } Swap-Gebühr), sodass Runden mit Verlust geschlossen werden.
copy-warning-trailing-distance = Der Trailing-Abstand ist mindestens so groß wie der Aktivierungsgewinn, sodass ein aktivierter Trail unter dem Einstieg verkaufen kann.

## Execution tab

copy-execution-title = Ausführungsqualität
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = Beliebig
copy-execution-limit-on =
    { $count ->
        [one] Pausiert bei durchschnittlich mehr als { $limit } über { $count } Trade
       *[other] Pausiert bei durchschnittlich mehr als { $limit } über { $count } Trades
    }
copy-execution-limit-off = Notabschaltung aus
copy-execution-arrival-samples =
    { $count ->
        [one] { $count } Trade in Echtzeit erfasst
       *[other] { $count } Trades in Echtzeit erfasst
    }
copy-execution-p95 = p95-Ankunft
copy-execution-median-slippage = Median-Slippage
copy-execution-slippage-samples =
    { $count ->
        [one] { $count } gemessene Ausführung
       *[other] { $count } gemessene Ausführungen
    }
copy-execution-worst-slippage = Schlechteste Slippage
copy-execution-average-slippage = Durchschnitt { $amount }
copy-execution-delay-title = Erkennungsverzögerung
copy-execution-delay-note = Zeit vom Block der Wallet bis zur Erkennung des Trades durch diesen Bot. Nachholungen nach Ausfallzeiten sind ausgenommen.
copy-execution-delay-limit = Balken über dem Ankunftslimit von { $limit } sind gelb.
copy-execution-fastest = Schnellste
copy-execution-average = Durchschnitt
copy-execution-slowest = Langsamste
copy-execution-fill-title = Ausführung gegenüber der Wallet
copy-execution-fill-note = Positiv bedeutet schlechter als die Wallet: bei einem Kauf mehr bezahlt, bei einem gespiegelten Verkauf weniger erhalten. Eine Paper-Ausführung eines Tokens ohne Pool-Preis wird zum Trade der Wallet selbst bepreist, misst also nichts und bleibt unberücksichtigt.
copy-execution-samples = Stichproben
copy-execution-median = Median
copy-execution-worst = Schlechteste
copy-execution-decisions = Entscheidungen im Zeitraum

## Compare view

copy-compare-title = Wallets vergleichen
copy-compare-back = Zurück zur Wallet
copy-compare-load-failed = Vergleich konnte nicht geladen werden: { $error }
copy-compare-loading = Vergleich wird geladen…
copy-compare-empty = Keine Aufgaben zum Vergleichen.
copy-compare-curve-title = Kumulierte realisierte GuV
copy-table-wallet = Wallet
copy-table-mode = Modus
copy-table-rounds = Runden
copy-table-realized = Realisiert
copy-table-profit-factor = Profitfaktor
copy-table-average-hold = Ø Haltedauer
copy-table-median-slippage = Median-Slippage

## Charts

copy-chart-curve-label = Kumulierte GuV { $amount } { -sol }
copy-chart-compare-label = Kumulierte GuV nach Aufgabe
copy-chart-empty-curve = In diesem Zeitraum noch keine geschlossenen Runden.
copy-chart-empty-bars = In diesem Zeitraum wurde nichts erfasst.
copy-chart-empty-histogram = Keine Ankunftsstichproben in diesem Zeitraum.
copy-chart-empty-compare = Keine geschlossenen Runden zum Vergleichen in diesem Zeitraum.
copy-chart-histogram-title = { $count } von { $total }

## Wallet profile

copy-profile-copy = Diese Wallet kopieren
copy-profile-copy-other = Mit anderen Regeln kopieren
copy-profile-loading = Wallet-Profil wird geladen…
copy-profile-watch-title = Beobachtung
copy-profile-watched = Beobachtet
copy-profile-watch-resume-hint = Das Fortsetzen einer Aufgabe beobachtet sie wieder
copy-profile-watch-add-hint = Das Hinzufügen einer Aufgabe startet die Beobachtung
copy-profile-stream = Stream
copy-profile-subscribed = Abonniert
copy-profile-not-subscribed = Nicht abonniert
copy-profile-sources =
    { $count ->
        [one] { $count } Quelle
       *[other] { $count } Quellen
    }
copy-profile-last-activity = Letzte Aktivität
copy-profile-last-error = Letzter Fehler
copy-profile-own-wallet = Dies ist eine Ihrer eigenen Wallets; sie zu kopieren wird abgelehnt.
copy-profile-observed-title = Beobachtete Trades
copy-profile-observed-none = Bisher keine Trades dieser Wallet in diesem Bot. Eine Paper-Aufgabe beobachtet sie, ohne { -sol } auszugeben.
copy-profile-swaps-seen = Erkannte Swaps
copy-profile-swaps-seen-note = Unterschiedliche Wallet-Swaps über Ihre Aufgaben hinweg
copy-profile-buys-sells = Käufe / Verkäufe
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = Gehandelte Tokens
copy-profile-first-seen = Erstmals gesehen
copy-profile-last-seen = Zuletzt gesehen
copy-profile-tasks-title = Ihre Aufgaben für diese Wallet
copy-table-task = Aufgabe

## Arm live dialog

copy-arm-acks-left =
    { $count ->
        [one] Noch { $count } Bestätigung anzukreuzen
       *[other] Noch { $count } Bestätigungen anzukreuzen
    }
copy-arm-readiness-title = Bereitschaft laut Paper-Buch
copy-arm-exposure-title = Exposure
copy-arm-per-copy = Pro Kopie
copy-arm-budget-left-value = { $left } von { $total } { -sol }
copy-arm-budget-left = Verbleibendes Live-Budget
copy-arm-budget-left-note = Paper-Ausgaben werden separat gezählt und verbrauchen es nicht
copy-arm-exits = Ausstiege
copy-arm-stop-note = Nicht vor einer Haltedauer von { $hold }: Bei schnellerem Fall wird tiefer geschlossen
copy-arm-shared = Diese Wallet wird auch von { $tasks } kopiert: Jede Aufgabe kopiert ihre Trades mit eigenem Budget.
copy-arm-unavailable = Live-Ausführung ist derzeit nicht verfügbar; siehe die letzte Prüfung.
copy-arm-ack-real-sol = Echte { -sol }: Diese Aufgabe kann bis zu { $budget } { -sol } aus Ihrer Wallet ausgeben, höchstens { $trade } { -sol } pro Kopie.
copy-arm-ack-fees = Live-Kopien zahlen echte Netzwerkgebühren und Slippage; Paper-Ergebnisse versprechen keine Live-Ergebnisse.
copy-arm-ack-unready = Einige Bereitschaftsprüfungen wurden nicht bestanden. Diese Aufgabe trotzdem scharfschalten.
copy-arm-lead = „{ $name }“ kopiert die Trades dieser Wallet mit echten Swaps aus Ihrer Wallet.
copy-arm-confirmation-missing = Die Live-Bestätigung konnte nicht geladen werden
copy-arm-armed = Live-Kopieren scharfgeschaltet
copy-arm-failed = Live-Kopieren konnte nicht scharfgeschaltet werden

## Holdings tab

copy-holdings-title = Bestände
copy-holdings-view-label = Bestandsansicht
copy-holdings-view-open = Offen ({ $count })
copy-holdings-view-closed = Geschlossene Runden ({ $count })
copy-holdings-reset = Paper-Buch zurücksetzen
copy-holdings-live-note = Live-Kopien sind echte Positionen.
copy-holdings-open-positions = Offene Positionen
copy-holdings-token-details = Token-Details öffnen
copy-holdings-opened = Eröffnet { $time }
copy-holdings-no-pool-price = Kein Pool-Preis
copy-holdings-close = Schließen
copy-holdings-write-off = Abschreiben
copy-holdings-activity = Aktivität
copy-holdings-no-exit-rule = Keine Ausstiegsregel
copy-holdings-watch-stop = Stop { $level }
copy-holdings-watch-stop-until = Stop { $level } in { $span }
copy-holdings-watch-take = Take { $level }
copy-holdings-watch-trail = Trail { $level }
copy-holdings-watch-trail-arms = Trail aktiviert bei { $level }
copy-holdings-watch-time = Zeit ≤ { $level }
copy-holdings-watch-time-until = Zeit ≤ { $level } in { $span }
copy-holdings-watch-wallet-sells = Wallet-Verkäufe
copy-holdings-empty = Keine offenen Paper-Bestände. Von der Wallet kopierte Käufe erscheinen hier.
copy-holdings-col-token = Token
copy-holdings-col-cost = Kosten
copy-holdings-col-entry = Einstieg
copy-holdings-col-mark = Bewertung
copy-holdings-col-peak = Höchststand
copy-holdings-col-pnl = GuV
copy-holdings-col-exit-rules = Ausstiegsregeln
copy-holdings-col-held = Gehalten
copy-holdings-col-actions = Aktionen
copy-holdings-col-invested = Investiert
copy-holdings-col-proceeds = Erlös
copy-holdings-col-exit = Ausstieg
copy-holdings-col-closed = Geschlossen
copy-holdings-price-note = Preise sind in SOL pro Token angegeben. Der Einstieg enthält Slippage und Gebühren des Kaufs; Höchststand und Ausstiegsniveaus beziehen sich darauf, sodass ein Bestand mit einem Höchststand unter dem Einstieg eröffnet wird. Fahren Sie mit der Maus über einen Wert, um den Pool-Preis zu sehen.
copy-holdings-paused-rules = Pausiert: keine neuen Kopien. Ihre Ausstiegsregeln schließen diese Bestände weiterhin.
copy-holdings-paused-mirror = Pausiert: keine neuen Kopien. Die Verkäufe der Wallet schließen diese Bestände weiterhin.
copy-holdings-paused-hybrid = Pausiert: keine neuen Kopien. Die Verkäufe der Wallet und Ihre Ausstiegsregeln schließen diese Bestände weiterhin.
copy-holdings-closed-load-failed = Geschlossene Runden konnten nicht geladen werden: { $error }
copy-holdings-closed-loading = Geschlossene Runden werden geladen…
copy-holdings-closed-empty = Noch keine geschlossenen Runden.
copy-holdings-closed-latest = Die letzten { $shown } von { $total } Runden.
copy-holdings-close-title = Paper-Bestand schließen
copy-holdings-close-message = { $token } im Paper-Buch zum Pool-Preis ({ $price }) mit Slippage und Gebühren der Aufgabe verkaufen.
copy-holdings-close-confirm = Bestand schließen
copy-holdings-write-off-title = Paper-Bestand abschreiben
copy-holdings-write-off-message = Für { $token } gibt es keinen Pool-Preis zum Verkaufen. Beim Abschreiben wird er mit null geschlossen und seine Kosten von { $cost } als Verlust verbucht.
copy-holdings-keep = Behalten
copy-holdings-written-off = { $token } abgeschrieben
copy-holdings-closed = { $token } geschlossen
copy-holdings-written-off-detail = Ohne Erlös geschlossen
copy-holdings-sold-at = Verkauft zu { $price }
copy-holdings-close-failed = Bestand konnte nicht geschlossen werden
copy-holdings-reset-message = „{ $name }“ neu starten: Paper-Bestände, Ausgaben, Ausführungen, Ausstiege und übersprungene Trades werden entfernt. Die Regeln und die Wallet bleiben.
copy-holdings-reset-cancel = Verlauf behalten
copy-holdings-reset-done = Paper-Buch zurückgesetzt
copy-holdings-reset-detail =
    { $count ->
        [one] { $count } Entscheidung entfernt
       *[other] { $count } Entscheidungen entfernt
    }
copy-holdings-reset-failed = Paper-Buch konnte nicht zurückgesetzt werden

## Activity tab

copy-activity-title = Aktivität
copy-activity-filter-label = Aktivitätsfilter
copy-filter-all = Alle
copy-outcome-paper-filled = Paper-Kauf
copy-outcome-live-submitted = Live-Kauf übermittelt
copy-outcome-live-confirmed = Live-Kauf bestätigt
copy-outcome-live-failed = Live-Kauf fehlgeschlagen
copy-outcome-paper-sell-observed = Paper-Verkauf · Wallet hat verkauft
copy-outcome-live-sell-submitted = Live-Verkauf übermittelt
copy-outcome-live-sell-failed = Live-Verkauf fehlgeschlagen
copy-outcome-skipped = Übersprungen
copy-activity-decision = Entscheidung
copy-activity-paper-exit = Paper-Ausstieg · { $rule }
copy-activity-filled = { $input } zu { $price } · Wallet kaufte { $target }
copy-activity-filled-slippage = { $input } zu { $price } · Wallet kaufte { $target } · Slippage { $slippage }
copy-activity-filled-unpriced = { $input } zu { $price } · zum Trade der Wallet bepreist, kein Pool-Preis
copy-activity-live-sized = { $sized } · Wallet kaufte { $target }
copy-activity-sell-nothing = Wallet verkaufte { $amount } · nichts im Bestand, das verkauft werden könnte
copy-activity-written-off = Mit null abgeschrieben: kein Pool-Preis
copy-activity-sold = { $tokens } Tokens für { $proceeds } zu { $price }
copy-activity-full-close = Vollständig geschlossen
copy-activity-partial-exit = Ausstieg: { $pct }
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = Minimum { $amount }
copy-activity-skip-maximum = Maximum { $value }
copy-activity-skip-stale = { $arrival } zu spät, Limit { $limit }
copy-activity-skip-latency = Durchschnitt { $average }, Limit { $limit }
copy-activity-arrival-replayed = { $span } nach dem Block nachgeholt
copy-activity-arrival-seen = { $span } nach dem Block erkannt
copy-activity-link-wallet-tx = Wallet-Tx
copy-activity-link-own-tx = Ihre Tx
copy-activity-only-token = Nur dieser Token
copy-activity-skipped-group = Übersprungen ×{ $count }
copy-activity-group-detail =
    { $tokens ->
        [one] { $tokens } Token · seit { $since }
       *[other] { $tokens } Tokens · seit { $since }
    }
copy-activity-mint-filter =
    .placeholder = Token-Mint
    .aria-label = Nach Token-Mint filtern
copy-activity-clear = Leeren
copy-activity-load-failed = Aktivität konnte nicht geladen werden: { $error }
copy-activity-loading = Aktivität wird geladen…
copy-activity-no-match = Nichts entspricht diesem Filter.
copy-activity-empty = Noch keine Entscheidungen. Ausführungen, Ausstiege und übersprungene Trades erscheinen hier, sobald die Wallet handelt.
copy-activity-load-older = Ältere laden
copy-activity-start = Anfang des Verlaufs
copy-activity-older-failed = Ältere Aktivität konnte nicht geladen werden

## Task editor

copy-step-wallet = Wallet
copy-step-sizing = Dimensionierung
copy-step-entry = Einstiegsfilter
copy-step-exits = Ausstiege
copy-step-review = Prüfung
copy-editor-title-edit = { $name } bearbeiten
copy-editor-title-clone = { $name } klonen
copy-editor-sub-edit = Aufgabe im Modus { $mode } · Änderungen gelten für die nächsten Entscheidungen
copy-editor-sub-clone = Gleiche Regeln, leeres Paper-Buch, Start im Paper-Modus
copy-editor-save-edit = Änderungen speichern
copy-editor-save-clone = Klon erstellen
copy-editor-save-create = Paper-Aufgabe erstellen
copy-editor-clone-suffix = (Kopie)
copy-editor-discard-edit = Änderungen verwerfen
copy-editor-discard-create = Diese Aufgabe verwerfen
copy-editor-discard-edit-message = Ihre Änderungen an „{ $name }“ sind nicht gespeichert.
copy-editor-discard-create-message = Die bisher eingegebene Wallet und die Regeln sind nicht gespeichert.
copy-editor-discard-confirm = Verwerfen
copy-editor-keep-editing = Weiter bearbeiten
copy-editor-toast-updated = Aufgabe aktualisiert
copy-editor-toast-clone = Klon erstellt
copy-editor-toast-created = Paper-Aufgabe erstellt
copy-unit-sol = { -sol }
copy-editor-any = Beliebig
copy-editor-duplicate = Wird bereits von { $tasks } kopiert. Diese Aufgabe kopiert dieselben Trades erneut, mit eigenen Regeln und eigenem Budget.
copy-editor-wallet = Wallet
copy-editor-wallet-identity = Die Wallet einer Aufgabe ist ihre Identität. Um eine andere Wallet mit diesen Regeln zu kopieren, klonen Sie die Aufgabe.
copy-editor-address-label = Wallet-Adresse
copy-editor-address-placeholder = Solana-Wallet-Adresse
copy-editor-address-help-clone = Gleiche Regeln mit leerem Paper-Buch. Behalten Sie diese Wallet, um andere Regeln daran zu testen, oder geben Sie eine andere Wallet ein.
copy-editor-address-help-create = Die Wallet, deren Käufe (und, falls gewählt, Verkäufe) diese Aufgabe kopiert.
copy-editor-name-label = Name <em>optional</em>
copy-editor-name-placeholder = z. B. Schneller Rotator
copy-editor-enabled-title = Trades der Wallet verarbeiten
copy-editor-enabled-help = Bei „Aus“ bleibt die Aufgabe pausiert, bis Sie sie fortsetzen.
copy-editor-note-live = Diese Aufgabe läuft live: Änderungen gelten für ihre nächsten echten Kopien.
copy-editor-note-paper = Aufgaben laufen im Paper-Modus, bis Sie sie scharfschalten: Trades werden zum Pool-Preis simuliert und nichts wird ausgegeben.
copy-editor-copy-size = Kopiergröße
copy-editor-sizing-fixed = Fester Betrag
copy-editor-sizing-ratio = Anteil am Wallet-Trade
copy-editor-amount-fixed = Betrag pro Kopie
copy-editor-amount-ratio = Anteil pro Trade
copy-editor-amount-help-fixed = Wird pro kopiertem Kauf ausgegeben, mindestens { $minimum }.
copy-editor-amount-help-ratio = Vom eigenen Kauf der Wallet, bis zum Limit pro Trade.
copy-editor-help-trade-cap = Keine einzelne Kopie gibt mehr aus.
copy-editor-help-token-cap = Gesamtausgaben für einen Token.
copy-editor-help-budget = Alles, was diese Aufgabe über ihre Laufzeit ausgeben darf; Paper und Live zählen jeweils ihre eigenen Ausgaben.
copy-editor-preview-title = Was eine Kopie kostet
copy-editor-preview-empty = Geben Sie die Dimensionierung ein, um die Kosten einer Kopie zu sehen.
copy-editor-preview-example = Die Wallet kauft { $target } → Sie kopieren <strong>{ $copy }</strong>
copy-editor-preview-once = Ein Token erhält eine einzelne Kopie von { $size }, da jeder Token nur einmal gekauft wird
copy-editor-preview-token-cap =
    { $count ->
        [one] Ein Token erhält höchstens { $count } Kopie von { $size }
       *[other] Ein Token erhält höchstens { $count } Kopien von { $size }
    }
copy-editor-preview-summary-exact = { $perToken }; das Budget reicht für etwa { $count } davon. Netzwerk- und Prioritätsgebühren kommen hinzu.
copy-editor-preview-summary-minimum = { $perToken }; das Budget reicht für mindestens { $count } davon. Netzwerk- und Prioritätsgebühren kommen hinzu.
copy-editor-target-min = Kleinster kopierter Wallet-Trade
copy-editor-target-min-help = Kleinere Käufe der Wallet ignorieren. Leer lassen für kein Minimum.
copy-editor-target-max = Größter kopierter Wallet-Trade
copy-editor-target-max-help = Größere Käufe der Wallet ignorieren. Leer lassen für kein Maximum.
copy-editor-buy-once-title = Jeden Token einmal kaufen
copy-editor-buy-once-help = Nur den ersten Kauf eines Tokens durch die Wallet kopieren; spätere Käufe werden übersprungen.
copy-editor-filter-require = Verlangen
copy-editor-filter-skip = Nicht verlangen
copy-editor-filter-help = Verlangen, dass ein Token Ihre Filterungs-Pipeline besteht, bevor er kopiert wird.
copy-editor-filter-warning = Mit der Standard-Filterung fällt fast jeder Token durch, sodass eine Aufgabe, die ein Bestehen verlangt, nichts kopiert. Verlangen Sie es nur, wenn Ihre Filter die Tokens durchlassen, die diese Wallet handelt.
copy-editor-exit-both = Beides
copy-editor-exit-help-buy-only = Ihre Regeln unten verkaufen jeden Bestand; die Verkäufe der Wallet werden ignoriert.
copy-editor-exit-help-hybrid = Was zuerst eintritt: Die Wallet verkauft oder eine Ihrer Regeln greift.
copy-editor-exit-help-mirror = Bestände werden nur verkauft, wenn die Wallet verkauft. Ihre Ausstiegsregeln laufen nicht.
copy-editor-who-sells = Wer verkauft
copy-editor-preset = Vorlage
copy-editor-preset-help = Eine Vorlage füllt alle Regeln unten aus; passen Sie danach beliebige davon an.
copy-editor-mirror-note = Diese Regeln laufen nicht, solange die Verkäufe der Wallet entscheiden. Sie gelten, wenn Sie zu „{ $mine }“ oder „{ $both }“ wechseln.
copy-editor-rule-inherit = Trader-Standard
copy-editor-inherit-value = Trader-Standard ({ $value })
copy-editor-rule-aria = Einstellung: { $rule }
copy-editor-rule-empty-uses = Leer verwendet den Trader-Standard: { $value }
copy-editor-rule-follows = Folgt dem Trader: { $summary }
copy-editor-rule-follows-plain = Folgt der Einstellung des Traders.
copy-editor-rule-off-note = Für diese Aufgabe aus, unabhängig von der Einstellung des Traders.
copy-editor-unnamed = Unbenannte Aufgabe
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = verarbeitet Trades nach dem Speichern
copy-editor-review-paused = pausiert gespeichert
copy-editor-error-address = Geben Sie eine gültige Solana-Wallet-Adresse ein.
copy-editor-error-sizing = Jeder Dimensionierungswert muss größer als null sein.
copy-editor-error-min-copy = Eine Kopie muss mindestens { $minimum } betragen: Erhöhen Sie den Betrag pro Kopie.
copy-editor-error-min-cap = Eine Kopie muss mindestens { $minimum } betragen: Erhöhen Sie das Limit pro Trade.
copy-editor-error-trade-cap = Das Limit pro Trade darf das Limit pro Token nicht überschreiten.
copy-editor-error-token-cap = Das Limit pro Token darf das Gesamtbudget nicht überschreiten.
copy-editor-error-slippage = Die Slippage muss zwischen { $min } und { $max } liegen.
copy-editor-error-target-limits = Die Limits für Wallet-Trades müssen null oder größer sein.
copy-editor-error-target-order = Der kleinste Wallet-Trade darf den größten nicht überschreiten.

## Copy notices

copy-notice-task-unnamed = Aufgabe #{ $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = Paper-Copy-Kauf
copy-notice-title-paper-sell = Paper-Copy-Verkauf
copy-notice-title-paper-closed = Paper-Bestand geschlossen
copy-notice-title-paper-exit = Paper-Ausstieg: { $rule }
copy-notice-title-live-buy-submitted = Live-Copy-Kauf übermittelt
copy-notice-title-live-buy-confirmed = Live-Copy-Kauf bestätigt
copy-notice-title-live-buy-failed = Live-Copy-Kauf fehlgeschlagen
copy-notice-title-live-sell-submitted = Live-Copy-Verkauf übermittelt
copy-notice-title-live-sell-failed = Live-Copy-Verkauf fehlgeschlagen
copy-notice-title-auto-paused = Copy-Aufgabe automatisch pausiert
copy-notice-detail-bought = Gekauft für { $amount } { -sol }
copy-notice-detail-sold = Verkauft für { $amount } { -sol }
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = { $percent } % des Bestands
copy-notice-detail-full-close = Vollständig geschlossen
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = Swap fehlgeschlagen
