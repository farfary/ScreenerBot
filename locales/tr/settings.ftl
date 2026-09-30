## Shared

settings-duration-minutes =
    { $count ->
        [one] { $count } dakika
       *[other] { $count } dakika
    }
settings-duration-hours =
    { $count ->
        [one] { $count } saat
       *[other] { $count } saat
    }

## settings_dialog.js

settings-dialog-title = Ayarlar
settings-dialog-close =
    .title = Kapat (ESC)
    .aria-label = Ayarları kapat
settings-dialog-save = Değişiklikleri Kaydet
settings-dialog-saving = Kaydediliyor...
settings-dialog-saved = Kaydedildi
settings-dialog-save-success = Ayarlar başarıyla kaydedildi
settings-dialog-save-failed = Ayarlar kaydedilemedi
settings-dialog-update-attention = Güncelleme dikkat gerektiriyor
settings-dialog-tab-interface = Arayüz
settings-dialog-tab-navigation = Gezinme
settings-dialog-tab-startup = Başlangıç
settings-dialog-tab-hints = İpuçları
settings-dialog-tab-data = Veri
settings-dialog-tab-security = Güvenlik
settings-dialog-tab-account = Hesap
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = Ajan Bağlantıları
settings-dialog-tab-updates = Güncellemeler
settings-dialog-tab-licenses = Lisanslar
settings-dialog-tab-about = Hakkında
settings-dialog-link-privacy = Gizlilik Politikası
settings-dialog-link-terms = Hizmet Şartları

## settings_dialog.js: Startup tab

settings-startup-section-title = Başlangıç Davranışı
settings-startup-auto-start-label = Trader'ı Otomatik Başlat
settings-startup-auto-start-hint = Uygulama açıldığında trader'ı otomatik olarak başlat
settings-startup-coming-soon = Yakında
settings-startup-default-page-label = Varsayılan Sayfa
settings-startup-default-page-hint = Uygulama açıldığında gösterilecek sayfa
settings-startup-page-dashboard = Panel
settings-startup-page-tokens = Tokenlar
settings-startup-page-positions = Pozisyonlar
settings-startup-page-wallet = Cüzdan
settings-startup-page-config = Yapılandırma
settings-startup-notifications-label = Arka Plan Bildirimlerini Göster
settings-startup-notifications-hint = Arka plandaki olaylar için bildirim göster

## settings_dialog.js: About tab

settings-about-logo =
    .alt = { -brand }
settings-about-tagline = Yerel Solana Trading Motoru
settings-about-link-github = { -github }
settings-about-link-docs = Belgeler
settings-about-link-telegram = { -telegram }
settings-about-link-website = Web sitesi
settings-about-credits = Solana trader'ları için geliştirildi
settings-about-copyright = © { $year } { -brand }. Tüm hakları saklıdır.

## interface_tab.js

settings-interface-section-appearance = Görünüm
settings-interface-theme-label = Tema
settings-interface-theme-hint = Tercih ettiğiniz renk düzenini seçin
settings-interface-theme-dark = Koyu
settings-interface-theme-light = Açık
settings-interface-language-label = Dil
settings-interface-language-hint = Panelin görüntüleme dili
settings-interface-logo-shape-label = Token Logo Şekli
settings-interface-logo-shape-hint = Daire her logoyu kırpar; Doğal her tasarımın kendi siluetini korur
settings-interface-logo-shape-circle = Daire
settings-interface-logo-shape-natural = Doğal
settings-interface-animations-label = Animasyonları Etkinleştir
settings-interface-animations-hint = Yumuşak geçişler ve efektler
settings-interface-compact-label = Kompakt Mod
settings-interface-compact-hint = Daha fazla içerik için dolguyu azalt
settings-interface-section-data = Veri ve Görüntü
settings-interface-refresh-label = Yenileme Aralığı
settings-interface-refresh-hint = Verilerin ne sıklıkla yenileneceği
settings-interface-refresh-seconds =
    { $count ->
        [one] { $count } saniye
       *[other] { $count } saniye
    }
settings-interface-refresh-minutes =
    { $count ->
        [one] { $count } dakika
       *[other] { $count } dakika
    }
settings-interface-ticker-label = Kayan Şeridi Göster
settings-interface-ticker-hint = Üst bölümde canlı metrik şeridi
settings-interface-page-size-label = Tablo Sayfa Boyutu
settings-interface-page-size-hint = Tablo sayfası başına varsayılan satır sayısı
settings-interface-page-size-rows =
    { $count ->
        [one] { $count } satır
       *[other] { $count } satır
    }
settings-interface-auto-expand-label = Kategorileri Otomatik Genişlet
settings-interface-auto-expand-hint = Yapılandırma kategorilerini varsayılan olarak genişlet
settings-interface-hints-label = Bağlamsal İpuçlarını Göster
settings-interface-hints-hint = Panel özelliklerini açıklayan yardım simgelerini göster
settings-interface-featured-label = Öne Çıkanlar Satırını Göster
settings-interface-featured-hint = Ana Sayfa ve Tokenlar sayfalarında öne çıkan tokenlar satırını göster
settings-interface-section-sound = Ses Efektleri
settings-interface-sounds-label = Sesleri Etkinleştir
settings-interface-sounds-hint = Gezinme, durum değişiklikleri ve sonuçlar için dokunsal ipuçları

