## Categories

hints-category-tokens = Tokens
hints-category-positions = Positionen
hints-category-filtering = Filterung
hints-category-trader = Auto-Trader
hints-category-services = Dienste
hints-category-wallet = Wallet
hints-category-wallets = Wallets
hints-category-tools = Tools
hints-category-config = Konfiguration
hints-category-config-telegram = { -telegram }
hints-category-token-details = Token-Details
hints-category-ui = Oberfläche

## tokens

hints-tokens-pool-service-title = Pool-Service-Tokens
hints-tokens-pool-service-content =
    Hier angezeigte Tokens haben:

    • **Alle Filterkriterien bestanden** — Liquiditäts-, Volumen-, Alters- und Sicherheitsprüfungen
    • **Gültige SOL-Liquiditätspools** — von unseren DEX-Decodern unterstützt (Raydium, Orca, Meteora usw.)
    • **Erfolgreiche Preisberechnung** — Preise direkt aus den On-Chain-Pool-Reserven berechnet

    Dies ist die zuverlässigste Token-Liste fürs Trading, da die Preise aus echten Pool-Daten stammen und nicht aus externen APIs.

    Klicken Sie auf einen Token, um Details anzusehen und den Blacklist-Status zu verwalten.
hints-tokens-no-market-title = Keine Marktdaten
hints-tokens-no-market-content =
    On-Chain entdeckte Tokens, für die Marktdaten von { -dexscreener } oder { -geckoterminal } fehlen.

    Häufige Gründe:
    • **Sehr neue Tokens** — noch nicht von Aggregatoren indexiert
    • **Geringes Handelsvolumen** — unter den Schwellenwerten der Aggregatoren
    • **Nicht gelistete Paare** — Handel auf DEXs, die Aggregatoren nicht erfassen

    Diese Tokens können dennoch gültige Pools haben und handelbar sein, es fehlen jedoch externe Marktkennzahlen.
hints-tokens-all-title = Alle Tokens
hints-tokens-all-content =
    Vollständige Datenbank aller entdeckten Tokens, unabhängig vom Filterstatus.

    Enthält:
    • Tokens, die die Filterung bestanden haben
    • Abgelehnte Tokens
    • Tokens ohne Marktdaten
    • Tokens auf der Blacklist

    Nutzen Sie diese Ansicht zur Recherche oder um Tokens zu finden, die herausgefiltert wurden.
hints-tokens-passed-title = Filterung bestanden
hints-tokens-passed-content =
    Tokens, die alle aktiven Filterkriterien bestanden haben.

    Die Filterung prüft unter anderem:
    • **Liquidität** — Mindestschwelle für SOL-Liquidität
    • **Volumen** — Anforderungen an das 24-Std.-Handelsvolumen
    • **Token-Alter** — Mindestzeit seit der Erstellung
    • **Sicherheit** — Grenzwerte für den { -rugcheck }-Risikoscore
    • **Marktkapitalisierung** — optionale FDV-/MC-Filter

    Filter konfigurieren Sie auf der Seite **Filterung**.
hints-tokens-rejected-title = Abgelehnte Tokens
hints-tokens-rejected-content =
    Tokens, die ein oder mehrere Filterkriterien nicht erfüllt haben.

    Jeder Token zeigt den konkreten Ablehnungsgrund:
    • Welcher Filter fehlgeschlagen ist
    • Der tatsächliche Wert im Vergleich zum geforderten Schwellenwert
    • Wann die Prüfung stattfand

    Prüfen Sie abgelehnte Tokens, um Ihre Filtereinstellungen zu verfeinern.
hints-tokens-blacklisted-title = Tokens auf der Blacklist
hints-tokens-blacklisted-content =
    Dauerhaft vom Trading ausgeschlossene Tokens.

    Gründe für die Blacklist:
    • **Manuelle Blacklist** — Tokens, die Sie ausdrücklich gesperrt haben
    • **Sicherheitsrisiken** — erkannte Rug-Pull-Indikatoren
    • **Verlustschwelle** — konfigurierte Verlustlimits überschritten
    • **Fehlgeschlagene Transaktionen** — wiederholte Swap-Fehler

    Tokens auf der Blacklist erscheinen nie in Bestanden-Listen und werden nie fürs Auto-Trading berücksichtigt.
hints-tokens-positions-title = Positions-Tokens
hints-tokens-positions-content =
    Tokens, die aktuell in offenen Positionen gehalten werden.

    Zeigt Echtzeitdaten zu Ihren aktiven Beständen:
    • Aktueller Preis aus den Pool-Reserven
    • Unrealisierte GuV
    • Positionsgröße und Einstiegspreis
    • Haltedauer

    Klicken Sie auf einen Token für die detaillierte Positionsverwaltung.
hints-tokens-recent-title = Kürzlich entdeckt
hints-tokens-recent-content =
    Neu entdeckte Tokens, sortiert nach Entdeckungszeit.

    Nützlich für:
    • Neue Token-Launches erkennen
    • Frische Liquidität beobachten
    • Frühe Einstiegsgelegenheiten

    Hinweis: Neue Tokens haben anfangs oft unvollständige Marktdaten.
