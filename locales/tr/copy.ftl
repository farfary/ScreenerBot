copy-skip-not-buy-swap = Cüzdan etkinliği bir alım değildi
copy-skip-task-disabled = Görev duraklatıldı
copy-skip-mode-transition-required = Yürütme modu ayrıca değiştirilmelidir
copy-skip-live-confirmation-required = Canlı yürütme onay gerektiriyor
copy-skip-unsupported-sizing-mode = Boyutlandırma modu henüz desteklenmiyor
copy-skip-self-copy = Cüzdan sizin kendi cüzdanlarınızdan biri
copy-skip-target-below-minimum = Cüzdan işlemi minimumun altında
copy-skip-target-above-maximum = Cüzdan işlemi maksimumun üzerinde
copy-skip-already-bought = Bu token zaten alındı (tek seferlik alım)
copy-skip-blacklisted = Token risk kontrolleri tarafından engellendi
copy-skip-filter-required = Token Filtrelemeyi geçemedi
copy-skip-budget-exhausted = Görev bütçesi tükendi
copy-skip-token-cap-reached = Token başına limite ulaşıldı
copy-skip-below-minimum-size = Kopya boyutu çok küçük
copy-skip-invalid-sizing = Görev boyutlandırması geçersiz
copy-skip-invalid-slippage = Görev kayması geçersiz
copy-skip-invalid-exit-policy = Görev çıkış kuralları geçersiz
copy-skip-invalid-price = Kullanılabilir piyasa fiyatı yok
copy-skip-not-sell-swap = Cüzdan etkinliği bir satım değildi
copy-skip-exit-mode-disabled = Cüzdan satışı yok sayıldı: görev kendi kurallarına göre satar
copy-skip-force-stopped = İşlem zorla durduruldu
copy-skip-copy-position-not-found = Bu göreve ait pozisyon yok
copy-skip-position-user-only = Pozisyonu siz yönetiyorsunuz
copy-skip-position-management-mismatch = Pozisyon artık kopya satışlarını izlemiyor
copy-skip-latency-kill-switch = Otomatik duraklatıldı: işlemler çok geç algılandı
copy-skip-claim-reconciled-abandoned = Yarım kalan canlı gönderim yeniden denenmeden kapatıldı
copy-skip-stale-observation = Kesinti sonrası yeniden oynatıldı, kopyalamak için çok eski
copy-skip-unknown-observation-time = Yeniden oynatılan işlemin blok zamanı yok
copy-skip-entry-blocked = Giriş engellendi

copy-entry-block-force-stopped = İşlem zorla durduruldu
copy-entry-block-loss-limit = Zarar limiti yeni girişleri engelliyor
copy-entry-block-connectivity = Gerekli servisler kullanılamıyor
copy-entry-block-position-limit = Açık pozisyon limitine ulaşıldı
copy-entry-block-already-open = Bir pozisyon zaten açık
copy-entry-block-reentry-cooldown = Token yeniden giriş bekleme süresi
copy-entry-block-open-cooldown = Genel giriş bekleme süresi
copy-entry-block-entry-reserved = Başka bir giriş işleniyor
copy-entry-block-blacklisted = Token risk kontrolleri tarafından engellendi
copy-entry-block-check-failed = Bir güvenlik kontrolü tamamlanamadı

copy-pause-user = Sizin tarafınızdan duraklatıldı
copy-pause-latency-kill-switch = Otomatik duraklatıldı: işlemler ortalama { $average } sn geç geldi (sınır { $threshold } sn)
copy-pause-watch-detached = Otomatik duraklatıldı: cüzdan artık izlenmiyor
copy-pause-watch-budget-exceeded = Duraklatıldı: bu cüzdan, yetişmeden önce { $limit } imzalık izleme kontrol sınırına ulaştı
copy-pause-helius-unavailable = Duraklatıldı: { -helius } cüzdan kontrolleri başarısız oldu
copy-pause-watch-processing-failed = Duraklatıldı: cüzdan etkinliği işlenemedi
copy-pause-unspecified = Duraklatıldı

copy-pause-short-user = sizce
copy-pause-short-latency-kill-switch = çok yavaş
copy-pause-short-watch-detached = izleme kayıp
copy-pause-short-watch-budget-exceeded = izleme limiti
copy-pause-short-helius-unavailable = izleme sağlayıcısı
copy-pause-short-watch-processing-failed = izleme işleme
copy-state-paused = Duraklatıldı
copy-state-paused-reason = Duraklatıldı · { $reason }

copy-readiness-history = Sanal geçmiş
copy-readiness-history-met =
    { $count ->
        [one] { $count } kapanmış sanal tur, { $needed } gerekli
       *[other] { $count } kapanmış sanal tur, { $needed } gerekli
    }
copy-readiness-history-short = { $needed } kapanmış sanal turdan { $count } tanesi
copy-readiness-profit = Sanalda kârlı
copy-readiness-profit-detail =
    { $count ->
        [one] { $count } turda { $realized } { -sol } gerçekleşti, { $wins } tanesi kazanıldı
       *[other] { $count } turda { $realized } { -sol } gerçekleşti, { $wins } tanesi kazanıldı
    }
copy-readiness-latency = İşlemler zamanında algılandı
copy-readiness-latency-detail = p95 geliş süresi { $p95 } sn, sınır { $limit } sn
copy-readiness-latency-none = Henüz geliş örneği yok
copy-readiness-priced = Her varlık fiyatlandı
copy-readiness-priced-ok = Her açık sanal varlığın havuz fiyatı var
copy-readiness-priced-missing =
    { $count ->
        [one] Havuz fiyatı olmayan açık varlık sayısı: { $count }
       *[other] Havuz fiyatı olmayan açık varlık sayısı: { $count }
    }
copy-readiness-runtime = Canlı yürütme kullanılabilir
copy-readiness-runtime-ok = Kurulum ve güvenlik kapıları canlı kopyalara izin veriyor

copy-live-block-setup-incomplete = Önce cüzdan ve RPC kurulumunu tamamlayın
copy-live-block-force-stop = Acil durdurma devrede
copy-live-block-copy-trading-disabled = Kopya işleme genel olarak duraklatıldı
copy-live-block-unavailable = Canlı yürütme kullanılamıyor

