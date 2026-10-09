copy-skip-not-buy-swap = Hoạt động của ví không phải lệnh mua
copy-skip-task-disabled = Tác vụ đang tạm dừng
copy-skip-mode-transition-required = Chế độ thực thi phải được thay đổi riêng
copy-skip-live-confirmation-required = Thực thi live cần được xác nhận
copy-skip-unsupported-sizing-mode = Chế độ xác định quy mô chưa được hỗ trợ
copy-skip-self-copy = Ví này là một trong các ví của bạn
copy-skip-target-below-minimum = Giao dịch của ví thấp hơn mức tối thiểu
copy-skip-target-above-maximum = Giao dịch của ví vượt mức tối đa
copy-skip-already-bought = Đã mua token này (chỉ mua một lần)
copy-skip-blacklisted = Token bị chặn bởi kiểm soát rủi ro
copy-skip-filter-required = Token chưa đạt bộ lọc
copy-skip-budget-exhausted = Ngân sách của tác vụ đã hết
copy-skip-token-cap-reached = Đã đạt giới hạn cho mỗi token
copy-skip-below-minimum-size = Quy mô copy quá nhỏ
copy-skip-invalid-sizing = Quy mô của tác vụ không hợp lệ
copy-skip-invalid-slippage = Trượt giá của tác vụ không hợp lệ
copy-skip-invalid-exit-policy = Quy tắc thoát của tác vụ không hợp lệ
copy-skip-invalid-price = Không có giá thị trường khả dụng
copy-skip-not-sell-swap = Hoạt động của ví không phải lệnh bán
copy-skip-exit-mode-disabled = Bỏ qua lệnh bán của ví: tác vụ bán theo quy tắc riêng
copy-skip-force-stopped = Giao dịch đang bị dừng cưỡng chế
copy-skip-copy-position-not-found = Không có vị thế nào thuộc tác vụ này
copy-skip-position-user-only = Vị thế do bạn quản lý
copy-skip-position-management-mismatch = Vị thế không còn theo lệnh bán copy
copy-skip-latency-kill-switch = Tự động tạm dừng: phát hiện giao dịch quá muộn
copy-skip-claim-reconciled-abandoned = Lệnh live bị gián đoạn đã được đóng, không thử lại
copy-skip-stale-observation = Phát lại sau thời gian ngừng hoạt động, quá cũ để copy
copy-skip-unknown-observation-time = Giao dịch phát lại không có thời gian block
copy-skip-entry-blocked = Lệnh vào bị chặn

copy-entry-block-force-stopped = Giao dịch đang bị dừng cưỡng chế
copy-entry-block-loss-limit = Giới hạn lỗ đang chặn các lệnh vào mới
copy-entry-block-connectivity = Các dịch vụ cần thiết không khả dụng
copy-entry-block-position-limit = Đã đạt giới hạn vị thế đang mở
copy-entry-block-already-open = Đã có một vị thế đang mở
copy-entry-block-reentry-cooldown = Thời gian chờ vào lại token
copy-entry-block-open-cooldown = Thời gian chờ vào lệnh chung
copy-entry-block-entry-reserved = Một lệnh vào khác đang được xử lý
copy-entry-block-blacklisted = Token bị chặn bởi kiểm soát rủi ro
copy-entry-block-check-failed = Không thể hoàn tất một bước kiểm tra an toàn

copy-pause-user = Bạn đã tạm dừng
copy-pause-latency-kill-switch = Tự động tạm dừng: giao dịch đến trễ trung bình { $average } giây (giới hạn { $threshold } giây)
copy-pause-watch-detached = Tự động tạm dừng: ví không còn được theo dõi
copy-pause-watch-budget-exceeded = Đã tạm dừng: ví này đạt giới hạn { $limit } chữ ký cho mỗi lần kiểm tra theo dõi trước khi bắt kịp
copy-pause-helius-unavailable = Đã tạm dừng: kiểm tra ví qua { -helius } thất bại
copy-pause-watch-processing-failed = Đã tạm dừng: không thể xử lý hoạt động của ví
copy-pause-unspecified = Đã tạm dừng

copy-pause-short-user = do bạn
copy-pause-short-latency-kill-switch = quá chậm
copy-pause-short-watch-detached = mất theo dõi
copy-pause-short-watch-budget-exceeded = giới hạn theo dõi
copy-pause-short-helius-unavailable = nhà cung cấp theo dõi
copy-pause-short-watch-processing-failed = xử lý theo dõi
copy-state-paused = Đã tạm dừng
copy-state-paused-reason = Đã tạm dừng · { $reason }

copy-readiness-history = Lịch sử thử nghiệm
copy-readiness-history-met =
    { $count ->
       *[other] { $count } vòng thử nghiệm đã đóng, cần { $needed }
    }
copy-readiness-history-short = { $count }/{ $needed } vòng thử nghiệm đã đóng
copy-readiness-profit = Có lãi khi thử nghiệm
copy-readiness-profit-detail =
    { $count ->
       *[other] Đã chốt { $realized } { -sol } qua { $count } vòng, thắng { $wins } vòng
    }
copy-readiness-latency = Giao dịch được phát hiện kịp thời
copy-readiness-latency-detail = Độ trễ p95 { $p95 } giây, giới hạn { $limit } giây
copy-readiness-latency-none = Chưa có mẫu thời gian đến
copy-readiness-priced = Mọi khoản nắm giữ đều có giá
copy-readiness-priced-ok = Mọi khoản nắm giữ thử nghiệm đang mở đều có giá pool
copy-readiness-priced-missing =
    { $count ->
       *[other] { $count } khoản nắm giữ đang mở không có giá pool
    }
copy-readiness-runtime = Có thể thực thi live
copy-readiness-runtime-ok = Thiết lập và các chốt an toàn cho phép copy live

copy-live-block-setup-incomplete = Hãy hoàn tất thiết lập ví và RPC trước
copy-live-block-force-stop = Dừng khẩn cấp đang được bật
copy-live-block-copy-trading-disabled = Xử lý copy đang bị tạm dừng toàn cục
copy-live-block-unavailable = Không thể thực thi live

copy-state-system-paused = Đã tạm dừng toàn cục
copy-state-force-stopped = Đã dừng cưỡng chế
copy-state-entries-blocked = Lệnh vào bị chặn
copy-state-running-live = Đang chạy
copy-state-running-paper = Đang chạy
copy-mode-paper = Thử nghiệm
copy-mode-live = Live
copy-exit-mode-buy-only = Quy tắc thoát của tôi
copy-exit-mode-mirror = Sao chép lệnh bán của ví
copy-exit-mode-hybrid = Lệnh bán của ví và quy tắc của tôi
copy-exit-target-sell = Ví đã bán
copy-exit-stop-loss = Stop loss
copy-exit-trailing-stop = Trailing stop
copy-exit-take-profit = Take profit
copy-exit-time-override = Quy tắc thời gian
copy-exit-manual = Đóng thủ công

