tokens-result-source-live = Canlı piyasa verileri
tokens-result-source-unavailable = { $label } kullanılamıyor — yeniden deneniyor
tokens-result-source-not-listed = { $label } üzerinde listelenmemiş
tokens-result-security-available = Güvenlik raporu mevcut
tokens-result-security-missing = { -rugcheck } raporu yok
tokens-result-chart-available = Grafik verisi mevcut
tokens-result-chart-missing = Henüz grafik verisi yok

# Token details dialog: shared tab states (ui/token_details/state_handling.js)

tokens-state-error-title = Veriler yüklenemedi
tokens-state-offline = Çevrimdışı görünüyorsunuz.
tokens-state-request-failed = İstek birkaç denemeden sonra başarısız oldu.
tokens-state-waiting = Veriler bekleniyor…

# Token details dialog: chart (ui/token_details/chart_tab.js)

tokens-chart-marker-entry = Giriş
tokens-chart-level-stop-loss = Zarar Durdur
tokens-chart-level-take-profit = Kâr Al

# Token details dialog: transactions tab (ui/token_details/transactions_tab.js)

tokens-transactions-loading = İşlemler yükleniyor…
tokens-transactions-empty-title = İşlem yok
tokens-transactions-empty-history = Bu token için cüzdan işlem geçmişi yok.
tokens-transactions-empty-data = Bu token için işlem verisi yok.
tokens-transactions-error-title = İşlemler yüklenemedi
tokens-transactions-error-message = İşlem geçmişi geçici olarak kullanılamıyor.
tokens-transactions-activity-title = 24sa etkinlik
tokens-transactions-activity-subtitle = Saatlik cüzdan işlemleri
tokens-transactions-metric-total = Toplam
tokens-transactions-metric-buys = Alımlar
tokens-transactions-metric-sells = Satımlar
tokens-transactions-recent-title = Son işlemler
tokens-transactions-shown = Gösterilen: { $count }
tokens-transactions-column-time = Zaman
tokens-transactions-column-type = Tür
tokens-transactions-column-price = Fiyat ({ -sol })
tokens-transactions-column-total = Toplam ({ -sol })
tokens-transactions-chart-missing = Grafik kitaplığı eksik
tokens-transactions-view-solscan = İşlemi { -solscan } üzerinde görüntüle

# Token details dialog: positions tab (ui/token_details/positions_tab.js)

tokens-positions-empty-title = Pozisyon yok
tokens-positions-no-token = Token seçilmedi.
tokens-positions-empty-message = Bu token için henüz pozisyon yok. Açmak için Al'ı kullanın.
tokens-positions-loading = Pozisyon yükleniyor…
tokens-positions-from-wallet-history = Cüzdan geçmişinden
tokens-positions-frozen = Dondurulmuş — satılamaz
tokens-positions-no-cost-basis = Maliyet bazı yok
tokens-positions-history-incomplete = Geçmiş eksik
tokens-positions-dca-count = DCA { $count }
tokens-positions-exit-count = Çıkışlar { $count }
tokens-positions-fact-avg-entry = Ort. Giriş
tokens-positions-fact-current = Güncel
tokens-positions-fact-tokens = Tokenlar
tokens-positions-fact-opened = Açılış
tokens-positions-fact-exit-price = Çıkış Fiyatı
tokens-positions-fact-native-received = Alınan { -sol }
tokens-positions-fact-closed-reason = Kapanış Nedeni
tokens-positions-fact-target-min = Kâr Hedefi Min.
tokens-positions-fact-target-max = Kâr Hedefi Maks.
tokens-positions-fact-highest = En Yüksek Fiyat
tokens-positions-fact-lowest = En Düşük Fiyat
tokens-positions-section-range = Hedefler { "&" } aralık
tokens-positions-section-market = Piyasa { "&" } varlıklar
tokens-positions-kicker = Pozisyon
tokens-positions-fallback-symbol = Token
tokens-positions-realized-pnl = Gerçekleşmiş K/Z
tokens-positions-unrealized-pnl = Gerçekleşmemiş K/Z
tokens-positions-size = Boyut

# Token details dialog: security tab (ui/token_details/security_tab.js)
# Risk names and descriptions come from the RugCheck report and render as sent.

