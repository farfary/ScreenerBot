services-health-component-unavailable = { $component } bileşeni kullanılamıyor
services-health-unavailable = Sağlık durumu kullanılamıyor
services-health-pools-not-running = Havuz hizmeti çalışmıyor
services-health-events-db-uninitialized = Olaylar veritabanı başlatılmadı
services-health-sol-price-not-running = { -sol } fiyat hizmeti çalışmıyor
services-health-sol-price-stale = { -sol } fiyat verisi eski ({ $seconds }sn önceki)
services-health-sol-price-no-data = Henüz { -sol } fiyat verisi yok
services-health-telegram-discovery = Keşif modu
services-health-telegram-disconnected = Bağlantı kesildi
services-health-wallet-watch-polling-only = Tespit yalnızca yoklamayla çalışıyor
services-health-assistant-tasks-disabled = Yapılandırmada devre dışı
services-health-connectivity-critical-unhealthy = Kritik uç noktalar sağlıksız: { $endpoints }
services-health-filtering-snapshot-stale = Filtreleme anlık görüntüsü { $seconds }sn eski

## Services page (pages/services.js)

services-status-healthy = Sağlıklı
services-status-starting = Başlıyor
services-status-degraded = Düşük performans
services-status-unhealthy = Sağlıksız
services-status-stopping = Durduruluyor
services-status-disabled = Devre dışı
services-status-unknown = Bilinmiyor

services-name-account = Hesap
services-name-assistant-scheduled-tasks = Asistan zamanlanmış görevleri
services-name-ata-cleanup = Token hesabı temizliği
services-name-connectivity = Bağlantı
services-name-copy-trading = Kopya işlem
services-name-events = Olaylar
services-name-filtering = Filtreleme
services-name-llm-analysis = LLM analizi
services-name-ohlcv = OHLCV
services-name-pool-pricing = Havuz fiyatlandırması
services-name-pools = Havuzlar
services-name-positions = Pozisyonlar
services-name-referral = Referans
services-name-rpc-stats = RPC istatistikleri
services-name-sol-price = { -sol } fiyatı
services-name-telegram = { -telegram }
services-name-tokens = Tokenlar
services-name-trader = Trader
services-name-transactions = İşlemler
services-name-update-check = Güncelleme denetimi
services-name-wallet = Cüzdan
services-name-wallet-watch = Cüzdan izleme
services-name-webserver = Web sunucusu

services-loading = Hizmetler yükleniyor...
services-load-failed = Hizmetler yüklenemedi
services-load-failed-description = Arka ucun yanıt vermesi bekleniyor. Otomatik olarak yeniden denenecek.
services-refresh-failed = Hizmetler yenilenemedi
services-search-placeholder = Hizmet ara...
services-summary-total = Toplam
services-summary-alerts = Uyarılar
services-summary-alerts-tooltip = { $degraded } düşük performans / { $unhealthy } sağlıksız
services-filter-status = Durum
services-filter-all-statuses = Tüm Durumlar
services-filter-all-services = Tüm Hizmetler
services-filter-enabled-only = Yalnızca Etkin
services-filter-disabled-only = Yalnızca Devre Dışı
services-col-service = Hizmet
services-col-health = Sağlık
services-col-priority = Öncelik
services-col-uptime = Çalışma Süresi
services-col-activity = Etkinlik
services-col-last-cycle = Son Döngü
services-col-avg-cycle = Ort. Döngü
services-col-avg-poll = Ort. Yoklama
services-col-cycle-rate = Döngü Hızı
services-col-tasks = Görevler
services-col-ops = İşlem/sn
services-col-errors = Hatalar
services-col-dependencies = Bağımlılıklar
services-dependencies-none = Yok
services-activity-busy = { $percent } meşgul
services-activity-polls =
    { $count ->
        [one] { $count } yoklama
       *[other] { $count } yoklama
    }
services-tasks-tooltip =
    { $count ->
        [one] { $count } görev
       *[other] { $count } görev
    }
    Son: { $last }
    Ort.: { $avg }
    Yoklama: { $poll }
    Boşta: { $idle }
    Toplam Yoklama: { $polls }
services-tasks-none = Ölçümlenen görev yok
