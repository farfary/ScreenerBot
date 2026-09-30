## Categories

hints-category-tokens = Tokenlar
hints-category-positions = Pozisyonlar
hints-category-filtering = Filtreleme
hints-category-trader = Otomatik Trader
hints-category-services = Hizmetler
hints-category-wallet = Cüzdan
hints-category-wallets = Cüzdanlar
hints-category-tools = Araçlar
hints-category-config = Yapılandırma
hints-category-config-telegram = { -telegram }
hints-category-token-details = Token Ayrıntıları
hints-category-ui = Arayüz

## tokens

hints-tokens-pool-service-title = Havuz Hizmeti Tokenları
hints-tokens-pool-service-content =
    Burada gösterilen tokenlar:

    • **Tüm filtreleme ölçütlerini geçti** — likidite, hacim, yaş ve güvenlik kontrolleri
    • **Geçerli SOL likidite havuzlarına sahip** — DEX çözücülerimiz tarafından destekleniyor (Raydium, Orca, Meteora vb.)
    • **Fiyat hesaplaması başarılı** — fiyatlar doğrudan zincir üstü havuz rezervlerinden hesaplanır

    Fiyatlar harici API'lerden değil gerçek havuz verilerinden türetildiği için işlem yapmak açısından en güvenilir token listesi budur.

    Ayrıntılı bilgi görmek ve kara liste durumunu yönetmek için herhangi bir tokena tıklayın.
hints-tokens-no-market-title = Piyasa Verisi Yok
hints-tokens-no-market-content =
    Zincir üstünde keşfedilen ancak { -dexscreener } veya { -geckoterminal } üzerinde piyasa verisi bulunmayan tokenlar.

    Yaygın nedenler:
    • **Çok yeni tokenlar** — henüz toplayıcılar tarafından dizinlenmedi
    • **Düşük işlem hacmi** — toplayıcı eşiklerinin altında
    • **Listelenmemiş çiftler** — toplayıcıların izlemediği DEX'lerde işlem görüyor

    Bu tokenların geçerli havuzları olabilir ve işlem yapılabilir, ancak harici piyasa metrikleri yoktur.
hints-tokens-all-title = Tüm Tokenlar
hints-tokens-all-content =
    Filtreleme durumundan bağımsız olarak keşfedilen tüm tokenların tam veritabanı.

    Şunları içerir:
    • Filtrelemeyi geçen tokenlar
    • Reddedilen tokenlar
    • Piyasa verisi olmayan tokenlar
    • Kara listedeki tokenlar

    Araştırma yapmak veya filtrelenmiş olabilecek tokenları bulmak için bu görünümü kullanın.
hints-tokens-passed-title = Filtrelemeyi Geçenler
hints-tokens-passed-content =
    Tüm etkin filtreleme ölçütlerini geçen tokenlar.

    Filtreleme kontrolleri şunları içerir:
    • **Likidite** — asgari SOL likidite eşiği
    • **Hacim** — 24sa işlem hacmi gereksinimleri
    • **Token yaşı** — oluşturulmasından bu yana asgari süre
    • **Güvenlik** — { -rugcheck } risk puanı sınırları
    • **Piyasa değeri** — isteğe bağlı FDV/piyasa değeri filtreleri

    Filtreleri **Filtreleme** sayfasında yapılandırın.
hints-tokens-rejected-title = Reddedilen Tokenlar
hints-tokens-rejected-content =
    Bir veya daha fazla filtreleme ölçütünü geçemeyen tokenlar.

    Her token için belirli reddedilme nedeni gösterilir:
    • Hangi filtre başarısız oldu
    • Gerçek değer ile gereken eşik
    • Kontrolün yapıldığı zaman

    Filtre ayarlarınızı inceltmek için reddedilen tokenları gözden geçirin.
hints-tokens-blacklisted-title = Kara Listedeki Tokenlar
hints-tokens-blacklisted-content =
    İşlemlerden kalıcı olarak çıkarılan tokenlar.

    Kara liste nedenleri şunları içerir:
    • **Manuel kara liste** — sizin açıkça engellediğiniz tokenlar
    • **Güvenlik riskleri** — tespit edilen rug pull göstergeleri
    • **Zarar eşiği** — yapılandırılan zarar limitlerinin aşılması
    • **Başarısız işlemler** — tekrarlanan takas hataları

    Kara listedeki tokenlar geçenler listesinde gösterilmez ve otomatik işlem için değerlendirilmez.
hints-tokens-positions-title = Pozisyon Tokenları
hints-tokens-positions-content =
    Şu anda açık pozisyonlarda tutulan tokenlar.

    Aktif varlıklarınız için gerçek zamanlı verileri gösterir:
    • Havuz rezervlerinden güncel fiyat
    • Gerçekleşmemiş K/Z
    • Pozisyon boyutu ve giriş fiyatı
    • Tutma süresi

    Ayrıntılı pozisyon yönetimi için herhangi bir tokena tıklayın.