tokens-security-analysis-pending = { -rugcheck } analizi sürüyor...
tokens-security-analyzing = Güvenlik analiz ediliyor…
tokens-security-pulse-title = Güvenlik Nabzı
tokens-security-pending-caption = Risk sinyalleri hâlâ toplanıyor.
tokens-security-control-title = Token Kontrolü
tokens-security-control-meta = Yetki durumu
tokens-security-updated = Güncelleme: { $time }
tokens-security-score-caption = 100 üzerinden normalize edilmiş token risk puanı.
tokens-security-score-label = Puan
tokens-security-rugged = Rug pull
tokens-security-grade-analyzing = Analiz ediliyor
tokens-security-grade-shielded = Korumalı
tokens-security-grade-safe = Güvenli
tokens-security-grade-caution = Dikkat
tokens-security-grade-vulnerable = Savunmasız
tokens-security-grade-unknown = Bilinmiyor
tokens-security-metric-token-type = Token Türü
tokens-security-metric-total-holders = Toplam Holder
tokens-security-metric-lp-providers = LP Sağlayıcıları
tokens-security-metric-graph-insiders = Grafikteki İçeridekiler
tokens-security-insiders-detected = Tespit edildi ({ $count })
tokens-security-insiders-clean = Temiz
tokens-security-authority-mint = Mint
tokens-security-authority-freeze = Dondurma
tokens-security-authority-immutable = Değiştirilemez
tokens-security-authority-mutable = Değiştirilebilir
tokens-security-authority-revoked = İptal edildi
tokens-security-authority-active = Etkin
tokens-security-holder-health-title = Holder Sağlığı
tokens-security-holders-unique = benzersiz
tokens-security-creator-share = Oluşturan Payı
tokens-security-gauge-top-10 = İlk 10
tokens-security-concentration-unknown = Bilinmiyor
tokens-security-concentration-critical = Kritik
tokens-security-concentration-high = Yüksek
tokens-security-concentration-moderate = Orta
tokens-security-concentration-healthy = Sağlıklı
tokens-security-transfer-title = Transfer Vergisi
tokens-security-transfer-no-fee = Ücret Yok
tokens-security-transfer-fee-percentage = Ücret Yüzdesi
tokens-security-transfer-max-fee = Azami Ücret Miktarı
tokens-security-transfer-authority = Ücret Yetkisi
tokens-security-transfer-note = Her transferde { $percent } ücret alınır.
tokens-security-transfer-none = Transfer ücreti tespit edilmedi.
tokens-security-risks-title = Güvenlik Riskleri
tokens-security-risks-none = Güvenlik riski tespit edilmedi.
tokens-security-risk-fallback-name = Güvenlik sinyali
tokens-security-risks-critical = { $count } kritik
tokens-security-risks-warnings =
    { $count ->
        [one] { $count } uyarı
       *[other] { $count } uyarı
    }
tokens-security-risks-info = { $count } bilgi
tokens-security-risks-incidents =
    { $count ->
        [one] { $count } olay bulundu
       *[other] { $count } olay bulundu
    }
tokens-security-top-holders-title = En Büyük Holderlar
tokens-security-top-holders-concentration = Yoğunlaşma: { $percent }
tokens-security-insider = İçeriden

# Token details dialog: overview tab (ui/token_details/overview_tab.js)
# 5M/1H/6H/24H period codes and the chart timeframe buttons are id codes shared
# with the chart and stay as sent.

tokens-overview-chart-checking = Veriler denetleniyor…
tokens-overview-banner-open = Token bannerını aç
tokens-overview-headline-label = Ana piyasa metrikleri
tokens-overview-price = Fiyat
tokens-overview-market-cap = Piyasa Değeri
tokens-overview-liquidity = Likidite
tokens-overview-volume = Hacim
tokens-overview-no-tags = Etiket yok
tokens-overview-info-title = Token Bilgisi
tokens-overview-profile = Yayımlanmış profil
tokens-overview-fact-mint = Mint
tokens-overview-fact-decimals = Ondalık
tokens-overview-fact-age = Yaş
tokens-overview-fact-dex = DEX
tokens-overview-fact-holders = Holderlar
tokens-overview-fact-top-10 = İlk 10 Payı
tokens-overview-tags = Etiketler
tokens-overview-liquidity-title = Likidite { "&" } Piyasa
tokens-overview-fact-fdv = FDV
tokens-overview-fact-pool-native = Havuz { -sol }
tokens-overview-fact-pool-token = Havuz Token
tokens-overview-pool = Havuz
tokens-overview-pulse-title = Piyasa Nabzı
tokens-overview-activity-title = İşlem Etkinliği
tokens-overview-buy-share = Alım: { $percent }
tokens-overview-buy-sell-ratio = { $ratio } A/S
tokens-overview-buys-24h = Alım 24H
tokens-overview-sells-24h = Satım 24H
tokens-overview-net-flow = Net Akış
tokens-overview-total-24h = 24H Toplam
tokens-overview-average-24h = 24H Ort.
tokens-overview-spike-5m = 5M Ani Artış
tokens-overview-rate-per-hour = { $amount }/sa
tokens-overview-rate-per-minute = { $amount }/dk
tokens-overview-spike-factor = { $factor }×
tokens-overview-flow-counts = Alım: { $buys } ({ $buyPercent }), Satım: { $sells } ({ $sellPercent }), Toplam: { $total }
tokens-overview-flow-no-data = İşlem verisi yok

