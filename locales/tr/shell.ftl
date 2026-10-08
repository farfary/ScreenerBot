shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-brand-logo =
    .alt = { -brand }
shell-version = v{ $version }

shell-header-brand =
    .aria-label = Panel ana sayfasını aç
    .title = Panel ana sayfası
shell-bot-card =
    .aria-label = Auto Trader durumu yükleniyor
shell-bot-label = Oto
shell-bot-status-loading = YÜKLENİYOR
shell-bot-today = Bugün
shell-explore-control =
    .aria-label = Keşif Modu. Tüm özellikleri etkinleştirmek için bir cüzdan ve RPC uç noktası bağlayın
    .title = İşlem, bakiye ve canlı zincir üstü veri için bir cüzdan ve RPC uç noktası bağlayın
shell-explore-title = Keşif Modu
shell-explore-detail = Cüzdan ve RPC bağlı değil
shell-explore-action = Kurulumu tamamla
shell-wallet-card =
    .aria-label = Cüzdan değeri; Pozisyonlar sayfasını aç
    .title = Cüzdan değeri ({ -sol } + tokenlar) · Pozisyonlar sayfasını aç
shell-wallet-worth-label = DEĞER
shell-wallet-native-label = { -sol }
shell-wallet-tokens-label = TKN
shell-sol-price-card =
    .aria-label = USD cinsinden { -sol } fiyatı — grafiği aç
    .title = { -sol } fiyatı · grafik için tıklayın
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24sa
shell-copy-card =
    .aria-label = Kopya işlem; Kopya işlem sayfasını aç
    .title = Kopya işlem · Kopya işlem sayfasını aç
shell-copy-label = KOPYA
shell-actions-more =
    .aria-label = Diğer üst çubuk eylemleri
    .title = Diğer eylemler
shell-actions-group =
    .aria-label = Üst çubuk eylemleri
shell-action-search =
    .aria-label = Token ara
    .title = Token ara (Ctrl/Cmd+K)
shell-action-featured =
    .aria-label = Öne çıkan tokenlar
    .title = Öne çıkan tokenlar
shell-action-notifications =
    .aria-label = Eylemler ve bildirimler
    .title = Eylemler ve bildirimler
shell-action-restart =
    .aria-label = Uygulamayı yeniden başlat
    .title = Uygulamayı yeniden başlat
shell-action-theme =
    .aria-label = Temayı değiştir
    .title = Temayı değiştir
shell-action-settings =
    .aria-label = Ayarlar
    .title = Ayarlar

shell-ticker-monitoring-segment =
    .title = Havuz hizmeti tarafından izlenen tokenlar
shell-ticker-monitoring = İzlenen:
shell-ticker-filtering-segment =
    .title = Filtreleme ölçütlerini geçen/geçemeyen tokenlar
shell-ticker-passed = Geçen:
shell-ticker-rejected = Reddedilen:
shell-ticker-pnl-segment =
    .title = Bugünkü gerçekleşmiş kâr ve zarar
shell-ticker-pnl = Bugünkü K/Z:
shell-ticker-rpc-segment =
    .title = Dakika başına RPC çağrısı ve başarı oranı
shell-ticker-rpc = RPC:
shell-ticker-rpc-rate = { $amount }/dk
shell-ticker-services-segment =
    .title = Arka plan hizmetlerinin sağlık durumu
shell-ticker-services-loading = Hizmetler: <strong>Yükleniyor</strong>

shell-notification-title = Eylemler
shell-notification-mark-all-read =
    .title = Tümünü okundu işaretle
shell-notification-clear-all =
    .title = Tümünü temizle
shell-notification-close =
    .aria-label = Kapat
shell-notification-tab-all = Tümü
shell-notification-tab-active = Aktif
shell-notification-tab-done = Bitti
shell-notification-tab-failed = Başarısız
shell-notification-filter-type-all = Tüm türler
shell-notification-filter-type-buy = Al
shell-notification-filter-type-sell = Sat
shell-notification-filter-type-open = Aç
shell-notification-filter-type-close = Kapat
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = Kısmi
shell-notification-filter-state-all = Tüm durumlar
shell-notification-filter-state-in-progress = Devam ediyor
shell-notification-filter-state-completed = Tamamlandı
shell-notification-filter-state-failed = Başarısız
shell-notification-filter-state-cancelled = İptal edildi
shell-notification-list =
    .aria-label = Bildirimler
