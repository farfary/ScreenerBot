errors-with-details = { $message }: { $details }

# Configuration
errors-config-save-failed = Yapılandırma kaydedilemedi

# Authentication
errors-auth-current-password-incorrect = Geçerli parola yanlış
errors-auth-current-password-required = Parolayı değiştirmek için geçerli parola gerekli
errors-auth-password-too-short = Parola en az 4 karakter olmalı
errors-auth-password-too-long = Parola en fazla 128 karakter olmalı
errors-auth-hash-failed = Parola özetlenemedi
errors-auth-not-enabled = Kimlik doğrulama etkin değil
errors-auth-no-password = Parola yapılandırılmamış
errors-auth-password-incorrect = Parola yanlış
errors-auth-required = Kimlik doğrulama gerekli. Bu uç noktaya erişmek için lütfen giriş yapın.
errors-auth-totp-invalid = 2FA kodu geçersiz veya süresi dolmuş
errors-auth-totp-verify-failed = 2FA kodu doğrulanamadı
errors-auth-totp-password-required = 2FA etkinleştirilmeden önce parola belirlenmeli
errors-auth-totp-uri-failed = TOTP URI'si oluşturulamadı
errors-auth-totp-qr-failed = QR kod oluşturulamadı
errors-auth-totp-secret-required = Gizli anahtar gerekli
errors-auth-totp-save-failed = TOTP yapılandırması kaydedilemedi
errors-auth-totp-code-invalid = Doğrulama kodu geçersiz. Lütfen kodu kontrol edip tekrar deneyin.
errors-auth-totp-code-verify-failed = Kod doğrulanamadı

# Request security
errors-security-invalid-local-request = İstek yerel panelden gelmelidir
errors-security-invalid-token = Güvenlik belirteci geçersiz
errors-security-token-required = Güvenlik belirteci gerekli. Bu uç noktaya yalnızca { -brand } içinden erişilebilir.

# Lockscreen
errors-lockscreen-no-password-set = Parola belirlenmemiş
errors-lockscreen-invalid-type = Parola türü geçersiz. 'pin4', 'pin6' veya 'text' olmalı
errors-lockscreen-invalid-format = Parola seçilen türle eşleşmiyor
errors-lockscreen-no-current-password = Şu anda belirlenmiş bir parola yok
errors-lockscreen-password-incorrect = Parola yanlış
errors-lockscreen-enable-needs-password = Önce parola belirlemeden kilit ekranı etkinleştirilemez

# Account
errors-account-signin-failed = Giriş başarısız oldu
errors-account-signin-refused = { $reason }
errors-account-signin-unavailable = Giriş kullanılamıyor
errors-account-browser-open-failed = Tarayıcınız açılamadı. Varsayılan tarayıcınızı açıp tekrar deneyin.
errors-account-signup-open-failed = Kayıt sayfası açılamadı. Tarayıcınızda screenerbot.io/signup adresini açın.
errors-account-credentials-required = E-posta adresinizi ve parolanızı girin.
errors-account-signout-failed = Çıkış başarısız oldu
errors-account-gateway-update-failed = Ağ geçidi ayarı güncellenemedi

# Localization
errors-i18n-locale-not-registered = Dil kayıtlı değil
errors-i18n-catalog-encode-failed = Katalog kodlanamadı

# System
errors-system-paths-init-failed = Uygulama klasörleri oluşturulamadı
errors-system-open-data-failed = Veri klasörü açılamadı
errors-system-url-empty = URL boş olamaz
errors-system-open-url-failed = URL açılamadı

# Initialization
errors-initialization-required = Bu uç noktaya erişmeden önce bot kurulumu gerekli. Lütfen kurulumu web arayüzünden tamamlayın.
errors-initialization-onboarding-update-failed = Karşılama durumu güncellenemedi
errors-initialization-validation-required = Kurulumu kaydetmeden önce kimlik bilgilerinin doğrulanması gerekli
errors-initialization-encrypt-failed = Özel anahtar şifrelenemedi