# Token details dialog: pools tab (ui/token_details/pools_links_tab.js)
# DEX names in pool data render as sent.

tokens-pools-empty-title = Havuz yok
tokens-pools-empty-message = Bu token için likidite havuzu tespit edilmedi.
tokens-pools-unknown = Bilinmiyor
tokens-pools-unknown-dex = Bilinmeyen DEX
tokens-pools-liquidity = Likidite
tokens-pools-volume-24h = 24sa Hacim
tokens-pools-base-role = Baz Rolü
tokens-pools-quote-role = Kote Rolü
tokens-pools-canonical = Kanonik
tokens-pools-summary-title = Havuz özeti
tokens-pools-breakdown-title = DEX dökümü
tokens-pools-all-title = Tüm havuzlar
tokens-pools-updated = Güncellendi
tokens-pools-role-base = Baz
tokens-pools-role-quote = Kote
tokens-pools-role-unknown = Bilinmiyor
tokens-pools-reserves = Rezerv hesapları
tokens-pools-no-reserves = Rezerv hesabı yok
tokens-pools-address-pool = Havuz
    .title = Havuzu kopyala
tokens-pools-address-base = Baz mint
    .title = Baz mint'i kopyala
tokens-pools-address-quote = Kote mint
    .title = Kote mint'i kopyala
    .title = Eşleşen mint'i kopyala

# Token details dialog: links tab (ui/token_details/pools_links_tab.js)

tokens-links-empty = Bu token için resmi web sitesi veya sosyal medya bağlantısı yok.
tokens-links-info-title = Token bilgisi
tokens-links-mint-address = Mint adresi
tokens-links-data-source = Veri kaynağı
tokens-links-security = Güvenlik
tokens-links-profile-title = Token profili
tokens-links-profile-published-title = Yayımlanmış profil içeriği
tokens-links-profile-published-note = Medya, açıklama ve resmi bağlantılar, yayımlanmadan önce incelenen ücretli profil içeriğidir. Bu, sahipliği veya token güvenliğini doğrulamaz.
tokens-links-profile-create-note = Bu tokenın herkese açık profiline incelenmiş bir logo, proje açıklaması ve resmi bağlantılar ekleyin.
tokens-links-profile-update-hint = Bu token profilini screenerbot.io üzerinde güncelleyin
tokens-links-profile-create-hint = screenerbot.io üzerinde token profili oluşturun
tokens-links-profile-update = Profili güncelle
tokens-links-profile-create = Profil oluştur
tokens-links-media-title = Medya varlıkları
tokens-links-media-fallback-symbol = Token
tokens-links-media-logo = Logo
tokens-links-media-banner = Banner
tokens-links-media-banner-alt = { $symbol } banner
tokens-links-media-open = Görseli aç
tokens-links-description-title = Açıklama
tokens-links-explorers-title = Gezginler { "&" } analiz
tokens-links-websites-title = Resmi web siteleri
tokens-links-socials-title = Sosyal medya
tokens-links-explorer-solana-explorer = { -solana-explorer }
tokens-links-explorer-geckoterminal = { -geckoterminal }
tokens-links-explorer-dextools = { -dextools }
tokens-links-explorer-coingecko = { -coingecko }
tokens-links-explorer-jupiter-swap = { -jupiter } Swap
tokens-links-social-twitter = { -twitter } / { -x }
tokens-links-social-x = { -x } ({ -twitter })
tokens-links-social-telegram = { -telegram }
tokens-links-social-discord = { -discord }
tokens-links-social-medium = { -medium }
tokens-links-social-github = { -github }
tokens-links-social-youtube = { -youtube }
tokens-links-social-reddit = { -reddit }
tokens-links-social-facebook = { -facebook }
tokens-links-social-instagram = { -instagram }
tokens-links-social-linkedin = { -linkedin }
tokens-links-social-tiktok = { -tiktok }
tokens-links-social-fallback = Sosyal

