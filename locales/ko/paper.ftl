paper-readiness-history = 모의 거래 기록
paper-readiness-history-met =
    { $count ->
       *[other] 종료된 모의 라운드 { $count }개, 필요 { $needed }개
    }
paper-readiness-history-short = 종료된 모의 라운드 { $needed }개 중 { $count }개
paper-readiness-profit = 모의 거래 수익
paper-readiness-profit-detail =
    { $count ->
       *[other] { $count }개 라운드에서 { $realized } { -sol } 실현, { $wins }개 수익
    }
paper-readiness-exits = 청산이 체결됨
paper-readiness-exits-ok = 최근 모의 청산이 체결되었습니다
paper-readiness-exits-failing = 연속으로 체결되지 않은 모의 청산: { $count }
paper-live-block-setup-incomplete = 먼저 지갑과 RPC 설정을 완료하세요
paper-live-block-force-stop = 긴급 중지가 작동 중입니다
