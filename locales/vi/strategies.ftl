strategies-filter-all = Tất cả
strategies-filter-entry = Vào lệnh
strategies-filter-exit = Thoát lệnh
strategies-type-entry = Vào lệnh
strategies-type-exit = Thoát lệnh
strategies-list-empty-title = Chưa có chiến lược nào
strategies-list-empty-hint = Tạo chiến lược đầu tiên của bạn
strategies-new = Chiến lược mới
strategies-import =
    .title = Nhập chiến lược
    .aria-label = Nhập chiến lược
strategies-item-enable =
    .title = Bật
strategies-item-disable =
    .title = Tắt

strategies-new-name = Chiến lược mới

strategies-editor-name =
    .placeholder = Tên chiến lược
strategies-editor-dirty =
    .title = Có thay đổi chưa lưu
strategies-action-validate = Kiểm tra
strategies-editor-empty = Chọn một chiến lược để chỉnh sửa hoặc tạo chiến lược mới
strategies-conditions-empty-title = Chưa có điều kiện nào
strategies-conditions-empty-hint = Dùng "{ strategies-add-condition }" để bắt đầu xây dựng
strategies-add-condition = Thêm điều kiện
strategies-modal-close =
    .aria-label = Đóng
strategies-card-move-up =
    .title = Chuyển lên
strategies-card-move-down =
    .title = Chuyển xuống
strategies-card-duplicate =
    .title = Nhân bản
strategies-card-delete =
    .title = Xóa
# $name is the condition name.
strategies-card-delete-confirm = Gỡ điều kiện
    .message = Gỡ "{ $name }" khỏi chiến lược này?

strategies-summary-param = { $label }: { $value }
strategies-summary-none = Không có tham số
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = Thiết lập của chiến lược ({ $value })
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount } giây
strategies-value-duration-minutes = { $amount } phút
strategies-value-duration-hours = { $amount } giờ
strategies-value-duration-days = { $amount } ngày
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = Giờ địa phương của bạn: { $time }
strategies-hint-price-impact = Một giao dịch cỡ này làm giá dịch chuyển khoảng { $amount }%

strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
       *[other] { $amount } giờ
    }
strategies-value-candles =
    { $count ->
       *[other] { $amount } nến
    }

strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = giờ
strategies-unit-multiplier = ×
strategies-unit-utc = UTC

strategies-catalog-search =
    .placeholder = Tìm điều kiện...
    .aria-label = Tìm điều kiện
strategies-catalog-search-clear =
    .aria-label = Xóa tìm kiếm
strategies-catalog-no-description = Không có mô tả

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = Hoạt động
strategies-editor-timeframe = Khung thời gian mặc định
    .title = Khung thời gian nến cho các điều kiện không tự đặt khung riêng
strategies-editor-priority = Mức ưu tiên
    .title = Thứ tự đánh giá từ 1 đến 100; số nhỏ hơn được đánh giá trước
strategies-editor-description = Mô tả
strategies-editor-description-field =
    .placeholder = Chiến lược này làm gì
    .aria-label = Mô tả chiến lược
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = Tất cả chiến lược vào lệnh
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = Quản lý
strategies-editor-entry-binding-hint = Quản lý các vị thế do những chiến lược vào lệnh được chọn mở. Khi không chọn chiến lược nào, nó quản lý mọi vị thế, kể cả vị thế thủ công và sao chép.
strategies-editor-entry-binding-empty = Chưa có chiến lược vào lệnh nào
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id } (đã xóa)
strategies-action-export = Xuất
strategies-action-test = Thử với một token
# $token is the tested token's symbol or address.
strategies-test-title = Thử với { $token }
# $token is the tested token's symbol or address.
strategies-test-running = Đang thử với { $token }…
strategies-test-verdict-pass = Đạt
strategies-test-verdict-fail = Không đạt
strategies-test-verdict-unknown = Không rõ

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = Điều kiện cần thỏa mãn: { $mode }
strategies-group-match = Cần thỏa mãn: { $mode }
strategies-match-all = tất cả
strategies-match-any = ít nhất một
strategies-match-mode =
    .aria-label = Tất cả hoặc ít nhất một điều kiện
strategies-invert = Đảo ngược
    .title = Đảo ngược: đúng khi mục này không khớp
strategies-chip-off = Tắt
strategies-chip-inverted = Đã đảo
strategies-row-switch =
    .title = Tắt để vẫn lưu giữ nhưng không đánh giá
    .aria-label = Được chiến lược này đánh giá
strategies-row-expand =
    .aria-label = Hiện hoặc ẩn tham số
