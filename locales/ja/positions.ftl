# Position details labels.

# State reasons. Ids come from POSITION_CREATED_REASON in src/positions/database/types.rs.
positions-state-reason-position-created = ポジションを作成しました


# Source: scripts/pages/positions.js

## Views, origin and toolbar

# Ids are the position status values (POSITION_STATUS_LABELS, ui/position_status.js).
positions-status-open = オープン
positions-status-closed = クローズ済み
positions-status-archived = アーカイブ済み
positions-origin-copy = コピー
positions-origin-manual = 手動
positions-origin-wallet = ウォレット
positions-origin-copy-link =
    .title = このポジションを開いたコピータスクを開く
positions-holding-frozen = 凍結中
    .title = ミント権限によりこのトークンアカウントが凍結されました。残高は移動も売却もできません
positions-toolbar-total = 合計
positions-toolbar-delete-all = すべて削除
positions-search-placeholder = シンボルまたはミントで検索...
positions-filter-origin = 発生元
positions-filter-origin-all = すべての発生元
positions-filter-origin-auto = 自動トレーダー
positions-filter-origin-copy = コピートレード
positions-delete-all-tooltip = アーカイブ済みのポジションをすべて完全に削除します

## Columns

positions-column-token = トークン
positions-column-archived-at = アーカイブ日時
positions-column-entry-time = エントリー時刻
positions-column-exit-time = エグジット時刻
positions-column-avg-entry = 平均エントリー（{ -sol }）
positions-column-avg-exit = 平均エグジット（{ -sol }）
positions-column-current-price = 現在（{ -sol }）
positions-column-total-invested = 総投資額
positions-column-proceeds = 売却代金
positions-column-pnl = 損益
positions-column-pnl-percent = 損益 %
positions-column-size = サイズ
positions-column-dca = DCA
positions-column-exits = エグジット
positions-column-unrealized-pnl = 含み損益
positions-column-unrealized-percent = 含み損益 %

## Cells

# Shown instead of a figure the wallet history cannot support.
positions-unknown-basis = このウォレットの履歴に取得原価がありません（エアドロップ、USD建ての約定、または SOL を含まないスワップ）
positions-unknown-history = このラウンドはオンチェーン残高と一致しません
positions-dca-count =
    { $count ->
       *[other] DCA { $count }回
    }
positions-exit-count =
    { $count ->
       *[other] エグジット { $count }回
    }

## Row actions

positions-action-add =
    .title = ポジションに追加（DCA）
    .aria-label = ポジションに追加
positions-action-sell =
    .title = 売却（全量または % 指定の部分売却）
    .aria-label = ポジションを売却
positions-action-sell-frozen = ミント権限により凍結されているため、この保有分は売却できません
positions-action-remove =
    .title = 削除（アーカイブまたは完全削除）
    .aria-label = ポジションを削除
positions-action-restore =
    .title = オープン/クローズ済みに戻す
    .aria-label = ポジションを復元
positions-action-delete =
    .title = 完全に削除
    .aria-label = 完全に削除
positions-action-in-progress = 処理中…

## Live state of a row

positions-caption-buying = 購入中
# $step is the label of the current action step.
positions-caption-buying-step = 購入中 · { $step }
positions-caption-selling = 売却中
positions-caption-selling-step = 売却中 · { $step }
positions-caption-closing = クローズ中
positions-caption-failed = 失敗
# $error is the failure text of the action.
positions-caption-failed-detail = 失敗 · { $error }
positions-step-adding = 追加中
positions-pending-buying = 購入中…
positions-pending-buy-failed = 購入に失敗しました

## Messages and confirmations

positions-load-failed = ポジションを更新できませんでした
positions-toast-not-found = ポジションデータが見つかりません
positions-toast-deleted = ポジションを削除しました
positions-toast-archived = ポジションをアーカイブしました
positions-toast-restored = ポジションを復元しました
positions-action-failed = 操作に失敗しました
positions-delete-title = ポジションを完全に削除
# $symbol is the token symbol.
positions-delete-message = { $symbol } を完全に削除しますか？ ポジションとその履歴がデータベースから削除され、元に戻せません。トランザクションとトークンのデータには影響しません。
positions-delete-confirm = 完全に削除
positions-delete-all-title = アーカイブ済みポジションをすべて削除
positions-delete-all-message =
    { $count ->
       *[other] アーカイブ済みのポジション { $count }件をすべて完全に削除しますか？ この操作は元に戻せません。トランザクションとトークンのデータには影響しません。
    }
