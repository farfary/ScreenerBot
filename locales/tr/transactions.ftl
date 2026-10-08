transactions-type-buy = Al
transactions-type-sell = Sat
transactions-type-swap = Takas
transactions-type-sol-transfer = SOL transferi
transactions-type-token-transfer = Token transferi
transactions-type-transfer = Transfer
transactions-type-dust = Toz
transactions-type-spam = Spam
transactions-type-ata-create = Hesap açıldı
transactions-type-ata-close = Kira geri alındı
transactions-type-ata = Token hesabı
transactions-type-liquidity-add = Likidite ekleme
transactions-type-liquidity-remove = Likidite çıkarma
transactions-type-nft = NFT
transactions-type-program = Program çağrısı
transactions-type-compute = Hesaplama
transactions-type-failed = Başarısız
transactions-type-unknown = Sınıflandırılmamış

transactions-type-with-detail = { $label } ({ $detail })
transactions-type-token-transfer-detail = { $label } { $mint } ({ $amount })
transactions-type-spam-detail = Spam airdrop ({ $mint })
transactions-type-described = { $description }

transactions-filter-all = Tüm türler
transactions-filter-transfer = Transferler
transactions-filter-ata = Kira ve hesaplar
transactions-filter-liquidity = Likidite
transactions-filter-program = Program çağrıları

transactions-direction-incoming = Gelen
transactions-direction-outgoing = Giden
transactions-direction-internal = Dahili
transactions-direction-unknown = Sınıflandırılmamış

transactions-status-pending = Bekliyor
transactions-status-confirmed = Onaylandı
transactions-status-finalized = Kesinleşti
transactions-status-failed = Başarısız
transactions-status-success = Başarılı
transactions-status-unknown = Bilinmiyor

transactions-ata-operation-creation = Oluşturma
transactions-ata-operation-closure = Kapatma

transactions-toolbar-title = İşlem geçmişi
transactions-search =
    .placeholder = İmzalarda ara…
    .aria-label = İşlem imzalarında ara
transactions-load-failed = İşlemler yenilenemedi
transactions-summary-total = Toplam
transactions-summary-estimate = Tahmin
transactions-summary-success = Başarılı
transactions-summary-failed = Başarısız
transactions-filter-wallet = Cüzdan
transactions-filter-type = Tür
transactions-filter-direction = Yön
transactions-filter-status = Durum
transactions-filter-all-directions = Tüm yönler
transactions-filter-all-statuses = Tüm durumlar
transactions-wallet-main = Ana cüzdan
transactions-col-time = Zaman
transactions-col-signature = İmza
transactions-col-type = Tür
transactions-col-direction = Yön
transactions-col-status = Durum
transactions-col-native-delta = Δ { -sol }
transactions-col-fees = Ücretler ({ -sol })
transactions-col-token = Token
transactions-col-router = Yönlendirici
transactions-col-instructions = Komut

transactions-dialog-copy-signature =
    .title = İmzayı kopyala
transactions-dialog-close =
    .title = Kapat (ESC)
transactions-dialog-tabs-label = İşlem ayrıntı bölümleri
transactions-dialog-meta-slot = Slot:
transactions-dialog-meta-fee = Ücret:
transactions-dialog-loading = Yükleniyor...
transactions-dialog-loading-details = İşlem ayrıntıları yükleniyor...
transactions-dialog-load-failed = İşlem ayrıntıları yüklenemedi
transactions-dialog-load-failed-reason = İşlem ayrıntıları yüklenemedi: { $reason }
transactions-dialog-not-found = İşlem bulunamadı
transactions-dialog-tab-overview = Genel bakış
transactions-dialog-tab-balances = Bakiyeler
transactions-dialog-tab-instructions = Komutlar
transactions-dialog-tab-logs = Günlükler
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = Ham
transactions-dialog-unknown = Bilinmiyor
transactions-dialog-unknown-asset = Bilinmeyen varlık
transactions-dialog-unavailable = Mevcut değil