strategies-add-group = Thêm nhóm
# $count is the number of conditions in the group.
strategies-group-delete-confirm = Gỡ nhóm
    .message = Gỡ nhóm này cùng { $count } điều kiện trong đó?

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems = Cần sửa { $count } vấn đề: { $first }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }, { $field }: { $problem }
strategies-problem-rules-empty = Thêm ít nhất một điều kiện đang bật
strategies-problem-group-empty = Nhóm cần ít nhất một điều kiện đang bật
strategies-problem-not-applicable = Không thể dùng điều kiện này trong loại chiến lược này
strategies-problem-unknown-condition = Loại điều kiện không xác định: { $type }
strategies-problem-too-deep = Quy tắc chỉ lồng nhau tối đa { $max } cấp; nhóm đảo ngược tính thêm một cấp
strategies-problem-required = Nhập một số
strategies-problem-range = Phải nằm trong khoảng từ { $min } đến { $max }
strategies-problem-invalid-option = Chọn một trong các tùy chọn có sẵn

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = Tất cả
strategies-catalog-categories =
    .aria-label = Danh mục
strategies-catalog-empty = Không có điều kiện nào khớp với tìm kiếm
strategies-catalog-exit-only = Chỉ dành cho chiến lược thoát lệnh
strategies-catalog-entry-only = Chỉ dành cho chiến lược vào lệnh
strategies-catalog-candles-note = { $description } Chỉ đọc nến.
strategies-data-candles = Nến
strategies-data-pool = Pool
strategies-data-position = Vị thế
strategies-data-market = Hoạt động thị trường
strategies-data-clock = Giờ trong ngày

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = lệnh
strategies-value-trades = { $amount } lệnh

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = Quá nhiều cấp
    .message = Quy tắc chỉ lồng nhau tối đa { $max } cấp
strategies-toast-priority-invalid = Mức ưu tiên không hợp lệ
    .message = Nhập một số nguyên từ 1 đến 100
strategies-toast-exported = Đã xuất chiến lược
# $name is the strategy name.
strategies-unsaved-confirm = Bỏ các thay đổi chưa lưu?
    .message = "{ $name }" có thay đổi chưa được lưu.
strategies-unsaved-discard = Bỏ thay đổi
strategies-import-error-json = Tệp không phải JSON hợp lệ
strategies-import-error-shape = Tệp không mô tả một chiến lược
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = Tệp dùng một điều kiện mà phiên bản này không có: { $type }
strategies-import-error-not-applicable = Không thể dùng { $name } trong loại chiến lược này

strategies-create-title = Tạo chiến lược mới
strategies-create-prompt = Chọn loại chiến lược bạn muốn tạo:
strategies-create-entry-name = Chiến lược vào lệnh
strategies-create-entry-description = Xác định điều kiện để mua một token
strategies-create-exit-name = Chiến lược thoát lệnh
strategies-create-exit-description = Xác định điều kiện để bán một token

strategies-delete-title = Xóa chiến lược
strategies-delete-message = Xóa chiến lược "{ $name }"? Không thể hoàn tác thao tác này.

strategies-toast-fix-validation = Vui lòng sửa các lỗi kiểm tra trước khi lưu
strategies-toast-load-failed = Tải thất bại
    .message = Không thể tải chiến lược từ máy chủ
strategies-toast-load-strategy-failed = Không thể tải chiến lược
strategies-toast-no-strategy = Chưa tạo chiến lược
    .message = Hãy thêm ít nhất một điều kiện hoặc nhấp 'Chiến lược mới' để tạo chiến lược trước
strategies-toast-name-required = Cần nhập tên
    .message = Nhập tên chiến lược trước khi lưu
strategies-toast-saved = Đã lưu chiến lược
    .message = Đã lưu "{ $name }" thành công
strategies-toast-save-failed = Lưu thất bại
    .message = Không thể lưu chiến lược vào cơ sở dữ liệu
strategies-toast-no-strategy-validate = Không có chiến lược để kiểm tra
strategies-toast-no-strategy-test = Không có chiến lược để thử
strategies-toast-test-save-first = Hãy lưu chiến lược trước khi thử: phép thử đọc các quy tắc đã lưu
strategies-toast-test-failed = Thử thất bại
    .message = Không thể thử chiến lược
strategies-toast-valid = Chiến lược hợp lệ
strategies-toast-invalid = Chiến lược có lỗi
strategies-toast-validation-failed = Kiểm tra thất bại
strategies-toast-item-enabled = Đã bật chiến lược
strategies-toast-item-disabled = Đã tắt chiến lược
strategies-toast-item-toggle-failed = Không thể bật/tắt chiến lược
strategies-toast-deleted = Đã xóa chiến lược
    .message = Đã xóa "{ $name }" thành công
strategies-toast-delete-failed = Xóa thất bại
    .message = Không thể xóa chiến lược khỏi cơ sở dữ liệu
strategies-toast-imported = Đã nhập chiến lược
strategies-toast-import-failed = Không thể nhập chiến lược
strategies-toast-unknown-condition = Điều kiện không xác định
    .message = Không tìm thấy loại điều kiện
strategies-toast-create-first = Hãy tạo chiến lược trước
    .message = Nhấp 'Chiến lược mới' để tạo chiến lược trước khi thêm điều kiện

strategies-condition-candle-size = Mẫu kích thước nến
    .description = Phát hiện các mẫu nến cụ thể: thân lớn, thân nhỏ (doji), bóng nến dài.
