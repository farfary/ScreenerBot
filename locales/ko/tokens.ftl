# Token data source status. Ids come from build_source_status in
# src/webserver/routes/tokens/source_status.rs. $label is a provider name and is
# not translated.

tokens-result-source-live = 실시간 시장 데이터
tokens-result-source-unavailable = { $label } 사용 불가 — 재시도 중
tokens-result-source-not-listed = { $label }에 등록되지 않음
tokens-result-security-available = 보안 리포트 있음
tokens-result-security-missing = { -rugcheck } 리포트 없음
tokens-result-chart-available = 차트 데이터 있음
tokens-result-chart-missing = 아직 차트 데이터가 없습니다

# Token details dialog: shared tab states (ui/token_details/state_handling.js)

tokens-state-error-title = 데이터를 불러올 수 없습니다
tokens-state-offline = 오프라인 상태인 것 같습니다.
tokens-state-request-failed = 여러 번 시도했지만 요청에 실패했습니다.
tokens-state-waiting = 데이터 대기 중…

# Token details dialog: chart (ui/token_details/chart_tab.js)

tokens-chart-marker-entry = 진입
tokens-chart-level-stop-loss = 손절
tokens-chart-level-take-profit = 익절

# Token details dialog: transactions tab (ui/token_details/transactions_tab.js)

tokens-transactions-loading = 트랜잭션을 불러오는 중…
tokens-transactions-empty-title = 트랜잭션 없음
tokens-transactions-empty-history = 이 토큰의 지갑 트랜잭션 기록이 없습니다.
tokens-transactions-empty-data = 이 토큰의 트랜잭션 데이터가 없습니다.
tokens-transactions-error-title = 트랜잭션을 불러올 수 없습니다
tokens-transactions-error-message = 트랜잭션 기록을 일시적으로 사용할 수 없습니다.
tokens-transactions-activity-title = 24h 활동
tokens-transactions-activity-subtitle = 시간별 지갑 트랜잭션
tokens-transactions-metric-total = 합계
tokens-transactions-metric-buys = 매수
tokens-transactions-metric-sells = 매도
tokens-transactions-recent-title = 최근 트랜잭션
tokens-transactions-shown = { $count }개 표시
tokens-transactions-column-time = 시간
tokens-transactions-column-type = 유형
tokens-transactions-column-price = 가격
tokens-transactions-column-total = 합계
tokens-transactions-chart-missing = 차트 라이브러리 없음
tokens-transactions-view-solscan = { -solscan }에서 트랜잭션 보기

# Token details dialog: positions tab (ui/token_details/positions_tab.js)

tokens-positions-empty-title = 포지션 없음
tokens-positions-no-token = 선택한 토큰이 없습니다.
tokens-positions-empty-message = 이 토큰의 포지션이 아직 없습니다. 매수를 사용하여 포지션을 여세요.
tokens-positions-loading = 포지션을 불러오는 중…
tokens-positions-from-wallet-history = 지갑 기록 기준
tokens-positions-frozen = 동결됨 — 매도할 수 없음
tokens-positions-no-cost-basis = 매입 원가 없음
tokens-positions-history-incomplete = 기록 불완전
tokens-positions-dca-count = DCA { $count }
tokens-positions-exit-count = 청산 { $count }
tokens-positions-fact-avg-entry = 평균 진입가
tokens-positions-fact-current = 현재가
tokens-positions-fact-tokens = 토큰
tokens-positions-fact-opened = 진입 시각
tokens-positions-fact-exit-price = 청산 가격
tokens-positions-fact-sol-received = 수령한 { -sol }
tokens-positions-fact-closed-reason = 종료 사유
tokens-positions-fact-target-min = 목표 수익 최소
tokens-positions-fact-target-max = 목표 수익 최대
tokens-positions-fact-highest = 최고가
tokens-positions-fact-lowest = 최저가
tokens-positions-section-range = 목표 및 범위
tokens-positions-section-market = 시장 및 보유 수량
tokens-positions-kicker = 포지션
tokens-positions-fallback-symbol = 토큰
tokens-positions-realized-pnl = 실현 손익
tokens-positions-unrealized-pnl = 미실현 손익
tokens-positions-size = 규모

