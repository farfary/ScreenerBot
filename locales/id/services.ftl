# Service health messages. Ids are used by the Service::health implementations in
# src/services/implementations/*.rs and by src/webserver/routes/services/handlers.rs.

services-health-component-unavailable = Komponen { $component } tidak tersedia
services-health-unavailable = Status kesehatan tidak tersedia
services-health-pools-not-running = Pool service tidak berjalan
services-health-events-db-uninitialized = Database event belum diinisialisasi
services-health-sol-price-not-running = Layanan harga SOL tidak berjalan
services-health-sol-price-stale = Data harga SOL usang (berumur { $seconds } dtk)
services-health-sol-price-no-data = Belum ada data harga SOL
services-health-telegram-discovery = Mode penemuan
services-health-telegram-disconnected = Terputus
services-health-wallet-watch-polling-only = Deteksi berjalan hanya dengan polling
services-health-assistant-tasks-disabled = Dinonaktifkan di konfigurasi
services-health-connectivity-critical-unhealthy = Endpoint kritis tidak sehat: { $endpoints }
services-health-filtering-snapshot-stale = Snapshot pemfilteran berumur { $seconds } dtk

## Services page (pages/services.js)

services-status-healthy = Sehat
services-status-starting = Memulai
services-status-degraded = Menurun
services-status-unhealthy = Tidak Sehat
services-status-stopping = Menghentikan
services-status-disabled = Nonaktif
services-status-unknown = Tidak Diketahui

services-name-account = Akun
services-name-assistant-scheduled-tasks = Tugas terjadwal Asisten
services-name-ata-cleanup = Pembersihan akun token
services-name-connectivity = Konektivitas
services-name-copy-trading = Copy trading
services-name-events = Event
services-name-filtering = Pemfilteran
services-name-llm-analysis = Analisis LLM
services-name-ohlcv = OHLCV
services-name-pool-pricing = Penetapan harga pool
services-name-pools = Pool
services-name-positions = Posisi
services-name-referral = Referral
services-name-rpc-stats = Statistik RPC
services-name-sol-price = Harga { -sol }
services-name-telegram = { -telegram }
services-name-tokens = Token
services-name-trader = Trader
services-name-transactions = Transaksi
services-name-update-check = Pemeriksaan pembaruan
services-name-wallet = Dompet
services-name-wallet-watch = Pantau dompet
services-name-webserver = Web server

services-loading = Memuat layanan...
services-load-failed = Gagal memuat layanan
services-load-failed-description = Menunggu respons backend. Kami akan mencoba lagi secara otomatis.
services-refresh-failed = Tidak dapat menyegarkan layanan
services-search-placeholder = Cari layanan...
services-summary-total = Total
services-summary-alerts = Peringatan
services-summary-alerts-tooltip = { $degraded } menurun / { $unhealthy } tidak sehat
services-filter-status = Status
services-filter-all-statuses = Semua Status
services-filter-all-services = Semua Layanan
services-filter-enabled-only = Hanya Aktif
services-filter-disabled-only = Hanya Nonaktif
services-col-service = Layanan
services-col-health = Kesehatan
services-col-priority = Prioritas
services-col-uptime = Uptime
services-col-activity = Aktivitas
services-col-last-cycle = Siklus Terakhir
services-col-avg-cycle = Rata-rata Siklus
services-col-avg-poll = Rata-rata Polling
services-col-cycle-rate = Laju Siklus
services-col-tasks = Tugas
services-col-ops = Op/dtk
services-col-errors = Error
services-col-dependencies = Dependensi
services-dependencies-none = Tidak ada
services-activity-busy = { $percent } sibuk
services-activity-polls =
    { $count ->
       *[other] { $count } polling
    }
services-tasks-tooltip =
    { $count ->
       *[other] { $count } tugas
    }
    Terakhir: { $last }
    Rata-rata: { $avg }
    Polling: { $poll }
    Idle: { $idle }
    Total Polling: { $polls }
services-tasks-none = Tidak ada tugas terinstrumentasi
