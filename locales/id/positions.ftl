# Position details labels.

positions-state-reason-position-created = Posisi dibuat

positions-status-open = Terbuka
positions-status-closed = Tertutup
positions-status-archived = Diarsipkan
positions-origin-copy = Salin
positions-origin-manual = Manual
positions-origin-wallet = Dompet
positions-origin-copy-link =
    .title = Buka tugas salin yang membuka posisi ini
positions-holding-frozen = Dibekukan
    .title = Otoritas mint membekukan akun token ini - saldo tidak dapat ditransfer atau dijual
positions-toolbar-total = Total
positions-toolbar-delete-all = Hapus semua
positions-search-placeholder = Cari berdasarkan simbol atau mint...
positions-filter-origin = Asal
positions-filter-origin-all = Semua asal
positions-filter-origin-auto = Auto Trader
positions-filter-origin-copy = Copy Trading
positions-delete-all-tooltip = Hapus permanen semua posisi yang diarsipkan

positions-column-token = Token
positions-column-archived-at = Diarsipkan
positions-column-entry-time = Waktu Entry
positions-column-exit-time = Waktu Exit
positions-column-avg-entry = Rata-rata Entry ({ -sol })
positions-column-avg-exit = Rata-rata Exit ({ -sol })
positions-column-current-price = Saat Ini ({ -sol })
positions-column-total-invested = Total Diinvestasikan
positions-column-proceeds = Hasil
positions-column-pnl = P&L
positions-column-pnl-percent = P&L %
positions-column-size = Ukuran
positions-column-dca = DCA
positions-column-exits = Exit
positions-column-unrealized-pnl = P&L Belum Terealisasi
positions-column-unrealized-percent = Belum Terealisasi %

positions-unknown-basis = Tidak ada harga pokok dalam riwayat dompet ini (airdrop, fill berkuotasi USD, atau swap tanpa sisi SOL)
positions-unknown-history = Putaran ini tidak cocok dengan saldo on-chain
positions-dca-count =
    { $count ->
       *[other] { $count } DCA
    }
positions-exit-count =
    { $count ->
       *[other] { $count } exit
    }

positions-action-add =
    .title = Tambah ke posisi (DCA)
    .aria-label = Tambah ke posisi
positions-action-sell =
    .title = Jual (penuh atau parsial dalam %)
    .aria-label = Jual posisi
positions-action-sell-frozen = Dibekukan oleh otoritas mint - kepemilikan ini tidak dapat dijual
positions-action-remove =
    .title = Hapus (arsipkan atau hapus permanen)
    .aria-label = Hapus posisi
positions-action-restore =
    .title = Pulihkan ke Terbuka/Tertutup
    .aria-label = Pulihkan posisi
positions-action-delete =
    .title = Hapus permanen
    .aria-label = Hapus permanen
positions-action-in-progress = Sedang berjalan…

positions-caption-buying = Membeli
positions-caption-buying-step = Membeli · { $step }
positions-caption-selling = Menjual
positions-caption-selling-step = Menjual · { $step }
positions-caption-closing = Menutup
positions-caption-failed = Gagal
positions-caption-failed-detail = Gagal · { $error }
positions-step-adding = Menambah
positions-pending-buying = Membeli…
positions-pending-buy-failed = Pembelian gagal

positions-load-failed = Tidak dapat menyegarkan posisi
positions-toast-not-found = Data posisi tidak ditemukan
positions-toast-deleted = Posisi dihapus
positions-toast-archived = Posisi diarsipkan
positions-toast-restored = Posisi dipulihkan
positions-action-failed = Tindakan gagal
positions-delete-title = Hapus posisi permanen
positions-delete-message = Hapus { $symbol } secara permanen? Ini menghapus posisi dan riwayatnya dari database dan tidak dapat dibatalkan. Transaksi dan data token Anda tidak terpengaruh.
positions-delete-confirm = Hapus permanen
positions-delete-all-title = Hapus semua posisi yang diarsipkan
positions-delete-all-message =
    { $count ->
       *[other] Hapus permanen semua { $count } posisi yang diarsipkan? Tindakan ini tidak dapat dibatalkan. Transaksi dan data token tidak terpengaruh.
    }