strategies-condition-candle-size-param-pattern = Loại mẫu
    .description = Mẫu nến cần phát hiện
strategies-condition-candle-size-param-pattern-option-large-body = Thân lớn (biến động mạnh)
strategies-condition-candle-size-param-pattern-option-small-body = Thân nhỏ (doji/lưỡng lự)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Bóng trên dài (bị từ chối)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Bóng dưới dài (hỗ trợ)
strategies-condition-candle-size-param-threshold = Ngưỡng kích thước %
    .description = Ngưỡng phần trăm để phát hiện mẫu

strategies-condition-consecutive-candles = Nến liên tiếp
    .description = Phát hiện các nến xanh (tăng) hoặc đỏ (giảm) liên tiếp với bộ lọc kích thước tối thiểu.
strategies-condition-consecutive-candles-param-count = Số nến
    .description = Số nến liên tiếp cần có
strategies-condition-consecutive-candles-param-direction = Hướng nến
    .description = Màu/hướng của các nến liên tiếp
strategies-condition-consecutive-candles-param-direction-option-green = Xanh (tăng)
strategies-condition-consecutive-candles-param-direction-option-red = Đỏ (giảm)
strategies-condition-consecutive-candles-param-minimum-change = Biến động tối thiểu %
    .description = % biến động tối thiểu của mỗi nến (lọc nhiễu)

strategies-condition-liquidity-level = Mức thanh khoản pool
    .description = Kiểm tra thanh khoản pool tính bằng { -sol } (Vào lệnh: đảm bảo đủ thanh khoản, Thoát lệnh: phát hiện thanh khoản bị rút)
strategies-condition-liquidity-level-param-threshold = Ngưỡng thanh khoản ({ -sol })
    .description = Mức thanh khoản pool tính bằng { -sol }
strategies-condition-liquidity-level-param-comparison = So sánh
    .description = Cách so sánh thanh khoản pool với ngưỡng
