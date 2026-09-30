system-result-config-differs = Bellekteki yapılandırma diskteki sürümden farklı
system-result-config-matches = Bellekteki yapılandırma diskteki sürümle aynı

system-result-config-imported =
    { $count ->
        [one] { $count } bölüm başarıyla içe aktarıldı
       *[other] { $count } bölüm başarıyla içe aktarıldı
    }
system-result-config-imported-with-warnings =
    { $count ->
        [one] { $count } bölüm içe aktarıldı, uyarı sayısı { $warnings }: { $details }
       *[other] { $count } bölüm içe aktarıldı, uyarı sayısı { $warnings }: { $details }
    }

system-config-search =
    .placeholder = Ayarlarda ara...
system-config-export-title =
    .title = Yapılandırmayı dosyaya aktar
system-config-import-title =
    .title = Yapılandırmayı dosyadan içe aktar
system-config-reload = Diskten yeniden yükle
system-config-reset-defaults = Varsayılanlara sıfırla
system-config-select-section = Bir yapılandırma bölümü seçin
system-config-select-section-details = Ayrıntıları görmek için bir yapılandırma bölümü seçin.
system-config-no-metadata = <code>{ $section }</code> için meta veri yok
system-config-technical-settings = Teknik ayarlar
system-config-expand-title = Her bölümü ve iç içe her alt yapılandırmayı genişlet
system-config-collapse-title = Her bölümü ve iç içe her alt yapılandırmayı daralt
system-config-toolbar-no-changes = Bölümde değişiklik yok
system-config-toolbar-section-changes =
    { $count ->
        [one] Bölümde <strong>{ $count }</strong> değişiklik
       *[other] Bölümde <strong>{ $count }</strong> değişiklik
    }
system-config-toolbar-total-changes =
    { $count ->
        [one] Toplam <strong>{ $count }</strong> değişiklik
       *[other] Toplam <strong>{ $count }</strong> değişiklik
    }

system-config-loading = Yapılandırma yükleniyor…
system-config-refreshing = Yapılandırma yenileniyor…
system-config-saving-title = Değişiklikler kaydediliyor…
system-config-saving-detail = Yapılandırma güncelleniyor
system-config-validation-issues = <strong>Doğrulama sorunları algılandı.</strong> Lütfen vurgulanan alanları inceleyin.

system-config-save-changes = Değişiklikleri kaydet
system-config-saving = Kaydediliyor…
system-config-compare = Diskle karşılaştır
system-config-revert-section = Bölümü geri al
system-config-summary-critical = Kritik: { $count }
system-config-summary-performance = Performans: { $count }
system-config-summary-pending =
    { $count ->
        [one] Bekleyen değişiklik: { $count }
       *[other] Bekleyen değişiklik: { $count }
    }
system-config-summary-none = Meta veri özeti yok
system-config-fields-count =
    { $count ->
        [one] { $count } alan
       *[other] { $count } alan
    }
system-config-chip-pending = { $fields } · bekleyen { $pending }
system-config-chip-visible = { $visible } / { $fields }

system-config-field-unit = Birim: { $unit }
system-config-field-default = Varsayılan: { $value }
system-config-field-reset = Varsayılana sıfırla
system-config-array-invalid-title = Geçersiz dizi girdisi
system-config-json-invalid-title = Geçersiz JSON
system-config-list-separator = { ", " }
system-config-array-invalid-integer =
    { $count ->
        [one] Satır { $lines }: geçerli bir tam sayı olmalıdır.
       *[other] Satır { $lines }: geçerli bir tam sayı olmalıdır.
    }
system-config-array-invalid-number =
    { $count ->
        [one] Satır { $lines }: geçerli bir sayı olmalıdır.
       *[other] Satır { $lines }: geçerli bir sayı olmalıdır.
    }
system-config-array-invalid-boolean =
    { $count ->
        [one] Satır { $lines }: geçerli bir mantıksal değer olmalıdır.
       *[other] Satır { $lines }: geçerli bir mantıksal değer olmalıdır.
    }