# Dashboard state
errors-ui-state-save-failed = Durum kaydedilemedi
errors-ui-state-clear-failed = Durum temizlenemedi

# Agent control
errors-agent-config-failed = Ajan kontrolü yapılandırması başarısız oldu
errors-agent-invalid-parameters = Parametreler geçersiz
errors-agent-wallet-key-material = Cüzdan anahtar materyaline ajan erişemez
errors-agent-store-failed = Ajan kontrolü deposu başarısız oldu
errors-agent-invalid-pairing-request = Eşleştirme isteği geçersiz
errors-agent-pairing-rejected = Eşleştirme kimlik bilgisi reddedildi
errors-agent-disabled = Ajan kontrolü devre dışı
errors-agent-approval-not-pending = Onay artık beklemede değil
errors-agent-approval-not-found = Onay bulunamadı
errors-agent-bridge-task-failed = Ajan kontrolü köprü görevi başarısız oldu
errors-agent-task-failed = Ajan kontrolü görevi başarısız oldu
errors-agent-pairing-not-found = Bu kimliğe sahip etkin eşleştirme yok
errors-agent-permissions-update-failed = İzinler güncellenemedi

# Connectivity
errors-connectivity-endpoint-not-found = '{ $endpoint }' uç noktası bulunamadı veya izlenmiyor

# Copy trading
errors-copy-task-not-found = Kopyalama görevi bulunamadı
errors-copy-holding-not-found = Bu tokende açık sanal varlık yok
errors-copy-live-confirmation-required = Canlı kopya işlemi etkinleştirmek açık onay gerektirir
errors-copy-task-invalid = Kopyalama görevi geçersiz
errors-copy-request-rejected = Kopya işlem isteği reddedildi
errors-copy-task-limit = Etkin kopyalama görevi üst sınırına ulaşıldı
errors-copy-watch-rejected = Kopya hedefi izlenemedi
errors-copy-live-unavailable = Canlı kopya işlem kullanılamıyor
errors-copy-task-live = Silmeden önce canlı görevi duraklatın
errors-copy-task-owns-positions = Kopyalama görevinin hâlâ açık pozisyonları var
errors-copy-request-failed = Kopya işlem isteği başarısız oldu

# Strategies
errors-strategies-not-found = Strateji bulunamadı
errors-strategies-invalid-type = Strateji türü geçersiz. ENTRY veya EXIT olmalı
errors-strategies-list-failed = Stratejiler alınamadı
errors-strategies-get-failed = Strateji alınamadı
errors-strategies-serialize-rules-failed = Kurallar serileştirilemedi
errors-strategies-invalid-rules-json = Kurallar JSON'u geçersiz
errors-strategies-already-exists = '{ $id }' kimlikli strateji zaten var
errors-strategies-validation-failed = Strateji doğrulaması başarısız oldu
errors-strategies-create-failed = Strateji oluşturulamadı
errors-strategies-update-failed = Strateji güncellenemedi
errors-strategies-update-enabled-failed = Stratejinin etkin durumu güncellenemedi
errors-strategies-delete-failed = Strateji silinemedi
errors-strategies-deploy-failed = Strateji dağıtılamadı
errors-strategies-no-performance = Bu strateji için performans verisi yok
errors-strategies-performance-failed = Performans istatistikleri alınamadı
errors-strategies-schemas-failed = Koşul şemaları alınamadı
errors-strategies-evaluation-failed = Strateji değerlendirmesi başarısız oldu

# Transactions
errors-transactions-own-wallet-unavailable = Ana cüzdan yapılandırılmamış
errors-transactions-invalid-subject = İşlem öznesi geçerli bir Solana adresi değil
errors-transactions-subject-not-watched = İşlem öznesi izlenen bir cüzdan değil
errors-transactions-watch-store-unavailable = İzlenen cüzdanlar kullanılamıyor

