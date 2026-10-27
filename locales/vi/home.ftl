home-portfolio-title = Giá trị danh mục
home-portfolio-change-label = Hôm nay
home-wallet-unread = Không có số dư
home-trend-range = Từ 00:00 UTC
home-portfolio-change-incomparable =
    .title = Một số giá token bị thiếu vào đầu ngày hoặc đang thiếu, nên thay đổi hôm nay không được hiển thị.
home-stat-available = { -sol } khả dụng
home-stat-holdings = Token đang nắm giữ
home-stat-open-pnl = P&L đang mở
home-stat-realized-today = Đã chốt hôm nay

home-holdings-token-count =
    { $count ->
       *[other] { $count } token
    }
home-holdings-with-unpriced = { $tokens } · { $count } chưa có giá
home-holdings-unpriced-note =
    { $count ->
       *[other] { $count } token đang nắm giữ không có giá và được tính là 0 trong tổng
    }

home-wallet-copy =
    .title = Sao chép địa chỉ ví
    .aria-label = Sao chép địa chỉ ví
home-wallet-qr-open =
    .title = Hiện mã QR của ví
    .aria-label = Hiện mã QR của ví
home-wallet-qr-popover =
    .aria-label = Mã QR của ví
home-wallet-qr-receive = Nhận
home-wallet-qr-assets = { -sol } và token SPL
home-wallet-qr-close =
    .title = Đóng
    .aria-label = Đóng mã QR của ví
home-wallet-qr-preparing = Đang tạo mã QR
home-wallet-qr-unavailable = Không có mã QR
home-wallet-qr-image =
    .alt = Mã QR của địa chỉ ví chính

home-calendar-title = Lịch hiệu suất
home-calendar-previous =
    .title = Tháng trước
    .aria-label = Tháng trước
home-calendar-next =
    .title = Tháng sau
    .aria-label = Tháng sau
home-calendar-month-pnl = P&L tháng
home-calendar-trades = Giao dịch
home-calendar-win-days = Ngày lãi
home-calendar-loss-days = Ngày lỗ
home-calendar-best-day = Ngày tốt nhất
home-calendar-week = Tuần
home-calendar-day-label = { $date }, { $pnl }
home-calendar-pop-net-pnl = P&L ròng
home-calendar-pop-win-rate = Tỷ lệ thắng
home-calendar-pop-win-rate-value = { $rate } · { $wins } thắng / { $losses } thua
home-calendar-pop-gross-profit = Tổng lãi
home-calendar-pop-gross-loss = Tổng lỗ
home-calendar-pop-end-balance = Số dư cuối kỳ

## Auto Trader and Copy Trading status

home-status =
    .aria-label = Trạng thái tự động hóa
home-status-trader-title = Auto Trader
home-status-copy-title = Copy trading
home-status-running = Đang chạy
home-status-stopped = Đã dừng
home-status-setup-required = Cần thiết lập
home-status-open-positions = Vị thế đang mở
home-status-capacity = { $used } / { $limit }
home-status-entries-today = Lệnh vào hôm nay
home-status-exits-today = Lệnh thoát hôm nay
home-status-followed-wallets = Ví đang theo dõi
home-status-active-tasks = Tác vụ đang bật
home-status-last-copy = Giao dịch sao chép gần nhất
home-status-trader-unavailable = Hoàn tất thiết lập ví và RPC để dùng Auto Trader
home-status-copy-unavailable = Hoàn tất thiết lập ví và RPC để dùng Copy trading
home-status-setup = Hoàn tất thiết lập
home-status-trader-open = Mở Auto Trader
home-status-copy-open = Mở Copy trading

home-operations =
    .aria-label = Trạng thái danh mục và thị trường
home-exposure-title = Mức độ phơi nhiễm vị thế
home-exposure-open = đang mở
home-exposure-invested = Đã đầu tư
home-exposure-value = Giá trị hiện tại
home-exposure-position = Vị thế
home-exposure-held = Thời gian nắm giữ
home-exposure-empty = Không có vị thế đang mở
home-exposure-avg-size = Quy mô trung bình
home-exposure-avg-hold = Thời gian nắm giữ trung bình
home-exposure-best = Tốt nhất
home-exposure-worst = Kém nhất
home-pipeline-title = Quy trình thị trường
home-pipeline-tracked = Đã ghi nhận
home-pipeline-monitored = Đang theo dõi
home-pipeline-passed = Đạt bộ lọc
home-pipeline-rejected = Bị loại
home-pipeline-blacklisted = Trong danh sách đen
home-pipeline-live-price = Giá trực tiếp
home-pipeline-ohlcv = OHLCV
