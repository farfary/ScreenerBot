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
strategies-summary-period-seconds = Khoảng nhìn lại: { $amount } giây
strategies-summary-period-minutes = Khoảng nhìn lại: { $amount } phút
strategies-summary-period-hours = Khoảng nhìn lại: { $amount } giờ

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
strategies-action-export = Xuất

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
strategies-data-candles = Nến
strategies-data-pool = Pool
strategies-data-position = Vị thế
strategies-data-market = Hoạt động thị trường

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
    .description = Phát hiện các mẫu nến cụ thể: thân lớn, thân nhỏ (doji), bóng nến dài
strategies-condition-candle-size-param-pattern = Loại mẫu
    .description = Mẫu nến cần phát hiện
strategies-condition-candle-size-param-pattern-option-large-body = Thân lớn (biến động mạnh)
strategies-condition-candle-size-param-pattern-option-small-body = Thân nhỏ (doji/lưỡng lự)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Bóng trên dài (bị từ chối)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Bóng dưới dài (hỗ trợ)
strategies-condition-candle-size-param-threshold = Ngưỡng kích thước %
    .description = Ngưỡng phần trăm để phát hiện mẫu

strategies-condition-consecutive-candles = Nến liên tiếp
    .description = Phát hiện các nến xanh (tăng) hoặc đỏ (giảm) liên tiếp với bộ lọc kích thước tối thiểu
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
    .description = Kiểm tra vị thế đã được nắm giữ bao lâu (cho chiến lược thoát lệnh - thoát theo thời gian)
strategies-condition-position-holding-time-param-hours = Ngưỡng thời gian (giờ)
    .description = Khoảng thời gian tính bằng giờ kể từ khi mở vị thế
strategies-condition-position-holding-time-param-comparison = So sánh
    .description = Cách so sánh tuổi vị thế với ngưỡng
strategies-condition-position-holding-time-param-comparison-option-greater-than = Lâu hơn (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = Ít nhất (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Mới hơn ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = Nhiều nhất (≤)

strategies-condition-price-breakout = Breakout giá
    .description = Phát hiện giá vượt lên trên kháng cự (đỉnh của kỳ) hoặc xuống dưới hỗ trợ (đáy của kỳ)
strategies-condition-price-breakout-param-lookback = Khoảng nhìn lại
    .description = Số nến để xác định mức hỗ trợ/kháng cự
strategies-condition-price-breakout-param-direction = Hướng breakout
    .description = Hướng của breakout
strategies-condition-price-breakout-param-direction-option-upward = Lên (phá kháng cự)
strategies-condition-price-breakout-param-direction-option-downward = Xuống (phá hỗ trợ)
strategies-condition-price-breakout-param-confirmation = Xác nhận %
    .description = Giá phải vượt qua mức đó bao xa để xác nhận breakout (tránh tín hiệu giả)

strategies-condition-price-change-percent = Biến động giá %
    .description = Kiểm tra giá có thay đổi theo ngưỡng phần trăm trong một khoảng thời gian hay không
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
    .description = Kiểm tra giá ở trên, dưới hoặc trong khoảng của đường trung bình động đơn giản (SMA)
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
    .description = Phát hiện khối lượng tăng vọt so với khối lượng trung bình (cho thấy sự quan tâm tăng)
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
    .description = Kích hoạt khi đường trung bình động nhanh cắt đường chậm trong vài nến gần nhất. Chỉ đọc nến.
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
    .description = Kích hoạt khi lợi nhuận chưa thực hiện của vị thế đang mở cao hơn hoặc thấp hơn một tỷ lệ phần trăm của giá vào lệnh, tính theo giá pool. Chỉ dùng cho chiến lược thoát lệnh.
strategies-condition-position-profit-param-comparison = So sánh
    .description = Lợi nhuận phải tối thiểu hay tối đa bằng ngưỡng
strategies-condition-position-profit-param-comparison-option-above = Ít nhất (≥)
strategies-condition-position-profit-param-comparison-option-below = Nhiều nhất (≤)
strategies-condition-position-profit-param-threshold-pct = Ngưỡng lợi nhuận %
    .description = Lợi nhuận chưa thực hiện theo phần trăm giá vào lệnh; giá trị âm là lỗ

strategies-condition-drawdown-from-peak = Sụt giảm từ đỉnh
    .description = Kích hoạt khi giá pool giảm một tỷ lệ phần trăm so với giá cao nhất kể từ lúc vào lệnh, sau khi đỉnh đó đã đủ cao hơn giá vào lệnh. Chỉ đọc giá pool. Chỉ dùng cho chiến lược thoát lệnh.
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
strategies-error-field-peak-price = giá đỉnh
strategies-error-field-slow-period = chu kỳ chậm
strategies-error-field-window = khung
strategies-error-field-within-candles = số nến giao cắt

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

strategies-error-indicator-consecutive-candles = nến liên tiếp
strategies-error-indicator-moving-average = đường trung bình động
strategies-error-indicator-price-breakout = breakout giá
strategies-error-indicator-price-change-lookback = khoảng nhìn lại biến động giá
strategies-error-indicator-volume-spike = khối lượng tăng vọt
strategies-error-indicator-moving-average-cross = giao cắt đường trung bình động
strategies-error-indicator-rsi = RSI

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
