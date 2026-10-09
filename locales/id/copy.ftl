copy-skip-not-buy-swap = Aktivitas dompet bukan pembelian
copy-skip-task-disabled = Tugas dijeda
copy-skip-mode-transition-required = Mode eksekusi harus diubah secara terpisah
copy-skip-live-confirmation-required = Eksekusi live memerlukan konfirmasi
copy-skip-unsupported-sizing-mode = Mode ukuran belum didukung
copy-skip-self-copy = Dompet ini milik Anda sendiri
copy-skip-target-below-minimum = Trade dompet di bawah minimum
copy-skip-target-above-maximum = Trade dompet di atas maksimum
copy-skip-already-bought = Token ini sudah dibeli (beli sekali)
copy-skip-blacklisted = Token diblokir oleh kontrol risiko
copy-skip-filter-required = Token tidak lolos Pemfilteran
copy-skip-budget-exhausted = Anggaran tugas habis
copy-skip-token-cap-reached = Batas per token tercapai
copy-skip-below-minimum-size = Ukuran salinan terlalu kecil
copy-skip-invalid-sizing = Ukuran tugas tidak valid
copy-skip-invalid-slippage = Slippage tugas tidak valid
copy-skip-invalid-exit-policy = Aturan exit tugas tidak valid
copy-skip-invalid-price = Tidak ada harga pasar yang dapat digunakan
copy-skip-not-sell-swap = Aktivitas dompet bukan penjualan
copy-skip-exit-mode-disabled = Penjualan dompet diabaikan: tugas menjual berdasarkan aturannya sendiri
copy-skip-force-stopped = Trading dihentikan paksa
copy-skip-copy-position-not-found = Tidak ada posisi milik tugas ini
copy-skip-position-user-only = Posisi dikelola oleh Anda
copy-skip-position-management-mismatch = Posisi tidak lagi mengikuti penjualan salinan
copy-skip-latency-kill-switch = Dijeda otomatis: trade terdeteksi terlalu lambat
copy-skip-claim-reconciled-abandoned = Pengiriman live yang terputus ditutup tanpa percobaan ulang
copy-skip-stale-observation = Diputar ulang setelah downtime, terlalu lama untuk disalin
copy-skip-unknown-observation-time = Trade yang diputar ulang tidak memiliki waktu blok
copy-skip-entry-blocked = Entry diblokir

copy-entry-block-force-stopped = Trading dihentikan paksa
copy-entry-block-loss-limit = Batas kerugian memblokir entry baru
copy-entry-block-connectivity = Layanan yang diperlukan tidak tersedia
copy-entry-block-position-limit = Batas posisi terbuka tercapai
copy-entry-block-already-open = Posisi sudah terbuka
copy-entry-block-reentry-cooldown = Cooldown entry ulang token
copy-entry-block-open-cooldown = Cooldown entry global
copy-entry-block-entry-reserved = Entry lain sedang diproses
copy-entry-block-blacklisted = Token diblokir oleh kontrol risiko
copy-entry-block-check-failed = Pemeriksaan keamanan tidak dapat diselesaikan

copy-pause-user = Dijeda oleh Anda
copy-pause-latency-kill-switch = Dijeda otomatis: trade tiba terlambat rata-rata { $average }d (batas { $threshold }d)
copy-pause-watch-detached = Dijeda otomatis: dompet tidak lagi dipantau
copy-pause-watch-budget-exceeded = Dijeda: dompet ini mencapai batas pemeriksaan pantau { $limit } signature sebelum berhasil menyusul
copy-pause-helius-unavailable = Dijeda: pemeriksaan dompet { -helius } gagal
copy-pause-watch-processing-failed = Dijeda: aktivitas dompet tidak dapat diproses
copy-pause-unspecified = Dijeda

copy-pause-short-user = oleh Anda
copy-pause-short-latency-kill-switch = terlalu lambat
copy-pause-short-watch-detached = pantauan hilang
copy-pause-short-watch-budget-exceeded = batas pantau
copy-pause-short-helius-unavailable = penyedia pantau
copy-pause-short-watch-processing-failed = pemrosesan pantau
copy-state-paused = Dijeda
copy-state-paused-reason = Dijeda · { $reason }

copy-readiness-history = Riwayat paper
copy-readiness-history-met =
    { $count ->
       *[other] { $count } putaran paper tertutup, dibutuhkan { $needed }
    }
copy-readiness-history-short = { $count } dari { $needed } putaran paper tertutup
copy-readiness-profit = Menguntungkan di paper
copy-readiness-profit-detail =
    { $count ->
       *[other] { $realized } { -sol } terealisasi dalam { $count } putaran, { $wins } menang
    }
copy-readiness-latency = Trade terdeteksi tepat waktu
copy-readiness-latency-detail = kedatangan p95 { $p95 }d, batas { $limit }d
copy-readiness-latency-none = Belum ada sampel kedatangan
copy-readiness-priced = Setiap kepemilikan memiliki harga
copy-readiness-priced-ok = Setiap kepemilikan paper yang terbuka memiliki harga pool
copy-readiness-priced-missing =
    { $count ->
       *[other] { $count } kepemilikan terbuka tanpa harga pool
    }
copy-readiness-runtime = Eksekusi live tersedia
copy-readiness-runtime-ok = Penyiapan dan gerbang keamanan mengizinkan salinan live

copy-live-block-setup-incomplete = Selesaikan penyiapan dompet dan RPC terlebih dahulu
copy-live-block-force-stop = Penghentian darurat sedang aktif
copy-live-block-copy-trading-disabled = Pemrosesan salinan dijeda secara global
copy-live-block-unavailable = Eksekusi live tidak tersedia

## Task state, mode and exit labels.

copy-state-system-paused = Dijeda global
copy-state-force-stopped = Dihentikan paksa
copy-state-entries-blocked = Entry diblokir
copy-state-running-live = Berjalan
copy-state-running-paper = Berjalan
copy-mode-paper = Paper
copy-mode-live = Live
copy-exit-mode-buy-only = Aturan exit saya
copy-exit-mode-mirror = Tiru penjualan dompet
copy-exit-mode-hybrid = Penjualan dompet dan aturan saya
copy-exit-target-sell = Dompet menjual
copy-exit-stop-loss = Stop loss
copy-exit-trailing-stop = Trailing stop
copy-exit-take-profit = Take profit
copy-exit-time-override = Aturan waktu
copy-exit-manual = Ditutup manual

