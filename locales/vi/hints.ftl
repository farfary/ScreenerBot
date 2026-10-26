## Categories

hints-category-positions = Vị thế
hints-category-filtering = Lọc
hints-category-trader = Trader tự động
hints-category-services = Dịch vụ
hints-category-wallet = Ví
hints-category-wallets = Các ví
hints-category-config = Cấu hình
hints-category-config-telegram = { -telegram }
hints-category-token-details = Chi tiết token
hints-category-ui = Giao diện

## positions

hints-positions-overview-title = Tổng quan vị thế
hints-positions-overview-content =
    Các khoản token bạn đang nắm giữ và vị thế giao dịch hiện tại.

    Chỉ số chính:
    • **Điểm vào lệnh** — giá trung bình đã trả (gồm cả DCA)
    • **Giá hiện tại** — giá trực tiếp từ dự trữ của pool
    • **P&L** — lãi/lỗ chưa chốt tính theo SOL và %
    • **Quy mô** — tổng số token đang nắm giữ

    Nhấp vào bất kỳ vị thế nào để xem các tùy chọn quản lý chi tiết.
hints-positions-dca-title = DCA (trung bình giá)
hints-positions-dca-content =
    DCA cho phép thêm vào các vị thế hiện có ở những mức giá khác nhau.

    Khi DCA được kích hoạt:
    • Mua thêm token
    • Điểm vào lệnh được tính lại theo trung bình có trọng số
    • Quy mô vị thế tăng lên
    • Số lần vào lệnh tăng thêm

    Cấu hình quy tắc DCA trong cài đặt **Trader tự động**.
hints-positions-partial-exit-title = Thoát một phần
hints-positions-partial-exit-content =
    Bán một phần vị thế và giữ lại phần còn lại.

    Lợi ích:
    • Chốt một phần lợi nhuận mà vẫn giữ mức độ tiếp xúc
    • Giảm quy mô vị thế mà không đóng hoàn toàn
    • Triển khai các bậc take profit

    Mỗi lần thoát một phần được ghi riêng để theo dõi P&L chính xác.
hints-positions-management-title = Quản lý vị thế
hints-positions-management-content =
    Cách quản lý xác định tự động hóa nào được phép hành động trên một vị thế.

    • Trader tự động: thoát an toàn, thoát theo chính sách và DCA tự động
    • Chỉ người dùng: không có hành động tự động
    • Tác vụ copy: thoát an toàn và bán theo copy
    • Kết hợp: thoát an toàn, thoát theo chính sách và bán theo copy

    Bạn tự bán hoặc thêm vào vị thế. Các lệnh mua thủ công mặc định do bạn quản lý để bot không bán token mà bạn cố ý mua. Hãy tắt tùy chọn này để giao lại vị thế cho Trader tự động.

## filtering

hints-filtering-overview-title = Lọc token
hints-filtering-overview-content =
    Bộ lọc quyết định token nào đủ điều kiện để giao dịch.

    Token phải đạt **tất cả tiêu chí đang bật** mới xuất hiện trong danh sách đạt bộ lọc:
    • Chỉ số của { -dexscreener } (thanh khoản, khối lượng, v.v.)
    • Chỉ số của { -geckoterminal } (vốn hóa, FDV)
    • Phân tích bảo mật của { -rugcheck }
    • Bộ lọc meta (tuổi token, v.v.)

    Các tiêu chí đã tắt sẽ bị bỏ qua hoàn toàn.
hints-filtering-dexscreener-title = Bộ lọc { -dexscreener }
hints-filtering-dexscreener-content =
    Các bộ lọc dựa trên dữ liệu thị trường của { -dexscreener }.

    • **Thanh khoản** — thanh khoản USD tối thiểu trong pool
    • **Khối lượng 24h** — khối lượng giao dịch tối thiểu
    • **Giao dịch** — ngưỡng hoạt động (mua/bán)
    • **Biến động giá** — bộ lọc độ biến động

    Dữ liệu { -dexscreener } được cập nhật vài phút một lần.
