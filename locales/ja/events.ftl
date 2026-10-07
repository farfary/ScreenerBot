# Event display text. Default ids come from src/events/recorders/; task ids
# from ScheduledTaskOutcome in src/events/display_text.rs. Arguments are data
# (subtype codes, method and API names, task names) and are not translated.
events-ohlcv-default = OHLCV イベント: { $subtype }
events-filtering-default = フィルタリングイベント: { $subtype }
events-trader-default = トレーダーイベント: { $subtype }
events-rpc-default = RPC { $method } - { $action }
events-api-default = { $api } - { $action }

# $name is the user-chosen scheduled task name.
events-task-completed = タスク「{ $name }」が完了しました
events-task-failed = タスク「{ $name }」が失敗しました
events-task-timed-out = タスク「{ $name }」がタイムアウトしました

# Event type column labels for the stable scheduled-task subtype codes.
events-subtype-task-completed = タスク完了
events-subtype-task-failed = タスク失敗
events-subtype-task-timed-out = タスクタイムアウト

# Shown when an event carries no display text.
events-message-none = メッセージなし

# Producer messages. Arguments are identifiers, counts and error text; counts
# are pre-formatted so digits are never grouped.
events-ohlcv-cache-cleanup-failed = OHLCV キャッシュのクリーンアップに失敗しました
events-ohlcv-gap-cleanup-failed = 補完済みギャップレコードのクリーンアップに失敗しました
events-ohlcv-gap-fill-failed = { $mint } のギャップ補完でエラーが発生しました
events-ohlcv-backfill-scheduled = { $pool } 経由で { $mint } のマルチ時間足バックフィルを予約しました
events-ohlcv-fetch-failed = { $pool } 経由での { $mint } の OHLCV 取得に失敗しました: { $error }
events-ohlcv-gap-detection-failed = { $pool } 経由での { $mint } のギャップ検出に失敗しました
events-ohlcv-fetch-success = { $mint } の OHLCV データポイントを { $count }件保存しました
events-ohlcv-retention-backfill-failed = { $pool } 経由での { $mint } の保持期間バックフィルに失敗しました
events-ohlcv-empty-fetch = { $pool } 経由での { $mint } の OHLCV 取得結果が空でした
events-ohlcv-pool-discovery-failed = { $mint } のプール探索に失敗しました
events-ohlcv-pool-discovery-success = { $mint } のプールを検出しました
events-ohlcv-process-token-error = { $mint } の処理中にエラーが発生しました: { $error }
events-ohlcv-rate-limit-hit = { $mint } の処理中にレート制限が発動しました
events-ohlcv-pool-unavailable = { $mint } に利用可能な正常なプールがないため、処理を延期します
events-ohlcv-token-missing = 処理中にトークン { $mint } が見つかりませんでした
events-monitors-stopped = 自動売買モニターを停止しました
events-monitors-starting = 自動売買モニターを起動中です
events-entry-monitor-started = エントリー機会モニターを開始しました
events-exit-monitor-started = エグジット/ポジションモニターを開始しました
events-trader-service-stopped = トレーダーサービスを正常に停止しました
events-trader-service-stopping = トレーダーサービスのシャットダウンを開始しました
events-trader-service-started = トレーダーサービスの初期化が完了し、稼働中です
events-trader-auto-trading-error = 自動売買でエラーが発生しました
events-trader-trading-enabled = 取引は有効で、稼働中です
events-trader-trading-disabled = 取引は設定で無効になっています
events-trader-service-initializing = トレーダーサービスの初期化を開始します
events-connectivity-monitoring-stopped = 接続監視を停止しました
events-connectivity-monitoring-started = 接続監視を開始しました（間隔={ $seconds }秒）
events-connectivity-service-initialized = 接続サービスを { $count }個のモニターで初期化しました
events-connectivity-critical-unhealthy = 重要なエンドポイント { $count }件が異常です。システムは処理を一時停止する必要があります
events-connectivity-endpoint-recovered = エンドポイントが { $from } から正常な状態に復旧しました
events-position-entry-not-landed = { $symbol } の購入はオンチェーンに反映されなかったため、ポジションを削除しました
events-position-fill-after-force-close = { $symbol } の取引がポジションの強制クローズ後にオンチェーンに反映されたため、記帳してポジションを再計算しました
events-position-swap-unbooked = オンチェーンで確定した { $symbol } のスワップがまだポジションに記帳されていません。記帳されるまで再検証します

## Events page (pages/events.js, ui/event_labels.js)

# Category ids from EventCategory in src/events/types.rs, plus the legacy entry and learner categories.
events-category-swap = スワップ
events-category-transaction = トランザクション
events-category-pool = プール
events-category-position = ポジション
events-category-token = トークン
events-category-wallet = ウォレット
events-category-trader = トレーダー
events-category-entry = エントリー
events-category-system = システム
events-category-ohlcv = OHLCV
events-category-rpc = RPC
events-category-api = API
events-category-security = セキュリティ
events-category-connectivity = 接続
events-category-filtering = フィルタリング
events-category-scheduled-task = スケジュールタスク
events-category-learner = ラーナー
events-category-other = その他

events-loading = イベントを読み込み中...
events-load-failed = イベントを読み込めませんでした
events-load-failed-description = バックエンドの応答を待っています。自動的に再試行します。
events-load-error = イベントを読み込めませんでした
events-search-placeholder = イベントを検索...
events-summary-total = 合計
events-filter-category = カテゴリー
events-filter-all-categories = すべてのカテゴリー
events-filter-all-severities = すべての重大度
events-col-time = 時刻
events-col-category = カテゴリー
events-col-type = 種類
events-col-severity = 重大度
events-col-message = メッセージ
events-col-token = トークン
events-col-details = 詳細
# $count is the number of payload entries not shown in the preview.
events-payload-more = ほか +{ $count }件

## Event details dialog (ui/events_dialog.js)

events-dialog-title = イベントの詳細
events-dialog-close =
    .aria-label = ダイアログを閉じる
events-dialog-payload = ペイロード
events-dialog-copy = 詳細をコピー
events-dialog-copy-title =
    .title = イベントの詳細をすべてコピー
events-dialog-copy-done = コピーしました
events-dialog-copy-failed = 失敗
events-dialog-not-available = 該当なし
# $category is the category label; shown when an event has no message.
events-dialog-category-event = { $category } イベント
events-dialog-field-id = イベント ID
events-dialog-field-severity = 重大度
events-dialog-field-category = カテゴリー
events-dialog-field-subtype = サブタイプ
events-dialog-field-mint = トークンミント
events-dialog-field-reference = 参照
events-dialog-field-time = イベント時刻
events-dialog-field-age = 経過時間
events-dialog-field-created = 作成日時
# Copied event text: section headings and one "label: value" line per field.
events-dialog-export-heading = イベントの詳細
events-dialog-export-message = メッセージ
events-dialog-export-payload = ペイロード
events-dialog-export-line = { $label }: { $value }
