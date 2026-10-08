# Wallet page labels.

# Wallet types. Ids come from WalletType in src/wallets/types.rs.
wallets-type-generated = 生成
wallets-type-imported = インポート
wallets-type-migrated = 移行

# Why a watched wallet is paused. Ids come from WatchDisableReason in
# src/wallets/watch/types.rs. $limit is a signature count.
wallets-watch-disabled-user = ユーザーが一時停止
wallets-watch-disabled-signature-budget = 一時停止: 追いつく前に { $limit }件のシグネチャのチェック上限に達しました
wallets-watch-disabled-unknown = 一時停止: 保存されたウォッチの安全上の理由を読み取れませんでした
wallets-watch-disabled-helius-unavailable = 一時停止: 高アクティビティ用プロバイダーを利用できません。カーソルは保持されています
wallets-watch-disabled-processing-failed = 一時停止: ウォレットのアクティビティを処理できませんでした。カーソルは保持されています

# Last runtime problem of a watch. Ids come from WatchRuntimeError in
# src/wallets/watch/types.rs.
wallets-watch-error-provider-unavailable = 高アクティビティ用プロバイダーを利用できません。ウォッチを一時停止しました
wallets-watch-error-provider-repeated-failure = { -helius } のチェックが繰り返し失敗しました。ウォッチを一時停止しました
wallets-watch-error-processing-repeated-failure = ウォレットのアクティビティ処理が繰り返し失敗しました。ウォッチを一時停止しました
wallets-watch-error-position-unreadable = ウォレットウォッチが保存済みの位置を読み取れませんでした。再試行します
wallets-watch-error-provider-check-failed = 高アクティビティ用プロバイダーのチェックに失敗しました。再試行します
wallets-watch-error-decode-failed = 高アクティビティのトランザクションをデコードできませんでした。カーソルは保持されています
wallets-watch-error-processing-failed = ウォレットのアクティビティを処理できませんでした。再試行します
wallets-watch-error-position-save-failed = ウォレットウォッチが位置を保存できませんでした。再試行します

# Why a watched wallet is paused, as a second line under its status. Ids come from
# WatchDisableReason in src/wallets/watch/types.rs, named after the serialized kind.
# The `unknown` kind has no detail line.
wallets-watch-reason-user = ユーザーが一時停止しました。
wallets-watch-reason-signature-budget = このウォレットのアクティビティは、現在のウォッチでチェックできる量を超えています。
wallets-watch-reason-helius-unavailable = { -helius } のチェックに失敗しました。保存済みの進捗は保持されています。
wallets-watch-reason-processing-failed = ウォレットのアクティビティを処理できませんでした。保存済みの進捗は保持されています。

# Vocabulary shared by the wallet tables and dialogs.
wallets-field-name = ウォレット名
wallets-field-notes = メモ
wallets-field-private-key = 秘密鍵
wallets-modal-close =
    .aria-label = モーダルを閉じる
wallets-this-wallet = このウォレット
wallets-summary-native = { -sol }
wallets-copied-private-key = 秘密鍵

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = メインウォレット
wallets-tab-secondaries = セカンダリ
wallets-tab-archive = アーカイブ
wallets-tab-watched = ウォッチ中
wallets-refresh-failed = ウォレットを更新できませんでした
wallets-action-failed = 失敗
wallets-toast-failed = 失敗: { $reason }
wallets-create-busy = 作成中...
wallets-create-fallback = 作成に失敗しました
wallets-create-done = ウォレット「{ $name }」を作成しました
wallets-import-busy = インポート中...
wallets-import-failed = インポートに失敗しました
wallets-import-done = ウォレット「{ $name }」をインポートしました
wallets-archive-busy = アーカイブ中...
wallets-archive-confirm-text = <strong>{ $name }</strong> をアーカイブしますか？
wallets-archive-done = ウォレットをアーカイブしました
wallets-restore-done = ウォレットを復元しました
wallets-export-busy = 復号中...
wallets-export-revealed = 鍵を表示しました。取り扱いに注意してください
wallets-delete-busy = 削除中...
wallets-delete-confirm-text = <strong>{ $name }</strong> を削除しますか？
wallets-delete-done = ウォレットを完全に削除しました

