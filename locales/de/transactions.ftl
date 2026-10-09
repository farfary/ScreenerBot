# Transaction type labels.

transactions-type-buy = Kauf
transactions-type-sell = Verkauf
transactions-type-swap = Swap
transactions-type-sol-transfer = SOL-Überweisung
transactions-type-token-transfer = Token-Übertragung
transactions-type-transfer = Übertragung
transactions-type-dust = Dust
transactions-type-spam = Spam
transactions-type-ata-create = Konto eröffnet
transactions-type-ata-close = Miete zurückgeholt
transactions-type-ata = Token-Konto
transactions-type-liquidity-add = Liquidität hinzufügen
transactions-type-liquidity-remove = Liquidität entfernen
transactions-type-nft = NFT
transactions-type-program = Programmaufruf
transactions-type-compute = Compute
transactions-type-failed = Fehlgeschlagen
transactions-type-unknown = Nicht klassifiziert

transactions-type-with-detail = { $label } ({ $detail })
transactions-type-token-transfer-detail = { $label } { $mint } ({ $amount })
transactions-type-spam-detail = Spam-Airdrop ({ $mint })
transactions-type-described = { $description }

transactions-filter-all = Alle Typen
transactions-filter-transfer = Übertragungen
transactions-filter-ata = Miete und Konten
transactions-filter-liquidity = Liquidität
transactions-filter-program = Programmaufrufe

transactions-direction-incoming = Eingehend
transactions-direction-outgoing = Ausgehend
transactions-direction-internal = Intern
transactions-direction-unknown = Nicht klassifiziert

transactions-status-pending = Ausstehend
transactions-status-confirmed = Bestätigt
transactions-status-finalized = Finalisiert
transactions-status-failed = Fehlgeschlagen
transactions-status-success = Erfolgreich
transactions-status-unknown = Unbekannt

transactions-ata-operation-creation = Erstellung
transactions-ata-operation-closure = Schließung

## Transactions page (pages/transactions.js)

transactions-toolbar-title = Transaktionshistorie
transactions-search =
    .placeholder = Signaturen suchen…
    .aria-label = Transaktionssignaturen suchen
transactions-load-failed = Transaktionen konnten nicht aktualisiert werden
transactions-setup-gate-title = Transaktionen erfordern eine Wallet
transactions-summary-total = Gesamt
transactions-summary-success = Erfolgreich
transactions-summary-failed = Fehlgeschlagen
transactions-filter-wallet = Wallet
transactions-filter-type = Typ
transactions-filter-direction = Richtung
transactions-filter-status = Status
transactions-filter-all-directions = Alle Richtungen
transactions-filter-all-statuses = Alle Status
transactions-wallet-main = Haupt-Wallet
transactions-col-time = Zeit
transactions-col-signature = Signatur
transactions-col-type = Typ
transactions-col-direction = Richtung
transactions-col-status = Status
transactions-col-native-delta = Δ { -sol }
transactions-col-fees = Gebühren ({ -sol })
transactions-col-token = Token
transactions-col-router = Router
transactions-col-instructions = Instr.

## Transaction details dialog (ui/transaction_details_dialog.js)

transactions-dialog-copy-signature =
    .title = Signatur kopieren
transactions-dialog-close =
    .title = Schließen (ESC)
transactions-dialog-tabs-label = Abschnitte der Transaktionsdetails
transactions-dialog-meta-slot = Slot:
transactions-dialog-meta-fee = Gebühr:
transactions-dialog-loading = Wird geladen...
transactions-dialog-loading-details = Transaktionsdetails werden geladen...
transactions-dialog-load-failed = Transaktionsdetails konnten nicht geladen werden
transactions-dialog-load-failed-reason = Transaktionsdetails konnten nicht geladen werden: { $reason }
transactions-dialog-not-found = Transaktion nicht gefunden
transactions-dialog-tab-overview = Übersicht
transactions-dialog-tab-balances = Guthaben
transactions-dialog-tab-instructions = Instruktionen
transactions-dialog-tab-logs = Logs
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = Rohdaten
transactions-dialog-unknown = Unbekannt
transactions-dialog-unknown-asset = Unbekanntes Asset
transactions-dialog-unavailable = Nicht verfügbar

## Transaction details dialog: overview

