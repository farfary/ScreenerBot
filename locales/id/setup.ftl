# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

setup-wallet-required = Masukkan private key dompet.
setup-wallet-json-recognized = Format key JSON 64-byte dikenali.
setup-wallet-json-invalid = Gunakan array JSON yang berisi tepat 64 nilai byte (0–255).
setup-wallet-format-invalid = Gunakan private key base58 atau array JSON 64-byte.
setup-wallet-base58-recognized = Format key Base58 dikenali.

setup-rpc-required = Masukkan setidaknya satu endpoint RPC.
setup-dialog-rpc-optional-note = Opsional: biarkan kolom kosong untuk mempertahankan endpoint RPC yang sudah dikonfigurasi.
setup-rpc-too-many = Gunakan tidak lebih dari 10 endpoint RPC.
setup-rpc-url-invalid = Setiap endpoint harus berupa URL HTTPS yang valid.
setup-rpc-url-credentials = URL RPC tidak boleh memuat nama pengguna atau kata sandi.
setup-rpc-url-fragment = URL RPC tidak boleh memuat fragmen.
setup-rpc-public-endpoint = RPC Solana publik tidak dapat mendukung polling berkelanjutan.
setup-rpc-private-host = Endpoint RPC tidak boleh menggunakan host lokal atau jaringan privat.
setup-rpc-duplicate = Hapus endpoint RPC yang duplikat.
setup-rpc-ready =
    { $count ->
       *[other] { $count } endpoint HTTPS siap diuji.
    }

setup-wallet-verified = Dompet terverifikasi
setup-wallet-unverified = Dompet tidak dapat diverifikasi
setup-wallet-address-detail = Alamat { $address }
setup-wallet-format-hint = Periksa format private key.
setup-rpc-none-working = Tidak ada RPC mainnet yang berfungsi
setup-rpc-health-failed = Tidak ada endpoint yang lolos pemeriksaan kesehatan mainnet.
setup-rpc-partial = { $working } berfungsi; { $failed } tidak tersedia
setup-rpc-verified =
    { $count ->
       *[other] { $count } endpoint mainnet terverifikasi
    }
setup-rpc-fastest = Tercepat: { $url } ({ $latency } ms).
setup-error-request-failed = Permintaan gagal ({ $status })
setup-error-restart-timeout = { -brand } belum terhubung kembali.

setup-verify-wallet-parsing = Mengurai private key
setup-verify-wallet-parsing-detail = Memeriksa key dan menurunkan alamat publiknya.
setup-verify-wallet-waiting = Menunggu validasi
setup-verify-rpc-testing = Menguji Solana mainnet
setup-verify-rpc-testing-detail =
    { $count ->
       *[other] Memeriksa { $count } endpoint.
    }
setup-verify-rpc-waiting = Menunggu pengujian endpoint
setup-verify-save-waiting = Menunggu penyimpanan
setup-verify-save-running = Mengenkripsi dan menyimpan
setup-verify-save-running-detail = Menulis konfigurasi terverifikasi di perangkat ini.
setup-verify-save-done = Konfigurasi disimpan
setup-verify-save-done-detail = Private key dienkripsi; endpoint RPC yang berfungsi disimpan.
setup-verify-save-failed = Tidak dapat menyimpan penyiapan
setup-verify-save-skipped = Tidak disimpan. Perbaiki masalah di atas, lalu verifikasi lagi.
setup-verify-request-failed = Permintaan verifikasi gagal
setup-verify-summary-checking = Memeriksa koneksi dompet dan Solana mainnet Anda.
setup-verify-summary-running = Memverifikasi kredensial persis seperti yang Anda masukkan.
setup-verify-summary-saving = Kredensial terverifikasi. Menyimpan dengan aman.
setup-verify-summary-failed = Tinjau masalahnya, lalu verifikasi lagi.

setup-error-credentials-failed = Verifikasi kredensial gagal.
setup-error-save-failed = Penyiapan tidak dapat disimpan.
setup-error-verify-failed = Verifikasi gagal.
setup-error-explore-failed = Mode Jelajah tidak dapat dimulai.
setup-error-gateway-failed = Pengaturan pengiriman transaksi tidak dapat disimpan.
setup-action-review-credentials = Tinjau kredensial

setup-explore-opening = Membuka Mode Jelajah…
setup-complete-restarting = Memulai ulang { -brand } dengan konfigurasi terverifikasi Anda.
setup-complete-finishing = Menyelesaikan mulai ulang…
setup-complete-ready = { -brand } siap. Membuka dasbor…
setup-complete-stored = Konfigurasi terverifikasi Anda tersimpan dengan aman di perangkat ini.

