## Reply keyboard words. Icons come from ReplyCommand in src/telegram/reply.rs.

telegram-reply-status = Trạng thái
telegram-reply-balance = Số dư
telegram-reply-positions = Vị thế
telegram-reply-pause = Tạm dừng
telegram-reply-resume = Tiếp tục
telegram-reply-stop = Dừng
telegram-reply-stats = Thống kê
telegram-reply-menu = Menu
telegram-reply-help = Trợ giúp

## Inline keyboard buttons.

telegram-button-positions = Vị thế
telegram-button-balance = Số dư
telegram-button-stats = Thống kê
telegram-button-tokens = Token
telegram-button-pause = Tạm dừng
telegram-button-stop = Dừng
telegram-button-settings = Cài đặt
telegram-button-refresh = Làm mới
telegram-button-menu = Menu
telegram-button-back = Quay lại
telegram-button-back-to-menu = Về menu
telegram-button-back-to-tokens = Về danh sách token
telegram-button-cancel = Hủy
telegram-button-close-all-positions = Đóng tất cả vị thế
telegram-button-sell-percent = Bán { $percent }%
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = Đưa vào danh sách đen
telegram-button-blacklist-symbol = Đưa { $symbol } vào danh sách đen
telegram-button-close-position = Đóng vị thế
telegram-button-confirm-close = Xác nhận đóng
telegram-button-confirm-close-all = Đóng TẤT CẢ vị thế
telegram-button-confirm-sell = Xác nhận bán { $percent }%
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = XÁC NHẬN DỪNG CƯỠNG CHẾ
telegram-button-confirm-buy = Mua { $amount } { -sol }
telegram-button-notifications = Thông báo
telegram-button-trading = Giao dịch
telegram-button-entry-monitor = Giám sát vào lệnh
telegram-button-exit-monitor = Giám sát thoát lệnh
telegram-button-auto-trading = Giao dịch tự động
telegram-button-force-stop = Dừng cưỡng chế
telegram-button-notify-opened = Đã mở
telegram-button-notify-closed = Đã đóng
telegram-button-notify-partial = Một phần
telegram-button-notify-dca = DCA
telegram-button-notify-errors = Lỗi
telegram-button-details = Chi tiết
telegram-button-position = Vị thế
telegram-button-sell-more = Bán thêm
telegram-button-more-dca = DCA thêm
telegram-button-history = Lịch sử
telegram-button-status = Trạng thái
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = Xác thực lại
telegram-button-previous = Trước
telegram-button-next = Sau
telegram-button-passed = Đạt
telegram-button-rejected = Bị loại
telegram-button-new-24h = Mới (24h)
telegram-button-all-tokens = Tất cả token
telegram-button-search-token = Tìm token
telegram-button-filter-stats = Thống kê bộ lọc
telegram-button-refresh-stats = Làm mới thống kê
telegram-button-view-position = Xem vị thế
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    Lệnh không xác định: { $command }

    Dùng /help để xem các lệnh khả dụng.
telegram-session-expired =
    <b>Phiên đã hết hạn</b>

    Dùng /login để xác thực lại.
telegram-2fa-required =
    <b>Yêu cầu 2FA</b>

    Vui lòng nhập mã xác thực gồm 6 chữ số.
telegram-account-locked =
    <b>Tài khoản đã bị khóa</b>

    Quá nhiều lần thử thất bại.
    Hãy thử lại sau { $seconds ->
       *[other] { $seconds } giây.
    }
telegram-code-invalid = Vui lòng nhập mã hợp lệ gồm 6 chữ số.
telegram-authenticated =
    <b>Đã xác thực!</b>

    Bạn đã có quyền dùng các lệnh của bot.
telegram-wrong-code =
    <b>Sai mã</b>

    { $remaining ->
       *[other] Còn { $remaining } lần thử.
    }
telegram-auth-required =
    <b>Yêu cầu xác thực</b>

    Vui lòng nhập mật khẩu của bạn để tiếp tục.

    <i>Hãy gõ mật khẩu và gửi đi.</i>
telegram-login-required =
    <b>Yêu cầu đăng nhập</b>

    Vui lòng nhập mã xác thực gồm 6 chữ số:
telegram-session-activated =
    <b>Phiên đã được kích hoạt</b>

    2FA chưa được cấu hình. Phiên của bạn hiện đã hoạt động.

    <i>Mẹo: Bật 2FA trong phần cài đặt Bảo mật để an toàn hơn.</i>

## Chat discovery.

telegram-discovery-hello = Xin chào { $name }!
telegram-discovery-default-name = Người dùng
telegram-discovery-detected = <b>Đã phát hiện cuộc trò chuyện!</b>
telegram-discovery-details =
    Chat ID: <code>{ $chat_id }</code>
    Loại: { $chat_type }

    Vui lòng vào bảng điều khiển { -brand } và nhấp vào cuộc trò chuyện này để chọn.
telegram-chat-type-private = riêng tư
telegram-chat-type-group = nhóm
telegram-chat-type-supergroup = siêu nhóm
telegram-chat-type-channel = kênh

## Menus.

telegram-menu-title =
    <b>Bảng điều khiển</b>

    Chọn một tùy chọn để xem thông tin hoặc điều khiển bot.
telegram-menu-positions-empty =
    <b>Không có vị thế đang mở</b>

    Đang chờ cơ hội mới...
telegram-menu-positions-title = <b>Vị thế ({ $count })</b>
telegram-menu-positions-hint = <i>Chạm vào một vị thế để quản lý.</i>
telegram-menu-settings =
    <b>Cài đặt</b>

    Cấu hình thông báo và các tham số giao dịch.
telegram-settings-notifications =
    <b>Cài đặt thông báo</b>

    Bật/tắt thông báo:
telegram-settings-trading =
    <b>Điều khiển giao dịch</b>

    Bật/tắt các tính năng giao dịch:
telegram-pagination-expired = Phiên phân trang đã hết hạn.

## Status commands.

telegram-status-state-stopped = <b>ĐÃ DỪNG</b> (Đang dừng cưỡng chế)
telegram-status-state-active = <b>ĐANG HOẠT ĐỘNG</b>
telegram-status-state-paused = <b>TẠM DỪNG</b>
telegram-status-on = BẬT
telegram-status-off = TẮT
telegram-status-body =
    <b>Trạng thái hệ thống</b>

    <b>Hệ thống</b>
    Trạng thái — { $state }
    Thời gian hoạt động — { $uptime }
    Phiên bản — v{ $version }

    <b>Giao dịch</b>
    Vào lệnh — { $entries }
    Thoát lệnh — { $exits }
    Vị thế — { $positions }
telegram-positions-empty =
    <b>Không có vị thế đang mở</b>

    Đang chờ cơ hội...
telegram-positions-title = <b>Vị thế đang mở ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+{ $count } vị thế khác...</i>
telegram-positions-summary =
    <b>Tổng quan danh mục</b>
    Đã đầu tư — { $invested } { -sol }
    P{ "&amp;" }L ròng — { $pnl } { -sol }
telegram-balance-body =
    <b>Số dư ví</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>Thống kê hằng ngày</b>

    Vị thế — { $positions }
    Đã đầu tư — { $invested } { -sol }
    P{ "&amp;" }L — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } đã sẵn sàng!</b>

    Giao dịch đang <b>được bật</b>.

    Dùng bàn phím bên dưới để điều khiển bot.
    Gõ /help để xem các lệnh khả dụng.
telegram-stop-already = <b>Giao dịch đã được tắt</b>
telegram-stop-done =
    <b>Đã tắt giao dịch</b>

    Tất cả bộ giám sát giao dịch (vào lệnh { "&amp;" } thoát lệnh) đã dừng.
    Dùng /pause để chỉ dừng vào lệnh.
telegram-stop-failed =
    <b>Không tắt được giao dịch</b>

    Lỗi: { $detail }
telegram-pause-done =
    <b>Đã tạm dừng giám sát vào lệnh</b>

    Sẽ không mở vị thế mới.
    Bộ giám sát thoát lệnh vẫn tiếp tục chạy.
telegram-pause-failed =
    <b>Không tạm dừng được vào lệnh</b>

    Lỗi: { $detail }