## Shared wording

copy-request-failed = Permintaan gagal
copy-keep-paused = Tetap dijeda
copy-paused-suffix = · dijeda
copy-mode-paused = { $mode } · dijeda
copy-task-ref = “{ $name }” ({ $mode })
copy-metric-realized-pnl = P&L terealisasi
copy-metric-unrealized-pnl = P&L belum terealisasi
copy-metric-win-rate = Win rate
copy-metric-budget-spent = Anggaran terpakai
copy-metric-median-arrival = Median kedatangan
copy-metric-open-holdings = Kepemilikan terbuka
copy-record-won-lost = { $won } menang · { $lost } kalah
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = Fill
copy-kind-exits = Exit
copy-kind-skips = Dilewati
copy-kind-errors = Error
copy-field-per-trade-cap = Batas per trade
copy-field-per-token-cap = Batas per token
copy-field-total-budget = Total anggaran
copy-field-slippage = Slippage
copy-rules-wallet-sells-only = Hanya penjualan dompet
copy-filter-copy-setting-required = Pengaturan salin (wajib)
copy-filter-copy-setting-not-required = Pengaturan salin (tidak wajib)
copy-count-closed-rounds =
    { $count ->
       *[other] { $count } putaran tertutup
    }
copy-count-open-holdings =
    { $count ->
       *[other] { $count } kepemilikan terbuka
    }
copy-unrealized-partial =
    { $priced ->
       *[other] { $priced } kepemilikan ada harga · { $unpriced } tanpa harga
    }
copy-unrealized-unpriced =
    { $count ->
       *[other] { $count } kepemilikan tanpa harga
    }
copy-range-24h = 24j
copy-range-7d = 7h
copy-range-30d = 30h
copy-range-all = Semua
copy-range-label =
    .aria-label = Rentang tanggal

## Page strip

copy-page-title = Copy Trading
copy-page-beta = Beta
copy-strip-loading = Memuat
copy-strip-unavailable = Tidak tersedia
copy-strip-setup-required = Penyiapan diperlukan · copy trading memerlukan dompet dan RPC
copy-strip-pause-all = Jeda semua
copy-strip-resume = Lanjutkan pemrosesan
copy-strip-settings = Pengaturan
copy-strip-add-wallet = Tambah dompet
copy-strip-paused-globally = Dijeda global · tidak ada salinan baru, exit tetap berjalan
copy-strip-force-stopped = Dihentikan paksa · tidak ada yang disalin
copy-strip-loss-limit = Batas kerugian · entry baru diblokir, exit tetap berjalan
copy-strip-idle-paused =
    { $count ->
       *[other] Idle · { $count } tugas dijeda
    }
copy-strip-idle-empty = Idle · belum ada tugas
copy-strip-processing = Memproses · { $paper } paper
copy-strip-processing-live = Memproses · { $live } live · { $paper } paper
copy-figures-label =
    .aria-label = Total copy trading
copy-figure-marked-at-pool = Dinilai pada harga pool
copy-figure-across-tasks = Di semua tugas
copy-figure-budget-lifetime = Total pengeluaran tugas aktif
copy-figure-budget-none = Tidak ada tugas aktif
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
       *[other] { $count } trade
    }
copy-figure-arrival-none = Tidak ada sampel dari tugas aktif

## Page frame

copy-load-failed = Copy trading tidak dapat dimuat: { $error }
copy-resume-all-title = Lanjutkan pemrosesan salinan
copy-resume-all-message =
    { $count ->
       *[other] { $count } tugas live akan mengirim swap sungguhan saat dompetnya bertransaksi lagi.
    }
copy-toast-resumed-all = Pemrosesan salinan dilanjutkan
copy-toast-paused-all = Semua pemrosesan salinan dijeda
copy-toast-global-failed = Pemrosesan salinan tidak dapat diubah

## Onboarding

copy-onboarding-title = Salin dompet yang Anda percaya, buktikan dulu di Paper
copy-onboarding-body = Setiap tugas dimulai di Paper: trade target disimulasikan pada harga pool dengan slippage dan biaya Anda, dan aturan exit Anda berjalan pada buku paper. Aktifkan Live per dompet setelah hasil paper-nya memenuhi syarat.
copy-onboarding-add = Tambahkan dompet pertama Anda
copy-setup-gate-title = Copy trading memerlukan dompet
copy-onboarding-observe = Amati
copy-onboarding-observe-detail = Deteksi swap dompet tanpa mengeluarkan { -sol }.
copy-onboarding-evaluate = Evaluasi
copy-onboarding-evaluate-detail = Baca P&L paper, win rate, trade yang dilewati, kecepatan deteksi, dan slippage.
copy-onboarding-arm = Aktifkan
copy-onboarding-arm-detail = Lolos pemeriksaan kesiapan, lalu aktifkan swap sungguhan.

## Wallet list

copy-list-label =
    .aria-label = Dompet yang disalin
copy-list-title = Dompet
copy-list-compare = Bandingkan
copy-list-sort-label = Urutkan dompet
copy-list-count = { $active } aktif · { $total } total
copy-sort-pnl = P&L
copy-sort-state = Status
copy-sort-name = Nama
copy-compare-label =
    .aria-label = Bandingkan dompet

## Dialog chrome

copy-dialog-close =
    .aria-label = Tutup
copy-editor-title-add = Tambah dompet
copy-editor-sub-add = Tugas baru dimulai di Paper
copy-arm-title = Aktifkan live copying
copy-arm-sub = Swap sungguhan dari dompet Anda
copy-arm-keep-paper = Tetap di Paper
copy-arm-confirm = Aktifkan live
copy-profile-title = Profil dompet
copy-profile-sub = Yang telah dilihat bot ini dari dompet tersebut

