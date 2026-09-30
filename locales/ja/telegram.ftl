# Telegram bot text. Server-only: rendered by src/telegram/text.rs, never sent to the dashboard.
#
# Messages are sent as Telegram HTML. The only tags are b, i, u, s, code and pre,
# without attributes; links are built in Rust. A line break is a literal newline.
# Icons are prepended by Rust and never appear here. Copyable values (chat ids)
# arrive as arguments and are wrapped in code inside the message. Keep the
# command names (/status) and the literal ampersand placeable unchanged.

## Reply keyboard words. Icons come from ReplyCommand in src/telegram/reply.rs.

telegram-reply-status = ステータス
telegram-reply-balance = 残高
telegram-reply-positions = ポジション
telegram-reply-pause = 一時停止
telegram-reply-resume = 再開
telegram-reply-stop = 停止
telegram-reply-stats = 統計
telegram-reply-menu = メニュー
telegram-reply-help = ヘルプ

## Inline keyboard buttons.

telegram-button-positions = ポジション
telegram-button-balance = 残高
telegram-button-stats = 統計
telegram-button-tokens = トークン
telegram-button-pause = 一時停止
telegram-button-stop = 停止
telegram-button-settings = 設定
telegram-button-refresh = 更新
telegram-button-menu = メニュー
telegram-button-back = 戻る
telegram-button-back-to-menu = メニューに戻る
telegram-button-back-to-tokens = トークンに戻る
telegram-button-cancel = キャンセル
telegram-button-close-all-positions = 全ポジションをクローズ
telegram-button-sell-percent = { $percent }% 売却
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = ブラックリスト
telegram-button-blacklist-symbol = { $symbol } をブラックリスト
telegram-button-close-position = ポジションをクローズ
telegram-button-confirm-close = クローズを確定
telegram-button-confirm-close-all = 全ポジションをクローズ
telegram-button-confirm-sell = { $percent }% の売却を確定
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = 強制停止を確定
telegram-button-confirm-buy = { $amount } { -sol } を購入
telegram-button-notifications = 通知
telegram-button-trading = 取引
telegram-button-entry-monitor = エントリーモニター
telegram-button-exit-monitor = エグジットモニター
telegram-button-auto-trading = 自動取引
telegram-button-force-stop = 強制停止
telegram-button-notify-opened = オープン
telegram-button-notify-closed = クローズ
telegram-button-notify-partial = 部分
telegram-button-notify-dca = DCA
telegram-button-notify-errors = エラー
telegram-button-details = 詳細
telegram-button-position = ポジション
telegram-button-sell-more = 追加売却
telegram-button-more-dca = DCA を追加
telegram-button-history = 履歴
telegram-button-status = ステータス
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = 再認証
telegram-button-previous = 前へ
telegram-button-next = 次へ
telegram-button-passed = 通過
telegram-button-rejected = 除外
telegram-button-new-24h = 新規（24h）
telegram-button-all-tokens = すべてのトークン
telegram-button-search-token = トークンを検索
telegram-button-filter-stats = フィルター統計
telegram-button-refresh-stats = 統計を更新
telegram-button-view-position = ポジションを表示
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    不明なコマンドです: { $command }

    利用できるコマンドは /help で確認できます。
telegram-session-expired =
    <b>セッションの有効期限が切れました</b>

    /login で再認証してください。
telegram-2fa-required =
    <b>2FA が必要です</b>

    認証アプリの6桁のコードを入力してください。
telegram-account-locked =
    <b>アカウントがロックされました</b>

    失敗が多すぎます。
    { $seconds ->
       *[other] { $seconds }秒後にもう一度お試しください。
    }
telegram-code-invalid = 有効な6桁のコードを入力してください。
telegram-authenticated =
    <b>認証しました</b>

    ボットのコマンドを使用できます。
telegram-wrong-code =
    <b>コードが違います</b>

    { $remaining ->
       *[other] 残り{ $remaining }回試行できます。
    }
telegram-auth-required =
    <b>認証が必要です</b>

    続行するにはパスワードを入力してください。

    <i>パスワードを入力して送信してください。</i>
telegram-login-required =
    <b>ログインが必要です</b>

    認証アプリの6桁のコードを入力してください:
telegram-session-activated =
    <b>セッションを有効化しました</b>

    2FA は設定されていません。セッションは有効になりました。

    <i>ヒント: セキュリティ設定で 2FA を有効にすると、より安全になります。</i>

## Chat discovery.