# Token details dialog: security tab (ui/token_details/security_tab.js)
# Risk names and descriptions come from the RugCheck report and render as sent.

tokens-security-analysis-pending = { -rugcheck } 분석 진행 중...
tokens-security-analyzing = 보안 분석 중…
tokens-security-pulse-title = 보안 동향
tokens-security-pending-caption = 위험 시그널을 아직 수집하는 중입니다.
tokens-security-control-title = 토큰 제어
tokens-security-control-meta = 권한 상태
tokens-security-updated = 업데이트: { $time }
tokens-security-score-caption = 100점 만점으로 정규화한 토큰 위험 점수입니다.
tokens-security-score-label = 점수
tokens-security-rugged = 러그풀 발생
tokens-security-grade-analyzing = 분석 중
tokens-security-grade-shielded = 보호됨
tokens-security-grade-safe = 안전
tokens-security-grade-caution = 주의
tokens-security-grade-vulnerable = 취약
tokens-security-grade-unknown = 알 수 없음
tokens-security-metric-token-type = 토큰 유형
tokens-security-metric-total-holders = 전체 홀더
tokens-security-metric-lp-providers = LP 제공자
tokens-security-metric-graph-insiders = 그래프 인사이더
tokens-security-insiders-detected = 감지됨 ({ $count })
tokens-security-insiders-clean = 이상 없음
tokens-security-authority-mint = 민트
tokens-security-authority-freeze = 동결
tokens-security-authority-immutable = 변경 불가
tokens-security-authority-mutable = 변경 가능
tokens-security-authority-revoked = 폐기됨
tokens-security-authority-active = 활성
tokens-security-holder-health-title = 홀더 상태
tokens-security-holders-unique = 고유
tokens-security-creator-share = 생성자 비중
tokens-security-gauge-top-10 = 상위 10
tokens-security-concentration-unknown = 알 수 없음
tokens-security-concentration-critical = 치명적
tokens-security-concentration-high = 높음
tokens-security-concentration-moderate = 보통
tokens-security-concentration-healthy = 양호
tokens-security-transfer-title = 전송 세금
tokens-security-transfer-no-fee = 수수료 없음
tokens-security-transfer-fee-percentage = 수수료 비율
tokens-security-transfer-max-fee = 최대 수수료 금액
tokens-security-transfer-authority = 수수료 권한
tokens-security-transfer-note = 모든 전송에 { $percent } 수수료가 부과됩니다.
tokens-security-transfer-none = 감지된 전송 수수료가 없습니다.
tokens-security-risks-title = 보안 위험
tokens-security-risks-none = 감지된 보안 위험이 없습니다.
tokens-security-risk-fallback-name = 보안 시그널
tokens-security-risks-critical = 치명적 { $count }건
tokens-security-risks-warnings =
    { $count ->
       *[other] 경고 { $count }건
    }
tokens-security-risks-info = 정보 { $count }건
tokens-security-risks-incidents =
    { $count ->
       *[other] 사고 { $count }건 발견
    }
tokens-security-top-holders-title = 상위 홀더
tokens-security-top-holders-concentration = 집중도 { $percent }
tokens-security-insider = 인사이더

# Token details dialog: overview tab (ui/token_details/overview_tab.js)
# 5M/1H/6H/24H period codes and the chart timeframe buttons are id codes shared
# with the chart and stay as sent.