hints-tokens-ohlcv-title = OHLCV-Datenverwaltung
hints-tokens-ohlcv-content =
    Anzeigen und Verwalten der für Tokens gespeicherten OHLCV-Daten (Kerzen).

    Zeigt:
    • **Kerzenanzahl** — insgesamt gespeicherte Datenpunkte
    • **Backfill-Fortschritt** — Abschlussstatus je Zeitrahmen
    • **Datenspanne** — Zeitabdeckung in Stunden
    • **Pool-Anzahl** — verfolgte Liquiditätspools
    • **Status** — aktiv überwacht oder inaktiv

    Aktionen:
    • **Löschen** — alle OHLCV-Daten eines Tokens entfernen
    • **Bereinigen** — Daten inaktiver Tokens gesammelt entfernen

    OHLCV-Daten werden dauerhaft aufbewahrt und nie automatisch gelöscht.

## positions

hints-positions-overview-title = Positionsübersicht
hints-positions-overview-content =
    Ihre aktuellen Token-Bestände und Trading-Positionen.

    Wichtige Kennzahlen:
    • **Einstiegspreis** — durchschnittlich gezahlter Preis (inkl. DCA)
    • **Aktueller Preis** — Live-Preis aus den Pool-Reserven
    • **GuV** — unrealisierter Gewinn/Verlust in SOL und %
    • **Größe** — insgesamt gehaltene Token-Menge

    Klicken Sie auf eine Position für detaillierte Verwaltungsoptionen.
hints-positions-dca-title = DCA (Durchschnittskosten-Strategie)
hints-positions-dca-content =
    Mit DCA können Sie zu unterschiedlichen Preisen zu bestehenden Positionen nachkaufen.

    Wenn DCA ausgelöst wird:
    • Zusätzliche Tokens werden gekauft
    • Der Einstiegspreis wird als gewichteter Durchschnitt neu berechnet
    • Die Positionsgröße steigt
    • Der Einstiegszähler erhöht sich

    DCA-Regeln konfigurieren Sie in den **Auto-Trader**-Einstellungen.
hints-positions-partial-exit-title = Teilausstieg
hints-positions-partial-exit-content =
    Verkaufen Sie einen Teil Ihrer Position und behalten Sie den Rest.

    Vorteile:
    • Teilgewinne sichern und weiter investiert bleiben
    • Positionsgröße verringern, ohne vollständig zu schließen
    • Take-Profit-Staffeln umsetzen

    Jeder Teilausstieg wird separat erfasst, damit die GuV exakt nachverfolgt wird.
hints-positions-management-title = Positionsverwaltung
hints-positions-management-content =
    Die Verwaltung legt fest, welche Automatisierung auf eine Position zugreifen darf:

    • Auto-Trader: Sicherheitsausstiege, Regelausstiege und Auto-DCA
    • Nur Nutzer: keine automatischen Aktionen
    • Copy-Aufgabe: Sicherheitsausstiege und Copy-Verkäufe
    • Hybrid: Sicherheitsausstiege, Regelausstiege und Copy-Verkäufe

    Sie verkaufen oder kaufen selbst nach. Manuelle Käufe sind standardmäßig manuell verwaltet, damit der Bot keinen Token verkauft, den Sie bewusst gekauft haben. Deaktivieren Sie dies, um die Position an den Auto-Trader zurückzugeben.

## filtering

hints-filtering-overview-title = Token-Filterung
hints-filtering-overview-content =
    Die Filterung bestimmt, welche Tokens für den Handel infrage kommen.

    Tokens müssen **alle aktivierten Kriterien** erfüllen, um in der Bestanden-Liste zu erscheinen:
    • { -dexscreener }-Kennzahlen (Liquidität, Volumen usw.)
    • { -geckoterminal }-Kennzahlen (Marktkapitalisierung, FDV)
    • { -rugcheck }-Sicherheitsanalyse
    • Meta-Filter (Token-Alter usw.)

    Deaktivierte Kriterien werden vollständig übersprungen.
hints-filtering-dexscreener-title = { -dexscreener }-Filter
hints-filtering-dexscreener-content =
    Filter auf Basis der { -dexscreener }-Marktdaten:

    • **Liquidität** — minimale USD-Liquidität in Pools
    • **Volumen 24 Std.** — minimales Handelsvolumen
    • **Transaktionen** — Aktivitätsschwellen (Käufe/Verkäufe)
    • **Preisänderung** — Volatilitätsfilter

    { -dexscreener }-Daten werden alle paar Minuten aktualisiert.
hints-filtering-geckoterminal-title = { -geckoterminal }-Filter
hints-filtering-geckoterminal-content =
    Filter auf Basis der { -geckoterminal }-Marktdaten:

    • **Marktkapitalisierung** — minimale Marktkapitalisierung
    • **FDV** — Grenzen der voll verwässerten Bewertung
    • **Reserve-Verhältnis** — Indikatoren für die Pool-Gesundheit

    { -geckoterminal } hat oft Daten zu neueren Tokens.
