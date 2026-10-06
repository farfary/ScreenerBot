wallets-type-generated = Oluşturulmuş
wallets-type-imported = İçe aktarılmış
wallets-type-migrated = Taşınmış

wallets-watch-disabled-user = Sizin tarafınızdan duraklatıldı
wallets-watch-disabled-signature-budget = Duraklatıldı: geride kalan işlemler yakalanmadan imza kontrol sınırına ulaşıldı (sınır: { $limit })
wallets-watch-disabled-unknown = Duraklatıldı: kayıtlı izleme güvenlik nedeni okunamadı
wallets-watch-disabled-helius-unavailable = Duraklatıldı: yüksek etkinlik sağlayıcısı kullanılamıyor; imleç korundu
wallets-watch-disabled-processing-failed = Duraklatıldı: cüzdan etkinliği işlenemedi; imleç korundu

wallets-watch-error-provider-unavailable = Yüksek etkinlik sağlayıcısı kullanılamıyor; izleme duraklatıldı
wallets-watch-error-provider-repeated-failure = { -helius } kontrolleri art arda başarısız oldu; izleme duraklatıldı
wallets-watch-error-processing-repeated-failure = Cüzdan etkinliği işleme art arda başarısız oldu; izleme duraklatıldı
wallets-watch-error-position-unreadable = Cüzdan izleme kayıtlı konumunu okuyamadı; yeniden deneniyor
wallets-watch-error-provider-check-failed = Yüksek etkinlik sağlayıcısı kontrolü başarısız oldu; yeniden deneniyor
wallets-watch-error-decode-failed = Yüksek etkinlikli işlem çözülemedi; imleç korundu
wallets-watch-error-processing-failed = Cüzdan etkinliği işlenemedi; yeniden deneniyor
wallets-watch-error-position-save-failed = Cüzdan izleme konumunu kaydedemedi; yeniden deneniyor

wallets-watch-reason-user = Sizin tarafınızdan duraklatıldı.
wallets-watch-reason-signature-budget = Bu cüzdanın etkinliği, mevcut izlemenin kontrol edebileceğinden fazla.
wallets-watch-reason-helius-unavailable = { -helius } kontrolleri başarısız oldu. Kayıtlı ilerleme korunuyor.
wallets-watch-reason-processing-failed = Cüzdan etkinliği işlenemedi. Kayıtlı ilerleme korunuyor.

wallets-field-address = Adres
wallets-field-name = Cüzdan adı
wallets-field-notes = Notlar
wallets-field-private-key = Özel anahtar
wallets-address-copy = Adresi kopyala
wallets-modal-close =
    .aria-label = Pencereyi kapat
wallets-this-wallet = bu cüzdan
wallets-summary-native = { -sol }
wallets-copied-address = Adres
wallets-copied-mint = Mint adresi
wallets-copied-private-key = Özel anahtar

wallets-tab-main = Ana cüzdan
wallets-tab-secondaries = İkincil cüzdanlar
wallets-tab-archive = Arşiv
wallets-tab-watched = İzlenenler
wallets-refresh-failed = Cüzdanlar yenilenemedi
wallets-action-failed = Başarısız
wallets-toast-failed = Başarısız: { $reason }
wallets-create-busy = Oluşturuluyor...
wallets-create-fallback = Oluşturma başarısız
wallets-create-done = "{ $name }" cüzdanı oluşturuldu!
wallets-import-busy = İçe aktarılıyor...
wallets-import-failed = İçe aktarma başarısız
wallets-import-done = "{ $name }" cüzdanı içe aktarıldı!
wallets-archive-busy = Arşivleniyor...
wallets-archive-confirm-text = <strong>{ $name }</strong> arşivlensin mi?
wallets-archive-done = Cüzdan arşivlendi
wallets-restore-done = Cüzdan geri yüklendi
wallets-export-busy = Şifre çözülüyor...
wallets-export-revealed = Anahtar gösterildi - dikkatli olun
wallets-delete-busy = Siliniyor...
wallets-delete-confirm-text = <strong>{ $name }</strong> silinsin mi?
wallets-delete-done = Cüzdan kalıcı olarak silindi

wallets-add-title = Cüzdan ekle
wallets-add-tab-create = Yeni oluştur
wallets-add-tab-import = Mevcut olanı içe aktar
wallets-create-name-input =
    .placeholder = örn. İşlem cüzdanı
