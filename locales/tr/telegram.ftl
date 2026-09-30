## Reply keyboard words. Icons come from ReplyCommand in src/telegram/reply.rs.

telegram-reply-status = Durum
telegram-reply-balance = Bakiye
telegram-reply-positions = Pozisyonlar
telegram-reply-pause = Duraklat
telegram-reply-resume = Sürdür
telegram-reply-stop = Durdur
telegram-reply-stats = İstatistik
telegram-reply-menu = Menü
telegram-reply-help = Yardım

## Inline keyboard buttons.

telegram-button-positions = Pozisyonlar
telegram-button-balance = Bakiye
telegram-button-stats = İstatistik
telegram-button-tokens = Tokenlar
telegram-button-pause = Duraklat
telegram-button-stop = Durdur
telegram-button-settings = Ayarlar
telegram-button-refresh = Yenile
telegram-button-menu = Menü
telegram-button-back = Geri
telegram-button-back-to-menu = Menüye Dön
telegram-button-back-to-tokens = Tokenlara Dön
telegram-button-cancel = İptal
telegram-button-close-all-positions = Tüm Pozisyonları Kapat
telegram-button-sell-percent = Sat %{ $percent }
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = Kara Liste
telegram-button-blacklist-symbol = Kara Liste: { $symbol }
telegram-button-close-position = Pozisyonu Kapat
telegram-button-confirm-close = Kapatmayı Onayla
telegram-button-confirm-close-all = TÜM Pozisyonları Kapat
telegram-button-confirm-sell = Satışı Onayla %{ $percent }
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = ZORLA DURDURMAYI ONAYLA
telegram-button-confirm-buy = Al { $amount } { -sol }
telegram-button-notifications = Bildirimler
telegram-button-trading = İşlem
telegram-button-entry-monitor = Giriş İzleyici
telegram-button-exit-monitor = Çıkış İzleyici
telegram-button-auto-trading = Otomatik İşlem
telegram-button-force-stop = Zorla Durdur
telegram-button-notify-opened = Açıldı
telegram-button-notify-closed = Kapandı
telegram-button-notify-partial = Kısmi
telegram-button-notify-dca = DCA
telegram-button-notify-errors = Hatalar
telegram-button-details = Ayrıntılar
telegram-button-position = Pozisyon
telegram-button-sell-more = Daha Fazla Sat
telegram-button-more-dca = Daha Fazla DCA
telegram-button-history = Geçmiş
telegram-button-status = Durum
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = Yeniden Doğrula
telegram-button-previous = Önceki
telegram-button-next = Sonraki
telegram-button-passed = Geçti
telegram-button-rejected = Reddedildi
telegram-button-new-24h = Yeni (24sa)
telegram-button-all-tokens = Tüm Tokenlar
telegram-button-search-token = Token Ara
telegram-button-filter-stats = Filtre İstatistikleri
telegram-button-refresh-stats = İstatistikleri Yenile
telegram-button-view-position = Pozisyonu Gör
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    Bilinmeyen komut: { $command }

    Kullanılabilir komutları görmek için /help yazın.
telegram-session-expired =
    <b>Oturum Süresi Doldu</b>

    Yeniden doğrulamak için /login kullanın.
telegram-2fa-required =
    <b>2FA Gerekli</b>

    Lütfen 6 haneli doğrulayıcı kodunuzu girin.
telegram-account-locked =
    <b>Hesap Kilitlendi</b>

    Çok fazla başarısız deneme.
    Tekrar deneyebilmek için beklemeniz gereken süre: { $seconds ->
        [one] { $seconds } sn.
       *[other] { $seconds } sn.
    }
telegram-code-invalid = Lütfen geçerli bir 6 haneli kod girin.
telegram-authenticated =
    <b>Doğrulandı!</b>

    Artık bot komutlarına erişebilirsiniz.
telegram-wrong-code =
    <b>Hatalı Kod</b>

    { $remaining ->
        [one] Kalan deneme hakkı: { $remaining }.
       *[other] Kalan deneme hakkı: { $remaining }.
    }
