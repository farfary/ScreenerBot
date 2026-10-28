setup-wallet-required = Bir cüzdan özel anahtarı girin.
setup-wallet-json-recognized = 64 baytlık JSON anahtar biçimi tanındı.
setup-wallet-json-invalid = Tam olarak 64 bayt değeri (0–255) içeren bir JSON dizisi kullanın.
setup-wallet-format-invalid = Base58 özel anahtar veya 64 baytlık bir JSON dizisi kullanın.
setup-wallet-base58-recognized = Base58 anahtar biçimi tanındı.

setup-rpc-required = En az bir RPC uç noktası girin.
setup-dialog-rpc-optional-note = İsteğe bağlı: Zaten yapılandırılmış RPC uç noktalarını korumak için alanı boş bırakın.
setup-rpc-too-many = En fazla 10 RPC uç noktası kullanın.
setup-rpc-url-invalid = Her uç nokta geçerli bir HTTPS URL'si olmalıdır.
setup-rpc-url-credentials = RPC URL'leri kullanıcı adı veya parola içeremez.
setup-rpc-url-fragment = RPC URL'leri parça (fragment) içeremez.
setup-rpc-public-endpoint = Herkese açık Solana RPC'si sürekli yoklamayı destekleyemez.
setup-rpc-private-host = RPC uç noktaları yerel veya özel ağ ana bilgisayarlarını kullanamaz.
setup-rpc-duplicate = Yinelenen RPC uç noktalarını kaldırın.
setup-rpc-ready =
    { $count ->
        [one] Test için hazır HTTPS uç noktası: { $count }.
       *[other] Test için hazır HTTPS uç noktası: { $count }.
    }

setup-wallet-verified = Cüzdan doğrulandı
setup-wallet-unverified = Cüzdan doğrulanamadı
setup-wallet-address-detail = Adres { $address }
setup-wallet-format-hint = Özel anahtar biçimini kontrol edin.
setup-rpc-none-working = Çalışan mainnet RPC'si yok
setup-rpc-health-failed = Hiçbir uç nokta mainnet sağlık kontrollerini geçemedi.
setup-rpc-partial = Çalışan: { $working }; kullanılamayan: { $failed }
setup-rpc-verified =
    { $count ->
        [one] Doğrulanan mainnet uç noktası: { $count }
       *[other] Doğrulanan mainnet uç noktası: { $count }
    }
setup-rpc-fastest = En hızlı: { $url } ({ $latency }ms).
setup-error-request-failed = İstek başarısız oldu ({ $status })
setup-error-restart-timeout = { -brand } henüz yeniden bağlanmadı.

setup-verify-wallet-parsing = Özel anahtar ayrıştırılıyor
setup-verify-wallet-parsing-detail = Anahtar kontrol ediliyor ve genel adresi türetiliyor.
setup-verify-wallet-waiting = Doğrulama bekleniyor
setup-verify-rpc-testing = Solana mainnet test ediliyor
setup-verify-rpc-testing-detail =
    { $count ->
        [one] Kontrol edilen uç nokta: { $count }.
       *[other] Kontrol edilen uç nokta: { $count }.
    }
setup-verify-rpc-waiting = Uç nokta testleri bekleniyor
setup-verify-save-waiting = Kaydetme bekleniyor
setup-verify-save-running = Şifreleniyor ve kaydediliyor
setup-verify-save-running-detail = Doğrulanan yapılandırma bu cihaza yazılıyor.
setup-verify-save-done = Yapılandırma kaydedildi
setup-verify-save-done-detail = Özel anahtar şifrelendi; çalışan RPC uç noktaları saklandı.
setup-verify-save-failed = Kurulum kaydedilemedi
setup-verify-save-skipped = Kaydedilmedi. Yukarıdaki sorunu düzeltin, ardından yeniden doğrulayın.
setup-verify-request-failed = Doğrulama isteği başarısız oldu
setup-verify-summary-checking = Cüzdanınız ve Solana mainnet bağlantıları kontrol ediliyor.
setup-verify-summary-running = Girdiğiniz kimlik bilgileri aynen doğrulanıyor.
setup-verify-summary-saving = Kimlik bilgileri doğrulandı. Güvenli şekilde kaydediliyor.
setup-verify-summary-failed = Sorunu inceleyin, ardından yeniden doğrulayın.