hints-filtering-geckoterminal-title = Bộ lọc { -geckoterminal }
hints-filtering-geckoterminal-content =
    Các bộ lọc dựa trên dữ liệu thị trường của { -geckoterminal }.

    • **Vốn hóa** — vốn hóa thị trường tối thiểu
    • **FDV** — giới hạn giá trị pha loãng hoàn toàn
    • **Tỷ lệ dự trữ** — chỉ báo sức khỏe của pool

    { -geckoterminal } thường có dữ liệu cho các token mới hơn.
hints-filtering-rugcheck-title = Bộ lọc bảo mật
hints-filtering-rugcheck-content =
    Phân tích bảo mật từ { -rugcheck }.xyz.

    • **Điểm rủi ro** — xếp hạng rủi ro tổng thể (0-100)
    • **Quyền mint** — có thể mint thêm token mới không?
    • **Quyền freeze** — có thể đóng băng việc chuyển không?
    • **Holder lớn nhất** — rủi ro tập trung

    Điểm rủi ro càng cao thì càng có nhiều dấu hiệu đáng ngờ.
hints-filtering-meta-title = Bộ lọc meta
hints-filtering-meta-content =
    Các tiêu chí lọc bổ sung.

    • **Tuổi token** — thời gian tối thiểu kể từ khi tạo token
    • **Tuổi pool** — thời gian tối thiểu kể từ khi tạo pool
    • **Có website** — yêu cầu có liên kết mạng xã hội/website
    • **Có mạng xã hội** — yêu cầu có { -twitter }/{ -telegram }

    Các bộ lọc này giúp loại bỏ các token quá mới hoặc đáng ngờ.

## trader

hints-trader-overview-title = Trader tự động
hints-trader-overview-content =
    Công cụ giao dịch tự động giám sát token và thực hiện giao dịch.

    Thành phần:
    • **Giám sát vào lệnh** — theo dõi cơ hội mua
    • **Giám sát thoát lệnh** — quản lý lệnh bán và take profit
    • **Giám sát DCA** — xử lý việc trung bình giá vị thế
    • **Kiểm soát rủi ro** — giới hạn lỗ và các chốt an toàn

    Bật/tắt giao dịch từ bảng điều khiển.
hints-trader-entry-title = Giám sát vào lệnh
hints-trader-entry-content =
    Theo dõi các token đã lọc để tìm tín hiệu vào lệnh.

    Các kiểm tra khi đánh giá vào lệnh:
    • Token đạt bộ lọc hiện tại
    • Chưa có vị thế
    • Không nằm trong danh sách đen
    • Chưa vượt giới hạn vị thế
    • Đáp ứng điều kiện chiến lược (nếu có cấu hình)

    Cấu hình quy mô vào lệnh và giới hạn trong Cấu hình.
hints-trader-exit-title = Giám sát thoát lệnh
hints-trader-exit-content =
    Giám sát các vị thế đang mở để tìm tín hiệu thoát lệnh.

    Các điều kiện thoát:
    • **Take profit** — đạt giá mục tiêu
    • **Stop loss** — vượt mức lỗ tối đa
    • **Trailing stop** — giá quay đầu từ đỉnh
    • **Thoát theo chiến lược** — đáp ứng điều kiện tùy chỉnh
    • **Theo thời gian** — thời gian nắm giữ tối đa

    Cấu hình các ngưỡng trong Cấu hình.

## services

hints-services-overview-title = Dịch vụ hệ thống
hints-services-overview-content =
    Các dịch vụ chạy nền vận hành { -brand }.

    Trạng thái dịch vụ:
    • **Đang chạy** (xanh lá) — hoạt động bình thường
    • **Đang khởi động** (vàng) — đang khởi tạo
    • **Đã dừng** (đỏ) — không chạy
    • **Lỗi** (cảnh báo) — thất bại, có thể tự khởi động lại

    Các dịch vụ có phụ thuộc lẫn nhau và khởi động theo thứ tự.
