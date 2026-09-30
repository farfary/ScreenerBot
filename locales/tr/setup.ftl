setup-wallet-required = Bir cüzdan özel anahtarı girin.
setup-wallet-json-recognized = 64 baytlık JSON anahtar biçimi tanındı.
setup-wallet-json-invalid = Tam olarak 64 bayt değeri (0–255) içeren bir JSON dizisi kullanın.
setup-wallet-format-invalid = Base58 özel anahtar veya 64 baytlık bir JSON dizisi kullanın.
setup-wallet-base58-recognized = Base58 anahtar biçimi tanındı.

setup-rpc-required = En az bir RPC uç noktası girin.
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
setup-error-restart-timeout = Kurulum kaydedildi, ancak { -brand } henüz yeniden bağlanmadı.

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
setup-verify-save-skipped = Kaydedilmedi
setup-verify-request-failed = Doğrulama isteği başarısız oldu
setup-verify-summary-checking = Cüzdanınız ve Solana mainnet bağlantıları kontrol ediliyor.
setup-verify-summary-running = Girdiğiniz kimlik bilgileri aynen doğrulanıyor.
setup-verify-summary-saving = Kimlik bilgileri doğrulandı. Güvenli şekilde kaydediliyor.
setup-verify-summary-failed = Sorunu inceleyin, ardından yeniden doğrulayın.

setup-error-credentials-failed = Kimlik bilgisi doğrulaması başarısız oldu.
setup-error-save-failed = Kurulum kaydedilemedi.
setup-error-verify-failed = Doğrulama başarısız oldu.
setup-error-explore-failed = Keşif Modu başlatılamadı.
setup-error-gateway-failed = Ağ geçidi tercihi kaydedilemedi.
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
setup-dialog-rpc-label = RPC uç noktaları
setup-dialog-rpc-input =
    .placeholder = https://uc-noktaniz... (satır başına bir tane)
setup-dialog-rpc-hint = Premium bir sağlayıcı ({ -helius }, { -quicknode }, { -alchemy }) şiddetle önerilir — herkese açık Solana RPC'sinde istek sınırı vardır ve çalışmayabilir.
setup-dialog-submit = Doğrula ve bağlan
setup-dialog-working = Çalışıyor…
setup-dialog-validating = Doğrulanıyor…
setup-dialog-saving = Kaydediliyor…
setup-dialog-restarting = Yeniden başlatılıyor…
setup-dialog-saved = Kurulum kaydedildi — { -brand } tam modda yeniden başlatılıyor…
setup-dialog-error-missing-fields = Hem bir cüzdan özel anahtarı hem de en az bir RPC URL'si girin.
setup-dialog-error-validation = Doğrulama başarısız oldu.
setup-dialog-error-incomplete = Kurulum tamamlanamadı.
setup-dialog-error-restart-helper = Otomatik yeniden başlatma yardımcısı kullanılamıyor. Paneli birazdan yeniden yükleyin.
setup-dialog-error-unexpected = Beklenmeyen hata.

setup-wizard-progress =
    .aria-label = Kurulum ilerlemesi
setup-wizard-step-credentials = Kimlik bilgileri
setup-wizard-step-verification = Doğrulama
setup-wizard-step-complete = Tamamlandı
setup-wizard-credentials-title = Kimlik bilgilerini yapılandırın
setup-wizard-credentials-description = Yerel bir cüzdan ve güvenilir Solana mainnet RPC uç noktaları bağlayın.
setup-wizard-wallet-toggle =
    .title = Özel anahtarı göster
    .aria-label = Özel anahtarı göster
setup-wizard-wallet-security-note = Kaydedilmeden önce şifrelenir.
setup-wizard-rpc-title = RPC uç noktaları
setup-wizard-rpc-input =
    .placeholder = Satır başına bir HTTPS URL'si
setup-wizard-rpc-guidance = Sürekli yoklama için güvenilir mainnet RPC önerilir.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = önerilen
setup-wizard-gateway-title = Ücretsiz işlem gönderimi
setup-wizard-gateway-hint = Oturum açıkken kullanılabilir. RPC'niz yedek olarak kullanılabilir kalır.
setup-wizard-account-title = { -brand } hesabı
setup-wizard-account-optional = İsteğe bağlı
setup-wizard-account-loading = Hesap durumu kontrol ediliyor…
setup-wizard-verify-title = Doğrula ve kaydet
setup-wizard-verify-list =
    .aria-label = Kurulum doğrulama durumu
setup-wizard-verify-wallet = Cüzdan
setup-wizard-verify-rpc = Solana RPC
setup-wizard-verify-save = Güvenli yapılandırma
setup-wizard-complete-title = Kurulum kaydedildi
setup-wizard-reconnect = Bağlantıyı yeniden dene
setup-wizard-reload = Paneli yeniden yükle
setup-wizard-error-title = Kurulum dikkat gerektiriyor
setup-wizard-explore = Paneli keşfet
setup-wizard-continue = Devam
