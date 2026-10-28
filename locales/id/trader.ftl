# Trader page labels.

trader-tab-stats = Statistik
trader-tab-strategies = Strategi
trader-tab-stop-loss = Stop Loss
trader-tab-trailing-stop = Trailing Stop
trader-tab-take-profit = Take Profit
trader-tab-time-rules = Exit Waktu
trader-tab-dca = DCA
trader-tab-settings = Pengaturan

trader-feature-coming-soon = Segera Hadir
    .message = Fitur ini segera hadir dan belum tersedia.
trader-feature-beta = Beta
trader-feature-disabled = Nonaktif
    .message = Fitur ini sedang dinonaktifkan.

trader-status-title = Auto Trader
trader-status-running = Berjalan
trader-status-stopped = Berhenti
trader-status-setup-required = Perlu penyiapan
trader-setup-action = Selesaikan penyiapan
trader-toggle-start-failed = Gagal memulai trader
trader-toggle-stop-failed = Gagal menghentikan trader
trader-halt-title = Trading dihentikan
trader-halt-reason-default = Penghentian paksa manual
trader-halt-resume = Lanjutkan
trader-monitor-entry = Monitor Entry
trader-monitor-exit = Monitor Exit
trader-loss-limit-title = Batas Kerugian Periode
trader-loss-limit-resume = Lanjutkan trading
trader-loss-limit-reset = Reset periode
trader-loss-limit-off = Nonaktif
trader-loss-limit-resets-in = Direset dalam { $hours } { $minutes }
trader-force-stop = Hentikan Paksa Semuanya
trader-force-stop-hint = Menghentikan semua monitor, tugas salin, dan trade sekaligus.
trader-force-stop-idle = Tidak ada yang berjalan untuk dihentikan oleh hentian paksa.
trader-run-state-force-stopped = Dihentikan paksa
trader-run-state-waiting = Menunggu layanan
trader-run-state-idle = Monitor mati
trader-entries-label = Entry baru
trader-entries-open = Terbuka
trader-entries-monitor-off = Monitor Entry mati
trader-entries-no-live-strategy = Tidak ada strategi entry live
trader-entries-no-paper-strategy = Tidak ada strategi entry paper
trader-action-start = Mulai Auto Trader
trader-action-stop = Hentikan Auto Trader
trader-notice-loss-limit = Batas kerugian tercapai
trader-notice-loss-limit-message = Entry live baru dijeda sampai periode direset. Exit tetap berjalan.
trader-notice-stats-failed = Statistik tidak dapat dimuat
trader-controls-group =
    .aria-label = Kontrol trading
trader-monitor-dormant = Mulai bersama Auto Trader

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

trader-stats-window =
    .aria-label = Jendela statistik
trader-stats-book =
    .aria-label = Buku trading
trader-exposure-cash = Kas
trader-exposure-open = Terbuka
trader-exposure-unrealized = Belum terealisasi
trader-exposure-open-title = Posisi yang memakai slot, dari batas posisi terbuka
trader-exposure-in-use = Terpakai
trader-exposure-in-use-title = { -sol } yang terpakai di posisi terbuka
# $amount is the formatted count.
trader-exposure-unpriced =
    { $count ->
       *[other] { $amount } posisi terbuka belum punya harga live
    }
# $date is a formatted date; $amount is a formatted SOL amount.
trader-exposure-cash-title = Putaran paper sejak { $date } · dimulai dengan { $amount }
trader-paper-reset = Atur ulang putaran paper
trader-paper-reset-title = Atur ulang putaran paper?
trader-paper-reset-run-message = Setiap posisi paper yang terbuka, termasuk kepemilikan paper tugas salin, ditutup pada harga saat ini dan putaran baru dimulai dengan saldo awal yang diatur. Putaran sebelumnya tetap ada di riwayat.
trader-paper-reset-done = Putaran paper baru dimulai
trader-paper-reset-failed = Putaran paper tidak dapat diatur ulang
trader-realized-title = Performa Terealisasi
trader-metric-net-pnl = P&L Bersih
trader-metric-win-rate = Win Rate
trader-metric-profit-factor = Profit Factor
trader-metric-max-drawdown = Drawdown Maks
trader-metric-avg-win-loss = Rata-rata Menang / Kalah
trader-metric-closed-trades = Trade Tertutup
trader-metric-median-hold = Median Durasi Hold
trader-metric-fees = Biaya Jaringan
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
trader-stats-excluded =
    { $count ->
       *[other] { $amount } trade tertutup tidak dihitung: harga pokoknya tidak lengkap, jadi P&L-nya tidak diketahui.
    }