hints-tokens-recent-title = Yeni Keşfedilenler
hints-tokens-recent-content =
    Keşfedilme zamanına göre sıralanmış yeni keşfedilen tokenlar.

    Şunlar için kullanışlıdır:
    • Yeni token lansmanlarını yakalamak
    • Yeni likiditeyi izlemek
    • Erken giriş fırsatları

    Not: Yeni tokenların piyasa verisi başlangıçta eksik olabilir.
hints-tokens-ohlcv-title = OHLCV Veri Yönetimi
hints-tokens-ohlcv-content =
    Tokenlar için saklanan OHLCV (mum) verilerini görüntüleyin ve yönetin.

    Şunları gösterir:
    • **Mum Sayısı** — saklanan toplam veri noktası
    • **Geçmiş Veri Doldurma İlerlemesi** — zaman dilimi tamamlanma durumu
    • **Veri Aralığı** — saat cinsinden zaman kapsamı
    • **Havuz Sayısı** — izlenen likidite havuzları
    • **Durum** — etkin izleme veya etkin değil

    Eylemler:
    • **Sil** — bir token için tüm OHLCV verilerini kaldır
    • **Temizle** — etkin olmayan token verilerini toplu kaldır

    OHLCV verileri kalıcı olarak saklanır ve otomatik silinmez.

## positions

hints-positions-overview-title = Pozisyonlara Genel Bakış
hints-positions-overview-content =
    Güncel token varlıklarınız ve işlem pozisyonlarınız.

    Temel metrikler:
    • **Giriş Fiyatı** — ödenen ortalama fiyat (DCA dahil)
    • **Güncel Fiyat** — havuz rezervlerinden canlı fiyat
    • **K/Z** — SOL ve % cinsinden gerçekleşmemiş kâr/zarar
    • **Boyut** — tutulan toplam token miktarı

    Ayrıntılı yönetim seçenekleri için herhangi bir pozisyona tıklayın.
hints-positions-dca-title = DCA (Maliyet Ortalaması)
hints-positions-dca-content =
    DCA, mevcut pozisyonlara farklı fiyatlardan ekleme yapmanızı sağlar.

    DCA tetiklendiğinde:
    • Ek token satın alınır
    • Giriş fiyatı ağırlıklı ortalama olarak yeniden hesaplanır
    • Pozisyon boyutu artar
    • Giriş sayısı artar

    DCA kurallarını **Otomatik Trader** ayarlarında yapılandırın.
hints-positions-partial-exit-title = Kısmi Çıkış
hints-positions-partial-exit-content =
    Pozisyonunuzun bir kısmını satıp geri kalanını elinizde tutun.

    Avantajları:
    • Piyasada kalırken bir miktar kârı kilitleyin
    • Pozisyonu tamamen kapatmadan boyutunu küçültün
    • Kâr al basamakları uygulayın

    Doğru K/Z takibi için her kısmi çıkış ayrı olarak kaydedilir.
hints-positions-management-title = Pozisyon Yönetimi
hints-positions-management-content =
    Yönetim, hangi otomasyonun bir pozisyon üzerinde işlem yapabileceğini belirler:

    • Otomatik Trader: güvenlik çıkışları, politika çıkışları ve otomatik DCA
    • Yalnızca Kullanıcı: otomatik işlem yok
    • Kopyalama Görevi: güvenlik çıkışları ve kopya satışları
    • Karma: güvenlik çıkışları, politika çıkışları ve kopya satışları

    Satışı veya eklemeyi kendiniz yaparsınız. Manuel alımlar varsayılan olarak manuel yönetimdedir; böylece bot bilerek aldığınız bir tokenı satamaz. Pozisyonu otomatik trader'a geri vermek için bunu kapatın.

## filtering

hints-filtering-overview-title = Token Filtreleme
hints-filtering-overview-content =
    Filtreleme, hangi tokenların işleme uygun olduğunu belirler.

    Geçenler listesinde görünmek için tokenlar **etkin tüm ölçütleri** geçmelidir:
    • { -dexscreener } metrikleri (likidite, hacim vb.)
    • { -geckoterminal } metrikleri (piyasa değeri, FDV)
    • { -rugcheck } güvenlik analizi
    • Meta filtreler (token yaşı vb.)

    Devre dışı ölçütler tamamen atlanır.
hints-filtering-dexscreener-title = { -dexscreener } Filtreleri
hints-filtering-dexscreener-content =
    { -dexscreener } piyasa verilerine dayalı filtreler:

    • **Likidite** — havuzlardaki asgari USD likiditesi
    • **Hacim 24sa** — asgari işlem hacmi
    • **İşlemler** — etkinlik eşikleri (alım/satım)
    • **Fiyat Değişimi** — oynaklık filtreleri

    { -dexscreener } verileri birkaç dakikada bir güncellenir.
