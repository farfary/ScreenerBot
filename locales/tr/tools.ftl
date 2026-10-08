## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = Araçlar
tools-category-wallet = Cüzdan
tools-category-token = Token
tools-category-single-token = Tekli Token
tools-category-utilities = Yardımcı Araçlar
tools-sidebar-hint = Başlamak için bir araç seçin
tools-help-button =
    .aria-label = Bu araç için yardımı göster
tools-help-unavailable = Yardım mevcut değil
tools-placeholder-title = Bir Araç Seçin
tools-placeholder-subtitle = Başlamak için kenar çubuğundan bir araç seçin
tools-placeholder-hint-wallets = Cüzdan araçları Solana cüzdanlarınızı yönetmenize yardımcı olur
tools-placeholder-hint-secure = Tüm işlemler güvence altındadır ve mümkün olduğunda geri alınabilir

tools-status-ready = Kullanıma hazır
tools-status-coming = Yakında
tools-status-beta = Beta - hatalar olabilir
tools-status-disabled = Şu anda devre dışı
tools-status-badge-coming = Yakında
tools-status-badge-beta = Beta
tools-toast-coming-soon = Bu araç yakında geliyor
tools-toast-disabled = Bu araç şu anda devre dışı

## Tool names. `-title` names the tool in the navigation and the header, `-summary` is the
## navigation line, `-description` is the header line. Ids are the tool ids of the registry.

tools-tool-wallet-cleanup-title = Cüzdan Temizleme
tools-tool-wallet-cleanup-summary = Boş ATA'ları kapat
tools-tool-wallet-cleanup-description = { -sol } geri almak için boş İlişkili Token Hesaplarını (ATA) kapatın
tools-tool-burn-tokens-title = Token Yak
tools-tool-burn-tokens-summary = Tokenları kalıcı olarak yok et
tools-tool-burn-tokens-description = Cüzdanınızdaki tokenları kalıcı olarak yok edin
tools-tool-token-analyzer-title = Token Analizcisi
tools-tool-token-analyzer-summary = Derin token analizi
tools-tool-token-analyzer-description = Herhangi bir Solana tokenının çok boyutlu içgörülerle derin analizi
tools-tool-create-token-title = Token Oluştur
tools-tool-create-token-summary = Yeni SPL token dağıt
tools-tool-create-token-description = Solana üzerinde yeni bir SPL token dağıtın
tools-tool-trade-watcher-title = İşlem İzleyici
tools-tool-trade-watcher-summary = İşlemleri izle ve otomatik eylem al
tools-tool-trade-watcher-description = Token işlemlerini izleyin ve otomatik alım/satım eylemlerini tetikleyin
tools-tool-token-watch-title = Holder İzleme
tools-tool-token-watch-summary = Yeni token holderlarını takip et
tools-tool-token-watch-description = Yeni token holderlarını gerçek zamanlı takip edin ve izleyin
tools-tool-buy-multi-wallets-title = Çoklu Alım
tools-tool-buy-multi-wallets-summary = Cüzdanlar arasında alımları koordine et
tools-tool-buy-multi-wallets-description = Rastgele miktarlarla birden fazla cüzdanda koordineli alım emirleri yürütün
tools-tool-sell-multi-wallets-title = Çoklu Satım
tools-tool-sell-multi-wallets-summary = Cüzdanlar arasında satımları koordine et
tools-tool-sell-multi-wallets-description = Birden fazla cüzdanda { -sol } birleştirmeyle koordineli satış emirleri yürütün
tools-tool-wallet-consolidation-title = Cüzdan Birleştirme
tools-tool-wallet-consolidation-nav-title = Birleştirme
tools-tool-wallet-consolidation-summary = Cüzdan fonlarını birleştir
tools-tool-wallet-consolidation-description = Alt cüzdanlardaki { -sol } ve tokenları ana cüzdanda birleştirin
tools-tool-airdrop-checker-title = Airdrop Denetleyici
tools-tool-airdrop-checker-summary = Bekleyen airdrop'ları denetle
tools-tool-airdrop-checker-description = Bekleyen airdrop'ları ve talep edilebilir ödülleri denetleyin
tools-tool-wallet-generator-title = Cüzdan Oluşturucu
tools-tool-wallet-generator-summary = Yeni anahtar çiftleri oluştur
tools-tool-wallet-generator-description = Yeni Solana anahtar çiftlerini güvenle oluşturun

## Shared by the tools

