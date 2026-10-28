trader-tab-stats = Thống kê
trader-tab-strategies = Chiến lược
trader-tab-stop-loss = Stop loss
trader-tab-trailing-stop = Trailing stop
trader-tab-take-profit = Take profit
trader-tab-time-rules = Thoát theo thời gian
trader-tab-dca = DCA
trader-tab-settings = Cài đặt

trader-feature-coming-soon = Sắp ra mắt
    .message = Tính năng này sắp ra mắt và chưa khả dụng.
trader-feature-beta = Beta
trader-feature-disabled = Đã tắt
    .message = Tính năng này hiện đang tắt.

trader-status-title = Auto Trader
trader-status-running = Đang chạy
trader-status-stopped = Đã dừng
trader-status-setup-required = Cần thiết lập
trader-setup-action = Hoàn tất thiết lập
trader-toggle-start-failed = Không thể khởi động Trader
trader-toggle-stop-failed = Không thể dừng Trader
trader-halt-title = Giao dịch đã bị dừng
trader-halt-reason-default = Dừng cưỡng chế thủ công
trader-halt-resume = Tiếp tục
trader-monitor-entry = Giám sát vào lệnh
trader-monitor-exit = Giám sát thoát lệnh
trader-loss-limit-title = Giới hạn lỗ theo kỳ
trader-loss-limit-resume = Tiếp tục giao dịch
trader-loss-limit-reset = Đặt lại kỳ
trader-loss-limit-off = Tắt
trader-loss-limit-resets-in = Đặt lại sau { $hours } { $minutes }
trader-force-stop = Dừng cưỡng chế mọi thứ
trader-force-stop-hint = Dừng ngay mọi trình theo dõi, tác vụ sao chép và giao dịch.
trader-force-stop-idle = Không có gì đang chạy để lệnh dừng cưỡng chế phải dừng.
trader-run-state-force-stopped = Dừng cưỡng bức
trader-run-state-waiting = Đang chờ dịch vụ
trader-run-state-idle = Giám sát đang tắt
trader-entries-label = Lệnh vào mới
trader-entries-open = Đang mở
trader-entries-monitor-off = Giám sát vào lệnh đang tắt
trader-entries-no-live-strategy = Không có chiến lược vào lệnh Live
trader-entries-no-paper-strategy = Không có chiến lược vào lệnh thử nghiệm
trader-action-start = Bật Auto Trader
trader-action-stop = Dừng Auto Trader
trader-notice-loss-limit = Đã chạm giới hạn lỗ
trader-notice-loss-limit-message = Lệnh vào Live mới tạm dừng cho đến khi kỳ được đặt lại. Lệnh thoát vẫn tiếp tục.
trader-notice-stats-failed = Không tải được thống kê
trader-controls-group =
    .aria-label = Điều khiển giao dịch
trader-monitor-dormant = Khởi động cùng Auto Trader

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

trader-stats-window =
    .aria-label = Khoảng thống kê
trader-stats-book =
    .aria-label = Sổ giao dịch
trader-exposure-cash = Tiền mặt
trader-exposure-open = Đang mở
trader-exposure-unrealized = Chưa chốt
trader-exposure-open-title = Vị thế chiếm một suất, trên giới hạn vị thế mở
trader-exposure-in-use = Đang dùng
trader-exposure-in-use-title = { -sol } đã chi vào các vị thế mở
# $amount is the formatted count.
trader-exposure-unpriced =
    { $count ->
       *[other] { $amount } vị thế mở chưa có giá trực tiếp
    }