# $ratio is a formatted ratio.
trader-stats-payoff = Payoff { $ratio }
trader-stats-streak-win =
    { $count ->
       *[other] { $amount } menang beruntun
    }
trader-stats-streak-loss =
    { $count ->
       *[other] { $amount } kalah beruntun
    }
# $wins and $losses are formatted durations.
trader-stats-hold-split = Menang { $wins } · kalah { $losses }
trader-stats-fees-basis = Termasuk dalam P&L bersih
trader-stats-loading = Memuat statistik...
trader-open-strategies = Buka Strategi

trader-daily-title = P&L Harian
trader-daily-subtitle = { -sol } terealisasi per hari, beserta total berjalan
trader-daily-chart = Untung rugi terealisasi harian dalam { -sol }
trader-weekly-title = P&L Mingguan
trader-weekly-subtitle = { -sol } terealisasi per minggu ISO, beserta total berjalan
trader-weekly-chart = Untung rugi terealisasi mingguan dalam { -sol }
# $date is a formatted date; $pnl a signed SOL amount.
trader-daily-bar =
    { $count ->
       *[other] { $date } · { $pnl } · { $amount } trade
    }
trader-weekly-bar =
    { $count ->
       *[other] Minggu { $date } · { $pnl } · { $amount } trade
    }
trader-extreme-best = Trade terbaik
trader-extreme-worst = Trade terburuk
trader-extreme-runs = Terbanyak berturut-turut
trader-outcome-title = Hasil Trade
trader-outcome-subtitle = Trade tertutup menurut return, dalam persen
# $value, $from and $to are formatted signed percentages.
trader-outcome-below = Di bawah { $value }
trader-outcome-range = { $from } sampai { $to }
trader-outcome-above = { $value } atau lebih
# $range is one of the three messages above; $pnl a signed SOL amount.
trader-outcome-bucket =
    { $count ->
       *[other] { $range } · { $amount } trade · { $pnl }
    }

trader-stats-empty-day = Tidak ada trade tertutup dalam 24 jam terakhir
trader-stats-empty-all = Belum ada trade tertutup
trader-stats-empty-hint = Trade tertutup muncul di sini setelah posisi terjual. Mulai Auto Trader, atau pilih periode yang lebih panjang.
trader-stats-empty-days =
    { $count ->
       *[other] Tidak ada trade tertutup dalam { $amount } hari terakhir
    }
trader-exits-title = Alasan Exit
trader-exits-subtitle = Bagaimana trade tertutup berakhir, dan hasil tiap exit
trader-sources-title = Sumber Entry
trader-sources-subtitle = Apa yang membuka trade tertutup
trader-recent-title = Penutupan Terbaru
trader-recent-subtitle = Trade tertutup terbaru di jendela ini
trader-column-token = Token
trader-column-exit = Exit
trader-column-source = Sumber
trader-column-trades = Trade
trader-column-win-rate = Win Rate
trader-column-avg-return = Rata-rata Return
trader-column-net-pnl = P&L Bersih
trader-column-held = Ditahan
trader-column-closed = Ditutup
trader-column-pnl = P&L
trader-column-return = Return

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
trader-stop-loss-impact = Exit saat turun { $threshold }% dari entry
trader-stop-loss-hold-immediate = Segera
trader-stop-loss-hold-delay = Jeda { $span }
trader-stop-loss-price-falls = Harga Turun
trader-stop-loss-threshold-reached = Ambang tercapai
trader-stop-loss-sell-part = Jual { $part }
trader-stop-loss-part-of-position = Sebagian posisi
trader-stop-loss-sell-rest = Jual Sisanya
trader-stop-loss-still-below = Masih di bawah stop
trader-stop-loss-summary = Kerugian dibatasi hingga <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Catatan:</strong> Stop loss melindungi dari kerugian lebih besar dengan exit lebih awal

