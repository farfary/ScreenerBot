settings-duration-minutes =
    { $count ->
       *[other] { $count } phút
    }
settings-duration-hours =
    { $count ->
       *[other] { $count } giờ
    }

settings-dialog-title = Cài đặt
settings-dialog-close =
    .title = Đóng (ESC)
    .aria-label = Đóng cài đặt
settings-dialog-save = Lưu thay đổi
settings-dialog-saving = Đang lưu...
settings-dialog-saved = Đã lưu
settings-dialog-save-success = Đã lưu cài đặt
settings-dialog-save-failed = Không thể lưu cài đặt
settings-dialog-update-attention = Bản cập nhật cần được chú ý
settings-dialog-tab-interface = Giao diện
settings-dialog-tab-navigation = Điều hướng
settings-dialog-tab-startup = Khởi động
settings-dialog-tab-hints = Gợi ý
settings-dialog-tab-data = Dữ liệu
settings-dialog-tab-security = Bảo mật
settings-dialog-tab-account = Tài khoản
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = Kết nối agent
settings-dialog-tab-updates = Cập nhật
settings-dialog-tab-licenses = Giấy phép
settings-dialog-tab-about = Giới thiệu
settings-dialog-link-privacy = Chính sách quyền riêng tư
settings-dialog-link-terms = Điều khoản dịch vụ

settings-startup-section-title = Hành vi khởi động
settings-startup-auto-start-label = Tự động khởi động Trader
settings-startup-auto-start-hint = Tự động bật Trader khi mở ứng dụng
settings-startup-coming-soon = Sắp ra mắt
settings-startup-default-page-label = Trang mặc định
settings-startup-default-page-hint = Trang hiển thị khi mở ứng dụng
settings-startup-page-dashboard = Bảng điều khiển
settings-startup-page-tokens = Token
settings-startup-page-positions = Vị thế
settings-startup-page-wallet = Ví
settings-startup-page-config = Cấu hình
settings-startup-notifications-label = Hiện thông báo nền
settings-startup-notifications-hint = Hiển thị thông báo cho các sự kiện chạy nền

settings-about-tagline = Công cụ giao dịch Solana gốc
settings-about-link-github = { -github }
settings-about-link-docs = Tài liệu
settings-about-link-telegram = { -telegram }
settings-about-link-website = Website
settings-about-credits = Được xây dựng cho trader Solana
settings-about-copyright = © { $year } { -brand }. Bảo lưu mọi quyền.

settings-interface-section-appearance = Giao diện hiển thị
settings-interface-theme-label = Chủ đề
settings-interface-theme-hint = Chọn bảng màu bạn thích
settings-interface-theme-dark = Tối
settings-interface-theme-light = Sáng
settings-interface-language-label = Ngôn ngữ
settings-interface-language-hint = Ngôn ngữ hiển thị của bảng điều khiển
settings-interface-logo-shape-label = Hình dạng logo token
settings-interface-logo-shape-hint = Hình tròn sẽ cắt mọi logo; Tự nhiên giữ nguyên đường viền riêng của từng hình ảnh
settings-interface-logo-shape-circle = Hình tròn
settings-interface-logo-shape-natural = Tự nhiên
settings-interface-animations-label = Bật hiệu ứng động
settings-interface-animations-hint = Chuyển cảnh và hiệu ứng mượt mà
settings-interface-compact-label = Chế độ gọn
settings-interface-compact-hint = Giảm khoảng đệm để hiển thị nhiều nội dung hơn
settings-interface-section-data = Dữ liệu và hiển thị
settings-interface-refresh-label = Chu kỳ làm mới
settings-interface-refresh-hint = Tần suất làm mới dữ liệu
settings-interface-refresh-seconds =
    { $count ->
       *[other] { $count } giây
    }
settings-interface-refresh-minutes =
    { $count ->
       *[other] { $count } phút
    }
settings-interface-ticker-label = Hiện thanh ticker
settings-interface-ticker-hint = Thanh chỉ số trực tiếp ở phần đầu trang
settings-interface-page-size-label = Số dòng mỗi trang bảng
settings-interface-page-size-hint = Số dòng mặc định trên mỗi trang bảng
settings-interface-page-size-rows =
    { $count ->
       *[other] { $count } dòng
    }
settings-interface-auto-expand-label = Tự động mở rộng danh mục
settings-interface-auto-expand-hint = Mặc định mở rộng các danh mục cấu hình
settings-interface-hints-label = Hiện gợi ý theo ngữ cảnh
settings-interface-hints-hint = Hiển thị biểu tượng trợ giúp giải thích các tính năng của bảng điều khiển
settings-interface-featured-label = Hiện hàng nổi bật
settings-interface-featured-hint = Hiển thị hàng token nổi bật ở trang chủ và trang Token
settings-interface-section-sound = Hiệu ứng âm thanh
settings-interface-sounds-label = Bật âm thanh
settings-interface-sounds-hint = Âm báo phản hồi cho điều hướng, thay đổi trạng thái và kết quả

