# Telegram bot text. Server-only: rendered by src/telegram/text.rs, never sent to the dashboard.
#
# Messages are sent as Telegram HTML. The only tags are b, i, u, s, code and pre,
# without attributes; links are built in Rust. A line break is a literal newline.
# Icons are prepended by Rust and never appear here. Copyable values (chat ids)
# arrive as arguments and are wrapped in code inside the message. Keep the
# command names (/status) and the literal ampersand placeable unchanged.

## Reply keyboard words. Icons come from ReplyCommand in src/telegram/reply.rs.

telegram-reply-status = Status
telegram-reply-balance = Saldo
telegram-reply-positions = Posisi
telegram-reply-pause = Jeda
telegram-reply-resume = Lanjutkan
telegram-reply-stop = Stop
telegram-reply-stats = Statistik
telegram-reply-menu = Menu
telegram-reply-help = Bantuan

## Inline keyboard buttons.

telegram-button-positions = Posisi
telegram-button-balance = Saldo
telegram-button-stats = Statistik
telegram-button-tokens = Token
telegram-button-pause = Jeda
telegram-button-stop = Stop
telegram-button-settings = Pengaturan
telegram-button-refresh = Segarkan
telegram-button-menu = Menu
telegram-button-back = Kembali
telegram-button-back-to-menu = Kembali ke Menu
telegram-button-back-to-tokens = Kembali ke Token
telegram-button-cancel = Batal
telegram-button-close-all-positions = Tutup Semua Posisi
telegram-button-sell-percent = Jual { $percent }%
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = Daftar Hitam
telegram-button-blacklist-symbol = Daftar Hitamkan { $symbol }
telegram-button-close-position = Tutup Posisi
telegram-button-confirm-close = Konfirmasi Tutup
telegram-button-confirm-close-all = Tutup SEMUA Posisi
telegram-button-confirm-sell = Konfirmasi Jual { $percent }%
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = KONFIRMASI HENTIKAN PAKSA
telegram-button-confirm-buy = Beli { $amount } { -sol }
telegram-button-notifications = Notifikasi
telegram-button-trading = Trading
telegram-button-entry-monitor = Monitor Entry
telegram-button-exit-monitor = Monitor Exit
telegram-button-auto-trading = Auto Trading
telegram-button-force-stop = Hentikan Paksa
telegram-button-notify-opened = Dibuka
telegram-button-notify-closed = Ditutup
telegram-button-notify-partial = Parsial
telegram-button-notify-dca = DCA
telegram-button-notify-errors = Error
telegram-button-details = Detail
telegram-button-position = Posisi
telegram-button-sell-more = Jual Lagi
telegram-button-more-dca = DCA Lagi
telegram-button-history = Riwayat
telegram-button-status = Status
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = Autentikasi Ulang
telegram-button-previous = Sebelumnya
telegram-button-next = Berikutnya
telegram-button-passed = Lolos
telegram-button-rejected = Ditolak
telegram-button-new-24h = Baru (24h)
telegram-button-all-tokens = Semua Token
telegram-button-search-token = Cari Token
telegram-button-filter-stats = Statistik Filter
telegram-button-refresh-stats = Segarkan Statistik
telegram-button-view-position = Lihat Posisi
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    Perintah tidak dikenal: { $command }

    Gunakan /help untuk melihat perintah yang tersedia.
telegram-session-expired =
    <b>Sesi Berakhir</b>

    Gunakan /login untuk autentikasi ulang.
telegram-2fa-required =
    <b>2FA Diperlukan</b>

    Masukkan kode autentikator 6 digit Anda.
telegram-account-locked =
    <b>Akun Terkunci</b>

    Terlalu banyak percobaan gagal.
    Coba lagi dalam { $seconds ->
       *[other] { $seconds } dtk.
    }
telegram-code-invalid = Masukkan kode 6 digit yang valid.
telegram-authenticated =
    <b>Terautentikasi!</b>

    Anda kini memiliki akses ke perintah bot.
