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

## Wallet replacement and wallet data reset.

startup-wallet-replacement-incomplete-title = Không thể hoàn tất việc thay ví
startup-wallet-replacement-incomplete-detail =
    { -brand } đã dừng khi đang chuyển sang ví giao dịch mới. Không có gì bị xóa: khóa của ví trước vẫn ở trong kho ví, còn lịch sử cục bộ của nó hoặc vẫn ở chỗ cũ, hoặc nằm trong thư mục sao lưu được ghi trong tệp nhật ký.

    Nguyên nhân:
    { $error }
startup-wallet-replacement-incomplete-remedy = Khởi động lại { -brand } để tiếp tục việc thay ví. Nếu thông báo này xuất hiện lại, hãy sao chép chi tiết và gửi kèm tệp nhật ký cho bộ phận hỗ trợ tại t.me/screenerbotio_support. Không chỉnh sửa, di chuyển hay xóa wallet_replacement.json hoặc thư mục sao lưu.
startup-wallet-reset-refused-title = Dữ liệu ví chưa được đặt lại
startup-wallet-reset-refused-open-positions = Ví giao dịch đang có vị thế mở ({ $count }), nên dữ liệu ví không được đặt lại.
startup-wallet-reset-refused-positions-unreadable = Không đọc được kho vị thế nên không thể loại trừ vị thế đang mở, và lịch sử cục bộ của ví được giữ lại.
startup-wallet-reset-refused-remedy = Không có gì được di chuyển. Hãy khởi động { -brand } mà không đặt lại, đóng hoặc lưu trữ các vị thế mở của ví giao dịch, rồi đặt lại dữ liệu ví lần nữa.
startup-wallet-reset-refused-unreadable-remedy = Không có gì được di chuyển. Hãy khởi động { -brand } mà không đặt lại để nó kiểm tra kho vị thế, rồi đặt lại dữ liệu ví lần nữa. Nếu thông báo này xuất hiện lại, hãy gửi tệp nhật ký cho bộ phận hỗ trợ tại t.me/screenerbotio_support.
startup-wallet-reset-incomplete-title = Không thể hoàn tất việc đặt lại dữ liệu ví
startup-wallet-reset-incomplete-detail =
    { -brand } đã dừng khi đang chuyển dữ liệu ví vào { $directory } và không thể đưa mọi tệp trở lại. Một phần lịch sử cục bộ của ví nằm trong thư mục đó và một phần vẫn ở chỗ cũ.

    Nguyên nhân:
    { $error }
startup-wallet-reset-incomplete-remedy = Không khởi động lại { -brand } và không di chuyển, chỉnh sửa hay xóa bất kỳ tệp nào trong { $directory }. Hãy sao chép chi tiết và gửi kèm tệp nhật ký cho bộ phận hỗ trợ tại t.me/screenerbotio_support.

## Generic failures.

startup-generic-title = { -brand } không thể khởi động
startup-generic-remedy = Hãy xem tệp nhật ký để biết chi tiết, sau đó khởi động lại ứng dụng. Nếu sự cố vẫn còn, hãy liên hệ hỗ trợ tại t.me/screenerbotio_support.
startup-generic-detail = { $error }
startup-failure-directories = Không tạo được các thư mục cần thiết: { $error }
startup-failure-config-load = Không tải được cấu hình: { $error }
startup-failure-actions-sync = Không đồng bộ được hành động từ cơ sở dữ liệu: { $error }
startup-failure-analysis-init = Không khởi tạo được công cụ phân tích: { $error }
startup-failure-assistant-init = Không khởi tạo được công cụ trò chuyện của Trợ lý: { $error }
startup-failure-wallets-init = Không khởi tạo được các ví: { $error }
startup-failure-wallet-validation = Không xác thực được tính nhất quán của ví: { $error }
startup-failure-storage-phase = Giai đoạn lưu trữ dừng đột ngột: { $error }
