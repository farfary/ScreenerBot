## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = Tools
tools-category-wallet = Wallet
tools-category-token = Token
tools-category-single-token = Einzelner Token
tools-category-utilities = Hilfsprogramme
tools-sidebar-hint = Wählen Sie ein Tool, um zu beginnen
tools-help-button =
    .aria-label = Hilfe zu diesem Tool anzeigen
tools-help-unavailable = Hilfe nicht verfügbar
tools-placeholder-title = Tool auswählen
tools-placeholder-subtitle = Wählen Sie in der Seitenleiste ein Tool, um zu beginnen
tools-placeholder-hint-wallets = Wallet-Tools helfen bei der Verwaltung Ihrer Solana-Wallets
tools-placeholder-hint-secure = Alle Vorgänge sind abgesichert und nach Möglichkeit umkehrbar

tools-status-ready = Einsatzbereit
tools-status-coming = Demnächst verfügbar
tools-status-beta = Beta – kann Fehler enthalten
tools-status-disabled = Derzeit deaktiviert
tools-status-badge-coming = Demnächst
tools-status-badge-beta = Beta
tools-toast-coming-soon = Dieses Tool ist demnächst verfügbar
tools-toast-disabled = Dieses Tool ist derzeit deaktiviert

## Tool names. `-title` names the tool in the navigation and the header, `-summary` is the
## navigation line, `-description` is the header line. Ids are the tool ids of the registry.

tools-tool-wallet-cleanup-title = Wallet-Bereinigung
tools-tool-wallet-cleanup-summary = Leere ATAs schließen
tools-tool-wallet-cleanup-description = Leere Associated Token Accounts schließen, um { -sol } zurückzuholen
tools-tool-burn-tokens-title = Tokens verbrennen
tools-tool-burn-tokens-summary = Tokens dauerhaft vernichten
tools-tool-burn-tokens-description = Tokens aus Ihrer Wallet dauerhaft vernichten
tools-tool-token-analyzer-title = Token-Analyse
tools-tool-token-analyzer-summary = Tiefgehende Token-Analyse
tools-tool-token-analyzer-description = Tiefgehende Analyse jedes Solana-Tokens mit mehrdimensionalen Einblicken
tools-tool-create-token-title = Token erstellen
tools-tool-create-token-summary = Neuen SPL-Token bereitstellen
tools-tool-create-token-description = Einen neuen SPL-Token auf Solana bereitstellen
tools-tool-trade-watcher-title = Trade-Watcher
tools-tool-trade-watcher-summary = Trades überwachen und automatisch handeln
tools-tool-trade-watcher-description = Token-Trades überwachen und automatische Kauf-/Verkaufsaktionen auslösen
tools-tool-token-watch-title = Holder-Watch
tools-tool-token-watch-summary = Neue Token-Holder verfolgen
tools-tool-token-watch-description = Neue Token-Holder in Echtzeit verfolgen und überwachen
tools-tool-buy-multi-wallets-title = Multi-Buy
tools-tool-buy-multi-wallets-summary = Käufe über Wallets koordinieren
tools-tool-buy-multi-wallets-description = Koordinierte Kaufaufträge über mehrere Wallets mit zufälligen Beträgen ausführen
tools-tool-sell-multi-wallets-title = Multi-Sell
tools-tool-sell-multi-wallets-summary = Verkäufe über Wallets koordinieren
tools-tool-sell-multi-wallets-description = Koordinierte Verkaufsaufträge über mehrere Wallets mit { -sol }-Zusammenführung ausführen
tools-tool-wallet-consolidation-title = Wallet-Zusammenführung
tools-tool-wallet-consolidation-nav-title = Zusammenführung
tools-tool-wallet-consolidation-summary = Wallet-Guthaben zusammenführen
tools-tool-wallet-consolidation-description = { -sol } und Tokens aus Sub-Wallets in die Haupt-Wallet zusammenführen
tools-tool-airdrop-checker-title = Airdrop-Prüfung
tools-tool-airdrop-checker-summary = Ausstehende Airdrops prüfen
tools-tool-airdrop-checker-description = Auf ausstehende Airdrops und einlösbare Belohnungen prüfen
tools-tool-wallet-generator-title = Wallet-Generator
tools-tool-wallet-generator-summary = Neue Keypairs erzeugen
tools-tool-wallet-generator-description = Neue Solana-Keypairs sicher erzeugen

## Shared by the tools

