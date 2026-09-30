# Results of system and configuration operations. Ids come from
# src/webserver/routes/config/operations.rs (diff) and
# src/webserver/routes/config/import_export.rs (import).

system-result-config-differs = 메모리의 설정이 디스크 버전과 다릅니다
system-result-config-matches = 메모리의 설정이 디스크 버전과 일치합니다

# $count is the number of imported sections, $warnings the number of problems and
# $details their text joined with commas.
system-result-config-imported =
    { $count ->
       *[other] 섹션 { $count }개를 가져왔습니다
    }
system-result-config-imported-with-warnings =
    { $count ->
       *[other] 섹션 { $count }개를 가져왔습니다
    } ({ $warnings ->
       *[other] 경고 { $warnings }건
    }): { $details }

# The Config page (pages/config.js, config.html and pages/config/*) and the
# import/export dialog (ui/config_import_export_dialog.js). Field labels, hints,
# units and section names come from config.ftl; only the page's own text is here.

## Config page: sidebar and toolbar

system-config-search =
    .placeholder = 설정 검색...
system-config-export-title =
    .title = 설정을 파일로 내보내기
system-config-import-title =
    .title = 파일에서 설정 가져오기
system-config-reload = 디스크에서 다시 불러오기
system-config-reset-defaults = 기본값으로 초기화
system-config-select-section = 설정 섹션을 선택하세요
system-config-select-section-details = 설정 섹션을 선택하면 상세 정보를 볼 수 있습니다.
system-config-no-metadata = <code>{ $section }</code>의 메타데이터가 없습니다
system-config-technical-settings = 기술 설정
system-config-expand-title = 모든 섹션과 중첩된 하위 설정 펼치기
system-config-collapse-title = 모든 섹션과 중첩된 하위 설정 접기
system-config-toolbar-no-changes = 섹션 변경 사항 없음
system-config-toolbar-section-changes =
    { $count ->
       *[other] 섹션 변경 <strong>{ $count }</strong>건
    }
system-config-toolbar-total-changes =
    { $count ->
       *[other] 전체 변경 <strong>{ $count }</strong>건
    }

## Config page: state banner

system-config-loading = 설정 불러오는 중…
system-config-refreshing = 설정 새로 고치는 중…
system-config-saving-title = 변경 사항 저장 중…
system-config-saving-detail = 설정 업데이트 중
system-config-validation-issues = <strong>검증 문제가 발견되었습니다.</strong> 강조 표시된 필드를 확인하세요.

## Config page: section header and category chips

system-config-save-changes = 변경 사항 저장
system-config-saving = 저장 중…
system-config-compare = 디스크와 비교
system-config-revert-section = 섹션 되돌리기
system-config-summary-critical = 중요 { $count }개
system-config-summary-performance = 성능 { $count }개
system-config-summary-pending =
    { $count ->
       *[other] 대기 중인 변경 { $count }건
    }
system-config-summary-none = 메타데이터 요약 없음
system-config-fields-count =
    { $count ->
       *[other] 필드 { $count }개
    }
# $fields is the field count above; $pending and $visible are counts.
system-config-chip-pending = { $fields } · 대기 { $pending }건
system-config-chip-visible = { $fields } 중 { $visible }

## Config page: field rows

system-config-field-unit = 단위: { $unit }
system-config-field-default = 기본값: { $value }
system-config-field-reset = 기본값으로 초기화
system-config-array-invalid-title = 잘못된 배열 항목
system-config-json-invalid-title = 잘못된 JSON
system-config-list-separator = { ", " }
# Ids of the array-entry messages come from FieldType in src/config/metadata.rs.
# $lines is the list of offending line numbers.
system-config-array-invalid-integer =
    { $count ->
       *[other] { $lines }번 줄은 올바른 정수여야 합니다.
    }
system-config-array-invalid-number =
    { $count ->
       *[other] { $lines }번 줄은 올바른 숫자여야 합니다.
    }
