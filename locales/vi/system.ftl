system-result-config-differs = Cấu hình trong bộ nhớ khác với phiên bản trên đĩa
system-result-config-matches = Cấu hình trong bộ nhớ khớp với phiên bản trên đĩa

system-result-config-imported =
    Đã nhập thành công { $count ->
       *[other] { $count } phần
    }
system-result-config-imported-with-warnings =
    Đã nhập { $count ->
       *[other] { $count } phần
    } với { $warnings ->
       *[other] { $warnings } cảnh báo
    }: { $details }

system-config-search =
    .placeholder = Tìm cài đặt...
system-config-export-title =
    .title = Xuất cấu hình ra tệp
system-config-import-title =
    .title = Nhập cấu hình từ tệp
system-config-reload = Tải lại từ đĩa
system-config-reset-defaults = Đặt lại về mặc định
system-config-select-section = Chọn một phần cấu hình
system-config-select-section-details = Chọn một phần cấu hình để xem chi tiết.
system-config-no-metadata = Không có siêu dữ liệu cho <code>{ $section }</code>
system-config-technical-settings = Cài đặt kỹ thuật
system-config-expand-title = Mở rộng mọi phần và mọi cấu hình con lồng nhau
system-config-collapse-title = Thu gọn mọi phần và mọi cấu hình con lồng nhau
system-config-toolbar-no-changes = Không có thay đổi trong phần này
system-config-toolbar-section-changes =
    { $count ->
       *[other] <strong>{ $count }</strong> thay đổi trong phần này
    }
system-config-toolbar-total-changes =
    { $count ->
       *[other] Tổng <strong>{ $count }</strong> thay đổi
    }

system-config-loading = Đang tải cấu hình…
system-config-refreshing = Đang làm mới cấu hình…
system-config-saving-title = Đang lưu thay đổi…
system-config-saving-detail = Đang cập nhật cấu hình
system-config-validation-issues = <strong>Phát hiện lỗi kiểm tra.</strong> Vui lòng xem lại các trường được đánh dấu.

system-config-save-changes = Lưu thay đổi
system-config-saving = Đang lưu…
system-config-compare = So sánh với đĩa
system-config-revert-section = Hoàn tác phần này
system-config-summary-critical = { $count } quan trọng
system-config-summary-performance = { $count } hiệu năng
system-config-summary-pending =
    { $count ->
       *[other] { $count } thay đổi đang chờ
    }
system-config-summary-none = Không có tóm tắt siêu dữ liệu
system-config-fields-count =
    { $count ->
       *[other] { $count } trường
    }
system-config-chip-pending = { $fields } · { $pending } đang chờ
system-config-chip-visible = { $visible }/{ $fields }

system-config-field-default = Mặc định: { $value }
system-config-field-reset = Đặt lại về mặc định
system-config-array-invalid-title = Mục mảng không hợp lệ
system-config-json-invalid-title = JSON không hợp lệ
system-config-list-separator = { ", " }
system-config-array-invalid-integer =
    { $count ->
       *[other] Dòng { $lines } phải là số nguyên hợp lệ.
    }
system-config-array-invalid-number =
    { $count ->
       *[other] Dòng { $lines } phải là số hợp lệ.
    }
system-config-array-invalid-boolean =
    { $count ->
       *[other] Dòng { $lines } phải là giá trị đúng/sai hợp lệ.
    }
system-config-array-invalid-value =
    { $count ->
       *[other] Dòng { $lines } phải là giá trị hợp lệ.
    }

system-config-telegram-actions = Thao tác
system-config-telegram-test-title = Kiểm tra kết nối
system-config-telegram-test-description = Gửi tin nhắn thử để xác nhận cấu hình { -telegram } của bạn hoạt động
system-config-telegram-send-test = Gửi tin nhắn thử
system-config-telegram-sending = Đang gửi...
system-config-telegram-configure-token-title = Hãy cấu hình token bot trước
system-config-telegram-configure-token-status = Cấu hình token bot ở trên để bật kiểm tra
system-config-telegram-test-sent-status = Đã gửi tin nhắn thử thành công! Hãy kiểm tra { -telegram } của bạn.
system-config-telegram-test-sent = Đã gửi tin nhắn thử { -telegram }
system-config-telegram-test-failed = Không thể gửi tin nhắn thử
system-config-telegram-auth-title = Xác thực bot
system-config-telegram-totp-title = Xác thực hai yếu tố (TOTP)
system-config-telegram-totp-configured = Đã cấu hình
system-config-telegram-totp-not-configured = Chưa cấu hình
system-config-telegram-totp-active = Xác thực hai yếu tố đang bật. Phiên { -telegram } đã hết hạn cần mã TOTP từ ứng dụng xác thực của bạn.
system-config-telegram-totp-inactive = Bật xác thực hai yếu tố trong cài đặt Bảo mật để bảo vệ các lệnh { -telegram }.
system-config-telegram-totp-note = TOTP dùng chung với màn hình khóa của bảng điều khiển. Hãy cấu hình trong cài đặt Bảo mật.
system-config-telegram-require-2fa = Yêu cầu 2FA cho các lệnh
system-config-telegram-save-rejected = Lưu bị từ chối ({ $status })
system-config-telegram-save-failed = Không thể lưu cài đặt { -telegram }

