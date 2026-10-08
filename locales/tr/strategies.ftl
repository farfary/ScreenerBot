strategies-filter-all = Tümü
strategies-filter-entry = Giriş
strategies-filter-exit = Çıkış
strategies-type-entry = Giriş
strategies-type-exit = Çıkış
strategies-list-empty-title = Henüz strateji yok
strategies-list-empty-hint = İlk stratejinizi oluşturun
strategies-new = Yeni strateji
strategies-import =
    .title = Stratejiyi içe aktar
    .aria-label = Stratejiyi içe aktar
strategies-item-enable =
    .title = Etkinleştir
strategies-item-disable =
    .title = Devre dışı bırak

strategies-new-name = Yeni strateji

strategies-editor-name =
    .placeholder = Strateji adı
strategies-editor-dirty =
    .title = Kaydedilmemiş değişiklikler
strategies-editor-enabled =
    .aria-label = Strateji etkin
    .title = Strateji etkin
strategies-action-validate = Doğrula
strategies-editor-empty = Düzenlemek için bir strateji seçin veya yeni bir tane oluşturun
strategies-conditions-empty-title = Henüz koşul yok
strategies-conditions-empty-hint = Oluşturmaya başlamak için "{ strategies-add-condition }" seçeneğini kullanın
strategies-add-condition = Koşul ekle
strategies-modal-close =
    .aria-label = Kapat
strategies-card-move-up =
    .title = Yukarı taşı
strategies-card-move-down =
    .title = Aşağı taşı
strategies-card-duplicate =
    .title = Çoğalt
strategies-card-delete =
    .title = Sil
# $name is the condition name.
strategies-card-delete-confirm = Koşulu kaldır
    .message = "{ $name }" bu stratejiden kaldırılsın mı?

strategies-summary-param = { $label }: { $value }
strategies-summary-none = Parametre yok
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = Strateji ayarı ({ $value })
strategies-summary-period-seconds = Periyot: { $amount }sn
strategies-summary-period-minutes = Periyot: { $amount }dk
strategies-summary-period-hours = Periyot: { $amount }sa

strategies-value-percent = %{ $amount }
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
        [one] { $amount } saat
       *[other] { $amount } saat
    }
strategies-value-candles =
    { $count ->
        [one] { $amount } mum
       *[other] { $amount } mum
    }

strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = sa
strategies-unit-multiplier = ×

strategies-catalog-search =
    .placeholder = Koşullarda ara...
strategies-catalog-search-clear =
    .aria-label = Aramayı temizle
strategies-catalog-fold-all = Tümünü daralt
strategies-catalog-unfold-all = Tümünü genişlet
strategies-catalog-no-description = Açıklama yok

strategies-create-title = Yeni strateji oluştur
strategies-create-prompt = Oluşturmak istediğiniz strateji türünü seçin:
strategies-create-entry-name = Giriş stratejisi
strategies-create-entry-description = Bir tokenın ne zaman alınacağına dair koşulları belirleyin
strategies-create-exit-name = Çıkış stratejisi
strategies-create-exit-description = Bir tokenın ne zaman satılacağına dair koşulları belirleyin

strategies-delete-title = Stratejiyi sil
strategies-delete-message = "{ $name }" stratejisi silinsin mi? Bu işlem geri alınamaz.

strategies-toast-fix-validation = Kaydetmeden önce lütfen doğrulama hatalarını düzeltin
strategies-toast-enabled = Strateji etkinleştirildi
    .message = "{ $name }" etkinleştirildi
strategies-toast-disabled = Strateji devre dışı bırakıldı
    .message = "{ $name }" devre dışı bırakıldı
strategies-toast-toggle-failed = Değiştirme başarısız
    .message = Strateji durumu güncellenemedi
strategies-toast-load-failed = Yükleme başarısız
    .message = Stratejiler sunucudan yüklenemedi
strategies-toast-load-strategy-failed = Strateji yüklenemedi
strategies-toast-no-strategy = Strateji oluşturulmadı
    .message = En az bir koşul ekleyin veya önce bir strateji oluşturmak için "Yeni strateji"ye tıklayın
