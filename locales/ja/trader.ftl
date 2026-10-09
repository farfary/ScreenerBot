# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = 損切り
trader-exit-type-take-profit = 利確
trader-exit-type-roi = ROI 目標
trader-exit-type-roi-exit = ROI 目標
trader-exit-type-trailing-stop = トレーリングストップ
trader-exit-type-time-override = 時間による上書き
trader-exit-type-time-rule = 時間ルール
trader-exit-type-manual = 手動
trader-exit-type-manual-close = 手動
trader-exit-type-dca = DCA
trader-exit-type-unknown = 不明

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = 統計
trader-tab-strategy-control = ストラテジー制御
trader-tab-strategies = ストラテジー
trader-tab-stop-loss = 損切り
trader-tab-trailing-stop = トレーリングストップ
trader-tab-roi = 利確
trader-tab-time-rules = 時間ルール
trader-tab-dca = DCA
trader-tab-settings = 設定

## Feature status badges and their messages

trader-feature-coming-soon = 近日公開
    .message = この機能は近日公開予定で、まだ利用できません。
trader-feature-beta = ベータ
trader-feature-disabled = 無効
    .message = この機能は現在無効です。

## Status bar and trading controls

trader-status-title = 自動トレーダー
trader-status-loading = 読み込み中...
trader-status-running = 実行中
trader-status-stopped = 停止中
trader-status-setup-required = セットアップが必要です
trader-status-unavailable = 自動トレーダーを使うには、ウォレットと RPC のセットアップを完了してください
trader-toggle-on = 有効
trader-toggle-off = 無効
trader-toggle-unavailable = 利用不可
trader-toggle-start-failed = トレーダーを開始できませんでした
trader-toggle-stop-failed = トレーダーを停止できませんでした
trader-controls-title = トレード制御
trader-halt-title = 取引停止中
trader-halt-reason-default = 手動による強制停止
trader-halt-resume = 再開
trader-monitor-entry = エントリーモニター
trader-monitor-exit = エグジットモニター
trader-monitor-master-off = 自動トレーダーが無効です
trader-loss-limit-title = 期間損失上限
trader-loss-limit-resume = 取引を再開
trader-loss-limit-reset = 期間をリセット
trader-loss-limit-off = オフ
trader-loss-limit-none = 期間損失上限は設定されていません
# $hours and $minutes are formatted spans such as "2h" and "5m".
trader-loss-limit-resets-in = { $hours } { $minutes }後にリセット
trader-loss-limit-reached = 上限到達
trader-force-stop = すべてを強制停止

## Confirmations. `.message` is the body and `.confirm` the confirming button.

trader-force-stop-confirm = 取引を強制停止
    .message = すべての取引処理を直ちに停止します。続行しますか？
    .confirm = 取引を停止
trader-loss-limit-resume-confirm = 損失上限後に再開
    .message = 期間損失上限により新規エントリーが停止されています。再開すると、期間がリセットされる前にトレーダーが再びポジションを開けるようになります。続行しますか？
trader-loss-limit-reset-confirm = 損失上限の期間をリセット
    .message = 現在の期間の累積損失をクリアし、新しい期間を開始します。続行しますか？

## Toasts

trader-toast-control-failed = 自動トレーダーの操作に失敗しました
trader-toast-force-stop-on = 強制停止を有効にしました
trader-toast-force-stop-failed = 強制停止を有効にできませんでした
trader-toast-force-stop-cleared = 強制停止を解除しました
trader-toast-resume-failed = 取引を再開できませんでした
trader-toast-loss-limit-reset-failed = 損失上限をリセットできませんでした
trader-toast-entry-monitor-failed = エントリーモニターを切り替えられませんでした
trader-toast-exit-monitor-failed = エグジットモニターを切り替えられませんでした
trader-toast-load-failed = 読み込み失敗
    .message = トレーダー設定を読み込めませんでした
trader-toast-saved = 設定を保存しました
    .message = トレーダー設定を適用しました
trader-toast-save-failed = 保存失敗
    .message = トレーダー設定を保存できませんでした
trader-toast-feature-enabled = 機能を有効にしました
trader-toast-feature-disabled = 機能を無効にしました
trader-toast-feature-applied = 自動トレーダーの設定を適用しました
trader-toast-strategy-enabled = ストラテジーを有効にしました
    .message = ストラテジーは稼働中です
trader-toast-strategy-disabled = ストラテジーを無効にしました
    .message = ストラテジーは停止中です
trader-toast-strategy-failed = 更新失敗
    .message = ストラテジーのステータスを更新できませんでした

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = 統計期間
trader-stats-window-day = 24H
trader-stats-window-week = 7D
trader-stats-window-month = 30D
trader-realized-title = 確定パフォーマンス
trader-metric-net-pnl = 純損益
trader-metric-win-rate = 勝率
trader-metric-profit-factor = プロフィットファクター
trader-metric-max-drawdown = 最大ドローダウン
trader-metric-capital = 稼働中の資金
trader-metric-avg-win-loss = 平均利益 / 損失
trader-metric-closed-trades = クローズ済み取引
trader-metric-median-hold = 保有時間の中央値
trader-stats-empty = この期間にクローズ済みの取引はありません
# $won and $lost are formatted SOL amounts.
trader-stats-won-lost = 利益 { $won } · 損失 { $lost }
# $wins and $losses are the plural messages below.
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
       *[other] { $amount }勝
    }