telegram-resume-done =
    <b>Đã tiếp tục giám sát vào lệnh</b>

    Đang theo dõi các tín hiệu vào lệnh.
telegram-resume-failed =
    <b>Không tiếp tục được vào lệnh</b>

    Lỗi: { $detail }
telegram-force-stop-confirm =
    <b>DỪNG CƯỠNG CHẾ</b>

    Thao tác này sẽ dừng ngay TOÀN BỘ hoạt động giao dịch:
    • Không vào lệnh mới
    • Không thoát lệnh (kể cả stop loss)
    • Không thực hiện DCA
telegram-force-stop-warning = <b>Đây là hành động khẩn cấp!</b>
telegram-force-stop-question = Bạn có chắc không?
telegram-force-stop-active =
    <b>ĐÃ KÍCH HOẠT DỪNG CƯỠNG CHẾ</b>

    Toàn bộ giao dịch đã bị dừng.

    Dùng /resume_trading để xóa cờ này.
telegram-resume-trading-not-stopped =
    <b>Giao dịch không bị dừng cưỡng chế</b>

    Không cần thao tác gì.
telegram-resume-trading-done =
    <b>Đã tiếp tục giao dịch</b>

    Cờ dừng cưỡng chế đã được xóa.
    Hoạt động giao dịch bình thường có thể tiếp tục.

## Help.

telegram-help-title = <b>Trợ giúp { -brand }</b>
telegram-help-heading-dashboard = Bảng điều khiển
telegram-help-heading-market = Thị trường
telegram-help-heading-trading = Giao dịch
telegram-help-heading-safety = An toàn
telegram-help-heading-system = Hệ thống
telegram-help-commands-dashboard =
    /status — Trạng thái hệ thống { "&amp;" } thời gian hoạt động
    /stats — Hiệu suất hằng ngày
    /balance — Số dư ví
    /positions — Vị thế đang mở
telegram-help-commands-market =
    /tokens — Trình khám phá token
    /rejected — Token bị lọc
telegram-help-commands-trading =
    /start — Bật hệ thống giao dịch
    /stop — Tắt hệ thống giao dịch
    /pause — Tạm dừng vào lệnh mới
    /resume — Tiếp tục vào lệnh mới
    /menu — Menu tương tác
telegram-help-commands-safety =
    /force_stop — <b>DỪNG KHẨN CẤP</b>
    /resume_trading — Xóa trạng thái khẩn cấp
telegram-help-commands-system =
    /update — Trạng thái cập nhật { "&amp;" } cài đặt
    /login — Xác thực 2FA
telegram-help-tip = <i>Mẹo: Chạm vào một lệnh để chạy.</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>Đã là bản mới nhất</b>

    Đang chạy v{ $version }, được cài đặt tự động.
telegram-update-up-to-date =
    <b>Đã là bản mới nhất</b>

    Đang chạy v{ $version }.
telegram-update-check-failed =
    <b>Kiểm tra cập nhật thất bại</b>

    { $reason }
telegram-update-unreachable = Không kết nối được tới screenerbot.io.
telegram-update-installing = <b>Đang cài đặt v{ $version }</b>
telegram-update-restarting =
    { -brand } đang khởi động lại sang phiên bản mới. Giao dịch sẽ tự tiếp tục.
telegram-update-install-failed =
    <b>Không cài đặt được v{ $version }</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } đã được tải xuống</b>

    Bản phát hành này cũng cập nhật ứng dụng máy tính, nên trình cài đặt phải chạy trên máy. Hãy mở Cài đặt → Cập nhật ở đó.
telegram-update-downloading =
    <b>Đang tải v{ $version }</b>

    { $percent }% của { $size } MB.
telegram-update-available =
    <b>Đã có v{ $version }</b>

    { $how }
    Dung lượng tải: { $size } MB.

    Bản này tự tải xuống; hãy gửi /update lần nữa khi đã sẵn sàng.
telegram-update-how-core = Cài đặt âm thầm kèm một lần khởi động lại ngắn.
telegram-update-how-installer = Cần chạy trình cài đặt máy tính một lần.

## Shared values and units. Numbers arrive formatted; only the unit words live here.

