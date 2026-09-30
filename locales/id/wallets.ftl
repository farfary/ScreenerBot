# Wallet page labels.

wallets-type-generated = Dibuat
wallets-type-imported = Diimpor
wallets-type-migrated = Dimigrasikan

wallets-watch-disabled-user = Dijeda oleh Anda
wallets-watch-disabled-signature-budget = Dijeda: mencapai batas pemeriksaan { $limit } signature sebelum berhasil menyusul
wallets-watch-disabled-unknown = Dijeda: alasan keamanan pemantauan yang tersimpan tidak dapat dibaca
wallets-watch-disabled-helius-unavailable = Dijeda: penyedia aktivitas tinggi tidak tersedia; kursor dipertahankan
wallets-watch-disabled-processing-failed = Dijeda: aktivitas dompet tidak dapat diproses; kursor dipertahankan

wallets-watch-error-provider-unavailable = Penyedia aktivitas tinggi tidak tersedia; pemantauan dijeda
wallets-watch-error-provider-repeated-failure = Pemeriksaan { -helius } gagal berulang kali; pemantauan dijeda
wallets-watch-error-processing-repeated-failure = Pemrosesan aktivitas dompet gagal berulang kali; pemantauan dijeda
wallets-watch-error-position-unreadable = Pemantauan dompet tidak dapat membaca posisi tersimpannya; mencoba lagi
wallets-watch-error-provider-check-failed = Pemeriksaan penyedia aktivitas tinggi gagal; mencoba lagi
wallets-watch-error-decode-failed = Transaksi aktivitas tinggi tidak dapat didekode; kursor dipertahankan
wallets-watch-error-processing-failed = Aktivitas dompet tidak dapat diproses; mencoba lagi
wallets-watch-error-position-save-failed = Pemantauan dompet tidak dapat menyimpan posisinya; mencoba lagi

wallets-watch-reason-user = Dijeda oleh Anda.
wallets-watch-reason-signature-budget = Dompet ini memiliki aktivitas lebih banyak daripada yang dapat diperiksa pemantauan saat ini.
wallets-watch-reason-helius-unavailable = Pemeriksaan { -helius } gagal. Progres tersimpan dipertahankan.
wallets-watch-reason-processing-failed = Aktivitas dompet tidak dapat diproses. Progres tersimpan dipertahankan.

wallets-field-address = Alamat
wallets-field-name = Nama Dompet
wallets-field-notes = Catatan
wallets-field-private-key = Private Key
wallets-address-copy = Salin alamat
wallets-modal-close =
    .aria-label = Tutup modal
wallets-this-wallet = dompet ini
wallets-summary-sol = { -sol }
wallets-copied-address = Alamat
wallets-copied-mint = Alamat mint
wallets-copied-private-key = Private key

wallets-tab-main = Dompet Utama
wallets-tab-secondaries = Sekunder
wallets-tab-archive = Arsip
wallets-tab-watched = Dipantau
wallets-refresh-failed = Tidak dapat menyegarkan dompet
wallets-action-failed = Gagal
wallets-toast-failed = Gagal: { $reason }
wallets-create-busy = Membuat...
wallets-create-fallback = Pembuatan gagal
wallets-create-done = Dompet "{ $name }" berhasil dibuat!
wallets-import-busy = Mengimpor...
wallets-import-failed = Impor gagal
wallets-import-done = Dompet "{ $name }" berhasil diimpor!
wallets-archive-busy = Mengarsipkan...
wallets-archive-confirm-text = Yakin ingin mengarsipkan <strong>{ $name }</strong>?
wallets-archive-done = Dompet diarsipkan
wallets-restore-done = Dompet dipulihkan
wallets-export-busy = Mendekripsi...
wallets-export-revealed = Key ditampilkan - tangani dengan hati-hati
wallets-delete-busy = Menghapus...
wallets-delete-confirm-text = Yakin ingin menghapus <strong>{ $name }</strong>?
wallets-delete-done = Dompet dihapus permanen