strategies-condition-liquidity-level-param-comparison-option-greater-than = Lớn hơn (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = Lớn hơn hoặc bằng (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = Nhỏ hơn ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = Nhỏ hơn hoặc bằng (≤)

strategies-condition-position-holding-time = Thời gian nắm giữ vị thế
    .description = Kiểm tra vị thế đã được nắm giữ bao lâu
strategies-condition-position-holding-time-param-hours = Ngưỡng thời gian (giờ)
    .description = Khoảng thời gian tính bằng giờ kể từ khi mở vị thế
strategies-condition-position-holding-time-param-comparison = So sánh
    .description = Cách so sánh tuổi vị thế với ngưỡng
strategies-condition-position-holding-time-param-comparison-option-greater-than = Lâu hơn (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = Ít nhất (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Mới hơn ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = Nhiều nhất (≤)

strategies-condition-price-breakout = Breakout giá
    .description = Phát hiện giá vượt lên trên kháng cự (đỉnh của kỳ) hoặc xuống dưới hỗ trợ (đáy của kỳ).
strategies-condition-price-breakout-param-lookback = Khoảng nhìn lại
    .description = Số nến để xác định mức hỗ trợ/kháng cự
strategies-condition-price-breakout-param-direction = Hướng breakout
    .description = Hướng của breakout
strategies-condition-price-breakout-param-direction-option-upward = Lên (phá kháng cự)
strategies-condition-price-breakout-param-direction-option-downward = Xuống (phá hỗ trợ)
strategies-condition-price-breakout-param-confirmation = Xác nhận %
    .description = Giá phải vượt qua mức đó bao xa để xác nhận breakout (tránh tín hiệu giả)

strategies-condition-price-change-percent = Biến động giá %
    .description = Kiểm tra giá có thay đổi theo ngưỡng phần trăm trong một khoảng thời gian hay không.
strategies-condition-price-change-percent-param-percentage = Ngưỡng biến động %
    .description = Phần trăm biến động giá để kích hoạt (0.1-1000%)
strategies-condition-price-change-percent-param-direction = Hướng
    .description = Hướng biến động giá
strategies-condition-price-change-percent-param-direction-option-above = Tăng (+%)
strategies-condition-price-change-percent-param-direction-option-below = Giảm (-%)
strategies-condition-price-change-percent-param-direction-option-within = Trong khoảng (±%)
strategies-condition-price-change-percent-param-time-value = Khoảng nhìn lại
    .description = Khoảng thời gian nhìn lại để đo thay đổi, theo đơn vị chọn trong ô (1-3600 giây, 1-1440 phút, 1-720 giờ)
strategies-condition-price-change-percent-param-time-unit = Đơn vị thời gian
    .description = Đơn vị thời gian của khoảng nhìn lại
strategies-condition-price-change-percent-param-time-unit-option-seconds = giây
strategies-condition-price-change-percent-param-time-unit-option-minutes = phút
strategies-condition-price-change-percent-param-time-unit-option-hours = giờ

strategies-condition-price-to-ma = Giá so với đường trung bình động
    .description = Kiểm tra giá ở trên, dưới hoặc trong khoảng của đường trung bình động đơn giản (SMA).
strategies-condition-price-to-ma-param-period = Chu kỳ MA
    .description = Số nến để tính đường trung bình động
strategies-condition-price-to-ma-param-position = Vị trí
    .description = Vị trí của giá so với MA
strategies-condition-price-to-ma-param-position-option-above = Trên MA
strategies-condition-price-to-ma-param-position-option-below = Dưới MA
strategies-condition-price-to-ma-param-position-option-within = Trong khoảng
strategies-condition-price-to-ma-param-distance = Khoảng cách %
    .description = Khoảng cách tối thiểu so với MA (với TRÊN/DƯỚI) hoặc khoảng tối đa (với TRONG KHOẢNG)

strategies-condition-volume-spike = Khối lượng tăng vọt
    .description = Phát hiện khối lượng tăng vọt so với khối lượng trung bình (cho thấy sự quan tâm tăng).
strategies-condition-volume-spike-param-lookback = Khoảng nhìn lại
    .description = Số nến để tính khối lượng trung bình
strategies-condition-volume-spike-param-multiplier = Hệ số khối lượng
    .description = Cao hơn trung bình bao nhiêu lần (ví dụ: 2.0 = 200% mức trung bình)

strategies-condition-rsi = RSI (Chỉ số sức mạnh tương đối)
    .description = Kích hoạt khi RSI, làm mượt theo Wilder trên các nến đã đóng, cao hơn hoặc thấp hơn một ngưỡng.
strategies-condition-rsi-param-period = Chu kỳ RSI
    .description = Số nến dùng để tính RSI; điều kiện cần nhiều hơn giá trị này một nến
strategies-condition-rsi-param-comparison = So sánh
    .description = RSI phải thấp hơn hay cao hơn ngưỡng
strategies-condition-rsi-param-comparison-option-below = Thấp hơn (quá bán)
strategies-condition-rsi-param-comparison-option-above = Cao hơn (quá mua)
strategies-condition-rsi-param-threshold = Ngưỡng RSI
    .description = Mức RSI từ 1 đến 99 (30 và 70 là các mức quá bán và quá mua thường dùng)

strategies-condition-ma-cross = Giao cắt đường trung bình động
    .description = Kích hoạt khi đường trung bình động nhanh cắt đường chậm trong vài nến gần nhất.
strategies-condition-ma-cross-param-ma-type = Loại đường trung bình
    .description = Đường trung bình hàm mũ cho các nến gần đây trọng số lớn hơn; đường trung bình đơn giản coi mọi nến như nhau
strategies-condition-ma-cross-param-ma-type-option-ema = Hàm mũ (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = Đơn giản (SMA)
strategies-condition-ma-cross-param-fast-period = Chu kỳ nhanh
    .description = Số nến của đường trung bình nhanh
strategies-condition-ma-cross-param-slow-period = Chu kỳ chậm
    .description = Số nến của đường trung bình chậm; phải lớn hơn chu kỳ nhanh
strategies-condition-ma-cross-param-direction = Hướng giao cắt
    .description = Đường nhanh cắt đường chậm theo hướng nào
strategies-condition-ma-cross-param-direction-option-bullish = Tăng giá (đường nhanh cắt lên)
strategies-condition-ma-cross-param-direction-option-bearish = Giảm giá (đường nhanh cắt xuống)
strategies-condition-ma-cross-param-within-candles = Giao cắt trong
    .description = Số nến gần nhất được phép chứa điểm giao cắt; 1 nghĩa là chỉ nến mới nhất

strategies-condition-position-profit = Lợi nhuận vị thế
    .description = Kích hoạt khi lợi nhuận chưa thực hiện của vị thế đang mở cao hơn hoặc thấp hơn một tỷ lệ phần trăm của giá vào lệnh, tính theo giá pool.
strategies-condition-position-profit-param-comparison = So sánh
    .description = Lợi nhuận phải tối thiểu hay tối đa bằng ngưỡng
strategies-condition-position-profit-param-comparison-option-above = Ít nhất (≥)
strategies-condition-position-profit-param-comparison-option-below = Nhiều nhất (≤)
strategies-condition-position-profit-param-threshold-pct = Ngưỡng lợi nhuận %
    .description = Lợi nhuận chưa thực hiện theo phần trăm giá vào lệnh; giá trị âm là lỗ

strategies-condition-drawdown-from-peak = Sụt giảm từ đỉnh
    .description = Kích hoạt khi giá pool giảm một tỷ lệ phần trăm so với giá cao nhất kể từ lần mua cuối, sau khi đỉnh đó đã đủ cao hơn giá vào lệnh.
strategies-condition-drawdown-from-peak-param-drop-pct = Sụt giảm từ đỉnh %
    .description = Giá hiện tại phải thấp hơn đỉnh bao nhiêu
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = Kích hoạt sau lợi nhuận %
    .description = Đỉnh phải từng cao hơn giá vào lệnh ít nhất mức này thì điều kiện mới có thể kích hoạt; 0 nghĩa là luôn sẵn sàng

strategies-condition-buy-pressure = Áp lực mua
    .description = Kích hoạt khi lệnh mua chiếm ít nhất một tỷ lệ giao dịch trong một khung thời gian và khung đó có đủ giao dịch. Đọc số lệnh mua và bán của token.
strategies-condition-buy-pressure-param-window = Khung thời gian
    .description = Khoảng thời gian mà số lệnh mua và bán bao phủ
strategies-condition-buy-pressure-param-window-option-5m = 5 phút
strategies-condition-buy-pressure-param-window-option-1h = 1 giờ
strategies-condition-buy-pressure-param-window-option-6h = 6 giờ
strategies-condition-buy-pressure-param-window-option-24h = 24 giờ
strategies-condition-buy-pressure-param-min-buy-share-pct = Tỷ lệ mua tối thiểu %
    .description = Lệnh mua tính theo phần trăm tổng giao dịch trong khung
strategies-condition-buy-pressure-param-min-transactions = Số giao dịch tối thiểu
    .description = Khung có ít giao dịch hơn sẽ không bao giờ kích hoạt điều kiện

strategies-condition-transaction-velocity = Tốc độ giao dịch
    .description = Kích hoạt khi tốc độ giao dịch của cửa sổ gần đây ít nhất hoặc nhiều nhất bằng một bội số tốc độ của cửa sổ dài hơn chứa nó. Đọc số lệnh mua và bán của token.
strategies-condition-transaction-velocity-param-window = Cửa sổ
    .description = Cửa sổ gần đây và cửa sổ dài hơn dùng để so sánh
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = 5 phút so với 1 giờ
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = 1 giờ so với 6 giờ
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = 6 giờ so với 24 giờ
strategies-condition-transaction-velocity-param-comparison = So sánh
    .description = Tốc độ gần đây phải ít nhất hay nhiều nhất bằng tỷ lệ
strategies-condition-transaction-velocity-param-comparison-option-above = Ít nhất (≥)
strategies-condition-transaction-velocity-param-comparison-option-below = Nhiều nhất (≤)
strategies-condition-transaction-velocity-param-ratio = Tỷ lệ tốc độ
    .description = Số giao dịch mỗi giây của cửa sổ gần đây chia cho số giao dịch mỗi giây của cửa sổ dài
strategies-condition-transaction-velocity-param-min-transactions = Số giao dịch tối thiểu
    .description = Cửa sổ gần đây có ít giao dịch hơn không bao giờ kích hoạt điều kiện

strategies-condition-pullback-from-high = Điều chỉnh từ đỉnh
    .description = Kích hoạt khi giá trực tiếp nằm giữa mức giảm tối thiểu và tối đa dưới đỉnh cao nhất của các nến gần nhất.
strategies-condition-pullback-from-high-param-lookback = Khoảng nhìn lại
    .description = Số nến dùng để lấy đỉnh cao nhất
strategies-condition-pullback-from-high-param-min-drop-pct = Mức giảm tối thiểu %
    .description = Giá phải thấp hơn đỉnh ít nhất chừng này
strategies-condition-pullback-from-high-param-max-drop-pct = Mức giảm tối đa %
    .description = Giá phải thấp hơn đỉnh nhiều nhất chừng này; phải lớn hơn mức giảm tối thiểu

strategies-condition-profit-giveback = Trả lại lợi nhuận
    .description = Kích hoạt khi vị thế đã trả lại một phần lợi nhuận chưa chốt tốt nhất kể từ lần mua gần nhất, sau khi lợi nhuận đó đạt một mức.
strategies-condition-profit-giveback-param-arm-after-profit-pct = Kích hoạt sau lợi nhuận %
    .description = Lợi nhuận tốt nhất phải đạt phần trăm này của giá vào trước khi điều kiện có thể kích hoạt
strategies-condition-profit-giveback-param-giveback-pct = Mức trả lại %
    .description = Phần lợi nhuận tốt nhất đã trả lại; 100% nghĩa là giá đã về lại giá vào

strategies-condition-token-age = Tuổi token
    .description = Kích hoạt khi thời gian kể từ khi token được tạo on-chain dài hơn hoặc ngắn hơn một khoảng thời gian. Token không rõ thời điểm tạo khiến điều kiện chưa xác định.
strategies-condition-token-age-param-comparison = So sánh
    .description = Token phải cũ hơn hay mới hơn khoảng thời gian
strategies-condition-token-age-param-comparison-option-older-than = Cũ hơn (≥)
strategies-condition-token-age-param-comparison-option-younger-than = Mới hơn ({ "<" })
strategies-condition-token-age-param-time-value = Tuổi
    .description = Thời gian kể từ khi token được tạo, theo đơn vị chọn trong ô (1-10000)
strategies-condition-token-age-param-time-unit = Đơn vị thời gian
    .description = Đơn vị của tuổi
strategies-condition-token-age-param-time-unit-option-minutes = phút
strategies-condition-token-age-param-time-unit-option-hours = giờ
strategies-condition-token-age-param-time-unit-option-days = ngày

strategies-condition-vwap = VWAP
    .description = Kích hoạt khi giá trực tiếp cao hơn hoặc thấp hơn giá trung bình gia quyền theo khối lượng của các nến gần nhất ít nhất một khoảng cách.
strategies-condition-vwap-param-lookback = Khoảng nhìn lại
    .description = Số nến mà trung bình gia quyền theo khối lượng bao phủ
strategies-condition-vwap-param-position = Vị trí
    .description = Vị trí của giá so với VWAP
strategies-condition-vwap-param-position-option-above = Trên VWAP
strategies-condition-vwap-param-position-option-below = Dưới VWAP
strategies-condition-vwap-param-distance-pct = Khoảng cách %
    .description = Khoảng cách tối thiểu tới VWAP; 0 nghĩa là khoảng cách bất kỳ ở phía đã chọn

strategies-condition-volatility-range = Biên độ biến động
    .description = Kích hoạt khi biên độ thực trung bình của các nến gần nhất, tính theo phần trăm giá trực tiếp, cao hơn hoặc thấp hơn một ngưỡng. Hai điều kiện này trong một nhóm tạo thành một khoảng.
strategies-condition-volatility-range-param-period = Chu kỳ ATR
    .description = Số nến dùng để làm mượt biên độ thực trung bình; điều kiện cần thêm một nến
strategies-condition-volatility-range-param-comparison = So sánh
    .description = Biên độ phải cao hơn hay thấp hơn ngưỡng
strategies-condition-volatility-range-param-comparison-option-above = Cao hơn
strategies-condition-volatility-range-param-comparison-option-below = Thấp hơn
strategies-condition-volatility-range-param-threshold-pct = Ngưỡng biên độ %
    .description = Biên độ thực trung bình theo phần trăm giá trực tiếp

strategies-condition-bollinger-bands = Dải Bollinger
    .description = Kích hoạt khi giá trực tiếp dưới dải dưới hoặc trên dải trên, hoặc khi các dải co hẹp lại. Các dải cách giá đóng cửa trung bình vài độ lệch chuẩn.
strategies-condition-bollinger-bands-param-period = Chu kỳ
    .description = Số nến mà trung bình và độ lệch chuẩn bao phủ
strategies-condition-bollinger-bands-param-std-dev = Độ lệch chuẩn
    .description = Mỗi dải cách trung bình bao nhiêu độ lệch chuẩn
strategies-condition-bollinger-bands-param-signal = Tín hiệu
    .description = Điều các dải phải thể hiện
strategies-condition-bollinger-bands-param-signal-option-below-lower = Dưới dải dưới
strategies-condition-bollinger-bands-param-signal-option-above-upper = Trên dải trên
strategies-condition-bollinger-bands-param-signal-option-squeeze = Co thắt
strategies-condition-bollinger-bands-param-squeeze-width-pct = Độ rộng co thắt %
    .description = Với co thắt: khoảng cách lớn nhất giữa các dải, theo phần trăm của trung bình

strategies-condition-liquidity-to-size = Thanh khoản so với quy mô
    .description = Kích hoạt khi dự trữ của pool ít nhất hoặc dưới một bội số của quy mô đang đặt cược: quy mô giao dịch trong chiến lược vào lệnh, giá trị còn lại của vị thế trong chiến lược thoát.
strategies-condition-liquidity-to-size-param-comparison = So sánh
    .description = Dự trữ phải ít nhất hay dưới bội số
strategies-condition-liquidity-to-size-param-comparison-option-at-least = Ít nhất (≥)
strategies-condition-liquidity-to-size-param-comparison-option-below = Dưới ({ "<" })
strategies-condition-liquidity-to-size-param-multiple = Bội số thanh khoản
    .description = Dự trữ pool chia cho quy mô đang đặt cược; bội số N làm giá dịch chuyển khoảng 1/(N+1)

strategies-condition-position-adds = Mua thêm vào vị thế
    .description = Kích hoạt khi số lần mua thêm sau lần vào đầu tiên, hoặc số lần bán một phần, ít nhất hoặc nhiều nhất bằng một số.
strategies-condition-position-adds-param-subject = Đối tượng đếm
    .description = Hành động nào được đếm
strategies-condition-position-adds-param-subject-option-adds = Lần mua thêm
strategies-condition-position-adds-param-subject-option-partial-sells = Lần bán một phần
strategies-condition-position-adds-param-comparison = So sánh
    .description = Số đếm phải ít nhất hay nhiều nhất bằng con số
strategies-condition-position-adds-param-comparison-option-at-least = Ít nhất (≥)
strategies-condition-position-adds-param-comparison-option-at-most = Nhiều nhất (≤)
strategies-condition-position-adds-param-count = Số lượng
    .description = Số hành động

strategies-condition-time-of-day = Giờ trong ngày
    .description = Kích hoạt khi giờ hiện tại nằm trong một cửa sổ giờ UTC. Giờ bắt đầu sau giờ kết thúc sẽ vắt qua nửa đêm.
strategies-condition-time-of-day-param-start-hour = Giờ bắt đầu (UTC)
    .description = Giờ đầu tiên của cửa sổ, từ 0 đến 23
strategies-condition-time-of-day-param-end-hour = Giờ kết thúc (UTC)
    .description = Giờ cửa sổ kết thúc, từ 1 đến 24; chính giờ đó nằm ngoài cửa sổ

strategies-condition-liquidity-change-since-entry = Thay đổi thanh khoản từ lúc vào
    .description = Kích hoạt khi dự trữ pool đã giảm hoặc tăng ít nhất một phần trăm kể từ khi mở vị thế. Vị thế không có thanh khoản lúc vào được ghi lại khiến điều kiện chưa xác định.
strategies-condition-liquidity-change-since-entry-param-comparison = So sánh
    .description = Dự trữ phải đã giảm hay đã tăng
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = Giảm
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = Tăng
strategies-condition-liquidity-change-since-entry-param-change-pct = Mức thay đổi %
    .description = Thay đổi dự trữ pool kể từ lúc vào

strategies-condition-param-timeframe = Khung thời gian
    .description = Kích thước nến: độ dài mỗi cây nến mà điều kiện đọc (khung thời gian của chiến lược nếu không đặt)
strategies-condition-timeframe-option-1m = 1 phút
strategies-condition-timeframe-option-5m = 5 phút
strategies-condition-timeframe-option-15m = 15 phút
strategies-condition-timeframe-option-1h = 1 giờ
strategies-condition-timeframe-option-4h = 4 giờ
strategies-condition-timeframe-option-12h = 12 giờ
strategies-condition-timeframe-option-1d = 1 ngày

strategies-condition-category-price-analysis = Phân tích giá
strategies-condition-category-candle-patterns = Mẫu nến
strategies-condition-category-technical-indicators = Chỉ báo kỹ thuật
strategies-condition-category-market-context = Bối cảnh thị trường
strategies-condition-category-position-performance = Vị thế & hiệu suất
strategies-condition-category-volume-analysis = Phân tích khối lượng

strategies-error-missing-parameter = Thiếu tham số { $field }
strategies-error-parameter-type = Tham số { $field } phải là { $expected }
strategies-error-invalid-value = "{ $value }" không phải giá trị { $field } hợp lệ
strategies-error-missing-data = { $data } không khả dụng
strategies-error-no-candle-data = Khung thời gian { $timeframe } không có dữ liệu nến
strategies-error-insufficient-history = Không đủ lịch sử cho { $indicator }: có { $available } giây, cần { $required } giây
strategies-error-insufficient-candles = Không đủ nến cho { $indicator }: có { $available }, cần { $required }
strategies-error-stale-candle-data = Dữ liệu nến khung { $timeframe } đã cũ: tuổi { $age } giây vượt quá { $max } giây
strategies-error-stale-market-data = Số lượng giao dịch khung { $window } đã cũ: tuổi { $age } giây vượt quá { $max } giây
strategies-error-unusable-data = Không thể dùng giá trị { $field } "{ $value }"
strategies-error-invalid-rule-tree = Cây quy tắc không hợp lệ: { $reason }
strategies-error-evaluation-timeout = Đánh giá chiến lược hết thời gian sau { $timeout } ms
strategies-error-condition-not-applicable = { $condition } không thể dùng trong chiến lược loại { $strategy }
strategies-error-invalid-rules = Không đọc được các quy tắc: { $reason }

strategies-error-field-average-volume = khối lượng trung bình
strategies-error-field-candle-open = giá mở cửa của nến
strategies-error-field-comparison = so sánh
strategies-error-field-condition-type = loại điều kiện
strategies-error-field-confirmation = xác nhận
strategies-error-field-count = số lượng
strategies-error-field-current-price = giá hiện tại
strategies-error-field-direction = hướng
strategies-error-field-distance = khoảng cách
strategies-error-field-hours = giờ
strategies-error-field-lookback = khoảng nhìn lại
strategies-error-field-minimum-change = biến động tối thiểu
strategies-error-field-multiplier = hệ số
strategies-error-field-pattern = mẫu
strategies-error-field-percentage = phần trăm
strategies-error-field-period = chu kỳ
strategies-error-field-position = vị trí
strategies-error-field-threshold = ngưỡng
strategies-error-field-time-unit = đơn vị thời gian
strategies-error-field-time-value = giá trị thời gian
strategies-error-field-timeframe = khung thời gian
strategies-error-field-arm-after-profit = kích hoạt sau lợi nhuận
strategies-error-field-drop-percentage = tỷ lệ sụt giảm
strategies-error-field-entry-price = giá vào lệnh
strategies-error-field-fast-period = chu kỳ nhanh
strategies-error-field-minimum-buy-share = tỷ lệ mua tối thiểu
strategies-error-field-minimum-transactions = số giao dịch tối thiểu
strategies-error-field-moving-average-type = loại đường trung bình
strategies-error-field-moving-average = đường trung bình động
strategies-error-field-reference-candle-close = giá đóng cửa nến tham chiếu
strategies-error-field-peak-price = giá đỉnh
strategies-error-field-slow-period = chu kỳ chậm
strategies-error-field-window = khung
strategies-error-field-within-candles = số nến giao cắt
strategies-error-field-change = mức thay đổi
strategies-error-field-end-hour = giờ kết thúc
strategies-error-field-entry-liquidity = thanh khoản lúc vào
strategies-error-field-giveback = mức trả lại
strategies-error-field-highest-high = đỉnh cao nhất
strategies-error-field-long-window-transactions = giao dịch của cửa sổ dài
strategies-error-field-maximum-drop = mức giảm tối đa
strategies-error-field-minimum-drop = mức giảm tối thiểu
strategies-error-field-multiple = bội số
strategies-error-field-pool-liquidity = thanh khoản pool
strategies-error-field-ratio = tỷ lệ
strategies-error-field-remaining-position-value = giá trị còn lại của vị thế
strategies-error-field-signal = tín hiệu
strategies-error-field-squeeze-width = độ rộng co thắt
strategies-error-field-standard-deviations = độ lệch chuẩn
strategies-error-field-start-hour = giờ bắt đầu
strategies-error-field-subject = đối tượng đếm
strategies-error-field-token-creation-time = thời điểm tạo token
strategies-error-field-total-volume = tổng khối lượng
strategies-error-field-trade-size = quy mô giao dịch
strategies-error-field-vwap = VWAP

strategies-error-expected-boolean = giá trị đúng/sai
strategies-error-expected-number = một số
strategies-error-expected-string = một chuỗi

strategies-error-data-current-price = Giá hiện tại
strategies-error-data-liquidity-data = Dữ liệu thanh khoản
strategies-error-data-market-data = Dữ liệu thị trường
strategies-error-data-ohlcv-data = Dữ liệu OHLCV
strategies-error-data-position-data = Dữ liệu vị thế
strategies-error-data-peak-price = Giá đỉnh
strategies-error-data-transaction-counts = Số lượng giao dịch
strategies-error-data-entry-liquidity = Thanh khoản lúc vào
strategies-error-data-entry-pool-liquidity = Thanh khoản của pool lúc vào
strategies-error-data-real-pool-reserves = Thanh khoản thực của pool
strategies-error-data-remaining-position-value = Giá trị còn lại của vị thế
strategies-error-data-token-creation-time = Thời điểm tạo token
strategies-error-data-trade-size = Quy mô giao dịch

strategies-error-indicator-consecutive-candles = nến liên tiếp
strategies-error-indicator-moving-average = đường trung bình động
strategies-error-indicator-price-breakout = breakout giá
strategies-error-indicator-price-change-lookback = khoảng nhìn lại biến động giá
strategies-error-indicator-volume-spike = khối lượng tăng vọt
strategies-error-indicator-moving-average-cross = giao cắt đường trung bình động
strategies-error-indicator-rsi = RSI
strategies-error-indicator-bollinger-bands = dải Bollinger
strategies-error-indicator-pullback-from-high = điều chỉnh từ đỉnh
strategies-error-indicator-volatility-range = biên độ biến động
strategies-error-indicator-vwap = VWAP

strategies-error-rule-branch-node-missing-conditions = Nút nhánh thiếu điều kiện
strategies-error-rule-branch-node-missing-operator = Nút nhánh thiếu toán tử
strategies-error-rule-branch-node-must-have-at-least-one-child = Nút nhánh phải có ít nhất một nút con
strategies-error-rule-invalid-rule-tree-structure = Cấu trúc cây quy tắc không hợp lệ
strategies-error-rule-leaf-node-missing-condition = Nút lá thiếu điều kiện
strategies-error-rule-not-operator-must-have-exactly-one-child = Toán tử NOT phải có đúng một nút con
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = Mỗi nhóm cần ít nhất một điều kiện đang bật
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = Toán tử NOT không được có nút con đã tắt
strategies-error-rule-rules-nest-deeper-than-four-levels = Các quy tắc không được lồng sâu hơn bốn cấp
strategies-error-rule-the-root-group-cannot-be-switched-off = Nhóm gốc không thể bị tắt

strategies-readiness-priced = Mọi vị thế đều có giá
strategies-readiness-priced-ok = Mọi vị thế thử nghiệm đang mở đều có giá pool
strategies-readiness-priced-missing =
    { $count ->
       *[other] { $count } vị thế thử nghiệm đang mở không có giá pool
    }
strategies-readiness-runtime = Có thể giao dịch live
strategies-readiness-runtime-ok = Thiết lập và các chốt an toàn cho phép giao dịch live