# wallets.html: Add Wallet dialog.
wallets-add-title = ウォレットを追加
wallets-add-tab-create = 新規作成
wallets-add-tab-import = 既存をインポート
wallets-create-name-input =
    .placeholder = 例: トレード用ウォレット
wallets-create-name-hint = このウォレットを識別するための分かりやすい名前
wallets-create-notes-input =
    .placeholder = 説明や用途（任意）...
wallets-create-submit = ウォレットを作成
wallets-import-warning-title = セキュリティ警告
wallets-import-warning-body = 秘密鍵は信頼できる提供元のものだけをインポートしてください。鍵は暗号化され、このデバイス上に安全に保存されます。
wallets-import-name-input =
    .placeholder = 例: マイウォレット
wallets-import-key-input =
    .placeholder = Base58 文字列または JSON 配列 [1,2,3,...]
wallets-import-key-toggle =
    .aria-label = 秘密鍵の表示を切り替え
wallets-import-key-hint = Base58 エンコードの鍵またはバイト配列形式に対応しています
wallets-import-notes-input =
    .placeholder = 説明（任意）...
wallets-import-submit = ウォレットをインポート

# wallets.html: Watch Wallet dialog.
wallets-watch-add-title = ウォレットをウォッチ
wallets-watch-add-address = ウォレットアドレス
wallets-watch-add-address-input =
    .placeholder = Solana アドレス
wallets-watch-add-address-hint = ウォレットのオンチェーンアクティビティを記録し、{ -telegram } の設定に従って取引アラートを送信します。
wallets-watch-add-label = ラベル
wallets-watch-add-label-input =
    .placeholder = 名前（任意）
wallets-watch-add-submit = ウォッチを追加

# wallets.html and watched.js: watch options dialog.
wallets-watch-budget-title-options = ウォレットウォッチのオプション
wallets-watch-budget-title-restore = ウォレットウォッチを復元
wallets-watch-budget-close =
    .aria-label = 閉じる
wallets-watch-budget-label-signatures = 1回のチェックで確認するシグネチャ数
wallets-watch-budget-label-transactions = 1回のチェックで確認する成功済みフルトランザクション数
wallets-watch-budget-hint-signatures = 現在の上限: { $limit }。1回のチェックあたり 500～5,000 件のシグネチャを 100 件単位で選択してください。
wallets-watch-budget-hint-transactions = 現在の上限: { $limit }。1回のチェックあたり 500～5,000 件の成功済みトランザクションを 100 件単位で選択してください。
wallets-watch-budget-error-range = 1回のチェックあたり 500～5,000 件のレコードを 100 件単位で選択してください。
wallets-watch-budget-error-ack = 前回完了したチェック以降のシグネチャがスキップされることを確認してください。
wallets-watch-budget-save-failed = ウォッチの上限を保存できませんでした。
wallets-watch-budget-save = 上限を保存
wallets-watch-budget-resume = 現時点から再開
wallets-watch-budget-resume-notice = このウォレットは追いつく前にチェック上限に達しました。「現時点から再開」は最新のウォレットアクティビティから開始し、前回完了したチェック以降のアクティビティはコピーされません。
wallets-watch-budget-resume-tasks = コピータスクは、コピートレードで各タスクを再開するまで一時停止のままです。
wallets-watch-budget-resume-ack = 見逃したアクティビティはコピーされないことを理解しました。
wallets-watch-budget-resumed = 現在のウォレットヘッドからウォッチを再開しました
wallets-watch-budget-updated = ウォレットウォッチの上限を更新しました
wallets-watch-helius-allow = 必要に応じて { -helius } による追いつきを許可
wallets-watch-helius-try = { -helius } を使って追いつきを試行
wallets-watch-helius-stop = このウォレットの { -helius } による追いつきを停止
wallets-watch-helius-description-approved = このウォレットでは { -helius } による追いつきが許可されています。オフにすると標準のチェックに戻りますが、アクティビティの多いウォレットでは遅れる可能性があります。
wallets-watch-helius-description-available = { -helius } は、未チェック区間をスキップせずに、保存済みの位置から成功済みの Solana トランザクションをチェックできます。プロバイダーのクレジットを多く消費する場合があり、遅れる可能性も残ります。
wallets-watch-helius-description-unavailable = { -helius } による追いつきは利用できません。利用するには、有効な { -helius } RPC エンドポイントを設定してください。
wallets-watch-helius-description-unsupported = このウォッチに対応する追いつき用プロバイダーはありません。上限に達した場合は「現時点から再開」を利用できます。
wallets-watch-helius-allow-title = このウォレットで { -helius } による追いつきを許可
wallets-watch-helius-allow-message = { -helius } は、未チェック区間をスキップせずに、保存済みの位置から成功済みの Solana トランザクションをチェックできます。現在の料金は、返されたフルトランザクション 100 件あたり 10 クレジット（切り上げ）で、1回のリクエストにつき最低 10 クレジットです。1回のチェックで複数のリクエストが発生する場合があり、使用量とプロバイダーの料金は変動することがあります。コピータスクは、別途再開するまで一時停止のままです。
wallets-watch-helius-allow-confirm = このウォレットで許可
wallets-watch-helius-stop-message = このウォレットは標準のチェックに戻ります。アクティビティの多いウォレットはウォッチの上限に達し、再び一時停止する場合があります。他のウォレットと { -helius } RPC の設定は変更されません。
wallets-watch-helius-stop-confirm = このウォレットで停止
wallets-watch-helius-stop-keep = 許可を維持
wallets-watch-helius-restored = 保存済みの進捗からウォッチを復元しました。コピータスクは一時停止のままです
wallets-watch-helius-allowed = 必要に応じて、このウォレットで { -helius } による追いつきを許可しました
wallets-watch-helius-stopped = このウォレットの { -helius } による追いつきを停止しました
wallets-watch-helius-update-failed = ウォレットの追いつき設定を更新できませんでした