wallets-add-title = Tambah Dompet
wallets-add-tab-create = Buat Baru
wallets-add-tab-import = Impor yang Ada
wallets-create-name-input =
    .placeholder = mis., Dompet Trading
wallets-create-name-hint = Nama yang mudah diingat untuk mengenali dompet ini
wallets-create-notes-input =
    .placeholder = Deskripsi atau tujuan (opsional)...
wallets-create-submit = Buat Dompet
wallets-import-warning-title = Peringatan Keamanan
wallets-import-warning-body = Impor private key hanya dari sumber tepercaya. Key Anda akan dienkripsi dan disimpan dengan aman di perangkat ini.
wallets-import-name-input =
    .placeholder = mis., Dompet Saya
wallets-import-key-input =
    .placeholder = String Base58 atau array JSON [1,2,3,...]
wallets-import-key-toggle =
    .aria-label = Tampilkan/sembunyikan private key
wallets-import-key-hint = Mendukung key berenkode base58 atau format array byte
wallets-import-notes-input =
    .placeholder = Deskripsi (opsional)...
wallets-import-submit = Impor Dompet

wallets-watch-add-title = Pantau Dompet
wallets-watch-add-address = Alamat Dompet
wallets-watch-add-address-input =
    .placeholder = Alamat Solana
wallets-watch-add-address-hint = Mencatat aktivitas on-chain dompet dan mengirim peringatan trade melalui pengaturan { -telegram } Anda.
wallets-watch-add-label = Label
wallets-watch-add-label-input =
    .placeholder = Nama (opsional)
wallets-watch-add-submit = Tambah Pantauan

wallets-watch-budget-title-options = Opsi pemantauan dompet
wallets-watch-budget-title-restore = Pulihkan pemantauan dompet
wallets-watch-budget-close =
    .aria-label = Tutup
wallets-watch-budget-label-signatures = Signature diperiksa per pemeriksaan
wallets-watch-budget-label-transactions = Transaksi penuh yang berhasil diperiksa per pemeriksaan
wallets-watch-budget-hint-signatures = Batas saat ini: { $limit }. Pilih 500–5.000 signature per pemeriksaan dengan kelipatan 100.
wallets-watch-budget-hint-transactions = Batas saat ini: { $limit }. Pilih 500–5.000 transaksi berhasil per pemeriksaan dengan kelipatan 100.
wallets-watch-budget-error-range = Pilih antara 500 dan 5.000 catatan per pemeriksaan dengan kelipatan 100 catatan.
wallets-watch-budget-error-ack = Konfirmasikan bahwa signature sejak pemeriksaan terakhir yang selesai akan dilewati.
wallets-watch-budget-save-failed = Batas pemantauan tidak dapat disimpan.
wallets-watch-budget-save = Simpan batas
wallets-watch-budget-resume = Lanjutkan dari sekarang
wallets-watch-budget-resume-notice = Dompet ini mencapai batas pemeriksaan sebelum berhasil menyusul. Lanjutkan dari sekarang dimulai dari aktivitas dompet terbaru; aktivitas sejak pemeriksaan terakhir yang selesai tidak akan disalin.
wallets-watch-budget-resume-tasks = Tugas salin tetap dijeda sampai Anda melanjutkan setiap tugas di Copy Trading.
wallets-watch-budget-resume-ack = Saya mengerti aktivitas yang terlewat tidak akan disalin.
wallets-watch-budget-resumed = Pemantauan dilanjutkan dari posisi terbaru dompet
wallets-watch-budget-updated = Batas pemantauan dompet diperbarui
wallets-watch-helius-allow = Izinkan penyusulan { -helius } jika diperlukan
wallets-watch-helius-try = Coba menyusul menggunakan { -helius }
wallets-watch-helius-stop = Hentikan penyusulan { -helius } untuk dompet ini
wallets-watch-helius-description-approved = Penyusulan { -helius } diizinkan untuk dompet ini. Menonaktifkannya akan kembali ke pemeriksaan standar, yang bisa tertinggal pada dompet yang sibuk.
wallets-watch-helius-description-available = { -helius } dapat memeriksa transaksi Solana yang berhasil dari posisi tersimpan tanpa melewati interval yang belum diperiksa. Ini dapat memakai lebih banyak kredit penyedia dan tetap bisa tertinggal.
wallets-watch-helius-description-unavailable = Penyusulan { -helius } tidak tersedia. Konfigurasikan endpoint RPC { -helius } yang aktif untuk menggunakannya.
wallets-watch-helius-description-unsupported = Tidak ada penyedia penyusulan yang didukung untuk pemantauan ini. Lanjutkan dari sekarang tersedia jika pemantauan mencapai batasnya.
wallets-watch-helius-allow-title = Izinkan penyusulan { -helius } untuk dompet ini
wallets-watch-helius-allow-message = { -helius } dapat memeriksa transaksi Solana yang berhasil dari posisi tersimpan tanpa melewati interval yang belum diperiksa. Saat ini biayanya 10 kredit per 100 transaksi penuh yang dikembalikan, dibulatkan ke atas, dengan minimum 10 kredit per permintaan. Satu pemeriksaan dapat membuat beberapa permintaan; penggunaan dan harga penyedia dapat berbeda. Tugas salin tetap dijeda sampai dilanjutkan secara terpisah.
wallets-watch-helius-allow-confirm = Izinkan untuk dompet ini
wallets-watch-helius-stop-message = Dompet ini akan kembali ke pemeriksaan standar. Dompet yang sibuk dapat mencapai batas pemantauannya dan dijeda lagi. Dompet lain dan konfigurasi RPC { -helius } Anda tidak berubah.
wallets-watch-helius-stop-confirm = Hentikan untuk dompet ini
wallets-watch-helius-stop-keep = Tetap izinkan
wallets-watch-helius-restored = Pemantauan dipulihkan dari progres tersimpan; tugas salin tetap dijeda
wallets-watch-helius-allowed = Penyusulan { -helius } diizinkan untuk dompet ini bila diperlukan
wallets-watch-helius-stopped = Penyusulan { -helius } dihentikan untuk dompet ini
wallets-watch-helius-update-failed = Pengaturan penyusulan dompet tidak dapat diperbarui