# Wallet
errors-wallet-unavailable = Ana cüzdan kullanılamıyor
errors-wallet-changed = Ana cüzdan değişti; sayfayı yenileyip tekrar deneyin
errors-wallet-qr-failed = Cüzdan QR kodu oluşturulamadı

# Updates
errors-updates-none-available = İndirilecek güncelleme yok
errors-updates-version-changed = Mevcut güncelleme değişti; güncellemeleri yeniden denetleyin
errors-updates-check-failed = Güncelleme denetimi başarısız oldu
errors-updates-download-failed = Güncelleme indirmesi başlatılamadı
errors-updates-history-unavailable = Sürüm geçmişi kullanılamıyor
errors-updates-apply-failed = Güncelleme uygulanamadı
errors-updates-install-failed = Güncelleme yükleyicisi açılamadı

# Telegram
errors-telegram-settings-update-failed = Ayarlar güncellenemedi
errors-telegram-disabled = { -telegram } etkin değil
errors-telegram-not-configured = Bot belirteci veya sohbet kimliği yapılandırılmamış
errors-telegram-send-failed = Mesaj gönderilemedi
errors-telegram-notifier-failed = Bildirici oluşturulamadı
errors-telegram-token-required = Önce bot belirteci yapılandırılmalı
errors-telegram-discovery-failed = Keşif başlatılamadı
errors-telegram-chat-select-failed = Sohbet seçilemedi

# Assistant chat
errors-chat-message-empty = Mesaj boş olamaz
errors-chat-message-too-long = Mesaj 10.000 karakterlik üst sınırı aşıyor
errors-chat-database-unavailable = Sohbet veritabanı başlatılmadı
errors-chat-session-not-found = Sohbet oturumu bulunamadı: { $id }
errors-chat-session-validate-failed = Oturum doğrulanamadı
errors-chat-engine-unavailable = Sohbet motoru başlatılmadı
errors-chat-process-failed = Sohbet mesajı işlenemedi
errors-chat-stream-serialize-failed = Sohbet olayı serileştirilemedi
errors-chat-sessions-list-failed = Sohbet oturumları listelenemedi
errors-chat-session-create-failed = Sohbet oturumu oluşturulamadı
errors-chat-session-get-failed = Sohbet oturumu alınamadı
errors-chat-messages-get-failed = Sohbet mesajları alınamadı
errors-chat-session-delete-failed = Sohbet oturumu silinemedi
errors-chat-messages-load-failed = Mesajlar alınamadı
errors-chat-summarize-empty = Boş sohbet oturumu özetlenemez
errors-chat-provider-invalid = Geçersiz sağlayıcı: { $provider }
errors-chat-summary-save-failed = Özet kaydedilemedi
errors-chat-title-empty-session = Boş sohbet oturumu için başlık oluşturulamaz
errors-chat-no-user-message = Oturumda kullanıcı mesajı bulunamadı
errors-chat-title-save-failed = Oturum başlığı güncellenemedi
errors-chat-confirmation-save-failed = Onay yanıtı kaydedilemedi
errors-chat-confirmation-failed = Onay işlenemedi
errors-chat-summary-failed = Özet oluşturulamadı

# Assistant automation
errors-automation-database-unavailable = Veritabanı başlatılmadı
errors-automation-tasks-list-failed = Görevler listelenemedi
errors-automation-name-empty = Görev adı boş olamaz
errors-automation-instruction-empty = Görev talimatı boş olamaz
errors-automation-schedule-type-invalid = schedule_type geçersiz. interval, daily veya weekly olmalı
errors-automation-schedule-value-invalid = schedule_value geçersiz
errors-automation-task-create-failed = Görev oluşturulamadı
errors-automation-task-not-found = Görev bulunamadı
errors-automation-task-get-failed = Görev alınamadı
errors-automation-schedule-invalid = Zamanlama geçersiz
errors-automation-tool-permissions-invalid = tool_permissions 'full' veya 'readonly' olmalı
errors-automation-priority-invalid = priority 'low', 'medium' veya 'high' olmalı
errors-automation-task-update-failed = Görev güncellenemedi
errors-automation-task-running-delete = Görev çalışırken silinemez
errors-automation-task-delete-failed = Görev silinemedi
errors-automation-task-toggle-failed = Görev durumu değiştirilemedi
errors-automation-task-disabled = Devre dışı bir görev çalıştırılamaz
errors-automation-task-already-running = Görev zaten çalışıyor
errors-automation-runs-list-failed = Çalıştırmalar listelenemedi
errors-automation-recent-runs-failed = Son çalıştırmalar listelenemedi
errors-automation-run-not-found = Çalıştırma bulunamadı
errors-automation-run-get-failed = Çalıştırma alınamadı
errors-automation-stats-failed = İstatistikler alınamadı