copy-request-failed = Yêu cầu thất bại
copy-keep-paused = Giữ tạm dừng
copy-paused-suffix = · đã tạm dừng
copy-mode-paused = { $mode } · đã tạm dừng
copy-task-ref = “{ $name }” ({ $mode })
copy-metric-realized-pnl = P&L đã chốt
copy-metric-unrealized-pnl = P&L chưa chốt
copy-metric-win-rate = Tỷ lệ thắng
copy-metric-budget-spent = Ngân sách đã chi
copy-metric-median-arrival = Thời gian đến trung vị
copy-metric-open-holdings = Khoản nắm giữ đang mở
copy-record-won-lost = { $won } thắng · { $lost } thua
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = Lệnh khớp
copy-kind-exits = Lệnh thoát
copy-kind-skips = Bỏ qua
copy-kind-errors = Lỗi
copy-field-per-trade-cap = Giới hạn mỗi giao dịch
copy-field-per-token-cap = Giới hạn mỗi token
copy-field-total-budget = Tổng ngân sách
copy-field-slippage = Trượt giá
copy-rules-wallet-sells-only = Chỉ theo lệnh bán của ví
copy-filter-copy-setting-required = Cài đặt copy (bắt buộc)
copy-filter-copy-setting-not-required = Cài đặt copy (không bắt buộc)
copy-count-closed-rounds =
    { $count ->
       *[other] { $count } vòng đã đóng
    }
copy-count-open-holdings =
    { $count ->
       *[other] { $count } khoản nắm giữ đang mở
    }
copy-unrealized-partial =
    { $priced ->
       *[other] { $priced } khoản nắm giữ có giá · { $unpriced } không có giá
    }
copy-unrealized-unpriced =
    { $count ->
       *[other] { $count } khoản nắm giữ không có giá
    }
copy-range-24h = 24h
copy-range-7d = 7d
copy-range-30d = 30d
copy-range-all = Tất cả
copy-range-label =
    .aria-label = Khoảng ngày

copy-page-title = Copy trading
copy-page-beta = Beta
copy-strip-loading = Đang tải
copy-strip-unavailable = Không khả dụng
copy-strip-setup-required = Cần thiết lập · giao dịch sao chép cần ví và RPC
copy-strip-pause-all = Tạm dừng tất cả
copy-strip-resume = Tiếp tục xử lý
copy-strip-settings = Cài đặt
copy-strip-add-wallet = Thêm ví
copy-strip-paused-globally = Đã tạm dừng toàn cục · không có lệnh copy mới, lệnh thoát vẫn chạy
copy-strip-force-stopped = Đã dừng cưỡng chế · không copy gì cả
copy-strip-loss-limit = Giới hạn lỗ · lệnh vào mới bị chặn, lệnh thoát vẫn chạy
copy-strip-idle-paused =
    { $count ->
       *[other] Chờ · { $count } tác vụ đã tạm dừng
    }
copy-strip-idle-empty = Chờ · chưa có tác vụ nào
copy-strip-processing = Đang xử lý · { $paper } thử nghiệm
copy-strip-processing-live = Đang xử lý · { $live } live · { $paper } thử nghiệm
copy-figures-label =
    .aria-label = Tổng số copy trading
copy-figure-marked-at-pool = Định giá theo giá pool
copy-figure-across-tasks = Trên tất cả tác vụ
copy-figure-budget-lifetime = Tổng chi tiêu của các tác vụ đang bật
copy-figure-budget-none = Không có tác vụ nào đang bật
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
       *[other] { $count } giao dịch
    }
copy-figure-arrival-none = Không có mẫu từ các tác vụ đang bật

copy-load-failed = Không thể tải copy trading: { $error }
copy-resume-all-title = Tiếp tục xử lý copy
copy-resume-all-message =
    { $count ->
       *[other] { $count } tác vụ live sẽ gửi swap thật khi ví của chúng giao dịch lần nữa.
    }
copy-toast-resumed-all = Đã tiếp tục xử lý copy
copy-toast-paused-all = Đã tạm dừng toàn bộ xử lý copy
copy-toast-global-failed = Không thể thay đổi xử lý copy

copy-onboarding-title = Copy những ví bạn tin tưởng, kiểm chứng chúng ở chế độ thử nghiệm trước
copy-onboarding-body = Mọi tác vụ đều bắt đầu ở chế độ thử nghiệm: giao dịch của ví mục tiêu được mô phỏng theo giá pool với trượt giá và phí của bạn, còn quy tắc thoát của bạn chạy trên sổ thử nghiệm. Kích hoạt chạy thật cho từng ví khi kết quả thử nghiệm đủ tốt.
copy-onboarding-add = Thêm ví đầu tiên
copy-setup-gate-title = Giao dịch sao chép cần một ví
copy-onboarding-observe = Quan sát
copy-onboarding-observe-detail = Phát hiện các swap của ví mà không tốn { -sol }.
copy-onboarding-evaluate = Đánh giá
copy-onboarding-evaluate-detail = Xem P&L thử nghiệm, tỷ lệ thắng, các lệnh bỏ qua, tốc độ phát hiện và trượt giá.
copy-onboarding-arm = Kích hoạt
copy-onboarding-arm-detail = Vượt qua các bước kiểm tra sẵn sàng, rồi bật swap thật.

copy-list-label =
    .aria-label = Ví đang copy
copy-list-title = Ví
copy-list-compare = So sánh
copy-list-sort-label = Sắp xếp ví
copy-list-count = { $active } đang hoạt động · { $total } tổng
copy-sort-pnl = P&L
copy-sort-state = Trạng thái
copy-sort-name = Tên
copy-compare-label =
    .aria-label = So sánh ví

copy-dialog-close =
    .aria-label = Đóng
copy-editor-title-add = Thêm ví
copy-editor-sub-add = Tác vụ mới bắt đầu ở chế độ thử nghiệm
copy-arm-title = Kích hoạt copy live
copy-arm-sub = Swap thật từ ví của bạn
copy-arm-keep-paper = Giữ thử nghiệm
copy-arm-confirm = Kích hoạt chạy thật
copy-profile-title = Hồ sơ ví
copy-profile-sub = Những gì bot này đã thấy về ví

