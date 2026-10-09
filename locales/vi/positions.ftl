positions-state-reason-position-created = Đã tạo vị thế

positions-status-open = Đang mở
positions-status-closed = Đã đóng
positions-status-archived = Đã lưu trữ
# Empty positions table per status (POSITION_EMPTY_LABELS)
positions-open-empty = Không có vị thế mở
    .message = Một vị thế xuất hiện tại đây khi auto trader hoặc lệnh mua thủ công mở nó.
positions-closed-empty = Không có vị thế đã đóng
    .message = Một vị thế chuyển sang đây khi đã bán hết.
positions-archived-empty = Không có vị thế lưu trữ
    .message = Các vị thế bạn gỡ bằng Lưu trữ được giữ tại đây.
positions-origin-copy = Copy
positions-origin-manual = Thủ công
positions-origin-wallet = Ví
positions-origin-copy-link =
    .title = Mở tác vụ copy đã tạo vị thế này
positions-holding-frozen = Bị đóng băng
    .title = Quyền mint đã đóng băng tài khoản token này - không thể chuyển hoặc bán số dư
positions-toolbar-total = Tổng
positions-toolbar-delete-all = Xóa tất cả
positions-search-placeholder = Tìm theo ký hiệu hoặc mint...
positions-filter-origin = Nguồn
positions-filter-origin-all = Tất cả nguồn
positions-filter-origin-auto = Auto Trader
positions-filter-origin-copy = Copy trading
positions-delete-all-tooltip = Xóa vĩnh viễn tất cả vị thế đã lưu trữ

positions-column-token = Token
positions-column-archived-at = Đã lưu trữ
positions-column-entry-time = Thời gian vào lệnh
positions-column-exit-time = Thời gian thoát lệnh
positions-column-avg-entry = Vào TB ({ -sol })
positions-column-avg-exit = Thoát TB ({ -sol })
positions-column-current-price = Hiện tại ({ -sol })
positions-column-total-invested = Tổng đã đầu tư ({ -sol })
positions-column-proceeds = Tiền thu về ({ -sol })
positions-column-pnl = P&L ({ -sol })
positions-column-pnl-percent = P&L %
positions-column-size = Quy mô
positions-column-dca = DCA
positions-column-exits = Lần thoát
positions-column-unrealized-pnl = P&L chưa chốt ({ -sol })
positions-column-unrealized-percent = % chưa chốt

positions-unknown-basis = Lịch sử của ví này không có giá vốn (airdrop, lệnh khớp tính bằng USD hoặc swap không có chân SOL)
positions-unknown-history = Vòng này không khớp với số dư trên chuỗi
positions-dca-count =
    { $count ->
       *[other] { $count } DCA
    }
positions-exit-count =
    { $count ->
       *[other] { $count } lần thoát
    }

positions-action-add =
    .title = Thêm vào vị thế (DCA)
    .aria-label = Thêm vào vị thế
positions-action-sell =
    .title = Bán (toàn bộ hoặc một phần theo %)
    .aria-label = Bán vị thế
positions-action-sell-frozen = Bị quyền mint đóng băng - không thể bán số token này
positions-action-remove =
    .title = Gỡ (lưu trữ hoặc xóa)
    .aria-label = Gỡ vị thế
positions-action-restore =
    .title = Khôi phục về Đang mở/Đã đóng
    .aria-label = Khôi phục vị thế
positions-action-delete =
    .title = Xóa vĩnh viễn
    .aria-label = Xóa vĩnh viễn
positions-action-in-progress = Đang xử lý…

positions-caption-buying = Đang mua
positions-caption-buying-step = Đang mua · { $step }
positions-caption-selling = Đang bán
positions-caption-selling-step = Đang bán · { $step }
positions-caption-closing = Đang đóng
positions-caption-failed = Thất bại
positions-caption-failed-detail = Thất bại · { $error }
positions-step-adding = Đang thêm
positions-pending-buying = Đang mua…
positions-pending-buy-failed = Mua thất bại