# $date is a formatted date; $amount is a formatted SOL amount.
trader-exposure-cash-title = Đợt thử nghiệm từ { $date } · bắt đầu với { $amount }
trader-paper-reset = Đặt lại đợt thử nghiệm
trader-paper-reset-title = Đặt lại đợt thử nghiệm?
trader-paper-reset-run-message = Mọi vị thế thử nghiệm đang mở, kể cả các khoản nắm giữ thử nghiệm của tác vụ copy, sẽ đóng theo giá hiện tại và một đợt mới bắt đầu với số dư ban đầu đã cài đặt. Các đợt trước vẫn nằm trong lịch sử.
trader-paper-reset-done = Đã bắt đầu đợt thử nghiệm mới
trader-paper-reset-failed = Không thể đặt lại đợt thử nghiệm
trader-realized-title = Hiệu suất đã chốt
trader-metric-net-pnl = P&L ròng
trader-metric-win-rate = Tỷ lệ thắng
trader-metric-profit-factor = Hệ số lợi nhuận
trader-metric-max-drawdown = Mức sụt giảm tối đa
trader-metric-avg-win-loss = Thắng / thua TB
trader-metric-closed-trades = Lệnh đã đóng
trader-metric-median-hold = Thời gian nắm giữ trung vị
trader-metric-fees = Phí mạng
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
trader-stats-excluded =
    { $count ->
       *[other] Đã loại { $amount } giao dịch đã đóng: giá vốn chưa đầy đủ nên P&L không xác định.
    }
# $ratio is a formatted ratio.
trader-stats-payoff = Payoff { $ratio }
trader-stats-streak-win =
    { $count ->
       *[other] { $amount } lệnh thắng liên tiếp
    }
trader-stats-streak-loss =
    { $count ->
       *[other] { $amount } lệnh thua liên tiếp
    }
# $wins and $losses are formatted durations.
trader-stats-hold-split = Thắng { $wins } · thua { $losses }
trader-stats-fees-basis = Đã tính trong P&L ròng
trader-stats-loading = Đang tải thống kê...
trader-open-strategies = Mở Chiến lược

trader-daily-title = P&L hằng ngày
trader-daily-subtitle = { -sol } đã chốt mỗi ngày, kèm tổng lũy kế
trader-daily-chart = Lãi/lỗ đã chốt hằng ngày tính bằng { -sol }
trader-weekly-title = P&L hằng tuần
trader-weekly-subtitle = { -sol } đã chốt mỗi tuần ISO, kèm tổng lũy kế
trader-weekly-chart = Lãi/lỗ đã chốt hằng tuần tính bằng { -sol }
# $date is a formatted date; $pnl a signed SOL amount.
trader-daily-bar =
    { $count ->
       *[other] { $date } · { $pnl } · { $amount } lệnh
    }
trader-weekly-bar =
    { $count ->
       *[other] Tuần từ { $date } · { $pnl } · { $amount } lệnh
    }
trader-extreme-best = Lệnh tốt nhất
trader-extreme-worst = Lệnh tệ nhất
trader-extreme-runs = Nhiều nhất liên tiếp
trader-outcome-title = Kết quả lệnh
trader-outcome-subtitle = Lệnh đã đóng theo lợi nhuận, tính bằng phần trăm
# $value, $from and $to are formatted signed percentages.
trader-outcome-below = Dưới { $value }
trader-outcome-range = Từ { $from } đến { $to }
trader-outcome-above = { $value } trở lên
# $range is one of the three messages above; $pnl a signed SOL amount.
trader-outcome-bucket =
    { $count ->
       *[other] { $range } · { $amount } lệnh · { $pnl }
    }

trader-stats-empty-day = Không có lệnh đã đóng trong 24 giờ qua
trader-stats-empty-all = Chưa có lệnh đã đóng
trader-stats-empty-hint = Giao dịch đã đóng xuất hiện ở đây khi một vị thế được bán. Hãy bật Auto Trader hoặc chọn khoảng thời gian dài hơn.
trader-stats-empty-days =
    { $count ->
       *[other] Không có lệnh đã đóng trong { $amount } ngày qua
    }
