notifications-action-swap-buy = Al
notifications-action-swap-sell = Sat
notifications-action-position-open = Aç
notifications-action-position-close = Kapat
notifications-action-position-dca = DCA
notifications-action-position-partial-exit = Kısmi Çıkış
notifications-action-manual-order = Manuel
notifications-action-unknown = İşlem

actions-step-evaluate = Değerlendiriliyor
actions-step-validate = Doğrulanıyor
actions-step-quote = Fiyat Teklifi Alınıyor
actions-step-swap = Takas Yürütülüyor
actions-step-verify = Doğrulanıyor
actions-step-unknown = İşleniyor
actions-step-evaluate-short = Değerlendiriliyor
actions-step-validate-short = Kontrol
actions-step-quote-short = Teklif
actions-step-swap-short = Takas
actions-step-verify-short = Onaylanıyor
actions-step-unknown-short = Çalışıyor

actions-failure-recorded = { $message }
actions-failure-unknown = Bilinmeyen hata
actions-failure-interrupted = Uygulama yeniden başlatıldığı için kesildi
actions-failure-validation = Doğrulama başarısız
actions-failure-quote = Fiyat teklifi alınamadı
actions-failure-swap = Takas başarısız
actions-failure-trade = İşlem başarısız
actions-failure-entry = Giriş başarısız
actions-failure-exit = Çıkış başarısız
actions-failure-dca = DCA başarısız
actions-failure-verification-expired = Doğrulama süresi doldu: işlem zincire hiç ulaşmadı
actions-failure-verification-gave-up = Doğrulamadan vazgeçildi
actions-failure-transaction-failed = İşlem zincirde başarısız oldu
actions-failure-sell-transaction-failed = Satış işlemi zincirde başarısız oldu
actions-failure-dca-verification-failed = DCA doğrulaması başarısız

notifications-empty-all = İşlem yok
notifications-empty-active = Etkin işlem yok
notifications-empty-completed = Tamamlanan işlem yok
notifications-empty-failed = Başarısız işlem yok
notifications-source-auto = Otomatik
notifications-source-manual = Manuel
notifications-cancelled = İptal edildi
notifications-load-failed = Yüklenemedi
notifications-clear-title = Bildirimleri temizle
notifications-clear-message = Bu listedeki tüm bildirimler kapatılsın mı? Bildirimler Tamamlanan/Başarısız geçmişinde kalır.
notifications-clear-failed = Bildirimler temizlenemedi
notifications-stream-lag-title = İşlem akışı geride kaldı
notifications-stream-lag-missed =
    { $count ->
        [one] { $count } güncelleme
       *[other] { $count } güncelleme
    } kaçırıldı — yenileniyor
notifications-stream-lag-refreshing = Yenileniyor
notifications-sync-failed = İşlemler yenilenemedi

# Action rows (ui/action_row.js): day headings, the copy-trading source, the detail.
notifications-day-today = Bugün
notifications-day-yesterday = Dün
notifications-source-copy = Kopya işlem
notifications-steps = Adımlar
notifications-detail-transaction = İşlem
notifications-detail-fill-price = Gerçekleşen fiyat
    .title = Bu takastaki { -sol } tutarının token miktarına bölümü
notifications-detail-fee = Ağ ücreti
notifications-detail-pnl = Pozisyon K/Z
notifications-detail-route = Rota
notifications-detail-duration = Süre
notifications-detail-reason = Neden
notifications-view-token = Token'ı görüntüle
notifications-view-position = Pozisyonu görüntüle
notifications-view-transaction = İşlemi görüntüle
notifications-unrecorded = Pozisyona kaydedilmedi
notifications-unrecorded-detail = Bu takas pozisyona kaydedilmedi ve gerçekleşmemiş olabilir. İşlemini kontrol edin.
