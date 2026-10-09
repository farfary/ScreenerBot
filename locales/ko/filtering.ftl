# Filter rejection reasons. Message ids derive from the stored rejection codes
# (src/filtering/sources/rejection.rs); rows hold codes, never this text.

filtering-reject-no-decimals = 데이터베이스에 소수 자릿수 없음
filtering-reject-token-too-new = 토큰이 너무 새로움
filtering-reject-cooldown-filtered = 쿨다운으로 제외됨
filtering-reject-dex-data-missing = { -dexscreener } 데이터 없음
filtering-reject-gecko-data-missing = { -geckoterminal } 데이터 없음
filtering-reject-rug-data-missing = { -rugcheck } 데이터 없음
filtering-reject-onchain-numeric-symbol = 숫자로만 된 심볼 (사기)
filtering-reject-onchain-empty-symbol = 빈 심볼 (사기)
filtering-reject-onchain-suspicious-symbol = 의심스러운 심볼 (사기)
filtering-reject-onchain-known-scam-authority = 알려진 사기 권한
filtering-reject-onchain-immutable-with-freeze = 변경 불가 + 동결 권한 (사기)
filtering-reject-onchain-high-risk-score = 온체인 위험 점수 높음
filtering-reject-dex-empty-name = 이름 없음
filtering-reject-dex-empty-symbol = 심볼 없음
filtering-reject-dex-empty-logo = 로고 URL 없음
filtering-reject-dex-empty-website = 웹사이트 URL 없음
filtering-reject-dex-txn-5m = 5m 트랜잭션 부족
filtering-reject-dex-txn-1h = 1h 트랜잭션 부족
filtering-reject-dex-zero-liq = 유동성 없음
filtering-reject-dex-liq-low = 유동성이 너무 낮음
filtering-reject-dex-liq-high = 유동성이 너무 높음
filtering-reject-dex-mcap-low = 시가총액이 너무 낮음
filtering-reject-dex-mcap-high = 시가총액이 너무 높음
filtering-reject-dex-vol-low = 거래량이 너무 낮음
filtering-reject-dex-vol-missing = 거래량 없음
filtering-reject-dex-fdv-low = FDV가 너무 낮음
filtering-reject-dex-fdv-high = FDV가 너무 높음
filtering-reject-dex-vol5m-low = 5m 거래량이 너무 낮음
filtering-reject-dex-vol5m-missing = 5m 거래량 없음
filtering-reject-dex-vol1h-low = 1h 거래량이 너무 낮음
filtering-reject-dex-vol1h-missing = 1h 거래량 없음
filtering-reject-dex-vol6h-low = 6h 거래량이 너무 낮음
filtering-reject-dex-vol6h-missing = 6h 거래량 없음
filtering-reject-dex-price-change-5m-low = 5m 가격 변동이 너무 낮음
filtering-reject-dex-price-change-5m-high = 5m 가격 변동이 너무 높음
filtering-reject-dex-price-change-low = 가격 변동이 너무 낮음
filtering-reject-dex-price-change-high = 가격 변동이 너무 높음
filtering-reject-dex-price-change-6h-low = 6h 가격 변동이 너무 낮음
filtering-reject-dex-price-change-6h-high = 6h 가격 변동이 너무 높음
filtering-reject-dex-price-change-24h-low = 24h 가격 변동이 너무 낮음
filtering-reject-dex-price-change-24h-high = 24h 가격 변동이 너무 높음
filtering-reject-gecko-liq-low = 유동성이 너무 낮음
filtering-reject-gecko-liq-high = 유동성이 너무 높음
filtering-reject-gecko-mcap-low = 시가총액이 너무 낮음
filtering-reject-gecko-mcap-high = 시가총액이 너무 높음
filtering-reject-gecko-vol5m-low = 5m 거래량이 너무 낮음
filtering-reject-gecko-vol5m-missing = 5m 거래량 없음
filtering-reject-gecko-vol1h-low = 1h 거래량이 너무 낮음
filtering-reject-gecko-vol1h-missing = 1h 거래량 없음
filtering-reject-gecko-vol24h-low = 24h 거래량이 너무 낮음
filtering-reject-gecko-vol24h-missing = 24h 거래량 없음
filtering-reject-gecko-price-change-5m-low = 5m 가격 변동이 너무 낮음
filtering-reject-gecko-price-change-5m-high = 5m 가격 변동이 너무 높음
filtering-reject-gecko-price-change-1h-low = 1h 가격 변동이 너무 낮음
filtering-reject-gecko-price-change-1h-high = 1h 가격 변동이 너무 높음
filtering-reject-gecko-price-change-24h-low = 24h 가격 변동이 너무 낮음
filtering-reject-gecko-price-change-24h-high = 24h 가격 변동이 너무 높음
filtering-reject-gecko-pool-count-low = 풀 수가 너무 적음
filtering-reject-gecko-pool-count-high = 풀 수가 너무 많음
filtering-reject-gecko-pool-count-missing = 풀 수 없음
filtering-reject-gecko-reserve-low = 준비금이 너무 낮음
filtering-reject-gecko-reserve-missing = 준비금 없음
filtering-reject-rug-rugged = 러그풀된 토큰
filtering-reject-rug-score = 위험 점수가 너무 높음
filtering-reject-rug-level-danger = 위험 수준 높음
filtering-reject-rug-mint-authority = 민트 권한 존재
filtering-reject-rug-freeze-authority = 동결 권한 존재
filtering-reject-rug-top-holder = 최대 홀더 비율이 너무 높음
filtering-reject-rug-top3-holders = 상위 3 홀더 비율이 너무 높음
filtering-reject-rug-min-holders = 홀더 수 부족
filtering-reject-rug-insider-count = 인사이더 홀더가 너무 많음
filtering-reject-rug-insider-pct = 인사이더 비율이 너무 높음
filtering-reject-rug-creator-pct = 생성자 보유량이 너무 높음
filtering-reject-rug-transfer-fee-present = 전송 수수료 존재
filtering-reject-rug-transfer-fee-high = 전송 수수료가 너무 높음
filtering-reject-rug-graph-insiders = 그래프 인사이더가 너무 많음
filtering-reject-rug-lp-providers-low = LP 공급자가 너무 적음
filtering-reject-rug-lp-providers-missing = LP 공급자 정보 없음
filtering-reject-rug-lp-lock-low = LP 락이 너무 낮음
filtering-reject-rug-lp-lock-missing = LP 락 정보 없음
filtering-reject-llm-analysis-rejected = LLM 분석 제외: { $reason } (신뢰도 { $confidence }%, { $provider })
filtering-reject-llm-analysis-rejected-generic = LLM 분석 제외
filtering-reject-unknown = { $code }