copy-state-system-paused = Genel olarak duraklatıldı
copy-state-force-stopped = Zorla durduruldu
copy-state-entries-blocked = Girişler engellendi
copy-state-running-live = Çalışıyor
copy-state-running-paper = Çalışıyor
copy-mode-paper = Sanal
copy-mode-live = Canlı
copy-exit-mode-buy-only = Benim çıkış kurallarım
copy-exit-mode-mirror = Cüzdan satışlarını yansıt
copy-exit-mode-hybrid = Cüzdan satışları ve kurallarım
copy-exit-target-sell = Cüzdan sattı
copy-exit-stop-loss = Zarar durdur
copy-exit-trailing-stop = İz süren stop
copy-exit-take-profit = Kâr al
copy-exit-time-override = Süre kuralı
copy-exit-manual = Elle kapatıldı

copy-request-failed = İstek başarısız
copy-keep-paused = Duraklatılmış tut
copy-paused-suffix = · duraklatıldı
copy-mode-paused = { $mode } · duraklatıldı
copy-task-ref = “{ $name }” ({ $mode })
copy-metric-realized-pnl = Gerçekleşmiş K/Z
copy-metric-unrealized-pnl = Gerçekleşmemiş K/Z
copy-metric-win-rate = Kazanma oranı
copy-metric-budget-spent = Harcanan bütçe
copy-metric-median-arrival = Medyan geliş süresi
copy-metric-open-holdings = Açık varlıklar
copy-record-won-lost = { $won } kazanç · { $lost } kayıp
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = Dolumlar
copy-kind-exits = Çıkışlar
copy-kind-skips = Atlananlar
copy-kind-errors = Hatalar
copy-field-per-trade-cap = İşlem başına üst sınır
copy-field-per-token-cap = Token başına üst sınır
copy-field-total-budget = Toplam bütçe
copy-field-slippage = Kayma
copy-rules-wallet-sells-only = Yalnızca cüzdan satışları
copy-filter-copy-setting-required = Kopya ayarı (gerekli)
copy-filter-copy-setting-not-required = Kopya ayarı (gerekli değil)
copy-count-closed-rounds =
    { $count ->
        [one] { $count } kapanmış tur
       *[other] { $count } kapanmış tur
    }
copy-count-open-holdings =
    { $count ->
        [one] { $count } açık varlık
       *[other] { $count } açık varlık
    }
copy-unrealized-partial =
    { $priced ->
        [one] { $priced } fiyatlı varlık · { $unpriced } fiyatsız
       *[other] { $priced } fiyatlı varlık · { $unpriced } fiyatsız
    }
copy-unrealized-unpriced =
    { $count ->
        [one] { $count } fiyatsız varlık
       *[other] { $count } fiyatsız varlık
    }
copy-range-24h = 24s
copy-range-7d = 7g
copy-range-30d = 30g
copy-range-all = Tümü
copy-range-label =
    .aria-label = Tarih aralığı

copy-page-title = Kopya İşlem
copy-page-beta = Beta
copy-strip-loading = Yükleniyor
copy-strip-unavailable = Kullanılamıyor
copy-strip-setup-required = Kurulum gerekli · kopya işlem cüzdan ve RPC gerektirir
copy-strip-pause-all = Tümünü duraklat
copy-strip-resume = İşlemeyi sürdür
copy-strip-settings = Ayarlar
copy-strip-add-wallet = Cüzdan ekle
copy-strip-paused-globally = Genel olarak duraklatıldı · yeni kopya yok, çıkışlar çalışmaya devam eder
copy-strip-force-stopped = Zorla durduruldu · hiçbir şey kopyalanmıyor
copy-strip-loss-limit = Zarar limiti · yeni girişler engellendi, çıkışlar çalışmaya devam eder
copy-strip-idle-paused =
    { $count ->
        [one] Boşta · { $count } görev duraklatıldı
       *[other] Boşta · { $count } görev duraklatıldı
    }
copy-strip-idle-empty = Boşta · henüz görev yok
copy-strip-processing = İşleniyor · { $paper } sanal
copy-strip-processing-live = İşleniyor · { $live } canlı · { $paper } sanal
copy-figures-label =
    .aria-label = Kopya işlem toplamları
copy-figure-marked-at-pool = Havuz fiyatından değerlendirildi
copy-figure-across-tasks = Tüm görevlerde
copy-figure-budget-lifetime = Etkin görevlerin toplam harcaması
copy-figure-budget-none = Etkin görev yok
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
        [one] { $count } işlem
       *[other] { $count } işlem
    }
copy-figure-arrival-none = Etkin görevlerden örnek yok

copy-load-failed = Kopya işlem yüklenemedi: { $error }
copy-resume-all-title = Kopya işlemeyi sürdür
copy-resume-all-message =
    { $count ->
        [one] Canlı görev sayısı: { $count }. Cüzdanları yeniden işlem yaptığında gerçek takaslar gönderecek.
       *[other] Canlı görev sayısı: { $count }. Cüzdanları yeniden işlem yaptığında gerçek takaslar gönderecek.
    }
copy-toast-resumed-all = Kopya işleme sürdürüldü
copy-toast-paused-all = Tüm kopya işleme duraklatıldı
copy-toast-global-failed = Kopya işleme değiştirilemedi

copy-onboarding-title = Güvendiğiniz cüzdanları kopyalayın, önce Sanalda sınayın
copy-onboarding-body = Her görev Sanalda başlar: hedef işlemler havuz fiyatından, sizin kaymanız ve ücretlerinizle simüle edilir ve çıkış kurallarınız sanal defterde çalışır. Sanal sonuçlar hak ettiğinde her cüzdan için Canlıyı etkinleştirin.
copy-onboarding-add = İlk cüzdanınızı ekleyin
copy-setup-gate-title = Kopya işlem bir cüzdan gerektirir
copy-onboarding-observe = Gözlemle
copy-onboarding-observe-detail = { -sol } harcamadan cüzdanın takaslarını algılayın.
copy-onboarding-evaluate = Değerlendir
copy-onboarding-evaluate-detail = Sanal K/Z, kazanma oranı, atlananlar, algılama hızı ve kaymayı inceleyin.
copy-onboarding-arm = Etkinleştir
copy-onboarding-arm-detail = Hazırlık kontrollerini geçin, ardından gerçek takasları açın.

copy-list-label =
    .aria-label = Kopyalanan cüzdanlar
copy-list-title = Cüzdanlar
copy-list-compare = Karşılaştır
copy-list-sort-label = Cüzdanları sırala
copy-list-count = { $active } etkin · toplam { $total }
copy-sort-pnl = K/Z
copy-sort-state = Durum
copy-sort-name = Ad
copy-compare-label =
    .aria-label = Cüzdanları karşılaştır

copy-dialog-close =
    .aria-label = Kapat
