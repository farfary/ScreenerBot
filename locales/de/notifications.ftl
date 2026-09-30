notifications-action-swap-buy = Kauf
notifications-action-swap-sell = Verkauf
notifications-action-position-open = Eröffnen
notifications-action-position-close = Schließen
notifications-action-position-dca = DCA
notifications-action-position-partial-exit = Teilausstieg
notifications-action-manual-order = Manuell
notifications-action-unknown = Aktion

actions-step-evaluate = Auswertung
actions-step-validate = Validierung
actions-step-quote = Angebot wird abgerufen
actions-step-swap = Swap wird ausgeführt
actions-step-verify = Überprüfung
actions-step-unknown = Verarbeitung
actions-step-evaluate-short = Auswertung
actions-step-validate-short = Prüfung
actions-step-quote-short = Angebot
actions-step-swap-short = Swap läuft
actions-step-verify-short = Bestätigung
actions-step-unknown-short = Läuft

actions-failure-recorded = { $message }
actions-failure-unknown = Unbekannter Fehler
actions-failure-interrupted = Durch Neustart der Anwendung unterbrochen
actions-failure-validation = Validierung fehlgeschlagen
actions-failure-quote = Angebot fehlgeschlagen
actions-failure-swap = Swap fehlgeschlagen
actions-failure-trade = Trade fehlgeschlagen
actions-failure-entry = Einstieg fehlgeschlagen
actions-failure-exit = Ausstieg fehlgeschlagen
actions-failure-dca = DCA fehlgeschlagen
actions-failure-verification-expired = Überprüfung abgelaufen: Die Transaktion wurde nie bestätigt
actions-failure-verification-gave-up = Überprüfung abgebrochen
actions-failure-transaction-failed = Die Transaktion ist on-chain fehlgeschlagen
actions-failure-sell-transaction-failed = Die Verkaufstransaktion ist on-chain fehlgeschlagen
actions-failure-dca-verification-failed = DCA-Überprüfung fehlgeschlagen

notifications-empty-all = Keine Aktionen
notifications-empty-active = Keine aktiven Aktionen
notifications-empty-completed = Keine abgeschlossenen Aktionen
notifications-empty-failed = Keine fehlgeschlagenen Aktionen
notifications-source-auto = Auto
notifications-source-manual = Manuell
notifications-state-locked = Status wird vom Tab bestimmt
notifications-cancelled = Abgebrochen
notifications-dismiss = Schließen
notifications-dismiss-failed = Benachrichtigung konnte nicht geschlossen werden
notifications-load-failed = Laden fehlgeschlagen
notifications-mark-read-failed = Benachrichtigungen konnten nicht als gelesen markiert werden
notifications-clear-title = Benachrichtigungen leeren
notifications-clear-message = Alle Benachrichtigungen dieser Liste schließen? Sie bleiben im Verlauf unter „Abgeschlossen“/„Fehlgeschlagen“ erhalten.
notifications-clear-failed = Benachrichtigungen konnten nicht geleert werden
notifications-stream-lag-title = Aktions-Stream ist im Rückstand
notifications-stream-lag-missed =
    { $count ->
        [one] { $count } Aktualisierung verpasst
       *[other] { $count } Aktualisierungen verpasst
    } — wird aktualisiert
notifications-stream-lag-refreshing = Wird aktualisiert
notifications-sync-failed = Aktionen konnten nicht aktualisiert werden