# Codes no longer emitted; they appear only in stored rows and keep their wording.
filtering-reject-dex-fdv-missing = FDV 없음
filtering-reject-dex-price-change-5m-missing = 5m 가격 변동 없음
filtering-reject-dex-price-change-missing = 가격 변동 없음
filtering-reject-dex-price-change-6h-missing = 6h 가격 변동 없음
filtering-reject-dex-price-change-24h-missing = 24h 가격 변동 없음
filtering-reject-gecko-liq-missing = 유동성 없음
filtering-reject-gecko-mcap-missing = 시가총액 없음
filtering-reject-gecko-price-change-5m-missing = 5m 가격 변동 없음
filtering-reject-gecko-price-change-1h-missing = 1h 가격 변동 없음
filtering-reject-gecko-price-change-24h-missing = 24h 가격 변동 없음
filtering-reject-rug-transfer-fee-missing = 전송 수수료 데이터 없음

# Rejection categories used to group reasons.
filtering-reject-category-security = 보안 문제
filtering-reject-category-distribution = 홀더 분포
filtering-reject-category-liquidity-lock = LP 락 문제
filtering-reject-category-fees = 전송 수수료
filtering-reject-category-liquidity = 유동성
filtering-reject-category-volume = 거래량
filtering-reject-category-market-cap = 시가총액/FDV
filtering-reject-category-price-action = 가격 움직임
filtering-reject-category-activity = 거래 활동
filtering-reject-category-data-quality = 누락된 데이터
filtering-reject-category-timing = 타이밍 필터
filtering-reject-category-market = 시장 데이터
filtering-reject-category-other = 기타