telegram-wrong-code =
    <b>Kode Salah</b>

    { $remaining ->
       *[other] Sisa { $remaining } percobaan.
    }
telegram-auth-required =
    <b>Autentikasi Diperlukan</b>

    Masukkan kata sandi Anda untuk melanjutkan.

    <i>Ketik kata sandi Anda lalu kirim.</i>
telegram-login-required =
    <b>Login Diperlukan</b>

    Masukkan kode autentikator 6 digit Anda:
telegram-session-activated =
    <b>Sesi Diaktifkan</b>

    2FA belum dikonfigurasi. Sesi Anda kini aktif.

    <i>Tips: Aktifkan 2FA di pengaturan Keamanan untuk keamanan lebih baik.</i>

## Chat discovery.

telegram-discovery-hello = Halo { $name }!
telegram-discovery-default-name = Pengguna
telegram-discovery-detected = <b>Chat terdeteksi!</b>
telegram-discovery-details =
    Chat ID: <code>{ $chat_id }</code>
    Jenis: { $chat_type }

    Buka dasbor { -brand } dan klik chat ini untuk memilihnya.
telegram-chat-type-private = privat
telegram-chat-type-group = grup
telegram-chat-type-supergroup = supergrup
telegram-chat-type-channel = channel

## Menus.

telegram-menu-title =
    <b>Panel Kontrol</b>

    Pilih opsi untuk melihat informasi atau mengendalikan bot.
telegram-menu-positions-empty =
    <b>Tidak Ada Posisi Terbuka</b>

    Menunggu peluang baru...
telegram-menu-positions-title = <b>Posisi ({ $count })</b>
telegram-menu-positions-hint = <i>Ketuk posisi untuk mengelolanya.</i>
telegram-menu-settings =
    <b>Pengaturan</b>

    Atur notifikasi dan parameter trading.
telegram-settings-notifications =
    <b>Pengaturan Notifikasi</b>

    Aktifkan/nonaktifkan notifikasi:
telegram-settings-trading =
    <b>Kontrol Trading</b>

    Aktifkan/nonaktifkan fitur trading:
telegram-pagination-expired = Sesi halaman telah berakhir.

## Status commands.

telegram-status-state-stopped = <b>BERHENTI</b> (Hentikan Paksa Aktif)
telegram-status-state-active = <b>AKTIF</b>
telegram-status-state-paused = <b>DIJEDA</b>
telegram-status-on = Aktif
telegram-status-off = Nonaktif
telegram-status-body =
    <b>Status Sistem</b>

    <b>Sistem</b>
    Kondisi — { $state }
    Uptime — { $uptime }
    Versi — v{ $version }

    <b>Trading</b>
    Entry — { $entries }
    Exit — { $exits }
    Posisi — { $positions }
telegram-positions-empty =
    <b>Tidak Ada Posisi Terbuka</b>

    Menunggu peluang...
telegram-positions-title = <b>Posisi Terbuka ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+{ $count } lainnya...</i>
telegram-positions-summary =
    <b>Ringkasan Portofolio</b>
    Diinvestasikan — { $invested } { -sol }
    Net P{ "&amp;" }L — { $pnl } { -sol }
telegram-balance-body =
    <b>Saldo Dompet</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>Statistik Harian</b>

    Posisi — { $positions }
    Diinvestasikan — { $invested } { -sol }
    P{ "&amp;" }L — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } Siap!</b>

    Trading <b>diaktifkan</b>.

    Gunakan keyboard di bawah untuk mengendalikan bot.
    Ketik /help untuk melihat perintah yang tersedia.
telegram-stop-already = <b>Trading sudah dinonaktifkan</b>
telegram-stop-done =
    <b>Trading Dinonaktifkan</b>

    Semua monitor trading (entry { "&amp;" } exit) dihentikan.
    Gunakan /pause untuk menghentikan entry saja.
telegram-stop-failed =
    <b>Gagal menonaktifkan trading</b>

    Error: { $detail }
telegram-pause-done =
    <b>Monitor Entry Dijeda</b>

    Tidak ada posisi baru yang akan dibuka.
    Monitor exit tetap berjalan.