shell-notification-empty = Henüz eylem yok
shell-notification-loading-more = Daha fazla yükleniyor...
shell-notification-back-to-top =
    .title = Başa dön

shell-status-bar-version = v
shell-status-bar-uptime = Çalışma
shell-status-bar-memory = Bel.
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/dk
shell-status-bar-trading = İşlem
shell-status-bar-positions = Poz.
shell-status-bar-tokens = Tokenlar

shell-splash-starting = { -brand } başlatılıyor
shell-splash-waiting = Yerel çekirdeğin yanıt vermesi bekleniyor.
shell-splash-failed = { -brand } başlatılamadı
shell-splash-failed-detail = Günlük dosyasını kontrol edin, ardından uygulamayı yeniden başlatın.

shell-connection-connected = Çekirdek bağlı
shell-connection-waiting = Çekirdek bekleniyor…
shell-connection-retry-now = Şimdi yeniden dene
shell-connection-overlay-detail = Çekirdeğe ulaşılamıyor. İşlem duraklatıldı; otomatik olarak düzelecek.
shell-connection-restored = Çekirdek bağlantısı yeniden kuruldu

shell-trader-control-failed = Trader kontrolü başarısız oldu
shell-notification-button-unread = Eylemler ve bildirimler, okunmamış: { $count }
shell-restart-confirm-title = Botu yeniden başlat
shell-restart-confirm-message =
    Botu yeniden başlatmak istediğinizden emin misiniz?

    Bu işlem:
    • Tüm hizmetleri durdurur
    • İşlemi yeniden başlatır
    • Yaklaşık 10-15 saniye sürer

    Tüm aktif işlemler kesintiye uğrayacaktır.
shell-restart-confirm-action = Yeniden başlat
shell-restart-progress = Bot yeniden başlatılıyor
shell-restart-failed = Yeniden başlatma başarısız
shell-restart-failed-status = Yeniden başlatma başarısız: { $status }
shell-restart-helper-unavailable = Otomatik yeniden başlatma yardımcısı kullanılamıyor. Paneli birazdan yeniden yükleyin.

shell-page-title-fallback = Panel
shell-page-load-failed = Sayfa yüklenemedi
shell-page-offline-detail = Çekirdeğe şu anda ulaşılamıyor. Bağlantı geri geldiğinde bu sayfa otomatik olarak yüklenecek.

shell-bot-state-explore = KEŞİF
shell-bot-state-halted = DURDURULDU
shell-bot-state-off = KAPALI
shell-bot-state-waiting = BEKLİYOR
shell-bot-state-idle = BOŞTA
shell-bot-state-entry-paused = GİRİŞ DURAKLATILDI
shell-bot-state-running = ÇALIŞIYOR
shell-bot-control-explore = Auto Trader Keşif Modu'nda kullanılamaz. Cüzdan ve RPC kurulumunu açın.
shell-bot-control-halted = Acil durdurma etkin. Auto Trader kontrollerini açın.
shell-bot-control-off = Auto Trader kapalı. Etkinleştirmek için tıklayın.
shell-bot-control-waiting = Auto Trader etkin ve çekirdek hizmetleri bekliyor. Devre dışı bırakmak için tıklayın.
shell-bot-control-idle = Auto Trader etkin, ancak iki izleyici de kapalı. Auto Trader kontrollerini açın.
shell-bot-control-entry-paused = Kayıp koruması girişleri duraklattı; çıkışlar sürebilir. Auto Trader kontrollerini açın.
shell-bot-control-running = Auto Trader çalışıyor. Devre dışı bırakmak için tıklayın.

shell-wallet-card-summary = Cüzdan değeri: { $equity } { -sol } ({ $balance } { -sol } nakit, { $tokens } token); Pozisyonlar sayfasını aç
shell-copy-running-live = { $count } canlı
shell-copy-running-paper = { $count } sanal
shell-copy-value-paused = Duraklatıldı
shell-copy-value-idle = Boşta
shell-copy-sub-active = { $active } / { $total } aktif

