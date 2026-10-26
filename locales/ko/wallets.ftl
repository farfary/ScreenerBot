# Wallet page labels.

# Wallet types. Ids come from WalletType in src/wallets/types.rs.
wallets-type-generated = 생성됨
wallets-type-imported = 가져옴
wallets-type-migrated = 이전됨

# Why a watched wallet is paused. Ids come from WatchDisableReason in
# src/wallets/watch/types.rs. $limit is a signature count.
wallets-watch-disabled-user = 사용자가 일시 중지함
wallets-watch-disabled-signature-budget = 일시 중지됨: 따라잡기 전에 서명 { $limit }건 확인 한도에 도달했습니다
wallets-watch-disabled-unknown = 일시 중지됨: 저장된 감시 안전 사유를 읽을 수 없습니다
wallets-watch-disabled-helius-unavailable = 일시 중지됨: 고활동 제공자를 사용할 수 없습니다. 커서는 유지됩니다
wallets-watch-disabled-processing-failed = 일시 중지됨: 지갑 활동을 처리하지 못했습니다. 커서는 유지됩니다

# Last runtime problem of a watch. Ids come from WatchRuntimeError in
# src/wallets/watch/types.rs.
wallets-watch-error-provider-unavailable = 고활동 제공자를 사용할 수 없어 감시가 일시 중지되었습니다
wallets-watch-error-provider-repeated-failure = { -helius } 확인이 반복해서 실패하여 감시가 일시 중지되었습니다
wallets-watch-error-processing-repeated-failure = 지갑 활동 처리가 반복해서 실패하여 감시가 일시 중지되었습니다
wallets-watch-error-position-unreadable = 지갑 감시가 저장된 위치를 읽지 못했습니다. 다시 시도합니다
wallets-watch-error-provider-check-failed = 고활동 제공자 확인에 실패했습니다. 다시 시도합니다
wallets-watch-error-decode-failed = 고활동 트랜잭션을 디코딩하지 못했습니다. 커서는 유지됩니다
wallets-watch-error-processing-failed = 지갑 활동을 처리하지 못했습니다. 다시 시도합니다
wallets-watch-error-position-save-failed = 지갑 감시가 위치를 저장하지 못했습니다. 다시 시도합니다

# Why a watched wallet is paused, as a second line under its status. Ids come from
# WatchDisableReason in src/wallets/watch/types.rs, named after the serialized kind.
# The `unknown` kind has no detail line.
wallets-watch-reason-user = 사용자가 일시 중지했습니다.
wallets-watch-reason-signature-budget = 이 지갑은 현재 감시 설정으로 확인할 수 있는 양보다 활동이 많습니다.
wallets-watch-reason-helius-unavailable = { -helius } 확인에 실패했습니다. 저장된 진행 상태는 유지됩니다.
wallets-watch-reason-processing-failed = 지갑 활동을 처리하지 못했습니다. 저장된 진행 상태는 유지됩니다.

# Vocabulary shared by the wallet tables and dialogs.
wallets-modal-close =
    .aria-label = 창 닫기
wallets-summary-native = { -sol }
wallets-copied-private-key = 개인 키

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = 메인 지갑
wallets-tab-other = 기타 지갑
wallets-tab-watched = 감시 중
wallets-refresh-failed = 지갑을 새로 고치지 못했습니다
wallets-action-failed = 실패
wallets-toast-failed = 실패: { $reason }
wallets-export-busy = 복호화 중...
wallets-export-revealed = 키가 표시되었습니다. 안전하게 다루세요

# wallets.html: Watch Wallet dialog.
wallets-watch-add-title = 지갑 감시
wallets-watch-add-address = 지갑 주소
wallets-watch-add-address-input =
    .placeholder = Solana 주소
wallets-watch-add-address-hint = 지갑의 온체인 활동을 기록하고 { -telegram } 설정에 따라 거래 알림을 보냅니다.
wallets-watch-add-label = 라벨
wallets-watch-add-label-input =
    .placeholder = 이름 (선택)
wallets-watch-add-submit = 감시 추가

# wallets.html and watched.js: watch options dialog.
wallets-watch-budget-title-options = 지갑 감시 옵션
wallets-watch-budget-title-restore = 지갑 감시 복원
wallets-watch-budget-close =
    .aria-label = 닫기