hints-services-health-title = Sức khỏe dịch vụ
hints-services-health-content =
    Các chỉ báo sức khỏe cho biết trạng thái dịch vụ.

    • **Thời gian hoạt động** — thời gian kể từ lần khởi động gần nhất
    • **Tác vụ** — các thao tác nền đang hoạt động
    • **Lỗi** — số lỗi gần đây
    • **Chỉ số** — dữ liệu hiệu năng (nếu có)

    Các dịch vụ quan trọng ảnh hưởng đến khả năng giao dịch.

## wallet

hints-wallet-overview-title = Tổng quan ví
hints-wallet-overview-content =
    Trạng thái ví Solana đang kết nối của bạn.

    Hiển thị:
    • **Số dư SOL** — SOL gốc dùng cho phí và giao dịch
    • **Token đang nắm giữ** — token SPL kèm giá trị
    • **Biến động 24h** — thay đổi giá trị danh mục
    • **Lịch sử** — ảnh chụp nhanh số dư theo thời gian

    Số dư được làm mới mỗi phút.
hints-wallet-tokens-title = Số dư token
hints-wallet-tokens-content =
    Các token SPL đang nắm giữ trong ví của bạn.

    Hiển thị:
    • Ký hiệu và tên token
    • Số lượng đang nắm giữ
    • Giá trị hiện tại theo SOL/USD
    • Giá từ pool hoặc dữ liệu thị trường

    Các tài khoản token trống được tự động đóng, và phí rent của chúng được trả về ví.

## wallets

hints-wallets-main-title = Ví chính
hints-wallets-main-content =
    Ví chính dùng cho mọi hoạt động giao dịch.

    • **Giao dịch tự động** — các lệnh vào/thoát được thực hiện từ ví này
    • **Hiển thị số dư** — hiển thị ở thanh tiêu đề và bảng điều khiển
    • **Token đang nắm giữ** — token SPL do ví này nắm giữ

## config

hints-config-overview-title = Cấu hình
hints-config-overview-content =
    Cài đặt toàn hệ thống cho { -brand }.

    Các nhóm:
    • **Trader** — quy tắc vào/thoát lệnh, quy mô vị thế
    • **Lọc** — ngưỡng bộ lọc token
    • **Swap** — cài đặt định tuyến và trượt giá
    • **RPC** — cấu hình node
    • **Dịch vụ** — cài đặt các dịch vụ chạy nền

    Thay đổi có hiệu lực ngay lập tức (tải lại nóng).
hints-config-telegram-title = Thông báo { -telegram }
hints-config-telegram-content =
    { "*" }*Nhận cảnh báo giao dịch tức thì qua { -telegram }**

    Nhận thông báo về giao dịch, vị thế và các sự kiện quan trọng ngay trong { -telegram }.

    { "*" }*Các bước thiết lập:**

    1. **Tạo bot:**
       • Mở { -telegram } và nhắn cho @BotFather
       • Gửi /newbot và làm theo hướng dẫn
       • Sao chép token bot (có dạng: 123456:ABC-DEF...)

    2. **Lấy Chat ID của bạn:**
       • Nhắn cho @userinfobot hoặc @getidsbot
       • Sao chép ID dạng số mà bot trả về

    3. **Cấu hình trong { -brand }:**
       • Bật công tắc thông báo
       • Dán token bot và chat ID
       • Nhấp "Kiểm tra kết nối" để xác minh

    { "*" }*Bạn sẽ nhận được:**
    • Xác nhận thực hiện giao dịch
    • Cập nhật vị thế (vào/thoát lệnh)
    • Thông báo lỗi

    { "*" }*Quyền riêng tư:**
    Tin nhắn được gửi trực tiếp từ { -brand } đến bot { -telegram } của bạn — không qua máy chủ bên thứ ba nào.