setup-error-credentials-failed = Kimlik bilgisi doğrulaması başarısız oldu.
setup-error-save-failed = Kurulum kaydedilemedi.
setup-error-verify-failed = Doğrulama başarısız oldu.
setup-error-explore-failed = Keşif Modu başlatılamadı.
setup-error-gateway-failed = İşlem gönderme ayarı kaydedilemedi.
setup-action-review-credentials = Kimlik bilgilerini gözden geçir

setup-explore-opening = Keşif Modu açılıyor…
setup-complete-restarting = { -brand } doğrulanan yapılandırmanızla yeniden başlatılıyor.
setup-complete-finishing = Yeniden başlatma tamamlanıyor…
setup-complete-ready = { -brand } hazır. Panel açılıyor…
setup-complete-stored = Doğrulanan yapılandırmanız bu cihazda güvenle saklanıyor.

setup-wallet-show-key = Özel anahtarı göster
setup-wallet-hide-key = Özel anahtarı gizle
setup-wallet-copy =
    .aria-label = Cüzdan adresini kopyala
    .title = Cüzdan adresini kopyala
setup-wallet-copy-done =
    .aria-label = Cüzdan adresi kopyalandı
    .title = Kopyalandı
setup-wallet-copy-failed =
    .aria-label = Cüzdan adresi kopyalanamadı
    .title = Kopyalama başarısız

setup-dialog-title = Cüzdan ve RPC kurulumu
setup-dialog-subtitle = İşlem ve canlı zincir üstü veriyi etkinleştirmek için Solana cüzdanınızı ve premium bir RPC uç noktasını bağlayın. Özel anahtarınız bu cihazda şifrelenir ve cihazdan asla çıkmaz.
setup-dialog-close =
    .title = Kapat
    .aria-label = Kapat
setup-dialog-wallet-label = Cüzdan özel anahtarı
setup-dialog-wallet-input =
    .placeholder = Base58 dizesi veya JSON dizisi [1,2,3,...]
setup-dialog-submit = Doğrula ve bağlan
setup-dialog-working = Çalışıyor…
setup-dialog-validating = Doğrulanıyor…
setup-dialog-saving = Kaydediliyor…
setup-dialog-restarting = Yeniden başlatılıyor…
setup-dialog-saved = Kurulum kaydedildi — { -brand } tam modda yeniden başlatılıyor…
setup-dialog-error-validation = Doğrulama başarısız oldu.
setup-dialog-error-incomplete = Kurulum tamamlanamadı.
setup-dialog-error-restart-helper = Otomatik yeniden başlatma yardımcısı kullanılamıyor. Paneli birazdan yeniden yükleyin.
setup-dialog-error-unexpected = Beklenmeyen hata.
setup-dialog-replace-title = İşlem cüzdanını değiştir
setup-dialog-replace-subtitle = Başka bir cüzdanla işlem yapın. Özel anahtarı bu cihazda şifrelenir ve cihazdan asla çıkmaz.
setup-dialog-replace-submit = Doğrula ve değiştir
setup-dialog-replace-saved = Değişiklik hazırlandı — { -brand } yeni cüzdanla yeniden başlatılıyor…
setup-dialog-replace-confirm-intro = İşlem cüzdanını değiştirmek:
setup-dialog-replace-confirm-stop = Trader'ı ve kopya işlemi durdurur. Yeniden başlatmadan sonra bunları tekrar açın.
setup-dialog-replace-confirm-backup = Mevcut cüzdanın pozisyonlarını, işlemlerini ve geçmişini bir yedek klasörüne taşır.
setup-dialog-replace-confirm-keep = Mevcut cüzdanın anahtarını Diğer cüzdanlar altında tutar; anahtarı orada dışa aktarmaya devam edebilirsiniz.
setup-dialog-replace-confirm-funds = Hiçbir varlık taşınmaz: tokenlar ve { -sol } mevcut cüzdanda kalır.
setup-dialog-replace-confirm-rpc = Yapılandırılmış RPC uç noktalarını girdiklerinizle değiştirir; alan boş kalırsa mevcut uç noktalar korunur.
setup-dialog-replace-confirm-restart = { -brand } yeni cüzdanla yeniden başlatılır.