telegram-discovery-hello = こんにちは、{ $name } さん！
telegram-discovery-default-name = ユーザー
telegram-discovery-detected = <b>チャットを検出しました</b>
telegram-discovery-details =
    チャット ID: <code>{ $chat_id }</code>
    種類: { $chat_type }

    { -brand } のダッシュボードで、このチャットをクリックして選択してください。
telegram-chat-type-private = プライベート
telegram-chat-type-group = グループ
telegram-chat-type-supergroup = スーパーグループ
telegram-chat-type-channel = チャンネル

## Menus.

telegram-menu-title =
    <b>コントロールパネル</b>

    オプションを選んで、情報の確認やボットの操作ができます。
telegram-menu-positions-empty =
    <b>オープンポジションなし</b>

    新しい機会を待っています...
telegram-menu-positions-title = <b>ポジション（{ $count }）</b>
telegram-menu-positions-hint = <i>ポジションをタップして管理できます。</i>
telegram-menu-settings =
    <b>設定</b>

    通知と取引パラメータを設定します。
telegram-settings-notifications =
    <b>通知設定</b>

    通知のオン / オフを切り替えます:
telegram-settings-trading =
    <b>取引コントロール</b>

    取引機能を切り替えます:
telegram-pagination-expired = ページ送りのセッションの有効期限が切れました。

## Status commands.

telegram-status-state-stopped = <b>停止中</b>（強制停止が有効）
telegram-status-state-active = <b>稼働中</b>
telegram-status-state-paused = <b>一時停止中</b>
telegram-status-on = 有効
telegram-status-off = 無効
telegram-status-body =
    <b>システムステータス</b>

    <b>システム</b>
    状態 — { $state }
    稼働時間 — { $uptime }
    バージョン — v{ $version }

    <b>取引</b>
    エントリー — { $entries }
    エグジット — { $exits }
    ポジション — { $positions }
telegram-positions-empty =
    <b>オープンポジションなし</b>

    機会を待っています...
telegram-positions-title = <b>オープンポジション（{ $count }）</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol }（{ $pnl_pct }%）
telegram-positions-more = <i>ほか{ $count }件...</i>
telegram-positions-summary =
    <b>ポートフォリオ概要</b>
    投資額 — { $invested } { -sol }
    純 P{ "&amp;" }L — { $pnl } { -sol }
telegram-balance-body =
    <b>ウォレット残高</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>日次統計</b>

    ポジション — { $positions }
    投資額 — { $invested } { -sol }
    P{ "&amp;" }L — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } の準備ができました</b>

    取引は<b>有効</b>です。

    下のキーボードでボットを操作できます。
    利用できるコマンドは /help で確認できます。
telegram-stop-already = <b>取引はすでに無効です</b>
telegram-stop-done =
    <b>取引を無効にしました</b>

    すべての取引モニター（エントリーとエグジット）を停止しました。
    エントリーだけを止めるには /pause を使います。
telegram-stop-failed =
    <b>取引を無効にできませんでした</b>

    エラー: { $detail }
telegram-pause-done =
    <b>エントリーモニターを一時停止しました</b>

    新しいポジションは開かれません。
    エグジットモニターは動作を続けます。
telegram-pause-failed =
    <b>エントリーを一時停止できませんでした</b>

    エラー: { $detail }
telegram-resume-done =
    <b>エントリーモニターを再開しました</b>

    エントリーシグナルの監視を再開しました。
telegram-resume-failed =
    <b>エントリーを再開できませんでした</b>

    エラー: { $detail }
telegram-force-stop-confirm =
    <b>強制停止</b>

    すべての取引活動を直ちに停止します:
    • 新規エントリーなし
    • エグジットなし（損切りを含む）
    • DCA なし
telegram-force-stop-warning = <b>これは緊急操作です。</b>
telegram-force-stop-question = 実行してよろしいですか？
telegram-force-stop-active =
    <b>強制停止を有効にしました</b>

    すべての取引を停止しました。

    /resume_trading でこのフラグを解除できます。
telegram-resume-trading-not-stopped =
    <b>取引は強制停止されていません</b>

    操作は不要です。
telegram-resume-trading-done =
    <b>取引を再開しました</b>

    強制停止フラグを解除しました。
    通常の取引操作を再開できます。

## Help.

telegram-help-title = <b>{ -brand } ヘルプ</b>
telegram-help-heading-dashboard = ダッシュボード
telegram-help-heading-market = マーケット
telegram-help-heading-trading = 取引
telegram-help-heading-safety = 安全
telegram-help-heading-system = システム
telegram-help-commands-dashboard =
    /status — システムステータスと稼働時間
    /stats — 日次パフォーマンス
    /balance — ウォレット残高
    /positions — オープンポジション
telegram-help-commands-market =
    /tokens — トークンエクスプローラー
    /rejected — フィルタリングされたトークン