wallets-export-title = Ekspor Private Key
wallets-export-warning-title = Peringatan Keamanan Kritis
wallets-export-warning-body = Jangan pernah membagikan private key Anda kepada siapa pun. Siapa pun yang memiliki akses ke key ini dapat mencuri semua dana dari dompet ini.
wallets-export-key-label = Private Key (Base58)
wallets-export-copy =
    .title = Salin ke clipboard
    .aria-label = Salin ke clipboard
wallets-export-reveal = Tampilkan Key

wallets-archive-title = Arsipkan Dompet
wallets-archive-note = Dompet yang diarsipkan tidak digunakan dalam operasi apa pun tetapi dapat dipulihkan kapan saja.
wallets-archive-confirm = Ya, Arsipkan
wallets-delete-title = Hapus Dompet
wallets-delete-warning-title = Tindakan ini tidak dapat dibatalkan!
wallets-delete-warning-body = Menghapus dompet ini akan menghapusnya secara permanen beserta private key terenkripsinya dari perangkat ini.
wallets-delete-confirm = Ya, Hapus

wallets-bulk-import-title = Impor Dompet
wallets-bulk-import-submit = Impor Dompet
wallets-bulk-step-upload = Unggah File
wallets-bulk-step-map = Petakan Kolom
wallets-bulk-step-results = Hasil
wallets-bulk-import-file-warning-body = Impor file hanya dari sumber tepercaya. Private key akan dienkripsi dan disimpan dengan aman di perangkat ini.
wallets-bulk-drop-title = Letakkan file Anda di sini
wallets-bulk-drop-subtitle = atau klik untuk menelusuri
wallets-bulk-drop-formats = Mendukung CSV dan Excel (.xlsx, .xls)
wallets-bulk-file-remove =
    .aria-label = Hapus file
