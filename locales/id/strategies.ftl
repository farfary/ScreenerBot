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

strategies-summary-param = { $label }: { $value }
strategies-summary-parts =
    { $count ->
        [1] { $first }
        [2] { $first }, { $second }
       *[3] { $first }, { $second }, { $third }
    }
strategies-summary-none = Tanpa parameter
strategies-summary-period-seconds = Periode: { $amount } dtk
strategies-summary-period-minutes = Periode: { $amount } mnt
strategies-summary-period-hours = Periode: { $amount } jam

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
strategies-unit-sol = { -sol }
strategies-unit-hours = jam
strategies-unit-multiplier = ×

strategies-catalog-search =
    .placeholder = Cari kondisi...
strategies-catalog-search-clear =
    .aria-label = Hapus pencarian
strategies-catalog-fold-all = Ciutkan Semua
strategies-catalog-unfold-all = Bentangkan Semua
strategies-catalog-no-description = Tidak ada deskripsi

strategies-create-title = Buat Strategi Baru
strategies-create-prompt = Pilih jenis strategi yang ingin Anda buat:
strategies-create-entry-name = Strategi Entry
strategies-create-entry-description = Tentukan kondisi kapan MEMBELI token
strategies-create-exit-name = Strategi Exit
strategies-create-exit-description = Tentukan kondisi kapan MENJUAL token

strategies-delete-title = Hapus Strategi
strategies-delete-message = Hapus strategi "{ $name }"? Tindakan ini tidak dapat dibatalkan.

strategies-toast-fix-validation = Perbaiki kesalahan validasi sebelum menyimpan
strategies-toast-enabled = Strategi Diaktifkan
    .message = "{ $name }" diaktifkan
strategies-toast-disabled = Strategi Dinonaktifkan
    .message = "{ $name }" dinonaktifkan
strategies-toast-toggle-failed = Gagal Mengalihkan
    .message = Gagal memperbarui status strategi
strategies-toast-load-failed = Gagal Memuat
    .message = Gagal memuat strategi dari server
strategies-toast-created = Strategi Baru
    .message =
        { $type ->
            [EXIT] Strategi exit baru dibuat
           *[ENTRY] Strategi entry baru dibuat
        }
strategies-toast-load-strategy-failed = Gagal memuat strategi
strategies-toast-no-strategy = Belum Ada Strategi
    .message = Tambahkan setidaknya satu kondisi atau klik 'Strategi Baru' untuk membuat strategi terlebih dahulu
strategies-toast-no-conditions-save = Tidak Ada Kondisi
    .message = Tambahkan setidaknya satu kondisi ke strategi sebelum menyimpan
strategies-toast-name-required = Nama Wajib Diisi
    .message = Masukkan nama strategi sebelum menyimpan
strategies-toast-saved = Strategi Disimpan
    .message = "{ $name }" berhasil disimpan
strategies-toast-save-failed = Gagal Menyimpan
    .message = Gagal menyimpan strategi ke database
strategies-toast-no-strategy-validate = Tidak ada strategi untuk divalidasi
strategies-toast-no-conditions-validate = Tidak Ada Kondisi
    .message = Tambahkan setidaknya satu kondisi sebelum memvalidasi
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
strategies-toast-condition-added = Kondisi Ditambahkan
    .message = { $name } ditambahkan ke strategi

strategies-condition-candle-size = Pola Ukuran Candle
    .description = Deteksi pola candle tertentu: body besar, body kecil (doji), sumbu panjang
strategies-condition-candle-size-param-pattern = Jenis Pola
    .description = Pola candle yang dideteksi
strategies-condition-candle-size-param-pattern-option-large-body = Body Besar (Pergerakan Kuat)
strategies-condition-candle-size-param-pattern-option-small-body = Body Kecil (Doji/Ragu-ragu)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Sumbu Atas Panjang (Penolakan)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Sumbu Bawah Panjang (Support)
strategies-condition-candle-size-param-threshold = Ambang Ukuran %
    .description = Ambang persentase untuk deteksi pola

strategies-condition-consecutive-candles = Candle Berurutan
    .description = Deteksi candle hijau (bullish) atau merah (bearish) berurutan dengan filter ukuran minimum
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
    .description = Periksa berapa lama posisi di-hold (untuk strategi exit - exit berbasis waktu)
