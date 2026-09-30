# Contextual hints: one title and one body per hint, keyed by the hint id.
# Bodies use a small markdown subset (**bold** and bullet lines) that the hint popover renders.

## Categories

hints-category-tokens = Token
hints-category-positions = Posisi
hints-category-filtering = Pemfilteran
hints-category-trader = Auto Trader
hints-category-services = Layanan
hints-category-wallet = Dompet
hints-category-wallets = Dompet
hints-category-tools = Alat
hints-category-config = Konfigurasi
hints-category-config-telegram = { -telegram }
hints-category-token-details = Detail Token
hints-category-ui = Antarmuka

## tokens

hints-tokens-pool-service-title = Token Pool Service
hints-tokens-pool-service-content =
    Token yang ditampilkan di sini telah:

    • **Lolos semua kriteria filter** — pemeriksaan likuiditas, volume, usia, dan keamanan
    • **Memiliki pool SOL yang valid** — didukung oleh decoder DEX kami (Raydium, Orca, Meteora, dll.)
    • **Berhasil dihitung harganya** — harga dihitung langsung dari cadangan pool on-chain

    Ini adalah daftar token paling andal untuk trading karena harga berasal dari data pool sebenarnya, bukan API eksternal.

    Klik token mana pun untuk melihat informasi detail dan mengelola status daftar hitam.
hints-tokens-no-market-title = Tanpa Data Pasar
hints-tokens-no-market-content =
    Token yang ditemukan on-chain tetapi tidak memiliki data pasar dari { -dexscreener } atau { -geckoterminal }.

    Alasan umum:
    • **Token sangat baru** — belum diindeks oleh agregator
    • **Volume trading rendah** — di bawah ambang agregator
    • **Pair tidak terdaftar** — diperdagangkan di DEX yang tidak dilacak agregator

    Token ini mungkin tetap memiliki pool yang valid dan dapat diperdagangkan, tetapi tidak memiliki metrik pasar eksternal.
hints-tokens-all-title = Semua Token
hints-tokens-all-content =
    Database lengkap token yang ditemukan, apa pun status pemfilterannya.

    Mencakup:
    • Token yang lolos filter
    • Token yang ditolak
    • Token tanpa data pasar
    • Token dalam daftar hitam

    Gunakan tampilan ini untuk riset atau untuk menemukan token yang mungkin tersaring.
hints-tokens-passed-title = Lolos Filter
hints-tokens-passed-content =
    Token yang lolos semua kriteria filter yang aktif.

    Pemeriksaan filter meliputi:
    • **Likuiditas** — ambang likuiditas SOL minimum
    • **Volume** — persyaratan volume trading 24h
    • **Usia token** — waktu minimum sejak pembuatan
    • **Keamanan** — batas skor risiko { -rugcheck }
    • **Kapitalisasi pasar** — filter FDV/MC opsional

    Atur filter di halaman **Pemfilteran**.
hints-tokens-rejected-title = Token Ditolak
hints-tokens-rejected-content =
    Token yang gagal memenuhi satu atau lebih kriteria filter.

    Setiap token menampilkan alasan penolakan spesifik:
    • Filter mana yang gagal
    • Nilai aktual dibandingkan ambang yang dibutuhkan
    • Kapan pemeriksaan dilakukan

    Tinjau token yang ditolak untuk menyempurnakan pengaturan filter Anda.
hints-tokens-blacklisted-title = Token Daftar Hitam
hints-tokens-blacklisted-content =
    Token yang dikecualikan secara permanen dari trading.

    Alasan masuk daftar hitam meliputi:
    • **Daftar hitam manual** — token yang Anda blokir sendiri
    • **Risiko keamanan** — indikator rug pull terdeteksi
    • **Ambang kerugian** — melebihi batas kerugian yang dikonfigurasi
    • **Transaksi gagal** — kegagalan swap berulang

    Token dalam daftar hitam tidak pernah ditampilkan di daftar lolos atau dipertimbangkan untuk auto-trading.
hints-tokens-positions-title = Token Posisi
hints-tokens-positions-content =
    Token yang saat ini dipegang dalam posisi terbuka.

    Menampilkan data real-time untuk kepemilikan aktif Anda:
    • Harga saat ini dari cadangan pool
    • P&L belum terealisasi
    • Ukuran posisi dan harga entry
    • Lama dipegang

    Klik token mana pun untuk pengelolaan posisi secara detail.