telegram-pause-failed =
    <b>Gagal menjeda entry</b>

    Error: { $detail }
telegram-resume-done =
    <b>Monitor Entry Dilanjutkan</b>

    Kini mengawasi sinyal entry.
telegram-resume-failed =
    <b>Gagal melanjutkan entry</b>

    Error: { $detail }
telegram-force-stop-confirm =
    <b>HENTIKAN PAKSA</b>

    Ini akan langsung menghentikan SEMUA aktivitas trading:
    • Tanpa entry baru
    • Tanpa exit (termasuk stop loss)
    • Tanpa operasi DCA
telegram-force-stop-warning = <b>Ini adalah tindakan darurat!</b>
telegram-force-stop-question = Apakah Anda yakin?
telegram-force-stop-active =
    <b>HENTIKAN PAKSA DIAKTIFKAN</b>

    Semua trading telah dihentikan.

    Gunakan /resume_trading untuk menghapus status ini.
telegram-resume-trading-not-stopped =
    <b>Trading tidak dihentikan paksa</b>

    Tidak perlu tindakan.
telegram-resume-trading-done =
    <b>Trading Dilanjutkan</b>

    Status hentikan paksa telah dihapus.
    Operasi trading normal kini dapat dilanjutkan.

## Help.

telegram-help-title = <b>Bantuan { -brand }</b>
telegram-help-heading-dashboard = Dasbor
telegram-help-heading-market = Pasar
telegram-help-heading-trading = Trading
telegram-help-heading-safety = Keamanan
telegram-help-heading-system = Sistem
telegram-help-commands-dashboard =
    /status — Status sistem { "&amp;" } uptime
    /stats — Performa harian
    /balance — Saldo dompet
    /positions — Posisi terbuka
telegram-help-commands-market =
    /tokens — Penjelajah token
    /rejected — Token yang difilter
telegram-help-commands-trading =
    /start — Aktifkan sistem trading
    /stop — Nonaktifkan sistem trading
    /pause — Jeda entry baru
    /resume — Lanjutkan entry baru
    /menu — Menu interaktif
telegram-help-commands-safety =
    /force_stop — <b>PENGHENTIAN DARURAT</b>
    /resume_trading — Hapus status darurat
telegram-help-commands-system =
    /update — Status pembaruan { "&amp;" } instalasi
    /login — Autentikasi 2FA
telegram-help-tip = <i>Tips: Ketuk perintah untuk menjalankannya.</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>Sudah terbaru</b>

    Menjalankan v{ $version }, diinstal otomatis.
telegram-update-up-to-date =
    <b>Sudah terbaru</b>

    Menjalankan v{ $version }.
telegram-update-check-failed =
    <b>Pemeriksaan pembaruan gagal</b>

    { $reason }
telegram-update-unreachable = screenerbot.io tidak dapat dijangkau.
telegram-update-installing = <b>Menginstal v{ $version }</b>
telegram-update-restarting =
    { -brand } sedang restart ke versi baru. Trading dilanjutkan otomatis.
telegram-update-install-failed =
    <b>Tidak dapat menginstal v{ $version }</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } telah diunduh</b>

    Rilis ini juga memperbarui aplikasi desktop, sehingga penginstalnya harus dijalankan di komputer tersebut. Buka Pengaturan → Pembaruan di sana.
telegram-update-downloading =
    <b>Mengunduh v{ $version }</b>

    { $percent }% dari { $size } MB.
telegram-update-available =
    <b>v{ $version } tersedia</b>

    { $how }
    Ukuran unduhan: { $size } MB.

    Diunduh otomatis; kirim /update lagi setelah siap.
telegram-update-how-core = Diinstal secara senyap dengan restart singkat.
telegram-update-how-installer = Memerlukan penginstal desktop dijalankan sekali.

## Shared values and units. Numbers arrive formatted; only the unit words live here.