telegram-help-commands-trading =
    /start — 取引システムを有効化
    /stop — 取引システムを無効化
    /pause — 新規エントリーを一時停止
    /resume — 新規エントリーを再開
    /menu — インタラクティブメニュー
telegram-help-commands-safety =
    /force_stop — <b>緊急停止</b>
    /resume_trading — 緊急状態を解除
telegram-help-commands-system =
    /update — アップデートの状態とインストール
    /login — 2FA 認証
telegram-help-tip = <i>ヒント: コマンドをタップすると実行できます。</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>最新の状態です</b>

    v{ $version } で動作中です。自動的にインストールされました。
telegram-update-up-to-date =
    <b>最新の状態です</b>

    v{ $version } で動作中です。
telegram-update-check-failed =
    <b>アップデートの確認に失敗しました</b>

    { $reason }
telegram-update-unreachable = screenerbot.io に接続できませんでした。
telegram-update-installing = <b>v{ $version } をインストール中</b>
telegram-update-restarting =
    { -brand } は新しいバージョンで再起動します。取引は自動的に再開されます。
telegram-update-install-failed =
    <b>v{ $version } をインストールできませんでした</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } をダウンロードしました</b>

    このリリースにはデスクトップアプリの更新も含まれるため、そのマシンでインストーラーを実行する必要があります。マシン上で 設定 → アップデート を開いてください。
telegram-update-downloading =
    <b>v{ $version } をダウンロード中</b>

    { $size } MB 中 { $percent }%。
telegram-update-available =
    <b>v{ $version } が利用可能です</b>

    { $how }
    ダウンロードサイズ: { $size } MB。

    自動でダウンロードされます。準備ができたらもう一度 /update を送信してください。
telegram-update-how-core = 短い再起動でサイレントにインストールされます。
telegram-update-how-installer = デスクトップのインストーラーを1回実行する必要があります。

## Shared values and units. Numbers arrive formatted; only the unit words live here.

telegram-value-unknown = 不明
telegram-value-na = 該当なし
telegram-percent-value = { $percent }%
telegram-price-sol = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds }秒
telegram-duration-minutes = { $minutes }分
telegram-duration-minutes-seconds = { $minutes }分{ $seconds }秒
telegram-duration-hours = { $hours }時間
telegram-duration-hours-minutes = { $hours }時間{ $minutes }分
telegram-duration-days = { $days }日
telegram-duration-days-hours = { $days }日{ $hours }時間
telegram-pnl = { $sol } { -sol }（{ $percent }%）
telegram-amount-sol = { $amount } { -sol }
telegram-error-line = エラー: { $detail }
telegram-ai-reasoning =
    <b>LLM 分析</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens. Icons come from Rust.

telegram-row-entry = エントリー — { $price } { -sol }
telegram-row-exit = エグジット — { $price } { -sol }
telegram-row-current = 現在 — { $price } { -sol }
telegram-row-invested = 投資額 — { $amount } { -sol }
telegram-row-received = 受取額 — { $amount } { -sol }
telegram-row-value = 評価額 — { $amount } { -sol }
telegram-row-total = 合計 — { $amount } { -sol }
telegram-row-tokens = トークン — { $tokens }
telegram-row-duration = 期間 — { $duration }
telegram-row-reason = 理由 — { $reason }
telegram-row-remaining = 残り — { $percent }%
telegram-row-pnl = P{ "&amp;" }L — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>ポジションをオープンしました</b>
telegram-notify-opened-size = サイズ — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = 価格 — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>ポジションをクローズしました</b> — 利益
telegram-notify-closed-title-loss = <b>ポジションをクローズしました</b> — 損失
telegram-notify-closed-reason-unspecified = クローズ
telegram-notify-partial-title = <b>部分エグジット</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — { $percent }% 売却
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = 追加 — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = 平均 — { $price } { -sol }
telegram-notify-severity-critical = <b>重大なエラー</b>
telegram-notify-severity-error = <b>エラー</b>
telegram-notify-severity-warning = <b>警告</b>
telegram-notify-severity-info = <b>情報</b>
telegram-notify-alert-title = <b>取引アラート</b>
telegram-notify-alert-token = トークン: <code>${ $symbol }</code>
telegram-notify-alert-mint = ミント: <code>{ $mint }</code>
telegram-notify-alert-bought = アクション: { $amount } { -sol } を購入
telegram-notify-alert-sold = アクション: { $amount } { -sol } を売却
telegram-notify-alert-wallet = ウォレット: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b>（ペーパー）
telegram-notify-copy-task = タスク: { $task }
telegram-notify-scheduled-completed = <b>スケジュールタスクが完了しました</b>
telegram-notify-scheduled-failed = <b>スケジュールタスクが失敗しました</b>
telegram-notify-scheduled-timed-out = <b>スケジュールタスクがタイムアウトしました</b>
telegram-notify-scheduled-error = エラー: { $error }
telegram-notify-summary-title = <b>日次サマリー</b> — { $date }
telegram-notify-summary-performance = <b>パフォーマンス</b>
telegram-notify-summary-trades = 取引 — { $total }（{ $wins }{ $win_icon } { $losses }{ $loss_icon }）
telegram-notify-summary-win-rate = 勝率 — { $percent }%
telegram-notify-summary-pnl = P{ "&amp;" }L — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = オープンポジション — { $count }
telegram-notify-started-title = <b>{ -brand } が起動しました</b>
telegram-notify-started-version = <b>バージョン</b> — { $version }
telegram-notify-started-mode = <b>モード</b> — { $mode }
telegram-notify-started-ready = 取引の準備ができました。
telegram-notify-stopped-title = <b>{ -brand } が停止しました</b>
telegram-notify-stopped-reason = <b>理由</b> — { $reason }
telegram-notify-stopped-goodbye = ご利用ありがとうございました。{ $icon }
telegram-notify-start-mode-normal = 通常
telegram-notify-stop-reason-graceful = 正常なシャットダウン
telegram-notify-update-available =
    <b>アップデート v{ $version } が利用可能です</b>

    { $how }
    ダウンロードサイズ: { $size } MB