# LLM providers
errors-llm-config-update-failed = LLM yapılandırması güncellenemedi
errors-llm-provider-unknown = Bilinmeyen sağlayıcı: { $provider }
errors-llm-manager-unavailable = LLM yöneticisi başlatılmadı
errors-llm-provider-disabled = '{ $provider }' sağlayıcısı yapılandırılmamış veya devre dışı
errors-llm-provider-config-update-failed = Sağlayıcı yapılandırması güncellenemedi
errors-llm-provider-test-failed = Sağlayıcı testi başarısız oldu
errors-llm-provider-refused = { $reason }

# LLM analysis
errors-llm-analysis-config-update-failed = Analiz yapılandırması güncellenemedi
errors-llm-analysis-unavailable = Analiz motoru başlatılmadı
errors-llm-analysis-disabled = LLM özellikleri devre dışı. Önce [llm] bölümünü etkinleştirin.
errors-llm-analysis-priority-invalid = Öncelik geçersiz: '{ $priority }'. 'high', 'medium' veya 'low' kullanın.
errors-llm-analysis-evaluation-failed = Model analizi başarısız oldu
errors-llm-analysis-instructions-list-failed = Talimatlar listelenemedi
errors-llm-analysis-instruction-not-found = Talimat bulunamadı: { $id }
errors-llm-analysis-instruction-get-failed = Talimat alınamadı
errors-llm-analysis-instruction-created-retrieve-failed = Oluşturulan talimat alınamadı
errors-llm-analysis-instruction-create-failed = Talimat oluşturulamadı
errors-llm-analysis-instruction-updated-retrieve-failed = Güncellenen talimat alınamadı
errors-llm-analysis-instruction-update-failed = Talimat güncellenemedi
errors-llm-analysis-instruction-delete-failed = Talimat silinemedi
errors-llm-analysis-instructions-reorder-failed = Talimatlar yeniden sıralanamadı
errors-llm-analysis-decisions-list-failed = Karar geçmişi listelenemedi
errors-llm-analysis-decision-not-found = Karar bulunamadı: { $id }
errors-llm-analysis-decision-get-failed = Karar alınamadı