hints-filtering-rugcheck-title = Sicherheitsfilter
hints-filtering-rugcheck-content =
    Sicherheitsanalyse von { -rugcheck }.xyz:

    • **Risikoscore** — Gesamtrisikobewertung (0–100)
    • **Mint-Berechtigung** — können neue Tokens geprägt werden?
    • **Freeze-Berechtigung** — können Transfers eingefroren werden?
    • **Top-Holder** — Konzentrationsrisiko

    Höhere Risikoscores deuten auf mehr mögliche Warnsignale hin.
hints-filtering-meta-title = Meta-Filter
hints-filtering-meta-content =
    Zusätzliche Filterkriterien:

    • **Token-Alter** — Mindestzeit seit der Token-Erstellung
    • **Pool-Alter** — Mindestzeit seit der Pool-Erstellung
    • **Hat Website** — Social-/Website-Links verlangen
    • **Hat Socials** — Twitter/{ -telegram } verlangen

    Sie helfen, sehr neue oder verdächtige Tokens auszusortieren.

## trader

hints-trader-overview-title = Auto-Trader
hints-trader-overview-content =
    Automatisierte Trading-Engine, die Tokens überwacht und Trades ausführt.

    Komponenten:
    • **Einstiegsmonitor** — sucht nach Kaufgelegenheiten
    • **Ausstiegsmonitor** — verwaltet Verkäufe und Take-Profits
    • **DCA-Monitor** — steuert das Nachkaufen zur Durchschnittsbildung
    • **Risikokontrollen** — Verlustlimits und Sicherheitsschranken

    Starten und stoppen Sie das Trading im Kontrollfeld.
hints-trader-entry-title = Einstiegsmonitor
hints-trader-entry-content =
    Überwacht gefilterte Tokens auf Einstiegssignale.

    Die Einstiegsbewertung prüft:
    • Token besteht die aktuelle Filterung
    • Noch nicht in einer Position
    • Nicht auf der Blacklist
    • Positionslimits nicht überschritten
    • Strategiebedingungen erfüllt (falls konfiguriert)

    Einstiegsgröße und Limits konfigurieren Sie in der Konfiguration.
hints-trader-exit-title = Ausstiegsmonitor
hints-trader-exit-content =
    Überwacht offene Positionen auf Ausstiegssignale.

    Ausstiegsauslöser:
    • **Take Profit** — Kursziel erreicht
    • **Stop-Loss** — maximaler Verlust überschritten
    • **Trailing-Stop** — Preis ist vom Hoch zurückgefallen
    • **Strategie-Ausstieg** — eigene Bedingungen erfüllt
    • **Zeitbasiert** — maximale Haltedauer

    Schwellenwerte konfigurieren Sie in der Konfiguration.

## services

hints-services-overview-title = Systemdienste
hints-services-overview-content =
    Hintergrunddienste, die { -brand } antreiben.

    Dienstzustände:
    • **Läuft** (grün) — arbeitet normal
    • **Startet** (gelb) — wird initialisiert
    • **Gestoppt** (rot) — läuft nicht
    • **Fehler** (Warnung) — fehlgeschlagen, startet ggf. automatisch neu

    Dienste haben Abhängigkeiten und starten in Reihenfolge.
hints-services-health-title = Dienstzustand
hints-services-health-content =
    Zustandsanzeigen zeigen den Status eines Dienstes:

    • **Laufzeit** — Zeit seit dem letzten Start
    • **Aufgaben** — aktive Hintergrundvorgänge
    • **Fehler** — Anzahl aktueller Fehler
    • **Metriken** — Leistungsdaten (falls verfügbar)

    Kritische Dienste beeinflussen die Trading-Fähigkeit.

## wallet

hints-wallet-overview-title = Wallet-Übersicht
hints-wallet-overview-content =
    Status Ihrer verbundenen Solana-Wallet.

    Zeigt:
    • **SOL-Guthaben** — natives SOL für Gebühren und Trading
    • **Token-Bestände** — SPL-Tokens mit Werten
    • **24-Std.-Änderung** — Änderung des Portfoliowerts
    • **Verlauf** — Guthaben-Snapshots im Zeitverlauf

    Guthaben werden jede Minute aktualisiert.
hints-wallet-tokens-title = Token-Guthaben
hints-wallet-tokens-content =
    In Ihrer Wallet gehaltene SPL-Tokens.

    Zeigt:
    • Token-Symbol und -Name
    • Gehaltene Menge
    • Aktueller Wert in SOL/USD
    • Preis aus Pool- oder Marktdaten

    Leere Token-Konten können in den Einstellungen bereinigt werden.

## wallets

