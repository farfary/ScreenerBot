# Account and ScreenerBot data access status. Each message is the headline;
# `.detail` explains what happens instead. Ids come from DataAccess in
# src/data_server/access.rs.

account-data-access-ready = Data { -brand } aktif
    .detail = Candle bersama, registri pool, laporan keamanan, dan identitas token disajikan dari screenerbot.io.

account-data-access-disabled = Data { -brand } dimatikan
    .detail = Sumber { -brand } dimatikan di pengaturan Anda, sehingga data hanya berasal dari penyedia publik.

account-data-access-offline = Data { -brand } offline
    .detail = Tidak ada koneksi jaringan. Data akan berlanjut sendiri setelah koneksi kembali.

account-data-access-signed-out = Data { -brand } memerlukan akun
    .detail = Grafik, pool, laporan keamanan, dan identitas token berasal dari penyedia publik. Penyedia tersebut lebih lambat, dibatasi rate, dan riwayatnya lebih terbatas. Masuk gratis dan tidak mengubah hal lain tentang cara { -brand } berjalan.

account-data-access-reauthorization-required = Data { -brand } mengharuskan Anda masuk lagi
    .detail = Perangkat ini diotorisasi sebelum data { -brand } tersedia. Masuk lagi untuk memulihkannya — penyedia publik digunakan sampai saat itu.

account-data-access-version-unsupported = Data { -brand } memerlukan versi lebih baru
    .detail = Versi ini tidak dilayani lagi. Perbarui ke { $minimum } atau lebih baru untuk kembali menggunakan data { -brand }; penyedia publik digunakan sampai saat itu.

account-data-access-unreachable = Data { -brand } tidak merespons
    .detail = Layanan tidak menjawab. Penyedia publik digunakan, dan { -brand } akan terus mencoba lagi.

account-data-access-unknown = Data { -brand } belum diperiksa
    .detail = { -brand } belum memerlukan data bersama pada sesi ini.

## Account panel (ui/account/panel.js), shared by Setup and Settings

account-scope-data-read = Data pasar { -brand }
account-scope-rpc-submit = Pengiriman transaksi bertanda tangan gratis
account-scope-vote = Voting token
account-scope-referral-read = Pendapatan referral
account-scope-account-read = Detail akun

account-panel-request-failed = Gagal. Silakan coba lagi.
account-panel-checking = Memeriksa status akun…
account-panel-status-unavailable = Status akun tidak tersedia.
account-panel-browser-notice = Selesaikan proses masuk di browser Anda, lalu kembali ke sini. Panel ini akan diperbarui.
account-panel-browser-timeout = Proses masuk di browser tidak diselesaikan. Anda dapat memulainya lagi.
account-panel-unavailable = Fitur akun tidak tersedia saat ini. Lanjutkan pengaturan tanpa masuk.
account-panel-retry-status = Coba lagi status akun
account-panel-signed-in-fallback = Sudah masuk
account-panel-features =
    .aria-label = Fitur akun
account-panel-sign-out = Keluar
account-panel-signing-out = Keluar…
account-panel-sign-in = Masuk
account-panel-signing-in = Masuk…
account-panel-sign-in-wallet = Masuk dengan dompet
account-panel-opening-browser = Membuka browser…
account-panel-continue-browser = Lanjutkan di browser
account-panel-sign-in-email = Masuk dengan email
account-panel-new-to = Baru di { -brand }?
account-panel-create-account = Buat akun
account-panel-unlocks-title = Termasuk dalam akun
account-panel-back-to-options = Kembali ke opsi masuk
account-panel-email-label = Email
account-panel-email-input =
    .placeholder = anda@contoh.com
account-panel-password-label = Kata Sandi
account-panel-password-input =
    .placeholder = Kata sandi Anda
account-panel-need-account = Butuh akun atau lupa kata sandi?
account-panel-open-website = Buka screenerbot.io