setup-wizard-progress =
    .aria-label = Kurulum ilerlemesi
setup-wizard-wallet-security-note = Kaydedilmeden önce şifrelenir.
setup-wizard-rpc-title = RPC uç noktaları
setup-wizard-rpc-input =
    .placeholder = Satır başına bir HTTPS URL'si
setup-wizard-rpc-guidance = Güvenilir bir mainnet sağlayıcısı kullanın; herkese açık uç noktalar desteklenmez.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = önerilen
setup-wizard-account-title = { -brand } hesabı
setup-wizard-account-optional = İsteğe bağlı
setup-wizard-account-loading = Hesap durumu kontrol ediliyor…
setup-wizard-verify-title = Doğrula ve kaydet
setup-wizard-verify-list =
    .aria-label = Kurulum doğrulama durumu
setup-wizard-verify-wallet = Cüzdan
setup-wizard-verify-save = Güvenli yapılandırma
setup-wizard-complete-title = Kurulum kaydedildi
setup-wizard-reconnect = Bağlantıyı yeniden dene
setup-wizard-reload = Paneli yeniden yükle
setup-wizard-error-title = Kurulum dikkat gerektiriyor
setup-wizard-explore = Paneli keşfet
setup-wizard-continue = Devam

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = Cüzdan
setup-wizard-step-connection = Bağlantı
setup-wizard-step-verify = Doğrula
setup-wizard-step-ready = Hazır
setup-wallet-title = İşlem cüzdanınızı kurun
setup-wallet-description = { -brand } uygulamasının işlem yaptığı cüzdan. Anahtarı bu bilgisayarda şifrelenir ve bilgisayardan asla çıkmaz.
setup-wallet-source =
    .aria-label = Cüzdan kaynağı
setup-wallet-create-title = Yeni cüzdan oluştur
setup-wallet-create-detail = { -brand } yeni bir cüzdan oluşturur. Anahtarını kaydedersiniz, ardından cüzdana { -sol } gönderirsiniz.
setup-wallet-import-title = Cüzdan içe aktar
setup-wallet-import-detail = Elinizdeki bir özel anahtarı kullanın (base58 veya 64 sayılık bir JSON dizisi).
setup-wallet-creating = Cüzdanınız oluşturuluyor…
setup-wallet-create-failed = Cüzdan oluşturulamadı. Tekrar deneyin.
setup-backup-title = Özel anahtarınızı kaydedin
setup-backup-description = Bu anahtara sahip olan herkes cüzdanı kontrol eder. { -brand } onu kurtaramaz.
setup-backup-address = Adres
setup-backup-key = Özel anahtar
setup-backup-copy-address = Adresi kopyala
setup-backup-download = İndir
setup-backup-copied = Kopyalandı. Kaydettikten sonra panonuzu temizleyin.
setup-backup-machine-warning = { -brand } uygulamasının sakladığı şifreli kopya yalnızca bu bilgisayarda açılır. Bilgisayar kaybolur ya da değiştirilirse cüzdanı yalnızca şimdi kaydettiğiniz anahtar geri getirir. Anahtarı çevrimdışı saklayın, örneğin kâğıtta veya bir parola yöneticisinde.
setup-backup-ack-saved = Özel anahtarı bu bilgisayarın dışında güvenli bir yere kaydettim.
setup-backup-ack-control = Bu anahtara sahip herkesin bu cüzdandaki fonları alabileceğini anlıyorum.
setup-backup-ack-recovery = { -brand } uygulamasının bu anahtarı kurtaramayacağını anlıyorum. Anahtar olmadan kaybolan bir bilgisayar, kaybolan fonlar demektir.
setup-backup-create-another = Başka bir tane oluştur
setup-backup-replace-confirm = Bu anahtar atılsın ve yeni bir cüzdan oluşturulsun mu? Bunu yalnızca cüzdana henüz bir şey göndermediyseniz yapın.
setup-connection-title = { -brand } { -solana } ağına nasıl bağlansın?
setup-connection-description = Bunu daha sonra Ayarlar → Bağlantı bölümünden değiştirebilirsiniz.
setup-connection-choice =
    .aria-label = Bağlantı
