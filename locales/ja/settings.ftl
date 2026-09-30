## Shared

settings-duration-minutes =
    { $count ->
       *[other] { $count }分
    }
settings-duration-hours =
    { $count ->
       *[other] { $count }時間
    }

## settings_dialog.js

settings-dialog-title = 設定
settings-dialog-close =
    .title = 閉じる（ESC）
    .aria-label = 設定を閉じる
settings-dialog-save = 変更を保存
settings-dialog-saving = 保存中...
settings-dialog-saved = 保存しました
settings-dialog-save-success = 設定を保存しました
settings-dialog-save-failed = 設定の保存に失敗しました
settings-dialog-update-attention = アップデートの確認が必要です
settings-dialog-tab-interface = インターフェース
settings-dialog-tab-navigation = ナビゲーション
settings-dialog-tab-startup = 起動
settings-dialog-tab-hints = ヒント
settings-dialog-tab-data = データ
settings-dialog-tab-security = セキュリティ
settings-dialog-tab-account = アカウント
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = エージェント接続
settings-dialog-tab-updates = アップデート
settings-dialog-tab-licenses = ライセンス
settings-dialog-tab-about = 概要
settings-dialog-link-privacy = プライバシーポリシー
settings-dialog-link-terms = 利用規約

## settings_dialog.js: Startup tab

settings-startup-section-title = 起動時の動作
settings-startup-auto-start-label = トレーダーを自動開始
settings-startup-auto-start-hint = 起動時にトレーダーを自動的に開始します
settings-startup-coming-soon = 近日公開
settings-startup-default-page-label = デフォルトのページ
settings-startup-default-page-hint = アプリを開いたときに表示するページ
settings-startup-page-dashboard = ダッシュボード
settings-startup-page-tokens = トークン
settings-startup-page-positions = ポジション
settings-startup-page-wallet = ウォレット
settings-startup-page-config = 設定
settings-startup-notifications-label = バックグラウンド通知を表示
settings-startup-notifications-hint = バックグラウンドのイベントを通知します

## settings_dialog.js: About tab

settings-about-logo =
    .alt = { -brand }
settings-about-tagline = ネイティブ Solana トレーディングエンジン
settings-about-link-github = { -github }
settings-about-link-docs = ドキュメント
settings-about-link-telegram = { -telegram }
settings-about-link-website = ウェブサイト
settings-about-credits = Solana トレーダーのために開発
settings-about-copyright = © { $year } { -brand }. All rights reserved.

## interface_tab.js

settings-interface-section-appearance = 外観
settings-interface-theme-label = テーマ
settings-interface-theme-hint = お好みの配色を選択します
settings-interface-theme-dark = ダーク
settings-interface-theme-light = ライト
settings-interface-language-label = 言語
settings-interface-language-hint = ダッシュボードの表示言語
settings-interface-logo-shape-label = トークンロゴの形状
settings-interface-logo-shape-hint = 円形はすべてのロゴを円で切り抜き、ナチュラルは各ロゴ本来のシルエットを保ちます
settings-interface-logo-shape-circle = 円形
settings-interface-logo-shape-natural = ナチュラル
settings-interface-animations-label = アニメーションを有効化
settings-interface-animations-hint = なめらかな切り替えと効果
settings-interface-compact-label = コンパクトモード
settings-interface-compact-hint = 余白を減らして表示量を増やします
settings-interface-section-data = データと表示
settings-interface-refresh-label = 更新間隔
settings-interface-refresh-hint = データを更新する頻度
settings-interface-refresh-seconds =
    { $count ->
       *[other] { $count }秒
    }
settings-interface-refresh-minutes =
    { $count ->
       *[other] { $count }分
    }
settings-interface-ticker-label = ティッカーバーを表示
settings-interface-ticker-hint = ヘッダーにライブ指標のティッカーを表示します
settings-interface-page-size-label = テーブルのページサイズ
settings-interface-page-size-hint = 1 ページあたりの既定の行数
settings-interface-page-size-rows =
    { $count ->
       *[other] { $count }行
    }
settings-interface-auto-expand-label = カテゴリを自動展開
settings-interface-auto-expand-hint = 設定カテゴリを既定で展開します
settings-interface-hints-label = コンテキストヒントを表示
settings-interface-hints-hint = ダッシュボードの機能を説明するヘルプアイコンを表示します
settings-interface-featured-label = 注目行を表示
settings-interface-featured-hint = ホームとトークンのページに注目トークンの行を表示します
settings-interface-section-sound = サウンドエフェクト
settings-interface-sounds-label = サウンドを有効化
settings-interface-sounds-hint = 操作、状態の変化、結果を音で知らせます