tools-validation-mint-required = Lütfen bir token mint adresi girin
tools-validation-mint-format = Token mint adresi biçimi geçersiz
tools-validation-mint-invalid = Lütfen geçerli bir mint adresi girin

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = Token Ayrıntıları
tools-create-token-name-label = Token Adı
tools-create-token-name-input =
    .placeholder = Tokenım
tools-create-token-symbol-label = Sembol
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = Ondalık
tools-create-token-supply-label = Başlangıç Arzı
tools-create-token-description-label = Açıklama
tools-create-token-description-input =
    .placeholder = Token açıklaması...
tools-create-token-image-title = Token Görseli
tools-create-token-image-drop = Görseli buraya bırakın veya yüklemek için tıklayın
tools-create-token-image-hint = Önerilen: 512x512 PNG
tools-create-token-action-preview = Önizleme
tools-create-token-action-create = Token Oluştur

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = Ayarlar yükleniyor...
tools-holder-watch-saved = Holder İzleme ayarları kaydedildi
tools-holder-watch-save-failed = Ayarlar kaydedilemedi
tools-holder-watch-save-error = Ayarlar kaydedilirken hata oluştu
tools-holder-watch-settings-title = Holder İzleme Ayarları
tools-holder-watch-enabled-label = Holder İzlemeyi Etkinleştir
tools-holder-watch-interval-label = Denetim Aralığı (saniye)
tools-holder-watch-interval-hint = Holder sayılarının ne sıklıkla denetleneceği (10-3600sn)
tools-holder-watch-max-tokens-label = Azami İzlenen Token
tools-holder-watch-max-tokens-hint = Aynı anda izlenecek azami token sayısı
tools-holder-watch-notify-new-label = Yeni Holderlarda Bildir
tools-holder-watch-notify-drop-label = Holder Düşüşünde Bildir
tools-holder-watch-min-change-label = Asgari Holder Değişimi
tools-holder-watch-min-change-hint = Bildirimi tetikleyecek asgari holder değişimi
tools-holder-watch-drop-percent-label = Holder Düşüş Eşiği (%)
tools-holder-watch-drop-percent-hint = Uyarıyı tetikleyecek düşüş yüzdesi
tools-holder-watch-action-save = Ayarları Kaydet
tools-holder-watch-tokens-title = İzlenen Tokenlar
tools-holder-watch-token-input =
    .placeholder = Token mint adresini girin...
tools-holder-watch-empty = İzlenen token yok
tools-holder-watch-empty-hint = İzlemeye başlamak için yukarıya bir token mint adresi ekleyin
tools-holder-watch-coming-soon = Token izleme özelliği yakında geliyor

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = Token Analiz Et
tools-analyzer-mint-input =
    .placeholder = Token mint adresini yapıştırın...
tools-analyzer-action-analyze = Analiz Et
tools-analyzer-action-analyzing = Analiz ediliyor...
tools-analyzer-action-copy-report = Raporu Kopyala
tools-analyzer-loading = Token analiz ediliyor...
tools-analyzer-failed = Token analiz edilemedi
tools-analyzer-empty = Analiz için bir token mint adresi girin
tools-analyzer-empty-hint = Herhangi bir Solana tokenı hakkında kapsamlı içgörüler edinin
tools-analyzer-tab-overview = Genel Bakış
tools-analyzer-tab-security = Güvenlik
tools-analyzer-tab-market = Piyasa
tools-analyzer-tab-liquidity = Likidite
tools-analyzer-unknown-token = Bilinmeyen Token

# Token header actions.
tools-analyzer-favorite-add =
    .title = Favorilere ekle
    .aria-label = Favorilere ekle
tools-analyzer-favorite-already = Zaten Favorilerde
tools-analyzer-favorite-added = Favorilere eklendi: { $symbol }
tools-analyzer-favorite-failed = Favorilere eklenemedi
tools-analyzer-blacklist-add =
    .title = Kara listeye ekle
    .aria-label = Kara listeye ekle
tools-analyzer-blacklist-title = Tokenı Kara Listeye Al
tools-analyzer-blacklist-message = Kara listeye alınacak token: { $symbol }. Bu token işlemlerin dışında bırakılacak. Onaylıyor musunuz?
tools-analyzer-blacklist-confirm = Kara Listeye Al
tools-analyzer-blacklisted = Kara listede
tools-analyzer-blacklist-done = Kara listeye alındı: { $symbol }
tools-analyzer-blacklist-failed = Token kara listeye alınamadı