tokens-overview-chart-checking = 데이터 확인 중…
tokens-overview-banner-open = 토큰 배너 열기
tokens-overview-headline-label = 주요 시장 지표
tokens-overview-price = 가격
tokens-overview-market-cap = 시가총액
tokens-overview-liquidity = 유동성
tokens-overview-volume = 거래량
tokens-overview-volume-24h = 거래량 24H
tokens-overview-no-tags = 태그 없음
tokens-overview-info-title = 토큰 정보
tokens-overview-profile = 게시된 프로필
tokens-overview-fact-mint = 민트
tokens-overview-fact-decimals = 소수 자릿수
tokens-overview-fact-age = 경과 기간
tokens-overview-fact-dex = DEX
tokens-overview-fact-holders = 홀더
tokens-overview-fact-top-10 = 상위 10 보유
tokens-overview-tags = 태그
tokens-overview-liquidity-title = 유동성 및 시장
tokens-overview-fact-fdv = FDV
tokens-overview-fact-pool-sol = 풀 { -sol }
tokens-overview-fact-pool-token = 풀 토큰
tokens-overview-pool = 풀
tokens-overview-pulse-title = 시장 동향
tokens-overview-activity-title = 트랜잭션 활동
tokens-overview-buy-share = 매수 { $percent }
tokens-overview-buy-sell-ratio = 매수/매도 { $ratio }
tokens-overview-buys-24h = 매수 24H
tokens-overview-sells-24h = 매도 24H
tokens-overview-net-flow = 순유입
tokens-overview-total-24h = 합계 24H
tokens-overview-average-24h = 평균 24H
tokens-overview-spike-5m = 5M 급증
tokens-overview-rate-per-hour = { $amount }/h
tokens-overview-rate-per-minute = { $amount }/m
tokens-overview-spike-factor = { $factor }×
tokens-overview-flow-counts = 매수: { $buys } ({ $buyPercent }), 매도: { $sells } ({ $sellPercent }), 합계: { $total }
tokens-overview-flow-no-data = 트랜잭션 데이터 없음

# Token details dialog: pools tab (ui/token_details/pools_links_tab.js)
# DEX names in pool data render as sent.

tokens-pools-empty-title = 풀 없음
tokens-pools-empty-message = 이 토큰에서 감지된 유동성 풀이 없습니다.
tokens-pools-unknown = 알 수 없음
tokens-pools-unknown-dex = 알 수 없는 DEX
tokens-pools-total = 전체 풀
tokens-pools-liquidity = 유동성
tokens-pools-volume-24h = 24h 거래량
tokens-pools-base-role = 기준 역할
tokens-pools-quote-role = 견적 역할
tokens-pools-canonical-title = 대표 풀
tokens-pools-canonical = 대표
tokens-pools-dex = DEX
tokens-pools-summary-title = 풀 요약
tokens-pools-breakdown-title = DEX별 내역
tokens-pools-all-title = 전체 풀
tokens-pools-updated = 업데이트
tokens-pools-role-base = 기준
tokens-pools-role-quote = 견적
tokens-pools-role-unknown = 알 수 없음
tokens-pools-reserves = 리저브 계정
tokens-pools-no-reserves = 리저브 계정 없음
tokens-pools-address-copy = 주소 복사
tokens-pools-address-pool = 풀
    .title = 풀 복사
tokens-pools-address-base = 기준 민트
    .title = 기준 민트 복사
tokens-pools-address-quote = 견적 민트
    .title = 견적 민트 복사
tokens-pools-address-paired = 페어 민트
    .title = 페어 민트 복사

# Token details dialog: links tab (ui/token_details/pools_links_tab.js)

tokens-links-empty = 이 토큰의 공식 웹사이트나 소셜 링크가 없습니다.
tokens-links-info-title = 토큰 정보
tokens-links-mint-address = 민트 주소
tokens-links-data-source = 데이터 소스
tokens-links-security = 보안
tokens-links-profile-title = 토큰 프로필
tokens-links-profile-published-title = 게시된 프로필 콘텐츠
tokens-links-profile-published-note = 미디어, 설명, 공식 링크는 게시 전에 검토되는 유료 프로필 콘텐츠입니다. 소유권이나 토큰 안전성을 검증하는 것은 아닙니다.
tokens-links-profile-create-note = 검토를 거친 로고, 프로젝트 설명, 공식 링크를 이 토큰의 공개 프로필에 추가하세요.
tokens-links-profile-update-hint = screenerbot.io에서 이 토큰 프로필 업데이트
tokens-links-profile-create-hint = screenerbot.io에서 토큰 프로필 만들기
tokens-links-profile-update = 프로필 업데이트
tokens-links-profile-create = 프로필 만들기
tokens-links-media-title = 미디어 자산
tokens-links-media-fallback-symbol = 토큰
tokens-links-media-logo = 로고
tokens-links-media-banner = 배너
tokens-links-media-banner-alt = { $symbol } 배너
tokens-links-media-open = 이미지 열기
tokens-links-description-title = 설명
tokens-links-explorers-title = 익스플로러 및 분석
tokens-links-websites-title = 공식 웹사이트
tokens-links-socials-title = 소셜 미디어
tokens-links-explorer-solana-explorer = { -solana-explorer }
tokens-links-explorer-geckoterminal = { -geckoterminal }
tokens-links-explorer-dextools = { -dextools }
tokens-links-explorer-coingecko = { -coingecko }
tokens-links-explorer-jupiter-swap = { -jupiter } 스왑
tokens-links-social-twitter = { -twitter } / { -x }
tokens-links-social-x = { -x } ({ -twitter })
tokens-links-social-telegram = { -telegram }
tokens-links-social-discord = { -discord }
tokens-links-social-medium = { -medium }
tokens-links-social-github = { -github }
tokens-links-social-youtube = { -youtube }
tokens-links-social-reddit = { -reddit }
tokens-links-social-facebook = { -facebook }
tokens-links-social-instagram = { -instagram }
tokens-links-social-linkedin = { -linkedin }
tokens-links-social-tiktok = { -tiktok }
tokens-links-social-fallback = 소셜