wallets-create-name-hint = Bu cüzdanı tanımlamak için kolay bir ad
wallets-create-notes-input =
    .placeholder = İsteğe bağlı açıklama veya amaç...
wallets-create-submit = Cüzdan oluştur
wallets-import-warning-title = Güvenlik uyarısı
wallets-import-warning-body = Özel anahtarları yalnızca güvenilir kaynaklardan içe aktarın. Anahtarınız şifrelenir ve bu cihazda güvenle saklanır.
wallets-import-name-input =
    .placeholder = örn. Cüzdanım
wallets-import-key-input =
    .placeholder = Base58 dizesi veya JSON dizisi [1,2,3,...]
wallets-import-key-toggle =
    .aria-label = Özel anahtar görünürlüğünü değiştir
wallets-import-key-hint = Base58 kodlu anahtar veya bayt dizisi biçimini destekler
wallets-import-notes-input =
    .placeholder = İsteğe bağlı açıklama...
wallets-import-submit = Cüzdanı içe aktar

wallets-watch-add-title = Cüzdan izle
wallets-watch-add-address = Cüzdan adresi
wallets-watch-add-address-input =
    .placeholder = Solana adresi
wallets-watch-add-address-hint = Cüzdanın zincir üstü etkinliğini kaydeder ve { -telegram } ayarlarınız üzerinden işlem uyarıları gönderir.
wallets-watch-add-label = Etiket
wallets-watch-add-label-input =
    .placeholder = İsteğe bağlı ad
wallets-watch-add-submit = İzleme ekle

wallets-watch-budget-title-options = Cüzdan izleme seçenekleri
wallets-watch-budget-title-restore = Cüzdan izlemeyi geri yükle
wallets-watch-budget-close =
    .aria-label = Kapat
wallets-watch-budget-label-signatures = Kontrol başına denetlenen imza
wallets-watch-budget-label-transactions = Kontrol başına denetlenen başarılı tam işlem
wallets-watch-budget-hint-signatures = Mevcut sınır: { $limit }. Kontrol başına 500–5.000 imza arasında 100'er adımla seçin.
wallets-watch-budget-hint-transactions = Mevcut sınır: { $limit }. Kontrol başına 500–5.000 başarılı işlem arasında 100'er adımla seçin.
wallets-watch-budget-error-range = Kontrol başına 500 ile 5.000 arasında, 100'lük adımlarla bir kayıt sayısı seçin.
wallets-watch-budget-error-ack = Son tamamlanan kontrolden bu yana gelen imzaların atlanacağını onaylayın.
wallets-watch-budget-save-failed = İzleme sınırı kaydedilemedi.
wallets-watch-budget-save = Sınırı kaydet
wallets-watch-budget-resume = Şimdiden devam et
wallets-watch-budget-resume-notice = Bu cüzdan geride kalan işlemleri yakalamadan kontrol sınırına ulaştı. Şimdiden devam et seçeneği en son cüzdan etkinliğinden başlar; son tamamlanan kontrolden bu yana olan etkinlik kopyalanmaz.
wallets-watch-budget-resume-tasks = Kopyalama görevleri, Kopya işlem bölümünde her görevi tek tek devam ettirene kadar duraklatılmış kalır.
wallets-watch-budget-resume-ack = Kaçırılan etkinliğin kopyalanmayacağını anlıyorum.
wallets-watch-budget-resumed = İzleme, cüzdanın güncel noktasından devam ettirildi
wallets-watch-budget-updated = Cüzdan izleme sınırı güncellendi
wallets-watch-helius-allow = Gerekirse { -helius } ile yakalamaya izin ver
wallets-watch-helius-try = { -helius } ile yakalamayı dene
wallets-watch-helius-stop = Bu cüzdan için { -helius } ile yakalamayı durdur
wallets-watch-helius-description-approved = Bu cüzdan için { -helius } ile yakalamaya izin verildi. Kapatırsanız standart kontrollere dönülür; yoğun bir cüzdanda geride kalınabilir.
wallets-watch-helius-description-available = { -helius }, kontrol edilmeyen aralığı atlamadan kayıtlı konumdan itibaren başarılı Solana işlemlerini kontrol edebilir. Daha fazla sağlayıcı kredisi harcayabilir ve yine de geride kalabilir.
wallets-watch-helius-description-unavailable = { -helius } ile yakalama kullanılamıyor. Kullanmak için etkin bir { -helius } RPC uç noktası yapılandırın.
wallets-watch-helius-description-unsupported = Bu izleme için desteklenen bir yakalama sağlayıcısı yok. İzleme sınırına ulaşırsa Şimdiden devam et seçeneği kullanılabilir.
wallets-watch-helius-allow-title = Bu cüzdan için { -helius } ile yakalamaya izin ver
wallets-watch-helius-allow-message = { -helius }, kontrol edilmeyen aralığı atlamadan kayıtlı konumdan itibaren başarılı Solana işlemlerini kontrol edebilir. Şu anda, döndürülen her 100 tam işlem için 10 kredi (yukarı yuvarlanır) ve istek başına en az 10 kredi ücretlendirir. Bir kontrol birden fazla istek yapabilir; kullanım ve sağlayıcı fiyatlandırması değişebilir. Kopyalama görevleri ayrıca devam ettirilene kadar duraklatılmış kalır.
wallets-watch-helius-allow-confirm = Bu cüzdan için izin ver
wallets-watch-helius-stop-message = Bu cüzdan standart kontrollere dönecek. Yoğun bir cüzdan izleme sınırına ulaşıp yeniden duraklatılabilir. Diğer cüzdanlar ve { -helius } RPC yapılandırmanız değişmez.
wallets-watch-helius-stop-confirm = Bu cüzdan için durdur
wallets-watch-helius-stop-keep = İzinli kalsın
wallets-watch-helius-restored = İzleme kayıtlı ilerlemeden geri yüklendi; kopyalama görevleri duraklatılmış kalır
wallets-watch-helius-allowed = Bu cüzdan için gerektiğinde { -helius } ile yakalamaya izin verildi
wallets-watch-helius-stopped = Bu cüzdan için { -helius } ile yakalama durduruldu
wallets-watch-helius-update-failed = Cüzdan yakalama ayarı güncellenemedi