transactions-dialog-failed-title = İşlem başarısız oldu
transactions-dialog-no-program-error = Program hatası sağlanmadı.
transactions-dialog-story-title = Ne oldu
transactions-dialog-router-via = { $router } üzerinden
transactions-dialog-flow-paid = Ödenen
transactions-dialog-flow-received = Alınan
transactions-dialog-flow-from = Gönderen
transactions-dialog-flow-to = Alıcı
transactions-dialog-flow-amount = Miktar
transactions-dialog-net-wallet-change = Net cüzdan değişimi:
transactions-dialog-processed = Solana'da işlendi
transactions-dialog-execution-title = Yürütme
transactions-dialog-metric-execution-price = Yürütme fiyatı
transactions-dialog-metric-effective-received = Fiilen alınan
transactions-dialog-metric-effective-spent = Fiilen harcanan
transactions-dialog-metric-network-fee = Ağ ücreti
transactions-dialog-metric-estimated-pnl = Tahmini K/Z
transactions-dialog-metric-net-native-change = Net { -sol } değişimi
transactions-dialog-route-title = Rota ve varlıklar
transactions-dialog-route-router = Yönlendirici
transactions-dialog-route-input-asset = Giriş varlığı
transactions-dialog-route-output-asset = Çıkış varlığı
transactions-dialog-route-pool = Havuz
transactions-dialog-route-program = Program
transactions-dialog-tech-title = Teknik ayrıntılar
transactions-dialog-tech-summary = İmza, slot ve kaynaklar
transactions-dialog-tech-signature = İmza
transactions-dialog-tech-timestamp = Zaman damgası
transactions-dialog-tech-slot = Slot
transactions-dialog-tech-exact-fee = Kesin ücret
transactions-dialog-tech-accounts = Hesaplar
transactions-dialog-tech-instructions = Komutlar
transactions-dialog-tech-compute-units = Hesaplama birimi
transactions-dialog-tech-token-decimals = Token ondalığı

transactions-dialog-balances-native-title = { -sol } bakiye değişimleri
transactions-dialog-balances-native-empty = { -sol } bakiye değişimi yok
transactions-dialog-balances-token-title = Token bakiye değişimleri
transactions-dialog-balances-token-empty = Token bakiye değişimi yok
transactions-dialog-balances-net-native = Net { -sol } değişimi
transactions-dialog-balances-fee = İşlem ücreti
transactions-dialog-col-account = Hesap
transactions-dialog-col-token = Token
transactions-dialog-col-pre-balance = Önceki bakiye
transactions-dialog-col-post-balance = Sonraki bakiye
transactions-dialog-col-change = Değişim
transactions-dialog-col-type = Tür
transactions-dialog-col-rent = Kira ({ -sol })
transactions-dialog-instructions-empty = Komut bulunamadı
transactions-dialog-instructions-count =
    { $count ->
        [one] { $count } komut
       *[other] { $count } komut
    }
transactions-dialog-instruction-program-id = Program kimliği
transactions-dialog-instruction-accounts = Hesaplar ({ $count })
transactions-dialog-instruction-data = Veri
transactions-dialog-logs-empty = Günlük yok
transactions-dialog-logs-filter = Günlükleri filtrele...
transactions-dialog-logs-no-match = Eşleşen günlük yok
transactions-dialog-logs-count =
    { $count ->
        [one] { $count } günlük
       *[other] { $count } günlük
    }
transactions-dialog-ata-empty = Bu işlemde ATA operasyonu yok
transactions-dialog-ata-summary-title = ATA analiz özeti
transactions-dialog-ata-creations = Oluşturmalar
transactions-dialog-ata-closures = Kapatmalar
transactions-dialog-ata-rent-spent = Harcanan kira
transactions-dialog-ata-rent-recovered = Geri alınan kira
transactions-dialog-ata-net-rent = Net kira etkisi
transactions-dialog-ata-operations-title = ATA operasyonları ({ $count })
transactions-dialog-raw-copy = JSON'u kopyala
transactions-dialog-raw-empty = Ham veri yok

# Empty table (scripts/pages/transactions.js)
transactions-empty = Henüz işlem yok
    .message = İşlem cüzdanının swap ve transferleri zincirde onaylandığında burada görünür.