positions-load-failed = Không thể làm mới vị thế
positions-toast-not-found = Không tìm thấy dữ liệu vị thế
positions-toast-deleted = Đã xóa vị thế
positions-toast-archived = Đã lưu trữ vị thế
positions-toast-restored = Đã khôi phục vị thế
positions-buy-adds-to-archived = Token này đã có một vị thế đang mở trong kho lưu trữ. Lệnh mua được cộng vào vị thế đó, và vị thế quay lại danh sách vị thế đang mở. Vị thế giữ nguyên chế độ quản lý hiện tại.
positions-action-failed = Thao tác thất bại
positions-delete-title = Xóa vĩnh viễn vị thế
positions-delete-message = Xóa vĩnh viễn { $symbol }? Thao tác này xóa vị thế và lịch sử của nó khỏi cơ sở dữ liệu và không thể hoàn tác. Giao dịch và dữ liệu token của bạn không bị ảnh hưởng.
positions-delete-confirm = Xóa vĩnh viễn
positions-delete-all-title = Xóa tất cả vị thế đã lưu trữ
positions-delete-all-message =
    { $count ->
       *[other] Xóa vĩnh viễn tất cả { $count } vị thế đã lưu trữ? Không thể hoàn tác. Giao dịch và dữ liệu token không bị ảnh hưởng.
    }
positions-delete-all-message-empty = Xóa vĩnh viễn tất cả vị thế đã lưu trữ? Không thể hoàn tác.
positions-delete-all-confirm = Xóa tất cả
positions-delete-all-done =
    { $count ->
       *[other] Đã xóa { $count } vị thế đã lưu trữ
    }
positions-delete-all-failed = Không thể xóa các vị thế đã lưu trữ

positions-remove-title = Gỡ vị thế
positions-remove-open-warning = <strong>Vị thế này vẫn đang mở.</strong> Bot đang nắm giữ token này. Gỡ vị thế sẽ giải phóng slot giao dịch và ngừng theo dõi, nhưng <strong>không</strong> bán token. Hãy bán trước nếu bạn muốn nhận lại { -sol }.
positions-remove-modes =
    .aria-label = Chế độ gỡ
positions-remove-archive = Lưu trữ
positions-remove-recommended = Đề xuất
positions-remove-archive-description = Ẩn vào tab Đã lưu trữ. Có thể hoàn tác bất cứ lúc nào - không bán gì và mọi giao dịch vẫn được lưu lại.
positions-remove-delete = Xóa vĩnh viễn
positions-remove-delete-description = Xóa vị thế này và toàn bộ lịch sử của nó khỏi cơ sở dữ liệu.
positions-remove-danger = Thao tác này xóa vĩnh viễn vị thế và lịch sử của nó. <strong>Không thể hoàn tác.</strong> Giao dịch và dữ liệu token của bạn không bị ảnh hưởng.
positions-remove-confirm-archive = Lưu trữ vị thế

positions-management-changed = Đã đặt quản lý vị thế thành { $mode }
positions-details-load-failed = Không thể tải chi tiết vị thế
positions-details-mint-label = Địa chỉ mint
positions-details-management-failed = Không thể cập nhật quản lý vị thế
positions-details-favorite-add =
    .title = Thêm vào yêu thích
    .aria-label = Thêm vào yêu thích
positions-details-favorite-remove =
    .title = Xóa khỏi yêu thích
    .aria-label = Xóa khỏi yêu thích
positions-details-view-solscan =
    .title = Xem trên { -solscan }
    .aria-label = Xem token trên { -solscan }
positions-details-close =
    .title = Đóng (Esc)
    .aria-label = Đóng
positions-details-chart-section =
    .aria-label = Biểu đồ giá
positions-details-loading-chart = Đang tải biểu đồ...
positions-details-activity-section =
    .aria-label = Hoạt động
positions-details-activity-title = Hoạt động
positions-details-split-handle =
    .aria-label = Đổi kích thước biểu đồ và hoạt động
positions-details-activity-pane =
    .aria-label = Khung hoạt động
positions-details-activity-expand =
    .title = Mở rộng hoạt động
    .aria-label = Mở rộng hoạt động
positions-details-summary-section =
    .aria-label = Tóm tắt vị thế
positions-details-loading = Đang tải vị thế...

