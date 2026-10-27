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
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount }sn
strategies-value-duration-minutes = { $amount }dk
strategies-value-duration-hours = { $amount }sa
strategies-value-duration-days = { $amount }g
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = Yerel saatiniz: { $time }
strategies-hint-price-impact = Bu büyüklükte bir işlem fiyatı yaklaşık %{ $amount } oynatır

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
strategies-unit-utc = UTC

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
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = Tüm giriş stratejileri
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = Yönetir
strategies-editor-entry-binding-hint = İşaretli giriş stratejilerinin açtığı pozisyonları yönetir. Hiçbiri işaretli değilse manuel ve kopyalananlar dahil her pozisyonu yönetir.
strategies-editor-entry-binding-empty = Henüz giriş stratejisi yok
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id } (silindi)
strategies-action-export = Dışa aktar
strategies-action-test = Bir tokenla test et
# $token is the tested token's symbol or address.
strategies-test-title = { $token } ile test
# $token is the tested token's symbol or address.
strategies-test-running = { $token } ile test ediliyor…
strategies-test-verdict-pass = Sağlanıyor
strategies-test-verdict-fail = Sağlanmıyor
strategies-test-verdict-unknown = Bilinmiyor

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
strategies-catalog-candles-note = { $description } Yalnızca mumları okur.
strategies-data-candles = Mumlar
strategies-data-pool = Havuz
strategies-data-position = Pozisyon
strategies-data-market = Piyasa etkinliği
strategies-data-clock = Günün saati

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
strategies-toast-no-strategy-test = Test edilecek strateji yok
strategies-toast-test-save-first = Test etmeden önce stratejiyi kaydedin: test kayıtlı kuralları okur
strategies-toast-test-failed = Test başarısız
    .message = Strateji test edilemedi
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
    .description = Belirli mum desenlerini algılar: büyük gövde, küçük gövde (doji), uzun fitiller.
strategies-condition-candle-size-param-pattern = Desen türü
    .description = Algılanacak mum deseni
strategies-condition-candle-size-param-pattern-option-large-body = Büyük gövde (güçlü hareket)
strategies-condition-candle-size-param-pattern-option-small-body = Küçük gövde (doji/kararsızlık)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Uzun üst fitil (reddedilme)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Uzun alt fitil (destek)
strategies-condition-candle-size-param-threshold = Boyut eşiği %
    .description = Desen algılama için yüzde eşiği

strategies-condition-consecutive-candles = Ardışık mumlar
    .description = Minimum boyut filtresiyle ardışık yeşil (yükseliş) veya kırmızı (düşüş) mumları algılar.
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
    .description = Bir pozisyonun ne kadar süredir tutulduğunu kontrol eder
strategies-condition-position-holding-time-param-hours = Zaman eşiği (saat)
    .description = Pozisyon açıldığından beri geçen süre (saat)
strategies-condition-position-holding-time-param-comparison = Karşılaştırma
    .description = Pozisyon yaşının eşikle nasıl karşılaştırılacağı
