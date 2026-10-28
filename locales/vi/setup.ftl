setup-wallet-required = Nhập khóa riêng tư của ví.
setup-wallet-json-recognized = Đã nhận dạng định dạng khóa JSON 64 byte.
setup-wallet-json-invalid = Hãy dùng mảng JSON chứa đúng 64 giá trị byte (0–255).
setup-wallet-format-invalid = Hãy dùng khóa riêng tư base58 hoặc mảng JSON 64 byte.
setup-wallet-base58-recognized = Đã nhận dạng định dạng khóa base58.

setup-rpc-required = Nhập ít nhất một endpoint RPC.
setup-dialog-rpc-optional-note = Không bắt buộc: để trống ô này để giữ các endpoint RPC đã cấu hình.
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
setup-error-restart-timeout = { -brand } chưa kết nối lại.

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
setup-verify-save-skipped = Chưa lưu. Hãy khắc phục sự cố ở trên rồi xác minh lại.
setup-verify-request-failed = Yêu cầu xác minh thất bại
setup-verify-summary-checking = Đang kiểm tra kết nối ví và Solana mainnet của bạn.
setup-verify-summary-running = Đang xác minh chính xác thông tin bạn đã nhập.
setup-verify-summary-saving = Đã xác minh thông tin. Đang lưu an toàn.
setup-verify-summary-failed = Hãy xem lại sự cố rồi xác minh lại.

setup-error-credentials-failed = Xác minh thông tin thất bại.
setup-error-save-failed = Không thể lưu thiết lập.
setup-error-verify-failed = Xác minh thất bại.
setup-error-explore-failed = Không thể bắt đầu Chế độ khám phá.
setup-error-gateway-failed = Không thể lưu cài đặt gửi giao dịch.
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
setup-dialog-submit = Xác thực & kết nối
setup-dialog-working = Đang xử lý…
setup-dialog-validating = Đang xác thực…
setup-dialog-saving = Đang lưu…
setup-dialog-restarting = Đang khởi động lại…
setup-dialog-saved = Đã lưu thiết lập - đang khởi động lại { -brand } ở chế độ đầy đủ…
setup-dialog-error-validation = Xác thực thất bại.
setup-dialog-error-incomplete = Không thể hoàn tất thiết lập.
setup-dialog-error-restart-helper = Trình hỗ trợ khởi động lại tự động không khả dụng. Hãy tải lại bảng điều khiển sau ít phút.
setup-dialog-error-unexpected = Lỗi không mong đợi.
setup-dialog-replace-title = Thay ví giao dịch
setup-dialog-replace-subtitle = Giao dịch bằng một ví khác. Khóa riêng của ví được mã hóa trên thiết bị này và không bao giờ rời khỏi nó.
setup-dialog-replace-submit = Xác thực & thay
setup-dialog-replace-saved = Đã chuẩn bị thay ví - đang khởi động lại { -brand } với ví mới…
setup-dialog-replace-confirm-intro = Khi thay ví giao dịch:
setup-dialog-replace-confirm-stop = Trader và copy trading sẽ dừng. Hãy bật lại chúng sau khi khởi động lại.
setup-dialog-replace-confirm-backup = Vị thế, giao dịch và lịch sử của ví hiện tại được chuyển vào một thư mục sao lưu.
setup-dialog-replace-confirm-keep = Khóa của ví hiện tại được giữ trong Ví khác, nơi bạn vẫn có thể xuất nó.
setup-dialog-replace-confirm-funds = Không chuyển tiền: token và { -sol } vẫn ở trong ví hiện tại.
setup-dialog-replace-confirm-rpc = Thay các endpoint RPC đã cấu hình bằng những endpoint bạn nhập; nếu để trống, các endpoint hiện tại được giữ nguyên.
setup-dialog-replace-confirm-restart = { -brand } khởi động lại với ví mới.

setup-wizard-progress =
    .aria-label = Tiến độ thiết lập
setup-wizard-wallet-security-note = Được mã hóa trước khi lưu.
setup-wizard-rpc-title = Endpoint RPC
setup-wizard-rpc-input =
    .placeholder = Mỗi dòng một URL HTTPS
setup-wizard-rpc-guidance = Hãy dùng nhà cung cấp mainnet ổn định; endpoint công khai không được hỗ trợ.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = nên dùng
setup-wizard-account-title = Tài khoản { -brand }
setup-wizard-account-optional = Tùy chọn
setup-wizard-account-loading = Đang kiểm tra trạng thái tài khoản…
setup-wizard-verify-title = Xác minh và lưu
setup-wizard-verify-list =
    .aria-label = Trạng thái xác minh thiết lập
setup-wizard-verify-wallet = Ví
setup-wizard-verify-save = Cấu hình bảo mật
setup-wizard-complete-title = Đã lưu thiết lập
setup-wizard-reconnect = Thử kết nối lại
setup-wizard-reload = Tải lại bảng điều khiển
setup-wizard-error-title = Thiết lập cần được xử lý
setup-wizard-explore = Khám phá bảng điều khiển
setup-wizard-continue = Tiếp tục

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = Ví
setup-wizard-step-connection = Kết nối
setup-wizard-step-verify = Kiểm tra
setup-wizard-step-ready = Sẵn sàng
setup-wallet-title = Thiết lập ví giao dịch của bạn
setup-wallet-description = Ví mà { -brand } dùng để giao dịch. Khóa của ví được mã hóa trên máy tính này và không bao giờ rời khỏi máy.
setup-wallet-source =
    .aria-label = Nguồn ví
