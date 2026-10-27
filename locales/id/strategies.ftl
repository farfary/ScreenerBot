# Strategies page: the strategy list, the condition editor and the condition catalog.

strategies-filter-all = Semua
strategies-filter-entry = Entry
strategies-filter-exit = Exit
strategies-type-entry = Entry
strategies-type-exit = Exit
strategies-list-empty-title = Belum ada strategi
strategies-list-empty-hint = Buat strategi pertama Anda
strategies-new = Strategi Baru
strategies-import =
    .title = Impor Strategi
    .aria-label = Impor Strategi
strategies-item-enable =
    .title = Aktifkan
strategies-item-disable =
    .title = Nonaktifkan

strategies-new-name = Strategi Baru

strategies-editor-name =
    .placeholder = Nama strategi
strategies-editor-dirty =
    .title = Perubahan belum disimpan
strategies-action-validate = Validasi
strategies-editor-empty = Pilih strategi untuk diedit, atau buat yang baru
strategies-conditions-empty-title = Belum ada kondisi
strategies-conditions-empty-hint = Gunakan "{ strategies-add-condition }" untuk mulai menyusun
strategies-add-condition = Tambah Kondisi
strategies-modal-close =
    .aria-label = Tutup
strategies-card-move-up =
    .title = Pindah ke atas
strategies-card-move-down =
    .title = Pindah ke bawah
strategies-card-duplicate =
    .title = Duplikat
strategies-card-delete =
    .title = Hapus
# $name is the condition name.
strategies-card-delete-confirm = Hapus kondisi
    .message = Hapus "{ $name }" dari strategi ini?

strategies-summary-param = { $label }: { $value }
strategies-summary-none = Tanpa parameter
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = Pengaturan strategi ({ $value })
strategies-summary-period-seconds = Periode lookback: { $amount } dtk
strategies-summary-period-minutes = Periode lookback: { $amount } mnt
strategies-summary-period-hours = Periode lookback: { $amount } jam

strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
       *[other] { $amount } jam
    }
strategies-value-candles =
    { $count ->
       *[other] { $amount } candle
    }

strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = jam
strategies-unit-multiplier = ×

strategies-catalog-search =
    .placeholder = Cari kondisi...
    .aria-label = Cari kondisi
strategies-catalog-search-clear =
    .aria-label = Hapus pencarian
strategies-catalog-no-description = Tidak ada deskripsi

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = Aktif
strategies-editor-timeframe = Timeframe default
    .title = Timeframe candle untuk kondisi yang tidak menetapkan sendiri
strategies-editor-priority = Prioritas
    .title = Urutan evaluasi dari 1 sampai 100; angka lebih kecil dievaluasi lebih dulu
strategies-editor-description = Deskripsi
strategies-editor-description-field =
    .placeholder = Apa yang dilakukan strategi ini
    .aria-label = Deskripsi strategi
strategies-action-export = Ekspor

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = Cocokkan { $mode } kondisi ini
strategies-group-match = Cocokkan { $mode } kondisi di sini
strategies-match-all = semua
strategies-match-any = salah satu
strategies-match-mode =
    .aria-label = Cocokkan semua atau salah satu
strategies-invert = Balikkan
    .title = Balikkan: benar saat ini tidak cocok
strategies-chip-off = Mati
strategies-chip-inverted = Dibalik
strategies-row-switch =
    .title = Matikan untuk tetap menyimpan tanpa dievaluasi
    .aria-label = Dievaluasi oleh strategi ini
strategies-row-expand =
    .aria-label = Tampilkan atau sembunyikan parameter