tools-validation-mint-required = Bitte geben Sie eine Token-Mint-Adresse ein
tools-validation-mint-format = Ungültiges Format der Token-Mint-Adresse
tools-validation-mint-invalid = Bitte geben Sie eine gültige Mint-Adresse ein

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = Token-Details
tools-create-token-name-label = Token-Name
tools-create-token-name-input =
    .placeholder = Mein Token
tools-create-token-symbol-label = Symbol
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = Dezimalstellen
tools-create-token-supply-label = Anfangsangebot
tools-create-token-description-label = Beschreibung
tools-create-token-description-input =
    .placeholder = Token-Beschreibung...
tools-create-token-image-title = Token-Bild
tools-create-token-image-drop = Bild hier ablegen oder zum Hochladen klicken
tools-create-token-image-hint = Empfohlen: 512x512 PNG
tools-create-token-action-preview = Vorschau
tools-create-token-action-create = Token erstellen

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = Einstellungen werden geladen...
tools-holder-watch-saved = Holder-Watch-Einstellungen gespeichert
tools-holder-watch-save-failed = Einstellungen konnten nicht gespeichert werden
tools-holder-watch-save-error = Fehler beim Speichern der Einstellungen
tools-holder-watch-settings-title = Holder-Watch-Einstellungen
tools-holder-watch-enabled-label = Holder-Überwachung aktivieren
tools-holder-watch-interval-label = Prüfintervall (Sekunden)
tools-holder-watch-interval-hint = Wie oft die Holder-Zahlen geprüft werden (10–3600 s)
tools-holder-watch-max-tokens-label = Max. überwachte Tokens
tools-holder-watch-max-tokens-hint = Maximale Anzahl gleichzeitig überwachter Tokens
tools-holder-watch-notify-new-label = Bei neuen Holdern benachrichtigen
tools-holder-watch-notify-drop-label = Bei Holder-Rückgang benachrichtigen
tools-holder-watch-min-change-label = Min. Holder-Änderung
tools-holder-watch-min-change-hint = Minimale Holder-Änderung, die eine Benachrichtigung auslöst
tools-holder-watch-drop-percent-label = Schwelle für Holder-Rückgang (%)
tools-holder-watch-drop-percent-hint = Prozentualer Rückgang, der einen Alarm auslöst
tools-holder-watch-action-save = Einstellungen speichern
tools-holder-watch-tokens-title = Überwachte Tokens
tools-holder-watch-token-input =
    .placeholder = Token-Mint-Adresse eingeben...
tools-holder-watch-empty = Keine Tokens werden überwacht
tools-holder-watch-empty-hint = Fügen Sie oben eine Token-Mint-Adresse hinzu, um die Überwachung zu starten
tools-holder-watch-coming-soon = Token-Überwachung demnächst verfügbar

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = Token analysieren
tools-analyzer-mint-input =
    .placeholder = Token-Mint-Adresse einfügen...
tools-analyzer-action-analyze = Analysieren
tools-analyzer-action-analyzing = Wird analysiert...
tools-analyzer-action-copy-report = Bericht kopieren
tools-analyzer-loading = Token wird analysiert...
tools-analyzer-failed = Token konnte nicht analysiert werden
tools-analyzer-empty = Geben Sie zur Analyse eine Token-Mint-Adresse ein
tools-analyzer-empty-hint = Umfassende Einblicke in jeden Solana-Token
tools-analyzer-tab-overview = Übersicht
tools-analyzer-tab-security = Sicherheit
tools-analyzer-tab-market = Markt
tools-analyzer-tab-liquidity = Liquidität
tools-analyzer-unknown-token = Unbekannter Token

tools-analyzer-favorite-add =
    .title = Zu Favoriten hinzufügen
    .aria-label = Zu Favoriten hinzufügen
tools-analyzer-favorite-already = Bereits in den Favoriten
tools-analyzer-favorite-added = { $symbol } zu Favoriten hinzugefügt
tools-analyzer-favorite-failed = Hinzufügen zu Favoriten fehlgeschlagen
tools-analyzer-blacklist-add =
    .title = Auf die Blacklist setzen
    .aria-label = Auf die Blacklist setzen
tools-analyzer-blacklist-title = Token auf die Blacklist setzen
tools-analyzer-blacklist-message = { $symbol } auf die Blacklist setzen? Dieser Token wird vom Trading ausgeschlossen.
tools-analyzer-blacklist-confirm = Auf Blacklist setzen
tools-analyzer-blacklisted = Auf Blacklist
tools-analyzer-blacklist-done = { $symbol } auf die Blacklist gesetzt
tools-analyzer-blacklist-failed = Token konnte nicht auf die Blacklist gesetzt werden