system-config-array-invalid-boolean =
    { $count ->
       *[other] { $lines }번 줄은 올바른 불리언이어야 합니다.
    }
system-config-array-invalid-value =
    { $count ->
       *[other] { $lines }번 줄은 올바른 값이어야 합니다.
    }

## Config page: Telegram actions

system-config-telegram-actions = 작업
system-config-telegram-test-title = 연결 테스트
system-config-telegram-test-description = 테스트 메시지를 보내 { -telegram } 설정이 정상 작동하는지 확인합니다
system-config-telegram-send-test = 테스트 메시지 보내기
system-config-telegram-sending = 전송 중...
system-config-telegram-configure-token-title = 먼저 봇 토큰을 설정하세요
system-config-telegram-configure-token-status = 테스트를 사용하려면 위에서 봇 토큰을 설정하세요
system-config-telegram-test-sent-status = 테스트 메시지를 전송했습니다. { -telegram }을 확인하세요.
system-config-telegram-test-sent = { -telegram } 테스트 메시지를 전송했습니다
system-config-telegram-test-failed = 테스트 메시지를 전송하지 못했습니다
system-config-telegram-auth-title = 봇 인증
system-config-telegram-totp-title = 이중 인증 (TOTP)
system-config-telegram-totp-configured = 설정됨
system-config-telegram-totp-not-configured = 설정되지 않음
system-config-telegram-totp-active = 이중 인증이 활성화되어 있습니다. 만료된 { -telegram } 세션은 인증 앱의 TOTP 코드가 필요합니다.
system-config-telegram-totp-inactive = 보안 설정에서 이중 인증을 활성화하여 { -telegram } 명령을 보호하세요.
system-config-telegram-totp-note = TOTP는 대시보드 잠금 화면과 공유됩니다. 보안 설정에서 설정하세요.
system-config-telegram-require-2fa = 명령에 2FA 요구
# $status is the HTTP status code.
system-config-telegram-save-rejected = 저장이 거부되었습니다 ({ $status })
system-config-telegram-save-failed = { -telegram } 설정을 저장하지 못했습니다

## Config page: operations

system-config-saved = 설정이 저장되었습니다
system-config-save-failed = 설정을 저장하지 못했습니다
system-config-reloaded = 디스크에서 설정을 다시 불러왔습니다
system-config-reload-failed = 설정을 다시 불러오지 못했습니다
system-config-diff-title = 설정 차이
system-config-diff-console = 브라우저 콘솔에 출력했습니다
system-config-diff-failed = 차이를 계산하지 못했습니다
system-config-reset-title = 설정 초기화
system-config-reset-message =
    전체 설정을 내장 기본값으로 초기화합니다. 현재 설정이 모두 사라집니다.

    이 작업은 되돌릴 수 없습니다.
system-config-reset-done-title = 설정 초기화됨
system-config-reset-done-message = 모든 설정이 기본값으로 복원되었습니다
system-config-reset-failed = 설정을 초기화하지 못했습니다
system-config-load-failed = 설정을 불러오지 못했습니다
system-config-metadata-failed = 설정 메타데이터를 불러오지 못했습니다

## Import and export dialogs: shared

system-config-dialog-close =
    .aria-label = 닫기
system-config-select-none = 모두 선택 해제
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
       *[other] 변경 { $count }건
    }
system-config-sections-count =
    { $count ->
       *[other] 섹션 { $count }개
    }

## Import and export dialogs: section descriptions. Ids are the section names of
## src/webserver/routes/config/import_export.rs.

system-config-section-hint-rpc = RPC 엔드포인트 및 연결 설정
system-config-section-hint-trader = 트레이딩 규칙 및 자동화
system-config-section-hint-positions = 포지션 관리 설정
system-config-section-hint-filtering = 토큰 필터링 규칙 및 임계값
system-config-section-hint-swaps = 스왑 실행 설정
system-config-section-hint-tokens = 토큰 탐색 및 데이터 소스
system-config-section-hint-sol-price = { -sol } 가격 서비스 설정
system-config-section-hint-events = 이벤트 기록 설정
system-config-section-hint-services = 백그라운드 서비스 설정
system-config-section-hint-monitoring = 시스템 모니터링 설정
system-config-section-hint-ohlcv = 캔들 데이터 설정
system-config-section-hint-gui = 대시보드 및 UI 설정
system-config-section-hint-telegram = { -telegram } 봇 설정