hints-tokens-recent-title = Baru Ditemukan
hints-tokens-recent-content =
    Token yang baru ditemukan, diurutkan berdasarkan waktu penemuan.

    Berguna untuk:
    • Melihat peluncuran token baru
    • Memantau likuiditas baru
    • Peluang entry awal

    Catatan: token baru mungkin belum memiliki data pasar lengkap pada awalnya.
hints-tokens-ohlcv-title = Manajemen Data OHLCV
hints-tokens-ohlcv-content =
    Lihat dan kelola data OHLCV (candlestick) yang tersimpan untuk token.

    Menampilkan:
    • **Jumlah Candle** — total titik data yang tersimpan
    • **Progres Backfill** — status kelengkapan timeframe
    • **Rentang Data** — cakupan waktu dalam jam
    • **Jumlah Pool** — pool likuiditas yang dilacak
    • **Status** — pemantauan aktif atau nonaktif

    Aksi:
    • **Hapus** — hapus semua data OHLCV untuk token
    • **Bersihkan** — hapus massal data token yang tidak aktif

    Data OHLCV disimpan permanen dan tidak pernah dihapus otomatis.

## positions

hints-positions-overview-title = Ringkasan Posisi
hints-positions-overview-content =
    Kepemilikan token dan posisi trading Anda saat ini.

    Metrik utama:
    • **Harga Entry** — harga rata-rata yang dibayar (termasuk DCA)
    • **Harga Saat Ini** — harga langsung dari cadangan pool
    • **P&L** — untung/rugi belum terealisasi dalam SOL dan %
    • **Ukuran** — total jumlah token yang dipegang

    Klik posisi mana pun untuk opsi pengelolaan detail.
hints-positions-dca-title = DCA (Dollar Cost Average)
hints-positions-dca-content =
    DCA memungkinkan penambahan ke posisi yang ada pada harga berbeda.

    Saat DCA terpicu:
    • Token tambahan dibeli
    • Harga entry dihitung ulang sebagai rata-rata tertimbang
    • Ukuran posisi bertambah
    • Jumlah entry bertambah

    Atur aturan DCA di pengaturan **Auto Trader**.
hints-positions-partial-exit-title = Exit Parsial
hints-positions-partial-exit-content =
    Jual sebagian posisi Anda dan pertahankan sisanya.

    Manfaat:
    • Amankan sebagian profit sambil tetap terpapar
    • Kurangi ukuran posisi tanpa menutup sepenuhnya
    • Terapkan tangga take-profit

    Setiap exit parsial dicatat terpisah untuk pelacakan P&L yang akurat.
hints-positions-management-title = Pengelolaan Posisi
hints-positions-management-content =
    Pengelolaan menentukan otomatisasi mana yang boleh bertindak pada posisi:

    • Auto Trader: exit keamanan, exit kebijakan, dan auto-DCA
    • Hanya Pengguna: tanpa aksi otomatis
    • Tugas Salin: exit keamanan dan copy-sell
    • Hibrida: exit keamanan, exit kebijakan, dan copy-sell

    Anda menjual atau menambah sendiri. Pembelian manual secara default dikelola manual sehingga bot tidak dapat menjual token yang sengaja Anda beli. Nonaktifkan untuk mengembalikan posisi ke auto-trader.

## filtering

hints-filtering-overview-title = Pemfilteran Token
hints-filtering-overview-content =
    Pemfilteran menentukan token mana yang layak diperdagangkan.

    Token harus lolos **semua kriteria yang aktif** agar muncul di daftar lolos:
    • Metrik { -dexscreener } (likuiditas, volume, dll.)
    • Metrik { -geckoterminal } (kapitalisasi pasar, FDV)
    • Analisis keamanan { -rugcheck }
    • Filter meta (usia token, dll.)

    Kriteria yang dinonaktifkan dilewati sepenuhnya.
hints-filtering-dexscreener-title = Filter { -dexscreener }
hints-filtering-dexscreener-content =
    Filter berdasarkan data pasar { -dexscreener }:

    • **Likuiditas** — likuiditas USD minimum di pool
    • **Volume 24h** — volume trading minimum
    • **Transaksi** — ambang aktivitas (beli/jual)
    • **Perubahan Harga** — filter volatilitas

    Data { -dexscreener } diperbarui setiap beberapa menit.
hints-filtering-geckoterminal-title = Filter { -geckoterminal }
hints-filtering-geckoterminal-content =
    Filter berdasarkan data pasar { -geckoterminal }:

    • **Kapitalisasi Pasar** — kapitalisasi pasar minimum
    • **FDV** — batas Fully Diluted Valuation
    • **Rasio Cadangan** — indikator kesehatan pool

    { -geckoterminal } sering memiliki data untuk token yang lebih baru.
