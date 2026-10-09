## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = Công cụ
tools-category-wallet = Ví
tools-category-token = Token
tools-category-single-token = Một token
tools-category-utilities = Tiện ích
tools-sidebar-hint = Chọn một công cụ để bắt đầu
tools-help-button =
    .aria-label = Xem trợ giúp cho công cụ này
tools-help-unavailable = Không có trợ giúp
tools-placeholder-title = Chọn một công cụ
tools-placeholder-subtitle = Chọn một công cụ ở thanh bên để bắt đầu
tools-placeholder-hint-wallets = Các công cụ ví giúp bạn quản lý ví Solana
tools-placeholder-hint-secure = Mọi thao tác đều được bảo mật và có thể hoàn tác khi khả thi

tools-status-ready = Sẵn sàng sử dụng
tools-status-coming = Sắp ra mắt
tools-status-beta = Beta - có thể có lỗi
tools-status-disabled = Hiện đã tắt
tools-status-badge-coming = Sắp ra mắt
tools-status-badge-beta = Beta
tools-toast-coming-soon = Công cụ này sắp ra mắt
tools-toast-disabled = Công cụ này hiện đã tắt
tools-setup-gate-title = Công cụ này cần có ví

## Tool names. `-title` names the tool in the navigation and the header, `-summary` is the
## navigation line, `-description` is the header line. Ids are the tool ids of the registry.

tools-tool-wallet-cleanup-title = Dọn dẹp ví
tools-tool-wallet-cleanup-summary = Đóng ATA trống
tools-tool-wallet-cleanup-description = Đóng các Associated Token Account trống để thu hồi { -sol }
tools-tool-burn-tokens-title = Burn token
tools-tool-burn-tokens-summary = Hủy vĩnh viễn token
tools-tool-burn-tokens-description = Hủy vĩnh viễn token khỏi ví của bạn
tools-tool-token-analyzer-title = Phân tích token
tools-tool-token-analyzer-summary = Phân tích chuyên sâu token
tools-tool-token-analyzer-description = Phân tích chuyên sâu mọi token Solana với góc nhìn đa chiều
tools-tool-create-token-title = Tạo token
tools-tool-create-token-summary = Triển khai token SPL mới
tools-tool-create-token-description = Triển khai một token SPL mới trên Solana
tools-tool-trade-watcher-title = Trade Watcher
tools-tool-trade-watcher-summary = Giám sát giao dịch và tự động hành động
tools-tool-trade-watcher-description = Giám sát các giao dịch của token và kích hoạt hành động mua/bán tự động
tools-tool-token-watch-title = Holder Watch
tools-tool-token-watch-summary = Theo dõi holder mới của token
tools-tool-token-watch-description = Theo dõi và giám sát holder mới của token theo thời gian thực
tools-tool-buy-multi-wallets-title = Mua nhiều lần
tools-tool-buy-multi-wallets-summary = Phối hợp mua trên nhiều ví
tools-tool-buy-multi-wallets-description = Thực hiện các lệnh mua phối hợp trên nhiều ví với số lượng ngẫu nhiên
tools-tool-sell-multi-wallets-title = Bán nhiều lần
tools-tool-sell-multi-wallets-summary = Phối hợp bán trên nhiều ví
tools-tool-sell-multi-wallets-description = Thực hiện các lệnh bán phối hợp trên nhiều ví kèm gộp { -sol }
tools-tool-wallet-consolidation-title = Gộp số dư ví
tools-tool-wallet-consolidation-nav-title = Gộp số dư
tools-tool-wallet-consolidation-summary = Gộp tiền trong các ví
tools-tool-wallet-consolidation-description = Gộp { -sol } và token từ các ví phụ về ví chính
tools-tool-airdrop-checker-title = Kiểm tra airdrop
tools-tool-airdrop-checker-summary = Kiểm tra airdrop đang chờ
tools-tool-airdrop-checker-description = Kiểm tra airdrop đang chờ và phần thưởng có thể nhận
tools-tool-wallet-generator-title = Tạo ví
tools-tool-wallet-generator-summary = Tạo keypair mới
tools-tool-wallet-generator-description = Tạo keypair Solana mới một cách an toàn

## Shared by the tools

