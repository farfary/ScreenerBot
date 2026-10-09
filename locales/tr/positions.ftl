positions-state-reason-position-created = Pozisyon oluşturuldu

positions-status-open = Açık
positions-status-closed = Kapalı
positions-status-archived = Arşivlenmiş
# Empty positions table per status (POSITION_EMPTY_LABELS)
positions-open-empty = Açık pozisyon yok
    .message = Otomatik işlemci veya manuel bir alım pozisyon açtığında burada görünür.
positions-closed-empty = Kapalı pozisyon yok
    .message = Bir pozisyon tamamen satıldığında buraya taşınır.
positions-archived-empty = Arşivlenmiş pozisyon yok
    .message = Arşivle ile kaldırdığınız pozisyonlar burada tutulur.
positions-origin-copy = Kopya
positions-origin-manual = Manuel
positions-origin-wallet = Cüzdan
positions-origin-copy-link =
    .title = Bu pozisyonu açan kopyalama görevini aç
positions-holding-frozen = Dondurulmuş
    .title = Mint yetkisi bu token hesabını dondurdu - bakiye aktarılamaz veya satılamaz
positions-toolbar-total = Toplam
positions-toolbar-delete-all = Tümünü sil
positions-search-placeholder = Sembol veya mint ile ara...
positions-filter-origin = Kaynak
positions-filter-origin-all = Tüm kaynaklar
positions-filter-origin-auto = Auto Trader
positions-filter-origin-copy = Kopya işlem
positions-delete-all-tooltip = Arşivlenmiş tüm pozisyonları kalıcı olarak sil

positions-column-token = Token
positions-column-archived-at = Arşivlenme
positions-column-entry-time = Giriş zamanı
positions-column-exit-time = Çıkış zamanı
positions-column-avg-entry = Ort. giriş ({ -sol })
positions-column-avg-exit = Ort. çıkış ({ -sol })
positions-column-current-price = Güncel ({ -sol })
positions-column-total-invested = Toplam yatırım ({ -sol })
positions-column-proceeds = Gelir ({ -sol })
positions-column-pnl = K/Z ({ -sol })
positions-column-pnl-percent = K/Z %
positions-column-size = Boyut
positions-column-dca = DCA
positions-column-exits = Çıkışlar
positions-column-unrealized-pnl = Gerçekleşmemiş K/Z ({ -sol })
positions-column-unrealized-percent = Gerçekleşmemiş %

positions-unknown-basis = Bu cüzdanın geçmişinde maliyet bazı yok (airdrop, USD cinsinden dolum veya SOL bacağı olmayan takas)
positions-unknown-history = Bu tur zincir üstü bakiye ile uyuşmuyor
positions-dca-count =
    { $count ->
        [one] { $count } DCA
       *[other] { $count } DCA
    }
positions-exit-count =
    { $count ->
        [one] { $count } çıkış
       *[other] { $count } çıkış
    }

positions-action-add =
    .title = Pozisyona ekle (DCA)
    .aria-label = Pozisyona ekle
positions-action-sell =
    .title = Sat (tamamı veya % kısmi)
    .aria-label = Pozisyonu sat
positions-action-sell-frozen = Mint yetkisi tarafından dondurulmuş - bu varlık satılamaz
positions-action-remove =
    .title = Kaldır (arşivle veya sil)
    .aria-label = Pozisyonu kaldır
positions-action-restore =
    .title = Açık/Kapalı durumuna geri yükle
    .aria-label = Pozisyonu geri yükle
positions-action-delete =
    .title = Kalıcı olarak sil
    .aria-label = Kalıcı olarak sil
positions-action-in-progress = Devam ediyor…

positions-caption-buying = Alınıyor
positions-caption-buying-step = Alınıyor · { $step }
positions-caption-selling = Satılıyor
positions-caption-selling-step = Satılıyor · { $step }
positions-caption-closing = Kapatılıyor
positions-caption-failed = Başarısız
positions-caption-failed-detail = Başarısız · { $error }
positions-step-adding = Ekleniyor
positions-pending-buying = Alınıyor…
positions-pending-buy-failed = Alım başarısız