copy-editor-title-add = Cüzdan ekle
copy-editor-sub-add = Yeni görevler Sanalda başlar
copy-arm-title = Canlı kopyalamayı etkinleştir
copy-arm-sub = Cüzdanınızdan gerçek takaslar
copy-arm-keep-paper = Sanalda kal
copy-arm-confirm = Canlıyı etkinleştir
copy-profile-title = Cüzdan profili
copy-profile-sub = Bu botun cüzdanda gördükleri

copy-settings-title = Kopya işlem ayarları
copy-settings-subtitle = Tüm görevler için genel politika
copy-settings-filter-warning = Varsayılan Filtreleme kurulumuyla bu neredeyse her tokenı reddeder, bu yüzden hiçbir şey kopyalanmaz. Filtreleriniz cüzdanlarınızın işlem yaptığı tokenları geçirmiyorsa kapalı bırakın.
copy-settings-unit-seconds = sn
copy-settings-unit-trades = işlem
copy-settings-unit-tasks = görev
copy-settings-unit-rounds = tur
copy-settings-save = Ayarları kaydet
copy-settings-load-failed = Kopya ayarları yüklenemedi
copy-settings-saved = Kopya işlem ayarları kaydedildi

copy-tab-overview = Genel Bakış
copy-tab-holdings = Varlıklar
copy-tab-activity = Etkinlik
copy-tab-rules = Kurallar
copy-tab-execution = Yürütme
copy-tabs-label = Görev görünümleri
copy-workspace-select = Çalışma alanını açmak için bir cüzdan seçin.
copy-workspace-loading = Görev yükleniyor…
copy-workspace-load-failed = Bu görev yüklenemedi: { $error }

copy-state-detail-paper = Sanalda çalışıyor · işlemler simüle edilir, hiçbir şey harcanmaz
copy-state-detail-live = Canlı çalışıyor · cüzdan işlemleri gerçek takaslarla kopyalanır
copy-state-detail-system-paused = Bekliyor · kopya işleme genel olarak duraklatıldı, çıkışlar çalışmaya devam eder
copy-state-detail-entries-blocked = Girişler zarar limiti nedeniyle engellendi · çıkışlar çalışmaya devam eder
copy-state-detail-force-stopped = Zorla durduruldu · hiçbir şey kopyalanmıyor

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = Sürdürmek aynı limiti korur, bu yüzden işlemler hâlâ geç geliyorsa yeniden duraklar. RPC akışını kontrol edin veya Ayarlar'dan geliş limitini yükseltin.
copy-paused-resume-detached = Sürdürmek cüzdanı yeniden izler.
copy-paused-holdings-rules =
    { $count ->
        [one] Çıkış kuralları açık varlıkları hâlâ kapatır (açık varlık: { $count }).
       *[other] Çıkış kuralları açık varlıkları hâlâ kapatır (açık varlık: { $count }).
    }
copy-paused-holdings-mirror =
    { $count ->
        [one] Cüzdanın satışları açık varlıkları hâlâ kapatır (açık varlık: { $count }).
       *[other] Cüzdanın satışları açık varlıkları hâlâ kapatır (açık varlık: { $count }).
    }
copy-paused-holdings-hybrid =
    { $count ->
        [one] Cüzdanın satışları ve çıkış kuralları açık varlıkları hâlâ kapatır (açık varlık: { $count }).
       *[other] Cüzdanın satışları ve çıkış kuralları açık varlıkları hâlâ kapatır (açık varlık: { $count }).
    }

copy-watch-state-catching-up = Cüzdan izleme: Yetişiyor. Bu cüzdan { -helius } üzerinden kontrol ediliyor.
copy-watch-state-watching = Cüzdan izleme: İzleniyor. Bu cüzdan { -helius } üzerinden kontrol ediliyor.
copy-watch-last-check = Son kontrol: { $ago }.
copy-watch-recovery-active = Cüzdan izleme etkin
copy-watch-recovery-catching-up = Cüzdan izleme yetişiyor
copy-watch-recovery-still-paused = Kopya görevi hâlâ duraklatılmış durumda. Hazır olduğunuzda kopyalamayı sürdürün.
copy-watch-recovery-title = Cüzdan izlemeyi geri yükle
copy-watch-recovery-processing-failed = Cüzdan etkinliği işlenemedi. Kaydedilen ilerleme korunuyor. Sorun çözüldükten sonra yeniden deneyin.
copy-watch-recovery-provider-failed = { -helius } kontrolleri başarısız oldu. Kaydedilen ilerleme korunuyor. Sağlayıcı kullanılabilir olduğunda yeniden deneyin.
copy-watch-recovery-budget-intro = Bu cüzdanın etkinliği, mevcut izlemenin kontrol edebileceğinden fazla. Nasıl devam edeceğinizi seçin.
copy-watch-approve = { -helius } ile yetişmeyi dene
copy-watch-approve-help = Kaydedilen ilerlemeden devam eder. Daha fazla { -helius } kredisi kullanabilir ve yine de geride kalabilir.
copy-watch-approve-unavailable = { -helius } ile yetişme kullanılamıyor. Kontrol edilmemiş etkinliği atlamadan devam etmek için etkin bir { -helius } RPC uç noktası yapılandırın.
copy-watch-no-provider = Bu izleme için desteklenen bir yetişme sağlayıcısı yok.
copy-watch-budget-label = Kontrol başına denetlenen imza
copy-watch-budget-hint = Ya da kontrol edilmemiş etkinliği atlayıp şu andan devam edin. Kontrol başına { $min }–{ $max } imza seçin; daha yüksek limit daha fazla RPC çağrısı kullanabilir.
copy-watch-ack = Kaçırılan etkinliğin kopyalanmayacağını anlıyorum.
copy-watch-toast-range = Yoklama başına { $min } ile { $max } arasında imza seçin ({ $step } imzalık adımlarla)
copy-watch-toast-ack = Son tamamlanan kontrolden bu yana gelen imzaların atlanacağını onaylayın
copy-watch-resumed = Cüzdan izleme şu andan itibaren sürdürüldü; kopya görevi duraklatılmış durumda
copy-watch-resume-failed = Cüzdan izleme sürdürülemedi
copy-watch-retry-started = Cüzdan izleme yeniden denemesi kaydedilen ilerlemeden başladı; kopya görevi duraklatılmış durumda
copy-watch-retry-failed = Cüzdan izleme yeniden denenemedi
copy-watch-approve-title = Bu cüzdan için { -helius } ile yetişmeye izin ver
copy-watch-approve-message = { -helius }, başarılı Solana işlemlerini kaydedilen ilerlemeden itibaren, kontrol edilmemiş aralığı atlamadan kontrol edebilir. Şu anda döndürülen her 100 tam işlem için (yukarı yuvarlanarak) 10 kredi, istek başına en az 10 kredi ücret alır. Bir kontrol birden fazla istek yapabilir; kullanım ve sağlayıcı fiyatlandırması değişebilir. Kopyalama, siz ayrıca sürdürene kadar duraklatılmış kalır.
copy-watch-approve-confirm = Bu cüzdan için izin ver
copy-watch-approved = Cüzdan izleme kaydedilen ilerlemeden başladı; kopya görevi duraklatılmış durumda
copy-watch-restore-failed = Cüzdan izleme geri yüklenemedi

