# Telegram bot text. Server-only: rendered by src/telegram/text.rs, never sent to the dashboard.

## Reply keyboard words.

telegram-reply-status = 상태
telegram-reply-balance = 잔액
telegram-reply-positions = 포지션
telegram-reply-pause = 일시 중지
telegram-reply-resume = 재개
telegram-reply-stop = 중지
telegram-reply-stats = 통계
telegram-reply-menu = 메뉴
telegram-reply-help = 도움말

## Inline keyboard buttons.

telegram-button-positions = 포지션
telegram-button-balance = 잔액
telegram-button-stats = 통계
telegram-button-tokens = 토큰
telegram-button-pause = 일시 중지
telegram-button-stop = 중지
telegram-button-settings = 설정
telegram-button-refresh = 새로 고침
telegram-button-menu = 메뉴
telegram-button-back = 뒤로
telegram-button-back-to-menu = 메뉴로 돌아가기
telegram-button-back-to-tokens = 토큰으로 돌아가기
telegram-button-cancel = 취소
telegram-button-close-all-positions = 모든 포지션 종료
telegram-button-sell-percent = { $percent }% 매도
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = 블랙리스트
telegram-button-blacklist-symbol = { $symbol } 블랙리스트
telegram-button-close-position = 포지션 종료
telegram-button-confirm-close = 종료 확인
telegram-button-confirm-close-all = 모든 포지션 종료
telegram-button-confirm-sell = { $percent }% 매도 확인
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = 강제 중지 확인
telegram-button-confirm-buy = { $amount } { -sol } 매수
telegram-button-notifications = 알림
telegram-button-trading = 거래
telegram-button-entry-monitor = 진입 모니터
telegram-button-exit-monitor = 청산 모니터
telegram-button-auto-trading = 자동 거래
telegram-button-force-stop = 강제 중지
telegram-button-notify-opened = 진입
telegram-button-notify-closed = 종료
telegram-button-notify-partial = 부분
telegram-button-notify-dca = DCA
telegram-button-notify-errors = 오류
telegram-button-details = 상세
telegram-button-position = 포지션
telegram-button-sell-more = 추가 매도
telegram-button-more-dca = DCA 추가
telegram-button-history = 기록
telegram-button-status = 상태
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = 재인증
telegram-button-previous = 이전
telegram-button-next = 다음
telegram-button-passed = 통과
telegram-button-rejected = 제외
telegram-button-new-24h = 신규 (24h)
telegram-button-all-tokens = 전체 토큰
telegram-button-search-token = 토큰 검색
telegram-button-filter-stats = 필터 통계
telegram-button-refresh-stats = 통계 새로 고침
telegram-button-view-position = 포지션 보기
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    알 수 없는 명령: { $command }

    사용 가능한 명령은 /help로 확인하세요.
telegram-session-expired =
    <b>세션 만료</b>

    다시 인증하려면 /login을 사용하세요.
telegram-2fa-required =
    <b>2FA 필요</b>

    6자리 인증 코드를 입력하세요.
telegram-account-locked =
    <b>계정 잠김</b>

    실패한 시도가 너무 많습니다.
    { $seconds }초 후 다시 시도하세요.
telegram-code-invalid = 올바른 6자리 코드를 입력하세요.
telegram-authenticated =
    <b>인증되었습니다!</b>

    이제 봇 명령을 사용할 수 있습니다.
telegram-wrong-code =
    <b>잘못된 코드</b>

    남은 시도 횟수: { $remaining }회
telegram-auth-required =
    <b>인증 필요</b>

    계속하려면 비밀번호를 입력하세요.

    <i>비밀번호를 입력하여 전송하세요.</i>
telegram-login-required =
    <b>로그인 필요</b>

    6자리 인증 코드를 입력하세요:
telegram-session-activated =
    <b>세션 활성화됨</b>

    2FA가 설정되어 있지 않습니다. 세션이 활성화되었습니다.

    <i>팁: 보안 설정에서 2FA를 활성화하면 보안이 강화됩니다.</i>