telegram-value-unknown = Tidak diketahui
telegram-value-na = T/A
telegram-percent-value = { $percent }%
telegram-price-native = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds } dtk
telegram-duration-minutes = { $minutes } mnt
telegram-duration-minutes-seconds = { $minutes } mnt { $seconds } dtk
telegram-duration-hours = { $hours } jam
telegram-duration-hours-minutes = { $hours } jam { $minutes } mnt
telegram-duration-days = { $days } hari
telegram-duration-days-hours = { $days } hari { $hours } jam
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-native = { $amount } { -sol }
telegram-error-line = Error: { $detail }
telegram-ai-reasoning =
    <b>Analisis LLM</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens. Icons come from Rust.

telegram-row-entry = Entry — { $price } { -sol }
telegram-row-exit = Exit — { $price } { -sol }
telegram-row-current = Saat ini — { $price } { -sol }
telegram-row-invested = Diinvestasikan — { $amount } { -sol }
telegram-row-received = Diterima — { $amount } { -sol }
telegram-row-value = Nilai — { $amount } { -sol }
telegram-row-total = Total — { $amount } { -sol }
telegram-row-tokens = Token — { $tokens }
telegram-row-duration = Durasi — { $duration }
telegram-row-reason = Alasan — { $reason }
telegram-row-remaining = Sisa — { $percent }%
telegram-row-pnl = P{ "&amp;" }L — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>Posisi Dibuka</b>
telegram-notify-opened-size = Ukuran — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = Harga — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>Posisi Ditutup</b> — Profit
telegram-notify-closed-title-loss = <b>Posisi Ditutup</b> — Rugi
telegram-notify-closed-reason-unspecified = Ditutup
telegram-notify-partial-title = <b>Exit Parsial</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — Terjual { $percent }%
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = Ditambahkan — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = Rata-rata — { $price } { -sol }
telegram-notify-unbooked-title = <b>Swap Belum Dibukukan</b>
telegram-notify-unbooked-body = Swap yang terkonfirmasi on-chain belum masuk ke posisinya. Swap diverifikasi ulang sampai dibukukan.
telegram-notify-unbooked-signature = Transaksi: <code>{ $signature }</code>
telegram-notify-severity-critical = <b>Error Kritis</b>
telegram-notify-severity-error = <b>Error</b>
telegram-notify-severity-warning = <b>Peringatan</b>
telegram-notify-severity-info = <b>Info</b>
telegram-notify-alert-title = <b>Peringatan Trade</b>
telegram-notify-alert-token = Token: <code>${ $symbol }</code>
telegram-notify-alert-mint = Mint: <code>{ $mint }</code>
telegram-notify-alert-bought = Aksi: membeli { $amount } { -sol }
telegram-notify-alert-sold = Aksi: menjual { $amount } { -sol }
telegram-notify-alert-wallet = Dompet: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (paper)
telegram-notify-copy-task = Tugas: { $task }
telegram-notify-scheduled-completed = <b>Tugas Terjadwal selesai</b>
telegram-notify-scheduled-failed = <b>Tugas Terjadwal gagal</b>
telegram-notify-scheduled-timed-out = <b>Tugas Terjadwal habis waktu</b>
telegram-notify-scheduled-error = Error: { $error }
telegram-notify-summary-title = <b>Ringkasan Harian</b> — { $date }
telegram-notify-summary-performance = <b>Performa</b>
telegram-notify-summary-trades = Trade — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = Win Rate — { $percent }%
telegram-notify-summary-pnl = P{ "&amp;" }L — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = Posisi Terbuka — { $count }
telegram-notify-started-title = <b>{ -brand } Dimulai</b>
telegram-notify-started-version = <b>Versi</b> — { $version }
telegram-notify-started-mode = <b>Mode</b> — { $mode }
telegram-notify-started-ready = Siap untuk trading!
telegram-notify-stopped-title = <b>{ -brand } Berhenti</b>
telegram-notify-stopped-reason = <b>Alasan</b> — { $reason }
telegram-notify-stopped-goodbye = Sampai jumpa! { $icon }
telegram-notify-start-mode-normal = Normal
telegram-notify-stop-reason-graceful = Shutdown normal
telegram-notify-update-available =
    <b>Pembaruan v{ $version } tersedia</b>

    { $how }
    Ukuran unduhan: { $size } MB