positions-delete-all-message-empty = Hapus permanen semua posisi yang diarsipkan? Tindakan ini tidak dapat dibatalkan.
positions-delete-all-confirm = Hapus semua
positions-delete-all-done =
    { $count ->
       *[other] { $count } posisi yang diarsipkan dihapus
    }
positions-delete-all-failed = Gagal menghapus posisi yang diarsipkan

positions-remove-title = Hapus posisi
positions-remove-open-warning = <strong>Posisi ini masih terbuka.</strong> Bot sedang memegang token ini. Menghapusnya membebaskan slot trade dan menghentikan pelacakan - tetapi ini <strong>tidak</strong> menjualnya. Jual terlebih dahulu jika Anda ingin { -sol } Anda kembali.
positions-remove-modes =
    .aria-label = Mode penghapusan
positions-remove-archive = Arsipkan
positions-remove-recommended = Disarankan
positions-remove-archive-description = Sembunyikan ke tab Diarsipkan. Dapat dipulihkan kapan saja - tidak ada yang dijual dan semua trade tetap tercatat.
positions-remove-delete = Hapus permanen
positions-remove-delete-description = Hapus posisi ini beserta seluruh riwayatnya dari database.
positions-remove-danger = Ini menghapus posisi dan riwayatnya secara permanen. <strong>Tindakan ini tidak dapat dibatalkan.</strong> Transaksi dan data token Anda tidak terpengaruh.
positions-remove-confirm-archive = Arsipkan posisi

positions-management-changed = Pengelolaan posisi diatur ke { $mode }
positions-details-load-failed = Gagal memuat detail posisi
positions-details-mint-label = Alamat mint
positions-details-management-failed = Gagal memperbarui pengelolaan posisi
positions-details-favorite-add =
    .title = Tambah ke favorit
    .aria-label = Tambah ke favorit
positions-details-favorite-remove =
    .title = Hapus dari favorit
    .aria-label = Hapus dari favorit
positions-details-view-solscan =
    .title = Lihat di { -solscan }
    .aria-label = Lihat token di { -solscan }
positions-details-close =
    .title = Tutup (Esc)
    .aria-label = Tutup
positions-details-chart-section =
    .aria-label = Grafik harga
positions-details-loading-chart = Memuat grafik...
positions-details-activity-section =
    .aria-label = Aktivitas
positions-details-activity-title = Aktivitas
positions-details-split-handle =
    .aria-label = Ubah ukuran grafik dan aktivitas
positions-details-activity-pane =
    .aria-label = Panel aktivitas
positions-details-activity-expand =
    .title = Perluas aktivitas
    .aria-label = Perluas aktivitas
positions-details-summary-section =
    .aria-label = Ringkasan posisi
positions-details-loading = Memuat posisi...

positions-management-auto-trader = Auto Trader
positions-management-user-only = Hanya Pengguna
positions-management-copy-task = Tugas Salin
positions-management-hybrid = Hibrida
positions-pane-show-chart = Tampilkan grafik
positions-pane-show-activity = Tampilkan aktivitas
positions-pane-restore-activity = Pulihkan aktivitas
positions-pane-expand-chart =
    .title = Perluas grafik
    .aria-label = Perluas grafik

positions-risk-low = Risiko rendah
positions-risk-medium = Risiko sedang
positions-risk-high = Risiko tinggi
positions-risk-unknown = Risiko tidak diketahui
positions-busy-buying = Pembelian sedang berjalan…
positions-busy-selling = Penjualan sedang berjalan…
positions-busy-closing = Penutupan sedang berjalan…
positions-header-avg-entry = Rata-rata entry
positions-header-buy-count =
    { $count ->
       *[other] { $count } pembelian
    }