tools-validation-mint-required = Vui lòng nhập địa chỉ mint của token
tools-validation-mint-format = Định dạng địa chỉ mint của token không hợp lệ
tools-validation-mint-invalid = Vui lòng nhập địa chỉ mint hợp lệ

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = Chi tiết token
tools-create-token-name-label = Tên token
tools-create-token-name-input =
    .placeholder = Token của tôi
tools-create-token-symbol-label = Ký hiệu
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = Số chữ số thập phân
tools-create-token-supply-label = Nguồn cung ban đầu
tools-create-token-description-label = Mô tả
tools-create-token-description-input =
    .placeholder = Mô tả token...
tools-create-token-image-title = Hình ảnh token
tools-create-token-image-drop = Thả ảnh vào đây hoặc nhấp để tải lên
tools-create-token-image-hint = Khuyến nghị: PNG 512x512
tools-create-token-action-preview = Xem trước
tools-create-token-action-create = Tạo token

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = Đang tải cài đặt...
tools-holder-watch-saved = Đã lưu cài đặt Holder Watch
tools-holder-watch-save-failed = Không lưu được cài đặt
tools-holder-watch-save-error = Lỗi khi lưu cài đặt
tools-holder-watch-settings-title = Cài đặt Holder Watch
tools-holder-watch-enabled-label = Bật theo dõi holder
tools-holder-watch-interval-label = Chu kỳ kiểm tra
tools-holder-watch-interval-hint = Tần suất kiểm tra số lượng holder (10-3600 giây)
tools-holder-watch-max-tokens-label = Số token theo dõi tối đa
tools-holder-watch-max-tokens-hint = Số token tối đa được theo dõi cùng lúc
tools-holder-watch-notify-new-label = Thông báo khi có holder mới
tools-holder-watch-notify-drop-label = Thông báo khi holder giảm
tools-holder-watch-min-change-label = Mức thay đổi holder tối thiểu
tools-holder-watch-min-change-hint = Mức thay đổi holder tối thiểu để kích hoạt thông báo
tools-holder-watch-drop-percent-label = Ngưỡng giảm holder
tools-holder-watch-drop-percent-hint = Phần trăm giảm để kích hoạt cảnh báo
tools-holder-watch-action-save = Lưu cài đặt
tools-holder-watch-tokens-title = Token đang theo dõi
tools-holder-watch-token-input =
    .placeholder = Nhập địa chỉ mint của token...
tools-holder-watch-empty = Chưa theo dõi token nào
tools-holder-watch-empty-hint = Thêm địa chỉ mint của token ở trên để bắt đầu theo dõi
tools-holder-watch-coming-soon = Tính năng theo dõi token sắp ra mắt

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = Phân tích token
tools-analyzer-mint-input =
    .placeholder = Dán địa chỉ mint của token...
tools-analyzer-action-analyze = Phân tích
tools-analyzer-action-analyzing = Đang phân tích...
tools-analyzer-action-copy-report = Sao chép báo cáo
tools-analyzer-loading = Đang phân tích token...
tools-analyzer-failed = Không phân tích được token
tools-analyzer-empty = Nhập địa chỉ mint của token để phân tích
tools-analyzer-empty-hint = Nhận thông tin toàn diện về mọi token Solana
tools-analyzer-tab-overview = Tổng quan
tools-analyzer-tab-security = Bảo mật
tools-analyzer-tab-market = Thị trường
tools-analyzer-tab-liquidity = Thanh khoản
tools-analyzer-unknown-token = Token không xác định

tools-analyzer-favorite-add =
    .title = Thêm vào yêu thích
    .aria-label = Thêm vào yêu thích
tools-analyzer-favorite-already = Đã có trong yêu thích
tools-analyzer-favorite-added = Đã thêm { $symbol } vào yêu thích
tools-analyzer-favorite-failed = Không thêm được vào yêu thích
tools-analyzer-blacklist-add =
    .title = Thêm vào danh sách đen
    .aria-label = Thêm vào danh sách đen