hints-wallets-main-title = Haupt-Wallet
hints-wallets-main-content =
    Die primäre Wallet für alle Trading-Vorgänge.

    • **Auto-Trading** — Einstiegs- und Ausstiegstrades werden von dieser Wallet ausgeführt
    • **Guthabenanzeige** — wird in Kopfzeile und Dashboard angezeigt
    • **Token-Bestände** — von dieser Wallet gehaltene SPL-Tokens

    Die Haupt-Wallet ändern Sie, indem Sie bei einer sekundären Wallet „Als Haupt-Wallet festlegen“ wählen.
hints-wallets-secondary-title = Sekundäre Wallets
hints-wallets-secondary-content =
    Zusätzliche Wallets für Multi-Wallet-Vorgänge.

    • **Multi-Wallet-Trading** — Käufe/Verkäufe über mehrere Wallets koordinieren
    • **Portfolio-Trennung** — nach Strategie oder Zweck organisieren
    • **Eigene Guthaben** — jede Wallet hat eigenes SOL und eigene Tokens

    Sekundäre Wallets werden vom Auto-Trading nur verwendet, wenn sie ausdrücklich konfiguriert sind.

## tools

hints-tools-wallet-cleanup-title = Wallet-Bereinigung
hints-tools-wallet-cleanup-content =
    { "*" }*SOL aus leeren Token-Konten zurückholen**

    { "*" }*Was sind ATAs?**
    Associated Token Accounts (ATAs) sind Solana-Konten, die Ihre Tokens halten. Jeder Token, mit dem Sie interagieren, erzeugt ein ATA, das ca. 0,002 SOL Miete bindet.

    { "*" }*Warum leere ATAs bereinigen?**
    • Miete zurückholen (ca. 0,002 SOL pro ATA)
    • Aktive Trader können Hunderte leere ATAs ansammeln
    • 100 leere ATAs = ca. 0,2 SOL zurückholbar

    { "*" }*So funktioniert es:**
    • Scannt Ihre Wallet nach ATAs mit Guthaben null
    • Zeigt den insgesamt zurückholbaren SOL-Betrag
    • Schließt leere Konten, um die Miete zurückzuerhalten

    { "*" }*Auto-Bereinigung:**
    Wenn aktiviert, werden leere ATAs alle 5 Minuten im Hintergrund automatisch gescannt und geschlossen.

    { "*" }*Wichtig:**
    • Schließt nur Konten mit einem Guthaben von genau 0
    • Fehlgeschlagene Schließungen werden zwischengespeichert, um wiederholte Versuche zu vermeiden
    • Große Wallets erfordern ggf. mehrere Bereinigungsläufe
hints-tools-burn-tokens-title = Tokens verbrennen
hints-tools-burn-tokens-content =
    { "*" }*Tokens dauerhaft vernichten**

    Beim Verbrennen werden Tokens dauerhaft aus Ihrer Wallet und aus dem Umlauf entfernt.

    { "*" }*Was beim Verbrennen passiert:**
    • Tokens werden an eine Burn-Adresse gesendet (nicht wiederherstellbar)
    • Das Token-Guthaben wird null
    • Das ATA kann danach über die Wallet-Bereinigung geschlossen werden, um ca. 0,002 SOL Miete zurückzuholen

    { "*" }*Token-Kategorien:**
    • **Offene Positionen** - Können nicht verbrannt werden (aktive Trades)
    • **Geschlossene Positionen** - Reste aus früheren Trades
    • **Mit Wert** - Tokens mit Liquidität (besser verkaufen)
    • **Keine Liquidität** - Dust/wertlose Tokens (sicher zu verbrennen)

    { "*" }*Warnung:** Diese Aktion ist **unumkehrbar**. Verbrannte Tokens können unter keinen Umständen wiederhergestellt werden.

    { "*" }*Nach dem Verbrennen:** Führen Sie die Wallet-Bereinigung aus, um leere ATAs zu schließen und SOL-Miete zurückzuholen.
hints-tools-wallet-generator-title = Wallet-Generator
hints-tools-wallet-generator-content =
    { "*" }*Neue Solana-Keypairs erzeugen**

    Erstellen Sie neue Wallets sicher auf Ihrem Gerät.

    { "*" }*Funktionen:**
    • Erzeugt kryptografisch sichere Keypairs
    • Optionales Vanity-Adresspräfix (z. B. „SOL...“)
    • Export als base58 oder JSON-Array

    { "*" }*Sicherheit:**
    • Schlüssel werden lokal erzeugt
    • Werden nie über das Netzwerk übertragen
    • Sichern Sie Schlüssel immer sicher
