## Portfolio overview

home-portfolio-title = 포트폴리오 가치
home-portfolio-today = 오늘
home-stat-available = 사용 가능한 { -sol }
home-stat-holdings = 토큰 보유 현황
home-stat-open-pnl = 미실현 손익
home-stat-realized-today = 오늘 실현 손익

home-holdings-token-count =
    { $count ->
       *[other] 토큰 { $count }개
    }
home-holdings-with-unpriced = { $tokens } · 가격 없음 { $count }개
home-holdings-unpriced-note =
    { $count ->
       *[other] 가격을 확인할 수 없는 보유 토큰 { $count }개는 합계에서 0으로 계산됩니다
    }

## Wallet address and QR code

home-wallet-copy =
    .title = 지갑 주소 복사
    .aria-label = 지갑 주소 복사
home-wallet-qr-open =
    .title = 지갑 QR 코드 표시
    .aria-label = 지갑 QR 코드 표시
home-wallet-qr-popover =
    .aria-label = 지갑 QR 코드
home-wallet-qr-receive = 받기
home-wallet-qr-assets = { -sol } 및 SPL 토큰
home-wallet-qr-close =
    .title = 닫기
    .aria-label = 지갑 QR 코드 닫기
home-wallet-qr-preparing = QR 코드 준비 중
home-wallet-qr-unavailable = QR 코드를 사용할 수 없음
home-wallet-qr-image =
    .alt = 메인 지갑 주소 QR 코드

## Performance calendar

home-calendar-title = 성과 캘린더
home-calendar-previous =
    .title = 이전 달
    .aria-label = 이전 달
home-calendar-next =
    .title = 다음 달
    .aria-label = 다음 달
home-calendar-month-pnl = 월간 손익
home-calendar-trades = 거래
home-calendar-pop-net-pnl = 순손익
home-calendar-pop-win-rate = 승률
home-calendar-pop-win-rate-value = { $rate } · { $wins }승 / { $losses }패
home-calendar-pop-gross-profit = 총수익
home-calendar-pop-gross-loss = 총손실
home-calendar-pop-end-balance = 종료 잔액

## Position exposure and market pipeline

home-operations =
    .aria-label = 포트폴리오 및 시장 상태
home-exposure-title = 포지션 노출
home-exposure-open = 보유 중
home-exposure-invested = 투자금
home-exposure-avg-size = 평균 규모
home-exposure-avg-hold = 평균 보유 시간
home-exposure-best = 최고
home-exposure-worst = 최저
home-pipeline-title = 시장 파이프라인
home-pipeline-tracked = 추적 중
home-pipeline-priced = 가격 확인됨
home-pipeline-passed = 필터 통과
home-pipeline-not-passed = 미통과 (추적 전체)
home-pipeline-blacklisted = 블랙리스트
home-pipeline-ohlcv = OHLCV
