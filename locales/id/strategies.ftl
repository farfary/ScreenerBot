# Strategies page: the strategy list, the condition editor and the condition catalog.

strategies-type-entry = Entry
strategies-type-exit = Exit
strategies-list-empty-title = Belum ada strategi
strategies-list-empty-hint = Buat strategi pertama Anda
strategies-new = Strategi Baru
strategies-import =
    .title = Impor Strategi
    .aria-label = Impor Strategi

strategies-new-name = Strategi Baru

strategies-editor-name =
    .placeholder = Nama strategi
strategies-editor-dirty =
    .title = Perubahan belum disimpan
strategies-action-validate = Validasi
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
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount } dtk
strategies-value-duration-minutes = { $amount } mnt
strategies-value-duration-hours = { $amount } jam
strategies-value-duration-days = { $amount } hr
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = Waktu lokal Anda: { $time }
strategies-hint-price-impact = Trade sebesar ini menggerakkan harga sekitar { $amount }%

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
strategies-unit-utc = UTC

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
strategies-editor-order = Urutan
    .title = Urutan evaluasi dari 1 sampai 100; angka lebih kecil dievaluasi lebih dulu
strategies-editor-description = Deskripsi
strategies-editor-description-field =
    .placeholder = Apa yang dilakukan strategi ini
    .aria-label = Deskripsi strategi
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = Semua strategi masuk
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = Mengelola
strategies-editor-entry-binding-hint = Mengelola posisi yang dibuka oleh strategi masuk yang dicentang. Tanpa centang, strategi ini mengelola setiap posisi, termasuk yang manual dan disalin.
strategies-editor-entry-binding-empty = Belum ada strategi masuk
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id } (dihapus)
strategies-action-export = Ekspor
strategies-action-test = Uji dengan token
# $token is the tested token's symbol or address.
strategies-test-title = Uji dengan { $token }
# $token is the tested token's symbol or address.
strategies-test-running = Menguji dengan { $token }…
strategies-test-verdict-pass = Terpenuhi
strategies-test-verdict-fail = Tidak terpenuhi
strategies-test-verdict-unknown = Tidak diketahui

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
strategies-data-clock = Waktu dalam hari

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
# $min and $max are the bounds of the evaluation order.
strategies-import-error-priority = Prioritas dalam file harus bilangan bulat dari { $min } sampai { $max }
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
strategies-toast-no-strategy-test = Tidak ada strategi untuk diuji
strategies-toast-test-save-first = Simpan strategi sebelum mengujinya: pengujian membaca aturan yang tersimpan
strategies-toast-test-failed = Pengujian gagal
    .message = Gagal menguji strategi
strategies-toast-valid = Strategi valid
strategies-toast-invalid = Strategi memiliki kesalahan
strategies-toast-validation-failed = Validasi gagal
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

strategies-condition-transaction-velocity = Kecepatan Transaksi
    .description = Aktif saat laju transaksi jendela terbaru minimal atau maksimal sebuah kelipatan dari laju jendela lebih panjang yang memuatnya. Membaca jumlah beli dan jual token.
strategies-condition-transaction-velocity-param-window = Jendela
    .description = Jendela terbaru dan jendela lebih panjang pembandingnya
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = 5 Menit vs 1 Jam
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = 1 Jam vs 6 Jam
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = 6 Jam vs 24 Jam
strategies-condition-transaction-velocity-param-comparison = Perbandingan
    .description = Apakah laju terbaru harus minimal atau maksimal rasio tersebut
strategies-condition-transaction-velocity-param-comparison-option-above = Minimal (≥)
strategies-condition-transaction-velocity-param-comparison-option-below = Maksimal (≤)
strategies-condition-transaction-velocity-param-ratio = Rasio Laju
    .description = Transaksi per detik jendela terbaru dibagi transaksi per detik jendela panjang
strategies-condition-transaction-velocity-param-min-transactions = Transaksi Minimum
    .description = Jendela terbaru dengan transaksi lebih sedikit tidak pernah mengaktifkan kondisi

strategies-condition-pullback-from-high = Koreksi dari Tertinggi
    .description = Aktif saat harga live berada antara penurunan minimum dan maksimum di bawah harga tertinggi candle terakhir.
