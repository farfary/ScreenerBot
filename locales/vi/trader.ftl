trader-exit-type-stop-loss = Stop loss
trader-exit-type-take-profit = Take profit
trader-exit-type-roi = Mục tiêu ROI
trader-exit-type-roi-exit = Mục tiêu ROI
trader-exit-type-trailing-stop = Trailing stop
trader-exit-type-time-override = Ghi đè theo thời gian
trader-exit-type-time-rule = Quy tắc thời gian
trader-exit-type-manual = Thủ công
trader-exit-type-manual-close = Thủ công
trader-exit-type-dca = DCA
trader-exit-type-unknown = Không rõ

trader-tab-stats = Thống kê
trader-tab-strategy-control = Điều khiển chiến lược
trader-tab-strategies = Chiến lược
trader-tab-stop-loss = Stop loss
trader-tab-trailing-stop = Trailing stop
trader-tab-roi = Take profit
trader-tab-time-rules = Quy tắc thời gian
trader-tab-dca = DCA
trader-tab-settings = Cài đặt

trader-feature-coming-soon = Sắp ra mắt
    .message = Tính năng này sắp ra mắt và chưa khả dụng.
trader-feature-beta = Beta
trader-feature-disabled = Đã tắt
    .message = Tính năng này hiện đang tắt.

trader-status-title = Auto Trader
trader-status-loading = Đang tải...
trader-status-running = Đang chạy
trader-status-stopped = Đã dừng
trader-status-setup-required = Cần thiết lập
trader-status-unavailable = Hoàn tất thiết lập ví và RPC để dùng Auto Trader
trader-toggle-on = BẬT
trader-toggle-off = TẮT
trader-toggle-unavailable = KHÔNG KHẢ DỤNG
trader-toggle-start-failed = Không thể khởi động Trader
trader-toggle-stop-failed = Không thể dừng Trader
trader-controls-title = Điều khiển giao dịch
trader-halt-title = GIAO DỊCH ĐÃ BỊ DỪNG
trader-halt-reason-default = Dừng cưỡng chế thủ công
trader-halt-resume = Tiếp tục
trader-monitor-entry = Giám sát vào lệnh
trader-monitor-exit = Giám sát thoát lệnh
trader-monitor-master-off = Auto Trader đang tắt
trader-loss-limit-title = Giới hạn lỗ theo kỳ
trader-loss-limit-resume = Tiếp tục giao dịch
trader-loss-limit-reset = Đặt lại kỳ
trader-loss-limit-off = Tắt
trader-loss-limit-none = Chưa cấu hình giới hạn lỗ theo kỳ
trader-loss-limit-resets-in = Đặt lại sau { $hours } { $minutes }
trader-loss-limit-reached = ĐÃ ĐẠT GIỚI HẠN
trader-force-stop = Dừng cưỡng chế mọi thứ

trader-force-stop-confirm = Dừng cưỡng chế giao dịch
    .message = Thao tác này sẽ dừng ngay TẤT CẢ hoạt động giao dịch. Tiếp tục?
    .confirm = Dừng giao dịch
trader-loss-limit-resume-confirm = Tiếp tục sau giới hạn lỗ
    .message = Giới hạn lỗ theo kỳ đã chặn các lệnh vào mới. Tiếp tục sẽ cho phép Trader mở vị thế trở lại trước khi kỳ được đặt lại. Tiếp tục?
trader-loss-limit-reset-confirm = Đặt lại kỳ giới hạn lỗ
    .message = Thao tác này xóa khoản lỗ tích lũy của kỳ hiện tại và bắt đầu một kỳ mới. Tiếp tục?

trader-toast-control-failed = Điều khiển Auto Trader thất bại
trader-toast-force-stop-on = Đã kích hoạt dừng cưỡng chế
trader-toast-force-stop-failed = Không thể kích hoạt dừng cưỡng chế
trader-toast-force-stop-cleared = Đã gỡ dừng cưỡng chế
trader-toast-resume-failed = Không thể tiếp tục giao dịch
trader-toast-loss-limit-reset-failed = Không thể đặt lại giới hạn lỗ
trader-toast-entry-monitor-failed = Không thể bật/tắt giám sát vào lệnh
trader-toast-exit-monitor-failed = Không thể bật/tắt giám sát thoát lệnh
trader-toast-load-failed = Tải thất bại
    .message = Không thể tải cấu hình Trader