hints-tools-multi-buy-title = Multi-Buy
hints-tools-multi-buy-content =
    { "*" }*Käufe über mehrere Wallets koordinieren**

    Führen Sie Kaufaufträge über mehrere Sub-Wallets mit zufälligen Beträgen aus, um organische Kaufaktivität zu simulieren.

    { "*" }*So funktioniert es:**
    1. Erstellt Sub-Wallets oder nutzt vorhandene
    2. Verteilt SOL von der Haupt-Wallet auf die Sub-Wallets
    3. Führt Kaufaufträge mit zufälligen Beträgen und Verzögerungen aus
    4. Jede Wallet kauft unabhängig mit eigenen Signaturen

    { "*" }*Wallet-Einstellungen:**
    • **Wallet-Anzahl** — Anzahl der zu verwendenden Sub-Wallets (2–10)
    • **SOL-Puffer** — pro Wallet für Gebühren reserviertes SOL (ca. 0,015)

    { "*" }*Betragseinstellungen:**
    • **Min./Max. SOL** — Bereich der Kaufbeträge pro Wallet
    • **Gesamtlimit** — optionale Obergrenze für das insgesamt ausgegebene SOL

    { "*" }*Ausführungseinstellungen:**
    • **Verzögerung** — zufällige Verzögerung zwischen Transaktionen
    • **Parallelität** — parallele Ausführung (1 = sequenziell)
    • **Slippage** — maximal akzeptable Slippage
    • **Router** — Swap-Routing (Auto, Jupiter, Raydium)

    { "*" }*Wichtig:**
    • Erfordert ausreichend SOL in der Haupt-Wallet
    • Fehlgeschlagene Käufe werden protokolliert, stoppen die Sitzung aber nicht
    • Sub-Wallets können sitzungsübergreifend wiederverwendet werden
hints-tools-multi-sell-title = Multi-Sell
hints-tools-multi-sell-content =
    { "*" }*Verkäufe über mehrere Wallets koordinieren**

    Verkaufen Sie einen bestimmten Token aus allen Sub-Wallets, die ihn halten, mit automatischer SOL-Zusammenführung.

    { "*" }*So funktioniert es:**
    1. Scannt Sub-Wallets nach Token-Guthaben
    2. Füllt Wallets mit wenig SOL optional für Gebühren auf
    3. Führt Verkaufsaufträge mit konfigurierbarem Prozentsatz aus
    4. Führt den Erlös in der Haupt-Wallet zusammen

    { "*" }*Verkaufseinstellungen:**
    • **Verkauf %** — Prozentsatz der zu verkaufenden Tokens (Standard 100 %)
    • **Min. SOL für Gebühr** — für die Transaktion mindestens benötigtes SOL
    • **Auto-Aufladung** — bei Bedarf SOL von der Haupt-Wallet übertragen

    { "*" }*Aktionen nach dem Verkauf:**
    • **SOL zusammenführen** — gesamtes SOL zurück an die Haupt-Wallet übertragen
    • **ATAs schließen** — Token-Konten schließen, um Miete zurückzuholen (je ca. 0,002 SOL)

    { "*" }*Ausführungseinstellungen:**
    • **Verzögerung** — zufällige Verzögerung zwischen Transaktionen
    • **Parallelität** — parallele Ausführung
    • **Slippage** — maximal akzeptable Slippage
    • **Router** — bevorzugtes Swap-Routing

    { "*" }*Tipps:**
    • Die Vorschau zeigt alle Wallets, die den Token halten
    • Wählen Sie Wallets ab, aus denen Sie nicht verkaufen möchten
    • Die Zusammenführung erfolgt nach Abschluss aller Verkäufe
hints-tools-trade-watcher-title = Trade-Watcher
hints-tools-trade-watcher-content =
    { "*" }*Trades überwachen und automatische Aktionen auslösen**

    Beobachten Sie die Handelsaktivität eines Tokens und reagieren Sie automatisch auf Trades.

    { "*" }*Watch-Typen:**
    • **Kaufen bei Verkauf** — automatisch kaufen, wenn jemand verkauft (Dips mitnehmen)
    • **Verkaufen bei Kauf** — automatisch verkaufen, wenn jemand kauft (dem Markt folgen)
    • **Nur benachrichtigen** — Benachrichtigungen ohne Aktion

    { "*" }*So funktioniert es:**
    1. Token-Mint-Adresse eingeben
    2. Auf „Pools suchen“ klicken, um verfügbare Liquiditätspools zu finden
    3. Einen zu überwachenden Pool wählen (für Kauf-/Verkaufsaktionen erforderlich)
    4. Auslösebetrag festlegen (minimale Trade-Größe, auf die reagiert wird)
    5. Aktionsbetrag festlegen (wie viel SOL gekauft/verkauft wird)
    6. Watch starten

    { "*" }*Voraussetzungen:**
    • Gültige Token-Mint-Adresse
    • Pool-Auswahl (für Kauf-/Verkaufsaktionen)
    • Ausreichendes SOL-Guthaben für die Aktionsbeträge

    { "*" }*{ -telegram }-Integration:**
    Konfigurieren Sie { -telegram } unter Konfiguration → { -telegram }, um sofort benachrichtigt zu werden, wenn Watches auslösen.