strategies-toast-no-conditions-save = Koşul yok
    .message = Kaydetmeden önce stratejiye en az bir koşul ekleyin
strategies-toast-name-required = Ad gerekli
    .message = Kaydetmeden önce bir strateji adı girin
strategies-toast-saved = Strateji kaydedildi
    .message = "{ $name }" başarıyla kaydedildi
strategies-toast-save-failed = Kaydetme başarısız
    .message = Strateji veritabanına kaydedilemedi
strategies-toast-no-strategy-validate = Doğrulanacak strateji yok
strategies-toast-no-conditions-validate = Koşul yok
    .message = Doğrulamadan önce en az bir koşul ekleyin
strategies-toast-valid = Strateji geçerli
strategies-toast-invalid = Stratejide hatalar var
strategies-toast-validation-failed = Doğrulama başarısız
strategies-toast-item-enabled = Strateji etkinleştirildi
strategies-toast-item-disabled = Strateji devre dışı bırakıldı
strategies-toast-item-toggle-failed = Strateji değiştirilemedi
strategies-toast-deleted = Strateji silindi
    .message = "{ $name }" başarıyla kaldırıldı
strategies-toast-delete-failed = Silme başarısız
    .message = Strateji veritabanından silinemedi
strategies-toast-imported = Strateji içe aktarıldı
strategies-toast-import-failed = Strateji içe aktarılamadı
strategies-toast-unknown-condition = Bilinmeyen koşul
    .message = Koşul türü bulunamadı
strategies-toast-create-first = Önce strateji oluşturun
    .message = Koşul eklemeden önce bir strateji oluşturmak için "Yeni strateji"ye tıklayın
strategies-toast-condition-added = Koşul eklendi
    .message = Stratejiye eklendi: { $name }

strategies-condition-candle-size = Mum boyutu deseni
    .description = Belirli mum desenlerini algılar: büyük gövde, küçük gövde (doji), uzun fitiller
strategies-condition-candle-size-param-pattern = Desen türü
    .description = Algılanacak mum deseni
strategies-condition-candle-size-param-pattern-option-large-body = Büyük gövde (güçlü hareket)
strategies-condition-candle-size-param-pattern-option-small-body = Küçük gövde (doji/kararsızlık)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Uzun üst fitil (reddedilme)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Uzun alt fitil (destek)
strategies-condition-candle-size-param-threshold = Boyut eşiği %
    .description = Desen algılama için yüzde eşiği

strategies-condition-consecutive-candles = Ardışık mumlar
    .description = Minimum boyut filtresiyle ardışık yeşil (yükseliş) veya kırmızı (düşüş) mumları algılar
strategies-condition-consecutive-candles-param-count = Mum sayısı
    .description = Gereken ardışık mum sayısı
strategies-condition-consecutive-candles-param-direction = Mum yönü
    .description = Ardışık mumların rengi/yönü
strategies-condition-consecutive-candles-param-direction-option-green = Yeşil (yükseliş)
strategies-condition-consecutive-candles-param-direction-option-red = Kırmızı (düşüş)
strategies-condition-consecutive-candles-param-minimum-change = Minimum değişim %
    .description = Her mum için minimum % değişim (gürültüyü filtreler)

strategies-condition-liquidity-level = Havuz likidite seviyesi
    .description = Havuz likiditesini { -sol } cinsinden kontrol eder (Giriş: yeterli likiditeyi sağla, Çıkış: likidite çekilmesini algıla)
strategies-condition-liquidity-level-param-threshold = Likidite eşiği ({ -sol })
    .description = { -sol } cinsinden havuz likidite seviyesi
strategies-condition-liquidity-level-param-comparison = Karşılaştırma
    .description = Havuz likiditesinin eşikle nasıl karşılaştırılacağı