hints-filtering-rugcheck-title = Filter Keamanan
hints-filtering-rugcheck-content =
    Analisis keamanan dari { -rugcheck }.xyz:

    • **Skor Risiko** — peringkat risiko keseluruhan (0-100)
    • **Otoritas Mint** — apakah token baru dapat dicetak?
    • **Otoritas Freeze** — apakah transfer dapat dibekukan?
    • **Holder Teratas** — risiko konsentrasi

    Skor risiko yang lebih tinggi menunjukkan lebih banyak potensi red flag.
hints-filtering-meta-title = Filter Meta
hints-filtering-meta-content =
    Kriteria pemfilteran tambahan:

    • **Usia Token** — waktu minimum sejak token dibuat
    • **Usia Pool** — waktu minimum sejak pool dibuat
    • **Punya Website** — wajibkan tautan sosial/website
    • **Punya Sosial** — wajibkan { -twitter }/{ -telegram }

    Ini membantu menyaring token yang sangat baru atau mencurigakan.

## trader

hints-trader-overview-title = Auto Trader
hints-trader-overview-content =
    Mesin trading otomatis yang memantau token dan mengeksekusi trade.

    Komponen:
    • **Monitor Entry** — mengawasi peluang beli
    • **Monitor Exit** — mengelola jual dan take-profit
    • **Monitor DCA** — menangani averaging posisi
    • **Kontrol Risiko** — batas kerugian dan gerbang keamanan

    Mulai/hentikan trading dari panel kontrol.
hints-trader-entry-title = Monitor Entry
hints-trader-entry-content =
    Mengawasi token yang lolos filter untuk sinyal entry.

    Pemeriksaan evaluasi entry:
    • Token lolos pemfilteran saat ini
    • Belum ada dalam posisi
    • Tidak masuk daftar hitam
    • Batas posisi belum terlampaui
    • Kondisi strategi terpenuhi (jika dikonfigurasi)

    Atur ukuran entry dan batas di Konfigurasi.
hints-trader-exit-title = Monitor Exit
hints-trader-exit-content =
    Memantau posisi terbuka untuk sinyal exit.

    Pemicu exit:
    • **Take Profit** — target harga tercapai
    • **Stop Loss** — kerugian maksimum terlampaui
    • **Trailing Stop** — harga turun dari puncak
    • **Exit Strategi** — kondisi kustom terpenuhi
    • **Berbasis Waktu** — durasi hold maksimum

    Atur ambang di Konfigurasi.

## services

hints-services-overview-title = Layanan Sistem
hints-services-overview-content =
    Layanan latar belakang yang menjalankan { -brand }.

    Status layanan:
    • **Berjalan** (hijau) — beroperasi normal
    • **Memulai** (kuning) — sedang inisialisasi
    • **Berhenti** (merah) — tidak berjalan
    • **Error** (peringatan) — gagal, mungkin restart otomatis

    Layanan memiliki dependensi dan dimulai secara berurutan.
hints-services-health-title = Kesehatan Layanan
hints-services-health-content =
    Indikator kesehatan menunjukkan status layanan:

    • **Uptime** — waktu sejak mulai terakhir
    • **Tugas** — operasi latar belakang yang aktif
    • **Error** — jumlah error terbaru
    • **Metrik** — data performa (jika tersedia)

    Layanan kritis memengaruhi kemampuan trading.

## wallet

hints-wallet-overview-title = Ringkasan Dompet
hints-wallet-overview-content =
    Status dompet Solana Anda yang terhubung.

    Menampilkan:
    • **Saldo SOL** — SOL native untuk gas dan trading
    • **Kepemilikan Token** — token SPL beserta nilainya
    • **Perubahan 24h** — perubahan nilai portofolio
    • **Riwayat** — snapshot saldo dari waktu ke waktu

    Saldo diperbarui setiap menit.
hints-wallet-tokens-title = Saldo Token
hints-wallet-tokens-content =
    Token SPL yang dipegang di dompet Anda.

    Menampilkan:
    • Simbol dan nama token
    • Jumlah yang dipegang
    • Nilai saat ini dalam SOL/USD
    • Harga dari pool atau data pasar

    Akun token kosong dapat dibersihkan di Pengaturan.

## wallets