wallets-export-title = Özel anahtarı dışa aktar
wallets-export-warning-title = Kritik güvenlik uyarısı
wallets-export-warning-body = Özel anahtarınızı asla kimseyle paylaşmayın. Bu anahtara erişen herkes bu cüzdandaki tüm fonları çalabilir.
wallets-export-key-label = Özel anahtar (Base58)
wallets-export-copy =
    .title = Panoya kopyala
    .aria-label = Panoya kopyala
wallets-export-reveal = Anahtarı göster

wallets-archive-title = Cüzdanı arşivle
wallets-archive-note = Arşivlenen cüzdanlar hiçbir işlemde kullanılmaz ancak istediğiniz zaman geri yüklenebilir.
wallets-archive-confirm = Evet, arşivle
wallets-delete-title = Cüzdanı sil
wallets-delete-warning-title = Bu işlem geri alınamaz!
wallets-delete-warning-body = Bu cüzdanı silmek, cüzdanı ve şifreli özel anahtarını bu cihazdan kalıcı olarak kaldırır.
wallets-delete-confirm = Evet, sil

wallets-bulk-import-title = Cüzdanları içe aktar
wallets-bulk-import-submit = Cüzdanları içe aktar
wallets-bulk-step-upload = Dosya yükle
wallets-bulk-step-map = Sütunları eşle
wallets-bulk-step-results = Sonuçlar
wallets-bulk-import-file-warning-body = Dosyaları yalnızca güvenilir kaynaklardan içe aktarın. Özel anahtarlar şifrelenir ve bu cihazda güvenle saklanır.
wallets-bulk-drop-title = Dosyanızı buraya bırakın
wallets-bulk-drop-subtitle = veya göz atmak için tıklayın
wallets-bulk-drop-formats = CSV ve Excel desteklenir (.xlsx, .xls)
wallets-bulk-file-remove =
    .aria-label = Dosyayı kaldır
wallets-bulk-map-subtitle = Dosya sütunlarınızı cüzdan alanlarıyla eşleştirin
wallets-bulk-preview-title = Önizleme (ilk 5 satır)
wallets-bulk-summary-valid = <strong>{ $count }</strong> geçerli
wallets-bulk-summary-invalid = <strong>{ $count }</strong> geçersiz
wallets-bulk-summary-duplicate =
    { $count ->
        [one] <strong>{ $count }</strong> yinelenen
       *[other] <strong>{ $count }</strong> yinelenen
    }