system-config-saved = Đã lưu cấu hình
system-config-save-failed = Không thể lưu cấu hình
system-config-reloaded = Đã tải lại cấu hình từ đĩa
system-config-reload-failed = Không thể tải lại cấu hình
system-config-diff-title = Khác biệt cấu hình
system-config-diff-console = Đã ghi vào console của trình duyệt
system-config-diff-failed = Không thể tính khác biệt
system-config-reset-title = Đặt lại cấu hình
system-config-reset-message =
    Thao tác này sẽ đặt lại toàn bộ cấu hình về giá trị mặc định tích hợp. Mọi cài đặt hiện tại sẽ bị mất.

    Không thể hoàn tác thao tác này.
system-config-reset-done-title = Đã đặt lại cấu hình
system-config-reset-done-message = Đã khôi phục mọi cài đặt về giá trị mặc định
system-config-reset-failed = Không thể đặt lại cấu hình
system-config-load-failed = Không thể tải cấu hình
system-config-metadata-failed = Không thể tải siêu dữ liệu cấu hình

system-config-dialog-close =
    .aria-label = Đóng
system-config-select-none = Bỏ chọn tất cả
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
       *[other] { $count } thay đổi
    }
system-config-sections-count =
    { $count ->
       *[other] { $count } phần
    }

system-config-section-hint-chains = Bật chuỗi, endpoint RPC và định tuyến swap
system-config-section-hint-trader = Quy tắc giao dịch và tự động hóa
system-config-section-hint-positions = Cài đặt quản lý vị thế
system-config-section-hint-filtering = Quy tắc và ngưỡng lọc token
system-config-section-hint-tokens = Khám phá token và nguồn dữ liệu
system-config-section-hint-events = Cài đặt ghi sự kiện
system-config-section-hint-services = Cài đặt dịch vụ nền
system-config-section-hint-monitoring = Cấu hình giám sát hệ thống
system-config-section-hint-ohlcv = Cài đặt dữ liệu nến
system-config-section-hint-gui = Cài đặt bảng điều khiển và giao diện
system-config-section-hint-telegram = Cấu hình bot { -telegram }

system-config-export-dialog-title = Xuất cấu hình
system-config-export-intro = Chọn các phần cấu hình cần xuất. Tệp đã xuất có thể được nhập lại sau này để khôi phục hoặc chia sẻ cài đặt.
system-config-export-sections = Các phần
system-config-export-timestamp = Bao gồm dấu thời gian xuất
system-config-sections-selected =
    { $count ->
       *[other] Đã chọn { $count } phần
    }
system-config-exporting = Đang xuất...
system-config-export-invalid-response = Phản hồi không hợp lệ từ máy chủ
system-config-exported-title = Đã xuất cấu hình
system-config-exported-message =
    { $count ->
       *[other] Đã xuất { $count } phần
    }
system-config-export-failed-title = Xuất thất bại
system-config-export-failed = Không thể xuất cấu hình

system-config-import-dialog-title = Nhập cấu hình
system-config-import-upload-intro = Tải lên tệp cấu hình đã xuất trước đó. Bạn có thể xem trước và chọn các phần cần nhập.
system-config-import-dropzone-title = Thả tệp cấu hình vào đây
system-config-import-dropzone-hint = hoặc nhấp để chọn
system-config-import-analyzing = Đang phân tích cấu hình...
system-config-import-preview = Xem trước
system-config-import-preview-intro = Xem lại các phần cấu hình bên dưới. Chọn các phần cần nhập.
system-config-import-sections = Các phần trong tệp
system-config-import-select-valid = Chọn tất cả phần hợp lệ
system-config-import-merge-label = Gộp với cấu hình hiện có
system-config-import-merge-hint = Chỉ cập nhật các trường có trong tệp. Bỏ chọn = thay thế toàn bộ phần.
system-config-import-save-label = Lưu vào đĩa
system-config-import-save-hint = Ghi thay đổi vào config.toml sau khi nhập
system-config-import-selected = Nhập mục đã chọn
system-config-import-warnings =
    { $count ->
       *[other] { $count } cảnh báo
    }
system-config-import-warning-unknown-section = Phần không xác định "{ $section }" sẽ bị bỏ qua
system-config-import-warning-sensitive-field = Nhập { $field } có thể ghi đè cài đặt xác thực
system-config-import-section-error = { $detail }
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = Không có trong tệp
system-config-import-status-invalid = Cấu hình không hợp lệ
system-config-import-status-unchanged = Không có thay đổi
system-config-import-not-included = Không có trong tệp
system-config-import-show-changes = Hiện thay đổi
system-config-import-hide-changes = Ẩn thay đổi
system-config-import-value-current = Giá trị hiện tại
system-config-import-value-new = Giá trị mới
system-config-import-more-changes =
    { $count ->
       *[other] +{ $count } thay đổi nữa
    }
system-config-import-value-items =
    { "[" }{ $count ->
       *[other] { $count } mục
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
       *[other] { $count } khóa
    }{ "}" }
system-config-importing = Đang nhập...
system-config-import-failed = Nhập thất bại
system-config-import-invalid-file-title = Tệp không hợp lệ
system-config-import-invalid-file = Không thể phân tích tệp cấu hình
system-config-imported-title = Đã nhập cấu hình
system-config-imported-message =
    { $count ->
       *[other] Đã nhập { $count } phần
    }
system-config-import-failed-title = Nhập thất bại
system-config-import-failed-message = Không thể nhập cấu hình