## security_tab.js

settings-security-loading = Güvenlik ayarları yükleniyor...
settings-security-load-failed = Güvenlik ayarları yüklenemedi

settings-security-type-pin4 = 4 Haneli PIN
settings-security-type-pin6 = 6 Haneli PIN
settings-security-type-text = Metin Parolası
settings-security-type-unset = Ayarlanmadı

settings-security-lockscreen-title = Panel Kilit Ekranı
settings-security-lockscreen-description = Panelinizi bir PIN veya parola ile koruyun. Kilit ekranı tetiklendiğinde devam etmek için kimlik doğrulaması gerekir.
settings-security-enable-label = Kilit Ekranını Etkinleştir
settings-security-enable-hint = Panelinizi parola doğrulamasıyla koruyun
settings-security-password-status-label = Parola Durumu
settings-security-password-current = Geçerli: { $type }
settings-security-password-none = Parola ayarlanmadı
settings-security-change = Değiştir
settings-security-remove = Kaldır
settings-security-set-password = Parola Belirle
settings-security-auto-lock-label = Hareketsizlik Sonrası Otomatik Kilit
settings-security-auto-lock-hint = Belirli bir süre işlem yapılmazsa otomatik olarak kilitle
settings-security-auto-lock-never = Hiçbir zaman
settings-security-lock-blur-label = Pencere Odağı Kaybedince Kilitle
settings-security-lock-blur-hint = Başka bir uygulamaya geçtiğinizde otomatik olarak kilitle
settings-security-quick-actions-title = Hızlı İşlemler
settings-security-lock-now-label = Paneli Şimdi Kilitle
settings-security-lock-now-hint = Paneli hemen kilitle
settings-security-lock-now = Şimdi Kilitle
settings-security-lock-not-ready = Kilitlenemiyor - kilit ekranı hazır değil
settings-security-setting-save-failed = Güvenlik ayarı kaydedilemedi

## security_tab.js: two-factor authentication

settings-security-2fa-title = İki Adımlı Doğrulama
settings-security-2fa-description = Bir doğrulayıcı uygulaması (Google Authenticator, Authy vb.) ile ek bir güvenlik katmanı ekleyin
settings-security-2fa-status-label = 2FA Durumu
settings-security-2fa-status-enabled = İki adımlı doğrulama etkin
settings-security-2fa-status-none = Yapılandırılmadı
settings-security-2fa-disable = 2FA'yı Devre Dışı Bırak
settings-security-2fa-enable = 2FA'yı Etkinleştir

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = Kapat
settings-security-password-set-title = Parola Belirle
settings-security-password-change-title = Parolayı Değiştir
settings-security-password-current-label = Geçerli Parola
settings-security-password-current-input =
    .placeholder = Geçerli parolayı girin
settings-security-password-type-label = Parola Türü
settings-security-password-new-label = Yeni Parola
settings-security-password-new-input =
    .placeholder = Yeni parolayı girin
settings-security-password-confirm-label = Parolayı Onayla
settings-security-password-confirm-input =
    .placeholder = Parolayı onaylayın
settings-security-password-update = Parolayı Güncelle
settings-security-placeholder-pin4 = 4 haneli PIN girin
settings-security-placeholder-pin6 = 6 haneli PIN girin
settings-security-placeholder-text = Parolayı girin
settings-security-password-required = Lütfen bir parola girin
settings-security-password-mismatch = Parolalar eşleşmiyor
settings-security-pin4-invalid = PIN tam olarak 4 haneli olmalıdır
settings-security-pin6-invalid = PIN tam olarak 6 haneli olmalıdır
settings-security-text-too-short = Parola en az 4 karakter olmalıdır
settings-security-password-saved = Parola kaydedildi
settings-security-password-save-failed = Parola kaydedilemedi
settings-security-password-save-failed-detail = Parola kaydedilemedi: { $message }

## security_tab.js: remove password dialog

