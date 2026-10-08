# Transaction type labels. Ids come from TransactionType::kind() in
# src/transactions/types.rs; the dashboard maps them in ui/transaction_type.js.

transactions-type-buy = Beli
transactions-type-sell = Jual
transactions-type-swap = Swap
transactions-type-sol-transfer = Transfer SOL
transactions-type-token-transfer = Transfer token
transactions-type-transfer = Transfer
transactions-type-dust = Dust
transactions-type-spam = Spam
transactions-type-ata-create = Akun dibuka
transactions-type-ata-close = Rent diklaim kembali
transactions-type-ata = Akun token
transactions-type-liquidity-add = Tambah likuiditas
transactions-type-liquidity-remove = Tarik likuiditas
transactions-type-nft = NFT
transactions-type-program = Panggilan program
transactions-type-compute = Compute
transactions-type-failed = Gagal
transactions-type-unknown = Tidak terklasifikasi

transactions-type-with-detail = { $label } ({ $detail })
transactions-type-token-transfer-detail = { $label } { $mint } ({ $amount })
transactions-type-spam-detail = Airdrop spam ({ $mint })
transactions-type-described = { $description }

transactions-filter-all = Semua Jenis
transactions-filter-transfer = Transfer
transactions-filter-ata = Rent & akun
transactions-filter-liquidity = Likuiditas
transactions-filter-program = Panggilan program

transactions-direction-incoming = Masuk
transactions-direction-outgoing = Keluar
transactions-direction-internal = Internal
transactions-direction-unknown = Tidak terklasifikasi

transactions-status-pending = Tertunda
transactions-status-confirmed = Terkonfirmasi
transactions-status-finalized = Final
transactions-status-failed = Gagal
transactions-status-success = Berhasil
transactions-status-unknown = Tidak diketahui

transactions-ata-operation-creation = Pembuatan
transactions-ata-operation-closure = Penutupan

transactions-toolbar-title = Riwayat transaksi
transactions-search =
    .placeholder = Cari signature…
    .aria-label = Cari signature transaksi
transactions-load-failed = Tidak dapat menyegarkan transaksi
transactions-summary-total = Total
transactions-summary-estimate = Perkiraan
transactions-summary-success = Berhasil
transactions-summary-failed = Gagal
transactions-filter-wallet = Dompet
transactions-filter-type = Jenis
transactions-filter-direction = Arah
transactions-filter-status = Status
transactions-filter-all-directions = Semua Arah
transactions-filter-all-statuses = Semua Status
transactions-wallet-main = Dompet utama
transactions-col-time = Waktu
transactions-col-signature = Signature
transactions-col-type = Jenis
transactions-col-direction = Arah
transactions-col-status = Status
transactions-col-native-delta = Δ { -sol }
transactions-col-fees = Biaya ({ -sol })
transactions-col-token = Token
transactions-col-router = Router
transactions-col-instructions = Instr.

transactions-dialog-copy-signature =
    .title = Salin signature
transactions-dialog-close =
    .title = Tutup (ESC)
transactions-dialog-tabs-label = Bagian detail transaksi
transactions-dialog-meta-slot = Slot:
transactions-dialog-meta-fee = Biaya:
transactions-dialog-loading = Memuat...
transactions-dialog-loading-details = Memuat detail transaksi...
transactions-dialog-load-failed = Gagal memuat detail transaksi
transactions-dialog-load-failed-reason = Gagal memuat detail transaksi: { $reason }
transactions-dialog-not-found = Transaksi tidak ditemukan
transactions-dialog-tab-overview = Ringkasan
transactions-dialog-tab-balances = Saldo
transactions-dialog-tab-instructions = Instruksi
transactions-dialog-tab-logs = Log
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = Mentah
transactions-dialog-unknown = Tidak diketahui
transactions-dialog-unknown-asset = Aset tidak diketahui
transactions-dialog-unavailable = Tidak tersedia

