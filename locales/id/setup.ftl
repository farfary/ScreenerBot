# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

setup-wallet-required = Masukkan private key dompet.
setup-wallet-json-recognized = Format key JSON 64-byte dikenali.
setup-wallet-json-invalid = Gunakan array JSON yang berisi tepat 64 nilai byte (0–255).
setup-wallet-format-invalid = Gunakan private key base58 atau array JSON 64-byte.
setup-wallet-base58-recognized = Format key Base58 dikenali.

setup-rpc-required = Masukkan setidaknya satu endpoint RPC.
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
setup-error-restart-timeout = Penyiapan tersimpan, tetapi { -brand } belum terhubung kembali.

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
setup-verify-save-skipped = Tidak disimpan
setup-verify-request-failed = Permintaan verifikasi gagal
setup-verify-summary-checking = Memeriksa koneksi dompet dan Solana mainnet Anda.
setup-verify-summary-running = Memverifikasi kredensial persis seperti yang Anda masukkan.
setup-verify-summary-saving = Kredensial terverifikasi. Menyimpan dengan aman.
setup-verify-summary-failed = Tinjau masalahnya, lalu verifikasi lagi.

setup-error-credentials-failed = Verifikasi kredensial gagal.
setup-error-save-failed = Penyiapan tidak dapat disimpan.
setup-error-verify-failed = Verifikasi gagal.
setup-error-explore-failed = Mode Jelajah tidak dapat dimulai.
setup-error-gateway-failed = Preferensi gateway tidak dapat disimpan.
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

setup-dialog-title = Siapkan dompet & RPC
setup-dialog-subtitle = Hubungkan dompet Solana Anda dan endpoint RPC premium untuk mengaktifkan trading dan data on-chain langsung. Private key Anda dienkripsi di perangkat ini dan tidak pernah keluar darinya.
setup-dialog-close =
    .title = Tutup
    .aria-label = Tutup
setup-dialog-wallet-label = Private key dompet
setup-dialog-wallet-input =
    .placeholder = String Base58 atau array JSON [1,2,3,...]
setup-dialog-rpc-label = Endpoint RPC
setup-dialog-rpc-input =
    .placeholder = https://endpoint-anda... (satu per baris)
setup-dialog-rpc-hint = Penyedia premium ({ -helius }, { -quicknode }, { -alchemy }) sangat disarankan — RPC Solana publik dibatasi lajunya dan mungkin tidak berfungsi.
setup-dialog-submit = Validasi & hubungkan
setup-dialog-working = Memproses…
setup-dialog-validating = Memvalidasi…
setup-dialog-saving = Menyimpan…
setup-dialog-restarting = Memulai ulang…
setup-dialog-saved = Penyiapan disimpan — memulai ulang { -brand } dalam mode penuh…
setup-dialog-error-missing-fields = Masukkan private key dompet dan setidaknya satu URL RPC.
setup-dialog-error-validation = Validasi gagal.
setup-dialog-error-incomplete = Penyiapan tidak dapat diselesaikan.
setup-dialog-error-restart-helper = Helper mulai ulang otomatis tidak tersedia. Muat ulang dasbor sebentar lagi.
setup-dialog-error-unexpected = Error tak terduga.

setup-wizard-progress =
    .aria-label = Progres penyiapan
setup-wizard-step-credentials = Kredensial
setup-wizard-step-verification = Verifikasi
setup-wizard-step-complete = Selesai
setup-wizard-credentials-title = Konfigurasi kredensial
setup-wizard-credentials-description = Hubungkan dompet lokal dan endpoint RPC Solana mainnet yang andal.
setup-wizard-wallet-toggle =
    .title = Tampilkan private key
    .aria-label = Tampilkan private key
setup-wizard-wallet-security-note = Dienkripsi sebelum disimpan.
setup-wizard-rpc-title = Endpoint RPC
setup-wizard-rpc-input =
    .placeholder = Satu URL HTTPS per baris
setup-wizard-rpc-guidance = RPC mainnet yang andal disarankan untuk polling berkelanjutan.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = disarankan
setup-wizard-gateway-title = Pengiriman transaksi gratis
setup-wizard-gateway-hint = Tersedia saat masuk. RPC Anda tetap tersedia sebagai cadangan.
setup-wizard-account-title = Akun { -brand }
setup-wizard-account-optional = Opsional
setup-wizard-account-loading = Memeriksa status akun…
setup-wizard-verify-title = Verifikasi dan simpan
setup-wizard-verify-list =
    .aria-label = Status verifikasi penyiapan
setup-wizard-verify-wallet = Dompet
setup-wizard-verify-rpc = RPC Solana
setup-wizard-verify-save = Konfigurasi aman
setup-wizard-complete-title = Penyiapan disimpan
setup-wizard-reconnect = Coba hubungkan lagi
setup-wizard-reload = Muat ulang dasbor
setup-wizard-error-title = Penyiapan perlu perhatian
setup-wizard-explore = Jelajahi dasbor
setup-wizard-continue = Lanjutkan
