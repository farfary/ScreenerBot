## Portfolio overview

home-portfolio-title = Nilai portofolio
home-portfolio-today = hari ini
home-stat-available = { -sol } tersedia
home-stat-holdings = Kepemilikan token
home-stat-open-pnl = P&L terbuka
home-stat-realized-today = Terealisasi hari ini

home-holdings-token-count =
    { $count ->
       *[other] { $count } token
    }
home-holdings-with-unpriced = { $tokens } · { $count } tanpa harga
home-holdings-unpriced-note =
    { $count ->
       *[other] { $count } token yang dimiliki tidak memiliki harga dan dihitung 0 dalam total
    }

## Wallet address and QR code

home-wallet-copy =
    .title = Salin alamat dompet
    .aria-label = Salin alamat dompet
home-wallet-qr-open =
    .title = Tampilkan kode QR dompet
    .aria-label = Tampilkan kode QR dompet
home-wallet-qr-popover =
    .aria-label = Kode QR dompet
home-wallet-qr-receive = Terima
home-wallet-qr-assets = { -sol } dan token SPL
home-wallet-qr-close =
    .title = Tutup
    .aria-label = Tutup kode QR dompet
home-wallet-qr-preparing = Menyiapkan kode QR
home-wallet-qr-unavailable = Kode QR tidak tersedia
home-wallet-qr-image =
    .alt = Kode QR untuk alamat dompet utama

## Performance calendar

home-calendar-title = Kalender performa
home-calendar-previous =
    .title = Bulan sebelumnya
    .aria-label = Bulan sebelumnya
home-calendar-next =
    .title = Bulan berikutnya
    .aria-label = Bulan berikutnya
home-calendar-month-pnl = P&L bulan ini
home-calendar-trades = Trade
home-calendar-pop-net-pnl = P&L bersih
home-calendar-pop-win-rate = Win rate
home-calendar-pop-win-rate-value = { $rate } · { $wins }M / { $losses }K
home-calendar-pop-gross-profit = Profit kotor
home-calendar-pop-gross-loss = Rugi kotor
home-calendar-pop-end-balance = Saldo akhir

## Position exposure and market pipeline

home-operations =
    .aria-label = Status portofolio dan pasar
home-exposure-title = Eksposur posisi
home-exposure-open = terbuka
home-exposure-invested = Diinvestasikan
home-exposure-avg-size = Ukuran rata-rata
home-exposure-avg-hold = Hold rata-rata
home-exposure-best = Terbaik
home-exposure-worst = Terburuk
home-pipeline-title = Pipeline pasar
home-pipeline-tracked = Dilacak
home-pipeline-priced = Berharga
home-pipeline-passed = Lolos filter
home-pipeline-rejected = Ditolak
home-pipeline-blacklisted = Masuk daftar hitam
home-pipeline-ohlcv = OHLCV