## Settings dialog

copy-settings-title = Pengaturan copy trading
copy-settings-subtitle = Kebijakan global untuk setiap tugas
copy-settings-filter-warning = Dengan pengaturan Pemfilteran default, ini menolak hampir semua token sehingga tidak ada yang disalin. Biarkan nonaktif kecuali filter Anda meloloskan token yang diperdagangkan dompet Anda.
copy-settings-unit-seconds = dtk
copy-settings-unit-trades = trade
copy-settings-unit-tasks = tugas
copy-settings-unit-rounds = ronde
copy-settings-save = Simpan pengaturan
copy-settings-load-failed = Pengaturan salin tidak dapat dimuat
copy-settings-saved = Pengaturan copy trading disimpan

## Workspace

copy-tab-overview = Ringkasan
copy-tab-holdings = Kepemilikan
copy-tab-activity = Aktivitas
copy-tab-rules = Aturan
copy-tab-execution = Eksekusi
copy-tabs-label = Tampilan tugas
copy-workspace-select = Pilih dompet untuk membuka ruang kerjanya.
copy-workspace-loading = Memuat tugas…
copy-workspace-load-failed = Tugas ini tidak dapat dimuat: { $error }

copy-state-detail-paper = Berjalan di Paper · trade disimulasikan, tidak ada dana yang dikeluarkan
copy-state-detail-live = Berjalan live · trade dompet disalin dengan swap sungguhan
copy-state-detail-system-paused = Menunggu · pemrosesan salinan dijeda global, exit tetap berjalan
copy-state-detail-entries-blocked = Entry diblokir oleh batas kerugian · exit tetap berjalan
copy-state-detail-force-stopped = Dihentikan paksa · tidak ada yang disalin

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = Melanjutkan mempertahankan batas yang sama, sehingga tugas dijeda lagi selama trade masih tiba terlambat. Periksa aliran RPC atau naikkan batas kedatangan di Pengaturan.
copy-paused-resume-detached = Melanjutkan akan memantau dompet lagi.
copy-paused-holdings-rules =
    { $count ->
       *[other] Aturan exit-nya tetap menutup { $count } kepemilikan terbukanya.
    }
copy-paused-holdings-mirror =
    { $count ->
       *[other] Penjualan dompet tetap menutup { $count } kepemilikan terbukanya.
    }
copy-paused-holdings-hybrid =
    { $count ->
       *[other] Penjualan dompet dan aturan exit-nya tetap menutup { $count } kepemilikan terbukanya.
    }

copy-watch-state-catching-up = Pantauan dompet: Menyusul. Memeriksa dompet ini melalui { -helius }.
copy-watch-state-watching = Pantauan dompet: Memantau. Memeriksa dompet ini melalui { -helius }.
copy-watch-last-check = Pemeriksaan terakhir { $ago }.
copy-watch-recovery-active = Pantauan dompet aktif
copy-watch-recovery-catching-up = Pantauan dompet sedang menyusul
copy-watch-recovery-still-paused = Tugas salin masih dijeda. Lanjutkan penyalinan saat Anda siap.
copy-watch-recovery-title = Pulihkan pantauan dompet
copy-watch-recovery-processing-failed = Aktivitas dompet tidak dapat diproses. Progres tersimpan dipertahankan. Coba lagi setelah masalah teratasi.
copy-watch-recovery-provider-failed = Pemeriksaan { -helius } gagal. Progres tersimpan dipertahankan. Coba lagi saat penyedia tersedia.
copy-watch-recovery-budget-intro = Dompet ini memiliki aktivitas lebih banyak daripada yang dapat diperiksa pantauan saat ini. Pilih cara melanjutkan.
copy-watch-approve = Coba menyusul menggunakan { -helius }
copy-watch-approve-help = Melanjutkan dari progres tersimpan. Dapat menggunakan lebih banyak kredit { -helius } dan tetap bisa tertinggal.
copy-watch-approve-unavailable = Penyusulan { -helius } tidak tersedia. Konfigurasikan endpoint RPC { -helius } yang aktif untuk melanjutkan tanpa melewatkan aktivitas yang belum diperiksa.
copy-watch-no-provider = Tidak ada penyedia penyusulan yang didukung untuk pantauan ini.
copy-watch-budget-label = Signature diperiksa per pemeriksaan
copy-watch-budget-hint = Atau lewati aktivitas yang belum diperiksa dan lanjutkan dari sekarang. Pilih { $min }–{ $max } signature per pemeriksaan; batas lebih tinggi dapat menggunakan lebih banyak panggilan RPC.
copy-watch-ack = Saya memahami aktivitas yang terlewat tidak akan disalin.
copy-watch-toast-range = Pilih antara { $min } dan { $max } signature per polling dengan kelipatan { $step } signature
copy-watch-toast-ack = Konfirmasikan bahwa signature sejak pemeriksaan terakhir yang selesai akan dilewati
copy-watch-resumed = Pantauan dompet dilanjutkan dari sekarang; tugas salin tetap dijeda
copy-watch-resume-failed = Pantauan dompet tidak dapat dilanjutkan
copy-watch-retry-started = Percobaan ulang pantauan dompet dimulai dari progres tersimpan; tugas salin tetap dijeda
copy-watch-retry-failed = Pantauan dompet tidak dapat dicoba ulang
copy-watch-approve-title = Izinkan penyusulan { -helius } untuk dompet ini
copy-watch-approve-message = { -helius } dapat memeriksa transaksi Solana yang berhasil dari progres tersimpan tanpa melewatkan interval yang belum diperiksa. Saat ini biayanya 10 kredit per 100 transaksi lengkap yang dikembalikan, dibulatkan ke atas, dengan minimum 10 kredit per permintaan. Satu pemeriksaan dapat membuat beberapa permintaan; penggunaan dan harga penyedia dapat berbeda. Penyalinan tetap dijeda sampai Anda melanjutkannya secara terpisah.
copy-watch-approve-confirm = Izinkan untuk dompet ini
copy-watch-approved = Pantauan dompet dimulai dari progres tersimpan; tugas salin tetap dijeda
copy-watch-restore-failed = Pantauan dompet tidak dapat dipulihkan