# Wallets
errors-wallets-list-failed = Cüzdanlar listelenemedi
errors-wallets-name-empty = Cüzdan adı boş olamaz
errors-wallets-create-failed = Cüzdan oluşturulamadı
errors-wallets-key-empty = Özel anahtar boş olamaz
errors-wallets-already-exists = Cüzdan zaten var
errors-wallets-key-invalid = Özel anahtar biçimi geçersiz
errors-wallets-import-failed = Cüzdan içe aktarılamadı
errors-wallets-summary-failed = Cüzdan özeti alınamadı
errors-wallets-no-main-wallet = Ana cüzdan yapılandırılmamış
errors-wallets-main-get-failed = Ana cüzdan alınamadı
errors-wallets-not-found = Cüzdan bulunamadı
errors-wallets-get-failed = Cüzdan alınamadı
errors-wallets-update-failed = Cüzdan güncellenemedi
errors-wallets-delete-failed = Cüzdan silinemedi
errors-wallets-export-failed = Cüzdan dışa aktarılamadı
errors-wallets-set-main-failed = Ana cüzdan ayarlanamadı
errors-wallets-archive-failed = Cüzdan arşivlenemedi
errors-wallets-restore-failed = Cüzdan geri yüklenemedi
errors-wallets-export-format-unsupported = Şu anda yalnızca CSV biçimi destekleniyor
errors-wallets-export-confirmation-required = Şunu girerek onaylamalısınız: "{ $confirmation }"
errors-wallets-export-no-ids = Cüzdan kimliği verilmedi
errors-wallets-export-bulk-failed = Cüzdanlar dışa aktarılamadı
errors-wallets-export-no-match = Verilen kimliklerle eşleşen cüzdan bulunamadı
errors-wallets-import-file-too-large = Dosya { $megabytes }MB üst sınırını aşıyor
errors-wallets-import-read-failed = Yüklenen dosya okunamadı
errors-wallets-import-no-file = Dosya yüklenmedi. Çok parçalı formda 'file' alanını kullanın
errors-wallets-import-encoding-invalid = CSV dosyası UTF-8 kodlamalı olmalı
errors-wallets-import-csv-parse-failed = CSV dosyası ayrıştırılamadı
errors-wallets-import-excel-parse-failed = Excel dosyası ayrıştırılamadı
errors-wallets-import-format-unsupported = Desteklenmeyen dosya biçimi. .csv, .xlsx veya .xls kullanın
errors-wallets-import-file-empty = Dosyada veri satırı yok
errors-wallets-import-existing-check-failed = Mevcut cüzdanlar denetlenemedi
errors-wallets-import-mapping-invalid = Gerekli sütunlar eksik: { $columns }
errors-wallets-import-session-not-found = İçe aktarma oturumu bulunamadı veya süresi doldu. Lütfen dosyayı yeniden yükleyin
errors-wallets-import-no-valid-rows = İçe aktarılacak geçerli satır yok

# Wallet watching
errors-wallet-watch-list-failed = İzleme hedefleri listelenemedi
errors-wallet-watch-address-empty = Adres boş olamaz
errors-wallet-watch-add-failed = İzleme hedefi eklenemedi
errors-wallet-watch-remove-failed = İzleme hedefi kaldırılamadı
errors-wallet-watch-update-failed = İzleme hedefi güncellenemedi
errors-wallet-watch-budget-failed = İzleme bütçesi güncellenemedi
errors-wallet-watch-resume-failed = İzleme sürdürülemedi
errors-wallet-watch-approval-failed = { -helius } onayı güncellenemedi
errors-wallet-watch-status-failed = İzleme durumu alınamadı

# Tools
errors-tools-wallet-failed = Cüzdan alınamadı
errors-tools-wallet-address-failed = Cüzdan adresi alınamadı
errors-tools-accounts-scan-failed = Hesaplar taranamadı
errors-tools-token-accounts-scan-failed = Token hesapları taranamadı
errors-tools-token-accounts-get-failed = Token hesapları alınamadı
errors-tools-cleanup-failed = Temizlik başarısız oldu
errors-tools-cache-clear-failed = Önbellek temizlenemedi
errors-tools-no-tokens = Yakmak için token seçilmedi
errors-tools-burn-failed = Tokenlar yakılamadı
errors-tools-favorites-list-failed = Favoriler alınamadı
errors-tools-favorite-type-invalid = Araç türü geçersiz. Şunlardan biri olmalı: { $types }
errors-tools-favorite-add-failed = Favori eklenemedi
errors-tools-favorite-not-found = Favori bulunamadı
errors-tools-favorite-update-failed = Favori güncellenemedi
errors-tools-favorite-delete-failed = Favori silinemedi
errors-tools-favorite-use-failed = Kullanım sayısı güncellenemedi
errors-tools-pool-search-failed = Token için havuz araması başarısız oldu: { $mint }
errors-tools-watched-list-failed = İzlenen tokenlar listelenemedi
errors-tools-watched-add-failed = İzlenen token eklenemedi
errors-tools-watched-delete-failed = İzlenen token silinemedi
errors-tools-mint-invalid = Token mint adresi geçersiz
errors-tools-wallets-get-failed = Cüzdanlar alınamadı
errors-tools-balance-failed = Cüzdan bakiyesi alınamadı
errors-tools-session-active = Başka bir çoklu cüzdan işlemi zaten sürüyor
errors-tools-config-invalid = Araç yapılandırması geçersiz: { $reason }
errors-tools-config-rejected = Araç yapılandırması geçersiz
errors-tools-consolidate-failed = Cüzdanlar birleştirilemedi
errors-tools-ata-cleanup-failed = ATA temizliği başarısız oldu
errors-tools-routers-unavailable = Takas yönlendiricileri henüz hazır değil
errors-tools-router-disabled-chain-settings = { $router } Ayarlar > Zincirler bölümünde devre dışı
errors-tools-router-unknown = Bilinmeyen takas yönlendiricisi: '{ $router }'
errors-tools-session-type-mismatch = Oturum türü { $actual }, beklenen { $expected }
errors-tools-session-not-found = Oturum bulunamadı
errors-tools-session-complete = Oturum zaten tamamlandı