settings-security-loading = Đang tải cài đặt bảo mật...
settings-security-load-failed = Không thể tải cài đặt bảo mật

settings-security-type-pin4 = PIN 4 chữ số
settings-security-type-pin6 = PIN 6 chữ số
settings-security-type-text = Mật khẩu dạng văn bản
settings-security-type-unset = Chưa đặt

settings-security-lockscreen-title = Màn hình khóa bảng điều khiển
settings-security-lockscreen-description = Bảo vệ bảng điều khiển bằng mã PIN hoặc mật khẩu. Màn hình khóa sẽ xuất hiện khi được kích hoạt và yêu cầu xác thực để tiếp tục.
settings-security-enable-label = Bật màn hình khóa
settings-security-enable-hint = Bảo vệ bảng điều khiển bằng xác thực mật khẩu
settings-security-password-status-label = Trạng thái mật khẩu
settings-security-password-current = Hiện tại: { $type }
settings-security-password-none = Chưa đặt mật khẩu
settings-security-change = Đổi
settings-security-remove = Xóa
settings-security-set-password = Đặt mật khẩu
settings-security-auto-lock-label = Tự động khóa khi không hoạt động
settings-security-auto-lock-hint = Tự động khóa sau một khoảng thời gian không hoạt động
settings-security-auto-lock-never = Không bao giờ
settings-security-lock-blur-label = Khóa khi cửa sổ mất tiêu điểm
settings-security-lock-blur-hint = Tự động khóa khi bạn chuyển sang ứng dụng khác
settings-security-quick-actions-title = Thao tác nhanh
settings-security-lock-now-label = Khóa bảng điều khiển ngay
settings-security-lock-now-hint = Khóa bảng điều khiển ngay lập tức
settings-security-lock-now = Khóa ngay
settings-security-lock-not-ready = Không thể khóa - màn hình khóa chưa sẵn sàng
settings-security-setting-save-failed = Không thể lưu cài đặt bảo mật

settings-security-2fa-title = Xác thực hai yếu tố
settings-security-2fa-description = Thêm một lớp bảo mật bằng ứng dụng xác thực (Google Authenticator, Authy, v.v.)
settings-security-2fa-status-label = Trạng thái 2FA
settings-security-2fa-status-enabled = Xác thực hai yếu tố đã được bật
settings-security-2fa-status-none = Chưa cấu hình
settings-security-2fa-disable = Tắt 2FA
settings-security-2fa-enable = Bật 2FA

settings-security-modal-close =
    .aria-label = Đóng
settings-security-password-set-title = Đặt mật khẩu
settings-security-password-change-title = Đổi mật khẩu
settings-security-password-current-label = Mật khẩu hiện tại
settings-security-password-current-input =
    .placeholder = Nhập mật khẩu hiện tại
settings-security-password-type-label = Loại mật khẩu
settings-security-password-new-label = Mật khẩu mới
settings-security-password-new-input =
    .placeholder = Nhập mật khẩu mới
settings-security-password-confirm-label = Xác nhận mật khẩu
settings-security-password-confirm-input =
    .placeholder = Xác nhận mật khẩu
settings-security-password-update = Cập nhật mật khẩu
settings-security-placeholder-pin4 = Nhập PIN 4 chữ số
settings-security-placeholder-pin6 = Nhập PIN 6 chữ số
settings-security-placeholder-text = Nhập mật khẩu
settings-security-password-required = Vui lòng nhập mật khẩu
settings-security-password-mismatch = Mật khẩu không khớp
settings-security-pin4-invalid = PIN phải gồm đúng 4 chữ số
settings-security-pin6-invalid = PIN phải gồm đúng 6 chữ số
settings-security-text-too-short = Mật khẩu phải có ít nhất 4 ký tự
settings-security-password-saved = Đã lưu mật khẩu
settings-security-password-save-failed = Không thể lưu mật khẩu
settings-security-password-save-failed-detail = Không thể lưu mật khẩu: { $message }

settings-security-remove-title = Xóa mật khẩu
settings-security-remove-description = Nhập mật khẩu hiện tại để gỡ bảo vệ màn hình khóa.
settings-security-remove-confirm = Xóa mật khẩu
settings-security-current-required = Vui lòng nhập mật khẩu hiện tại
settings-security-password-removed = Đã xóa mật khẩu
settings-security-password-remove-failed = Không thể xóa mật khẩu
settings-security-password-remove-failed-detail = Không thể xóa mật khẩu: { $message }

settings-security-2fa-enable-title = Bật xác thực hai yếu tố
settings-security-2fa-password-prompt = Nhập mật khẩu của bạn để tiếp tục:
settings-security-2fa-password-input =
    .placeholder = Nhập mật khẩu
settings-security-2fa-continue = Tiếp tục
settings-security-2fa-manual-code = Mã nhập thủ công:
settings-security-2fa-qr =
    .alt = Mã QR TOTP