positions-delete-all-message-empty = アーカイブ済みのポジションをすべて完全に削除しますか？ この操作は元に戻せません。
positions-delete-all-confirm = すべて削除
positions-delete-all-done =
    { $count ->
       *[other] アーカイブ済みのポジションを { $count }件削除しました
    }
positions-delete-all-failed = アーカイブ済みポジションの削除に失敗しました

# Source: scripts/ui/position_remove_dialog.js

## Remove position dialog

positions-remove-title = ポジションを削除
# Inline markup: emphasis on the opening sentence and on "not".
positions-remove-open-warning = <strong>このポジションはまだオープンです。</strong>ボットはこのトークンを保有しています。削除するとトレードスロットが解放され、追跡が停止しますが、売却は<strong>されません</strong>。{ -sol } を戻したい場合は先に売却してください。
positions-remove-modes =
    .aria-label = 削除モード
positions-remove-archive = アーカイブ
positions-remove-recommended = 推奨
positions-remove-archive-description = アーカイブタブに移して非表示にします。いつでも元に戻せます。売却は行われず、すべての取引は記録に残ります。
positions-remove-delete = 完全に削除
positions-remove-delete-description = このポジションとその全履歴をデータベースから消去します。
# Inline markup: emphasis on the irreversibility sentence.
positions-remove-danger = ポジションとその履歴を完全に削除します。<strong>この操作は元に戻せません。</strong>トランザクションとトークンのデータには影響しません。
positions-remove-confirm-archive = ポジションをアーカイブ

# Source: scripts/ui/position_details_dialog.js, scripts/ui/position_details/panes.js

## Position details frame

# Message shown after a management change. $mode is the label of the new mode.
positions-management-changed = ポジション管理を { $mode } に設定しました
positions-details-load-failed = ポジションの詳細を読み込めませんでした
positions-details-mint-label = ミントアドレス
positions-details-management-failed = ポジション管理の更新に失敗しました
positions-details-favorite-add =
    .title = お気に入りに追加
    .aria-label = お気に入りに追加
positions-details-favorite-remove =
    .title = お気に入りから削除
    .aria-label = お気に入りから削除
positions-details-view-solscan =
    .title = { -solscan } で表示
    .aria-label = { -solscan } でトークンを表示
positions-details-close =
    .title = 閉じる（Esc）
    .aria-label = 閉じる
positions-details-chart-section =
    .aria-label = 価格チャート
positions-details-loading-chart = チャートを読み込み中...
positions-details-activity-section =
    .aria-label = アクティビティ
positions-details-activity-title = アクティビティ
positions-details-split-handle =
    .aria-label = チャートとアクティビティのサイズを変更
positions-details-activity-pane =
    .aria-label = アクティビティペイン
positions-details-activity-expand =
    .title = アクティビティを展開
    .aria-label = アクティビティを展開
positions-details-summary-section =
    .aria-label = ポジションの概要
positions-details-loading = ポジションを読み込み中...

## Management modes. Ids are the PositionManagement serde ids (src/positions/types.rs).

positions-management-auto-trader = 自動トレーダー
positions-management-user-only = ユーザーのみ
positions-management-copy-task = コピータスク
positions-management-hybrid = ハイブリッド
positions-pane-show-chart = チャートを表示
positions-pane-show-activity = アクティビティを表示
positions-pane-restore-activity = アクティビティを元に戻す
positions-pane-expand-chart =
    .title = チャートを展開
    .aria-label = チャートを展開

# Source: scripts/ui/position_details/header.js

## Position details header

positions-risk-low = 低リスク
positions-risk-medium = 中リスク
positions-risk-high = 高リスク
positions-risk-unknown = リスク不明
positions-busy-buying = 購入処理中…
positions-busy-selling = 売却処理中…
positions-busy-closing = クローズ処理中…
positions-header-avg-entry = 平均エントリー
# $count is the number of buys: the entry plus each add.
positions-header-buy-count =
    { $count ->
       *[other] 購入 { $count }回
    }