hints-wallets-main-title = Dompet Utama
hints-wallets-main-content =
    Dompet utama yang digunakan untuk semua operasi trading.

    • **Auto-Trading** — trade entry/exit dieksekusi dari dompet ini
    • **Tampilan Saldo** — ditampilkan di header dan dasbor
    • **Kepemilikan Token** — token SPL yang dipegang dompet ini

    Ganti dompet utama dengan memilih "Jadikan Utama" pada dompet sekunder mana pun.
hints-wallets-secondary-title = Dompet Sekunder
hints-wallets-secondary-content =
    Dompet tambahan untuk operasi multi-dompet.

    • **Trading Multi-Dompet** — koordinasikan beli/jual lintas dompet
    • **Pemisahan Portofolio** — atur berdasarkan strategi atau tujuan
    • **Saldo Independen** — setiap dompet memiliki SOL/token sendiri

    Dompet sekunder tidak digunakan oleh auto-trading kecuali dikonfigurasi secara eksplisit.

## tools

hints-tools-wallet-cleanup-title = Alat Pembersihan Dompet
hints-tools-wallet-cleanup-content =
    { "*" }*Klaim Kembali SOL dari Akun Token Kosong**

    { "*" }*Apa itu ATA?**
    Associated Token Account (ATA) adalah akun Solana yang menyimpan token Anda. Setiap token yang Anda gunakan membuat ATA yang membutuhkan rent ~0.002 SOL.

    { "*" }*Mengapa membersihkan ATA kosong?**
    • Klaim kembali rent (~0.002 SOL per ATA)
    • Trader aktif dapat mengumpulkan ratusan ATA kosong
    • 100 ATA kosong = ~0.2 SOL dapat diklaim

    { "*" }*Cara kerja:**
    • Memindai dompet Anda untuk ATA dengan saldo nol
    • Menampilkan total SOL yang dapat diklaim
    • Menutup akun kosong untuk memulihkan rent

    { "*" }*Pembersihan Otomatis:**
    Jika diaktifkan, otomatis memindai dan menutup ATA kosong setiap 5 menit di latar belakang.

    { "*" }*Penting:**
    • Hanya menutup akun dengan saldo tepat 0
    • Penutupan yang gagal disimpan di cache untuk menghindari retry berulang
    • Dompet besar mungkin memerlukan beberapa kali pembersihan
hints-tools-burn-tokens-title = Alat Burn Token
hints-tools-burn-tokens-content =
    { "*" }*Hancurkan Token Secara Permanen**

    Burn token menghapusnya secara permanen dari dompet Anda dan dari peredaran.

    { "*" }*Yang terjadi saat burn:**
    • Token dikirim ke alamat burn (tidak dapat dipulihkan)
    • Saldo token menjadi nol
    • ATA kemudian dapat ditutup lewat Pembersihan Dompet untuk mengklaim kembali rent ~0.002 SOL

    { "*" }*Kategori Token:**
    • **Posisi Terbuka** - Tidak dapat di-burn (trade aktif)
    • **Posisi Tertutup** - Sisa dari trade sebelumnya
    • **Bernilai** - Token dengan likuiditas (pertimbangkan menjual)
    • **Likuiditas Nol** - Token dust/tak bernilai (aman di-burn)

    { "*" }*Peringatan:** Tindakan ini **tidak dapat dibatalkan**. Token yang di-burn tidak dapat dipulihkan dalam kondisi apa pun.

    { "*" }*Setelah burn:** Jalankan Pembersihan Dompet untuk menutup ATA kosong dan mengklaim kembali rent SOL.
hints-tools-wallet-generator-title = Alat Generator Dompet
hints-tools-wallet-generator-content =
    { "*" }*Buat Keypair Solana Baru**

    Buat dompet baru dengan aman di perangkat Anda.

    { "*" }*Fitur:**
    • Menghasilkan keypair yang aman secara kriptografis
    • Prefiks alamat vanity opsional (mis., "SOL...")
    • Ekspor sebagai base58 atau array JSON

    { "*" }*Keamanan:**
    • Key dibuat secara lokal
    • Tidak pernah dikirim melalui jaringan
    • Selalu cadangkan key dengan aman