# Token details dialog: frame, header and data sources (ui/token_details_dialog.js)

tokens-dialog-tab-overview = Genel Bakış
tokens-dialog-tab-security = Güvenlik
tokens-dialog-tab-positions = Pozisyonlar
tokens-dialog-tab-pools = Havuzlar
tokens-dialog-tab-links = Bağlantılar
tokens-dialog-tab-transactions = İşlemler
tokens-dialog-sections = Token ayrıntı bölümleri
tokens-dialog-close =
    .title = Kapat (ESC)
    .aria-label = Token ayrıntılarını kapat
tokens-dialog-unknown-symbol = Bilinmiyor
tokens-dialog-unknown-name = Bilinmeyen Token
tokens-dialog-market-summary = Piyasa özeti
tokens-dialog-price-loading = Fiyat yükleniyor
tokens-dialog-unit-native = { -sol }
tokens-dialog-market-metrics = Piyasa metrikleri
tokens-dialog-metric-market-cap = Piyasa değeri
tokens-dialog-metric-volume-24h = 24sa hacim
tokens-dialog-change-24h = 24 saatlik değişim { $change }
tokens-dialog-buy = Al
    .title = Bu tokenı al
tokens-dialog-sell = Sat
    .title = Pozisyonu sat
tokens-dialog-sell-unavailable = Satılacak açık pozisyon yok
tokens-dialog-details = Ayrıntılar
tokens-dialog-sources = Kaynaklar
tokens-dialog-sources-status = Veri kaynağı durumu
tokens-dialog-updated-label = Güncellendi
tokens-dialog-just-now = Az önce
tokens-dialog-updated-at = Güncelleme: { $time }
tokens-dialog-updated-unavailable = Güncelleme zamanı kullanılamıyor
tokens-dialog-error-title = Token verileri yüklenemedi
tokens-dialog-waiting-token = Token verileri bekleniyor…
tokens-dialog-loading-overview = Genel bakış yükleniyor…
tokens-dialog-loading-security = Güvenlik yükleniyor…
tokens-dialog-loading-pools = Havuzlar yükleniyor…
tokens-dialog-loading-links = Bağlantılar yükleniyor…
tokens-dialog-chart-still-checking = Henüz grafik verisi yok — denetim sürüyor…
tokens-dialog-no-data = Veri yok
tokens-dialog-source-token = Token
tokens-dialog-source-market = Piyasa
tokens-dialog-source-security = Güvenlik
tokens-dialog-source-chart = Grafik
tokens-dialog-status-pending = Bekliyor
tokens-dialog-status-loading = Yükleniyor
tokens-dialog-status-ready = Hazır
tokens-dialog-status-unavailable = Kullanılamıyor
tokens-dialog-status-cached = Önbellekte
tokens-dialog-source-summary = { $source } verisi: { $status }
tokens-dialog-badge-pool-price = Havuz fiyatı
tokens-dialog-badge-pool-price-hint = Gerçek zamanlı zincir üstü havuzdan alınan fiyat
tokens-dialog-badge-api-price = API fiyatı
tokens-dialog-badge-api-price-hint = Önbellekteki piyasa verisinden (API) alınan fiyat
tokens-dialog-badge-profile = Yayımlanmış profil
    .title = Yayımlanmak üzere incelenen ücretli profil içeriği; denetim veya sahiplik doğrulaması değildir.
tokens-dialog-badge-low-risk-hint = Güncel { -rugcheck } puanına göre düşük risk; kimlik doğrulaması değildir.
tokens-dialog-badge-immutable = Değiştirilemez
tokens-dialog-badge-mutable = Değiştirilebilir
tokens-dialog-badge-position = Pozisyon
tokens-dialog-badge-blacklisted = Kara listede

# Tokens page: sub-tabs (scripts/pages/tokens/constants.js)
# Ids are the view values of /api/tokens/list.