positions-header-exit-price = Harga exit
positions-header-closed-ago = ditutup { $ago }
positions-header-realized-pnl = P&L Terealisasi
positions-header-usd-note = USD pada harga { -sol } hari ini
positions-header-returned = Dikembalikan
positions-header-of-invested = dari { $amount } yang diinvestasikan
positions-header-price = Harga
positions-header-last-price = Harga terakhir
positions-header-pool-ago = pool · { $ago }
positions-header-unrealized-pnl = P&L Belum Terealisasi
positions-header-pnl-last-price = P&L pada harga terakhir
positions-header-value = Nilai
positions-header-last-value = Nilai terakhir
positions-header-invested = { $amount } diinvestasikan
positions-header-origin-hint = Cara posisi ini dibuka
positions-header-risk-hint = Skor { -rugcheck } - makin rendah makin aman
positions-header-frozen = Dibekukan
    .title = Otoritas mint membekukan kepemilikan ini
positions-header-managed-by = Dikelola oleh
positions-header-management-select =
    .aria-label = Pengelolaan posisi

positions-origin-unknown = tidak diketahui
positions-origin-copied-task = Disalin · tugas { $task }
positions-origin-manual-entry = Entry manual
positions-origin-wallet-entry = Entry dompet
positions-origin-auto-strategy = Otomatis · { $strategy }
positions-origin-auto-entry = Entry otomatis

positions-pending-adding = Menambah
positions-pending-adding-amount = Menambah { $amount }
positions-pending-selling = Menjual
positions-pending-selling-percent = Menjual { $percent }
positions-pending-confirming = { $label } · mengonfirmasi
    .title = Terkirim dan menunggu konfirmasi on-chain. Angka diperbarui setelah terverifikasi.

positions-trade-add = Tambah
    .title = Tambah ke posisi
positions-trade-sell = Jual
    .title = Jual sebagian posisi
positions-trade-close = Tutup posisi
    .title = Jual semuanya dan tutup
positions-trade-token = Detail token
    .title = Buka detail token

positions-favorite-token-fallback = Token
positions-favorite-added = { $symbol } ditambahkan ke favorit
positions-favorite-removed = { $symbol } dihapus dari favorit
positions-favorite-add-failed = Gagal menambahkan favorit
positions-favorite-remove-failed = Gagal menghapus favorit
positions-favorite-update-failed = Gagal memperbarui favorit

positions-summary-position = Posisi
positions-summary-price-path = Jalur harga
positions-summary-network-fees = Biaya jaringan
positions-summary-risk = Risiko
positions-summary-market = Pasar
positions-summary-market-now = Pasar saat ini
positions-summary-links = Tautan
positions-fact-tokens-fallback = token
positions-fact-bought = Dibeli
positions-fact-holding = Kepemilikan
positions-fact-sold = Dijual
positions-fact-realized = Terealisasi
positions-fact-opened = Dibuka
positions-fact-closed = Ditutup
positions-fact-reason = Alasan
positions-fact-archived = Diarsipkan
positions-fact-entry = Entry
positions-fact-exit = Exit
positions-fact-total = Total
positions-fact-verified = Terverifikasi on-chain
positions-fact-confirming = Mengonfirmasi
positions-fact-share-of-bought = { $percent } dari yang dibeli
positions-fact-share-of-invested = { $percent } dari yang diinvestasikan
positions-fact-entry-count =
    { $count ->
        [0] 1 entry
       *[other] 1 entry + { $count } tambahan
    }
positions-fact-partial-exits-back =
    { $count ->
       *[other] { $count } exit parsial · { $returned } kembali
    }