trader-toast-saved = Đã lưu cấu hình
    .message = Đã áp dụng cài đặt Trader
trader-toast-save-failed = Lưu thất bại
    .message = Không thể lưu cấu hình Trader
trader-toast-feature-enabled = Đã bật tính năng
trader-toast-feature-disabled = Đã tắt tính năng
trader-toast-feature-applied = Đã áp dụng cài đặt Auto Trader
trader-toast-strategy-enabled = Đã bật chiến lược
    .message = Chiến lược đang hoạt động
trader-toast-strategy-disabled = Đã tắt chiến lược
    .message = Chiến lược không hoạt động
trader-toast-strategy-failed = Cập nhật thất bại
    .message = Không thể cập nhật trạng thái chiến lược

trader-stats-window =
    .aria-label = Khoảng thống kê
trader-stats-window-day = 24H
trader-stats-window-week = 7D
trader-stats-window-month = 30D
trader-realized-title = Hiệu suất đã chốt
trader-metric-net-pnl = P&L ròng
trader-metric-win-rate = Tỷ lệ thắng
trader-metric-profit-factor = Hệ số lợi nhuận
trader-metric-max-drawdown = Mức sụt giảm tối đa
trader-metric-capital = Vốn đang dùng
trader-metric-avg-win-loss = Thắng / thua TB
trader-metric-closed-trades = Lệnh đã đóng
trader-metric-median-hold = Thời gian nắm giữ trung vị
trader-stats-empty = Không có lệnh đã đóng trong khoảng này
trader-stats-won-lost = Thắng { $won } · thua { $lost }
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
       *[other] { $amount } thắng
    }
trader-stats-losses =
    { $count ->
       *[other] { $amount } thua
    }
trader-stats-expected = Kỳ vọng { $amount } mỗi lệnh
trader-stats-profit-factor-basis = Tổng thắng ÷ tổng thua
trader-stats-drawdown-basis = Mức sụt giảm đã chốt sâu nhất từ đỉnh đến đáy
trader-stats-slots =
    { $count ->
       *[other] Đã dùng { $used }/{ $max } slot vị thế
    }
trader-stats-avg-basis = Kết quả trung bình của một lệnh thắng so với lệnh thua
trader-stats-closed =
    { $count ->
       *[other] { $amount } vị thế đã đóng
    }
trader-stats-hold-average = TB { $span }
trader-stats-excluded =
    { $count ->
       *[other] Đã loại { $amount } vòng đã đóng - không có giá vốn đầy đủ nên không thể tính P&L chính xác.
    }

trader-daily-title = P&L hằng ngày
trader-daily-subtitle = { -sol } đã chốt mỗi ngày, kèm tổng lũy kế
trader-daily-loading = Đang tải P&L hằng ngày...
trader-daily-chart = Lãi/lỗ đã chốt hằng ngày tính bằng { -sol }
trader-extreme-best = Lệnh tốt nhất
trader-extreme-worst = Lệnh tệ nhất

trader-exit-title = Phân tích chiến lược thoát lệnh
trader-exit-subtitle = Cách các vị thế được đóng và mỗi lần thoát thu về bao nhiêu
trader-exit-loading = Đang tải dữ liệu thoát lệnh...
trader-exit-empty-day = Không có lệnh đã đóng trong 24 giờ qua
trader-exit-empty-days =
    { $count ->
       *[other] Không có lệnh đã đóng trong { $amount } ngày qua
    }
trader-exit-share =
    { $count ->
       *[other] { $amount } lệnh · { $share } số lần thoát
    }
trader-exit-average = TB { $value }