# Overview tab.
tools-analyzer-card-quick-stats = Hızlı İstatistikler
tools-analyzer-card-market-summary = Piyasa Özeti
tools-analyzer-card-token-info = Token Bilgisi
tools-analyzer-stat-holders = Holderlar
tools-analyzer-stat-decimals = Ondalık
tools-analyzer-stat-safety-score = Güvenlik Puanı
tools-analyzer-stat-pools = Havuzlar
tools-analyzer-stat-volume-24h = 24sa Hacim
tools-analyzer-stat-change-24h = 24sa Değişim
tools-analyzer-stat-market-cap = Piyasa Değeri
tools-analyzer-stat-liquidity = Likidite
tools-analyzer-info-mint = Mint Adresi
tools-analyzer-info-description = Açıklama
tools-analyzer-info-supply = Arz

# Security tab.
tools-analyzer-security-empty = Güvenlik verisi yok
tools-analyzer-security-empty-hint = Bu token için güvenlik analizi mevcut değil
tools-analyzer-card-safety-score = Güvenlik Puanı
tools-analyzer-score-good = İyi
tools-analyzer-score-moderate = Orta
tools-analyzer-score-risky = Riskli
tools-analyzer-raw-score = Ham Risk Puanı: { $score }
tools-analyzer-card-authorities = Token Yetkileri
tools-analyzer-authority-mint = Mint Yetkisi
tools-analyzer-authority-freeze = Dondurma Yetkisi
tools-analyzer-authority-transfer-fee = Transfer Ücreti
tools-analyzer-authority-mutable = Değiştirilebilir
tools-analyzer-authority-active = Etkin
tools-analyzer-authority-revoked = İptal edildi
tools-analyzer-card-holder-concentration = Holder Yoğunlaşması
tools-analyzer-top-holders = ilk 10 holder tarafından tutuluyor
tools-analyzer-risks-title = Güvenlik Riskleri ({ $count })
tools-analyzer-risks-title-none = Güvenlik Riskleri
tools-analyzer-risks-none = Güvenlik riski tespit edilmedi

# Market tab.
tools-analyzer-market-empty = Piyasa verisi yok
tools-analyzer-market-empty-hint = Bu token için piyasa verisi mevcut değil
tools-analyzer-card-price = Güncel Fiyat
tools-analyzer-card-price-changes = Fiyat Değişimleri
tools-analyzer-card-volume = İşlem Hacmi
tools-analyzer-card-transactions = 24sa İşlemler
tools-analyzer-card-valuation = Değerleme
tools-analyzer-stat-window-1h = 1sa
tools-analyzer-stat-window-6h = 6sa
tools-analyzer-stat-window-24h = 24sa
tools-analyzer-stat-volume-1h = 1sa Hacim
tools-analyzer-stat-volume-6h = 6sa Hacim
tools-analyzer-stat-fdv = Tam Seyreltilmiş Değer
tools-analyzer-txn-buys = Alımlar
tools-analyzer-txn-sells = Satımlar

# Liquidity tab.
tools-analyzer-liquidity-empty = Likidite verisi yok
tools-analyzer-liquidity-empty-hint = Bu token için havuz bulunamadı
tools-analyzer-card-total-liquidity = Toplam Likidite
tools-analyzer-card-pools = Havuzlar
tools-analyzer-active-pools =
    { $count ->
        [one] Aktif Havuz
       *[other] Aktif Havuz
    }
tools-analyzer-card-pool-details = Havuz Ayrıntıları
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = Likidite ({ -sol })
tools-analyzer-pools-column-status = Durum
tools-analyzer-pool-primary = Birincil