# Token details dialog: frame, header and data sources (ui/token_details_dialog.js)

tokens-dialog-tab-overview = 개요
tokens-dialog-tab-security = 보안
tokens-dialog-tab-positions = 포지션
tokens-dialog-tab-pools = 풀
tokens-dialog-tab-links = 링크
tokens-dialog-tab-transactions = 트랜잭션
tokens-dialog-sections = 토큰 상세 섹션
tokens-dialog-close =
    .title = 닫기 (ESC)
    .aria-label = 토큰 상세 닫기
tokens-dialog-unknown-symbol = 알 수 없음
tokens-dialog-unknown-name = 알 수 없는 토큰
tokens-dialog-market-summary = 시장 요약
tokens-dialog-price-loading = 가격 불러오는 중
tokens-dialog-unit-sol = { -sol }
tokens-dialog-market-metrics = 시장 지표
tokens-dialog-metric-market-cap = 시가총액
tokens-dialog-metric-volume-24h = 24h 거래량
tokens-dialog-change-24h = 24시간 변동 { $change }
tokens-dialog-buy = 매수
    .title = 이 토큰 매수
tokens-dialog-sell = 매도
    .title = 포지션 매도
tokens-dialog-sell-unavailable = 매도할 보유 포지션이 없습니다
tokens-dialog-details = 상세
tokens-dialog-sources = 소스
tokens-dialog-sources-status = 데이터 소스 상태
tokens-dialog-updated-label = 업데이트
tokens-dialog-just-now = 방금
tokens-dialog-updated-at = 업데이트: { $time }
tokens-dialog-updated-unavailable = 업데이트 시간을 알 수 없습니다
tokens-dialog-error-title = 토큰 데이터를 불러올 수 없습니다
tokens-dialog-waiting-token = 토큰 데이터 대기 중…
tokens-dialog-loading-overview = 개요를 불러오는 중…
tokens-dialog-loading-security = 보안 정보를 불러오는 중…
tokens-dialog-loading-pools = 풀을 불러오는 중…
tokens-dialog-loading-links = 링크를 불러오는 중…
tokens-dialog-chart-still-checking = 아직 차트 데이터가 없습니다 — 계속 확인하는 중…
tokens-dialog-no-data = 데이터 없음
tokens-dialog-source-token = 토큰
tokens-dialog-source-market = 시장
tokens-dialog-source-security = 보안
tokens-dialog-source-chart = 차트
tokens-dialog-status-pending = 대기 중
tokens-dialog-status-loading = 불러오는 중
tokens-dialog-status-ready = 준비됨
tokens-dialog-status-unavailable = 사용 불가
tokens-dialog-status-cached = 캐시됨
tokens-dialog-source-summary = { $source } 데이터: { $status }
tokens-dialog-badge-pool-price = 풀 가격
tokens-dialog-badge-pool-price-hint = 실시간 온체인 풀 기준 가격
tokens-dialog-badge-api-price = API 가격
tokens-dialog-badge-api-price-hint = 캐시된 시장 데이터(API) 기준 가격
tokens-dialog-badge-profile = 게시된 프로필
    .title = 게시 전에 검토되는 유료 프로필 콘텐츠이며 감사나 소유권 검증이 아닙니다.