hints-tools-multi-buy-title = Alat Multi-beli
hints-tools-multi-buy-content =
    { "*" }*Koordinasikan Pembelian di Beberapa Dompet**

    Jalankan order beli di beberapa sub-dompet dengan jumlah acak untuk mensimulasikan aktivitas pembelian organik.

    { "*" }*Cara kerja:**
    1. Membuat atau menggunakan sub-dompet yang ada
    2. Mendistribusikan SOL dari dompet utama ke sub-dompet
    3. Mengeksekusi order beli dengan jumlah dan jeda acak
    4. Setiap dompet membeli secara independen dengan signature unik

    { "*" }*Pengaturan Dompet:**
    • **Jumlah Dompet** — jumlah sub-dompet yang digunakan (2-10)
    • **Buffer SOL** — SOL yang dicadangkan per dompet untuk biaya (~0.015)

    { "*" }*Pengaturan Jumlah:**
    • **SOL Min/Maks** — rentang jumlah beli per dompet
    • **Batas Total** — batas opsional total SOL yang dibelanjakan

    { "*" }*Pengaturan Eksekusi:**
    • **Jeda** — jeda acak antar transaksi
    • **Konkurensi** — eksekusi paralel (1 = berurutan)
    • **Slippage** — slippage maksimum yang dapat diterima
    • **Router** — rute swap (Auto, { -jupiter }, Raydium)

    { "*" }*Penting:**
    • Membutuhkan SOL yang cukup di dompet utama
    • Pembelian yang gagal dicatat di log tetapi tidak menghentikan sesi
    • Sub-dompet dapat digunakan ulang di beberapa sesi
hints-tools-multi-sell-title = Alat Multi-jual
hints-tools-multi-sell-content =
    { "*" }*Koordinasikan Penjualan di Beberapa Dompet**

    Jual token dari semua sub-dompet yang memegang token tertentu dengan konsolidasi SOL otomatis.

    { "*" }*Cara kerja:**
    1. Memindai sub-dompet untuk saldo token
    2. Opsional mengisi ulang dompet yang SOL-nya sedikit untuk biaya
    3. Mengeksekusi order jual dengan persentase yang dapat diatur
    4. Mengonsolidasikan hasil kembali ke dompet utama

    { "*" }*Pengaturan Jual:**
    • **% Jual** — persentase token yang dijual (default 100%)
    • **SOL Min untuk Biaya** — SOL minimum yang dibutuhkan untuk transaksi
    • **Isi Ulang Otomatis** — transfer SOL dari dompet utama jika perlu

    { "*" }*Aksi Setelah Jual:**
    • **Konsolidasi SOL** — transfer semua SOL kembali ke dompet utama
    • **Tutup ATA** — tutup akun token untuk mengklaim kembali rent (~0.002 SOL masing-masing)

    { "*" }*Pengaturan Eksekusi:**
    • **Jeda** — jeda acak antar transaksi
    • **Konkurensi** — eksekusi paralel
    • **Slippage** — slippage maksimum yang dapat diterima
    • **Router** — preferensi rute swap

    { "*" }*Tips:**
    • Pratinjau menampilkan semua dompet yang memegang token
    • Batalkan pilihan dompet yang tidak ingin dijual
    • Konsolidasi terjadi setelah semua penjualan selesai
hints-tools-trade-watcher-title = Alat Trade Watcher
hints-tools-trade-watcher-content =
    { "*" }*Pantau Trade & Picu Aksi Otomatis**

    Pantau aktivitas trading token dan bereaksi otomatis saat trade terjadi.

    { "*" }*Jenis Pemantauan:**
    • **Beli saat Jual** — beli otomatis saat ada yang menjual (tangkap dip)
    • **Jual saat Beli** — jual otomatis saat ada yang membeli (ikuti pasar)
    • **Hanya Notifikasi** — dapatkan peringatan tanpa melakukan aksi

    { "*" }*Cara kerja:**
    1. Masukkan alamat mint token
    2. Klik "Cari Pool" untuk menemukan pool likuiditas yang tersedia
    3. Pilih pool untuk dipantau (wajib untuk aksi beli/jual)
    4. Atur jumlah pemicu (ukuran trade minimum untuk direspons)
    5. Atur jumlah aksi (berapa SOL yang dibeli/dijual)
    6. Mulai pemantauan

    { "*" }*Persyaratan:**
    • Alamat mint token yang valid
    • Pemilihan pool (untuk aksi beli/jual)
    • Saldo SOL yang cukup untuk jumlah aksi

    { "*" }*Integrasi { -telegram }:**
    Atur { -telegram } di Konfigurasi → { -telegram } untuk menerima notifikasi instan saat pemantauan terpicu.