## Chat discovery.

telegram-discovery-hello = 안녕하세요, { $name }님!
telegram-discovery-default-name = 사용자
telegram-discovery-detected = <b>채팅이 감지되었습니다!</b>
telegram-discovery-details =
    채팅 ID: <code>{ $chat_id }</code>
    유형: { $chat_type }

    { -brand } 대시보드로 이동하여 이 채팅을 클릭해 선택하세요.
telegram-chat-type-private = 개인
telegram-chat-type-group = 그룹
telegram-chat-type-supergroup = 슈퍼그룹
telegram-chat-type-channel = 채널

## Menus.

telegram-menu-title =
    <b>제어판</b>

    옵션을 선택하여 정보를 확인하거나 봇을 제어하세요.
telegram-menu-positions-empty =
    <b>보유 포지션 없음</b>

    새로운 기회를 기다리는 중...
telegram-menu-positions-title = <b>포지션 ({ $count })</b>
telegram-menu-positions-hint = <i>포지션을 탭하여 관리하세요.</i>
telegram-menu-settings =
    <b>설정</b>

    알림과 거래 매개변수를 설정합니다.
telegram-settings-notifications =
    <b>알림 설정</b>

    알림을 켜거나 끄세요:
telegram-settings-trading =
    <b>거래 제어</b>

    거래 기능을 켜거나 끄세요:
telegram-pagination-expired = 페이지 세션이 만료되었습니다.

## Status commands.

telegram-status-state-stopped = <b>중지됨</b> (강제 중지 활성)
telegram-status-state-active = <b>활성</b>
telegram-status-state-paused = <b>일시 중지됨</b>
telegram-status-on = 사용
telegram-status-off = 사용 안 함
telegram-status-body =
    <b>시스템 상태</b>

    <b>시스템</b>
    상태 — { $state }
    가동 시간 — { $uptime }
    버전 — v{ $version }

    <b>거래</b>
    진입 — { $entries }
    청산 — { $exits }
    포지션 — { $positions }
telegram-positions-empty =
    <b>보유 포지션 없음</b>

    기회를 기다리는 중...
telegram-positions-title = <b>보유 포지션 ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+{ $count }개 더...</i>
telegram-positions-summary =
    <b>포트폴리오 요약</b>
    투자금 — { $invested } { -sol }
    순 P{ "&amp;" }L — { $pnl } { -sol }
telegram-balance-body =
    <b>지갑 잔액</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>일일 통계</b>

    포지션 — { $positions }
    투자금 — { $invested } { -sol }
    P{ "&amp;" }L — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } 준비 완료!</b>

    거래가 <b>활성화</b>되었습니다.

    아래 키보드로 봇을 제어하세요.
    사용 가능한 명령은 /help를 입력하세요.
telegram-stop-already = <b>거래가 이미 비활성화되어 있습니다</b>
telegram-stop-done =
    <b>거래 비활성화됨</b>

    모든 거래 모니터(진입 { "&amp;" } 청산)가 중지되었습니다.
    진입만 중지하려면 /pause를 사용하세요.
telegram-stop-failed =
    <b>거래를 비활성화하지 못했습니다</b>

    오류: { $detail }
telegram-pause-done =
    <b>진입 모니터 일시 중지됨</b>

    새 포지션이 열리지 않습니다.
    청산 모니터는 계속 실행됩니다.
telegram-pause-failed =
    <b>진입을 일시 중지하지 못했습니다</b>

    오류: { $detail }
telegram-resume-done =
    <b>진입 모니터 재개됨</b>

    이제 진입 시그널을 감시합니다.
telegram-resume-failed =
    <b>진입을 재개하지 못했습니다</b>

    오류: { $detail }
telegram-force-stop-confirm =
    <b>강제 중지</b>

    모든 거래 활동이 즉시 중단됩니다:
    • 신규 진입 없음
    • 청산 없음 (손절 포함)
    • DCA 작업 없음