tools-analyzer-blacklist-title = Đưa token vào danh sách đen
tools-analyzer-blacklist-message = Đưa { $symbol } vào danh sách đen? Token này sẽ bị loại khỏi giao dịch.
tools-analyzer-blacklist-confirm = Đưa vào danh sách đen
tools-analyzer-blacklisted = Trong danh sách đen
tools-analyzer-blacklist-done = Đã đưa { $symbol } vào danh sách đen
tools-analyzer-blacklist-failed = Không đưa được token vào danh sách đen

tools-analyzer-card-quick-stats = Thống kê nhanh
tools-analyzer-card-market-summary = Tóm tắt thị trường
tools-analyzer-card-token-info = Thông tin token
tools-analyzer-stat-holders = Holder
tools-analyzer-stat-decimals = Số chữ số thập phân
tools-analyzer-stat-safety-score = Điểm an toàn
tools-analyzer-stat-pools = Pool
tools-analyzer-stat-volume-24h = Khối lượng 24h
tools-analyzer-stat-change-24h = Biến động 24h
tools-analyzer-stat-market-cap = Vốn hóa
tools-analyzer-stat-liquidity = Thanh khoản
tools-analyzer-info-mint = Địa chỉ mint
tools-analyzer-info-description = Mô tả
tools-analyzer-info-supply = Nguồn cung

tools-analyzer-security-empty = Không có dữ liệu bảo mật
tools-analyzer-security-empty-hint = Không có phân tích bảo mật cho token này
tools-analyzer-card-safety-score = Điểm an toàn
tools-analyzer-score-good = Tốt
tools-analyzer-score-moderate = Trung bình
tools-analyzer-score-risky = Rủi ro
tools-analyzer-raw-score = Điểm rủi ro gốc: { $score }
tools-analyzer-card-authorities = Quyền của token
tools-analyzer-authority-mint = Quyền mint
tools-analyzer-authority-freeze = Quyền freeze
tools-analyzer-authority-transfer-fee = Phí chuyển
tools-analyzer-authority-mutable = Có thể thay đổi
tools-analyzer-authority-active = Đang hoạt động
tools-analyzer-authority-revoked = Đã thu hồi
tools-analyzer-card-holder-concentration = Mức tập trung holder
tools-analyzer-top-holders = do 10 holder lớn nhất nắm giữ
tools-analyzer-risks-title = Rủi ro bảo mật ({ $count })
tools-analyzer-risks-title-none = Rủi ro bảo mật
tools-analyzer-risks-none = Không phát hiện rủi ro bảo mật

tools-analyzer-market-empty = Không có dữ liệu thị trường
tools-analyzer-market-empty-hint = Không có dữ liệu thị trường cho token này
tools-analyzer-card-price = Giá hiện tại
tools-analyzer-card-price-changes = Biến động giá
tools-analyzer-card-volume = Khối lượng giao dịch
tools-analyzer-card-transactions = Giao dịch 24h
tools-analyzer-card-valuation = Định giá
tools-analyzer-stat-window-1h = 1h
tools-analyzer-stat-window-6h = 6h
tools-analyzer-stat-window-24h = 24h
tools-analyzer-stat-volume-1h = Khối lượng 1h
tools-analyzer-stat-volume-6h = Khối lượng 6h
tools-analyzer-stat-fdv = Giá trị pha loãng hoàn toàn
tools-analyzer-txn-buys = Lệnh mua
tools-analyzer-txn-sells = Lệnh bán

tools-analyzer-liquidity-empty = Không có dữ liệu thanh khoản
tools-analyzer-liquidity-empty-hint = Không tìm thấy pool nào cho token này
tools-analyzer-card-total-liquidity = Tổng thanh khoản
tools-analyzer-card-pools = Pool
tools-analyzer-active-pools =
    { $count ->
       *[other] Pool đang hoạt động
    }
tools-analyzer-card-pool-details = Chi tiết pool
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = Thanh khoản ({ -sol })
tools-analyzer-pools-column-status = Trạng thái
tools-analyzer-pool-primary = Chính