trader-stats-losses =
    { $count ->
       *[other] { $amount }敗
    }
# $amount is a formatted SOL amount.
trader-stats-expected = 1取引あたりの期待値 { $amount }
trader-stats-profit-factor-basis = 総利益 ÷ 総損失
trader-stats-drawdown-basis = 確定ベースでの最大の高値から安値までの下落
# $count is the position limit and selects the plural.
trader-stats-slots =
    { $count ->
       *[other] ポジションスロット { $used } / { $max } 使用中
    }
trader-stats-avg-basis = 勝ちトレードと負けトレードの平均結果
trader-stats-closed =
    { $count ->
       *[other] クローズ済みポジション { $amount }件
    }
# $span is a formatted duration.
trader-stats-hold-average = 平均 { $span }
trader-stats-excluded =
    { $count ->
       *[other] クローズ済みラウンド { $amount }件を除外しました。取得原価が完全ではないため、正確な損益を算出できません。
    }

## Stats: daily P&L and extremes

trader-daily-title = 日次損益
trader-daily-subtitle = 1日ごとの確定 { -sol } と累計
trader-daily-loading = 日次損益を読み込み中...
trader-daily-chart = { -sol } 建ての日次確定損益
trader-extreme-best = 最良の取引
trader-extreme-worst = 最悪の取引

## Stats: exit breakdown

trader-exit-title = エグジット戦略の内訳
trader-exit-subtitle = ポジションのクローズ方法と、各エグジットの成果
trader-exit-loading = エグジットデータを読み込み中...
trader-exit-empty-day = 過去24時間にクローズ済みの取引はありません
trader-exit-empty-days =
    { $count ->
       *[other] 過去{ $amount }日間にクローズ済みの取引はありません
    }
# $share is a formatted percentage of all exits.
trader-exit-share =
    { $count ->
       *[other] { $amount }件の取引 · エグジット全体の { $share }
    }
# $value is a formatted average percentage.
trader-exit-average = 平均 { $value }

## Shared example vocabulary

trader-impact-label = 影響:
trader-current-label = 現在:
trader-readable-label = 読みやすい表記:
trader-example-how-it-works = 仕組み
trader-step-entry = エントリー
trader-step-initial-position = 初期ポジション
trader-step-auto-exit = 自動エグジット
trader-step-exit = エグジット
trader-step-full-exit = ポジション全体のエグジット
# $value is a percentage without its sign, as typed.
trader-value-percent = { $value }%
# $value is a percentage such as "20.0", shown after a plus sign.
trader-example-profit = +{ $value }% の利益

## Stop loss

trader-stop-loss-title = 損切り
trader-stop-loss-subtitle = 損失がしきい値を超えたときにポジションを自動でエグジットします
# $threshold is the threshold as typed.
trader-stop-loss-impact = エントリーから { $threshold }% 下落したらエグジット
trader-stop-loss-hold-immediate = 即時
# $span is a formatted duration.
trader-stop-loss-hold-delay = { $span }の遅延
trader-stop-loss-price-falls = 価格が下落
trader-stop-loss-threshold-reached = しきい値に到達
trader-stop-loss-partial = 部分エグジットを許可
# $loss is the loss percentage with its sign.
trader-stop-loss-summary = 損失を <strong>{ $loss }</strong> に限定
trader-stop-loss-note = <strong>注意:</strong> 損切りは早めにエグジットすることで、より大きな損失を防ぎます

## Trailing stop

trader-trailing-title = トレーリングストップ
trader-trailing-subtitle = 価格の上昇に追従して、利益を自動で守ります
# $value is the activation percentage as typed.
trader-trailing-activation-impact = +{ $value }% の利益で追従を開始
# $value is the trail distance percentage as typed.
trader-trailing-distance-impact = 高値から -{ $value }% でエグジット
trader-trailing-activation = 発動
trader-trailing-peak = 高値
# $value is a formatted percentage.
trader-trailing-final = 最終 +{ $value }%
# $value is a formatted percentage.
trader-trailing-summary-protected = 利益 <strong>{ $value }</strong> を確保
# $value is a formatted percentage.
trader-trailing-summary-avoided = 高値からの損失 <strong>{ $value }</strong> を回避

## Take profit

trader-roi-title = 利確
trader-roi-subtitle = 利益が目標に達したときにポジション全体を自動でエグジットします
# $target is the target percentage as typed.
trader-roi-impact = +{ $target }% の利益でエグジット
trader-roi-example-title = シナリオ例
trader-roi-initial-buy = 初回購入
trader-roi-target-hit = 目標到達
trader-roi-full-position = ポジション全体
trader-roi-sold = 100% 売却
# $target is the target percentage as typed.
trader-roi-summary = <strong>+{ $target }%</strong> の利益を確保

