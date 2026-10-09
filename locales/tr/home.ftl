## Portfolio overview

home-portfolio-title = Portföy değeri
home-portfolio-today = bugün
home-stat-available = Kullanılabilir { -sol }
home-stat-holdings = Token varlıkları
home-stat-open-pnl = Açık K/Z
home-stat-realized-today = Bugün gerçekleşen

home-holdings-token-count =
    { $count ->
        [one] { $count } token
       *[other] { $count } token
    }
home-holdings-with-unpriced = { $tokens } · { $count } fiyatsız
home-holdings-unpriced-note =
    { $count ->
        [one] { $count } elde tutulan tokenın fiyatı mevcut değil ve toplamda 0 sayılıyor
       *[other] { $count } elde tutulan tokenın fiyatı mevcut değil ve toplamda 0 sayılıyor
    }

## Wallet address and QR code

home-wallet-copy =
    .title = Cüzdan adresini kopyala
    .aria-label = Cüzdan adresini kopyala
home-wallet-qr-open =
    .title = Cüzdan QR kodunu göster
    .aria-label = Cüzdan QR kodunu göster
home-wallet-qr-popover =
    .aria-label = Cüzdan QR kodu
home-wallet-qr-receive = Al
home-wallet-qr-assets = { -sol } ve SPL tokenları
home-wallet-qr-close =
    .title = Kapat
    .aria-label = Cüzdan QR kodunu kapat
home-wallet-qr-preparing = QR kodu hazırlanıyor
home-wallet-qr-unavailable = QR kodu kullanılamıyor
home-wallet-qr-image =
    .alt = Ana cüzdan adresinin QR kodu

## Performance calendar

home-calendar-title = Performans takvimi
home-calendar-previous =
    .title = Önceki ay
    .aria-label = Önceki ay
home-calendar-next =
    .title = Sonraki ay
    .aria-label = Sonraki ay
home-calendar-month-pnl = Aylık K/Z
home-calendar-trades = İşlemler
home-calendar-pop-net-pnl = Net K/Z
home-calendar-pop-win-rate = Kazanma oranı
home-calendar-pop-win-rate-value = { $rate } · { $wins }K / { $losses }Z
home-calendar-pop-gross-profit = Brüt kâr
home-calendar-pop-gross-loss = Brüt zarar
home-calendar-pop-end-balance = Dönem sonu bakiyesi

## Position exposure and market pipeline

home-operations =
    .aria-label = Portföy ve piyasa durumu
home-exposure-title = Pozisyon riski
home-exposure-open = açık
home-exposure-invested = Yatırılan
home-exposure-avg-size = Ortalama boyut
home-exposure-avg-hold = Ortalama tutma süresi
home-exposure-best = En iyi
home-exposure-worst = En kötü
home-pipeline-title = Piyasa hattı
home-pipeline-tracked = Takip edilen
home-pipeline-priced = Fiyatlanan
home-pipeline-passed = Filtreleri geçen
home-pipeline-not-passed = Geçmeyen (tüm takip edilenler)
home-pipeline-blacklisted = Kara listedeki
home-pipeline-ohlcv = OHLCV
