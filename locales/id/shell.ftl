# Dashboard shell: header, ticker, notification drawer and status bar.

shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-brand-logo =
    .alt = { -brand }
shell-version = v{ $version }

shell-header-brand =
    .aria-label = Buka beranda dasbor
    .title = Beranda dasbor
shell-bot-card =
    .aria-label = Memuat status Auto Trader
shell-bot-label = Auto
shell-bot-status-loading = MEMUAT
shell-bot-today = Hari ini
shell-explore-control =
    .aria-label = Mode Jelajah. Hubungkan dompet dan endpoint RPC untuk mengaktifkan semua fitur
    .title = Hubungkan dompet dan endpoint RPC untuk mengaktifkan trading, saldo, dan data on-chain langsung
shell-explore-title = Mode Jelajah
shell-explore-detail = Dompet & RPC belum terhubung
shell-explore-action = Selesaikan penyiapan
shell-wallet-card =
    .aria-label = Nilai dompet; buka Posisi
    .title = Nilai dompet ({ -sol } + token) · buka Posisi
shell-wallet-worth-label = NILAI
shell-wallet-native-label = { -sol }
shell-wallet-tokens-label = TKN
shell-sol-price-card =
    .aria-label = Harga { -sol } dalam USD — buka grafik
    .title = Harga { -sol } · klik untuk grafik
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24h
shell-copy-card =
    .aria-label = Copy trading; buka Copy Trading
    .title = Copy trading · buka Copy Trading
shell-copy-label = COPY TRADING
shell-actions-more =
    .aria-label = Aksi header lainnya
    .title = Aksi lainnya
shell-actions-group =
    .aria-label = Aksi header
shell-action-search =
    .aria-label = Cari token
    .title = Cari token (Ctrl/Cmd+K)
shell-action-featured =
    .aria-label = Token unggulan
    .title = Token unggulan
shell-action-notifications =
    .aria-label = Aksi dan notifikasi
    .title = Aksi dan notifikasi
shell-action-restart =
    .aria-label = Mulai ulang aplikasi
    .title = Mulai ulang aplikasi
shell-action-theme =
    .aria-label = Ganti tema
    .title = Ganti tema
shell-action-settings =
    .aria-label = Pengaturan
    .title = Pengaturan

shell-ticker-monitoring-segment =
    .title = Token yang dipantau oleh Pool Service
shell-ticker-monitoring = Dipantau:
shell-ticker-filtering-segment =
    .title = Token yang lolos/gagal kriteria pemfilteran
shell-ticker-passed = Lolos:
shell-ticker-rejected = Ditolak:
shell-ticker-pnl-segment =
    .title = Untung rugi terealisasi hari ini
shell-ticker-pnl = P&L Hari Ini:
shell-ticker-rpc-segment =
    .title = Panggilan RPC per menit dan tingkat keberhasilan
shell-ticker-rpc = RPC:
shell-ticker-rpc-rate = { $amount }/mnt
shell-ticker-services-segment =
    .title = Status kesehatan layanan latar belakang
shell-ticker-services-loading = Layanan: <strong>Memuat</strong>

shell-notification-title = Aksi
shell-notification-mark-all-read =
    .title = Tandai semua sudah dibaca
shell-notification-clear-all =
    .title = Hapus semua
shell-notification-close =
    .aria-label = Tutup
shell-notification-tab-all = Semua
shell-notification-tab-active = Aktif
shell-notification-tab-done = Selesai
shell-notification-tab-failed = Gagal
shell-notification-filter-type-all = Semua Jenis
shell-notification-filter-type-buy = Beli
shell-notification-filter-type-sell = Jual
shell-notification-filter-type-open = Buka
shell-notification-filter-type-close = Tutup
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = Parsial
shell-notification-filter-state-all = Semua Status
shell-notification-filter-state-in-progress = Sedang Berjalan
shell-notification-filter-state-completed = Selesai
shell-notification-filter-state-failed = Gagal
shell-notification-filter-state-cancelled = Dibatalkan
shell-notification-list =
    .aria-label = Notifikasi