strategies-add-group = Tambah Grup
# $count is the number of conditions in the group.
strategies-group-delete-confirm = Hapus grup
    .message = Hapus grup ini beserta { $count } kondisi di dalamnya?

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems = { $count } masalah perlu diperbaiki: { $first }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }, { $field }: { $problem }
strategies-problem-rules-empty = Tambahkan setidaknya satu kondisi yang aktif
strategies-problem-group-empty = Grup memerlukan setidaknya satu kondisi yang aktif
strategies-problem-not-applicable = Kondisi ini tidak dapat dipakai pada jenis strategi ini
strategies-problem-unknown-condition = Jenis kondisi tidak dikenal: { $type }
strategies-problem-too-deep = Aturan dapat bersarang paling banyak { $max } tingkat; grup yang dibalik dihitung sebagai satu tingkat tambahan
strategies-problem-required = Masukkan angka
strategies-problem-range = Harus antara { $min } dan { $max }
strategies-problem-invalid-option = Pilih salah satu opsi yang tersedia

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = Semua
strategies-catalog-categories =
    .aria-label = Kategori
strategies-catalog-empty = Tidak ada kondisi yang cocok dengan pencarian
strategies-catalog-exit-only = Hanya strategi exit
strategies-catalog-entry-only = Hanya strategi entry
strategies-catalog-candles-note = { $description } Hanya membaca candle.
strategies-data-candles = Candle
strategies-data-pool = Pool
strategies-data-position = Posisi
strategies-data-market = Aktivitas pasar

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = trade
strategies-value-trades = { $amount } trade

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = Terlalu Banyak Tingkat
    .message = Aturan dapat bersarang paling banyak { $max } tingkat
strategies-toast-priority-invalid = Prioritas Tidak Valid
    .message = Masukkan bilangan bulat dari 1 sampai 100
strategies-toast-exported = Strategi diekspor
# $name is the strategy name.
strategies-unsaved-confirm = Buang perubahan yang belum disimpan?
    .message = "{ $name }" memiliki perubahan yang belum disimpan.
strategies-unsaved-discard = Buang
strategies-import-error-json = File bukan JSON yang valid
strategies-import-error-shape = File tidak menjelaskan sebuah strategi
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = File memakai kondisi yang tidak ada di versi ini: { $type }
strategies-import-error-not-applicable = { $name } tidak dapat dipakai pada jenis strategi ini

strategies-create-title = Buat Strategi Baru
strategies-create-prompt = Pilih jenis strategi yang ingin Anda buat:
strategies-create-entry-name = Strategi Entry
strategies-create-entry-description = Tentukan kondisi kapan membeli token
strategies-create-exit-name = Strategi Exit
strategies-create-exit-description = Tentukan kondisi kapan menjual token

strategies-delete-title = Hapus Strategi
strategies-delete-message = Hapus strategi "{ $name }"? Tindakan ini tidak dapat dibatalkan.

strategies-toast-fix-validation = Perbaiki kesalahan validasi sebelum menyimpan
strategies-toast-load-failed = Gagal Memuat
    .message = Gagal memuat strategi dari server
strategies-toast-load-strategy-failed = Gagal memuat strategi
strategies-toast-no-strategy = Belum Ada Strategi
    .message = Tambahkan setidaknya satu kondisi atau klik 'Strategi Baru' untuk membuat strategi terlebih dahulu
strategies-toast-name-required = Nama Wajib Diisi
    .message = Masukkan nama strategi sebelum menyimpan
strategies-toast-saved = Strategi Disimpan
    .message = "{ $name }" berhasil disimpan
strategies-toast-save-failed = Gagal Menyimpan
    .message = Gagal menyimpan strategi ke database
strategies-toast-no-strategy-validate = Tidak ada strategi untuk divalidasi
strategies-toast-valid = Strategi valid
strategies-toast-invalid = Strategi memiliki kesalahan
strategies-toast-validation-failed = Validasi gagal
strategies-toast-item-enabled = Strategi diaktifkan
strategies-toast-item-disabled = Strategi dinonaktifkan
strategies-toast-item-toggle-failed = Gagal mengalihkan strategi
strategies-toast-deleted = Strategi Dihapus
    .message = "{ $name }" berhasil dihapus
strategies-toast-delete-failed = Gagal Menghapus
    .message = Gagal menghapus strategi dari database
strategies-toast-imported = Strategi diimpor
strategies-toast-import-failed = Gagal mengimpor strategi
strategies-toast-unknown-condition = Kondisi Tidak Dikenal
    .message = Jenis kondisi tidak ditemukan
