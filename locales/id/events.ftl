# Event display text. Default ids come from src/events/recorders/; task ids
# from ScheduledTaskOutcome in src/events/display_text.rs. Arguments are data
# (subtype codes, method and API names, task names) and are not translated.
events-ohlcv-default = Event OHLCV: { $subtype }
events-filtering-default = Event pemfilteran: { $subtype }
events-trader-default = Event trader: { $subtype }
events-rpc-default = RPC { $method } - { $action }
events-api-default = { $api } - { $action }

events-task-completed = Tugas '{ $name }' selesai
events-task-failed = Tugas '{ $name }' gagal
events-task-timed-out = Tugas '{ $name }' habis waktu

events-subtype-task-completed = Tugas selesai
events-subtype-task-failed = Tugas gagal
events-subtype-task-timed-out = Tugas habis waktu

events-message-none = Tidak ada pesan

events-ohlcv-cache-cleanup-failed = Gagal membersihkan cache OHLCV
events-ohlcv-gap-cleanup-failed = Gagal membersihkan catatan gap yang sudah terisi
events-ohlcv-gap-fill-failed = Error pengisian gap untuk { $mint }
events-ohlcv-backfill-scheduled = Backfill multi-timeframe dijadwalkan untuk { $mint } melalui { $pool }
events-ohlcv-fetch-failed = Gagal mengambil OHLCV untuk { $mint } melalui { $pool }: { $error }
events-ohlcv-gap-detection-failed = Deteksi gap gagal untuk { $mint } melalui { $pool }
events-ohlcv-fetch-success = { $count } titik OHLCV disimpan untuk { $mint }
events-ohlcv-retention-backfill-failed = Backfill retensi gagal untuk { $mint } melalui { $pool }
events-ohlcv-empty-fetch = Pengambilan OHLCV kosong untuk { $mint } melalui { $pool }
events-ohlcv-pool-discovery-failed = Penemuan pool gagal untuk { $mint }
events-ohlcv-pool-discovery-success = Pool ditemukan untuk { $mint }
events-ohlcv-process-token-error = Error saat memproses { $mint }: { $error }
events-ohlcv-rate-limit-hit = Batas laju terpicu saat memproses { $mint }
events-ohlcv-pool-unavailable = Tidak ada pool sehat untuk { $mint }; ditunda
events-ohlcv-token-missing = Token { $mint } hilang saat pemrosesan
events-monitors-stopped = Monitor trading otomatis dihentikan
events-monitors-starting = Monitor trading otomatis sedang dimulai
events-entry-monitor-started = Monitor peluang entry dimulai
events-exit-monitor-started = Monitor exit/posisi dimulai
events-trader-service-stopped = Layanan trader dihentikan dengan baik
events-trader-service-stopping = Penghentian layanan trader dimulai
events-trader-service-started = Layanan trader terinisialisasi penuh dan berjalan
events-trader-auto-trading-error = Trading otomatis mengalami error
events-trader-trading-enabled = Trading diaktifkan dan berjalan
events-trader-trading-disabled = Trading dinonaktifkan di konfigurasi
events-trader-service-initializing = Inisialisasi layanan trader dimulai
events-connectivity-monitoring-stopped = Pemantauan konektivitas dihentikan
events-connectivity-monitoring-started = Pemantauan konektivitas dimulai (interval={ $seconds } dtk)
events-connectivity-service-initialized = Layanan konektivitas diinisialisasi dengan { $count } monitor
events-connectivity-critical-unhealthy = { $count } endpoint kritis tidak sehat - Sistem sebaiknya menjeda operasi
events-connectivity-endpoint-recovered = Endpoint pulih dari { $from } ke sehat

events-category-swap = Swap
events-category-transaction = Transaksi
events-category-pool = Pool
events-category-position = Posisi
events-category-token = Token
events-category-wallet = Dompet
events-category-trader = Trader
events-category-entry = Entry
events-category-system = Sistem
events-category-ohlcv = OHLCV
events-category-rpc = RPC
events-category-api = API
events-category-security = Keamanan
events-category-connectivity = Konektivitas
events-category-filtering = Pemfilteran
events-category-scheduled-task = Tugas terjadwal
events-category-learner = Learner
events-category-other = Lainnya

events-loading = Memuat event...
events-load-failed = Gagal memuat event
events-load-failed-description = Menunggu backend merespons. Kami akan mencoba lagi secara otomatis.
events-load-error = Tidak dapat memuat event
events-search-placeholder = Cari event...
events-summary-total = Total
events-filter-category = Kategori
events-filter-all-categories = Semua Kategori
events-filter-all-severities = Semua Tingkat Keparahan
events-col-time = Waktu
events-col-category = Kategori
events-col-type = Jenis
events-col-severity = Tingkat Keparahan
events-col-message = Pesan
events-col-token = Token
events-col-details = Detail
events-payload-more = +{ $count } lagi

events-dialog-title = Detail event
events-dialog-close =
    .aria-label = Tutup dialog
events-dialog-payload = Payload
events-dialog-copy = Salin Detail
events-dialog-copy-title =
    .title = Salin semua detail event
events-dialog-copy-done = Disalin!
events-dialog-copy-failed = Gagal
events-dialog-not-available = T/A
events-dialog-category-event = Event { $category }
events-dialog-field-id = ID Event
events-dialog-field-severity = Tingkat Keparahan
events-dialog-field-category = Kategori
events-dialog-field-subtype = Subjenis
events-dialog-field-mint = Mint Token
events-dialog-field-reference = Referensi
events-dialog-field-time = Waktu Event
events-dialog-field-age = Usia
events-dialog-field-created = Dibuat
events-dialog-export-heading = DETAIL EVENT
events-dialog-export-message = PESAN
events-dialog-export-payload = PAYLOAD
events-dialog-export-line = { $label }: { $value }