telegram-notify-update-how-installer = このリリースにはデスクトップアプリの更新も含まれるため、インストーラーを1回実行する必要があります。
telegram-notify-update-ready =
    <b>アップデート v{ $version } の準備ができました</b>

    { $how }
telegram-notify-update-ready-silent = /update を送信するとすぐに適用されます。送信しない場合は、次回 { -brand } の起動時にインストールされます。
telegram-notify-update-ready-installer = 設定 → アップデート を開いて、インストーラーを実行してください。
telegram-notify-update-applying =
    <b>v{ $version } をインストール中</b>

    バックエンドを再起動中です。取引は自動的に再開されます。
telegram-notify-new-tokens =
    <b>フィルタリングアラート</b>

    { $count ->
       *[other] 条件に一致する新しいトークンが{ $count }個見つかりました。
    }
telegram-notify-crash =
    <b>ボットがクラッシュしました</b>

    <b>場所:</b> <code>{ $location }</code>
    <b>エラー:</b> <code>{ $error }</code>
telegram-notify-crash-restart = ボットを再起動してください。

## Filter results page.

telegram-filter-results-title = <b>フィルター結果</b>（{ $count }）
telegram-filter-results-empty = <i>トークンが見つかりません。</i>
telegram-filter-results-page = <i>{ $page } / { $total } ページ</i>

## Position screens.

telegram-position-not-found = ポジションが見つかりません
telegram-position-no-positions = クローズするポジションがありません
telegram-position-history-empty =
    <b>取引履歴</b>

    クローズ済みのポジションはまだありません。
telegram-position-history-title = <b>最近の取引</b>
telegram-position-history-more = <i>ほか{ $count }件の取引...</i>
telegram-position-confirm-hint = <i>30秒以内に確定すると実行されます。</i>
telegram-position-confirm-close-title = <b>ポジションをクローズしますか？</b>
telegram-position-confirm-close-selling = { $tokens } トークンを売却
telegram-position-confirm-close-estimated = 見込み — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>30秒以内に確定してください</i>
telegram-position-confirm-sell =
    <b>売却の確認</b>

    トークン — { $symbol }
    数量 — { $percent }%
    トークン数 — { $tokens }
telegram-position-confirm-dca =
    <b>追加購入の確認</b>

    トークン — { $symbol }
    追加 — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>全ポジションをクローズしますか？</b>

    件数 — { $count }
telegram-position-confirm-close-all-hint =
    <i>オープンポジションをすべて成行で売却します。
    30秒以内に確定してください。</i>
telegram-position-confirm-force-stop =
    <b>強制停止</b>

    すべての取引を直ちに停止します:
    • 新規エントリーなし
    • エグジットなし
    • DCA なし
telegram-position-confirm-force-stop-warning = <b>これは緊急操作です。</b>
telegram-position-confirm-blacklist =
    <b>トークンをブラックリストに追加しますか？</b>

    トークン — { $symbol }
    ミント — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>ポジションをクローズし、今後のエントリーを防ぎます。</i>