hints-tools-wallet-consolidation-title = Alat Konsolidasi Dompet
hints-tools-wallet-consolidation-content =
    { "*" }*Kelola dan Konsolidasikan Dana Sub-Dompet**

    Lihat semua sub-dompet dan konsolidasikan SOL, token, serta klaim kembali rent ATA ke dompet utama Anda.

    { "*" }*Ringkasan Menampilkan:**
    • **Sub-dompet** — total jumlah sub-dompet yang dibuat
    • **Total SOL** — gabungan saldo SOL di semua sub-dompet
    • **Jenis Token** — jumlah token berbeda yang dipegang
    • **Rent yang Dapat Diklaim** — SOL yang terkunci di ATA kosong

    { "*" }*Aksi:**
    • **Transfer SOL** — pindahkan semua SOL dari dompet terpilih ke dompet utama
    • **Transfer Token** — pindahkan semua token ke dompet utama
    • **Bersihkan ATA** — tutup akun token kosong untuk pengembalian rent

    { "*" }*Info Tabel:**
    • Kotak centang untuk memilih dompet pada operasi massal
    • Nama, alamat, saldo SOL, jumlah token, ATA kosong
    • Dompet kosong diredupkan agar mudah dikenali

    { "*" }*Tips:**
    • Gunakan setelah Multi-jual untuk mengumpulkan sisa SOL
    • Bersihkan ATA secara berkala untuk mengklaim kembali rent
    • Dompet kosong dapat digunakan ulang untuk operasi berikutnya

## config

hints-config-overview-title = Konfigurasi
hints-config-overview-content =
    Pengaturan seluruh sistem untuk { -brand }.

    Kategori:
    • **Trader** — aturan entry/exit, ukuran posisi
    • **Pemfilteran** — ambang filter token
    • **Swap** — pengaturan rute dan slippage
    • **RPC** — konfigurasi node
    • **Layanan** — pengaturan layanan latar belakang

    Perubahan berlaku langsung (hot reload).
hints-config-telegram-title = Notifikasi { -telegram }
hints-config-telegram-content =
    { "*" }*Terima peringatan trading instan lewat { -telegram }**

    Dapatkan notifikasi tentang trade, posisi, dan event penting langsung di { -telegram }.

    { "*" }*Langkah Pengaturan:**

    1. **Buat bot:**
       • Buka { -telegram } dan kirim pesan ke @BotFather
       • Kirim /newbot dan ikuti petunjuknya
       • Salin token bot (bentuknya: 123456:ABC-DEF...)

    2. **Dapatkan Chat ID Anda:**
       • Kirim pesan ke @userinfobot atau @getidsbot
       • Salin ID numerik yang dikembalikan

    3. **Atur di { -brand }:**
       • Aktifkan sakelar notifikasi
       • Tempel token bot dan chat ID
       • Klik "Uji Koneksi" untuk memverifikasi

    { "*" }*Yang akan Anda terima:**
    • Konfirmasi eksekusi trade
    • Pembaruan posisi (entry/exit)
    • Peringatan Trade Watcher
    • Notifikasi error

    { "*" }*Privasi:**
    Pesan dikirim langsung dari { -brand } ke bot { -telegram } Anda — tanpa server pihak ketiga.
hints-config-telegram-password-title = Kata Sandi Autentikasi Bot
hints-config-telegram-password-content =
    { "*" }*Amankan bot { -telegram } Anda dengan autentikasi kata sandi**

    Saat Anda berinteraksi dengan bot { -brand } { -telegram } Anda, Anda perlu mengautentikasi dengan kata sandi ini sebelum menjalankan perintah sensitif.

    { "*" }*Mengapa mengatur kata sandi?**
    • Mencegah pengguna tidak sah mengendalikan bot Anda
    • Diperlukan untuk menjalankan perintah trading lewat { -telegram }
    • Minimal 8 karakter

    { "*" }*Cara kerja:**
    1. Atur kata sandi di sini pada dasbor
    2. Saat Anda mengirim perintah trading ke bot, bot akan meminta autentikasi
    3. Masukkan kata sandi untuk memverifikasi identitas Anda
    4. Opsional aktifkan 2FA untuk keamanan tambahan

    { "*" }*Catatan:** Kata sandi disimpan sebagai hash SHA256 yang aman — kami tidak pernah menyimpan teks aslinya.
