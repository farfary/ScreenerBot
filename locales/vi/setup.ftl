setup-wallet-required = Nhập khóa riêng tư của ví.
setup-wallet-json-recognized = Đã nhận dạng định dạng khóa JSON 64 byte.
setup-wallet-json-invalid = Hãy dùng mảng JSON chứa đúng 64 giá trị byte (0–255).
setup-wallet-format-invalid = Hãy dùng khóa riêng tư base58 hoặc mảng JSON 64 byte.
setup-wallet-base58-recognized = Đã nhận dạng định dạng khóa base58.

setup-rpc-required = Nhập ít nhất một endpoint RPC.
setup-rpc-too-many = Chỉ dùng tối đa 10 endpoint RPC.
setup-rpc-url-invalid = Mỗi endpoint phải là một URL HTTPS hợp lệ.
setup-rpc-url-credentials = URL RPC không được chứa tên người dùng hoặc mật khẩu.
setup-rpc-url-fragment = URL RPC không được chứa fragment.
setup-rpc-public-endpoint = RPC Solana công khai không thể hỗ trợ polling liên tục.
setup-rpc-private-host = Endpoint RPC không được dùng host cục bộ hoặc mạng riêng.
setup-rpc-duplicate = Hãy xóa các endpoint RPC trùng lặp.
setup-rpc-ready =
    { $count ->
       *[other] { $count } endpoint HTTPS sẵn sàng để kiểm tra.
    }

setup-wallet-verified = Đã xác minh ví
setup-wallet-unverified = Không thể xác minh ví
setup-wallet-address-detail = Địa chỉ { $address }
setup-wallet-format-hint = Hãy kiểm tra định dạng khóa riêng tư.
setup-rpc-none-working = Không có RPC mainnet nào hoạt động
setup-rpc-health-failed = Không có endpoint nào vượt qua kiểm tra tình trạng mainnet.
setup-rpc-partial = { $working } hoạt động; { $failed } không khả dụng
setup-rpc-verified =
    { $count ->
       *[other] Đã xác minh { $count } endpoint mainnet
    }
setup-rpc-fastest = Nhanh nhất: { $url } ({ $latency } ms).
setup-error-request-failed = Yêu cầu thất bại ({ $status })
setup-error-restart-timeout = Thiết lập đã được lưu nhưng { -brand } chưa kết nối lại.

setup-verify-wallet-parsing = Đang phân tích khóa riêng tư
setup-verify-wallet-parsing-detail = Đang kiểm tra khóa và tạo địa chỉ công khai.
setup-verify-wallet-waiting = Đang chờ xác thực
setup-verify-rpc-testing = Đang kiểm tra Solana mainnet
setup-verify-rpc-testing-detail =
    { $count ->
       *[other] Đang kiểm tra { $count } endpoint.
    }
setup-verify-rpc-waiting = Đang chờ kiểm tra endpoint
setup-verify-save-waiting = Đang chờ lưu
setup-verify-save-running = Đang mã hóa và lưu
setup-verify-save-running-detail = Đang ghi cấu hình đã xác minh trên thiết bị này.
setup-verify-save-done = Đã lưu cấu hình
setup-verify-save-done-detail = Đã mã hóa khóa riêng tư; đã lưu các endpoint RPC hoạt động.
setup-verify-save-failed = Không thể lưu thiết lập
setup-verify-save-skipped = Chưa lưu
setup-verify-request-failed = Yêu cầu xác minh thất bại
setup-verify-summary-checking = Đang kiểm tra kết nối ví và Solana mainnet của bạn.
setup-verify-summary-running = Đang xác minh chính xác thông tin bạn đã nhập.
setup-verify-summary-saving = Đã xác minh thông tin. Đang lưu an toàn.
setup-verify-summary-failed = Hãy xem lại sự cố rồi xác minh lại.

setup-error-credentials-failed = Xác minh thông tin thất bại.
setup-error-save-failed = Không thể lưu thiết lập.
setup-error-verify-failed = Xác minh thất bại.
setup-error-explore-failed = Không thể bắt đầu Chế độ khám phá.
setup-error-gateway-failed = Không thể lưu tùy chọn gateway.
setup-action-review-credentials = Xem lại thông tin

setup-explore-opening = Đang mở Chế độ khám phá…
setup-complete-restarting = Đang khởi động lại { -brand } với cấu hình đã xác minh của bạn.
setup-complete-finishing = Đang hoàn tất khởi động lại…
setup-complete-ready = { -brand } đã sẵn sàng. Đang mở bảng điều khiển…
setup-complete-stored = Cấu hình đã xác minh của bạn được lưu an toàn trên thiết bị này.