positions-fact-held = di-hold { $age }
positions-fact-vs-entry = { $percent } vs entry
positions-fact-exit-vs-peak = Exit vs puncak
positions-fact-now-vs-peak = Saat ini vs puncak
positions-fact-entry-range = Rentang entry
positions-range-low = Terendah
positions-range-peak = Puncak
positions-range-now = Saat ini
positions-range-label-exit = Harga entry dan exit di antara titik terendah dan puncak
positions-range-label-now = Harga entry dan saat ini di antara titik terendah dan puncak
positions-fact-mint-authority = Otoritas mint
positions-fact-freeze-authority = Otoritas freeze
positions-fact-active = Aktif
positions-fact-pool = Pool
positions-fact-pool-liquidity = Likuiditas { $amount } { -sol }
positions-fact-market-cap = Kapitalisasi pasar
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = Likuiditas
positions-fact-volume-24h = Volume 24h
positions-fact-price-change = Perubahan harga
positions-change-period-1h = 1h
positions-change-period-24h = 24h
positions-fact-holders = Holder
positions-link-website = Situs web
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

positions-activity-load-failed = Aktivitas tidak dapat dimuat
positions-activity-loading = Memuat aktivitas...
positions-activity-empty = Belum ada aktivitas pada token ini di dompet ini
positions-activity-filter-empty = Tidak ada aktivitas yang cocok dengan filter ini
positions-activity-round-count =
    { $count ->
       *[other] { $count } putaran
    }
positions-activity-event-count =
    { $count ->
       *[other] { $count } event
    }
positions-activity-pending-count = { $count } tertunda
positions-activity-failed-count = { $count } gagal
positions-filter-all = Semua
positions-filter-trades = Trade
positions-filter-buys = Beli
positions-filter-sells = Jual
positions-filter-wallet = Dompet
positions-filter-issues = Masalah
positions-activity-filters =
    .aria-label = Filter aktivitas
positions-activity-totals =
    .aria-label = Semua putaran pada token ini
positions-activity-realized-all = Terealisasi, semua putaran
positions-activity-invested = Diinvestasikan
positions-activity-returned = Dikembalikan
positions-activity-opened = Dibuka { $when }
positions-activity-round-title = Posisi { $index }
positions-activity-this-position = Posisi ini
positions-activity-dates-unavailable = Tanggal tidak tersedia
positions-activity-wallet-title = Transaksi dompet
positions-activity-outside =
    { $count ->
       *[other] Di luar posisi mana pun · { $range } · { $count } event
    }
positions-details-signature-label = Signature

positions-state-open = Posisi terbuka
positions-state-closing = Posisi sedang ditutup
positions-state-closed = Posisi tertutup
positions-state-exit-pending = Exit posisi tertunda
positions-state-exit-failed = Exit posisi gagal
positions-state-phantom = Posisi phantom
positions-state-reconciling = Posisi sedang direkonsiliasi