tokens-view-favorites = Favoriler
tokens-view-pool = Havuz Hizmeti
tokens-view-no-market = Piyasa Verisi Yok
tokens-view-all = Tüm Tokenlar
tokens-view-passed = Geçti
tokens-view-rejected = Reddedildi
tokens-view-blacklisted = Kara listede
tokens-view-positions = Pozisyonlar
tokens-view-recent = Son
tokens-view-ohlcv = OHLCV Verisi
# Empty token table per view (TOKEN_VIEW_EMPTY_LABELS)
tokens-view-pool-empty = Henüz fiyatlanmış token yok
    .message = Tokenler filtrelemeyi geçip havuz fiyatları hesaplandığında burada görünür.
tokens-view-no-market-empty = Piyasa verisi olmayan token yok
    .message = Tokenler, piyasa verisi kaynakları onları henüz listelemediği sürece burada görünür.
tokens-view-all-empty = Henüz keşfedilen token yok
    .message = Keşfin bulduğu her token, filtreleme sonucu ne olursa olsun burada görünür.
tokens-view-passed-empty = Filtrelemeyi geçen token yok
    .message = Tüm etkin filtreleri geçen tokenler burada görünür. Bu liste boş kalırsa Filtreleme sayfasını gözden geçirin.
tokens-view-rejected-empty = Reddedilen token yok
    .message = Bir filtreden geçemeyen tokenler nedeniyle birlikte burada görünür.
tokens-view-blacklisted-empty = Kara listede token yok
    .message = Sizin veya güvenlik kontrollerinin işlemden hariç tuttuğu tokenler burada görünür.
tokens-view-positions-empty = Pozisyonda token yok
    .message = Açık pozisyonlarda tutulan tokenler burada görünür.
tokens-view-recent-empty = Yeni token yok
    .message = Yeni keşfedilen tokenler bulundukça burada görünür.
tokens-ohlcv-empty = Henüz grafik verisi yok
    .message = Tokenler mumları toplanmaya başladığında burada görünür.

# Tokens page: token cell (scripts/pages/tokens/formatters.js)

tokens-cell-logo-enlarge = Büyütmek için tıklayın
tokens-boost-title = screenerbot.io üzerinde { $boosts } boost
tokens-cell-action-add =
    .title = Pozisyona ekle (DCA)
    .aria-label = Pozisyona ekle
tokens-cell-action-sell =
    .title = Sat (tamamı veya % kısmi)
    .aria-label = Tokenı sat
tokens-cell-action-buy =
    .title = Pozisyon al
    .aria-label = Tokenı al
tokens-cell-external-links =
    .title = Harici bağlantılar
    .aria-label = Harici bağlantılar

# Tokens page: table states shared by the token lists (scripts/pages/tokens/*.js)

tokens-table-loading-title = Tokenlar yükleniyor…
tokens-table-loading-description = Seçilen token görünümü hazırlanıyor.
tokens-table-retry-hint = Sekme değiştirin veya tekrar deneyin.
tokens-filter-all = Tümü

# Tokens page: favorites (scripts/pages/tokens/favorites.js)

tokens-favorites-load-failed-title = Favoriler yüklenemedi
tokens-favorites-load-failed-toast = Favoriler yüklenemedi
tokens-favorites-total = Toplam Favori
tokens-favorites-empty-title = Henüz Favori Yok
    .message = Burada tutmak için herhangi bir listede bir tokeni yıldızlayın.

# Tokens page: OHLCV data view (scripts/pages/tokens/ohlcv.js)
# Status ids come from /api/ohlcv/tokens; priority ids are Priority::as_str in src/ohlcvs/types.rs.

tokens-column-token = Token
tokens-column-status = Durum
tokens-ohlcv-delete =
    .title = OHLCV verilerini sil
    .aria-label = OHLCV verilerini sil