hints-filtering-geckoterminal-title = { -geckoterminal } Filtreleri
hints-filtering-geckoterminal-content =
    { -geckoterminal } piyasa verilerine dayalı filtreler:

    • **Piyasa Değeri** — asgari piyasa değeri
    • **FDV** — Tam Seyreltilmiş Değerleme sınırları
    • **Rezerv Oranı** — havuz sağlığı göstergeleri

    { -geckoterminal } genellikle daha yeni tokenlar için veriye sahiptir.
hints-filtering-rugcheck-title = Güvenlik Filtreleri
hints-filtering-rugcheck-content =
    { -rugcheck }.xyz tarafından yapılan güvenlik analizi:

    • **Risk Puanı** — genel risk derecelendirmesi (0-100)
    • **Mint Yetkisi** — yeni token basılabilir mi?
    • **Dondurma Yetkisi** — transferler dondurulabilir mi?
    • **En Büyük Holderlar** — yoğunlaşma riski

    Yüksek risk puanları daha fazla olası uyarı işareti anlamına gelir.
hints-filtering-meta-title = Meta Filtreler
hints-filtering-meta-content =
    Ek filtreleme ölçütleri:

    • **Token Yaşı** — token oluşturulduktan sonra geçen asgari süre
    • **Havuz Yaşı** — havuz oluşturulduktan sonra geçen asgari süre
    • **Web Sitesi Var** — sosyal medya/web sitesi bağlantıları iste
    • **Sosyal Medya Var** — { -twitter }/{ -telegram } iste

    Bunlar çok yeni veya şüpheli tokenları elemeye yardımcı olur.

## trader

hints-trader-overview-title = Otomatik Trader
hints-trader-overview-content =
    Tokenları izleyen ve işlemleri yürüten otomatik işlem motoru.

    Bileşenler:
    • **Giriş İzleyici** — alım fırsatlarını izler
    • **Çıkış İzleyici** — satışları ve kâr almaları yönetir
    • **DCA İzleyici** — pozisyon ortalamasını yönetir
    • **Risk Kontrolleri** — zarar limitleri ve güvenlik kapıları

    İşlemi kontrol panelinden başlatın veya durdurun.
hints-trader-entry-title = Giriş İzleyici
hints-trader-entry-content =
    Filtrelenmiş tokenları giriş sinyalleri için izler.

    Giriş değerlendirmesi şunları denetler:
    • Token güncel filtrelemeyi geçiyor
    • Zaten bir pozisyonda değil
    • Kara listede değil
    • Pozisyon limitleri aşılmamış
    • Strateji koşulları karşılanmış (yapılandırıldıysa)

    Giriş boyutunu ve limitleri Yapılandırma'da ayarlayın.
hints-trader-exit-title = Çıkış İzleyici
hints-trader-exit-content =
    Açık pozisyonları çıkış sinyalleri için izler.

    Çıkış tetikleyicileri:
    • **Kâr Al** — fiyat hedefine ulaşıldı
    • **Zarar Durdur** — azami zarar aşıldı
    • **İz Süren Stop** — fiyat zirveden geri çekildi
    • **Strateji Çıkışı** — özel koşullar karşılandı
    • **Zamana Dayalı** — azami tutma süresi

    Eşikleri Yapılandırma'da ayarlayın.

## services

hints-services-overview-title = Sistem Hizmetleri
hints-services-overview-content =
    { -brand } uygulamasını çalıştıran arka plan hizmetleri.

    Hizmet durumları:
    • **Çalışıyor** (yeşil) — normal çalışıyor
    • **Başlıyor** (sarı) — başlatılıyor
    • **Durdu** (kırmızı) — çalışmıyor
    • **Hata** (uyarı) — başarısız oldu, otomatik yeniden başlayabilir

    Hizmetlerin bağımlılıkları vardır ve sırayla başlar.
hints-services-health-title = Hizmet Sağlığı
hints-services-health-content =
    Sağlık göstergeleri hizmet durumunu gösterir:

    • **Çalışma Süresi** — son başlatmadan bu yana geçen süre
    • **Görevler** — etkin arka plan işlemleri
    • **Hatalar** — son hata sayısı
    • **Metrikler** — performans verileri (varsa)

    Kritik hizmetler işlem yeteneğini etkiler.

## wallet

hints-wallet-overview-title = Cüzdana Genel Bakış
hints-wallet-overview-content =
    Bağlı Solana cüzdanınızın durumu.

    Şunları gösterir:
    • **SOL Bakiyesi** — ücretler ve işlem için yerel SOL
    • **Token Varlıkları** — değerleriyle SPL tokenları
    • **24sa Değişim** — portföy değeri değişimi
    • **Geçmiş** — zaman içindeki bakiye anlık görüntüleri

    Bakiyeler dakikada bir yenilenir.
hints-wallet-tokens-title = Token Bakiyeleri
hints-wallet-tokens-content =
    Cüzdanınızda tutulan SPL tokenları.

    Şunları gösterir:
    • Token sembolü ve adı
    • Tutulan miktar
    • SOL/USD cinsinden güncel değer
    • Havuzdan veya piyasa verisinden alınan fiyat

    Boş token hesapları Ayarlar'dan temizlenebilir.

