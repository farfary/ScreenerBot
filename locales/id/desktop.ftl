# Text owned by the Electron shell: tray, application menu, native dialogs, the
# splash and the boot-error screen. Server-only: read from the packaged catalogs
# by electron/src/l10n.js, never sent to the dashboard. Fatal startup errors
# arrive already rendered from startup.ftl; only their chrome lives here.
#
# Menu roles (Edit, Window, Quit and the like) are not listed: the operating
# system localizes them.

desktop-action-ok = OK

desktop-splash-starting = Memulai { -brand }
desktop-splash-restarting = Memulai ulang { -brand }
desktop-splash-recovering = Memulihkan
desktop-splash-opening-dashboard = Membuka dasbor
desktop-splash-checking-dependencies = Memeriksa dependensi
desktop-splash-installing-dependencies = Memasang dependensi sistem
desktop-splash-installing-dependencies-detail = { -brand } memerlukan Microsoft Visual C++ Redistributable untuk berjalan.
desktop-splash-resetting-wallet = Mereset data dompet
desktop-splash-resetting-wallet-detail = Data dompet yang ada dicadangkan sebelum dihapus.
desktop-splash-updating = Memperbarui ke v{ $version }
desktop-splash-updating-detail = Pengaturan dan data Anda tetap seperti semula.
desktop-splash-restoring = Memulihkan v{ $version }
desktop-splash-restoring-detail = Pembaruan v{ $failed } tidak dapat dimulai, sehingga versi sebelumnya mengambil alih.

desktop-boot-title-fallback = { -brand } tidak dapat dimulai
desktop-boot-detail-fallback = Backend berhenti secara tak terduga.
desktop-boot-remedy-label = Cara memperbaiki
desktop-boot-log-file-label = File log:
desktop-boot-action-reset-wallet = Reset data dompet & mulai ulang
desktop-boot-action-working = Memproses...
desktop-boot-action-open-logs = Buka folder log
desktop-boot-action-copy = Salin detail
desktop-boot-action-copied = Disalin
desktop-boot-action-quit = Keluar
desktop-boot-subtitle-wallet-mismatch = Dompet yang berbeda terdeteksi
desktop-boot-subtitle-port-in-use = Port jaringan yang diperlukan sedang dipakai
desktop-boot-subtitle-lock-held = { -brand } sudah berjalan
desktop-boot-subtitle-config-invalid = Masalah konfigurasi
desktop-boot-subtitle-directory-setup = Masalah penyimpanan
desktop-boot-subtitle-generic = Error saat memulai

desktop-boot-error-title = { -brand } tidak dapat dimulai
desktop-boot-error-remedy = Buka folder log untuk melihat apa yang terjadi, lalu mulai ulang aplikasi. Jika masalah berlanjut, hubungi dukungan di t.me/screenerbotio_support.
desktop-boot-error-default = Backend berhenti secara tak terduga sebelum dasbor siap.
desktop-boot-error-restore-failed = Backend hasil pembaruan gagal dan versi sebelumnya tidak dapat dipulihkan ({ $error }).
desktop-boot-error-spawn-failed = Program backend tidak dapat dijalankan ({ $error }).
desktop-boot-error-spawn-missing = Program backend tidak dapat dijalankan. Mungkin hilang atau diblokir oleh perangkat lunak keamanan.
desktop-boot-error-exited-running = Backend berhenti saat dasbor berjalan (kode keluar { $code }).
desktop-boot-error-exited-early = Backend berhenti sebelum dasbor siap (kode keluar { $code }).
desktop-boot-error-dashboard-load = Dasbor gagal dimuat ({ $description }, { $code }).
desktop-boot-error-renderer-gone = Renderer dasbor berhenti ({ $reason }).
desktop-boot-error-unresponsive = Dasbor tidak merespons.
desktop-boot-error-url-failed = URL dasbor tidak dapat dimuat ({ $error }).
desktop-boot-error-relaunch-setup = Tidak dapat menjalankan ulang backend setelah penyiapan.
desktop-boot-error-relaunch-recovery = Tidak dapat menjalankan ulang backend untuk pemulihan.
desktop-boot-error-restart-offline = Backend tidak kembali online setelah dimulai ulang.
desktop-boot-error-recovery-offline = Pemulihan selesai tetapi backend tidak menjadi siap.
desktop-boot-error-start-timeout = Backend tidak selesai dimulai tepat waktu. Ini dapat terjadi pada penjalanan pertama yang lambat atau jika program lain memblokir koneksi.