telegram-force-stop-warning = <b>긴급 조치입니다!</b>
telegram-force-stop-question = 계속하시겠습니까?
telegram-force-stop-active =
    <b>강제 중지 활성화됨</b>

    모든 거래가 중단되었습니다.

    이 플래그를 해제하려면 /resume_trading을 사용하세요.
telegram-resume-trading-not-stopped =
    <b>거래가 강제 중지 상태가 아닙니다</b>

    필요한 조치가 없습니다.
telegram-resume-trading-done =
    <b>거래 재개됨</b>

    강제 중지 플래그가 해제되었습니다.
    이제 정상적인 거래 동작이 재개될 수 있습니다.

## Help.

telegram-help-title = <b>{ -brand } 도움말</b>
telegram-help-heading-dashboard = 대시보드
telegram-help-heading-market = 시장
telegram-help-heading-trading = 거래
telegram-help-heading-safety = 안전
telegram-help-heading-system = 시스템
telegram-help-commands-dashboard =
    /status — 시스템 상태 { "&amp;" } 가동 시간
    /stats — 일일 성과
    /balance — 지갑 잔액
    /positions — 보유 포지션
telegram-help-commands-market =
    /tokens — 토큰 탐색기
    /rejected — 필터링된 토큰
telegram-help-commands-trading =
    /start — 거래 시스템 활성화
    /stop — 거래 시스템 비활성화
    /pause — 신규 진입 일시 중지
    /resume — 신규 진입 재개
    /menu — 대화형 메뉴
telegram-help-commands-safety =
    /force_stop — <b>긴급 중단</b>
    /resume_trading — 긴급 상태 해제
telegram-help-commands-system =
    /update — 업데이트 상태 { "&amp;" } 설치
    /login — 2FA 인증
telegram-help-tip = <i>팁: 명령을 탭하면 실행됩니다.</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>최신 버전입니다</b>

    v{ $version } 실행 중, 자동으로 설치되었습니다.
telegram-update-up-to-date =
    <b>최신 버전입니다</b>

    v{ $version } 실행 중.
telegram-update-check-failed =
    <b>업데이트 확인 실패</b>

    { $reason }
telegram-update-unreachable = screenerbot.io에 연결할 수 없습니다.
telegram-update-installing = <b>v{ $version } 설치 중</b>
telegram-update-restarting =
    재시작 중: { -brand }. 새 버전이 적용되며 거래는 자동으로 재개됩니다.
telegram-update-install-failed =
    <b>v{ $version }을 설치할 수 없습니다</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } 다운로드 완료</b>

    이 릴리스는 데스크톱 앱도 업데이트하므로 해당 기기에서 설치 프로그램을 실행해야 합니다. 설정 → 업데이트를 여세요.
telegram-update-downloading =
    <b>v{ $version } 다운로드 중</b>

    { $size } MB 중 { $percent }%.
telegram-update-available =
    <b>v{ $version } 업데이트 가능</b>

    { $how }
    다운로드 크기: { $size } MB.

    자동으로 다운로드됩니다. 준비되면 /update를 다시 보내세요.
telegram-update-how-core = 짧은 재시작과 함께 자동으로 설치됩니다.
telegram-update-how-installer = 데스크톱 설치 프로그램을 한 번 실행해야 합니다.

## Shared values and units.

telegram-value-unknown = 알 수 없음
telegram-value-na = 해당 없음
telegram-percent-value = { $percent }%
telegram-price-native = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds }초
telegram-duration-minutes = { $minutes }분
telegram-duration-minutes-seconds = { $minutes }분 { $seconds }초
telegram-duration-hours = { $hours }시간
telegram-duration-hours-minutes = { $hours }시간 { $minutes }분
telegram-duration-days = { $days }일
telegram-duration-days-hours = { $days }일 { $hours }시간
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-native = { $amount } { -sol }
telegram-error-line = 오류: { $detail }
telegram-ai-reasoning =
    <b>LLM 분석</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens.