wallets-bulk-done = Bitti
wallets-bulk-file-invalid = Geçersiz dosya türü. Lütfen CSV veya Excel dosyaları kullanın.
wallets-bulk-preview-busy = İşleniyor...
wallets-bulk-preview-fallback = Dosya işlenemedi
wallets-bulk-preview-failed = Dosya işlenemedi: { $reason }
wallets-bulk-column-select = -- Sütun seçin --
wallets-bulk-preview-empty = Dosyada veri satırı bulunamadı
wallets-bulk-preview-status = Durum
wallets-bulk-status-valid = Geçerli
wallets-bulk-status-duplicate = Yinelenen
wallets-bulk-status-invalid = Geçersiz
wallets-bulk-import-busy = İçe aktarılıyor...
wallets-bulk-import-toast =
    { $count ->
        [one] İçe aktarılan cüzdan: { $count }
       *[other] İçe aktarılan cüzdan: { $count }
    }
wallets-bulk-import-error = İçe aktarma başarısız: { $reason }
wallets-bulk-result-success-title = İçe aktarma başarılı
wallets-bulk-result-success-detail =
    { $count ->
        [one] Tüm cüzdanlar ({ $count }) başarıyla içe aktarıldı
       *[other] Tüm cüzdanlar ({ $count }) başarıyla içe aktarıldı
    }
wallets-bulk-result-partial-title = Kısmen başarılı
wallets-bulk-result-partial-detail = İçe aktarılan: { $imported }, başarısız: { $failed }
wallets-bulk-result-failed-title = İçe aktarma başarısız
wallets-bulk-result-failed-detail =
    { $count ->
        [one] Tüm cüzdanlar ({ $count }) içe aktarılamadı
       *[other] Tüm cüzdanlar ({ $count }) içe aktarılamadı
    }
wallets-bulk-result-imported = İçe aktarılan
wallets-bulk-result-failed = Başarısız

wallets-bulk-export-title = Cüzdanları dışa aktar
wallets-bulk-export-format = Biçim
wallets-bulk-export-format-csv = CSV (.csv)
wallets-bulk-export-format-xlsx = Excel (.xlsx)
wallets-bulk-export-include-archived = Arşivlenmiş cüzdanları dahil et
wallets-bulk-export-safe-title = Güvenli dışa aktarma
wallets-bulk-export-safe-body = Yalnızca cüzdan adreslerini ve meta verileri dışa aktarır. Özel anahtar içermez.
wallets-bulk-export-safe-submit = Adresleri dışa aktar
wallets-bulk-export-or = veya
wallets-bulk-export-danger-title = Tehlikeli dışa aktarma
wallets-bulk-export-danger-body = Özel anahtarları dışa aktarmaya dahil eder. Bu dosyaya sahip olan herkes fonlarınızı çalabilir.
wallets-bulk-export-danger-submit = Özel anahtarlarla dışa aktar
wallets-bulk-export-busy = Dışa aktarılıyor...
wallets-bulk-export-done = Cüzdanlar şuraya aktarıldı: { $filename }
wallets-bulk-export-fallback = Dışa aktarma başarısız
wallets-bulk-export-error = Dışa aktarma başarısız: { $reason }
wallets-bulk-confirm-title = Tehlikeli dışa aktarmayı onayla
wallets-bulk-confirm-warning =
    { $count ->
        [one] Dışa aktarmak üzere olduğunuz özel anahtar sayısı: <strong>{ $count }</strong>. Bu son derece tehlikelidir!
       *[other] Dışa aktarmak üzere olduğunuz özel anahtar sayısı: <strong>{ $count }</strong>. Bu son derece tehlikelidir!
    }
wallets-bulk-confirm-risk-steal = Bu dosyaya sahip olan herkes tüm fonları çalabilir
wallets-bulk-confirm-risk-share = Bu dosyayı asla kimseyle paylaşmayın
wallets-bulk-confirm-risk-delete = Kullandıktan hemen sonra dosyayı silin
wallets-bulk-confirm-prompt = Onaylamak için aşağıdaki ifadeyi yazın
wallets-bulk-confirm-submit = Anahtarları dışa aktar

wallets-holdings-col-token = Token
wallets-holdings-col-balance = Bakiye
wallets-holdings-col-value = Değer ({ -sol })
wallets-holdings-col-type = Tür
wallets-holdings-col-decimals = Ondalık
wallets-holdings-col-mint = Mint
wallets-holdings-empty-title = Token varlığı yok
wallets-holdings-empty-message = Bu cüzdanın sahip olduğu tokenlar burada görünecek.
wallets-holdings-no-main = Ana cüzdan yok
wallets-holdings-main-tag = Ana
wallets-holdings-main-title = Ana cüzdan
wallets-holdings-tokens = Tokenlar
wallets-holdings-last-used = Son kullanım
wallets-holdings-never = Hiç
wallets-holdings-search =
    .placeholder = Sembol veya mint ile ara...
