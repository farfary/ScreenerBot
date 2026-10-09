copy-skip-not-buy-swap = ウォレットの動きは購入ではありませんでした
copy-skip-task-disabled = タスクは一時停止中です
copy-skip-mode-transition-required = 実行モードは別途変更する必要があります
copy-skip-live-confirmation-required = ライブ実行には確認が必要です
copy-skip-unsupported-sizing-mode = このサイジングモードはまだ対応していません
copy-skip-self-copy = このウォレットはご自身のものです
copy-skip-target-below-minimum = ウォレットの取引が最小額を下回っています
copy-skip-target-above-maximum = ウォレットの取引が最大額を上回っています
copy-skip-already-bought = このトークンは購入済みです（購入は 1 回のみ）
copy-skip-blacklisted = トークンはリスク管理によりブロックされています
copy-skip-filter-required = トークンがフィルタリングを通過しませんでした
copy-skip-budget-exhausted = タスクの予算を使い切りました
copy-skip-token-cap-reached = トークンごとの上限に達しました
copy-skip-below-minimum-size = コピーサイズが小さすぎます
copy-skip-invalid-sizing = タスクのサイジングが無効です
copy-skip-invalid-slippage = タスクのスリッページが無効です
copy-skip-invalid-exit-policy = タスクのエグジットルールが無効です
copy-skip-invalid-price = 使用できる市場価格がありません
copy-skip-not-sell-swap = ウォレットの動きは売却ではありませんでした
copy-skip-exit-mode-disabled = ウォレットの売却は無視しました: このタスクは独自のルールで売却します
copy-skip-force-stopped = 取引は強制停止されています
copy-skip-copy-position-not-found = このタスクが保有するポジションがありません
copy-skip-position-user-only = ポジションはユーザーが管理しています
copy-skip-position-management-mismatch = ポジションはコピー売却に従わなくなりました
copy-skip-latency-kill-switch = 自動一時停止: 取引の検出が遅すぎました
copy-skip-claim-reconciled-abandoned = 中断されたライブ送信は再試行せずに終了しました
copy-skip-stale-observation = 停止期間後に再生されたため、コピーするには古すぎます
copy-skip-unknown-observation-time = 再生された取引にブロック時刻がありません
copy-skip-entry-blocked = エントリーがブロックされました

copy-entry-block-force-stopped = 取引は強制停止されています
copy-entry-block-loss-limit = 損失上限により新規エントリーがブロックされています
copy-entry-block-connectivity = 必要なサービスが利用できません
copy-entry-block-position-limit = オープンポジションの上限に達しました
copy-entry-block-already-open = ポジションはすでにオープンしています
copy-entry-block-reentry-cooldown = トークン再エントリーのクールダウン
copy-entry-block-open-cooldown = 全体のエントリークールダウン
copy-entry-block-entry-reserved = 別のエントリーを処理中です
copy-entry-block-blacklisted = トークンはリスク管理によりブロックされています
copy-entry-block-check-failed = 安全性チェックを完了できませんでした

copy-pause-user = ユーザーが一時停止
copy-pause-latency-kill-switch = 自動一時停止: 取引の到着が平均 { $average }秒遅れました（上限 { $threshold }秒）
copy-pause-watch-detached = 自動一時停止: ウォレットのウォッチが外れました
copy-pause-watch-budget-exceeded = 一時停止: このウォレットは追いつく前に、ウォッチのチェック上限（{ $limit } シグネチャ）に達しました
copy-pause-helius-unavailable = 一時停止: { -helius } のウォレットチェックに失敗しました
copy-pause-watch-processing-failed = 一時停止: ウォレットの動きを処理できませんでした
copy-pause-unspecified = 一時停止中

copy-pause-short-user = ユーザー操作
copy-pause-short-latency-kill-switch = 遅延超過
copy-pause-short-watch-detached = ウォッチ切断
copy-pause-short-watch-budget-exceeded = ウォッチ上限
copy-pause-short-helius-unavailable = ウォッチプロバイダー
copy-pause-short-watch-processing-failed = ウォッチ処理
copy-state-paused = 一時停止中
copy-state-paused-reason = 一時停止中 · { $reason }

copy-readiness-history = ペーパー履歴
copy-readiness-history-met =
    { $count ->
       *[other] クローズ済みペーパーラウンド { $count }件（必要数: { $needed }）
    }
copy-readiness-history-short = クローズ済みペーパーラウンド { $count }/{ $needed }
copy-readiness-profit = ペーパーで黒字
copy-readiness-profit-detail =
    { $count ->
       *[other] { $count }ラウンドで { $realized } { -sol } を確定、{ $wins }勝
    }
copy-readiness-latency = 取引を時間内に検出
copy-readiness-latency-detail = p95 到着 { $p95 }秒、上限 { $limit }秒
copy-readiness-latency-none = 到着サンプルはまだありません
copy-readiness-priced = すべての保有に価格あり
copy-readiness-priced-ok = オープン中のペーパー保有すべてにプール価格があります
copy-readiness-priced-missing =
    { $count ->
       *[other] プール価格のないオープン保有が { $count }件あります
    }
copy-readiness-runtime = ライブ実行が利用可能
copy-readiness-runtime-ok = セットアップと安全ゲートによりライブコピーが許可されています

copy-live-block-setup-incomplete = 先にウォレットと RPC のセットアップを完了してください
copy-live-block-force-stop = 緊急停止が有効です
copy-live-block-copy-trading-disabled = コピー処理が全体で一時停止されています
copy-live-block-unavailable = ライブ実行は利用できません

## Task state, mode and exit labels