hints-config-telegram-password-title = Mật khẩu xác thực bot
hints-config-telegram-password-content =
    { "*" }*Bảo vệ bot { -telegram } của bạn bằng mật khẩu**

    Khi tương tác với bot { -telegram } của { -brand }, bạn cần xác thực bằng mật khẩu này trước khi thực hiện các lệnh nhạy cảm.

    { "*" }*Vì sao nên đặt mật khẩu?**
    • Ngăn người không được phép điều khiển bot của bạn
    • Bắt buộc để thực hiện lệnh giao dịch qua { -telegram }
    • Phải có ít nhất 8 ký tự

    { "*" }*Cách hoạt động:**
    1. Đặt mật khẩu tại đây trong bảng điều khiển
    2. Khi bạn nhắn lệnh giao dịch cho bot, bot sẽ yêu cầu xác thực
    3. Nhập mật khẩu để xác minh danh tính
    4. Có thể bật 2FA để tăng cường bảo mật

    { "*" }*Lưu ý:** Mật khẩu được lưu dưới dạng băm SHA256 an toàn — chúng tôi không bao giờ lưu văn bản gốc.
hints-config-telegram-totp-title = Xác thực hai yếu tố (2FA)
hints-config-telegram-totp-content =
    { "*" }*Thêm một lớp bảo mật với TOTP 2FA**

    Xác thực hai yếu tố dùng mật khẩu dùng một lần theo thời gian (TOTP) từ các ứng dụng như Google Authenticator, Authy hoặc 1Password.

    { "*" }*Vì sao nên bật 2FA?**
    • Dù ai đó biết mật khẩu của bạn, họ vẫn không thể truy cập bot nếu không có mã
    • Mã 6 chữ số đổi mỗi 30 giây
    • Hoạt động ngoại tuyến sau khi thiết lập

    { "*" }*Quy trình thiết lập:**
    1. Nhấp "Bật 2FA" và nhập mật khẩu của bạn
    2. Quét mã QR bằng ứng dụng xác thực của bạn
    3. Nhập mã 6 chữ số để xác minh thiết lập

    { "*" }*Ứng dụng tương thích:**
    • Google Authenticator
    • Authy
    • 1Password
    • Microsoft Authenticator
    • Mọi ứng dụng tương thích TOTP

    { "*" }*Lưu ý quan trọng:** Hãy lưu khóa bí mật ở nơi an toàn. Nếu bạn mất quyền truy cập ứng dụng xác thực, bạn sẽ cần tắt 2FA từ bảng điều khiển này.

## token_details

hints-token-details-chart-title = Biểu đồ giá (OHLCV)
hints-token-details-chart-content =
    { "*" }*Quan trọng:** Biểu đồ này hiển thị **dữ liệu OHLCV đã lưu đệm** để đánh giá chiến lược, *không phải* giá thực thi trực tiếp.

    { "*" }*Vì sao dùng dữ liệu đã lưu đệm?**
    • **Mục đích:** Dùng cho các chiến lược và chỉ báo tự động (ví dụ: RSI, MA).
    • **Độ mới:** Việc cập nhật phụ thuộc vào mức ưu tiên của token (vị thế đang mở = cập nhật nhanh hơn).
    • **Nguồn:** Tổng hợp từ { -dexscreener }/{ -geckoterminal }, không phải RPC on-chain trực tiếp.

    { "*" }*Thực tế về giá trên DEX:**
    Trong DeFi, token được giao dịch trên **nhiều pool** (Raydium, Orca, Meteora). Mỗi pool có một mức giá riêng tùy độ sâu thanh khoản và các giao dịch gần đây.
    • **Giá trên biểu đồ:** Mức trung bình/tổng hợp trên các thị trường.
    • **Giá swap:** Tỷ giá cụ thể bạn nhận được từ tuyến tốt nhất tại đúng thời điểm giao dịch.

    { "*" }Hãy chờ đợi những chênh lệch nhỏ giữa biểu đồ này và giá thực thi cuối cùng của bạn.*

    { "*" }*Trạng thái:** "Đang chờ dữ liệu" nghĩa là các tiến trình nền đang tải nến mới.