setup-wallet-create-title = Tạo ví mới
setup-wallet-create-detail = { -brand } tạo một ví mới. Bạn lưu khóa của ví, rồi gửi { -sol } vào ví.
setup-wallet-import-title = Nhập ví
setup-wallet-import-detail = Dùng khóa riêng tư bạn đã có (base58 hoặc mảng JSON gồm 64 số).
setup-wallet-creating = Đang tạo ví của bạn…
setup-wallet-create-failed = Không thể tạo ví. Hãy thử lại.
setup-backup-title = Lưu khóa riêng tư của bạn
setup-backup-description = Ai có khóa này đều kiểm soát được ví. { -brand } không thể khôi phục khóa.
setup-backup-address = Địa chỉ
setup-backup-key = Khóa riêng tư
setup-backup-copy-address = Sao chép địa chỉ
setup-backup-download = Tải xuống
setup-backup-copied = Đã sao chép. Hãy xóa bộ nhớ tạm sau khi lưu.
setup-backup-machine-warning = Bản sao mã hóa mà { -brand } giữ chỉ mở được trên máy tính này. Nếu máy bị mất hoặc thay thế, chỉ khóa bạn lưu bây giờ mới khôi phục được ví. Hãy cất giữ khóa ngoại tuyến, ví dụ trên giấy hoặc trong trình quản lý mật khẩu.
setup-backup-ack-saved = Tôi đã lưu khóa riêng tư ở nơi an toàn, bên ngoài máy tính này.
setup-backup-ack-control = Tôi hiểu rằng bất kỳ ai có khóa này đều có thể lấy tiền trong ví.
setup-backup-ack-recovery = Tôi hiểu rằng { -brand } không thể khôi phục khóa này. Không có khóa, mất máy tính đồng nghĩa với mất tiền.
setup-backup-create-another = Tạo ví khác
setup-backup-replace-confirm = Bỏ khóa này và tạo ví mới? Chỉ làm vậy nếu bạn chưa gửi gì vào ví.
setup-connection-title = { -brand } nên kết nối với { -solana } như thế nào?
setup-connection-description = Bạn có thể đổi lại sau trong Cài đặt → Kết nối.
setup-connection-choice =
    .aria-label = Kết nối
setup-connection-service-title = Dịch vụ { -brand }
setup-connection-service-badge = Không cần RPC
setup-connection-service-detail = Dùng tài khoản { -brand } miễn phí của bạn.
setup-connection-service-prices = Giá được làm mới vài giây một lần
setup-connection-service-features = Copy trading, theo dõi ví, số dư của ví khác, lịch sử cũ, swap trực tiếp qua pool và giá pool trực tiếp cần RPC của riêng bạn
setup-connection-service-pause = Giao dịch tạm dừng nếu bạn đăng xuất hoặc không truy cập được screenerbot.io
setup-connection-service-privacy = Khóa riêng tư của bạn không bao giờ rời khỏi máy tính này: các swap được ký tại đây, và screenerbot.io chỉ dùng địa chỉ công khai của ví để đọc số dư và giao dịch
setup-connection-own-title = RPC của riêng bạn
setup-connection-own-badge = Nhanh nhất
setup-connection-own-detail = Giá khoảng hai lần mỗi giây và đầy đủ tính năng. Cần một endpoint RPC từ nhà cung cấp.
setup-connection-signin-required = Đăng nhập hoặc tạo tài khoản miễn phí để dùng dịch vụ { -brand }.
setup-connection-reauthorize = Đăng nhập lại để cho phép dịch vụ { -brand } truy vấn ví.
setup-connection-sending = Gửi giao dịch qua { -brand }
setup-connection-sending-hint = Có sẵn khi bạn đã đăng nhập. Khi không thể, RPC của bạn sẽ được dùng.
setup-wizard-account-required = Bắt buộc
setup-wizard-verify-connection = Kết nối
setup-verify-service-checking = Đang kiểm tra dịch vụ { -brand }
setup-verify-service-ready = Dịch vụ { -brand } đã sẵn sàng
setup-verify-service-busy = Dịch vụ { -brand } hiện đang bận. Hãy thử lại sau giây lát hoặc dùng RPC của riêng bạn.
setup-verify-service-daily-limit = Tài khoản của bạn đã đạt giới hạn hôm nay của dịch vụ { -brand }. Hãy thử lại sau 00:00 UTC hoặc dùng RPC của riêng bạn.
setup-verify-service-unreachable = Không truy cập được screenerbot.io. Hãy kiểm tra kết nối internet.
setup-verify-service-version = Dịch vụ { -brand } không còn hỗ trợ phiên bản này. Hãy cập nhật { -brand } để tiếp tục.
setup-verify-service-signin = Đăng nhập để dùng dịch vụ { -brand }.
setup-verify-save-done-service-detail = Đã mã hóa khóa riêng tư; đã chọn dịch vụ { -brand }.
setup-action-try-again = Thử lại
setup-action-use-own-rpc = Dùng RPC của tôi
setup-action-sign-in-again = Đăng nhập lại
setup-ready-title = { -brand } đã sẵn sàng
setup-fund-title = Nạp tiền vào ví giao dịch
setup-fund-waiting = Đang chờ { -sol }…
setup-fund-received = Đã nhận { $amount } { -sol }
setup-fund-balance-unavailable = Đang đọc số dư… Số dư sẽ tự cập nhật.
setup-fund-open-dashboard = Mở bảng điều khiển