telegram-auth-required =
    <b>Doğrulama Gerekli</b>

    Devam etmek için lütfen parolanızı girin.

    <i>Parolanızı yazıp gönderin.</i>
telegram-login-required =
    <b>Giriş Gerekli</b>

    Lütfen 6 haneli doğrulayıcı kodunuzu girin:
telegram-session-activated =
    <b>Oturum Etkinleştirildi</b>

    2FA yapılandırılmamış. Oturumunuz artık etkin.

    <i>İpucu: Daha iyi güvenlik için Güvenlik ayarlarından 2FA'yı etkinleştirin.</i>

## Chat discovery.

telegram-discovery-hello = Merhaba { $name }!
telegram-discovery-default-name = Kullanıcı
telegram-discovery-detected = <b>Sohbet algılandı!</b>
telegram-discovery-details =
    Sohbet Kimliği: <code>{ $chat_id }</code>
    Tür: { $chat_type }

    Lütfen { -brand } panelinde bu sohbete tıklayarak seçin.
telegram-chat-type-private = özel
telegram-chat-type-group = grup
telegram-chat-type-supergroup = süper grup
telegram-chat-type-channel = kanal

## Menus.

telegram-menu-title =
    <b>Kontrol Paneli</b>

    Bilgi görmek veya botu yönetmek için bir seçenek belirleyin.
telegram-menu-positions-empty =
    <b>Açık Pozisyon Yok</b>

    Yeni fırsatlar bekleniyor...
telegram-menu-positions-title = <b>Pozisyonlar ({ $count })</b>
telegram-menu-positions-hint = <i>Yönetmek için bir pozisyona dokunun.</i>
telegram-menu-settings =
    <b>Ayarlar</b>

    Bildirimleri ve işlem parametrelerini yapılandırın.
telegram-settings-notifications =
    <b>Bildirim Ayarları</b>

    Bildirimleri açın veya kapatın:
telegram-settings-trading =
    <b>İşlem Kontrolleri</b>

    İşlem özelliklerini açın veya kapatın:
telegram-pagination-expired = Sayfalama oturumunun süresi doldu.

## Status commands.

telegram-status-state-stopped = <b>DURDURULDU</b> (Zorla Durdurma Etkin)
telegram-status-state-active = <b>ETKİN</b>
telegram-status-state-paused = <b>DURAKLATILDI</b>
telegram-status-on = AÇIK
telegram-status-off = KAPALI
telegram-status-body =
    <b>Sistem Durumu</b>

    <b>Sistem</b>
    Durum — { $state }
    Çalışma süresi — { $uptime }
    Sürüm — v{ $version }

    <b>İşlem</b>
    Girişler — { $entries }
    Çıkışlar — { $exits }
    Pozisyonlar — { $positions }
telegram-positions-empty =
    <b>Açık Pozisyon Yok</b>

    Fırsatlar bekleniyor...
telegram-positions-title = <b>Açık Pozisyonlar ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+{ $count } tane daha...</i>
telegram-positions-summary =
    <b>Portföy Özeti</b>
    Yatırılan — { $invested } { -sol }
    Net K{ "&amp;" }Z — { $pnl } { -sol }
telegram-balance-body =
    <b>Cüzdan Bakiyesi</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>Günlük İstatistikler</b>

    Pozisyonlar — { $positions }
    Yatırılan — { $invested } { -sol }
    K{ "&amp;" }Z — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } Hazır!</b>

    İşlem <b>etkin</b>.

    Botu yönetmek için aşağıdaki klavyeyi kullanın.
    Kullanılabilir komutlar için /help yazın.
telegram-stop-already = <b>İşlem zaten devre dışı</b>
telegram-stop-done =
    <b>İşlem Devre Dışı</b>

    Tüm işlem izleyicileri (giriş { "&amp;" } çıkış) durduruldu.
    Yalnızca girişleri durdurmak için /pause kullanın.
telegram-stop-failed =
    <b>İşlem devre dışı bırakılamadı</b>

    Hata: { $detail }
telegram-pause-done =
    <b>Giriş İzleyici Duraklatıldı</b>

    Yeni pozisyon açılmayacak.
    Çıkış izleyici çalışmaya devam ediyor.