settings-security-2fa-code-prompt = Nhập mã 6 chữ số từ ứng dụng xác thực của bạn:
settings-security-2fa-verify-enable = Xác minh và bật
settings-security-2fa-password-required = Vui lòng nhập mật khẩu của bạn
settings-security-2fa-setup-failed = Không thể thiết lập 2FA
settings-security-2fa-code-invalid-length = Vui lòng nhập mã 6 chữ số
settings-security-2fa-code-invalid = Mã không hợp lệ
settings-security-2fa-enabled = Đã bật xác thực hai yếu tố
settings-security-2fa-verify-failed = Không thể xác minh mã
settings-security-2fa-disable-title = Tắt xác thực hai yếu tố
settings-security-2fa-disable-prompt = Nhập mật khẩu của bạn để tắt 2FA:
settings-security-2fa-disable-failed = Không thể tắt 2FA
settings-security-2fa-disabled = Đã tắt xác thực hai yếu tố

settings-agent-category-analysis = Phân tích
settings-agent-category-portfolio = Danh mục
settings-agent-category-trading = Giao dịch
settings-agent-category-config = Cấu hình
settings-agent-category-system = Hệ thống
settings-agent-category-analysis-description = Phân tích token, dữ liệu thị trường và kiểm tra bảo mật.
settings-agent-category-portfolio-description = Vị thế đang mở, số dư và P&L.
settings-agent-category-trading-description = Mua, bán và đóng vị thế bằng tiền thật.
settings-agent-category-config-description = Mọi cài đặt của bot, gồm cả endpoint RPC. Không bao giờ gồm khóa ví.
settings-agent-category-system-description = Trạng thái, sự kiện và dừng khẩn cấp.
settings-agent-category-analysis-inline = phân tích
settings-agent-category-portfolio-inline = danh mục
settings-agent-category-trading-inline = giao dịch
settings-agent-category-config-inline = cấu hình
settings-agent-category-system-inline = hệ thống

settings-agent-level-allow = Cho phép
settings-agent-level-ask-user = Hỏi
settings-agent-level-deny = Tắt
settings-agent-level-allow-hint = Chạy ngay lập tức.
settings-agent-level-ask-user-hint = Chờ bạn duyệt trong ứng dụng.
settings-agent-level-deny-hint = Bị từ chối và ẩn khỏi agent.

settings-agent-preset-full = Toàn quyền
settings-agent-preset-ask = Hỏi trước
settings-agent-preset-read = Chỉ đọc
settings-agent-preset-full-description = Mọi thứ chạy mà không cần hỏi. Khóa ví vẫn không thể truy cập.
settings-agent-preset-ask-description = Mọi hành động đều chờ bạn duyệt trong ứng dụng.
settings-agent-preset-read-description = Chỉ đọc phân tích và danh mục. Không thể thay đổi gì.
settings-agent-preset-custom = Tùy chỉnh
settings-agent-preset-group =
    .aria-label = Mẫu quyền
settings-agent-permission-group = Quyền { $category }