trader-impact-label = Tác động:
trader-current-label = Hiện tại:
trader-readable-label = Dễ đọc:
trader-example-how-it-works = Cách hoạt động
trader-step-entry = Vào lệnh
trader-step-initial-position = Vị thế ban đầu
trader-step-auto-exit = Thoát tự động
trader-step-exit = Thoát lệnh
trader-step-full-exit = Thoát toàn bộ vị thế
trader-value-percent = { $value }%
trader-example-profit = Lãi +{ $value }%

trader-stop-loss-title = Stop loss
trader-stop-loss-subtitle = Tự động thoát vị thế khi mức lỗ vượt ngưỡng của bạn
trader-stop-loss-threshold-badge = Giới hạn lỗ
trader-stop-loss-hold-badge = Độ trễ tùy chọn
trader-stop-loss-impact = Thoát khi giảm { $threshold }% so với giá vào
trader-stop-loss-hold-immediate = Ngay lập tức
trader-stop-loss-hold-delay = Trễ { $span }
trader-stop-loss-price-falls = Giá giảm
trader-stop-loss-threshold-reached = Đạt ngưỡng
trader-stop-loss-partial = Cho phép thoát một phần
trader-stop-loss-summary = Lỗ được giới hạn ở <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Lưu ý:</strong> Stop loss bảo vệ bạn khỏi các khoản lỗ lớn hơn bằng cách thoát sớm

trader-trailing-title = Trailing stop
trader-trailing-subtitle = Tự động bảo vệ lợi nhuận bằng cách bám theo giá khi giá tăng
trader-trailing-activation-badge = Thời điểm bắt đầu
trader-trailing-distance-badge = Biên an toàn
trader-trailing-activation-impact = Bắt đầu bám theo khi lãi +{ $value }%
trader-trailing-distance-impact = Thoát khi giảm -{ $value }% so với đỉnh
trader-trailing-activation = Kích hoạt
trader-trailing-peak = Đỉnh
trader-trailing-final = Cuối cùng +{ $value }%
trader-trailing-summary-protected = Đã bảo vệ <strong>{ $value }</strong> lợi nhuận
trader-trailing-summary-avoided = Đã tránh <strong>{ $value }</strong> lỗ tính từ đỉnh

trader-roi-title = Take profit
trader-roi-subtitle = Tự động thoát toàn bộ vị thế khi lợi nhuận đạt mục tiêu của bạn
trader-roi-target-badge = Một mục tiêu
trader-roi-impact = Thoát khi lãi +{ $target }%
trader-roi-example-title = Kịch bản ví dụ
trader-roi-initial-buy = Lần mua đầu
trader-roi-target-hit = Đạt mục tiêu
trader-roi-full-position = Toàn bộ vị thế
trader-roi-sold = Đã bán 100%
trader-roi-summary = Đã chốt lãi <strong>+{ $target }%</strong>

trader-time-title = Thoát lệnh theo thời gian
trader-time-subtitle = Tự động thoát vị thế sau thời gian nắm giữ tối đa nếu mức lỗ vượt ngưỡng
trader-time-hold-badge = Kích hoạt theo thời gian
trader-time-loss-badge = Cổng lỗ
trader-time-unit-seconds = giây
trader-time-unit-minutes = phút
trader-time-unit-hours = giờ
trader-time-unit-days = ngày
trader-time-conversion-default = 168 giờ = 7 ngày
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
       *[other] { $amount } giây
    }
trader-duration-minutes =
    { $count ->
       *[other] { $amount } phút
    }
trader-duration-hours =
    { $count ->
       *[other] { $amount } giờ
    }
trader-duration-days =
    { $count ->
       *[other] { $amount } ngày
    }
trader-time-loss-impact = Thoát nếu giảm { $value }% trở lên sau thời gian nắm giữ
trader-time-day = Ngày { $day }
trader-time-position-opened = Đã mở vị thế
trader-time-limit = Hạn thời gian
trader-time-hold-reached = Đã hết thời gian nắm giữ
trader-time-loss-met = Đạt ngưỡng lỗ
trader-time-note = <strong>Lưu ý:</strong> Các vị thế đang có lãi hoặc lỗ nhỏ hơn sẽ KHÔNG bị thoát
trader-time-positions-title = Trạng thái vị thế hiện tại
trader-time-positions-loading = Đang tải vị thế...
trader-time-positions-empty = Không có vị thế đang mở
trader-time-positions-hold = Thời gian nắm giữ:
trader-time-positions-roi = ROI:

trader-strategy-entry-title = Chiến lược vào lệnh
trader-strategy-entry-subtitle = Các tín hiệu có thể mở vị thế mới.
trader-strategy-exit-title = Chiến lược thoát lệnh
trader-strategy-exit-subtitle = Các tín hiệu có thể đóng hoặc bảo vệ vị thế đang mở.
trader-strategy-active-unknown = -- đang hoạt động
trader-strategy-active = { $enabled }/{ $total } đang hoạt động
trader-strategy-loading = Đang tải chiến lược...
trader-strategy-load-failed = Không thể tải chiến lược
trader-strategy-empty = Chưa có chiến lược nào
trader-strategy-no-description = Chưa có mô tả.
trader-strategy-unnamed = Chiến lược chưa đặt tên
trader-strategy-type-unknown = Chiến lược
trader-strategy-priority-auto = Tự động
trader-strategy-priority = Ưu tiên { $priority }

trader-dca-title = DCA (trung bình giá)
trader-dca-subtitle = Tự động thêm vào các vị thế đang lỗ để hạ giá vào trung bình
trader-dca-threshold-badge = Điều kiện vào lệnh
trader-dca-example-title = Ví dụ DCA
trader-dca-example = 0.01 { -sol } ban đầu → DCA #1: 0.005 { -sol } @ -10% → DCA #2: 0.005 { -sol } @ thêm -10%
trader-dca-info-title = Thông tin chiến lược DCA
trader-dca-info-subtitle = Những điều cần lưu ý khi giao dịch DCA
trader-dca-how-title = DCA hoạt động thế nào
trader-dca-how-trigger = <strong>Điều kiện kích hoạt:</strong> Vị thế giảm dưới ngưỡng DCA (ví dụ: -10%)
trader-dca-how-action = <strong>Hành động:</strong> Thêm { -sol } để giảm giá vốn trung bình
trader-dca-how-repeat = <strong>Lặp lại:</strong> Có thể DCA nhiều lần theo số lần tối đa
trader-dca-risk-title = Cảnh báo rủi ro
trader-dca-risk-exposure = <strong>Rủi ro tăng:</strong> DCA làm tăng tổng vốn chịu rủi ro của mỗi vị thế
trader-dca-risk-knife = <strong>Bắt dao rơi:</strong> DCA không giúp được nếu token tiếp tục xu hướng giảm
trader-dca-risk-cooldown = <strong>Thời gian chờ:</strong> Dùng thời gian chờ để tránh các lần DCA dồn dập

trader-sizing-title = Quy mô vị thế
trader-sizing-subtitle = Kiểm soát số tiền đầu tư cho mỗi vị thế
trader-sizing-positions-badge = Kiểm soát rủi ro
trader-sizing-trade-size-badge = Mỗi vị thế
trader-timing-title = Thời gian & thời gian chờ
trader-timing-subtitle = Kiểm soát khoảng thời gian giữa các thao tác
trader-timing-close-cooldown = Thời gian chờ sau khi đóng vị thế
trader-timing-close-cooldown-hint = Số phút chờ trước khi mở lại cùng một token
trader-timing-concurrency = Số luồng kiểm tra vào lệnh
trader-timing-concurrency-hint = Số token kiểm tra cùng lúc (càng cao càng nhanh nhưng tốn CPU hơn)
trader-timing-unit-minutes = phút
trader-timing-unit-tokens = token
trader-timing-intervals = Chu kỳ giám sát
trader-timing-intervals-badge = Chỉ đọc
trader-timing-intervals-hint = Được cấu hình trong mã (không chỉnh được qua giao diện)
trader-timing-intervals-value = <strong>Giám sát vào lệnh:</strong> 30 giây | <strong>Giám sát thoát lệnh:</strong> 5 giây
