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
wallets-field-address = 주소
wallets-field-name = 지갑 이름
wallets-field-notes = 메모
wallets-field-private-key = 개인 키
wallets-address-copy = 주소 복사
wallets-modal-close =
    .aria-label = 창 닫기
wallets-this-wallet = 이 지갑
wallets-summary-native = { -sol }
wallets-copied-address = 주소
wallets-copied-mint = 민트 주소
wallets-copied-private-key = 개인 키

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = 메인 지갑
wallets-tab-secondaries = 보조 지갑
wallets-tab-archive = 보관함
wallets-tab-watched = 감시 중
wallets-refresh-failed = 지갑을 새로 고치지 못했습니다
wallets-action-failed = 실패
wallets-toast-failed = 실패: { $reason }
wallets-create-busy = 생성 중...
wallets-create-fallback = 생성에 실패했습니다
wallets-create-done = 지갑 "{ $name }"이 생성되었습니다.
wallets-import-busy = 가져오는 중...
wallets-import-failed = 가져오기에 실패했습니다
wallets-import-done = 지갑 "{ $name }"을 가져왔습니다.
wallets-archive-busy = 보관 중...
wallets-archive-confirm-text = 지갑 <strong>{ $name }</strong>을 보관하시겠습니까?
wallets-archive-done = 지갑이 보관되었습니다
wallets-restore-done = 지갑이 복원되었습니다
wallets-export-busy = 복호화 중...
wallets-export-revealed = 키가 표시되었습니다. 안전하게 다루세요
wallets-delete-busy = 삭제 중...
wallets-delete-confirm-text = 지갑 <strong>{ $name }</strong>을 삭제하시겠습니까?
wallets-delete-done = 지갑이 영구 삭제되었습니다

# wallets.html: Add Wallet dialog.
wallets-add-title = 지갑 추가
wallets-add-tab-create = 새로 만들기
wallets-add-tab-import = 기존 지갑 가져오기
wallets-create-name-input =
    .placeholder = 예: 트레이딩 지갑
wallets-create-name-hint = 이 지갑을 구분할 수 있는 이름
wallets-create-notes-input =
    .placeholder = 설명 또는 용도 (선택)...
wallets-create-submit = 지갑 생성
wallets-import-warning-title = 보안 경고
wallets-import-warning-body = 신뢰할 수 있는 출처의 개인 키만 가져오세요. 키는 암호화되어 이 기기에 안전하게 저장됩니다.
wallets-import-name-input =
    .placeholder = 예: 내 지갑
wallets-import-key-input =
    .placeholder = Base58 문자열 또는 JSON 배열 [1,2,3,...]
wallets-import-key-toggle =
    .aria-label = 개인 키 표시 전환
wallets-import-key-hint = base58 인코딩 키 또는 바이트 배열 형식을 지원합니다
wallets-import-notes-input =
    .placeholder = 설명 (선택)...
wallets-import-submit = 지갑 가져오기

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

# wallets.html: Archive and Delete dialogs.
wallets-archive-title = 지갑 보관
wallets-archive-note = 보관된 지갑은 어떤 작업에도 사용되지 않지만 언제든 복원할 수 있습니다.
wallets-archive-confirm = 예, 보관
wallets-delete-title = 지갑 삭제
wallets-delete-warning-title = 이 작업은 되돌릴 수 없습니다.
wallets-delete-warning-body = 이 지갑을 삭제하면 지갑과 암호화된 개인 키가 이 기기에서 영구적으로 제거됩니다.
wallets-delete-confirm = 예, 삭제

# wallets.html and bulk_operations.js: bulk import.
wallets-bulk-import-title = 지갑 가져오기
wallets-bulk-import-submit = 지갑 가져오기
wallets-bulk-step-upload = 파일 업로드
wallets-bulk-step-map = 열 매핑
wallets-bulk-step-results = 결과
wallets-bulk-import-file-warning-body = 신뢰할 수 있는 출처의 파일만 가져오세요. 개인 키는 암호화되어 이 기기에 안전하게 저장됩니다.
wallets-bulk-drop-title = 파일을 여기에 놓으세요
wallets-bulk-drop-subtitle = 또는 클릭하여 찾아보기
wallets-bulk-drop-formats = CSV 및 Excel (.xlsx, .xls) 지원
wallets-bulk-file-remove =
    .aria-label = 파일 제거
wallets-bulk-map-subtitle = 파일의 열을 지갑 필드와 연결하세요
wallets-bulk-preview-title = 미리보기 (처음 5행)
wallets-bulk-summary-valid = 유효 <strong>{ $count }</strong>건
wallets-bulk-summary-invalid = 무효 <strong>{ $count }</strong>건
wallets-bulk-summary-duplicate =
    { $count ->
       *[other] 중복 <strong>{ $count }</strong>건
    }