tokens-dialog-badge-low-risk-hint = 현재 { -rugcheck } 점수 기준으로 위험이 낮음. 신원 검증은 아닙니다.
tokens-dialog-badge-immutable = 변경 불가
tokens-dialog-badge-mutable = 변경 가능
tokens-dialog-badge-auth = 권한:
tokens-dialog-badge-update-authority = 업데이트 권한:
tokens-dialog-badge-position = 포지션
tokens-dialog-badge-blacklisted = 블랙리스트

# Tokens page: sub-tabs (scripts/pages/tokens/constants.js)
# Ids are the view values of /api/tokens/list.

tokens-view-favorites = 즐겨찾기
tokens-view-pool = 풀 서비스
tokens-view-no-market = 시장 데이터 없음
tokens-view-all = 전체 토큰
tokens-view-passed = 통과
tokens-view-rejected = 제외
tokens-view-blacklisted = 블랙리스트
tokens-view-positions = 포지션
tokens-view-recent = 최근
tokens-view-ohlcv = OHLCV 데이터

# Tokens page: token cell (scripts/pages/tokens/formatters.js)

tokens-cell-logo-enlarge = 클릭하여 확대
tokens-boost-title = screenerbot.io에서 { $boosts }회 부스트됨
tokens-cell-action-add =
    .title = 포지션 추가 매수 (DCA)
    .aria-label = 포지션 추가 매수
tokens-cell-action-sell =
    .title = 매도 (전체 또는 % 부분)
    .aria-label = 토큰 매도
tokens-cell-action-buy =
    .title = 포지션 매수
    .aria-label = 토큰 매수
tokens-cell-external-links =
    .title = 외부 링크
    .aria-label = 외부 링크

# Tokens page: table states shared by the token lists (scripts/pages/tokens/*.js)

tokens-table-loading-title = 토큰을 불러오는 중…
tokens-table-loading-description = 선택한 토큰 보기를 준비하는 중입니다.
tokens-table-retry-hint = 탭을 전환하거나 다시 시도하세요.
tokens-filter-all = 전체

# Tokens page: favorites (scripts/pages/tokens/favorites.js)

tokens-favorites-load-failed-title = 즐겨찾기를 불러올 수 없습니다
tokens-favorites-load-failed-toast = 즐겨찾기를 불러올 수 없습니다
tokens-favorites-total = 전체 즐겨찾기
tokens-favorites-empty-title = 즐겨찾기가 아직 없습니다
tokens-favorites-empty-description = 검색 단축키: { $shortcut }. 토큰을 찾아 즐겨찾기에 추가하세요.

# Tokens page: OHLCV data view (scripts/pages/tokens/ohlcv.js)
# Status ids come from /api/ohlcv/tokens; priority ids are Priority::as_str in src/ohlcvs/types.rs.

tokens-column-token = 토큰
tokens-column-status = 상태
tokens-ohlcv-delete =
    .title = OHLCV 데이터 삭제
    .aria-label = OHLCV 데이터 삭제