# Filtering page: sub-tabs, sources, status, analytics, explorer and configuration.

## Sub-tabs and sources. Source ids are FilterSource::as_str plus the `meta` settings tab.

filtering-tab-status = 상태
filtering-tab-analytics = 분석
filtering-tab-explorer = 익스플로러
filtering-source-core = 코어
filtering-source-onchain = 온체인
filtering-source-dexscreener = { -dexscreener }
filtering-source-geckoterminal = { -geckoterminal }
filtering-source-rugcheck = { -rugcheck }
filtering-source-llm-analysis = LLM 분석

## Time range

filtering-range-1h = 1H
filtering-range-6h = 6H
filtering-range-24h = 24H
filtering-range-7d = 7D
filtering-range-all = 전체
filtering-range-all-time = 전체 기간
filtering-range-custom = 직접 설정
filtering-range-now = 현재
# $start and $end are formatted moments, or the open-ended markers.
filtering-range-span = { $start } → { $end }
# $min and $max are the two ends of a value range.
filtering-range-bounds = { $min } – { $max }

## Footer status line

filtering-footer-saving = 변경 사항 저장 중...
filtering-footer-refreshing = 스냅샷 새로 고치는 중...
filtering-footer-unsaved = 저장되지 않은 변경 사항 있음
# $time is a relative time such as "5m ago".
filtering-footer-last-saved = 마지막 저장: { $time }
filtering-footer-in-sync = 설정이 동기화됨

## Info bar and status metrics

filtering-info-total = 전체
filtering-info-priced = 가격 있음
filtering-info-passed = 통과
filtering-info-positions = 포지션
filtering-info-blacklisted = 블랙리스트
filtering-info-cache = 캐시
# A count followed by its share of the total, e.g. "120 (4.0%)".
filtering-count-share = { $count } ({ $share })
filtering-refresh-building = 생성 중…
filtering-refresh-never = 없음

filtering-status-loading = 통계 불러오는 중...
filtering-status-total = 전체 토큰
filtering-status-total-detail = 필터링 캐시 내
filtering-status-total-detail-building = 스냅샷 생성 중 - 다음 새로 고침에 집계됩니다
filtering-status-priced = 가격 있음
filtering-status-priced-detail = { $share }에 가격 있음
filtering-status-passed = 필터 통과
filtering-status-passed-detail = { $share } 통과
filtering-status-positions = 보유 포지션
filtering-status-positions-detail = 진행 중인 거래
filtering-status-blacklisted = 블랙리스트
filtering-status-blacklisted-detail = 표시된 토큰
filtering-status-ohlcv = OHLCV 있음
filtering-status-ohlcv-detail = 과거 데이터
filtering-status-refresh = 마지막 새로 고침
filtering-status-refresh-building = 첫 스냅샷 생성 중
filtering-status-refresh-none = 아직 새로 고침 안 됨
filtering-status-no-rejections = 제외 데이터가 없습니다

## Analytics

filtering-analytics-loading = { $range } 분석 불러오는 중…
filtering-analytics-scanned = 전체 스캔
# $time is a relative time such as "5m ago".
filtering-analytics-updated = 업데이트: { $time }
filtering-analytics-passed = 통과한 토큰
filtering-analytics-pass-rate = 통과율 <strong>{ $share }</strong>
filtering-analytics-rejected = 제외된 토큰
filtering-analytics-rejection-rate = 제외율 <strong>{ $share }</strong>
filtering-analytics-by-category = 카테고리별 제외
filtering-analytics-by-source = 소스별 제외
filtering-analytics-no-category = 카테고리 데이터 없음
filtering-analytics-no-source = 소스 데이터 없음
filtering-analytics-top-reasons = 주요 제외 사유
filtering-analytics-no-data = 사용 가능한 데이터가 없습니다
filtering-analytics-column-reason = 사유
filtering-analytics-column-category = 카테고리
filtering-analytics-column-count = 건수
filtering-analytics-column-share = %
filtering-analytics-column-impact = 영향
# $amount is the formatted count, $count selects the plural.
filtering-tokens-count =
    { $count ->
       *[other] 토큰 { $amount }개
    }