telegram-row-entry = 진입 — { $price } { -sol }
telegram-row-exit = 청산 — { $price } { -sol }
telegram-row-current = 현재 — { $price } { -sol }
telegram-row-invested = 투자금 — { $amount } { -sol }
telegram-row-received = 수령 — { $amount } { -sol }
telegram-row-value = 가치 — { $amount } { -sol }
telegram-row-total = 합계 — { $amount } { -sol }
telegram-row-tokens = 토큰 — { $tokens }
telegram-row-duration = 보유 시간 — { $duration }
telegram-row-reason = 사유 — { $reason }
telegram-row-remaining = 잔여 — { $percent }%
telegram-row-pnl = P{ "&amp;" }L — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>포지션 진입</b>
telegram-notify-opened-size = 규모 — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = 가격 — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>포지션 종료</b> — 수익
telegram-notify-closed-title-loss = <b>포지션 종료</b> — 손실
telegram-notify-closed-reason-unspecified = 종료됨
telegram-notify-partial-title = <b>부분 청산</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — { $percent }% 매도
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = 추가 — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = 평균 — { $price } { -sol }
telegram-notify-unbooked-title = <b>기록되지 않은 스왑</b>
telegram-notify-unbooked-body = 온체인에서 확정된 스왑이 아직 포지션에 기록되지 않았습니다. 기록될 때까지 다시 검증합니다.
telegram-notify-unbooked-signature = 트랜잭션: <code>{ $signature }</code>
telegram-notify-severity-critical = <b>치명적 오류</b>
telegram-notify-severity-error = <b>오류</b>
telegram-notify-severity-warning = <b>경고</b>
telegram-notify-severity-info = <b>정보</b>
telegram-notify-alert-title = <b>거래 알림</b>
telegram-notify-alert-token = 토큰: <code>${ $symbol }</code>
telegram-notify-alert-mint = 민트: <code>{ $mint }</code>
telegram-notify-alert-bought = 동작: { $amount } { -sol } 매수
telegram-notify-alert-sold = 동작: { $amount } { -sol } 매도
telegram-notify-alert-wallet = 지갑: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (모의)
telegram-notify-copy-task = 작업: { $task }
telegram-notify-scheduled-completed = <b>예약 작업 완료</b>
telegram-notify-scheduled-failed = <b>예약 작업 실패</b>
telegram-notify-scheduled-timed-out = <b>예약 작업 시간 초과</b>
telegram-notify-scheduled-error = 오류: { $error }
telegram-notify-summary-title = <b>일일 요약</b> — { $date }
telegram-notify-summary-performance = <b>성과</b>
telegram-notify-summary-trades = 거래 — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = 승률 — { $percent }%
telegram-notify-summary-pnl = P{ "&amp;" }L — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = 보유 포지션 — { $count }
telegram-notify-started-title = <b>{ -brand } 시작됨</b>
telegram-notify-started-version = <b>버전</b> — { $version }
telegram-notify-started-mode = <b>모드</b> — { $mode }
telegram-notify-started-ready = 거래 준비 완료!
telegram-notify-stopped-title = <b>{ -brand } 중지됨</b>
telegram-notify-stopped-reason = <b>사유</b> — { $reason }
telegram-notify-stopped-goodbye = 안녕히 가세요! { $icon }
telegram-notify-start-mode-normal = 일반
telegram-notify-stop-reason-graceful = 정상 종료
telegram-notify-update-available =
    <b>업데이트 v{ $version } 사용 가능</b>

    { $how }
    다운로드 크기: { $size } MB
telegram-notify-update-how-installer = 이 릴리스는 데스크톱 앱도 업데이트하므로 설치 프로그램을 한 번 실행해야 합니다.
telegram-notify-update-ready =
    <b>업데이트 v{ $version } 준비 완료</b>

    { $how }