setup-wallet-show-key = Tampilkan private key
setup-wallet-hide-key = Sembunyikan private key
setup-wallet-copy =
    .aria-label = Salin alamat dompet
    .title = Salin alamat dompet
setup-wallet-copy-done =
    .aria-label = Alamat dompet disalin
    .title = Disalin
setup-wallet-copy-failed =
    .aria-label = Tidak dapat menyalin alamat dompet
    .title = Gagal menyalin

setup-dialog-close =
    .title = Tutup
    .aria-label = Tutup
setup-dialog-wallet-label = Private key dompet
setup-dialog-wallet-input =
    .placeholder = String Base58 atau array JSON [1,2,3,...]
setup-dialog-working = Memproses…
setup-dialog-validating = Memvalidasi…
setup-dialog-saving = Menyimpan…
setup-dialog-restarting = Memulai ulang…
setup-dialog-error-validation = Validasi gagal.
setup-dialog-error-incomplete = Penyiapan tidak dapat diselesaikan.
setup-dialog-error-restart-helper = Helper mulai ulang otomatis tidak tersedia. Muat ulang dasbor sebentar lagi.
setup-dialog-error-unexpected = Error tak terduga.
setup-dialog-replace-title = Ganti dompet trading
setup-dialog-replace-subtitle = Trading dengan dompet lain. Private key-nya dienkripsi di perangkat ini dan tidak pernah meninggalkannya.
setup-dialog-replace-submit = Validasi & ganti
setup-dialog-replace-saved = Penggantian disiapkan — memulai ulang { -brand } dengan dompet baru…
setup-dialog-replace-confirm-intro = Mengganti dompet trading:
setup-dialog-replace-confirm-stop = Menghentikan trader dan copy trading. Aktifkan kembali setelah dimulai ulang.
setup-dialog-replace-confirm-backup = Memindahkan posisi, transaksi, dan riwayat dompet saat ini ke folder cadangan.
setup-dialog-replace-confirm-keep = Menyimpan key dompet saat ini di Dompet Lain, tempat Anda tetap dapat mengekspornya.
setup-dialog-replace-confirm-funds = Tidak memindahkan dana: token dan { -sol } tetap di dompet saat ini.
setup-dialog-replace-confirm-rpc = Mengganti endpoint RPC yang dikonfigurasi dengan yang Anda masukkan; jika kolom dibiarkan kosong, endpoint saat ini tetap dipakai.
setup-dialog-replace-confirm-restart = Memulai ulang { -brand } dengan dompet baru.

setup-wizard-progress =
    .aria-label = Progres penyiapan
setup-wizard-wallet-security-note = Dienkripsi sebelum disimpan.
setup-wizard-rpc-title = Endpoint RPC
setup-wizard-rpc-input =
    .placeholder = Satu URL HTTPS per baris
setup-wizard-rpc-guidance = Gunakan penyedia mainnet yang andal; endpoint publik tidak didukung.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = disarankan
setup-wizard-account-title = Akun { -brand }
setup-wizard-account-optional = Opsional
setup-wizard-account-loading = Memeriksa status akun…
setup-wizard-verify-title = Verifikasi dan simpan
setup-wizard-verify-list =
    .aria-label = Status verifikasi penyiapan
setup-wizard-verify-wallet = Dompet
setup-wizard-verify-save = Konfigurasi aman
setup-wizard-complete-title = Penyiapan disimpan
setup-wizard-reconnect = Coba hubungkan lagi
setup-wizard-reload = Muat ulang dasbor
setup-wizard-error-title = Penyiapan perlu perhatian
setup-wizard-explore = Jelajahi dasbor
setup-wizard-continue = Lanjutkan

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = Wallet
setup-wizard-step-connection = Koneksi
setup-wizard-step-verify = Verifikasi
setup-wizard-step-ready = Siap
setup-wallet-title = Siapkan wallet trading Anda
setup-wallet-description = Wallet yang dipakai { -brand } untuk trading. Kuncinya dienkripsi di komputer ini dan tidak pernah keluar darinya.
setup-wallet-source =
    .aria-label = Sumber wallet