settings-security-remove-title = Parolayı Kaldır
settings-security-remove-description = Kilit ekranı korumasını kaldırmak için geçerli parolanızı girin.
settings-security-remove-confirm = Parolayı Kaldır
settings-security-current-required = Lütfen geçerli parolanızı girin
settings-security-password-removed = Parola kaldırıldı
settings-security-password-remove-failed = Parola kaldırılamadı
settings-security-password-remove-failed-detail = Parola kaldırılamadı: { $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = İki Adımlı Doğrulamayı Etkinleştir
settings-security-2fa-password-prompt = Devam etmek için parolanızı girin:
settings-security-2fa-password-input =
    .placeholder = Parolayı girin
settings-security-2fa-continue = Devam
settings-security-2fa-manual-code = Manuel giriş kodu:
settings-security-2fa-qr =
    .alt = TOTP QR Kodu
settings-security-2fa-code-prompt = Doğrulayıcı uygulamanızdaki 6 haneli kodu girin:
settings-security-2fa-verify-enable = Doğrula ve Etkinleştir
settings-security-2fa-password-required = Lütfen parolanızı girin
settings-security-2fa-setup-failed = 2FA kurulamadı
settings-security-2fa-code-invalid-length = Lütfen 6 haneli bir kod girin
settings-security-2fa-code-invalid = Geçersiz kod
settings-security-2fa-enabled = İki adımlı doğrulama etkinleştirildi
settings-security-2fa-verify-failed = Kod doğrulanamadı
settings-security-2fa-disable-title = İki Adımlı Doğrulamayı Devre Dışı Bırak
settings-security-2fa-disable-prompt = 2FA'yı devre dışı bırakmak için parolanızı girin:
settings-security-2fa-disable-failed = 2FA devre dışı bırakılamadı
settings-security-2fa-disabled = İki adımlı doğrulama devre dışı bırakıldı

## agent_connections_tab.js

settings-agent-category-analysis = Analiz
settings-agent-category-portfolio = Portföy
settings-agent-category-trading = İşlem
settings-agent-category-config = Yapılandırma
settings-agent-category-system = Sistem
settings-agent-category-analysis-description = Token analizi, piyasa verileri ve güvenlik kontrolleri.
settings-agent-category-portfolio-description = Açık pozisyonlar, bakiyeler ve K/Z.
settings-agent-category-trading-description = Gerçek fonlarla pozisyon alma, satma ve kapatma.
settings-agent-category-config-description = RPC uç noktaları dahil tüm bot ayarları. Cüzdan anahtarları asla.
settings-agent-category-system-description = Durum, olaylar ve acil durdurma.
settings-agent-category-analysis-inline = analiz
settings-agent-category-portfolio-inline = portföy
settings-agent-category-trading-inline = işlem
settings-agent-category-config-inline = yapılandırma
settings-agent-category-system-inline = sistem

settings-agent-level-allow = İzin ver
settings-agent-level-ask-user = Sor
settings-agent-level-deny = Kapalı
settings-agent-level-allow-hint = Hemen çalışır.
settings-agent-level-ask-user-hint = Uygulamada onayınızı bekler.
settings-agent-level-deny-hint = Reddedilir ve ajandan gizlenir.

settings-agent-preset-full = Tam erişim
settings-agent-preset-ask = Önce sor
settings-agent-preset-read = Salt okunur
settings-agent-preset-full-description = Her şey sormadan çalışır. Cüzdan anahtarlarına erişilemez.
settings-agent-preset-ask-description = Her işlem uygulamada onayınızı bekler.
settings-agent-preset-read-description = Analiz ve portföy okumaları. Hiçbir şey değiştirilemez.
settings-agent-preset-custom = Özel
settings-agent-preset-group =
    .aria-label = İzin ön ayarı
settings-agent-permission-group = İzin: { $category }

settings-agent-summary-asks-only = Sınırlı — onay istenen: { $asking }
settings-agent-summary-off-only = Sınırlı — kapalı: { $off }
settings-agent-summary-asks-and-off = Sınırlı — onay istenen: { $asking }; kapalı: { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = Genel stdio MCP

settings-agent-note-placeholder = /absolute/path/to/screenerbot yolunu { -brand } ikili dosyanızın mutlak yolu ile değiştirin — çalışan uygulama bu sistemde yürütülebilir dosya yolunu belirleyemedi.
settings-agent-note-data-dir = { -brand } varsayılan olmayan bir veri dizini ile çalışıyorsa, istemcide de SCREENERBOT_DATA_DIR değerini aynı yola ayarlayın (ek bir -e / --env bayrağı veya env girdisi).
settings-agent-note-codex-run = Komutu çalıştırın veya TOML bloğunu ~/.codex/config.toml ($CODEX_HOME/config.toml) dosyasına ekleyin. Ardından { -codex } uygulamasını yeniden başlatın.
settings-agent-note-codex-get = `codex mcp get screenerbot` çıktısında gizli anahtarı maskeler.
settings-agent-note-claude-code = { -claude } Code: komutu çalıştırın, ardından { -claude } Code uygulamasını yeniden başlatın. `claude mcp get screenerbot` gizli anahtar dahil yapılandırılmış ortamı yazdırır.
settings-agent-note-claude-desktop = { -claude } Desktop: JSON'u claude_desktop_config.json dosyasında `mcpServers` altına birleştirin ve uygulamayı yeniden başlatın.
settings-agent-note-openclaw = Komutu çalıştırın, ardından kaydedilen stdio sunucusunun başladığını ve araçları sunduğunu doğrulamak için `openclaw mcp doctor screenerbot --probe` komutunu kullanın.
settings-agent-note-hermes = Bunu { -hermes } yapılandırma dosyasında `mcp_servers` altına ekleyin, ardından { -hermes } uygulamasını yeniden başlatın.
settings-agent-note-generic = stdio konuşan herhangi bir MCP istemcisi: istemcinin sunucu listesini tuttuğu yerde bu komutu bu argümanlar ve ortamla çalıştırın.
settings-agent-block-codex-command = { -codex } CLI — terminal komutu
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (yedek)
settings-agent-block-claude-command = { -claude } Code — terminal komutu
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — terminal komutu
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = Genel stdio MCP istemcisi

settings-agent-name-required = Bu bağlantı için bir ad girin.
settings-agent-name-too-long = Ad en fazla { $max } karakter olmalıdır.
settings-agent-name-control-characters = Ad kontrol karakterleri içermemelidir.

settings-agent-title = Ajan Bağlantıları
settings-agent-description = { -claude }, { -codex }, { -hermes }, { -openclaw } veya herhangi bir stdio MCP istemcisini bağlayın. { -brand } çalışır durumda kalmalıdır. Her bağlantı kendi izinlerini taşır: varsayılan olarak tam erişim, istediğiniz zaman bağlantı başına sınırlanabilir. Hiçbir bağlantı cüzdan anahtarınızı okuyamaz veya değiştiremez.
settings-agent-name-label = Bağlantı adı
settings-agent-name-hint = Bağlantıları ayırt edebilmeniz için aşağıdaki listede gösterilir.
settings-agent-name-input =
    .placeholder = Dizüstü kodlama ajanı
settings-agent-client-label = İstemci
settings-agent-client-hint = Bağlantı oluşturulduktan sonra gösterilecek kurulumu seçer.
settings-agent-permissions-label = İzinler
settings-agent-permissions-hint = Yeni bir bağlantı her şeyi yapabilir. Herhangi bir kategoriyi şimdi veya daha sonra aşağıdaki listeden sınırlandırın — cüzdan anahtarlarına her iki durumda da erişilemez.
settings-agent-create = Bağlantı oluştur
settings-agent-issued-group =
    .aria-label = Yeni bağlantı kimlik bilgisi
settings-agent-issued-warning = Gizli anahtarı şimdi kopyalayın. Yalnızca bir kez gösterilir ve tekrar alınamaz — kaybederseniz bağlantıyı iptal edip yeniden oluşturun. { -brand } yalnızca tek yönlü bir doğrulayıcı tutar; MCP istemciniz açık metni kendi yapılandırmasında saklar.
settings-agent-issued-client-id = İstemci Kimliği
settings-agent-issued-secret = Tek kullanımlık gizli anahtar
settings-agent-setup-for = Kurulum:
settings-agent-done = Tamam
settings-agent-list-title = Bağlantılar
settings-agent-loading = Bağlantılar yükleniyor...
settings-agent-active-count = { $count } etkin
settings-agent-empty = Henüz bağlantı yok. Bir istemci eşlemek için yukarıdan bir tane oluşturun.
settings-agent-empty-active = Etkin bağlantı yok.
settings-agent-revoked-title = İptal edilen bağlantılar
settings-agent-created = Oluşturulma: { $time }
settings-agent-last-used = Son kullanım: { $time }
settings-agent-never-used = Hiç kullanılmadı
settings-agent-permissions-edit = İzinler
settings-agent-revoke = İptal et
settings-agent-permissions-save = İzinleri kaydet

settings-agent-load-failed = Ajan Bağlantıları yüklenemedi
settings-agent-list-failed = Bağlantılar yüklenemedi
settings-agent-create-failed = Bağlantı oluşturulamadı.
settings-agent-unreachable-create = Bağlantıyı oluşturmak için { -brand } uygulamasına ulaşılamadı.
settings-agent-permissions-update-failed = İzinler güncellenemedi
settings-agent-permissions-updated = İzinler güncellendi
settings-agent-permissions-updated-detail = Bağlantının bir sonraki isteğinde geçerli olur.
settings-agent-unreachable-save = Kaydetmek için { -brand } uygulamasına ulaşılamadı
settings-agent-revoke-title = Bağlantıyı iptal et
settings-agent-revoke-message = "{ $label }" bağlantısı iptal edilsin mi? İstemci bir sonraki isteğinde çalışmayı durdurur ve geri yüklenemez.
settings-agent-revoke-fallback-name = bu bağlantı
settings-agent-revoke-failed = Bağlantı iptal edilemedi
settings-agent-unreachable-revoke = İptal etmek için { -brand } uygulamasına ulaşılamadı

## telegram_tab.js

settings-telegram-loading = { -telegram } ayarları yükleniyor...
settings-telegram-load-failed = { -telegram } ayarları yüklenemedi
settings-telegram-unknown = Bilinmiyor
settings-telegram-session-active = Etkin: { $duration }
settings-telegram-sessions-empty = Etkin oturum yok
settings-telegram-session-revoke = İptal et

settings-telegram-connection-title = Bağlantı
settings-telegram-connection-description = Bildirim almak ve { -brand } uygulamasını uzaktan kontrol etmek için { -telegram } botunuzu bağlayın.
settings-telegram-enable-label = { -telegram } Etkinleştir
settings-telegram-enable-hint = { -telegram } bot entegrasyonunu etkinleştir
settings-telegram-token-label = Bot Token'ı
settings-telegram-token-saved = Token kaydedildi
settings-telegram-token-help = Bunu { -telegram } üzerinde @BotFather'dan alın
settings-telegram-token-input-saved =
    .placeholder = Token kaydedildi (değiştirmek için yenisini girin)
settings-telegram-token-input =
    .placeholder = Bot token'ını girin
settings-telegram-token-toggle =
    .title = Göster/Gizle
settings-telegram-chat-label = Sohbet Kimliği
settings-telegram-chat-connected = Bağlı sohbet:
settings-telegram-chat-discover-hint = Sohbet kimliğinizi otomatik olarak bulun
settings-telegram-chat-change =
    .title = Değiştir
settings-telegram-chat-discover = Sohbet Kimliğini Bul
settings-telegram-discovery-step-add = Botunuzu bir { -telegram } grubuna ekleyin veya onunla doğrudan sohbet başlatın
settings-telegram-discovery-step-privacy = Gruplar için: @BotFather → /mybots → [botunuz] → Bot Settings → Group Privacy yolunu kontrol edin
settings-telegram-discovery-privacy = <strong>Gizlilik Modu KAPALI:</strong> Bot tüm grup mesajlarını alır<br/><strong>Gizlilik Modu AÇIK:</strong> Bot yalnızca @etiketlendiğinde mesaj alır
settings-telegram-discovery-step-send = Herhangi bir mesaj gönderin (Gizlilik Modu AÇIK ise botunuzu @etiketleyin)
settings-telegram-discovery-listening = Mesajlar dinleniyor...
settings-telegram-discovery-select = Seç
settings-telegram-chat-id-label = Kimlik:
settings-telegram-language-label = Mesaj dili
settings-telegram-language-hint = { -telegram } bot mesajlarının ve düğmelerinin dili
settings-telegram-language-follow-app = Uygulama dilini izle
settings-telegram-test-label = Bağlantıyı Test Et
settings-telegram-test-hint = Yapılandırmayı doğrulamak için test mesajı gönder
settings-telegram-test-send = Test Gönder
settings-telegram-test-sending = Gönderiliyor...

settings-telegram-chat-type-private = özel
settings-telegram-chat-type-group = grup
settings-telegram-chat-type-supergroup = süper grup
settings-telegram-chat-type-channel = kanal

settings-telegram-auth-title = Komut Kimlik Doğrulaması
settings-telegram-auth-description = { -telegram } komutları, panel kilit ekranıyla aynı 2FA'yı kullanır.
settings-telegram-auth-protected = Korumalı
settings-telegram-auth-disabled = Devre dışı
settings-telegram-auth-not-configured = Yapılandırılmadı
settings-telegram-auth-error = Hata
settings-telegram-auth-protected-note = Komutlar kilit ekranı 2FA'sı ile korunur. Oturumlar sona erdiğinde kullanıcılar doğrulayıcı kodlarını <code>/login</code> komutuyla girmelidir.
settings-telegram-auth-disabled-note = Kilit ekranı 2FA'sı yapılandırılmış ancak { -telegram } için devre dışı. { -telegram } komutlarını korumak için yukarıdaki "Komutlar için 2FA iste" seçeneğini etkinleştirin.
settings-telegram-auth-missing-note = Kilit ekranı 2FA'sı yapılandırılmadı. 2FA olmadan süresi dolan oturumlar doğrulama yapılmadan otomatik olarak yeniden etkinleşir.
settings-telegram-auth-managed-in = 2FA şuradan yönetilir:
settings-telegram-auth-configure-in = 2FA yapılandırması için şuraya gidin:
settings-telegram-auth-configure-suffix = — böylece { -telegram } komutları için doğrulama zorunlu olur.
settings-telegram-security-link = Güvenlik Ayarları
settings-telegram-timeout-title = Oturum Zaman Aşımı
settings-telegram-timeout-description = Doğrulanmış bir oturumun ne kadar süre etkin kalacağı
settings-telegram-sessions-title = Etkin Oturumlar

settings-telegram-notifications-title = Bildirim Ayarları
settings-telegram-notifications-description = Hangi olayların { -telegram } bildirimi tetikleyeceğini seçin.
settings-telegram-notify-opened-label = Pozisyon Açıldı
settings-telegram-notify-opened-hint = Yeni bir pozisyon açıldığında bildir
settings-telegram-notify-closed-label = Pozisyon Kapandı
settings-telegram-notify-closed-hint = Bir pozisyon kapandığında bildir
settings-telegram-notify-partial-label = Kısmi Çıkış
settings-telegram-notify-partial-hint = Kısmi pozisyon çıkışlarında bildir
settings-telegram-notify-dca-label = DCA Yürütüldü
settings-telegram-notify-dca-hint = DCA emirleri yürütüldüğünde bildir
settings-telegram-notify-errors-label = Hatalar
settings-telegram-notify-errors-hint = Hata ve başarısızlıklarda bildir
settings-telegram-notify-startup-label = Açılış/Kapanış
settings-telegram-notify-startup-hint = Bot başladığında veya durduğunda bildir
settings-telegram-notify-filtering-label = Filtreleme Uyarıları
settings-telegram-notify-filtering-hint = Yeni tokenlar filtreleme ölçütlerini geçtiğinde bildir
settings-telegram-notify-trades-label = İşlem Uyarıları
settings-telegram-notify-trades-hint = İzlenen tokenlarda önemli işlemlerde bildir
settings-telegram-notify-daily-label = Günlük Özet
settings-telegram-notify-daily-hint = Günlük işlem etkinliği ve K/Z özeti al

settings-telegram-features-title = Özellikler
settings-telegram-features-description = { -telegram } bot yeteneklerini yapılandırın.
settings-telegram-commands-label = Komutları Etkinleştir
settings-telegram-commands-hint = Botun { -telegram } komutlarıyla kontrol edilmesine izin ver
settings-telegram-require-2fa-label = Komutlar için 2FA iste
settings-telegram-require-2fa-hint = Oturumlar sona erdiğinde yeniden etkinleştirmek için 2FA kodu iste. Kilit ekranı 2FA'sını kullanır.
settings-telegram-inline-label = Satır İçi Eylem Düğmeleri
settings-telegram-inline-hint = Bildirim mesajlarında eylem düğmelerini göster

settings-telegram-setting-save-failed = { -telegram } ayarı kaydedilemedi
settings-telegram-discovery-start-failed = Bulma işlemi başlatılamadı
settings-telegram-chat-selected = Sohbet seçildi
settings-telegram-chat-select-failed = Sohbet seçilemedi
settings-telegram-test-sent = Test mesajı gönderildi
settings-telegram-test-failed = Test mesajı başarısız oldu
settings-telegram-session-revoked = Oturum iptal edildi
settings-telegram-session-revoke-failed = Oturum iptal edilemedi

## licenses_tab.js

settings-licenses-title = Açık Kaynak Lisansları
settings-licenses-subtitle = { -brand } aşağıdaki açık kaynak yazılımlarla geliştirilmiştir
settings-licenses-footer = Lisansların tam metinleri proje deposunda ve her bağımlılığın kaynak kodunda mevcuttur.
settings-licenses-category-framework = Uygulama Çatısı
settings-licenses-category-solana = Solana Blokzinciri
settings-licenses-category-data = Veri ve Depolama
settings-licenses-category-networking = Ağ
settings-licenses-category-cryptography = Kriptografi ve Kodlama
settings-licenses-category-assets = Arayüz Varlıkları
settings-licenses-desc-electron = Masaüstü uygulama çatısı
settings-licenses-desc-tokio = Rust için asenkron çalışma zamanı
settings-licenses-desc-axum = Web sunucusu çatısı
settings-licenses-desc-tower = Servis soyutlamaları
settings-licenses-desc-hyper = HTTP uygulaması
settings-licenses-desc-solana-sdk = Solana SDK çekirdeği
settings-licenses-desc-solana-client = RPC istemcisi
settings-licenses-desc-solana-program = Program kütüphanesi
settings-licenses-desc-spl-token = SPL Token programı
settings-licenses-desc-spl-token-2022 = Token-2022 uzantıları
settings-licenses-desc-spl-associated-token-account = İlişkili token hesapları
settings-licenses-desc-sqlite = Gömülü veritabanı motoru
settings-licenses-desc-rusqlite = SQLite Rust bağlayıcıları
settings-licenses-desc-r2d2 = Veritabanı bağlantı havuzu
settings-licenses-desc-serde = Serileştirme çatısı
settings-licenses-desc-toml = Yapılandırma ayrıştırma
settings-licenses-desc-reqwest = HTTP istemcisi
settings-licenses-desc-tokio-tungstenite = WebSocket istemcisi
settings-licenses-desc-rustls = TLS uygulaması
settings-licenses-desc-blake3 = Özet fonksiyonu
settings-licenses-desc-sha-2 = SHA-256/512 özetleme
settings-licenses-desc-bs58 = Base58 kodlama
settings-licenses-desc-base64 = Base64 kodlama
settings-licenses-desc-lucide-icons = Simge yazı tipi kütüphanesi
settings-licenses-desc-inter = Arayüz yazı tipi
settings-licenses-desc-jetbrains-mono = Sabit aralıklı yazı tipi
settings-licenses-desc-orbitron = Başlık yazı tipi

## hints_tab.js

settings-hints-title = Bağlamsal İpuçları
settings-hints-description = Bağlamsal ipuçları, panel özelliklerini açıklayan yardım simgeleridir. Aşağıdaki tüm ipuçlarını inceleyin ve "Bir daha gösterme" ile gizlediklerinizi tek tek veya toplu olarak geri yükleyin.
settings-hints-hidden-label = Gizli İpuçları
settings-hints-hidden-summary = Toplam { $total } ipucundan { $hidden } tanesi şu anda gizli.
settings-hints-restore-all = Tüm İpuçlarını Geri Yükle
settings-hints-toggle-shown =
    .title = Bu ipucunu göster
settings-hints-toggle-shown-title = Gösteriliyor
settings-hints-toggle-hidden-title = Gizli — göstermek için açın
settings-hints-restore-title = Tüm İpuçlarını Geri Yükle
settings-hints-restore-message = Gizlediklerinizin tümü dahil, tüm bağlamsal ipuçları yeniden gösterilsin mi?
settings-hints-restore-confirm = Tümünü Geri Yükle
settings-hints-restored = Tüm ipuçları geri yüklendi

## account_tab.js

settings-account-title = { -brand } hesabı
settings-account-description = Ücretsiz ve isteğe bağlı. { -brand } hesap olmadan da işlem yapar, keşfeder ve grafik çizer — yalnızca herkese açık sağlayıcıları kullanır. Aşağıdaki panel, giriş yapmanın neler eklediğini listeler.
settings-account-data-title = { -brand } verileri
settings-account-data-description = screenerbot.io üzerinde ortak bir piyasa verisi hizmeti işletiyoruz: yedi zaman diliminde birleştirilmiş mumlar, çözümlenmiş havuz kaydı, önbelleğe alınmış güvenlik raporları ve normalleştirilmiş token kimliği. Her kurulumun herkese açık sağlayıcılar tarafından ayrı ayrı hız sınırına takılmaması için vardır; ortak maliyetin bir sahibi olması için kullanımı hesap gerektirir.
settings-account-data-fallback = Hizmet kullanılamadığında { -brand } otomatik olarak herkese açık sağlayıcılara geçer. Hiçbir şey durmaz; grafikler daha yavaş dolar ve daha az geçmiş içerir.
settings-account-gateway-title = İşlem gönderme
settings-account-gateway-description = Giriş yaptığınızda { -brand }, takaslarınızı kendi RPC'niz yerine screenerbot.io üzerinden yayınlayabilir. Botunuz her işlemi yine bu makinede oluşturur ve imzalar — sunucu yalnızca iletir ve imzalı bir işlemi imzasını geçersiz kılmadan değiştiremez.
settings-account-gateway-label = İşlem göndermek için { -brand } RPC'sini kullan
settings-account-gateway-hint = Yalnızca gönderim. Fiyat verileri her zaman kendi RPC'nizden gelir — havuz yoklaması ortak bir uç nokta için çok ağırdır, bu yüzden oraya asla gönderilmez.
settings-account-manage-title = Hesabınızı yönetme
settings-account-manage-description = Parolanız, e-posta adresiniz, bağlı cihazlarınız ve yönlendirme ödemeleriniz web sitesinde yönetilir. Orada bir cihazı iptal etmek, bu cihaz dahil her yerde oturumunu kapatır.
settings-account-open-dashboard = Panelinizi açın

## navigation_tab.js

settings-navigation-title = Gezinme Sekmeleri
settings-navigation-hint = Yeniden sıralamak için öğeleri sürükleyin. Görünürlüğü anahtarla değiştirin.
settings-navigation-note = Değişiklikler kaydettikten sonra uygulanır. Gezinme çubuğundaki güncellemeleri görmek için sayfayı yenileyin.
settings-navigation-drag-handle =
    .title = Yeniden sıralamak için sürükleyin
settings-navigation-defaults-failed = Varsayılan gezinme yüklenemedi
settings-navigation-reset = Gezinme varsayılanlara sıfırlandı

## data_tab.js

settings-data-storage-title = Veritabanı Depolaması
settings-data-storage-description = İşlem verilerinizi, pozisyonlarınızı ve geçmiş bilgilerinizi depolayan tüm veritabanlarına genel bakış.
settings-data-stats-loading = Veritabanı istatistikleri yükleniyor...
settings-data-stats-load-failed = Veritabanı istatistikleri yüklenemedi
settings-data-total-storage = Toplam Veritabanı Depolaması
settings-data-db-tokens = Tokenlar
settings-data-db-transactions = İşlemler
settings-data-db-positions = Pozisyonlar
settings-data-db-events = Olaylar
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = Cüzdan
settings-data-db-pools = Havuzlar
settings-data-db-strategies = Stratejiler
settings-data-db-actions = Eylemler
settings-data-directory-label = Veri Dizini
settings-data-directory-copied = Veri dizini
settings-data-config-path-copied = Yapılandırma yolu
settings-data-path-unavailable = Kullanılamıyor
settings-data-path-copy-title = Yolu kopyalamak için tıklayın
settings-data-path-copy-failed = Yol kopyalanamadı

settings-data-config-title = Yapılandırma Yönetimi
settings-data-config-description = Bot yapılandırmanızı dışa aktarın, içe aktarın ve yönetin. Büyük değişikliklerden önce yedek alın.
settings-data-config-export = Yapılandırmayı Dışa Aktar
settings-data-config-import = Yapılandırmayı İçe Aktar
settings-data-config-reset = Varsayılanlara Sıfırla
settings-data-config-location-label = Yapılandırma Konumu
settings-data-config-fetch-failed = Yapılandırma alınamadı
settings-data-config-exported = Yapılandırma dışa aktarıldı
settings-data-config-export-failed = Yapılandırma dışa aktarılamadı: { $message }
settings-data-config-import-title = Yapılandırmayı İçe Aktar
settings-data-config-import-message = Bu yapılandırma içe aktarılsın mı? Mevcut ayarların üzerine yazılacak. Cüzdan kimlik bilgileri korunacak.
settings-data-config-imported = Yapılandırma başarıyla içe aktarıldı. Bazı değişiklikler yeniden başlatma gerektirebilir.
settings-data-config-import-failed = Yapılandırma içe aktarılamadı: { $message }
settings-data-config-reset-title = Yapılandırmayı Sıfırla
settings-data-config-reset-message = Tüm ayarlar varsayılanlara sıfırlansın mı? Cüzdan kimlik bilgileriniz korunacak, ancak diğer tüm ayarlar sıfırlanacak.
settings-data-config-reset-done = Yapılandırma varsayılanlara sıfırlandı
settings-data-config-reset-failed = Yapılandırma sıfırlanamadı: { $message }
settings-data-unknown-error = Bilinmeyen hata

settings-data-cleanup-title = Veri Temizliği
settings-data-cleanup-description = Eski veya kullanılmayan verileri kaldırarak disk alanı açın. Bu işlemler geri alınamaz.
settings-data-ohlcv-cleanup-label = OHLCV Veri Temizliği
settings-data-ohlcv-cleanup-hint = Belirtilen süredir etkin olmayan tokenların mum verilerini kaldırın.
settings-data-cleanup-hours-unit = saat
settings-data-cleanup-ohlcv = OHLCV'yi Temizle
settings-data-cleanup-running = Temizleniyor...
settings-data-cleanup-hours-invalid = Geçersiz saat değeri
settings-data-cleanup-confirm-title = OHLCV Verilerini Sil
settings-data-cleanup-confirm-message =
    { $hours ->
        [one] { $hours } saatten
       *[other] { $hours } saatten
    } uzun süredir etkin olmayan tokenların OHLCV verileri silinsin mi?
settings-data-cleanup-done =
    { $count ->
        [one] { $count } etkin olmayan token temizlendi
       *[other] { $count } etkin olmayan token temizlendi
    }
settings-data-cleanup-failed = Temizlik başarısız oldu
settings-data-cleanup-failed-detail = Temizlik başarısız oldu: { $message }

settings-data-cache-clear-label = Tüm OHLCV Önbelleğini Temizle
settings-data-cache-clear-hint = Önbelleğe alınmış tüm mum verilerini silin ve izlenen her tokenı baştan yeniden alın. Grafikler hatalı görünüyorsa veya bir veri mantığı güncellemesinden sonra kullanın.
settings-data-cache-clear = OHLCV Önbelleğini Temizle
settings-data-cache-clearing = Temizleniyor...
settings-data-cache-confirm-title = Tüm OHLCV Önbelleğini Temizle
settings-data-cache-confirm-message = Her token için önbelleğe alınmış tüm mum verileri silinsin mi? İzlenen tokenlar geçmişlerini baştan yeniden alacak. Bu işlem geri alınamaz.
settings-data-candles-count =
    { $count ->
        [one] { $count } mum
       *[other] { $count } mum
    }
settings-data-tokens-count =
    { $count ->
        [one] { $count } token
       *[other] { $count } token
    }
settings-data-cache-cleared = Temizlendi: { $candles }, { $tokens } için; yeniden alınıyor
settings-data-cache-clear-failed = OHLCV önbelleği temizlenemedi
settings-data-cache-clear-failed-detail = OHLCV önbelleği temizlenemedi: { $message }

settings-data-ui-cache-label = Arayüz Durumu Önbelleği
settings-data-ui-cache-hint = Kayıtlı tablo tercihlerini, filtre durumlarını ve görünüm ayarlarını temizleyin.
settings-data-ui-cache-clear = Arayüz Önbelleğini Temizle
settings-data-ui-cache-confirm-title = Arayüz Durumunu Temizle
settings-data-ui-cache-confirm-message = Kayıtlı tüm arayüz tercihleri temizlensin mi? Bu işlem tablo sütunlarını, filtreleri ve görünüm ayarlarını sıfırlar.
settings-data-ui-cache-cleared =
    { $count ->
        [one] Önbellekteki { $count } arayüz ayarı temizlendi
       *[other] Önbellekteki { $count } arayüz ayarı temizlendi
    }

settings-data-folder-label = Veri Klasörünü Aç
settings-data-folder-hint = Tüm { -brand } verilerini içeren klasörü dosya yöneticinizde açın.
settings-data-folder-open = Klasörü Aç
settings-data-folder-open-failed = Veri klasörü açılamadı