## Explorer

filtering-explorer-top-reasons = 주요 사유
filtering-explorer-recent = 최근 제외
filtering-explorer-none = 데이터 없음
filtering-explorer-none-recent = 최근 항목 없음
filtering-explorer-search =
    .placeholder = 사유 검색...
filtering-explorer-overview = 개요
filtering-explorer-no-match = 일치하는 사유 없음
filtering-explorer-column-token = 토큰
filtering-explorer-column-source = 소스
filtering-explorer-column-time = 시각
filtering-explorer-page = { $page }페이지
filtering-explorer-no-results = 결과 없음
filtering-explorer-empty = 토큰을 찾을 수 없습니다
filtering-explorer-empty-filtered = 필터와 일치하는 토큰이 없습니다
filtering-explorer-load-failed = 토큰을 불러오지 못했습니다

## Configuration panels

filtering-config-loading = 설정 불러오는 중…
# $query is the text typed in the filter box.
filtering-config-no-match = “{ $query }”와 일치하는 매개변수가 없습니다
filtering-config-no-parameters = 이 소스는 매개변수를 제공하지 않습니다
# $source is the source name.
filtering-source-off = { $source } 필터링이 꺼져 있습니다 - 이 매개변수는 평가되지 않습니다.
filtering-toolbar-filter =
    .placeholder = 매개변수 필터
    .aria-label = 매개변수 필터
filtering-toolbar-clear =
    .aria-label = 필터 지우기
# $count selects the plural, $amount is the number shown.
filtering-parameter-count =
    { $count ->
       *[other] 매개변수 { $amount }개
    }
# $count is the total and selects the plural.
filtering-parameter-count-filtered =
    { $count ->
       *[other] 매개변수 { $total }개 중 { $visible }개
    }
filtering-group-enable =
    .aria-label = { $group } 검사 사용
filtering-field-min = 최소
filtering-field-max = 최대
# $label is the parameter name.
filtering-field-min-aria =
    .aria-label = 최소 { $label }
filtering-field-max-aria =
    .aria-label = 최대 { $label }
# $default is the shipped value, $label the parameter name.
filtering-field-reset =
    .title = 기본값으로 초기화 ({ $default })
    .aria-label = { $label } 기본값으로 초기화

## Toasts. A message value is the title; `.message` is the body.

filtering-toast-saved = 설정 저장됨
    .message = 필터링 설정을 저장하고 스냅샷을 새로 고쳤습니다
filtering-toast-save-failed = 저장 실패
    .message = 필터링 설정을 저장하지 못했습니다
filtering-toast-reset = 변경 사항 초기화됨
    .message = 마지막으로 저장된 상태로 설정을 복원했습니다
filtering-toast-refresh-failed = 새로 고침 실패
    .message = 필터링 스냅샷을 새로 고치지 못했습니다
filtering-toast-exported = 설정 내보내기 완료
    .message = 필터링 설정이 파일에 저장되었습니다
filtering-toast-imported = 설정 가져오기 완료
    .message = 파일에서 필터링 설정을 불러왔습니다
filtering-toast-import-failed = 가져오기 실패
    .message = 설정을 가져오지 못했습니다 - 잘못된 파일 형식입니다
filtering-toast-load-failed = 불러오기 실패
    .message = 필터링 설정을 불러오지 못했습니다
filtering-toast-range-missing = 시작일과 종료일을 모두 선택하세요
filtering-toast-range-order = 시작 시각은 종료 시각보다 이전이어야 합니다
filtering-toast-range-future = 종료 시각은 미래일 수 없습니다