hints-tools-wallet-consolidation-title = Wallet-Zusammenführung
hints-tools-wallet-consolidation-content =
    { "*" }*Sub-Wallet-Guthaben verwalten und zusammenführen**

    Zeigen Sie alle Sub-Wallets an und führen Sie SOL und Tokens zusammen sowie ATA-Miete zurück in Ihre Haupt-Wallet.

    { "*" }*Die Zusammenfassung zeigt:**
    • **Sub-Wallets** — Gesamtzahl der erstellten Sub-Wallets
    • **SOL gesamt** — kombiniertes SOL-Guthaben aller Sub-Wallets
    • **Token-Typen** — Anzahl verschiedener gehaltener Tokens
    • **Zurückholbare Miete** — in leeren ATAs gebundenes SOL

    { "*" }*Aktionen:**
    • **SOL übertragen** — gesamtes SOL der gewählten Wallets an die Haupt-Wallet senden
    • **Tokens übertragen** — alle Tokens an die Haupt-Wallet senden
    • **ATAs bereinigen** — leere Token-Konten für die Mietrückerstattung schließen

    { "*" }*Tabelleninfos:**
    • Kontrollkästchen zur Auswahl von Wallets für Stapelvorgänge
    • Name, Adresse, SOL-Guthaben, Token-Anzahl, leere ATAs
    • Leere Wallets sind abgedunkelt und leicht erkennbar

    { "*" }*Tipps:**
    • Nach Multi-Sell verwenden, um verbleibendes SOL einzusammeln
    • ATAs regelmäßig bereinigen, um Miete zurückzuholen
    • Leere Wallets können für künftige Vorgänge wiederverwendet werden

## config

hints-config-overview-title = Konfiguration
hints-config-overview-content =
    Systemweite Einstellungen für { -brand }.

    Kategorien:
    • **Trader** — Einstiegs-/Ausstiegsregeln, Positionsgrößen
    • **Filterung** — Schwellenwerte der Token-Filter
    • **Swaps** — Routing- und Slippage-Einstellungen
    • **RPC** — Node-Konfiguration
    • **Dienste** — Einstellungen der Hintergrunddienste

    Änderungen werden sofort wirksam (Hot Reload).
hints-config-telegram-title = { -telegram }-Benachrichtigungen
hints-config-telegram-content =
    { "*" }*Trading-Alarme sofort per { -telegram } erhalten**

    Lassen Sie sich direkt in { -telegram } über Trades, Positionen und wichtige Ereignisse benachrichtigen.

    { "*" }*Einrichtungsschritte:**

    1. **Bot erstellen:**
       • Öffnen Sie { -telegram } und schreiben Sie @BotFather
       • Senden Sie /newbot und folgen Sie den Anweisungen
       • Kopieren Sie das Bot-Token (sieht so aus: 123456:ABC-DEF...)

    2. **Chat-ID ermitteln:**
       • Schreiben Sie @userinfobot oder @getidsbot
       • Kopieren Sie die zurückgegebene numerische ID

    3. **In { -brand } konfigurieren:**
       • Schalter für Benachrichtigungen aktivieren
       • Bot-Token und Chat-ID einfügen
       • Auf „Verbindung testen“ klicken, um zu prüfen

    { "*" }*Das erhalten Sie:**
    • Bestätigungen von Trade-Ausführungen
    • Positionsupdates (Einstieg/Ausstieg)
    • Trade-Watcher-Alarme
    • Fehlerbenachrichtigungen

    { "*" }*Datenschutz:**
    Nachrichten werden direkt von { -brand } an Ihren { -telegram }-Bot gesendet — ohne Server Dritter.
hints-config-telegram-password-title = Bot-Authentifizierungspasswort
hints-config-telegram-password-content =
    { "*" }*Sichern Sie Ihren { -telegram }-Bot mit Passwortauthentifizierung ab**

    Wenn Sie mit Ihrem { -brand }-{ -telegram }-Bot interagieren, müssen Sie sich vor sensiblen Befehlen mit diesem Passwort authentifizieren.

    { "*" }*Warum ein Passwort festlegen?**
    • Verhindert, dass Unbefugte Ihren Bot steuern
    • Erforderlich für Trading-Befehle über { -telegram }
    • Muss mindestens 8 Zeichen lang sein

    { "*" }*So funktioniert es:**
    1. Legen Sie hier im Dashboard ein Passwort fest
    2. Wenn Sie Ihrem Bot einen Trading-Befehl senden, fragt er nach Authentifizierung
    3. Geben Sie Ihr Passwort ein, um Ihre Identität zu bestätigen
    4. Aktivieren Sie optional 2FA für zusätzliche Sicherheit

    { "*" }*Hinweis:** Das Passwort wird als sicherer SHA256-Hash gespeichert — wir speichern nie den Klartext.