strategies-condition-pullback-from-high-param-lookback = Periode Lookback
    .description = Jumlah candle tempat harga tertinggi diambil
strategies-condition-pullback-from-high-param-min-drop-pct = Penurunan Minimum %
    .description = Harga harus setidaknya sejauh ini di bawah tertinggi
strategies-condition-pullback-from-high-param-max-drop-pct = Penurunan Maksimum %
    .description = Harga paling jauh sejauh ini di bawah tertinggi; harus lebih besar dari penurunan minimum

strategies-condition-profit-giveback = Pengembalian Profit
    .description = Aktif saat posisi telah mengembalikan sebagian keuntungan belum terealisasi terbaiknya sejak pembelian terakhir, setelah keuntungan itu mencapai suatu level.
strategies-condition-profit-giveback-param-arm-after-profit-pct = Aktif Setelah Profit %
    .description = Keuntungan terbaik harus sudah mencapai persentase harga masuk ini sebelum kondisi bisa aktif
strategies-condition-profit-giveback-param-giveback-pct = Pengembalian %
    .description = Bagian keuntungan terbaik yang dikembalikan; 100% berarti harga kembali ke harga masuk

strategies-condition-token-age = Umur Token
    .description = Aktif saat waktu sejak pembuatan on-chain token lebih lama atau lebih singkat dari suatu durasi. Token tanpa waktu pembuatan yang diketahui membuat kondisi tidak terputuskan.
strategies-condition-token-age-param-comparison = Perbandingan
    .description = Apakah token harus lebih tua atau lebih muda dari durasi
strategies-condition-token-age-param-comparison-option-older-than = Lebih Tua Dari (≥)
strategies-condition-token-age-param-comparison-option-younger-than = Lebih Baru Dari ({ "<" })
strategies-condition-token-age-param-time-value = Umur
    .description = Waktu sejak token dibuat, dalam satuan yang dipilih di dalam kolom (1-10000)
strategies-condition-token-age-param-time-unit = Satuan Waktu
    .description = Satuan umur
strategies-condition-token-age-param-time-unit-option-minutes = mnt
strategies-condition-token-age-param-time-unit-option-hours = jam
strategies-condition-token-age-param-time-unit-option-days = hr

strategies-condition-vwap = VWAP
    .description = Aktif saat harga live berada di atas atau di bawah harga rata-rata tertimbang volume dari candle terakhir dengan jarak minimal tertentu.
strategies-condition-vwap-param-lookback = Periode Lookback
    .description = Jumlah candle yang dicakup rata-rata tertimbang volume
strategies-condition-vwap-param-position = Posisi
    .description = Posisi harga terhadap VWAP
strategies-condition-vwap-param-position-option-above = Di Atas VWAP
strategies-condition-vwap-param-position-option-below = Di Bawah VWAP
strategies-condition-vwap-param-distance-pct = Jarak %
    .description = Jarak minimum dari VWAP; 0 berarti jarak berapa pun di sisi yang dipilih

strategies-condition-volatility-range = Rentang Volatilitas
    .description = Aktif saat average true range candle terakhir, sebagai persentase harga live, berada di atas atau di bawah ambang. Dua kondisi ini dalam satu grup membentuk rentang.
strategies-condition-volatility-range-param-period = Periode ATR
    .description = Jumlah candle untuk menghaluskan average true range; kondisi butuh satu candle lebih banyak
strategies-condition-volatility-range-param-comparison = Perbandingan
    .description = Apakah rentang harus di atas atau di bawah ambang
strategies-condition-volatility-range-param-comparison-option-above = Di Atas
strategies-condition-volatility-range-param-comparison-option-below = Di Bawah
strategies-condition-volatility-range-param-threshold-pct = Ambang Rentang %
    .description = Average true range sebagai persentase harga live

strategies-condition-bollinger-bands = Bollinger Bands
    .description = Aktif saat harga live di bawah band bawah atau di atas band atas, atau saat band menyempit. Band berada beberapa deviasi standar di sekitar rata-rata penutupan.