wallets-bulk-map-subtitle = Cocokkan kolom file Anda dengan kolom dompet
wallets-bulk-preview-title = Pratinjau (5 Baris Pertama)
wallets-bulk-summary-valid = <strong>{ $count }</strong> valid
wallets-bulk-summary-invalid = <strong>{ $count }</strong> tidak valid
wallets-bulk-summary-duplicate =
    { $count ->
       *[other] <strong>{ $count }</strong> duplikat
    }
wallets-bulk-done = Selesai
wallets-bulk-file-invalid = Jenis file tidak valid. Gunakan file CSV atau Excel.
wallets-bulk-preview-busy = Memproses...
wallets-bulk-preview-fallback = Gagal memproses file
wallets-bulk-preview-failed = Gagal memproses file: { $reason }
wallets-bulk-column-select = -- Pilih kolom --
wallets-bulk-preview-empty = Tidak ada baris data di file
wallets-bulk-preview-status = Status
wallets-bulk-status-valid = Valid
wallets-bulk-status-duplicate = Duplikat
wallets-bulk-status-invalid = Tidak valid
wallets-bulk-import-busy = Mengimpor...
wallets-bulk-import-toast =
    { $count ->
       *[other] { $count } dompet diimpor
    }
wallets-bulk-import-error = Impor gagal: { $reason }
wallets-bulk-result-success-title = Impor Berhasil
wallets-bulk-result-success-detail =
    { $count ->
       *[other] Semua { $count } dompet berhasil diimpor
    }
wallets-bulk-result-partial-title = Sebagian Berhasil
wallets-bulk-result-partial-detail = { $imported } diimpor, { $failed } gagal
wallets-bulk-result-failed-title = Impor Gagal
wallets-bulk-result-failed-detail =
    { $count ->
       *[other] Semua { $count } dompet gagal diimpor
    }
wallets-bulk-result-imported = Diimpor
wallets-bulk-result-failed = Gagal

wallets-bulk-export-title = Ekspor Dompet
wallets-bulk-export-format = Format
wallets-bulk-export-format-csv = CSV (.csv)
wallets-bulk-export-format-xlsx = Excel (.xlsx)
wallets-bulk-export-include-archived = Sertakan dompet yang diarsipkan
wallets-bulk-export-safe-title = Ekspor Aman
wallets-bulk-export-safe-body = Ekspor hanya alamat dompet dan metadata. Tanpa private key.
wallets-bulk-export-safe-submit = Ekspor Alamat
wallets-bulk-export-or = atau
wallets-bulk-export-danger-title = Ekspor Berbahaya
wallets-bulk-export-danger-body = Sertakan private key dalam ekspor. Siapa pun yang memiliki file ini dapat mencuri dana Anda.
wallets-bulk-export-danger-submit = Ekspor dengan Private Key
wallets-bulk-export-busy = Mengekspor...
wallets-bulk-export-done = Dompet diekspor ke { $filename }
wallets-bulk-export-fallback = Ekspor gagal
wallets-bulk-export-error = Ekspor gagal: { $reason }
wallets-bulk-confirm-title = Konfirmasi Ekspor Berbahaya
wallets-bulk-confirm-warning =
    { $count ->
       *[other] Anda akan mengekspor <strong>{ $count }</strong> private key. Ini sangat berbahaya!
    }
wallets-bulk-confirm-risk-steal = Siapa pun yang memiliki file ini dapat mencuri semua dana
wallets-bulk-confirm-risk-share = Jangan pernah membagikan file ini kepada siapa pun
wallets-bulk-confirm-risk-delete = Hapus file segera setelah digunakan
wallets-bulk-confirm-prompt = Ketik frasa di bawah untuk mengonfirmasi
wallets-bulk-confirm-submit = Ekspor Key

wallets-holdings-col-token = Token
wallets-holdings-col-balance = Saldo
wallets-holdings-col-value = Nilai ({ -sol })
wallets-holdings-col-type = Jenis
wallets-holdings-col-decimals = Desimal
wallets-holdings-col-mint = Mint
wallets-holdings-empty-title = Tidak ada kepemilikan token
wallets-holdings-empty-message = Token yang dimiliki dompet ini akan muncul di sini.
wallets-holdings-no-main = Tidak ada dompet utama
wallets-holdings-main-tag = Utama
wallets-holdings-main-title = Dompet Utama
wallets-holdings-tokens = Token
wallets-holdings-last-used = Terakhir digunakan
wallets-holdings-never = Belum pernah
wallets-holdings-search =
    .placeholder = Cari berdasarkan simbol atau mint...