tokens-ohlcv-status-active = 활성
tokens-ohlcv-status-inactive = 비활성
tokens-ohlcv-priority-critical = 치명적
tokens-ohlcv-priority-high = 높음
tokens-ohlcv-priority-medium = 보통
tokens-ohlcv-priority-low = 낮음
tokens-ohlcv-column-priority = 우선순위
tokens-ohlcv-column-backfill = 백필
tokens-ohlcv-column-data-span = 데이터 범위
tokens-ohlcv-column-gaps = 공백
tokens-ohlcv-column-pools = 풀
tokens-ohlcv-column-last-fetch = 마지막 조회
tokens-ohlcv-timeframe-complete = { $timeframe }: 완료
tokens-ohlcv-timeframe-pending = { $timeframe }: 대기 중
tokens-ohlcv-load-failed-title = OHLCV 데이터를 불러올 수 없습니다
tokens-ohlcv-load-failed-toast = OHLCV 데이터를 불러올 수 없습니다
tokens-ohlcv-total = 전체 토큰
tokens-ohlcv-active = 활성
tokens-ohlcv-db-size = DB 크기
tokens-ohlcv-cleanup = 비활성 정리
tokens-ohlcv-delete-title = OHLCV 데이터 삭제
tokens-ohlcv-delete-message = { $mint }... 의 모든 OHLCV 데이터를 삭제하시겠습니까?
tokens-ohlcv-delete-done = 삭제됨: 캔들 { $candles }개, 풀 { $pools }개
tokens-ohlcv-delete-failed = OHLCV 데이터를 삭제하지 못했습니다
tokens-ohlcv-cleanup-title = 비활성 토큰 삭제
tokens-ohlcv-cleanup-message = 지정한 시간보다 오래된 비활성 토큰을 삭제합니다
tokens-ohlcv-cleanup-placeholder = 시간...
tokens-ohlcv-cleanup-invalid = 양수를 입력하세요
tokens-ohlcv-cleanup-done =
    { $count ->
       *[other] 비활성 토큰 { $count }개를 정리했습니다
    }
tokens-ohlcv-cleanup-failed = OHLCV 데이터를 정리하지 못했습니다

# Tokens page: token lists (scripts/pages/tokens.js)
# The list statuses shown in the Status column come from row flags, not ids.

tokens-summary-total = 전체
tokens-summary-priced = 가격 있음
tokens-summary-positions = 포지션
tokens-summary-blacklisted = 블랙리스트
tokens-search-placeholder = 심볼 또는 민트로 검색...
tokens-table-waiting-title = 토큰을 계속 불러오는 중...
tokens-table-waiting-description = 백엔드 응답을 기다리는 중입니다. 자동으로 다시 시도합니다.
tokens-load-failed-toast = 토큰을 불러올 수 없습니다
tokens-row-data-missing = 토큰 데이터를 찾을 수 없습니다
tokens-column-price-sol = 가격 ({ -sol })
tokens-column-liquidity = 유동성
tokens-column-volume-24h = 24h 거래량
tokens-column-fdv = FDV
tokens-column-market-cap = 시가총액
tokens-column-change-1h = 1h
tokens-column-change-24h = 24h
tokens-column-txns-5m = 트랜잭션 5m
tokens-column-txns-1h = 트랜잭션 1h
tokens-column-txns-6h = 트랜잭션 6h
tokens-column-txns-24h = 트랜잭션 24h
tokens-column-risk-score = 위험 점수
tokens-column-reject-reason = 제외 사유
tokens-column-blacklist-reason = 블랙리스트 사유
tokens-column-updated = 업데이트
tokens-column-birth = 생성
tokens-column-first-seen = 최초 발견
tokens-badge-price = 가격
tokens-badge-ohlcv = OHLCV
tokens-badge-position = 포지션
tokens-badge-blacklisted = 블랙리스트
tokens-badge-blacklisted-title = 블랙리스트 토큰
tokens-badge-blacklisted-reasons = 블랙리스트: { $reasons }
tokens-links-menu-copy-mint = 민트 복사
tokens-links-copy-failed = 민트를 복사하지 못했습니다
tokens-lightbox-token-age = 토큰 경과 기간

# Global search dialog (scripts/ui/search_dialog.js)

tokens-search-placeholder-dialog = 이름, 심볼 또는 민트 검색...
tokens-search-input-label = 토큰 검색
tokens-search-results-label = 검색 결과
tokens-search-hint = 토큰 이름, 심볼을 입력하거나 민트를 붙여넣으세요
tokens-search-no-matches = 일치하는 항목이 없습니다 — 다른 검색어를 시도하세요
tokens-search-tip-nav = 이동
tokens-search-tip-open = 열기
tokens-search-tip-close = 닫기
tokens-search-failed = 검색에 실패했습니다
tokens-search-error = 오류: { $message }
tokens-search-action-favorite =
    .title = 즐겨찾기에 추가
    .aria-label = 즐겨찾기에 추가
tokens-search-action-blacklist =
    .title = 블랙리스트에 추가
    .aria-label = 블랙리스트에 추가