# Configuration import and reload
errors-config-reload-failed = Yapılandırma yeniden yüklenemedi
errors-config-reset-failed = Yapılandırma sıfırlanamadı
errors-config-disk-parse-failed = Diskteki yapılandırma ayrıştırılamadı
errors-config-disk-read-failed = Diskteki yapılandırma okunamadı
errors-config-update-failed = Yapılandırma güncellenemedi
errors-config-import-not-object = Yapılandırma bir JSON nesnesi olmalı
errors-config-import-no-sections = İçe aktarılacak geçerli bölüm bulunamadı
errors-config-import-validation-failed = Yapılandırma doğrulaması başarısız oldu. Hiçbir değişiklik uygulanmadı.
errors-config-import-commit-failed = Yapılandırma değişiklikleri kaydedilemedi
errors-config-import-failed = Yapılandırma içe aktarılamadı

# Filtering
errors-filtering-analytics-failed = Analizler alınamadı
errors-filtering-refresh-failed = Filtreleme anlık görüntüsü yeniden oluşturulamadı
errors-filtering-rejection-stats-failed = Reddedilme istatistikleri alınamadı
errors-filtering-rejected-tokens-failed = Reddedilen tokenlar alınamadı
errors-filtering-csv-header-failed = CSV başlığı yazılamadı
errors-filtering-csv-record-failed = CSV kaydı yazılamadı
errors-filtering-csv-finalize-failed = CSV tamamlanamadı
errors-filtering-export-response-failed = Yanıt oluşturulamadı

# OHLCV
errors-ohlcv-fetch-failed = OHLCV verileri alınamadı
errors-ohlcv-pools-failed = Havuzlar alınamadı
errors-ohlcv-gaps-failed = Boşluklar alınamadı
errors-ohlcv-refresh-failed = Yenilenemedi
errors-ohlcv-monitor-start-failed = İzleme başlatılamadı
errors-ohlcv-monitor-stop-failed = İzleme durdurulamadı
errors-ohlcv-activity-failed = Etkinlik kaydedilemedi
errors-ohlcv-list-failed = OHLCV tokenları listelenemedi
errors-ohlcv-delete-failed = Token verileri silinemedi
errors-ohlcv-clear-failed = OHLCV önbelleği temizlenemedi
errors-ohlcv-cleanup-failed = Etkin olmayan tokenlar temizlenemedi

