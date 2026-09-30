# Results of system and configuration operations. Ids come from
# src/webserver/routes/config/operations.rs (diff) and
# src/webserver/routes/config/import_export.rs (import).

system-result-config-differs = メモリ上の設定がディスク上のバージョンと異なります
system-result-config-matches = メモリ上の設定がディスク上のバージョンと一致しています

# $count is the number of imported sections, $warnings the number of problems and
# $details their text joined with commas.
system-result-config-imported =
    { $count ->
       *[other] { $count }件のセクションをインポートしました
    }
system-result-config-imported-with-warnings =
    { $count ->
       *[other] { $count }件のセクションを
    }警告付きでインポートしました（{ $warnings ->
       *[other] 警告 { $warnings }件
    }）: { $details }

# The Config page (pages/config.js, config.html and pages/config/*) and the
# import/export dialog (ui/config_import_export_dialog.js). Field labels, hints,
# units and section names come from config.ftl; only the page's own text is here.

## Config page: sidebar and toolbar

system-config-search =
    .placeholder = 設定を検索...
system-config-export-title =
    .title = 設定をファイルにエクスポート
system-config-import-title =
    .title = ファイルから設定をインポート
system-config-reload = ディスクから再読み込み
system-config-reset-defaults = デフォルトに戻す
system-config-select-section = 設定セクションを選択してください
system-config-select-section-details = 設定セクションを選択すると詳細を表示します。
system-config-no-metadata = <code>{ $section }</code> のメタデータはありません
system-config-technical-settings = 技術設定
system-config-expand-title = すべてのセクションとネストされたサブ設定を展開
system-config-collapse-title = すべてのセクションとネストされたサブ設定を折りたたむ
system-config-toolbar-no-changes = セクションの変更なし
system-config-toolbar-section-changes =
    { $count ->
       *[other] セクション内の変更 <strong>{ $count }</strong>件
    }
system-config-toolbar-total-changes =
    { $count ->
       *[other] 変更合計 <strong>{ $count }</strong>件
    }

## Config page: state banner

system-config-loading = 設定を読み込み中…
system-config-refreshing = 設定を更新中…
system-config-saving-title = 変更を保存中…
system-config-saving-detail = 設定を更新しています
system-config-validation-issues = <strong>検証の問題が検出されました。</strong>ハイライトされたフィールドを確認してください。

## Config page: section header and category chips

system-config-save-changes = 変更を保存
system-config-saving = 保存中…
system-config-compare = ディスクと比較
system-config-revert-section = セクションを元に戻す
system-config-summary-critical = 重要 { $count }件
system-config-summary-performance = パフォーマンス { $count }件
system-config-summary-pending =
    { $count ->
       *[other] 保留中の変更 { $count }件
    }
system-config-summary-none = メタデータの概要なし
system-config-fields-count =
    { $count ->
       *[other] フィールド { $count }件
    }
# $fields is the field count above; $pending and $visible are counts.
system-config-chip-pending = { $fields } · 保留 { $pending }件
system-config-chip-visible = { $fields }中 { $visible }件

## Config page: field rows

system-config-field-unit = 単位: { $unit }
system-config-field-default = デフォルト: { $value }
system-config-field-reset = デフォルトに戻す
system-config-array-invalid-title = 配列の項目が無効です
system-config-json-invalid-title = JSON が無効です
system-config-list-separator = { "、" }
# Ids of the array-entry messages come from FieldType in src/config/metadata.rs.
# $lines is the list of offending line numbers.
system-config-array-invalid-integer =
    { $count ->
       *[other] { $lines } 行目は有効な整数である必要があります。
    }
system-config-array-invalid-number =
    { $count ->
       *[other] { $lines } 行目は有効な数値である必要があります。
    }
system-config-array-invalid-boolean =
    { $count ->
       *[other] { $lines } 行目は有効な真偽値である必要があります。
    }
system-config-array-invalid-value =
    { $count ->
       *[other] { $lines } 行目は有効な値である必要があります。
    }

## Config page: Telegram actions

system-config-telegram-actions = 操作
system-config-telegram-test-title = 接続テスト
system-config-telegram-test-description = テストメッセージを送信して、{ -telegram } の設定が機能しているか確認します
system-config-telegram-send-test = テストメッセージを送信
system-config-telegram-sending = 送信中...
system-config-telegram-configure-token-title = 先にボットトークンを設定してください
system-config-telegram-configure-token-status = テストを有効にするには、上でボットトークンを設定してください
system-config-telegram-test-sent-status = テストメッセージを送信しました。{ -telegram } を確認してください。
system-config-telegram-test-sent = { -telegram } のテストメッセージを送信しました
system-config-telegram-test-failed = テストメッセージを送信できませんでした
system-config-telegram-auth-title = ボット認証
system-config-telegram-totp-title = 二要素認証（TOTP）
system-config-telegram-totp-configured = 設定済み
system-config-telegram-totp-not-configured = 未設定
system-config-telegram-totp-active = 二要素認証は有効です。期限切れの { -telegram } セッションでは、認証アプリの TOTP コードが必要です。
system-config-telegram-totp-inactive = { -telegram } コマンドを保護するには、セキュリティ設定で二要素認証を有効にしてください。
system-config-telegram-totp-note = TOTP はダッシュボードのロック画面と共通です。セキュリティ設定で構成してください。
system-config-telegram-require-2fa = コマンドに 2FA を必須にする
# $status is the HTTP status code.
system-config-telegram-save-rejected = 保存が拒否されました（{ $status }）
system-config-telegram-save-failed = { -telegram } の設定を保存できませんでした