## security_tab.js

settings-security-loading = セキュリティ設定を読み込み中...
settings-security-load-failed = セキュリティ設定の読み込みに失敗しました

settings-security-type-pin4 = 4 桁の PIN
settings-security-type-pin6 = 6 桁の PIN
settings-security-type-text = テキストパスワード
settings-security-type-unset = 未設定

settings-security-lockscreen-title = ダッシュボードのロック画面
settings-security-lockscreen-description = PIN またはパスワードでダッシュボードを保護します。ロック画面が表示されると、続行するには認証が必要になります。
settings-security-enable-label = ロック画面を有効化
settings-security-enable-hint = パスワード認証でダッシュボードを保護します
settings-security-password-status-label = パスワードの状態
settings-security-password-current = 現在: { $type }
settings-security-password-none = パスワードは未設定です
settings-security-change = 変更
settings-security-remove = 削除
settings-security-set-password = パスワードを設定
settings-security-auto-lock-label = 操作がない場合に自動ロック
settings-security-auto-lock-hint = 一定時間操作がないと自動的にロックします
settings-security-auto-lock-never = しない
settings-security-lock-blur-label = ウィンドウが非アクティブになったらロック
settings-security-lock-blur-hint = 別のアプリケーションに切り替えると自動的にロックします
settings-security-quick-actions-title = クイックアクション
settings-security-lock-now-label = ダッシュボードを今すぐロック
settings-security-lock-now-hint = ダッシュボードをただちにロックします
settings-security-lock-now = 今すぐロック
settings-security-lock-not-ready = ロックできません - ロック画面の準備ができていません
settings-security-setting-save-failed = セキュリティ設定を保存できませんでした

## security_tab.js: two-factor authentication

settings-security-2fa-title = 2 段階認証
settings-security-2fa-description = 認証アプリ（Google Authenticator、Authy など）でセキュリティをさらに強化します
settings-security-2fa-status-label = 2FA の状態
settings-security-2fa-status-enabled = 2 段階認証は有効です
settings-security-2fa-status-none = 未設定
settings-security-2fa-disable = 2FA を無効化
settings-security-2fa-enable = 2FA を有効化

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = 閉じる
settings-security-password-set-title = パスワードを設定
settings-security-password-change-title = パスワードを変更
settings-security-password-current-label = 現在のパスワード
settings-security-password-current-input =
    .placeholder = 現在のパスワードを入力
settings-security-password-type-label = パスワードの種類
settings-security-password-new-label = 新しいパスワード
settings-security-password-new-input =
    .placeholder = 新しいパスワードを入力
settings-security-password-confirm-label = パスワードの確認
settings-security-password-confirm-input =
    .placeholder = パスワードを再入力
settings-security-password-update = パスワードを更新
settings-security-placeholder-pin4 = 4 桁の PIN を入力
settings-security-placeholder-pin6 = 6 桁の PIN を入力
settings-security-placeholder-text = パスワードを入力
settings-security-password-required = パスワードを入力してください
settings-security-password-mismatch = パスワードが一致しません
settings-security-pin4-invalid = PIN は 4 桁で入力してください
settings-security-pin6-invalid = PIN は 6 桁で入力してください
settings-security-text-too-short = パスワードは 4 文字以上で入力してください
settings-security-password-saved = パスワードを保存しました
settings-security-password-save-failed = パスワードの保存に失敗しました
settings-security-password-save-failed-detail = パスワードの保存に失敗しました: { $message }

## security_tab.js: remove password dialog