transactions-dialog-failed-title = Transaktion fehlgeschlagen
transactions-dialog-no-program-error = Es wurde kein Programmfehler übermittelt.
transactions-dialog-story-title = Was passiert ist
transactions-dialog-router-via = über { $router }
transactions-dialog-flow-paid = Bezahlt
transactions-dialog-flow-received = Erhalten
transactions-dialog-flow-from = Von
transactions-dialog-flow-to = An
transactions-dialog-flow-amount = Menge
transactions-dialog-net-wallet-change = Netto-Wallet-Änderung:
transactions-dialog-processed = Auf Solana verarbeitet
transactions-dialog-execution-title = Ausführung
transactions-dialog-metric-execution-price = Ausführungspreis
transactions-dialog-metric-effective-received = Effektiv erhalten
transactions-dialog-metric-effective-spent = Effektiv ausgegeben
transactions-dialog-metric-network-fee = Netzwerkgebühr
transactions-dialog-metric-estimated-pnl = Geschätzte GuV
transactions-dialog-metric-net-native-change = Netto-{ -sol }-Änderung
transactions-dialog-route-title = Route und Assets
transactions-dialog-route-router = Router
transactions-dialog-route-input-asset = Eingabe-Asset
transactions-dialog-route-output-asset = Ausgabe-Asset
transactions-dialog-route-pool = Pool
transactions-dialog-route-program = Programm
transactions-dialog-tech-title = Technische Details
transactions-dialog-tech-summary = Signatur, Slot und Ressourcen
transactions-dialog-tech-signature = Signatur
transactions-dialog-tech-timestamp = Zeitstempel
transactions-dialog-tech-slot = Slot
transactions-dialog-tech-exact-fee = Exakte Gebühr
transactions-dialog-tech-accounts = Konten
transactions-dialog-tech-instructions = Instruktionen
transactions-dialog-tech-compute-units = Compute Units
transactions-dialog-tech-token-decimals = Token-Dezimalstellen

## Transaction details dialog: balances, instructions, logs, ATA and raw tabs

transactions-dialog-balances-native-title = { -sol }-Guthabenänderungen
transactions-dialog-balances-native-empty = Keine { -sol }-Guthabenänderungen
transactions-dialog-balances-token-title = Token-Guthabenänderungen
transactions-dialog-balances-token-empty = Keine Token-Guthabenänderungen
transactions-dialog-balances-net-native = Netto-{ -sol }-Änderung
transactions-dialog-balances-fee = Transaktionsgebühr
transactions-dialog-col-account = Konto
transactions-dialog-col-token = Token
transactions-dialog-col-pre-balance = Guthaben davor
transactions-dialog-col-post-balance = Guthaben danach
transactions-dialog-col-change = Änderung
transactions-dialog-col-type = Typ
transactions-dialog-col-rent = Miete ({ -sol })
transactions-dialog-instructions-empty = Keine Instruktionen gefunden
transactions-dialog-instructions-count =
    { $count ->
        [one] { $count } Instruktion
       *[other] { $count } Instruktionen
    }
transactions-dialog-instruction-program-id = Programm-ID
transactions-dialog-instruction-accounts = Konten ({ $count })
transactions-dialog-instruction-data = Daten
transactions-dialog-logs-empty = Keine Logs verfügbar
transactions-dialog-logs-filter = Logs filtern...
transactions-dialog-logs-no-match = Keine passenden Logs
transactions-dialog-logs-count =
    { $count ->
        [one] { $count } Log
       *[other] { $count } Logs
    }
transactions-dialog-ata-empty = Keine ATA-Vorgänge in dieser Transaktion
transactions-dialog-ata-summary-title = ATA-Analyse-Zusammenfassung
transactions-dialog-ata-creations = Erstellungen
transactions-dialog-ata-closures = Schließungen
transactions-dialog-ata-rent-spent = Ausgegebene Miete
transactions-dialog-ata-rent-recovered = Zurückgeholte Miete
transactions-dialog-ata-net-rent = Netto-Mietauswirkung
transactions-dialog-ata-operations-title = ATA-Vorgänge ({ $count })
transactions-dialog-raw-copy = JSON kopieren
transactions-dialog-raw-empty = Keine Rohdaten verfügbar

# Empty table (scripts/pages/transactions.js)
transactions-empty = Noch keine Transaktionen
    .message = Swaps und Überweisungen der Trading-Wallet erscheinen hier, sobald sie on-chain bestätigt sind.
