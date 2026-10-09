# Tools page: the shell, the token tools, the trading tools, the wallet tools and the
# multi-wallet tools.

## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = Alat
tools-category-wallet = Dompet
tools-category-token = Token
tools-category-single-token = Token Tunggal
tools-category-utilities = Utilitas
tools-sidebar-hint = Pilih alat untuk memulai
tools-help-button =
    .aria-label = Tampilkan bantuan untuk alat ini
tools-help-unavailable = Bantuan tidak tersedia
tools-placeholder-title = Pilih Alat
tools-placeholder-subtitle = Pilih alat dari sidebar untuk memulai
tools-placeholder-hint-wallets = Alat dompet membantu mengelola dompet Solana Anda
tools-placeholder-hint-secure = Semua operasi diamankan dan dapat dibatalkan bila memungkinkan

tools-status-ready = Siap digunakan
tools-status-coming = Segera hadir
tools-status-beta = Beta - mungkin ada bug
tools-status-disabled = Sedang dinonaktifkan
tools-status-badge-coming = Segera Hadir
tools-status-badge-beta = Beta
tools-toast-coming-soon = Alat ini segera hadir
tools-toast-disabled = Alat ini sedang dinonaktifkan
tools-setup-gate-title = Alat ini memerlukan dompet

## Tool names.

tools-tool-wallet-cleanup-title = Pembersihan Dompet
tools-tool-wallet-cleanup-summary = Tutup ATA kosong
tools-tool-wallet-cleanup-description = Tutup Associated Token Account kosong untuk mengklaim kembali { -sol }
tools-tool-burn-tokens-title = Burn Token
tools-tool-burn-tokens-summary = Hancurkan token secara permanen
tools-tool-burn-tokens-description = Hancurkan token dari dompet Anda secara permanen
tools-tool-token-analyzer-title = Penganalisis Token
tools-tool-token-analyzer-summary = Analisis token mendalam
tools-tool-token-analyzer-description = Analisis mendalam token Solana apa pun dengan wawasan multidimensi
tools-tool-create-token-title = Buat Token
tools-tool-create-token-summary = Deploy token SPL baru
tools-tool-create-token-description = Deploy token SPL baru di Solana
tools-tool-trade-watcher-title = Trade Watcher
tools-tool-trade-watcher-summary = Pantau trade & aksi otomatis
tools-tool-trade-watcher-description = Pantau trade token dan picu aksi beli/jual otomatis
tools-tool-token-watch-title = Holder Watch
tools-tool-token-watch-summary = Lacak holder token baru
tools-tool-token-watch-description = Lacak dan pantau holder token baru secara real-time
tools-tool-buy-multi-wallets-title = Multi-beli
tools-tool-buy-multi-wallets-summary = Koordinasikan pembelian lintas dompet
tools-tool-buy-multi-wallets-description = Jalankan order beli terkoordinasi di beberapa dompet dengan jumlah acak
tools-tool-sell-multi-wallets-title = Multi-jual
tools-tool-sell-multi-wallets-summary = Koordinasikan penjualan lintas dompet
tools-tool-sell-multi-wallets-description = Jalankan order jual terkoordinasi di beberapa dompet dengan konsolidasi { -sol }
tools-tool-wallet-consolidation-title = Konsolidasi Dompet
tools-tool-wallet-consolidation-nav-title = Konsolidasi
tools-tool-wallet-consolidation-summary = Konsolidasikan dana dompet
tools-tool-wallet-consolidation-description = Konsolidasikan { -sol } dan token dari sub-dompet kembali ke dompet utama
tools-tool-airdrop-checker-title = Pemeriksa Airdrop
tools-tool-airdrop-checker-summary = Periksa airdrop tertunda
tools-tool-airdrop-checker-description = Periksa airdrop tertunda dan reward yang dapat diklaim
tools-tool-wallet-generator-title = Generator Dompet
tools-tool-wallet-generator-summary = Buat keypair baru
tools-tool-wallet-generator-description = Buat keypair Solana baru dengan aman

## Shared by the tools