positions-header-exit-price = エグジット価格
# $ago is the elapsed time since the close, for example "3h ago".
positions-header-closed-ago = { $ago }にクローズ
positions-header-realized-pnl = 確定損益
positions-header-usd-note = 本日の { -sol } 価格での USD 換算
positions-header-returned = 回収額
# $amount is the formatted SOL amount invested.
positions-header-of-invested = 投資額 { $amount } のうち
positions-header-price = 価格
positions-header-last-price = 直近価格
positions-header-pool-ago = プール · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = 含み損益
positions-header-pnl-last-price = 直近価格での損益
positions-header-value = 評価額
positions-header-last-value = 直近評価額
positions-header-invested = 投資額 { $amount }
positions-header-origin-hint = このポジションの開始方法
positions-header-risk-hint = { -rugcheck } スコア（低いほど安全）
positions-header-frozen = 凍結中
    .title = ミント権限によりこの保有分が凍結されました
positions-header-managed-by = 管理元
positions-header-management-select =
    .aria-label = ポジション管理

## Entry origin shown in the header badge

positions-origin-unknown = 不明
# $task is the copy task id. The source wallet follows in its own element.
positions-origin-copied-task = コピー · タスク { $task }
positions-origin-manual-entry = 手動エントリー
positions-origin-wallet-entry = ウォレットエントリー
# $strategy is the strategy id.
positions-origin-auto-strategy = 自動 · { $strategy }
positions-origin-auto-entry = 自動エントリー

## Swaps that are submitted and not yet booked

positions-pending-adding = 追加中
positions-pending-adding-amount = { $amount } を追加中
positions-pending-selling = 売却中
# $percent is the formatted share of the position being sold.
positions-pending-selling-percent = { $percent } を売却中
# $label is the pending swap wording.
positions-pending-confirming = { $label } · 確認中
    .title = 送信済みで、オンチェーンでの確認待ちです。検証が完了すると数値が更新されます。

## Trade controls

positions-trade-add = 追加購入
    .title = ポジションに追加
positions-trade-sell = 売却
    .title = ポジションの一部を売却
positions-trade-close = ポジションをクローズ
    .title = 全量を売却してクローズ
positions-trade-token = トークン詳細
    .title = トークン詳細を開く

## Favorites

positions-favorite-token-fallback = トークン
# $symbol is the token symbol.
positions-favorite-added = { $symbol } をお気に入りに追加しました
positions-favorite-removed = { $symbol } をお気に入りから削除しました
positions-favorite-add-failed = お気に入りの追加に失敗しました
positions-favorite-remove-failed = お気に入りの削除に失敗しました
positions-favorite-update-failed = お気に入りの更新に失敗しました

# Source: scripts/ui/position_details/summary.js

## Summary rail

positions-summary-position = ポジション
positions-summary-price-path = 価格推移
positions-summary-network-fees = ネットワーク手数料
positions-summary-risk = リスク
positions-summary-market = マーケット
positions-summary-market-now = 現在のマーケット
positions-summary-links = リンク
positions-fact-tokens-fallback = トークン
positions-fact-bought = 購入
positions-fact-holding = 保有
positions-fact-sold = 売却
positions-fact-realized = 確定
positions-fact-opened = オープン
positions-fact-closed = クローズ
positions-fact-reason = 理由
positions-fact-archived = アーカイブ
positions-fact-entry = エントリー
positions-fact-exit = エグジット
positions-fact-total = 合計
positions-fact-verified = オンチェーンで検証済み
positions-fact-confirming = 確認中
# $percent is the formatted share, for example "12.5%".
positions-fact-share-of-bought = 購入量の { $percent }
positions-fact-share-of-invested = 投資額の { $percent }
# $count is the number of adds after the entry.
positions-fact-entry-count =
    { $count ->
        [0] エントリー1回
       *[other] エントリー1回 + 追加購入{ $count }回
    }
# $count is the number of partial exits, $returned the formatted SOL amount.
positions-fact-partial-exits-back =
    { $count ->
       *[other] 部分エグジット { $count }回 · { $returned } 回収
    }