copy-settings-title = Cài đặt copy trading
copy-settings-subtitle = Chính sách chung cho mọi tác vụ
copy-settings-filter-warning = Với thiết lập Lọc mặc định, tùy chọn này loại gần như mọi token nên sẽ không copy gì cả. Hãy để tắt trừ khi bộ lọc của bạn cho qua các token mà ví giao dịch.
copy-settings-unit-seconds = giây
copy-settings-unit-trades = giao dịch
copy-settings-unit-tasks = tác vụ
copy-settings-unit-closed-rounds = vòng đã đóng
copy-settings-save = Lưu cài đặt
copy-settings-load-failed = Không thể tải cài đặt copy
copy-settings-saved = Đã lưu cài đặt copy trading

copy-tab-overview = Tổng quan
copy-tab-holdings = Nắm giữ
copy-tab-activity = Hoạt động
copy-tab-rules = Quy tắc
copy-tab-execution = Thực thi
copy-tabs-label = Chế độ xem tác vụ
copy-workspace-select = Chọn một ví để mở không gian làm việc của nó.
copy-workspace-loading = Đang tải tác vụ…
copy-workspace-load-failed = Không thể tải tác vụ này: { $error }

copy-state-detail-paper = Đang chạy ở chế độ thử nghiệm · giao dịch được mô phỏng, không tốn tiền
copy-state-detail-live = Đang chạy live · giao dịch của ví được copy bằng swap thật
copy-state-detail-system-paused = Đang chờ · xử lý copy bị tạm dừng toàn cục, lệnh thoát vẫn chạy
copy-state-detail-entries-blocked = Lệnh vào bị chặn bởi giới hạn lỗ · lệnh thoát vẫn chạy
copy-state-detail-force-stopped = Đã dừng cưỡng chế · không copy gì cả

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = Tiếp tục sẽ giữ nguyên giới hạn, nên tác vụ sẽ lại tạm dừng nếu giao dịch vẫn đến trễ. Hãy kiểm tra luồng RPC hoặc tăng giới hạn thời gian đến trong Cài đặt.
copy-paused-resume-detached = Tiếp tục sẽ theo dõi ví trở lại.
copy-paused-holdings-rules =
    { $count ->
       *[other] Quy tắc thoát của tác vụ vẫn đóng { $count } khoản nắm giữ đang mở của nó.
    }
copy-paused-holdings-mirror =
    { $count ->
       *[other] Lệnh bán của ví vẫn đóng { $count } khoản nắm giữ đang mở của tác vụ.
    }
copy-paused-holdings-hybrid =
    { $count ->
       *[other] Lệnh bán của ví và quy tắc thoát của tác vụ vẫn đóng { $count } khoản nắm giữ đang mở của nó.
    }

copy-watch-state-catching-up = Theo dõi ví: Đang bắt kịp. Đang kiểm tra ví này qua { -helius }.
copy-watch-state-watching = Theo dõi ví: Đang theo dõi. Đang kiểm tra ví này qua { -helius }.
copy-watch-last-check = Kiểm tra lần cuối { $ago }.
copy-watch-recovery-active = Theo dõi ví đang hoạt động
copy-watch-recovery-catching-up = Theo dõi ví đang bắt kịp
copy-watch-recovery-still-paused = Tác vụ copy vẫn đang tạm dừng. Hãy tiếp tục copy khi bạn sẵn sàng.
copy-watch-recovery-title = Khôi phục theo dõi ví
copy-watch-recovery-processing-failed = Không thể xử lý hoạt động của ví. Tiến độ đã lưu được giữ nguyên. Hãy thử lại sau khi sự cố được khắc phục.
copy-watch-recovery-provider-failed = Kiểm tra qua { -helius } thất bại. Tiến độ đã lưu được giữ nguyên. Hãy thử lại khi nhà cung cấp khả dụng.
copy-watch-recovery-budget-intro = Ví này có nhiều hoạt động hơn mức theo dõi hiện tại có thể kiểm tra. Hãy chọn cách tiếp tục.
copy-watch-approve = Thử bắt kịp bằng { -helius }
copy-watch-approve-help = Tiếp tục từ tiến độ đã lưu. Có thể tốn thêm credit { -helius } và vẫn có thể bị chậm.
copy-watch-approve-unavailable = Không thể bắt kịp qua { -helius }. Hãy cấu hình một endpoint RPC { -helius } đang bật để tiếp tục mà không bỏ qua hoạt động chưa kiểm tra.
copy-watch-no-provider = Không có nhà cung cấp bắt kịp nào được hỗ trợ cho lượt theo dõi này.
copy-watch-budget-label = Số chữ ký kiểm tra mỗi lần
copy-watch-budget-hint = Hoặc bỏ qua hoạt động chưa kiểm tra và tiếp tục từ bây giờ. Chọn từ { $min } đến { $max } chữ ký mỗi lần kiểm tra; giới hạn cao hơn có thể dùng nhiều lệnh gọi RPC hơn.
copy-watch-ack = Tôi hiểu rằng hoạt động bị bỏ lỡ sẽ không được copy.
copy-watch-toast-range = Chọn từ { $min } đến { $max } chữ ký mỗi lần thăm dò, theo bước { $step } chữ ký
copy-watch-toast-ack = Hãy xác nhận rằng các chữ ký kể từ lần kiểm tra hoàn tất gần nhất sẽ bị bỏ qua
copy-watch-resumed = Đã tiếp tục theo dõi ví từ bây giờ; tác vụ copy vẫn đang tạm dừng
copy-watch-resume-failed = Không thể tiếp tục theo dõi ví
copy-watch-retry-started = Đã bắt đầu thử lại theo dõi ví từ tiến độ đã lưu; tác vụ copy vẫn đang tạm dừng
copy-watch-retry-failed = Không thể thử lại theo dõi ví
copy-watch-approve-title = Cho phép bắt kịp qua { -helius } cho ví này
copy-watch-approve-message = { -helius } có thể kiểm tra các giao dịch Solana thành công từ tiến độ đã lưu mà không bỏ qua khoảng chưa kiểm tra. Hiện tại phí là 10 credit cho mỗi 100 giao dịch đầy đủ trả về, làm tròn lên, tối thiểu 10 credit mỗi yêu cầu. Một lần kiểm tra có thể gồm nhiều yêu cầu; mức sử dụng và giá của nhà cung cấp có thể thay đổi. Copy vẫn tạm dừng cho đến khi bạn tiếp tục riêng.
copy-watch-approve-confirm = Cho phép cho ví này
copy-watch-approved = Đã bắt đầu theo dõi ví từ tiến độ đã lưu; tác vụ copy vẫn đang tạm dừng
copy-watch-restore-failed = Không thể khôi phục theo dõi ví

