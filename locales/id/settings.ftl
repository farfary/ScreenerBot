## Shared

settings-duration-minutes =
    { $count ->
       *[other] { $count } menit
    }
settings-duration-hours =
    { $count ->
       *[other] { $count } jam
    }

## settings_dialog.js

settings-dialog-title = Pengaturan
settings-dialog-close =
    .title = Tutup (ESC)
    .aria-label = Tutup pengaturan
settings-dialog-save = Simpan Perubahan
settings-dialog-saving = Menyimpan...
settings-dialog-saved = Tersimpan
settings-dialog-save-success = Pengaturan berhasil disimpan
settings-dialog-save-failed = Gagal menyimpan pengaturan
settings-dialog-update-attention = Pembaruan perlu perhatian
settings-dialog-tab-interface = Antarmuka
settings-dialog-tab-navigation = Navigasi
settings-dialog-tab-startup = Startup
settings-dialog-tab-hints = Petunjuk
settings-dialog-tab-data = Data
settings-dialog-tab-security = Keamanan
settings-dialog-tab-account = Akun
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = Koneksi Agen
settings-dialog-tab-updates = Pembaruan
settings-dialog-tab-licenses = Lisensi
settings-dialog-tab-about = Tentang
settings-dialog-link-privacy = Kebijakan Privasi
settings-dialog-link-terms = Ketentuan Layanan

## settings_dialog.js: Startup tab

settings-startup-section-title = Perilaku Startup
settings-startup-auto-start-label = Mulai Otomatis Trader
settings-startup-auto-start-hint = Jalankan trader otomatis saat aplikasi dibuka
settings-startup-coming-soon = Segera Hadir
settings-startup-default-page-label = Halaman Default
settings-startup-default-page-hint = Halaman yang ditampilkan saat membuka aplikasi
settings-startup-page-dashboard = Dasbor
settings-startup-page-tokens = Token
settings-startup-page-positions = Posisi
settings-startup-page-wallet = Dompet
settings-startup-page-config = Konfigurasi
settings-startup-notifications-label = Tampilkan Notifikasi Latar Belakang
settings-startup-notifications-hint = Tampilkan notifikasi untuk peristiwa di latar belakang

## settings_dialog.js: About tab

settings-about-tagline = Mesin Trading Solana Native
settings-about-link-github = { -github }
settings-about-link-docs = Dokumentasi
settings-about-link-telegram = { -telegram }
settings-about-link-website = Situs web
settings-about-credits = Dibuat untuk trader Solana
settings-about-copyright = © { $year } { -brand }. Hak cipta dilindungi.

## interface_tab.js

settings-interface-section-appearance = Tampilan
settings-interface-theme-label = Tema
settings-interface-theme-hint = Pilih skema warna yang Anda sukai
settings-interface-theme-dark = Gelap
settings-interface-theme-light = Terang
settings-interface-language-label = Bahasa
settings-interface-language-hint = Bahasa tampilan dasbor
settings-interface-logo-shape-label = Bentuk Logo Token
settings-interface-logo-shape-hint = Lingkaran memotong setiap logo; Natural mempertahankan siluet asli tiap gambar
settings-interface-logo-shape-circle = Lingkaran
settings-interface-logo-shape-natural = Natural
settings-interface-animations-label = Aktifkan Animasi
settings-interface-animations-hint = Transisi dan efek yang halus
settings-interface-compact-label = Mode Ringkas
settings-interface-compact-hint = Kurangi padding agar muat lebih banyak konten
settings-interface-section-data = Data & Tampilan
settings-interface-refresh-label = Interval Penyegaran
settings-interface-refresh-hint = Seberapa sering data disegarkan
settings-interface-refresh-seconds =
    { $count ->
       *[other] { $count } detik
    }
settings-interface-refresh-minutes =
    { $count ->
       *[other] { $count } menit
    }
settings-interface-ticker-label = Tampilkan Bar Ticker
settings-interface-ticker-hint = Ticker metrik langsung di header
settings-interface-page-size-label = Ukuran Halaman Tabel
settings-interface-page-size-hint = Jumlah baris default per halaman tabel
settings-interface-page-size-rows =
    { $count ->
       *[other] { $count } baris
    }
settings-interface-auto-expand-label = Buka Kategori Otomatis
settings-interface-auto-expand-hint = Buka kategori konfigurasi secara default
settings-interface-hints-label = Tampilkan Petunjuk Kontekstual
settings-interface-hints-hint = Tampilkan ikon bantuan yang menjelaskan fitur dasbor
settings-interface-featured-label = Tampilkan Baris Unggulan
settings-interface-featured-hint = Tampilkan baris token unggulan di halaman Beranda dan Token
settings-interface-section-sound = Efek Suara
settings-interface-sounds-label = Aktifkan Suara
settings-interface-sounds-hint = Isyarat taktil untuk navigasi, perubahan status, dan hasil