# $age is the elapsed time of the hold.
positions-fact-held = 保有 { $age }
# $percent is the signed change against the entry price.
positions-fact-vs-entry = エントリー比 { $percent }
positions-fact-exit-vs-peak = エグジットと高値の比較
positions-fact-now-vs-peak = 現在と高値の比較
positions-fact-entry-range = エントリー範囲
positions-range-low = 安値
positions-range-peak = 高値
positions-range-now = 現在
positions-range-label-exit = 安値から高値までの間のエントリー価格とエグジット価格
positions-range-label-now = 安値から高値までの間のエントリー価格と現在価格
positions-fact-mint-authority = ミント権限
positions-fact-freeze-authority = フリーズ権限
positions-fact-active = 有効
positions-fact-pool = プール
# $amount is the formatted liquidity in SOL.
positions-fact-pool-liquidity = 流動性 { $amount } { -sol }
positions-fact-market-cap = 時価総額
# $value is the formatted fully diluted valuation in USD.
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = 流動性
positions-fact-volume-24h = 出来高 24h
positions-fact-price-change = 価格変動
positions-change-period-1h = 1h
positions-change-period-24h = 24h
positions-fact-holders = ホルダー
positions-link-website = ウェブサイト
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

# Source: scripts/ui/position_details/activity.js, scripts/ui/position_details/activity_event.js

## Activity

positions-activity-load-failed = アクティビティを読み込めませんでした
positions-activity-loading = アクティビティを読み込み中...
positions-activity-empty = このウォレットでは、このトークンに関する動きはまだありません
positions-activity-filter-empty = このフィルターに一致するアクティビティはありません
positions-activity-round-count =
    { $count ->
       *[other] { $count }ラウンド
    }
positions-activity-event-count =
    { $count ->
       *[other] { $count }件のイベント
    }
positions-activity-pending-count = 保留中 { $count }件
positions-activity-failed-count = 失敗 { $count }件
positions-filter-all = すべて
positions-filter-trades = 取引
positions-filter-buys = 購入
positions-filter-sells = 売却
positions-filter-wallet = ウォレット
positions-filter-issues = 問題
positions-activity-filters =
    .aria-label = アクティビティを絞り込み
positions-activity-totals =
    .aria-label = このトークンのすべてのラウンド
positions-activity-realized-all = 確定損益（全ラウンド）
positions-activity-invested = 投資額
positions-activity-returned = 回収額
# $when is the formatted open time of a round that has not closed.
positions-activity-opened = { $when }にオープン
# $index is the 1-based number of the round.
positions-activity-round-title = ポジション { $index }
positions-activity-this-position = このポジション
positions-activity-dates-unavailable = 日付を取得できません
positions-activity-wallet-title = ウォレットのトランザクション
# $range is the date range, $count the number of events.
positions-activity-outside =
    { $count ->
       *[other] ポジション外 · { $range } · { $count }件のイベント
    }
positions-details-signature-label = シグネチャ

## State history milestones. Ids are the PositionState names (src/positions/database/types.rs).

positions-state-open = ポジションがオープン
positions-state-closing = ポジションをクローズ中
positions-state-closed = ポジションがクローズ
positions-state-exit-pending = ポジションのエグジット待ち
positions-state-exit-failed = ポジションのエグジットに失敗
positions-state-phantom = ポジションがファントム状態
positions-state-reconciling = ポジションを照合中

## Activity events