copy-state-system-paused = 全体で一時停止中
copy-state-force-stopped = 強制停止
copy-state-entries-blocked = エントリーをブロック中
copy-state-running-live = 実行中
copy-state-running-paper = 実行中
copy-mode-paper = ペーパー
copy-mode-live = ライブ
copy-exit-mode-buy-only = 自分のエグジットルール
copy-exit-mode-mirror = ウォレットの売却をミラー
copy-exit-mode-hybrid = ウォレットの売却と自分のルール
copy-exit-target-sell = ウォレットが売却
copy-exit-stop-loss = 損切り
copy-exit-trailing-stop = トレーリングストップ
copy-exit-take-profit = 利確
copy-exit-time-override = 時間ルール
copy-exit-manual = 手動でクローズ

## Shared wording

copy-request-failed = リクエストに失敗しました
copy-keep-paused = 一時停止のまま
copy-paused-suffix = · 一時停止中
copy-mode-paused = { $mode } · 一時停止中
copy-task-ref = 「{ $name }」（{ $mode }）
copy-metric-realized-pnl = 確定損益
copy-metric-unrealized-pnl = 含み損益
copy-metric-win-rate = 勝率
copy-metric-budget-spent = 使用済み予算
copy-metric-median-arrival = 到着の中央値
copy-metric-open-holdings = オープン保有
copy-record-won-lost = { $won }勝 · { $lost }敗
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = 約定
copy-kind-exits = エグジット
copy-kind-skips = スキップ
copy-kind-errors = エラー
copy-field-per-trade-cap = 取引ごとの上限
copy-field-per-token-cap = トークンごとの上限
copy-field-total-budget = 総予算
copy-field-slippage = スリッページ
copy-rules-wallet-sells-only = ウォレットの売却のみ
copy-filter-copy-setting-required = コピートレード設定（必須）
copy-filter-copy-setting-not-required = コピートレード設定（必須ではない）
copy-count-closed-rounds =
    { $count ->
       *[other] クローズ済みラウンド { $count }件
    }
copy-count-open-holdings =
    { $count ->
       *[other] オープン保有 { $count }件
    }
copy-unrealized-partial =
    { $priced ->
       *[other] 価格あり { $priced }件 · 価格なし { $unpriced }件
    }
copy-unrealized-unpriced =
    { $count ->
       *[other] 価格なしの保有 { $count }件
    }
copy-range-24h = 24h
copy-range-7d = 7d
copy-range-30d = 30d
copy-range-all = すべて
copy-range-label =
    .aria-label = 期間

## Page strip

copy-page-title = コピートレード
copy-page-beta = ベータ
copy-strip-loading = 読み込み中
copy-strip-unavailable = 利用不可
copy-strip-setup-required = セットアップが必要 · コピートレードにはウォレットと RPC が必要です
copy-strip-pause-all = すべて一時停止
copy-strip-resume = 処理を再開
copy-strip-settings = 設定
copy-strip-add-wallet = ウォレットを追加
copy-strip-paused-globally = 全体で一時停止中 · 新規コピーなし、エグジットは実行
copy-strip-force-stopped = 強制停止 · コピーは行われません
copy-strip-loss-limit = 損失上限 · 新規エントリーをブロック、エグジットは実行
copy-strip-idle-paused =
    { $count ->
       *[other] 待機中 · 一時停止中のタスク { $count }件
    }
copy-strip-idle-empty = 待機中 · タスクはまだありません
copy-strip-processing = 処理中 · ペーパー { $paper }件
copy-strip-processing-live = 処理中 · ライブ { $live }件 · ペーパー { $paper }件
copy-figures-label =
    .aria-label = コピートレードの合計
copy-figure-marked-at-pool = プール価格で評価
copy-figure-across-tasks = 全タスクの合計
copy-figure-budget-lifetime = 有効なタスクの累計使用額
copy-figure-budget-none = 有効なタスクはありません
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
       *[other] 取引 { $count }件
    }
copy-figure-arrival-none = 有効なタスクのサンプルはありません

## Page frame

copy-load-failed = コピートレードを読み込めませんでした: { $error }
copy-resume-all-title = コピー処理を再開
copy-resume-all-message =
    { $count ->
       *[other] ライブタスク { $count }件は、ウォレットが再び取引すると実際のスワップを送信します。
    }
copy-toast-resumed-all = コピー処理を再開しました
copy-toast-paused-all = すべてのコピー処理を一時停止しました
copy-toast-global-failed = コピー処理を変更できませんでした

## Onboarding

copy-onboarding-title = 信頼できるウォレットをコピーし、まずペーパーで検証しましょう
copy-onboarding-body = すべてのタスクはペーパーで開始します。対象の取引はプール価格、設定したスリッページ、手数料でシミュレートされ、エグジットルールはペーパー上のポジションに適用されます。ペーパーの結果が十分だったウォレットから、ウォレットごとにライブを有効化します。
copy-onboarding-add = 最初のウォレットを追加
copy-setup-gate-title = コピートレードにはウォレットが必要です
copy-onboarding-observe = 観察
copy-onboarding-observe-detail = { -sol } を使わずにウォレットのスワップを検出します。
copy-onboarding-evaluate = 評価
copy-onboarding-evaluate-detail = ペーパー損益、勝率、スキップ、検出速度、スリッページを確認します。
copy-onboarding-arm = 有効化
copy-onboarding-arm-detail = 準備チェックに合格したら、実際のスワップを有効にします。

## Wallet list

copy-list-label =
    .aria-label = コピー中のウォレット
copy-list-title = ウォレット
copy-list-compare = 比較
copy-list-sort-label = ウォレットを並べ替え
copy-list-count = 有効 { $active }件 · 合計 { $total }件
copy-sort-pnl = 損益
copy-sort-state = 状態
copy-sort-name = 名前
copy-compare-label =
    .aria-label = ウォレットを比較

## Dialog chrome

copy-dialog-close =
    .aria-label = 閉じる
copy-editor-title-add = ウォレットを追加
copy-editor-sub-add = 新しいタスクはペーパーで開始します
copy-arm-title = ライブコピーを有効化
copy-arm-sub = ご自身のウォレットからの実際のスワップ
copy-arm-keep-paper = ペーパーのまま
copy-arm-confirm = ライブを有効化
copy-profile-title = ウォレットプロファイル
copy-profile-sub = このボットが確認したウォレットの情報

