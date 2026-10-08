## Wallet mismatch.

startup-wallet-mismatch-title = Ví đã thay đổi
startup-wallet-mismatch-detail =
    Ví trong cấu hình của bạn không khớp với ví được ghi trong lịch sử cục bộ của máy tính này.

    Ví hiện tại: { $current }
    Ví trước đó: { $stored }

    Dữ liệu cục bộ bị ảnh hưởng: { $systems }

    Điều này thường xảy ra sau khi nhập một khóa riêng tư khác hoặc khôi phục một cấu hình khác. Giao dịch, vị thế và lịch sử thuộc về ví trước đó và phải được xóa trước khi ví mới có thể khởi động an toàn.
startup-wallet-mismatch-systems-default = Giao dịch, Vị thế, Lịch sử ví
startup-wallet-mismatch-remedy =
    Hãy xóa lịch sử cục bộ của ví trước đó để tiếp tục (cơ sở dữ liệu của bạn sẽ được tự động sao lưu trước):

      - Trong ứng dụng: chọn "{ $action }" bên dưới.
      - Từ terminal: chạy  screenerbot --clean-wallet-data

    Tiền trên chuỗi không bị ảnh hưởng; chỉ lịch sử giao dịch/vị thế cục bộ của máy tính này được đặt lại. Bản sao lưu được ghi tại:
      { $path }
startup-recovery-reset-wallet = Đặt lại dữ liệu ví và khởi động lại

## Port in use.

startup-port-in-use-title = Cổng mạng đang bận
startup-port-in-use-detail = Cổng bảng điều khiển { $address } đã được sử dụng.
startup-port-in-use-remedy = Một chương trình khác đang dùng cổng mà { -brand } cần. Hãy đóng chương trình đó, hoặc đổi cổng máy chủ web trong Cài đặt, rồi khởi động lại { -brand }.

## Another instance is running.

startup-lock-held-title = { -brand } đang chạy
startup-lock-held-detail = Một bản { -brand } khác đang chạy trên máy tính này nên không thể khởi động bản thứ hai.
startup-lock-held-remedy = Hãy chuyển sang cửa sổ đang mở. Nếu không thấy cửa sổ nào, hãy thoát mọi tiến trình { -brand } chạy nền rồi thử lại. Nếu sự cố vẫn còn sau khi khởi động lại máy, tệp khóa có thể đã cũ và có thể xóa khỏi thư mục dữ liệu (.screenerbot.lock).

## Configuration.

startup-config-invalid-title = Không đọc được cấu hình
startup-config-parse-detail = Không phân tích được config.toml: { $detail }
startup-config-load-parse-detail = Không tải được cấu hình: không phân tích được config.toml: { $detail }
startup-config-parse-remedy = Không đọc được tệp cấu hình của bạn. Hãy khôi phục bản sao lưu từ thư mục dữ liệu, hoặc đặt lại cấu hình về mặc định rồi thiết lập lại ví và RPC.
startup-config-load-parse-remedy = Hãy khôi phục một cấu hình hợp lệ hoặc hoàn tất thiết lập lại.
startup-option-invalid-title = Tùy chọn khởi động không hợp lệ
startup-option-invalid-remedy = Một tùy chọn dòng lệnh không hợp lệ. Hãy khởi động { -brand } mà không dùng tùy chọn đó, hoặc sửa lại rồi thử lại.

## Storage upgrade.

startup-storage-upgrade-title = Không thể nâng cấp dữ liệu của bạn
startup-storage-upgrade-detail =
    { -brand } không thể nâng cấp { $database } lên phiên bản này và đã dừng trước khi thay đổi. Dữ liệu của bạn không bị thay đổi.

    Nguyên nhân:
    { $error }
startup-storage-upgrade-remedy = Sao chép chi tiết và gửi kèm tệp nhật ký đến bộ phận hỗ trợ tại t.me/screenerbotio_support. Đừng chỉnh sửa, di chuyển hoặc xóa cơ sở dữ liệu: { -brand } sẽ mở lại sau khi bản sửa lỗi được cài đặt.

## Generic failures.

startup-generic-title = { -brand } không thể khởi động
startup-generic-remedy = Hãy xem tệp nhật ký để biết chi tiết, sau đó khởi động lại ứng dụng. Nếu sự cố vẫn còn, hãy liên hệ hỗ trợ tại t.me/screenerbotio_support.
startup-generic-detail = { $error }
startup-failure-directories = Không tạo được các thư mục cần thiết: { $error }
startup-failure-config-load = Không tải được cấu hình: { $error }
startup-failure-actions-init = Không khởi tạo được cơ sở dữ liệu hành động: { $error }
startup-failure-actions-sync = Không đồng bộ được hành động từ cơ sở dữ liệu: { $error }
startup-failure-strategy-init = Không khởi tạo được hệ thống chiến lược: { $error }
startup-failure-analysis-init = Không khởi tạo được công cụ phân tích: { $error }
startup-failure-assistant-init = Không khởi tạo được công cụ trò chuyện của Trợ lý: { $error }
startup-failure-wallets-init = Không khởi tạo được các ví: { $error }
startup-failure-wallet-validation = Không xác thực được tính nhất quán của ví: { $error }
