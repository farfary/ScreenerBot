# Trader page labels.

trader-exit-type-stop-loss = Stop Loss
trader-exit-type-take-profit = Take Profit
trader-exit-type-roi = Target ROI
trader-exit-type-roi-exit = Target ROI
trader-exit-type-trailing-stop = Trailing Stop
trader-exit-type-time-override = Override Waktu
trader-exit-type-time-rule = Aturan Waktu
trader-exit-type-manual = Manual
trader-exit-type-manual-close = Manual
trader-exit-type-dca = DCA
trader-exit-type-unknown = Tidak diketahui

trader-tab-stats = Statistik
trader-tab-strategy-control = Kontrol Strategi
trader-tab-strategies = Strategi
trader-tab-stop-loss = Stop Loss
trader-tab-trailing-stop = Trailing Stop
trader-tab-roi = Take Profit
trader-tab-time-rules = Aturan Waktu
trader-tab-dca = DCA
trader-tab-settings = Pengaturan

trader-feature-coming-soon = Segera Hadir
    .message = Fitur ini segera hadir dan belum tersedia.
trader-feature-beta = Beta
trader-feature-disabled = Nonaktif
    .message = Fitur ini sedang dinonaktifkan.

trader-status-title = Auto Trader
trader-status-loading = Memuat...
trader-status-running = Berjalan
trader-status-stopped = Berhenti
trader-status-setup-required = Perlu penyiapan
trader-status-unavailable = Selesaikan penyiapan dompet dan RPC untuk menggunakan Auto Trader
trader-toggle-on = AKTIF
trader-toggle-off = NONAKTIF
trader-toggle-unavailable = TIDAK TERSEDIA
trader-toggle-start-failed = Gagal memulai trader
trader-toggle-stop-failed = Gagal menghentikan trader
trader-controls-title = Kontrol Trading
trader-halt-title = TRADING DIHENTIKAN
trader-halt-reason-default = Penghentian paksa manual
trader-halt-resume = Lanjutkan
trader-monitor-entry = Monitor Entry
trader-monitor-exit = Monitor Exit
trader-monitor-master-off = Auto Trader nonaktif
trader-loss-limit-title = Batas Kerugian Periode
trader-loss-limit-resume = Lanjutkan trading
trader-loss-limit-reset = Reset periode
trader-loss-limit-off = Nonaktif
trader-loss-limit-none = Batas kerugian periode belum dikonfigurasi
trader-loss-limit-resets-in = Direset dalam { $hours } { $minutes }
trader-loss-limit-reached = BATAS TERCAPAI
trader-force-stop = Hentikan Paksa Semuanya

trader-force-stop-confirm = Hentikan Paksa Trading
    .message = Ini akan langsung menghentikan SEMUA operasi trading. Lanjutkan?
    .confirm = Hentikan Trading
trader-loss-limit-resume-confirm = Lanjutkan Setelah Batas Kerugian
    .message = Batas kerugian periode menghentikan entry baru. Melanjutkan membuat trader dapat membuka posisi lagi sebelum periode direset. Lanjutkan?
trader-loss-limit-reset-confirm = Reset Periode Batas Kerugian
    .message = Ini menghapus akumulasi kerugian periode saat ini dan memulai periode baru. Lanjutkan?

trader-toast-control-failed = Kontrol Auto Trader gagal
trader-toast-force-stop-on = Penghentian paksa diaktifkan
trader-toast-force-stop-failed = Tidak dapat mengaktifkan penghentian paksa
trader-toast-force-stop-cleared = Penghentian paksa dibersihkan
trader-toast-resume-failed = Tidak dapat melanjutkan trading
trader-toast-loss-limit-reset-failed = Tidak dapat mereset batas kerugian
trader-toast-entry-monitor-failed = Tidak dapat mengalihkan monitor entry
trader-toast-exit-monitor-failed = Tidak dapat mengalihkan monitor exit
trader-toast-load-failed = Gagal Memuat
    .message = Gagal memuat konfigurasi trader
trader-toast-saved = Konfigurasi Disimpan
    .message = Pengaturan trader berhasil diterapkan
trader-toast-save-failed = Gagal Menyimpan
    .message = Gagal menyimpan konfigurasi trader
trader-toast-feature-enabled = Fitur Diaktifkan
trader-toast-feature-disabled = Fitur Dinonaktifkan
trader-toast-feature-applied = Pengaturan Auto Trader diterapkan
trader-toast-strategy-enabled = Strategi Diaktifkan
    .message = Strategi aktif
trader-toast-strategy-disabled = Strategi Dinonaktifkan
    .message = Strategi tidak aktif
trader-toast-strategy-failed = Pembaruan Gagal
    .message = Gagal memperbarui status strategi

trader-stats-window =
    .aria-label = Jendela statistik