positions-load-failed = Pozisyonlar yenilenemedi
positions-toast-not-found = Pozisyon verisi bulunamadı
positions-toast-deleted = Pozisyon silindi
positions-toast-archived = Pozisyon arşivlendi
positions-toast-restored = Pozisyon geri yüklendi
positions-buy-adds-to-archived = Bu tokenin arşivde zaten açık bir pozisyonu var. Alım bu pozisyona eklenir ve pozisyon açık pozisyonlara geri döner. Pozisyonun mevcut yönetim modu korunur.
positions-action-failed = İşlem başarısız
positions-delete-title = Pozisyonu kalıcı olarak sil
positions-delete-message = { $symbol } kalıcı olarak silinsin mi? Pozisyon ve geçmişi veritabanından kaldırılır, bu işlem geri alınamaz. İşlemleriniz ve token verileriniz etkilenmez.
positions-delete-confirm = Kalıcı olarak sil
positions-delete-all-title = Arşivlenmiş tüm pozisyonları sil
positions-delete-all-message =
    { $count ->
        [one] Arşivlenmiş tüm pozisyonlar ({ $count }) kalıcı olarak silinsin mi? Bu işlem geri alınamaz. İşlemler ve token verileri etkilenmez.
       *[other] Arşivlenmiş tüm pozisyonlar ({ $count }) kalıcı olarak silinsin mi? Bu işlem geri alınamaz. İşlemler ve token verileri etkilenmez.
    }
positions-delete-all-message-empty = Arşivlenmiş tüm pozisyonlar kalıcı olarak silinsin mi? Bu işlem geri alınamaz.
positions-delete-all-confirm = Tümünü sil
positions-delete-all-done =
    { $count ->
        [one] Arşivlenmiş pozisyon silindi: { $count }
       *[other] Arşivlenmiş pozisyon silindi: { $count }
    }
positions-delete-all-failed = Arşivlenmiş pozisyonlar silinemedi

positions-remove-title = Pozisyonu kaldır
positions-remove-open-warning = <strong>Bu pozisyon hâlâ açık.</strong> Bot bu tokenı elinde tutuyor. Kaldırmak işlem yuvasını boşaltır ve takibi durdurur, ancak satış yapmaz: <strong>satmaz</strong>. { -sol } bakiyenizi geri istiyorsanız önce satın.
positions-remove-modes =
    .aria-label = Kaldırma modu
positions-remove-archive = Arşivle
positions-remove-recommended = Önerilen
positions-remove-archive-description = Arşivlenmiş sekmesine gizler. İstediğiniz zaman geri alınabilir - hiçbir şey satılmaz ve tüm işlemler kayıtta kalır.
positions-remove-delete = Kalıcı olarak sil
positions-remove-delete-description = Bu pozisyonu ve tüm geçmişini veritabanından siler.
positions-remove-danger = Pozisyon ve geçmişi kalıcı olarak kaldırılır. <strong>Bu işlem geri alınamaz.</strong> İşlemleriniz ve token verileriniz etkilenmez.
positions-remove-confirm-archive = Pozisyonu arşivle

positions-management-changed = Pozisyon yönetimi ayarlandı: { $mode }
positions-details-load-failed = Pozisyon ayrıntıları yüklenemedi
positions-details-mint-label = Mint adresi
positions-details-management-failed = Pozisyon yönetimi güncellenemedi
positions-details-favorite-add =
    .title = Favorilere ekle
    .aria-label = Favorilere ekle
positions-details-favorite-remove =
    .title = Favorilerden kaldır
    .aria-label = Favorilerden kaldır
positions-details-view-solscan =
    .title = { -solscan } üzerinde görüntüle
    .aria-label = Tokenı { -solscan } üzerinde görüntüle
positions-details-close =
    .title = Kapat (Esc)
    .aria-label = Kapat
positions-details-chart-section =
    .aria-label = Fiyat grafiği
positions-details-loading-chart = Grafik yükleniyor...
positions-details-activity-section =
    .aria-label = Etkinlik
positions-details-activity-title = Etkinlik
positions-details-split-handle =
    .aria-label = Grafik ve etkinliği yeniden boyutlandır
positions-details-activity-pane =
    .aria-label = Etkinlik paneli
positions-details-activity-expand =
    .title = Etkinliği genişlet
    .aria-label = Etkinliği genişlet
positions-details-summary-section =
    .aria-label = Pozisyon özeti
positions-details-loading = Pozisyon yükleniyor...