tokens-ohlcv-status-active = Etkin
tokens-ohlcv-status-inactive = Etkin değil
tokens-ohlcv-priority-critical = Kritik
tokens-ohlcv-priority-high = Yüksek
tokens-ohlcv-priority-medium = Orta
tokens-ohlcv-priority-low = Düşük
tokens-ohlcv-column-priority = Öncelik
tokens-ohlcv-column-backfill = Geçmiş Veri Doldurma
tokens-ohlcv-column-data-span = Veri Aralığı
tokens-ohlcv-column-gaps = Boşluklar
tokens-ohlcv-column-pools = Havuzlar
tokens-ohlcv-column-last-fetch = Son Getirme
tokens-ohlcv-timeframe-complete = { $timeframe }: Tamamlandı
tokens-ohlcv-timeframe-pending = { $timeframe }: Bekliyor
tokens-ohlcv-load-failed-title = OHLCV verileri yüklenemedi
tokens-ohlcv-load-failed-toast = OHLCV verileri yüklenemedi
tokens-ohlcv-total = Toplam Token
tokens-ohlcv-active = Etkin
tokens-ohlcv-db-size = Veritabanı Boyutu
tokens-ohlcv-cleanup = Etkin Olmayanları Temizle
tokens-ohlcv-delete-title = OHLCV Verilerini Sil
tokens-ohlcv-delete-token-message = { $token } için tüm OHLCV verileri silinsin mi?
tokens-ohlcv-delete-done =
    Silindi: { $candles ->
        [one] { $candles } mum
       *[other] { $candles } mum
    }, { $pools ->
        [one] { $pools } havuz
       *[other] { $pools } havuz
    }
tokens-ohlcv-delete-failed = OHLCV verileri silinemedi
tokens-ohlcv-cleanup-title = Etkin Olmayan Tokenları Sil
tokens-ohlcv-cleanup-message = Belirtilen saatten daha eski etkin olmayan tokenları sil
tokens-ohlcv-cleanup-placeholder = Saat...
tokens-ohlcv-cleanup-invalid = Lütfen pozitif bir sayı girin
tokens-ohlcv-cleanup-done =
    Temizlenen etkin olmayan token: { $count ->
        [one] { $count }
       *[other] { $count }
    }
tokens-ohlcv-cleanup-failed = OHLCV verileri temizlenemedi

# Tokens page: token lists (scripts/pages/tokens.js)
# The list statuses shown in the Status column come from row flags, not ids.

tokens-summary-total = Toplam
tokens-summary-pool-priced = Havuz Fiyatı Olan
tokens-summary-positions = Pozisyonlar
tokens-summary-blacklisted = Kara listede
tokens-search-placeholder = Sembol veya mint ile ara...
tokens-table-waiting-title = Tokenlar hâlâ yükleniyor...
tokens-table-waiting-description = Arka ucun yanıt vermesi bekleniyor. Otomatik olarak yeniden denenecek.
tokens-load-failed-toast = Tokenlar yüklenemedi
tokens-row-data-missing = Token verisi bulunamadı
tokens-column-price-sol = Fiyat ({ -sol })
tokens-column-liquidity = Likidite
tokens-column-volume-24h = 24sa Hacim
tokens-column-fdv = FDV
tokens-column-market-cap = Piy. Değeri
tokens-column-change-1h = 1sa
tokens-column-change-24h = 24sa
tokens-column-txns-5m = İşlem 5dk
tokens-column-txns-1h = İşlem 1sa
tokens-column-txns-6h = İşlem 6sa
tokens-column-txns-24h = İşlem 24sa
tokens-column-risk-score = Risk Puanı
tokens-column-reject-reason = Reddedilme Nedeni
tokens-column-blacklist-reason = Kara Liste Nedeni
tokens-column-updated = Güncellendi
tokens-column-birth = Doğuş
tokens-column-first-seen = İlk Görülme
tokens-badge-price = Fiyat
tokens-badge-ohlcv = OHLCV
tokens-badge-position = Pozisyon
tokens-badge-blacklisted = Kara listede
tokens-badge-blacklisted-title = Kara listedeki token
tokens-badge-blacklisted-reasons = Kara listede: { $reasons }
tokens-links-menu-copy-mint = Mint'i Kopyala
tokens-links-copy-failed = Mint kopyalanamadı
tokens-lightbox-token-age = Token Yaşı

# Global search dialog (scripts/ui/search_dialog.js)

tokens-search-placeholder-dialog = Ad, sembol veya mint ara...
tokens-search-input-label = Token ara
tokens-search-results-label = Arama sonuçları
tokens-search-tip-nav = gezin
tokens-search-tip-open = aç
tokens-search-tip-close = kapat
tokens-search-failed = Arama başarısız oldu
tokens-search-error = Hata: { $message }
tokens-search-clear =
    .title = Aramayı temizle
    .aria-label = Aramayı temizle