strategies-toast-create-first = Buat Strategi Terlebih Dahulu
    .message = Klik 'Strategi Baru' untuk membuat strategi sebelum menambahkan kondisi

strategies-condition-candle-size = Pola Ukuran Candle
    .description = Deteksi pola candle tertentu: body besar, body kecil (doji), sumbu panjang.
strategies-condition-candle-size-param-pattern = Jenis Pola
    .description = Pola candle yang dideteksi
strategies-condition-candle-size-param-pattern-option-large-body = Body Besar (Pergerakan Kuat)
strategies-condition-candle-size-param-pattern-option-small-body = Body Kecil (Doji/Ragu-ragu)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Sumbu Atas Panjang (Penolakan)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Sumbu Bawah Panjang (Support)
strategies-condition-candle-size-param-threshold = Ambang Ukuran %
    .description = Ambang persentase untuk deteksi pola

strategies-condition-consecutive-candles = Candle Berurutan
    .description = Deteksi candle hijau (bullish) atau merah (bearish) berurutan dengan filter ukuran minimum.
strategies-condition-consecutive-candles-param-count = Jumlah Candle
    .description = Jumlah candle berurutan yang diperlukan
strategies-condition-consecutive-candles-param-direction = Arah Candle
    .description = Warna/arah candle berurutan
strategies-condition-consecutive-candles-param-direction-option-green = Hijau (Bullish)
strategies-condition-consecutive-candles-param-direction-option-red = Merah (Bearish)
strategies-condition-consecutive-candles-param-minimum-change = Perubahan Minimum %
    .description = Perubahan % minimum untuk setiap candle (menyaring noise)

strategies-condition-liquidity-level = Level Likuiditas Pool
    .description = Periksa likuiditas pool dalam { -sol } (Entry: pastikan likuiditas cukup, Exit: deteksi penarikan likuiditas)
strategies-condition-liquidity-level-param-threshold = Ambang Likuiditas ({ -sol })
    .description = Level likuiditas pool dalam { -sol }
strategies-condition-liquidity-level-param-comparison = Perbandingan
    .description = Cara membandingkan likuiditas pool dengan ambang
