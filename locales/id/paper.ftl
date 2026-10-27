paper-readiness-history = Riwayat paper
paper-readiness-history-met =
    { $count ->
       *[other] { $count } putaran paper tertutup, dibutuhkan { $needed }
    }
paper-readiness-history-short = { $count } dari { $needed } putaran paper tertutup
paper-readiness-profit = Menguntungkan di paper
paper-readiness-profit-detail =
    { $count ->
       *[other] { $realized } { -sol } terealisasi dalam { $count } putaran, { $wins } menang
    }
paper-readiness-exits = Exit terisi
paper-readiness-exits-ok = Exit paper terbaru terisi
paper-readiness-exits-failing = Exit paper berturut-turut tanpa terisi: { $count }
paper-live-block-setup-incomplete = Selesaikan penyiapan dompet dan RPC terlebih dahulu
paper-live-block-force-stop = Penghentian darurat sedang aktif