telegram-notify-update-how-installer = Rilis ini juga memperbarui aplikasi desktop, sehingga penginstalnya harus dijalankan sekali.
telegram-notify-update-ready =
    <b>Pembaruan v{ $version } siap</b>

    { $how }
telegram-notify-update-ready-silent = Kirim /update untuk menerapkannya sekarang, atau akan diinstal saat { -brand } dimulai berikutnya.
telegram-notify-update-ready-installer = Buka Pengaturan → Pembaruan untuk menjalankan penginstal.
telegram-notify-update-applying =
    <b>Menginstal v{ $version }</b>

    Backend sedang restart; trading dilanjutkan otomatis.
telegram-notify-new-tokens =
    <b>Peringatan Pemfilteran</b>

    { $count ->
       *[other] Ditemukan { $count } token baru yang sesuai kriteria Anda.
    }
telegram-notify-crash =
    <b>Bot Crash!</b>

    <b>Lokasi:</b> <code>{ $location }</code>
    <b>Error:</b> <code>{ $error }</code>
telegram-notify-crash-restart = Silakan restart bot.

## Filter results page.

telegram-filter-results-title = <b>Hasil Filter</b> ({ $count })
telegram-filter-results-empty = <i>Tidak ada token ditemukan.</i>
telegram-filter-results-page = <i>Halaman { $page } dari { $total }</i>

## Position screens.

telegram-position-not-found = Posisi tidak ditemukan
telegram-position-no-positions = Tidak ada posisi untuk ditutup
telegram-position-history-empty =
    <b>Riwayat Trade</b>

    Belum ada posisi tertutup.
telegram-position-history-title = <b>Trade Terbaru</b>
telegram-position-history-more = <i>+{ $count } trade lainnya...</i>
telegram-position-confirm-hint = <i>Konfirmasi dalam 30 dtk untuk mengeksekusi.</i>
telegram-position-confirm-close-title = <b>Tutup Posisi?</b>
telegram-position-confirm-close-selling = Menjual { $tokens } token
telegram-position-confirm-close-estimated = Perkiraan — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>Konfirmasi dalam 30 dtk</i>
telegram-position-confirm-sell =
    <b>Konfirmasi Jual</b>

    Token — { $symbol }
    Jumlah — { $percent }%
    Token — { $tokens }
telegram-position-confirm-dca =
    <b>Konfirmasi Beli Lagi</b>

    Token — { $symbol }
    Tambah — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>Tutup Semua Posisi?</b>

    Jumlah — { $count }
telegram-position-confirm-close-all-hint =
    <i>Ini akan menjual semua posisi terbuka di harga pasar.
    Konfirmasi dalam 30 dtk.</i>
telegram-position-confirm-force-stop =
    <b>HENTIKAN PAKSA</b>

    Ini akan langsung menghentikan SEMUA trading:
    • Tanpa entry baru
    • Tanpa exit
    • Tanpa DCA
telegram-position-confirm-force-stop-warning = <b>Ini adalah tindakan darurat.</b>
telegram-position-confirm-blacklist =
    <b>Daftar Hitamkan Token?</b>

    Token — { $symbol }
    Mint — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>Ini akan menutup posisi dan mencegah entry di masa depan.</i>
telegram-position-selling = Menjual { $percent }% { $symbol }...
telegram-position-sell-done =
    <b>Jual Dieksekusi</b>

    Token — { $symbol }
    Terjual — { $percent }%
    Diterima — { $amount } { -sol }
telegram-position-sell-failed = <b>Jual Gagal</b>
telegram-position-adding = Menambahkan { $amount } { -sol } ke { $symbol }...
telegram-position-dca-done =
    <b>DCA Dieksekusi</b>

    Token — { $symbol }
    Ditambahkan — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA Gagal</b>
telegram-position-closing-all = Menutup semua posisi...
telegram-position-close-all-done =
    <b>Tutup Semua Selesai</b>

    Ditutup — { $closed }
    Gagal — { $failed }