## security_tab.js

settings-security-loading = Memuat pengaturan keamanan...
settings-security-load-failed = Gagal memuat pengaturan keamanan

settings-security-type-pin4 = PIN 4 Digit
settings-security-type-pin6 = PIN 6 Digit
settings-security-type-text = Kata Sandi Teks
settings-security-type-unset = Belum Diatur

settings-security-lockscreen-title = Layar Kunci Dasbor
settings-security-lockscreen-description = Lindungi dasbor Anda dengan PIN atau kata sandi. Layar kunci akan muncul saat dipicu dan memerlukan autentikasi untuk melanjutkan.
settings-security-enable-label = Aktifkan Layar Kunci
settings-security-enable-hint = Lindungi dasbor Anda dengan autentikasi kata sandi
settings-security-password-status-label = Status Kata Sandi
settings-security-password-current = Saat ini: { $type }
settings-security-password-none = Belum ada kata sandi
settings-security-change = Ubah
settings-security-remove = Hapus
settings-security-set-password = Atur Kata Sandi
settings-security-auto-lock-label = Kunci Otomatis Setelah Tidak Aktif
settings-security-auto-lock-hint = Kunci otomatis setelah periode tanpa aktivitas
settings-security-auto-lock-never = Tidak Pernah
settings-security-lock-blur-label = Kunci Saat Jendela Kehilangan Fokus
settings-security-lock-blur-hint = Kunci otomatis saat Anda beralih ke aplikasi lain
settings-security-quick-actions-title = Aksi Cepat
settings-security-lock-now-label = Kunci Dasbor Sekarang
settings-security-lock-now-hint = Kunci dasbor seketika
settings-security-needs-password = Atur kata sandi terlebih dahulu untuk memakai ini
settings-security-needs-lockscreen = Aktifkan layar kunci terlebih dahulu untuk memakai ini
settings-security-lock-now = Kunci Sekarang
settings-security-lock-not-ready = Tidak dapat mengunci - layar kunci belum siap
settings-security-setting-save-failed = Tidak dapat menyimpan pengaturan keamanan

## security_tab.js: two-factor authentication

settings-security-2fa-title = Autentikasi Dua Faktor
settings-security-2fa-description = Tambahkan lapisan keamanan ekstra dengan aplikasi autentikator (Google Authenticator, Authy, dll.)
settings-security-2fa-status-label = Status 2FA
settings-security-2fa-status-enabled = Autentikasi dua faktor aktif
settings-security-2fa-status-none = Belum dikonfigurasi
settings-security-2fa-disable = Nonaktifkan 2FA
settings-security-2fa-enable = Aktifkan 2FA

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = Tutup
settings-security-password-set-title = Atur Kata Sandi
settings-security-password-change-title = Ubah Kata Sandi
settings-security-password-current-label = Kata Sandi Saat Ini
settings-security-password-current-input =
    .placeholder = Masukkan kata sandi saat ini
settings-security-password-type-label = Jenis Kata Sandi
settings-security-password-new-label = Kata Sandi Baru
settings-security-password-new-input =
    .placeholder = Masukkan kata sandi baru
settings-security-password-confirm-label = Konfirmasi Kata Sandi
settings-security-password-confirm-input =
    .placeholder = Konfirmasi kata sandi
settings-security-password-update = Perbarui Kata Sandi
settings-security-placeholder-pin4 = Masukkan PIN 4 digit
settings-security-placeholder-pin6 = Masukkan PIN 6 digit
settings-security-placeholder-text = Masukkan kata sandi
settings-security-password-required = Masukkan kata sandi
settings-security-password-mismatch = Kata sandi tidak cocok
settings-security-pin4-invalid = PIN harus tepat 4 digit
settings-security-pin6-invalid = PIN harus tepat 6 digit
settings-security-text-too-short = Kata sandi minimal 4 karakter
settings-security-password-saved = Kata sandi disimpan
settings-security-password-save-failed = Gagal menyimpan kata sandi
settings-security-password-save-failed-detail = Gagal menyimpan kata sandi: { $message }

## security_tab.js: remove password dialog