wallets-holdings-export = Ekspor Key
wallets-holdings-export-tooltip = Ekspor private key dompet ini
wallets-list-col-name = Nama
wallets-list-col-balance = Saldo ({ -sol })
wallets-list-col-type = Jenis
wallets-list-col-created = Dibuat
wallets-list-col-actions = Aksi
wallets-list-action-export = Ekspor private key
wallets-list-action-archive = Arsipkan dompet
wallets-list-action-restore = Pulihkan dompet
wallets-list-action-delete = Hapus permanen
wallets-list-count = Dompet
wallets-list-search =
    .placeholder = Cari berdasarkan nama atau alamat...
wallets-list-loading-title = Memuat dompet…
wallets-list-loading-description = Menyiapkan tampilan dompet yang dipilih.
wallets-secondaries-empty-title = Tidak ada dompet sekunder
wallets-secondaries-empty-message = Buat dompet tambahan untuk mengatur aktivitas trading Anda di beberapa akun.
wallets-secondaries-add = Tambah Dompet
wallets-archive-empty-title = Tidak ada dompet yang diarsipkan
wallets-archive-empty-message = Dompet yang Anda arsipkan akan tersimpan dengan aman di sini untuk referensi nanti.

wallets-watched-col-wallet = Dompet
wallets-watched-col-status = Status
wallets-watched-col-progress = Progres tersimpan
wallets-watched-col-last-check = Pemeriksaan terakhir
wallets-watched-unlabelled = Dompet tanpa label
wallets-watched-generic-name = dompet
wallets-watched-not-synced = Belum disinkronkan
wallets-watched-not-checked = Belum diperiksa
wallets-watched-action-copy = Copy trade
    .title = Buka dompet ini di Copy Trading
wallets-watched-action-restore = Pulihkan pantauan
wallets-watched-action-options = Opsi pantauan
wallets-watched-action-retry = Coba lagi pantauan
wallets-watched-action-pause = Jeda
wallets-watched-action-enable = Aktifkan
wallets-watched-action-remove =
    .title = Hapus
    .aria-label = Hapus { $name }
wallets-watch-state-paused = Dijeda
wallets-watch-state-catching-up = Menyusul
wallets-watch-state-watching = Memantau
wallets-watch-state-streaming = Streaming
wallets-watch-state-polling = Polling
wallets-watched-detail-helius = Diperiksa melalui { -helius } untuk dompet ini.
wallets-watched-empty-title = Tidak ada alamat yang dipantau
wallets-watched-empty-message = Gunakan Pantau Dompet untuk mencatat aktivitas on-chain dompet publik.
wallets-watched-count = Dipantau
wallets-watched-search =
    .placeholder = Cari dompet yang dipantau...
wallets-watched-add = Pantau Dompet
wallets-watched-refresh = Segarkan dompet yang dipantau
wallets-watched-loading-title = Memuat dompet yang dipantau...
wallets-watched-loading-description = Mengambil target pengamatan.
wallets-watched-load-error-title = Alamat yang dipantau tidak dapat dimuat
wallets-watched-load-error-description = Gunakan segarkan untuk mencoba lagi.
wallets-watched-address-invalid = Masukkan alamat dompet Solana yang valid.
wallets-watched-added = Pantauan dompet ditambahkan
wallets-watched-duplicate = Dompet itu sudah dipantau.
wallets-watched-add-failed = Pantauan dompet tidak dapat ditambahkan.
wallets-watched-retried = Pantauan dompet dipulihkan dengan kursor tersimpannya
wallets-watched-paused = Pantauan dompet dijeda
wallets-watched-enabled = Pantauan dompet diaktifkan
wallets-watched-removed = Pantauan dompet dihapus
wallets-watched-update-failed = Pantauan dompet tidak dapat diperbarui