wallets-watch-budget-label-signatures = 확인당 서명 확인 수
wallets-watch-budget-label-transactions = 확인당 성공한 전체 트랜잭션 확인 수
wallets-watch-budget-hint-signatures = 현재 한도: { $limit }. 확인당 서명 500~5,000건을 100건 단위로 선택하세요.
wallets-watch-budget-hint-transactions = 현재 한도: { $limit }. 확인당 성공한 트랜잭션 500~5,000건을 100건 단위로 선택하세요.
wallets-watch-budget-error-range = 확인당 레코드 500~5,000건을 100건 단위로 선택하세요.
wallets-watch-budget-error-ack = 마지막으로 완료된 확인 이후의 서명은 건너뛴다는 점에 동의해야 합니다.
wallets-watch-budget-save-failed = 감시 한도를 저장하지 못했습니다.
wallets-watch-budget-save = 한도 저장
wallets-watch-budget-resume = 지금부터 재개
wallets-watch-budget-resume-notice = 이 지갑은 따라잡기 전에 확인 한도에 도달했습니다. 지금부터 재개하면 최신 지갑 활동부터 시작하며, 마지막으로 완료된 확인 이후의 활동은 복사되지 않습니다.
wallets-watch-budget-resume-tasks = 카피 작업은 카피 트레이딩에서 각 작업을 재개할 때까지 일시 중지 상태로 유지됩니다.
wallets-watch-budget-resume-ack = 놓친 활동은 복사되지 않는다는 점을 이해했습니다.
wallets-watch-budget-resumed = 현재 지갑 최신 지점부터 감시를 재개했습니다
wallets-watch-budget-updated = 지갑 감시 한도가 업데이트되었습니다
wallets-watch-helius-allow = 필요 시 { -helius } 따라잡기 허용
wallets-watch-helius-try = { -helius }로 따라잡기 시도
wallets-watch-helius-stop = 이 지갑의 { -helius } 따라잡기 중지
wallets-watch-helius-description-approved = 이 지갑에 { -helius } 따라잡기가 허용되어 있습니다. 끄면 표준 확인으로 돌아가며, 활동이 많은 지갑은 뒤처질 수 있습니다.
wallets-watch-helius-description-available = { -helius }은 확인하지 않은 구간을 건너뛰지 않고 저장된 위치부터 성공한 Solana 트랜잭션을 확인할 수 있습니다. 제공자 크레딧을 더 사용할 수 있으며 여전히 뒤처질 수 있습니다.
wallets-watch-helius-description-unavailable = { -helius } 따라잡기를 사용할 수 없습니다. 사용하려면 활성화된 { -helius } RPC 엔드포인트를 설정하세요.
wallets-watch-helius-description-unsupported = 이 감시에서 지원하는 따라잡기 제공자가 없습니다. 감시가 한도에 도달하면 지금부터 재개를 사용할 수 있습니다.
wallets-watch-helius-allow-title = 이 지갑에 { -helius } 따라잡기 허용
wallets-watch-helius-allow-message = { -helius }은 확인하지 않은 구간을 건너뛰지 않고 저장된 위치부터 성공한 Solana 트랜잭션을 확인할 수 있습니다. 현재 반환된 전체 트랜잭션 100건당 10 크레딧(올림)이 부과되며 요청당 최소 10 크레딧입니다. 한 번의 확인에 여러 요청이 발생할 수 있으며 사용량과 제공자 요금은 달라질 수 있습니다. 카피 작업은 별도로 재개할 때까지 일시 중지 상태로 유지됩니다.
wallets-watch-helius-allow-confirm = 이 지갑에 허용
wallets-watch-helius-stop-message = 이 지갑은 표준 확인으로 돌아갑니다. 활동이 많은 지갑은 감시 한도에 도달해 다시 일시 중지될 수 있습니다. 다른 지갑과 { -helius } RPC 설정은 변경되지 않습니다.
wallets-watch-helius-stop-confirm = 이 지갑에서 중지
wallets-watch-helius-stop-keep = 허용 유지
wallets-watch-helius-restored = 저장된 진행 상태로 감시를 복원했습니다. 카피 작업은 일시 중지 상태로 유지됩니다
wallets-watch-helius-allowed = 필요 시 이 지갑에 { -helius } 따라잡기가 허용되었습니다
wallets-watch-helius-stopped = 이 지갑의 { -helius } 따라잡기가 중지되었습니다
wallets-watch-helius-update-failed = 지갑 따라잡기 설정을 업데이트하지 못했습니다

# wallets.html: Export Private Key dialog.
wallets-export-title = 개인 키 내보내기
wallets-export-warning-title = 중대한 보안 경고
wallets-export-warning-body = 개인 키를 절대 다른 사람과 공유하지 마세요. 이 키에 접근할 수 있는 사람은 이 지갑의 모든 자금을 훔칠 수 있습니다.
wallets-export-key-label = 개인 키 (Base58)
wallets-export-copy =
    .title = 클립보드에 복사
    .aria-label = 클립보드에 복사
wallets-export-reveal = 키 표시

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = 토큰
wallets-holdings-col-balance = 잔액
wallets-holdings-col-value = 가치 ({ -sol })
wallets-holdings-col-type = 유형
wallets-holdings-col-decimals = 소수 자릿수
wallets-holdings-empty-title = 보유 토큰 없음
wallets-holdings-unreadable-title = 보유 자산 알 수 없음
wallets-holdings-empty-message = 이 지갑이 보유한 토큰이 여기에 표시됩니다.
wallets-funding-title = 시작하려면 거래 지갑에 입금하세요
wallets-funding-message = 이 지갑에는 { -sol }가 없습니다. 거래소나 다른 지갑에서 이 주소로 { -sol }를 보내세요. 매수할 때마다 그 금액과 네트워크 수수료가 듭니다.
wallets-holdings-no-main = 메인 지갑 없음
wallets-holdings-main-title = 메인 지갑
wallets-holdings-tokens = 토큰
wallets-holdings-last-used = 마지막 사용
wallets-holdings-never = 없음
wallets-holdings-search =
    .placeholder = 심볼 또는 민트로 검색...