## Settings dialog

copy-settings-title = コピートレード設定
copy-settings-subtitle = すべてのタスクに適用される全体ポリシー
copy-settings-filter-warning = デフォルトのフィルタリング設定では、ほぼすべてのトークンが除外されるため、何もコピーされません。ウォレットが取引するトークンをフィルターが通過する場合を除き、オフのままにしてください。
copy-settings-unit-seconds = 秒
copy-settings-unit-trades = 件
copy-settings-unit-tasks = 件
copy-settings-unit-rounds = 回
copy-settings-save = 設定を保存
copy-settings-load-failed = コピー設定を読み込めませんでした
copy-settings-saved = コピートレード設定を保存しました

## Workspace

copy-tab-overview = 概要
copy-tab-holdings = 保有
copy-tab-activity = アクティビティ
copy-tab-rules = ルール
copy-tab-execution = 実行
copy-tabs-label = タスクの表示
copy-workspace-select = ウォレットを選択すると、そのワークスペースを開きます。
copy-workspace-loading = タスクを読み込み中…
copy-workspace-load-failed = このタスクを読み込めませんでした: { $error }

copy-state-detail-paper = ペーパーで実行中 · 取引はシミュレートされ、資金は使用されません
copy-state-detail-live = ライブで実行中 · ウォレットの取引を実際のスワップでコピーします
copy-state-detail-system-paused = 待機中 · コピー処理は全体で一時停止中、エグジットは実行
copy-state-detail-entries-blocked = 損失上限によりエントリーをブロック中 · エグジットは実行
copy-state-detail-force-stopped = 強制停止 · コピーは行われません

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = 再開しても同じ上限が適用されるため、取引の到着が遅い間は再び一時停止します。RPC ストリームを確認するか、設定で到着の上限を引き上げてください。
copy-paused-resume-detached = 再開すると、ウォレットのウォッチを再開します。
copy-paused-holdings-rules =
    { $count ->
       *[other] エグジットルールにより、オープン中の保有 { $count }件は引き続きクローズされます。
    }
copy-paused-holdings-mirror =
    { $count ->
       *[other] ウォレットの売却により、オープン中の保有 { $count }件は引き続きクローズされます。
    }
copy-paused-holdings-hybrid =
    { $count ->
       *[other] ウォレットの売却とエグジットルールにより、オープン中の保有 { $count }件は引き続きクローズされます。
    }

copy-watch-state-catching-up = ウォレットウォッチ: 追跡中。{ -helius } 経由でこのウォレットをチェックしています。
copy-watch-state-watching = ウォレットウォッチ: ウォッチ中。{ -helius } 経由でこのウォレットをチェックしています。
copy-watch-last-check = 最終チェック: { $ago }。
copy-watch-recovery-active = ウォレットウォッチは有効です
copy-watch-recovery-catching-up = ウォレットウォッチは追いついている途中です
copy-watch-recovery-still-paused = コピータスクは一時停止中です。準備ができたらコピーを再開してください。
copy-watch-recovery-title = ウォレットウォッチを復旧
copy-watch-recovery-processing-failed = ウォレットの動きを処理できませんでした。保存済みの進捗は保持されています。問題の解決後に再試行してください。
copy-watch-recovery-provider-failed = { -helius } のチェックに失敗しました。保存済みの進捗は保持されています。プロバイダーが利用可能になったら再試行してください。
copy-watch-recovery-budget-intro = このウォレットのアクティビティは、現在のウォッチでチェックできる量を超えています。続行方法を選択してください。
copy-watch-approve = { -helius } を使って追いつく
copy-watch-approve-help = 保存済みの進捗から続行します。{ -helius } のクレジットをより多く消費する場合があり、それでも遅れる可能性があります。
copy-watch-approve-unavailable = { -helius } での追いつきは利用できません。未チェックのアクティビティをスキップせずに続行するには、有効な { -helius } RPC エンドポイントを設定してください。
copy-watch-no-provider = このウォッチに対応する追いつき用プロバイダーはありません。
copy-watch-budget-label = 1 回のチェックで確認するシグネチャ数
copy-watch-budget-hint = または未チェックのアクティビティをスキップして、現時点から再開します。1 回のチェックあたり { $min }～{ $max } シグネチャで指定してください。上限を上げると RPC 呼び出しが増える場合があります。
copy-watch-ack = 見逃したアクティビティはコピーされないことを理解しました。
copy-watch-toast-range = 1 回のポーリングあたりのシグネチャ数は { $min }～{ $max } の範囲で、{ $step } 刻みで指定してください
copy-watch-toast-ack = 最後に完了したチェック以降のシグネチャがスキップされることを確認してください
copy-watch-resumed = ウォレットウォッチを現時点から再開しました。コピータスクは一時停止のままです
copy-watch-resume-failed = ウォレットウォッチを再開できませんでした
copy-watch-retry-started = 保存済みの進捗からウォレットウォッチの再試行を開始しました。コピータスクは一時停止のままです
copy-watch-retry-failed = ウォレットウォッチを再試行できませんでした
copy-watch-approve-title = このウォレットで { -helius } による追いつきを許可
copy-watch-approve-message = { -helius } は、未チェックの区間をスキップせずに、保存済みの進捗から成功した Solana トランザクションをチェックできます。現在の料金は、返された完全なトランザクション 100 件あたり 10 クレジット（切り上げ）で、1 リクエストあたり最低 10 クレジットです。1 回のチェックで複数のリクエストが発生する場合があり、使用量とプロバイダーの料金は変動することがあります。コピーは、別途再開するまで一時停止のままです。
copy-watch-approve-confirm = このウォレットで許可
copy-watch-approved = 保存済みの進捗からウォレットウォッチを開始しました。コピータスクは一時停止のままです
copy-watch-restore-failed = ウォレットウォッチを復旧できませんでした