positions-management-auto-trader = Auto Trader
positions-management-user-only = Chỉ người dùng
positions-management-copy-task = Tác vụ copy
positions-management-hybrid = Kết hợp
positions-pane-show-chart = Hiện biểu đồ
positions-pane-show-activity = Hiện hoạt động
positions-pane-restore-activity = Khôi phục hoạt động
positions-pane-expand-chart =
    .title = Mở rộng biểu đồ
    .aria-label = Mở rộng biểu đồ

positions-risk-low = Rủi ro thấp
positions-risk-medium = Rủi ro trung bình
positions-risk-high = Rủi ro cao
positions-risk-unknown = Chưa rõ rủi ro
positions-busy-buying = Đang mua…
positions-busy-selling = Đang bán…
positions-busy-closing = Đang đóng…
positions-header-avg-entry = Vào TB
positions-header-buy-count =
    { $count ->
       *[other] { $count } lần mua
    }
positions-header-exit-price = Giá thoát
positions-header-closed-ago = đóng { $ago }
positions-header-realized-pnl = P&L đã chốt
positions-header-usd-note = USD theo giá { -sol } hôm nay
positions-header-returned = Thu về
positions-header-of-invested = trên { $amount } đã đầu tư
positions-header-price = Giá
positions-header-last-price = Giá gần nhất
positions-header-pool-ago = pool · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = P&L chưa chốt
positions-header-pnl-last-price = P&L theo giá gần nhất
positions-header-value = Giá trị
positions-header-last-value = Giá trị gần nhất
positions-header-invested = { $amount } đã đầu tư
positions-header-origin-hint = Cách vị thế này được mở
positions-header-risk-hint = Điểm { -rugcheck } - càng thấp càng an toàn
positions-header-frozen = Bị đóng băng
    .title = Quyền mint đã đóng băng số token này
positions-header-managed-by = Quản lý bởi
positions-header-management-select =
    .aria-label = Quản lý vị thế

positions-origin-unknown = không rõ
positions-origin-copied-task = Đã copy · tác vụ { $task }
positions-origin-manual-entry = Vào lệnh thủ công
positions-origin-wallet-entry = Vào lệnh từ ví
positions-origin-auto-strategy = Tự động · { $strategy }
positions-origin-auto-entry = Vào lệnh tự động

positions-pending-adding = Đang thêm
positions-pending-adding-amount = Đang thêm { $amount }
positions-pending-selling = Đang bán
positions-pending-selling-percent = Đang bán { $percent }
positions-pending-confirming = { $label } · đang xác nhận
    .title = Đã gửi và đang chờ xác nhận trên chuỗi. Các số liệu sẽ cập nhật sau khi được xác minh.

positions-trade-add = Thêm
    .title = Thêm vào vị thế
positions-trade-sell = Bán
    .title = Bán một phần vị thế
positions-trade-close = Đóng vị thế
    .title = Bán toàn bộ và đóng
positions-trade-token = Chi tiết token
    .title = Mở chi tiết token

positions-favorite-token-fallback = Token
positions-favorite-added = Đã thêm { $symbol } vào yêu thích
positions-favorite-removed = Đã xóa { $symbol } khỏi yêu thích
positions-favorite-add-failed = Không thể thêm vào yêu thích
positions-favorite-remove-failed = Không thể xóa khỏi yêu thích
positions-favorite-update-failed = Không thể cập nhật yêu thích

positions-summary-position = Vị thế
positions-summary-price-path = Diễn biến giá
positions-summary-network-fees = Phí mạng
positions-summary-risk = Rủi ro
positions-summary-market = Thị trường
positions-summary-market-now = Thị trường hiện tại
positions-summary-links = Liên kết
positions-fact-tokens-fallback = token
positions-fact-bought = Đã mua
positions-fact-holding = Đang nắm giữ
positions-fact-sold = Đã bán
positions-fact-realized = Đã chốt
positions-fact-opened = Đã mở
positions-fact-closed = Đã đóng
positions-fact-reason = Lý do
positions-fact-archived = Đã lưu trữ
positions-fact-entry = Vào lệnh
positions-fact-exit = Thoát lệnh
positions-fact-total = Tổng
positions-fact-verified = Đã xác minh trên chuỗi
positions-fact-confirming = Đang xác nhận
positions-fact-share-of-bought = { $percent } số đã mua
positions-fact-share-of-invested = { $percent } số đã đầu tư
positions-fact-entry-count =
    { $count ->
        [0] 1 lần vào lệnh
       *[other] 1 lần vào lệnh + { $count } lần thêm
    }