tokens-search-recent = Son
tokens-search-recent-label = Son aramalar
tokens-search-lists-label = Token listeleri
tokens-search-tab-trending = Trend
tokens-search-kinds = Ad · sembol · mint
tokens-search-empty-trending = Bot ilk havuzlarını fiyatladığında trend tokenlar burada görünür.
tokens-search-empty-positions = Şu anda açık pozisyon yok.
tokens-search-empty-favorites = Bir tokeni yıldızlayın, sonraki aramada burada sizi bekler.
tokens-search-empty-boosted = Şu anda boost'lu token yok.
tokens-search-list-failed = Bu liste yüklenemedi.
tokens-search-searching = Piyasalarda aranıyor…
# $count is the number of tokens found.
tokens-search-result-count =
    { $count ->
        [one] { $count } sonuç
       *[other] { $count } sonuç
    }
tokens-search-order = En iyi eşleşme önce, sonra 24sa hacim
tokens-search-metric-mc = PD
    .title = Piyasa değeri
tokens-search-metric-fdv = FDV
    .title = Tamamen seyreltilmiş değerleme
tokens-search-metric-liq = Lik
    .title = Likidite
tokens-search-metric-vol = Hcm
    .title = 24sa hacim
tokens-search-more =
    .title = Diğer işlemler
    .aria-label = Diğer işlemler
# $query is the text the user typed.
tokens-search-no-match = “{ $query }” ile eşleşen token yok.

# Featured dialog (scripts/ui/featured_dialog.js)
# Category and source ids are those of CATEGORIES; provider names are terms.

tokens-featured-category-boosted = Boost'lu
tokens-featured-category-jupiter-organic = { -jupiter } En Organik
tokens-featured-category-jupiter-traded = { -jupiter } En Çok İşlem Gören
tokens-featured-category-dexscreener-trending = { -dexscreener } Trend
tokens-featured-source-jupiter = { -jupiter }
tokens-featured-source-dexscreener = { -dexscreener }
tokens-featured-note-boosted = Ekipleri tarafından tanıtılıyor
tokens-featured-security-risky = Riskli
tokens-featured-load-failed = Öne çıkanlar yüklenemedi
tokens-featured-network-error = Ağ hatası: { $message }
tokens-featured-title = Öne Çıkan
tokens-featured-subtitle = Önce boost'lu tokenlar, ardından Solana genelinde trend olanlar
tokens-featured-boost = Bir Tokenı Boost'la
tokens-featured-close =
    .title = Kapat (ESC)
tokens-featured-loading = Öne çıkanlar ve trendler yükleniyor...
tokens-featured-error-hint = Bağlantıyı kontrol edin veya tekrar deneyin
tokens-featured-empty = Şu anda kullanılabilir token yok
tokens-featured-count =
    { $count ->
        [one] { $count } token
       *[other] { $count } token
    }
tokens-featured-stat-market-cap = Piyasa Değeri
tokens-featured-stat-liquidity = Likidite
tokens-featured-stat-volume = Hacim 24H
tokens-featured-stat-holders = Holderlar
tokens-featured-stat-txns = İşlem 24H
tokens-featured-buy = Al
    .title = Al: { $symbol }
tokens-featured-security-score = Güvenlik puanı: { $score }/100
tokens-featured-social-website = Web sitesi
tokens-featured-social-twitter = { -twitter }

# Featured row (scripts/ui/featured_row.js)

tokens-featured-row-view-all = Tümü
    .title = Tüm Öne Çıkan görünümünü aç
tokens-featured-row-scroll-start =
    .aria-label = Önceki tokenları göster
tokens-featured-row-scroll-end =
    .aria-label = Daha fazla token göster
tokens-featured-row-empty = Öne çıkan token yok
tokens-featured-row-title = { $name } ({ $symbol })
tokens-featured-row-boosted-title = { $name } ({ $symbol }) — boost: { $boosts }

# Pool selector dialog (scripts/ui/pool_selector.js)

tokens-pool-selector-title = Havuz Seç
tokens-pool-selector-loading = Havuzlar yükleniyor...
tokens-pool-selector-empty = Bu token için havuz bulunamadı
tokens-pool-selector-load-failed = Havuzlar yüklenemedi: { $message }
tokens-pool-selector-count =
    { $count ->
        [one] { $count } havuz bulundu
       *[other] { $count } havuz bulundu
    }
tokens-pool-selector-liquidity = { $amount } lik.
    .title = Likidite
tokens-pool-selector-volume = { $amount } 24sa
    .title = 24sa Hacim

# Token identity chips and address rows (scripts/ui/token_identity.js)