strategies-condition-liquidity-level-param-comparison-option-greater-than = Lebih Besar Dari (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = Lebih Besar atau Sama Dengan (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = Lebih Kecil Dari ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = Lebih Kecil atau Sama Dengan (≤)

strategies-condition-position-holding-time = Durasi Hold Posisi
    .description = Periksa berapa lama posisi di-hold
strategies-condition-position-holding-time-param-hours = Ambang Waktu (Jam)
    .description = Durasi dalam jam sejak posisi dibuka
strategies-condition-position-holding-time-param-comparison = Perbandingan
    .description = Cara membandingkan usia posisi dengan ambang
strategies-condition-position-holding-time-param-comparison-option-greater-than = Lebih Lama Dari (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = Minimal (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Lebih Baru Dari ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = Maksimal (≤)

strategies-condition-price-breakout = Breakout Harga
    .description = Deteksi harga menembus di atas resistance (tertinggi periode) atau di bawah support (terendah periode).
strategies-condition-price-breakout-param-lookback = Periode Lookback
    .description = Jumlah candle untuk menentukan level support/resistance
strategies-condition-price-breakout-param-direction = Arah Breakout
    .description = Arah breakout
strategies-condition-price-breakout-param-direction-option-upward = Ke Atas (Tembus Resistance)
strategies-condition-price-breakout-param-direction-option-downward = Ke Bawah (Tembus Support)
strategies-condition-price-breakout-param-confirmation = Konfirmasi %
    .description = Seberapa jauh melewati level untuk mengonfirmasi breakout (menghindari sinyal palsu)

strategies-condition-price-change-percent = Perubahan Harga %
    .description = Periksa apakah harga berubah melampaui ambang persentase dalam suatu periode waktu.
strategies-condition-price-change-percent-param-percentage = Ambang Perubahan %
    .description = Persentase perubahan harga yang memicu (0.1-1000%)
strategies-condition-price-change-percent-param-direction = Arah
    .description = Arah pergerakan harga
strategies-condition-price-change-percent-param-direction-option-above = Naik (+%)
strategies-condition-price-change-percent-param-direction-option-below = Turun (-%)
strategies-condition-price-change-percent-param-direction-option-within = Dalam Rentang (±%)
strategies-condition-price-change-percent-param-time-value = Periode lookback
    .description = Seberapa jauh ke belakang perubahan diukur, dalam satuan yang dipilih di dalam kolom (1-3600 dtk, 1-1440 mnt, 1-720 jam)
strategies-condition-price-change-percent-param-time-unit = Satuan Waktu
    .description = Satuan waktu untuk periode lookback
strategies-condition-price-change-percent-param-time-unit-option-seconds = dtk
strategies-condition-price-change-percent-param-time-unit-option-minutes = mnt
strategies-condition-price-change-percent-param-time-unit-option-hours = jam

strategies-condition-price-to-ma = Harga vs Moving Average
    .description = Periksa apakah harga di atas, di bawah, atau dalam rentang Simple Moving Average-nya.
strategies-condition-price-to-ma-param-period = Periode MA
    .description = Jumlah candle untuk perhitungan moving average
strategies-condition-price-to-ma-param-position = Posisi
    .description = Posisi harga relatif terhadap MA
strategies-condition-price-to-ma-param-position-option-above = Di Atas MA
strategies-condition-price-to-ma-param-position-option-below = Di Bawah MA
strategies-condition-price-to-ma-param-position-option-within = Dalam Rentang
strategies-condition-price-to-ma-param-distance = Jarak %
    .description = Jarak minimum dari MA (untuk DI ATAS/DI BAWAH) atau rentang maksimum (untuk DALAM RENTANG)

strategies-condition-volume-spike = Lonjakan Volume
    .description = Deteksi lonjakan volume dibandingkan volume rata-rata (menandakan minat meningkat).
strategies-condition-volume-spike-param-lookback = Periode Lookback
    .description = Jumlah candle untuk menghitung volume rata-rata
strategies-condition-volume-spike-param-multiplier = Pengali Volume
    .description = Berapa kali di atas rata-rata (mis., 2.0 = 200% dari rata-rata)

strategies-condition-rsi = RSI (Relative Strength Index)
    .description = Terpicu saat RSI, dengan penghalusan Wilder pada candle yang sudah ditutup, berada di atas atau di bawah ambang.
strategies-condition-rsi-param-period = Periode RSI
    .description = Jumlah candle yang dipakai untuk menghitung RSI; kondisi ini membutuhkan satu candle lebih banyak dari nilai ini
strategies-condition-rsi-param-comparison = Perbandingan
    .description = Apakah RSI harus di bawah atau di atas ambang
strategies-condition-rsi-param-comparison-option-below = Di Bawah (Oversold)
strategies-condition-rsi-param-comparison-option-above = Di Atas (Overbought)
strategies-condition-rsi-param-threshold = Ambang RSI
    .description = Level RSI dari 1 sampai 99 (30 dan 70 adalah level oversold dan overbought yang lazim)

strategies-condition-ma-cross = Persilangan Moving Average
    .description = Terpicu saat moving average cepat memotong yang lambat dalam beberapa candle terakhir.
strategies-condition-ma-cross-param-ma-type = Jenis Average
    .description = Average eksponensial memberi bobot lebih pada candle terbaru; average sederhana membobot semua candle sama
strategies-condition-ma-cross-param-ma-type-option-ema = Eksponensial (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = Sederhana (SMA)
strategies-condition-ma-cross-param-fast-period = Periode Cepat
    .description = Jumlah candle pada average cepat
strategies-condition-ma-cross-param-slow-period = Periode Lambat
    .description = Jumlah candle pada average lambat; harus lebih besar dari periode cepat
strategies-condition-ma-cross-param-direction = Arah Persilangan
    .description = Ke arah mana average cepat memotong yang lambat
strategies-condition-ma-cross-param-direction-option-bullish = Bullish (Cepat Memotong ke Atas)
strategies-condition-ma-cross-param-direction-option-bearish = Bearish (Cepat Memotong ke Bawah)
strategies-condition-ma-cross-param-within-candles = Persilangan dalam
    .description = Berapa candle terakhir yang boleh memuat persilangan; 1 berarti hanya candle terakhir

strategies-condition-position-profit = Profit Posisi
    .description = Terpicu saat profit belum terealisasi dari posisi terbuka berada di atas atau di bawah suatu persentase dari harga entry, diukur pada harga pool.
strategies-condition-position-profit-param-comparison = Perbandingan
    .description = Apakah profit harus minimal atau maksimal sebesar ambang
strategies-condition-position-profit-param-comparison-option-above = Minimal (≥)
strategies-condition-position-profit-param-comparison-option-below = Maksimal (≤)
strategies-condition-position-profit-param-threshold-pct = Ambang Profit %
    .description = Profit belum terealisasi sebagai persentase dari harga entry; nilai negatif berarti rugi

strategies-condition-drawdown-from-peak = Penurunan dari Puncak
    .description = Terpicu saat harga pool turun sebesar persentase tertentu dari harga tertinggi sejak pembelian terakhir, setelah puncak itu cukup jauh di atas harga entry.
strategies-condition-drawdown-from-peak-param-drop-pct = Penurunan dari Puncak %
    .description = Seberapa jauh harga saat ini harus di bawah puncak
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = Aktif Setelah Profit %
    .description = Puncak harus pernah setidaknya sejauh ini di atas harga entry sebelum kondisi bisa terpicu; 0 berarti selalu aktif

strategies-condition-buy-pressure = Tekanan Beli
    .description = Terpicu saat pembelian minimal membentuk persentase tertentu dari transaksi dalam suatu jendela waktu dan jendela itu punya cukup transaksi. Membaca jumlah beli dan jual token.
strategies-condition-buy-pressure-param-window = Jendela Waktu
    .description = Periode yang dicakup jumlah beli dan jual
strategies-condition-buy-pressure-param-window-option-5m = 5 Menit
strategies-condition-buy-pressure-param-window-option-1h = 1 Jam
strategies-condition-buy-pressure-param-window-option-6h = 6 Jam
strategies-condition-buy-pressure-param-window-option-24h = 24 Jam
strategies-condition-buy-pressure-param-min-buy-share-pct = Porsi Beli Minimum %
    .description = Pembelian sebagai persentase dari semua transaksi dalam jendela
strategies-condition-buy-pressure-param-min-transactions = Transaksi Minimum
    .description = Jendela dengan transaksi lebih sedikit tidak pernah memicu kondisi ini

strategies-condition-param-timeframe = Timeframe
    .description = Ukuran candle: durasi setiap candle yang dibaca kondisi (timeframe strategi jika tidak diatur)
strategies-condition-timeframe-option-1m = 1 Menit
strategies-condition-timeframe-option-5m = 5 Menit
strategies-condition-timeframe-option-15m = 15 Menit
strategies-condition-timeframe-option-1h = 1 Jam
strategies-condition-timeframe-option-4h = 4 Jam
strategies-condition-timeframe-option-12h = 12 Jam
strategies-condition-timeframe-option-1d = 1 Hari

strategies-condition-category-price-analysis = Analisis Harga
strategies-condition-category-candle-patterns = Pola Candle
strategies-condition-category-technical-indicators = Indikator Teknikal
strategies-condition-category-market-context = Konteks Pasar
strategies-condition-category-position-performance = Posisi & Performa
strategies-condition-category-volume-analysis = Analisis Volume

strategies-error-missing-parameter = Parameter { $field } tidak ada
strategies-error-parameter-type = Parameter { $field } harus berupa { $expected }
strategies-error-invalid-value = "{ $value }" bukan { $field } yang valid
strategies-error-missing-data = { $data } tidak tersedia
strategies-error-no-candle-data = Timeframe { $timeframe } tidak memiliki data candle
strategies-error-insufficient-history = Riwayat tidak cukup untuk { $indicator }: tersedia { $available } dtk, dibutuhkan { $required } dtk
strategies-error-insufficient-candles = Candle tidak cukup untuk { $indicator }: ada { $available }, dibutuhkan { $required }
strategies-error-stale-candle-data = Data candle { $timeframe } sudah usang: usianya { $age } dtk melebihi { $max } dtk
strategies-error-invalid-rule-tree = Pohon aturan tidak valid: { $reason }
strategies-error-evaluation-timeout = Evaluasi strategi habis waktu setelah { $timeout } ms
strategies-error-condition-not-applicable = { $condition } tidak dapat dipakai pada strategi { $strategy }
strategies-error-invalid-rules = Aturan tidak dapat dibaca: { $reason }

strategies-error-field-average-volume = volume rata-rata
strategies-error-field-candle-open = open candle
strategies-error-field-comparison = perbandingan
strategies-error-field-condition-type = jenis kondisi
strategies-error-field-confirmation = konfirmasi
strategies-error-field-count = jumlah
strategies-error-field-current-price = harga saat ini
strategies-error-field-direction = arah
strategies-error-field-distance = jarak
strategies-error-field-hours = jam
strategies-error-field-lookback = lookback
strategies-error-field-minimum-change = perubahan minimum
strategies-error-field-multiplier = pengali
strategies-error-field-pattern = pola
strategies-error-field-percentage = persentase
strategies-error-field-period = periode
strategies-error-field-position = posisi
strategies-error-field-threshold = ambang
strategies-error-field-time-unit = satuan waktu
strategies-error-field-time-value = nilai waktu
strategies-error-field-timeframe = timeframe
strategies-error-field-arm-after-profit = aktif setelah profit
strategies-error-field-drop-percentage = persentase penurunan
strategies-error-field-entry-price = harga entry
strategies-error-field-fast-period = periode cepat
strategies-error-field-minimum-buy-share = porsi beli minimum
strategies-error-field-minimum-transactions = transaksi minimum
strategies-error-field-moving-average-type = jenis average
strategies-error-field-peak-price = harga puncak
strategies-error-field-slow-period = periode lambat
strategies-error-field-window = jendela
strategies-error-field-within-candles = jumlah candle persilangan

strategies-error-expected-boolean = boolean
strategies-error-expected-number = angka
strategies-error-expected-string = string

strategies-error-data-current-price = Harga saat ini
strategies-error-data-liquidity-data = Data likuiditas
strategies-error-data-market-data = Data pasar
strategies-error-data-ohlcv-data = Data OHLCV
strategies-error-data-position-data = Data posisi
strategies-error-data-peak-price = Harga puncak
strategies-error-data-transaction-counts = Jumlah transaksi

strategies-error-indicator-consecutive-candles = candle berurutan
strategies-error-indicator-moving-average = moving average
strategies-error-indicator-price-breakout = breakout harga
strategies-error-indicator-price-change-lookback = lookback perubahan harga
strategies-error-indicator-volume-spike = lonjakan volume
strategies-error-indicator-moving-average-cross = persilangan moving average
strategies-error-indicator-rsi = RSI

strategies-error-rule-branch-node-missing-conditions = Node cabang tidak memiliki kondisi
strategies-error-rule-branch-node-missing-operator = Node cabang tidak memiliki operator
strategies-error-rule-branch-node-must-have-at-least-one-child = Node cabang harus memiliki setidaknya satu anak
strategies-error-rule-invalid-rule-tree-structure = Struktur pohon aturan tidak valid
strategies-error-rule-leaf-node-missing-condition = Node daun tidak memiliki kondisi
strategies-error-rule-not-operator-must-have-exactly-one-child = Operator NOT harus memiliki tepat satu anak
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = Grup membutuhkan minimal satu kondisi yang aktif
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = Operator NOT tidak boleh memiliki anak yang dimatikan
strategies-error-rule-rules-nest-deeper-than-four-levels = Aturan tidak boleh bersarang lebih dari empat tingkat
strategies-error-rule-the-root-group-cannot-be-switched-off = Grup akar tidak dapat dimatikan
