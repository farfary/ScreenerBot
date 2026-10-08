shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-version = v{ $version }

shell-header-brand =
    .aria-label = Mở trang chủ bảng điều khiển
    .title = Trang chủ bảng điều khiển
shell-bot-card =
    .aria-label = Đang tải trạng thái Auto Trader
shell-bot-label = Tự động
shell-bot-status-loading = ĐANG TẢI
shell-bot-today = Hôm nay
shell-explore-control =
    .aria-label = Chế độ khám phá. Kết nối ví và endpoint RPC để bật mọi tính năng
    .title = Kết nối ví và endpoint RPC để bật giao dịch, số dư và dữ liệu on-chain trực tiếp
shell-explore-title = Chế độ khám phá
shell-explore-detail = Chưa kết nối ví & RPC
shell-explore-action = Hoàn tất thiết lập
shell-setup-gate-detail = Chế độ khám phá chạy không có ví hay RPC. Hoàn tất thiết lập để kết nối chúng.
shell-wallet-card =
    .aria-label = Giá trị ví; mở Vị thế
    .title = Giá trị ví ({ -sol } + token) · mở Vị thế
shell-wallet-worth-label = GIÁ TRỊ
shell-wallet-native-label = { -sol }
shell-wallet-tokens-label = TKN
shell-sol-price-card =
    .aria-label = Giá { -sol } theo USD - mở biểu đồ
    .title = Giá { -sol } · nhấp để xem biểu đồ
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24h
shell-copy-card =
    .aria-label = Copy trading; mở Copy trading
    .title = Copy trading · mở Copy trading
shell-copy-label = COPY TRADING
shell-actions-more =
    .aria-label = Thêm thao tác trên thanh đầu trang
    .title = Thêm thao tác
shell-actions-group =
    .aria-label = Thao tác trên thanh đầu trang
shell-action-search =
    .aria-label = Tìm token
    .title = Tìm token (Ctrl/Cmd+K)
shell-action-featured =
    .aria-label = Token nổi bật
    .title = Token nổi bật
shell-action-notifications =
    .aria-label = Hành động và thông báo
    .title = Hành động và thông báo
shell-action-restart =
    .aria-label = Khởi động lại ứng dụng
    .title = Khởi động lại ứng dụng
shell-action-theme =
    .aria-label = Đổi giao diện
    .title = Đổi giao diện
shell-action-settings =
    .aria-label = Cài đặt
    .title = Cài đặt
shell-tabs-scroll-start =
    .aria-label = Hiện các tab trước
    .title = Hiện các tab trước
shell-tabs-scroll-end =
    .aria-label = Hiện thêm tab
    .title = Hiện thêm tab

shell-ticker-monitoring-segment =
    .title = Token đang được Pool Service theo dõi
shell-ticker-monitoring = Đang theo dõi:
shell-ticker-filtering-segment =
    .title = Token đạt/không đạt tiêu chí lọc
shell-ticker-passed = Đạt:
shell-ticker-rejected = Bị loại:
shell-ticker-pnl-segment =
    .title = Lãi/lỗ đã chốt hôm nay
shell-ticker-pnl = P&L hôm nay:
shell-ticker-rpc-segment =
    .title = Số lệnh gọi RPC mỗi phút và tỷ lệ thành công
shell-ticker-rpc = RPC:
shell-ticker-rpc-rate = { $amount }/phút
shell-ticker-services-segment =
    .title = Tình trạng hoạt động của các dịch vụ nền
shell-ticker-services-loading = Dịch vụ: <strong>Đang tải</strong>

shell-notification-title = Hành động
shell-notification-mark-all-read =
    .title = Đánh dấu tất cả đã đọc
shell-notification-clear-all =
    .title = Xóa tất cả
shell-notification-close =
    .aria-label = Đóng
shell-notification-tab-all = Tất cả
shell-notification-tab-active = Đang chạy
shell-notification-tab-done = Hoàn tất
shell-notification-tab-failed = Thất bại
shell-notification-filter-type-all = Mọi loại
shell-notification-filter-type-buy = Mua
shell-notification-filter-type-sell = Bán
shell-notification-filter-type-open = Mở
shell-notification-filter-type-close = Đóng
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = Một phần
shell-notification-filter-state-all = Mọi trạng thái
shell-notification-filter-state-in-progress = Đang xử lý
shell-notification-filter-state-completed = Đã hoàn tất
shell-notification-filter-state-failed = Thất bại
shell-notification-filter-state-cancelled = Đã hủy
shell-notification-list =
    .aria-label = Thông báo