setup-wallet-create-title = Buat wallet baru
setup-wallet-create-detail = { -brand } membuat wallet baru. Anda menyimpan kuncinya, lalu mengirim { -sol } ke wallet itu.
setup-wallet-import-title = Impor wallet
setup-wallet-import-detail = Gunakan kunci privat yang sudah Anda miliki (base58 atau array JSON berisi 64 angka).
setup-wallet-creating = Membuat wallet Anda…
setup-wallet-create-failed = Wallet tidak dapat dibuat. Coba lagi.
setup-backup-title = Simpan kunci privat Anda
setup-backup-description = Siapa pun yang memegang kunci ini mengendalikan wallet. { -brand } tidak dapat memulihkannya.
setup-backup-address = Alamat
setup-backup-key = Kunci privat
setup-backup-copy-address = Salin alamat
setup-backup-download = Unduh
setup-backup-copied = Tersalin. Kosongkan clipboard setelah Anda menyimpannya.
setup-backup-machine-warning = Salinan terenkripsi yang disimpan { -brand } hanya bisa dibuka di komputer ini. Jika komputer hilang atau diganti, hanya kunci yang Anda simpan sekarang yang dapat memulihkan wallet. Simpan secara offline, misalnya di kertas atau di pengelola kata sandi.
setup-backup-ack-saved = Saya sudah menyimpan kunci privat di tempat aman, di luar komputer ini.
setup-backup-ack-control = Saya paham bahwa siapa pun yang memegang kunci ini dapat mengambil dana di wallet ini.
setup-backup-ack-recovery = Saya paham { -brand } tidak dapat memulihkan kunci ini. Tanpanya, komputer yang hilang berarti dana yang hilang.
setup-backup-create-another = Buat yang lain
setup-backup-replace-confirm = Buang kunci ini dan buat wallet baru? Lakukan ini hanya jika Anda belum mengirim apa pun ke wallet itu.
setup-connection-title = Bagaimana { -brand } terhubung ke { -solana }?
setup-connection-description = Anda dapat mengubahnya nanti di Pengaturan → Koneksi.
setup-connection-choice =
    .aria-label = Koneksi
setup-connection-service-title = Layanan { -brand }
setup-connection-service-badge = Tidak perlu RPC
setup-connection-service-detail = Menggunakan akun { -brand } gratis Anda.
setup-connection-service-prices = Harga diperbarui setiap beberapa detik
setup-connection-service-features = Copy trading, pemantauan wallet, saldo wallet lain, riwayat lama, swap pool langsung, dan harga pool langsung memerlukan RPC Anda sendiri
setup-connection-service-pause = Trading dijeda jika Anda keluar atau screenerbot.io tidak dapat dijangkau
setup-connection-service-privacy = Kunci privat Anda tidak pernah meninggalkan komputer ini: swap ditandatangani di sini, dan screenerbot.io hanya memakai alamat publik dompet Anda untuk membaca saldo dan transaksinya
setup-connection-own-title = RPC Anda sendiri
setup-connection-own-badge = Tercepat
setup-connection-own-detail = Harga sekitar dua kali per detik dan semua fitur. Memerlukan endpoint RPC dari penyedia.
setup-connection-signin-required = Masuk atau buat akun gratis untuk menggunakan layanan { -brand }.
setup-connection-reauthorize = Masuk lagi untuk mengizinkan pembacaan wallet oleh layanan { -brand }.
setup-connection-sending = Kirim transaksi melalui { -brand }
setup-connection-sending-hint = Tersedia saat Anda masuk. Jika tidak memungkinkan, RPC Anda yang dipakai.
setup-wizard-account-required = Wajib
setup-wizard-verify-connection = Koneksi
setup-verify-service-checking = Memeriksa layanan { -brand }
setup-verify-service-ready = Layanan { -brand } siap
setup-verify-service-busy = Layanan { -brand } sedang sibuk. Coba lagi sebentar lagi, atau gunakan RPC Anda sendiri.
setup-verify-service-daily-limit = Akun Anda mencapai batas hari ini untuk layanan { -brand }. Coba lagi setelah 00:00 UTC, atau gunakan RPC Anda sendiri.
setup-verify-service-unreachable = screenerbot.io tidak dapat dijangkau. Periksa koneksi internet Anda.
setup-verify-service-version = Versi ini tidak lagi didukung oleh layanan { -brand }. Perbarui { -brand } untuk melanjutkan.
setup-verify-service-signin = Masuk untuk menggunakan layanan { -brand }.
setup-verify-save-done-service-detail = Kunci privat dienkripsi; layanan { -brand } dipilih.
setup-action-try-again = Coba lagi
setup-action-use-own-rpc = Gunakan RPC saya sendiri
setup-action-sign-in-again = Masuk lagi
setup-ready-title = { -brand } siap
setup-fund-title = Isi dana wallet trading Anda
setup-fund-waiting = Menunggu { -sol }…
setup-fund-received = Diterima { $amount } { -sol }
setup-fund-balance-unavailable = Membaca saldo… Saldo diperbarui dengan sendirinya.
setup-fund-open-dashboard = Buka dasbor