shell-notification-empty = Belum ada aksi
shell-notification-loading-more = Memuat lebih banyak...
shell-notification-back-to-top =
    .title = Kembali ke atas

shell-status-bar-version = v
shell-status-bar-uptime = Aktif
shell-status-bar-memory = Mem
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/mnt
shell-status-bar-trading = Trading
shell-status-bar-positions = Pos
shell-status-bar-tokens = Token

shell-splash-starting = Memulai { -brand }
shell-splash-waiting = Menunggu core lokal merespons.
shell-splash-failed = { -brand } tidak dapat dimulai
shell-splash-failed-detail = Periksa file log, lalu mulai ulang aplikasi.

shell-connection-connected = Core Terhubung
shell-connection-waiting = Menunggu core…
shell-connection-retry-now = Coba lagi sekarang
shell-connection-overlay-detail = Core tidak dapat dijangkau. Trading dijeda; akan pulih otomatis.
shell-connection-restored = Koneksi core dipulihkan

shell-trader-control-failed = Kontrol trader gagal
shell-notification-button-unread = Aksi dan notifikasi, { $count } belum dibaca
shell-restart-confirm-title = Mulai Ulang Bot
shell-restart-confirm-message =
    Yakin ingin memulai ulang bot?

    Ini akan:
    • Menghentikan semua layanan
    • Memulai ulang proses
    • Memakan waktu ~10-15 dtk

    Semua operasi yang sedang berjalan akan terganggu.
shell-restart-confirm-action = Mulai Ulang
shell-restart-progress = Memulai ulang bot
shell-restart-failed = Mulai ulang gagal
shell-restart-failed-status = Mulai ulang gagal: { $status }
shell-restart-helper-unavailable = Helper mulai ulang otomatis tidak tersedia. Muat ulang dasbor sebentar lagi.

shell-page-title-fallback = Dasbor
shell-page-load-failed = Gagal Memuat Halaman
shell-page-offline-detail = Core tidak dapat dijangkau saat ini. Halaman ini akan dimuat otomatis setelah koneksi kembali.

shell-bot-state-explore = JELAJAH
shell-bot-state-halted = DIHENTIKAN
shell-bot-state-off = NONAKTIF
shell-bot-state-waiting = MENUNGGU
shell-bot-state-idle = SIAGA
shell-bot-state-entry-paused = ENTRY DIJEDA
shell-bot-state-running = BERJALAN
shell-bot-control-explore = Auto Trader tidak tersedia di Mode Jelajah. Buka penyiapan dompet dan RPC.
shell-bot-control-halted = Penghentian darurat aktif. Buka kontrol Auto Trader.
shell-bot-control-off = Auto Trader nonaktif. Klik untuk mengaktifkan.
shell-bot-control-waiting = Auto Trader aktif dan menunggu layanan core. Klik untuk menonaktifkan.
shell-bot-control-idle = Auto Trader aktif, tetapi kedua monitor nonaktif. Buka kontrol Auto Trader.
shell-bot-control-entry-paused = Perlindungan kerugian menjeda entry; exit dapat berlanjut. Buka kontrol Auto Trader.
shell-bot-control-running = Auto Trader berjalan. Klik untuk menonaktifkan.

shell-wallet-card-summary = Nilai dompet: { $equity } { -sol } ({ $balance } { -sol } tunai, { $tokens } token); buka Posisi
shell-copy-running-live = { $count } live
shell-copy-running-paper = { $count } paper
shell-copy-value-paused = Dijeda
shell-copy-value-idle = Siaga
shell-copy-sub-active = { $active } dari { $total } aktif

