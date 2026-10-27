trader-exit-type-stop-loss = Zarar durdur
trader-exit-type-take-profit = Kâr al
trader-exit-type-roi = Kâr al
trader-exit-type-roi-exit = Kâr al
trader-exit-type-trailing-stop = İz süren stop
trader-exit-type-time-override = Zaman çıkışı
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
trader-tab-take-profit = Kâr al
trader-tab-time-rules = Zaman çıkışı
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
trader-setup-action = Kurulumu tamamla
trader-toggle-on = AÇIK
trader-toggle-off = KAPALI
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
trader-force-stop-hint = Tüm izleyicileri, kopyalama görevlerini ve işlemleri bir anda durdurur.
trader-force-stop-idle = Zorla durdurmanın durduracağı çalışan bir şey yok.

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
trader-stop-loss-impact = Girişten %{ $threshold } düşünce çık
trader-stop-loss-hold-immediate = Anında
trader-stop-loss-hold-delay = Gecikme: { $span }
trader-stop-loss-price-falls = Fiyat düşer
trader-stop-loss-threshold-reached = Eşiğe ulaşıldı
trader-stop-loss-sell-part = { $part } Sat
trader-stop-loss-part-of-position = Pozisyonun bir kısmı
trader-stop-loss-sell-rest = Kalanı Sat
trader-stop-loss-still-below = Hâlâ stop seviyesinin altında
trader-stop-loss-summary = Zarar şununla sınırlandı: <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Not:</strong> Zarar durdur, erken çıkarak daha büyük zararlara karşı korur

trader-trailing-title = İz süren stop
trader-trailing-subtitle = Fiyat yükselirken onu takip ederek kârı otomatik olarak koruyun
trader-trailing-activation-impact = +%{ $value } kârda izlemeye başlar
trader-trailing-distance-impact = Tepeden -%{ $value } düşüşte çıkar
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = Devreye girdiğinde kilitlenen asgari kâr: { $value }
trader-trailing-locked-minimum-warning = Stop devreye girdiğinde giriş fiyatında veya altında kalır; yalnızca kârdayken satar. Bir taban için başa baş stopu açın.
trader-trailing-activation = Etkinleşme
trader-trailing-peak = Tepe
trader-trailing-final = Nihai +%{ $value }
trader-trailing-summary-protected = Korunan kâr: <strong>{ $value }</strong>
trader-trailing-summary-avoided = Tepeden kaçınılan zarar: <strong>{ $value }</strong>

## Break-even stop

trader-break-even-title = Başa baş stop
trader-break-even-subtitle = Pozisyon kâra geçtikten sonra zarara dönmeden önce tamamını sat
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = Son alımdan bu yana en iyi fiyat { $trigger } olduğunda, { $offset } seviyesine geri düşüş tamamını satar.