tokens-search-no-mint = 토큰에 민트 주소가 없습니다
tokens-search-open-failed = 토큰 상세를 열지 못했습니다
tokens-search-copy-failed = 클립보드에 복사하지 못했습니다
tokens-search-favorite-added = 즐겨찾기에 추가됨: { $symbol }
tokens-search-favorite-already = 이미 즐겨찾기에 있습니다
tokens-search-favorite-failed = 즐겨찾기에 추가하지 못했습니다
tokens-search-blacklist-message = 블랙리스트에 추가할 토큰: { $symbol }. 이 토큰은 거래에서 제외됩니다.
tokens-search-blacklist-done = 블랙리스트 등록됨: { $symbol }
tokens-search-blacklisted = 블랙리스트 등록됨
tokens-search-blacklist-failed = 토큰을 블랙리스트에 추가하지 못했습니다

# Featured dialog (scripts/ui/featured_dialog.js)
# Category and source ids are those of CATEGORIES; provider names are terms.

tokens-featured-category-boosted = 부스트됨
tokens-featured-category-jupiter-organic = { -jupiter } 오가닉 상위
tokens-featured-category-jupiter-traded = { -jupiter } 거래 상위
tokens-featured-category-dexscreener-trending = { -dexscreener } 트렌딩
tokens-featured-source-jupiter = { -jupiter }
tokens-featured-source-dexscreener = { -dexscreener }
tokens-featured-note-boosted = 팀이 직접 홍보 중
tokens-featured-security-risky = 위험
tokens-featured-load-failed = 추천 목록을 불러오지 못했습니다
tokens-featured-network-error = 네트워크 오류: { $message }
tokens-featured-title = 추천
tokens-featured-subtitle = 부스트된 토큰이 먼저, 이어서 Solana 전반의 트렌딩 토큰
tokens-featured-boost = 토큰 부스트
tokens-featured-close =
    .title = 닫기 (ESC)
tokens-featured-loading = 추천 및 트렌딩 토큰을 불러오는 중...
tokens-featured-error-hint = 연결을 확인하거나 다시 시도하세요
tokens-featured-empty = 현재 사용할 수 있는 토큰이 없습니다
tokens-featured-count =
    { $count ->
       *[other] 토큰 { $count }개
    }
tokens-featured-stat-market-cap = 시가총액
tokens-featured-stat-liquidity = 유동성
tokens-featured-stat-volume = 거래량 24H
tokens-featured-stat-holders = 홀더
tokens-featured-stat-txns = 트랜잭션 24H
tokens-featured-buy = 매수
    .title = { $symbol } 매수
tokens-featured-security-score = 보안 점수: { $score }/100
tokens-featured-social-website = 웹사이트
tokens-featured-social-twitter = { -twitter }

# Featured row (scripts/ui/featured_row.js)

tokens-featured-row-view-all = 전체
    .title = 전체 추천 보기 열기
tokens-featured-row-scroll-start =
    .aria-label = 이전 토큰 보기
tokens-featured-row-scroll-end =
    .aria-label = 더 많은 토큰 보기
tokens-featured-row-empty = 추천 토큰 없음
tokens-featured-row-title = { $name } ({ $symbol })
tokens-featured-row-boosted-title = { $name } ({ $symbol }) — 부스트 { $boosts }회

# Pool selector dialog (scripts/ui/pool_selector.js)

tokens-pool-selector-title = 풀 선택
tokens-pool-selector-loading = 풀을 불러오는 중...
tokens-pool-selector-empty = 이 토큰의 풀을 찾을 수 없습니다
tokens-pool-selector-load-failed = 풀을 불러오지 못했습니다: { $message }
tokens-pool-selector-count =
    { $count ->
       *[other] 풀 { $count }개 발견
    }
tokens-pool-selector-liquidity = { $amount } 유동성
    .title = 유동성
tokens-pool-selector-volume = { $amount } 24h
    .title = 24h 거래량

# Token identity chips and address rows (scripts/ui/token_identity.js)

tokens-identity-unknown-asset = 알 수 없는 자산
tokens-identity-copy-address =
    .title = 주소 복사
    .aria-label = 주소 복사
tokens-identity-copy-signature =
    .title = 서명 복사
    .aria-label = 서명 복사