settings-agent-summary-asks-only = Giới hạn — hỏi trước với { $asking }
settings-agent-summary-off-only = Giới hạn — không có { $off }
settings-agent-summary-asks-and-off = Giới hạn — hỏi trước với { $asking }; không có { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = MCP stdio chung

settings-agent-note-placeholder = Thay /absolute/path/to/screenerbot bằng đường dẫn tuyệt đối tới tệp nhị phân { -brand } của bạn — ứng dụng đang chạy không thể xác định đường dẫn tệp thực thi của nó trên hệ thống này.
settings-agent-note-data-dir = Nếu bạn chạy { -brand } với thư mục dữ liệu không mặc định, hãy đặt thêm SCREENERBOT_DATA_DIR trên client (thêm một cờ -e / --env khác, hoặc một mục env) trỏ tới cùng đường dẫn đó.
settings-agent-note-codex-run = Chạy lệnh, hoặc thêm khối TOML vào ~/.codex/config.toml ($CODEX_HOME/config.toml). Sau đó khởi động lại { -codex }.
settings-agent-note-codex-get = `codex mcp get screenerbot` che khóa bí mật trong kết quả xuất.
settings-agent-note-claude-code = { -claude } Code: chạy lệnh, rồi khởi động lại { -claude } Code. `claude mcp get screenerbot` sẽ in ra môi trường đã cấu hình, gồm cả khóa bí mật.
settings-agent-note-claude-desktop = { -claude } Desktop: gộp JSON vào claude_desktop_config.json trong `mcpServers` rồi khởi động lại ứng dụng.
settings-agent-note-openclaw = Chạy lệnh, sau đó dùng `openclaw mcp doctor screenerbot --probe` để xác minh máy chủ stdio đã lưu khởi động được và cung cấp các công cụ.
settings-agent-note-hermes = Thêm đoạn này vào `mcp_servers` trong tệp cấu hình của { -hermes }, rồi khởi động lại { -hermes }.
settings-agent-note-generic = Mọi client MCP hỗ trợ stdio: chạy lệnh này với các đối số và môi trường này, ở nơi client lưu danh sách máy chủ của nó.
settings-agent-block-codex-command = { -codex } CLI — lệnh terminal
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (dự phòng)
settings-agent-block-claude-command = { -claude } Code — lệnh terminal
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — lệnh terminal
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = Client MCP stdio chung

settings-agent-name-required = Hãy nhập tên cho kết nối này.
settings-agent-name-too-long = Tên không được dài quá { $max } ký tự.
settings-agent-name-control-characters = Tên không được chứa ký tự điều khiển.

settings-agent-title = Kết nối agent
settings-agent-description = Kết nối { -claude }, { -codex }, { -hermes }, { -openclaw } hoặc bất kỳ client MCP stdio nào. { -brand } phải luôn chạy. Mỗi kết nối có quyền riêng: mặc định toàn quyền, có thể giới hạn theo từng kết nối bất cứ lúc nào. Không kết nối nào có thể đọc hoặc thay đổi khóa ví của bạn.
settings-agent-name-label = Tên kết nối
settings-agent-name-hint = Hiển thị trong danh sách bên dưới để bạn phân biệt các kết nối.
settings-agent-name-input =
    .placeholder = Agent lập trình trên laptop
settings-agent-client-label = Client
settings-agent-client-hint = Chọn cách thiết lập hiển thị sau khi tạo kết nối.
settings-agent-permissions-label = Quyền
settings-agent-permissions-hint = Kết nối mới có thể làm mọi thứ. Hãy giới hạn bất kỳ danh mục nào ngay bây giờ hoặc sau này từ danh sách bên dưới — khóa ví không bao giờ truy cập được trong cả hai trường hợp.
settings-agent-create = Tạo kết nối
settings-agent-issued-group =
    .aria-label = Thông tin xác thực của kết nối mới
settings-agent-issued-warning = Hãy sao chép khóa bí mật ngay. Khóa chỉ hiển thị một lần và không thể lấy lại — hãy thu hồi và tạo lại kết nối nếu bạn làm mất. { -brand } chỉ giữ một bản xác minh một chiều; client MCP của bạn lưu bản gốc trong cấu hình riêng của nó.
settings-agent-issued-client-id = Client ID
settings-agent-issued-secret = Khóa bí mật dùng một lần
settings-agent-setup-for = Thiết lập cho
settings-agent-done = Xong
settings-agent-list-title = Kết nối
settings-agent-loading = Đang tải kết nối...
settings-agent-active-count = { $count } đang hoạt động
settings-agent-empty = Chưa có kết nối nào. Hãy tạo một kết nối ở trên để ghép nối client.
settings-agent-empty-active = Không có kết nối đang hoạt động.
settings-agent-revoked-title = Kết nối đã thu hồi
settings-agent-created = Đã tạo { $time }
settings-agent-last-used = Dùng lần cuối { $time }
settings-agent-never-used = Chưa từng dùng
settings-agent-permissions-edit = Quyền
settings-agent-revoke = Thu hồi
settings-agent-permissions-save = Lưu quyền

settings-agent-load-failed = Không thể tải Kết nối agent
settings-agent-list-failed = Không thể tải danh sách kết nối
settings-agent-create-failed = Không thể tạo kết nối.
settings-agent-unreachable-create = Không thể kết nối tới { -brand } để tạo kết nối.
settings-agent-permissions-update-failed = Không thể cập nhật quyền
settings-agent-permissions-updated = Đã cập nhật quyền
settings-agent-permissions-updated-detail = Áp dụng cho yêu cầu tiếp theo của kết nối.
settings-agent-unreachable-save = Không thể kết nối tới { -brand } để lưu
settings-agent-revoke-title = Thu hồi kết nối
settings-agent-revoke-message = Thu hồi "{ $label }"? Client sẽ ngừng hoạt động từ yêu cầu tiếp theo và không thể khôi phục.
settings-agent-revoke-fallback-name = kết nối này
settings-agent-revoke-failed = Không thể thu hồi kết nối
settings-agent-unreachable-revoke = Không thể kết nối tới { -brand } để thu hồi

settings-telegram-loading = Đang tải cài đặt { -telegram }...
settings-telegram-load-failed = Không thể tải cài đặt { -telegram }
settings-telegram-unknown = Không rõ
settings-telegram-session-active = Đang hoạt động: { $duration }
settings-telegram-sessions-empty = Không có phiên đang hoạt động
settings-telegram-session-revoke = Thu hồi

settings-telegram-connection-title = Kết nối
settings-telegram-connection-description = Kết nối bot { -telegram } của bạn để nhận thông báo và điều khiển { -brand } từ xa.
settings-telegram-enable-label = Bật { -telegram }
settings-telegram-enable-hint = Bật tích hợp bot { -telegram }
settings-telegram-token-label = Bot token
settings-telegram-token-saved = Đã lưu token
settings-telegram-token-help = Lấy token từ @BotFather trên { -telegram }
settings-telegram-token-input-saved =
    .placeholder = Đã lưu token (nhập token mới để thay đổi)
settings-telegram-token-input =
    .placeholder = Nhập bot token
settings-telegram-token-toggle =
    .title = Hiện/Ẩn
settings-telegram-chat-label = Chat ID
settings-telegram-chat-connected = Đã kết nối với chat:
settings-telegram-chat-discover-hint = Tự động tìm chat ID của bạn
settings-telegram-chat-change =
    .title = Thay đổi
settings-telegram-chat-discover = Tìm Chat ID
settings-telegram-discovery-step-add = Thêm bot của bạn vào nhóm { -telegram }, hoặc bắt đầu chat riêng với bot
settings-telegram-discovery-step-privacy = Với nhóm: kiểm tra @BotFather → /mybots → [bot của bạn] → Bot Settings → Group Privacy
settings-telegram-discovery-privacy = <strong>Privacy Mode TẮT:</strong> Bot nhận mọi tin nhắn trong nhóm<br/><strong>Privacy Mode BẬT:</strong> Bot chỉ nhận tin nhắn khi được @mention
settings-telegram-discovery-step-send = Gửi một tin nhắn bất kỳ (hoặc @mention bot của bạn nếu Privacy Mode đang BẬT)
settings-telegram-discovery-listening = Đang chờ tin nhắn...
settings-telegram-discovery-select = Chọn
settings-telegram-chat-id-label = ID:
settings-telegram-language-label = Ngôn ngữ tin nhắn
settings-telegram-language-hint = Ngôn ngữ của tin nhắn và nút bấm của bot { -telegram }
settings-telegram-language-follow-app = Theo ngôn ngữ ứng dụng
settings-telegram-test-label = Kiểm tra kết nối
settings-telegram-test-hint = Gửi tin nhắn thử để xác minh cấu hình
settings-telegram-test-send = Gửi thử
settings-telegram-test-sending = Đang gửi...

settings-telegram-chat-type-private = riêng tư
settings-telegram-chat-type-group = nhóm
settings-telegram-chat-type-supergroup = siêu nhóm
settings-telegram-chat-type-channel = kênh

settings-telegram-auth-title = Xác thực lệnh
settings-telegram-auth-description = Các lệnh { -telegram } dùng chung 2FA với màn hình khóa bảng điều khiển.
settings-telegram-auth-protected = Đã bảo vệ
settings-telegram-auth-disabled = Đã tắt
settings-telegram-auth-not-configured = Chưa cấu hình
settings-telegram-auth-error = Lỗi
settings-telegram-auth-protected-note = Các lệnh được bảo vệ bằng 2FA của màn hình khóa. Khi phiên hết hạn, người dùng phải nhập mã từ ứng dụng xác thực qua lệnh <code>/login</code>.
settings-telegram-auth-disabled-note = 2FA của màn hình khóa đã được cấu hình nhưng bị tắt cho { -telegram }. Hãy bật "Yêu cầu 2FA cho lệnh" ở trên để bảo vệ các lệnh { -telegram }.
settings-telegram-auth-missing-note = 2FA của màn hình khóa chưa được cấu hình. Nếu không có 2FA, các phiên hết hạn sẽ tự kích hoạt lại mà không cần xác minh.
settings-telegram-auth-managed-in = 2FA được quản lý trong
settings-telegram-auth-configure-in = Cấu hình 2FA trong
settings-telegram-auth-configure-suffix = để yêu cầu xác minh cho các lệnh { -telegram }.
settings-telegram-security-link = Cài đặt bảo mật
settings-telegram-timeout-title = Thời gian chờ phiên
settings-telegram-timeout-description = Thời gian một phiên đã xác thực còn hiệu lực
settings-telegram-sessions-title = Phiên đang hoạt động

settings-telegram-notifications-title = Cài đặt thông báo
settings-telegram-notifications-description = Chọn sự kiện nào sẽ gửi thông báo { -telegram }.
settings-telegram-notify-opened-label = Vị thế đã mở
settings-telegram-notify-opened-hint = Thông báo khi mở vị thế mới
settings-telegram-notify-closed-label = Vị thế đã đóng
settings-telegram-notify-closed-hint = Thông báo khi vị thế được đóng
settings-telegram-notify-partial-label = Thoát một phần
settings-telegram-notify-partial-hint = Thông báo khi thoát một phần vị thế
settings-telegram-notify-dca-label = DCA đã thực hiện
settings-telegram-notify-dca-hint = Thông báo khi lệnh DCA được thực hiện
settings-telegram-notify-errors-label = Lỗi
settings-telegram-notify-errors-hint = Thông báo khi có lỗi và sự cố
settings-telegram-notify-startup-label = Khởi động/Tắt
settings-telegram-notify-startup-hint = Thông báo khi bot khởi động hoặc dừng
settings-telegram-notify-filtering-label = Cảnh báo lọc
settings-telegram-notify-filtering-hint = Thông báo khi token mới đạt tiêu chí lọc
settings-telegram-notify-trades-label = Cảnh báo giao dịch
settings-telegram-notify-trades-hint = Thông báo về các giao dịch lớn của token đang theo dõi
settings-telegram-notify-daily-label = Tóm tắt hằng ngày
settings-telegram-notify-daily-hint = Nhận tóm tắt hoạt động giao dịch và P&L hằng ngày

settings-telegram-features-title = Tính năng
settings-telegram-features-description = Cấu hình khả năng của bot { -telegram }.
settings-telegram-commands-label = Bật lệnh
settings-telegram-commands-hint = Cho phép điều khiển bot qua lệnh { -telegram }
settings-telegram-require-2fa-label = Yêu cầu 2FA cho lệnh
settings-telegram-require-2fa-hint = Khi phiên hết hạn, yêu cầu mã 2FA để kích hoạt lại. Dùng 2FA của màn hình khóa.
settings-telegram-inline-label = Nút thao tác nội tuyến
settings-telegram-inline-hint = Hiển thị nút thao tác trong tin nhắn thông báo

settings-telegram-setting-save-failed = Không thể lưu cài đặt { -telegram }
settings-telegram-discovery-start-failed = Không thể bắt đầu tìm kiếm
settings-telegram-chat-selected = Đã chọn chat
settings-telegram-chat-select-failed = Không thể chọn chat
settings-telegram-test-sent = Đã gửi tin nhắn thử
settings-telegram-test-failed = Gửi tin nhắn thử thất bại
settings-telegram-session-revoked = Đã thu hồi phiên
settings-telegram-session-revoke-failed = Không thể thu hồi phiên

settings-licenses-title = Giấy phép mã nguồn mở
settings-licenses-subtitle = { -brand } được xây dựng bằng các phần mềm mã nguồn mở sau
settings-licenses-footer = Toàn văn giấy phép có trong kho lưu trữ dự án và trong mã nguồn của từng thư viện phụ thuộc.
settings-licenses-category-framework = Framework ứng dụng
settings-licenses-category-solana = Blockchain Solana
settings-licenses-category-data = Dữ liệu và lưu trữ
settings-licenses-category-networking = Mạng
settings-licenses-category-cryptography = Mật mã và mã hóa
settings-licenses-category-assets = Tài nguyên giao diện
settings-licenses-desc-electron = Framework ứng dụng desktop
settings-licenses-desc-tokio = Runtime bất đồng bộ cho Rust
settings-licenses-desc-axum = Framework máy chủ web
settings-licenses-desc-tower = Trừu tượng hóa dịch vụ
settings-licenses-desc-hyper = Triển khai HTTP
settings-licenses-desc-solana-sdk = Lõi Solana SDK
settings-licenses-desc-solana-client = Client RPC
settings-licenses-desc-solana-program = Thư viện chương trình
settings-licenses-desc-spl-token = Chương trình SPL Token
settings-licenses-desc-spl-token-2022 = Các phần mở rộng Token-2022
settings-licenses-desc-spl-associated-token-account = Tài khoản token liên kết
settings-licenses-desc-sqlite = Công cụ cơ sở dữ liệu nhúng
settings-licenses-desc-rusqlite = Binding SQLite cho Rust
settings-licenses-desc-r2d2 = Pool kết nối cơ sở dữ liệu
settings-licenses-desc-serde = Framework tuần tự hóa
settings-licenses-desc-toml = Phân tích cấu hình
settings-licenses-desc-reqwest = Client HTTP
settings-licenses-desc-tokio-tungstenite = Client WebSocket
settings-licenses-desc-rustls = Triển khai TLS
settings-licenses-desc-blake3 = Hàm băm
settings-licenses-desc-sha-2 = Băm SHA-256/512
settings-licenses-desc-bs58 = Mã hóa Base58
settings-licenses-desc-base64 = Mã hóa Base64
settings-licenses-desc-lucide-icons = Thư viện phông biểu tượng
settings-licenses-desc-inter = Phông chữ giao diện
settings-licenses-desc-jetbrains-mono = Phông chữ đơn cách
settings-licenses-desc-orbitron = Phông chữ tiêu đề
settings-licenses-desc-vazirmatn = Phông chữ tiếng Ả Rập và tiếng Ba Tư
settings-licenses-desc-noto-sans-devanagari = Phông chữ Devanagari
settings-licenses-desc-noto-sans-sc = Phông chữ tiếng Trung giản thể
settings-licenses-desc-pretendard = Phông chữ tiếng Hàn
settings-licenses-desc-pretendard-jp = Phông chữ tiếng Nhật

settings-hints-title = Gợi ý theo ngữ cảnh
settings-hints-description = Gợi ý theo ngữ cảnh là các biểu tượng trợ giúp giải thích tính năng của bảng điều khiển. Xem lại mọi gợi ý bên dưới và khôi phục những gợi ý bạn đã ẩn bằng "Không hiện lại" — từng cái một hoặc tất cả cùng lúc.
settings-hints-hidden-label = Gợi ý đã ẩn
settings-hints-hidden-summary = Hiện đang ẩn { $hidden } trên { $total } gợi ý.
settings-hints-restore-all = Khôi phục tất cả gợi ý
settings-hints-toggle-shown =
    .title = Hiện gợi ý này
settings-hints-toggle-shown-title = Đang hiện
settings-hints-toggle-hidden-title = Đang ẩn — bật để hiện
settings-hints-restore-title = Khôi phục tất cả gợi ý
settings-hints-restore-message = Hiện lại tất cả gợi ý theo ngữ cảnh, kể cả những gợi ý bạn đã ẩn?
settings-hints-restore-confirm = Khôi phục tất cả
settings-hints-restored = Đã khôi phục tất cả gợi ý

settings-account-title = Tài khoản { -brand }
settings-account-description = Miễn phí và tùy chọn. { -brand } vẫn giao dịch, khám phá và vẽ biểu đồ mà không cần tài khoản — chỉ là dùng các nhà cung cấp công khai. Bảng bên dưới liệt kê những gì đăng nhập mang lại.
settings-account-data-title = Dữ liệu { -brand }
settings-account-data-description = Chúng tôi vận hành dịch vụ dữ liệu thị trường dùng chung tại screenerbot.io: nến tổng hợp trên bảy khung thời gian, sổ đăng ký pool đã phân giải, báo cáo bảo mật được lưu đệm và danh tính token đã chuẩn hóa. Dịch vụ này giúp mỗi bản cài đặt không bị các nhà cung cấp công khai giới hạn tốc độ riêng lẻ, và cần có tài khoản để chi phí dùng chung đó gắn với một chủ thể cụ thể.
settings-account-data-fallback = Khi dịch vụ không khả dụng, { -brand } tự động chuyển sang các nhà cung cấp công khai. Mọi thứ vẫn chạy; biểu đồ tải chậm hơn và có ít lịch sử hơn.
settings-account-gateway-title = Gửi giao dịch
settings-account-gateway-description = Khi bạn đã đăng nhập, { -brand } có thể phát các swap của bạn qua screenerbot.io thay vì RPC của riêng bạn. Bot vẫn xây dựng và ký mọi giao dịch trên máy này — máy chủ chỉ chuyển tiếp, và không thể sửa giao dịch đã ký mà không làm chữ ký mất hiệu lực.
settings-account-gateway-label = Dùng RPC của { -brand } để gửi giao dịch
settings-account-gateway-hint = Chỉ để gửi giao dịch. Dữ liệu giá luôn lấy từ RPC của riêng bạn — việc thăm dò pool quá nặng đối với endpoint dùng chung nên không bao giờ gửi tới đó.
settings-account-manage-title = Quản lý tài khoản
settings-account-manage-description = Mật khẩu, địa chỉ email, thiết bị đã kết nối và khoản chi trả giới thiệu được quản lý trên website. Thu hồi một thiết bị ở đó sẽ đăng xuất thiết bị đó ở mọi nơi, kể cả thiết bị này.
settings-account-open-dashboard = Mở bảng điều khiển của bạn

settings-navigation-title = Các tab điều hướng
settings-navigation-hint = Kéo các mục để sắp xếp lại. Bật/tắt hiển thị bằng công tắc.
settings-navigation-section-layout = Bố cục
settings-navigation-overflow-label = Thẻ không vừa
settings-navigation-overflow-hint = Cuộn hàng thẻ sang ngang, hoặc gom các thẻ không vừa vào menu Thêm ở cuối hàng.
settings-navigation-overflow-scroll = Cuộn
settings-navigation-overflow-menu = Menu Thêm
settings-navigation-note = Thay đổi có hiệu lực sau khi lưu. Hãy làm mới trang để thấy cập nhật trên thanh điều hướng.
settings-navigation-drag-handle =
    .title = Kéo để sắp xếp lại
settings-navigation-defaults-failed = Không thể tải điều hướng mặc định
settings-navigation-reset = Đã đặt lại điều hướng về mặc định

settings-data-storage-title = Lưu trữ cơ sở dữ liệu
settings-data-storage-description = Tổng quan về tất cả cơ sở dữ liệu lưu dữ liệu giao dịch, vị thế và thông tin lịch sử của bạn.
settings-data-stats-loading = Đang tải thống kê cơ sở dữ liệu...
settings-data-stats-load-failed = Không thể tải thống kê cơ sở dữ liệu
settings-data-total-storage = Tổng dung lượng cơ sở dữ liệu
settings-data-db-tokens = Token
settings-data-db-transactions = Giao dịch
settings-data-db-positions = Vị thế
settings-data-db-events = Sự kiện
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = Ví
settings-data-db-pools = Pool
settings-data-db-strategies = Chiến lược
settings-data-db-actions = Hành động
settings-data-directory-label = Thư mục dữ liệu
settings-data-directory-copied = Thư mục dữ liệu
settings-data-config-path-copied = Đường dẫn cấu hình
settings-data-path-unavailable = Không khả dụng
settings-data-path-copy-title = Nhấn để sao chép đường dẫn
settings-data-path-copy-failed = Không thể sao chép đường dẫn

settings-data-config-title = Quản lý cấu hình
settings-data-config-description = Xuất, nhập và quản lý cấu hình bot của bạn. Hãy sao lưu trước khi thực hiện thay đổi lớn.
settings-data-config-export = Xuất cấu hình
settings-data-config-import = Nhập cấu hình
settings-data-config-reset = Đặt lại mặc định
settings-data-config-location-label = Vị trí cấu hình
settings-data-config-fetch-failed = Không thể lấy cấu hình
settings-data-config-exported = Đã xuất cấu hình
settings-data-config-export-failed = Không thể xuất cấu hình: { $message }
settings-data-config-import-title = Nhập cấu hình
settings-data-config-import-message = Nhập cấu hình này? Cài đặt hiện tại sẽ bị ghi đè. Thông tin xác thực ví sẽ được giữ nguyên.
settings-data-config-imported = Đã nhập cấu hình. Một số thay đổi có thể cần khởi động lại.
settings-data-config-import-failed = Không thể nhập cấu hình: { $message }
settings-data-config-reset-title = Đặt lại cấu hình
settings-data-config-reset-message = Đặt lại mọi cài đặt về mặc định? Thông tin xác thực ví của bạn sẽ được giữ nguyên, nhưng mọi cài đặt khác sẽ bị đặt lại.
settings-data-config-reset-done = Đã đặt lại cấu hình về mặc định
settings-data-config-reset-failed = Không thể đặt lại cấu hình: { $message }
settings-data-unknown-error = Lỗi không xác định

settings-data-cleanup-title = Dọn dẹp dữ liệu
settings-data-cleanup-description = Giải phóng dung lượng đĩa bằng cách xóa dữ liệu cũ hoặc không dùng. Các thao tác này không thể hoàn tác.
settings-data-ohlcv-cleanup-label = Dọn dữ liệu OHLCV
settings-data-ohlcv-cleanup-hint = Xóa dữ liệu nến của các token không hoạt động trong khoảng thời gian đã chỉ định.
settings-data-cleanup-hours-unit = giờ
settings-data-cleanup-ohlcv = Dọn OHLCV
settings-data-cleanup-running = Đang dọn...
settings-data-cleanup-hours-invalid = Giá trị giờ không hợp lệ
settings-data-cleanup-confirm-title = Xóa dữ liệu OHLCV
settings-data-cleanup-confirm-message =
    Xóa dữ liệu OHLCV của các token không hoạt động quá { $hours ->
       *[other] { $hours } giờ
    }?
settings-data-cleanup-done =
    Đã dọn { $count ->
       *[other] { $count } token không hoạt động
    }
settings-data-cleanup-failed = Dọn dẹp thất bại
settings-data-cleanup-failed-detail = Dọn dẹp thất bại: { $message }

settings-data-cache-clear-label = Xóa toàn bộ bộ nhớ đệm OHLCV
settings-data-cache-clear-hint = Xóa toàn bộ dữ liệu nến đã lưu đệm và lấy lại từ đầu cho mọi token đang theo dõi. Dùng khi biểu đồ có vẻ sai hoặc sau khi cập nhật logic dữ liệu.
settings-data-cache-clear = Xóa bộ nhớ đệm OHLCV
settings-data-cache-clearing = Đang xóa...
settings-data-cache-confirm-title = Xóa toàn bộ bộ nhớ đệm OHLCV
settings-data-cache-confirm-message = Xóa toàn bộ dữ liệu nến đã lưu đệm của mọi token? Các token đang theo dõi sẽ lấy lại lịch sử từ đầu. Không thể hoàn tác.
settings-data-candles-count =
    { $count ->
       *[other] { $count } nến
    }
settings-data-tokens-count =
    { $count ->
       *[other] { $count } token
    }
settings-data-cache-cleared = Đã xóa { $candles } trên { $tokens }; đang lấy lại
settings-data-cache-clear-failed = Không thể xóa bộ nhớ đệm OHLCV
settings-data-cache-clear-failed-detail = Không thể xóa bộ nhớ đệm OHLCV: { $message }

settings-data-ui-cache-label = Bộ nhớ đệm trạng thái giao diện
settings-data-ui-cache-hint = Xóa tùy chọn bảng, trạng thái bộ lọc và cài đặt chế độ xem đã lưu.
settings-data-ui-cache-clear = Xóa bộ nhớ đệm giao diện
settings-data-ui-cache-confirm-title = Xóa trạng thái giao diện
settings-data-ui-cache-confirm-message = Xóa mọi tùy chọn giao diện đã lưu? Thao tác này sẽ đặt lại cột bảng, bộ lọc và cài đặt chế độ xem.
settings-data-ui-cache-cleared =
    Đã xóa { $count ->
       *[other] { $count } cài đặt giao diện đã lưu đệm
    }

settings-data-folder-label = Mở thư mục dữ liệu
settings-data-folder-hint = Mở thư mục chứa toàn bộ dữ liệu { -brand } trong trình quản lý tệp của bạn.
settings-data-folder-open = Mở thư mục
settings-data-folder-open-failed = Không thể mở thư mục dữ liệu