shell-ticker-services-healthy = Layanan: <strong>Sehat</strong>
shell-ticker-services-issues =
    { $count ->
       *[other] Layanan: <strong>{ $count } Masalah</strong>
    }

shell-agent-request-title = Permintaan agen
shell-agent-request-client-fallback = Agen yang terhubung
shell-agent-request-message = { $client } ingin menjalankan "{ $tool }" di { -brand }. Permintaan ini { $expiry }.
shell-agent-request-message-arguments = { $client } ingin menjalankan "{ $tool }" di { -brand }. Argumen: { $summary }. Permintaan ini { $expiry }.
shell-agent-request-expires-minutes = kedaluwarsa dalam { $minutes } mnt
shell-agent-request-expires-seconds = kedaluwarsa dalam { $seconds } dtk
shell-agent-request-approve = Setujui
shell-agent-request-deny = Tolak

shell-toast-copied = { $label } disalin
shell-toast-copy-failed = Gagal menyalin
shell-toast-still-running = Masih berjalan — periksa pusat notifikasi
shell-toast-dismiss =
    .aria-label = Tutup
shell-confirm-title = Konfirmasi Tindakan
shell-confirm-message = Apakah Anda yakin?

shell-assistant-label = Asisten
shell-assistant-dialog =
    .aria-label = Asisten

shell-status-bar-trading-active = Aktif
shell-status-bar-trading-inactive = Nonaktif

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title } dibatalkan
shell-action-swap-buy-live = Membeli
shell-action-swap-buy-done = Dibeli
shell-action-swap-buy-failed = Pembelian gagal
shell-action-swap-sell-live = Menjual
shell-action-swap-sell-done = Dijual
shell-action-swap-sell-failed = Penjualan gagal
shell-action-position-open-live = Membuka posisi
shell-action-position-open-done = Dibuka
shell-action-position-open-failed = Pembukaan gagal
shell-action-position-close-live = Menutup posisi
shell-action-position-close-done = Ditutup
shell-action-position-close-failed = Penutupan gagal
shell-action-position-dca-live = Menambah ke posisi
shell-action-position-dca-done = Ditambahkan ke
shell-action-position-dca-failed = Penambahan gagal
shell-action-partial-exit-live = Exit parsial
shell-action-partial-exit-done = Exit parsial
shell-action-partial-exit-failed = Exit parsial gagal
shell-action-manual-order-live = Menempatkan order
shell-action-manual-order-done = Order ditempatkan
shell-action-manual-order-failed = Order gagal
shell-action-trade-live = Trade
shell-action-trade-done = Trade selesai
shell-action-trade-failed = Trade gagal
shell-action-via-router = { $action } melalui { $router }
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = menghindari { $venue }
shell-action-cost-guard-avoiding-cost = menghindari { $venue } · { $cost }
shell-action-cost-guard-avoiding-unnamed = menghindari satu platform
shell-action-cost-guard-avoiding-unnamed-cost = menghindari satu platform · { $cost }
shell-action-cost-guard-avoided = { $outcome } · menghindari sewa { $cost } di { $venue }
shell-action-cost-guard-avoided-unnamed = { $outcome } · menghindari sewa platform { $cost }
shell-action-exit-full = Exit penuh
shell-action-exit-percent = Exit { $percent }

shell-exit-title = Tutup { -brand }?
shell-exit-description = Pilih cara Anda ingin menutup aplikasi
shell-exit-minimize = Minimalkan ke Tray
shell-exit-minimize-detail = Tetap berjalan di latar belakang
shell-exit-quit = Keluar dari Aplikasi
shell-exit-quit-detail = Tutup sepenuhnya dan hentikan semua layanan

shell-lightbox-save =
    .title = Simpan gambar
shell-lightbox-close =
    .title = Tutup (ESC)

shell-theme-light = Terang
shell-theme-dark = Gelap
shell-theme-switch-to-light = Ganti ke tema terang
shell-theme-switch-to-dark = Ganti ke tema gelap
