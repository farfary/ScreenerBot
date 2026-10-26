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
strategies-summary-period-seconds = Geriye bakış: { $amount }sn
strategies-summary-period-minutes = Geriye bakış: { $amount }dk
strategies-summary-period-hours = Geriye bakış: { $amount }sa

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
    .aria-label = Koşullarda ara
strategies-catalog-search-clear =
    .aria-label = Aramayı temizle
strategies-catalog-no-description = Açıklama yok

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = Etkin
strategies-editor-timeframe = Varsayılan zaman dilimi
    .title = Kendi zaman dilimini belirlemeyen koşullar için mum zaman dilimi
strategies-editor-priority = Öncelik
    .title = 1 ile 100 arasında değerlendirme sırası; küçük sayılar önce değerlendirilir
strategies-editor-description = Açıklama
strategies-editor-description-field =
    .placeholder = Bu stratejinin ne yaptığı
    .aria-label = Strateji açıklaması
strategies-action-export = Dışa aktar

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = Bu koşulların { $mode } sağlanmalı
strategies-group-match = Bunların { $mode } sağlanmalı
strategies-match-all = tümü
strategies-match-any = en az biri
strategies-match-mode =
    .aria-label = Tümü veya en az bir koşul
strategies-invert = Ters çevir
    .title = Ters çevir: bu eşleşmediğinde doğru olur
strategies-chip-off = Kapalı
strategies-chip-inverted = Ters çevrilmiş
strategies-row-switch =
    .title = Kayıtlı kalsın ama değerlendirilmesin diye kapatın
    .aria-label = Bu strateji tarafından değerlendirilir
strategies-row-expand =
    .aria-label = Parametreleri göster veya gizle
strategies-add-group = Grup ekle
# $count is the number of conditions in the group.
strategies-group-delete-confirm = Grubu kaldır
    .message =
        { $count ->
            [one] Bu grup ve içindeki koşul kaldırılsın mı?
           *[other] Bu grup ve içindeki { $count } koşul kaldırılsın mı?
        }

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems =
    { $count ->
        [one] { $count } sorun düzeltilmeli: { $first }
       *[other] { $count } sorun düzeltilmeli: { $first }
    }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }, { $field }: { $problem }
strategies-problem-rules-empty = En az bir açık koşul ekleyin
strategies-problem-group-empty = Bir grubun en az bir açık koşula ihtiyacı var
strategies-problem-not-applicable = Bu koşul bu strateji türünde kullanılamaz
strategies-problem-unknown-condition = Bilinmeyen koşul türü: { $type }
strategies-problem-too-deep = Kurallar en fazla { $max } düzey iç içe geçebilir; ters çevrilmiş bir grup bir düzey daha sayılır
strategies-problem-required = Bir sayı girin
strategies-problem-range = { $min } ile { $max } arasında olmalı
strategies-problem-invalid-option = Listelenen seçeneklerden birini seçin

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = Tümü
strategies-catalog-categories =
    .aria-label = Kategoriler
strategies-catalog-empty = Aramanızla eşleşen koşul yok
strategies-catalog-exit-only = Yalnızca çıkış stratejileri
strategies-catalog-entry-only = Yalnızca giriş stratejileri
strategies-data-candles = Mumlar
strategies-data-pool = Havuz
strategies-data-position = Pozisyon
strategies-data-market = Piyasa etkinliği

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = işlem
strategies-value-trades =
    { $count ->
        [one] { $amount } işlem
       *[other] { $amount } işlem
    }

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = Çok fazla düzey
    .message = Kurallar en fazla { $max } düzey iç içe geçebilir
strategies-toast-priority-invalid = Geçersiz öncelik
    .message = 1 ile 100 arasında bir tam sayı girin
strategies-toast-exported = Strateji dışa aktarıldı
# $name is the strategy name.
strategies-unsaved-confirm = Kaydedilmemiş değişiklikler silinsin mi?
    .message = "{ $name }" kaydedilmemiş değişiklikler içeriyor.
strategies-unsaved-discard = Değişiklikleri sil
strategies-import-error-json = Dosya geçerli bir JSON değil
strategies-import-error-shape = Dosya bir strateji tanımlamıyor
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = Dosya bu sürümde bulunmayan bir koşul kullanıyor: { $type }
strategies-import-error-not-applicable = { $name } bu strateji türünde kullanılamaz

strategies-create-title = Yeni strateji oluştur
strategies-create-prompt = Oluşturmak istediğiniz strateji türünü seçin:
strategies-create-entry-name = Giriş stratejisi
strategies-create-entry-description = Bir tokenın ne zaman alınacağına dair koşulları belirleyin
strategies-create-exit-name = Çıkış stratejisi
strategies-create-exit-description = Bir tokenın ne zaman satılacağına dair koşulları belirleyin