strategies-condition-position-holding-time-param-hours = Ambang Waktu (Jam)
    .description = Durasi dalam jam sejak posisi dibuka
strategies-condition-position-holding-time-param-comparison = Perbandingan
    .description = Cara membandingkan usia posisi dengan ambang
strategies-condition-position-holding-time-param-comparison-option-greater-than = Lebih Lama Dari (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = Minimal (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Lebih Baru Dari ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = Maksimal (≤)

strategies-condition-price-breakout = Breakout Harga
    .description = Deteksi harga menembus di atas resistance (tertinggi periode) atau di bawah support (terendah periode)
strategies-condition-price-breakout-param-lookback = Periode Lookback
    .description = Jumlah candle untuk menentukan level support/resistance
strategies-condition-price-breakout-param-direction = Arah Breakout
    .description = Arah breakout
strategies-condition-price-breakout-param-direction-option-upward = Ke Atas (Tembus Resistance)
strategies-condition-price-breakout-param-direction-option-downward = Ke Bawah (Tembus Support)
strategies-condition-price-breakout-param-confirmation = Konfirmasi %
    .description = Seberapa jauh melewati level untuk mengonfirmasi breakout (menghindari sinyal palsu)

strategies-condition-price-change-percent = Perubahan Harga %
    .description = Periksa apakah harga berubah melampaui ambang persentase dalam suatu periode waktu
strategies-condition-price-change-percent-param-percentage = Ambang Perubahan %
    .description = Persentase perubahan harga yang memicu (0.1-1000%)
strategies-condition-price-change-percent-param-direction = Arah
    .description = Arah pergerakan harga
strategies-condition-price-change-percent-param-direction-option-above = Naik (+%)
strategies-condition-price-change-percent-param-direction-option-below = Turun (-%)
strategies-condition-price-change-percent-param-direction-option-within = Dalam Rentang (±%)
strategies-condition-price-change-percent-param-time-value = Periode Waktu
    .description = Nilai periode lookback (1-3600 untuk detik, 1-1440 untuk menit, 1-720 untuk jam)
strategies-condition-price-change-percent-param-time-unit = Satuan Waktu
    .description = Satuan waktu untuk periode lookback
strategies-condition-price-change-percent-param-time-unit-option-seconds = Detik
strategies-condition-price-change-percent-param-time-unit-option-minutes = Menit
strategies-condition-price-change-percent-param-time-unit-option-hours = Jam

strategies-condition-price-to-ma = Harga vs Moving Average
    .description = Periksa apakah harga di atas, di bawah, atau dalam rentang Simple Moving Average-nya
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
    .description = Deteksi lonjakan volume dibandingkan volume rata-rata (menandakan minat meningkat)
strategies-condition-volume-spike-param-lookback = Periode Lookback
    .description = Jumlah candle untuk menghitung volume rata-rata
strategies-condition-volume-spike-param-multiplier = Pengali Volume
    .description = Berapa kali di atas rata-rata (mis., 2.0 = 200% dari rata-rata)

strategies-condition-param-timeframe = Timeframe
    .description = Timeframe candle yang dianalisis (default ke timeframe strategi jika tidak diatur)
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

strategies-error-expected-boolean = boolean
strategies-error-expected-number = angka
strategies-error-expected-string = string

strategies-error-data-current-price = Harga saat ini
strategies-error-data-liquidity-data = Data likuiditas
strategies-error-data-market-data = Data pasar
strategies-error-data-ohlcv-data = Data OHLCV
strategies-error-data-position-data = Data posisi

strategies-error-indicator-consecutive-candles = candle berurutan
strategies-error-indicator-moving-average = moving average
strategies-error-indicator-price-breakout = breakout harga
strategies-error-indicator-price-change-lookback = lookback perubahan harga
strategies-error-indicator-volume-spike = lonjakan volume

strategies-error-rule-branch-node-missing-conditions = Node cabang tidak memiliki kondisi
strategies-error-rule-branch-node-missing-operator = Node cabang tidak memiliki operator
strategies-error-rule-branch-node-must-have-at-least-one-child = Node cabang harus memiliki setidaknya satu anak
strategies-error-rule-invalid-rule-tree-structure = Struktur pohon aturan tidak valid
strategies-error-rule-leaf-node-missing-condition = Node daun tidak memiliki kondisi
strategies-error-rule-not-operator-must-have-exactly-one-child = Operator NOT harus memiliki tepat satu anak
