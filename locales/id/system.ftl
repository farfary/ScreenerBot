# Results of system and configuration operations. Ids come from
# src/webserver/routes/config/operations.rs (diff) and
# src/webserver/routes/config/import_export.rs (import).

system-result-config-differs = Konfigurasi di memori berbeda dari versi di disk
system-result-config-matches = Konfigurasi di memori sama dengan versi di disk

system-result-config-imported =
    Berhasil mengimpor { $count ->
       *[other] { $count } bagian
    }
system-result-config-imported-with-warnings =
    Mengimpor { $count ->
       *[other] { $count } bagian
    } dengan { $warnings ->
       *[other] { $warnings } peringatan
    }: { $details }

system-config-search =
    .placeholder = Cari pengaturan...
system-config-export-title =
    .title = Ekspor konfigurasi ke file
system-config-import-title =
    .title = Impor konfigurasi dari file
system-config-reload = Muat Ulang dari Disk
system-config-reset-defaults = Reset ke Default
system-config-select-section = Pilih bagian konfigurasi
system-config-select-section-details = Pilih bagian konfigurasi untuk melihat detail.
system-config-no-metadata = Tidak ada metadata untuk <code>{ $section }</code>
system-config-technical-settings = Pengaturan Teknis
system-config-expand-title = Bentangkan setiap bagian dan setiap sub-konfigurasi bersarang
system-config-collapse-title = Ciutkan setiap bagian dan setiap sub-konfigurasi bersarang
system-config-toolbar-no-changes = Tidak ada perubahan bagian
system-config-toolbar-section-changes =
    { $count ->
       *[other] <strong>{ $count }</strong> perubahan di bagian
    }
system-config-toolbar-total-changes =
    { $count ->
       *[other] <strong>{ $count }</strong> total perubahan
    }

system-config-loading = Memuat konfigurasi…
system-config-refreshing = Menyegarkan konfigurasi…
system-config-saving-title = Menyimpan perubahan…
system-config-saving-detail = Memperbarui konfigurasi
system-config-validation-issues = <strong>Masalah validasi terdeteksi.</strong> Periksa kolom yang disorot.

system-config-save-changes = Simpan Perubahan
system-config-saving = Menyimpan…
system-config-compare = Bandingkan dengan Disk
system-config-revert-section = Kembalikan Bagian
system-config-summary-critical = { $count } kritis
system-config-summary-performance = { $count } performa
system-config-summary-pending =
    { $count ->
       *[other] { $count } perubahan tertunda
    }
system-config-summary-none = Tidak ada ringkasan metadata
system-config-fields-count =
    { $count ->
       *[other] { $count } kolom
    }
system-config-chip-pending = { $fields } · { $pending } tertunda
system-config-chip-visible = { $visible } dari { $fields }

system-config-field-default = Default: { $value }
system-config-field-reset = Reset ke default
system-config-array-invalid-title = Entri array tidak valid
system-config-json-invalid-title = JSON tidak valid
system-config-list-separator = { ", " }
system-config-array-invalid-integer =
    { $count ->
       *[other] Baris { $lines } harus berupa bilangan bulat yang valid.
    }
system-config-array-invalid-number =
    { $count ->
       *[other] Baris { $lines } harus berupa angka yang valid.
    }
system-config-array-invalid-boolean =
    { $count ->
       *[other] Baris { $lines } harus berupa boolean yang valid.
    }
system-config-array-invalid-value =
    { $count ->
       *[other] Baris { $lines } harus berupa nilai yang valid.
    }

system-config-telegram-actions = Aksi
system-config-telegram-test-title = Uji Koneksi
system-config-telegram-test-description = Kirim pesan uji untuk memastikan konfigurasi { -telegram } Anda berfungsi
system-config-telegram-send-test = Kirim Pesan Uji
system-config-telegram-sending = Mengirim...
system-config-telegram-configure-token-title = Konfigurasikan token bot terlebih dahulu
system-config-telegram-configure-token-status = Konfigurasikan token bot di atas untuk mengaktifkan pengujian
system-config-telegram-test-sent-status = Pesan uji berhasil dikirim! Periksa { -telegram } Anda.
system-config-telegram-test-sent = Pesan uji { -telegram } terkirim
system-config-telegram-test-failed = Gagal mengirim pesan uji
system-config-telegram-auth-title = Autentikasi Bot
system-config-telegram-totp-title = Autentikasi Dua Faktor (TOTP)
system-config-telegram-totp-configured = Terkonfigurasi
system-config-telegram-totp-not-configured = Belum Dikonfigurasi
system-config-telegram-totp-active = Autentikasi dua faktor aktif. Sesi { -telegram } yang kedaluwarsa memerlukan kode TOTP dari aplikasi autentikator Anda.
system-config-telegram-totp-inactive = Aktifkan autentikasi dua faktor di pengaturan Keamanan untuk melindungi perintah { -telegram }.
system-config-telegram-totp-note = TOTP dibagikan dengan layar kunci dasbor. Konfigurasikan di pengaturan Keamanan.
system-config-telegram-require-2fa = Wajibkan 2FA untuk perintah
system-config-telegram-save-rejected = Penyimpanan ditolak ({ $status })
system-config-telegram-save-failed = Tidak dapat menyimpan pengaturan { -telegram }