# Copied report. Each line is one message; values arrive already formatted.
tools-analyzer-report-empty = Kopyalanacak analiz yok
tools-analyzer-report-label = Analiz raporu
tools-analyzer-report-title = Token Analiz Raporu
tools-analyzer-report-token = Token: { $symbol } ({ $name })
tools-analyzer-report-mint = Mint: { $mint }
tools-analyzer-report-price = Fiyat: { $sol }
tools-analyzer-report-price-with-usd = Fiyat: { $sol } ({ $usd })
tools-analyzer-report-security = Güvenlik:
tools-analyzer-report-safety-score = - Güvenlik Puanı: { $score }/100
tools-analyzer-report-mint-authority = - Mint Yetkisi: { $state }
tools-analyzer-report-freeze-authority = - Dondurma Yetkisi: { $state }
tools-analyzer-report-risks = - Riskler: { $count }
tools-analyzer-report-market = Piyasa:
tools-analyzer-report-volume = - 24sa Hacim: { $amount }
tools-analyzer-report-change = - 24sa Değişim: { $amount }
tools-analyzer-report-market-cap = - Piyasa Değeri: { $amount }
tools-analyzer-report-liquidity = Likidite:
tools-analyzer-report-liquidity-total = - Toplam: { $amount }
tools-analyzer-report-pools = - Havuzlar: { $count }
tools-analyzer-report-generated = Oluşturulma: { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

tools-watch-type-buy-on-sell = Satışta Al
tools-watch-type-sell-on-buy = Alımda Sat
tools-watch-type-notify = Bildir
tools-watch-type-notify-only = Yalnızca Bildir

tools-trade-watcher-setup-title = İzlemeyi Ayarla
tools-trade-watcher-mint-label = Token Mint Adresi
tools-trade-watcher-mint-input =
    .placeholder = Token mint adresini girin...
tools-trade-watcher-action-search-pools = Havuz Ara
tools-trade-watcher-pool-label = Seçilen Havuz
tools-trade-watcher-pool-none = Havuz seçilmedi
tools-trade-watcher-pool-clear =
    .title = Havuzu temizle
tools-trade-watcher-pool-selected = Seçilen havuz: { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = İzleme Türü
tools-trade-watcher-type-hint = Satışta Al: Biri sattığında otomatik al. Alımda Sat: Biri aldığında otomatik sat.
tools-trade-watcher-trigger-label = Tetikleyici Miktar ({ -sol })
tools-trade-watcher-trigger-hint = Eylemi tetikleyecek asgari işlem boyutu ({ -sol } cinsinden)
tools-trade-watcher-action-amount-label = Eylem Miktarı ({ -sol })
tools-trade-watcher-action-amount-hint = Tetiklendiğinde alınacak/satılacak miktar
tools-trade-watcher-slippage-label = Kayma (%)
tools-trade-watcher-slippage-hint = İşlemler için kabul edilebilir azami kayma
tools-trade-watcher-active-title = Etkin İzlemeler
tools-trade-watcher-empty = Etkin izleme yok
tools-trade-watcher-empty-hint = Yukarıda bir izleme yapılandırın ve izlemeye başlamak için "İzlemeyi Başlat"a tıklayın
tools-trade-watcher-action-start = İzlemeyi Başlat
tools-trade-watcher-action-starting = Başlatılıyor...
tools-trade-watcher-action-stop-all = Tümünü Durdur
tools-trade-watcher-action-stopping = Durduruluyor...
tools-trade-watcher-started = İzleme başlatıldı: { $token }...
tools-trade-watcher-start-failed = İzleme başlatılamadı
tools-trade-watcher-stopped = İzleme durduruldu
tools-trade-watcher-stop-failed = İzleme durdurulamadı
tools-trade-watcher-stopped-all = Tüm izlemeler durduruldu
tools-trade-watcher-stop-all-failed = İzlemeler durdurulamadı
tools-trade-watcher-load-failed = İzlemeler yüklenemedi
tools-trade-watcher-column-token = Token
tools-trade-watcher-column-type = Tür
tools-trade-watcher-column-trigger = Tetikleyici
tools-trade-watcher-column-action = Eylem
tools-trade-watcher-column-triggered = Tetiklenen
tools-trade-watcher-stop-watch =
    .title = İzlemeyi durdur

## Results returned by the tools backend. Failures are catalog text; the technical cause
## travels separately as details and is appended by the dashboard.

tools-burn-failure-native-asset = { -sol } yakılamaz
tools-burn-failure-open-position = Açık pozisyonlardaki tokenlar yakılamaz
tools-burn-failure-account-not-found = Token hesabı bulunamadı
tools-burn-failure-zero-balance = Token bakiyesi zaten sıfır
tools-burn-failure-transaction = İşlem başarısız oldu
tools-burn-warning-open-position = Açık pozisyonlardaki tokenlar yakılamaz
tools-burn-warning-closed-position = Kapalı pozisyondan kalan
tools-burn-warning-worth = Değeri ~{ $amount } { -sol }
tools-multi-buy-warning-insufficient = Yetersiz bakiye. Gereken { $needed } { -sol }, mevcut { $have } { -sol }
tools-multi-buy-warning-over-limit = Gereken toplam { -sol } ({ $needed }) limiti ({ $limit }) aşıyor
tools-multi-sell-warning-no-wallets = İkincil cüzdan bulunamadı
tools-multi-sell-warning-no-balance = Hiçbir cüzdanda token bakiyesi yok
tools-multi-op-buy-failed = Alım başarısız oldu
tools-multi-op-sell-failed = Satım başarısız oldu
tools-multi-op-transfer-failed = Transfer başarısız oldu
tools-multi-op-balance-failed = Bakiye alınamadı
tools-multi-op-mint-invalid = Mint adresi geçersiz
tools-multi-buy-session-failed = Çoklu alım başarısız oldu
tools-multi-sell-session-failed = Çoklu satım başarısız oldu
tools-multi-session-aborted = İşlem kullanıcı tarafından iptal edildi

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = Cüzdanı Tara
tools-wallet-action-scanning = Taranıyor...
tools-wallet-scan-failed = Tarama başarısız oldu: { $reason }
tools-wallet-amount-approx = ~{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
        [one] Seçilen cüzdan: { $count }
       *[other] Seçilen cüzdan: { $count }
    }
tools-wallet-transfer-failed = Transfer başarısız oldu: { $reason }
tools-wallet-cleanup-failed = Temizlik başarısız oldu: { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = Tarama Sonuçları
tools-wallet-cleanup-stat-empty = Boş ATA'lar
tools-wallet-cleanup-stat-reclaimable = Geri Alınabilir { -sol }
tools-wallet-cleanup-stat-failed = Başarısız (önbellekte)
tools-wallet-cleanup-prompt = Boş ATA'ları bulmak için "Cüzdanı Tara"ya tıklayın
tools-wallet-cleanup-prompt-hint = Bu işlem cüzdanınızdaki tüm token hesaplarını denetler
tools-wallet-cleanup-action-cleanup = Tümünü Temizle
tools-wallet-cleanup-action-cleaning = Temizleniyor...
tools-wallet-cleanup-scanning = Cüzdan taranıyor...
tools-wallet-cleanup-found =
    { $count ->
        [one] Değeri ~{ $amount } olan boş ATA sayısı: { $count }
       *[other] Değeri ~{ $amount } olan boş ATA sayısı: { $count }
    }
tools-wallet-cleanup-clean = Boş ATA bulunamadı - cüzdan temiz!
tools-wallet-cleanup-scan-failed = ATA'lar taranamadı
tools-wallet-cleanup-done =
    { $count ->
        [one] Temizlenen ATA sayısı: { $count }
       *[other] Temizlenen ATA sayısı: { $count }
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = Token Yak
tools-burn-info-title = Yakmak nedir?
tools-burn-info-body = Yakmak, tokenları kalıcı olarak yok eder ve geri alınamaz hale getirir. Yaktıktan sonra boş ATA'ları kapatıp token başına ~0,002 { -sol } kirayı geri almak için Cüzdan Temizleme'yi çalıştırın.
tools-burn-stat-total = Toplam Token
tools-burn-stat-selected = Seçilen
tools-burn-stat-rent = Geri Alınabilir Kira
tools-burn-prompt = Tokenları bulmak için "Cüzdanı Tara"ya tıklayın
tools-burn-scanning = Cüzdandaki tokenlar taranıyor...
tools-burn-scan-failed = Tokenlar taranamadı
tools-burn-empty = Cüzdanda token bulunamadı
tools-burn-action-burn = Seçilenleri Yak ({ $count })
tools-burn-action-burning = Yakılıyor...
tools-burn-cannot-burn = Yakılamaz
tools-burn-no-value = Değeri yok

# Category titles and descriptions. Ids are the token categories of the scan.
tools-burn-category-open-position = Açık Pozisyonlar
tools-burn-category-has-value = Değerli
tools-burn-category-closed-position = Kapalı Pozisyonlar
tools-burn-category-zero-liquidity = Sıfır Likidite
tools-burn-category-hint-open-position = Açık pozisyonlardaki tokenlar yakılamaz
tools-burn-category-hint-has-value = Yakmak yerine satmayı düşünün
tools-burn-category-hint-closed-position = Kapalı işlemlerden kalanlar
tools-burn-category-hint-zero-liquidity = Yakmak güvenli - piyasa değeri yok

tools-burn-confirm-title = Yakmayı Onayla
tools-burn-confirm-message =
    { $count ->
        [one] Yakılacak token sayısı: <strong>{ $count }</strong>. Emin misiniz?
       *[other] Yakılacak token sayısı: <strong>{ $count }</strong>. Emin misiniz?
    }
tools-burn-confirm-value = Toplam tahmini değer: <strong>{ $amount }</strong>
tools-burn-confirm-continue = Devam
tools-burn-final-title = Son Uyarı
tools-burn-final-headline = Bu eylem GERİ ALINAMAZ!
tools-burn-final-message =
    { $count ->
        [one] Aşağıdaki { $count } token kalıcı olarak yok edilecek ve hiçbir koşulda geri getirilemeyecek.
       *[other] Aşağıdaki { $count } token kalıcı olarak yok edilecek ve hiçbir koşulda geri getirilemeyecek.
    }
tools-burn-final-confirm = Evet, Tokenları Yak
tools-burn-toast-burned =
    { $total ->
        [one] Yakılan token: { $successful }/{ $total }. ~{ $amount } geri almak için Cüzdan Temizleme'yi çalıştırın
       *[other] Yakılan token: { $successful }/{ $total }. ~{ $amount } geri almak için Cüzdan Temizleme'yi çalıştırın
    }
tools-burn-toast-failed =
    { $count ->
        [one] Yakılamayan token: { $count }
       *[other] Yakılamayan token: { $count }
    }
tools-burn-failed = Yakma başarısız oldu: { $reason }
tools-burn-failures-title =
    { $count ->
        [one] Yakılamayan token: { $count }
       *[other] Yakılamayan token: { $count }
    }
tools-burn-failure-unknown = Neden bildirilmedi

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = Hakkında
tools-airdrop-about-body = Popüler Solana protokollerinde bekleyen airdrop'ları, talep edilebilir ödülleri ve talep edilmemiş tahsisleri denetleyin.
tools-airdrop-list-title = Kullanılabilir Airdrop'lar
tools-airdrop-prompt = Talep edilebilir olanları taramak için "Airdrop'ları Denetle"ye tıklayın
tools-airdrop-action-check = Airdrop'ları Denetle
tools-airdrop-action-claim-all = Tümünü Talep Et

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = Oluşturucu Seçenekleri
tools-generator-warning-title = Özel anahtarlarınızı güvenli saklayın!
tools-generator-warning-body = Oluşturulan anahtar çiftleri yerel olarak üretilir ve asla iletilmez. Anahtarlarınızı her zaman güvenli bir yerde yedekleyin.
tools-generator-count-label = Cüzdan Sayısı
tools-generator-vanity-label = Özel (Vanity) Adres (belirli karakterlerle başlar)
tools-generator-prefix-label = Önek
tools-generator-prefix-input =
    .placeholder = ör. SOL
tools-generator-prefix-hint = Daha uzun önekler üstel olarak daha uzun sürede üretilir
tools-generator-list-title = Oluşturulan Cüzdanlar
tools-generator-empty = Henüz cüzdan oluşturulmadı
tools-generator-action-generate = Oluştur
tools-generator-action-generating = Oluşturuluyor...
tools-generator-count-invalid = Lütfen 1 ile 10 arasında bir sayı girin
tools-generator-no-keypairs = Anahtar çifti döndürülmedi
tools-generator-generated =
    { $count ->
        [one] Oluşturulan cüzdan: { $count }
       *[other] Oluşturulan cüzdan: { $count }
    }
tools-generator-failed = Cüzdanlar oluşturulamadı: { $reason }
tools-generator-copy-public-key =
    .title = Açık anahtarı kopyala
tools-generator-copy-private-key =
    .title = Özel anahtarı kopyala
tools-generator-remove =
    .title = Listeden kaldır
tools-generator-reveal =
    .title = Özel anahtarı göster
tools-generator-public-key-label = Açık Anahtar:
tools-generator-private-key-label = Özel Anahtar:
tools-generator-public-key-name = Açık anahtar
tools-generator-private-key-copied = Özel anahtar kopyalandı
tools-generator-private-key-warning = Bu anahtara sahip olan herkes cüzdanı kontrol eder
tools-generator-export-empty = Dışa aktarılacak cüzdan yok
tools-generator-exported = Cüzdanlar dışa aktarıldı - güvenle saklayın

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = Özet
tools-consolidation-stat-wallets = Alt cüzdanlar
tools-consolidation-stat-native = Toplam { -sol }
tools-consolidation-stat-tokens = Token Türleri
tools-consolidation-stat-rent = Geri Alınabilir Kira
tools-consolidation-wallets-title = Cüzdanlar
tools-consolidation-loading-wallets = Cüzdanlar yükleniyor...
tools-consolidation-loading-data = Cüzdan verileri yükleniyor...
tools-consolidation-action-transfer-native = { -sol } Aktar
tools-consolidation-action-transfer-tokens = Tüm Tokenları Aktar
tools-consolidation-action-cleanup = ATA'ları Temizle
tools-consolidation-action-transferring = Aktarılıyor...
tools-consolidation-column-name = Ad
tools-consolidation-column-native = { -sol } Bakiyesi
tools-consolidation-column-tokens = Tokenlar
tools-consolidation-column-atas = Boş ATA'lar
tools-consolidation-empty = Alt cüzdan bulunamadı
tools-consolidation-empty-hint = Başlamak için Çoklu Alım ile alt cüzdanlar oluşturun
tools-consolidation-load-failed = Yüklenemedi: { $reason }
tools-consolidation-select-prompt = Birleştirilecek cüzdanları seçin
tools-consolidation-selection-totals =
    | { $amount } | { $tokens ->
        [one] { $tokens } token
       *[other] { $tokens } token
    } | { $atas ->
        [one] { $atas } boş ATA
       *[other] { $atas } boş ATA
    }
tools-consolidation-transferred-native = { $amount } ana cüzdana aktarıldı
tools-consolidation-transferred-tokens =
    { $count ->
        [one] Ana cüzdana aktarılan token: { $count }
       *[other] Ana cüzdana aktarılan token: { $count }
    }
tools-consolidation-cleaned =
    { $count ->
        [one] Kapatılan ATA: { $count }, geri alınan: { $amount }
       *[other] Kapatılan ATA: { $count }, geri alınan: { $amount }
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = Token
tools-multi-mint-label = Token Mint Adresi
tools-multi-mint-input =
    .placeholder = Token mint adresini yapıştırın...
tools-multi-execution-title = Yürütme Ayarları
tools-multi-delay-min-label = Asgari Gecikme (ms)
tools-multi-delay-max-label = Azami Gecikme (ms)
tools-multi-concurrency-label = Eşzamanlılık
tools-multi-concurrency-sequential = { $count } (Sıralı)
tools-multi-concurrency-parallel = { $count } paralel
tools-multi-slippage-label = Kayma (%)
tools-multi-router-label = Yönlendirici
tools-multi-router-auto = Otomatik (En İyi Rota)
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = Doğrudan Havuz
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = İlerleme
tools-multi-progress-preparing = Hazırlanıyor...
tools-multi-status-line = { $label } ({ $completed }/{ $total })
tools-multi-column-wallet = Cüzdan
tools-multi-column-route = Rota
tools-multi-column-status = Durum
tools-multi-op-completed = Tamamlandı
tools-multi-op-failed = Başarısız
tools-multi-action-stop = Durdur
tools-multi-action-loading = Yükleniyor...
tools-multi-start-failed = Başlatılamadı: { $reason }

# Session states. Ids are the states of a multi-wallet session.
tools-multi-state-pending = Bekliyor
tools-multi-state-funding = Fonlanıyor
tools-multi-state-executing = Yürütülüyor
tools-multi-state-consolidating = Birleştiriliyor
tools-multi-state-completed = Tamamlandı
tools-multi-state-failed = Başarısız
tools-multi-state-aborted = İptal edildi

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = Birden fazla cüzdanda almak istediğiniz token
tools-multi-buy-wallets-title = Cüzdan Ayarları
tools-multi-buy-wallet-count-label = Cüzdan Sayısı
tools-multi-buy-wallet-count-option =
    { $count ->
        [one] { $count } cüzdan
       *[other] { $count } cüzdan
    }
tools-multi-buy-wallet-count-hint = Kullanılacak alt cüzdan sayısı
tools-multi-buy-buffer-label = Cüzdan Başına { -sol } Tamponu
tools-multi-buy-buffer-hint = Ücretler için ayrılır (asgari 0,015 { -sol })
tools-multi-buy-amounts-title = Miktar Ayarları
tools-multi-buy-min-label = Cüzdan Başına Asgari { -sol }
tools-multi-buy-min-hint = Asgari alım miktarı
tools-multi-buy-max-label = Cüzdan Başına Azami { -sol }
tools-multi-buy-max-hint = Azami alım miktarı
tools-multi-buy-limit-label = Toplam { -sol } Limiti (isteğe bağlı)
tools-multi-buy-limit-hint = Azami toplam harcama
tools-multi-buy-preview-title = Önizleme
tools-multi-buy-preview-create = Oluşturulacak Cüzdanlar
tools-multi-buy-preview-amount = Cüzdan Başına Miktar
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = Gereken Toplam { -sol }
tools-multi-buy-preview-balance = Ana Bakiye
tools-multi-buy-action-preview = Önizleme
tools-multi-buy-action-start = Çoklu Alımı Başlat
tools-multi-buy-executing = Alımlar yürütülüyor...
tools-multi-buy-column-spent = Harcanan { -sol }
tools-multi-buy-column-tokens = Tokenlar
tools-multi-buy-preview-failed = Önizleme başarısız oldu: { $reason }
tools-multi-buy-started = Çoklu alım başlatıldı
tools-multi-buy-stopped = Çoklu alım durduruldu
tools-multi-buy-completed = Çoklu alım tamamlandı! Başarılı: { $successful }/{ $total }

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = Tokenı tutan cüzdanları taramak için bir token adresi girin
tools-multi-sell-action-scan = Tara
tools-multi-sell-settings-title = Satış Ayarları
tools-multi-sell-percent-label = Satış Yüzdesi
tools-multi-sell-percent-hint = Cüzdan başına satılacak token yüzdesi
tools-multi-sell-min-fee-label = Ücret için Asgari { -sol }
tools-multi-sell-min-fee-hint = İşlem ücreti için gereken asgari { -sol }
tools-multi-sell-topup-label = Gerekirse otomatik bakiye yükle
tools-multi-sell-topup-hint = Alt cüzdanın bakiyesi yetersizse ana cüzdandan { -sol } aktar
tools-multi-sell-post-title = Satış Sonrası Eylemler
tools-multi-sell-consolidate-label = { -sol } ana cüzdanda birleştirilsin
tools-multi-sell-consolidate-hint = Alt cüzdanlardaki tüm { -sol } ana cüzdana geri aktarılır
tools-multi-sell-close-atas-label = Satıştan sonra token ATA'larını kapat
tools-multi-sell-close-atas-hint = ATA başına ~0,002 { -sol } geri alın
tools-multi-sell-wallets-title = Tokenı Tutan Cüzdanlar
tools-multi-sell-empty = Hiçbir alt cüzdan bu tokenı tutmuyor
tools-multi-sell-column-tokens = Tokenlar
tools-multi-sell-column-native = { -sol } Bakiyesi
tools-multi-sell-column-topup = Bakiye Yükleme Gerekli
tools-multi-sell-none-selected = Cüzdan seçilmedi
tools-multi-sell-select-required = Lütfen en az bir cüzdan seçin
tools-multi-sell-action-start = Çoklu Satımı Başlat
tools-multi-sell-executing = Satımlar yürütülüyor...
tools-multi-sell-column-sold = Satılan Tokenlar
tools-multi-sell-column-received = Alınan { -sol }
tools-multi-sell-started = Çoklu satım başlatıldı
tools-multi-sell-stopped = Çoklu satım durduruldu
tools-multi-sell-completed = Çoklu satım tamamlandı! Alınan: { $amount }

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = Favoriler
tools-favorites-saved = Kayıtlı Favoriler
tools-favorites-save-current = Mevcut Olanı Kaydet
tools-favorites-empty = Henüz kayıtlı favori yok
tools-favorites-no-label = Etiket yok
tools-favorites-uses = { $count }x
tools-favorites-remove = Kaldır
tools-favorites-loaded = Favori yüklendi: { $name }
tools-favorites-default-name = Yapılandırma
tools-favorites-mint-required = Lütfen önce bir token mint adresi girin
tools-favorites-add-title = Favori Ekle
tools-favorites-add-message = Bu favori için bir etiket girin
tools-favorites-add-placeholder = Etiket (isteğe bağlı)...
tools-favorites-saved-toast = Favorilere kaydedildi
tools-favorites-save-failed = Favori kaydedilemedi
tools-favorites-remove-title = Favoriyi Kaldır
tools-favorites-remove-message = Bu favori kaldırılsın mı?
tools-favorites-removed-toast = Favori kaldırıldı
tools-favorites-remove-failed = Favori kaldırılamadı
