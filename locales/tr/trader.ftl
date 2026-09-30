trader-exit-type-stop-loss = Zarar durdur
trader-exit-type-take-profit = Kâr al
trader-exit-type-roi = ROI hedefi
trader-exit-type-roi-exit = ROI hedefi
trader-exit-type-trailing-stop = İz süren stop
trader-exit-type-time-override = Zaman geçersiz kılma
trader-exit-type-time-rule = Zaman kuralı
trader-exit-type-manual = Manuel
trader-exit-type-manual-close = Manuel
trader-exit-type-dca = DCA
trader-exit-type-unknown = Bilinmiyor

trader-tab-stats = İstatistikler
trader-tab-strategy-control = Strateji kontrolü
trader-tab-strategies = Stratejiler
trader-tab-stop-loss = Zarar durdur
trader-tab-trailing-stop = İz süren stop
trader-tab-roi = Kâr al
trader-tab-time-rules = Zaman kuralları
trader-tab-dca = DCA
trader-tab-settings = Ayarlar

trader-feature-coming-soon = Yakında
    .message = Bu özellik yakında geliyor, henüz kullanılamıyor.
trader-feature-beta = Beta
trader-feature-disabled = Devre dışı
    .message = Bu özellik şu anda devre dışı.

trader-status-title = Auto Trader
trader-status-loading = Yükleniyor...
trader-status-running = Çalışıyor
trader-status-stopped = Durduruldu
trader-status-setup-required = Kurulum gerekli
trader-status-unavailable = Auto Trader'ı kullanmak için cüzdan ve RPC kurulumunu tamamlayın
trader-toggle-on = AÇIK
trader-toggle-off = KAPALI
trader-toggle-unavailable = KULLANILAMIYOR
trader-toggle-start-failed = Trader başlatılamadı
trader-toggle-stop-failed = Trader durdurulamadı
trader-controls-title = İşlem kontrolleri
trader-halt-title = İŞLEM DURDURULDU
trader-halt-reason-default = Manuel zorla durdurma
trader-halt-resume = Devam et
trader-monitor-entry = Giriş izleyici
trader-monitor-exit = Çıkış izleyici
trader-monitor-master-off = Auto Trader kapalı
trader-loss-limit-title = Dönemlik zarar limiti
trader-loss-limit-resume = İşleme devam et
trader-loss-limit-reset = Dönemi sıfırla
trader-loss-limit-off = Kapalı
trader-loss-limit-none = Dönemlik zarar limiti yapılandırılmadı
trader-loss-limit-resets-in = Sıfırlanmaya kalan: { $hours } { $minutes }
trader-loss-limit-reached = LİMİTE ULAŞILDI
trader-force-stop = Her şeyi zorla durdur

trader-force-stop-confirm = İşlemi zorla durdur
    .message = Bu, TÜM işlem operasyonlarını hemen durdurur. Devam edilsin mi?
    .confirm = İşlemi durdur
trader-loss-limit-resume-confirm = Zarar limitinden sonra devam et
    .message = Dönemlik zarar limiti yeni girişleri durdurdu. Devam etmek, dönem sıfırlanmadan trader'ın yeniden pozisyon açmasına izin verir. Devam edilsin mi?
trader-loss-limit-reset-confirm = Zarar limiti dönemini sıfırla
    .message = Bu, mevcut dönemde biriken zararı temizler ve yeni bir dönem başlatır. Devam edilsin mi?

trader-toast-control-failed = Auto Trader kontrolü başarısız oldu
trader-toast-force-stop-on = Zorla durdurma etkinleştirildi
trader-toast-force-stop-failed = Zorla durdurma etkinleştirilemedi
trader-toast-force-stop-cleared = Zorla durdurma kaldırıldı
trader-toast-resume-failed = İşleme devam edilemedi
trader-toast-loss-limit-reset-failed = Zarar limiti sıfırlanamadı
trader-toast-entry-monitor-failed = Giriş izleyici değiştirilemedi
trader-toast-exit-monitor-failed = Çıkış izleyici değiştirilemedi
trader-toast-load-failed = Yükleme başarısız
    .message = Trader yapılandırması yüklenemedi