positions-fact-partial-exits-back =
    { $count ->
       *[other] { $count } lần thoát một phần · thu về { $returned }
    }
positions-fact-held = nắm giữ { $age }
positions-fact-vs-entry = { $percent } so với giá vào
positions-fact-exit-vs-peak = Thoát so với đỉnh
positions-fact-now-vs-peak = Hiện tại so với đỉnh
positions-fact-entry-range = Vùng vào lệnh
positions-range-low = Thấp
positions-range-peak = Đỉnh
positions-range-now = Hiện tại
positions-range-label-exit = Giá vào và giá thoát trong khoảng từ đáy đến đỉnh
positions-range-label-now = Giá vào và giá hiện tại trong khoảng từ đáy đến đỉnh
positions-fact-mint-authority = Quyền mint
positions-fact-freeze-authority = Quyền freeze
positions-fact-active = Đang hoạt động
positions-fact-pool = Pool
positions-fact-pool-liquidity = Thanh khoản { $amount } { -sol }
positions-fact-market-cap = Vốn hóa
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = Thanh khoản
positions-fact-volume-24h = Khối lượng 24h
positions-fact-price-change = Biến động giá
positions-change-period-1h = 1h
positions-change-period-24h = 24h
positions-fact-holders = Holder
positions-link-website = Website
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

positions-activity-load-failed = Không thể tải hoạt động
positions-activity-loading = Đang tải hoạt động...
positions-activity-empty = Chưa có gì xảy ra với token này trong ví này
positions-activity-filter-empty = Không có hoạt động nào khớp bộ lọc này
positions-activity-round-count =
    { $count ->
       *[other] { $count } vòng
    }
positions-activity-event-count =
    { $count ->
       *[other] { $count } sự kiện
    }
positions-activity-pending-count = Đang chờ: { $count }
positions-activity-failed-count = Thất bại: { $count }
positions-filter-all = Tất cả
positions-filter-trades = Giao dịch mua bán
positions-filter-buys = Mua
positions-filter-sells = Bán
positions-filter-wallet = Ví
positions-filter-issues = Sự cố
positions-activity-filters =
    .aria-label = Lọc hoạt động
positions-activity-totals =
    .aria-label = Tất cả vòng của token này
positions-activity-realized-all = Đã chốt, mọi vòng
positions-activity-invested = Đã đầu tư
positions-activity-returned = Thu về
positions-activity-opened = Mở { $when }
positions-activity-round-title = Vị thế { $index }
positions-activity-this-position = Vị thế này
positions-activity-dates-unavailable = Không có ngày
positions-activity-wallet-title = Giao dịch của ví
positions-activity-outside =
    { $count ->
       *[other] Ngoài mọi vị thế · { $range } · { $count } sự kiện
    }
positions-details-signature-label = Chữ ký

positions-state-open = Vị thế đang mở
positions-state-closing = Vị thế đang đóng
positions-state-closed = Vị thế đã đóng
positions-state-exit-pending = Vị thế đang chờ thoát
positions-state-exit-failed = Thoát vị thế thất bại
positions-state-phantom = Vị thế ảo
positions-state-reconciling = Vị thế đang đối soát