settings-security-remove-title = パスワードを削除
settings-security-remove-description = ロック画面の保護を解除するには、現在のパスワードを入力してください。
settings-security-remove-confirm = パスワードを削除
settings-security-current-required = 現在のパスワードを入力してください
settings-security-password-removed = パスワードを削除しました
settings-security-password-remove-failed = パスワードの削除に失敗しました
settings-security-password-remove-failed-detail = パスワードの削除に失敗しました: { $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = 2 段階認証を有効化
settings-security-2fa-password-prompt = 続行するにはパスワードを入力してください:
settings-security-2fa-password-input =
    .placeholder = パスワードを入力
settings-security-2fa-continue = 続行
settings-security-2fa-manual-code = 手動入力用コード:
settings-security-2fa-qr =
    .alt = TOTP の QR コード
settings-security-2fa-code-prompt = 認証アプリの 6 桁のコードを入力してください:
settings-security-2fa-verify-enable = 認証して有効化
settings-security-2fa-password-required = パスワードを入力してください
settings-security-2fa-setup-failed = 2FA の設定に失敗しました
settings-security-2fa-code-invalid-length = 6 桁のコードを入力してください
settings-security-2fa-code-invalid = コードが無効です
settings-security-2fa-enabled = 2 段階認証を有効にしました
settings-security-2fa-verify-failed = コードの認証に失敗しました
settings-security-2fa-disable-title = 2 段階認証を無効化
settings-security-2fa-disable-prompt = 2FA を無効にするにはパスワードを入力してください:
settings-security-2fa-disable-failed = 2FA の無効化に失敗しました
settings-security-2fa-disabled = 2 段階認証を無効にしました

## agent_connections_tab.js

settings-agent-category-analysis = 分析
settings-agent-category-portfolio = ポートフォリオ
settings-agent-category-trading = 取引
settings-agent-category-config = 設定
settings-agent-category-system = システム
settings-agent-category-analysis-description = トークン分析、市場データ、セキュリティチェック。
settings-agent-category-portfolio-description = オープンポジション、残高、損益。
settings-agent-category-trading-description = 実資金でのポジションの購入、売却、クローズ。
settings-agent-category-config-description = RPC エンドポイントを含むすべてのボット設定。ウォレットキーは対象外です。
settings-agent-category-system-description = ステータス、イベント、緊急停止。
settings-agent-category-analysis-inline = 分析
settings-agent-category-portfolio-inline = ポートフォリオ
settings-agent-category-trading-inline = 取引
settings-agent-category-config-inline = 設定
settings-agent-category-system-inline = システム

settings-agent-level-allow = 許可
settings-agent-level-ask-user = 確認
settings-agent-level-deny = 無効
settings-agent-level-allow-hint = ただちに実行されます。
settings-agent-level-ask-user-hint = アプリ内でユーザーの承認を待ちます。
settings-agent-level-deny-hint = 拒否され、エージェントには表示されません。

settings-agent-preset-full = フルアクセス
settings-agent-preset-ask = 確認してから実行
settings-agent-preset-read = 読み取り専用
settings-agent-preset-full-description = すべて確認なしで実行されます。ウォレットキーには常にアクセスできません。
settings-agent-preset-ask-description = すべての操作でアプリ内の承認を待ちます。
settings-agent-preset-read-description = 分析とポートフォリオの読み取りのみ。何も変更できません。
settings-agent-preset-custom = カスタム
settings-agent-preset-group =
    .aria-label = 権限プリセット
settings-agent-permission-group = { $category }の権限

settings-agent-summary-asks-only = 制限あり — { $asking }は確認が必要
settings-agent-summary-off-only = 制限あり — { $off }は無効
settings-agent-summary-asks-and-off = 制限あり — { $asking }は確認が必要、{ $off }は無効

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = 汎用 stdio MCP

settings-agent-note-placeholder = /absolute/path/to/screenerbot を { -brand } バイナリの絶対パスに置き換えてください。実行中のアプリは、このシステムでは実行ファイルのパスを取得できませんでした。
settings-agent-note-data-dir = { -brand } を既定以外のデータディレクトリで実行している場合は、クライアント側でも SCREENERBOT_DATA_DIR を同じパスに設定してください（別の -e / --env フラグ、または env エントリ）。
settings-agent-note-codex-run = コマンドを実行するか、TOML ブロックを ~/.codex/config.toml（$CODEX_HOME/config.toml）に追加してください。その後、{ -codex } を再起動してください。
settings-agent-note-codex-get = `codex mcp get screenerbot` は出力内のシークレットをマスクします。
settings-agent-note-claude-code = { -claude } Code: コマンドを実行してから、{ -claude } Code を再起動してください。`claude mcp get screenerbot` は、シークレットを含む設定済みの環境変数を出力します。
settings-agent-note-claude-desktop = { -claude } Desktop: JSON を claude_desktop_config.json の `mcpServers` にマージし、アプリを再起動してください。
settings-agent-note-openclaw = コマンドを実行してから、`openclaw mcp doctor screenerbot --probe` で、保存した stdio サーバーが起動しツールを公開していることを確認してください。
settings-agent-note-hermes = { -hermes } の設定ファイルの `mcp_servers` に次を追加してから、{ -hermes } を再起動してください。
settings-agent-note-generic = stdio に対応する任意の MCP クライアントで使えます。クライアントのサーバー一覧の場所に、このコマンドを引数と環境変数とともに登録してください。
settings-agent-block-codex-command = { -codex } CLI — ターミナルコマンド
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml（代替）
settings-agent-block-claude-command = { -claude } Code — ターミナルコマンド
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — ターミナルコマンド
settings-agent-block-hermes = { -hermes } — mcp_servers（YAML）
settings-agent-block-generic = 汎用 stdio MCP クライアント

settings-agent-name-required = この接続の名前を入力してください。
settings-agent-name-too-long = 名前は { $max } 文字以内で入力してください。
settings-agent-name-control-characters = 名前に制御文字を含めることはできません。

settings-agent-title = エージェント接続
settings-agent-description = { -claude }、{ -codex }、{ -hermes }、{ -openclaw }、または任意の stdio MCP クライアントを接続できます。{ -brand } は起動したままにしてください。各接続はそれぞれ独自の権限を持ちます。既定ではフルアクセスで、いつでも接続ごとに制限できます。どの接続もウォレットキーの読み取りや変更はできません。
settings-agent-name-label = 接続名
settings-agent-name-hint = 下の一覧に表示され、接続を区別するために使います。
settings-agent-name-input =
    .placeholder = ノート PC のコーディングエージェント
settings-agent-client-label = クライアント
settings-agent-client-hint = 接続の作成後に表示されるセットアップ手順を切り替えます。
settings-agent-permissions-label = 権限
settings-agent-permissions-hint = 新しい接続はすべての操作を実行できます。カテゴリごとの制限は今でも、下の一覧から後でも設定できます。どちらの場合もウォレットキーには常にアクセスできません。
settings-agent-create = 接続を作成
settings-agent-issued-group =
    .aria-label = 新しい接続の認証情報
settings-agent-issued-warning = シークレットは今すぐコピーしてください。表示は 1 回のみで、再取得はできません。紛失した場合は、接続を取り消して作り直してください。{ -brand } は一方向の検証用データのみを保持し、平文は MCP クライアントが自身の設定内に保存します。
settings-agent-issued-client-id = クライアント ID
settings-agent-issued-secret = ワンタイムシークレット
settings-agent-setup-for = セットアップ対象
settings-agent-done = 完了
settings-agent-list-title = 接続
settings-agent-loading = 接続を読み込み中...
settings-agent-active-count = 有効 { $count }件
settings-agent-empty = 接続はまだありません。上でクライアントとの接続を作成してください。
settings-agent-empty-active = 有効な接続はありません。
settings-agent-revoked-title = 取り消し済みの接続
settings-agent-created = 作成: { $time }
settings-agent-last-used = 最終使用: { $time }
settings-agent-never-used = 未使用
settings-agent-permissions-edit = 権限
settings-agent-revoke = 取り消し
settings-agent-permissions-save = 権限を保存

settings-agent-load-failed = エージェント接続の読み込みに失敗しました
settings-agent-list-failed = 接続を読み込めませんでした
settings-agent-create-failed = 接続を作成できませんでした。
settings-agent-unreachable-create = 接続を作成するための { -brand } に到達できませんでした。
settings-agent-permissions-update-failed = 権限を更新できませんでした
settings-agent-permissions-updated = 権限を更新しました
settings-agent-permissions-updated-detail = この接続の次回のリクエストから適用されます。
settings-agent-unreachable-save = 保存するための { -brand } に到達できませんでした
settings-agent-revoke-title = 接続を取り消し
settings-agent-revoke-message = 「{ $label }」を取り消しますか？クライアントは次回のリクエストから動作しなくなり、元に戻せません。
settings-agent-revoke-fallback-name = この接続
settings-agent-revoke-failed = 接続を取り消せませんでした
settings-agent-unreachable-revoke = 取り消すための { -brand } に到達できませんでした

## telegram_tab.js

settings-telegram-loading = { -telegram } 設定を読み込み中...
settings-telegram-load-failed = { -telegram } 設定の読み込みに失敗しました
settings-telegram-unknown = 不明
settings-telegram-session-active = 有効: { $duration }
settings-telegram-sessions-empty = 有効なセッションはありません
settings-telegram-session-revoke = 取り消し

settings-telegram-connection-title = 接続
settings-telegram-connection-description = { -telegram } ボットを接続すると、通知を受け取り、{ -brand } をリモートで操作できます。
settings-telegram-enable-label = { -telegram } を有効化
settings-telegram-enable-hint = { -telegram } ボット連携を有効にします
settings-telegram-token-label = ボットトークン
settings-telegram-token-saved = トークン保存済み
settings-telegram-token-help = { -telegram } の @BotFather で取得します
settings-telegram-token-input-saved =
    .placeholder = トークン保存済み（変更するには新しいトークンを入力）
settings-telegram-token-input =
    .placeholder = ボットトークンを入力
settings-telegram-token-toggle =
    .title = 表示/非表示
settings-telegram-chat-label = チャット ID
settings-telegram-chat-connected = 接続中のチャット:
settings-telegram-chat-discover-hint = チャット ID を自動的に検出します
settings-telegram-chat-change =
    .title = 変更
settings-telegram-chat-discover = チャット ID を検出
settings-telegram-discovery-step-add = ボットを { -telegram } グループに追加するか、ボットとのダイレクトチャットを開始します
settings-telegram-discovery-step-privacy = グループの場合: @BotFather → /mybots → [your bot] → Bot Settings → Group Privacy を確認します
settings-telegram-discovery-privacy = <strong>プライバシーモード OFF:</strong> ボットはグループのすべてのメッセージを受信します<br/><strong>プライバシーモード ON:</strong> ボットは @メンションされたメッセージのみ受信します
settings-telegram-discovery-step-send = 任意のメッセージを送信します（プライバシーモードが ON の場合はボットに @メンションします）
settings-telegram-discovery-listening = メッセージを待機中...
settings-telegram-discovery-select = 選択
settings-telegram-chat-id-label = ID:
settings-telegram-language-label = メッセージの言語
settings-telegram-language-hint = { -telegram } ボットのメッセージとボタンの言語
settings-telegram-language-follow-app = アプリの言語に合わせる
settings-telegram-test-label = 接続テスト
settings-telegram-test-hint = テストメッセージを送信して設定を確認します
settings-telegram-test-send = テスト送信
settings-telegram-test-sending = 送信中...

settings-telegram-chat-type-private = プライベート
settings-telegram-chat-type-group = グループ
settings-telegram-chat-type-supergroup = スーパーグループ
settings-telegram-chat-type-channel = チャンネル

settings-telegram-auth-title = コマンド認証
settings-telegram-auth-description = { -telegram } コマンドには、ダッシュボードのロック画面と同じ 2FA を使用します。
settings-telegram-auth-protected = 保護中
settings-telegram-auth-disabled = 無効
settings-telegram-auth-not-configured = 未設定
settings-telegram-auth-error = エラー
settings-telegram-auth-protected-note = コマンドはロック画面の 2FA で保護されています。セッションの有効期限が切れた場合は、<code>/login</code> コマンドで認証アプリのコードを入力する必要があります。
settings-telegram-auth-disabled-note = ロック画面の 2FA は設定されていますが、{ -telegram } では無効です。{ -telegram } コマンドを保護するには、上の「コマンドに 2FA を要求」を有効にしてください。
settings-telegram-auth-missing-note = ロック画面の 2FA が設定されていません。2FA がないと、期限切れのセッションは認証なしで自動的に再有効化されます。
settings-telegram-auth-managed-in = 2FA の管理場所:
settings-telegram-auth-configure-in = 2FA を
settings-telegram-auth-configure-suffix = で設定すると、{ -telegram } コマンドの認証を必須にできます。
settings-telegram-security-link = セキュリティ設定
settings-telegram-timeout-title = セッションのタイムアウト
settings-telegram-timeout-description = 認証済みセッションが有効な期間
settings-telegram-sessions-title = 有効なセッション

settings-telegram-notifications-title = 通知設定
settings-telegram-notifications-description = { -telegram } 通知の対象とするイベントを選択します。
settings-telegram-notify-opened-label = ポジションのオープン
settings-telegram-notify-opened-hint = 新しいポジションが開かれたときに通知します
settings-telegram-notify-closed-label = ポジションのクローズ
settings-telegram-notify-closed-hint = ポジションがクローズされたときに通知します
settings-telegram-notify-partial-label = 部分エグジット
settings-telegram-notify-partial-hint = ポジションの部分エグジット時に通知します
settings-telegram-notify-dca-label = DCA 実行
settings-telegram-notify-dca-hint = DCA 注文が実行されたときに通知します
settings-telegram-notify-errors-label = エラー
settings-telegram-notify-errors-hint = エラーや失敗を通知します
settings-telegram-notify-startup-label = 起動/停止
settings-telegram-notify-startup-hint = ボットの起動または停止時に通知します
settings-telegram-notify-filtering-label = フィルタリングアラート
settings-telegram-notify-filtering-hint = 新しいトークンがフィルタリング条件を通過したときに通知します
settings-telegram-notify-trades-label = 取引アラート
settings-telegram-notify-trades-hint = ウォッチ中のトークンの大口取引を通知します
settings-telegram-notify-daily-label = 日次サマリー
settings-telegram-notify-daily-hint = 1 日の取引状況と損益のサマリーを受け取ります

settings-telegram-features-title = 機能
settings-telegram-features-description = { -telegram } ボットの機能を設定します。
settings-telegram-commands-label = コマンドを有効化
settings-telegram-commands-hint = { -telegram } コマンドによるボットの操作を許可します
settings-telegram-require-2fa-label = コマンドに 2FA を要求
settings-telegram-require-2fa-hint = セッションの期限切れ後の再有効化に 2FA コードを必須にします。ロック画面の 2FA を使用します。
settings-telegram-inline-label = インラインアクションボタン
settings-telegram-inline-hint = 通知メッセージにアクションボタンを表示します

settings-telegram-setting-save-failed = { -telegram } 設定を保存できませんでした
settings-telegram-discovery-start-failed = 検出を開始できませんでした
settings-telegram-chat-selected = チャットを選択しました
settings-telegram-chat-select-failed = チャットを選択できませんでした
settings-telegram-test-sent = テストメッセージを送信しました
settings-telegram-test-failed = テストメッセージの送信に失敗しました
settings-telegram-session-revoked = セッションを取り消しました
settings-telegram-session-revoke-failed = セッションを取り消せませんでした

## licenses_tab.js

settings-licenses-title = オープンソースライセンス
settings-licenses-subtitle = { -brand } は次のオープンソースソフトウェアを使用して開発されています
settings-licenses-footer = ライセンスの全文は、プロジェクトのリポジトリおよび各依存ライブラリのソースコードで確認できます。
settings-licenses-category-framework = アプリケーションフレームワーク
settings-licenses-category-solana = Solana ブロックチェーン
settings-licenses-category-data = データとストレージ
settings-licenses-category-networking = ネットワーク
settings-licenses-category-cryptography = 暗号とエンコーディング
settings-licenses-category-assets = UI アセット
settings-licenses-desc-electron = デスクトップアプリケーションフレームワーク
settings-licenses-desc-tokio = Rust の非同期ランタイム
settings-licenses-desc-axum = ウェブサーバーフレームワーク
settings-licenses-desc-tower = サービス抽象化
settings-licenses-desc-hyper = HTTP 実装
settings-licenses-desc-solana-sdk = Solana SDK コア
settings-licenses-desc-solana-client = RPC クライアント
settings-licenses-desc-solana-program = プログラムライブラリ
settings-licenses-desc-spl-token = SPL Token プログラム
settings-licenses-desc-spl-token-2022 = Token-2022 拡張
settings-licenses-desc-spl-associated-token-account = アソシエイテッドトークンアカウント
settings-licenses-desc-sqlite = 組み込みデータベースエンジン
settings-licenses-desc-rusqlite = SQLite の Rust バインディング
settings-licenses-desc-r2d2 = データベース接続プール
settings-licenses-desc-serde = シリアライズフレームワーク
settings-licenses-desc-toml = 設定ファイルの解析
settings-licenses-desc-reqwest = HTTP クライアント
settings-licenses-desc-tokio-tungstenite = WebSocket クライアント
settings-licenses-desc-rustls = TLS 実装
settings-licenses-desc-blake3 = ハッシュ関数
settings-licenses-desc-sha-2 = SHA-256/512 ハッシュ
settings-licenses-desc-bs58 = Base58 エンコーディング
settings-licenses-desc-base64 = Base64 エンコーディング
settings-licenses-desc-lucide-icons = アイコンフォントライブラリ
settings-licenses-desc-inter = インターフェースフォント
settings-licenses-desc-jetbrains-mono = 等幅フォント
settings-licenses-desc-orbitron = ディスプレイフォント
settings-licenses-desc-vazirmatn = アラビア語・ペルシア語フォント
settings-licenses-desc-noto-sans-devanagari = デーヴァナーガリー文字フォント
settings-licenses-desc-noto-sans-sc = 簡体字中国語フォント
settings-licenses-desc-pretendard = 韓国語フォント
settings-licenses-desc-pretendard-jp = 日本語フォント

## hints_tab.js

settings-hints-title = コンテキストヒント
settings-hints-description = コンテキストヒントは、ダッシュボードの機能を説明するヘルプアイコンです。下のヒントをすべて確認でき、「今後表示しない」で非表示にしたヒントを、1 件ずつまたはまとめて元に戻せます。
settings-hints-hidden-label = 非表示のヒント
settings-hints-hidden-summary = 全 { $total }件のうち { $hidden }件のヒントが現在非表示です。
settings-hints-restore-all = すべてのヒントを復元
settings-hints-toggle-shown =
    .title = このヒントを表示
settings-hints-toggle-shown-title = 表示中
settings-hints-toggle-hidden-title = 非表示 — 有効にすると表示されます
settings-hints-restore-title = すべてのヒントを復元
settings-hints-restore-message = 非表示にしたものを含め、すべてのコンテキストヒントを再び表示しますか？
settings-hints-restore-confirm = すべて復元
settings-hints-restored = すべてのヒントを復元しました

## account_tab.js

settings-account-title = { -brand } アカウント
settings-account-description = 無料で、任意です。{ -brand } はアカウントなしでも、取引、銘柄探索、チャート表示ができます。その場合は公開プロバイダーを利用します。ログインすると何が追加されるかは、下のパネルに表示されます。
settings-account-data-title = { -brand } データ
settings-account-data-description = screenerbot.io で共有の市場データサービスを運営しています。7 つの時間足にまたがる集約ローソク足、解決済みのプールレジストリ、キャッシュされたセキュリティレポート、正規化されたトークン情報を提供します。すべてのインストールが公開プロバイダーごとに個別のレート制限を受けないようにするためのもので、この共有コストの負担先を明確にするため、利用にはアカウントが必要です。
settings-account-data-fallback = 利用できない場合、{ -brand } は自動的に公開プロバイダーに切り替わります。動作は止まらず、チャートの表示が遅くなり、履歴が少なくなるだけです。
settings-account-gateway-title = トランザクションの送信
settings-account-gateway-description = ログイン中は、{ -brand } がスワップを自前の RPC ではなく screenerbot.io 経由で送信できます。トランザクションの作成と署名は引き続きこのマシン上で行われ、サーバーは中継するだけです。署名済みトランザクションを変更すると署名が無効になるため、改変はできません。
settings-account-gateway-label = トランザクション送信に { -brand } の RPC を使用
settings-account-gateway-hint = 送信のみが対象です。価格データは常にご自身の RPC から取得します。プールのポーリングは共有エンドポイントには負荷が大きすぎるため、そちらには送信しません。
settings-account-manage-title = アカウントの管理
settings-account-manage-description = パスワード、メールアドレス、接続済みデバイス、紹介報酬の受け取りは、ウェブサイトで管理します。そこでデバイスを取り消すと、このデバイスを含むすべての場所でログアウトされます。
settings-account-open-dashboard = ダッシュボードを開く

## navigation_tab.js

settings-navigation-title = ナビゲーションタブ
settings-navigation-hint = 項目をドラッグして並べ替えます。スイッチで表示を切り替えます。
settings-navigation-note = 変更は保存後に適用されます。ナビゲーションバーに反映するにはページを再読み込みしてください。
settings-navigation-drag-handle =
    .title = ドラッグして並べ替え
settings-navigation-defaults-failed = デフォルトのナビゲーションを読み込めませんでした
settings-navigation-reset = ナビゲーションをデフォルトに戻しました

## data_tab.js

settings-data-storage-title = データベースストレージ
settings-data-storage-description = 取引データ、ポジション、履歴情報を保存しているすべてのデータベースの概要です。
settings-data-stats-loading = データベース統計を読み込み中...
settings-data-stats-load-failed = データベース統計の読み込みに失敗しました
settings-data-total-storage = データベースの合計容量
settings-data-db-tokens = トークン
settings-data-db-transactions = トランザクション
settings-data-db-positions = ポジション
settings-data-db-events = イベント
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = ウォレット
settings-data-db-pools = プール
settings-data-db-strategies = ストラテジー
settings-data-db-actions = アクション
settings-data-directory-label = データディレクトリ
settings-data-directory-copied = データディレクトリ
settings-data-config-path-copied = 設定ファイルのパス
settings-data-path-unavailable = 利用不可
settings-data-path-copy-title = クリックしてパスをコピー
settings-data-path-copy-failed = パスのコピーに失敗しました

settings-data-config-title = 設定の管理
settings-data-config-description = ボット設定のエクスポート、インポート、管理を行います。大きな変更の前にバックアップを取ってください。
settings-data-config-export = 設定をエクスポート
settings-data-config-import = 設定をインポート
settings-data-config-reset = デフォルトに戻す
settings-data-config-location-label = 設定ファイルの場所
settings-data-config-fetch-failed = 設定の取得に失敗しました
settings-data-config-exported = 設定をエクスポートしました
settings-data-config-export-failed = 設定のエクスポートに失敗しました: { $message }
settings-data-config-import-title = 設定をインポート
settings-data-config-import-message = この設定をインポートしますか？現在の設定は上書きされます。ウォレットの認証情報は保持されます。
settings-data-config-imported = 設定をインポートしました。変更によっては再起動が必要な場合があります。
settings-data-config-import-failed = 設定のインポートに失敗しました: { $message }
settings-data-config-reset-title = 設定をリセット
settings-data-config-reset-message = すべての設定をデフォルトに戻しますか？ウォレットの認証情報は保持されますが、それ以外のすべての設定がリセットされます。
settings-data-config-reset-done = 設定をデフォルトに戻しました
settings-data-config-reset-failed = 設定のリセットに失敗しました: { $message }
settings-data-unknown-error = 不明なエラー

settings-data-cleanup-title = データのクリーンアップ
settings-data-cleanup-description = 古いデータや不要なデータを削除してディスク容量を確保します。この操作は元に戻せません。
settings-data-ohlcv-cleanup-label = OHLCV データのクリーンアップ
settings-data-ohlcv-cleanup-hint = 指定した時間、アクティブでないトークンのローソク足データを削除します。
settings-data-cleanup-hours-unit = 時間
settings-data-cleanup-ohlcv = OHLCV をクリーンアップ
settings-data-cleanup-running = クリーンアップ中...
settings-data-cleanup-hours-invalid = 時間の値が無効です
settings-data-cleanup-confirm-title = OHLCV データを削除
settings-data-cleanup-confirm-message =
    { $hours ->
       *[other] { $hours }時間以上アクティブでないトークンの OHLCV データを削除しますか？
    }
settings-data-cleanup-done =
    { $count ->
       *[other] 非アクティブなトークン { $count }件をクリーンアップしました
    }
settings-data-cleanup-failed = クリーンアップに失敗しました
settings-data-cleanup-failed-detail = クリーンアップに失敗しました: { $message }

settings-data-cache-clear-label = すべての OHLCV キャッシュを消去
settings-data-cache-clear-hint = キャッシュされたローソク足データをすべて消去し、監視中のすべてのトークンを最初から再取得します。チャートの表示がおかしい場合や、データ処理の更新後に使用してください。
settings-data-cache-clear = OHLCV キャッシュを消去
settings-data-cache-clearing = 消去中...
settings-data-cache-confirm-title = すべての OHLCV キャッシュを消去
settings-data-cache-confirm-message = すべてのトークンのキャッシュ済みローソク足データを消去しますか？監視中のトークンは履歴を最初から再取得します。この操作は元に戻せません。
settings-data-candles-count =
    { $count ->
       *[other] ローソク足 { $count }本
    }
settings-data-tokens-count =
    { $count ->
       *[other] { $count }トークン
    }
settings-data-cache-cleared = { $candles }（{ $tokens }分）を消去しました。再取得中です
settings-data-cache-clear-failed = OHLCV キャッシュの消去に失敗しました
settings-data-cache-clear-failed-detail = OHLCV キャッシュの消去に失敗しました: { $message }

settings-data-ui-cache-label = UI 状態キャッシュ
settings-data-ui-cache-hint = 保存済みのテーブル設定、フィルター状態、表示設定を消去します。
settings-data-ui-cache-clear = UI キャッシュを消去
settings-data-ui-cache-confirm-title = UI 状態を消去
settings-data-ui-cache-confirm-message = 保存済みの UI 設定をすべて消去しますか？テーブルの列、フィルター、表示設定がリセットされます。
settings-data-ui-cache-cleared =
    { $count ->
       *[other] キャッシュされた UI 設定 { $count }件を消去しました
    }

settings-data-folder-label = データフォルダーを開く
settings-data-folder-hint = { -brand } のすべてのデータを含むフォルダーをファイルマネージャーで開きます。
settings-data-folder-open = フォルダーを開く
settings-data-folder-open-failed = データフォルダーを開けませんでした