positions-management-auto-trader = Auto Trader
positions-management-user-only = Yalnızca kullanıcı
positions-management-copy-task = Kopyalama görevi
positions-management-hybrid = Hibrit
positions-pane-show-chart = Grafiği göster
positions-pane-show-activity = Etkinliği göster
positions-pane-restore-activity = Etkinliği geri yükle
positions-pane-expand-chart =
    .title = Grafiği genişlet
    .aria-label = Grafiği genişlet

positions-risk-low = Düşük risk
positions-risk-medium = Orta risk
positions-risk-high = Yüksek risk
positions-risk-unknown = Risk bilinmiyor
positions-busy-buying = Alım sürüyor…
positions-busy-selling = Satım sürüyor…
positions-busy-closing = Kapatma sürüyor…
positions-header-avg-entry = Ort. giriş
positions-header-buy-count =
    { $count ->
        [one] { $count } alım
       *[other] { $count } alım
    }
positions-header-exit-price = Çıkış fiyatı
positions-header-closed-ago = { $ago } kapatıldı
positions-header-realized-pnl = Gerçekleşmiş K/Z
positions-header-usd-note = USD, bugünkü { -sol } fiyatıyla
positions-header-returned = İade edilen
positions-header-of-invested = Yatırılan: { $amount }
positions-header-price = Fiyat
positions-header-last-price = Son fiyat
positions-header-pool-ago = havuz · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = Gerçekleşmemiş K/Z
positions-header-pnl-last-price = Son fiyattaki K/Z
positions-header-value = Değer
positions-header-last-value = Son değer
positions-header-invested = { $amount } yatırıldı
positions-header-origin-hint = Bu pozisyonun nasıl açıldığı
positions-header-risk-hint = { -rugcheck } puanı - düşük olan daha güvenli
positions-header-frozen = Dondurulmuş
    .title = Mint yetkisi bu varlığı dondurdu
positions-header-managed-by = Yöneten
positions-header-management-select =
    .aria-label = Pozisyon yönetimi

positions-origin-unknown = bilinmiyor
positions-origin-copied-task = Kopyalandı · görev { $task }
positions-origin-manual-entry = Manuel giriş
positions-origin-wallet-entry = Cüzdan girişi
positions-origin-auto-strategy = Otomatik · { $strategy }
positions-origin-auto-entry = Otomatik giriş

positions-pending-adding = Ekleniyor
positions-pending-adding-amount = Ekleniyor: { $amount }
positions-pending-selling = Satılıyor
positions-pending-selling-percent = Satılıyor: { $percent }
positions-pending-confirming = { $label } · onaylanıyor
    .title = Gönderildi, zincir üstü onay bekleniyor. Doğrulandığında rakamlar güncellenir.

positions-trade-add = Ekle
    .title = Pozisyona ekle
positions-trade-sell = Sat
    .title = Pozisyonun bir kısmını sat
positions-trade-close = Pozisyonu kapat
    .title = Hepsini sat ve kapat
positions-trade-token = Token ayrıntıları
    .title = Token ayrıntılarını aç

positions-favorite-token-fallback = Token
positions-favorite-added = { $symbol } favorilere eklendi
positions-favorite-removed = { $symbol } favorilerden kaldırıldı
positions-favorite-add-failed = Favori eklenemedi
positions-favorite-remove-failed = Favori kaldırılamadı
positions-favorite-update-failed = Favoriler güncellenemedi

positions-summary-position = Pozisyon
positions-summary-price-path = Fiyat seyri
positions-summary-network-fees = Ağ ücretleri
positions-summary-risk = Risk
positions-summary-market = Piyasa
positions-summary-market-now = Piyasa şimdi
positions-summary-links = Bağlantılar
positions-fact-tokens-fallback = token
positions-fact-bought = Alınan
positions-fact-holding = Eldeki
positions-fact-sold = Satılan
positions-fact-realized = Gerçekleşen
positions-fact-opened = Açılış
positions-fact-closed = Kapanış
positions-fact-reason = Neden
positions-fact-archived = Arşivlenme
positions-fact-entry = Giriş
positions-fact-exit = Çıkış
positions-fact-total = Toplam
positions-fact-verified = Zincirde doğrulandı
positions-fact-confirming = Onaylanıyor
positions-fact-share-of-bought = Alımın payı: { $percent }
positions-fact-share-of-invested = Yatırımın payı: { $percent }
positions-fact-entry-count =
    { $count ->
        [0] 1 giriş
        [one] 1 giriş + { $count } ekleme
       *[other] 1 giriş + { $count } ekleme
    }