telegram-pause-failed =
    <b>Girişler duraklatılamadı</b>

    Hata: { $detail }
telegram-resume-done =
    <b>Giriş İzleyici Sürdürüldü</b>

    Giriş sinyalleri izleniyor.
telegram-resume-failed =
    <b>Girişler sürdürülemedi</b>

    Hata: { $detail }
telegram-force-stop-confirm =
    <b>ZORLA DURDUR</b>

    Bu işlem TÜM işlem etkinliğini hemen durdurur:
    • Yeni giriş yok
    • Çıkış yok (zarar durdur dahil)
    • DCA işlemi yok
telegram-force-stop-warning = <b>Bu bir acil durum işlemidir!</b>
telegram-force-stop-question = Emin misiniz?
telegram-force-stop-active =
    <b>ZORLA DURDURMA ETKİNLEŞTİRİLDİ</b>

    Tüm işlemler durduruldu.

    Bu bayrağı temizlemek için /resume_trading kullanın.
telegram-resume-trading-not-stopped =
    <b>İşlem zorla durdurulmamış</b>

    Yapılacak bir şey yok.
telegram-resume-trading-done =
    <b>İşlem Sürdürüldü</b>

    Zorla durdurma bayrağı temizlendi.
    Normal işlem operasyonları yeniden başlayabilir.

## Help.

telegram-help-title = <b>{ -brand } Yardım</b>
telegram-help-heading-dashboard = Panel
telegram-help-heading-market = Piyasa
telegram-help-heading-trading = İşlem
telegram-help-heading-safety = Güvenlik
telegram-help-heading-system = Sistem
telegram-help-commands-dashboard =
    /status — Sistem durumu { "&amp;" } çalışma süresi
    /stats — Günlük performans
    /balance — Cüzdan bakiyesi
    /positions — Açık pozisyonlar
telegram-help-commands-market =
    /tokens — Token gezgini
    /rejected — Filtrelenen tokenlar
telegram-help-commands-trading =
    /start — İşlem sistemini etkinleştir
    /stop — İşlem sistemini devre dışı bırak
    /pause — Yeni girişleri duraklat
    /resume — Yeni girişleri sürdür
    /menu — Etkileşimli menü
telegram-help-commands-safety =
    /force_stop — <b>ACİL DURDURMA</b>
    /resume_trading — Acil durum durumunu temizle
telegram-help-commands-system =
    /update — Güncelleme durumu { "&amp;" } kurulum
    /login — 2FA doğrulaması
telegram-help-tip = <i>İpucu: Çalıştırmak için bir komuta dokunun.</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>Güncel</b>

    v{ $version } çalışıyor, otomatik olarak kuruldu.
telegram-update-up-to-date =
    <b>Güncel</b>

    v{ $version } çalışıyor.
telegram-update-check-failed =
    <b>Güncelleme denetimi başarısız</b>

    { $reason }
telegram-update-unreachable = screenerbot.io adresine ulaşılamadı.
telegram-update-installing = <b>v{ $version } kuruluyor</b>
telegram-update-restarting =
    { -brand } yeni sürüme geçmek için yeniden başlıyor. İşlem otomatik olarak sürer.
telegram-update-install-failed =
    <b>v{ $version } kurulamadı</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } indirildi</b>

    Bu sürüm masaüstü uygulamasını da güncelliyor, bu yüzden yükleyicisinin makinede çalıştırılması gerekir. Orada Ayarlar → Güncellemeler bölümünü açın.
telegram-update-downloading =
    <b>v{ $version } indiriliyor</b>

    { $size } MB içinden %{ $percent }.
telegram-update-available =
    <b>v{ $version } mevcut</b>

    { $how }
    İndirme boyutu: { $size } MB.

    Kendiliğinden iner; hazır olduğunda /update komutunu yeniden gönderin.
telegram-update-how-core = Kısa bir yeniden başlatmayla sessizce kurulur.
telegram-update-how-installer = Masaüstü yükleyicisinin bir kez çalıştırılması gerekir.

## Shared values and units. Numbers arrive formatted; only the unit words live here.