strategies-delete-title = Stratejiyi sil
strategies-delete-message = "{ $name }" stratejisi silinsin mi? Bu işlem geri alınamaz.

strategies-toast-fix-validation = Kaydetmeden önce lütfen doğrulama hatalarını düzeltin
strategies-toast-load-failed = Yükleme başarısız
    .message = Stratejiler sunucudan yüklenemedi
strategies-toast-load-strategy-failed = Strateji yüklenemedi
strategies-toast-no-strategy = Strateji oluşturulmadı
    .message = En az bir koşul ekleyin veya önce bir strateji oluşturmak için "Yeni strateji"ye tıklayın
strategies-toast-name-required = Ad gerekli
    .message = Kaydetmeden önce bir strateji adı girin
strategies-toast-saved = Strateji kaydedildi
    .message = "{ $name }" başarıyla kaydedildi
strategies-toast-save-failed = Kaydetme başarısız
    .message = Strateji veritabanına kaydedilemedi
strategies-toast-no-strategy-validate = Doğrulanacak strateji yok
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
strategies-condition-price-change-percent-param-time-value = Geriye bakış
    .description = Değişimin ne kadar geriden ölçüldüğü, alanın içinde seçilen birimle (1-3600 sn, 1-1440 dk, 1-720 sa)
strategies-condition-price-change-percent-param-time-unit = Zaman birimi
    .description = Geriye bakış dönemi için zaman birimi
strategies-condition-price-change-percent-param-time-unit-option-seconds = sn
strategies-condition-price-change-percent-param-time-unit-option-minutes = dk
strategies-condition-price-change-percent-param-time-unit-option-hours = sa

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

strategies-condition-rsi = RSI (Göreceli Güç Endeksi)
    .description = Kapanmış mumlarda Wilder yumuşatmasıyla hesaplanan RSI bir eşiğin üstünde veya altında olduğunda tetiklenir.
strategies-condition-rsi-param-period = RSI dönemi
    .description = RSI'nin hesaplandığı mum sayısı; koşul bu değerden bir mum fazlasına ihtiyaç duyar
strategies-condition-rsi-param-comparison = Karşılaştırma
    .description = RSI'nin eşiğin altında mı üstünde mi olması gerektiği
strategies-condition-rsi-param-comparison-option-below = Altında (aşırı satım)
strategies-condition-rsi-param-comparison-option-above = Üstünde (aşırı alım)
strategies-condition-rsi-param-threshold = RSI eşiği
    .description = 1 ile 99 arasında RSI seviyesi (aşırı satım için 30, aşırı alım için 70 yaygındır)

strategies-condition-ma-cross = Hareketli ortalama kesişimi
    .description = Hızlı bir hareketli ortalama son birkaç mum içinde yavaş olanı kestiğinde tetiklenir. Yalnızca mumları okur.
strategies-condition-ma-cross-param-ma-type = Ortalama türü
    .description = Üstel ortalamalar yakın tarihli mumlara daha çok ağırlık verir; basit ortalamalar tüm mumlara eşit ağırlık verir