## wallets

hints-wallets-main-title = Ana Cüzdan
hints-wallets-main-content =
    Tüm işlem operasyonları için kullanılan birincil cüzdan.

    • **Otomatik İşlem** — giriş/çıkış işlemleri bu cüzdandan yürütülür
    • **Bakiye Gösterimi** — üst bilgide ve panelde gösterilir
    • **Token Varlıkları** — bu cüzdanda tutulan SPL tokenları

    Ana cüzdanı değiştirmek için herhangi bir ikincil cüzdanda "Ana Yap" seçeneğini kullanın.
hints-wallets-secondary-title = İkincil Cüzdanlar
hints-wallets-secondary-content =
    Çoklu cüzdan operasyonları için ek cüzdanlar.

    • **Çoklu Cüzdan İşlemi** — cüzdanlar arasında alım/satımları koordine edin
    • **Portföy Ayrımı** — strateji veya amaca göre düzenleyin
    • **Bağımsız Bakiyeler** — her cüzdanın kendi SOL/tokenları vardır

    İkincil cüzdanlar, açıkça yapılandırılmadıkça otomatik işlem tarafından kullanılmaz.

## tools

hints-tools-wallet-cleanup-title = Cüzdan Temizleme Aracı
hints-tools-wallet-cleanup-content =
    { "*" }*Boş Token Hesaplarından SOL Geri Alın**

    { "*" }*ATA nedir?**
    İlişkili Token Hesapları (ATA), tokenlarınızı tutan Solana hesaplarıdır. Etkileşime girdiğiniz her token, ~0,002 SOL kira (rent) gerektiren bir ATA oluşturur.

    { "*" }*Boş ATA'lar neden temizlenmeli?**
    • Kirayı geri alın (ATA başına ~0,002 SOL)
    • Aktif trader'larda yüzlerce boş ATA birikebilir
    • 100 boş ATA = geri alınabilir ~0,2 SOL

    { "*" }*Nasıl çalışır:**
    • Cüzdanınızda bakiyesi sıfır olan ATA'ları tarar
    • Geri alınabilir toplam SOL miktarını gösterir
    • Kirayı geri almak için boş hesapları kapatır

    { "*" }*Otomatik Temizlik:**
    Etkinleştirildiğinde boş ATA'ları arka planda her 5 dakikada bir otomatik olarak tarar ve kapatır.

    { "*" }*Önemli:**
    • Yalnızca bakiyesi tam olarak 0 olan hesapları kapatır
    • Başarısız kapatmalar, tekrar deneme yığılmasını önlemek için önbelleğe alınır
    • Büyük cüzdanlar birden fazla temizlik turu gerektirebilir
hints-tools-burn-tokens-title = Token Yakma Aracı
hints-tools-burn-tokens-content =
    { "*" }*Tokenları Kalıcı Olarak Yok Edin**

    Token yakmak, tokenları cüzdanınızdan ve dolaşımdan kalıcı olarak kaldırır.

    { "*" }*Yaktığınızda ne olur:**
    • Tokenlar bir yakma adresine gönderilir (geri alınamaz)
    • Token bakiyesi sıfır olur
    • ATA daha sonra Cüzdan Temizleme ile kapatılıp ~0,002 SOL kira geri alınabilir

    { "*" }*Token Kategorileri:**
    • **Açık Pozisyonlar** - Yakılamaz (aktif işlemler)
    • **Kapalı Pozisyonlar** - Geçmiş işlemlerden kalanlar
    • **Değerli** - Likiditesi olan tokenlar (bunun yerine satmayı düşünün)
    • **Sıfır Likidite** - Toz/değersiz tokenlar (yakmak güvenli)

    { "*" }*Uyarı:** Bu eylem **geri alınamaz**. Yakılan tokenlar hiçbir koşulda geri getirilemez.

    { "*" }*Yaktıktan sonra:** Boş ATA'ları kapatıp SOL kirasını geri almak için Cüzdan Temizleme'yi çalıştırın.
hints-tools-wallet-generator-title = Cüzdan Oluşturucu Aracı
hints-tools-wallet-generator-content =
    { "*" }*Yeni Solana Anahtar Çiftleri Oluşturun**

    Cihazınızda güvenli şekilde yeni cüzdanlar oluşturun.

    { "*" }*Özellikler:**
    • Kriptografik olarak güvenli anahtar çiftleri üretir
    • İsteğe bağlı özel (vanity) adres öneki (örn. "SOL...")
    • base58 veya JSON dizisi olarak dışa aktarım

    { "*" }*Güvenlik:**
    • Anahtarlar yerel olarak üretilir
    • Ağ üzerinden asla iletilmez
    • Anahtarları her zaman güvenli şekilde yedekleyin