strategies-condition-liquidity-level-param-comparison-option-greater-than = Büyüktür (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = Büyük veya eşittir (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = Küçüktür ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = Küçük veya eşittir (≤)

strategies-condition-position-holding-time = Pozisyon tutma süresi
    .description = Bir pozisyonun ne kadar süredir tutulduğunu kontrol eder (çıkış stratejileri için - zamana dayalı çıkışlar)
strategies-condition-position-holding-time-param-hours = Zaman eşiği (saat)
    .description = Pozisyon açıldığından beri geçen süre (saat)
strategies-condition-position-holding-time-param-comparison = Karşılaştırma
    .description = Pozisyon yaşının eşikle nasıl karşılaştırılacağı
strategies-condition-position-holding-time-param-comparison-option-greater-than = Daha eski (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = En az (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Daha yeni ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = En fazla (≤)

strategies-condition-price-breakout = Fiyat kırılımı
    .description = Fiyatın direncin (dönem zirvesi) üstüne veya desteğin (dönem dibi) altına kırılmasını algılar
strategies-condition-price-breakout-param-lookback = Geriye bakış dönemi
    .description = Destek/direnç seviyesini bulmak için mum sayısı
strategies-condition-price-breakout-param-direction = Kırılım yönü
    .description = Kırılımın yönü
strategies-condition-price-breakout-param-direction-option-upward = Yukarı (direnç kırılımı)
strategies-condition-price-breakout-param-direction-option-downward = Aşağı (destek kırılımı)
strategies-condition-price-breakout-param-confirmation = Onay %
    .description = Kırılımı onaylamak için seviyenin ne kadar ötesine geçileceği (yanlış sinyalleri önler)

strategies-condition-price-change-percent = Fiyat değişimi %
    .description = Fiyatın bir zaman diliminde yüzde eşiği kadar değişip değişmediğini kontrol eder
strategies-condition-price-change-percent-param-percentage = Değişim eşiği %
    .description = Tetiklenecek yüzde fiyat değişimi (0,1-1000%)
strategies-condition-price-change-percent-param-direction = Yön
    .description = Fiyat hareketi yönü
strategies-condition-price-change-percent-param-direction-option-above = Artış (+%)
strategies-condition-price-change-percent-param-direction-option-below = Kayıp (-%)
strategies-condition-price-change-percent-param-direction-option-within = Aralık içinde (±%)
strategies-condition-price-change-percent-param-time-value = Zaman dilimi
    .description = Geriye bakış dönemi değeri (saniye için 1-3600, dakika için 1-1440, saat için 1-720)
strategies-condition-price-change-percent-param-time-unit = Zaman birimi
    .description = Geriye bakış dönemi için zaman birimi
strategies-condition-price-change-percent-param-time-unit-option-seconds = Saniye
strategies-condition-price-change-percent-param-time-unit-option-minutes = Dakika
strategies-condition-price-change-percent-param-time-unit-option-hours = Saat

strategies-condition-price-to-ma = Fiyat ve hareketli ortalama
    .description = Fiyatın Basit Hareketli Ortalamasının üstünde, altında veya aralığında olup olmadığını kontrol eder
strategies-condition-price-to-ma-param-period = HO dönemi
    .description = Hareketli ortalama hesabı için mum sayısı
strategies-condition-price-to-ma-param-position = Konum
    .description = Fiyatın HO'ya göre konumu
strategies-condition-price-to-ma-param-position-option-above = HO üstünde
strategies-condition-price-to-ma-param-position-option-below = HO altında
strategies-condition-price-to-ma-param-position-option-within = Aralık içinde
strategies-condition-price-to-ma-param-distance = Uzaklık %
    .description = HO'dan minimum uzaklık (ÜSTÜNDE/ALTINDA için) veya maksimum aralık (ARALIK İÇİNDE için)

strategies-condition-volume-spike = Hacim sıçraması
    .description = Ortalama hacme kıyasla hacim sıçramalarını algılar (artan ilgiyi gösterir)
strategies-condition-volume-spike-param-lookback = Geriye bakış dönemi
    .description = Ortalama hacmi hesaplamak için mum sayısı
strategies-condition-volume-spike-param-multiplier = Hacim çarpanı
    .description = Ortalamanın kaç katı üstünde (örn. 2,0 = ortalamanın %200'ü)

strategies-condition-param-timeframe = Zaman dilimi
    .description = Analiz edilecek mum zaman dilimi (ayarlanmazsa strateji zaman dilimi kullanılır)
strategies-condition-timeframe-option-1m = 1 dakika
strategies-condition-timeframe-option-5m = 5 dakika
strategies-condition-timeframe-option-15m = 15 dakika
strategies-condition-timeframe-option-1h = 1 saat
strategies-condition-timeframe-option-4h = 4 saat
strategies-condition-timeframe-option-12h = 12 saat
strategies-condition-timeframe-option-1d = 1 gün

strategies-condition-category-price-analysis = Fiyat analizi
strategies-condition-category-candle-patterns = Mum desenleri
strategies-condition-category-technical-indicators = Teknik göstergeler
strategies-condition-category-market-context = Piyasa bağlamı
strategies-condition-category-position-performance = Pozisyon ve performans
strategies-condition-category-volume-analysis = Hacim analizi

strategies-error-missing-parameter = { $field } parametresi eksik
strategies-error-parameter-type = { $field } parametresi { $expected } olmalıdır
strategies-error-invalid-value = "{ $value }" geçerli bir { $field } değil
strategies-error-missing-data = { $data } mevcut değil
strategies-error-no-candle-data = { $timeframe } zaman diliminde mum verisi yok
strategies-error-insufficient-history = { $indicator } için yeterli geçmiş yok: mevcut { $available }sn, gereken { $required }sn
strategies-error-insufficient-candles = { $indicator } için yeterli mum yok: mevcut { $available }, gereken { $required }
strategies-error-stale-candle-data = { $timeframe } mum verisi eski: yaş { $age }sn, sınır { $max }sn
strategies-error-invalid-rule-tree = Geçersiz kural ağacı: { $reason }
strategies-error-evaluation-timeout = Strateji değerlendirmesi { $timeout }ms sonra zaman aşımına uğradı
strategies-error-invalid-rules = Kurallar okunamadı: { $reason }

strategies-error-field-average-volume = ortalama hacim
strategies-error-field-candle-open = mum açılışı
strategies-error-field-comparison = karşılaştırma
strategies-error-field-condition-type = koşul türü
strategies-error-field-confirmation = onay
strategies-error-field-count = sayı
strategies-error-field-current-price = güncel fiyat
strategies-error-field-direction = yön
strategies-error-field-distance = uzaklık
strategies-error-field-hours = saat
strategies-error-field-lookback = geriye bakış
strategies-error-field-minimum-change = minimum değişim
strategies-error-field-multiplier = çarpan
strategies-error-field-pattern = desen
strategies-error-field-percentage = yüzde
strategies-error-field-period = dönem
strategies-error-field-position = konum
strategies-error-field-threshold = eşik
strategies-error-field-time-unit = zaman birimi
strategies-error-field-time-value = zaman değeri
strategies-error-field-timeframe = zaman dilimi

strategies-error-expected-boolean = bir mantıksal değer
strategies-error-expected-number = bir sayı
strategies-error-expected-string = bir metin

strategies-error-data-current-price = Güncel fiyat
strategies-error-data-liquidity-data = Likidite verisi
strategies-error-data-market-data = Piyasa verisi
strategies-error-data-ohlcv-data = OHLCV verisi
strategies-error-data-position-data = Pozisyon verisi

strategies-error-indicator-consecutive-candles = ardışık mumlar
strategies-error-indicator-moving-average = hareketli ortalama
strategies-error-indicator-price-breakout = fiyat kırılımı
strategies-error-indicator-price-change-lookback = fiyat değişimi geriye bakışı
strategies-error-indicator-volume-spike = hacim sıçraması

strategies-error-rule-branch-node-missing-conditions = Dal düğümünde koşullar eksik
strategies-error-rule-branch-node-missing-operator = Dal düğümünde operatör eksik
strategies-error-rule-branch-node-must-have-at-least-one-child = Dal düğümünün en az bir alt öğesi olmalıdır
strategies-error-rule-invalid-rule-tree-structure = Geçersiz kural ağacı yapısı
strategies-error-rule-leaf-node-missing-condition = Yaprak düğümünde koşul eksik
strategies-error-rule-not-operator-must-have-exactly-one-child = NOT operatörünün tam olarak bir alt öğesi olmalıdır