telegram-position-selling = { $symbol } の { $percent }% を売却中...
telegram-position-sell-done =
    <b>売却を実行しました</b>

    トークン — { $symbol }
    売却 — { $percent }%
    受取額 — { $amount } { -sol }
telegram-position-sell-failed = <b>売却に失敗しました</b>
telegram-position-adding = { $symbol } に { $amount } { -sol } を追加中...
telegram-position-dca-done =
    <b>DCA を実行しました</b>

    トークン — { $symbol }
    追加 — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA に失敗しました</b>
telegram-position-closing-all = 全ポジションをクローズ中...
telegram-position-close-all-done =
    <b>全クローズが完了しました</b>

    クローズ — { $closed }
    失敗 — { $failed }
telegram-position-blacklisted =
    <b>トークンをブラックリストに追加しました</b>

    トークン — { $symbol }
    状態 — クローズ済み { "&amp;" } ブラックリスト登録済み

## Token screens.

telegram-token-not-found = トークンが見つかりません
telegram-token-not-found-prefix = トークンが見つかりません。より長いプレフィックスで検索してみてください。
telegram-token-stats-failed = 統計を取得できませんでした: { $detail }
telegram-token-list-failed = トークンを取得できませんでした: { $detail }
telegram-token-list-empty = <b>{ $view }</b> ビューにトークンが見つかりません。
telegram-token-view-passed = フィルター通過
telegram-token-view-rejected = 除外
telegram-token-view-recent = 最近追加
telegram-token-view-all = すべてのトークン
telegram-token-list-title = <b>{ $name }</b>（{ $page }/{ $total } ページ）
telegram-token-list-stats = 流動性: { $liquidity } • 価格: { $price }
telegram-token-list-hint = <i>/token_ID をタップすると詳細を表示します</i>
telegram-token-explorer =
    <b>マーケットエクスプローラー</b>

    <b>概要</b>
    フィルター通過 — { $passed }
    除外 — { $rejected }
    価格取得中 — { $priced }
    検出合計 — { $total }

    <i>閲覧するカテゴリを選択してください:</i>
telegram-token-filter-title = <b>フィルター分析</b>
telegram-token-filter-distribution = <b>分布</b>
telegram-token-filter-passed = 通過 — { $count }（{ $percent }%）
telegram-token-filter-rejected = 除外 — { $count }（{ $percent }%）
telegram-token-filter-blacklisted = ブラックリスト — { $count }
telegram-token-filter-coverage = <b>カバレッジ</b>
telegram-token-filter-priced = プール価格あり — { $count }
telegram-token-filter-open = オープンポジション — { $count }
telegram-token-filter-total = 検出合計 — { $count }
telegram-token-filter-updated = <b>最終更新</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>{ $interval }ごとに自動更新</i>
telegram-token-detail-active = <b>アクティブなポジション</b>
telegram-token-detail-price = 価格 — { $price } { -sol }
telegram-token-detail-liquidity = 流動性 — { $value }
telegram-token-detail-volume = 24h 出来高 — { $value }
telegram-token-detail-change = 24h 変動 — { $value }
telegram-token-detail-risk = リスク評価: { $score }/100
telegram-token-detail-risk-unknown = リスク評価: 不明
telegram-token-detail-action = <i>アクションを選択してください:</i>
telegram-token-search =
    <b>マーケットを検索</b>

    検索するシンボルまたはミントアドレスを入力してください:

    <i>例: /token_BONK または /token_So11111</i>
telegram-token-confirm-buy =
    <b>直接購入の確認</b>

    トークン — ${ $symbol }
    ミント — <code>{ $mint }</code>
    数量 — { $amount } { -sol }

    <i>30秒以内に確定すると実行されます。</i>
telegram-token-confirm-blacklist =
    <b>トークンをブラックリストに追加しますか？</b>

    トークン — ${ $symbol }
    ミント — <code>{ $mint }</code>

    <i>このトークンがフィルターを通過しなくなります。</i>
telegram-token-blacklisted =
    <b>トークンをブラックリストに追加しました</b>

    トークン — ${ $symbol }
    状態 — ブラックリストに追加済み
telegram-token-blacklist-failed = <b>ブラックリストへの追加に失敗しました</b>
telegram-token-buy-processing =
    <b>購入を処理中...</b>

    トークン — ${ $symbol }
    数量 — { $amount } { -sol }
telegram-token-buy-done =
    <b>購入に成功しました</b>

    トークン — ${ $symbol }
    数量 — { $amount } { -sol }

    <i>詳細は /positions で確認できます</i>
telegram-token-buy-failed =
    <b>購入に失敗しました</b>

    トークン — ${ $symbol }
    エラー — { $detail }