## Time-based exit

trader-time-title = 時間ベースのエグジット
trader-time-subtitle = 損失がしきい値を超えている場合、最大保有時間の経過後にポジションを自動でエグジットします
trader-time-unit-seconds = 秒
trader-time-unit-minutes = 分
trader-time-unit-hours = 時間
trader-time-unit-days = 日
# Shown before the configured duration loads.
trader-time-conversion-default = 168時間 = 7日
# $duration and $readable are formatted durations.
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
       *[other] { $amount }秒
    }
trader-duration-minutes =
    { $count ->
       *[other] { $amount }分
    }
trader-duration-hours =
    { $count ->
       *[other] { $amount }時間
    }
trader-duration-days =
    { $count ->
       *[other] { $amount }日
    }
# $value is the loss percentage as typed, without its sign.
trader-time-loss-impact = 保有期間経過後に { $value }% 以上下落していればエグジット
# $day is the day number of the example.
trader-time-day = { $day }日目
trader-time-position-opened = ポジションをオープン
trader-time-limit = 時間制限
trader-time-hold-reached = 保有期間に到達
trader-time-loss-met = 損失しきい値に到達
trader-time-note = <strong>注意:</strong> 利益が出ているポジションや損失が小さいポジションはエグジットされません
trader-time-positions-title = 現在のポジション状況
trader-time-positions-loading = ポジションを読み込み中...
trader-time-positions-empty = オープンポジションはありません
trader-time-positions-hold = 保有時間:
trader-time-positions-roi = ROI:

## Strategy control

trader-strategy-entry-title = エントリーストラテジー
trader-strategy-entry-subtitle = 新規ポジションを開くシグナルです。
trader-strategy-exit-title = エグジットストラテジー
trader-strategy-exit-subtitle = オープンポジションをクローズまたは保護するシグナルです。
trader-strategy-active-unknown = -- 稼働中
trader-strategy-active = { $enabled }/{ $total } 稼働中
trader-strategy-loading = ストラテジーを読み込み中...
trader-strategy-load-failed = ストラテジーを読み込めませんでした
trader-strategy-empty = ストラテジーが定義されていません
trader-strategy-no-description = 説明がありません。
trader-strategy-unnamed = 名称未設定のストラテジー
trader-strategy-priority-auto = 自動
trader-strategy-priority = 優先度 { $priority }

## Dollar-cost averaging

trader-dca-title = ドルコスト平均法
trader-dca-subtitle = 含み損のポジションに自動で追加購入し、平均エントリー価格を下げます
trader-dca-example-title = DCA の例
trader-dca-example = 初回 0.01 { -sol } → DCA #1: 0.005 { -sol } @ -10% → DCA #2: さらに -10% で 0.005 { -sol }
trader-dca-info-title = DCA ストラテジー情報
trader-dca-info-subtitle = DCA 取引における重要な注意点
trader-dca-how-title = DCA の仕組み
trader-dca-how-trigger = <strong>トリガー:</strong> ポジションが DCA しきい値を下回る（例: -10%）
trader-dca-how-action = <strong>アクション:</strong> { -sol } を追加して平均取得原価を下げる
trader-dca-how-repeat = <strong>繰り返し:</strong> 最大回数まで複数回 DCA できる
trader-dca-risk-title = リスクに関する警告
trader-dca-risk-exposure = <strong>エクスポージャーの増加:</strong> DCA はポジションごとのリスク資金の合計を増やします
trader-dca-risk-knife = <strong>落ちるナイフ:</strong> トークンが下落トレンドを続ける場合、DCA は効果がありません
trader-dca-risk-cooldown = <strong>クールダウン:</strong> クールダウンを使って、立て続けの DCA エントリーを避けましょう

## General settings

trader-sizing-title = ポジションサイズ
trader-sizing-subtitle = 1ポジションあたりの投資額を管理します
trader-timing-title = タイミングとクールダウン
trader-timing-subtitle = 各処理の間隔を管理します
trader-timing-close-cooldown = ポジションクローズ後のクールダウン
trader-timing-close-cooldown-hint = 同じトークンを再びオープンするまでの待機時間（分）
trader-timing-concurrency = エントリーチェックの同時実行数
trader-timing-concurrency-hint = 同時にチェックするトークン数（大きいほど高速ですが CPU 負荷が増えます）
trader-timing-unit-minutes = 分
trader-timing-unit-tokens = 銘柄
trader-timing-intervals = モニター間隔
trader-timing-intervals-badge = 読み取り専用
trader-timing-intervals-hint = アプリで固定されており、変更できません
trader-timing-intervals-value = <strong>エントリーモニター:</strong> 30秒 | <strong>エグジットモニター:</strong> 5秒