telegram-value-unknown = Không rõ
telegram-value-na = N/A
telegram-percent-value = { $percent }%
telegram-price-native = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds } giây
telegram-duration-minutes = { $minutes } phút
telegram-duration-minutes-seconds = { $minutes } phút { $seconds } giây
telegram-duration-hours = { $hours } giờ
telegram-duration-hours-minutes = { $hours } giờ { $minutes } phút
telegram-duration-days = { $days } ngày
telegram-duration-days-hours = { $days } ngày { $hours } giờ
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-native = { $amount } { -sol }
telegram-error-line = Lỗi: { $detail }
telegram-ai-reasoning =
    <b>Phân tích LLM</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens. Icons come from Rust.

telegram-row-entry = Điểm vào lệnh — { $price } { -sol }
telegram-row-exit = Thoát lệnh — { $price } { -sol }
telegram-row-current = Hiện tại — { $price } { -sol }
telegram-row-invested = Đã đầu tư — { $amount } { -sol }
telegram-row-received = Đã nhận — { $amount } { -sol }
telegram-row-value = Giá trị — { $amount } { -sol }
telegram-row-total = Tổng — { $amount } { -sol }
telegram-row-tokens = Token — { $tokens }
telegram-row-duration = Thời lượng — { $duration }
telegram-row-reason = Lý do — { $reason }
telegram-row-remaining = Còn lại — { $percent }%
telegram-row-pnl = P{ "&amp;" }L — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>Đã mở vị thế</b>
telegram-notify-opened-size = Quy mô — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = Giá — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>Đã đóng vị thế</b> — Có lãi
telegram-notify-closed-title-loss = <b>Đã đóng vị thế</b> — Thua lỗ
telegram-notify-closed-reason-unspecified = Đã đóng
telegram-notify-partial-title = <b>Thoát một phần</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — Đã bán { $percent }%
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = Đã thêm — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = TB — { $price } { -sol }
telegram-notify-severity-critical = <b>Lỗi nghiêm trọng</b>
telegram-notify-severity-error = <b>Lỗi</b>
telegram-notify-severity-warning = <b>Cảnh báo</b>
telegram-notify-severity-info = <b>Thông tin</b>
telegram-notify-alert-title = <b>Cảnh báo giao dịch</b>
telegram-notify-alert-token = Token: <code>${ $symbol }</code>
telegram-notify-alert-mint = Mint: <code>{ $mint }</code>
telegram-notify-alert-bought = Hành động: đã mua { $amount } { -sol }
telegram-notify-alert-sold = Hành động: đã bán { $amount } { -sol }
telegram-notify-alert-wallet = Ví: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (giao dịch thử)
telegram-notify-copy-task = Tác vụ: { $task }
telegram-notify-scheduled-completed = <b>Tác vụ theo lịch đã hoàn tất</b>
telegram-notify-scheduled-failed = <b>Tác vụ theo lịch thất bại</b>
telegram-notify-scheduled-timed-out = <b>Tác vụ theo lịch đã hết thời gian</b>
telegram-notify-scheduled-error = Lỗi: { $error }
telegram-notify-summary-title = <b>Tổng kết hằng ngày</b> — { $date }
telegram-notify-summary-performance = <b>Hiệu suất</b>
telegram-notify-summary-trades = Giao dịch — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = Tỷ lệ thắng — { $percent }%
telegram-notify-summary-pnl = P{ "&amp;" }L — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = Vị thế đang mở — { $count }
telegram-notify-started-title = <b>{ -brand } đã khởi động</b>
telegram-notify-started-version = <b>Phiên bản</b> — { $version }
telegram-notify-started-mode = <b>Chế độ</b> — { $mode }
telegram-notify-started-ready = Sẵn sàng giao dịch!
telegram-notify-stopped-title = <b>{ -brand } đã dừng</b>
telegram-notify-stopped-reason = <b>Lý do</b> — { $reason }
telegram-notify-stopped-goodbye = Tạm biệt! { $icon }
telegram-notify-start-mode-normal = Bình thường
telegram-notify-stop-reason-graceful = Tắt máy êm
telegram-notify-update-available =
    <b>Đã có bản cập nhật v{ $version }</b>

    { $how }
    Dung lượng tải: { $size } MB