# Trader and manual trading
errors-trade-already-running = Trader zaten çalışıyor
errors-trade-already-stopped = Trader zaten durdurulmuş
errors-trade-config-update-failed = Trader yapılandırması güncellenemedi
errors-trade-trader-unavailable = Otomatik trader'ı kullanmadan önce cüzdan ve RPC kurulumunu tamamlayın
errors-trade-force-stop-active = Acil durdurma etkin; önce onu kaldırın
errors-trade-template-not-found = Şu adla bir trader şablonu yok: { $template }
errors-trade-manual-force-stopped = Acil durdurma etkinken manuel işlem devre dışıdır
errors-trade-core-services-not-ready = Çekirdek hizmetler işleme hazır değil: { $pending }
errors-trade-mint-invalid = Token mint adresi geçersiz: { $mint }
errors-trade-blacklisted = Token kara listede: { $mint }
errors-trade-slippage-invalid = Kayma %{ $slippage } değeri (0, { $maximum }] aralığında olmalı
errors-trade-percentage-invalid = Satış yüzdesi { $percentage } değeri (0, 100] aralığında olmalı
errors-trade-record-failed = Manuel işlem kaydedilemedi
errors-trade-task-cancelled = Uygulama kapandığı için manuel işlem yanıt vermeden durdu; sonucu pozisyonda yer alır
errors-trade-no-open-position = Bu token için açık pozisyon yok: { $mint }
errors-trade-size-invalid = İşlem boyutu geçersiz: { $amount } { -sol }
errors-trade-management-invalid = Pozisyon yönetimi geçersiz: { $management }
errors-trade-strategy-evaluation-failed = Token için strateji değerlendirmesi başarısız oldu: { $mint }
errors-trade-token-data-missing = Token verisi kullanılamıyor: { $mint }
errors-trade-endpoints-unhealthy = Sağlıklı uç nokta yok
errors-trade-dependency-failed = Bağımlılık başarısız oldu: { $dependency }
errors-trade-storage-failed = İşlem isteği tamamlanamadı
errors-trade-manual-failed = Manuel işlem başarısız oldu
errors-trade-manual-refused = { $reason }
errors-trade-swap-too-large = Hiçbir takas rotası gönderilecek kadar küçük bir işlem oluşturamadı
    .hint = En iyi rota, tek bir işlemin taşıyabileceğinden daha fazla hesap gerektirdi; bu yüzden hiçbir şey gönderilmedi ve harcanmadı. Farklı bir rota için birazdan tekrar deneyin veya başka bir takas yönlendiricisini etkinleştirin.
errors-trade-wallet-not-configured = Cüzdan yapılandırılmamış
errors-trade-amount-sol-invalid = Alım için amount_sol gerekli ve pozitif olmalı
errors-trade-no-tokens-in-wallet = Bu pozisyon için cüzdanda token bulunamadı. Token bakiyesi 0; pozisyon takasla kapatılamaz.
errors-trade-percentage-range = percentage (0, 100] aralığında olmalı
errors-trade-amount-tokens-invalid = amount_tokens pozitif olmalı
errors-trade-sell-amount-zero = Hesaplanan satış miktarı sıfır

# Swap quotes. The message is the dialog headline; `.hint` is what the user can do.
errors-trade-quote-registry-unavailable = Takas yönlendirmesi henüz hazır değil
    .hint = Takas hizmeti hâlâ başlıyor. Hizmetlerin hazır olmasını bekleyin, ardından tekrar deneyin.
errors-trade-quote-no-routers-enabled = Etkin takas sağlayıcısı yok
    .hint = Trader ayarlarında en az bir takas yönlendiricisini etkinleştirin, ardından tekrar deneyin.
errors-trade-quote-not-tradable = Bu token şu anda işlem görmüyor
    .hint = Likidite veya takas rotası yok. Token henüz başlatılmamış, terk edilmiş olabilir veya havuzu olmayabilir. Daha sonra tekrar deneyin veya başka bir token seçin.
errors-trade-quote-no-route = Kullanılabilir takas rotası yok
    .hint = Hiçbir sağlayıcı bu işlemi istenen miktarda yönlendiremedi. Daha küçük bir miktar deneyin veya biraz sonra tekrar deneyin.
errors-trade-quote-rate-limited = Takas sağlayıcıları isteklerimizi sınırlıyor
    .hint = Takas sağlayıcıları istekleri kısıtlıyor. Birkaç saniye bekleyip tekrar deneyin.