positions-fact-partial-exits-back =
    { $count ->
        [one] Kısmi çıkış: { $count } · { $returned } geri döndü
       *[other] Kısmi çıkış: { $count } · { $returned } geri döndü
    }
positions-fact-held = { $age } tutuldu
positions-fact-vs-entry = Girişe göre { $percent }
positions-fact-exit-vs-peak = Çıkış / tepe
positions-fact-now-vs-peak = Şimdi / tepe
positions-fact-entry-range = Giriş aralığı
positions-range-low = Dip
positions-range-peak = Tepe
positions-range-now = Şimdi
positions-range-label-exit = Dip ile tepe arasında giriş ve çıkış fiyatı
positions-range-label-now = Dip ile tepe arasında giriş ve güncel fiyat
positions-fact-mint-authority = Mint yetkisi
positions-fact-freeze-authority = Dondurma yetkisi
positions-fact-active = Aktif
positions-fact-pool = Havuz
positions-fact-pool-liquidity = { $amount } { -sol } likidite
positions-fact-market-cap = Piyasa değeri
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = Likidite
positions-fact-volume-24h = Hacim 24sa
positions-fact-price-change = Fiyat değişimi
positions-change-period-1h = 1sa
positions-change-period-24h = 24sa
positions-fact-holders = Holder
positions-link-website = Web sitesi
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

positions-activity-load-failed = Etkinlik yüklenemedi
positions-activity-loading = Etkinlik yükleniyor...
positions-activity-empty = Bu cüzdanda bu token ile henüz hiçbir şey olmadı
positions-activity-filter-empty = Bu filtreyle eşleşen etkinlik yok
positions-activity-round-count =
    { $count ->
        [one] { $count } tur
       *[other] { $count } tur
    }
positions-activity-event-count =
    { $count ->
        [one] { $count } olay
       *[other] { $count } olay
    }
positions-activity-pending-count = Bekleyen: { $count }
positions-activity-failed-count = Başarısız: { $count }
positions-filter-all = Tümü
positions-filter-trades = İşlemler
positions-filter-buys = Alımlar
positions-filter-sells = Satımlar
positions-filter-wallet = Cüzdan
positions-filter-issues = Sorunlar
positions-activity-filters =
    .aria-label = Etkinliği filtrele
positions-activity-totals =
    .aria-label = Bu tokendaki tüm turlar
positions-activity-realized-all = Gerçekleşmiş, tüm turlar
positions-activity-invested = Yatırılan
positions-activity-returned = İade edilen
positions-activity-opened = Açılış: { $when }
positions-activity-round-title = Pozisyon { $index }
positions-activity-this-position = Bu pozisyon
positions-activity-dates-unavailable = Tarihler mevcut değil
positions-activity-wallet-title = Cüzdan işlemleri
positions-activity-outside =
    { $count ->
        [one] Hiçbir pozisyona ait değil · { $range } · { $count } olay
       *[other] Hiçbir pozisyona ait değil · { $range } · { $count } olay
    }
positions-details-signature-label = İmza

positions-state-open = Pozisyon açık
positions-state-closing = Pozisyon kapanıyor
positions-state-closed = Pozisyon kapalı
positions-state-exit-pending = Pozisyon çıkışı bekliyor
positions-state-exit-failed = Pozisyon çıkışı başarısız
positions-state-phantom = Hayalet pozisyon
positions-state-reconciling = Pozisyon mutabakatta