# wallets.html: Export Private Key dialog.
wallets-export-title = 秘密鍵をエクスポート
wallets-export-warning-title = 重大なセキュリティ警告
wallets-export-warning-body = 秘密鍵は絶対に誰にも共有しないでください。この鍵にアクセスできる人は、このウォレットの資金をすべて盗むことができます。
wallets-export-key-label = 秘密鍵（Base58）
wallets-export-copy =
    .title = クリップボードにコピー
    .aria-label = クリップボードにコピー
wallets-export-reveal = 鍵を表示

# wallets.html: Archive and Delete dialogs.
wallets-archive-title = ウォレットをアーカイブ
wallets-archive-note = アーカイブしたウォレットはどの処理でも使用されませんが、いつでも復元できます。
wallets-archive-confirm = はい、アーカイブする
wallets-delete-title = ウォレットを削除
wallets-delete-warning-title = この操作は元に戻せません
wallets-delete-warning-body = このウォレットを削除すると、ウォレットと暗号化された秘密鍵がこのデバイスから完全に削除されます。
wallets-delete-confirm = はい、削除する

# wallets.html and bulk_operations.js: bulk import.
wallets-bulk-import-title = ウォレットをインポート
wallets-bulk-import-submit = ウォレットをインポート
wallets-bulk-step-upload = ファイルをアップロード
wallets-bulk-step-map = 列のマッピング
wallets-bulk-step-results = 結果
wallets-bulk-import-file-warning-body = ファイルは信頼できる提供元のものだけをインポートしてください。秘密鍵は暗号化され、このデバイス上に安全に保存されます。
wallets-bulk-drop-title = ここにファイルをドロップ
wallets-bulk-drop-subtitle = またはクリックして参照
wallets-bulk-drop-formats = CSV と Excel（.xlsx、.xls）に対応
wallets-bulk-file-remove =
    .aria-label = ファイルを削除
wallets-bulk-map-subtitle = ファイルの列をウォレットのフィールドに対応付けてください
wallets-bulk-preview-title = プレビュー（先頭5行）
wallets-bulk-summary-valid = 有効 <strong>{ $count }</strong>件
wallets-bulk-summary-invalid = 無効 <strong>{ $count }</strong>件
wallets-bulk-summary-duplicate =
    { $count ->
       *[other] 重複 <strong>{ $count }</strong>件
    }