## Config page: operations

system-config-saved = 設定を保存しました
system-config-save-failed = 設定を保存できませんでした
system-config-reloaded = ディスクから設定を再読み込みしました
system-config-reload-failed = 設定を再読み込みできませんでした
system-config-diff-title = 設定の差分
system-config-diff-console = ブラウザーコンソールに出力しました
system-config-diff-failed = 差分を計算できませんでした
system-config-reset-title = 設定をリセット
system-config-reset-message =
    設定全体を組み込みのデフォルト値にリセットします。現在の設定はすべて失われます。

    この操作は元に戻せません。
system-config-reset-done-title = 設定をリセットしました
system-config-reset-done-message = すべての設定をデフォルト値に戻しました
system-config-reset-failed = 設定をリセットできませんでした
system-config-load-failed = 設定を読み込めませんでした
system-config-metadata-failed = 設定メタデータを読み込めませんでした

## Import and export dialogs: shared

system-config-dialog-close =
    .aria-label = 閉じる
system-config-select-none = すべて選択解除
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
       *[other] 変更 { $count }件
    }
system-config-sections-count =
    { $count ->
       *[other] セクション { $count }件
    }

## Import and export dialogs: section descriptions. Ids are the section names of
## src/webserver/routes/config/import_export.rs.

system-config-section-hint-rpc = RPC エンドポイントと接続設定
system-config-section-hint-trader = 取引ルールと自動化
system-config-section-hint-positions = ポジション管理の設定
system-config-section-hint-filtering = トークンのフィルタリングルールとしきい値
system-config-section-hint-swaps = スワップ実行の設定
system-config-section-hint-tokens = トークン探索とデータソース
system-config-section-hint-sol-price = { -sol } 価格サービスの設定
system-config-section-hint-events = イベント記録の設定
system-config-section-hint-services = バックグラウンドサービスの設定
system-config-section-hint-monitoring = システム監視の設定
system-config-section-hint-ohlcv = ローソク足データの設定
system-config-section-hint-gui = ダッシュボードと UI の設定
system-config-section-hint-telegram = { -telegram } ボットの設定

## Export dialog

system-config-export-dialog-title = 設定をエクスポート
system-config-export-intro = エクスポートする設定セクションを選択してください。エクスポートしたファイルは、後でインポートして設定を復元したり共有したりできます。
system-config-export-sections = セクション
system-config-export-timestamp = エクスポート日時を含める
system-config-sections-selected =
    { $count ->
       *[other] { $count }件のセクションを選択中
    }
system-config-exporting = エクスポート中...
system-config-export-invalid-response = サーバーからの応答が無効です
system-config-exported-title = 設定をエクスポートしました
system-config-exported-message =
    { $count ->
       *[other] { $count }件のセクションをエクスポートしました
    }
system-config-export-failed-title = エクスポート失敗
system-config-export-failed = 設定をエクスポートできませんでした

## Import dialog

system-config-import-dialog-title = 設定をインポート
system-config-import-upload-intro = 以前にエクスポートした設定ファイルをアップロードしてください。インポートするセクションをプレビューして選択できます。
system-config-import-dropzone-title = 設定ファイルをここにドロップ
system-config-import-dropzone-hint = またはクリックして参照
system-config-import-analyzing = 設定を分析中...
system-config-import-preview = プレビュー
system-config-import-preview-intro = 以下の設定セクションを確認し、インポートするセクションを選択してください。
system-config-import-sections = ファイル内のセクション
system-config-import-select-valid = 有効なものをすべて選択
system-config-import-merge-label = 既存の設定とマージ
system-config-import-merge-hint = ファイルに含まれるフィールドのみ更新します。オフの場合はセクション全体を置き換えます。
system-config-import-save-label = ディスクに保存
system-config-import-save-hint = インポート後に変更を config.toml に保存します
system-config-import-selected = 選択したものをインポート
system-config-import-warnings =
    { $count ->
       *[other] 警告 { $count }件
    }
# $section is a section name from the file, $field a dotted setting path, $detail the
# technical reason a section failed to parse.
system-config-import-warning-unknown-section = 不明なセクション「{ $section }」は無視されます
system-config-import-warning-sensitive-field = { $field } をインポートすると認証設定が上書きされる可能性があります
system-config-import-section-error = { $detail }
# $sections and $changes are the counts above, already worded.
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = ファイルにありません
system-config-import-status-invalid = 設定が無効です
system-config-import-status-unchanged = 変更なし
system-config-import-not-included = ファイルに含まれていません
system-config-import-show-changes = 変更を表示
system-config-import-hide-changes = 変更を隠す
system-config-import-value-current = 現在の値
system-config-import-value-new = 新しい値
system-config-import-more-changes =
    { $count ->
       *[other] ほか { $count }件の変更
    }
system-config-import-value-items =
    { "[" }{ $count ->
       *[other] { $count }項目
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
       *[other] { $count }キー
    }{ "}" }
system-config-importing = インポート中...
system-config-import-failed = インポートに失敗しました
system-config-import-invalid-file-title = 無効なファイル
system-config-import-invalid-file = 設定ファイルを解析できませんでした
system-config-imported-title = 設定をインポートしました
system-config-imported-message =
    { $count ->
       *[other] { $count }件のセクションをインポートしました
    }
system-config-import-failed-title = インポート失敗
system-config-import-failed-message = 設定をインポートできませんでした