telegram-position-blacklisted =
    <b>Token Masuk Daftar Hitam</b>

    Token — { $symbol }
    Status — Ditutup { "&amp;" } Masuk Daftar Hitam

## Token screens.

telegram-token-not-found = Token tidak ditemukan
telegram-token-not-found-prefix = Token tidak ditemukan. Coba cari dengan prefiks yang lebih panjang.
telegram-token-stats-failed = Gagal mengambil statistik: { $detail }
telegram-token-list-failed = Gagal mengambil token: { $detail }
telegram-token-list-empty = Tidak ada token di tampilan <b>{ $view }</b>.
telegram-token-view-passed = Lolos Filter
telegram-token-view-rejected = Ditolak
telegram-token-view-recent = Baru Ditambahkan
telegram-token-view-all = Semua Token
telegram-token-list-title = <b>{ $name }</b> (Halaman { $page }/{ $total })
telegram-token-list-stats = Lik: { $liquidity } • Harga: { $price }
telegram-token-list-hint = <i>Ketuk /token_ID untuk melihat detail</i>
telegram-token-explorer =
    <b>Penjelajah Pasar</b>

    <b>Ringkasan</b>
    Lolos Filter — { $passed }
    Ditolak — { $rejected }
    Harga Aktif — { $priced }
    Total Ditemukan — { $total }

    <i>Pilih kategori untuk dijelajahi:</i>
telegram-token-filter-title = <b>Analisis Filter</b>
telegram-token-filter-distribution = <b>Distribusi</b>
telegram-token-filter-passed = Lolos — { $count } ({ $percent }%)
telegram-token-filter-rejected = Ditolak — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = Daftar Hitam — { $count }
telegram-token-filter-coverage = <b>Cakupan</b>
telegram-token-filter-priced = Dengan Harga Pool — { $count }
telegram-token-filter-open = Posisi Terbuka — { $count }
telegram-token-filter-total = Total Ditemukan — { $count }
telegram-token-filter-updated = <b>Terakhir Diperbarui</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>Diperbarui otomatis setiap { $interval }</i>
telegram-token-detail-active = <b>Posisi Aktif</b>
telegram-token-detail-price = Harga — { $price } { -sol }
telegram-token-detail-liquidity = Likuiditas — { $value }
telegram-token-detail-volume = Volume 24h — { $value }
telegram-token-detail-change = Perubahan 24h — { $value }
telegram-token-detail-risk = Penilaian Risiko: { $score }/100
telegram-token-detail-risk-unknown = Penilaian Risiko: Tidak Diketahui
telegram-token-detail-action = <i>Pilih aksi:</i>
telegram-token-search =
    <b>Cari Pasar</b>

    Masukkan simbol atau alamat mint untuk mencari:

    <i>Contoh: /token_BONK atau /token_So11111</i>
telegram-token-confirm-buy =
    <b>Konfirmasi Beli Langsung</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>
    Jumlah — { $amount } { -sol }

    <i>Konfirmasi dalam 30 dtk untuk mengeksekusi.</i>
telegram-token-confirm-blacklist =
    <b>Daftar Hitamkan Token?</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>

    <i>Ini akan mencegah token ini memenuhi filter.</i>
telegram-token-blacklisted =
    <b>Token Masuk Daftar Hitam</b>

    Token — ${ $symbol }
    Status — Ditambahkan ke daftar hitam
telegram-token-blacklist-failed = <b>Daftar Hitam Gagal</b>
telegram-token-buy-processing =
    <b>Memproses Pembelian...</b>

    Token — ${ $symbol }
    Jumlah — { $amount } { -sol }
telegram-token-buy-done =
    <b>Pembelian Berhasil</b>

    Token — ${ $symbol }
    Jumlah — { $amount } { -sol }

    <i>Lihat detail di /positions</i>
telegram-token-buy-failed =
    <b>Pembelian Gagal</b>

    Token — ${ $symbol }
    Error — { $detail }