telegram-notify-update-ready-silent = 지금 적용하려면 /update를 보내세요. 그렇지 않으면 다음에 { -brand } 시작 시 설치됩니다.
telegram-notify-update-ready-installer = 설정 → 업데이트를 열어 설치 프로그램을 실행하세요.
telegram-notify-update-applying =
    <b>v{ $version } 설치 중</b>

    백엔드를 재시작하는 중이며 거래는 자동으로 재개됩니다.
telegram-notify-new-tokens =
    <b>필터링 알림</b>

    { $count ->
       *[other] 조건에 맞는 새 토큰 { $count }개를 발견했습니다.
    }
telegram-notify-crash =
    <b>봇이 중단되었습니다!</b>

    <b>위치:</b> <code>{ $location }</code>
    <b>오류:</b> <code>{ $error }</code>
telegram-notify-crash-restart = 봇을 다시 시작하세요.

## Filter results page.

telegram-filter-results-title = <b>필터 결과</b> ({ $count })
telegram-filter-results-empty = <i>토큰이 없습니다.</i>
telegram-filter-results-page = <i>{ $page } / { $total } 페이지</i>

## Position screens.

telegram-position-not-found = 포지션을 찾을 수 없습니다
telegram-position-no-positions = 종료할 포지션이 없습니다
telegram-position-history-empty =
    <b>거래 기록</b>

    종료된 포지션이 아직 없습니다.
telegram-position-history-title = <b>최근 거래</b>
telegram-position-history-more = <i>+{ $count }건의 거래 더...</i>
telegram-position-confirm-hint = <i>30초 이내에 확인하면 실행됩니다.</i>
telegram-position-confirm-close-title = <b>포지션을 종료하시겠습니까?</b>
telegram-position-confirm-close-selling = 토큰 { $tokens } 매도
telegram-position-confirm-close-estimated = 예상 — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>30초 이내에 확인하세요</i>
telegram-position-confirm-sell =
    <b>매도 확인</b>

    토큰 — { $symbol }
    수량 — { $percent }%
    토큰 — { $tokens }
telegram-position-confirm-dca =
    <b>추가 매수 확인</b>

    토큰 — { $symbol }
    추가 — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>모든 포지션을 종료하시겠습니까?</b>

    개수 — { $count }
telegram-position-confirm-close-all-hint =
    <i>모든 보유 포지션을 시장가로 매도합니다.
    30초 이내에 확인하세요.</i>
telegram-position-confirm-force-stop =
    <b>강제 중지</b>

    모든 거래가 즉시 중단됩니다:
    • 신규 진입 없음
    • 청산 없음
    • DCA 없음
telegram-position-confirm-force-stop-warning = <b>긴급 조치입니다.</b>
telegram-position-confirm-blacklist =
    <b>토큰을 블랙리스트에 추가하시겠습니까?</b>

    토큰 — { $symbol }
    민트 — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>포지션이 종료되고 향후 진입이 차단됩니다.</i>
telegram-position-selling = { $symbol }의 { $percent }%를 매도하는 중...
telegram-position-sell-done =
    <b>매도 실행됨</b>

    토큰 — { $symbol }
    매도 — { $percent }%
    수령 — { $amount } { -sol }
telegram-position-sell-failed = <b>매도 실패</b>
telegram-position-adding = { $symbol }에 { $amount } { -sol } 추가하는 중...
telegram-position-dca-done =
    <b>DCA 실행됨</b>

    토큰 — { $symbol }
    추가 — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA 실패</b>
telegram-position-closing-all = 모든 포지션을 종료하는 중...
telegram-position-close-all-done =
    <b>전체 종료 완료</b>

    종료 — { $closed }
    실패 — { $failed }
telegram-position-blacklisted =
    <b>토큰 블랙리스트 등록됨</b>

    토큰 — { $symbol }
    상태 — 종료 { "&amp;" } 블랙리스트 등록