tools-validation-mint-required = Masukkan alamat mint token
tools-validation-mint-format = Format alamat mint token tidak valid
tools-validation-mint-invalid = Masukkan alamat mint yang valid

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = Detail Token
tools-create-token-name-label = Nama Token
tools-create-token-name-input =
    .placeholder = Token Saya
tools-create-token-symbol-label = Simbol
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = Desimal
tools-create-token-supply-label = Suplai Awal
tools-create-token-description-label = Deskripsi
tools-create-token-description-input =
    .placeholder = Deskripsi token...
tools-create-token-image-title = Gambar Token
tools-create-token-image-drop = Letakkan gambar di sini atau klik untuk mengunggah
tools-create-token-image-hint = Disarankan: PNG 512x512
tools-create-token-action-preview = Pratinjau
tools-create-token-action-create = Buat Token

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = Memuat pengaturan...
tools-holder-watch-saved = Pengaturan Holder Watch disimpan
tools-holder-watch-save-failed = Gagal menyimpan pengaturan
tools-holder-watch-save-error = Kesalahan saat menyimpan pengaturan
tools-holder-watch-settings-title = Pengaturan Holder Watch
tools-holder-watch-enabled-label = Aktifkan Pemantauan Holder
tools-holder-watch-interval-label = Interval Pemeriksaan
tools-holder-watch-interval-hint = Seberapa sering jumlah holder diperiksa (10-3600 dtk)
tools-holder-watch-max-tokens-label = Maks. Token Dipantau
tools-holder-watch-max-tokens-hint = Jumlah maksimum token yang dipantau bersamaan
tools-holder-watch-notify-new-label = Beri Notifikasi saat Ada Holder Baru
tools-holder-watch-notify-drop-label = Beri Notifikasi saat Holder Turun
tools-holder-watch-min-change-label = Perubahan Holder Minimum
tools-holder-watch-min-change-hint = Perubahan holder minimum untuk memicu notifikasi
tools-holder-watch-drop-percent-label = Ambang Penurunan Holder
tools-holder-watch-drop-percent-hint = Persentase penurunan untuk memicu peringatan
tools-holder-watch-action-save = Simpan Pengaturan
tools-holder-watch-tokens-title = Token Dipantau
tools-holder-watch-token-input =
    .placeholder = Masukkan alamat mint token...
tools-holder-watch-empty = Belum ada token yang dipantau
tools-holder-watch-empty-hint = Tambahkan alamat mint token di atas untuk mulai memantau
tools-holder-watch-coming-soon = Fitur pemantauan token segera hadir

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = Analisis Token
tools-analyzer-mint-input =
    .placeholder = Tempel alamat mint token...
tools-analyzer-action-analyze = Analisis
tools-analyzer-action-analyzing = Menganalisis...
tools-analyzer-action-copy-report = Salin Laporan
tools-analyzer-loading = Menganalisis token...
tools-analyzer-failed = Gagal menganalisis token
tools-analyzer-empty = Masukkan alamat mint token untuk dianalisis
tools-analyzer-empty-hint = Dapatkan wawasan lengkap tentang token Solana apa pun
tools-analyzer-tab-overview = Ringkasan
tools-analyzer-tab-security = Keamanan
tools-analyzer-tab-market = Pasar
tools-analyzer-tab-liquidity = Likuiditas
tools-analyzer-unknown-token = Token Tidak Dikenal

tools-analyzer-favorite-add =
    .title = Tambah ke Favorit
    .aria-label = Tambah ke Favorit
tools-analyzer-favorite-already = Sudah ada di Favorit
tools-analyzer-favorite-added = { $symbol } ditambahkan ke favorit
tools-analyzer-favorite-failed = Gagal menambahkan ke favorit
tools-analyzer-blacklist-add =
    .title = Tambah ke Daftar Hitam
    .aria-label = Tambah ke Daftar Hitam