trader-toast-saved = Yapılandırma kaydedildi
    .message = Trader ayarları başarıyla uygulandı
trader-toast-save-failed = Kaydetme başarısız
    .message = Trader yapılandırması kaydedilemedi
trader-toast-feature-enabled = Özellik etkinleştirildi
trader-toast-feature-disabled = Özellik devre dışı bırakıldı
trader-toast-feature-applied = Auto Trader ayarı uygulandı
trader-toast-strategy-enabled = Strateji etkinleştirildi
    .message = Strateji aktif
trader-toast-strategy-disabled = Strateji devre dışı bırakıldı
    .message = Strateji aktif değil
trader-toast-strategy-failed = Güncelleme başarısız
    .message = Strateji durumu güncellenemedi

trader-stats-window =
    .aria-label = İstatistik aralığı
trader-stats-window-day = 24sa
trader-stats-window-week = 7g
trader-stats-window-month = 30g
trader-realized-title = Gerçekleşmiş performans
trader-metric-net-pnl = Net K/Z
trader-metric-win-rate = Kazanma oranı
trader-metric-profit-factor = Kâr faktörü
trader-metric-max-drawdown = Maks. düşüş (drawdown)
trader-metric-capital = Çalışan sermaye
trader-metric-avg-win-loss = Ort. kazanç / kayıp
trader-metric-closed-trades = Kapanan işlemler
trader-metric-median-hold = Medyan tutma
trader-stats-empty = Bu aralıkta kapanan işlem yok
trader-stats-won-lost = { $won } kazanıldı · { $lost } kaybedildi
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
        [one] { $amount } kazanç
       *[other] { $amount } kazanç
    }
trader-stats-losses =
    { $count ->
        [one] { $amount } kayıp
       *[other] { $amount } kayıp
    }
trader-stats-expected = İşlem başına beklenen { $amount }
trader-stats-profit-factor-basis = Brüt kazanç ÷ brüt kayıp
trader-stats-drawdown-basis = Gerçekleşmiş en derin tepeden çukura düşüş
trader-stats-slots =
    { $count ->
        [one] { $used } / { $max } pozisyon yuvası kullanıldı
       *[other] { $used } / { $max } pozisyon yuvası kullanıldı
    }
trader-stats-avg-basis = Kazanan ve kaybeden işlemin ortalama sonucu
trader-stats-closed =
    { $count ->
        [one] { $amount } pozisyon kapandı
       *[other] { $amount } pozisyon kapandı
    }
trader-stats-hold-average = ort. { $span }
trader-stats-excluded =
    { $count ->
        [one] { $amount } kapalı tur hariç tutuldu — tam maliyet bazı yok, bu yüzden güvenilir K/Z hesaplanamaz.
       *[other] { $amount } kapalı tur hariç tutuldu — tam maliyet bazı yok, bu yüzden güvenilir K/Z hesaplanamaz.
    }

trader-daily-title = Günlük K/Z
trader-daily-subtitle = Günlük gerçekleşmiş { -sol }, kümülatif toplamla
trader-daily-loading = Günlük K/Z yükleniyor...
trader-daily-chart = { -sol } cinsinden günlük gerçekleşmiş kâr ve zarar
trader-extreme-best = En iyi işlem
trader-extreme-worst = En kötü işlem

trader-exit-title = Çıkış stratejisi dökümü
trader-exit-subtitle = Pozisyonların nasıl kapandığı ve her çıkışın getirisi
trader-exit-loading = Çıkış verileri yükleniyor...
trader-exit-empty-day = Son 24 saatte kapanan işlem yok
trader-exit-empty-days =
    { $count ->
        [one] Son { $amount } günde kapanan işlem yok
       *[other] Son { $amount } günde kapanan işlem yok
    }
trader-exit-share =
    { $count ->
        [one] { $amount } işlem · çıkışlardaki pay: { $share }
       *[other] { $amount } işlem · çıkışlardaki pay: { $share }
    }
trader-exit-average = ort. { $value }