positions-event-kind-entry = Vào lệnh
positions-event-kind-dca = Lần thêm
positions-event-kind-partial-exit = Thoát một phần
positions-event-kind-exit = Thoát lệnh
positions-event-kind-buy = Ví mua
positions-event-kind-sell = Ví bán
positions-event-kind-transfer = Chuyển
positions-event-kind-ata = Tài khoản token
positions-event-kind-other = Giao dịch
positions-event-state-pending = Đang chờ
positions-event-state-failed = Thất bại
positions-event-state-synthetic = Tổng hợp
positions-chain-status-failed-detail = Thất bại: { $error }
positions-event-tokens-fallback = token
positions-event-entry-submitted = Đã gửi lệnh mua { $amount }
positions-event-entry-for = Đã mua { $amount } với giá { $sol }
positions-event-entry = Đã mua { $amount }
positions-event-dca-submitted = Đã gửi lệnh thêm { $amount }
positions-event-dca-for = Đã thêm { $amount } với giá { $sol }
positions-event-dca = Đã thêm { $amount }
positions-event-partial-exit-submitted-percent = Đã gửi lệnh thoát một phần { $percent } với { $amount }
positions-event-partial-exit-submitted = Đã gửi lệnh thoát một phần với { $amount }
positions-event-sold-percent-for = Đã bán { $amount } ({ $percent }) với giá { $sol }
positions-event-sold-percent = Đã bán { $amount } ({ $percent })
positions-event-sold-for = Đã bán { $amount } với giá { $sol }
positions-event-sold = Đã bán { $amount }
positions-event-exit-submitted = Đã gửi lệnh thoát toàn bộ vị thế
positions-event-exit-for = Đã đóng với { $amount } được bán với giá { $sol }
positions-event-exit-closed = Đã đóng vị thế
positions-event-wallet-bought = Ví đã mua { $amount } ở nơi khác
positions-event-wallet-sold = Ví đã bán { $amount } ở nơi khác
positions-event-received = Đã nhận { $amount }
positions-event-sent = Đã gửi { $amount }
positions-event-transferred = Đã chuyển { $amount }
positions-event-ata = Hoạt động tài khoản token
positions-event-wallet-transaction = Giao dịch của ví liên quan đến { $amount }
positions-event-price-per-token = { $price } { -sol } / token
positions-event-wallet-change = Ví thay đổi { $amount }
positions-event-after-title = Vị thế sau sự kiện này
positions-event-capital-invested = Vốn đã đầu tư
positions-event-average-entry = Giá vào trung bình
positions-event-transfers-title = Chuyển token
positions-event-transfer-amount = Số lượng
positions-event-transfer-mint = Mint
positions-event-transfer-from = Từ
positions-event-transfer-to = Đến
positions-event-no-signature = Không có chữ ký trên chuỗi
positions-event-click-to-copy = Nhấp để sao chép
positions-event-solscan = { -solscan }
positions-event-token-amount = Số lượng token
positions-event-trade-price = Giá giao dịch
positions-event-native-amount = Số lượng { -sol }
positions-event-cost-basis = Giá vốn
positions-event-usd-value = Giá trị USD
positions-event-network-fee = Phí mạng
positions-event-router = Router
positions-event-slot = Slot
positions-event-chain-status = Trạng thái chuỗi
positions-event-transaction-type = Loại giao dịch
positions-event-direction = Hướng
positions-event-wallet-native-change = Thay đổi { -sol } của ví
positions-event-instructions = Instruction
positions-event-compute-units = Compute unit
positions-event-accounts = Tài khoản
positions-event-record-id = ID bản ghi
positions-event-time-unavailable = Không có thời gian
positions-event-details = Chi tiết
positions-event-hide-details = Ẩn chi tiết

positions-chart-type-candles = Nến
positions-chart-type-line = Đường
positions-chart-type-area = Vùng
positions-chart-type-group =
    .aria-label = Loại biểu đồ
positions-chart-overlays-group =
    .aria-label = Lớp phủ biểu đồ
positions-chart-ema = EMA
    .title = Đường trung bình động hàm mũ, 9 và 21
positions-chart-fit = Vừa khung
    .title = Căn khung theo vòng đời của vị thế này
positions-chart-timeframes-group =
    .aria-label = Khung thời gian
positions-chart-pane-group =
    .aria-label = Khung biểu đồ
positions-chart-unavailable = Công cụ biểu đồ không khả dụng
positions-chart-collecting = Đang thu thập dữ liệu biểu đồ…
positions-chart-no-data = Chưa có dữ liệu biểu đồ cho token này
positions-chart-avg-entry = Vào TB
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = Vào TB
positions-chart-legend-avg-entry-off-scale = Vào TB (ngoài thang)
positions-chart-dropped-events =
    { $count ->
       *[other] { $count } sự kiện không có nến ở khung thời gian này
    }
positions-chart-level = Mức
positions-chart-level-above = { $label } { $price } nằm trên vùng đang xem
positions-chart-level-below = { $label } { $price } nằm dưới vùng đang xem
positions-chart-scale-hint = Kéo trục giá để thu phóng tới mức đó
positions-chart-pnl-at-bar = P&L tại thanh nến
positions-chart-click-to-locate = Nhấp để định vị