trader-exits-title = Lý do thoát lệnh
trader-exits-subtitle = Các lệnh đã đóng kết thúc ra sao, và mỗi lần thoát mang lại gì
trader-sources-title = Nguồn vào lệnh
trader-sources-subtitle = Điều gì đã mở các lệnh đã đóng
trader-recent-title = Lệnh đóng gần đây
trader-recent-subtitle = Các lệnh đóng mới nhất trong khoảng này
trader-column-token = Token
trader-column-exit = Thoát lệnh
trader-column-source = Nguồn
trader-column-trades = Lệnh
trader-column-win-rate = Tỷ lệ thắng
trader-column-avg-return = Lợi nhuận TB
trader-column-net-pnl = P&L ròng
trader-column-held = Nắm giữ
trader-column-closed = Đã đóng
trader-column-pnl = P&L
trader-column-return = Lợi nhuận

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
trader-stop-loss-impact = Thoát khi giảm { $threshold }% so với giá vào
trader-stop-loss-hold-immediate = Ngay lập tức
trader-stop-loss-hold-delay = Trễ { $span }
trader-stop-loss-price-falls = Giá giảm
trader-stop-loss-threshold-reached = Đạt ngưỡng
trader-stop-loss-sell-part = Bán { $part }
trader-stop-loss-part-of-position = Một phần vị thế
trader-stop-loss-sell-rest = Bán phần còn lại
trader-stop-loss-still-below = Vẫn dưới mức stop
trader-stop-loss-summary = Lỗ được giới hạn ở <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Lưu ý:</strong> Stop loss bảo vệ bạn khỏi các khoản lỗ lớn hơn bằng cách thoát sớm