hints-config-telegram-totp-title = Autentikasi Dua Faktor (2FA)
hints-config-telegram-totp-content =
    { "*" }*Tambahkan lapisan keamanan ekstra dengan TOTP 2FA**

    Autentikasi dua faktor menggunakan kata sandi sekali pakai berbasis waktu (TOTP) dari aplikasi seperti Google Authenticator, Authy, atau 1Password.

    { "*" }*Mengapa mengaktifkan 2FA?**
    • Meskipun seseorang mengetahui kata sandi Anda, mereka tidak dapat mengakses bot tanpa kode
    • Kode 6 digit berganti setiap 30 detik
    • Berfungsi offline setelah diatur

    { "*" }*Proses pengaturan:**
    1. Klik "Aktifkan 2FA" dan masukkan kata sandi Anda
    2. Pindai kode QR dengan aplikasi autentikator Anda
    3. Masukkan kode 6 digit untuk memverifikasi pengaturan

    { "*" }*Aplikasi yang kompatibel:**
    • Google Authenticator
    • Authy
    • 1Password
    • Microsoft Authenticator
    • Aplikasi apa pun yang kompatibel dengan TOTP

    { "*" }*Penting:** Simpan secret key Anda di tempat yang aman. Jika Anda kehilangan akses ke aplikasi autentikator, Anda harus menonaktifkan 2FA dari dasbor ini.

## token_details

hints-token-details-chart-title = Grafik Harga (OHLCV)
hints-token-details-chart-content =
    { "*" }*Penting:** Grafik ini menampilkan **data OHLCV dari cache** untuk evaluasi strategi, *bukan* harga eksekusi langsung.

    { "*" }*Mengapa Data Cache?**
    • **Tujuan:** Digunakan oleh strategi otomatis dan indikator (mis., RSI, MA).
    • **Kesegaran:** Pembaruan bergantung pada prioritas token (Posisi terbuka = pembaruan lebih cepat).
    • **Sumber:** Diagregasi dari { -dexscreener }/{ -geckoterminal }, bukan RPC on-chain langsung.

    { "*" }*Realitas Harga DEX:**
    Di DeFi, token diperdagangkan di **banyak pool** (Raydium, Orca, Meteora). Setiap pool memiliki harga unik berdasarkan kedalaman likuiditas dan trade terbaru.
    • **Harga Grafik:** Rata-rata/agregat di berbagai pasar.
    • **Harga Swap:** Kurs spesifik yang Anda dapat dari rute terbaik pada saat trade.

    { "*" }Perkirakan selisih kecil antara grafik ini dan harga eksekusi akhir Anda.*

    { "*" }*Status:** "Menunggu data" berarti worker latar belakang sedang mengambil candle terbaru.
hints-token-details-token-info-title = Informasi Token
hints-token-details-token-info-content =
    Metadata token dasar dari sumber on-chain dan pasar.

        • **Mint** — alamat token unik di Solana (klik untuk menyalin)
        • **Desimal** — presisi token (biasanya 6-9)
        • **Usia** — waktu sejak pool/token utama dibuat
        • **DEX** — tempat trading utama token ini
        • **Holder** — dompet unik yang memegang token
        • **Top 10 Hold** — % yang dipegang 10 dompet teratas

        Jumlah holder yang lebih banyak dan konsentrasi yang lebih rendah umumnya menandakan distribusi yang lebih sehat.
hints-token-details-liquidity-title = Likuiditas & Data Pasar
hints-token-details-liquidity-content =
    Metrik pasar dari pool SOL dengan likuiditas tertinggi.

        • **FDV** — harga × total suplai (harga agregator)
        • **Likuiditas** — nilai USD cadangan pool
        • **Pool SOL** / **Pool Token** — cadangan langsung yang menentukan harga pool

        { "*" }*Mengapa penting:**
        • Likuiditas lebih dalam = slippage lebih rendah
        • Pool dangkal dapat bergerak karena trade kecil
        • Cadangan pool langsung menentukan harga eksekusi swap

        Data diperbarui berkala dari { -dexscreener }/{ -geckoterminal } ditambah pembacaan pool on-chain.
hints-token-details-market-pulse-title = Denyut Pasar
hints-token-details-market-pulse-content =
    Pergerakan harga dan volume trading USD berbagi timeline **5M / 1H / 6H / 24H** yang sama sehingga momentum dan partisipasi dapat dibandingkan langsung.

    { "*" }*Interpretasi:**
    • **Harga** — persentase perubahan dari agregator, bukan harga eksekusi pool langsung.
    • **Volume Tinggi** — minat lebih kuat, penemuan harga lebih efisien, dan exit lebih mudah.
    • **Volume Rendah** — slippage lebih besar, spread lebih lebar, dan exit besar lebih sulit.
    • **Volume Tinggi + Likuiditas Rendah** — volatilitas dan risiko eksekusi meningkat.

    Data pasar diagregasi dari DEX utama lewat { -dexscreener }/{ -geckoterminal }, sehingga perubahan harga dapat berbeda dari harga pool on-chain saat ini.