trader-stats-window-day = 24H
trader-stats-window-week = 7D
trader-stats-window-month = 30D
trader-realized-title = Performa Terealisasi
trader-metric-net-pnl = P&L Bersih
trader-metric-win-rate = Win Rate
trader-metric-profit-factor = Profit Factor
trader-metric-max-drawdown = Drawdown Maks
trader-metric-capital = Modal yang Bekerja
trader-metric-avg-win-loss = Rata-rata Menang / Kalah
trader-metric-closed-trades = Trade Tertutup
trader-metric-median-hold = Median Durasi Hold
trader-stats-empty = Tidak ada trade tertutup di jendela ini
trader-stats-won-lost = { $won } menang · { $lost } kalah
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
       *[other] { $amount } menang
    }
trader-stats-losses =
    { $count ->
       *[other] { $amount } kalah
    }
trader-stats-expected = { $amount } diharapkan per trade
trader-stats-profit-factor-basis = Total menang ÷ total kalah
trader-stats-drawdown-basis = Penurunan terdalam dari puncak ke dasar yang terealisasi
trader-stats-slots =
    { $count ->
       *[other] { $used } dari { $max } slot posisi terpakai
    }
trader-stats-avg-basis = Rata-rata hasil trade menang vs kalah
trader-stats-closed =
    { $count ->
       *[other] { $amount } posisi ditutup
    }
trader-stats-hold-average = rata-rata { $span }
trader-stats-excluded =
    { $count ->
       *[other] { $amount } putaran tertutup dikecualikan — tidak ada harga pokok lengkap, sehingga tidak ada P&L yang akurat.
    }

trader-daily-title = P&L Harian
trader-daily-subtitle = { -sol } terealisasi per hari, beserta total berjalan
trader-daily-loading = Memuat P&L harian...
trader-daily-chart = Untung rugi terealisasi harian dalam { -sol }
trader-extreme-best = Trade terbaik
trader-extreme-worst = Trade terburuk

trader-exit-title = Rincian Strategi Exit
trader-exit-subtitle = Cara posisi ditutup, dan hasil dari setiap exit
trader-exit-loading = Memuat data exit...
trader-exit-empty-day = Tidak ada trade tertutup dalam 24 jam terakhir
trader-exit-empty-days =
    { $count ->
       *[other] Tidak ada trade tertutup dalam { $amount } hari terakhir
    }
trader-exit-share =
    { $count ->
       *[other] { $amount } trade · { $share } dari exit
    }
trader-exit-average = rata-rata { $value }

trader-impact-label = Dampak:
trader-current-label = Saat ini:
trader-readable-label = Terbaca:
trader-example-how-it-works = Cara Kerja
trader-step-entry = Entry
trader-step-initial-position = Posisi awal
trader-step-auto-exit = Exit Otomatis
trader-step-exit = Exit
trader-step-full-exit = Exit posisi penuh
trader-value-percent = { $value }%
trader-example-profit = +{ $value }% untung

trader-stop-loss-title = Stop Loss
trader-stop-loss-subtitle = Exit otomatis dari posisi saat kerugiannya melebihi ambang Anda
trader-stop-loss-threshold-badge = Batas kerugian
trader-stop-loss-hold-badge = Jeda opsional
trader-stop-loss-impact = Exit saat turun { $threshold }% dari entry
trader-stop-loss-hold-immediate = Segera
trader-stop-loss-hold-delay = Jeda { $span }
trader-stop-loss-price-falls = Harga Turun
trader-stop-loss-threshold-reached = Ambang tercapai
trader-stop-loss-partial = Exit parsial diizinkan
trader-stop-loss-summary = Kerugian dibatasi hingga <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Catatan:</strong> Stop loss melindungi dari kerugian lebih besar dengan exit lebih awal

trader-trailing-title = Trailing Stop
trader-trailing-subtitle = Lindungi keuntungan secara otomatis dengan mengikuti harga saat naik
trader-trailing-activation-badge = Kapan dimulai
trader-trailing-distance-badge = Margin aman
trader-trailing-activation-impact = Mulai mengikuti pada untung +{ $value }%
trader-trailing-distance-impact = Exit pada -{ $value }% dari puncak
trader-trailing-activation = Aktivasi
trader-trailing-peak = Puncak
trader-trailing-final = +{ $value }% akhir
trader-trailing-summary-protected = Melindungi untung <strong>{ $value }</strong>
trader-trailing-summary-avoided = Menghindari rugi <strong>{ $value }</strong> dari puncak

trader-roi-title = Take Profit
trader-roi-subtitle = Exit seluruh posisi secara otomatis saat untung mencapai target Anda
trader-roi-target-badge = Target tunggal
trader-roi-impact = Exit pada untung +{ $target }%
trader-roi-example-title = Skenario Contoh
trader-roi-initial-buy = Pembelian awal
trader-roi-target-hit = Target Tercapai
trader-roi-full-position = Posisi Penuh
trader-roi-sold = 100% terjual
trader-roi-summary = Mengunci untung <strong>+{ $target }%</strong>