telegram-notify-update-how-installer = Bản phát hành này cũng cập nhật ứng dụng máy tính, nên trình cài đặt phải chạy một lần.
telegram-notify-update-ready =
    <b>Bản cập nhật v{ $version } đã sẵn sàng</b>

    { $how }
telegram-notify-update-ready-silent = Gửi /update để áp dụng ngay, hoặc bản này sẽ được cài vào lần { -brand } khởi động tiếp theo.
telegram-notify-update-ready-installer = Mở Cài đặt → Cập nhật để chạy trình cài đặt.
telegram-notify-update-applying =
    <b>Đang cài đặt v{ $version }</b>

    Phần backend đang khởi động lại; giao dịch sẽ tự tiếp tục.
telegram-notify-new-tokens =
    <b>Cảnh báo bộ lọc</b>

    { $count ->
       *[other] Tìm thấy { $count } token mới phù hợp với tiêu chí của bạn.
    }
telegram-notify-crash =
    <b>Bot đã gặp sự cố!</b>

    <b>Vị trí:</b> <code>{ $location }</code>
    <b>Lỗi:</b> <code>{ $error }</code>
telegram-notify-crash-restart = Vui lòng khởi động lại bot.

## Filter results page.

telegram-filter-results-title = <b>Kết quả lọc</b> ({ $count })
telegram-filter-results-empty = <i>Không tìm thấy token nào.</i>
telegram-filter-results-page = <i>Trang { $page } / { $total }</i>

## Position screens.

telegram-position-not-found = Không tìm thấy vị thế
telegram-position-no-positions = Không có vị thế để đóng
telegram-position-history-empty =
    <b>Lịch sử giao dịch</b>

    Chưa có vị thế nào đã đóng.
telegram-position-history-title = <b>Giao dịch gần đây</b>
telegram-position-history-more = <i>+{ $count } giao dịch khác...</i>
telegram-position-confirm-hint = <i>Xác nhận trong vòng 30 giây để thực hiện.</i>
telegram-position-confirm-close-title = <b>Đóng vị thế?</b>
telegram-position-confirm-close-selling = Bán { $tokens } token
telegram-position-confirm-close-estimated = Ước tính — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>Xác nhận trong vòng 30 giây</i>
telegram-position-confirm-sell =
    <b>Xác nhận bán</b>

    Token — { $symbol }
    Số lượng — { $percent }%
    Token — { $tokens }
telegram-position-confirm-dca =
    <b>Xác nhận mua thêm</b>

    Token — { $symbol }
    Thêm — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>Đóng tất cả vị thế?</b>

    Số lượng — { $count }
telegram-position-confirm-close-all-hint =
    <i>Thao tác này sẽ bán theo giá thị trường tất cả vị thế đang mở.
    Xác nhận trong vòng 30 giây.</i>
telegram-position-confirm-force-stop =
    <b>DỪNG CƯỠNG CHẾ</b>

    Thao tác này sẽ dừng ngay TOÀN BỘ giao dịch:
    • Không vào lệnh mới
    • Không thoát lệnh
    • Không DCA
telegram-position-confirm-force-stop-warning = <b>Đây là hành động khẩn cấp.</b>
telegram-position-confirm-blacklist =
    <b>Đưa token vào danh sách đen?</b>

    Token — { $symbol }
    Mint — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>Thao tác này sẽ đóng vị thế và chặn các lần vào lệnh sau này.</i>
telegram-position-selling = Đang bán { $percent }% { $symbol }...
telegram-position-sell-done =
    <b>Đã thực hiện bán</b>

    Token — { $symbol }
    Đã bán — { $percent }%
    Đã nhận — { $amount } { -sol }
telegram-position-sell-failed = <b>Bán thất bại</b>
telegram-position-adding = Đang thêm { $amount } { -sol } vào { $symbol }...
telegram-position-dca-done =
    <b>Đã thực hiện DCA</b>

    Token — { $symbol }
    Đã thêm — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA thất bại</b>
telegram-position-closing-all = Đang đóng tất cả vị thế...
telegram-position-close-all-done =
    <b>Đã đóng tất cả xong</b>

    Đã đóng — { $closed }
    Thất bại — { $failed }