strategies-condition-bollinger-bands-param-period = Periode
    .description = Jumlah candle yang dicakup rata-rata dan deviasi standar
strategies-condition-bollinger-bands-param-std-dev = Deviasi Standar
    .description = Berapa deviasi standar jarak setiap band dari rata-rata
strategies-condition-bollinger-bands-param-signal = Sinyal
    .description = Apa yang harus ditunjukkan band
strategies-condition-bollinger-bands-param-signal-option-below-lower = Di Bawah Band Bawah
strategies-condition-bollinger-bands-param-signal-option-above-upper = Di Atas Band Atas
strategies-condition-bollinger-bands-param-signal-option-squeeze = Squeeze
strategies-condition-bollinger-bands-param-squeeze-width-pct = Lebar Squeeze %
    .description = Untuk squeeze: jarak terbesar antar band, sebagai persentase rata-rata

strategies-condition-liquidity-to-size = Likuiditas terhadap Ukuran
    .description = Aktif saat cadangan pool minimal atau kurang dari kelipatan ukuran yang dipertaruhkan: ukuran trade pada strategi masuk, nilai sisa posisi pada strategi keluar.
strategies-condition-liquidity-to-size-param-comparison = Perbandingan
    .description = Apakah cadangan harus minimal atau kurang dari kelipatan
strategies-condition-liquidity-to-size-param-comparison-option-at-least = Minimal (≥)
strategies-condition-liquidity-to-size-param-comparison-option-below = Kurang Dari ({ "<" })
strategies-condition-liquidity-to-size-param-multiple = Kelipatan Likuiditas
    .description = Cadangan pool dibagi ukuran yang dipertaruhkan; kelipatan N menggerakkan harga sekitar 1/(N+1)

strategies-condition-position-adds = Penambahan Posisi
    .description = Aktif saat jumlah pembelian tambahan setelah entri pertama, atau penjualan parsial, minimal atau maksimal suatu angka.
strategies-condition-position-adds-param-subject = Dihitung
    .description = Aksi mana yang dihitung
strategies-condition-position-adds-param-subject-option-adds = Pembelian Tambahan
strategies-condition-position-adds-param-subject-option-partial-sells = Penjualan Parsial
strategies-condition-position-adds-param-comparison = Perbandingan
    .description = Apakah jumlah harus minimal atau maksimal angka tersebut
strategies-condition-position-adds-param-comparison-option-at-least = Minimal (≥)
strategies-condition-position-adds-param-comparison-option-at-most = Maksimal (≤)
strategies-condition-position-adds-param-count = Jumlah
    .description = Jumlah aksi

strategies-condition-time-of-day = Waktu dalam Hari
    .description = Aktif saat jam sekarang berada dalam jendela jam UTC. Awal setelah akhir melewati tengah malam.
strategies-condition-time-of-day-param-start-hour = Jam Mulai (UTC)
    .description = Jam pertama jendela, dari 0 sampai 23
strategies-condition-time-of-day-param-end-hour = Jam Akhir (UTC)
    .description = Jam berakhirnya jendela, dari 1 sampai 24; jam itu sendiri di luar jendela

strategies-condition-liquidity-change-since-entry = Perubahan Likuiditas Sejak Masuk
    .description = Aktif saat cadangan pool turun atau naik setidaknya suatu persentase sejak posisi dibuka. Posisi tanpa likuiditas masuk tercatat membuat kondisi tidak terputuskan.