hints-config-telegram-totp-title = Zwei-Faktor-Authentifizierung (2FA)
hints-config-telegram-totp-content =
    { "*" }*Eine zusätzliche Sicherheitsebene mit TOTP-2FA**

    Die Zwei-Faktor-Authentifizierung nutzt zeitbasierte Einmalpasswörter (TOTP) aus Apps wie Google Authenticator, Authy oder 1Password.

    { "*" }*Warum 2FA aktivieren?**
    • Selbst wenn jemand Ihr Passwort kennt, kommt er ohne den Code nicht an Ihren Bot
    • 6-stellige Codes wechseln alle 30 Sekunden
    • Funktioniert nach der Einrichtung offline

    { "*" }*Einrichtung:**
    1. Klicken Sie auf „2FA aktivieren“ und geben Sie Ihr Passwort ein
    2. Scannen Sie den QR-Code mit Ihrer Authenticator-App
    3. Geben Sie den 6-stelligen Code ein, um die Einrichtung zu bestätigen

    { "*" }*Kompatible Apps:**
    • Google Authenticator
    • Authy
    • 1Password
    • Microsoft Authenticator
    • Jede TOTP-kompatible App

    { "*" }*Wichtig:** Bewahren Sie Ihren geheimen Schlüssel an einem sicheren Ort auf. Verlieren Sie den Zugriff auf Ihre Authenticator-App, müssen Sie 2FA in diesem Dashboard deaktivieren.

## token_details

hints-token-details-chart-title = Preis-Chart (OHLCV)
hints-token-details-chart-content =
    { "*" }*Wichtig:** Dieser Chart zeigt **zwischengespeicherte OHLCV-Daten** für die Strategiebewertung, *nicht* den Live-Ausführungspreis.

    { "*" }*Warum zwischengespeicherte Daten?**
    • **Zweck:** Wird von automatisierten Strategien und Indikatoren (z. B. RSI, MA) verwendet.
    • **Aktualität:** Aktualisierungen hängen von der Token-Priorität ab (offene Positionen = schnellere Updates).
    • **Quelle:** Aggregiert von { -dexscreener }/{ -geckoterminal }, nicht direkt per On-Chain-RPC.

    { "*" }*Die Realität des DEX-Preises:**
    In DeFi werden Tokens über **mehrere Pools** gehandelt (Raydium, Orca, Meteora). Jeder Pool hat je nach Liquiditätstiefe und jüngsten Trades einen eigenen Preis.
    • **Chart-Preis:** Ein Durchschnitt/Aggregat über die Märkte.
    • **Swap-Preis:** Der konkrete Kurs der besten Route im exakten Moment des Trades.

    { "*" }Rechnen Sie mit kleinen Abweichungen zwischen diesem Chart und Ihrem endgültigen Ausführungspreis.*

    { "*" }*Status:** „Warte auf Daten“ bedeutet, dass Hintergrundprozesse neue Kerzen abrufen.
hints-token-details-token-info-title = Token-Informationen
hints-token-details-token-info-content =
    Grundlegende Token-Metadaten aus On-Chain- und Marktquellen.

        • **Mint** — eindeutige Token-Adresse auf Solana (Klick zum Kopieren)
        • **Dezimalstellen** — Token-Genauigkeit (meist 6–9)
        • **Alter** — Zeit seit der Erstellung des primären Pools/Tokens
        • **DEX** — primärer Handelsplatz dieses Tokens
        • **Holder** — eindeutige Wallets, die den Token halten
        • **Top-10-Anteil** — % im Besitz der Top-10-Wallets

        Eine höhere Holder-Zahl und geringere Konzentration deuten in der Regel auf eine gesündere Verteilung hin.
hints-token-details-liquidity-title = Liquidität & Marktdaten
hints-token-details-liquidity-content =
    Marktkennzahlen aus dem SOL-Pool mit der höchsten Liquidität.

        • **FDV** — Preis × Gesamtangebot (Aggregator-Preis)
        • **Liquidität** — USD-Wert der Pool-Reserven
        • **Pool-SOL** / **Pool-Token** — Live-Reserven, die den Pool-Preis bestimmen

        { "*" }*Warum das wichtig ist:**
        • Tiefere Liquidität = geringere Slippage
        • Flache Pools können sich schon bei kleinen Trades bewegen
        • Pool-Reserven bestimmen direkt den Swap-Ausführungspreis

        Die Daten werden regelmäßig von { -dexscreener }/{ -geckoterminal } sowie durch On-Chain-Pool-Abfragen aktualisiert.
hints-token-details-market-pulse-title = Marktpuls
hints-token-details-market-pulse-content =
    Preisbewegung und USD-Handelsvolumen teilen dieselbe **5M / 1H / 6H / 24H**-Zeitachse, sodass Momentum und Beteiligung direkt verglichen werden können.

    { "*" }*Interpretation:**
    • **Preis** — vom Aggregator abgeleitete prozentuale Änderung, nicht der Live-Ausführungspreis des Pools.
    • **Hohes Volumen** — stärkeres Interesse, effizientere Preisfindung und leichtere Ausstiege.
    • **Niedriges Volumen** — höhere Slippage, größere Spreads und schwierigere große Ausstiege.
    • **Hohes Volumen + geringe Liquidität** — erhöhte Volatilität und Ausführungsrisiko.

    Marktdaten werden über { -dexscreener }/{ -geckoterminal } über die großen DEXs aggregiert, daher kann die Preisänderung vom aktuellen On-Chain-Pool-Preis abweichen.