trader-trailing-title = Trailing Stop
trader-trailing-subtitle = Lindungi keuntungan secara otomatis dengan mengikuti harga saat naik
trader-trailing-activation-impact = Mulai mengikuti pada untung +{ $value }%
trader-trailing-distance-impact = Exit pada -{ $value }% dari puncak
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = Minimum terkunci saat aktif: { $value }
trader-trailing-locked-minimum-warning = Stop berada di harga masuk atau di bawahnya saat aktif; stop hanya menjual saat untung. Aktifkan stop impas untuk batas bawah.
trader-trailing-activation = Aktivasi
trader-trailing-peak = Puncak
trader-trailing-final = +{ $value }% akhir
trader-trailing-summary-protected = Melindungi untung <strong>{ $value }</strong>
trader-trailing-summary-avoided = Menghindari rugi <strong>{ $value }</strong> dari puncak
# $step is the band's position in the list, from 1.
trader-trailing-band = Pita { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-profit-label = Profit puncak pita { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-distance-label = Jarak trailing pita { $step }
trader-trailing-band-profit-caption = Dari profit puncak
trader-trailing-band-distance-caption = Ikuti sejauh
trader-trailing-add-band = Tambah pita
# $step is the band's position in the list, from 1.
trader-trailing-remove-band = Hapus pita { $step }
# $min and $max are the configured bounds of a band's peak profit.
trader-trailing-band-error-profit-range = Profit puncak setiap pita harus antara { $min }% dan { $max }%
# $min and $max are the configured bounds of a band's distance.
trader-trailing-band-error-distance-range = Jarak setiap pita harus antara { $min }% dan { $max }%
trader-trailing-band-error-order = Setiap pita membutuhkan profit puncak lebih tinggi dari aktivasi dan pita sebelumnya
trader-trailing-band-error-distance = Setiap pita harus mengikuti dengan jarak lebih kecil dari profit puncaknya

## Break-even stop

trader-break-even-title = Stop Impas
trader-break-even-subtitle = Setelah posisi masuk untung, jual semuanya sebelum berubah menjadi rugi
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = Begitu harga terbaik sejak pembelian terakhir mencapai { $trigger }, turun kembali ke { $offset } menjual semuanya.

trader-take-profit-title = Take Profit
trader-take-profit-subtitle = Menjual posisi secara bertahap saat profit mencapai setiap level
trader-take-profit-initial-buy = Pembelian awal
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = Tahap { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = Level profit tahap { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = Porsi jual tahap { $step }
trader-take-profit-profit-caption = Level profit
trader-take-profit-sell-caption = Porsi jual
trader-take-profit-add-step = Tambah tahap
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = Hapus tahap { $step }
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = Setiap level profit harus antara { $min }% dan { $max }%
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = Setiap porsi harus antara { $min }% dan { $max }%
trader-take-profit-error-order = Setiap tahap perlu level profit lebih tinggi dari tahap sebelumnya
# $max is the largest total share, the whole position.
trader-take-profit-error-total = Semua tahap bersama-sama dapat menjual paling banyak { $max }%
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = { $runner }% posisi tetap terbuka sebagai runner
trader-take-profit-runner-none = Tahap terakhir menjual sisa posisi
trader-take-profit-runner-warning = Dengan trailing stop, stop impas, kedua exit waktu, dan semua strategi exit nonaktif, runner hanya ditutup oleh stop loss atau secara manual
trader-take-profit-info = Porsi dihitung dari posisi awal. Jika harga melewati beberapa level sekaligus, satu tahap dijual per pemeriksaan.
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = Pada +{ $profit }%, jual { $share }%
trader-take-profit-example-runner = Runner
trader-take-profit-example-runner-detail = Diserahkan ke aturan keluar lainnya
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = Menjual <strong>{ $sold }%</strong> hingga profit +{ $profit }%

trader-time-loss-title = Tutup Posisi Rugi
trader-time-subtitle = Exit posisi secara otomatis setelah durasi hold maksimum jika kerugian melebihi ambang
trader-time-flat-title = Tutup Posisi Stagnan
trader-time-flat-subtitle = Bebaskan modal dari posisi yang tidak bergerak naik maupun turun
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = Posisi yang ditahan { $span } atau lebih dengan P&L dalam ±{ $band } ditutup.
trader-time-unit-seconds = detik
trader-time-unit-minutes = menit
trader-time-unit-hours = jam
trader-time-unit-days = hari
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
trader-time-loss-hint = Kerugian saat exit waktu menjual setelah waktu tahan berlalu (40 = exit di -40%)
trader-time-day = Hari { $day }
trader-time-position-opened = Posisi dibuka
trader-time-limit = Batas Waktu
trader-time-hold-reached = Masa hold tercapai
trader-time-loss-met = Ambang kerugian terpenuhi
trader-time-note = <strong>Catatan:</strong> Posisi yang untung atau rugi lebih kecil TIDAK akan di-exit
trader-time-positions-title = Status Posisi Saat Ini
trader-time-positions-loading = Memuat posisi...
trader-time-positions-empty = Tidak ada posisi terbuka
trader-time-positions-empty-message = Posisi terbuka muncul di sini dengan waktu tahan dan ROI terhadap batas waktu.
trader-time-positions-token = Token
trader-time-positions-hold = Durasi Hold
trader-time-positions-roi = ROI
trader-time-positions-rule = Exit Waktu
trader-time-status-off = Exit waktu nonaktif
trader-time-status-none = Saat ini tidak ada exit waktu yang berlaku
trader-time-status-not-managed = Tidak dikelola Auto Trader
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = Rugi { $pnl }; ditutup dalam { $remaining }
trader-time-status-losing-due = Rugi { $pnl }; ditutup pada pemeriksaan berikutnya
# $held and $remaining are formatted durations.
trader-time-status-flat = Stagnan selama { $held }; ditutup dalam { $remaining }
trader-time-status-flat-due = Stagnan selama { $held }; ditutup pada pemeriksaan berikutnya

trader-dca-title = Dollar-Cost Averaging
trader-dca-subtitle = Tambah otomatis ke posisi yang rugi untuk menurunkan harga entry rata-rata Anda
trader-dca-measure-average = Rata-rata masuk
trader-dca-measure-first-entry = Masuk pertama
trader-dca-step = Tambahan { $step }
trader-dca-drop-label = Penurunan harga tambahan { $step }
trader-dca-size-label = Ukuran tambahan { $step }, kali pembelian pertama
trader-dca-drop-caption = Penurunan Harga
trader-dca-size-caption = Beli × Pembelian Pertama
trader-dca-add-step = Tambah level
trader-dca-remove-step = Hapus tambahan { $step }
trader-dca-step-amount = = { $amount }
trader-dca-step-below-minimum = Di bawah transaksi minimum { $minimum }: tambahan ini tidak dikirim
trader-dca-error-drop-range = Setiap penurunan harus antara { $min }% dan { $max }%
trader-dca-error-size-range = Setiap ukuran harus antara { $min }× dan { $max }× pembelian pertama
trader-dca-error-order = Diukur dari masuk pertama, setiap tambahan perlu penurunan lebih dalam dari sebelumnya
trader-dca-cooldown-impact = Minimal { $span } di antara entry tambahan
trader-dca-min-hold-none = Tambahan pertama boleh langsung setelah masuk
trader-dca-min-hold-impact = Tambahan pertama menunggu setidaknya { $span } setelah masuk
trader-dca-capacity = Paling banyak yang bisa dipegang posisi ini: { $amount } ({ $multiple }× pembelian pertama)
trader-dca-info-title = Info Strategi DCA
trader-dca-info-subtitle = Pertimbangan penting untuk trading DCA
trader-dca-how-title = Cara Kerja DCA
trader-dca-how-trigger = <strong>Pemicu:</strong> harga turun di bawah level berikutnya, diukur dari rata-rata masuk atau masuk pertama
trader-dca-how-action = <strong>Aksi:</strong> Tambah lebih banyak { -sol } untuk menurunkan harga pokok rata-rata
trader-dca-how-repeat = <strong>Tangga:</strong> setiap tambahan pindah ke level berikutnya; tambahan berhenti setelah semua level terpakai
trader-dca-how-pause = <strong>Jeda:</strong> tidak ada tambahan selama yang sebelumnya masih diselesaikan, atau selama batas kerugian memblokir entri baru
trader-dca-risk-title = Peringatan Risiko
trader-dca-risk-exposure = <strong>Eksposur Meningkat:</strong> DCA menambah total modal yang berisiko per posisi
trader-dca-risk-knife = <strong>Falling Knife:</strong> DCA tidak membantu jika token terus turun
trader-dca-risk-cooldown = <strong>Cooldown:</strong> Gunakan cooldown untuk menghindari entry DCA beruntun

trader-sizing-title = Ukuran Posisi
trader-sizing-subtitle = Atur berapa banyak yang diinvestasikan per posisi
trader-timing-title = Waktu & Cooldown
trader-timing-subtitle = Atur jeda waktu antar operasi
trader-timing-close-cooldown = Cooldown Penutupan Posisi
trader-timing-close-cooldown-hint = Menit menunggu sebelum membuka kembali token yang sama
trader-timing-concurrency = Konkurensi Pemeriksaan Entry
trader-timing-concurrency-hint = Jumlah token yang diperiksa bersamaan (makin tinggi = makin cepat tetapi lebih banyak CPU)
trader-timing-unit-minutes = mnt
trader-timing-unit-tokens = token

## Service mode notice

trader-service-notice-title = Berjalan di layanan { -brand }
trader-service-notice-message = Harga diperbarui setiap beberapa detik, jadi exit cepat seperti trailing stop yang ketat bereaksi lebih lambat dibanding dengan RPC Anda sendiri.