tools-analyzer-card-quick-stats = Kurzstatistik
tools-analyzer-card-market-summary = Marktübersicht
tools-analyzer-card-token-info = Token-Informationen
tools-analyzer-stat-holders = Holder
tools-analyzer-stat-decimals = Dezimalstellen
tools-analyzer-stat-safety-score = Sicherheitsscore
tools-analyzer-stat-pools = Pools
tools-analyzer-stat-volume-24h = 24-Std.-Volumen
tools-analyzer-stat-change-24h = 24-Std.-Änderung
tools-analyzer-stat-market-cap = Marktkapitalisierung
tools-analyzer-stat-liquidity = Liquidität
tools-analyzer-info-mint = Mint-Adresse
tools-analyzer-info-description = Beschreibung
tools-analyzer-info-supply = Angebot

tools-analyzer-security-empty = Keine Sicherheitsdaten verfügbar
tools-analyzer-security-empty-hint = Für diesen Token ist keine Sicherheitsanalyse verfügbar
tools-analyzer-card-safety-score = Sicherheitsscore
tools-analyzer-score-good = Gut
tools-analyzer-score-moderate = Mäßig
tools-analyzer-score-risky = Riskant
tools-analyzer-raw-score = Roher Risikoscore: { $score }
tools-analyzer-card-authorities = Token-Berechtigungen
tools-analyzer-authority-mint = Mint-Berechtigung
tools-analyzer-authority-freeze = Freeze-Berechtigung
tools-analyzer-authority-transfer-fee = Transfergebühr
tools-analyzer-authority-mutable = Veränderlich
tools-analyzer-authority-active = Aktiv
tools-analyzer-authority-revoked = Widerrufen
tools-analyzer-card-holder-concentration = Holder-Konzentration
tools-analyzer-top-holders = im Besitz der Top-10-Holder
tools-analyzer-risks-title = Sicherheitsrisiken ({ $count })
tools-analyzer-risks-title-none = Sicherheitsrisiken
tools-analyzer-risks-none = Keine Sicherheitsrisiken erkannt

tools-analyzer-market-empty = Keine Marktdaten verfügbar
tools-analyzer-market-empty-hint = Für diesen Token sind keine Marktdaten verfügbar
tools-analyzer-card-price = Aktueller Preis
tools-analyzer-card-price-changes = Preisänderungen
tools-analyzer-card-volume = Handelsvolumen
tools-analyzer-card-transactions = Transaktionen (24 Std.)
tools-analyzer-card-valuation = Bewertung
tools-analyzer-stat-window-1h = 1 Std.
tools-analyzer-stat-window-6h = 6 Std.
tools-analyzer-stat-window-24h = 24 Std.
tools-analyzer-stat-volume-1h = Volumen 1 Std.
tools-analyzer-stat-volume-6h = Volumen 6 Std.
tools-analyzer-stat-fdv = Voll verwässerter Wert
tools-analyzer-txn-buys = Käufe
tools-analyzer-txn-sells = Verkäufe

tools-analyzer-liquidity-empty = Keine Liquiditätsdaten verfügbar
tools-analyzer-liquidity-empty-hint = Für diesen Token wurden keine Pools gefunden
tools-analyzer-card-total-liquidity = Gesamtliquidität
tools-analyzer-card-pools = Pools
tools-analyzer-active-pools =
    { $count ->
        [one] Aktiver Pool
       *[other] Aktive Pools
    }
tools-analyzer-card-pool-details = Pool-Details
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-address = Pool-Adresse
tools-analyzer-pools-column-liquidity = Liquidität ({ -sol })
tools-analyzer-pools-column-status = Status
tools-analyzer-pool-primary = Primär