system-config-array-invalid-value =
    { $count ->
        [one] Satır { $lines }: geçerli bir değer olmalıdır.
       *[other] Satır { $lines }: geçerli bir değer olmalıdır.
    }

system-config-telegram-actions = Eylemler
system-config-telegram-test-title = Bağlantıyı sına
system-config-telegram-test-description = { -telegram } yapılandırmanızın çalıştığını doğrulamak için bir test mesajı gönderin
system-config-telegram-send-test = Test mesajı gönder
system-config-telegram-sending = Gönderiliyor...
system-config-telegram-configure-token-title = Önce bot tokenını yapılandırın
system-config-telegram-configure-token-status = Testi etkinleştirmek için yukarıda bot tokenını yapılandırın
system-config-telegram-test-sent-status = Test mesajı başarıyla gönderildi! { -telegram } uygulamanızı kontrol edin.
system-config-telegram-test-sent = { -telegram } test mesajı gönderildi
system-config-telegram-test-failed = Test mesajı gönderilemedi
system-config-telegram-auth-title = Bot kimlik doğrulaması
system-config-telegram-totp-title = İki adımlı doğrulama (TOTP)
system-config-telegram-totp-configured = Yapılandırıldı
system-config-telegram-totp-not-configured = Yapılandırılmadı
system-config-telegram-totp-active = İki adımlı doğrulama etkin. Süresi dolan { -telegram } oturumları doğrulayıcı uygulamanızdan TOTP kodu gerektirir.
system-config-telegram-totp-inactive = { -telegram } komutlarını korumak için Güvenlik ayarlarında iki adımlı doğrulamayı etkinleştirin.
system-config-telegram-totp-note = TOTP, panel kilit ekranıyla paylaşılır. Güvenlik ayarlarından yapılandırın.
system-config-telegram-require-2fa = Komutlar için 2FA iste
system-config-telegram-save-rejected = Kaydetme reddedildi ({ $status })
system-config-telegram-save-failed = { -telegram } ayarı kaydedilemedi

system-config-saved = Yapılandırma kaydedildi
system-config-save-failed = Yapılandırma kaydedilemedi
system-config-reloaded = Yapılandırma diskten yeniden yüklendi
system-config-reload-failed = Yapılandırma yeniden yüklenemedi
system-config-diff-title = Yapılandırma farkı
system-config-diff-console = Tarayıcı konsoluna yazıldı
system-config-diff-failed = Fark hesaplanamadı
system-config-reset-title = Yapılandırmayı sıfırla
system-config-reset-message =
    Bu, tüm yapılandırmayı gömülü varsayılan değerlere sıfırlar. Mevcut tüm ayarlar kaybolur.

    Bu işlem geri alınamaz.
system-config-reset-done-title = Yapılandırma sıfırlandı
system-config-reset-done-message = Tüm ayarlar varsayılan değerlere döndürüldü
system-config-reset-failed = Yapılandırma sıfırlanamadı
system-config-load-failed = Yapılandırma yüklenemedi
system-config-metadata-failed = Yapılandırma meta verisi yüklenemedi

system-config-dialog-close =
    .aria-label = Kapat
system-config-select-none = Hiçbirini seçme
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
        [one] { $count } değişiklik
       *[other] { $count } değişiklik
    }
system-config-sections-count =
    { $count ->
        [one] { $count } bölüm
       *[other] { $count } bölüm
    }

system-config-section-hint-rpc = RPC uç noktaları ve bağlantı ayarları
system-config-section-hint-trader = İşlem kuralları ve otomasyon
system-config-section-hint-positions = Pozisyon yönetimi ayarları
system-config-section-hint-filtering = Token filtreleme kuralları ve eşikleri
system-config-section-hint-swaps = Takas yürütme ayarları
system-config-section-hint-tokens = Token keşfi ve veri kaynakları
system-config-section-hint-sol-price = { -sol } fiyat hizmeti yapılandırması
system-config-section-hint-events = Olay kaydı ayarları
system-config-section-hint-services = Arka plan hizmeti ayarları
system-config-section-hint-monitoring = Sistem izleme yapılandırması
system-config-section-hint-ohlcv = Mum verisi ayarları
system-config-section-hint-gui = Panel ve arayüz ayarları
system-config-section-hint-telegram = { -telegram } bot yapılandırması