copy-action-pause = Tạm dừng
copy-action-resume = Tiếp tục
copy-action-resume-copy = Tiếp tục copy
copy-action-resume-from-now = Tiếp tục từ bây giờ
copy-action-retry-watch = Thử lại theo dõi ví
copy-action-return-paper = Quay lại thử nghiệm
copy-action-edit-rules = Sửa quy tắc
copy-action-clone = Nhân bản
copy-action-profile = Hồ sơ ví
copy-resume-live-title = Tiếp tục copy live
copy-resume-live-message = “{ $name }” sẽ gửi swap thật từ ví của bạn khi ví này giao dịch lần nữa.
copy-resume-live-confirm = Tiếp tục live
copy-task-resumed = Đã tiếp tục tác vụ
copy-task-paused = Đã tạm dừng tác vụ
copy-task-state-failed = Không thể thay đổi trạng thái tác vụ
copy-return-paper-message = Các lệnh copy mới của “{ $name }” sẽ lại được mô phỏng, không tốn { -sol }.
copy-return-paper-cancel = Giữ live
copy-task-returned-paper = Tác vụ đã quay lại thử nghiệm
copy-mode-change-failed = Không thể thay đổi chế độ thực thi
copy-delete-title = Xóa tác vụ copy
copy-delete-message = Xóa “{ $name }”? Các quyết định và kết quả thử nghiệm của nó sẽ bị xóa và ví sẽ không còn được theo dõi cho tác vụ này.
copy-delete-confirm = Xóa tác vụ
copy-delete-cancel = Giữ tác vụ
copy-task-deleted = Đã xóa tác vụ copy
copy-task-delete-failed = Không thể xóa tác vụ copy

copy-overview-results = Kết quả
copy-analytics-load-failed = Không thể tải phân tích: { $error }
copy-analytics-loading = Đang tải phân tích…
copy-exit-bucket =
    { $count ->
       *[other] { $count } lệnh bán · { $pnl }
    }
copy-overview-average-win = Lãi trung bình
copy-overview-average-loss = Lỗ trung bình { $amount }
copy-overview-profit-factor = Hệ số lợi nhuận
copy-overview-profit-factor-note = Tổng lãi ÷ tổng lỗ
copy-overview-average-hold = Thời gian nắm giữ trung bình
copy-overview-average-hold-note = Từ lúc vào đến lúc thoát
copy-overview-best-round = Vòng tốt nhất
copy-overview-worst-round = Kém nhất { $amount }
copy-overview-curve-title = P&L lũy kế
copy-overview-exits-title = Lệnh bán theo cách thoát
copy-overview-skips-title = Lý do bỏ qua giao dịch
copy-book-title-live = Sổ live
copy-book-title-paper = Sổ thử nghiệm
copy-book-all-time = Từ trước đến nay
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
       *[other] lệnh mua
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
       *[other] lệnh thoát theo quy tắc của bạn
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
       *[other] lệnh bán của ví
    }
copy-book-manual-closes = <strong>{ $count }</strong> đóng thủ công
copy-book-skipped = <strong>{ $count }</strong> bỏ qua
copy-book-failed = <strong>{ $count }</strong> thất bại
copy-book-closed = { $count } đã đóng
copy-book-budget-note = Chi { $mode } { $total } · còn { $remaining }
copy-check-passed = đạt
copy-check-not-passed = chưa đạt
copy-readiness-title = Trước khi chạy live
copy-readiness-live-note = Tác vụ này đang giao dịch live. Hãy đưa nó về thử nghiệm từ phần đầu trang ở trên.
copy-readiness-all-pass = Mọi bước kiểm tra đều đạt.
copy-readiness-needs-review = Kích hoạt cần xem xét rõ ràng những mục chưa sẵn sàng.
copy-readiness-arm = Xem xét và kích hoạt live

