paper-readiness-history = Sanal geçmiş
paper-readiness-history-met =
    { $count ->
        [one] { $count } kapanmış sanal tur, { $needed } gerekli
       *[other] { $count } kapanmış sanal tur, { $needed } gerekli
    }
paper-readiness-history-short = { $needed } kapanmış sanal turdan { $count } tanesi
paper-readiness-profit = Sanalda kârlı
paper-readiness-profit-detail =
    { $count ->
        [one] { $count } turda { $realized } { -sol } gerçekleşti, { $wins } tanesi kazanıldı
       *[other] { $count } turda { $realized } { -sol } gerçekleşti, { $wins } tanesi kazanıldı
    }
paper-readiness-exits = Çıkışlar gerçekleşiyor
paper-readiness-exits-ok = Son sanal çıkışlar gerçekleşti
paper-readiness-exits-failing = Art arda gerçekleşmeyen sanal çıkışlar: { $count }
paper-live-block-setup-incomplete = Önce cüzdan ve RPC kurulumunu tamamlayın
paper-live-block-force-stop = Acil durdurma devrede
