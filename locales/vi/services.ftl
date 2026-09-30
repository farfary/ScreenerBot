services-health-component-unavailable = Thành phần { $component } không khả dụng
services-health-unavailable = Không có trạng thái sức khỏe
services-health-pools-not-running = Dịch vụ pool không chạy
services-health-events-db-uninitialized = Cơ sở dữ liệu sự kiện chưa được khởi tạo
services-health-sol-price-not-running = Dịch vụ giá { -sol } không chạy
services-health-sol-price-stale = Dữ liệu giá { -sol } đã cũ ({ $seconds } giây)
services-health-sol-price-no-data = Chưa có dữ liệu giá { -sol }
services-health-telegram-discovery = Chế độ khám phá
services-health-telegram-disconnected = Đã ngắt kết nối
services-health-wallet-watch-polling-only = Việc phát hiện chỉ chạy bằng thăm dò
services-health-assistant-tasks-disabled = Đã tắt trong cấu hình
services-health-connectivity-critical-unhealthy = Các endpoint quan trọng không ổn định: { $endpoints }
services-health-filtering-snapshot-stale = Ảnh chụp nhanh của bộ lọc đã cũ { $seconds } giây

## Services page (pages/services.js)

services-status-healthy = Ổn định
services-status-starting = Đang khởi động
services-status-degraded = Suy giảm
services-status-unhealthy = Không ổn định
services-status-stopping = Đang dừng
services-status-disabled = Đã tắt
services-status-unknown = Không rõ

services-name-account = Tài khoản
services-name-assistant-scheduled-tasks = Tác vụ theo lịch của Trợ lý
services-name-ata-cleanup = Dọn dẹp tài khoản token
services-name-connectivity = Kết nối
services-name-copy-trading = Copy trading
services-name-events = Sự kiện
services-name-filtering = Lọc
services-name-llm-analysis = Phân tích LLM
services-name-ohlcv = OHLCV
services-name-pool-analyzer = Bộ phân tích pool
services-name-pool-calculator = Bộ tính toán pool
services-name-pool-discovery = Khám phá pool
services-name-pool-fetcher = Bộ tải pool
services-name-pools = Pool
services-name-positions = Vị thế
services-name-referral = Giới thiệu
services-name-rpc-stats = Thống kê RPC
services-name-sol-price = Giá { -sol }
services-name-telegram = { -telegram }
services-name-tokens = Token
services-name-trader = Trader
services-name-transactions = Giao dịch
services-name-update-check = Kiểm tra cập nhật
services-name-wallet = Ví
services-name-wallet-watch = Theo dõi ví
services-name-webserver = Máy chủ web

services-loading = Đang tải dịch vụ...
services-load-failed = Không tải được dịch vụ
services-load-failed-description = Đang chờ phần backend phản hồi. Hệ thống sẽ tự thử lại.
services-refresh-failed = Không làm mới được dịch vụ
services-search-placeholder = Tìm dịch vụ...
services-summary-total = Tổng
services-summary-alerts = Cảnh báo
services-summary-alerts-tooltip = { $degraded } suy giảm / { $unhealthy } không ổn định
services-filter-status = Trạng thái
services-filter-all-statuses = Tất cả trạng thái
services-filter-all-services = Tất cả dịch vụ
services-filter-enabled-only = Chỉ dịch vụ đã bật
services-filter-disabled-only = Chỉ dịch vụ đã tắt
services-col-service = Dịch vụ
services-col-health = Sức khỏe
services-col-priority = Ưu tiên
services-col-uptime = Thời gian hoạt động
services-col-activity = Hoạt động
services-col-last-cycle = Chu kỳ gần nhất
services-col-avg-cycle = Chu kỳ TB
services-col-avg-poll = Thăm dò TB
services-col-cycle-rate = Tốc độ chu kỳ
services-col-tasks = Tác vụ
services-col-ops = Thao tác/giây
services-col-errors = Lỗi
services-col-dependencies = Phụ thuộc
services-dependencies-none = Không có
services-activity-busy = { $percent } bận
services-activity-polls =
    { $count ->
       *[other] { $count } lần thăm dò
    }
services-tasks-tooltip =
    { $count ->
       *[other] { $count } tác vụ
    }
    Gần nhất: { $last }
    TB: { $avg }
    Thăm dò: { $poll }
    Nhàn rỗi: { $idle }
    Tổng lần thăm dò: { $polls }
services-tasks-none = Không có tác vụ được theo dõi