telegram-value-unknown = Bilinmiyor
telegram-value-na = Yok
telegram-percent-value = { $percent }%
telegram-price-sol = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds }sn
telegram-duration-minutes = { $minutes }dk
telegram-duration-minutes-seconds = { $minutes }dk { $seconds }sn
telegram-duration-hours = { $hours }sa
telegram-duration-hours-minutes = { $hours }sa { $minutes }dk
telegram-duration-days = { $days }g
telegram-duration-days-hours = { $days }g { $hours }sa
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-sol = { $amount } { -sol }
telegram-error-line = Hata: { $detail }
telegram-ai-reasoning =
    <b>LLM Analizi</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens. Icons come from Rust.

telegram-row-entry = Giriş — { $price } { -sol }
telegram-row-exit = Çıkış — { $price } { -sol }
telegram-row-current = Güncel — { $price } { -sol }
telegram-row-invested = Yatırılan — { $amount } { -sol }
telegram-row-received = Alınan — { $amount } { -sol }
telegram-row-value = Değer — { $amount } { -sol }
telegram-row-total = Toplam — { $amount } { -sol }
telegram-row-tokens = Tokenlar — { $tokens }
telegram-row-duration = Süre — { $duration }
telegram-row-reason = Neden — { $reason }
telegram-row-remaining = Kalan — %{ $percent }
telegram-row-pnl = K{ "&amp;" }Z — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>Pozisyon Açıldı</b>
telegram-notify-opened-size = Boyut — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = Fiyat — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>Pozisyon Kapandı</b> — Kâr
telegram-notify-closed-title-loss = <b>Pozisyon Kapandı</b> — Zarar
telegram-notify-closed-reason-unspecified = Kapandı
telegram-notify-partial-title = <b>Kısmi Çıkış</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — Satılan: %{ $percent }
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = Eklenen — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = Ort. — { $price } { -sol }
telegram-notify-severity-critical = <b>Kritik Hata</b>
telegram-notify-severity-error = <b>Hata</b>
telegram-notify-severity-warning = <b>Uyarı</b>
telegram-notify-severity-info = <b>Bilgi</b>
telegram-notify-alert-title = <b>İşlem Uyarısı</b>
telegram-notify-alert-token = Token: <code>${ $symbol }</code>
telegram-notify-alert-mint = Mint: <code>{ $mint }</code>
telegram-notify-alert-bought = Eylem: { $amount } { -sol } alındı
telegram-notify-alert-sold = Eylem: { $amount } { -sol } satıldı
telegram-notify-alert-wallet = Cüzdan: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (sanal)
telegram-notify-copy-task = Görev: { $task }
telegram-notify-scheduled-completed = <b>Zamanlanmış Görev tamamlandı</b>
telegram-notify-scheduled-failed = <b>Zamanlanmış Görev başarısız oldu</b>
telegram-notify-scheduled-timed-out = <b>Zamanlanmış Görev zaman aşımına uğradı</b>
telegram-notify-scheduled-error = Hata: { $error }
telegram-notify-summary-title = <b>Günlük Özet</b> — { $date }
telegram-notify-summary-performance = <b>Performans</b>
telegram-notify-summary-trades = İşlemler — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = Kazanma Oranı — %{ $percent }
telegram-notify-summary-pnl = K{ "&amp;" }Z — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = Açık Pozisyonlar — { $count }
telegram-notify-started-title = <b>{ -brand } Başlatıldı</b>
telegram-notify-started-version = <b>Sürüm</b> — { $version }
telegram-notify-started-mode = <b>Mod</b> — { $mode }
telegram-notify-started-ready = İşleme hazır!
telegram-notify-stopped-title = <b>{ -brand } Durduruldu</b>
telegram-notify-stopped-reason = <b>Neden</b> — { $reason }
telegram-notify-stopped-goodbye = Hoşça kalın! { $icon }
telegram-notify-start-mode-normal = Normal
telegram-notify-stop-reason-graceful = Düzenli kapatma
telegram-notify-update-available =
    <b>v{ $version } güncellemesi mevcut</b>

    { $how }
    İndirme boyutu: { $size } MB