errors-trade-quote-timeout = Fiyat teklifi isteği zaman aşımına uğradı
    .hint = Takas sağlayıcıları zamanında yanıt vermedi. Bağlantınızı kontrol edip tekrar deneyin.
errors-trade-quote-router-rejected = Fiyat teklifi reddedildi
    .hint = Bir sağlayıcı güvenlik denetimlerimizden geçemeyen bir fiyat teklifi döndürdü ve atıldı. Yeni bir teklif almak için tekrar deneyin.
errors-trade-quote-not-offered-exact-out = Etkin hiçbir takas yönlendiricisi kesin çıktı miktarı için fiyat teklifi vermiyor
    .hint = Etkin takas yönlendiricileri bir işlemi yalnızca harcanan tutardan fiyatlar. Harcanacak tutarı girin veya başka bir takas yönlendiricisini etkinleştirin.
errors-trade-quote-not-offered-unsupported-venue = Etkin hiçbir takas yönlendiricisi bu tokenin havuzunda işlem yapmıyor
    .hint = Bu token, etkin takas yönlendiricilerinin henüz desteklemediği bir borsada işlem görüyor. Başka bir takas yönlendiricisini etkinleştirip tekrar deneyin.
errors-trade-quote-unavailable = Fiyat teklifi alınamadı
    .hint = Takas sağlayıcıları bu işlem için fiyat teklifi veremedi. Biraz sonra tekrar deneyin.

# Positions
errors-positions-not-found = Pozisyon bulunamadı
errors-positions-already-closed = Pozisyon zaten kapalı
errors-positions-force-close-failed = Pozisyon zorla kapatılamadı
errors-positions-already-archived = Pozisyon zaten arşivlenmiş
errors-positions-not-archived = Pozisyon arşivlenmemiş
errors-positions-archive-failed = Pozisyon arşivlenemedi
errors-positions-unarchive-failed = Pozisyon arşivden çıkarılamadı
errors-positions-management-invalid = Kopyaya ait yönetim, kopya kaynaklı bir pozisyon gerektirir
errors-positions-management-failed = Pozisyon yönetimi güncellenemedi
errors-positions-delete-failed = Pozisyon silinemedi
errors-positions-bulk-delete-failed = Arşivlenmiş pozisyonlar silinemedi
errors-positions-detail-failed = Pozisyon ayrıntıları yüklenemedi
errors-positions-resolve-failed = Pozisyon çözümlenemedi
errors-positions-wrapped-sol-activity = Wrapped SOL için token etkinliği yok

# Tokens
errors-tokens-database-unavailable = Token veritabanı kullanılamıyor
errors-tokens-blacklist-failed = Token kara listeye alınamadı
errors-tokens-blacklist-internal = Kara listeye alma işleminde iç hata
errors-tokens-unblacklist-failed = Kara listeden kaldırılamadı
errors-tokens-unblacklist-internal = Kara listeden kaldırma işleminde iç hata
errors-tokens-blacklist-status-failed = Kara liste durumu denetlenemedi
errors-tokens-blacklist-status-internal = Kara liste durumu denetiminde iç hata
errors-tokens-favorites-fetch-failed = Favoriler alınamadı
errors-tokens-favorite-add-failed = Favori eklenemedi
errors-tokens-favorite-remove-failed = Favori kaldırılamadı
errors-tokens-favorite-update-failed = Favori güncellenemedi
errors-tokens-detail-not-found = Token veritabanında veya harici kaynaklarda bulunamadı
errors-tokens-fetch-failed = Token alınamadı
errors-tokens-refresh-all-failed = Tüm veri kaynakları başarısız oldu
errors-tokens-refresh-failed = Token yenilenemedi
errors-tokens-search-query-required = 'q' arama sorgusu gerekli
errors-tokens-search-failed = Token araması başarısız oldu

# Actions and services
errors-actions-not-found = Eylem bulunamadı: { $id }
errors-services-not-found = '{ $name }' hizmeti bulunamadı