shell-notification-empty = Chưa có hành động nào
shell-notification-loading-more = Đang tải thêm...
shell-notification-back-to-top =
    .title = Về đầu trang

shell-status-bar-version = v
shell-status-bar-uptime = Hoạt động
shell-status-bar-memory = Bộ nhớ
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/phút
shell-status-bar-trading = Giao dịch
shell-status-bar-positions = Vị thế
shell-status-bar-tokens = Token

shell-splash-starting = Đang khởi động { -brand }
shell-splash-waiting = Đang chờ core cục bộ phản hồi.
shell-splash-failed = { -brand } không thể khởi động
shell-splash-failed-detail = Hãy kiểm tra tệp nhật ký rồi khởi động lại ứng dụng.

shell-connection-connected = Đã kết nối Core
shell-connection-waiting = Đang chờ core…
shell-connection-retry-now = Thử lại ngay
shell-connection-overlay-detail = Không thể kết nối tới core. Giao dịch đang tạm dừng; hệ thống sẽ tự khôi phục.
shell-connection-restored = Đã khôi phục kết nối với core

shell-trader-control-failed = Điều khiển Trader thất bại
shell-notification-button-unread = Hành động và thông báo, { $count } chưa đọc
shell-restart-confirm-title = Khởi động lại bot
shell-restart-confirm-message =
    Bạn có chắc muốn khởi động lại bot?

    Thao tác này sẽ:
    • Dừng mọi dịch vụ
    • Khởi động lại tiến trình
    • Mất khoảng 10-15 giây

    Mọi hoạt động đang chạy sẽ bị gián đoạn.
shell-restart-confirm-action = Khởi động lại
shell-restart-progress = Đang khởi động lại bot
shell-restart-failed = Khởi động lại thất bại
shell-restart-failed-status = Khởi động lại thất bại: { $status }
shell-restart-helper-unavailable = Trình hỗ trợ khởi động lại tự động không khả dụng. Hãy tải lại bảng điều khiển sau ít phút.

shell-page-title-fallback = Bảng điều khiển
shell-page-load-failed = Không thể tải trang
shell-page-offline-detail = Hiện không thể kết nối tới core. Trang sẽ tự tải khi kết nối được khôi phục.

shell-bot-state-explore = KHÁM PHÁ
shell-bot-state-halted = ĐÃ DỪNG
shell-bot-state-off = TẮT
shell-bot-state-waiting = ĐANG CHỜ
shell-bot-state-idle = NHÀN RỖI
shell-bot-state-entry-paused = TẠM DỪNG VÀO LỆNH
shell-bot-state-running = ĐANG CHẠY
shell-bot-control-explore = Auto Trader không khả dụng trong Chế độ khám phá. Mở phần thiết lập ví và RPC.
shell-bot-control-halted = Dừng khẩn cấp đang bật. Mở điều khiển Auto Trader.
shell-bot-control-off = Auto Trader đang tắt. Nhấp để bật.
shell-bot-control-waiting = Auto Trader đã bật và đang chờ các dịch vụ core. Nhấp để tắt.
shell-bot-control-idle = Auto Trader đã bật nhưng cả hai bộ giám sát đều tắt. Mở điều khiển Auto Trader.
shell-bot-control-entry-paused = Bảo vệ lỗ đã tạm dừng lệnh vào; lệnh thoát vẫn có thể tiếp tục. Mở điều khiển Auto Trader.
shell-bot-control-running = Auto Trader đang chạy. Nhấp để tắt.

shell-wallet-card-summary = Giá trị ví: { $equity } { -sol } ({ $balance } { -sol } tiền mặt, { $tokens } token); mở Vị thế
shell-copy-running-live = { $count } chạy thật
shell-copy-running-paper = { $count } giao dịch thử
shell-copy-value-paused = Tạm dừng
shell-copy-value-idle = Nhàn rỗi
shell-copy-sub-active = { $active }/{ $total } đang hoạt động