## Token screens.

telegram-token-not-found = 토큰을 찾을 수 없습니다
telegram-token-not-found-prefix = 토큰을 찾을 수 없습니다. 더 긴 접두사로 검색해 보세요.
telegram-token-stats-failed = 통계를 가져오지 못했습니다: { $detail }
telegram-token-list-failed = 토큰을 가져오지 못했습니다: { $detail }
telegram-token-list-empty = <b>{ $view }</b> 보기에 토큰이 없습니다.
telegram-token-view-passed = 필터 통과
telegram-token-view-rejected = 제외
telegram-token-view-recent = 최근 추가
telegram-token-view-all = 전체 토큰
telegram-token-list-title = <b>{ $name }</b> ({ $page }/{ $total } 페이지)
telegram-token-list-stats = 유동성: { $liquidity } • 가격: { $price }
telegram-token-list-hint = <i>/token_ID를 탭하면 상세 정보를 볼 수 있습니다</i>
telegram-token-explorer =
    <b>시장 탐색기</b>

    <b>개요</b>
    필터 통과 — { $passed }
    제외 — { $rejected }
    가격 활성 — { $priced }
    전체 발견 — { $total }

    <i>탐색할 카테고리를 선택하세요:</i>
telegram-token-filter-title = <b>필터 분석</b>
telegram-token-filter-distribution = <b>분포</b>
telegram-token-filter-passed = 통과 — { $count } ({ $percent }%)
telegram-token-filter-rejected = 제외 — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = 블랙리스트 — { $count }
telegram-token-filter-coverage = <b>커버리지</b>
telegram-token-filter-priced = 풀 가격 있음 — { $count }
telegram-token-filter-open = 보유 포지션 — { $count }
telegram-token-filter-total = 전체 발견 — { $count }
telegram-token-filter-updated = <b>마지막 업데이트</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>{ $interval }마다 자동 새로 고침</i>
telegram-token-detail-active = <b>활성 포지션</b>
telegram-token-detail-price = 가격 — { $price } { -sol }
telegram-token-detail-liquidity = 유동성 — { $value }
telegram-token-detail-volume = 24h 거래량 — { $value }
telegram-token-detail-change = 24h 변동 — { $value }
telegram-token-detail-risk = 위험 평가: { $score }/100
telegram-token-detail-risk-unknown = 위험 평가: 알 수 없음
telegram-token-detail-action = <i>동작을 선택하세요:</i>
telegram-token-search =
    <b>시장 검색</b>

    검색할 심볼 또는 민트 주소를 입력하세요:

    <i>예: /token_BONK 또는 /token_So11111</i>
telegram-token-confirm-buy =
    <b>직접 매수 확인</b>

    토큰 — ${ $symbol }
    민트 — <code>{ $mint }</code>
    수량 — { $amount } { -sol }

    <i>30초 이내에 확인하면 실행됩니다.</i>
telegram-token-confirm-blacklist =
    <b>토큰을 블랙리스트에 추가하시겠습니까?</b>

    토큰 — ${ $symbol }
    민트 — <code>{ $mint }</code>

    <i>이 토큰이 필터 조건을 충족하지 못하도록 차단됩니다.</i>
telegram-token-blacklisted =
    <b>토큰 블랙리스트 등록됨</b>

    토큰 — ${ $symbol }
    상태 — 블랙리스트에 추가됨
telegram-token-blacklist-failed = <b>블랙리스트 등록 실패</b>
telegram-token-buy-processing =
    <b>매수 처리 중...</b>

    토큰 — ${ $symbol }
    수량 — { $amount } { -sol }
telegram-token-buy-done =
    <b>매수 성공</b>

    토큰 — ${ $symbol }
    수량 — { $amount } { -sol }

    <i>상세 정보는 /positions에서 확인하세요</i>
telegram-token-buy-failed =
    <b>매수 실패</b>

    토큰 — ${ $symbol }
    오류 — { $detail }