wallets-bulk-done = 完了
wallets-bulk-file-invalid = ファイル形式が無効です。CSV または Excel ファイルを使用してください。
wallets-bulk-preview-busy = 処理中...
wallets-bulk-preview-fallback = ファイルの処理に失敗しました
wallets-bulk-preview-failed = ファイルの処理に失敗しました: { $reason }
wallets-bulk-column-select = -- 列を選択 --
wallets-bulk-preview-empty = ファイルにデータ行がありません
wallets-bulk-preview-status = ステータス
wallets-bulk-status-valid = 有効
wallets-bulk-status-duplicate = 重複
wallets-bulk-status-invalid = 無効
wallets-bulk-import-busy = インポート中...
wallets-bulk-import-toast =
    { $count ->
       *[other] ウォレットを { $count }件インポートしました
    }
wallets-bulk-import-error = インポートに失敗しました: { $reason }
wallets-bulk-result-success-title = インポート成功
wallets-bulk-result-success-detail =
    { $count ->
       *[other] { $count }件のウォレットをすべてインポートしました
    }
wallets-bulk-result-partial-title = 一部成功
wallets-bulk-result-partial-detail = インポート { $imported }件、失敗 { $failed }件
wallets-bulk-result-failed-title = インポート失敗
wallets-bulk-result-failed-detail =
    { $count ->
       *[other] { $count }件のウォレットすべてのインポートに失敗しました
    }
wallets-bulk-result-imported = インポート済み
wallets-bulk-result-failed = 失敗

# wallets.html and bulk_operations.js: bulk export.
wallets-bulk-export-title = ウォレットをエクスポート
wallets-bulk-export-format = 形式
wallets-bulk-export-format-csv = CSV (.csv)
wallets-bulk-export-format-xlsx = Excel (.xlsx)
wallets-bulk-export-include-archived = アーカイブ済みウォレットを含める
wallets-bulk-export-safe-title = 安全なエクスポート
wallets-bulk-export-safe-body = ウォレットのアドレスとメタデータのみをエクスポートします。秘密鍵は含まれません。
wallets-bulk-export-safe-submit = アドレスをエクスポート
wallets-bulk-export-or = または
wallets-bulk-export-danger-title = 危険なエクスポート
wallets-bulk-export-danger-body = エクスポートに秘密鍵を含めます。このファイルを入手した人は資金を盗むことができます。
wallets-bulk-export-danger-submit = 秘密鍵付きでエクスポート
wallets-bulk-export-busy = エクスポート中...
wallets-bulk-export-done = ウォレットを { $filename } にエクスポートしました
wallets-bulk-export-fallback = エクスポートに失敗しました
wallets-bulk-export-error = エクスポートに失敗しました: { $reason }
wallets-bulk-confirm-title = 危険なエクスポートの確認
wallets-bulk-confirm-warning =
    { $count ->
       *[other] 秘密鍵 <strong>{ $count }</strong>件をエクスポートしようとしています。極めて危険です。
    }
wallets-bulk-confirm-risk-steal = このファイルを入手した人は資金をすべて盗むことができます
wallets-bulk-confirm-risk-share = このファイルは絶対に誰にも共有しないでください
wallets-bulk-confirm-risk-delete = 使用後はすぐにファイルを削除してください
wallets-bulk-confirm-prompt = 確認のため、下のフレーズを入力してください
wallets-bulk-confirm-submit = 鍵をエクスポート

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = トークン
wallets-holdings-col-balance = 残高
wallets-holdings-col-value = 評価額（{ -sol }）
wallets-holdings-col-type = 種類
wallets-holdings-col-decimals = 小数桁数
wallets-holdings-empty-title = トークンの保有はありません
wallets-holdings-empty-message = このウォレットが保有するトークンがここに表示されます。
wallets-holdings-no-main = メインウォレットがありません
wallets-holdings-main-tag = メイン
wallets-holdings-main-title = メインウォレット
wallets-holdings-tokens = トークン
wallets-holdings-last-used = 最終使用
wallets-holdings-never = なし
wallets-holdings-search =
    .placeholder = シンボルまたはミントで検索...