telegram-notify-update-how-installer = Bu sürüm masaüstü uygulamasını da güncelliyor, bu yüzden yükleyicisinin bir kez çalıştırılması gerekir.
telegram-notify-update-ready =
    <b>v{ $version } güncellemesi hazır</b>

    { $how }
telegram-notify-update-ready-silent = Hemen uygulamak için /update gönderin; aksi halde { -brand } bir sonraki başlayışında kurulur.
telegram-notify-update-ready-installer = Yükleyiciyi çalıştırmak için Ayarlar → Güncellemeler bölümünü açın.
telegram-notify-update-applying =
    <b>v{ $version } kuruluyor</b>

    Arka uç yeniden başlıyor; işlem otomatik olarak sürer.
telegram-notify-new-tokens =
    <b>Filtreleme Uyarısı</b>

    { $count ->
        [one] Ölçütlerinize uyan yeni token sayısı: { $count }.
       *[other] Ölçütlerinize uyan yeni token sayısı: { $count }.
    }
telegram-notify-crash =
    <b>Bot Çöktü!</b>

    <b>Konum:</b> <code>{ $location }</code>
    <b>Hata:</b> <code>{ $error }</code>
telegram-notify-crash-restart = Lütfen botu yeniden başlatın.

## Filter results page.

telegram-filter-results-title = <b>Filtre Sonuçları</b> ({ $count })
telegram-filter-results-empty = <i>Token bulunamadı.</i>
telegram-filter-results-page = <i>Sayfa { $page } / { $total }</i>

## Position screens.

telegram-position-not-found = Pozisyon bulunamadı
telegram-position-no-positions = Kapatılacak pozisyon yok
telegram-position-history-empty =
    <b>İşlem Geçmişi</b>

    Henüz kapalı pozisyon yok.
telegram-position-history-title = <b>Son İşlemler</b>
telegram-position-history-more = <i>+{ $count } işlem daha...</i>
telegram-position-confirm-hint = <i>Yürütmek için 30sn içinde onaylayın.</i>
telegram-position-confirm-close-title = <b>Pozisyon Kapatılsın mı?</b>
telegram-position-confirm-close-selling = Satılan token: { $tokens }
telegram-position-confirm-close-estimated = Tahmini — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>30 saniye içinde onaylayın</i>
telegram-position-confirm-sell =
    <b>Satışı Onayla</b>

    Token — { $symbol }
    Miktar — %{ $percent }
    Tokenlar — { $tokens }
telegram-position-confirm-dca =
    <b>Ek Alımı Onayla</b>

    Token — { $symbol }
    Ekleme — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>Tüm Pozisyonlar Kapatılsın mı?</b>

    Adet — { $count }
telegram-position-confirm-close-all-hint =
    <i>Bu işlem tüm açık pozisyonları piyasa fiyatından satar.
    30sn içinde onaylayın.</i>
telegram-position-confirm-force-stop =
    <b>ZORLA DURDUR</b>

    Bu işlem TÜM işlemleri hemen durdurur:
    • Yeni giriş yok
    • Çıkış yok
    • DCA yok
telegram-position-confirm-force-stop-warning = <b>Bu bir acil durum işlemidir.</b>
telegram-position-confirm-blacklist =
    <b>Token Kara Listeye Alınsın mı?</b>

    Token — { $symbol }
    Mint — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>Bu işlem pozisyonu kapatır ve gelecekteki girişleri engeller.</i>
telegram-position-selling = { $symbol } tokenının %{ $percent } kadarı satılıyor...
telegram-position-sell-done =
    <b>Satış Gerçekleştirildi</b>

    Token — { $symbol }
    Satılan — %{ $percent }
    Alınan — { $amount } { -sol }
telegram-position-sell-failed = <b>Satış Başarısız</b>
telegram-position-adding = { $symbol } için { $amount } { -sol } ekleniyor...
telegram-position-dca-done =
    <b>DCA Gerçekleştirildi</b>

    Token — { $symbol }
    Eklenen — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA Başarısız</b>
telegram-position-closing-all = Tüm pozisyonlar kapatılıyor...
telegram-position-close-all-done =
    <b>Tümünü Kapatma Tamamlandı</b>

    Kapatılan — { $closed }
    Başarısız — { $failed }