hints-tools-multi-buy-title = Çoklu Alım Aracı
hints-tools-multi-buy-content =
    { "*" }*Birden Fazla Cüzdanda Alımları Koordine Edin**

    Organik alım etkinliğini taklit etmek için rastgele miktarlarla birden fazla alt cüzdanda alım emirleri yürütün.

    { "*" }*Nasıl çalışır:**
    1. Alt cüzdanlar oluşturur veya mevcut olanları kullanır
    2. Ana cüzdandan alt cüzdanlara SOL dağıtır
    3. Rastgele miktarlar ve gecikmelerle alım emirlerini yürütür
    4. Her cüzdan benzersiz imzalarla bağımsız olarak alım yapar

    { "*" }*Cüzdan Ayarları:**
    • **Cüzdan Sayısı** — kullanılacak alt cüzdan sayısı (2-10)
    • **SOL Tamponu** — ücretler için cüzdan başına ayrılan SOL (~0,015)

    { "*" }*Miktar Ayarları:**
    • **Min/Maks SOL** — cüzdan başına alım miktarı aralığı
    • **Toplam Limit** — harcanacak toplam SOL için isteğe bağlı üst sınır

    { "*" }*Yürütme Ayarları:**
    • **Gecikme** — işlemler arasında rastgele gecikme
    • **Eşzamanlılık** — paralel yürütme (1 = sıralı)
    • **Kayma** — kabul edilebilir azami kayma
    • **Yönlendirici** — takas yönlendirmesi (Otomatik, { -jupiter }, Raydium)

    { "*" }*Önemli:**
    • Ana cüzdanda yeterli SOL gerekir
    • Başarısız alımlar günlüğe kaydedilir ancak oturumu durdurmaz
    • Alt cüzdanlar oturumlar arasında yeniden kullanılabilir
hints-tools-multi-sell-title = Çoklu Satım Aracı
hints-tools-multi-sell-content =
    { "*" }*Birden Fazla Cüzdanda Satışları Koordine Edin**

    Belirli bir tokenı tutan tüm alt cüzdanlardan otomatik SOL birleştirmeyle satış yapın.

    { "*" }*Nasıl çalışır:**
    1. Alt cüzdanlarda token bakiyelerini tarar
    2. İsteğe bağlı olarak ücret için SOL'u düşük cüzdanlara bakiye yükler
    3. Yapılandırılabilir yüzdeyle satış emirlerini yürütür
    4. Gelirleri ana cüzdanda birleştirir

    { "*" }*Satış Ayarları:**
    • **Satış %** — satılacak token yüzdesi (varsayılan %100)
    • **Ücret için Min SOL** — işlem için gereken asgari SOL
    • **Otomatik Bakiye Yükleme** — gerekirse ana cüzdandan SOL aktar

    { "*" }*Satış Sonrası Eylemler:**
    • **SOL'u Birleştir** — tüm SOL'u ana cüzdana geri aktar
    • **ATA'ları Kapat** — kirayı geri almak için token hesaplarını kapat (her biri ~0,002 SOL)

    { "*" }*Yürütme Ayarları:**
    • **Gecikme** — işlemler arasında rastgele gecikme
    • **Eşzamanlılık** — paralel yürütme
    • **Kayma** — kabul edilebilir azami kayma
    • **Yönlendirici** — takas yönlendirme tercihi

    { "*" }*İpuçları:**
    • Önizleme, tokenı tutan tüm cüzdanları gösterir
    • Satış yapmak istemediğiniz cüzdanların işaretini kaldırın
    • Birleştirme tüm satışlar tamamlandıktan sonra yapılır
hints-tools-trade-watcher-title = İşlem İzleyici Aracı
hints-tools-trade-watcher-content =
    { "*" }*İşlemleri İzleyin ve Otomatik Eylemleri Tetikleyin**

    Bir tokenın işlem etkinliğini izleyin ve işlemler gerçekleştiğinde otomatik olarak tepki verin.

    { "*" }*İzleme Türleri:**
    • **Satışta Al** — biri sattığında otomatik al (düşüşleri yakala)
    • **Alımda Sat** — biri aldığında otomatik sat (piyasayı takip et)
    • **Yalnızca Bildir** — işlem yapmadan uyarı al

    { "*" }*Nasıl çalışır:**
    1. Bir token mint adresi girin
    2. Mevcut likidite havuzlarını bulmak için "Havuz Ara"ya tıklayın
    3. İzlenecek bir havuz seçin (alım/satım eylemleri için gerekli)
    4. Tetikleyici miktarını belirleyin (tepki verilecek asgari işlem boyutu)
    5. Eylem miktarını belirleyin (ne kadar SOL alınacak/satılacak)
    6. İzlemeyi başlatın

    { "*" }*Gereksinimler:**
    • Geçerli token mint adresi
    • Havuz seçimi (alım/satım eylemleri için)
    • Eylem miktarları için yeterli SOL bakiyesi

    { "*" }*{ -telegram } Entegrasyonu:**
    İzlemeler tetiklendiğinde anında bildirim almak için { -telegram } ayarlarını Yapılandırma → { -telegram } bölümünde yapın.
