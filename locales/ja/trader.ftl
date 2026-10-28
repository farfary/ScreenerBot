# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = 損切り
trader-exit-type-take-profit = 利確
trader-exit-type-roi = 利確
trader-exit-type-roi-exit = 利確
trader-exit-type-trailing-stop = トレーリングストップ
trader-exit-type-time-override = 時間エグジット
trader-exit-type-time-rule = 時間ルール
trader-exit-type-manual = 手動
trader-exit-type-manual-close = 手動
trader-exit-type-dca = DCA
trader-exit-type-unknown = 不明

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = 統計
trader-tab-strategies = ストラテジー
trader-tab-stop-loss = 損切り
trader-tab-trailing-stop = トレーリングストップ
trader-tab-take-profit = 利確
trader-tab-time-rules = 時間エグジット
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
trader-setup-action = セットアップを完了
trader-toggle-on = 有効
trader-toggle-off = 無効
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
trader-force-stop-hint = すべてのモニター、コピータスク、取引を一度に停止します。
trader-force-stop-idle = 強制停止で止める対象は現在何も動いていません。

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

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = 統計期間
trader-stats-book =
    .aria-label = 取引帳簿
trader-paper-run-since = { $date } からのペーパー運用
trader-paper-start = 開始時
trader-paper-cash = 現金
trader-paper-open = 建玉
trader-paper-unrealized = 含み損益
trader-paper-reset = ペーパー運用をリセット
trader-paper-reset-title = ペーパー運用をリセットしますか？
trader-paper-reset-run-message = 保有中のペーパーポジションは、コピータスクのペーパー保有も含めてすべて現在価格で決済され、設定された初期残高で新しい運用が始まります。以前の運用は履歴に残ります。
trader-paper-reset-done = 新しいペーパー運用を開始しました
trader-paper-reset-failed = ペーパー運用をリセットできませんでした
trader-paper-account-failed = ペーパー運用を読み込めませんでした
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
trader-weekly-title = 週次損益
trader-weekly-subtitle = ISO週ごとの確定 { -sol } と累計
trader-weekly-chart = { -sol } 建ての週次確定損益
trader-extreme-best = 最良の取引
trader-extreme-worst = 最悪の取引

## Stats: exit breakdown