copy-action-pause = Duraklat
copy-action-resume = Sürdür
copy-action-resume-copy = Kopyalamayı sürdür
copy-action-resume-from-now = Şu andan sürdür
copy-action-retry-watch = Cüzdan izlemeyi yeniden dene
copy-action-return-paper = Sanala dön
copy-action-edit-rules = Kuralları düzenle
copy-action-clone = Klonla
copy-action-profile = Cüzdan profili
copy-resume-live-title = Canlı kopyalamayı sürdür
copy-resume-live-message = “{ $name }”, bu cüzdan yeniden işlem yaptığında cüzdanınızdan gerçek takaslar gönderecek.
copy-resume-live-confirm = Canlıyı sürdür
copy-task-resumed = Görev sürdürüldü
copy-task-paused = Görev duraklatıldı
copy-task-state-failed = Görev durumu değiştirilemedi
copy-return-paper-message = “{ $name }” görevinin yeni kopyaları { -sol } harcamadan yeniden simüle edilecek.
copy-return-paper-cancel = Canlıda kal
copy-task-returned-paper = Görev Sanala döndürüldü
copy-mode-change-failed = Yürütme modu değiştirilemedi
copy-delete-title = Kopya görevini sil
copy-delete-message = “{ $name }” silinsin mi? Kararları ve sanal sonuçları kaldırılır ve cüzdan bu görev için artık izlenmez.
copy-delete-confirm = Görevi sil
copy-delete-cancel = Görevi koru
copy-task-deleted = Kopya görevi silindi
copy-task-delete-failed = Kopya görevi silinemedi

copy-overview-results = Sonuçlar
copy-analytics-load-failed = Analizler yüklenemedi: { $error }
copy-analytics-loading = Analizler yükleniyor…
copy-exit-bucket =
    { $count ->
        [one] { $count } satış · { $pnl }
       *[other] { $count } satış · { $pnl }
    }
copy-overview-average-win = Ortalama kazanç
copy-overview-average-loss = Ortalama kayıp { $amount }
copy-overview-profit-factor = Kâr faktörü
copy-overview-profit-factor-note = Brüt kazanç ÷ brüt kayıp
copy-overview-average-hold = Ortalama tutma süresi
copy-overview-average-hold-note = Girişten çıkışa
copy-overview-best-round = En iyi tur
copy-overview-worst-round = En kötü { $amount }
copy-overview-curve-title = Kümülatif K/Z
copy-overview-exits-title = Çıkışa göre satışlar
copy-overview-skips-title = İşlemler neden atlandı
copy-book-title-live = Canlı defter
copy-book-title-paper = Sanal defter
copy-book-all-time = Tüm zamanlar
copy-book-buys = <strong>{ $count }</strong> alım
copy-book-policy-exits = <strong>{ $count }</strong> kurallarınıza göre çıkış
copy-book-wallet-sells = <strong>{ $count }</strong> cüzdan satışı
copy-book-manual-closes = <strong>{ $count }</strong> elle kapatıldı
copy-book-skipped = <strong>{ $count }</strong> atlandı
copy-book-failed = <strong>{ $count }</strong> başarısız
copy-book-closed = { $count } kapandı
copy-book-budget-note = { $mode } harcaması: { $total } · { $remaining } kaldı
copy-check-passed = geçti
copy-check-not-passed = geçmedi
copy-readiness-title = Canlıya geçmeden önce
copy-readiness-live-note = Bu görev canlı işlem yapıyor. Yukarıdaki başlıktan Sanala döndürün.
copy-readiness-all-pass = Tüm kontroller geçiyor.
copy-readiness-needs-review = Etkinleştirme, hazır olmayanların açıkça gözden geçirilmesini gerektirir.
copy-readiness-arm = Gözden geçir ve canlıyı etkinleştir