strategies-condition-liquidity-change-since-entry-param-comparison = Perbandingan
    .description = Apakah cadangan harus turun atau naik
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = Turun Sebesar
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = Naik Sebesar
strategies-condition-liquidity-change-since-entry-param-change-pct = Perubahan %
    .description = Perubahan cadangan pool sejak masuk

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
strategies-error-stale-market-data = Jumlah transaksi { $window } sudah usang: usianya { $age } dtk melebihi { $max } dtk
strategies-error-unusable-data = Nilai { $field } "{ $value }" tidak dapat digunakan
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
strategies-error-field-moving-average = moving average
strategies-error-field-reference-candle-close = close candle acuan
strategies-error-field-peak-price = harga puncak
strategies-error-field-slow-period = periode lambat
strategies-error-field-window = jendela
strategies-error-field-within-candles = jumlah candle persilangan
strategies-error-field-change = perubahan
strategies-error-field-end-hour = jam akhir
strategies-error-field-entry-liquidity = likuiditas masuk
strategies-error-field-giveback = pengembalian
strategies-error-field-highest-high = tertinggi
strategies-error-field-long-window-transactions = transaksi jendela panjang
strategies-error-field-maximum-drop = penurunan maksimum
strategies-error-field-minimum-drop = penurunan minimum
strategies-error-field-multiple = kelipatan
strategies-error-field-pool-liquidity = likuiditas pool
strategies-error-field-ratio = rasio
strategies-error-field-remaining-position-value = nilai sisa posisi
strategies-error-field-signal = sinyal
strategies-error-field-squeeze-width = lebar squeeze
strategies-error-field-standard-deviations = deviasi standar
strategies-error-field-start-hour = jam mulai
strategies-error-field-subject = yang dihitung
strategies-error-field-token-creation-time = waktu pembuatan token
strategies-error-field-total-volume = volume total
strategies-error-field-trade-size = ukuran trade
strategies-error-field-vwap = VWAP

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
strategies-error-data-entry-liquidity = Likuiditas masuk
strategies-error-data-entry-pool-liquidity = Likuiditas pool masuk
strategies-error-data-real-pool-reserves = Likuiditas riil pool
strategies-error-data-remaining-position-value = Nilai sisa posisi
strategies-error-data-token-creation-time = Waktu pembuatan token
strategies-error-data-trade-size = Ukuran trade

strategies-error-indicator-consecutive-candles = candle berurutan
strategies-error-indicator-moving-average = moving average
strategies-error-indicator-price-breakout = breakout harga
strategies-error-indicator-price-change-lookback = lookback perubahan harga
strategies-error-indicator-volume-spike = lonjakan volume
strategies-error-indicator-moving-average-cross = persilangan moving average
strategies-error-indicator-rsi = RSI
strategies-error-indicator-bollinger-bands = Bollinger bands
strategies-error-indicator-pullback-from-high = koreksi dari tertinggi
strategies-error-indicator-volatility-range = rentang volatilitas
strategies-error-indicator-vwap = VWAP

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

strategies-readiness-history-since-met =
    { $count ->
       *[other] { $count } putaran paper tertutup sejak aturan diubah pada { $changed }, dibutuhkan { $needed }
    }
strategies-readiness-history-since-short = { $count } dari { $needed } putaran paper tertutup sejak aturan diubah pada { $changed }
strategies-readiness-priced = Setiap posisi memiliki harga
strategies-readiness-priced-ok = Setiap posisi paper yang terbuka memiliki harga pool
strategies-readiness-priced-missing =
    { $count ->
       *[other] { $count } posisi paper terbuka tanpa harga pool
    }
strategies-readiness-runtime = Trading live tersedia
strategies-readiness-runtime-ok = Penyiapan dan gerbang keamanan mengizinkan trading live

## Roster: the strategy list, one view per kind. Status and notice ids come from
## src/strategies/overview.rs.

strategies-back = Semua strategi
strategies-view-kind = Jenis strategi
strategies-view-all = Semua
strategies-search-placeholder = Cari strategi...
strategies-unnamed = Strategi tanpa nama
strategies-col-on = Aktif
strategies-col-strategy = Strategi
strategies-col-rank = Peringkat
    .title = Peringkat evaluasi di antara strategi entry aktif dalam satu mode, Urutan terendah lebih dulu; yang pertama memberi sinyal mengambil token