setup-connection-service-title = { -brand } hizmeti
setup-connection-service-badge = RPC gerekmez
setup-connection-service-detail = Ücretsiz { -brand } hesabınızı kullanır.
setup-connection-service-prices = Fiyatlar birkaç saniyede bir yenilenir
setup-connection-service-features = Kopya işlem, cüzdan izleme, diğer cüzdanların bakiyeleri, eski geçmiş, doğrudan havuz swap'ları ve canlı havuz fiyatları kendi RPC'nizi gerektirir
setup-connection-service-pause = Oturumu kapatırsanız veya screenerbot.io erişilemezse işlemler duraklar
setup-connection-service-privacy = Özel anahtarınız bu bilgisayardan asla çıkmaz: swap'lar burada imzalanır ve screenerbot.io bakiyeleri ve işlemleri okumak için yalnızca cüzdanınızın açık adresini kullanır
setup-connection-own-title = Kendi RPC'niz
setup-connection-own-badge = En hızlı
setup-connection-own-detail = Saniyede yaklaşık iki fiyat ve tüm özellikler. Bir sağlayıcıdan RPC uç noktası gerekir.
setup-connection-signin-required = { -brand } hizmetini kullanmak için oturum açın veya ücretsiz bir hesap oluşturun.
setup-connection-reauthorize = { -brand } hizmetinin cüzdan sorgularına izin vermek için yeniden oturum açın.
setup-connection-sending = İşlemleri { -brand } üzerinden gönder
setup-connection-sending-hint = Oturum açıkken kullanılabilir. Mümkün olmadığında RPC'niz kullanılır.
setup-wizard-account-required = Gerekli
setup-wizard-verify-connection = Bağlantı
setup-verify-service-checking = { -brand } hizmeti kontrol ediliyor
setup-verify-service-ready = { -brand } hizmeti hazır
setup-verify-service-busy = { -brand } hizmeti şu anda yoğun. Birazdan tekrar deneyin veya kendi RPC'nizi kullanın.
setup-verify-service-daily-limit = Hesabınız { -brand } hizmetinin bugünkü limitine ulaştı. 00:00 UTC'den sonra tekrar deneyin veya kendi RPC'nizi kullanın.
setup-verify-service-unreachable = screenerbot.io erişilemiyor. İnternet bağlantınızı kontrol edin.
setup-verify-service-version = Bu sürüm artık { -brand } hizmeti tarafından desteklenmiyor. Devam etmek için { -brand } uygulamasını güncelleyin.
setup-verify-service-signin = { -brand } hizmetini kullanmak için oturum açın.
setup-verify-save-done-service-detail = Özel anahtar şifrelendi; { -brand } hizmeti seçildi.
setup-action-try-again = Tekrar dene
setup-action-use-own-rpc = Kendi RPC'mi kullan
setup-action-sign-in-again = Yeniden oturum aç
setup-ready-title = { -brand } hazır
setup-fund-title = İşlem cüzdanınıza para yatırın
setup-fund-waiting = { -sol } bekleniyor…
setup-fund-received = { $amount } { -sol } alındı
setup-fund-balance-unavailable = Bakiye okunuyor… Kendiliğinden güncellenir.
setup-fund-open-dashboard = Paneli aç