system-config-export-dialog-title = Yapılandırmayı dışa aktar
system-config-export-intro = Dışa aktarılacak yapılandırma bölümlerini seçin. Dışa aktarılan dosya, ayarları geri yüklemek veya paylaşmak için daha sonra içe aktarılabilir.
system-config-export-sections = Bölümler
system-config-export-timestamp = Dışa aktarma zaman damgasını dahil et
system-config-sections-selected =
    { $count ->
        [one] { $count } bölüm seçildi
       *[other] { $count } bölüm seçildi
    }
system-config-exporting = Dışa aktarılıyor...
system-config-export-invalid-response = Sunucudan geçersiz yanıt
system-config-exported-title = Yapılandırma dışa aktarıldı
system-config-exported-message =
    { $count ->
        [one] { $count } bölüm dışa aktarıldı
       *[other] { $count } bölüm dışa aktarıldı
    }
system-config-export-failed-title = Dışa aktarma başarısız
system-config-export-failed = Yapılandırma dışa aktarılamadı

system-config-import-dialog-title = Yapılandırmayı içe aktar
system-config-import-upload-intro = Daha önce dışa aktardığınız bir yapılandırma dosyasını yükleyin. Önizleme yapıp hangi bölümlerin içe aktarılacağını seçebileceksiniz.
system-config-import-dropzone-title = Yapılandırma dosyasını buraya bırakın
system-config-import-dropzone-hint = veya göz atmak için tıklayın
system-config-import-analyzing = Yapılandırma analiz ediliyor...
system-config-import-preview = Önizleme
system-config-import-preview-intro = Aşağıdaki yapılandırma bölümlerini inceleyin. İçe aktarılacak bölümleri seçin.
system-config-import-sections = Dosyadaki bölümler
system-config-import-select-valid = Geçerlilerin tümünü seç
system-config-import-merge-label = Mevcut olanla birleştir
system-config-import-merge-hint = Yalnızca dosyada bulunan alanları günceller. İşaretli değilse bölümlerin tamamı değiştirilir.
system-config-import-save-label = Diske kaydet
system-config-import-save-hint = İçe aktarma sonrası değişiklikleri config.toml dosyasına kalıcı olarak yaz
system-config-import-selected = Seçilenleri içe aktar
system-config-import-warnings =
    { $count ->
        [one] { $count } uyarı
       *[other] { $count } uyarı
    }
system-config-import-warning-unknown-section = Bilinmeyen bölüm "{ $section }" yok sayılacak
system-config-import-warning-sensitive-field = { $field } alanını içe aktarmak kimlik doğrulama ayarlarının üzerine yazabilir
system-config-import-section-error = { $detail }
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = Dosyada yok
system-config-import-status-invalid = Geçersiz yapılandırma
system-config-import-status-unchanged = Değişiklik yok
system-config-import-not-included = Dosyaya dahil değil
system-config-import-show-changes = Değişiklikleri göster
system-config-import-hide-changes = Değişiklikleri gizle
system-config-import-value-current = Mevcut değer
system-config-import-value-new = Yeni değer
system-config-import-more-changes =
    { $count ->
        [one] +{ $count } değişiklik daha
       *[other] +{ $count } değişiklik daha
    }
system-config-import-value-items =
    { "[" }{ $count ->
        [one] { $count } öğe
       *[other] { $count } öğe
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
        [one] { $count } anahtar
       *[other] { $count } anahtar
    }{ "}" }
system-config-importing = İçe aktarılıyor...
system-config-import-failed = İçe aktarma başarısız
system-config-import-invalid-file-title = Geçersiz dosya
system-config-import-invalid-file = Yapılandırma dosyası ayrıştırılamadı
system-config-imported-title = Yapılandırma içe aktarıldı
system-config-imported-message =
    { $count ->
        [one] { $count } bölüm içe aktarıldı
       *[other] { $count } bölüm içe aktarıldı
    }
system-config-import-failed-title = İçe aktarma başarısız
system-config-import-failed-message = Yapılandırma içe aktarılamadı