tools-analyzer-blacklist-title = Masukkan Token ke Daftar Hitam
tools-analyzer-blacklist-message = Masukkan { $symbol } ke daftar hitam? Token ini akan dikecualikan dari trading.
tools-analyzer-blacklist-confirm = Daftar Hitamkan
tools-analyzer-blacklisted = Masuk Daftar Hitam
tools-analyzer-blacklist-done = { $symbol } dimasukkan ke daftar hitam
tools-analyzer-blacklist-failed = Gagal memasukkan token ke daftar hitam

tools-analyzer-card-quick-stats = Statistik Cepat
tools-analyzer-card-market-summary = Ringkasan Pasar
tools-analyzer-card-token-info = Informasi Token
tools-analyzer-stat-holders = Holder
tools-analyzer-stat-decimals = Desimal
tools-analyzer-stat-safety-score = Skor Keamanan
tools-analyzer-stat-pools = Pool
tools-analyzer-stat-volume-24h = Volume 24h
tools-analyzer-stat-change-24h = Perubahan 24h
tools-analyzer-stat-market-cap = Kapitalisasi Pasar
tools-analyzer-stat-liquidity = Likuiditas
tools-analyzer-info-mint = Alamat Mint
tools-analyzer-info-description = Deskripsi
tools-analyzer-info-supply = Suplai

tools-analyzer-security-empty = Data keamanan tidak tersedia
tools-analyzer-security-empty-hint = Analisis keamanan tidak tersedia untuk token ini
tools-analyzer-card-safety-score = Skor Keamanan
tools-analyzer-score-good = Baik
tools-analyzer-score-moderate = Sedang
tools-analyzer-score-risky = Berisiko
tools-analyzer-raw-score = Skor Risiko Mentah: { $score }
tools-analyzer-card-authorities = Otoritas Token
tools-analyzer-authority-mint = Otoritas Mint
tools-analyzer-authority-freeze = Otoritas Freeze
tools-analyzer-authority-transfer-fee = Biaya Transfer
tools-analyzer-authority-mutable = Dapat Diubah
tools-analyzer-authority-active = Aktif
tools-analyzer-authority-revoked = Dicabut
tools-analyzer-card-holder-concentration = Konsentrasi Holder
tools-analyzer-top-holders = dipegang oleh 10 holder teratas
tools-analyzer-risks-title = Risiko Keamanan ({ $count })
tools-analyzer-risks-title-none = Risiko Keamanan
tools-analyzer-risks-none = Tidak ada risiko keamanan terdeteksi

tools-analyzer-market-empty = Data pasar tidak tersedia
tools-analyzer-market-empty-hint = Data pasar tidak tersedia untuk token ini
tools-analyzer-card-price = Harga Saat Ini
tools-analyzer-card-price-changes = Perubahan Harga
tools-analyzer-card-volume = Volume Trading
tools-analyzer-card-transactions = Transaksi 24h
tools-analyzer-card-valuation = Valuasi
tools-analyzer-stat-window-1h = 1h
tools-analyzer-stat-window-6h = 6h
tools-analyzer-stat-window-24h = 24h
tools-analyzer-stat-volume-1h = Volume 1h
tools-analyzer-stat-volume-6h = Volume 6h
tools-analyzer-stat-fdv = Nilai Terdilusi Penuh
tools-analyzer-txn-buys = Pembelian
tools-analyzer-txn-sells = Penjualan

tools-analyzer-liquidity-empty = Data likuiditas tidak tersedia
tools-analyzer-liquidity-empty-hint = Tidak ada pool ditemukan untuk token ini
tools-analyzer-card-total-liquidity = Total Likuiditas
tools-analyzer-card-pools = Pool
tools-analyzer-active-pools =
    { $count ->
       *[other] Pool Aktif
    }
tools-analyzer-card-pool-details = Detail Pool
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = Likuiditas ({ -sol })
tools-analyzer-pools-column-status = Status
tools-analyzer-pool-primary = Utama