hints-tools-wallet-consolidation-title = Cüzdan Birleştirme Aracı
hints-tools-wallet-consolidation-content =
    { "*" }*Alt Cüzdan Fonlarını Yönetin ve Birleştirin**

    Tüm alt cüzdanları görüntüleyin; SOL'u, tokenları birleştirin ve ATA kirasını ana cüzdanınıza geri alın.

    { "*" }*Özet Şunları Gösterir:**
    • **Alt cüzdanlar** — oluşturulan alt cüzdanların toplam sayısı
    • **Toplam SOL** — tüm alt cüzdanlardaki birleşik SOL bakiyesi
    • **Token Türleri** — tutulan farklı token sayısı
    • **Geri Alınabilir Kira** — boş ATA'larda kilitli SOL

    { "*" }*Eylemler:**
    • **SOL Aktar** — seçili cüzdanlardaki tüm SOL'u ana cüzdana taşı
    • **Token Aktar** — tüm tokenları ana cüzdana taşı
    • **ATA'ları Temizle** — kira iadesi için boş token hesaplarını kapat

    { "*" }*Tablo Bilgisi:**
    • Toplu işlemler için cüzdan seçmek üzere onay kutusu
    • Ad, adres, SOL bakiyesi, token sayısı, boş ATA'lar
    • Boş cüzdanlar kolay ayırt edilmesi için soluk gösterilir

    { "*" }*İpuçları:**
    • Kalan SOL'u toplamak için Çoklu Satım'dan sonra kullanın
    • Kirayı geri almak için ATA'ları düzenli olarak temizleyin
    • Boş cüzdanlar gelecekteki operasyonlar için yeniden kullanılabilir

## config

hints-config-overview-title = Yapılandırma
hints-config-overview-content =
    { -brand } için sistem genelindeki ayarlar.

    Kategoriler:
    • **Trader** — giriş/çıkış kuralları, pozisyon boyutlandırma
    • **Filtreleme** — token filtre eşikleri
    • **Takaslar** — yönlendirme ve kayma ayarları
    • **RPC** — düğüm yapılandırması
    • **Hizmetler** — arka plan hizmeti ayarları

    Değişiklikler hemen geçerli olur (sıcak yeniden yükleme).
hints-config-telegram-title = { -telegram } Bildirimleri
hints-config-telegram-content =
    { "*" }*İşlem uyarılarını { -telegram } üzerinden anında alın**

    İşlemler, pozisyonlar ve önemli olaylar hakkında doğrudan { -telegram } üzerinden bildirim alın.

    { "*" }*Kurulum Adımları:**

    1. **Bir bot oluşturun:**
       • { -telegram } uygulamasını açın ve @BotFather'a mesaj gönderin
       • /newbot gönderin ve yönergeleri izleyin
       • Bot belirtecini kopyalayın (şuna benzer: 123456:ABC-DEF...)

    2. **Sohbet Kimliğinizi alın:**
       • @userinfobot veya @getidsbot'a mesaj gönderin
       • Döndürdüğü sayısal kimliği kopyalayın

    3. **{ -brand } içinde yapılandırın:**
       • Bildirim anahtarını etkinleştirin
       • Bot belirtecini ve sohbet kimliğini yapıştırın
       • Doğrulamak için "Bağlantıyı Sına"ya tıklayın

    { "*" }*Alacaklarınız:**
    • İşlem yürütme onayları
    • Pozisyon güncellemeleri (giriş/çıkış)
    • İşlem İzleyici uyarıları
    • Hata bildirimleri

    { "*" }*Gizlilik:**
    Mesajlar doğrudan { -brand } uygulamasından { -telegram } botunuza gönderilir — üçüncü taraf sunucu yoktur.
hints-config-telegram-password-title = Bot Kimlik Doğrulama Parolası
hints-config-telegram-password-content =
    { "*" }*{ -telegram } botunuzu parola doğrulamasıyla güvenceye alın**

    { -brand } { -telegram } botunuzla etkileşime girdiğinizde, hassas komutları yürütmeden önce bu parolayla kimliğinizi doğrulamanız gerekir.

    { "*" }*Neden parola belirlemelisiniz?**
    • Yetkisiz kullanıcıların botunuzu kontrol etmesini önler
    • { -telegram } üzerinden işlem komutlarını yürütmek için gereklidir
    • En az 8 karakter uzunluğunda olmalıdır

    { "*" }*Nasıl çalışır:**
    1. Burada, panelde bir parola belirleyin
    2. Botunuza bir işlem komutu gönderdiğinizde kimlik doğrulaması ister
    3. Kimliğinizi doğrulamak için parolanızı girin
    4. İsteğe bağlı olarak ek güvenlik için 2FA'yı etkinleştirin

    { "*" }*Not:** Parola güvenli bir SHA256 özeti olarak saklanır — düz metin olarak asla saklanmaz.