tokens-identity-unknown-asset = Bilinmeyen varlık
tokens-identity-copy-address =
    .title = Adresi kopyala
    .aria-label = Adresi kopyala
tokens-identity-copy-signature =
    .title = İmzayı kopyala
    .aria-label = İmzayı kopyala

tokens-rugcheck-risk-single-holder-ownership = Tek holder hakimiyeti
    .description = Tek bir holder, token arzının büyük bir kısmına sahip.
tokens-rugcheck-risk-low-liquidity = Düşük likidite
    .description = Token havuzunda likidite az.
tokens-rugcheck-risk-few-lp-providers = Az sayıda LP sağlayıcı
    .description = Likiditeyi yalnızca birkaç kullanıcı sağlıyor.
tokens-rugcheck-risk-high-holder-concentration = Yüksek holder yoğunluğu
    .description = İlk 10 holder, token arzının %50'sinden fazlasına sahip.
tokens-rugcheck-risk-top-10-holders-high-ownership = İlk 10 holder payı yüksek
    .description = İlk 10 holder, token arzının %70'inden fazlasına sahip.
tokens-rugcheck-risk-high-ownership = Yüksek sahiplik
    .description = En büyük holderlar, token arzının %80'inden fazlasına sahip.
tokens-rugcheck-risk-creator-rug-history = Oluşturanın rug pull geçmişi
    .description = Oluşturan, daha önce tokenlarda rug pull yapmış.
tokens-rugcheck-risk-large-lp-unlocked = LP'nin büyük kısmı kilitsiz
    .description = LP tokenlarının büyük kısmı kilitsiz; sahibi likiditeyi istediği zaman çekebilir.
tokens-rugcheck-risk-mutable-metadata = Değiştirilebilir meta veri
    .description = Sahibi, token meta verilerini değiştirebilir.
tokens-rugcheck-risk-few-holders = Az sayıda holder
    .description = Tokenı az sayıda cüzdan tutuyor.
tokens-rugcheck-risk-copycat-token = Taklit token
    .description = Bu token, doğrulanmış bir tokenın sembolünü kullanıyor.
tokens-rugcheck-risk-fee-config-enabled = Ücret yapılandırması açık
    .description = Sahibi, ücretleri istediği zaman değiştirebilir.
tokens-rugcheck-risk-high-holder-correlation = Yüksek holder korelasyonu
    .description = En büyük holderlar, arzdan benzer miktarlar tutuyor.
tokens-rugcheck-risk-freeze-authority-enabled = Dondurma yetkisi açık
    .description = Tokenlar dondurulabilir ve alım satımı engellenebilir.
tokens-rugcheck-risk-mint-authority-enabled = Mint yetkisi açık
    .description = Sahibi daha fazla token basabilir.
tokens-rugcheck-risk-missing-file-metadata = Meta veri dosyası eksik
    .description = Bu tokenla ilişkili bir meta veri dosyası yok.
tokens-rugcheck-risk-high-market-cap-per-holder = Holder başına yüksek piyasa değeri
    .description = Piyasa değeri, holder sayısına göre çok yüksek.
tokens-rugcheck-risk-symbol-mismatch = Sembol uyuşmazlığı
    .description = Token sembolü, meta veri dosyasıyla eşleşmiyor.
tokens-rugcheck-risk-name-mismatch = Ad uyuşmazlığı
    .description = Token adı, meta veri dosyasıyla eşleşmiyor.
tokens-rugcheck-risk-permanent-control-enabled = Kalıcı kontrol açık
    .description = Tokenı oluşturan, tüm tokenları kalıcı olarak kontrol edebilir.
tokens-rugcheck-risk-missing-metadata = Meta veri eksik
    .description = Bu token için meta veri bulunamadı.
tokens-rugcheck-risk-lp-unlock-soon = LP kilidi yakında açılıyor
    .description = LP tokenlarının kilidi yakında açılacak; sahibi likiditeyi çekebilecek.
tokens-rugcheck-risk-lp-vault-unlocked = LP kasası kilitsiz
    .description = Kasadaki LP tokenları geri alınabilir.
tokens-rugcheck-risk-mint-authority-locked = Mint yetkisi kilitli
    .description = Yeni token basımı kilitli.
tokens-rugcheck-risk-high-transfer-fee = Yüksek transfer ücreti
    .description = Bu tokenın her transferinden yüksek vergi alınır.