desktop-tray-tooltip = { -brand } - Bot Trading Solana
desktop-tray-show = Tampilkan { -brand }
desktop-tray-open-dashboard = Buka Dasbor
desktop-tray-quit = Keluar dari { -brand }

desktop-menu-open-data-folder = Buka Folder Data
desktop-menu-open-logs-folder = Buka Folder Log
desktop-menu-documentation = Dokumentasi
desktop-menu-telegram-support = Dukungan { -telegram }
desktop-menu-check-updates = Periksa Pembaruan...

desktop-menu-file = File
desktop-menu-edit = Edit
desktop-menu-view = Tampilan
desktop-menu-window = Jendela
desktop-menu-help = Bantuan
desktop-menu-reset-zoom = Reset Zoom
desktop-menu-zoom-in = Perbesar
desktop-menu-zoom-out = Perkecil
desktop-menu-keyboard-shortcuts = Pintasan Keyboard
desktop-menu-telegram-channel = Kanal { -telegram }
desktop-menu-telegram-community = Komunitas { -telegram }
desktop-menu-follow-x = Ikuti di { -x } ({ -twitter })
desktop-menu-visit-website = Kunjungi Situs Web
desktop-menu-about = Tentang { -brand }

desktop-about-title = Tentang { -brand }
desktop-about-message = { -brand }
desktop-about-detail =
    Versi { $version }

    Bot manajemen dompet dan trading otomatis Solana tingkat lanjut.

    https://screenerbot.io

    © 2024-2026 { -brand }

desktop-shortcuts-title = Pintasan Keyboard
desktop-shortcuts-message = Pintasan Keyboard { -brand }
desktop-shortcuts-body-mac =
    Pintasan Keyboard:

    Kontrol Jendela:
      Cmd+M          Minimalkan
      Cmd+W          Tutup Jendela
      Cmd+Q          Keluar
      Cmd+Ctrl+F     Layar Penuh

    Zoom:
      Cmd++          Perbesar
      Cmd+-          Perkecil
      Cmd+0          Reset Zoom

    Navigasi:
      Cmd+R          Muat Ulang Dasbor
      Cmd+Shift+D    Buka Folder Data

    Lainnya:
      F1             Buka Dokumentasi
      Cmd+Alt+I      DevTools
desktop-shortcuts-body-other =
    Pintasan Keyboard:

    Kontrol Jendela:
      Alt+F4         Keluar
      F11            Layar Penuh

    Zoom:
      Ctrl++         Perbesar
      Ctrl+-         Perkecil
      Ctrl+0         Reset Zoom

    Navigasi:
      Ctrl+R         Muat Ulang Dasbor
      Ctrl+Shift+D   Buka Folder Data

    Lainnya:
      F1             Buka Dokumentasi
      Ctrl+Shift+I   DevTools

desktop-close-title = Tutup { -brand }
desktop-close-message = Apa yang ingin Anda lakukan?
desktop-close-detail = { -brand } dapat terus berjalan di latar belakang. Bot trading akan terus memantau dan bertrading saat diminimalkan ke system tray.
desktop-close-minimize = Minimalkan ke Tray
desktop-close-quit = Keluar Sepenuhnya
desktop-close-cancel = Batal

desktop-vcredist-missing-title = Dependensi Hilang
desktop-vcredist-missing-message = Visual C++ Redistributable tidak ditemukan
desktop-vcredist-missing-detail = { -brand } memerlukan Microsoft Visual C++ Redistributable untuk berjalan. Pasang sekarang?
desktop-vcredist-install = Pasang & Perbaiki
desktop-vcredist-exit = Keluar
desktop-vcredist-not-found-title = Installer Tidak Ditemukan
desktop-vcredist-not-found-message = Tidak dapat menemukan { $name } dengan benar.
desktop-vcredist-done-title = Pemasangan Selesai
desktop-vcredist-done-message = Dependensi berhasil dipasang.
desktop-vcredist-done-detail = { -brand } akan segera dimulai.
desktop-vcredist-failed-title = Pemasangan Gagal
desktop-vcredist-failed-message = Silakan pasang Visual C++ Redistributable secara manual.