telegram-position-blacklisted =
    <b>Token Kara Listeye Alındı</b>

    Token — { $symbol }
    Durum — Kapatıldı { "&amp;" } Kara Listeye Alındı

## Token screens.

telegram-token-not-found = Token bulunamadı
telegram-token-not-found-prefix = Token bulunamadı. Daha uzun bir önekle aramayı deneyin.
telegram-token-stats-failed = İstatistikler alınamadı: { $detail }
telegram-token-list-failed = Tokenlar alınamadı: { $detail }
telegram-token-list-empty = <b>{ $view }</b> görünümünde token bulunamadı.
telegram-token-view-passed = Filtreyi Geçenler
telegram-token-view-rejected = Reddedilenler
telegram-token-view-recent = Yeni Eklenenler
telegram-token-view-all = Tüm Tokenlar
telegram-token-list-title = <b>{ $name }</b> (Sayfa { $page }/{ $total })
telegram-token-list-stats = Likidite: { $liquidity } • Fiyat: { $price }
telegram-token-list-hint = <i>Ayrıntılar için /token_ID öğesine dokunun</i>
telegram-token-explorer =
    <b>Piyasa Gezgini</b>

    <b>Genel Bakış</b>
    Filtreyi Geçen — { $passed }
    Reddedilen — { $rejected }
    Aktif Fiyatlı — { $priced }
    Toplam Keşfedilen — { $total }

    <i>Göz atmak için bir kategori seçin:</i>
telegram-token-filter-title = <b>Filtre Analizi</b>
telegram-token-filter-distribution = <b>Dağılım</b>
telegram-token-filter-passed = Geçti — { $count } ({ $percent }%)
telegram-token-filter-rejected = Reddedildi — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = Kara listede — { $count }
telegram-token-filter-coverage = <b>Kapsam</b>
telegram-token-filter-priced = Havuz Fiyatı Olan — { $count }
telegram-token-filter-open = Açık Pozisyonlar — { $count }
telegram-token-filter-total = Toplam Keşfedilen — { $count }
telegram-token-filter-updated = <b>Son Güncelleme</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>Her { $interval } aralıkta otomatik yenilenir</i>
telegram-token-detail-active = <b>Aktif Pozisyon</b>
telegram-token-detail-price = Fiyat — { $price } { -sol }
telegram-token-detail-liquidity = Likidite — { $value }
telegram-token-detail-volume = 24sa Hacim — { $value }
telegram-token-detail-change = 24sa Değişim — { $value }
telegram-token-detail-risk = Risk Değerlendirmesi: { $score }/100
telegram-token-detail-risk-unknown = Risk Değerlendirmesi: Bilinmiyor
telegram-token-detail-action = <i>Eylem seçin:</i>
telegram-token-search =
    <b>Piyasada Ara</b>

    Aramak için sembol veya mint adresi girin:

    <i>Örnek: /token_BONK veya /token_So11111</i>
telegram-token-confirm-buy =
    <b>Doğrudan Alımı Onayla</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>
    Miktar — { $amount } { -sol }

    <i>Yürütmek için 30sn içinde onaylayın.</i>
telegram-token-confirm-blacklist =
    <b>Token Kara Listeye Alınsın mı?</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>

    <i>Bu işlem tokenın filtreleri geçmesini engeller.</i>
telegram-token-blacklisted =
    <b>Token Kara Listeye Alındı</b>

    Token — ${ $symbol }
    Durum — Kara listeye eklendi
telegram-token-blacklist-failed = <b>Kara Listeye Ekleme Başarısız</b>
telegram-token-buy-processing =
    <b>Alım İşleniyor...</b>

    Token — ${ $symbol }
    Miktar — { $amount } { -sol }
telegram-token-buy-done =
    <b>Alım Başarılı</b>

    Token — ${ $symbol }
    Miktar — { $amount } { -sol }

    <i>Ayrıntıları /positions içinde görün</i>
telegram-token-buy-failed =
    <b>Alım Başarısız</b>

    Token — ${ $symbol }
    Hata — { $detail }
