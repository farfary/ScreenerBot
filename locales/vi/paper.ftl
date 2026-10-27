paper-readiness-history = Lịch sử thử nghiệm
paper-readiness-history-met =
    { $count ->
       *[other] { $count } vòng thử nghiệm đã đóng, cần { $needed }
    }
paper-readiness-history-short = { $count }/{ $needed } vòng thử nghiệm đã đóng
paper-readiness-profit = Có lãi khi thử nghiệm
paper-readiness-profit-detail =
    { $count ->
       *[other] Đã chốt { $realized } { -sol } qua { $count } vòng, thắng { $wins } vòng
    }
paper-readiness-exits = Lệnh thoát được khớp
paper-readiness-exits-ok = Các lệnh thoát thử nghiệm gần đây đã khớp
paper-readiness-exits-failing = Số lệnh thoát thử nghiệm liên tiếp không khớp: { $count }
paper-live-block-setup-incomplete = Hãy hoàn tất thiết lập ví và RPC trước
paper-live-block-force-stop = Dừng khẩn cấp đang được bật