trader-time-title = Exit Berbasis Waktu
trader-time-subtitle = Exit posisi secara otomatis setelah durasi hold maksimum jika kerugian melebihi ambang
trader-time-hold-badge = Pemicu waktu
trader-time-loss-badge = Gerbang kerugian
trader-time-unit-seconds = detik
trader-time-unit-minutes = menit
trader-time-unit-hours = jam
trader-time-unit-days = hari
trader-time-conversion-default = 168 jam = 7 hari
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
       *[other] { $amount } dtk
    }
trader-duration-minutes =
    { $count ->
       *[other] { $amount } mnt
    }
trader-duration-hours =
    { $count ->
       *[other] { $amount } jam
    }
trader-duration-days =
    { $count ->
       *[other] { $amount } hari
    }
trader-time-loss-impact = Exit jika turun { $value }% atau lebih setelah masa hold
trader-time-day = Hari { $day }
trader-time-position-opened = Posisi dibuka
trader-time-limit = Batas Waktu
trader-time-hold-reached = Masa hold tercapai
trader-time-loss-met = Ambang kerugian terpenuhi
trader-time-note = <strong>Catatan:</strong> Posisi yang untung atau rugi lebih kecil TIDAK akan di-exit
trader-time-positions-title = Status Posisi Saat Ini
trader-time-positions-loading = Memuat posisi...
trader-time-positions-empty = Tidak ada posisi terbuka
trader-time-positions-hold = Durasi Hold:
trader-time-positions-roi = ROI:

trader-strategy-entry-title = Strategi Entry
trader-strategy-entry-subtitle = Sinyal yang dapat membuka posisi baru.
trader-strategy-exit-title = Strategi Exit
trader-strategy-exit-subtitle = Sinyal yang dapat menutup atau melindungi posisi terbuka.
trader-strategy-active-unknown = -- aktif
trader-strategy-active = { $enabled }/{ $total } aktif
trader-strategy-loading = Memuat strategi...
trader-strategy-load-failed = Tidak dapat memuat strategi
trader-strategy-empty = Belum ada strategi yang ditentukan
trader-strategy-no-description = Tidak ada deskripsi.
trader-strategy-unnamed = Strategi tanpa nama
trader-strategy-type-unknown = Strategi
trader-strategy-priority-auto = Otomatis
trader-strategy-priority = Prioritas { $priority }

trader-dca-title = Dollar-Cost Averaging
trader-dca-subtitle = Tambah otomatis ke posisi yang rugi untuk menurunkan harga entry rata-rata Anda
trader-dca-threshold-badge = Pemicu entry
trader-dca-example-title = Contoh DCA
trader-dca-example = 0.01 { -sol } awal → DCA #1: 0.005 { -sol } @ -10% → DCA #2: 0.005 { -sol } @ -10% lagi
trader-dca-info-title = Info Strategi DCA
trader-dca-info-subtitle = Pertimbangan penting untuk trading DCA
trader-dca-how-title = Cara Kerja DCA
trader-dca-how-trigger = <strong>Pemicu:</strong> Posisi turun di bawah ambang DCA (mis., -10%)
trader-dca-how-action = <strong>Aksi:</strong> Tambah lebih banyak { -sol } untuk menurunkan harga pokok rata-rata
trader-dca-how-repeat = <strong>Ulangi:</strong> Dapat DCA beberapa kali sesuai jumlah maksimum
trader-dca-risk-title = Peringatan Risiko
trader-dca-risk-exposure = <strong>Eksposur Meningkat:</strong> DCA menambah total modal yang berisiko per posisi
trader-dca-risk-knife = <strong>Falling Knife:</strong> DCA tidak membantu jika token terus turun
trader-dca-risk-cooldown = <strong>Cooldown:</strong> Gunakan cooldown untuk menghindari entry DCA beruntun

trader-sizing-title = Ukuran Posisi
trader-sizing-subtitle = Atur berapa banyak yang diinvestasikan per posisi
trader-sizing-positions-badge = Kontrol risiko
trader-sizing-trade-size-badge = Per posisi
trader-timing-title = Waktu & Cooldown
trader-timing-subtitle = Atur jeda waktu antar operasi
trader-timing-close-cooldown = Cooldown Penutupan Posisi
trader-timing-close-cooldown-hint = Menit menunggu sebelum membuka kembali token yang sama
trader-timing-concurrency = Konkurensi Pemeriksaan Entry
trader-timing-concurrency-hint = Jumlah token yang diperiksa bersamaan (makin tinggi = makin cepat tetapi lebih banyak CPU)
trader-timing-unit-minutes = mnt
trader-timing-unit-tokens = token
trader-timing-intervals = Interval Monitor
trader-timing-intervals-badge = Hanya baca
trader-timing-intervals-hint = Dikonfigurasi di kode (tidak dapat diubah lewat UI)
trader-timing-intervals-value = <strong>Monitor Entry:</strong> 30 dtk | <strong>Monitor Exit:</strong> 5 dtk