positions-event-kind-entry = エントリー
positions-event-kind-dca = 追加購入
positions-event-kind-partial-exit = 部分エグジット
positions-event-kind-exit = エグジット
positions-event-kind-buy = ウォレット購入
positions-event-kind-sell = ウォレット売却
positions-event-kind-transfer = 送金
positions-event-kind-ata = トークンアカウント
positions-event-kind-other = トランザクション
positions-event-state-pending = 保留中
positions-event-state-failed = 失敗
positions-event-state-synthetic = 合成
# $error is the failure text reported by the chain.
positions-chain-status-failed-detail = 失敗: { $error }
positions-event-tokens-fallback = トークン
# In the descriptions below $amount is the token amount with its symbol, $sol the SOL amount
# and $percent the share of the position sold.
positions-event-entry-submitted = { $amount } の購入を送信しました
positions-event-entry-for = { $sol } で { $amount } を購入しました
positions-event-entry = { $amount } を購入しました
positions-event-dca-submitted = { $amount } の追加購入を送信しました
positions-event-dca-for = { $sol } で { $amount } を追加購入しました
positions-event-dca = { $amount } を追加購入しました
positions-event-partial-exit-submitted-percent = { $amount } の部分エグジット（{ $percent }）を送信しました
positions-event-partial-exit-submitted = { $amount } の部分エグジットを送信しました
positions-event-sold-percent-for = { $amount }（{ $percent }）を { $sol } で売却しました
positions-event-sold-percent = { $amount }（{ $percent }）を売却しました
positions-event-sold-for = { $amount } を { $sol } で売却しました
positions-event-sold = { $amount } を売却しました
positions-event-exit-submitted = ポジション全体のエグジットを送信しました
positions-event-exit-for = { $amount } を { $sol } で売却してクローズしました
positions-event-exit-closed = ポジションをクローズしました
positions-event-wallet-bought = ウォレットが別の場所で { $amount } を購入しました
positions-event-wallet-sold = ウォレットが別の場所で { $amount } を売却しました
positions-event-received = { $amount } を受け取りました
positions-event-sent = { $amount } を送信しました
positions-event-transferred = { $amount } を送金しました
positions-event-ata = トークンアカウントのアクティビティ
positions-event-wallet-transaction = { $amount } に関するウォレットのトランザクション
# $price is the formatted price per token in SOL.
positions-event-price-per-token = { $price } { -sol } / トークン
# $amount is the signed SOL change of the wallet.
positions-event-wallet-change = ウォレット増減 { $amount }
positions-event-after-title = このイベント後のポジション
positions-event-capital-invested = 投下資本
positions-event-average-entry = 平均エントリー
positions-event-transfers-title = トークン送金
positions-event-transfer-amount = 数量
positions-event-transfer-mint = ミント
positions-event-transfer-from = 送信元
positions-event-transfer-to = 送信先
positions-event-no-signature = オンチェーンのシグネチャがありません
positions-event-click-to-copy = クリックでコピー
positions-event-solscan = { -solscan }
positions-event-token-amount = トークン数量
positions-event-trade-price = 取引価格
positions-event-sol-amount = { -sol } 数量
positions-event-cost-basis = 取得原価
positions-event-usd-value = USD 価値
positions-event-network-fee = ネットワーク手数料
positions-event-router = ルーター
positions-event-slot = スロット
positions-event-chain-status = チェーンステータス
positions-event-transaction-type = トランザクション種別
positions-event-direction = 方向
positions-event-wallet-sol-change = ウォレットの { -sol } 増減
positions-event-instructions = インストラクション
positions-event-compute-units = コンピュートユニット
positions-event-accounts = アカウント
positions-event-record-id = レコード ID
positions-event-time-unavailable = 時刻を取得できません
positions-event-details = 詳細
positions-event-hide-details = 詳細を隠す

# Source: scripts/ui/position_details/chart.js

## Position chart

positions-chart-type-candles = ローソク足
positions-chart-type-line = ライン
positions-chart-type-area = エリア
positions-chart-type-group =
    .aria-label = チャートの種類
positions-chart-overlays-group =
    .aria-label = チャートのオーバーレイ
positions-chart-ema = EMA
    .title = 指数移動平均（9 と 21）
positions-chart-fit = 全体表示
    .title = このポジションの保有期間全体を表示
positions-chart-timeframes-group =
    .aria-label = 時間足
positions-chart-pane-group =
    .aria-label = チャートペイン
positions-chart-unavailable = チャートエンジンを利用できません
positions-chart-collecting = チャートデータを収集中…
positions-chart-no-data = このトークンのチャートデータはまだありません
positions-chart-avg-entry = 平均エントリー
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = 平均エントリー
positions-chart-legend-avg-entry-off-scale = 平均エントリー（範囲外）
positions-chart-dropped-events =
    { $count ->
       *[other] この時間足にローソク足がないイベント { $count }件
    }
positions-chart-level = レベル
# $label names the reference level, $price is its formatted price.
positions-chart-level-above = { $label } { $price } はこの表示範囲より上です
positions-chart-level-below = { $label } { $price } はこの表示範囲より下です
positions-chart-scale-hint = 価格軸をドラッグして範囲を広げます
positions-chart-pnl-at-bar = 足時点の損益
positions-chart-click-to-locate = クリックで位置を表示