wallets-holdings-export = Anahtarı dışa aktar
wallets-holdings-export-tooltip = Bu cüzdanın özel anahtarını dışa aktar
wallets-list-col-name = Ad
wallets-list-col-balance = Bakiye ({ -sol })
wallets-list-col-type = Tür
wallets-list-col-created = Oluşturulma
wallets-list-col-actions = İşlemler
wallets-list-action-export = Özel anahtarı dışa aktar
wallets-list-action-archive = Cüzdanı arşivle
wallets-list-action-restore = Cüzdanı geri yükle
wallets-list-action-delete = Kalıcı olarak sil
wallets-list-count = Cüzdanlar
wallets-list-search =
    .placeholder = Ad veya adres ile ara...
wallets-list-loading-title = Cüzdanlar yükleniyor…
wallets-list-loading-description = Seçilen cüzdan görünümü hazırlanıyor.
wallets-secondaries-empty-title = İkincil cüzdan yok
wallets-secondaries-empty-message = İşlem faaliyetlerinizi birden çok hesap arasında düzenlemek için ek cüzdanlar oluşturun.
wallets-secondaries-add = Cüzdan ekle
wallets-archive-empty-title = Arşivlenmiş cüzdan yok
wallets-archive-empty-message = Arşivlediğiniz cüzdanlar ileride başvurmanız için burada güvenle saklanır.

wallets-watched-col-wallet = Cüzdan
wallets-watched-col-status = Durum
wallets-watched-col-progress = Kaydedilen ilerleme
wallets-watched-col-last-check = Son kontrol
wallets-watched-unlabelled = Etiketsiz cüzdan
wallets-watched-generic-name = cüzdan
wallets-watched-not-synced = Henüz eşitlenmedi
wallets-watched-not-checked = Henüz kontrol edilmedi
wallets-watched-action-copy = Kopya işlem
    .title = Bu cüzdanı Kopya işlem bölümünde aç
wallets-watched-action-restore = İzlemeyi geri yükle
wallets-watched-action-options = İzleme seçenekleri
wallets-watched-action-retry = İzlemeyi yeniden dene
wallets-watched-action-pause = Duraklat
wallets-watched-action-enable = Etkinleştir
wallets-watched-action-remove =
    .title = Kaldır
    .aria-label = Kaldır: { $name }
wallets-watch-state-paused = Duraklatıldı
wallets-watch-state-catching-up = Yakalanıyor
wallets-watch-state-watching = İzleniyor
wallets-watch-state-streaming = Akış
wallets-watch-state-polling = Yoklama
wallets-watched-detail-helius = Bu cüzdan için { -helius } üzerinden kontrol ediliyor.
wallets-watched-empty-title = İzlenen adres yok
wallets-watched-empty-message = Herkese açık bir cüzdanın zincir üstü etkinliğini kaydetmek için Cüzdan izle'yi kullanın.
wallets-watched-count = İzlenenler
wallets-watched-search =
    .placeholder = İzlenen cüzdanlarda ara...
wallets-watched-add = Cüzdan izle
wallets-watched-refresh = İzlenen cüzdanları yenile
wallets-watched-loading-title = İzlenen cüzdanlar yükleniyor...
wallets-watched-loading-description = Gözlem hedefleri getiriliyor.
wallets-watched-load-error-title = İzlenen adresler yüklenemedi
wallets-watched-load-error-description = Yeniden denemek için yenileyin.
wallets-watched-address-invalid = Geçerli bir Solana cüzdan adresi girin.
wallets-watched-added = Cüzdan izleme eklendi
wallets-watched-duplicate = Bu cüzdan zaten izleniyor.
wallets-watched-add-failed = Cüzdan izleme eklenemedi.
wallets-watched-retried = Cüzdan izleme kayıtlı imleciyle geri yüklendi
wallets-watched-paused = Cüzdan izleme duraklatıldı
wallets-watched-enabled = Cüzdan izleme etkinleştirildi
wallets-watched-removed = Cüzdan izleme kaldırıldı
wallets-watched-update-failed = Cüzdan izleme güncellenemedi