settings-security-remove-title = Hapus Kata Sandi
settings-security-remove-description = Masukkan kata sandi Anda saat ini untuk menghapus perlindungan layar kunci.
settings-security-remove-confirm = Hapus Kata Sandi
settings-security-current-required = Masukkan kata sandi Anda saat ini
settings-security-password-removed = Kata sandi dihapus
settings-security-password-remove-failed = Gagal menghapus kata sandi
settings-security-password-remove-failed-detail = Gagal menghapus kata sandi: { $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = Aktifkan Autentikasi Dua Faktor
settings-security-2fa-password-prompt = Masukkan kata sandi Anda untuk melanjutkan:
settings-security-2fa-password-input =
    .placeholder = Masukkan kata sandi
settings-security-2fa-continue = Lanjutkan
settings-security-2fa-manual-code = Kode entri manual:
settings-security-2fa-qr =
    .alt = Kode QR TOTP
settings-security-2fa-code-prompt = Masukkan kode 6 digit dari aplikasi autentikator Anda:
settings-security-2fa-verify-enable = Verifikasi & Aktifkan
settings-security-2fa-password-required = Masukkan kata sandi Anda
settings-security-2fa-setup-failed = Gagal menyiapkan 2FA
settings-security-2fa-code-invalid-length = Masukkan kode 6 digit
settings-security-2fa-code-invalid = Kode tidak valid
settings-security-2fa-enabled = Autentikasi dua faktor diaktifkan
settings-security-2fa-verify-failed = Gagal memverifikasi kode
settings-security-2fa-disable-title = Nonaktifkan Autentikasi Dua Faktor
settings-security-2fa-disable-prompt = Masukkan kata sandi Anda untuk menonaktifkan 2FA:
settings-security-2fa-disable-failed = Gagal menonaktifkan 2FA
settings-security-2fa-disabled = Autentikasi dua faktor dinonaktifkan

## agent_connections_tab.js

settings-agent-category-analysis = Analisis
settings-agent-category-portfolio = Portofolio
settings-agent-category-trading = Trading
settings-agent-category-config = Konfigurasi
settings-agent-category-system = Sistem
settings-agent-category-analysis-description = Analisis token, data pasar, dan pemeriksaan keamanan.
settings-agent-category-portfolio-description = Posisi terbuka, saldo, dan P&L.
settings-agent-category-trading-description = Membeli, menjual, dan menutup posisi dengan dana sungguhan.
settings-agent-category-config-description = Semua pengaturan bot, termasuk endpoint RPC. Tidak pernah kunci dompet.
settings-agent-category-system-description = Status, peristiwa, dan penghentian darurat.
settings-agent-category-analysis-inline = analisis
settings-agent-category-portfolio-inline = portofolio
settings-agent-category-trading-inline = trading
settings-agent-category-config-inline = konfigurasi
settings-agent-category-system-inline = sistem

settings-agent-level-allow = Izinkan
settings-agent-level-ask-user = Tanya
settings-agent-level-deny = Nonaktif
settings-agent-level-allow-hint = Langsung dijalankan.
settings-agent-level-ask-user-hint = Menunggu persetujuan Anda di aplikasi.
settings-agent-level-deny-hint = Ditolak dan disembunyikan dari agen.

settings-agent-preset-full = Akses penuh
settings-agent-preset-ask = Tanya dulu
settings-agent-preset-read = Hanya baca
settings-agent-preset-full-description = Semua berjalan tanpa bertanya. Kunci dompet tetap tidak dapat diakses.
settings-agent-preset-ask-description = Setiap aksi menunggu persetujuan Anda di aplikasi.
settings-agent-preset-read-description = Pembacaan analisis dan portofolio. Tidak ada yang dapat diubah.
settings-agent-preset-custom = Kustom
settings-agent-preset-group =
    .aria-label = Preset izin
settings-agent-permission-group = Izin { $category }

settings-agent-summary-asks-only = Terbatas — meminta persetujuan untuk { $asking }
settings-agent-summary-off-only = Terbatas — tanpa { $off }
settings-agent-summary-asks-and-off = Terbatas — meminta persetujuan untuk { $asking }; tanpa { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = Stdio MCP generik

settings-agent-note-placeholder = Ganti /absolute/path/to/screenerbot dengan path absolut ke binary { -brand } Anda — aplikasi yang berjalan tidak dapat menampilkan path executable-nya di sistem ini.
settings-agent-note-data-dir = Jika Anda menjalankan { -brand } dengan direktori data non-default, atur juga SCREENERBOT_DATA_DIR pada klien (flag -e / --env tambahan, atau entri env) ke path yang sama.
settings-agent-note-codex-run = Jalankan perintah, atau tambahkan blok TOML ke ~/.codex/config.toml ($CODEX_HOME/config.toml). Mulai ulang { -codex } setelahnya.
settings-agent-note-codex-get = `codex mcp get screenerbot` menyamarkan secret pada outputnya.
settings-agent-note-claude-code = { -claude } Code: jalankan perintah, lalu mulai ulang { -claude } Code. `claude mcp get screenerbot` akan mencetak environment yang dikonfigurasi, termasuk secret.
settings-agent-note-claude-desktop = { -claude } Desktop: gabungkan JSON ke claude_desktop_config.json di bawah `mcpServers` lalu mulai ulang aplikasi.
settings-agent-note-openclaw = Jalankan perintah, lalu gunakan `openclaw mcp doctor screenerbot --probe` untuk memastikan server stdio yang tersimpan berjalan dan menyediakan tool.
settings-agent-note-hermes = Tambahkan ini di bawah `mcp_servers` pada file konfigurasi { -hermes }, lalu mulai ulang { -hermes }.
settings-agent-note-generic = Klien MCP apa pun yang mendukung stdio: jalankan perintah ini dengan args dan environment berikut, di mana pun klien menyimpan daftar servernya.
settings-agent-block-codex-command = { -codex } CLI — perintah terminal
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (cadangan)
settings-agent-block-claude-command = { -claude } Code — perintah terminal
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — perintah terminal
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = Klien stdio MCP generik

settings-agent-name-required = Masukkan nama untuk koneksi ini.
settings-agent-name-too-long = Nama maksimal { $max } karakter.
settings-agent-name-control-characters = Nama tidak boleh berisi karakter kontrol.

settings-agent-title = Koneksi Agen
settings-agent-description = Hubungkan { -claude }, { -codex }, { -hermes }, { -openclaw }, atau klien stdio MCP apa pun. { -brand } harus tetap berjalan. Setiap koneksi memiliki izinnya sendiri: akses penuh secara default, dapat dibatasi per koneksi kapan saja. Tidak ada koneksi yang dapat membaca atau mengubah kunci dompet Anda.
settings-agent-name-label = Nama koneksi
settings-agent-name-hint = Ditampilkan di daftar di bawah agar Anda dapat membedakan koneksi.
settings-agent-name-input =
    .placeholder = Agen coding laptop
settings-agent-client-label = Klien
settings-agent-client-hint = Menentukan panduan penyiapan yang ditampilkan setelah koneksi dibuat.
settings-agent-permissions-label = Izin
settings-agent-permissions-hint = Koneksi baru dapat melakukan semuanya. Batasi kategori apa pun sekarang, atau nanti dari daftar di bawah — kunci dompet tidak pernah dapat diakses dalam kondisi apa pun.
settings-agent-create = Buat koneksi
settings-agent-issued-group =
    .aria-label = Kredensial koneksi baru
settings-agent-issued-warning = Salin secret sekarang. Secret hanya ditampilkan sekali dan tidak dapat diambil lagi — cabut dan buat ulang koneksi jika hilang. { -brand } hanya menyimpan verifier satu arah; klien MCP Anda menyimpan teks aslinya di konfigurasinya sendiri.
settings-agent-issued-client-id = ID Klien
settings-agent-issued-secret = Secret sekali pakai
settings-agent-setup-for = Penyiapan untuk
settings-agent-done = Selesai
settings-agent-list-title = Koneksi
settings-agent-loading = Memuat koneksi...
settings-agent-active-count = { $count } aktif
settings-agent-empty = Belum ada koneksi. Buat satu di atas untuk memasangkan klien.
settings-agent-empty-active = Tidak ada koneksi aktif.
settings-agent-revoked-title = Koneksi yang dicabut
settings-agent-created = Dibuat { $time }
settings-agent-last-used = Terakhir digunakan { $time }
settings-agent-never-used = Belum pernah digunakan
settings-agent-permissions-edit = Izin
settings-agent-revoke = Cabut
settings-agent-permissions-save = Simpan izin

settings-agent-load-failed = Gagal memuat Koneksi Agen
settings-agent-list-failed = Tidak dapat memuat koneksi
settings-agent-create-failed = Tidak dapat membuat koneksi.
settings-agent-unreachable-create = Tidak dapat menjangkau { -brand } untuk membuat koneksi.
settings-agent-permissions-update-failed = Tidak dapat memperbarui izin
settings-agent-permissions-updated = Izin diperbarui
settings-agent-permissions-updated-detail = Berlaku pada permintaan berikutnya dari koneksi ini.
settings-agent-unreachable-save = Tidak dapat menjangkau { -brand } untuk menyimpan
settings-agent-revoke-title = Cabut koneksi
settings-agent-revoke-message = Cabut "{ $label }"? Klien akan berhenti berfungsi pada permintaan berikutnya dan tidak dapat dipulihkan.
settings-agent-revoke-fallback-name = koneksi ini
settings-agent-revoke-failed = Tidak dapat mencabut koneksi
settings-agent-unreachable-revoke = Tidak dapat menjangkau { -brand } untuk mencabut

## telegram_tab.js

settings-telegram-loading = Memuat pengaturan { -telegram }...
settings-telegram-load-failed = Gagal memuat pengaturan { -telegram }
settings-telegram-unknown = Tidak diketahui
settings-telegram-session-active = Aktif: { $duration }
settings-telegram-sessions-empty = Tidak ada sesi aktif
settings-telegram-session-revoke = Cabut

settings-telegram-connection-title = Koneksi
settings-telegram-connection-description = Hubungkan bot { -telegram } Anda untuk menerima notifikasi dan mengendalikan { -brand } dari jarak jauh.
settings-telegram-enable-label = Aktifkan { -telegram }
settings-telegram-enable-hint = Aktifkan integrasi bot { -telegram }
settings-telegram-token-label = Token Bot
settings-telegram-token-saved = Token disimpan
settings-telegram-token-help = Dapatkan dari @BotFather di { -telegram }
settings-telegram-token-input-saved =
    .placeholder = Token disimpan (masukkan yang baru untuk mengubah)
settings-telegram-token-input =
    .placeholder = Masukkan token bot
settings-telegram-token-toggle =
    .title = Tampilkan/Sembunyikan
settings-telegram-chat-label = ID Chat
settings-telegram-chat-connected = Terhubung ke chat:
settings-telegram-chat-discover-hint = Temukan ID chat Anda secara otomatis
settings-telegram-chat-change =
    .title = Ubah
settings-telegram-chat-discover = Temukan ID Chat
settings-telegram-discovery-step-add = Tambahkan bot Anda ke grup { -telegram }, atau mulai chat langsung dengannya
settings-telegram-discovery-step-privacy = Untuk grup: Periksa @BotFather → /mybots → [bot Anda] → Bot Settings → Group Privacy
settings-telegram-discovery-privacy = <strong>Privacy Mode OFF:</strong> Bot menerima semua pesan grup<br/><strong>Privacy Mode ON:</strong> Bot hanya menerima pesan saat di-@mention
settings-telegram-discovery-step-send = Kirim pesan apa pun (atau @mention bot Anda jika Privacy Mode ON)
settings-telegram-discovery-listening = Menunggu pesan...
settings-telegram-discovery-select = Pilih
settings-telegram-chat-id-label = ID:
settings-telegram-language-label = Bahasa pesan
settings-telegram-language-hint = Bahasa pesan dan tombol bot { -telegram }
settings-telegram-language-follow-app = Ikuti bahasa aplikasi
settings-telegram-test-label = Uji Koneksi
settings-telegram-test-hint = Kirim pesan uji untuk memverifikasi konfigurasi
settings-telegram-test-send = Kirim Uji
settings-telegram-test-sending = Mengirim...

settings-telegram-chat-type-private = pribadi
settings-telegram-chat-type-group = grup
settings-telegram-chat-type-supergroup = supergrup
settings-telegram-chat-type-channel = kanal

settings-telegram-auth-title = Autentikasi Perintah
settings-telegram-auth-description = Perintah { -telegram } menggunakan 2FA yang sama dengan layar kunci dasbor.
settings-telegram-auth-protected = Terlindungi
settings-telegram-auth-disabled = Nonaktif
settings-telegram-auth-not-configured = Belum Dikonfigurasi
settings-telegram-auth-error = Error
settings-telegram-auth-protected-note = Perintah dilindungi oleh 2FA layar kunci. Saat sesi berakhir, pengguna harus memberikan kode autentikator melalui perintah <code>/login</code>.
settings-telegram-auth-disabled-note = 2FA layar kunci sudah dikonfigurasi tetapi dinonaktifkan untuk { -telegram }. Aktifkan "Wajibkan 2FA untuk Perintah" di atas untuk melindungi perintah { -telegram }.
settings-telegram-auth-missing-note = 2FA layar kunci belum dikonfigurasi. Tanpa 2FA, sesi yang berakhir akan diaktifkan kembali otomatis tanpa verifikasi.
settings-telegram-auth-managed-in = 2FA dikelola di
settings-telegram-auth-configure-in = Konfigurasikan 2FA di
settings-telegram-auth-configure-suffix = untuk mewajibkan verifikasi pada perintah { -telegram }.
settings-telegram-security-link = Pengaturan Keamanan
settings-telegram-timeout-title = Batas Waktu Sesi
settings-telegram-timeout-description = Berapa lama sesi terautentikasi tetap aktif
settings-telegram-sessions-title = Sesi Aktif

settings-telegram-notifications-title = Pengaturan Notifikasi
settings-telegram-notifications-description = Pilih peristiwa yang memicu notifikasi { -telegram }.
settings-telegram-notify-opened-label = Posisi Dibuka
settings-telegram-notify-opened-hint = Beri tahu saat posisi baru dibuka
settings-telegram-notify-closed-label = Posisi Ditutup
settings-telegram-notify-closed-hint = Beri tahu saat posisi ditutup
settings-telegram-notify-partial-label = Exit Parsial
settings-telegram-notify-partial-hint = Beri tahu saat terjadi exit parsial posisi
settings-telegram-notify-dca-label = DCA Dijalankan
settings-telegram-notify-dca-hint = Beri tahu saat order DCA dijalankan
settings-telegram-notify-errors-label = Error
settings-telegram-notify-errors-hint = Beri tahu saat terjadi error dan kegagalan
settings-telegram-notify-startup-label = Startup/Shutdown
settings-telegram-notify-startup-hint = Beri tahu saat bot mulai atau berhenti
settings-telegram-notify-filtering-label = Peringatan Pemfilteran
settings-telegram-notify-filtering-hint = Beri tahu saat token baru lolos kriteria filter
settings-telegram-notify-trades-label = Peringatan Trade
settings-telegram-notify-trades-hint = Beri tahu saat ada trade signifikan pada token yang dipantau
settings-telegram-notify-daily-label = Ringkasan Harian
settings-telegram-notify-daily-hint = Terima ringkasan aktivitas trading dan P&L harian

settings-telegram-features-title = Fitur
settings-telegram-features-description = Atur kemampuan bot { -telegram }.
settings-telegram-commands-label = Aktifkan Perintah
settings-telegram-commands-hint = Izinkan pengendalian bot melalui perintah { -telegram }
settings-telegram-require-2fa-label = Wajibkan 2FA untuk Perintah
settings-telegram-require-2fa-hint = Saat sesi berakhir, wajibkan kode 2FA untuk mengaktifkan kembali. Menggunakan 2FA layar kunci.
settings-telegram-inline-label = Tombol Aksi Inline
settings-telegram-inline-hint = Tampilkan tombol aksi di pesan notifikasi

settings-telegram-setting-save-failed = Tidak dapat menyimpan pengaturan { -telegram }
settings-telegram-discovery-start-failed = Tidak dapat memulai pencarian
settings-telegram-chat-selected = Chat dipilih
settings-telegram-chat-select-failed = Tidak dapat memilih chat
settings-telegram-test-sent = Pesan uji terkirim
settings-telegram-test-failed = Pesan uji gagal
settings-telegram-session-revoked = Sesi dicabut
settings-telegram-session-revoke-failed = Tidak dapat mencabut sesi

## licenses_tab.js

settings-licenses-title = Lisensi Sumber Terbuka
settings-licenses-subtitle = { -brand } dibangun dengan perangkat lunak sumber terbuka berikut
settings-licenses-footer = Teks lisensi lengkap tersedia di repositori proyek dan di dalam kode sumber setiap dependensi.
settings-licenses-category-framework = Framework Aplikasi
settings-licenses-category-solana = Blockchain Solana
settings-licenses-category-data = Data & Penyimpanan
settings-licenses-category-networking = Jaringan
settings-licenses-category-cryptography = Kriptografi & Encoding
settings-licenses-category-assets = Aset UI
settings-licenses-desc-electron = Framework aplikasi desktop
settings-licenses-desc-tokio = Runtime async untuk Rust
settings-licenses-desc-axum = Framework server web
settings-licenses-desc-tower = Abstraksi layanan
settings-licenses-desc-hyper = Implementasi HTTP
settings-licenses-desc-solana-sdk = Inti SDK Solana
settings-licenses-desc-solana-client = Klien RPC
settings-licenses-desc-solana-program = Pustaka program
settings-licenses-desc-spl-token = Program SPL Token
settings-licenses-desc-spl-token-2022 = Ekstensi Token-2022
settings-licenses-desc-spl-associated-token-account = Akun token terasosiasi
settings-licenses-desc-sqlite = Mesin database tertanam
settings-licenses-desc-rusqlite = Binding SQLite untuk Rust
settings-licenses-desc-r2d2 = Pool koneksi database
settings-licenses-desc-serde = Framework serialisasi
settings-licenses-desc-toml = Parsing konfigurasi
settings-licenses-desc-reqwest = Klien HTTP
settings-licenses-desc-tokio-tungstenite = Klien WebSocket
settings-licenses-desc-rustls = Implementasi TLS
settings-licenses-desc-blake3 = Fungsi hash
settings-licenses-desc-sha-2 = Hashing SHA-256/512
settings-licenses-desc-bs58 = Encoding Base58
settings-licenses-desc-base64 = Encoding Base64
settings-licenses-desc-lucide-icons = Pustaka font ikon
settings-licenses-desc-inter = Font antarmuka
settings-licenses-desc-jetbrains-mono = Font monospace
settings-licenses-desc-orbitron = Font tampilan
settings-licenses-desc-vazirmatn = Font aksara Arab dan Persia
settings-licenses-desc-noto-sans-devanagari = Font aksara Dewanagari
settings-licenses-desc-noto-sans-sc = Font Tionghoa Sederhana
settings-licenses-desc-pretendard = Font Korea
settings-licenses-desc-pretendard-jp = Font Jepang

## hints_tab.js

settings-hints-title = Petunjuk Kontekstual
settings-hints-description = Petunjuk kontekstual adalah ikon bantuan yang menjelaskan fitur dasbor. Tinjau setiap petunjuk di bawah dan pulihkan yang Anda sembunyikan dengan "Jangan tampilkan lagi" — satu per satu atau sekaligus.
settings-hints-hidden-label = Petunjuk Tersembunyi
settings-hints-hidden-summary = { $hidden } dari { $total } petunjuk saat ini disembunyikan.
settings-hints-restore-all = Pulihkan Semua Petunjuk
settings-hints-toggle-shown =
    .title = Tampilkan petunjuk ini
settings-hints-toggle-shown-title = Ditampilkan
settings-hints-toggle-hidden-title = Disembunyikan — aktifkan untuk menampilkan
settings-hints-restore-title = Pulihkan Semua Petunjuk
settings-hints-restore-message = Tampilkan kembali semua petunjuk kontekstual, termasuk yang pernah Anda sembunyikan?
settings-hints-restore-confirm = Pulihkan Semua
settings-hints-restored = Semua petunjuk dipulihkan

## account_tab.js

settings-account-title = Akun { -brand }
settings-account-description = Gratis dan opsional. { -brand } melakukan trading, penemuan token, dan pembuatan grafik tanpa akun — hanya saja melalui penyedia publik. Panel di bawah mencantumkan apa yang ditambahkan saat Anda masuk.
settings-account-data-title = Data { -brand }
settings-account-data-description = Kami menjalankan layanan data pasar bersama di screenerbot.io: candle gabungan di tujuh timeframe, registri pool yang sudah teresolusi, laporan keamanan yang di-cache, dan identitas token yang dinormalisasi. Layanan ini ada agar setiap instalasi tidak terkena rate limit terpisah dari penyedia publik, dan penggunaannya memerlukan akun agar biaya bersama tersebut memiliki penanggung jawab.
settings-account-data-fallback = Saat layanan tidak tersedia, { -brand } beralih otomatis ke penyedia publik. Tidak ada yang berhenti; grafik terisi lebih lambat dan riwayatnya lebih sedikit.
settings-account-gateway-title = Mengirim transaksi
settings-account-gateway-description = Saat Anda masuk, { -brand } dapat menyiarkan swap Anda melalui screenerbot.io alih-alih RPC Anda sendiri. Bot Anda tetap membuat dan menandatangani setiap transaksi di mesin ini — server hanya meneruskannya, dan tidak dapat mengubah transaksi bertanda tangan tanpa membatalkan signature-nya.
settings-account-gateway-label = Gunakan RPC { -brand } untuk mengirim transaksi
settings-account-gateway-hint = Hanya untuk pengiriman. Data harga selalu berasal dari RPC Anda sendiri — polling pool terlalu berat untuk endpoint bersama, sehingga tidak pernah dikirim ke sana.
settings-account-manage-title = Mengelola akun Anda
settings-account-manage-description = Kata sandi, alamat email, perangkat terhubung, dan pembayaran referral Anda dikelola di situs web. Mencabut perangkat di sana akan mengeluarkannya dari semua tempat, termasuk yang ini.
settings-account-open-dashboard = Buka dasbor Anda

## navigation_tab.js

settings-navigation-title = Tab Navigasi
settings-navigation-hint = Seret item untuk mengurutkan ulang. Atur visibilitas dengan sakelar.
settings-navigation-section-layout = Tata Letak
settings-navigation-overflow-label = Tab yang Tidak Muat
settings-navigation-overflow-hint = Gulir baris tab ke samping, atau kumpulkan tab yang tidak muat di menu Lainnya di ujungnya.
settings-navigation-overflow-scroll = Gulir
settings-navigation-overflow-menu = Menu Lainnya
settings-navigation-drag-handle =
    .title = Seret untuk mengurutkan ulang
settings-navigation-defaults-failed = Tidak dapat memuat navigasi default
settings-navigation-reset = Navigasi dikembalikan ke default

## data_tab.js

settings-data-storage-title = Penyimpanan Database
settings-data-storage-description = Ringkasan semua database yang menyimpan data trading, posisi, dan informasi historis Anda.
settings-data-stats-loading = Memuat statistik database...
settings-data-stats-load-failed = Gagal memuat statistik database
settings-data-total-storage = Total Penyimpanan Database
settings-data-db-tokens = Token
settings-data-db-transactions = Transaksi
settings-data-db-positions = Posisi
settings-data-db-events = Peristiwa
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = Dompet
settings-data-db-pools = Pool
settings-data-db-strategies = Strategi
settings-data-db-actions = Aksi
settings-data-directory-label = Direktori Data
settings-data-directory-copied = Direktori data
settings-data-config-path-copied = Path konfigurasi
settings-data-path-unavailable = Tidak tersedia
settings-data-path-copy-title = Klik untuk menyalin path
settings-data-path-copy-failed = Gagal menyalin path

settings-data-config-title = Pengelolaan Konfigurasi
settings-data-config-description = Ekspor, impor, dan kelola konfigurasi bot Anda. Buat cadangan sebelum melakukan perubahan besar.
settings-data-config-export = Ekspor Konfigurasi
settings-data-config-import = Impor Konfigurasi
settings-data-config-reset = Kembalikan ke Default
settings-data-config-location-label = Lokasi Konfigurasi
settings-data-config-fetch-failed = Gagal mengambil konfigurasi
settings-data-config-exported = Konfigurasi diekspor
settings-data-config-export-failed = Gagal mengekspor konfigurasi: { $message }
settings-data-config-import-title = Impor Konfigurasi
settings-data-config-import-message = Impor konfigurasi ini? Pengaturan saat ini akan ditimpa. Kredensial dompet akan dipertahankan.
settings-data-config-imported = Konfigurasi berhasil diimpor. Beberapa perubahan mungkin memerlukan restart.
settings-data-config-import-failed = Gagal mengimpor konfigurasi: { $message }
settings-data-config-reset-title = Atur Ulang Konfigurasi
settings-data-config-reset-message = Kembalikan semua pengaturan ke default? Kredensial dompet Anda akan dipertahankan, tetapi semua pengaturan lain akan diatur ulang.
settings-data-config-reset-done = Konfigurasi dikembalikan ke default
settings-data-config-reset-failed = Gagal mengatur ulang konfigurasi: { $message }
settings-data-unknown-error = Kesalahan tidak dikenal

settings-data-cleanup-title = Pembersihan Data
settings-data-cleanup-description = Bebaskan ruang disk dengan menghapus data lama atau tidak terpakai. Aksi ini tidak dapat dibatalkan.
settings-data-ohlcv-cleanup-label = Pembersihan Data OHLCV
settings-data-ohlcv-cleanup-hint = Hapus data candlestick untuk token yang tidak aktif selama waktu yang ditentukan.
settings-data-cleanup-hours-unit = jam
settings-data-cleanup-ohlcv = Bersihkan OHLCV
settings-data-cleanup-running = Membersihkan...
settings-data-cleanup-hours-invalid = Nilai jam tidak valid
settings-data-cleanup-confirm-title = Hapus Data OHLCV
settings-data-cleanup-confirm-message =
    Hapus data OHLCV untuk token yang tidak aktif lebih dari { $hours ->
       *[other] { $hours } jam
    }?
settings-data-cleanup-done =
    Membersihkan { $count ->
       *[other] { $count } token tidak aktif
    }
settings-data-cleanup-failed = Pembersihan gagal
settings-data-cleanup-failed-detail = Pembersihan gagal: { $message }

settings-data-cache-clear-label = Hapus Semua Cache OHLCV
settings-data-cache-clear-hint = Hapus semua data candlestick yang di-cache dan ambil ulang semua token yang dipantau dari awal. Gunakan jika grafik terlihat salah atau setelah pembaruan logika data.
settings-data-cache-clear = Hapus Cache OHLCV
settings-data-cache-clearing = Menghapus...
settings-data-cache-confirm-title = Hapus Semua Cache OHLCV
settings-data-cache-confirm-message = Hapus semua data candlestick yang di-cache untuk setiap token? Token yang dipantau akan mengambil ulang riwayatnya dari awal. Tindakan ini tidak dapat dibatalkan.
settings-data-candles-count =
    { $count ->
       *[other] { $count } candle
    }
settings-data-tokens-count =
    { $count ->
       *[other] { $count } token
    }
settings-data-cache-cleared = Menghapus { $candles } dari { $tokens }; mengambil ulang
settings-data-cache-clear-failed = Gagal menghapus cache OHLCV
settings-data-cache-clear-failed-detail = Gagal menghapus cache OHLCV: { $message }

settings-data-ui-cache-label = Cache Status UI
settings-data-ui-cache-hint = Hapus preferensi tabel, status filter, dan pengaturan tampilan yang tersimpan.
settings-data-ui-cache-clear = Hapus Cache UI
settings-data-ui-cache-confirm-title = Hapus Status UI
settings-data-ui-cache-confirm-message = Hapus semua preferensi UI yang tersimpan? Ini akan mengatur ulang kolom tabel, filter, dan pengaturan tampilan.
settings-data-ui-cache-cleared =
    Menghapus { $count ->
       *[other] { $count } pengaturan UI yang di-cache
    }

settings-data-folder-label = Buka Folder Data
settings-data-folder-hint = Buka folder berisi semua data { -brand } di pengelola file Anda.
settings-data-folder-open = Buka Folder
settings-data-folder-open-failed = Tidak dapat membuka folder data