trader-take-profit-title = Kâr al
trader-take-profit-subtitle = Kâr her seviyeye ulaştıkça pozisyonu kademeli olarak satar
trader-take-profit-initial-buy = İlk alım
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = Kademe { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = Kademe { $step } kâr seviyesi
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = Kademe { $step } satış payı
trader-take-profit-profit-caption = Kâr seviyesi
trader-take-profit-sell-caption = Satış payı
trader-take-profit-add-step = Kademe ekle
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = Kademe { $step } kaldır
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = Her kâr seviyesi %{ $min } ile %{ $max } arasında olmalı
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = Her pay %{ $min } ile %{ $max } arasında olmalı
trader-take-profit-error-order = Her kademenin kâr seviyesi bir öncekinden yüksek olmalı
# $max is the largest total share, the whole position.
trader-take-profit-error-total = Kademeler birlikte en fazla %{ $max } satabilir
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = Pozisyonun %{ $runner } kadarı runner olarak açık kalır
trader-take-profit-runner-none = Son kademe pozisyonun kalanını satar
trader-take-profit-runner-warning = İz süren stop, başa baş stop, iki zaman çıkışı ve tüm çıkış stratejileri kapalıyken runner yalnızca zarar durdur ile ya da elle kapanır
trader-take-profit-info = Paylar ilk pozisyona göredir. Fiyat aynı anda birkaç seviyeyi geçerse her kontrolde bir kademe satılır.
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = +%{ $profit } kârda %{ $share } sat
trader-take-profit-example-runner = Runner
trader-take-profit-example-runner-detail = Diğer çıkış kurallarına bırakılır
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = +%{ $profit } kâra kadar <strong>%{ $sold }</strong> satar

trader-time-loss-title = Zarardaki pozisyonları kapat
trader-time-subtitle = Zarar eşiği aşılırsa, azami tutma süresinden sonra pozisyonlardan otomatik olarak çıkın
trader-time-flat-title = Durgun pozisyonları kapat
trader-time-flat-subtitle = Ne yükselen ne düşen pozisyonlardaki sermayeyi serbest bırak
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = { $span } veya daha uzun tutulan ve K/Z değeri ±{ $band } içinde kalan pozisyon kapatılır.
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
trader-time-loss-hint = Tutma süresi dolduktan sonra zaman çıkışının sattığı zarar (40 = -%40’ta çıkış)
trader-time-day = Gün { $day }
trader-time-position-opened = Pozisyon açıldı
trader-time-limit = Zaman sınırı
trader-time-hold-reached = Tutma süresine ulaşıldı
trader-time-loss-met = Zarar eşiği karşılandı
trader-time-note = <strong>Not:</strong> Kârda olan veya zararı daha küçük olan pozisyonlardan ÇIKILMAZ
trader-time-positions-title = Mevcut pozisyonların durumu
trader-time-positions-loading = Pozisyonlar yükleniyor...
trader-time-positions-empty = Açık pozisyon yok
trader-time-positions-empty-message = Açık pozisyonlar, tutma süreleri ve zaman sınırına göre ROI değerleriyle burada görünür.
trader-time-positions-token = Token
trader-time-positions-hold = Tutma süresi
trader-time-positions-roi = ROI
trader-time-positions-rule = Zaman çıkışı
trader-time-status-off = Zaman çıkışları kapalı
trader-time-status-none = Şu anda geçerli bir zaman çıkışı yok
trader-time-status-not-managed = Auto Trader tarafından yönetilmiyor
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = Zararda { $pnl }; { $remaining } içinde kapanır
trader-time-status-losing-due = Zararda { $pnl }; bir sonraki kontrolde kapanır
# $held and $remaining are formatted durations.
trader-time-status-flat = { $held } süredir durgun; { $remaining } içinde kapanır
trader-time-status-flat-due = { $held } süredir durgun; bir sonraki kontrolde kapanır

trader-strategy-entry-title = Giriş stratejileri
trader-strategy-entry-subtitle = Yeni bir pozisyon açabilen sinyaller.
trader-strategy-exit-title = Çıkış stratejileri
trader-strategy-exit-subtitle = Açık bir pozisyonu kapatabilen veya koruyabilen sinyaller.
trader-strategy-active-unknown = -- aktif
trader-strategy-active = { $enabled }/{ $total } aktif
trader-strategy-loading = Stratejiler yükleniyor...
trader-strategy-load-failed = Stratejiler yüklenemedi
trader-strategy-empty = Tanımlı strateji yok
trader-strategy-unnamed = Adsız strateji
trader-strategy-priority-auto = Otomatik
trader-strategy-priority = Öncelik { $priority }

trader-dca-title = DCA (maliyet ortalaması)
trader-dca-subtitle = Ortalama giriş fiyatınızı düşürmek için zarardaki pozisyonlara otomatik olarak ekleme yapın
trader-dca-threshold-impact = Pozisyon girişten %{ $value } aşağıdayken ek alım yapar
trader-dca-threshold-hint = DCA’nın ek alım yaptığı girişten düşüş (10 = -%10’da DCA)
trader-dca-count-impact =
    { $count ->
        [one] Pozisyon başına en fazla bir ek giriş
       *[other] Pozisyon başına en fazla { $count } ek giriş
    }
trader-dca-size-impact = Her ek giriş { $amount } alır
trader-dca-cooldown-impact = Ek girişler arasında en az { $span }
trader-dca-example =
    { $count ->
        [one] { $initial } alım → -%{ $threshold } düşüşte bir kez { $add } eklenir
       *[other] { $initial } alım → her ek -%{ $threshold } düşüş { $add } ekler, en fazla { $count } kez
    }
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
trader-timing-title = Zamanlama ve bekleme süreleri
trader-timing-subtitle = İşlemler arasındaki zamanlamayı belirleyin
trader-timing-close-cooldown = Pozisyon kapatma bekleme süresi
trader-timing-close-cooldown-hint = Aynı tokenı yeniden açmadan önce beklenecek dakika
trader-timing-concurrency = Giriş kontrolü eşzamanlılığı
trader-timing-concurrency-hint = Aynı anda kontrol edilecek token sayısı (yüksek = daha hızlı ama daha fazla CPU)
trader-timing-unit-minutes = dk
trader-timing-unit-tokens = token