strategies-condition-ma-cross-param-ma-type-option-ema = Üstel (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = Basit (SMA)
strategies-condition-ma-cross-param-fast-period = Hızlı dönem
    .description = Hızlı ortalamadaki mum sayısı
strategies-condition-ma-cross-param-slow-period = Yavaş dönem
    .description = Yavaş ortalamadaki mum sayısı; hızlı dönemden büyük olmalıdır
strategies-condition-ma-cross-param-direction = Kesişim yönü
    .description = Hızlı ortalamanın yavaş olanı hangi yönde kestiği
strategies-condition-ma-cross-param-direction-option-bullish = Yükseliş (hızlı yukarı keser)
strategies-condition-ma-cross-param-direction-option-bearish = Düşüş (hızlı aşağı keser)
strategies-condition-ma-cross-param-within-candles = Kesişim son mumlar içinde
    .description = Kesişimin kaç son mumda bulunabileceği; 1 yalnızca son mum demektir

strategies-condition-position-profit = Pozisyon kârı
    .description = Açık pozisyonun gerçekleşmemiş kârı giriş fiyatının belirli bir yüzdesinin üstünde veya altında olduğunda tetiklenir; havuz fiyatıyla ölçülür. Yalnızca çıkış stratejileri için.
strategies-condition-position-profit-param-comparison = Karşılaştırma
    .description = Kârın eşiğe en az mı en fazla mı eşit olması gerektiği
strategies-condition-position-profit-param-comparison-option-above = En az (≥)
strategies-condition-position-profit-param-comparison-option-below = En fazla (≤)
strategies-condition-position-profit-param-threshold-pct = Kâr eşiği %
    .description = Giriş fiyatının yüzdesi olarak gerçekleşmemiş kâr; negatif değerler zarardır

strategies-condition-drawdown-from-peak = Zirveden düşüş
    .description = Havuz fiyatı, girişten bu yana görülen en yüksek fiyattan bir yüzde kadar düştüğünde ve bu zirve giriş fiyatının yeterince üzerinde olduğunda tetiklenir. Yalnızca havuz fiyatlarını okur. Yalnızca çıkış stratejileri için.
strategies-condition-drawdown-from-peak-param-drop-pct = Zirveden düşüş %
    .description = Güncel fiyatın zirvenin ne kadar altında olması gerektiği
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = Kârdan sonra devreye al %
    .description = Koşulun tetiklenebilmesi için zirvenin giriş fiyatının en az bu kadar üzerine çıkmış olması gerekir; 0 her zaman etkin demektir

strategies-condition-buy-pressure = Alım baskısı
    .description = Alımlar bir zaman penceresindeki işlemlerin en az belirli bir payını oluşturduğunda ve pencerede yeterli işlem bulunduğunda tetiklenir. Tokenin alım ve satım sayılarını okur.
strategies-condition-buy-pressure-param-window = Zaman penceresi
    .description = Alım ve satım sayılarının kapsadığı dönem
strategies-condition-buy-pressure-param-window-option-5m = 5 dakika
strategies-condition-buy-pressure-param-window-option-1h = 1 saat
strategies-condition-buy-pressure-param-window-option-6h = 6 saat
strategies-condition-buy-pressure-param-window-option-24h = 24 saat
strategies-condition-buy-pressure-param-min-buy-share-pct = Asgari alım payı %
    .description = Alımların penceredeki tüm işlemlere yüzdesi
strategies-condition-buy-pressure-param-min-transactions = Asgari işlem sayısı
    .description = Daha az işlemi olan pencereler koşulu asla tetiklemez

strategies-condition-param-timeframe = Zaman dilimi
    .description = Mum boyutu: koşulun okuduğu her mumun süresi (ayarlanmazsa strateji zaman dilimi)
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
strategies-error-condition-not-applicable = { $condition }, { $strategy } stratejilerinde kullanılamaz
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
strategies-error-field-arm-after-profit = kârdan sonra devreye alma
strategies-error-field-drop-percentage = düşüş yüzdesi
strategies-error-field-entry-price = giriş fiyatı
strategies-error-field-fast-period = hızlı dönem
strategies-error-field-minimum-buy-share = asgari alım payı
strategies-error-field-minimum-transactions = asgari işlem sayısı
strategies-error-field-moving-average-type = ortalama türü
strategies-error-field-peak-price = zirve fiyatı
strategies-error-field-slow-period = yavaş dönem
strategies-error-field-window = pencere
strategies-error-field-within-candles = kesişim mum sayısı

strategies-error-expected-boolean = bir mantıksal değer
strategies-error-expected-number = bir sayı
strategies-error-expected-string = bir metin

strategies-error-data-current-price = Güncel fiyat
strategies-error-data-liquidity-data = Likidite verisi
strategies-error-data-market-data = Piyasa verisi
strategies-error-data-ohlcv-data = OHLCV verisi
strategies-error-data-position-data = Pozisyon verisi
strategies-error-data-peak-price = Zirve fiyatı
strategies-error-data-transaction-counts = İşlem sayıları

strategies-error-indicator-consecutive-candles = ardışık mumlar
strategies-error-indicator-moving-average = hareketli ortalama
strategies-error-indicator-price-breakout = fiyat kırılımı
strategies-error-indicator-price-change-lookback = fiyat değişimi geriye bakışı
strategies-error-indicator-volume-spike = hacim sıçraması
strategies-error-indicator-moving-average-cross = hareketli ortalama kesişimi
strategies-error-indicator-rsi = RSI

strategies-error-rule-branch-node-missing-conditions = Dal düğümünde koşullar eksik
strategies-error-rule-branch-node-missing-operator = Dal düğümünde operatör eksik
strategies-error-rule-branch-node-must-have-at-least-one-child = Dal düğümünün en az bir alt öğesi olmalıdır
strategies-error-rule-invalid-rule-tree-structure = Geçersiz kural ağacı yapısı
strategies-error-rule-leaf-node-missing-condition = Yaprak düğümünde koşul eksik
strategies-error-rule-not-operator-must-have-exactly-one-child = NOT operatörünün tam olarak bir alt öğesi olmalıdır
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = Bir grupta en az bir koşul açık olmalıdır
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = NOT operatörünün kapalı bir alt düğümü olamaz
strategies-error-rule-rules-nest-deeper-than-four-levels = Kurallar dört seviyeden daha derin iç içe geçemez
strategies-error-rule-the-root-group-cannot-be-switched-off = Kök grup kapatılamaz