## Export dialog

system-config-export-dialog-title = 설정 내보내기
system-config-export-intro = 내보낼 설정 섹션을 선택하세요. 내보낸 파일은 나중에 가져와 설정을 복원하거나 공유할 수 있습니다.
system-config-export-sections = 섹션
system-config-export-timestamp = 내보낸 시각 포함
system-config-sections-selected =
    { $count ->
       *[other] 섹션 { $count }개 선택됨
    }
system-config-exporting = 내보내는 중...
system-config-export-invalid-response = 서버 응답이 올바르지 않습니다
system-config-exported-title = 설정 내보내기 완료
system-config-exported-message =
    { $count ->
       *[other] 섹션 { $count }개를 내보냈습니다
    }
system-config-export-failed-title = 내보내기 실패
system-config-export-failed = 설정을 내보내지 못했습니다

## Import dialog

system-config-import-dialog-title = 설정 가져오기
system-config-import-upload-intro = 이전에 내보낸 설정 파일을 업로드하세요. 가져올 섹션을 미리 보고 선택할 수 있습니다.
system-config-import-dropzone-title = 설정 파일을 여기에 놓으세요
system-config-import-dropzone-hint = 또는 클릭하여 찾아보기
system-config-import-analyzing = 설정 분석 중...
system-config-import-preview = 미리보기
system-config-import-preview-intro = 아래 설정 섹션을 검토하고 가져올 섹션을 선택하세요.
system-config-import-sections = 파일 내 섹션
system-config-import-select-valid = 유효한 항목 모두 선택
system-config-import-merge-label = 기존 설정과 병합
system-config-import-merge-hint = 파일에 있는 필드만 업데이트합니다. 선택하지 않으면 섹션 전체를 교체합니다.
system-config-import-save-label = 디스크에 저장
system-config-import-save-hint = 가져온 후 변경 사항을 config.toml에 저장합니다
system-config-import-selected = 선택 항목 가져오기
system-config-import-warnings =
    { $count ->
       *[other] 경고 { $count }건
    }
# $section is a section name from the file, $field a dotted setting path, $detail the
# technical reason a section failed to parse.
system-config-import-warning-unknown-section = 알 수 없는 섹션은 무시됩니다: "{ $section }"
system-config-import-warning-sensitive-field = { $field } 가져오기는 인증 설정을 덮어쓸 수 있습니다
system-config-import-section-error = { $detail }
# $sections and $changes are the counts above, already worded.
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = 파일에 없음
system-config-import-status-invalid = 잘못된 설정
system-config-import-status-unchanged = 변경 사항 없음
system-config-import-not-included = 파일에 포함되지 않음
system-config-import-show-changes = 변경 사항 표시
system-config-import-hide-changes = 변경 사항 숨기기
system-config-import-value-current = 현재 값
system-config-import-value-new = 새 값
system-config-import-more-changes =
    { $count ->
       *[other] 변경 +{ $count }건 더 있음
    }
system-config-import-value-items =
    { "[" }{ $count ->
       *[other] 항목 { $count }개
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
       *[other] 키 { $count }개
    }{ "}" }
system-config-importing = 가져오는 중...
system-config-import-failed = 가져오기 실패
system-config-import-invalid-file-title = 잘못된 파일
system-config-import-invalid-file = 설정 파일을 해석하지 못했습니다
system-config-imported-title = 설정 가져오기 완료
system-config-imported-message =
    { $count ->
       *[other] 섹션 { $count }개를 가져왔습니다
    }
system-config-import-failed-title = 가져오기 실패
system-config-import-failed-message = 설정을 가져오지 못했습니다