copy-rules-title = Geçerli kurallar
copy-rules-size-ratio = Cüzdan işleminin payı: { $pct }
copy-rules-size-fixed = Kopya başına { $amount }
copy-rules-target-any = Herhangi bir boyut
copy-rules-target-min = En az { $amount }
copy-rules-target-max = En fazla { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = Görev ayarı · Trader: { $value }
copy-rules-source-default = Trader varsayılanı
copy-rules-not-used = Kullanılmıyor: cüzdanın satışları belirler
copy-rules-col-rule = Kural
copy-rules-col-applies = Geçerli
copy-rules-col-source = Kaynak
copy-rules-budget-note = { $mode } modunda harcanan: { $spent } · { $remaining } kaldı
copy-rules-token-copies =
    { $count ->
        [one] Bir token için yaklaşık { $count } tam kopya
       *[other] Bir token için yaklaşık { $count } tam kopya
    }
copy-rules-sizing = Boyutlandırma
copy-rules-copy-size = Kopya boyutu
copy-rules-entry-filters = Giriş filtreleri
copy-rules-target-size = Cüzdan işlem boyutu
copy-rules-repeat-buys = Tekrarlanan alımlar
copy-rules-repeat-first-only = Her tokenın yalnızca ilk alımı
copy-rules-repeat-every = Her alım, token başına üst sınıra kadar
copy-rules-filter-pass = Filtreleme geçişi
copy-rules-filter-required = Gerekli
copy-rules-filter-not-required = Gerekli değil
copy-rules-filter-task-override = Görev ayarı
copy-rules-exits = Çıkışlar
copy-rules-exits-inactive = Varlıklar yalnızca cüzdan sattığında satılır; aşağıdaki kurallar bu modda çalışmaz.

copy-rule-status = Durum
copy-rule-on = Açık
copy-rule-off = Kapalı
copy-rule-unit-seconds = sn
copy-rule-unit-minutes = dk
copy-rule-stop-loss-threshold = Şu zararda satar
copy-rule-stop-loss-min-hold = Şu kadar tutmadan önce değil
copy-rule-no-minimum = Minimum yok
copy-rule-partial-exits = Kısmi çıkışlar
copy-rule-partial-allowed = İzinli
copy-rule-partial-full-only = Yalnızca tam çıkış
copy-rule-partial-size = Kısmi çıkış boyutu
copy-rule-trailing-activation = Şu kazançta devreye girer
copy-rule-trailing-distance = Zirvenin şu kadar altında satar
copy-rule-take-profit-target = Şu kazançta satar
copy-rule-time-duration = Şu kadar tuttuktan sonra kontrol eder
copy-rule-time-threshold = K/Z şuna eşit veya altındayken satar
copy-preset-inherit = Trader varsayılanları
copy-preset-conservative = Temkinli
copy-preset-balanced = Dengeli
copy-preset-aggressive = Agresif
copy-preset-custom = Özel
copy-validate-stop-loss = Zarar durdur %0'ın üzerinde ve en fazla %100 olmalıdır.
copy-validate-partial-size = Kısmi çıkış boyutu %0 ile %100 arasında olmalıdır.
copy-validate-min-hold = Minimum tutma süresi tam sayı saniye olmalıdır.
copy-validate-trailing-activation = İz süren stop devreye girme eşiği %0'ın üzerinde ve en fazla %100 olmalıdır.
copy-validate-trailing-distance = İz süren stop mesafesi %0'ın üzerinde ve en fazla %100 olmalıdır.
copy-validate-take-profit = Kâr al %0'ın üzerinde olmalıdır.
copy-validate-time-duration = Süre kuralı sıfırdan büyük bir süre gerektirir.
copy-validate-time-threshold = Süre kuralı eşiği bir zarardır: %0 veya negatif bir sayı kullanın.
copy-warning-mirror = Varlıkları yalnızca cüzdanın satışları kapatır: hiçbir zarar durdur onları korumaz ve cüzdanın hiç satmadığı bir token elde kalır.
copy-warning-no-rules = Hiçbir çıkış kuralı açık değil ve cüzdan satışları yok sayılıyor: varlıklar hiç satılmaz.
copy-warning-no-stop-loss = Zarar durdur uygulanmıyor: düşen bir token, başka bir kural veya cüzdan satana kadar elde tutulur.
copy-warning-stop-delay = Zarar durdur her alımdan sonra { $hold } bekler: daha hızlı düşen bir token çok daha aşağıda kapanır (eşik: { $threshold }).
copy-warning-take-profit-cost = { $target } hedefli kâr al, satış maliyetini ({ $slippage } kayma ve { $fee } takas ücreti) karşılamıyor, bu yüzden turları zararla kapatır.
copy-warning-trailing-distance = İz süren mesafe, devreye girme kazancına eşit veya ondan büyük; bu yüzden devreye girmiş bir iz girişin altında satabilir.

copy-execution-title = Yürütme kalitesi
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = Herhangi
copy-execution-limit-on =
    { $count ->
        [one] Son { $count } işlemin ortalaması { $limit } üzerine çıkarsa duraklatır
       *[other] Son { $count } işlemin ortalaması { $limit } üzerine çıkarsa duraklatır
    }
copy-execution-limit-off = Acil durdurma anahtarı kapalı
copy-execution-arrival-samples =
    { $count ->
        [one] { $count } işlem gerçekleştiği anda görüldü
       *[other] { $count } işlem gerçekleştiği anda görüldü
    }
copy-execution-p95 = p95 geliş süresi
copy-execution-median-slippage = Medyan kayma
copy-execution-slippage-samples =
    { $count ->
        [one] { $count } ölçülen dolum
       *[other] { $count } ölçülen dolum
    }
copy-execution-worst-slippage = En kötü kayma
copy-execution-average-slippage = Ortalama { $amount }
copy-execution-delay-title = Algılama gecikmesi
copy-execution-delay-note = Cüzdanın bloğundan bu botun işlemi görmesine kadar geçen süre. Kesinti sonrası yeniden oynatmalar hariç tutulur.
copy-execution-delay-limit = { $limit } geliş limitini aşan çubuklar kehribar renginde gösterilir.
copy-execution-fastest = En hızlı
copy-execution-average = Ortalama
copy-execution-slowest = En yavaş
copy-execution-fill-title = Cüzdana göre dolum
copy-execution-fill-note = Pozitif, cüzdandan daha kötü demektir: alımda daha fazla ödendi, yansıtılan satışta daha az alındı. Havuz fiyatı olmayan bir tokenın sanal dolumu cüzdanın kendi işlem fiyatıyla fiyatlandığından hiçbir şey ölçmez ve dışarıda bırakılır.
copy-execution-samples = Örnekler
copy-execution-median = Medyan
copy-execution-worst = En kötü
copy-execution-decisions = Aralıktaki kararlar

copy-compare-title = Cüzdanları karşılaştır
copy-compare-back = Cüzdana dön
copy-compare-load-failed = Karşılaştırma yüklenemedi: { $error }
copy-compare-loading = Karşılaştırma yükleniyor…
copy-compare-empty = Karşılaştırılacak görev yok.
copy-compare-empty-message = Sonuçlarını diğerleriyle karşılaştırmak için bir kopyalama görevi ekleyin.
copy-compare-curve-title = Kümülatif gerçekleşmiş K/Z
copy-table-wallet = Cüzdan
copy-table-mode = Mod
copy-table-rounds = Turlar
copy-table-realized = Gerçekleşen
copy-table-profit-factor = Kâr faktörü
copy-table-average-hold = Ort. tutma
copy-table-median-slippage = Medyan kayma

copy-chart-curve-label = Kümülatif K/Z { $amount } { -sol }
copy-chart-compare-label = Göreve göre kümülatif K/Z
copy-chart-empty-curve = Bu aralıkta henüz kapanmış tur yok.
copy-chart-empty-bars = Bu aralıkta kayıt yok.
copy-chart-empty-histogram = Bu aralıkta geliş örneği yok.
copy-chart-empty-compare = Bu aralıkta karşılaştırılacak kapanmış tur yok.
copy-chart-histogram-title = { $count } / { $total }

copy-profile-copy = Bu cüzdanı kopyala
copy-profile-copy-other = Diğer kurallarla kopyala
copy-profile-loading = Cüzdan profili yükleniyor…
copy-profile-watch-title = İzleme
copy-profile-watched = İzleniyor
copy-profile-watch-resume-hint = Görevi sürdürmek cüzdanı yeniden izler
copy-profile-watch-add-hint = Görev eklemek izlemeyi başlatır
copy-profile-stream = Akış
copy-profile-subscribed = Abone
copy-profile-not-subscribed = Abone değil
copy-profile-sources =
    { $count ->
        [one] { $count } kaynak
       *[other] { $count } kaynak
    }
copy-profile-last-activity = Son etkinlik
copy-profile-last-error = Son hata
copy-profile-own-wallet = Bu sizin kendi cüzdanlarınızdan biri; kopyalanması reddedilir.
copy-profile-observed-title = Gözlenen işlemler
copy-profile-observed-none = Bu botta bu cüzdandan henüz işlem yok. Sanal görev, { -sol } harcamadan cüzdanı gözlemler.
copy-profile-swaps-seen = Görülen takaslar
copy-profile-swaps-seen-note = Görevleriniz genelinde benzersiz cüzdan takasları
copy-profile-buys-sells = Alımlar / satımlar
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = İşlem gören tokenlar
copy-profile-first-seen = İlk görülme
copy-profile-last-seen = Son görülme
copy-profile-tasks-title = Bu cüzdandaki görevleriniz
copy-table-task = Görev

copy-arm-acks-left =
    { $count ->
        [one] İşaretlenecek onay sayısı: { $count }
       *[other] İşaretlenecek onay sayısı: { $count }
    }
copy-arm-readiness-title = Sanal defterden hazırlık durumu
copy-arm-exposure-title = Risk
copy-arm-per-copy = Kopya başına
copy-arm-budget-left-value = { $left } / { $total } { -sol }
copy-arm-budget-left = Kalan canlı bütçe
copy-arm-budget-left-note = Sanal harcama ayrı sayılır ve bunu kullanmaz
copy-arm-exits = Çıkışlar
copy-arm-stop-note = { $hold } tutmadan önce değil: daha hızlı düşüş daha aşağıda kapanır
copy-arm-shared = Bu cüzdan ayrıca şu görevlerce kopyalanıyor: { $tasks }. Her görev işlemleri kendi bütçesiyle kopyalar.
copy-arm-unavailable = Canlı yürütme şu anda kullanılamıyor; son kontrole bakın.
copy-arm-ack-real-native = Gerçek { -sol }: bu görev cüzdanınızdan en fazla { $budget } { -sol } harcayabilir, kopya başına en fazla { $trade } { -sol }.
copy-arm-ack-fees = Canlı kopyalar gerçek ağ ücretleri ve kayma öder; sanal sonuçlar canlı sonuçların garantisi değildir.
copy-arm-ack-unready = Bazı hazırlık kontrolleri geçmedi. Yine de bu görevi etkinleştir.
copy-arm-lead = “{ $name }”, bu cüzdanın işlemlerini cüzdanınızdan gerçek takaslarla kopyalayacak.
copy-arm-confirmation-missing = Canlı onay yüklenemedi
copy-arm-armed = Canlı kopyalama etkinleştirildi
copy-arm-failed = Canlı kopyalama etkinleştirilemedi

copy-holdings-title = Varlıklar
copy-holdings-view-label = Varlık görünümü
copy-holdings-view-open = Açık ({ $count })
copy-holdings-view-closed = Kapanmış turlar ({ $count })
copy-holdings-reset = Sanal defteri sıfırla
copy-holdings-live-note = Canlı kopyalar gerçek pozisyonlardır.
copy-holdings-open-positions = Açık Pozisyonlar
copy-holdings-token-details = Token ayrıntılarını aç
copy-holdings-opened = Açılış: { $time }
copy-holdings-no-pool-price = Havuz fiyatı yok
copy-holdings-close = Kapat
copy-holdings-write-off = Zarara yaz
copy-holdings-activity = Etkinlik
copy-holdings-no-exit-rule = Çıkış kuralı yok
copy-holdings-watch-stop = Stop { $level }
copy-holdings-watch-stop-until = Stop { $level }, { $span } sonra
copy-holdings-watch-take = Kâr al { $level }
copy-holdings-watch-trail = İz { $level }
copy-holdings-watch-trail-arms = İz devreye girer { $level }
copy-holdings-watch-time = Süre ≤ { $level }
copy-holdings-watch-time-until = Süre ≤ { $level }, { $span } sonra
copy-holdings-watch-wallet-sells = Cüzdan satışları
copy-holdings-empty = Açık sanal varlık yok. Cüzdandan kopyalanan alımlar burada görünür.
copy-holdings-col-token = Token
copy-holdings-col-cost = Maliyet
copy-holdings-col-entry = Giriş
copy-holdings-col-mark = Güncel
copy-holdings-col-peak = Zirve
copy-holdings-col-pnl = K/Z
copy-holdings-col-exit-rules = Çıkış kuralları
copy-holdings-col-held = Tutulan
copy-holdings-col-actions = İşlemler
copy-holdings-col-invested = Yatırılan
copy-holdings-col-proceeds = Hasılat
copy-holdings-col-exit = Çıkış
copy-holdings-col-closed = Kapanış
copy-holdings-price-note = Fiyatlar token başına { -sol } cinsindendir. Giriş, alımın kaymasını ve ücretlerini içerir; zirve ve çıkış seviyeleri ona göre hesaplanır, bu yüzden bir varlık zirvesi girişin altında olacak şekilde açılır. Havuz fiyatı için üzerine gelin.
copy-holdings-paused-rules = Duraklatıldı: yeni kopya yok. Çıkış kurallarınız bu varlıkları hâlâ kapatır.
copy-holdings-paused-mirror = Duraklatıldı: yeni kopya yok. Cüzdanın satışları bu varlıkları hâlâ kapatır.
copy-holdings-paused-hybrid = Duraklatıldı: yeni kopya yok. Cüzdanın satışları ve çıkış kurallarınız bu varlıkları hâlâ kapatır.
copy-holdings-closed-load-failed = Kapanmış turlar yüklenemedi: { $error }
copy-holdings-closed-loading = Kapanmış turlar yükleniyor…
copy-holdings-closed-empty = Henüz kapanmış tur yok.
copy-holdings-closed-latest = Toplam { $total } turun son { $shown } tanesi.
copy-holdings-close-title = Sanal varlığı kapat
copy-holdings-close-message = { $token } sanal defterde havuz fiyatından ({ $price }), görevin kayması ve ücretleriyle satılsın.
copy-holdings-close-confirm = Varlığı kapat
copy-holdings-write-off-title = Sanal varlığı zarara yaz
copy-holdings-write-off-message = { $token } için satılacak havuz fiyatı yok. Zarara yazmak onu sıfırdan kapatır ve { $cost } maliyetini zarar olarak kaydeder.
copy-holdings-keep = Koru
copy-holdings-written-off = Zarara yazıldı: { $token }
copy-holdings-closed = Kapatıldı: { $token }
copy-holdings-written-off-detail = Sıfır hasılatla kapatıldı
copy-holdings-sold-at = { $price } fiyatından satıldı
copy-holdings-close-failed = Varlık kapatılamadı
copy-holdings-reset-message = “{ $name }” görevini baştan başlatın: sanal varlıkları, harcaması, dolumları, çıkışları ve atlananları kaldırılır. Kurallar ve cüzdan kalır.
copy-holdings-reset-cancel = Geçmişi koru
copy-holdings-reset-done = Sanal defter sıfırlandı
copy-holdings-reset-detail =
    { $count ->
        [one] { $count } karar kaldırıldı
       *[other] { $count } karar kaldırıldı
    }
copy-holdings-reset-failed = Sanal defter sıfırlanamadı

copy-activity-title = Etkinlik
copy-activity-filter-label = Etkinlik filtresi
copy-filter-all = Tümü
copy-outcome-paper-filled = Sanal alım
copy-outcome-live-submitted = Canlı alım gönderildi
copy-outcome-live-confirmed = Canlı alım onaylandı
copy-outcome-live-failed = Canlı alım başarısız
copy-outcome-paper-sell-observed = Sanal satış · cüzdan sattı
copy-outcome-live-sell-submitted = Canlı satış gönderildi
copy-outcome-live-sell-failed = Canlı satış başarısız
copy-outcome-skipped = Atlandı
copy-activity-decision = Karar
copy-activity-paper-exit = Sanal çıkış · { $rule }
copy-activity-filled = { $price } fiyatından { $input } · cüzdan { $target } aldı
copy-activity-filled-slippage = { $price } fiyatından { $input } · cüzdan { $target } aldı · kayma { $slippage }
copy-activity-filled-unpriced = { $price } fiyatından { $input } · cüzdan { $target } aldı · cüzdanın işlem fiyatıyla fiyatlandı, havuz fiyatı yok
copy-activity-live-sized = { $sized } · cüzdan { $target } aldı
copy-activity-sell-nothing = Cüzdan { $amount } sattı · satılacak varlık yok
copy-activity-written-off = Sıfırdan zarara yazıldı: havuz fiyatı yok
copy-activity-sold = { $tokens } token, { $proceeds } karşılığında { $price } fiyatından satıldı
copy-activity-full-close = Tam kapanış
copy-activity-partial-exit = { $pct } çıkış
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = minimum { $amount }
copy-activity-skip-maximum = maksimum { $value }
copy-activity-skip-stale = { $arrival } geç, sınır { $limit }
copy-activity-skip-latency = ortalama { $average }, sınır { $limit }
copy-activity-arrival-replayed = Bloktan { $span } sonra yeniden oynatıldı
copy-activity-arrival-seen = Bloktan { $span } sonra görüldü
copy-activity-link-wallet-tx = Cüzdan işlemi
copy-activity-link-own-tx = Sizin işleminiz
copy-activity-only-token = Yalnızca bu token
copy-activity-skipped-group = Atlanan ×{ $count }
copy-activity-group-detail =
    { $tokens ->
        [one] { $tokens } token · { $since } tarihinden beri
       *[other] { $tokens } token · { $since } tarihinden beri
    }
copy-activity-mint-filter =
    .placeholder = Token mint
    .aria-label = Token mint'e göre filtrele
copy-activity-clear = Temizle
copy-activity-load-failed = Etkinlik yüklenemedi: { $error }
copy-activity-loading = Etkinlik yükleniyor…
copy-activity-no-match = Bu filtreyle eşleşen bir şey yok.
copy-activity-empty = Henüz karar yok. Cüzdan işlem yaptıkça dolumlar, çıkışlar ve atlananlar burada görünür.
copy-activity-load-older = Daha eskileri yükle
copy-activity-start = Geçmişin başı
copy-activity-older-failed = Daha eski etkinlik yüklenemedi

copy-step-wallet = Cüzdan
copy-step-sizing = Boyutlandırma
copy-step-entry = Giriş filtreleri
copy-step-exits = Çıkışlar
copy-step-review = Özet
copy-editor-title-edit = Düzenle: { $name }
copy-editor-title-clone = Klonla: { $name }
copy-editor-sub-edit = { $mode } görevi · değişiklikler sonraki kararlarına uygulanır
copy-editor-sub-clone = Aynı kurallar, boş sanal defter, Sanalda başlar
copy-editor-save-edit = Değişiklikleri kaydet
copy-editor-save-clone = Klon oluştur
copy-editor-save-create = Sanal görev oluştur
copy-editor-clone-suffix = (kopya)
copy-editor-discard-edit = Değişiklikleri at
copy-editor-discard-create = Bu görevi at
copy-editor-discard-edit-message = “{ $name }” için yaptığınız değişiklikler kaydedilmedi.
copy-editor-discard-create-message = Şimdiye kadar girilen cüzdan ve kurallar kaydedilmedi.
copy-editor-discard-confirm = At
copy-editor-keep-editing = Düzenlemeye devam et
copy-editor-toast-updated = Görev güncellendi
copy-editor-toast-clone = Klon oluşturuldu
copy-editor-toast-created = Sanal görev oluşturuldu
copy-unit-native = { -sol }
copy-editor-any = Herhangi
copy-editor-duplicate = Zaten şu görevlerce kopyalanıyor: { $tasks }. Bu görev aynı işlemleri kendi kuralları ve bütçesiyle yeniden kopyalar.
copy-editor-wallet = Cüzdan
copy-editor-wallet-identity = Bir görevin cüzdanı onun kimliğidir. Başka bir cüzdanı bu kurallarla kopyalamak için görevi klonlayın.
copy-editor-address-label = Cüzdan adresi
copy-editor-address-placeholder = Solana cüzdan adresi
copy-editor-address-help-clone = Aynı kurallar, boş bir sanal defterle. Diğer kuralları denemek için bu cüzdanı koruyun veya başka bir cüzdan girin.
copy-editor-address-help-create = Bu görevin alımlarını (ve seçerseniz satışlarını) kopyaladığı cüzdan.
copy-editor-name-label = Ad <em>isteğe bağlı</em>
copy-editor-name-placeholder = ör. Hızlı rotasyoncu
copy-editor-enabled-title = Cüzdanın işlemlerini işle
copy-editor-enabled-help = Kapalıysa görev, siz sürdürene kadar duraklatılmış kalır.
copy-editor-note-live = Bu görev canlı: değişiklikler sonraki gerçek kopyalarına uygulanır.
copy-editor-note-paper = Görevler siz etkinleştirene kadar Sanalda çalışır: işlemler havuz fiyatından simüle edilir ve hiçbir şey harcanmaz.
copy-editor-copy-size = Kopya boyutu
copy-editor-sizing-fixed = Sabit tutar
copy-editor-sizing-ratio = Cüzdan işleminin payı
copy-editor-amount-fixed = Kopya başına tutar
copy-editor-amount-ratio = Her işlemin payı
copy-editor-amount-help-fixed = Kopyalanan her alımda harcanır, en az { $minimum }.
copy-editor-amount-help-ratio = Cüzdanın kendi alımından, işlem başına üst sınıra kadar.
copy-editor-help-trade-cap = Hiçbir tek kopya bundan fazla harcamaz.
copy-editor-help-token-cap = Bir token için harcanan toplam.
copy-editor-help-budget = Bu görevin ömrü boyunca harcayabileceği toplam; Sanal ve Canlı kendi harcamasını ayrı sayar.
copy-editor-preview-title = Bir kopyanın maliyeti
copy-editor-preview-empty = Bir kopyanın maliyetini görmek için boyutlandırmayı girin.
copy-editor-preview-example = Cüzdan { $target } alır → siz <strong>{ $copy }</strong> kopyalarsınız
copy-editor-preview-once = Her token bir kez alındığı için bir token tek kopya alır: { $size }
copy-editor-preview-token-cap =
    { $count ->
        [one] Bir token en fazla { $count } kopya alır, kopya boyutu { $size }
       *[other] Bir token en fazla { $count } kopya alır, kopya boyutu { $size }
    }
copy-editor-preview-summary-exact = { $perToken }; bütçe bunlardan yaklaşık { $count } tanesini karşılar. Ağ ve öncelik ücretleri ayrıca eklenir.
copy-editor-preview-summary-minimum = { $perToken }; bütçe bunlardan en az { $count } tanesini karşılar. Ağ ve öncelik ücretleri ayrıca eklenir.
copy-editor-target-min = Kopyalanan en küçük cüzdan işlemi
copy-editor-target-min-help = Cüzdanın daha küçük alımlarını yok sayın. Minimum istemiyorsanız boş bırakın.
copy-editor-target-max = Kopyalanan en büyük cüzdan işlemi
copy-editor-target-max-help = Cüzdanın daha büyük alımlarını yok sayın. Maksimum istemiyorsanız boş bırakın.
copy-editor-buy-once-title = Her tokenı bir kez al
copy-editor-buy-once-help = Yalnızca cüzdanın bir tokenı ilk alımını kopyalayın; sonraki alımlar atlanır.
copy-editor-filter-require = Gerekli kıl
copy-editor-filter-skip = Gerekli kılma
copy-editor-filter-help = Bir tokenın kopyalanmadan önce Filtreleme hattınızı geçmesini isteyin.
copy-editor-filter-warning = Varsayılan Filtreleme kurulumuyla neredeyse her token başarısız olur, bu yüzden geçişi zorunlu kılan bir görev hiçbir şey kopyalamaz. Yalnızca filtreleriniz bu cüzdanın işlem yaptığı tokenları geçiriyorsa zorunlu kılın.
copy-editor-exit-both = İkisi de
copy-editor-exit-help-buy-only = Aşağıdaki kurallarınız her varlığı satar; cüzdanın satışları yok sayılır.
copy-editor-exit-help-hybrid = Hangisi önce gelirse: cüzdan satar veya kurallarınızdan biri tetiklenir.
copy-editor-exit-help-mirror = Varlıklar yalnızca cüzdan sattığında satılır. Çıkış kurallarınız çalışmaz.
copy-editor-who-sells = Kim satar
copy-editor-preset = Ön ayar
copy-editor-preset-help = Bir ön ayar aşağıdaki her kuralı doldurur; sonrasında herhangi birini ayarlayabilirsiniz.
copy-editor-mirror-note = Cüzdanın satışları belirlerken bu kurallar çalışmaz. { $mine } veya { $both } seçeneğine geçerseniz geçerli olurlar.
copy-editor-rule-inherit = Trader varsayılanı
copy-editor-inherit-value = Trader varsayılanı ({ $value })
copy-editor-rule-aria = { $rule } ayarı
copy-editor-rule-empty-uses = Boş bırakılırsa Trader varsayılanı kullanılır: { $value }
copy-editor-rule-follows = Trader'ı izler: { $summary }
copy-editor-rule-follows-plain = Trader'ın ayarını izler.
copy-editor-rule-follows-own = Açma/kapama Trader'ı izler, değerler bu görevin: { $summary }
copy-editor-rule-off-note = Trader ne kullanırsa kullansın bu görev için kapalı.
copy-task-unnamed = Adsız görev
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = kaydedilince işlemleri işler
copy-editor-review-paused = duraklatılmış olarak kaydedilir
copy-editor-error-address = Geçerli bir Solana cüzdan adresi girin.
copy-editor-error-sizing = Her boyutlandırma değeri sıfırın üzerinde olmalıdır.
copy-editor-error-min-copy = Bir kopya en az { $minimum } olmalıdır: kopya başına tutarı yükseltin.
copy-editor-error-min-cap = Bir kopya en az { $minimum } olmalıdır: işlem başına üst sınırı yükseltin.
copy-editor-error-trade-cap = İşlem başına üst sınır, token başına üst sınırı aşamaz.
copy-editor-error-token-cap = Token başına üst sınır, toplam bütçeyi aşamaz.
copy-editor-error-slippage = Kayma { $min } ile { $max } arasında olmalıdır.
copy-editor-error-target-limits = Cüzdan işlem limitleri sıfır veya daha büyük olmalıdır.
copy-editor-error-target-order = En küçük cüzdan işlemi en büyüğünü aşamaz.

copy-notice-task-unnamed = Görev #{ $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = Sanal kopya alımı
copy-notice-title-paper-sell = Sanal kopya satışı
copy-notice-title-paper-closed = Sanal varlık kapatıldı
copy-notice-title-paper-exit = Sanal çıkış: { $rule }
copy-notice-title-live-buy-submitted = Canlı kopya alımı gönderildi
copy-notice-title-live-buy-confirmed = Canlı kopya alımı onaylandı
copy-notice-title-live-buy-failed = Canlı kopya alımı başarısız
copy-notice-title-live-sell-submitted = Canlı kopya satışı gönderildi
copy-notice-title-live-sell-failed = Canlı kopya satışı başarısız
copy-notice-title-auto-paused = Kopya görevi otomatik duraklatıldı
copy-notice-detail-bought = { $amount } { -sol } karşılığında alındı
copy-notice-detail-sold = { $amount } { -sol } karşılığında satıldı
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = Varlık payı: %{ $percent }
copy-notice-detail-full-close = Tam kapanış
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = Takas başarısız