hints-token-details-activity-title = Aktivitas Transaksi (Jumlah)
hints-token-details-activity-content =
    Menganalisis **jumlah trade** (beli vs. jual) di berbagai timeframe. Ini mengungkap niat trader terlepas dari ukuran trade.

    { "*" }*Rincian Metrik:**
    • **Timeframe:** Jendela 5M, 1H, 6H, 24H.
    • **Bar:** Rasio visual jumlah Beli (Hijau) vs. jumlah Jual (Merah).
    • **Rate:** Trade per menit (mis., "12.5/m"). Rate lebih tinggi = aktivitas viral.
    • **Jumlah:** Jumlah pasti beli/jual dan persentasenya.

    { "*" }*Metrik ringkasan:**
    • **Beli % 24H:** >50% bullish (lebih banyak pembeli), { "<" }50% bearish (lebih banyak penjual).
    • **Net Flow:** Total beli dikurangi jual. Positif = Akumulasi.
    • **Lonjakan 5M:** Seberapa lebih cepat trading *saat ini* dibanding rata-rata 1H.
      • **>1.0x:** Minat meningkat.
      • **>3.0x:** Breakout viral atau kepanikan.
      • **{ "<" }1.0x:** Mendingin.

    { "*" }*Tips Strategi:** "Beli %" tinggi dengan "Faktor Lonjakan" tinggi sering menandakan entry breakout yang kuat.
hints-token-details-security-title = Analisis Keamanan
hints-token-details-security-content =
    Penilaian risiko dari { -rugcheck }.xyz dan analisis on-chain.

    { "*" }*Skor Keamanan (0-100):**
    Skor yang lebih tinggi menunjukkan token lebih aman. Faktornya meliputi:
    • Izin otoritas (mint/freeze)
    • Konsentrasi holder
    • Status kunci LP
    • Pola risiko yang dikenal

    { "*" }*Indikator Risiko Utama:**
    • **Otoritas Mint** — dapat membuat token baru (risiko inflasi)
    • **Otoritas Freeze** — dapat membekukan akun token
    • **% Holder Teratas** — risiko konsentrasi
    • **Penyedia LP** — jumlah penyedia likuiditas

    Selalu verifikasi keamanan sebelum trading dalam jumlah besar.
hints-token-details-pools-title = Pool Likuiditas
hints-token-details-pools-content =
    Semua pool likuiditas yang ditemukan untuk token ini.

    { "*" }*Mengapa banyak pool penting:**
    • Setiap pool memiliki likuiditas dan harga berbeda
    • Router swap mencari rute terbaik di antara pool
    • Harga dapat berbeda 1-5% antar pool

    { "*" }*Informasi Pool:**
    • **DEX** — bursa yang menaungi pool
    • **Likuiditas** — nilai USD cadangan pool
    • **Volume** — aktivitas trading terbaru
    • **Harga** — harga pool saat ini

    Pool Service menghitung harga dari pair SOL dengan likuiditas tertinggi.

## ui

hints-ui-featured-title = Unggulan
hints-ui-featured-content =
    Token yang di-boost lebih dulu, lalu proyek trending dari { -jupiter } dan { -dexscreener }.

    { "*" }*Yang akan Anda lihat:**
    • Token yang di-boost — tim mereka membayar untuk promosi — disematkan di depan, ditandai emas
    • Token trending dari papan penemuan setelahnya
    • Klik token mana pun untuk membuka detail lengkapnya

    { "*" }*Mem-boost token:**
    Boost membeli visibilitas, bukan rekomendasi. Baris yang di-boost ditandai emas di mana pun
    muncul, termasuk di tabel token Anda, sehingga Anda selalu tahu mana yang mana. Boost token di
    { "*" }*screenerbot.io/boost**.

    { "*" }*Menonaktifkan baris ini:**
    Sembunyikan lewat **Pengaturan → Antarmuka → Tampilkan Baris Unggulan**. Aksi di header tetap
    membuka tampilan Unggulan lengkap.

## Hint popover chrome (ui/hint_popover.js)

hints-trigger =
    .aria-label = Bantuan: { $title }
hints-popover-close =
    .aria-label = Tutup
hints-popover-learn-more = Pelajari lebih lanjut
hints-popover-dismiss = Jangan tampilkan lagi