shell-ticker-services-healthy = Hizmetler: <strong>Sağlıklı</strong>
shell-ticker-services-issues =
    { $count ->
        [one] Hizmetler: <strong>{ $count } sorun</strong>
       *[other] Hizmetler: <strong>{ $count } sorun</strong>
    }

shell-agent-request-title = Ajan isteği
shell-agent-request-client-fallback = Eşleştirilmiş bir ajan
shell-agent-request-message = { $client }, { -brand } içinde "{ $tool }" komutunu çalıştırmak istiyor. Bu istek { $expiry }.
shell-agent-request-message-arguments = { $client }, { -brand } içinde "{ $tool }" komutunu çalıştırmak istiyor. Argümanlar: { $summary }. Bu istek { $expiry }.
shell-agent-request-expires-minutes = { $minutes }dk içinde sona erer
shell-agent-request-expires-seconds = { $seconds }sn içinde sona erer
shell-agent-request-approve = Onayla
shell-agent-request-deny = Reddet

shell-toast-copied = { $label } kopyalandı
shell-toast-copy-failed = Kopyalama başarısız
shell-toast-still-running = Hâlâ çalışıyor — bildirim merkezini kontrol edin
shell-toast-dismiss =
    .aria-label = Kapat
shell-confirm-title = Eylemi onayla
shell-confirm-message = Emin misiniz?

shell-assistant-label = Asistan
shell-assistant-dialog =
    .aria-label = Asistan

shell-status-bar-trading-active = Aktif
shell-status-bar-trading-inactive = Pasif

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title } iptal edildi
shell-action-swap-buy-live = Alınıyor
shell-action-swap-buy-done = Alındı
shell-action-swap-buy-failed = Alım başarısız
shell-action-swap-sell-live = Satılıyor
shell-action-swap-sell-done = Satıldı
shell-action-swap-sell-failed = Satım başarısız
shell-action-position-open-live = Pozisyon açılıyor
shell-action-position-open-done = Açıldı
shell-action-position-open-failed = Açma başarısız
shell-action-position-close-live = Pozisyon kapatılıyor
shell-action-position-close-done = Kapatıldı
shell-action-position-close-failed = Kapatma başarısız
shell-action-position-dca-live = Pozisyona ekleniyor
shell-action-position-dca-done = Eklendi
shell-action-position-dca-failed = Ekleme başarısız
shell-action-partial-exit-live = Kısmi çıkış
shell-action-partial-exit-done = Kısmi çıkış
shell-action-partial-exit-failed = Kısmi çıkış başarısız
shell-action-manual-order-live = Emir veriliyor
shell-action-manual-order-done = Emir verildi
shell-action-manual-order-failed = Emir başarısız
shell-action-trade-live = İşlem
shell-action-trade-done = İşlem tamamlandı
shell-action-trade-failed = İşlem başarısız
shell-action-via-router = { $router } üzerinden { $action }
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = { $venue } atlanıyor
shell-action-cost-guard-avoiding-cost = { $venue } atlanıyor · { $cost }
shell-action-cost-guard-avoiding-unnamed = bir platform atlanıyor
shell-action-cost-guard-avoiding-unnamed-cost = bir platform atlanıyor · { $cost }
shell-action-cost-guard-avoided = { $outcome } · { $venue } için { $cost } kira ödemesinden kaçınıldı
shell-action-cost-guard-avoided-unnamed = { $outcome } · { $cost } platform kirasından kaçınıldı
shell-action-exit-full = Tam çıkış
shell-action-exit-percent = { $percent } çıkış

shell-exit-title = { -brand } kapatılsın mı?
shell-exit-description = Uygulamayı nasıl kapatmak istediğinizi seçin
shell-exit-minimize = Tepsiye küçült
shell-exit-minimize-detail = Arka planda çalışmaya devam et
shell-exit-quit = Uygulamadan çık
shell-exit-quit-detail = Tamamen kapat ve tüm hizmetleri durdur

shell-lightbox-save =
    .title = Görseli kaydet
shell-lightbox-close =
    .title = Kapat (ESC)

shell-theme-light = Açık
shell-theme-dark = Koyu
shell-theme-switch-to-light = Açık temaya geç
shell-theme-switch-to-dark = Koyu temaya geç