copy-rules-title = Quy tắc đang áp dụng
copy-rules-size-ratio = { $pct } giao dịch của ví
copy-rules-size-fixed = { $amount } mỗi lần copy
copy-rules-target-any = Mọi quy mô
copy-rules-target-min = Tối thiểu { $amount }
copy-rules-target-max = Tối đa { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = Ghi đè của tác vụ · Trader { $value }
copy-rules-source-default = Mặc định của Trader
copy-rules-not-used = Không dùng: lệnh bán của ví quyết định
copy-rules-col-rule = Quy tắc
copy-rules-col-applies = Áp dụng
copy-rules-col-source = Nguồn
copy-rules-budget-note = Đã chi { $spent } ở { $mode } · còn { $remaining }
copy-rules-token-copies =
    { $count ->
       *[other] Khoảng { $count } lần copy đầy đủ cho một token
    }
copy-rules-sizing = Quy mô
copy-rules-copy-size = Quy mô copy
copy-rules-entry-filters = Bộ lọc điểm vào
copy-rules-target-size = Quy mô giao dịch của ví
copy-rules-repeat-buys = Mua lặp lại
copy-rules-repeat-first-only = Chỉ lần mua đầu tiên của mỗi token
copy-rules-repeat-every = Mọi lần mua, tối đa đến giới hạn mỗi token
copy-rules-filter-pass = Đạt bộ lọc
copy-rules-filter-required = Bắt buộc
copy-rules-filter-not-required = Không bắt buộc
copy-rules-filter-task-override = Ghi đè của tác vụ
copy-rules-exits = Lệnh thoát
copy-rules-exits-inactive = Các khoản nắm giữ chỉ được bán khi ví bán; các quy tắc bên dưới không chạy ở chế độ này.

copy-rule-status = Trạng thái
copy-rule-on = Bật
copy-rule-off = Tắt
copy-rule-unit-seconds = giây
copy-rule-unit-minutes = phút
copy-rule-stop-loss-threshold = Bán khi lỗ đến
copy-rule-stop-loss-min-hold = Không trước khi nắm giữ
copy-rule-no-minimum = Không có mức tối thiểu
copy-rule-partial-exits = Thoát một phần
copy-rule-partial-allowed = Cho phép
copy-rule-partial-full-only = Chỉ thoát toàn bộ
copy-rule-partial-size = Quy mô thoát một phần
copy-rule-trailing-activation = Kích hoạt khi lãi đến
copy-rule-trailing-distance = Bán khi thấp hơn đỉnh
copy-rule-take-profit-target = Bán khi lãi đến
copy-rule-time-duration = Kiểm tra sau khi nắm giữ
copy-rule-time-threshold = Bán khi P&L bằng hoặc thấp hơn
copy-preset-inherit = Mặc định của Trader
copy-preset-conservative = Thận trọng
copy-preset-balanced = Cân bằng
copy-preset-aggressive = Mạnh tay
copy-preset-custom = Tùy chỉnh
copy-validate-stop-loss = Stop loss phải lớn hơn 0% và tối đa 100%.
copy-validate-partial-size = Quy mô thoát một phần phải nằm trong khoảng 0% đến 100%.
copy-validate-min-hold = Thời gian nắm giữ tối thiểu phải là số giây nguyên.
copy-validate-trailing-activation = Ngưỡng kích hoạt trailing phải lớn hơn 0% và tối đa 100%.
copy-validate-trailing-distance = Khoảng cách trailing phải lớn hơn 0% và tối đa 100%.
copy-validate-take-profit = Take profit phải lớn hơn 0%.
copy-validate-time-duration = Quy tắc thời gian cần khoảng thời gian lớn hơn không.
copy-validate-time-threshold = Ngưỡng của quy tắc thời gian là một khoản lỗ: hãy dùng 0% hoặc số âm.
copy-warning-mirror = Chỉ lệnh bán của ví mới đóng các khoản nắm giữ: không có stop loss bảo vệ, và token mà ví không bao giờ bán sẽ được giữ mãi.
copy-warning-no-rules = Không có quy tắc thoát nào đang bật và lệnh bán của ví bị bỏ qua: các khoản nắm giữ sẽ không bao giờ được bán.
copy-warning-no-stop-loss = Không có stop loss nào áp dụng: token đang giảm sẽ được giữ cho đến khi một quy tắc khác hoặc ví bán.
copy-warning-stop-delay = Stop loss chờ { $hold } sau mỗi lần mua: token giảm nhanh hơn sẽ đóng ở mức thấp hơn nhiều so với { $threshold }.
copy-warning-take-profit-cost = Take profit tại { $target } không đủ bù chi phí bán (trượt giá { $slippage } và phí swap { $fee }), nên các vòng được đóng ở mức lỗ.
copy-warning-trailing-distance = Khoảng cách trailing bằng hoặc lớn hơn mức lãi kích hoạt, nên trailing đã kích hoạt có thể bán dưới giá vào.

copy-execution-title = Chất lượng thực thi
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = Bất kỳ
copy-execution-limit-on =
    { $count ->
       *[other] Tạm dừng khi trung bình trên { $limit } qua { $count } giao dịch
    }
copy-execution-limit-off = Công tắc ngắt khẩn cấp đang tắt
copy-execution-arrival-samples =
    { $count ->
       *[other] { $count } giao dịch được thấy ngay khi xảy ra
    }
copy-execution-p95 = Thời gian đến p95
copy-execution-median-slippage = Trượt giá trung vị
copy-execution-slippage-samples =
    { $count ->
       *[other] { $count } lệnh khớp đã đo
    }
copy-execution-worst-slippage = Trượt giá tệ nhất
copy-execution-average-slippage = Trung bình { $amount }
copy-execution-delay-title = Độ trễ phát hiện
copy-execution-delay-note = Thời gian từ block của ví đến khi bot này thấy giao dịch. Không tính các lần phát lại sau thời gian ngừng hoạt động.
copy-execution-delay-limit = Các cột vượt giới hạn thời gian đến { $limit } có màu hổ phách.
copy-execution-fastest = Nhanh nhất
copy-execution-average = Trung bình
copy-execution-slowest = Chậm nhất
copy-execution-fill-title = Khớp lệnh so với ví
copy-execution-fill-note = Số dương nghĩa là kém hơn ví: trả nhiều hơn khi mua, nhận ít hơn khi bán theo ví. Lệnh khớp thử nghiệm của token không có giá pool được định giá theo giao dịch của chính ví nên không đo được gì và bị loại ra.
copy-execution-samples = Mẫu
copy-execution-median = Trung vị
copy-execution-worst = Tệ nhất
copy-execution-decisions = Quyết định trong khoảng

copy-compare-title = So sánh ví
copy-compare-back = Quay lại ví
copy-compare-load-failed = Không thể tải phần so sánh: { $error }
copy-compare-loading = Đang tải phần so sánh…
copy-compare-empty = Không có tác vụ nào để so sánh.
copy-compare-empty-message = Thêm một tác vụ sao chép để so sánh kết quả với các tác vụ khác.
copy-compare-curve-title = P&L đã chốt lũy kế
copy-table-wallet = Ví
copy-table-mode = Chế độ
copy-table-rounds = Vòng
copy-table-realized = Đã chốt
copy-table-profit-factor = Hệ số lợi nhuận
copy-table-average-hold = Nắm giữ TB
copy-table-median-slippage = Trượt giá trung vị

copy-chart-curve-label = P&L lũy kế { $amount } { -sol }
copy-chart-compare-label = P&L lũy kế theo tác vụ
copy-chart-empty-curve = Chưa có vòng nào đã đóng trong khoảng này.
copy-chart-empty-bars = Không có gì được ghi nhận trong khoảng này.
copy-chart-empty-histogram = Không có mẫu thời gian đến trong khoảng này.
copy-chart-empty-compare = Không có vòng đã đóng nào để so sánh trong khoảng này.
copy-chart-histogram-title = { $count }/{ $total }

copy-profile-copy = Copy ví này
copy-profile-copy-other = Copy với quy tắc khác
copy-profile-loading = Đang tải hồ sơ ví…
copy-profile-watch-title = Theo dõi
copy-profile-watched = Đang theo dõi
copy-profile-watch-resume-hint = Tiếp tục một tác vụ sẽ theo dõi lại ví
copy-profile-watch-add-hint = Thêm một tác vụ sẽ bắt đầu theo dõi ví
copy-profile-stream = Luồng
copy-profile-subscribed = Đã đăng ký
copy-profile-not-subscribed = Chưa đăng ký
copy-profile-sources =
    { $count ->
       *[other] { $count } nguồn
    }
copy-profile-last-activity = Hoạt động gần nhất
copy-profile-last-error = Lỗi gần nhất
copy-profile-own-wallet = Đây là một trong các ví của bạn; không thể copy ví này.
copy-profile-observed-title = Giao dịch đã quan sát
copy-profile-observed-none = Bot này chưa có giao dịch nào từ ví này. Một tác vụ thử nghiệm sẽ quan sát ví mà không tốn { -sol }.
copy-profile-swaps-seen = Swap đã thấy
copy-profile-swaps-seen-note = Số swap riêng biệt của ví trên các tác vụ của bạn
copy-profile-buys-sells = Mua / bán
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = Token đã giao dịch
copy-profile-first-seen = Thấy lần đầu
copy-profile-last-seen = Thấy lần cuối
copy-profile-tasks-title = Các tác vụ của bạn trên ví này
copy-table-task = Tác vụ

copy-arm-acks-left =
    { $count ->
       *[other] Còn { $count } mục xác nhận cần đánh dấu
    }
copy-arm-readiness-title = Mức sẵn sàng từ sổ thử nghiệm
copy-arm-exposure-title = Mức phơi nhiễm
copy-arm-per-copy = Mỗi lần copy
copy-arm-budget-left-value = { $left }/{ $total } { -sol }
copy-arm-budget-left = Ngân sách live còn lại
copy-arm-budget-left-note = Chi tiêu thử nghiệm được tính riêng và không dùng ngân sách này
copy-arm-exits = Lệnh thoát
copy-arm-stop-note = Không trước khi nắm giữ { $hold }: giảm nhanh hơn sẽ đóng ở mức thấp hơn
copy-arm-shared = Ví này cũng được copy bởi { $tasks }: mỗi tác vụ copy giao dịch của nó bằng ngân sách riêng.
copy-arm-unavailable = Hiện không thể thực thi live; xem lần kiểm tra gần nhất.
copy-arm-ack-real-native = { -sol } thật: tác vụ này có thể chi tối đa { $budget } { -sol } từ ví của bạn, tối đa { $trade } { -sol } mỗi lần copy.
copy-arm-ack-fees = Các lệnh copy live phải trả phí mạng và trượt giá thật; kết quả thử nghiệm không đảm bảo kết quả live.
copy-arm-ack-unready = Một số bước kiểm tra sẵn sàng chưa đạt. Vẫn kích hoạt tác vụ này.
copy-arm-lead = “{ $name }” sẽ copy giao dịch của ví này bằng swap thật từ ví của bạn.
copy-arm-confirmation-missing = Không thể tải xác nhận live
copy-arm-armed = Đã kích hoạt copy live
copy-arm-failed = Không thể kích hoạt copy live

copy-holdings-title = Nắm giữ
copy-holdings-view-label = Chế độ xem nắm giữ
copy-holdings-view-open = Đang mở ({ $count })
copy-holdings-view-closed = Vòng đã đóng ({ $count })
copy-holdings-reset = Đặt lại sổ thử nghiệm
copy-holdings-live-note = Các lệnh copy live là vị thế thật.
copy-holdings-open-positions = Vị thế đang mở
copy-holdings-token-details = Mở chi tiết token
copy-holdings-opened = Đã mở { $time }
copy-holdings-no-pool-price = Không có giá pool
copy-holdings-close = Đóng
copy-holdings-write-off = Xóa sổ
copy-holdings-activity = Hoạt động
copy-holdings-no-exit-rule = Không có quy tắc thoát
copy-holdings-watch-stop = Stop { $level }
copy-holdings-watch-stop-until = Stop { $level } sau { $span }
copy-holdings-watch-take = Take { $level }
copy-holdings-watch-trail = Trail { $level }
copy-holdings-watch-trail-arms = Trail kích hoạt { $level }
copy-holdings-watch-time = Thời gian ≤ { $level }
copy-holdings-watch-time-until = Thời gian ≤ { $level } sau { $span }
copy-holdings-watch-wallet-sells = Ví bán
copy-holdings-empty = Không có khoản nắm giữ thử nghiệm nào đang mở. Các lệnh mua copy từ ví sẽ hiện ở đây.
copy-holdings-col-token = Token
copy-holdings-col-cost = Giá vốn
copy-holdings-col-entry = Điểm vào
copy-holdings-col-mark = Giá định giá
copy-holdings-col-peak = Đỉnh
copy-holdings-col-pnl = P&L
copy-holdings-col-exit-rules = Quy tắc thoát
copy-holdings-col-held = Đã giữ
copy-holdings-col-actions = Thao tác
copy-holdings-col-invested = Đã đầu tư
copy-holdings-col-proceeds = Số tiền thu về
copy-holdings-col-exit = Thoát
copy-holdings-col-closed = Đã đóng
copy-holdings-price-note = Giá tính bằng { -sol } trên mỗi token. Điểm vào đã gồm trượt giá và phí của lệnh mua; đỉnh và các mức thoát được tính tương đối so với đó, nên một khoản nắm giữ mở ra với đỉnh thấp hơn điểm vào. Di chuột qua một mục để xem giá pool của nó.
copy-holdings-paused-rules = Đã tạm dừng: không có lệnh copy mới. Quy tắc thoát của bạn vẫn đóng các khoản nắm giữ này.
copy-holdings-paused-mirror = Đã tạm dừng: không có lệnh copy mới. Lệnh bán của ví vẫn đóng các khoản nắm giữ này.
copy-holdings-paused-hybrid = Đã tạm dừng: không có lệnh copy mới. Lệnh bán của ví và quy tắc thoát của bạn vẫn đóng các khoản nắm giữ này.
copy-holdings-closed-load-failed = Không thể tải các vòng đã đóng: { $error }
copy-holdings-closed-loading = Đang tải các vòng đã đóng…
copy-holdings-closed-empty = Chưa có vòng nào đã đóng.
copy-holdings-closed-latest = { $shown } vòng gần nhất trong { $total } vòng.
copy-holdings-close-title = Đóng khoản nắm giữ thử nghiệm
copy-holdings-close-message = Bán { $token } trong sổ thử nghiệm theo giá pool ({ $price }) với trượt giá và phí của tác vụ.
copy-holdings-close-confirm = Đóng khoản nắm giữ
copy-holdings-write-off-title = Xóa sổ khoản nắm giữ thử nghiệm
copy-holdings-write-off-message = { $token } không có giá pool để bán. Xóa sổ sẽ đóng nó ở mức không và ghi nhận giá vốn { $cost } là khoản lỗ.
copy-holdings-keep = Giữ lại
copy-holdings-written-off = Đã xóa sổ { $token }
copy-holdings-closed = Đã đóng { $token }
copy-holdings-written-off-detail = Đóng với số tiền thu về bằng không
copy-holdings-sold-at = Đã bán tại { $price }
copy-holdings-close-failed = Không thể đóng khoản nắm giữ
copy-holdings-reset-message = Làm lại “{ $name }” từ đầu: các khoản nắm giữ thử nghiệm, chi tiêu, lệnh khớp, lệnh thoát và lệnh bỏ qua sẽ bị xóa. Quy tắc và ví được giữ lại.
copy-holdings-reset-cancel = Giữ lịch sử
copy-holdings-reset-done = Đã đặt lại sổ thử nghiệm
copy-holdings-reset-detail =
    { $count ->
       *[other] Đã xóa { $count } quyết định
    }
copy-holdings-reset-failed = Không thể đặt lại sổ thử nghiệm

copy-activity-title = Hoạt động
copy-activity-filter-label = Bộ lọc hoạt động
copy-filter-all = Tất cả
copy-outcome-paper-filled = Mua thử nghiệm
copy-outcome-live-submitted = Đã gửi lệnh mua live
copy-outcome-live-confirmed = Đã xác nhận lệnh mua live
copy-outcome-live-failed = Lệnh mua live thất bại
copy-outcome-paper-sell-observed = Bán thử nghiệm · ví đã bán
copy-outcome-live-sell-submitted = Đã gửi lệnh bán live
copy-outcome-live-sell-failed = Lệnh bán live thất bại
copy-outcome-skipped = Đã bỏ qua
copy-activity-decision = Quyết định
copy-activity-paper-exit = Thoát thử nghiệm · { $rule }
copy-activity-filled = { $input } tại { $price } · ví đã mua { $target }
copy-activity-filled-slippage = { $input } tại { $price } · ví đã mua { $target } · trượt giá { $slippage }
copy-activity-filled-unpriced = { $input } tại { $price } · ví đã mua { $target } · định giá theo giao dịch của ví, không có giá pool
copy-activity-live-sized = { $sized } · ví đã mua { $target }
copy-activity-sell-nothing = Ví đã bán { $amount } · không có gì đang giữ để bán
copy-activity-written-off = Đã xóa sổ ở mức không: không có giá pool
copy-activity-sold = { $tokens } token với { $proceeds } tại { $price }
copy-activity-full-close = Đóng toàn bộ
copy-activity-partial-exit = Thoát { $pct }
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = tối thiểu { $amount }
copy-activity-skip-maximum = tối đa { $value }
copy-activity-skip-stale = trễ { $arrival }, giới hạn { $limit }
copy-activity-skip-latency = trung bình { $average }, giới hạn { $limit }
copy-activity-arrival-replayed = Phát lại { $span } sau block
copy-activity-arrival-seen = Thấy { $span } sau block
copy-activity-link-wallet-tx = Giao dịch của ví
copy-activity-link-own-tx = Giao dịch của bạn
copy-activity-only-token = Chỉ token này
copy-activity-skipped-group = Đã bỏ qua ×{ $count }
copy-activity-group-detail =
    { $tokens ->
       *[other] { $tokens } token · từ { $since }
    }
copy-activity-mint-filter =
    .placeholder = Mint của token
    .aria-label = Lọc theo mint của token
copy-activity-clear = Xóa
copy-activity-load-failed = Không thể tải hoạt động: { $error }
copy-activity-loading = Đang tải hoạt động…
copy-activity-no-match = Không có gì khớp với bộ lọc này.
copy-activity-empty = Chưa có quyết định nào. Lệnh khớp, lệnh thoát và lệnh bỏ qua sẽ hiện ở đây khi ví giao dịch.
copy-activity-load-older = Tải cũ hơn
copy-activity-start = Đầu lịch sử
copy-activity-older-failed = Không thể tải hoạt động cũ hơn

copy-step-wallet = Ví
copy-step-sizing = Quy mô
copy-step-entry = Bộ lọc điểm vào
copy-step-exits = Lệnh thoát
copy-step-review = Xem lại
copy-editor-title-edit = Sửa { $name }
copy-editor-title-clone = Nhân bản { $name }
copy-editor-sub-edit = Tác vụ { $mode } · thay đổi áp dụng cho các quyết định tiếp theo
copy-editor-sub-clone = Cùng quy tắc, sổ thử nghiệm trống, bắt đầu ở chế độ thử nghiệm
copy-editor-save-edit = Lưu thay đổi
copy-editor-save-clone = Tạo bản nhân bản
copy-editor-save-create = Tạo tác vụ thử nghiệm
copy-editor-clone-suffix = (bản sao)
copy-editor-discard-edit = Hủy thay đổi
copy-editor-discard-create = Hủy tác vụ này
copy-editor-discard-edit-message = Các thay đổi của bạn với “{ $name }” chưa được lưu.
copy-editor-discard-create-message = Ví và các quy tắc đã nhập chưa được lưu.
copy-editor-discard-confirm = Hủy bỏ
copy-editor-keep-editing = Tiếp tục sửa
copy-editor-toast-updated = Đã cập nhật tác vụ
copy-editor-toast-clone = Đã tạo bản nhân bản
copy-editor-toast-created = Đã tạo tác vụ thử nghiệm
copy-unit-native = { -sol }
copy-editor-any = Bất kỳ
copy-editor-duplicate = Đã được copy bởi { $tasks }. Tác vụ này copy lại cùng các giao dịch đó, với quy tắc và ngân sách riêng.
copy-editor-wallet = Ví
copy-editor-wallet-identity = Ví của tác vụ chính là danh tính của nó. Để copy ví khác với các quy tắc này, hãy nhân bản tác vụ.
copy-editor-address-label = Địa chỉ ví
copy-editor-address-placeholder = Địa chỉ ví Solana
copy-editor-address-help-clone = Cùng quy tắc với sổ thử nghiệm trống. Giữ ví này để thử các quy tắc khác trên nó, hoặc nhập ví khác.
copy-editor-address-help-create = Ví mà tác vụ này copy các lệnh mua (và các lệnh bán, nếu bạn chọn).
copy-editor-name-label = Tên <em>tùy chọn</em>
copy-editor-name-placeholder = ví dụ: Người xoay vòng nhanh
copy-editor-enabled-title = Xử lý giao dịch của ví
copy-editor-enabled-help = Tắt sẽ giữ tác vụ tạm dừng cho đến khi bạn tiếp tục.
copy-editor-note-live = Tác vụ này đang live: thay đổi áp dụng cho các lệnh copy thật tiếp theo.
copy-editor-note-paper = Các tác vụ chạy ở chế độ thử nghiệm cho đến khi bạn kích hoạt: giao dịch được mô phỏng theo giá pool và không tốn tiền.
copy-editor-copy-size = Quy mô copy
copy-editor-sizing-fixed = Số lượng cố định
copy-editor-sizing-ratio = Tỷ lệ giao dịch của ví
copy-editor-amount-fixed = Số lượng mỗi lần copy
copy-editor-amount-ratio = Tỷ lệ mỗi giao dịch
copy-editor-amount-help-fixed = Chi cho mỗi lệnh mua được copy, tối thiểu { $minimum }.
copy-editor-amount-help-ratio = Tính trên lệnh mua của chính ví, tối đa đến giới hạn mỗi giao dịch.
copy-editor-help-trade-cap = Không lần copy nào chi nhiều hơn mức này.
copy-editor-help-token-cap = Tổng số tiền chi cho một token.
copy-editor-help-budget = Mọi thứ tác vụ này có thể chi trong suốt vòng đời; thử nghiệm và live mỗi bên tính chi tiêu riêng.
copy-editor-preview-title = Chi phí của một lần copy
copy-editor-preview-empty = Hãy nhập quy mô để xem chi phí của một lần copy.
copy-editor-preview-example = Ví mua { $target } → bạn copy <strong>{ $copy }</strong>
copy-editor-preview-once = Mỗi token chỉ được copy một lần với { $size }, vì mỗi token chỉ mua một lần
copy-editor-preview-token-cap =
    { $count ->
       *[other] Mỗi token được copy tối đa { $count } lần, mỗi lần { $size }
    }
copy-editor-preview-summary-exact = { $perToken }; ngân sách đủ cho khoảng { $count } lần. Phí mạng và phí ưu tiên tính thêm.
copy-editor-preview-summary-minimum = { $perToken }; ngân sách đủ cho ít nhất { $count } lần. Phí mạng và phí ưu tiên tính thêm.
copy-editor-target-min = Giao dịch nhỏ nhất của ví được copy
copy-editor-target-min-help = Bỏ qua các lệnh mua nhỏ hơn của ví. Để trống nếu không có mức tối thiểu.
copy-editor-target-max = Giao dịch lớn nhất của ví được copy
copy-editor-target-max-help = Bỏ qua các lệnh mua lớn hơn của ví. Để trống nếu không có mức tối đa.
copy-editor-buy-once-title = Mua mỗi token một lần
copy-editor-buy-once-help = Chỉ copy lần mua đầu tiên của ví với một token; các lần mua sau bị bỏ qua.
copy-editor-filter-require = Bắt buộc
copy-editor-filter-skip = Không bắt buộc
copy-editor-filter-help = Yêu cầu token đạt quy trình Lọc của bạn trước khi được copy.
copy-editor-filter-warning = Với thiết lập Lọc mặc định, gần như mọi token đều không đạt, nên tác vụ yêu cầu đạt bộ lọc sẽ không copy gì cả. Chỉ bật khi bộ lọc của bạn cho qua các token mà ví này giao dịch.
copy-editor-exit-both = Cả hai
copy-editor-exit-help-buy-only = Các quy tắc bên dưới của bạn bán mọi khoản nắm giữ; lệnh bán của ví bị bỏ qua.
copy-editor-exit-help-hybrid = Cái nào đến trước: ví bán, hoặc một trong các quy tắc của bạn kích hoạt.
copy-editor-exit-help-mirror = Các khoản nắm giữ chỉ được bán khi ví bán. Quy tắc thoát của bạn không chạy.
copy-editor-who-sells = Ai bán
copy-editor-preset = Mẫu
copy-editor-preset-help = Mẫu sẽ điền mọi quy tắc bên dưới; bạn có thể chỉnh bất kỳ quy tắc nào sau đó.
copy-editor-mirror-note = Các quy tắc này không chạy khi lệnh bán của ví quyết định. Chúng áp dụng nếu bạn chuyển sang { $mine } hoặc { $both }.
copy-editor-rule-inherit = Mặc định của Trader
copy-editor-inherit-value = Mặc định của Trader ({ $value })
copy-editor-rule-aria = Cài đặt { $rule }
copy-editor-rule-empty-uses = Để trống sẽ dùng mặc định của Trader: { $value }
copy-editor-rule-follows = Theo Trader: { $summary }
copy-editor-rule-follows-plain = Theo cài đặt của Trader.
copy-editor-rule-off-note = Tắt cho tác vụ này, bất kể Trader dùng gì.
copy-task-unnamed = Tác vụ chưa đặt tên
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = xử lý giao dịch sau khi lưu
copy-editor-review-paused = lưu ở trạng thái tạm dừng
copy-editor-error-address = Hãy nhập địa chỉ ví Solana hợp lệ.
copy-editor-error-sizing = Mọi giá trị quy mô phải lớn hơn không.
copy-editor-error-min-copy = Mỗi lần copy phải ít nhất { $minimum }: hãy tăng số lượng mỗi lần copy.
copy-editor-error-min-cap = Mỗi lần copy phải ít nhất { $minimum }: hãy tăng giới hạn mỗi giao dịch.
copy-editor-error-trade-cap = Giới hạn mỗi giao dịch không được vượt quá giới hạn mỗi token.
copy-editor-error-token-cap = Giới hạn mỗi token không được vượt quá tổng ngân sách.
copy-editor-error-slippage = Trượt giá phải nằm trong khoảng từ { $min } đến { $max }.
copy-editor-error-target-limits = Giới hạn giao dịch của ví phải bằng không hoặc lớn hơn.
copy-editor-error-target-order = Giao dịch nhỏ nhất của ví không được vượt quá giao dịch lớn nhất.

copy-notice-task-unnamed = Tác vụ #{ $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = Mua copy thử nghiệm
copy-notice-title-paper-sell = Bán copy thử nghiệm
copy-notice-title-paper-closed = Đã đóng khoản nắm giữ thử nghiệm
copy-notice-title-paper-exit = Thoát thử nghiệm: { $rule }
copy-notice-title-live-buy-submitted = Đã gửi lệnh mua copy live
copy-notice-title-live-buy-confirmed = Đã xác nhận lệnh mua copy live
copy-notice-title-live-buy-failed = Lệnh mua copy live thất bại
copy-notice-title-live-sell-submitted = Đã gửi lệnh bán copy live
copy-notice-title-live-sell-failed = Lệnh bán copy live thất bại
copy-notice-title-auto-paused = Tác vụ copy đã tự động tạm dừng
copy-notice-detail-bought = Đã mua với { $amount } { -sol }
copy-notice-detail-sold = Đã bán được { $amount } { -sol }
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = { $percent }% khoản nắm giữ
copy-notice-detail-full-close = Đóng toàn bộ
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = Swap thất bại