system-config-saved = Konfigurasi disimpan
system-config-save-failed = Tidak dapat menyimpan konfigurasi
system-config-reloaded = Konfigurasi dimuat ulang dari disk
system-config-reload-failed = Tidak dapat memuat ulang konfigurasi
system-config-diff-title = Selisih konfigurasi
system-config-diff-console = Ditulis ke konsol browser
system-config-diff-failed = Tidak dapat menghitung selisih
system-config-reset-title = Reset Konfigurasi
system-config-reset-message =
    Ini akan mereset seluruh konfigurasi ke nilai default bawaan. Semua pengaturan saat ini akan hilang.

    Tindakan ini tidak dapat dibatalkan.
system-config-reset-done-title = Konfigurasi direset
system-config-reset-done-message = Semua pengaturan dikembalikan ke nilai default
system-config-reset-failed = Tidak dapat mereset konfigurasi
system-config-load-failed = Tidak dapat memuat konfigurasi
system-config-metadata-failed = Tidak dapat memuat metadata konfigurasi

system-config-dialog-close =
    .aria-label = Tutup
system-config-select-none = Batalkan Pilihan
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
       *[other] { $count } perubahan
    }
system-config-sections-count =
    { $count ->
       *[other] { $count } bagian
    }

system-config-section-hint-chains = Aktivasi blockchain, endpoint RPC, dan perutean swap
system-config-section-hint-trader = Aturan trading dan otomatisasi
system-config-section-hint-positions = Pengaturan pengelolaan posisi
system-config-section-hint-filtering = Aturan dan ambang pemfilteran token
system-config-section-hint-tokens = Penemuan token dan sumber data
system-config-section-hint-events = Pengaturan perekaman event
system-config-section-hint-services = Pengaturan layanan latar belakang
system-config-section-hint-monitoring = Konfigurasi pemantauan sistem
system-config-section-hint-ohlcv = Pengaturan data candlestick
system-config-section-hint-gui = Pengaturan dasbor dan UI
system-config-section-hint-telegram = Konfigurasi bot { -telegram }

system-config-export-dialog-title = Ekspor Konfigurasi
system-config-export-intro = Pilih bagian konfigurasi yang akan diekspor. File hasil ekspor dapat diimpor nanti untuk memulihkan atau membagikan pengaturan.
system-config-export-sections = Bagian
system-config-export-timestamp = Sertakan stempel waktu ekspor
system-config-sections-selected =
    { $count ->
       *[other] { $count } bagian dipilih
    }
system-config-exporting = Mengekspor...
system-config-export-invalid-response = Respons tidak valid dari server
system-config-exported-title = Konfigurasi Diekspor
system-config-exported-message =
    { $count ->
       *[other] { $count } bagian diekspor
    }
system-config-export-failed-title = Ekspor Gagal
system-config-export-failed = Gagal mengekspor konfigurasi

system-config-import-dialog-title = Impor Konfigurasi
system-config-import-upload-intro = Unggah file konfigurasi yang sebelumnya diekspor. Anda dapat melihat pratinjau dan memilih bagian yang akan diimpor.
system-config-import-dropzone-title = Letakkan file konfigurasi di sini
system-config-import-dropzone-hint = atau klik untuk menelusuri
system-config-import-analyzing = Menganalisis konfigurasi...
system-config-import-preview = Pratinjau
system-config-import-preview-intro = Tinjau bagian konfigurasi di bawah. Pilih bagian yang akan diimpor.
system-config-import-sections = Bagian dalam File
system-config-import-select-valid = Pilih Semua yang Valid
system-config-import-merge-label = Gabungkan dengan yang ada
system-config-import-merge-hint = Hanya perbarui kolom yang ada di file. Tidak dicentang = ganti seluruh bagian.
system-config-import-save-label = Simpan ke disk
system-config-import-save-hint = Simpan perubahan ke config.toml setelah impor
system-config-import-selected = Impor yang Dipilih
system-config-import-warnings =
    { $count ->
       *[other] { $count } Peringatan
    }
system-config-import-warning-unknown-section = Bagian tidak dikenal "{ $section }" akan diabaikan
system-config-import-warning-sensitive-field = Mengimpor { $field } dapat menimpa pengaturan autentikasi
system-config-import-section-error = { $detail }
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = Tidak ada di file
system-config-import-status-invalid = Konfigurasi tidak valid
system-config-import-status-unchanged = Tidak ada perubahan
system-config-import-not-included = Tidak disertakan dalam file
system-config-import-show-changes = Tampilkan perubahan
system-config-import-hide-changes = Sembunyikan perubahan
system-config-import-value-current = Nilai saat ini
system-config-import-value-new = Nilai baru
system-config-import-more-changes =
    { $count ->
       *[other] +{ $count } perubahan lagi
    }
system-config-import-value-items =
    { "[" }{ $count ->
       *[other] { $count } item
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
       *[other] { $count } key
    }{ "}" }
system-config-importing = Mengimpor...
system-config-import-failed = Impor gagal
system-config-import-invalid-file-title = File Tidak Valid
system-config-import-invalid-file = Gagal mengurai file konfigurasi
system-config-imported-title = Konfigurasi Diimpor
system-config-imported-message =
    { $count ->
       *[other] { $count } bagian diimpor
    }
system-config-import-failed-title = Impor Gagal
system-config-import-failed-message = Gagal mengimpor konfigurasi
