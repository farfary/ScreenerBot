# Update system text. Ids come from DeferReason in src/version/types.rs, the
# update check state and the /api/updates responses.

updates-defer-automatic-install-disabled = Pemasangan otomatis dinonaktifkan. Pembaruan sudah siap dan akan diterapkan saat Anda memilihnya.
updates-defer-trading-active = Ada posisi, trade, atau operasi alat yang aktif, sehingga mulai ulang ditunda. Pembaruan diterapkan otomatis saat aplikasi tidak sibuk.
updates-defer-needs-installer = Rilis ini juga memperbarui shell desktop, sehingga installer perlu dijalankan sekali.

updates-check-failed = { $cause }

updates-download-started = Mengunduh pembaruan v{ $version }...
updates-apply-started = Memasang pembaruan. { -brand } dimulai ulang dan terhubung kembali secara otomatis.
updates-install-opened = Installer pembaruan terverifikasi dibuka. Selesaikan installer sistem operasi.

updates-installer-toast-title = Installer dibuka
updates-installer-toast-message = { -brand } akan keluar dengan bersih sekarang.

updates-tab-status = Status
updates-tab-release-notes = Catatan Rilis
updates-tab-preferences = Preferensi
updates-tab-sections = Bagian pembaruan
updates-checking-installation = Memeriksa instalasi ini...

updates-phase-idle-headline = Siap memeriksa pembaruan
updates-phase-idle-detail = { -brand } v{ $version } terpasang.
updates-phase-up-to-date-headline = Anda sudah menggunakan versi terbaru
updates-phase-up-to-date-detail = { -brand } v{ $version } adalah versi terbaru.
updates-phase-checking-headline = Memeriksa pembaruan
updates-phase-checking-detail = Mencari rilis terbaru yang dipublikasikan.
updates-phase-available-headline = Versi { $version } tersedia
updates-phase-downloading-headline = Mengunduh v{ $version }
updates-phase-verifying-headline = Memverifikasi v{ $version }
updates-phase-verifying-detail = Memeriksa unduhan terhadap checksum yang dipublikasikan.
updates-phase-ready-to-apply-headline = Versi { $version } siap
updates-phase-ready-to-apply-detail = Pembaruan dapat dipasang sekarang dengan mulai ulang singkat, atau otomatis pada peluncuran berikutnya.
updates-phase-ready-to-install-headline = Versi { $version } siap
updates-phase-ready-to-install-detail = Installer desktop siap menyelesaikan pembaruan ini.
updates-phase-applying-headline = Memasang pembaruan
updates-phase-applying-detail = { -brand } dimulai ulang ke versi baru.
updates-phase-applied-headline = Diperbarui ke v{ $version }
updates-phase-applied-detail = Pembaruan terpasang. Tidak ada yang perlu dilakukan lagi.
updates-phase-failed-headline = Pembaruan tidak selesai
updates-phase-failed-detail = Coba perbarui lagi.
updates-phase-check-failed-headline = Tidak dapat memeriksa pembaruan
updates-phase-check-failed-detail = Layanan rilis tidak dapat dijangkau.
updates-status-unavailable-headline = Status pembaruan tidak tersedia
updates-phase-unrecognized-detail = Status pembaruan yang dilaporkan tidak dikenali.
updates-status-load-failed-detail = Status instalasi tidak dapat dimuat.

updates-kind-core = Pembaruan core · { $size } · mulai ulang singkat
updates-kind-full = Pembaruan desktop · { $size } · perlu installer
updates-size-unknown = ukuran tidak diketahui

updates-action-check-now = Periksa sekarang
updates-action-check-again = Periksa lagi
updates-action-try-again = Coba lagi
updates-action-download = Unduh pembaruan
updates-action-restart = Mulai ulang untuk memperbarui
updates-action-open-installer = Buka installer

updates-busy-checking = Memeriksa...
updates-busy-resuming = Melanjutkan unduhan...
updates-busy-starting-download = Memulai unduhan...
updates-busy-restarting = Memulai ulang...
updates-busy-opening-installer = Membuka installer...

updates-progress-downloading = Mengunduh pembaruan
updates-progress-verifying = Memverifikasi pembaruan
updates-progress-transferred = { $done } dari { $total }
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }, { $percent }, { $transferred }

updates-detail-list-label = Detail instalasi
updates-detail-installed-version = Versi terpasang
updates-detail-system = Sistem
updates-detail-last-checked = Terakhir diperiksa
updates-detail-never = Belum pernah
updates-detail-available-version = Versi tersedia
updates-detail-download-size = Ukuran unduhan

updates-version-installed = Terpasang
updates-version-available = Tersedia

updates-notes-highlights = Sorotan
updates-notes-empty-title = Belum ada catatan rilis
updates-notes-empty-error = Riwayat rilis tidak dapat dimuat. Periksa koneksi dan coba lagi.
updates-notes-empty-none = Catatan rilis akan muncul di sini setelah rilis dipublikasikan.
updates-notes-history-notice = Menampilkan apa yang sudah diketahui instalasi ini — riwayat rilis tidak dapat dimuat.
updates-release-empty = Tidak ada perubahan yang tercantum untuk rilis ini.
updates-release-changes =
    { $count ->
       *[other] { $count } perubahan
    }

updates-preferences-unavailable-title = Preferensi pembaruan tidak tersedia
updates-preferences-unavailable-detail = Konfigurasi pembaruan tidak dapat dimuat.
updates-preference-fallback-name = preferensi pembaruan
updates-preference-save-failed = Tidak dapat menyimpan { $preference }

updates-request-failed = Permintaan gagal
updates-check-request-failed = Tidak dapat memeriksa pembaruan
updates-resume-failed = Tidak dapat melanjutkan unduhan pembaruan
updates-download-failed = Tidak dapat memulai unduhan pembaruan
updates-apply-failed = Tidak dapat memasang pembaruan
updates-install-failed = Tidak dapat membuka installer pembaruan
updates-apply-confirm-title = Pasang v{ $version }
updates-apply-confirm-message = { -brand } dimulai ulang ke versi baru. Trading berhenti beberapa detik dan berlanjut otomatis; posisi terbuka tidak tersentuh.
updates-install-confirm-title = Jalankan installer
updates-install-confirm-message = Installer terverifikasi dibuka dan { -brand } keluar dengan bersih. Selesaikan installer, lalu buka kembali { -brand }.

updates-version-number = v{ $version }