trader-trailing-title = Trailing stop
trader-trailing-subtitle = Tự động bảo vệ lợi nhuận bằng cách bám theo giá khi giá tăng
trader-trailing-activation-impact = Bắt đầu bám theo khi lãi +{ $value }%
trader-trailing-distance-impact = Thoát khi giảm -{ $value }% so với đỉnh
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = Mức tối thiểu được khóa khi kích hoạt: { $value }
trader-trailing-locked-minimum-warning = Khi kích hoạt, mức dừng nằm tại hoặc dưới giá vào; nó chỉ bán khi đang có lời. Hãy bật dừng lỗ hòa vốn để có mức sàn.
trader-trailing-activation = Kích hoạt
trader-trailing-peak = Đỉnh
trader-trailing-final = Cuối cùng +{ $value }%
trader-trailing-summary-protected = Đã bảo vệ <strong>{ $value }</strong> lợi nhuận
trader-trailing-summary-avoided = Đã tránh <strong>{ $value }</strong> lỗ tính từ đỉnh
# $step is the band's position in the list, from 1.
trader-trailing-band = Dải { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-profit-label = Lợi nhuận đỉnh của dải { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-distance-label = Khoảng cách trailing của dải { $step }
trader-trailing-band-profit-caption = Từ lợi nhuận đỉnh
trader-trailing-band-distance-caption = Bám cách
trader-trailing-add-band = Thêm dải
# $step is the band's position in the list, from 1.
trader-trailing-remove-band = Xóa dải { $step }
# $min and $max are the configured bounds of a band's peak profit.
trader-trailing-band-error-profit-range = Lợi nhuận đỉnh của mỗi dải phải từ { $min }% đến { $max }%
# $min and $max are the configured bounds of a band's distance.
trader-trailing-band-error-distance-range = Khoảng cách của mỗi dải phải từ { $min }% đến { $max }%
trader-trailing-band-error-order = Mỗi dải cần lợi nhuận đỉnh cao hơn mức kích hoạt và dải trước đó
trader-trailing-band-error-distance = Mỗi dải phải bám với khoảng cách nhỏ hơn lợi nhuận đỉnh của nó

## Break-even stop

trader-break-even-title = Dừng lỗ hòa vốn
trader-break-even-subtitle = Khi vị thế đã có lời, bán toàn bộ trước khi chuyển thành lỗ
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = Khi giá tốt nhất kể từ lần mua gần nhất đạt { $trigger }, giá quay về { $offset } sẽ bán toàn bộ.

trader-take-profit-title = Take profit
trader-take-profit-subtitle = Bán vị thế theo từng bậc khi lợi nhuận chạm mỗi mức
trader-take-profit-initial-buy = Lần mua đầu
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = Bậc { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = Mức lợi nhuận bậc { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = Phần bán của bậc { $step }
trader-take-profit-profit-caption = Mức lợi nhuận
trader-take-profit-sell-caption = Phần bán
trader-take-profit-add-step = Thêm bậc
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = Xóa bậc { $step }
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = Mỗi mức lợi nhuận phải nằm trong khoảng { $min }% đến { $max }%
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = Mỗi phần bán phải nằm trong khoảng { $min }% đến { $max }%
trader-take-profit-error-order = Mỗi bậc cần mức lợi nhuận cao hơn bậc trước
# $max is the largest total share, the whole position.
trader-take-profit-error-total = Tổng các bậc chỉ được bán tối đa { $max }%
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = { $runner }% vị thế vẫn mở như phần runner
trader-take-profit-runner-none = Bậc cuối bán phần còn lại của vị thế
trader-take-profit-runner-warning = Khi trailing stop, dừng lỗ hòa vốn, cả hai quy tắc thoát theo thời gian và mọi chiến lược thoát lệnh đều tắt, phần runner chỉ đóng bằng stop loss hoặc thủ công
trader-take-profit-info = Phần bán tính theo vị thế ban đầu. Khi giá vượt nhiều mức cùng lúc, mỗi lần kiểm tra chỉ bán một bậc.
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = Tại +{ $profit }%, bán { $share }%
trader-take-profit-example-runner = Runner
trader-take-profit-example-runner-detail = Để các quy tắc thoát khác xử lý
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = Bán <strong>{ $sold }%</strong> đến mức lợi nhuận +{ $profit }%

trader-time-loss-title = Đóng vị thế đang lỗ
trader-time-subtitle = Tự động thoát vị thế sau thời gian nắm giữ tối đa nếu mức lỗ vượt ngưỡng
trader-time-flat-title = Đóng vị thế đi ngang
trader-time-flat-subtitle = Giải phóng vốn khỏi các vị thế không tăng cũng không giảm
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = Vị thế giữ { $span } trở lên với lãi/lỗ trong khoảng ±{ $band } sẽ được đóng.
trader-time-unit-seconds = giây
trader-time-unit-minutes = phút
trader-time-unit-hours = giờ
trader-time-unit-days = ngày
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
trader-time-loss-hint = Mức lỗ mà thoát theo thời gian bán sau khi hết thời gian giữ (40 = thoát ở -40%)
trader-time-day = Ngày { $day }
trader-time-position-opened = Đã mở vị thế
trader-time-limit = Hạn thời gian
trader-time-hold-reached = Đã hết thời gian nắm giữ
trader-time-loss-met = Đạt ngưỡng lỗ
trader-time-note = <strong>Lưu ý:</strong> Các vị thế đang có lãi hoặc lỗ nhỏ hơn sẽ KHÔNG bị thoát
trader-time-positions-title = Trạng thái vị thế hiện tại
trader-time-positions-loading = Đang tải vị thế...
trader-time-positions-empty = Không có vị thế đang mở
trader-time-positions-empty-message = Các vị thế đang mở hiển thị tại đây cùng thời gian nắm giữ và ROI so với giới hạn thời gian.
trader-time-positions-token = Token
trader-time-positions-hold = Thời gian nắm giữ
trader-time-positions-roi = ROI
trader-time-positions-rule = Thoát theo thời gian
trader-time-status-off = Thoát theo thời gian đang tắt
trader-time-status-none = Hiện không có quy tắc thoát theo thời gian nào áp dụng
trader-time-status-not-managed = Không do Auto Trader quản lý
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = Đang lỗ { $pnl }; đóng sau { $remaining }
trader-time-status-losing-due = Đang lỗ { $pnl }; đóng ở lần kiểm tra tới
# $held and $remaining are formatted durations.
trader-time-status-flat = Đi ngang { $held }; đóng sau { $remaining }
trader-time-status-flat-due = Đi ngang { $held }; đóng ở lần kiểm tra tới

trader-dca-title = DCA (trung bình giá)
trader-dca-subtitle = Tự động thêm vào các vị thế đang lỗ để hạ giá vào trung bình
trader-dca-measure-average = Giá vào trung bình
trader-dca-measure-first-entry = Giá vào lần đầu
trader-dca-step = Mua thêm { $step }
trader-dca-drop-label = Mức giảm giá cho lần mua thêm { $step }
trader-dca-size-label = Kích thước lần mua thêm { $step }, bội số của lần mua đầu
trader-dca-drop-caption = Mức giảm giá
trader-dca-size-caption = Mua × lần mua đầu
trader-dca-add-step = Thêm mức
trader-dca-remove-step = Xóa lần mua thêm { $step }
trader-dca-step-amount = = { $amount }
trader-dca-step-below-minimum = Dưới giao dịch tối thiểu { $minimum }: lần mua thêm này không được gửi
trader-dca-error-drop-range = Mỗi mức giảm phải nằm trong khoảng { $min }% đến { $max }%
trader-dca-error-size-range = Mỗi kích thước phải nằm trong khoảng { $min }× đến { $max }× lần mua đầu
trader-dca-error-order = Khi đo từ giá vào lần đầu, mỗi lần mua thêm cần mức giảm sâu hơn lần trước
trader-dca-cooldown-impact = Cách nhau ít nhất { $span } giữa các lần vào thêm
trader-dca-min-hold-none = Lần mua thêm đầu tiên có thể diễn ra ngay sau khi vào lệnh
trader-dca-min-hold-impact = Lần mua thêm đầu tiên chờ ít nhất { $span } sau khi vào lệnh
trader-dca-capacity = Mức tối đa vị thế này có thể nắm giữ: { $amount } ({ $multiple }× lần mua đầu)
trader-dca-info-title = Thông tin chiến lược DCA
trader-dca-info-subtitle = Những điều cần lưu ý khi giao dịch DCA
trader-dca-how-title = DCA hoạt động thế nào
trader-dca-how-trigger = <strong>Kích hoạt:</strong> giá giảm xuống dưới mức tiếp theo, đo từ giá vào trung bình hoặc lần đầu
trader-dca-how-action = <strong>Hành động:</strong> Thêm { -sol } để giảm giá vốn trung bình
trader-dca-how-repeat = <strong>Bậc thang:</strong> mỗi lần mua thêm chuyển sang mức tiếp theo; dùng hết các mức thì dừng mua thêm
trader-dca-how-pause = <strong>Tạm dừng:</strong> không mua thêm khi lần trước vẫn đang được xử lý, hoặc khi giới hạn lỗ chặn lệnh vào mới
trader-dca-risk-title = Cảnh báo rủi ro
trader-dca-risk-exposure = <strong>Rủi ro tăng:</strong> DCA làm tăng tổng vốn chịu rủi ro của mỗi vị thế
trader-dca-risk-knife = <strong>Bắt dao rơi:</strong> DCA không giúp được nếu token tiếp tục xu hướng giảm
trader-dca-risk-cooldown = <strong>Thời gian chờ:</strong> Dùng thời gian chờ để tránh các lần DCA dồn dập

trader-sizing-title = Quy mô vị thế
trader-sizing-subtitle = Kiểm soát số tiền đầu tư cho mỗi vị thế
trader-timing-title = Thời gian & thời gian chờ
trader-timing-subtitle = Kiểm soát khoảng thời gian giữa các thao tác
trader-timing-close-cooldown = Thời gian chờ sau khi đóng vị thế
trader-timing-close-cooldown-hint = Số phút chờ trước khi mở lại cùng một token
trader-timing-concurrency = Số luồng kiểm tra vào lệnh
trader-timing-concurrency-hint = Số token kiểm tra cùng lúc (càng cao càng nhanh nhưng tốn CPU hơn)
trader-timing-unit-minutes = phút
trader-timing-unit-tokens = token

## Service mode notice

trader-service-notice-title = Đang chạy trên dịch vụ { -brand }
trader-service-notice-message = Giá được làm mới vài giây một lần, nên các lệnh thoát nhanh như trailing stop sát giá phản ứng chậm hơn so với dùng RPC của riêng bạn.