hints-config-telegram-totp-title = İki Faktörlü Doğrulama (2FA)
hints-config-telegram-totp-content =
    { "*" }*TOTP 2FA ile ek bir güvenlik katmanı ekleyin**

    İki faktörlü doğrulama, Google Authenticator, Authy veya 1Password gibi uygulamalardan zamana dayalı tek kullanımlık parolalar (TOTP) kullanır.

    { "*" }*2FA neden etkinleştirilmeli?**
    • Biri parolanızı bilse bile kod olmadan botunuza erişemez
    • 6 haneli kodlar her 30 saniyede bir değişir
    • Kurulumdan sonra çevrimdışı çalışır

    { "*" }*Kurulum süreci:**
    1. "2FA'yı Etkinleştir"e tıklayıp parolanızı girin
    2. QR kodu doğrulayıcı uygulamanızla tarayın
    3. Kurulumu doğrulamak için 6 haneli kodu girin

    { "*" }*Uyumlu uygulamalar:**
    • Google Authenticator
    • Authy
    • 1Password
    • Microsoft Authenticator
    • TOTP uyumlu herhangi bir uygulama

    { "*" }*Önemli:** Gizli anahtarınızı güvenli bir yerde saklayın. Doğrulayıcı uygulamanıza erişimi kaybederseniz 2FA'yı bu panelden devre dışı bırakmanız gerekir.

## token_details

hints-token-details-chart-title = Fiyat Grafiği (OHLCV)
hints-token-details-chart-content =
    { "*" }*Önemli:** Bu grafik, canlı işlem fiyatını değil, strateji değerlendirmesi için **önbellekteki OHLCV verilerini** gösterir.

    { "*" }*Neden Önbellekteki Veri?**
    • **Amaç:** Otomatik stratejiler ve göstergeler (ör. RSI, MA) tarafından kullanılır.
    • **Tazelik:** Güncellemeler token önceliğine bağlıdır (Açık pozisyonlar = Daha hızlı güncelleme).
    • **Kaynak:** Doğrudan zincir üstü RPC'den değil, { -dexscreener }/{ -geckoterminal } üzerinden toplanır.

    { "*" }*DEX Fiyat Gerçeği:**
    DeFi'de tokenlar **birden fazla havuzda** (Raydium, Orca, Meteora) işlem görür. Her havuzun fiyatı likidite derinliğine ve son işlemlere göre farklıdır.
    • **Grafik Fiyatı:** Piyasalar genelinde bir ortalama/toplam.
    • **Takas Fiyatı:** İşlem anında en iyi rotadan aldığınız belirli oran.

    { "*" }Bu grafik ile nihai işlem fiyatınız arasında küçük farklar bekleyin.*

    { "*" }*Durum:** "Veri bekleniyor", arka plan işçilerinin yeni mumları getirdiği anlamına gelir.
hints-token-details-token-info-title = Token Bilgisi
hints-token-details-token-info-content =
    Zincir üstü ve piyasa kaynaklarından temel token meta verileri.

        • **Mint** — Solana'daki benzersiz token adresi (kopyalamak için tıklayın)
        • **Ondalık** — token hassasiyeti (genellikle 6-9)
        • **Yaş** — birincil havuzun/tokenın oluşturulmasından bu yana geçen süre
        • **DEX** — bu token için birincil işlem platformu
        • **Holderlar** — tokenı tutan benzersiz cüzdanlar
        • **İlk 10 Payı** — ilk 10 cüzdanın tuttuğu %

        Yüksek holder sayısı ve düşük yoğunlaşma genellikle daha sağlıklı dağılıma işaret eder.
hints-token-details-liquidity-title = Likidite { "&" } Piyasa Verileri
hints-token-details-liquidity-content =
    En yüksek likiditeli SOL havuzundan piyasa metrikleri.

        • **FDV** — fiyat × toplam arz (toplayıcı fiyatı)
        • **Likidite** — havuz rezervlerinin USD değeri
        • **Havuz SOL** / **Havuz Token** — havuz fiyatını belirleyen canlı rezervler

        { "*" }*Neden önemli:**
        • Daha derin likidite = daha düşük kayma
        • Sığ havuzlar küçük işlemlerle hareket edebilir
        • Havuz rezervleri takas yürütme fiyatını doğrudan belirler

        Veriler { -dexscreener }/{ -geckoterminal } kaynaklarından ve zincir üstü havuz okumalarından düzenli aralıklarla yenilenir.
hints-token-details-market-pulse-title = Piyasa Nabzı
hints-token-details-market-pulse-content =
    Fiyat hareketi ve USD işlem hacmi aynı **5M / 1H / 6H / 24H** zaman çizelgesini paylaşır; böylece ivme ve katılım doğrudan karşılaştırılabilir.

    { "*" }*Yorumlama:**
    • **Fiyat** — toplayıcıdan türetilen yüzde değişim, canlı havuz yürütme fiyatı değil.
    • **Yüksek Hacim** — daha güçlü ilgi, daha verimli fiyat keşfi ve daha kolay çıkışlar.
    • **Düşük Hacim** — daha fazla kayma, daha geniş spread ve daha zor büyük çıkışlar.
    • **Yüksek Hacim + Düşük Likidite** — artan oynaklık ve yürütme riski.

    Piyasa verileri { -dexscreener }/{ -geckoterminal } üzerinden büyük DEX'ler genelinde toplandığından, fiyat değişimi güncel zincir üstü havuz fiyatından farklı olabilir.