copy-action-pause = Jeda
copy-action-resume = Lanjutkan
copy-action-resume-copy = Lanjutkan salin
copy-action-resume-from-now = Lanjutkan dari sekarang
copy-action-retry-watch = Coba lagi pantauan dompet
copy-action-return-paper = Kembali ke Paper
copy-action-edit-rules = Ubah aturan
copy-action-clone = Duplikat
copy-action-profile = Profil dompet
copy-resume-live-title = Lanjutkan live copying
copy-resume-live-message = “{ $name }” akan mengirim swap sungguhan dari dompet Anda saat dompet ini bertransaksi lagi.
copy-resume-live-confirm = Lanjutkan live
copy-task-resumed = Tugas dilanjutkan
copy-task-paused = Tugas dijeda
copy-task-state-failed = Status tugas tidak dapat diubah
copy-return-paper-message = Salinan baru oleh “{ $name }” akan disimulasikan lagi, tanpa mengeluarkan { -sol }.
copy-return-paper-cancel = Tetap live
copy-task-returned-paper = Tugas dikembalikan ke Paper
copy-mode-change-failed = Mode eksekusi tidak dapat diubah
copy-delete-title = Hapus tugas salin
copy-delete-message = Hapus “{ $name }”? Keputusan dan hasil paper-nya dihapus dan dompet tidak lagi dipantau untuk tugas ini.
copy-delete-confirm = Hapus tugas
copy-delete-cancel = Simpan tugas
copy-task-deleted = Tugas salin dihapus
copy-task-delete-failed = Tugas salin tidak dapat dihapus

## Overview tab

copy-overview-results = Hasil
copy-analytics-load-failed = Analitik tidak dapat dimuat: { $error }
copy-analytics-loading = Memuat analitik…
copy-exit-bucket =
    { $count ->
       *[other] { $count } penjualan · { $pnl }
    }
copy-overview-average-win = Rata-rata menang
copy-overview-average-loss = Rata-rata kalah { $amount }
copy-overview-profit-factor = Profit factor
copy-overview-profit-factor-note = Total menang ÷ total kalah
copy-overview-average-hold = Rata-rata hold
copy-overview-average-hold-note = Dari entry ke exit
copy-overview-best-round = Putaran terbaik
copy-overview-worst-round = Terburuk { $amount }
copy-overview-curve-title = P&L kumulatif
copy-overview-exits-title = Penjualan per exit
copy-overview-skips-title = Alasan trade dilewati
copy-book-title-live = Buku live
copy-book-title-paper = Buku paper
copy-book-all-time = Sepanjang waktu
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
       *[other] pembelian
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
       *[other] exit sesuai aturan Anda
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
       *[other] penjualan dompet
    }
copy-book-manual-closes = <strong>{ $count }</strong> ditutup manual
copy-book-skipped = <strong>{ $count }</strong> dilewati
copy-book-failed = <strong>{ $count }</strong> gagal
copy-book-closed = { $count } ditutup
copy-book-budget-note = Pengeluaran { $mode } dari { $total } · sisa { $remaining }
copy-check-passed = lolos
copy-check-not-passed = belum lolos
copy-readiness-title = Sebelum live
copy-readiness-live-note = Tugas ini trading live. Kembalikan ke Paper dari header di atas.
copy-readiness-all-pass = Semua pemeriksaan lolos.
copy-readiness-needs-review = Pengaktifan memerlukan tinjauan eksplisit atas hal yang belum siap.
copy-readiness-arm = Tinjau dan aktifkan live

## Rules tab and review