strategies-condition-position-holding-time-param-comparison-option-greater-than = Daha eski (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = En az (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Daha yeni ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = En fazla (≤)

strategies-condition-price-breakout = Fiyat kırılımı
    .description = Fiyatın direncin (dönem zirvesi) üstüne veya desteğin (dönem dibi) altına kırılmasını algılar.
strategies-condition-price-breakout-param-lookback = Geriye bakış dönemi
    .description = Destek/direnç seviyesini bulmak için mum sayısı
strategies-condition-price-breakout-param-direction = Kırılım yönü
    .description = Kırılımın yönü
strategies-condition-price-breakout-param-direction-option-upward = Yukarı (direnç kırılımı)
strategies-condition-price-breakout-param-direction-option-downward = Aşağı (destek kırılımı)
strategies-condition-price-breakout-param-confirmation = Onay %
    .description = Kırılımı onaylamak için seviyenin ne kadar ötesine geçileceği (yanlış sinyalleri önler)

strategies-condition-price-change-percent = Fiyat değişimi %
    .description = Fiyatın bir zaman diliminde yüzde eşiği kadar değişip değişmediğini kontrol eder.
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
    .description = Fiyatın Basit Hareketli Ortalamasının üstünde, altında veya aralığında olup olmadığını kontrol eder.
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
    .description = Ortalama hacme kıyasla hacim sıçramalarını algılar (artan ilgiyi gösterir).
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
    .description = Hızlı bir hareketli ortalama son birkaç mum içinde yavaş olanı kestiğinde tetiklenir.
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
    .description = Açık pozisyonun gerçekleşmemiş kârı giriş fiyatının belirli bir yüzdesinin üstünde veya altında olduğunda tetiklenir; havuz fiyatıyla ölçülür.
strategies-condition-position-profit-param-comparison = Karşılaştırma
    .description = Kârın eşiğe en az mı en fazla mı eşit olması gerektiği
strategies-condition-position-profit-param-comparison-option-above = En az (≥)
strategies-condition-position-profit-param-comparison-option-below = En fazla (≤)
strategies-condition-position-profit-param-threshold-pct = Kâr eşiği %
    .description = Giriş fiyatının yüzdesi olarak gerçekleşmemiş kâr; negatif değerler zarardır

strategies-condition-drawdown-from-peak = Zirveden düşüş
    .description = Havuz fiyatı, son alımdan bu yana görülen en yüksek fiyattan bir yüzde kadar düştüğünde ve bu zirve giriş fiyatının yeterince üzerinde olduğunda tetiklenir.
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

strategies-condition-transaction-velocity = İşlem Hızı
    .description = Yakın bir penceredeki işlem hızı, onu içeren daha uzun pencerenin hızının en az veya en fazla bir katı olduğunda tetiklenir. Token'ın alım ve satım sayılarını okur.
strategies-condition-transaction-velocity-param-window = Pencereler
    .description = Yakın pencere ve karşılaştırıldığı daha uzun pencere
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = 5 Dakika / 1 Saat
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = 1 Saat / 6 Saat
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = 6 Saat / 24 Saat
strategies-condition-transaction-velocity-param-comparison = Karşılaştırma
    .description = Yakın hızın oranın en az mı en fazla mı olması gerektiği
strategies-condition-transaction-velocity-param-comparison-option-above = En Az (≥)
strategies-condition-transaction-velocity-param-comparison-option-below = En Fazla (≤)
strategies-condition-transaction-velocity-param-ratio = Hız Oranı
    .description = Yakın penceredeki saniye başına işlem, uzun penceredeki saniye başına işleme bölünür
strategies-condition-transaction-velocity-param-min-transactions = En Az İşlem
    .description = Daha az işlemi olan yakın pencereler koşulu asla tetiklemez

strategies-condition-pullback-from-high = Tepeden Geri Çekilme
    .description = Canlı fiyat, son mumların en yüksek tepesinin altında en az ve en fazla düşüş arasında olduğunda tetiklenir.
strategies-condition-pullback-from-high-param-lookback = Geriye Bakış Süresi
    .description = En yüksek tepenin alındığı mum sayısı
strategies-condition-pullback-from-high-param-min-drop-pct = En Az Düşüş %
    .description = Fiyat tepenin en az bu kadar altında olmalıdır
strategies-condition-pullback-from-high-param-max-drop-pct = En Fazla Düşüş %
    .description = Fiyat tepenin en fazla bu kadar altında olmalıdır; en az düşüşten büyük olmalıdır

strategies-condition-profit-giveback = Kâr Geri Verme
    .description = Pozisyon, son alımdan bu yana en iyi gerçekleşmemiş kazancının bir kısmını, bu kazanç bir seviyeye ulaştıktan sonra geri verdiğinde tetiklenir.
strategies-condition-profit-giveback-param-arm-after-profit-pct = Kârdan Sonra Kur %
    .description = Koşulun tetiklenebilmesi için en iyi kazancın giriş fiyatının bu yüzdesine ulaşmış olması gerekir
strategies-condition-profit-giveback-param-giveback-pct = Geri Verme %
    .description = En iyi kazancın geri verilen payı; %100 fiyatın giriş fiyatına döndüğü anlamına gelir

strategies-condition-token-age = Token Yaşı
    .description = Token'ın zincir üstü oluşturulmasından bu yana geçen süre bir süreden uzun veya kısa olduğunda tetiklenir. Oluşturma zamanı bilinmeyen bir token koşulu belirsiz bırakır.
strategies-condition-token-age-param-comparison = Karşılaştırma
    .description = Token'ın süreden daha eski mi daha yeni mi olması gerektiği
strategies-condition-token-age-param-comparison-option-older-than = Daha Eski (≥)
strategies-condition-token-age-param-comparison-option-younger-than = Daha Yeni ({ "<" })
strategies-condition-token-age-param-time-value = Yaş
    .description = Token oluşturulduğundan beri geçen süre, alanın içinde seçilen birimde (1-10000)
strategies-condition-token-age-param-time-unit = Zaman Birimi
    .description = Yaşın birimi
strategies-condition-token-age-param-time-unit-option-minutes = dk
strategies-condition-token-age-param-time-unit-option-hours = sa
strategies-condition-token-age-param-time-unit-option-days = g

strategies-condition-vwap = VWAP
    .description = Canlı fiyat, son mumların hacim ağırlıklı ortalama fiyatının en az bir mesafe kadar üstünde veya altında olduğunda tetiklenir.
strategies-condition-vwap-param-lookback = Geriye Bakış Süresi
    .description = Hacim ağırlıklı ortalamanın kapsadığı mum sayısı
strategies-condition-vwap-param-position = Konum
    .description = Fiyatın VWAP'a göre konumu
strategies-condition-vwap-param-position-option-above = VWAP Üstünde
strategies-condition-vwap-param-position-option-below = VWAP Altında
strategies-condition-vwap-param-distance-pct = Mesafe %
    .description = VWAP'tan en az mesafe; 0 seçilen taraftaki her mesafe anlamına gelir

strategies-condition-volatility-range = Volatilite Aralığı
    .description = Son mumların ortalama gerçek aralığı, canlı fiyatın yüzdesi olarak bir eşiğin üstünde veya altında olduğunda tetiklenir. Bir grupta bunlardan ikisi bir aralık oluşturur.
strategies-condition-volatility-range-param-period = ATR Periyodu
    .description = Ortalama gerçek aralığın yumuşatıldığı mum sayısı; koşul bundan bir fazla mum gerektirir
strategies-condition-volatility-range-param-comparison = Karşılaştırma
    .description = Aralığın eşiğin üstünde mi altında mı olması gerektiği
strategies-condition-volatility-range-param-comparison-option-above = Üstünde
strategies-condition-volatility-range-param-comparison-option-below = Altında
strategies-condition-volatility-range-param-threshold-pct = Aralık Eşiği %
    .description = Canlı fiyatın yüzdesi olarak ortalama gerçek aralık

strategies-condition-bollinger-bands = Bollinger Bantları
    .description = Canlı fiyat alt bandın altında veya üst bandın üstünde olduğunda ya da bantlar sıkıştığında tetiklenir. Bantlar ortalama kapanışın birkaç standart sapma çevresindedir.
strategies-condition-bollinger-bands-param-period = Periyot
    .description = Ortalamanın ve standart sapmanın kapsadığı mum sayısı
strategies-condition-bollinger-bands-param-std-dev = Standart Sapmalar
    .description = Her bandın ortalamadan kaç standart sapma uzakta olduğu
strategies-condition-bollinger-bands-param-signal = Sinyal
    .description = Bantların göstermesi gereken
strategies-condition-bollinger-bands-param-signal-option-below-lower = Alt Bandın Altında
strategies-condition-bollinger-bands-param-signal-option-above-upper = Üst Bandın Üstünde
strategies-condition-bollinger-bands-param-signal-option-squeeze = Sıkışma
strategies-condition-bollinger-bands-param-squeeze-width-pct = Sıkışma Genişliği %
    .description = Sıkışma için: bantlar arasındaki en fazla mesafe, ortalamanın yüzdesi olarak

strategies-condition-liquidity-to-size = Büyüklüğe Göre Likidite
    .description = Havuz rezervleri, söz konusu büyüklüğün en az veya altında bir katı olduğunda tetiklenir: giriş stratejisinde işlem büyüklüğü, çıkış stratejisinde pozisyonun kalan değeri.
strategies-condition-liquidity-to-size-param-comparison = Karşılaştırma
    .description = Rezervlerin katın en az mı altında mı olması gerektiği
strategies-condition-liquidity-to-size-param-comparison-option-at-least = En Az (≥)
strategies-condition-liquidity-to-size-param-comparison-option-below = Altında ({ "<" })
strategies-condition-liquidity-to-size-param-multiple = Likidite Katı
    .description = Havuz rezervleri bölü söz konusu büyüklük; N katı fiyatı yaklaşık 1/(N+1) oynatır

strategies-condition-position-adds = Pozisyon Eklemeleri
    .description = İlk girişten sonra eklenen alımların veya kısmi satışların sayısı en az veya en fazla bir sayı olduğunda tetiklenir.
strategies-condition-position-adds-param-subject = Sayılan
    .description = Hangi işlemlerin sayıldığı
strategies-condition-position-adds-param-subject-option-adds = Eklenen Alımlar
strategies-condition-position-adds-param-subject-option-partial-sells = Kısmi Satışlar
strategies-condition-position-adds-param-comparison = Karşılaştırma
    .description = Sayının en az mı en fazla mı olması gerektiği
strategies-condition-position-adds-param-comparison-option-at-least = En Az (≥)
strategies-condition-position-adds-param-comparison-option-at-most = En Fazla (≤)
strategies-condition-position-adds-param-count = Sayı
    .description = İşlem sayısı

strategies-condition-time-of-day = Günün Saati
    .description = Geçerli saat bir UTC saat penceresi içindeyken tetiklenir. Bitişten sonraki bir başlangıç gece yarısını geçer.
strategies-condition-time-of-day-param-start-hour = Başlangıç Saati (UTC)
    .description = Pencerenin ilk saati, 0 ile 23 arası
strategies-condition-time-of-day-param-end-hour = Bitiş Saati (UTC)
    .description = Pencerenin bittiği saat, 1 ile 24 arası; o saatin kendisi pencerenin dışındadır

strategies-condition-liquidity-change-since-entry = Girişten Bu Yana Likidite Değişimi
    .description = Havuz rezervleri pozisyon açıldığından beri en az bir yüzde düştüğünde veya yükseldiğinde tetiklenir. Kayıtlı giriş likiditesi olmayan bir pozisyon koşulu belirsiz bırakır.
strategies-condition-liquidity-change-since-entry-param-comparison = Karşılaştırma
    .description = Rezervlerin düşmüş mü yükselmiş mi olması gerektiği
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = Düştü
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = Yükseldi
strategies-condition-liquidity-change-since-entry-param-change-pct = Değişim %
    .description = Girişten bu yana havuz rezervlerindeki değişim

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
strategies-error-stale-market-data = { $window } işlem sayıları eski: yaş { $age }sn, sınır { $max }sn
strategies-error-unusable-data = { $field } değeri "{ $value }" kullanılamaz
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
strategies-error-field-moving-average = hareketli ortalama
strategies-error-field-reference-candle-close = referans mum kapanışı
strategies-error-field-peak-price = zirve fiyatı
strategies-error-field-slow-period = yavaş dönem
strategies-error-field-window = pencere
strategies-error-field-within-candles = kesişim mum sayısı
strategies-error-field-change = değişim
strategies-error-field-end-hour = bitiş saati
strategies-error-field-entry-liquidity = giriş likiditesi
strategies-error-field-giveback = geri verme
strategies-error-field-highest-high = en yüksek tepe
strategies-error-field-long-window-transactions = uzun pencere işlemleri
strategies-error-field-maximum-drop = en fazla düşüş
strategies-error-field-minimum-drop = en az düşüş
strategies-error-field-multiple = kat
strategies-error-field-pool-liquidity = havuz likiditesi
strategies-error-field-ratio = oran
strategies-error-field-remaining-position-value = pozisyonun kalan değeri
strategies-error-field-signal = sinyal
strategies-error-field-squeeze-width = sıkışma genişliği
strategies-error-field-standard-deviations = standart sapmalar
strategies-error-field-start-hour = başlangıç saati
strategies-error-field-subject = sayılan
strategies-error-field-token-creation-time = token oluşturma zamanı
strategies-error-field-total-volume = toplam hacim
strategies-error-field-trade-size = işlem büyüklüğü
strategies-error-field-vwap = VWAP

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
strategies-error-data-entry-liquidity = Giriş likiditesi
strategies-error-data-entry-pool-liquidity = Giriş havuzunun likiditesi
strategies-error-data-real-pool-reserves = Havuzun gerçek likiditesi
strategies-error-data-remaining-position-value = Pozisyonun kalan değeri
strategies-error-data-token-creation-time = Token oluşturma zamanı
strategies-error-data-trade-size = İşlem büyüklüğü

strategies-error-indicator-consecutive-candles = ardışık mumlar
strategies-error-indicator-moving-average = hareketli ortalama
strategies-error-indicator-price-breakout = fiyat kırılımı
strategies-error-indicator-price-change-lookback = fiyat değişimi geriye bakışı
strategies-error-indicator-volume-spike = hacim sıçraması
strategies-error-indicator-moving-average-cross = hareketli ortalama kesişimi
strategies-error-indicator-rsi = RSI
strategies-error-indicator-bollinger-bands = Bollinger bantları
strategies-error-indicator-pullback-from-high = tepeden geri çekilme
strategies-error-indicator-volatility-range = volatilite aralığı
strategies-error-indicator-vwap = VWAP

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