positions-event-kind-entry = Entry
positions-event-kind-dca = Tambah
positions-event-kind-partial-exit = Exit parsial
positions-event-kind-exit = Exit
positions-event-kind-buy = Beli dompet
positions-event-kind-sell = Jual dompet
positions-event-kind-transfer = Transfer
positions-event-kind-ata = Akun token
positions-event-kind-other = Transaksi
positions-event-state-pending = Tertunda
positions-event-state-failed = Gagal
positions-event-state-synthetic = Sintetis
positions-chain-status-failed-detail = Gagal: { $error }
positions-event-tokens-fallback = token
positions-event-entry-submitted = Pembelian dikirim untuk { $amount }
positions-event-entry-for = Membeli { $amount } seharga { $sol }
positions-event-entry = Membeli { $amount }
positions-event-dca-submitted = Penambahan dikirim untuk { $amount }
positions-event-dca-for = Menambah { $amount } seharga { $sol }
positions-event-dca = Menambah { $amount }
positions-event-partial-exit-submitted-percent = Exit parsial { $percent } dikirim untuk { $amount }
positions-event-partial-exit-submitted = Exit parsial dikirim untuk { $amount }
positions-event-sold-percent-for = Menjual { $amount } ({ $percent }) seharga { $sol }
positions-event-sold-percent = Menjual { $amount } ({ $percent })
positions-event-sold-for = Menjual { $amount } seharga { $sol }
positions-event-sold = Menjual { $amount }
positions-event-exit-submitted = Exit posisi penuh dikirim
positions-event-exit-for = Ditutup dengan { $amount } terjual seharga { $sol }
positions-event-exit-closed = Posisi ditutup
positions-event-wallet-bought = Dompet membeli { $amount } di tempat lain
positions-event-wallet-sold = Dompet menjual { $amount } di tempat lain
positions-event-received = Menerima { $amount }
positions-event-sent = Mengirim { $amount }
positions-event-transferred = Mentransfer { $amount }
positions-event-ata = Aktivitas akun token
positions-event-wallet-transaction = Transaksi dompet yang melibatkan { $amount }
positions-event-price-per-token = { $price } { -sol } / token
positions-event-wallet-change = Perubahan dompet { $amount }
positions-event-after-title = Posisi setelah event ini
positions-event-capital-invested = Modal yang diinvestasikan
positions-event-average-entry = Rata-rata entry
positions-event-transfers-title = Transfer token
positions-event-transfer-amount = Jumlah
positions-event-transfer-mint = Mint
positions-event-transfer-from = Dari
positions-event-transfer-to = Ke
positions-event-no-signature = Tidak ada signature on-chain
positions-event-click-to-copy = Klik untuk menyalin
positions-event-solscan = { -solscan }
positions-event-token-amount = Jumlah token
positions-event-trade-price = Harga trade
positions-event-sol-amount = Jumlah { -sol }
positions-event-cost-basis = Harga pokok
positions-event-usd-value = Nilai USD
positions-event-network-fee = Biaya jaringan
positions-event-router = Router
positions-event-slot = Slot
positions-event-chain-status = Status chain
positions-event-transaction-type = Jenis transaksi
positions-event-direction = Arah
positions-event-wallet-sol-change = Perubahan { -sol } dompet
positions-event-instructions = Instruksi
positions-event-compute-units = Compute unit
positions-event-accounts = Akun
positions-event-record-id = ID catatan
positions-event-time-unavailable = Waktu tidak tersedia
positions-event-details = Detail
positions-event-hide-details = Sembunyikan detail

positions-chart-type-candles = Candle
positions-chart-type-line = Garis
positions-chart-type-area = Area
positions-chart-type-group =
    .aria-label = Jenis grafik
positions-chart-overlays-group =
    .aria-label = Overlay grafik
positions-chart-ema = EMA
    .title = Exponential moving average, 9 dan 21
positions-chart-fit = Pas
    .title = Sesuaikan bingkai dengan masa hidup posisi ini
positions-chart-timeframes-group =
    .aria-label = Timeframe
positions-chart-pane-group =
    .aria-label = Panel grafik
positions-chart-unavailable = Mesin grafik tidak tersedia
positions-chart-collecting = Mengumpulkan data grafik…
positions-chart-no-data = Belum ada data grafik untuk token ini
positions-chart-avg-entry = Rata-rata Entry
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = Rata-rata entry
positions-chart-legend-avg-entry-off-scale = Rata-rata entry (di luar skala)
positions-chart-dropped-events =
    { $count ->
       *[other] { $count } event tanpa candle pada timeframe ini
    }
positions-chart-level = Level
positions-chart-level-above = { $label } { $price } berada di atas tampilan ini
positions-chart-level-below = { $label } { $price } berada di bawah tampilan ini
positions-chart-scale-hint = Seret sumbu harga untuk menskalakan hingga ke sana
positions-chart-pnl-at-bar = P&L @ Bar
positions-chart-click-to-locate = Klik untuk menemukan