hints-token-details-activity-title = İşlem Etkinliği (Sayılar)
hints-token-details-activity-content =
    Birden fazla zaman diliminde **işlem sayısını** (alım ve satım) analiz eder. Bu, işlem boyutundan bağımsız olarak trader niyetini ortaya koyar.

    { "*" }*Metrik Dökümü:**
    • **Zaman Dilimleri:** 5M, 1H, 6H, 24H pencereleri.
    • **Çubuklar:** Alım sayısının (Yeşil) satım sayısına (Kırmızı) görsel oranı.
    • **Hız:** Dakika başına işlem (ör. "12,5/dk"). Yüksek hız = viral etkinlik.
    • **Sayılar:** Alım/satımların kesin sayısı ve yüzde payı.

    { "*" }*Özet metrikler:**
    • **24H Alım %:** >%50 yükseliş yönlüdür (daha çok alıcı), { "<" }%50 düşüş yönlüdür (daha çok satıcı).
    • **Net Akış:** Toplam alımlar eksi satımlar. Pozitif = Birikim.
    • **5M Ani Artış:** İşlemin *şu anda* 1H ortalamasına kıyasla ne kadar hızlı olduğu.
      • **>1,0x:** İvmelenen ilgi.
      • **>3,0x:** Viral kırılım veya panik olayı.
      • **{ "<" }1,0x:** Soğuyor.

    { "*" }*Strateji İpucu:** Yüksek "Alım %" ile yüksek "Ani Artış Çarpanı" genellikle güçlü bir kırılım girişine işaret eder.
hints-token-details-security-title = Güvenlik Analizi
hints-token-details-security-content =
    { -rugcheck }.xyz ve zincir üstü analizden risk değerlendirmesi.

    { "*" }*Güvenlik Puanı (0-100):**
    Yüksek puanlar daha güvenli tokenları gösterir. Etkenler:
    • Yetki izinleri (mint/dondurma)
    • Holder yoğunlaşması
    • LP kilit durumu
    • Bilinen risk desenleri

    { "*" }*Temel Risk Göstergeleri:**
    • **Mint Yetkisi** — yeni token oluşturabilir (enflasyon riski)
    • **Dondurma Yetkisi** — token hesaplarını dondurabilir
    • **En Büyük Holder %** — yoğunlaşma riski
    • **LP Sağlayıcıları** — likidite sağlayıcı sayısı

    Önemli tutarlarla işlem yapmadan önce güvenliği her zaman doğrulayın.
hints-token-details-pools-title = Likidite Havuzları
hints-token-details-pools-content =
    Bu token için keşfedilen tüm likidite havuzları.

    { "*" }*Birden fazla havuz neden önemli:**
    • Her havuzun likiditesi ve fiyatı farklıdır
    • Takas yönlendiricileri havuzlar arasında en iyi rotayı bulur
    • Havuzlar arasında fiyat %1-5 değişebilir

    { "*" }*Havuz Bilgisi:**
    • **DEX** — havuzu hangi borsa barındırıyor
    • **Likidite** — havuz rezervlerinin USD değeri
    • **Hacim** — son işlem etkinliği
    • **Fiyat** — güncel havuz fiyatı

    Havuz Hizmeti, fiyatları en yüksek likiditeli SOL çiftinden hesaplar.

## ui

hints-ui-featured-title = Öne Çıkan
hints-ui-featured-content =
    Önce boost'lu tokenlar, ardından { -jupiter } ve { -dexscreener } üzerindeki trend projeler.

    { "*" }*Göreceğiniz içerik:**
    • Boost'lu tokenlar — ekipleri tanıtım için ödeme yaptı — öne sabitlenir ve altın rengiyle işaretlenir
    • Ardından keşif panolarındaki trend tokenlar
    • Tam ayrıntıları açmak için herhangi bir tokena tıklayın

    { "*" }*Bir tokenı boost'lama:**
    Boost görünürlük satın alır, asla bir tavsiye değildir. Boost'lu satırlar, token tablonuz dahil
    göründükleri her yerde altın rengiyle işaretlenir; böylece hangisinin hangisi olduğunu her zaman bilirsiniz. Bir tokenı
    { "*" }*screenerbot.io/boost** adresinden boost'layın.

    { "*" }*Satırı devre dışı bırakma:**
    Şuradan gizleyin: **Ayarlar → Arayüz → Öne Çıkan Satırı Göster**. Üst bilgideki eylem yine de
    tam Öne Çıkan görünümünü açar.

## Hint popover chrome (ui/hint_popover.js)

hints-trigger =
    .aria-label = Yardım: { $title }
hints-popover-close =
    .aria-label = Kapat
hints-popover-learn-more = Daha fazla bilgi
hints-popover-dismiss = Bir daha gösterme