transactions-dialog-failed-title = Transaksi gagal
transactions-dialog-no-program-error = Tidak ada error program yang diberikan.
transactions-dialog-story-title = Apa yang terjadi
transactions-dialog-router-via = melalui { $router }
transactions-dialog-flow-paid = Dibayar
transactions-dialog-flow-received = Diterima
transactions-dialog-flow-from = Dari
transactions-dialog-flow-to = Ke
transactions-dialog-flow-amount = Jumlah
transactions-dialog-net-wallet-change = Perubahan bersih dompet:
transactions-dialog-processed = Diproses di Solana
transactions-dialog-execution-title = Eksekusi
transactions-dialog-metric-execution-price = Harga eksekusi
transactions-dialog-metric-effective-received = Diterima efektif
transactions-dialog-metric-effective-spent = Dikeluarkan efektif
transactions-dialog-metric-network-fee = Biaya jaringan
transactions-dialog-metric-estimated-pnl = Perkiraan P&L
transactions-dialog-metric-net-native-change = Perubahan bersih { -sol }
transactions-dialog-route-title = Rute dan aset
transactions-dialog-route-router = Router
transactions-dialog-route-input-asset = Aset masuk
transactions-dialog-route-output-asset = Aset keluar
transactions-dialog-route-pool = Pool
transactions-dialog-route-program = Program
transactions-dialog-tech-title = Detail teknis
transactions-dialog-tech-summary = Signature, slot, dan sumber daya
transactions-dialog-tech-signature = Signature
transactions-dialog-tech-timestamp = Stempel waktu
transactions-dialog-tech-slot = Slot
transactions-dialog-tech-exact-fee = Biaya pasti
transactions-dialog-tech-accounts = Akun
transactions-dialog-tech-instructions = Instruksi
transactions-dialog-tech-compute-units = Compute unit
transactions-dialog-tech-token-decimals = Desimal token

transactions-dialog-balances-native-title = Perubahan Saldo { -sol }
transactions-dialog-balances-native-empty = Tidak ada perubahan saldo { -sol }
transactions-dialog-balances-token-title = Perubahan Saldo Token
transactions-dialog-balances-token-empty = Tidak ada perubahan saldo token
transactions-dialog-balances-net-native = Perubahan Bersih { -sol }
transactions-dialog-balances-fee = Biaya Transaksi
transactions-dialog-col-account = Akun
transactions-dialog-col-token = Token
transactions-dialog-col-pre-balance = Saldo Sebelum
transactions-dialog-col-post-balance = Saldo Sesudah
transactions-dialog-col-change = Perubahan
transactions-dialog-col-type = Jenis
transactions-dialog-col-rent = Rent ({ -sol })
transactions-dialog-instructions-empty = Instruksi tidak ditemukan
transactions-dialog-instructions-count =
    { $count ->
       *[other] { $count } instruksi
    }
transactions-dialog-instruction-program-id = ID Program
transactions-dialog-instruction-accounts = Akun ({ $count })
transactions-dialog-instruction-data = Data
transactions-dialog-logs-empty = Tidak ada log
transactions-dialog-logs-filter = Filter log...
transactions-dialog-logs-no-match = Tidak ada log yang cocok
transactions-dialog-logs-count =
    { $count ->
       *[other] { $count } log
    }
transactions-dialog-ata-empty = Tidak ada operasi ATA dalam transaksi ini
transactions-dialog-ata-summary-title = Ringkasan Analisis ATA
transactions-dialog-ata-creations = Pembuatan
transactions-dialog-ata-closures = Penutupan
transactions-dialog-ata-rent-spent = Rent Dikeluarkan
transactions-dialog-ata-rent-recovered = Rent Dipulihkan
transactions-dialog-ata-net-rent = Dampak Bersih Rent
transactions-dialog-ata-operations-title = Operasi ATA ({ $count })
transactions-dialog-raw-copy = Salin JSON
transactions-dialog-raw-empty = Tidak ada data mentah