tools-analyzer-report-empty = Tidak ada analisis untuk disalin
tools-analyzer-report-label = Laporan analisis
tools-analyzer-report-title = Laporan Analisis Token
tools-analyzer-report-token = Token: { $symbol } ({ $name })
tools-analyzer-report-mint = Mint: { $mint }
tools-analyzer-report-price = Harga: { $sol }
tools-analyzer-report-price-with-usd = Harga: { $sol } ({ $usd })
tools-analyzer-report-security = Keamanan:
tools-analyzer-report-safety-score = - Skor Keamanan: { $score }/100
tools-analyzer-report-mint-authority = - Otoritas Mint: { $state }
tools-analyzer-report-freeze-authority = - Otoritas Freeze: { $state }
tools-analyzer-report-risks = - Risiko: { $count }
tools-analyzer-report-market = Pasar:
tools-analyzer-report-volume = - Volume 24h: { $amount }
tools-analyzer-report-change = - Perubahan 24h: { $amount }
tools-analyzer-report-market-cap = - Kapitalisasi Pasar: { $amount }
tools-analyzer-report-liquidity = Likuiditas:
tools-analyzer-report-liquidity-total = - Total: { $amount }
tools-analyzer-report-pools = - Pool: { $count }
tools-analyzer-report-generated = Dibuat: { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

tools-watch-type-buy-on-sell = Beli saat Jual
tools-watch-type-sell-on-buy = Jual saat Beli
tools-watch-type-notify = Notifikasi
tools-watch-type-notify-only = Hanya Notifikasi

tools-trade-watcher-setup-title = Atur Pemantauan
tools-trade-watcher-mint-label = Alamat Mint Token
tools-trade-watcher-mint-input =
    .placeholder = Masukkan alamat mint token...
tools-trade-watcher-action-search-pools = Cari Pool
tools-trade-watcher-pool-label = Pool Terpilih
tools-trade-watcher-pool-none = Belum ada pool dipilih
tools-trade-watcher-pool-clear =
    .title = Hapus pool
tools-trade-watcher-pool-selected = Pool terpilih: { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = Jenis Pemantauan
tools-trade-watcher-type-hint = Beli saat Jual: beli otomatis saat ada yang menjual. Jual saat Beli: jual otomatis saat ada yang membeli.
tools-trade-watcher-trigger-label = Jumlah Pemicu
tools-trade-watcher-trigger-hint = Ukuran trade minimum dalam { -sol } untuk memicu aksi
tools-trade-watcher-action-amount-label = Jumlah Aksi
tools-trade-watcher-action-amount-hint = Jumlah yang dibeli/dijual saat terpicu
tools-trade-watcher-slippage-label = Slippage
tools-trade-watcher-slippage-hint = Slippage maksimum yang dapat diterima untuk trade
tools-trade-watcher-active-title = Pemantauan Aktif
tools-trade-watcher-empty = Tidak ada pemantauan aktif
tools-trade-watcher-empty-hint = Atur pemantauan di atas lalu klik "Mulai Pantau" untuk mulai memantau
tools-trade-watcher-action-start = Mulai Pantau
tools-trade-watcher-action-starting = Memulai...
tools-trade-watcher-action-stop-all = Hentikan Semua
tools-trade-watcher-action-stopping = Menghentikan...
tools-trade-watcher-started = Pemantauan dimulai untuk { $token }...
tools-trade-watcher-start-failed = Gagal memulai pemantauan
tools-trade-watcher-stopped = Pemantauan dihentikan
tools-trade-watcher-stop-failed = Gagal menghentikan pemantauan
tools-trade-watcher-stopped-all = Semua pemantauan dihentikan
tools-trade-watcher-stop-all-failed = Gagal menghentikan pemantauan
tools-trade-watcher-load-failed = Gagal memuat pemantauan
tools-trade-watcher-column-token = Token
tools-trade-watcher-column-type = Jenis
tools-trade-watcher-column-trigger = Pemicu ({ -sol })
tools-trade-watcher-column-action = Aksi ({ -sol })
tools-trade-watcher-column-triggered = Terpicu
tools-trade-watcher-stop-watch =
    .title = Hentikan pemantauan

## Results returned by the tools backend.

tools-burn-failure-native-asset = Tidak dapat membakar { -sol }
tools-burn-failure-open-position = Tidak dapat membakar token dari posisi terbuka
tools-burn-failure-account-not-found = Akun token tidak ditemukan
tools-burn-failure-zero-balance = Saldo token sudah nol
tools-burn-failure-transaction = Transaksi gagal
tools-burn-warning-open-position = Tidak dapat membakar token dari posisi terbuka
tools-burn-warning-closed-position = Sisa dari posisi tertutup
tools-burn-warning-worth = Senilai ~{ $amount } { -sol }
tools-multi-buy-warning-insufficient = Saldo tidak cukup. Butuh { $needed } { -sol }, tersedia { $have } { -sol }
tools-multi-buy-warning-over-limit = Total { -sol } yang dibutuhkan ({ $needed }) melebihi batas ({ $limit })
tools-multi-sell-warning-no-wallets = Dompet sekunder tidak ditemukan
tools-multi-sell-warning-no-balance = Tidak ada dompet yang memiliki saldo token
tools-multi-op-buy-failed = Pembelian gagal
tools-multi-op-sell-failed = Penjualan gagal
tools-multi-op-transfer-failed = Transfer gagal
tools-multi-op-balance-failed = Gagal mendapatkan saldo
tools-multi-op-mint-invalid = Alamat mint tidak valid
tools-multi-buy-session-failed = Multi-beli gagal
tools-multi-sell-session-failed = Multi-jual gagal
tools-multi-session-aborted = Operasi dibatalkan oleh pengguna

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = Pindai Dompet
tools-wallet-action-scanning = Memindai...
tools-wallet-scan-failed = Pemindaian gagal: { $reason }
tools-wallet-amount-approx = ~{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
       *[other] Dipilih: { $count } dompet
    }
tools-wallet-transfer-failed = Transfer gagal: { $reason }
tools-wallet-cleanup-failed = Pembersihan gagal: { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = Hasil Pemindaian
tools-wallet-cleanup-stat-empty = ATA Kosong
tools-wallet-cleanup-stat-reclaimable = { -sol } yang Dapat Diklaim
tools-wallet-cleanup-stat-failed = Gagal (cache)
tools-wallet-cleanup-prompt = Klik "Pindai Dompet" untuk menemukan ATA kosong
tools-wallet-cleanup-prompt-hint = Ini akan memeriksa semua akun token di dompet Anda
tools-wallet-cleanup-action-cleanup = Bersihkan Semua
tools-wallet-cleanup-action-cleaning = Membersihkan...
tools-wallet-cleanup-scanning = Memindai dompet...
tools-wallet-cleanup-found =
    { $count ->
       *[other] Ditemukan { $count } ATA kosong senilai ~{ $amount }
    }
tools-wallet-cleanup-clean = Tidak ada ATA kosong - dompet bersih!
tools-wallet-cleanup-scan-failed = Gagal memindai ATA
tools-wallet-cleanup-done =
    { $count ->
       *[other] { $count } ATA dibersihkan
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = Burn Token
tools-burn-info-title = Apa itu burn?
tools-burn-info-body = Burn menghancurkan token secara permanen sehingga tidak dapat dipulihkan. Setelah burn, jalankan Pembersihan Dompet untuk menutup ATA kosong dan mengklaim kembali rent ~0.002 { -sol } per token.
tools-burn-stat-total = Total Token
tools-burn-stat-selected = Dipilih
tools-burn-stat-rent = Rent yang Dapat Diklaim
tools-burn-prompt = Klik "Pindai Dompet" untuk menemukan token
tools-burn-scanning = Memindai token di dompet...
tools-burn-scan-failed = Gagal memindai token
tools-burn-empty = Tidak ada token ditemukan di dompet
tools-burn-action-burn = Burn yang Dipilih ({ $count })
tools-burn-action-burning = Membakar...
tools-burn-cannot-burn = Tidak dapat di-burn
tools-burn-no-value = Tanpa nilai

tools-burn-category-open-position = Posisi Terbuka
tools-burn-category-has-value = Bernilai
tools-burn-category-closed-position = Posisi Tertutup
tools-burn-category-zero-liquidity = Likuiditas Nol
tools-burn-category-hint-open-position = Tidak dapat membakar token dari posisi terbuka
tools-burn-category-hint-has-value = Pertimbangkan menjual daripada burn
tools-burn-category-hint-closed-position = Sisa dari trade yang sudah ditutup
tools-burn-category-hint-zero-liquidity = Aman di-burn - tanpa nilai pasar

tools-burn-confirm-title = Konfirmasi Burn
tools-burn-confirm-message =
    { $count ->
       *[other] Yakin ingin burn <strong>{ $count }</strong> token?
    }
tools-burn-confirm-value = Total perkiraan nilai: <strong>{ $amount }</strong>
tools-burn-confirm-continue = Lanjutkan
tools-burn-final-title = Peringatan Terakhir
tools-burn-final-headline = Tindakan ini TIDAK DAPAT DIBATALKAN!
tools-burn-final-message =
    { $count ->
       *[other] { $count } token berikut akan dihancurkan secara permanen dan tidak dapat dipulihkan dalam kondisi apa pun.
    }
tools-burn-final-confirm = Ya, Burn Token
tools-burn-toast-burned =
    { $total ->
       *[other] { $successful }/{ $total } token di-burn. Jalankan Pembersihan Dompet untuk mengklaim kembali ~{ $amount }
    }
tools-burn-toast-failed =
    { $count ->
       *[other] { $count } token gagal di-burn
    }
tools-burn-failed = Burn gagal: { $reason }
tools-burn-failures-title =
    { $count ->
       *[other] { $count } token tidak dapat di-burn
    }
tools-burn-failure-unknown = Tidak ada alasan yang dilaporkan

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = Tentang
tools-airdrop-about-body = Periksa airdrop tertunda, reward yang dapat diklaim, dan alokasi yang belum diklaim di berbagai protokol Solana populer.
tools-airdrop-list-title = Airdrop Tersedia
tools-airdrop-prompt = Klik "Periksa Airdrop" untuk memindai klaim yang tersedia
tools-airdrop-action-check = Periksa Airdrop
tools-airdrop-action-claim-all = Klaim Semua

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = Opsi Generator
tools-generator-warning-title = Simpan private key Anda dengan aman!
tools-generator-warning-body = Keypair yang dibuat dihasilkan secara lokal dan tidak pernah dikirim. Selalu cadangkan key Anda di tempat yang aman.
tools-generator-count-label = Jumlah Dompet
tools-generator-vanity-label = Alamat Vanity (diawali karakter tertentu)
tools-generator-prefix-label = Prefiks
tools-generator-prefix-input =
    .placeholder = mis., SOL
tools-generator-prefix-hint = Prefiks yang lebih panjang membutuhkan waktu pembuatan yang jauh lebih lama
tools-generator-list-title = Dompet yang Dibuat
tools-generator-empty = Belum ada dompet yang dibuat
tools-generator-action-generate = Buat
tools-generator-action-generating = Membuat...
tools-generator-count-invalid = Masukkan angka antara 1 dan 10
tools-generator-no-keypairs = Tidak ada keypair yang dikembalikan
tools-generator-generated =
    { $count ->
       *[other] { $count } dompet dibuat
    }
tools-generator-failed = Gagal membuat dompet: { $reason }
tools-generator-copy-public-key =
    .title = Salin public key
tools-generator-copy-private-key =
    .title = Salin private key
tools-generator-remove =
    .title = Hapus dari daftar
tools-generator-reveal =
    .title = Tampilkan private key
tools-generator-public-key-label = Public Key:
tools-generator-private-key-label = Private Key:
tools-generator-public-key-name = Public key
tools-generator-private-key-copied = Private key disalin
tools-generator-private-key-warning = Siapa pun yang memiliki key ini mengendalikan dompet
tools-generator-export-empty = Tidak ada dompet untuk diekspor
tools-generator-exported = Dompet diekspor - simpan dengan aman

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = Ringkasan
tools-consolidation-stat-wallets = Sub-dompet
tools-consolidation-stat-native = Total { -sol }
tools-consolidation-stat-tokens = Jenis Token
tools-consolidation-stat-rent = Rent yang Dapat Diklaim
tools-consolidation-wallets-title = Dompet
tools-consolidation-loading-wallets = Memuat dompet...
tools-consolidation-loading-data = Memuat data dompet...
tools-consolidation-action-transfer-native = Transfer { -sol }
tools-consolidation-action-transfer-tokens = Transfer Semua Token
tools-consolidation-action-cleanup = Bersihkan ATA
tools-consolidation-action-transferring = Mentransfer...
tools-consolidation-column-name = Nama
tools-consolidation-column-native = Saldo ({ -sol })
tools-consolidation-column-tokens = Token
tools-consolidation-column-atas = ATA Kosong
tools-consolidation-empty = Sub-dompet tidak ditemukan
tools-consolidation-empty-hint = Buat sub-dompet dengan Multi-beli untuk memulai
tools-consolidation-load-failed = Gagal memuat: { $reason }
tools-consolidation-select-prompt = Pilih dompet untuk dikonsolidasikan
tools-consolidation-selection-totals =
    | { $amount } | { $tokens ->
       *[other] { $tokens } token
    } | { $atas ->
       *[other] { $atas } ATA kosong
    }
tools-consolidation-transferred-native = { $amount } ditransfer ke dompet utama
tools-consolidation-transferred-tokens =
    { $count ->
       *[other] { $count } token ditransfer ke dompet utama
    }
tools-consolidation-cleaned =
    { $count ->
       *[other] { $count } ATA ditutup, { $amount } diklaim kembali
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = Token
tools-multi-mint-label = Alamat Mint Token
tools-multi-mint-input =
    .placeholder = Tempel alamat mint token...
tools-multi-execution-title = Pengaturan Eksekusi
tools-multi-delay-min-label = Jeda Min
tools-unit-native = { -sol }
tools-unit-seconds = dtk
tools-unit-ms = ms
tools-multi-delay-max-label = Jeda Maks
tools-multi-concurrency-label = Konkurensi
tools-multi-concurrency-sequential = { $count } (Berurutan)
tools-multi-concurrency-parallel = { $count } paralel
tools-multi-slippage-label = Slippage
tools-multi-router-label = Router
tools-multi-router-auto = Otomatis (Rute Terbaik)
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = Pool Langsung
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = Progres
tools-multi-progress-preparing = Menyiapkan...
tools-multi-status-line = { $label } ({ $completed }/{ $total })
tools-multi-column-wallet = Dompet
tools-multi-column-route = Rute
tools-multi-column-status = Status
tools-multi-op-completed = Selesai
tools-multi-op-failed = Gagal
tools-multi-action-stop = Hentikan
tools-multi-action-loading = Memuat...
tools-multi-start-failed = Gagal memulai: { $reason }

tools-multi-state-pending = Menunggu
tools-multi-state-funding = Pendanaan
tools-multi-state-executing = Mengeksekusi
tools-multi-state-consolidating = Mengonsolidasi
tools-multi-state-completed = Selesai
tools-multi-state-failed = Gagal
tools-multi-state-aborted = Dibatalkan

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = Token yang ingin Anda beli di beberapa dompet
tools-multi-buy-wallets-title = Pengaturan Dompet
tools-multi-buy-wallet-count-label = Jumlah Dompet
tools-multi-buy-wallet-count-option =
    { $count ->
       *[other] { $count } dompet
    }
tools-multi-buy-wallet-count-hint = Jumlah sub-dompet yang digunakan
tools-multi-buy-buffer-label = Buffer { -sol } per Dompet
tools-multi-buy-buffer-hint = Dicadangkan untuk biaya (min. 0.015 { -sol })
tools-multi-buy-amounts-title = Pengaturan Jumlah
tools-multi-buy-min-label = { -sol } Min per Dompet
tools-multi-buy-min-hint = Jumlah beli minimum
tools-multi-buy-max-label = { -sol } Maks per Dompet
tools-multi-buy-max-hint = Jumlah beli maksimum
tools-multi-buy-limit-label = Batas Total { -sol } (opsional)
tools-multi-buy-limit-hint = Total pengeluaran maksimum
tools-multi-buy-preview-title = Pratinjau
tools-multi-buy-preview-create = Dompet yang Akan Dibuat
tools-multi-buy-preview-amount = Jumlah per Dompet
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = Total { -sol } yang Dibutuhkan
tools-multi-buy-preview-balance = Saldo Utama
tools-multi-buy-action-preview = Pratinjau
tools-multi-buy-action-start = Mulai Multi-beli
tools-multi-buy-executing = Mengeksekusi pembelian...
tools-multi-buy-column-spent = Terpakai ({ -sol })
tools-multi-buy-column-tokens = Token
tools-multi-buy-preview-failed = Pratinjau gagal: { $reason }
tools-multi-buy-started = Multi-beli dimulai
tools-multi-buy-stopped = Multi-beli dihentikan
tools-multi-buy-completed = Multi-beli selesai! { $successful }/{ $total } berhasil

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = Masukkan alamat token untuk memindai dompet yang memilikinya
tools-multi-sell-action-scan = Pindai
tools-multi-sell-settings-title = Pengaturan Jual
tools-multi-sell-percent-label = Persentase Jual
tools-multi-sell-percent-hint = % token yang dijual per dompet
tools-multi-sell-min-fee-label = { -sol } Min untuk Biaya
tools-multi-sell-min-fee-hint = { -sol } minimum yang dibutuhkan untuk biaya tx
tools-multi-sell-topup-label = Isi ulang otomatis jika perlu
tools-multi-sell-topup-hint = Transfer { -sol } dari dompet utama jika saldo sub-dompet tidak cukup
tools-multi-sell-post-title = Aksi Setelah Jual
tools-multi-sell-consolidate-label = Konsolidasikan { -sol } ke dompet utama
tools-multi-sell-consolidate-hint = Transfer semua { -sol } dari sub-dompet kembali ke dompet utama
tools-multi-sell-close-atas-label = Tutup ATA token setelah jual
tools-multi-sell-close-atas-hint = Klaim kembali ~0.002 { -sol } per ATA
tools-multi-sell-wallets-title = Dompet dengan Token
tools-multi-sell-empty = Tidak ada sub-dompet yang memiliki token ini
tools-multi-sell-column-tokens = Token
tools-multi-sell-column-native = Saldo ({ -sol })
tools-multi-sell-column-topup = Perlu Isi Ulang
tools-multi-sell-none-selected = Belum ada dompet dipilih
tools-multi-sell-select-required = Pilih setidaknya satu dompet
tools-multi-sell-action-start = Mulai Multi-jual
tools-multi-sell-executing = Mengeksekusi penjualan...
tools-multi-sell-column-sold = Token Terjual
tools-multi-sell-column-received = Diterima ({ -sol })
tools-multi-sell-started = Multi-jual dimulai
tools-multi-sell-stopped = Multi-jual dihentikan
tools-multi-sell-completed = Multi-jual selesai! { $amount } diterima

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = Favorit
tools-favorites-saved = Favorit Tersimpan
tools-favorites-save-current = Simpan Saat Ini
tools-favorites-empty = Belum ada favorit tersimpan
tools-favorites-no-label = Tanpa label
tools-favorites-uses = { $count }x
tools-favorites-remove = Hapus
tools-favorites-loaded = Favorit dimuat: { $name }
tools-favorites-default-name = Konfigurasi
tools-favorites-mint-required = Masukkan alamat mint token terlebih dahulu
tools-favorites-add-title = Tambah Favorit
tools-favorites-add-message = Masukkan label untuk favorit ini
tools-favorites-add-placeholder = Label (opsional)...
tools-favorites-saved-toast = Disimpan ke favorit
tools-favorites-save-failed = Gagal menyimpan favorit
tools-favorites-remove-title = Hapus Favorit
tools-favorites-remove-message = Hapus favorit ini?
tools-favorites-removed-toast = Favorit dihapus
tools-favorites-remove-failed = Gagal menghapus favorit