setup-wallet-show-key = Hiện khóa riêng tư
setup-wallet-hide-key = Ẩn khóa riêng tư
setup-wallet-copy =
    .aria-label = Sao chép địa chỉ ví
    .title = Sao chép địa chỉ ví
setup-wallet-copy-done =
    .aria-label = Đã sao chép địa chỉ ví
    .title = Đã sao chép
setup-wallet-copy-failed =
    .aria-label = Không thể sao chép địa chỉ ví
    .title = Sao chép thất bại

setup-dialog-title = Thiết lập ví & RPC
setup-dialog-subtitle = Kết nối ví Solana và endpoint RPC cao cấp để bật giao dịch và dữ liệu on-chain trực tiếp. Khóa riêng tư của bạn được mã hóa trên thiết bị này và không bao giờ rời khỏi thiết bị.
setup-dialog-close =
    .title = Đóng
    .aria-label = Đóng
setup-dialog-wallet-label = Khóa riêng tư của ví
setup-dialog-wallet-input =
    .placeholder = Chuỗi base58 hoặc mảng JSON [1,2,3,...]
setup-dialog-rpc-label = Endpoint RPC
setup-dialog-rpc-input =
    .placeholder = https://your-endpoint... (mỗi dòng một endpoint)
setup-dialog-rpc-hint = Nên dùng nhà cung cấp cao cấp ({ -helius }, { -quicknode }, { -alchemy }) - RPC Solana công khai bị giới hạn tốc độ và có thể không hoạt động.
setup-dialog-submit = Xác thực & kết nối
setup-dialog-working = Đang xử lý…
setup-dialog-validating = Đang xác thực…
setup-dialog-saving = Đang lưu…
setup-dialog-restarting = Đang khởi động lại…
setup-dialog-saved = Đã lưu thiết lập - đang khởi động lại { -brand } ở chế độ đầy đủ…
setup-dialog-error-missing-fields = Hãy nhập cả khóa riêng tư của ví và ít nhất một URL RPC.
setup-dialog-error-validation = Xác thực thất bại.
setup-dialog-error-incomplete = Không thể hoàn tất thiết lập.
setup-dialog-error-restart-helper = Trình hỗ trợ khởi động lại tự động không khả dụng. Hãy tải lại bảng điều khiển sau ít phút.
setup-dialog-error-unexpected = Lỗi không mong đợi.

setup-wizard-progress =
    .aria-label = Tiến độ thiết lập
setup-wizard-step-credentials = Thông tin đăng nhập
setup-wizard-step-verification = Xác minh
setup-wizard-step-complete = Hoàn tất
setup-wizard-credentials-title = Cấu hình thông tin đăng nhập
setup-wizard-credentials-description = Kết nối ví cục bộ và các endpoint RPC Solana mainnet ổn định.
setup-wizard-wallet-toggle =
    .title = Hiện khóa riêng tư
    .aria-label = Hiện khóa riêng tư
setup-wizard-wallet-security-note = Được mã hóa trước khi lưu.
setup-wizard-rpc-title = Endpoint RPC
setup-wizard-rpc-input =
    .placeholder = Mỗi dòng một URL HTTPS
setup-wizard-rpc-guidance = Nên dùng RPC mainnet ổn định để polling liên tục.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = nên dùng
setup-wizard-gateway-title = Gửi giao dịch miễn phí
setup-wizard-gateway-hint = Khả dụng khi đã đăng nhập. RPC của bạn vẫn được dùng làm phương án dự phòng.
setup-wizard-account-title = Tài khoản { -brand }
setup-wizard-account-optional = Tùy chọn
setup-wizard-account-loading = Đang kiểm tra trạng thái tài khoản…
setup-wizard-verify-title = Xác minh và lưu
setup-wizard-verify-list =
    .aria-label = Trạng thái xác minh thiết lập
setup-wizard-verify-wallet = Ví
setup-wizard-verify-rpc = RPC Solana
setup-wizard-verify-save = Cấu hình bảo mật
setup-wizard-complete-title = Đã lưu thiết lập
setup-wizard-reconnect = Thử kết nối lại
setup-wizard-reload = Tải lại bảng điều khiển
setup-wizard-error-title = Thiết lập cần được xử lý
setup-wizard-explore = Khám phá bảng điều khiển
setup-wizard-continue = Tiếp tục