copy-action-pause = 一時停止
copy-action-resume = 再開
copy-action-resume-copy = コピーを再開
copy-action-resume-from-now = 現時点から再開
copy-action-retry-watch = ウォレットウォッチを再試行
copy-action-return-paper = ペーパーに戻す
copy-action-edit-rules = ルールを編集
copy-action-clone = 複製
copy-action-profile = ウォレットプロファイル
copy-resume-live-title = ライブコピーを再開
copy-resume-live-message = このウォレットが再び取引すると、「{ $name }」はご自身のウォレットから実際のスワップを送信します。
copy-resume-live-confirm = ライブを再開
copy-task-resumed = タスクを再開しました
copy-task-paused = タスクを一時停止しました
copy-task-state-failed = タスクの状態を変更できませんでした
copy-return-paper-message = 「{ $name }」による新しいコピーは、{ -sol } を使わずに再びシミュレートされます。
copy-return-paper-cancel = ライブのまま
copy-task-returned-paper = タスクをペーパーに戻しました
copy-mode-change-failed = 実行モードを変更できませんでした
copy-delete-title = コピータスクを削除
copy-delete-message = 「{ $name }」を削除しますか？判断履歴とペーパーの結果は削除され、このタスクではウォレットのウォッチが行われなくなります。
copy-delete-confirm = タスクを削除
copy-delete-cancel = タスクを残す
copy-task-deleted = コピータスクを削除しました
copy-task-delete-failed = コピータスクを削除できませんでした

## Overview tab

copy-overview-results = 結果
copy-analytics-load-failed = 分析を読み込めませんでした: { $error }
copy-analytics-loading = 分析を読み込み中…
copy-exit-bucket =
    { $count ->
       *[other] 売却 { $count }件 · { $pnl }
    }
copy-overview-average-win = 平均利益
copy-overview-average-loss = 平均損失 { $amount }
copy-overview-profit-factor = プロフィットファクター
copy-overview-profit-factor-note = 総利益 ÷ 総損失
copy-overview-average-hold = 平均保有時間
copy-overview-average-hold-note = エントリーからエグジットまで
copy-overview-best-round = 最良のラウンド
copy-overview-worst-round = 最悪のラウンド { $amount }
copy-overview-curve-title = 累積損益
copy-overview-exits-title = エグジット別の売却
copy-overview-skips-title = 取引がスキップされた理由
copy-book-title-live = ライブ帳簿
copy-book-title-paper = ペーパー帳簿
copy-book-all-time = 全期間
copy-book-buys =
    <strong>{ $count }</strong>{ $count ->
       *[other] 件の購入
    }