tools-analyzer-report-empty = Không có phân tích để sao chép
tools-analyzer-report-label = Báo cáo phân tích
tools-analyzer-report-title = Báo cáo phân tích token
tools-analyzer-report-token = Token: { $symbol } ({ $name })
tools-analyzer-report-mint = Mint: { $mint }
tools-analyzer-report-price = Giá: { $sol }
tools-analyzer-report-price-with-usd = Giá: { $sol } ({ $usd })
tools-analyzer-report-security = Bảo mật:
tools-analyzer-report-safety-score = - Điểm an toàn: { $score }/100
tools-analyzer-report-mint-authority = - Quyền mint: { $state }
tools-analyzer-report-freeze-authority = - Quyền freeze: { $state }
tools-analyzer-report-risks = - Rủi ro: { $count }
tools-analyzer-report-market = Thị trường:
tools-analyzer-report-volume = - Khối lượng 24h: { $amount }
tools-analyzer-report-change = - Biến động 24h: { $amount }
tools-analyzer-report-market-cap = - Vốn hóa: { $amount }
tools-analyzer-report-liquidity = Thanh khoản:
tools-analyzer-report-liquidity-total = - Tổng: { $amount }
tools-analyzer-report-pools = - Pool: { $count }
tools-analyzer-report-generated = Tạo lúc: { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

tools-watch-type-buy-on-sell = Mua khi có bán
tools-watch-type-sell-on-buy = Bán khi có mua
tools-watch-type-notify = Thông báo
tools-watch-type-notify-only = Chỉ thông báo

tools-trade-watcher-setup-title = Thiết lập theo dõi
tools-trade-watcher-mint-label = Địa chỉ mint của token
tools-trade-watcher-mint-input =
    .placeholder = Nhập địa chỉ mint của token...
tools-trade-watcher-action-search-pools = Tìm pool
tools-trade-watcher-pool-label = Pool đã chọn
tools-trade-watcher-pool-none = Chưa chọn pool
tools-trade-watcher-pool-clear =
    .title = Bỏ chọn pool
tools-trade-watcher-pool-selected = Pool đã chọn: { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = Loại theo dõi
tools-trade-watcher-type-hint = Mua khi có bán: tự động mua khi có người bán. Bán khi có mua: tự động bán khi có người mua.
tools-trade-watcher-trigger-label = Số lượng kích hoạt
tools-trade-watcher-trigger-hint = Quy mô giao dịch tối thiểu tính bằng { -sol } để kích hoạt hành động
tools-trade-watcher-action-amount-label = Số lượng thực hiện
tools-trade-watcher-action-amount-hint = Số lượng mua/bán khi được kích hoạt
tools-trade-watcher-slippage-label = Trượt giá
tools-trade-watcher-slippage-hint = Mức trượt giá tối đa chấp nhận được cho các giao dịch
tools-trade-watcher-active-title = Các lượt theo dõi đang hoạt động
tools-trade-watcher-empty = Không có lượt theo dõi nào đang hoạt động
tools-trade-watcher-empty-hint = Cấu hình một lượt theo dõi ở trên rồi nhấp "Bắt đầu theo dõi" để bắt đầu giám sát
tools-trade-watcher-action-start = Bắt đầu theo dõi
tools-trade-watcher-action-starting = Đang bắt đầu...
tools-trade-watcher-action-stop-all = Dừng tất cả
tools-trade-watcher-action-stopping = Đang dừng...
tools-trade-watcher-started = Đã bắt đầu theo dõi { $token }...
tools-trade-watcher-start-failed = Không bắt đầu được việc theo dõi
tools-trade-watcher-stopped = Đã dừng theo dõi
tools-trade-watcher-stop-failed = Không dừng được việc theo dõi
tools-trade-watcher-stopped-all = Đã dừng tất cả lượt theo dõi
tools-trade-watcher-stop-all-failed = Không dừng được các lượt theo dõi
tools-trade-watcher-load-failed = Không tải được các lượt theo dõi
tools-trade-watcher-column-token = Token
tools-trade-watcher-column-type = Loại
tools-trade-watcher-column-trigger = Kích hoạt ({ -sol })
tools-trade-watcher-column-action = Hành động ({ -sol })
tools-trade-watcher-column-triggered = Đã kích hoạt
tools-trade-watcher-stop-watch =
    .title = Dừng theo dõi

## Results returned by the tools backend. Failures are catalog text; the technical cause
## travels separately as details and is appended by the dashboard.

tools-burn-failure-native-asset = Không thể burn { -sol }
tools-burn-failure-open-position = Không thể burn token của các vị thế đang mở
tools-burn-failure-account-not-found = Không tìm thấy tài khoản token
tools-burn-failure-zero-balance = Số dư token đã bằng 0
tools-burn-failure-transaction = Giao dịch thất bại
tools-burn-warning-open-position = Không thể burn token của các vị thế đang mở
tools-burn-warning-closed-position = Phần còn lại từ vị thế đã đóng
tools-burn-warning-worth = Trị giá ~{ $amount } { -sol }
tools-multi-buy-warning-insufficient = Số dư không đủ. Cần { $needed } { -sol }, hiện có { $have } { -sol }
tools-multi-buy-warning-over-limit = Tổng { -sol } cần dùng ({ $needed }) vượt quá giới hạn ({ $limit })
tools-multi-sell-warning-no-wallets = Không tìm thấy ví thứ cấp nào
tools-multi-sell-warning-no-balance = Không có ví nào có số dư token
tools-multi-op-buy-failed = Mua thất bại
tools-multi-op-sell-failed = Bán thất bại
tools-multi-op-transfer-failed = Chuyển thất bại
tools-multi-op-balance-failed = Không lấy được số dư
tools-multi-op-mint-invalid = Địa chỉ mint không hợp lệ
tools-multi-buy-session-failed = Mua nhiều lần thất bại
tools-multi-sell-session-failed = Bán nhiều lần thất bại
tools-multi-session-aborted = Người dùng đã hủy thao tác

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = Quét ví
tools-wallet-action-scanning = Đang quét...
tools-wallet-scan-failed = Quét thất bại: { $reason }
tools-wallet-amount-approx = ~{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
       *[other] Đã chọn: { $count } ví
    }
tools-wallet-transfer-failed = Chuyển thất bại: { $reason }
tools-wallet-cleanup-failed = Dọn dẹp thất bại: { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = Kết quả quét
tools-wallet-cleanup-stat-empty = ATA trống
tools-wallet-cleanup-stat-reclaimable = { -sol } có thể thu hồi
tools-wallet-cleanup-stat-failed = Thất bại (đã lưu đệm)
tools-wallet-cleanup-prompt = Nhấp "Quét ví" để tìm ATA trống
tools-wallet-cleanup-prompt-hint = Thao tác này sẽ kiểm tra mọi tài khoản token trong ví của bạn
tools-wallet-cleanup-action-cleanup = Dọn dẹp tất cả
tools-wallet-cleanup-action-cleaning = Đang dọn dẹp...
tools-wallet-cleanup-scanning = Đang quét ví...
tools-wallet-cleanup-found =
    { $count ->
       *[other] Tìm thấy { $count } ATA trống trị giá ~{ $amount }
    }
tools-wallet-cleanup-clean = Không có ATA trống - ví đã sạch!
tools-wallet-cleanup-scan-failed = Không quét được ATA
tools-wallet-cleanup-done =
    { $count ->
       *[other] Đã dọn dẹp { $count } ATA
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = Burn token
tools-burn-info-title = Burn là gì?
tools-burn-info-body = Burn sẽ hủy vĩnh viễn token và không thể khôi phục. Sau khi burn, hãy chạy Dọn dẹp ví để đóng ATA trống và thu hồi khoảng 0.002 { -sol } phí rent cho mỗi token.
tools-burn-stat-total = Tổng số token
tools-burn-stat-selected = Đã chọn
tools-burn-stat-rent = Phí rent có thể thu hồi
tools-burn-prompt = Nhấp "Quét ví" để tìm token
tools-burn-scanning = Đang quét token trong ví...
tools-burn-scan-failed = Không quét được token
tools-burn-empty = Không tìm thấy token nào trong ví
tools-burn-action-burn = Burn đã chọn ({ $count })
tools-burn-action-burning = Đang burn...
tools-burn-cannot-burn = Không thể burn
tools-burn-no-value = Không có giá trị

tools-burn-category-open-position = Vị thế đang mở
tools-burn-category-has-value = Có giá trị
tools-burn-category-closed-position = Vị thế đã đóng
tools-burn-category-zero-liquidity = Không có thanh khoản
tools-burn-category-hint-open-position = Không thể burn token của các vị thế đang mở
tools-burn-category-hint-has-value = Nên cân nhắc bán thay vì burn
tools-burn-category-hint-closed-position = Phần còn lại từ các giao dịch đã đóng
tools-burn-category-hint-zero-liquidity = Có thể burn an toàn - không có giá trị thị trường

tools-burn-confirm-title = Xác nhận burn
tools-burn-confirm-message =
    { $count ->
       *[other] Bạn có chắc muốn burn <strong>{ $count }</strong> token không?
    }
tools-burn-confirm-value = Tổng giá trị ước tính: <strong>{ $amount }</strong>
tools-burn-confirm-continue = Tiếp tục
tools-burn-final-title = Cảnh báo cuối cùng
tools-burn-final-headline = Hành động này KHÔNG THỂ HOÀN TÁC!
tools-burn-final-message =
    { $count ->
       *[other] { $count } token sau đây sẽ bị hủy vĩnh viễn và không thể khôi phục trong bất kỳ trường hợp nào.
    }
tools-burn-final-confirm = Có, burn token
tools-burn-toast-burned =
    { $total ->
       *[other] Đã burn { $successful }/{ $total } token. Hãy chạy Dọn dẹp ví để thu hồi ~{ $amount }
    }
tools-burn-toast-failed =
    { $count ->
       *[other] { $count } token burn thất bại
    }
tools-burn-failed = Burn thất bại: { $reason }
tools-burn-failures-title =
    { $count ->
       *[other] { $count } token không burn được
    }
tools-burn-failure-unknown = Không có lý do nào được báo cáo

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = Giới thiệu
tools-airdrop-about-body = Kiểm tra airdrop đang chờ, phần thưởng có thể nhận và các khoản phân bổ chưa nhận trên các giao thức Solana phổ biến.
tools-airdrop-list-title = Airdrop khả dụng
tools-airdrop-prompt = Nhấp "Kiểm tra airdrop" để quét các khoản có thể nhận
tools-airdrop-action-check = Kiểm tra airdrop
tools-airdrop-action-claim-all = Nhận tất cả

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = Tùy chọn tạo ví
tools-generator-warning-title = Hãy lưu giữ khóa riêng tư của bạn an toàn!
tools-generator-warning-body = Các keypair được tạo cục bộ và không bao giờ bị truyền đi. Luôn sao lưu khóa của bạn ở nơi an toàn.
tools-generator-count-label = Số lượng ví
tools-generator-vanity-label = Địa chỉ vanity (bắt đầu bằng các ký tự cụ thể)
tools-generator-prefix-label = Tiền tố
tools-generator-prefix-input =
    .placeholder = ví dụ: SOL
tools-generator-prefix-hint = Tiền tố càng dài thì thời gian tạo càng tăng theo cấp số nhân
tools-generator-list-title = Các ví đã tạo
tools-generator-empty = Chưa tạo ví nào
tools-generator-action-generate = Tạo
tools-generator-action-generating = Đang tạo...
tools-generator-count-invalid = Vui lòng nhập một số từ 1 đến 10
tools-generator-no-keypairs = Không có keypair nào được trả về
tools-generator-generated =
    { $count ->
       *[other] Đã tạo { $count } ví
    }
tools-generator-failed = Không tạo được ví: { $reason }
tools-generator-copy-public-key =
    .title = Sao chép khóa công khai
tools-generator-copy-private-key =
    .title = Sao chép khóa riêng tư
tools-generator-remove =
    .title = Xóa khỏi danh sách
tools-generator-reveal =
    .title = Hiện khóa riêng tư
tools-generator-public-key-label = Khóa công khai:
tools-generator-private-key-label = Khóa riêng tư:
tools-generator-public-key-name = Khóa công khai
tools-generator-private-key-copied = Đã sao chép khóa riêng tư
tools-generator-private-key-warning = Ai có khóa này sẽ kiểm soát được ví
tools-generator-export-empty = Không có ví để xuất
tools-generator-exported = Đã xuất ví - hãy lưu giữ an toàn

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = Tóm tắt
tools-consolidation-stat-wallets = Ví phụ
tools-consolidation-stat-native = Tổng { -sol }
tools-consolidation-stat-tokens = Số loại token
tools-consolidation-stat-rent = Phí rent có thể thu hồi
tools-consolidation-wallets-title = Ví
tools-consolidation-loading-wallets = Đang tải các ví...
tools-consolidation-loading-data = Đang tải dữ liệu ví...
tools-consolidation-action-transfer-native = Chuyển { -sol }
tools-consolidation-action-transfer-tokens = Chuyển tất cả token
tools-consolidation-action-cleanup = Dọn dẹp ATA
tools-consolidation-action-transferring = Đang chuyển...
tools-consolidation-column-name = Tên
tools-consolidation-column-native = Số dư ({ -sol })
tools-consolidation-column-tokens = Token
tools-consolidation-column-atas = ATA trống
tools-consolidation-empty = Không tìm thấy ví phụ nào
tools-consolidation-empty-hint = Tạo ví phụ bằng công cụ Mua nhiều lần để bắt đầu
tools-consolidation-load-failed = Không tải được: { $reason }
tools-consolidation-select-prompt = Chọn các ví để gộp số dư
tools-consolidation-selection-totals =
    | { $amount } | { $tokens ->
       *[other] { $tokens } token
    } | { $atas ->
       *[other] { $atas } ATA trống
    }
tools-consolidation-transferred-native = Đã chuyển { $amount } về ví chính
tools-consolidation-transferred-tokens =
    { $count ->
       *[other] Đã chuyển { $count } token về ví chính
    }
tools-consolidation-cleaned =
    { $count ->
       *[other] Đã đóng { $count } ATA, thu hồi { $amount }
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = Token
tools-multi-mint-label = Địa chỉ mint của token
tools-multi-mint-input =
    .placeholder = Dán địa chỉ mint của token...
tools-multi-execution-title = Cài đặt thực thi
tools-multi-delay-min-label = Độ trễ tối thiểu
tools-unit-native = { -sol }
tools-unit-seconds = giây
tools-unit-ms = ms
tools-multi-delay-max-label = Độ trễ tối đa
tools-multi-concurrency-label = Số luồng đồng thời
tools-multi-concurrency-sequential = { $count } (Tuần tự)
tools-multi-concurrency-parallel = { $count } song song
tools-multi-slippage-label = Trượt giá
tools-multi-router-label = Router
tools-multi-router-auto = Tự động (tuyến tốt nhất)
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = Pool trực tiếp
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = Tiến độ
tools-multi-progress-preparing = Đang chuẩn bị...
tools-multi-status-line = { $label } ({ $completed }/{ $total })
tools-multi-column-wallet = Ví
tools-multi-column-route = Tuyến
tools-multi-column-status = Trạng thái
tools-multi-op-completed = Hoàn tất
tools-multi-op-failed = Thất bại
tools-multi-action-stop = Dừng
tools-multi-action-loading = Đang tải...
tools-multi-start-failed = Không bắt đầu được: { $reason }

tools-multi-state-pending = Đang chờ
tools-multi-state-funding = Đang nạp tiền
tools-multi-state-executing = Đang thực thi
tools-multi-state-consolidating = Đang gộp số dư
tools-multi-state-completed = Hoàn tất
tools-multi-state-failed = Thất bại
tools-multi-state-aborted = Đã hủy

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = Token bạn muốn mua trên nhiều ví
tools-multi-buy-wallets-title = Cài đặt ví
tools-multi-buy-wallet-count-label = Số lượng ví
tools-multi-buy-wallet-count-option =
    { $count ->
       *[other] { $count } ví
    }
tools-multi-buy-wallet-count-hint = Số lượng ví phụ sẽ dùng
tools-multi-buy-buffer-label = { -sol } dự phòng cho mỗi ví
tools-multi-buy-buffer-hint = Dành cho phí (tối thiểu 0.015 { -sol })
tools-multi-buy-amounts-title = Cài đặt số lượng
tools-multi-buy-min-label = { -sol } tối thiểu mỗi ví
tools-multi-buy-min-hint = Số lượng mua tối thiểu
tools-multi-buy-max-label = { -sol } tối đa mỗi ví
tools-multi-buy-max-hint = Số lượng mua tối đa
tools-multi-buy-limit-label = Giới hạn tổng { -sol } (tùy chọn)
tools-multi-buy-limit-hint = Tổng chi tối đa
tools-multi-buy-preview-title = Xem trước
tools-multi-buy-preview-create = Số ví sẽ tạo
tools-multi-buy-preview-amount = Số lượng mỗi ví
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = Tổng { -sol } cần dùng
tools-multi-buy-preview-balance = Số dư ví chính
tools-multi-buy-action-preview = Xem trước
tools-multi-buy-action-start = Bắt đầu mua nhiều lần
tools-multi-buy-executing = Đang thực hiện các lệnh mua...
tools-multi-buy-column-spent = Đã chi ({ -sol })
tools-multi-buy-column-tokens = Token
tools-multi-buy-preview-failed = Xem trước thất bại: { $reason }
tools-multi-buy-started = Đã bắt đầu mua nhiều lần
tools-multi-buy-stopped = Đã dừng mua nhiều lần
tools-multi-buy-completed = Mua nhiều lần hoàn tất! Thành công { $successful }/{ $total }

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = Nhập địa chỉ token để quét các ví đang nắm giữ nó
tools-multi-sell-action-scan = Quét
tools-multi-sell-settings-title = Cài đặt bán
tools-multi-sell-percent-label = Phần trăm bán
tools-multi-sell-percent-hint = % token bán ở mỗi ví
tools-multi-sell-min-fee-label = { -sol } tối thiểu cho phí
tools-multi-sell-min-fee-hint = { -sol } tối thiểu cần có cho phí giao dịch
tools-multi-sell-topup-label = Tự động nạp thêm nếu cần
tools-multi-sell-topup-hint = Chuyển { -sol } từ ví chính nếu ví phụ không đủ số dư
tools-multi-sell-post-title = Hành động sau khi bán
tools-multi-sell-consolidate-label = Gộp { -sol } về ví chính
tools-multi-sell-consolidate-hint = Chuyển toàn bộ { -sol } từ các ví phụ về ví chính
tools-multi-sell-close-atas-label = Đóng ATA của token sau khi bán
tools-multi-sell-close-atas-hint = Thu hồi khoảng 0.002 { -sol } cho mỗi ATA
tools-multi-sell-wallets-title = Các ví có token
tools-multi-sell-empty = Không có ví phụ nào nắm giữ token này
tools-multi-sell-column-tokens = Token
tools-multi-sell-column-native = Số dư ({ -sol })
tools-multi-sell-column-topup = Cần nạp thêm
tools-multi-sell-none-selected = Chưa chọn ví nào
tools-multi-sell-select-required = Vui lòng chọn ít nhất một ví
tools-multi-sell-action-start = Bắt đầu bán nhiều lần
tools-multi-sell-executing = Đang thực hiện các lệnh bán...
tools-multi-sell-column-sold = Token đã bán
tools-multi-sell-column-received = Đã nhận ({ -sol })
tools-multi-sell-started = Đã bắt đầu bán nhiều lần
tools-multi-sell-stopped = Đã dừng bán nhiều lần
tools-multi-sell-completed = Bán nhiều lần hoàn tất! Đã nhận { $amount }

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = Yêu thích
tools-favorites-saved = Mục yêu thích đã lưu
tools-favorites-save-current = Lưu mục hiện tại
tools-favorites-empty = Chưa lưu mục yêu thích nào
tools-favorites-no-label = Không có nhãn
tools-favorites-uses = { $count }x
tools-favorites-remove = Xóa
tools-favorites-loaded = Đã tải mục yêu thích: { $name }
tools-favorites-default-name = Cấu hình
tools-favorites-mint-required = Vui lòng nhập địa chỉ mint của token trước
tools-favorites-add-title = Thêm vào yêu thích
tools-favorites-add-message = Nhập nhãn cho mục yêu thích này
tools-favorites-add-placeholder = Nhãn (tùy chọn)...
tools-favorites-saved-toast = Đã lưu vào yêu thích
tools-favorites-save-failed = Không lưu được mục yêu thích
tools-favorites-remove-title = Xóa mục yêu thích
tools-favorites-remove-message = Xóa mục yêu thích này?
tools-favorites-removed-toast = Đã xóa mục yêu thích
tools-favorites-remove-failed = Không xóa được mục yêu thích