telegram-position-blacklisted =
    <b>Token đã vào danh sách đen</b>

    Token — { $symbol }
    Trạng thái — Đã đóng { "&amp;" } đã vào danh sách đen

## Token screens.

telegram-token-not-found = Không tìm thấy token
telegram-token-not-found-prefix = Không tìm thấy token. Hãy thử tìm bằng tiền tố dài hơn.
telegram-token-stats-failed = Không lấy được thống kê: { $detail }
telegram-token-list-failed = Không lấy được danh sách token: { $detail }
telegram-token-list-empty = Không có token nào trong chế độ xem <b>{ $view }</b>.
telegram-token-view-passed = Đạt bộ lọc
telegram-token-view-rejected = Bị loại
telegram-token-view-recent = Mới thêm gần đây
telegram-token-view-all = Tất cả token
telegram-token-list-title = <b>{ $name }</b> (Trang { $page }/{ $total })
telegram-token-list-stats = Thanh khoản: { $liquidity } • Giá: { $price }
telegram-token-list-hint = <i>Chạm /token_ID để xem chi tiết</i>
telegram-token-explorer =
    <b>Trình khám phá thị trường</b>

    <b>Tổng quan</b>
    Đạt bộ lọc — { $passed }
    Bị loại — { $rejected }
    Có giá đang hoạt động — { $priced }
    Tổng số đã phát hiện — { $total }

    <i>Chọn một danh mục để duyệt:</i>
telegram-token-filter-title = <b>Phân tích bộ lọc</b>
telegram-token-filter-distribution = <b>Phân bố</b>
telegram-token-filter-passed = Đạt — { $count } ({ $percent }%)
telegram-token-filter-rejected = Bị loại — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = Trong danh sách đen — { $count }
telegram-token-filter-coverage = <b>Phạm vi</b>
telegram-token-filter-priced = Có giá pool — { $count }
telegram-token-filter-open = Vị thế đang mở — { $count }
telegram-token-filter-total = Tổng số đã phát hiện — { $count }
telegram-token-filter-updated = <b>Cập nhật lần cuối</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>Tự động làm mới mỗi { $interval }</i>
telegram-token-detail-active = <b>Vị thế đang hoạt động</b>
telegram-token-detail-price = Giá — { $price } { -sol }
telegram-token-detail-liquidity = Thanh khoản — { $value }
telegram-token-detail-volume = Khối lượng 24h — { $value }
telegram-token-detail-change = Biến động 24h — { $value }
telegram-token-detail-risk = Đánh giá rủi ro: { $score }/100
telegram-token-detail-risk-unknown = Đánh giá rủi ro: Không rõ
telegram-token-detail-action = <i>Chọn hành động:</i>
telegram-token-search =
    <b>Tìm kiếm thị trường</b>

    Nhập ký hiệu hoặc địa chỉ mint để tìm:

    <i>Ví dụ: /token_BONK hoặc /token_So11111</i>
telegram-token-confirm-buy =
    <b>Xác nhận mua trực tiếp</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>
    Số lượng — { $amount } { -sol }

    <i>Xác nhận trong vòng 30 giây để thực hiện.</i>
telegram-token-confirm-blacklist =
    <b>Đưa token vào danh sách đen?</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>

    <i>Thao tác này sẽ khiến token không thể đạt các bộ lọc.</i>
telegram-token-blacklisted =
    <b>Token đã vào danh sách đen</b>

    Token — ${ $symbol }
    Trạng thái — Đã thêm vào danh sách đen
telegram-token-blacklist-failed = <b>Đưa vào danh sách đen thất bại</b>
telegram-token-buy-processing =
    <b>Đang xử lý lệnh mua...</b>

    Token — ${ $symbol }
    Số lượng — { $amount } { -sol }
telegram-token-buy-done =
    <b>Mua thành công</b>

    Token — ${ $symbol }
    Số lượng — { $amount } { -sol }

    <i>Xem chi tiết trong /positions</i>
telegram-token-buy-failed =
    <b>Mua thất bại</b>

    Token — ${ $symbol }
    Lỗi — { $detail }