wallets-bulk-done = 완료
wallets-bulk-file-invalid = 잘못된 파일 형식입니다. CSV 또는 Excel 파일을 사용하세요.
wallets-bulk-preview-busy = 처리 중...
wallets-bulk-preview-fallback = 파일을 처리하지 못했습니다
wallets-bulk-preview-failed = 파일을 처리하지 못했습니다: { $reason }
wallets-bulk-column-select = -- 열 선택 --
wallets-bulk-preview-empty = 파일에서 데이터 행을 찾을 수 없습니다
wallets-bulk-preview-status = 상태
wallets-bulk-status-valid = 유효
wallets-bulk-status-duplicate = 중복
wallets-bulk-status-invalid = 무효
wallets-bulk-import-busy = 가져오는 중...
wallets-bulk-import-toast =
    { $count ->
       *[other] 지갑 { $count }개를 가져왔습니다
    }
wallets-bulk-import-error = 가져오기 실패: { $reason }
wallets-bulk-result-success-title = 가져오기 성공
wallets-bulk-result-success-detail =
    { $count ->
       *[other] 지갑 { $count }개를 모두 가져왔습니다
    }
wallets-bulk-result-partial-title = 부분 성공
wallets-bulk-result-partial-detail = 성공 { $imported }건, 실패 { $failed }건
wallets-bulk-result-failed-title = 가져오기 실패
wallets-bulk-result-failed-detail =
    { $count ->
       *[other] 지갑 { $count }개를 모두 가져오지 못했습니다
    }
wallets-bulk-result-imported = 가져옴
wallets-bulk-result-failed = 실패

# wallets.html and bulk_operations.js: bulk export.
wallets-bulk-export-title = 지갑 내보내기
wallets-bulk-export-format = 형식
wallets-bulk-export-format-csv = CSV (.csv)
wallets-bulk-export-format-xlsx = Excel (.xlsx)
wallets-bulk-export-include-archived = 보관된 지갑 포함
wallets-bulk-export-safe-title = 안전 내보내기
wallets-bulk-export-safe-body = 지갑 주소와 메타데이터만 내보냅니다. 개인 키는 포함되지 않습니다.
wallets-bulk-export-safe-submit = 주소 내보내기
wallets-bulk-export-or = 또는
wallets-bulk-export-danger-title = 위험한 내보내기
wallets-bulk-export-danger-body = 내보내기에 개인 키를 포함합니다. 이 파일을 가진 사람은 자금을 훔칠 수 있습니다.
wallets-bulk-export-danger-submit = 개인 키 포함 내보내기
wallets-bulk-export-busy = 내보내는 중...
wallets-bulk-export-done = 지갑을 내보냈습니다: { $filename }
wallets-bulk-export-fallback = 내보내기에 실패했습니다
wallets-bulk-export-error = 내보내기 실패: { $reason }
wallets-bulk-confirm-title = 위험한 내보내기 확인
wallets-bulk-confirm-warning =
    { $count ->
       *[other] 개인 키 <strong>{ $count }</strong>개를 내보내려 합니다. 매우 위험합니다.
    }
wallets-bulk-confirm-risk-steal = 이 파일을 가진 사람은 모든 자금을 훔칠 수 있습니다
wallets-bulk-confirm-risk-share = 이 파일을 절대 다른 사람과 공유하지 마세요
wallets-bulk-confirm-risk-delete = 사용 후 즉시 파일을 삭제하세요
wallets-bulk-confirm-prompt = 확인하려면 아래 문구를 입력하세요
wallets-bulk-confirm-submit = 키 내보내기

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = 토큰
wallets-holdings-col-balance = 잔액
wallets-holdings-col-value = 가치 ({ -sol })
wallets-holdings-col-type = 유형
wallets-holdings-col-decimals = 소수 자릿수
wallets-holdings-col-mint = 민트
wallets-holdings-empty-title = 보유 토큰 없음
wallets-holdings-empty-message = 이 지갑이 보유한 토큰이 여기에 표시됩니다.
wallets-holdings-no-main = 메인 지갑 없음
wallets-holdings-main-tag = 메인
wallets-holdings-main-title = 메인 지갑
wallets-holdings-tokens = 토큰
wallets-holdings-last-used = 마지막 사용
wallets-holdings-never = 없음
wallets-holdings-search =
    .placeholder = 심볼 또는 민트로 검색...
wallets-holdings-export = 키 내보내기
wallets-holdings-export-tooltip = 이 지갑의 개인 키를 내보냅니다
wallets-list-col-name = 이름
wallets-list-col-balance = 잔액 ({ -sol })
wallets-list-col-type = 유형
wallets-list-col-created = 생성일
wallets-list-col-actions = 작업
wallets-list-action-export = 개인 키 내보내기
wallets-list-action-archive = 지갑 보관
wallets-list-action-restore = 지갑 복원
wallets-list-action-delete = 영구 삭제
wallets-list-count = 지갑
wallets-list-search =
    .placeholder = 이름 또는 주소로 검색...
wallets-list-loading-title = 지갑 불러오는 중…
wallets-list-loading-description = 선택한 지갑 보기를 준비하고 있습니다.
wallets-secondaries-empty-title = 보조 지갑 없음
wallets-secondaries-empty-message = 지갑을 추가로 만들어 여러 계정에 걸쳐 트레이딩 활동을 관리하세요.
wallets-secondaries-add = 지갑 추가
wallets-archive-empty-title = 보관된 지갑 없음
wallets-archive-empty-message = 보관한 지갑이 나중에 참고할 수 있도록 여기에 안전하게 저장됩니다.

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