wallets-holdings-export = 키 내보내기
wallets-holdings-export-tooltip = 이 지갑의 개인 키를 내보냅니다
wallets-holdings-replace = 지갑 교체
wallets-holdings-replace-tooltip = 다른 지갑으로 거래합니다. 이 지갑의 키와 기록은 보관됩니다
wallets-list-col-name = 이름
wallets-list-col-balance = 잔액 ({ -sol })
wallets-list-col-type = 유형
wallets-list-col-created = 생성일
wallets-list-col-replaced = 교체일
wallets-list-replaced-on = { $date }
wallets-replace-abandoned-title = 지갑 교체가 취소되었습니다
wallets-replace-abandoned-open-positions = 재시작 시 트레이딩 지갑에 열린 포지션이 있어 ({ $count }) 교체가 취소되었습니다. 트레이딩 지갑은 바뀌지 않았으며, 새 키는 기타 지갑에 보관되어 있습니다.
wallets-replace-abandoned-positions-unreadable = 재시작 시 포지션 저장소를 읽을 수 없어 교체가 취소되었습니다. 트레이딩 지갑은 바뀌지 않았으며, 새 키는 기타 지갑에 보관되어 있습니다.
wallets-replace-abandoned-vault-changed = 재시작 시 지갑 보관소가 준비된 교체와 더 이상 일치하지 않아 교체가 취소되었습니다. 트레이딩 지갑은 바뀌지 않았으며, 새 키는 기타 지갑에 보관되어 있습니다.
wallets-list-col-actions = 작업
wallets-list-action-export = 개인 키 내보내기
wallets-list-count = 지갑
wallets-list-search =
    .placeholder = 이름 또는 주소로 검색...
wallets-list-loading-title = 지갑 불러오는 중…
wallets-list-loading-description = 선택한 지갑 보기를 준비하고 있습니다.
wallets-list-kept-empty-message = 교체된 트레이딩 지갑과 이전 버전의 지갑이 여기에 표시되어 계속 키를 내보낼 수 있습니다.
wallets-list-previous-trading-wallet = 이전 트레이딩 지갑
wallets-other-empty-title = 다른 지갑이 없습니다

# watched.js: watched wallets table and actions.
wallets-watched-col-wallet = 지갑
wallets-watched-col-status = 상태
wallets-watched-col-progress = 저장된 진행 상태
wallets-watched-col-last-check = 마지막 확인
wallets-watched-unlabelled = 라벨 없는 지갑
wallets-watched-generic-name = 지갑
wallets-watched-not-synced = 아직 동기화되지 않음
wallets-watched-not-checked = 아직 확인되지 않음
wallets-watched-action-copy = 카피 거래
    .title = 이 지갑을 카피 트레이딩에서 열기
wallets-watched-action-restore = 감시 복원
wallets-watched-action-options = 감시 옵션
wallets-watched-action-retry = 감시 재시도
wallets-watched-action-pause = 일시 중지
wallets-watched-action-enable = 활성화
wallets-watched-action-remove =
    .title = 제거
    .aria-label = 제거: { $name }
wallets-watch-state-paused = 일시 중지됨
wallets-watch-state-catching-up = 따라잡는 중
wallets-watch-state-watching = 감시 중
wallets-watch-state-streaming = 스트리밍
wallets-watch-state-polling = 폴링
wallets-watched-detail-helius = 이 지갑은 { -helius }를 통해 확인하고 있습니다.
wallets-watched-empty-title = 감시 중인 주소 없음
wallets-watched-empty-message = 지갑 감시를 사용하여 공개 지갑의 온체인 활동을 기록하세요.
wallets-watched-count = 감시 중
wallets-watched-search =
    .placeholder = 감시 지갑 검색...
wallets-watched-add = 지갑 감시
wallets-watched-refresh = 감시 지갑 새로 고침
wallets-watched-loading-title = 감시 지갑 불러오는 중...
wallets-watched-loading-description = 관찰 대상을 가져오는 중입니다.
wallets-watched-load-error-title = 감시 주소를 불러오지 못했습니다
wallets-watched-load-error-description = 새로 고침하여 다시 시도하세요.
wallets-watched-address-invalid = 올바른 Solana 지갑 주소를 입력하세요.
wallets-watched-added = 지갑 감시가 추가되었습니다
wallets-watched-duplicate = 이미 감시 중인 지갑입니다.
wallets-watched-add-failed = 지갑 감시를 추가하지 못했습니다.
wallets-watched-retried = 저장된 커서로 지갑 감시를 복원했습니다
wallets-watched-paused = 지갑 감시가 일시 중지되었습니다
wallets-watched-enabled = 지갑 감시가 활성화되었습니다
wallets-watched-removed = 지갑 감시가 제거되었습니다
wallets-watched-update-failed = 지갑 감시를 업데이트하지 못했습니다
wallets-setup-gate-title = 지갑을 사용하려면 설정이 필요합니다