trader-exit-title = エグジット戦略の内訳
trader-exit-subtitle = ポジションのクローズ方法と、各エグジットの成果
trader-exit-loading = エグジットデータを読み込み中...
trader-exit-empty-day = 過去24時間にクローズ済みの取引はありません
trader-exit-empty-all = クローズ済みの取引はまだありません
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
# $part is a formatted percentage of the position.
trader-stop-loss-sell-part = { $part } を売却
trader-stop-loss-part-of-position = ポジションの一部
trader-stop-loss-sell-rest = 残りを売却
trader-stop-loss-still-below = まだストップを下回っている
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
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = 有効化時に確保される最低利益: { $value }
trader-trailing-locked-minimum-warning = 有効化時のストップは建値以下になります。利益が出ているときにしか売却しません。下限が必要なら建値ストップをオンにしてください。
trader-trailing-activation = 発動
trader-trailing-peak = 高値
# $value is a formatted percentage.
trader-trailing-final = 最終 +{ $value }%
# $value is a formatted percentage.
trader-trailing-summary-protected = 利益 <strong>{ $value }</strong> を確保
# $value is a formatted percentage.
trader-trailing-summary-avoided = 高値からの損失 <strong>{ $value }</strong> を回避
# $step is the band's position in the list, from 1.
trader-trailing-band = 段階 { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-profit-label = 段階 { $step } のピーク利益
# $step is the band's position in the list, from 1.
trader-trailing-band-distance-label = 段階 { $step } のトレール幅
trader-trailing-band-profit-caption = ピーク利益から
trader-trailing-band-distance-caption = トレール幅
trader-trailing-add-band = 段階を追加
# $step is the band's position in the list, from 1.
trader-trailing-remove-band = 段階 { $step } を削除
# $min and $max are the configured bounds of a band's peak profit.
trader-trailing-band-error-profit-range = 各段階のピーク利益は { $min }% から { $max }% の範囲にしてください
# $min and $max are the configured bounds of a band's distance.
trader-trailing-band-error-distance-range = 各段階のトレール幅は { $min }% から { $max }% の範囲にしてください
trader-trailing-band-error-order = 各段階のピーク利益は有効化の値と前の段階より高くしてください
trader-trailing-band-error-distance = 各段階のトレール幅はそのピーク利益より小さくしてください

## Break-even stop

trader-break-even-title = 建値ストップ
trader-break-even-subtitle = 含み益になったポジションを、損失に転じる前にすべて売却します
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = 前回の買い以降の最高値が { $trigger } に達した後、{ $offset } まで戻るとすべて売却します。

## Take profit

trader-take-profit-title = 利確
trader-take-profit-subtitle = 利益が各水準に達するたびにポジションを段階的に売却します
trader-take-profit-initial-buy = 初回購入
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = 段階 { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = 段階 { $step } の利益水準
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = 段階 { $step } の売却割合
trader-take-profit-profit-caption = 利益水準
trader-take-profit-sell-caption = 売却割合
trader-take-profit-add-step = 段階を追加
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = 段階 { $step } を削除
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = 各利益水準は { $min }% から { $max }% の範囲にしてください
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = 各割合は { $min }% から { $max }% の範囲にしてください
trader-take-profit-error-order = 各段階の利益水準は前の段階より高くしてください
# $max is the largest total share, the whole position.
trader-take-profit-error-total = 全段階の合計で売却できるのは最大 { $max }% です
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = ポジションの { $runner }% はランナーとして保有を続けます
trader-take-profit-runner-none = 最後の段階でポジションの残りを売却します
trader-take-profit-runner-warning = トレーリングストップ、建値ストップ、両方の時間エグジット、すべてのエグジットストラテジーがオフのため、ランナーは損切りか手動でのみ決済されます
trader-take-profit-info = 割合は当初ポジションに対するものです。価格が一度に複数の水準を超えた場合、1 回のチェックで 1 段階ずつ売却します。
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = +{ $profit }% で { $share }% を売却
trader-take-profit-example-runner = ランナー
trader-take-profit-example-runner-detail = 他の決済ルールに任せます
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = +{ $profit }% の利益までに <strong>{ $sold }%</strong> を売却

## Time-based exit

trader-time-loss-title = 損失ポジションを決済
trader-time-subtitle = 損失がしきい値を超えている場合、最大保有時間の経過後にポジションを自動でエグジットします
trader-time-flat-title = 横ばいポジションを決済
trader-time-flat-subtitle = 上にも下にも動かないポジションの資金を解放します
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = { $span } 以上保有し、損益が ±{ $band } 以内のポジションを決済します。
trader-time-unit-seconds = 秒
trader-time-unit-minutes = 分
trader-time-unit-hours = 時間
trader-time-unit-days = 日
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
trader-time-loss-hint = 保有時間の経過後に時間エグジットが売却する損失（40 = -40% でエグジット）
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
trader-time-positions-empty-message = オープンポジションは、保有時間と時間制限に対する ROI とともにここに表示されます。
trader-time-positions-token = トークン
trader-time-positions-hold = 保有時間
trader-time-positions-roi = ROI
trader-time-positions-rule = 時間エグジット
trader-time-status-off = 時間エグジットはオフです
trader-time-status-none = 現在適用される時間エグジットはありません
trader-time-status-not-managed = 自動トレーダーの管理対象外
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = 損失 { $pnl }、{ $remaining } 後に決済
trader-time-status-losing-due = 損失 { $pnl }、次回のチェックで決済
# $held and $remaining are formatted durations.
trader-time-status-flat = { $held } 横ばい、{ $remaining } 後に決済
trader-time-status-flat-due = { $held } 横ばい、次回のチェックで決済

## Dollar-cost averaging

trader-dca-title = ドルコスト平均法
trader-dca-subtitle = 含み損のポジションに自動で追加購入し、平均エントリー価格を下げます
trader-dca-measure-average = 平均エントリー
trader-dca-measure-first-entry = 初回エントリー
trader-dca-step = 追加 { $step }
trader-dca-drop-label = 追加 { $step } の価格下落
trader-dca-size-label = 追加 { $step } のサイズ（初回購入の倍率）
trader-dca-drop-caption = 価格下落
trader-dca-size-caption = 購入 × 初回購入
trader-dca-add-step = レベルを追加
trader-dca-remove-step = 追加 { $step } を削除
trader-dca-step-amount = = { $amount }
trader-dca-step-below-minimum = 最小取引額 { $minimum } を下回るため、この追加購入は送信されません
trader-dca-error-drop-range = 各下落率は { $min }% から { $max }% の間にしてください
trader-dca-error-size-range = 各サイズは初回購入の { $min }× から { $max }× の間にしてください
trader-dca-error-order = 初回エントリーを基準にする場合、各追加は前回より深い下落が必要です
trader-dca-cooldown-impact = 追加エントリーの間隔は最低 { $span }
trader-dca-min-hold-none = 初回の追加購入はエントリー直後から可能です
trader-dca-min-hold-impact = 初回の追加購入はエントリー後少なくとも { $span } 待ちます
trader-dca-capacity = このポジションの最大保有額：{ $amount }（初回購入の { $multiple }×）
trader-dca-info-title = DCA ストラテジー情報
trader-dca-info-subtitle = DCA 取引における重要な注意点
trader-dca-how-title = DCA の仕組み
trader-dca-how-trigger = <strong>トリガー：</strong>価格が次のレベルを下回ったとき（平均または初回エントリーから測定）
trader-dca-how-action = <strong>アクション:</strong> { -sol } を追加して平均取得原価を下げる
trader-dca-how-repeat = <strong>ラダー：</strong>追加ごとに次のレベルへ進み、すべてのレベルを使うと追加は止まります
trader-dca-how-pause = <strong>一時停止：</strong>前回の追加購入が決済中の間、または損失上限が新規エントリーを止めている間は追加しません
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
