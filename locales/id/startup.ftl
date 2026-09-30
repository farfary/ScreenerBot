# Fatal startup errors. Server-only: rendered by src/errors/startup.rs into the
# finished text that the Electron shell displays, never sent to the dashboard.
#
# Remedies are written for the compiled binary. Paths, ports, wallet addresses
# and error details arrive as arguments. Keep the command-line flag, the file
# names and the support handle unchanged.

## Wallet mismatch.

startup-wallet-mismatch-title = Dompet berubah
startup-wallet-mismatch-detail =
    Dompet di konfigurasi Anda tidak cocok dengan dompet yang tercatat di riwayat lokal komputer ini.

    Dompet saat ini: { $current }
    Dompet sebelumnya: { $stored }

    Data lokal yang terdampak: { $systems }

    Ini biasanya terjadi setelah mengimpor private key yang berbeda atau memulihkan konfigurasi yang berbeda. Trading, posisi, dan riwayat milik dompet sebelumnya dan harus dihapus sebelum dompet baru dapat dimulai dengan aman.
startup-wallet-mismatch-systems-default = Transaksi, Posisi, Riwayat Dompet
startup-wallet-mismatch-remedy =
    Hapus riwayat lokal dompet sebelumnya untuk melanjutkan (database Anda dicadangkan otomatis terlebih dahulu):

      - Di aplikasi: pilih "{ $action }" di bawah.
      - Dari terminal: jalankan  screenerbot --clean-wallet-data

    Dana on-chain tidak terpengaruh; hanya riwayat trade/posisi lokal di komputer ini yang direset. Cadangan ditulis di:
      { $path }
startup-recovery-reset-wallet = Reset data dompet & restart

## Port in use.

startup-port-in-use-title = Port jaringan sedang digunakan
startup-port-in-use-detail = Port dasbor { $address } sudah digunakan.
startup-port-in-use-remedy = Program lain menggunakan port yang dibutuhkan { -brand }. Tutup program tersebut, atau ubah port webserver di Pengaturan, lalu mulai { -brand } lagi.

## Another instance is running.

startup-lock-held-title = { -brand } sudah berjalan
startup-lock-held-detail = Salinan { -brand } lain sudah berjalan di komputer ini, sehingga salinan kedua tidak dapat dimulai.
startup-lock-held-remedy = Beralih ke jendela yang sudah terbuka. Jika tidak ada, keluarkan proses { -brand } di latar belakang lalu coba lagi. Jika masalah berlanjut setelah reboot, file kunci mungkin sudah usang dan dapat dihapus dari folder data (.screenerbot.lock).

## Configuration.

startup-config-invalid-title = Konfigurasi tidak dapat dibaca
startup-config-parse-detail = config.toml tidak dapat diurai: { $detail }
startup-config-load-parse-detail = Gagal memuat konfigurasi: config.toml tidak dapat diurai: { $detail }
startup-config-parse-remedy = File konfigurasi Anda tidak dapat dibaca. Pulihkan cadangan dari folder data, atau reset konfigurasi ke default lalu atur dompet dan RPC Anda lagi.
startup-config-load-parse-remedy = Pulihkan konfigurasi yang valid atau selesaikan pengaturan awal lagi.
startup-option-invalid-title = Opsi startup tidak valid
startup-option-invalid-remedy = Opsi command-line tidak valid. Mulai { -brand } tanpa opsi tersebut, atau perbaiki lalu coba lagi.

## Generic failures.

startup-generic-title = { -brand } tidak dapat dimulai
startup-generic-remedy = Periksa file log untuk detailnya, lalu restart aplikasi. Jika masalah berlanjut, hubungi dukungan di t.me/screenerbotio_support.
startup-generic-detail = { $error }
startup-failure-directories = Gagal membuat direktori yang diperlukan: { $error }
startup-failure-config-load = Gagal memuat konfigurasi: { $error }
startup-failure-actions-init = Gagal menginisialisasi database aksi: { $error }
startup-failure-actions-sync = Gagal menyinkronkan aksi dari database: { $error }
startup-failure-strategy-init = Gagal menginisialisasi sistem strategi: { $error }
startup-failure-analysis-init = Gagal menginisialisasi mesin analisis: { $error }
startup-failure-assistant-init = Gagal menginisialisasi mesin chat Asisten: { $error }
startup-failure-wallets-init = Gagal menginisialisasi dompet: { $error }
startup-failure-wallet-validation = Gagal memvalidasi konsistensi dompet: { $error }