hints-token-details-token-info-title = Thông tin token
hints-token-details-token-info-content =
    Siêu dữ liệu cơ bản của token từ nguồn on-chain và thị trường.

        • **Mint** — địa chỉ token duy nhất trên Solana (nhấp để sao chép)
        • **Số chữ số thập phân** — độ chính xác của token (thường là 6-9)
        • **Tuổi** — thời gian kể từ khi pool/token chính được tạo
        • **DEX** — nơi giao dịch chính của token này
        • **Holder** — số ví duy nhất đang nắm giữ token
        • **Top 10 nắm giữ** — % do 10 ví lớn nhất nắm giữ

        Số holder càng cao và mức tập trung càng thấp thì phân bổ thường càng lành mạnh.
hints-token-details-liquidity-title = Thanh khoản và dữ liệu thị trường
hints-token-details-liquidity-content =
    Chỉ số thị trường từ pool SOL có thanh khoản cao nhất.

        • **FDV** — giá × tổng nguồn cung (giá của trình tổng hợp)
        • **Thanh khoản** — giá trị USD của dự trữ pool
        • **SOL trong pool** / **Token trong pool** — dự trữ trực tiếp quyết định giá pool

        { "*" }*Vì sao quan trọng:**
        • Thanh khoản càng sâu = trượt giá càng thấp
        • Pool nông có thể biến động chỉ với giao dịch nhỏ
        • Dự trữ của pool quyết định trực tiếp giá thực thi swap

        Dữ liệu được làm mới định kỳ từ { -dexscreener }/{ -geckoterminal } cùng với việc đọc pool on-chain.
hints-token-details-market-pulse-title = Nhịp thị trường
hints-token-details-market-pulse-content =
    Biến động giá và khối lượng giao dịch USD dùng chung mốc thời gian **5M / 1H / 6H / 24H** để có thể so sánh trực tiếp động lượng và mức độ tham gia.

    { "*" }*Cách diễn giải:**
    • **Giá** — phần trăm thay đổi do trình tổng hợp tính, không phải giá thực thi trực tiếp của pool.
    • **Khối lượng cao** — sự quan tâm mạnh hơn, khám phá giá hiệu quả hơn và dễ thoát lệnh hơn.
    • **Khối lượng thấp** — trượt giá lớn hơn, chênh lệch giá rộng hơn và khó thoát lệnh lớn hơn.
    • **Khối lượng cao + thanh khoản thấp** — biến động và rủi ro thực thi tăng cao.

    Dữ liệu thị trường được tổng hợp trên các DEX lớn qua { -dexscreener }/{ -geckoterminal }, nên biến động giá có thể khác với giá pool on-chain hiện tại.
hints-token-details-activity-title = Hoạt động giao dịch (số lượng)
hints-token-details-activity-content =
    Phân tích **số lượng giao dịch** (mua so với bán) trên nhiều khung thời gian. Điều này cho thấy ý định của trader bất kể quy mô giao dịch.

    { "*" }*Chi tiết chỉ số:**
    • **Khung thời gian:** các cửa sổ 5M, 1H, 6H, 24H.
    • **Thanh:** tỷ lệ trực quan giữa số lệnh mua (xanh lá) và số lệnh bán (đỏ).
    • **Tốc độ:** số giao dịch mỗi phút (ví dụ: "12.5/m"). Tốc độ càng cao = hoạt động lan truyền càng mạnh.
    • **Số lượng:** số lệnh mua/bán chính xác và tỷ lệ phần trăm của chúng.

    { "*" }*Chỉ số tóm tắt:**
    • **% mua 24H:** >50% là tích cực (nhiều người mua hơn), { "<" }50% là tiêu cực (nhiều người bán hơn).
    • **Dòng ròng:** Tổng mua trừ tổng bán. Dương = tích lũy.
    • **Đột biến 5M:** Tốc độ giao dịch *hiện tại* nhanh hơn bao nhiêu so với mức trung bình 1H.
      • **>1.0x:** Sự quan tâm đang tăng tốc.
      • **>3.0x:** Breakout lan truyền hoặc sự kiện hoảng loạn.
      • **{ "<" }1.0x:** Đang nguội dần.

    { "*" }*Mẹo chiến lược:** "% mua" cao cùng "hệ số đột biến" cao thường báo hiệu một điểm vào lệnh breakout mạnh.