shell-ticker-services-healthy = Dịch vụ: <strong>Ổn định</strong>
shell-ticker-services-issues =
    { $count ->
       *[other] Dịch vụ: <strong>{ $count } sự cố</strong>
    }

shell-agent-request-title = Yêu cầu từ tác nhân
shell-agent-request-client-fallback = Một tác nhân đã ghép nối
shell-agent-request-message = { $client } muốn chạy "{ $tool }" trong { -brand }. Yêu cầu này { $expiry }.
shell-agent-request-message-arguments = { $client } muốn chạy "{ $tool }" trong { -brand }. Đối số: { $summary }. Yêu cầu này { $expiry }.
shell-agent-request-expires-minutes = hết hạn sau { $minutes } phút
shell-agent-request-expires-seconds = hết hạn sau { $seconds } giây
shell-agent-request-approve = Chấp thuận
shell-agent-request-deny = Từ chối

shell-toast-copied = Đã sao chép { $label }
shell-toast-copy-failed = Sao chép thất bại
shell-toast-still-running = Vẫn đang chạy - hãy xem trung tâm thông báo
shell-toast-dismiss =
    .aria-label = Đóng
shell-confirm-title = Xác nhận thao tác
shell-confirm-message = Bạn có chắc không?

shell-assistant-label = Trợ lý
shell-assistant-dialog =
    .aria-label = Trợ lý

shell-status-bar-trading-active = Đang hoạt động
shell-status-bar-trading-inactive = Không hoạt động

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = Đã hủy { $title }
shell-action-swap-buy-live = Đang mua
shell-action-swap-buy-done = Đã mua
shell-action-swap-buy-failed = Mua thất bại
shell-action-swap-sell-live = Đang bán
shell-action-swap-sell-done = Đã bán
shell-action-swap-sell-failed = Bán thất bại
shell-action-position-open-live = Đang mở vị thế
shell-action-position-open-done = Đã mở
shell-action-position-open-failed = Mở thất bại
shell-action-position-close-live = Đang đóng vị thế
shell-action-position-close-done = Đã đóng
shell-action-position-close-failed = Đóng thất bại
shell-action-position-dca-live = Đang thêm vào vị thế
shell-action-position-dca-done = Đã thêm vào
shell-action-position-dca-failed = Thêm thất bại
shell-action-partial-exit-live = Thoát một phần
shell-action-partial-exit-done = Thoát một phần
shell-action-partial-exit-failed = Thoát một phần thất bại
shell-action-manual-order-live = Đang đặt lệnh
shell-action-manual-order-done = Đã đặt lệnh
shell-action-manual-order-failed = Đặt lệnh thất bại
shell-action-trade-live = Giao dịch
shell-action-trade-done = Đã giao dịch
shell-action-trade-failed = Giao dịch thất bại
shell-action-via-router = { $action } qua { $router }
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = tránh { $venue }
shell-action-cost-guard-avoiding-cost = tránh { $venue } · { $cost }
shell-action-cost-guard-avoiding-unnamed = tránh một sàn
shell-action-cost-guard-avoiding-unnamed-cost = tránh một sàn · { $cost }
shell-action-cost-guard-avoided = { $outcome } · đã tránh { $cost } phí thuê tại { $venue }
shell-action-cost-guard-avoided-unnamed = { $outcome } · đã tránh { $cost } phí thuê sàn
shell-action-exit-full = Thoát toàn bộ
shell-action-exit-percent = Thoát { $percent }

shell-exit-title = Đóng { -brand }?
shell-exit-description = Chọn cách bạn muốn đóng ứng dụng
shell-exit-minimize = Thu nhỏ xuống khay
shell-exit-minimize-detail = Tiếp tục chạy nền
shell-exit-quit = Thoát ứng dụng
shell-exit-quit-detail = Đóng hoàn toàn và dừng mọi dịch vụ

shell-lightbox-save =
    .title = Lưu ảnh
shell-lightbox-close =
    .title = Đóng (ESC)

shell-theme-light = Sáng
shell-theme-dark = Tối
shell-theme-switch-to-light = Chuyển sang giao diện sáng
shell-theme-switch-to-dark = Chuyển sang giao diện tối