wallets-holdings-export = 鍵をエクスポート
wallets-holdings-export-tooltip = このウォレットの秘密鍵をエクスポート
wallets-list-col-name = 名前
wallets-list-col-balance = 残高（{ -sol }）
wallets-list-col-type = 種類
wallets-list-col-created = 作成日時
wallets-list-col-actions = 操作
wallets-list-action-export = 秘密鍵をエクスポート
wallets-list-action-archive = ウォレットをアーカイブ
wallets-list-action-restore = ウォレットを復元
wallets-list-action-delete = 完全に削除
wallets-list-count = ウォレット
wallets-list-search =
    .placeholder = 名前またはアドレスで検索...
wallets-list-loading-title = ウォレットを読み込み中…
wallets-list-loading-description = 選択したウォレットビューを準備しています。
wallets-secondaries-empty-title = セカンダリウォレットはありません
wallets-secondaries-empty-message = ウォレットを追加して、複数のアカウントでトレード活動を整理できます。
wallets-secondaries-add = ウォレットを追加
wallets-archive-empty-title = アーカイブ済みのウォレットはありません
wallets-archive-empty-message = アーカイブしたウォレットは、今後の参照用にここに安全に保管されます。

# watched.js: watched wallets table and actions.
wallets-watched-col-wallet = ウォレット
wallets-watched-col-status = ステータス
wallets-watched-col-progress = 保存済みの進捗
wallets-watched-col-last-check = 最終チェック
wallets-watched-unlabelled = ラベルなしのウォレット
wallets-watched-generic-name = ウォレット
wallets-watched-not-synced = 未同期
wallets-watched-not-checked = 未チェック
wallets-watched-action-copy = コピートレード
    .title = このウォレットをコピートレードで開く
wallets-watched-action-restore = ウォッチを復元
wallets-watched-action-options = ウォッチのオプション
wallets-watched-action-retry = ウォッチを再試行
wallets-watched-action-pause = 一時停止
wallets-watched-action-enable = 有効化
wallets-watched-action-remove =
    .title = 削除
    .aria-label = { $name } を削除
wallets-watch-state-paused = 一時停止中
wallets-watch-state-catching-up = 追いつき中
wallets-watch-state-watching = ウォッチ中
wallets-watch-state-streaming = ストリーミング中
wallets-watch-state-polling = ポーリング中
wallets-watched-detail-helius = このウォレットを { -helius } 経由でチェックしています。
wallets-watched-empty-title = ウォッチ中のアドレスはありません
wallets-watched-empty-message = 「ウォレットをウォッチ」で、公開ウォレットのオンチェーンアクティビティを記録できます。
wallets-watched-count = ウォッチ中
wallets-watched-search =
    .placeholder = ウォッチ中のウォレットを検索...
wallets-watched-add = ウォレットをウォッチ
wallets-watched-refresh = ウォッチ中のウォレットを更新
wallets-watched-loading-title = ウォッチ中のウォレットを読み込み中...
wallets-watched-loading-description = 監視対象を取得しています。
wallets-watched-load-error-title = ウォッチ中のアドレスを読み込めませんでした
wallets-watched-load-error-description = 更新してもう一度お試しください。
wallets-watched-address-invalid = 有効な Solana ウォレットアドレスを入力してください。
wallets-watched-added = ウォレットウォッチを追加しました
wallets-watched-duplicate = そのウォレットはすでにウォッチ中です。
wallets-watched-add-failed = ウォレットウォッチを追加できませんでした。
wallets-watched-retried = 保存済みのカーソルでウォレットウォッチを復元しました
wallets-watched-paused = ウォレットウォッチを一時停止しました
wallets-watched-enabled = ウォレットウォッチを有効にしました
wallets-watched-removed = ウォレットウォッチを削除しました
wallets-watched-update-failed = ウォレットウォッチを更新できませんでした
wallets-setup-gate-title = ウォレットにはセットアップが必要です