hints-token-details-security-title = Phân tích bảo mật
hints-token-details-security-content =
    Đánh giá rủi ro từ { -rugcheck }.xyz và phân tích on-chain.

    { "*" }*Điểm an toàn (0-100):**
    Điểm càng cao thì token càng an toàn. Các yếu tố gồm:
    • Quyền hạn (mint/freeze)
    • Mức tập trung holder
    • Trạng thái khóa LP
    • Các mẫu rủi ro đã biết

    { "*" }*Chỉ báo rủi ro chính:**
    • **Quyền mint** — có thể tạo thêm token (rủi ro lạm phát)
    • **Quyền freeze** — có thể đóng băng tài khoản token
    • **% holder lớn nhất** — rủi ro tập trung
    • **Nhà cung cấp LP** — số lượng nhà cung cấp thanh khoản

    Luôn kiểm tra bảo mật trước khi giao dịch số tiền lớn.
hints-token-details-pools-title = Pool thanh khoản
hints-token-details-pools-content =
    Tất cả pool thanh khoản đã phát hiện cho token này.

    { "*" }*Vì sao nhiều pool lại quan trọng:**
    • Mỗi pool có thanh khoản và giá khác nhau
    • Router swap tìm tuyến tốt nhất giữa các pool
    • Giá có thể chênh 1-5% giữa các pool

    { "*" }*Thông tin pool:**
    • **DEX** — sàn giao dịch nào chứa pool
    • **Thanh khoản** — giá trị USD của dự trữ pool
    • **Khối lượng** — hoạt động giao dịch gần đây
    • **Giá** — giá pool hiện tại

    Dịch vụ pool tính giá từ cặp SOL có thanh khoản cao nhất.

## ui

hints-ui-featured-title = Nổi bật
hints-ui-featured-content =
    Token được boost trước, sau đó là các dự án thịnh hành từ { -jupiter } và { -dexscreener }.

    { "*" }*Bạn sẽ thấy:**
    • Token được boost — đội ngũ dự án đã trả tiền để quảng bá — được ghim lên đầu, đánh dấu màu vàng
    • Sau đó là các token thịnh hành từ các bảng khám phá
    • Nhấp vào bất kỳ token nào để mở chi tiết đầy đủ

    { "*" }*Boost một token:**
    Boost mua sự hiển thị, không bao giờ là lời khuyến nghị. Các dòng được boost được đánh dấu
    màu vàng ở mọi nơi chúng xuất hiện, kể cả trong bảng token của bạn, nên bạn luôn phân biệt
    được đâu là token nào. Boost một token tại
    { "*" }*screenerbot.io/boost**.

    { "*" }*Tắt hàng nổi bật:**
    Ẩn nó tại **Cài đặt → Giao diện → Hiện hàng nổi bật**. Nút ở tiêu đề vẫn mở
    toàn bộ chế độ xem Nổi bật.

## Hint popover chrome (ui/hint_popover.js)

hints-trigger =
    .aria-label = Trợ giúp: { $title }
hints-popover-close =
    .aria-label = Đóng
hints-popover-learn-more = Tìm hiểu thêm
hints-popover-dismiss = Không hiện lại