trader-impact-label = Etki:
trader-current-label = Güncel:
trader-readable-label = Okunabilir:
trader-example-how-it-works = Nasıl çalışır
trader-step-entry = Giriş
trader-step-initial-position = İlk pozisyon
trader-step-auto-exit = Otomatik çıkış
trader-step-exit = Çıkış
trader-step-full-exit = Tam pozisyon çıkışı
trader-value-percent = %{ $value }
trader-example-profit = +%{ $value } kâr

trader-stop-loss-title = Zarar durdur
trader-stop-loss-subtitle = Bir pozisyonun zararı eşiğinizi aştığında otomatik olarak çıkın
trader-stop-loss-threshold-badge = Zarar limiti
trader-stop-loss-hold-badge = İsteğe bağlı gecikme
trader-stop-loss-impact = Girişten %{ $threshold } düşünce çık
trader-stop-loss-hold-immediate = Anında
trader-stop-loss-hold-delay = Gecikme: { $span }
trader-stop-loss-price-falls = Fiyat düşer
trader-stop-loss-threshold-reached = Eşiğe ulaşıldı
trader-stop-loss-partial = Kısmi çıkışlara izin verilir
trader-stop-loss-summary = Zarar şununla sınırlandı: <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Not:</strong> Zarar durdur, erken çıkarak daha büyük zararlara karşı korur

trader-trailing-title = İz süren stop
trader-trailing-subtitle = Fiyat yükselirken onu takip ederek kârı otomatik olarak koruyun
trader-trailing-activation-badge = Ne zaman başlar
trader-trailing-distance-badge = Güvenlik payı
trader-trailing-activation-impact = +%{ $value } kârda izlemeye başlar
trader-trailing-distance-impact = Tepeden -%{ $value } düşüşte çıkar
trader-trailing-activation = Etkinleşme
trader-trailing-peak = Tepe
trader-trailing-final = Nihai +%{ $value }
trader-trailing-summary-protected = Korunan kâr: <strong>{ $value }</strong>
trader-trailing-summary-avoided = Tepeden kaçınılan zarar: <strong>{ $value }</strong>

trader-roi-title = Kâr al
trader-roi-subtitle = Kâr hedefinize ulaştığında pozisyonun tamamından otomatik olarak çıkın
trader-roi-target-badge = Tek hedef
trader-roi-impact = +%{ $target } kârda çık
trader-roi-example-title = Örnek senaryo
trader-roi-initial-buy = İlk alım
trader-roi-target-hit = Hedefe ulaşıldı
trader-roi-full-position = Tam pozisyon
trader-roi-sold = %100 satıldı
trader-roi-summary = Kilitlenen kâr: <strong>+%{ $target }</strong>

trader-time-title = Zamana dayalı çıkış
trader-time-subtitle = Zarar eşiği aşılırsa, azami tutma süresinden sonra pozisyonlardan otomatik olarak çıkın
trader-time-hold-badge = Zaman tetikleyici
trader-time-loss-badge = Zarar kapısı
trader-time-unit-seconds = saniye
trader-time-unit-minutes = dakika
trader-time-unit-hours = saat
trader-time-unit-days = gün
trader-time-conversion-default = 168 saat = 7 gün
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
        [one] { $amount } saniye
       *[other] { $amount } saniye
    }
trader-duration-minutes =
    { $count ->
        [one] { $amount } dakika
       *[other] { $amount } dakika
    }
trader-duration-hours =
    { $count ->
        [one] { $amount } saat
       *[other] { $amount } saat
    }
trader-duration-days =
    { $count ->
        [one] { $amount } gün
       *[other] { $amount } gün
    }
trader-time-loss-impact = Tutma süresinden sonra %{ $value } veya daha fazla düşüşteyse çık
trader-time-day = Gün { $day }
trader-time-position-opened = Pozisyon açıldı
trader-time-limit = Zaman sınırı
trader-time-hold-reached = Tutma süresine ulaşıldı
trader-time-loss-met = Zarar eşiği karşılandı
trader-time-note = <strong>Not:</strong> Kârda olan veya zararı daha küçük olan pozisyonlardan ÇIKILMAZ
trader-time-positions-title = Mevcut pozisyonların durumu
trader-time-positions-loading = Pozisyonlar yükleniyor...
trader-time-positions-empty = Açık pozisyon yok
trader-time-positions-hold = Tutma süresi:
trader-time-positions-roi = ROI:

trader-strategy-entry-title = Giriş stratejileri
trader-strategy-entry-subtitle = Yeni bir pozisyon açabilen sinyaller.
trader-strategy-exit-title = Çıkış stratejileri
trader-strategy-exit-subtitle = Açık bir pozisyonu kapatabilen veya koruyabilen sinyaller.
trader-strategy-active-unknown = -- aktif
trader-strategy-active = { $enabled }/{ $total } aktif
trader-strategy-loading = Stratejiler yükleniyor...
trader-strategy-load-failed = Stratejiler yüklenemedi
trader-strategy-empty = Tanımlı strateji yok
trader-strategy-no-description = Açıklama girilmemiş.
trader-strategy-unnamed = Adsız strateji
trader-strategy-type-unknown = Strateji
trader-strategy-priority-auto = Otomatik
trader-strategy-priority = Öncelik { $priority }

trader-dca-title = DCA (maliyet ortalaması)
trader-dca-subtitle = Ortalama giriş fiyatınızı düşürmek için zarardaki pozisyonlara otomatik olarak ekleme yapın
trader-dca-threshold-badge = Giriş tetikleyici
trader-dca-example-title = DCA örneği
trader-dca-example = 0,01 { -sol } ilk giriş → DCA #1: 0,005 { -sol } @ -%10 → DCA #2: 0,005 { -sol } @ %10 daha
trader-dca-info-title = DCA strateji bilgisi
trader-dca-info-subtitle = DCA işlemi için önemli hususlar
trader-dca-how-title = DCA nasıl çalışır
trader-dca-how-trigger = <strong>Tetikleyici:</strong> Pozisyon DCA eşiğinin altına düşer (örn. -%10)
trader-dca-how-action = <strong>Eylem:</strong> Ortalama maliyet bazını düşürmek için daha fazla { -sol } ekle
trader-dca-how-repeat = <strong>Tekrar:</strong> Azami sayıya göre birden çok kez DCA yapılabilir
trader-dca-risk-title = Risk uyarıları
trader-dca-risk-exposure = <strong>Artan maruziyet:</strong> DCA, pozisyon başına riske atılan toplam sermayeyi artırır
trader-dca-risk-knife = <strong>Düşen bıçak:</strong> Token düşüş eğilimini sürdürürse DCA yardımcı olmaz
trader-dca-risk-cooldown = <strong>Bekleme süresi:</strong> Ardı ardına DCA girişlerinden kaçınmak için bekleme süresi kullanın

trader-sizing-title = Pozisyon boyutlandırma
trader-sizing-subtitle = Pozisyon başına ne kadar yatırım yapılacağını belirleyin
trader-sizing-positions-badge = Risk kontrolü
trader-sizing-trade-size-badge = Pozisyon başına
trader-timing-title = Zamanlama ve bekleme süreleri
trader-timing-subtitle = İşlemler arasındaki zamanlamayı belirleyin
trader-timing-close-cooldown = Pozisyon kapatma bekleme süresi
trader-timing-close-cooldown-hint = Aynı tokenı yeniden açmadan önce beklenecek dakika
trader-timing-concurrency = Giriş kontrolü eşzamanlılığı
trader-timing-concurrency-hint = Aynı anda kontrol edilecek token sayısı (yüksek = daha hızlı ama daha fazla CPU)
trader-timing-unit-minutes = dakika
trader-timing-unit-tokens = token
trader-timing-intervals = İzleyici aralıkları
trader-timing-intervals-badge = Salt okunur
trader-timing-intervals-hint = Kodda yapılandırılır (arayüzden düzenlenemez)
trader-timing-intervals-value = <strong>Giriş izleyici:</strong> 30sn | <strong>Çıkış izleyici:</strong> 5sn