copy-rules-title = Aturan yang berlaku
copy-rules-size-ratio = { $pct } dari trade dompet
copy-rules-size-fixed = { $amount } per salinan
copy-rules-target-any = Ukuran berapa pun
copy-rules-target-min = Minimal { $amount }
copy-rules-target-max = Maksimal { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = Penggantian tugas · Trader { $value }
copy-rules-source-default = Default Trader
copy-rules-not-used = Tidak digunakan: penjualan dompet yang menentukan
copy-rules-col-rule = Aturan
copy-rules-col-applies = Berlaku
copy-rules-col-source = Sumber
copy-rules-budget-note = { $spent } terpakai di { $mode } · sisa { $remaining }
copy-rules-token-copies =
    { $count ->
       *[other] Sekitar { $count } salinan penuh untuk satu token
    }
copy-rules-sizing = Ukuran
copy-rules-copy-size = Ukuran salinan
copy-rules-entry-filters = Filter entry
copy-rules-target-size = Ukuran trade dompet
copy-rules-repeat-buys = Pembelian berulang
copy-rules-repeat-first-only = Hanya pembelian pertama setiap token
copy-rules-repeat-every = Setiap pembelian, hingga batas per token
copy-rules-filter-pass = Lolos Pemfilteran
copy-rules-filter-required = Wajib
copy-rules-filter-not-required = Tidak wajib
copy-rules-filter-task-override = Penggantian tugas
copy-rules-exits = Exit
copy-rules-exits-inactive = Kepemilikan hanya dijual saat dompet menjual; aturan di bawah tidak berjalan pada mode ini.

## Exit rules

copy-rule-status = Status
copy-rule-on = Aktif
copy-rule-off = Nonaktif
copy-rule-unit-seconds = dtk
copy-rule-unit-minutes = mnt
copy-rule-stop-loss-threshold = Menjual saat rugi sebesar
copy-rule-stop-loss-min-hold = Tidak sebelum ditahan selama
copy-rule-no-minimum = Tanpa minimum
copy-rule-partial-exits = Exit parsial
copy-rule-partial-allowed = Diizinkan
copy-rule-partial-full-only = Hanya exit penuh
copy-rule-partial-size = Ukuran exit parsial
copy-rule-trailing-activation = Aktif saat untung sebesar
copy-rule-trailing-distance = Menjual saat turun dari puncak sebesar
copy-rule-take-profit-target = Menjual saat untung sebesar
copy-rule-time-duration = Memeriksa setelah ditahan selama
copy-rule-time-threshold = Menjual selama P&L sama dengan atau di bawah
copy-preset-inherit = Default Trader
copy-preset-conservative = Konservatif
copy-preset-balanced = Seimbang
copy-preset-aggressive = Agresif
copy-preset-custom = Kustom
copy-validate-stop-loss = Stop loss harus di atas 0% dan maksimal 100%.
copy-validate-partial-size = Ukuran exit parsial harus antara 0% dan 100%.
copy-validate-min-hold = Hold minimum harus berupa bilangan bulat detik.
copy-validate-trailing-activation = Aktivasi trailing harus di atas 0% dan maksimal 100%.
copy-validate-trailing-distance = Jarak trailing harus di atas 0% dan maksimal 100%.
copy-validate-take-profit = Take profit harus di atas 0%.
copy-validate-time-duration = Aturan waktu memerlukan durasi di atas nol.
copy-validate-time-threshold = Ambang aturan waktu berupa kerugian: gunakan 0% atau angka negatif.
copy-warning-mirror = Hanya penjualan dompet yang menutup kepemilikan: tidak ada stop loss yang melindunginya, dan token yang tidak pernah dijual dompet tetap ditahan.
copy-warning-no-rules = Tidak ada aturan exit yang aktif dan penjualan dompet diabaikan: kepemilikan tidak pernah dijual.
copy-warning-no-stop-loss = Tidak ada stop loss yang berlaku: token yang turun ditahan sampai aturan lain atau dompet menjual.
copy-warning-stop-delay = Stop loss menunggu { $hold } setelah setiap pembelian: token yang turun lebih cepat akan tertutup jauh melewati { $threshold }.
copy-warning-take-profit-cost = Take profit di { $target } tidak menutupi biaya penjualan (slippage { $slippage } dan biaya swap { $fee }), sehingga putaran ditutup dengan rugi.
copy-warning-trailing-distance = Jarak trailing sama dengan atau lebih besar dari keuntungan aktivasinya, sehingga trail yang aktif dapat menjual di bawah harga entry.

## Execution tab

copy-execution-title = Kualitas eksekusi
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = Berapa pun
copy-execution-limit-on =
    { $count ->
       *[other] Dijeda jika rata-rata di atas { $limit } selama { $count } trade
    }
copy-execution-limit-off = Kill switch nonaktif
copy-execution-arrival-samples =
    { $count ->
       *[other] { $count } trade terlihat saat terjadi
    }
copy-execution-p95 = Kedatangan p95
copy-execution-median-slippage = Median slippage
copy-execution-slippage-samples =
    { $count ->
       *[other] { $count } fill terukur
    }
copy-execution-worst-slippage = Slippage terburuk
copy-execution-average-slippage = Rata-rata { $amount }
copy-execution-delay-title = Jeda deteksi
copy-execution-delay-note = Waktu dari blok dompet hingga bot ini melihat trade. Pemutaran ulang setelah downtime tidak dihitung.
copy-execution-delay-limit = Batang yang melewati batas kedatangan { $limit } berwarna kuning.
copy-execution-fastest = Tercepat
copy-execution-average = Rata-rata
copy-execution-slowest = Terlambat
copy-execution-fill-title = Fill dibandingkan dompet
copy-execution-fill-note = Positif berarti lebih buruk daripada dompet: membayar lebih saat beli, menerima lebih sedikit saat jual tiruan. Fill paper untuk token tanpa harga pool dihargai pada trade dompet itu sendiri, sehingga tidak mengukur apa pun dan tidak disertakan.
copy-execution-samples = Sampel
copy-execution-median = Median
copy-execution-worst = Terburuk
copy-execution-decisions = Keputusan dalam rentang

## Compare view

copy-compare-title = Bandingkan dompet
copy-compare-back = Kembali ke dompet
copy-compare-load-failed = Perbandingan tidak dapat dimuat: { $error }
copy-compare-loading = Memuat perbandingan…
copy-compare-empty = Tidak ada tugas untuk dibandingkan.
copy-compare-empty-message = Tambahkan tugas salin untuk membandingkan hasilnya dengan yang lain.
copy-compare-curve-title = P&L terealisasi kumulatif
copy-table-wallet = Dompet
copy-table-mode = Mode
copy-table-rounds = Putaran
copy-table-realized = Terealisasi
copy-table-profit-factor = Profit factor
copy-table-average-hold = Rata-rata hold
copy-table-median-slippage = Median slippage

## Charts

copy-chart-curve-label = P&L kumulatif { $amount } { -sol }
copy-chart-compare-label = P&L kumulatif per tugas
copy-chart-empty-curve = Belum ada putaran tertutup pada rentang ini.
copy-chart-empty-bars = Tidak ada yang tercatat pada rentang ini.
copy-chart-empty-histogram = Tidak ada sampel kedatangan pada rentang ini.
copy-chart-empty-compare = Tidak ada putaran tertutup untuk dibandingkan pada rentang ini.
copy-chart-histogram-title = { $count } dari { $total }

## Wallet profile

copy-profile-copy = Salin dompet ini
copy-profile-copy-other = Salin dengan aturan lain
copy-profile-loading = Memuat profil dompet…
copy-profile-watch-title = Pantau
copy-profile-watched = Dipantau
copy-profile-watch-resume-hint = Melanjutkan tugas akan memantaunya lagi
copy-profile-watch-add-hint = Menambahkan tugas akan mulai memantaunya
copy-profile-stream = Stream
copy-profile-subscribed = Berlangganan
copy-profile-not-subscribed = Tidak berlangganan
copy-profile-sources =
    { $count ->
       *[other] { $count } sumber
    }
copy-profile-last-activity = Aktivitas terakhir
copy-profile-last-error = Error terakhir
copy-profile-own-wallet = Ini salah satu dompet Anda sendiri; menyalinnya ditolak.
copy-profile-observed-title = Trade teramati
copy-profile-observed-none = Belum ada trade dari dompet ini di bot ini. Tugas Paper mengamatinya tanpa mengeluarkan { -sol }.
copy-profile-swaps-seen = Swap terlihat
copy-profile-swaps-seen-note = Swap dompet unik di seluruh tugas Anda
copy-profile-buys-sells = Beli / jual
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = Token diperdagangkan
copy-profile-first-seen = Pertama terlihat
copy-profile-last-seen = Terakhir terlihat
copy-profile-tasks-title = Tugas Anda pada dompet ini
copy-table-task = Tugas

## Arm live dialog

copy-arm-acks-left =
    { $count ->
       *[other] { $count } konfirmasi lagi untuk dicentang
    }
copy-arm-readiness-title = Kesiapan dari buku paper
copy-arm-exposure-title = Eksposur
copy-arm-per-copy = Per salinan
copy-arm-budget-left-value = { $left } dari { $total } { -sol }
copy-arm-budget-left = Sisa anggaran live
copy-arm-budget-left-note = Pengeluaran paper dihitung terpisah dan tidak memakainya
copy-arm-exits = Exit
copy-arm-stop-note = Tidak sebelum hold { $hold }: penurunan lebih cepat ditutup lebih rendah
copy-arm-shared = Dompet ini juga disalin oleh { $tasks }: setiap tugas menyalin trade-nya dengan anggarannya sendiri.
copy-arm-unavailable = Eksekusi live tidak tersedia saat ini; lihat pemeriksaan terakhir.
copy-arm-ack-real-native = { -sol } sungguhan: tugas ini dapat mengeluarkan hingga { $budget } { -sol } dari dompet Anda, maksimal { $trade } { -sol } per salinan.
copy-arm-ack-fees = Salinan live membayar biaya jaringan dan slippage yang nyata; hasil paper tidak menjamin hasil live.
copy-arm-ack-unready = Beberapa pemeriksaan kesiapan belum lolos. Tetap aktifkan tugas ini.
copy-arm-lead = “{ $name }” akan menyalin trade dompet ini dengan swap sungguhan dari dompet Anda.
copy-arm-confirmation-missing = Konfirmasi live tidak dapat dimuat
copy-arm-armed = Live copying diaktifkan
copy-arm-failed = Live copying tidak dapat diaktifkan

## Holdings tab

copy-holdings-title = Kepemilikan
copy-holdings-view-label = Tampilan kepemilikan
copy-holdings-view-open = Terbuka ({ $count })
copy-holdings-view-closed = Putaran tertutup ({ $count })
copy-holdings-reset = Atur ulang buku paper
copy-holdings-live-note = Salinan live adalah posisi sungguhan.
copy-holdings-open-positions = Posisi Terbuka
copy-holdings-token-details = Buka detail token
copy-holdings-opened = Dibuka { $time }
copy-holdings-no-pool-price = Tidak ada harga pool
copy-holdings-close = Tutup
copy-holdings-write-off = Hapus buku
copy-holdings-activity = Aktivitas
copy-holdings-no-exit-rule = Tanpa aturan exit
copy-holdings-watch-stop = Stop { $level }
copy-holdings-watch-stop-until = Stop { $level } dalam { $span }
copy-holdings-watch-take = Take { $level }
copy-holdings-watch-trail = Trail { $level }
copy-holdings-watch-trail-arms = Trail aktif { $level }
copy-holdings-watch-time = Waktu ≤ { $level }
copy-holdings-watch-time-until = Waktu ≤ { $level } dalam { $span }
copy-holdings-watch-wallet-sells = Dompet menjual
copy-holdings-empty = Tidak ada kepemilikan paper yang terbuka. Pembelian yang disalin dari dompet muncul di sini.
copy-holdings-col-token = Token
copy-holdings-col-cost = Biaya
copy-holdings-col-entry = Entry
copy-holdings-col-mark = Mark
copy-holdings-col-peak = Puncak
copy-holdings-col-pnl = P&L
copy-holdings-col-exit-rules = Aturan exit
copy-holdings-col-held = Durasi
copy-holdings-col-actions = Aksi
copy-holdings-col-invested = Diinvestasikan
copy-holdings-col-proceeds = Hasil
copy-holdings-col-exit = Exit
copy-holdings-col-closed = Ditutup
copy-holdings-price-note = Harga dalam { -sol } per token. Entry mencakup slippage dan biaya pembelian; puncak dan level exit relatif terhadapnya, sehingga kepemilikan dibuka dengan puncak di bawah entry. Arahkan kursor ke salah satunya untuk melihat harga pool-nya.
copy-holdings-paused-rules = Dijeda: tidak ada salinan baru. Aturan exit Anda tetap menutup kepemilikan ini.
copy-holdings-paused-mirror = Dijeda: tidak ada salinan baru. Penjualan dompet tetap menutup kepemilikan ini.
copy-holdings-paused-hybrid = Dijeda: tidak ada salinan baru. Penjualan dompet dan aturan exit Anda tetap menutup kepemilikan ini.
copy-holdings-closed-load-failed = Putaran tertutup tidak dapat dimuat: { $error }
copy-holdings-closed-loading = Memuat putaran tertutup…
copy-holdings-closed-empty = Belum ada putaran tertutup.
copy-holdings-closed-latest = { $shown } terbaru dari { $total } putaran.
copy-holdings-close-title = Tutup kepemilikan paper
copy-holdings-close-message = Jual { $token } di buku paper pada harga pool ({ $price }) dengan slippage dan biaya tugas.
copy-holdings-close-confirm = Tutup kepemilikan
copy-holdings-write-off-title = Hapus buku kepemilikan paper
copy-holdings-write-off-message = { $token } tidak memiliki harga pool untuk dijual. Menghapus buku menutupnya di nol dan mencatat biaya { $cost }-nya sebagai kerugian.
copy-holdings-keep = Simpan
copy-holdings-written-off = { $token } dihapus buku
copy-holdings-closed = { $token } ditutup
copy-holdings-written-off-detail = Ditutup dengan hasil nol
copy-holdings-sold-at = Dijual di { $price }
copy-holdings-close-failed = Kepemilikan tidak dapat ditutup
copy-holdings-reset-message = Mulai ulang “{ $name }”: kepemilikan paper, pengeluaran, fill, exit, dan trade yang dilewati dihapus. Aturan dan dompet tetap ada.
copy-holdings-reset-cancel = Simpan riwayat
copy-holdings-reset-done = Buku paper diatur ulang
copy-holdings-reset-detail =
    { $count ->
       *[other] { $count } keputusan dihapus
    }
copy-holdings-reset-failed = Buku paper tidak dapat diatur ulang

## Activity tab

copy-activity-title = Aktivitas
copy-activity-filter-label = Filter aktivitas
copy-filter-all = Semua
copy-outcome-paper-filled = Beli paper
copy-outcome-live-submitted = Beli live terkirim
copy-outcome-live-confirmed = Beli live terkonfirmasi
copy-outcome-live-failed = Beli live gagal
copy-outcome-paper-sell-observed = Jual paper · dompet menjual
copy-outcome-live-sell-submitted = Jual live terkirim
copy-outcome-live-sell-failed = Jual live gagal
copy-outcome-skipped = Dilewati
copy-activity-decision = Keputusan
copy-activity-paper-exit = Exit paper · { $rule }
copy-activity-filled = { $input } di { $price } · dompet membeli { $target }
copy-activity-filled-slippage = { $input } di { $price } · dompet membeli { $target } · slippage { $slippage }
copy-activity-filled-unpriced = { $input } di { $price } · dihargai pada trade dompet, tanpa harga pool
copy-activity-live-sized = { $sized } · dompet membeli { $target }
copy-activity-sell-nothing = Dompet menjual { $amount } · tidak ada yang dimiliki untuk dijual
copy-activity-written-off = Dihapus buku di nol: tidak ada harga pool
copy-activity-sold = { $tokens } token seharga { $proceeds } di { $price }
copy-activity-full-close = Tutup penuh
copy-activity-partial-exit = Exit { $pct }
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = minimum { $amount }
copy-activity-skip-maximum = maksimum { $value }
copy-activity-skip-stale = terlambat { $arrival }, batas { $limit }
copy-activity-skip-latency = rata-rata { $average }, batas { $limit }
copy-activity-arrival-replayed = Diputar ulang { $span } setelah blok
copy-activity-arrival-seen = Terlihat { $span } setelah blok
copy-activity-link-wallet-tx = Tx dompet
copy-activity-link-own-tx = Tx Anda
copy-activity-only-token = Hanya token ini
copy-activity-skipped-group = Dilewati ×{ $count }
copy-activity-group-detail =
    { $tokens ->
       *[other] { $tokens } token · sejak { $since }
    }
copy-activity-mint-filter =
    .placeholder = Mint token
    .aria-label = Filter berdasarkan mint token
copy-activity-clear = Bersihkan
copy-activity-load-failed = Aktivitas tidak dapat dimuat: { $error }
copy-activity-loading = Memuat aktivitas…
copy-activity-no-match = Tidak ada yang cocok dengan filter ini.
copy-activity-empty = Belum ada keputusan. Fill, exit, dan trade yang dilewati muncul di sini saat dompet bertransaksi.
copy-activity-load-older = Muat yang lebih lama
copy-activity-start = Awal riwayat
copy-activity-older-failed = Aktivitas yang lebih lama tidak dapat dimuat

## Task editor

copy-step-wallet = Dompet
copy-step-sizing = Ukuran
copy-step-entry = Filter entry
copy-step-exits = Exit
copy-step-review = Tinjau
copy-editor-title-edit = Ubah { $name }
copy-editor-title-clone = Duplikat { $name }
copy-editor-sub-edit = Tugas { $mode } · perubahan berlaku pada keputusan berikutnya
copy-editor-sub-clone = Aturan sama, buku paper kosong, dimulai di Paper
copy-editor-save-edit = Simpan perubahan
copy-editor-save-clone = Buat duplikat
copy-editor-save-create = Buat tugas paper
copy-editor-clone-suffix = (salinan)
copy-editor-discard-edit = Buang perubahan
copy-editor-discard-create = Buang tugas ini
copy-editor-discard-edit-message = Perubahan Anda pada “{ $name }” belum disimpan.
copy-editor-discard-create-message = Dompet dan aturan yang sudah dimasukkan belum disimpan.
copy-editor-discard-confirm = Buang
copy-editor-keep-editing = Lanjut mengubah
copy-editor-toast-updated = Tugas diperbarui
copy-editor-toast-clone = Duplikat dibuat
copy-editor-toast-created = Tugas paper dibuat
copy-unit-native = { -sol }
copy-editor-any = Berapa pun
copy-editor-duplicate = Sudah disalin oleh { $tasks }. Tugas ini menyalin trade yang sama lagi, dengan aturan dan anggarannya sendiri.
copy-editor-wallet = Dompet
copy-editor-wallet-identity = Dompet suatu tugas adalah identitasnya. Untuk menyalin dompet lain dengan aturan ini, duplikat tugasnya.
copy-editor-address-label = Alamat dompet
copy-editor-address-placeholder = Alamat dompet Solana
copy-editor-address-help-clone = Aturan sama dengan buku paper kosong. Pertahankan dompet ini untuk menguji aturan lain, atau masukkan dompet lain.
copy-editor-address-help-create = Dompet yang pembeliannya (dan penjualannya, jika Anda pilih) disalin tugas ini.
copy-editor-name-label = Nama <em>opsional</em>
copy-editor-name-placeholder = mis. Rotator cepat
copy-editor-enabled-title = Proses trade dompet
copy-editor-enabled-help = Jika nonaktif, tugas tetap dijeda sampai Anda melanjutkannya.
copy-editor-note-live = Tugas ini live: perubahan berlaku pada salinan sungguhan berikutnya.
copy-editor-note-paper = Tugas berjalan di Paper sampai Anda mengaktifkannya: trade disimulasikan pada harga pool dan tidak ada dana yang dikeluarkan.
copy-editor-copy-size = Ukuran salinan
copy-editor-sizing-fixed = Jumlah tetap
copy-editor-sizing-ratio = Porsi dari trade dompet
copy-editor-amount-fixed = Jumlah per salinan
copy-editor-amount-ratio = Porsi dari setiap trade
copy-editor-amount-help-fixed = Dikeluarkan pada setiap pembelian yang disalin, minimal { $minimum }.
copy-editor-amount-help-ratio = Dari pembelian dompet itu sendiri, hingga batas per trade.
copy-editor-help-trade-cap = Tidak ada salinan tunggal yang mengeluarkan lebih dari ini.
copy-editor-help-token-cap = Total yang dikeluarkan untuk satu token.
copy-editor-help-budget = Semua yang boleh dikeluarkan tugas ini selama masa pakainya; Paper dan Live masing-masing menghitung pengeluarannya sendiri.
copy-editor-preview-title = Biaya sebuah salinan
copy-editor-preview-empty = Masukkan ukuran untuk melihat biaya sebuah salinan.
copy-editor-preview-example = Dompet membeli { $target } → Anda menyalin <strong>{ $copy }</strong>
copy-editor-preview-once = Satu token mendapat satu salinan sebesar { $size }, karena setiap token dibeli sekali
copy-editor-preview-token-cap =
    { $count ->
       *[other] Satu token maksimal mendapat { $count } salinan sebesar { $size }
    }
copy-editor-preview-summary-exact = { $perToken }; anggaran mencukupi sekitar { $count } salinan. Biaya jaringan dan prioritas ditambahkan di atasnya.
copy-editor-preview-summary-minimum = { $perToken }; anggaran mencukupi minimal { $count } salinan. Biaya jaringan dan prioritas ditambahkan di atasnya.
copy-editor-target-min = Trade dompet terkecil yang disalin
copy-editor-target-min-help = Abaikan pembelian dompet yang lebih kecil. Kosongkan untuk tanpa minimum.
copy-editor-target-max = Trade dompet terbesar yang disalin
copy-editor-target-max-help = Abaikan pembelian dompet yang lebih besar. Kosongkan untuk tanpa maksimum.
copy-editor-buy-once-title = Beli setiap token sekali
copy-editor-buy-once-help = Salin hanya pembelian pertama dompet atas suatu token; pembelian berikutnya dilewati.
copy-editor-filter-require = Wajibkan
copy-editor-filter-skip = Jangan wajibkan
copy-editor-filter-help = Wajibkan token lolos pipeline Pemfilteran Anda sebelum disalin.
copy-editor-filter-warning = Dengan pengaturan Pemfilteran default hampir semua token gagal, sehingga tugas yang mewajibkan lolos tidak menyalin apa pun. Wajibkan hanya jika filter Anda meloloskan token yang diperdagangkan dompet ini.
copy-editor-exit-both = Keduanya
copy-editor-exit-help-buy-only = Aturan Anda di bawah menjual setiap kepemilikan; penjualan dompet diabaikan.
copy-editor-exit-help-hybrid = Mana yang lebih dulu: dompet menjual, atau salah satu aturan Anda terpicu.
copy-editor-exit-help-mirror = Kepemilikan hanya dijual saat dompet menjual. Aturan exit Anda tidak berjalan.
copy-editor-who-sells = Siapa yang menjual
copy-editor-preset = Preset
copy-editor-preset-help = Preset mengisi setiap aturan di bawah; sesuaikan sesudahnya jika perlu.
copy-editor-mirror-note = Aturan ini tidak berjalan selama penjualan dompet yang menentukan. Aturan berlaku jika Anda beralih ke { $mine } atau { $both }.
copy-editor-rule-inherit = Default Trader
copy-editor-inherit-value = Default Trader ({ $value })
copy-editor-rule-aria = Pengaturan { $rule }
copy-editor-rule-empty-uses = Kosong memakai default Trader: { $value }
copy-editor-rule-follows = Mengikuti Trader: { $summary }
copy-editor-rule-follows-plain = Mengikuti pengaturan Trader.
copy-editor-rule-follows-own = Mengikuti sakelar Trader dengan nilai tugas ini: { $summary }
copy-editor-rule-off-note = Nonaktif untuk tugas ini, apa pun yang dipakai Trader.
copy-task-unnamed = Tugas tanpa nama
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = memproses trade setelah disimpan
copy-editor-review-paused = disimpan dalam keadaan dijeda
copy-editor-error-address = Masukkan alamat dompet Solana yang valid.
copy-editor-error-sizing = Setiap nilai ukuran harus di atas nol.
copy-editor-error-min-copy = Sebuah salinan minimal { $minimum }: naikkan jumlah per salinan.
copy-editor-error-min-cap = Sebuah salinan minimal { $minimum }: naikkan batas per trade.
copy-editor-error-trade-cap = Batas per trade tidak boleh melebihi batas per token.
copy-editor-error-token-cap = Batas per token tidak boleh melebihi total anggaran.
copy-editor-error-slippage = Slippage harus antara { $min } dan { $max }.
copy-editor-error-target-limits = Batas trade dompet harus nol atau lebih.
copy-editor-error-target-order = Trade dompet terkecil tidak boleh melebihi yang terbesar.

## Copy notices

copy-notice-task-unnamed = Tugas #{ $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = Beli salinan paper
copy-notice-title-paper-sell = Jual salinan paper
copy-notice-title-paper-closed = Kepemilikan paper ditutup
copy-notice-title-paper-exit = Exit paper: { $rule }
copy-notice-title-live-buy-submitted = Beli salinan live terkirim
copy-notice-title-live-buy-confirmed = Beli salinan live terkonfirmasi
copy-notice-title-live-buy-failed = Beli salinan live gagal
copy-notice-title-live-sell-submitted = Jual salinan live terkirim
copy-notice-title-live-sell-failed = Jual salinan live gagal
copy-notice-title-auto-paused = Tugas salin dijeda otomatis
copy-notice-detail-bought = Dibeli seharga { $amount } { -sol }
copy-notice-detail-sold = Dijual seharga { $amount } { -sol }
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = { $percent }% dari kepemilikan
copy-notice-detail-full-close = Tutup penuh
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = Swap gagal