positions-event-kind-entry = Giriş
positions-event-kind-dca = Ekleme
positions-event-kind-partial-exit = Kısmi çıkış
positions-event-kind-exit = Çıkış
positions-event-kind-buy = Cüzdan alımı
positions-event-kind-sell = Cüzdan satımı
positions-event-kind-transfer = Transfer
positions-event-kind-ata = Token hesabı
positions-event-kind-other = İşlem
positions-event-state-pending = Bekliyor
positions-event-state-failed = Başarısız
positions-event-state-synthetic = Sentetik
positions-chain-status-failed-detail = Başarısız: { $error }
positions-event-tokens-fallback = token
positions-event-entry-submitted = Alım gönderildi: { $amount }
positions-event-entry-for = Alındı: { $amount }, ödenen: { $sol }
positions-event-entry = Alındı: { $amount }
positions-event-dca-submitted = Ekleme gönderildi: { $amount }
positions-event-dca-for = Eklendi: { $amount }, ödenen: { $sol }
positions-event-dca = Eklendi: { $amount }
positions-event-partial-exit-submitted-percent = Kısmi çıkış gönderildi ({ $percent }): { $amount }
positions-event-partial-exit-submitted = Kısmi çıkış gönderildi: { $amount }
positions-event-sold-percent-for = Satıldı: { $amount } ({ $percent }), alınan: { $sol }
positions-event-sold-percent = Satıldı: { $amount } ({ $percent })
positions-event-sold-for = Satıldı: { $amount }, alınan: { $sol }
positions-event-sold = Satıldı: { $amount }
positions-event-exit-submitted = Tam pozisyon çıkışı gönderildi
positions-event-exit-for = Kapatıldı, satılan: { $amount }, alınan: { $sol }
positions-event-exit-closed = Pozisyon kapatıldı
positions-event-wallet-bought = Cüzdan başka yerde aldı: { $amount }
positions-event-wallet-sold = Cüzdan başka yerde sattı: { $amount }
positions-event-received = Alındı: { $amount }
positions-event-sent = Gönderildi: { $amount }
positions-event-transferred = Transfer edildi: { $amount }
positions-event-ata = Token hesabı etkinliği
positions-event-wallet-transaction = Cüzdan işlemi: { $amount }
positions-event-price-per-token = { $price } { -sol } / token
positions-event-wallet-change = Cüzdan değişimi: { $amount }
positions-event-after-title = Bu olaydan sonra pozisyon
positions-event-capital-invested = Yatırılan sermaye
positions-event-average-entry = Ortalama giriş
positions-event-transfers-title = Token transferleri
positions-event-transfer-amount = Miktar
positions-event-transfer-mint = Mint
positions-event-transfer-from = Gönderen
positions-event-transfer-to = Alıcı
positions-event-no-signature = Zincir üstü imza yok
positions-event-click-to-copy = Kopyalamak için tıklayın
positions-event-solscan = { -solscan }
positions-event-token-amount = Token miktarı
positions-event-trade-price = İşlem fiyatı
positions-event-native-amount = { -sol } miktarı
positions-event-cost-basis = Maliyet bazı
positions-event-usd-value = USD değeri
positions-event-network-fee = Ağ ücreti
positions-event-router = Yönlendirici
positions-event-slot = Slot
positions-event-chain-status = Zincir durumu
positions-event-transaction-type = İşlem türü
positions-event-direction = Yön
positions-event-wallet-native-change = Cüzdan { -sol } değişimi
positions-event-instructions = Komutlar
positions-event-compute-units = Hesaplama birimi
positions-event-accounts = Hesaplar
positions-event-record-id = Kayıt kimliği
positions-event-time-unavailable = Zaman mevcut değil
positions-event-details = Ayrıntılar
positions-event-hide-details = Ayrıntıları gizle

positions-chart-type-candles = Mumlar
positions-chart-type-line = Çizgi
positions-chart-type-area = Alan
positions-chart-type-group =
    .aria-label = Grafik türü
positions-chart-overlays-group =
    .aria-label = Grafik katmanları
positions-chart-ema = EMA
    .title = Üstel hareketli ortalamalar, 9 ve 21
positions-chart-fit = Sığdır
    .title = Bu pozisyonun ömrünü çerçevele
positions-chart-timeframes-group =
    .aria-label = Zaman dilimi
positions-chart-pane-group =
    .aria-label = Grafik paneli
positions-chart-unavailable = Grafik motoru kullanılamıyor
positions-chart-collecting = Grafik verisi toplanıyor…
positions-chart-no-data = Bu token için henüz grafik verisi yok
positions-chart-avg-entry = Ort. giriş
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = Ort. giriş
positions-chart-legend-avg-entry-off-scale = Ort. giriş (ölçek dışı)
positions-chart-dropped-events =
    { $count ->
        [one] Bu zaman diliminde mumu olmayan olay: { $count }
       *[other] Bu zaman diliminde mumu olmayan olay: { $count }
    }
positions-chart-level = Seviye
positions-chart-level-above = { $label } { $price } bu görünümün üstünde
positions-chart-level-below = { $label } { $price } bu görünümün altında
positions-chart-scale-hint = Fiyat eksenini sürükleyerek ölçeği genişletin
positions-chart-pnl-at-bar = K/Z @ Mum
positions-chart-click-to-locate = Bulmak için tıklayın
