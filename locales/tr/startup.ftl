## Wallet mismatch.

startup-wallet-mismatch-title = Cüzdan değişti
startup-wallet-mismatch-detail =
    Yapılandırmanızdaki cüzdan, bu bilgisayarın yerel geçmişinde kayıtlı cüzdanla eşleşmiyor.

    Geçerli cüzdan: { $current }
    Önceki cüzdan: { $stored }

    Etkilenen yerel veriler: { $systems }

    Bu durum genellikle farklı bir özel anahtar içe aktarıldığında veya farklı bir yapılandırma geri yüklendiğinde olur. İşlemler, pozisyonlar ve geçmiş önceki cüzdana aittir; yeni cüzdanın güvenle başlayabilmesi için temizlenmeleri gerekir.
startup-wallet-mismatch-systems-default = İşlemler, Pozisyonlar, Cüzdan Geçmişi
startup-wallet-mismatch-remedy =
    Devam etmek için önceki cüzdanın yerel geçmişini temizleyin (veritabanlarınız önce otomatik olarak yedeklenir):

      - Uygulamada: aşağıdaki "{ $action }" seçeneğini seçin.
      - Terminalden: screenerbot --clean-wallet-data komutunu çalıştırın

    Zincir üzerindeki fonlar etkilenmez; yalnızca bu bilgisayarın yerel işlem/pozisyon geçmişi sıfırlanır. Yedekler şu konuma yazılır:
      { $path }
startup-recovery-reset-wallet = Cüzdan verilerini sıfırla ve yeniden başlat

## Port in use.

startup-port-in-use-title = Ağ portu meşgul
startup-port-in-use-detail = Panel portu { $address } zaten kullanımda.
startup-port-in-use-remedy = Başka bir program { -brand } uygulamasının ihtiyaç duyduğu portu kullanıyor. O programı kapatın veya Ayarlar'dan web sunucusu portunu değiştirin, ardından { -brand } uygulamasını yeniden başlatın.

## Another instance is running.

startup-lock-held-title = { -brand } zaten çalışıyor
startup-lock-held-detail = Bu bilgisayarda { -brand } uygulamasının başka bir kopyası zaten çalışıyor, bu yüzden ikincisi başlatılamaz.
startup-lock-held-remedy = Zaten açık olan pencereye geçin. Açık bir pencere görmüyorsanız arka plandaki tüm { -brand } işlemlerini kapatıp tekrar deneyin. Sorun yeniden başlattıktan sonra da sürerse kilit dosyası eski kalmış olabilir ve veri klasöründen silinebilir (.screenerbot.lock).

## Configuration.

startup-config-invalid-title = Yapılandırma okunamadı
startup-config-parse-detail = config.toml ayrıştırılamadı: { $detail }
startup-config-load-parse-detail = Yapılandırma yüklenemedi: config.toml ayrıştırılamadı: { $detail }
startup-config-parse-remedy = Yapılandırma dosyanız okunamadı. Veri klasöründen bir yedeği geri yükleyin veya yapılandırmayı varsayılanlara sıfırlayıp cüzdanınızı ve RPC ayarlarınızı yeniden yapın.
startup-config-load-parse-remedy = Geçerli bir yapılandırmayı geri yükleyin veya kurulumu yeniden tamamlayın.
startup-option-invalid-title = Geçersiz başlatma seçeneği
startup-option-invalid-remedy = Bir komut satırı seçeneği geçersiz. { -brand } uygulamasını bu seçenek olmadan başlatın veya düzeltip tekrar deneyin.

## Generic failures.

startup-generic-title = { -brand } başlatılamadı
startup-generic-remedy = Ayrıntılar için günlük dosyasına bakın, ardından uygulamayı yeniden başlatın. Sorun sürerse t.me/screenerbotio_support adresinden destekle iletişime geçin.
startup-generic-detail = { $error }
startup-failure-directories = Gerekli klasörler oluşturulamadı: { $error }
startup-failure-config-load = Yapılandırma yüklenemedi: { $error }
startup-failure-actions-init = Eylemler veritabanı başlatılamadı: { $error }
startup-failure-actions-sync = Eylemler veritabanından eşitlenemedi: { $error }
startup-failure-strategy-init = Strateji sistemi başlatılamadı: { $error }
startup-failure-analysis-init = Analiz motoru başlatılamadı: { $error }
startup-failure-assistant-init = Asistan sohbet motoru başlatılamadı: { $error }
startup-failure-wallets-init = Cüzdanlar başlatılamadı: { $error }
startup-failure-wallet-validation = Cüzdan tutarlılığı doğrulanamadı: { $error }