hints-token-details-activity-title = Transaktionsaktivität (Anzahl)
hints-token-details-activity-content =
    Analysiert die **Anzahl der Trades** (Käufe vs. Verkäufe) über mehrere Zeiträume. Das zeigt die Absicht der Trader unabhängig von der Trade-Größe.

    { "*" }*Aufschlüsselung der Kennzahlen:**
    • **Zeiträume:** 5M-, 1H-, 6H-, 24H-Fenster.
    • **Balken:** Visuelles Verhältnis von Kaufanzahl (grün) zu Verkaufsanzahl (rot).
    • **Rate:** Trades pro Minute (z. B. „12.5/m“). Höhere Raten = virale Aktivität.
    • **Anzahlen:** Exakte Zahl der Käufe/Verkäufe und ihr prozentualer Anteil.

    { "*" }*Zusammenfassende Kennzahlen:**
    • **24H-Kauf-%:** >50 % ist bullisch (mehr Käufer), { "<" }50 % ist bärisch (mehr Verkäufer).
    • **Nettofluss:** Käufe minus Verkäufe. Positiv = Akkumulation.
    • **5M-Spike:** Wie viel schneller *gerade jetzt* gehandelt wird als im 1H-Durchschnitt.
      • **>1.0x:** Zunehmendes Interesse.
      • **>3.0x:** Viraler Ausbruch oder Panikereignis.
      • **{ "<" }1.0x:** Kühlt ab.

    { "*" }*Strategietipp:** Ein hoher „Kauf-%“ mit hohem „Spike-Faktor“ signalisiert oft einen starken Ausbruchs-Einstieg.
hints-token-details-security-title = Sicherheitsanalyse
hints-token-details-security-content =
    Risikobewertung von { -rugcheck }.xyz und On-Chain-Analyse.

    { "*" }*Sicherheitsscore (0–100):**
    Höhere Scores bedeuten sicherere Tokens. Einflussfaktoren:
    • Berechtigungen (Mint/Freeze)
    • Holder-Konzentration
    • LP-Sperrstatus
    • Bekannte Risikomuster

    { "*" }*Wichtige Risikoindikatoren:**
    • **Mint-Berechtigung** — kann neue Tokens erzeugen (Inflationsrisiko)
    • **Freeze-Berechtigung** — kann Token-Konten einfrieren
    • **Top-Holder-%** — Konzentrationsrisiko
    • **LP-Anbieter** — Anzahl der Liquiditätsanbieter

    Prüfen Sie die Sicherheit immer, bevor Sie größere Beträge handeln.
hints-token-details-pools-title = Liquiditätspools
hints-token-details-pools-content =
    Alle entdeckten Liquiditätspools für diesen Token.

    { "*" }*Warum mehrere Pools wichtig sind:**
    • Jeder Pool hat andere Liquidität und Preise
    • Swap-Router finden die beste Route über Pools hinweg
    • Der Preis kann zwischen Pools um 1–5 % abweichen

    { "*" }*Pool-Informationen:**
    • **DEX** — welche Börse den Pool hostet
    • **Liquidität** — USD-Wert der Pool-Reserven
    • **Volumen** — jüngste Handelsaktivität
    • **Preis** — aktueller Pool-Preis

    Der Pool-Service berechnet Preise aus dem SOL-Paar mit der höchsten Liquidität.

## ui

hints-ui-featured-title = Hervorgehoben
hints-ui-featured-content =
    Zuerst geboostete Tokens, dann Trend-Projekte von Jupiter und { -dexscreener }.

    { "*" }*Das sehen Sie:**
    • Geboostete Tokens — ihre Teams haben für die Bewerbung bezahlt — vorne angeheftet und golden markiert
    • Danach Trend-Tokens aus den Discovery-Boards
    • Klicken Sie auf einen Token, um alle Details zu öffnen

    { "*" }*Einen Token boosten:**
    Ein Boost kauft Sichtbarkeit, niemals eine Empfehlung. Geboostete Zeilen sind überall golden
    markiert, auch in Ihrer Token-Tabelle, sodass Sie stets den Unterschied erkennen. Boosten Sie einen Token unter
    { "*" }*screenerbot.io/boost**.

    { "*" }*Zeile ausblenden:**
    Blenden Sie sie unter **Einstellungen → Oberfläche → Hervorgehobene Zeile anzeigen** aus. Die Kopfzeilenaktion öffnet
    weiterhin die vollständige Ansicht Hervorgehoben.

## Hint popover chrome (ui/hint_popover.js)

hints-trigger =
    .aria-label = Hilfe: { $title }
hints-popover-close =
    .aria-label = Schließen
hints-popover-learn-more = Mehr erfahren
hints-popover-dismiss = Nicht mehr anzeigen