tools-analyzer-report-empty = Keine Analyse zum Kopieren
tools-analyzer-report-label = Analysebericht
tools-analyzer-report-title = Token-Analysebericht
tools-analyzer-report-token = Token: { $symbol } ({ $name })
tools-analyzer-report-mint = Mint: { $mint }
tools-analyzer-report-price = Preis: { $sol }
tools-analyzer-report-price-with-usd = Preis: { $sol } ({ $usd })
tools-analyzer-report-security = Sicherheit:
tools-analyzer-report-safety-score = - Sicherheitsscore: { $score }/100
tools-analyzer-report-mint-authority = - Mint-Berechtigung: { $state }
tools-analyzer-report-freeze-authority = - Freeze-Berechtigung: { $state }
tools-analyzer-report-risks = - Risiken: { $count }
tools-analyzer-report-market = Markt:
tools-analyzer-report-volume = - 24-Std.-Volumen: { $amount }
tools-analyzer-report-change = - 24-Std.-Änderung: { $amount }
tools-analyzer-report-market-cap = - Marktkapitalisierung: { $amount }
tools-analyzer-report-liquidity = Liquidität:
tools-analyzer-report-liquidity-total = - Gesamt: { $amount }
tools-analyzer-report-pools = - Pools: { $count }
tools-analyzer-report-generated = Erstellt: { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

tools-watch-type-buy-on-sell = Kaufen bei Verkauf
tools-watch-type-sell-on-buy = Verkaufen bei Kauf
tools-watch-type-notify = Melden
tools-watch-type-notify-only = Nur benachrichtigen

tools-trade-watcher-setup-title = Watch einrichten
tools-trade-watcher-mint-label = Token-Mint-Adresse
tools-trade-watcher-mint-input =
    .placeholder = Token-Mint-Adresse eingeben...
tools-trade-watcher-action-search-pools = Pools suchen
tools-trade-watcher-pool-label = Ausgewählter Pool
tools-trade-watcher-pool-none = Kein Pool ausgewählt
tools-trade-watcher-pool-clear =
    .title = Pool entfernen
tools-trade-watcher-pool-selected = Ausgewählter Pool: { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = Watch-Typ
tools-trade-watcher-type-hint = Kaufen bei Verkauf: Automatisch kaufen, wenn jemand verkauft. Verkaufen bei Kauf: Automatisch verkaufen, wenn jemand kauft.
tools-trade-watcher-trigger-label = Auslösebetrag ({ -sol })
tools-trade-watcher-trigger-hint = Minimale Trade-Größe in { -sol }, die die Aktion auslöst
tools-trade-watcher-action-amount-label = Aktionsbetrag ({ -sol })
tools-trade-watcher-action-amount-hint = Betrag, der bei Auslösung gekauft/verkauft wird
tools-trade-watcher-slippage-label = Slippage (%)
tools-trade-watcher-slippage-hint = Maximal akzeptable Slippage für Trades
tools-trade-watcher-active-title = Aktive Watches
tools-trade-watcher-empty = Keine aktiven Watches
tools-trade-watcher-empty-hint = Konfigurieren Sie oben einen Watch und klicken Sie auf „Watch starten“, um die Überwachung zu beginnen
tools-trade-watcher-action-start = Watch starten
tools-trade-watcher-action-starting = Wird gestartet...
tools-trade-watcher-action-stop-all = Alle stoppen
tools-trade-watcher-action-stopping = Wird gestoppt...
tools-trade-watcher-started = Watch für { $token } gestartet...
tools-trade-watcher-start-failed = Watch konnte nicht gestartet werden
tools-trade-watcher-stopped = Watch gestoppt
tools-trade-watcher-stop-failed = Watch konnte nicht gestoppt werden
tools-trade-watcher-stopped-all = Alle Watches gestoppt
tools-trade-watcher-stop-all-failed = Watches konnten nicht gestoppt werden
tools-trade-watcher-load-failed = Watches konnten nicht geladen werden
tools-trade-watcher-column-token = Token
tools-trade-watcher-column-type = Typ
tools-trade-watcher-column-trigger = Auslöser
tools-trade-watcher-column-action = Aktion
tools-trade-watcher-column-triggered = Ausgelöst
tools-trade-watcher-stop-watch =
    .title = Watch stoppen

## Results returned by the tools backend. Failures are catalog text; the technical cause
## travels separately as details and is appended by the dashboard.

tools-burn-failure-native-asset = { -sol } kann nicht verbrannt werden
tools-burn-failure-open-position = Tokens aus offenen Positionen können nicht verbrannt werden
tools-burn-failure-account-not-found = Token-Konto nicht gefunden
tools-burn-failure-zero-balance = Das Token-Guthaben ist bereits null
tools-burn-failure-transaction = Transaktion fehlgeschlagen
tools-burn-warning-open-position = Tokens aus offenen Positionen können nicht verbrannt werden
tools-burn-warning-closed-position = Rest aus geschlossener Position
tools-burn-warning-worth = Wert ca. { $amount } { -sol }
tools-multi-buy-warning-insufficient = Guthaben nicht ausreichend. Benötigt { $needed } { -sol }, vorhanden { $have } { -sol }
tools-multi-buy-warning-over-limit = Benötigte { -sol } gesamt ({ $needed }) überschreiten das Limit ({ $limit })
tools-multi-sell-warning-no-wallets = Keine sekundären Wallets gefunden
tools-multi-sell-warning-no-balance = Keine Wallet hat ein Token-Guthaben
tools-multi-op-buy-failed = Kauf fehlgeschlagen
tools-multi-op-sell-failed = Verkauf fehlgeschlagen
tools-multi-op-transfer-failed = Übertragung fehlgeschlagen
tools-multi-op-balance-failed = Guthaben konnte nicht abgerufen werden
tools-multi-op-mint-invalid = Ungültige Mint-Adresse
tools-multi-buy-session-failed = Multi-Buy fehlgeschlagen
tools-multi-sell-session-failed = Multi-Sell fehlgeschlagen
tools-multi-session-aborted = Vorgang vom Nutzer abgebrochen

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = Wallet scannen
tools-wallet-action-scanning = Wird gescannt...
tools-wallet-scan-failed = Scan fehlgeschlagen: { $reason }
tools-wallet-amount-approx = ca. { $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
        [one] Ausgewählt: { $count } Wallet
       *[other] Ausgewählt: { $count } Wallets
    }
tools-wallet-transfer-failed = Übertragung fehlgeschlagen: { $reason }
tools-wallet-cleanup-failed = Bereinigung fehlgeschlagen: { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = Scan-Ergebnisse
tools-wallet-cleanup-stat-empty = Leere ATAs
tools-wallet-cleanup-stat-reclaimable = Zurückholbare { -sol }
tools-wallet-cleanup-stat-failed = Fehlgeschlagen (im Cache)
tools-wallet-cleanup-prompt = Klicken Sie auf „Wallet scannen“, um leere ATAs zu finden
tools-wallet-cleanup-prompt-hint = Dabei werden alle Token-Konten in Ihrer Wallet geprüft
tools-wallet-cleanup-action-cleanup = Alle bereinigen
tools-wallet-cleanup-action-cleaning = Wird bereinigt...
tools-wallet-cleanup-scanning = Wallet wird gescannt...
tools-wallet-cleanup-found =
    { $count ->
        [one] { $count } leeres ATA im Wert von ca. { $amount } gefunden
       *[other] { $count } leere ATAs im Wert von ca. { $amount } gefunden
    }
tools-wallet-cleanup-clean = Keine leeren ATAs gefunden – die Wallet ist sauber!
tools-wallet-cleanup-scan-failed = ATAs konnten nicht gescannt werden
tools-wallet-cleanup-done =
    { $count ->
        [one] { $count } ATA bereinigt
       *[other] { $count } ATAs bereinigt
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = Tokens verbrennen
tools-burn-info-title = Was ist Verbrennen?
tools-burn-info-body = Beim Verbrennen werden Tokens dauerhaft vernichtet und sind nicht wiederherstellbar. Führen Sie danach die Wallet-Bereinigung aus, um leere ATAs zu schließen und ca. 0,002 { -sol } Miete pro Token zurückzuholen.
tools-burn-stat-total = Tokens gesamt
tools-burn-stat-selected = Ausgewählt
tools-burn-stat-rent = Zurückholbare Miete
tools-burn-prompt = Klicken Sie auf „Wallet scannen“, um Tokens zu finden
tools-burn-scanning = Wallet wird nach Tokens gescannt...
tools-burn-scan-failed = Tokens konnten nicht gescannt werden
tools-burn-empty = Keine Tokens in der Wallet gefunden
tools-burn-action-burn = Auswahl verbrennen ({ $count })
tools-burn-action-burning = Wird verbrannt...
tools-burn-cannot-burn = Nicht verbrennbar
tools-burn-no-value = Kein Wert

tools-burn-category-open-position = Offene Positionen
tools-burn-category-has-value = Mit Wert
tools-burn-category-closed-position = Geschlossene Positionen
tools-burn-category-zero-liquidity = Keine Liquidität
tools-burn-category-hint-open-position = Tokens aus offenen Positionen können nicht verbrannt werden
tools-burn-category-hint-has-value = Besser verkaufen statt verbrennen
tools-burn-category-hint-closed-position = Reste aus geschlossenen Trades
tools-burn-category-hint-zero-liquidity = Sicher zu verbrennen – kein Marktwert

tools-burn-confirm-title = Verbrennen bestätigen
tools-burn-confirm-message =
    { $count ->
        [one] Möchten Sie wirklich <strong>{ $count }</strong> Token verbrennen?
       *[other] Möchten Sie wirklich <strong>{ $count }</strong> Tokens verbrennen?
    }
tools-burn-confirm-value = Geschätzter Gesamtwert: <strong>{ $amount }</strong>
tools-burn-confirm-continue = Weiter
tools-burn-final-title = Letzte Warnung
tools-burn-final-headline = Diese Aktion ist UNUMKEHRBAR!
tools-burn-final-message =
    { $count ->
        [one] Der folgende { $count } Token wird dauerhaft vernichtet und kann unter keinen Umständen wiederhergestellt werden.
       *[other] Die folgenden { $count } Tokens werden dauerhaft vernichtet und können unter keinen Umständen wiederhergestellt werden.
    }
tools-burn-final-confirm = Ja, Tokens verbrennen
tools-burn-toast-burned =
    { $total ->
        [one] { $successful }/{ $total } Token verbrannt. Führen Sie die Wallet-Bereinigung aus, um ca. { $amount } zurückzuholen
       *[other] { $successful }/{ $total } Tokens verbrannt. Führen Sie die Wallet-Bereinigung aus, um ca. { $amount } zurückzuholen
    }
tools-burn-toast-failed =
    { $count ->
        [one] { $count } Token konnte nicht verbrannt werden
       *[other] { $count } Tokens konnten nicht verbrannt werden
    }
tools-burn-failed = Verbrennen fehlgeschlagen: { $reason }
tools-burn-failures-title =
    { $count ->
        [one] { $count } Token konnte nicht verbrannt werden
       *[other] { $count } Tokens konnten nicht verbrannt werden
    }
tools-burn-failure-unknown = Es wurde kein Grund gemeldet

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = Info
tools-airdrop-about-body = Prüfen Sie auf ausstehende Airdrops, einlösbare Belohnungen und nicht abgeholte Zuteilungen bei beliebten Solana-Protokollen.
tools-airdrop-list-title = Verfügbare Airdrops
tools-airdrop-prompt = Klicken Sie auf „Airdrops prüfen“, um nach verfügbaren Ansprüchen zu suchen
tools-airdrop-action-check = Airdrops prüfen
tools-airdrop-action-claim-all = Alle einlösen

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = Generator-Optionen
tools-generator-warning-title = Bewahren Sie Ihre privaten Schlüssel sicher auf!
tools-generator-warning-body = Erzeugte Keypairs werden lokal erstellt und nie übertragen. Sichern Sie Ihre Schlüssel immer an einem sicheren Ort.
tools-generator-count-label = Anzahl der Wallets
tools-generator-vanity-label = Vanity-Adresse (beginnt mit bestimmten Zeichen)
tools-generator-prefix-label = Präfix
tools-generator-prefix-input =
    .placeholder = z. B. SOL
tools-generator-prefix-hint = Längere Präfixe benötigen exponentiell mehr Zeit zur Erzeugung
tools-generator-list-title = Erzeugte Wallets
tools-generator-empty = Noch keine Wallets erzeugt
tools-generator-action-generate = Erzeugen
tools-generator-action-generating = Wird erzeugt...
tools-generator-count-invalid = Bitte geben Sie eine Zahl zwischen 1 und 10 ein
tools-generator-no-keypairs = Keine Keypairs zurückgegeben
tools-generator-generated =
    { $count ->
        [one] { $count } Wallet erzeugt
       *[other] { $count } Wallets erzeugt
    }
tools-generator-failed = Wallets konnten nicht erzeugt werden: { $reason }
tools-generator-copy-public-key =
    .title = Öffentlichen Schlüssel kopieren
tools-generator-copy-private-key =
    .title = Privaten Schlüssel kopieren
tools-generator-remove =
    .title = Aus der Liste entfernen
tools-generator-reveal =
    .title = Privaten Schlüssel anzeigen
tools-generator-public-key-label = Öffentlicher Schlüssel:
tools-generator-private-key-label = Privater Schlüssel:
tools-generator-public-key-name = Öffentlicher Schlüssel
tools-generator-private-key-copied = Privater Schlüssel kopiert
tools-generator-private-key-warning = Wer diesen Schlüssel besitzt, kontrolliert die Wallet
tools-generator-export-empty = Keine Wallets zum Exportieren
tools-generator-exported = Wallets exportiert – sicher aufbewahren

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = Zusammenfassung
tools-consolidation-stat-wallets = Sub-Wallets
tools-consolidation-stat-native = { -sol } gesamt
tools-consolidation-stat-tokens = Token-Typen
tools-consolidation-stat-rent = Zurückholbare Miete
tools-consolidation-wallets-title = Wallets
tools-consolidation-loading-wallets = Wallets werden geladen...
tools-consolidation-loading-data = Wallet-Daten werden geladen...
tools-consolidation-action-transfer-native = { -sol } übertragen
tools-consolidation-action-transfer-tokens = Alle Tokens übertragen
tools-consolidation-action-cleanup = ATAs bereinigen
tools-consolidation-action-transferring = Wird übertragen...
tools-consolidation-column-name = Name
tools-consolidation-column-address = Adresse
tools-consolidation-column-native = { -sol }-Guthaben
tools-consolidation-column-tokens = Tokens
tools-consolidation-column-atas = Leere ATAs
tools-consolidation-empty = Keine Sub-Wallets gefunden
tools-consolidation-empty-hint = Erstellen Sie Sub-Wallets mit Multi-Buy, um zu beginnen
tools-consolidation-load-failed = Laden fehlgeschlagen: { $reason }
tools-consolidation-select-prompt = Wallets zum Zusammenführen auswählen
tools-consolidation-selection-totals =
    | { $amount } | { $tokens ->
        [one] { $tokens } Token
       *[other] { $tokens } Tokens
    } | { $atas ->
        [one] { $atas } leeres ATA
       *[other] { $atas } leere ATAs
    }
tools-consolidation-transferred-native = { $amount } an die Haupt-Wallet übertragen
tools-consolidation-transferred-tokens =
    { $count ->
        [one] { $count } Token an die Haupt-Wallet übertragen
       *[other] { $count } Tokens an die Haupt-Wallet übertragen
    }
tools-consolidation-cleaned =
    { $count ->
        [one] { $count } ATA geschlossen, { $amount } zurückgeholt
       *[other] { $count } ATAs geschlossen, { $amount } zurückgeholt
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = Token
tools-multi-mint-label = Token-Mint-Adresse
tools-multi-mint-input =
    .placeholder = Token-Mint-Adresse einfügen...
tools-multi-execution-title = Ausführungseinstellungen
tools-multi-delay-min-label = Min. Verzögerung (ms)
tools-multi-delay-max-label = Max. Verzögerung (ms)
tools-multi-concurrency-label = Parallelität
tools-multi-concurrency-sequential = { $count } (sequenziell)
tools-multi-concurrency-parallel = { $count } parallel
tools-multi-slippage-label = Slippage (%)
tools-multi-router-label = Router
tools-multi-router-auto = Auto (beste Route)
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = Direkter Pool
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = Fortschritt
tools-multi-progress-preparing = Wird vorbereitet...
tools-multi-status-line = { $label } ({ $completed }/{ $total })
tools-multi-column-wallet = Wallet
tools-multi-column-route = Route
tools-multi-column-status = Status
tools-multi-op-completed = Abgeschlossen
tools-multi-op-failed = Fehlgeschlagen
tools-multi-action-stop = Stopp
tools-multi-action-loading = Wird geladen...
tools-multi-start-failed = Start fehlgeschlagen: { $reason }

tools-multi-state-pending = Ausstehend
tools-multi-state-funding = Finanzierung
tools-multi-state-executing = Ausführung
tools-multi-state-consolidating = Zusammenführung
tools-multi-state-completed = Abgeschlossen
tools-multi-state-failed = Fehlgeschlagen
tools-multi-state-aborted = Abgebrochen

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = Der Token, den Sie über mehrere Wallets kaufen möchten
tools-multi-buy-wallets-title = Wallet-Einstellungen
tools-multi-buy-wallet-count-label = Wallet-Anzahl
tools-multi-buy-wallet-count-option =
    { $count ->
        [one] { $count } Wallet
       *[other] { $count } Wallets
    }
tools-multi-buy-wallet-count-hint = Anzahl der zu verwendenden Sub-Wallets
tools-multi-buy-buffer-label = { -sol }-Puffer pro Wallet
tools-multi-buy-buffer-hint = Für Gebühren reserviert (mind. 0,015 { -sol })
tools-multi-buy-amounts-title = Betragseinstellungen
tools-multi-buy-min-label = Min. { -sol } pro Wallet
tools-multi-buy-min-hint = Minimaler Kaufbetrag
tools-multi-buy-max-label = Max. { -sol } pro Wallet
tools-multi-buy-max-hint = Maximaler Kaufbetrag
tools-multi-buy-limit-label = Gesamtlimit in { -sol } (optional)
tools-multi-buy-limit-hint = Maximale Gesamtausgaben
tools-multi-buy-preview-title = Vorschau
tools-multi-buy-preview-create = Zu erstellende Wallets
tools-multi-buy-preview-amount = Betrag pro Wallet
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = Benötigte { -sol } gesamt
tools-multi-buy-preview-balance = Haupt-Guthaben
tools-multi-buy-action-preview = Vorschau
tools-multi-buy-action-start = Multi-Buy starten
tools-multi-buy-executing = Käufe werden ausgeführt...
tools-multi-buy-column-spent = Ausgegebene { -sol }
tools-multi-buy-column-tokens = Tokens
tools-multi-buy-preview-failed = Vorschau fehlgeschlagen: { $reason }
tools-multi-buy-started = Multi-Buy gestartet
tools-multi-buy-stopped = Multi-Buy gestoppt
tools-multi-buy-completed = Multi-Buy abgeschlossen! { $successful }/{ $total } erfolgreich

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = Geben Sie eine Token-Adresse ein, um nach Wallets zu suchen, die ihn halten
tools-multi-sell-action-scan = Scannen
tools-multi-sell-settings-title = Verkaufseinstellungen
tools-multi-sell-percent-label = Verkaufsanteil
tools-multi-sell-percent-hint = % der Tokens, die pro Wallet verkauft werden
tools-multi-sell-min-fee-label = Min. { -sol } für Gebühr
tools-multi-sell-min-fee-hint = Mindestens benötigte { -sol } für die Transaktionsgebühr
tools-multi-sell-topup-label = Bei Bedarf automatisch aufladen
tools-multi-sell-topup-hint = { -sol } von der Haupt-Wallet übertragen, wenn das Guthaben einer Sub-Wallet nicht ausreicht
tools-multi-sell-post-title = Aktionen nach dem Verkauf
tools-multi-sell-consolidate-label = { -sol } in der Haupt-Wallet zusammenführen
tools-multi-sell-consolidate-hint = Gesamte { -sol } aus den Sub-Wallets zurück an die Haupt-Wallet übertragen
tools-multi-sell-close-atas-label = Token-ATAs nach dem Verkauf schließen
tools-multi-sell-close-atas-hint = Ca. 0,002 { -sol } pro ATA zurückholen
tools-multi-sell-wallets-title = Wallets mit Token
tools-multi-sell-empty = Keine Sub-Wallet hält diesen Token
tools-multi-sell-column-tokens = Tokens
tools-multi-sell-column-native = { -sol }-Guthaben
tools-multi-sell-column-topup = Benötigt Aufladung
tools-multi-sell-none-selected = Keine Wallets ausgewählt
tools-multi-sell-select-required = Bitte wählen Sie mindestens eine Wallet aus
tools-multi-sell-action-start = Multi-Sell starten
tools-multi-sell-executing = Verkäufe werden ausgeführt...
tools-multi-sell-column-sold = Verkaufte Tokens
tools-multi-sell-column-received = Erhaltene { -sol }
tools-multi-sell-started = Multi-Sell gestartet
tools-multi-sell-stopped = Multi-Sell gestoppt
tools-multi-sell-completed = Multi-Sell abgeschlossen! { $amount } erhalten

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = Favoriten
tools-favorites-saved = Gespeicherte Favoriten
tools-favorites-save-current = Aktuelle speichern
tools-favorites-empty = Noch keine Favoriten gespeichert
tools-favorites-no-label = Keine Bezeichnung
tools-favorites-uses = { $count }x
tools-favorites-remove = Entfernen
tools-favorites-loaded = Favorit geladen: { $name }
tools-favorites-default-name = Konfig.
tools-favorites-mint-required = Bitte geben Sie zuerst eine Token-Mint-Adresse ein
tools-favorites-add-title = Favorit hinzufügen
tools-favorites-add-message = Geben Sie eine Bezeichnung für diesen Favoriten ein
tools-favorites-add-placeholder = Bezeichnung (optional)...
tools-favorites-saved-toast = In Favoriten gespeichert
tools-favorites-save-failed = Favorit konnte nicht gespeichert werden
tools-favorites-remove-title = Favorit entfernen
tools-favorites-remove-message = Diesen Favoriten entfernen?
tools-favorites-removed-toast = Favorit entfernt
tools-favorites-remove-failed = Favorit konnte nicht entfernt werden