copy-book-policy-exits =
    <strong>{ $count }</strong>{ $count ->
       *[other] 件がルールによるエグジット
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong>{ $count ->
       *[other] 件のウォレット売却
    }
copy-book-manual-closes = <strong>{ $count }</strong>件を手動でクローズ
copy-book-skipped = <strong>{ $count }</strong>件をスキップ
copy-book-failed = <strong>{ $count }</strong>件が失敗
copy-book-closed = クローズ { $count }件
copy-book-budget-note = { $mode }の使用額 { $total } · 残り { $remaining }
copy-check-passed = 合格
copy-check-not-passed = 不合格
copy-readiness-title = ライブ開始前の確認
copy-readiness-live-note = このタスクはライブで取引しています。上部のヘッダーからペーパーに戻せます。
copy-readiness-all-pass = すべてのチェックに合格しました。
copy-readiness-needs-review = 有効化するには、準備ができていない項目を明示的に確認する必要があります。
copy-readiness-arm = 確認してライブを有効化

## Rules tab and review

copy-rules-title = 適用中のルール
copy-rules-size-ratio = ウォレットの取引の { $pct }
copy-rules-size-fixed = 1 回のコピーあたり { $amount }
copy-rules-target-any = 任意のサイズ
copy-rules-target-min = { $amount } 以上
copy-rules-target-max = { $amount } 以下
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = タスクで上書き · トレーダー { $value }
copy-rules-source-default = トレーダーのデフォルト
copy-rules-not-used = 使用しません: ウォレットの売却で判断します
copy-rules-col-rule = ルール
copy-rules-col-applies = 適用内容
copy-rules-col-source = 設定元
copy-rules-budget-note = { $mode }で { $spent } 使用 · 残り { $remaining }
copy-rules-token-copies =
    { $count ->
       *[other] 1 トークンあたり約 { $count } 回分のフルコピー
    }
copy-rules-sizing = サイジング
copy-rules-copy-size = コピーサイズ
copy-rules-entry-filters = エントリーフィルター
copy-rules-target-size = ウォレットの取引サイズ
copy-rules-repeat-buys = 繰り返しの購入
copy-rules-repeat-first-only = 各トークンの最初の購入のみ
copy-rules-repeat-every = すべての購入（トークンごとの上限まで）
copy-rules-filter-pass = フィルタリング通過
copy-rules-filter-required = 必須
copy-rules-filter-not-required = 必須ではない
copy-rules-filter-task-override = タスクで上書き
copy-rules-exits = エグジット
copy-rules-exits-inactive = 保有はウォレットが売却したときにのみ売却されます。このモードでは、以下のルールは実行されません。

## Exit rules

copy-rule-status = 状態
copy-rule-on = 有効
copy-rule-off = 無効
copy-rule-unit-seconds = 秒
copy-rule-unit-minutes = 分
copy-rule-stop-loss-threshold = 次の損失で売却
copy-rule-stop-loss-min-hold = 保有後この時間まで実行しない
copy-rule-no-minimum = 最小なし
copy-rule-partial-exits = 部分エグジット
copy-rule-partial-allowed = 許可
copy-rule-partial-full-only = 全量エグジットのみ
copy-rule-partial-size = 部分エグジットのサイズ
copy-rule-trailing-activation = 次の利益で有効化
copy-rule-trailing-distance = ピークからの下落幅で売却
copy-rule-take-profit-target = 次の利益で売却
copy-rule-time-duration = 保有後の確認タイミング
copy-rule-time-threshold = 損益が次の値以下で売却
copy-preset-inherit = トレーダーのデフォルト
copy-preset-conservative = 保守的
copy-preset-balanced = バランス
copy-preset-aggressive = 積極的
copy-preset-custom = カスタム
copy-validate-stop-loss = 損切りは 0% より大きく 100% 以下にしてください。
copy-validate-partial-size = 部分エグジットのサイズは 0% から 100% の間にしてください。
copy-validate-min-hold = 最小保有時間は秒単位の整数にしてください。
copy-validate-trailing-activation = トレーリングの有効化は 0% より大きく 100% 以下にしてください。
copy-validate-trailing-distance = トレーリングの下落幅は 0% より大きく 100% 以下にしてください。
copy-validate-take-profit = 利確は 0% より大きくしてください。
copy-validate-time-duration = 時間ルールには 0 より大きい期間が必要です。
copy-validate-time-threshold = 時間ルールのしきい値は損失です。0% または負の数を指定してください。
copy-warning-mirror = 保有をクローズするのはウォレットの売却のみです。損切りによる保護はなく、ウォレットが売却しないトークンは保有され続けます。
copy-warning-no-rules = エグジットルールが無効で、ウォレットの売却も無視されます。保有は売却されません。
copy-warning-no-stop-loss = 損切りが適用されません。下落するトークンは、別のルールが働くかウォレットが売却するまで保有されます。
copy-warning-stop-delay = 損切りは各購入の後 { $hold } 待機します。それより速く下落したトークンは、{ $threshold } を大きく超えてクローズされます。
copy-warning-take-profit-cost = { $target } での利確は、売却コスト（スリッページ { $slippage } とスワップ手数料 { $fee }）を賄えないため、損失でラウンドがクローズされます。
copy-warning-trailing-distance = トレーリングの下落幅が有効化の利益以上のため、有効化されたトレールがエントリー価格を下回って売却する可能性があります。

## Execution tab

copy-execution-title = 実行品質
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = 任意
copy-execution-limit-on =
    { $count ->
       *[other] 直近 { $count }件の取引の平均が { $limit } を超えると一時停止
    }
copy-execution-limit-off = キルスイッチ無効
copy-execution-arrival-samples =
    { $count ->
       *[other] 発生時に確認した取引 { $count }件
    }
copy-execution-p95 = p95 到着
copy-execution-median-slippage = スリッページの中央値
copy-execution-slippage-samples =
    { $count ->
       *[other] 計測した約定 { $count }件
    }
copy-execution-worst-slippage = 最悪のスリッページ
copy-execution-average-slippage = 平均 { $amount }
copy-execution-delay-title = 検出の遅延
copy-execution-delay-note = ウォレットのブロックから、このボットが取引を検出するまでの時間です。停止期間後の再生は含まれません。
copy-execution-delay-limit = 到着の上限 { $limit } を超えたバーはアンバーで表示されます。
copy-execution-fastest = 最速
copy-execution-average = 平均
copy-execution-slowest = 最遅
copy-execution-fill-title = ウォレットとの約定差
copy-execution-fill-note = 正の値はウォレットより不利であることを示します。購入では高く支払い、ミラーした売却では受け取りが少ない場合です。プール価格のないトークンのペーパー約定はウォレット自身の取引価格で評価されるため、何も計測できず、対象から除外されます。
copy-execution-samples = サンプル
copy-execution-median = 中央値
copy-execution-worst = 最悪
copy-execution-decisions = 期間内の判断

## Compare view

copy-compare-title = ウォレットを比較
copy-compare-back = ウォレットに戻る
copy-compare-load-failed = 比較を読み込めませんでした: { $error }
copy-compare-loading = 比較を読み込み中…
copy-compare-empty = 比較するタスクがありません。
copy-compare-empty-message = コピータスクを追加すると、他のタスクと結果を比較できます。
copy-compare-curve-title = 累積確定損益
copy-table-wallet = ウォレット
copy-table-mode = モード
copy-table-rounds = ラウンド
copy-table-realized = 確定（{ -sol }）
copy-table-profit-factor = プロフィットファクター
copy-table-average-hold = 平均保有時間
copy-table-median-slippage = スリッページ中央値

## Charts

copy-chart-curve-label = 累積損益 { $amount } { -sol }
copy-chart-compare-label = タスク別の累積損益
copy-chart-empty-curve = この期間にクローズ済みのラウンドはまだありません。
copy-chart-empty-bars = この期間に記録はありません。
copy-chart-empty-histogram = この期間に到着サンプルはありません。
copy-chart-empty-compare = この期間に比較できるクローズ済みラウンドはありません。
copy-chart-histogram-title = { $count }/{ $total }

## Wallet profile

copy-profile-copy = このウォレットをコピー
copy-profile-copy-other = 別のルールでコピー
copy-profile-loading = ウォレットプロファイルを読み込み中…
copy-profile-watch-title = ウォッチ
copy-profile-watched = ウォッチ中
copy-profile-watch-resume-hint = タスクを再開すると、再びウォッチします
copy-profile-watch-add-hint = タスクを追加すると、ウォッチを開始します
copy-profile-stream = ストリーム
copy-profile-subscribed = 購読中
copy-profile-not-subscribed = 未購読
copy-profile-sources =
    { $count ->
       *[other] ソース { $count }件
    }
copy-profile-last-activity = 最終アクティビティ
copy-profile-last-error = 最後のエラー
copy-profile-own-wallet = これはご自身のウォレットの 1 つです。コピーは拒否されます。
copy-profile-observed-title = 観測した取引
copy-profile-observed-none = このボットでは、このウォレットの取引はまだありません。ペーパータスクなら { -sol } を使わずに観察できます。
copy-profile-swaps-seen = 確認したスワップ
copy-profile-swaps-seen-note = タスク全体での重複を除いたウォレットのスワップ数
copy-profile-buys-sells = 購入 / 売却
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = 取引したトークン
copy-profile-first-seen = 初回確認
copy-profile-last-seen = 最終確認
copy-profile-tasks-title = このウォレットに対するタスク
copy-table-task = タスク

## Arm live dialog

copy-arm-acks-left =
    { $count ->
       *[other] チェックが必要な確認事項が残り { $count }件
    }
copy-arm-readiness-title = ペーパー帳簿による準備状況
copy-arm-exposure-title = エクスポージャー
copy-arm-per-copy = 1 回のコピーあたり
copy-arm-budget-left-value = { $left } / { $total } { -sol }
copy-arm-budget-left = ライブ予算の残り
copy-arm-budget-left-note = ペーパーの使用額は別に集計され、この予算には含まれません
copy-arm-exits = エグジット
copy-arm-stop-note = { $hold } の保有前は実行しません。それより速い下落では、より低い価格でクローズされます
copy-arm-shared = このウォレットは { $tasks } でもコピーされています。各タスクは、それぞれの予算で取引をコピーします。
copy-arm-unavailable = 現在ライブ実行は利用できません。最後のチェックを確認してください。
copy-arm-ack-real-native = 実際の { -sol }: このタスクはウォレットから最大 { $budget } { -sol } を使用でき、1 回のコピーあたり最大 { $trade } { -sol } です。
copy-arm-ack-fees = ライブコピーでは実際のネットワーク手数料とスリッページが発生します。ペーパーの結果はライブの結果を保証しません。
copy-arm-ack-unready = 一部の準備チェックに合格していません。それでもこのタスクを有効化します。
copy-arm-lead = 「{ $name }」は、ご自身のウォレットからの実際のスワップでこのウォレットの取引をコピーします。
copy-arm-confirmation-missing = ライブの確認情報を読み込めませんでした
copy-arm-armed = ライブコピーを有効化しました
copy-arm-failed = ライブコピーを有効化できませんでした

## Holdings tab

copy-holdings-title = 保有
copy-holdings-view-label = 保有の表示
copy-holdings-view-open = オープン（{ $count }）
copy-holdings-view-closed = クローズ済みラウンド（{ $count }）
copy-holdings-reset = ペーパー帳簿をリセット
copy-holdings-live-note = ライブコピーは実際のポジションです。
copy-holdings-open-positions = オープンポジション
copy-holdings-token-details = トークンの詳細を開く
copy-holdings-opened = { $time } にオープン
copy-holdings-no-pool-price = プール価格なし
copy-holdings-close = クローズ
copy-holdings-write-off = 償却
copy-holdings-activity = アクティビティ
copy-holdings-no-exit-rule = エグジットルールなし
copy-holdings-watch-stop = 損切り { $level }
copy-holdings-watch-stop-until = { $span }後に損切り { $level }
copy-holdings-watch-take = 利確 { $level }
copy-holdings-watch-trail = トレール { $level }
copy-holdings-watch-trail-arms = トレール有効化 { $level }
copy-holdings-watch-time = 時間 ≤ { $level }
copy-holdings-watch-time-until = { $span }後に時間 ≤ { $level }
copy-holdings-watch-wallet-sells = ウォレットの売却
copy-holdings-empty = オープン中のペーパー保有はありません。ウォレットからコピーした購入がここに表示されます。
copy-holdings-col-token = トークン
copy-holdings-col-cost = 取得原価（{ -sol }）
copy-holdings-col-entry = エントリー（{ -sol }）
copy-holdings-col-mark = 評価価格（{ -sol }）
copy-holdings-col-peak = ピーク
copy-holdings-col-pnl = 損益（{ -sol }）
copy-holdings-col-exit-rules = エグジットルール
copy-holdings-col-held = 保有時間
copy-holdings-col-actions = 操作
copy-holdings-col-invested = 投資額（{ -sol }）
copy-holdings-col-proceeds = 売却代金（{ -sol }）
copy-holdings-col-exit = エグジット
copy-holdings-col-closed = クローズ
copy-holdings-price-note = 価格は 1 トークンあたりの { -sol } です。エントリーには購入時のスリッページと手数料が含まれます。ピークとエグジット水準はこれを基準とするため、保有はピークがエントリーを下回った状態で始まります。カーソルを合わせるとプール価格を確認できます。
copy-holdings-paused-rules = 一時停止中: 新規コピーは行われません。エグジットルールにより、これらの保有は引き続きクローズされます。
copy-holdings-paused-mirror = 一時停止中: 新規コピーは行われません。ウォレットの売却により、これらの保有は引き続きクローズされます。
copy-holdings-paused-hybrid = 一時停止中: 新規コピーは行われません。ウォレットの売却とエグジットルールにより、これらの保有は引き続きクローズされます。
copy-holdings-closed-load-failed = クローズ済みラウンドを読み込めませんでした: { $error }
copy-holdings-closed-loading = クローズ済みラウンドを読み込み中…
copy-holdings-closed-empty = クローズ済みラウンドはまだありません。
copy-holdings-closed-latest = 全 { $total }ラウンドのうち直近 { $shown }件。
copy-holdings-close-title = ペーパー保有をクローズ
copy-holdings-close-message = ペーパー帳簿で { $token } を、プール価格（{ $price }）とタスクのスリッページ・手数料で売却します。
copy-holdings-close-confirm = 保有をクローズ
copy-holdings-write-off-title = ペーパー保有を償却
copy-holdings-write-off-message = { $token } には売却できるプール価格がありません。償却すると 0 でクローズされ、取得原価 { $cost } が損失として計上されます。
copy-holdings-keep = 残す
copy-holdings-written-off = { $token } を償却しました
copy-holdings-closed = { $token } をクローズしました
copy-holdings-written-off-detail = 売却代金 0 でクローズしました
copy-holdings-sold-at = { $price } で売却
copy-holdings-close-failed = 保有をクローズできませんでした
copy-holdings-reset-message = 「{ $name }」をやり直します。ペーパー保有、使用額、約定、エグジット、スキップは削除されます。ルールとウォレットは残ります。
copy-holdings-reset-cancel = 履歴を残す
copy-holdings-reset-done = ペーパー帳簿をリセットしました
copy-holdings-reset-detail =
    { $count ->
       *[other] 判断 { $count }件を削除しました
    }
copy-holdings-reset-failed = ペーパー帳簿をリセットできませんでした

## Activity tab

copy-activity-title = アクティビティ
copy-activity-filter-label = アクティビティのフィルター
copy-filter-all = すべて
copy-outcome-paper-filled = ペーパー購入
copy-outcome-live-submitted = ライブ購入を送信
copy-outcome-live-confirmed = ライブ購入を確認
copy-outcome-live-failed = ライブ購入に失敗
copy-outcome-paper-sell-observed = ペーパー売却 · ウォレットが売却
copy-outcome-live-sell-submitted = ライブ売却を送信
copy-outcome-live-sell-failed = ライブ売却に失敗
copy-outcome-skipped = スキップ
copy-activity-decision = 判断
copy-activity-paper-exit = ペーパーエグジット · { $rule }
copy-activity-filled = { $input }（{ $price }）· ウォレットの購入 { $target }
copy-activity-filled-slippage = { $input }（{ $price }）· ウォレットの購入 { $target } · スリッページ { $slippage }
copy-activity-filled-unpriced = { $input }（{ $price }）· ウォレットの購入 { $target } · ウォレットの取引価格で評価、プール価格なし
copy-activity-live-sized = { $sized } · ウォレットの購入 { $target }
copy-activity-sell-nothing = ウォレットが { $amount } を売却 · 売却できる保有なし
copy-activity-written-off = 0 で償却: プール価格なし
copy-activity-sold = { $tokens } トークンを { $proceeds } で売却（{ $price }）
copy-activity-full-close = 全量クローズ
copy-activity-partial-exit = { $pct } エグジット
copy-activity-skip-detail = { $label }（{ $detail }）
copy-activity-skip-minimum-size = 最小 { $amount }
copy-activity-skip-maximum = 最大 { $value }
copy-activity-skip-stale = { $arrival } 遅延、上限 { $limit }
copy-activity-skip-latency = 平均 { $average }、上限 { $limit }
copy-activity-arrival-replayed = ブロックの { $span }後に再生
copy-activity-arrival-seen = ブロックの { $span }後に検出
copy-activity-link-wallet-tx = ウォレットの Tx
copy-activity-link-own-tx = 自分の Tx
copy-activity-only-token = このトークンのみ
copy-activity-skipped-group = スキップ ×{ $count }
copy-activity-group-detail =
    { $tokens ->
       *[other] { $tokens }トークン · { $since } 以降
    }
copy-activity-mint-filter =
    .placeholder = トークンのミント
    .aria-label = トークンのミントで絞り込み
copy-activity-clear = クリア
copy-activity-load-failed = アクティビティを読み込めませんでした: { $error }
copy-activity-loading = アクティビティを読み込み中…
copy-activity-no-match = このフィルターに一致するものはありません。
copy-activity-empty = 判断はまだありません。ウォレットが取引すると、約定、エグジット、スキップがここに表示されます。
copy-activity-load-older = さらに古いものを読み込む
copy-activity-start = 履歴の先頭
copy-activity-older-failed = 古いアクティビティを読み込めませんでした

## Task editor

copy-step-wallet = ウォレット
copy-step-sizing = サイジング
copy-step-entry = エントリーフィルター
copy-step-exits = エグジット
copy-step-review = 確認
copy-editor-title-edit = { $name } を編集
copy-editor-title-clone = { $name } を複製
copy-editor-sub-edit = { $mode }タスク · 変更は次の判断から適用されます
copy-editor-sub-clone = 同じルールで、ペーパー帳簿は空、ペーパーで開始します
copy-editor-save-edit = 変更を保存
copy-editor-save-clone = 複製を作成
copy-editor-save-create = ペーパータスクを作成
copy-editor-clone-suffix = （コピー）
copy-editor-discard-edit = 変更を破棄
copy-editor-discard-create = このタスクを破棄
copy-editor-discard-edit-message = 「{ $name }」への変更は保存されていません。
copy-editor-discard-create-message = ここまでに入力したウォレットとルールは保存されていません。
copy-editor-discard-confirm = 破棄
copy-editor-keep-editing = 編集を続ける
copy-editor-toast-updated = タスクを更新しました
copy-editor-toast-clone = 複製を作成しました
copy-editor-toast-created = ペーパータスクを作成しました
copy-unit-native = { -sol }
copy-editor-any = 任意
copy-editor-duplicate = { $tasks } ですでにコピー済みです。このタスクは、独自のルールと予算で同じ取引を再度コピーします。
copy-editor-wallet = ウォレット
copy-editor-wallet-identity = タスクのウォレットがそのタスクの識別子です。別のウォレットをこのルールでコピーするには、タスクを複製してください。
copy-editor-address-label = ウォレットアドレス
copy-editor-address-placeholder = Solana ウォレットアドレス
copy-editor-address-help-clone = 同じルールで、ペーパー帳簿は空です。このウォレットで別のルールを試すか、別のウォレットを入力してください。
copy-editor-address-help-create = このタスクが購入（選択した場合は売却も）をコピーするウォレットです。
copy-editor-name-label = 名前 <em>任意</em>
copy-editor-name-placeholder = 例: 高速ローテーター
copy-editor-enabled-title = ウォレットの取引を処理
copy-editor-enabled-help = 無効にすると、再開するまでタスクは一時停止のままです。
copy-editor-note-live = このタスクはライブです。変更は次の実際のコピーから適用されます。
copy-editor-note-paper = タスクは有効化するまでペーパーで実行されます。取引はプール価格でシミュレートされ、資金は使用されません。
copy-editor-copy-size = コピーサイズ
copy-editor-sizing-fixed = 固定額
copy-editor-sizing-ratio = ウォレットの取引に対する割合
copy-editor-amount-fixed = 1 回のコピーあたりの額
copy-editor-amount-ratio = 各取引に対する割合
copy-editor-amount-help-fixed = コピーする各購入で使用する額です。最小 { $minimum }。
copy-editor-amount-help-ratio = ウォレット自身の購入額に対する割合で、取引ごとの上限までです。
copy-editor-help-trade-cap = 1 回のコピーでこれを超えて使用しません。
copy-editor-help-token-cap = 1 つのトークンに使用する合計額です。
copy-editor-help-budget = このタスクが存続期間中に使用できる総額です。ペーパーとライブはそれぞれの使用額を集計します。
copy-editor-preview-title = コピーのコスト
copy-editor-preview-empty = サイジングを入力すると、コピーのコストを確認できます。
copy-editor-preview-example = ウォレットが { $target } を購入 → <strong>{ $copy }</strong> をコピー
copy-editor-preview-once = 各トークンは 1 回だけ購入するため、1 トークンあたり { $size } の 1 回のコピーのみです
copy-editor-preview-token-cap =
    { $count ->
       *[other] 1 トークンあたり最大 { $count } 回、{ $size } ずつコピーします
    }
copy-editor-preview-summary-exact = { $perToken }。予算で約 { $count } 回分をまかなえます。ネットワーク手数料と優先手数料は別途かかります。
copy-editor-preview-summary-minimum = { $perToken }。予算で少なくとも { $count } 回分をまかなえます。ネットワーク手数料と優先手数料は別途かかります。
copy-editor-target-min = コピーするウォレット取引の最小額
copy-editor-target-min-help = ウォレットの小さな購入を無視します。空欄で最小なしになります。
copy-editor-target-max = コピーするウォレット取引の最大額
copy-editor-target-max-help = ウォレットの大きな購入を無視します。空欄で最大なしになります。
copy-editor-buy-once-title = 各トークンを 1 回だけ購入
copy-editor-buy-once-help = ウォレットのトークンの最初の購入のみをコピーし、以降の購入はスキップします。
copy-editor-filter-require = 必須にする
copy-editor-filter-skip = 必須にしない
copy-editor-filter-help = トークンをコピーする前に、フィルタリングパイプラインの通過を必須にします。
copy-editor-filter-warning = デフォルトのフィルタリング設定では、ほぼすべてのトークンが不合格になるため、通過を必須にしたタスクは何もコピーしません。ウォレットが取引するトークンをフィルターが通過する場合にのみ、必須にしてください。
copy-editor-exit-both = 両方
copy-editor-exit-help-buy-only = 以下のルールですべての保有を売却します。ウォレットの売却は無視されます。
copy-editor-exit-help-hybrid = ウォレットが売却するか、ルールが発動するか、先に起きた方で売却します。
copy-editor-exit-help-mirror = 保有はウォレットが売却したときにのみ売却されます。エグジットルールは実行されません。
copy-editor-who-sells = 売却の判断
copy-editor-preset = プリセット
copy-editor-preset-help = プリセットは以下のすべてのルールを入力します。その後、個別に調整できます。
copy-editor-mirror-note = ウォレットの売却で判断している間、これらのルールは実行されません。「{ $mine }」または「{ $both }」に切り替えると適用されます。
copy-editor-rule-inherit = トレーダーのデフォルト
copy-editor-inherit-value = トレーダーのデフォルト（{ $value }）
copy-editor-rule-aria = { $rule }の設定
copy-editor-rule-empty-uses = 空欄の場合はトレーダーのデフォルトを使用します: { $value }
copy-editor-rule-follows = トレーダーに従います: { $summary }
copy-editor-rule-follows-plain = トレーダーの設定に従います。
copy-editor-rule-follows-own = オン/オフはトレーダーに従い、値はこのタスクのもの: { $summary }
copy-editor-rule-off-note = トレーダーの設定に関わらず、このタスクでは無効です。
copy-task-unnamed = 名前のないタスク
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = 保存後に取引を処理します
copy-editor-review-paused = 一時停止で保存します
copy-editor-error-address = 有効な Solana ウォレットアドレスを入力してください。
copy-editor-error-sizing = すべてのサイジング値は 0 より大きくしてください。
copy-editor-error-min-copy = 1 回のコピーは少なくとも { $minimum } 必要です。1 回のコピーあたりの額を増やしてください。
copy-editor-error-min-cap = 1 回のコピーは少なくとも { $minimum } 必要です。取引ごとの上限を引き上げてください。
copy-editor-error-trade-cap = 取引ごとの上限は、トークンごとの上限を超えられません。
copy-editor-error-token-cap = トークンごとの上限は、総予算を超えられません。
copy-editor-error-slippage = スリッページは { $min } から { $max } の間にしてください。
copy-editor-error-target-limits = ウォレット取引の制限は 0 以上にしてください。
copy-editor-error-target-order = ウォレット取引の最小額は、最大額を超えられません。

## Copy notices

copy-notice-task-unnamed = タスク #{ $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = ペーパーコピー購入
copy-notice-title-paper-sell = ペーパーコピー売却
copy-notice-title-paper-closed = ペーパー保有をクローズしました
copy-notice-title-paper-exit = ペーパーエグジット: { $rule }
copy-notice-title-live-buy-submitted = ライブコピー購入を送信しました
copy-notice-title-live-buy-confirmed = ライブコピー購入を確認しました
copy-notice-title-live-buy-failed = ライブコピー購入に失敗しました
copy-notice-title-live-sell-submitted = ライブコピー売却を送信しました
copy-notice-title-live-sell-failed = ライブコピー売却に失敗しました
copy-notice-title-auto-paused = コピータスクを自動一時停止しました
copy-notice-detail-bought = { $amount } { -sol } で購入
copy-notice-detail-sold = { $amount } { -sol } で売却
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = 保有の { $percent }%
copy-notice-detail-full-close = 全量クローズ
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = スワップに失敗しました