strategies-col-mode = Mode
strategies-col-status = Status
strategies-col-open = Terbuka
strategies-col-watching = Dipantau
strategies-col-kind = Jenis
strategies-col-rounds = Putaran
strategies-col-closed = Ditutup
strategies-col-win-rate = Rasio menang
strategies-col-realized = Terealisasi ({ -sol })
strategies-col-manages = Mengelola
strategies-col-last-signal = Sinyal terakhir
strategies-status-running = Berjalan
strategies-status-not-running = Tidak berjalan
strategies-status-off = Mati
strategies-status-setup-required = Perlu penyiapan
strategies-status-live-paused = Entry live dijeda
strategies-status-paper-cash = Kas paper kurang
strategies-status-slots-full = Slot penuh
strategies-status-entry-monitor-off = Monitor entry mati
strategies-status-exit-monitor-off = Monitor exit mati
strategies-notice-setup-required = Trading paper dimulai setelah penyiapan
strategies-notice-force-stopped = Penghentian darurat aktif
strategies-notice-trader-off = Auto Trader mati
strategies-notice-waiting = Menunggu layanan inti
strategies-notice-monitors-off = Monitor entry dan exit mati
strategies-notice-start = Mulai Auto Trader
# $closed and $needed count closed paper rounds; $check is the title of a readiness check.
strategies-readiness-progress = { $closed }/{ $needed } putaran
strategies-readiness-ready = Siap untuk live
strategies-readiness-failing = Belum terpenuhi: { $check }
# $book is the trading mode of the other book; $count its open positions.
strategies-open-other-book =
    { $count ->
       *[other] { $book }: { $count } posisi
    }
strategies-manages-all = Semua strategi entry
# $first is the first managed entry strategy; $more the count of the others.
strategies-manages-some =
    { $more ->
       *[other] { $first } dan { $more } lainnya
    }
strategies-exit-before = Diperiksa lebih dulu
strategies-exit-hint = Aturan exit ini diperiksa pada setiap posisi sebelum strategi exit mana pun
strategies-action-edit = Edit { $name }
strategies-action-delete = Hapus { $name }
strategies-roster-stale = Pembaruan terakhir

## Saving new rules: the server answers whether a save changed the rules a strategy trades on.

strategies-live-rules-confirm = Ubah aturan “{ $name }” yang live
    .message = Aturan baru berdagang dengan { -sol } sungguhan mulai pemeriksaan berikutnya, dan bukti paper strategi ini dimulai ulang dari penyimpanan ini.
strategies-live-rules-save = Simpan dan trading live
strategies-toast-evidence-restarted = Strategi Disimpan
    .message = Aturan “{ $name }” berubah; bukti paper-nya dimulai ulang dari sekarang

## Trading mode: arming a paper strategy live and returning it to paper.

strategies-mode-arm = Aktifkan live
strategies-mode-return-paper = Kembali ke Paper
strategies-mode-arm-title = Aktifkan “{ $name }” secara live
strategies-mode-arm-lead = Entri baru “{ $name }” akan bertransaksi dengan swap sungguhan dari dompet trading Anda.
strategies-mode-arm-lead-exit = “{ $name }” akan menjual posisi live dengan swap sungguhan dari wallet trading Anda saat memberi sinyal.
strategies-mode-arm-readiness = Kesiapan dari buku paper
strategies-mode-arm-open-paper = Posisi paper yang terbuka tetap paper dan selesai di paper.
strategies-mode-arm-ack-real-native = Entri baru membelanjakan { -sol } sungguhan dari dompet trading Anda.
strategies-mode-arm-ack-real-native-exit = Posisi live yang dikelolanya dijual dengan swap sungguhan.
strategies-mode-arm-ack-fees = Swap live membayar biaya jaringan dan slippage yang nyata; hasil paper tidak menjamin hasil live.
strategies-mode-arm-ack-unready = Beberapa pemeriksaan kesiapan belum lolos. Tetap aktifkan strategi ini.
strategies-mode-arm-unavailable = Trading live tidak tersedia saat ini; lihat pemeriksaan terakhir.
strategies-mode-return-title = Kembalikan “{ $name }” ke Paper
strategies-mode-return-message = Entri baru dibuka di buku paper tanpa membelanjakan { -sol }. Posisi live yang terbuka tetap live dan selesai secara live.
strategies-mode-return-message-exit = Strategi ini berhenti menilai posisi live; posisi tersebut tetap memakai aturan exit-nya.
strategies-mode-readiness-failed = Kesiapan strategi tidak dapat dibaca
strategies-mode-toast-armed = { $name } sekarang trading live
strategies-mode-toast-paper = { $name } sekarang trading di Paper
strategies-mode-toast-failed = Mode trading tidak dapat diubah
