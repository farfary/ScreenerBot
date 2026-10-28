# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

# Source: scripts/core/setup_runtime.js

## Wallet key validation

setup-wallet-required = ウォレットの秘密鍵を入力してください。
setup-wallet-json-recognized = 64バイトの JSON 鍵形式を認識しました。
setup-wallet-json-invalid = ちょうど 64 個のバイト値（0～255）を含む JSON 配列を使用してください。
setup-wallet-format-invalid = base58 形式の秘密鍵、または 64 バイトの JSON 配列を使用してください。
setup-wallet-base58-recognized = Base58 鍵形式を認識しました。

## RPC endpoint validation

setup-rpc-required = RPC エンドポイントを1つ以上入力してください。
setup-dialog-rpc-optional-note = 任意: 設定済みの RPC エンドポイントを使い続ける場合は、空欄のままにしてください。
setup-rpc-too-many = RPC エンドポイントは 10 個までにしてください。
setup-rpc-url-invalid = すべてのエンドポイントは有効な HTTPS URL である必要があります。
setup-rpc-url-credentials = RPC URL にユーザー名やパスワードを含めることはできません。
setup-rpc-url-fragment = RPC URL にフラグメントを含めることはできません。
setup-rpc-public-endpoint = 公開 Solana RPC では継続的なポーリングに対応できません。
setup-rpc-private-host = RPC エンドポイントにローカルまたはプライベートネットワークのホストは使用できません。
setup-rpc-duplicate = 重複している RPC エンドポイントを削除してください。
setup-rpc-ready =
    { $count ->
       *[other] HTTPS エンドポイント { $count }件のテストの準備ができました。
    }

## Verification results

setup-wallet-verified = ウォレットを検証しました
setup-wallet-unverified = ウォレットを検証できませんでした
setup-wallet-address-detail = アドレス { $address }
setup-wallet-format-hint = 秘密鍵の形式を確認してください。
setup-rpc-none-working = 動作するメインネット RPC がありません
setup-rpc-health-failed = メインネットのヘルスチェックに通ったエンドポイントはありません。
setup-rpc-partial = 動作中 { $working }件、利用不可 { $failed }件
setup-rpc-verified =
    { $count ->
       *[other] メインネットエンドポイント { $count }件を検証しました
    }
setup-rpc-fastest = 最速: { $url }（{ $latency }ミリ秒）。
setup-error-request-failed = リクエストに失敗しました（{ $status }）
setup-error-restart-timeout = { -brand } がまだ再接続していません。

# Source: scripts/core/setup.js

## Verification steps

setup-verify-wallet-parsing = 秘密鍵を解析中
setup-verify-wallet-parsing-detail = 鍵を確認し、公開アドレスを導出しています。
setup-verify-wallet-waiting = 検証待ち
setup-verify-rpc-testing = Solana メインネットをテスト中
setup-verify-rpc-testing-detail =
    { $count ->
       *[other] エンドポイント { $count }件を確認しています。
    }
setup-verify-rpc-waiting = エンドポイントのテスト待ち
setup-verify-save-waiting = 保存待ち
setup-verify-save-running = 暗号化して保存中
setup-verify-save-running-detail = 検証済みの設定をこのデバイスに書き込んでいます。
setup-verify-save-done = 設定を保存しました
setup-verify-save-done-detail = 秘密鍵を暗号化し、動作する RPC エンドポイントを保存しました。
setup-verify-save-failed = セットアップを保存できませんでした
setup-verify-save-skipped = 未保存。上の問題を解決してから、もう一度検証してください。
setup-verify-request-failed = 検証リクエストに失敗しました
setup-verify-summary-checking = ウォレットと Solana メインネットの接続を確認しています。
setup-verify-summary-running = 入力された認証情報をそのまま検証しています。
setup-verify-summary-saving = 認証情報を検証しました。安全に保存しています。
setup-verify-summary-failed = 問題を確認してから、もう一度検証してください。

## Errors

setup-error-credentials-failed = 認証情報の検証に失敗しました。
setup-error-save-failed = セットアップを保存できませんでした。
setup-error-verify-failed = 検証に失敗しました。
setup-error-explore-failed = Explore モードを開始できませんでした。
setup-error-gateway-failed = トランザクション送信の設定を保存できませんでした。
setup-action-review-credentials = 認証情報を確認

## Completion

setup-explore-opening = Explore モードを開いています…
setup-complete-restarting = 検証済みの設定で { -brand } を再起動しています。
setup-complete-finishing = 再起動を完了しています…
setup-complete-ready = { -brand } の準備が整いました。ダッシュボードを開いています…
setup-complete-stored = 検証済みの設定は、このデバイスに安全に保存されています。

## Wallet controls (shared with the setup dialog)

setup-wallet-show-key = 秘密鍵を表示
setup-wallet-hide-key = 秘密鍵を非表示
setup-wallet-copy =
    .aria-label = ウォレットアドレスをコピー
    .title = ウォレットアドレスをコピー
setup-wallet-copy-done =
    .aria-label = ウォレットアドレスをコピーしました
    .title = コピーしました
setup-wallet-copy-failed =
    .aria-label = ウォレットアドレスをコピーできませんでした
    .title = コピーに失敗しました

# Source: scripts/ui/setup_dialog.js

## Setup dialog

setup-dialog-close =
    .title = 閉じる
    .aria-label = 閉じる
setup-dialog-wallet-label = ウォレットの秘密鍵
setup-dialog-wallet-input =
    .placeholder = Base58 文字列または JSON 配列 [1,2,3,...]
setup-dialog-working = 処理中…
setup-dialog-validating = 検証中…
setup-dialog-saving = 保存中…
setup-dialog-restarting = 再起動中…
setup-dialog-error-validation = 検証に失敗しました。
setup-dialog-error-incomplete = セットアップを完了できませんでした。
setup-dialog-error-restart-helper = 自動再起動ヘルパーを利用できません。しばらくしてからダッシュボードを再読み込みしてください。
setup-dialog-error-unexpected = 予期しないエラーです。
setup-dialog-replace-title = 取引ウォレットを置き換える
setup-dialog-replace-subtitle = 別のウォレットで取引します。その秘密鍵はこのデバイス上で暗号化され、外部に送信されることはありません。
setup-dialog-replace-submit = 検証して置き換え
setup-dialog-replace-saved = 置き換えを準備しました。新しいウォレットで { -brand } を再起動しています…
setup-dialog-replace-confirm-intro = 取引ウォレットを置き換えると:
setup-dialog-replace-confirm-stop = トレーダーとコピートレードが停止します。再起動後に再度オンにしてください。
setup-dialog-replace-confirm-backup = 現在のウォレットのポジション、トランザクション、履歴がバックアップフォルダーに移動します。
setup-dialog-replace-confirm-keep = 現在のウォレットの鍵は「その他のウォレット」に残り、引き続きエクスポートできます。
setup-dialog-replace-confirm-funds = 資金は移動しません。トークンと { -sol } は現在のウォレットに残ります。
setup-dialog-replace-confirm-rpc = 設定済みの RPC エンドポイントを入力したものに置き換えます。フィールドが空の場合は現在のエンドポイントが残ります。
setup-dialog-replace-confirm-restart = 新しいウォレットで { -brand } が再起動します。

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = セットアップの進捗
setup-wizard-wallet-security-note = 保存前に暗号化されます。
setup-wizard-rpc-title = RPC エンドポイント
setup-wizard-rpc-input =
    .placeholder = 1行に HTTPS URL を1つ
setup-wizard-rpc-guidance = 信頼できるメインネットのプロバイダーを使ってください。公開エンドポイントには対応していません。
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = 推奨
setup-wizard-account-title = { -brand } アカウント
setup-wizard-account-optional = 任意
setup-wizard-account-loading = アカウントの状態を確認中…
setup-wizard-verify-title = 検証して保存
setup-wizard-verify-list =
    .aria-label = セットアップ検証のステータス
setup-wizard-verify-wallet = ウォレット
setup-wizard-verify-save = 安全な設定
setup-wizard-complete-title = セットアップを保存しました
setup-wizard-reconnect = 接続を再試行
setup-wizard-reload = ダッシュボードを再読み込み
setup-wizard-error-title = セットアップの確認が必要です
setup-wizard-explore = ダッシュボードを見る
setup-wizard-continue = 続ける

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = ウォレット
setup-wizard-step-connection = 接続
setup-wizard-step-verify = 確認
setup-wizard-step-ready = 完了
setup-wallet-title = 取引用ウォレットを設定
setup-wallet-description = { -brand } が取引に使うウォレットです。鍵はこのコンピューター上で暗号化され、外に出ることはありません。
setup-wallet-source =
    .aria-label = ウォレットの取得方法
setup-wallet-create-title = 新しいウォレットを作成
setup-wallet-create-detail = { -brand } が新しいウォレットを作成します。鍵を保存してから、そのウォレットに { -sol } を送ってください。
setup-wallet-import-title = ウォレットをインポート
setup-wallet-import-detail = お持ちの秘密鍵を使います（base58 または 64 個の数値の JSON 配列）。
setup-wallet-creating = ウォレットを作成中…
setup-wallet-create-failed = ウォレットを作成できませんでした。もう一度お試しください。
setup-backup-title = 秘密鍵を保存
setup-backup-description = この鍵を持つ人は誰でもウォレットを操作できます。{ -brand } は鍵を復元できません。
setup-backup-address = アドレス
setup-backup-key = 秘密鍵
setup-backup-copy-address = アドレスをコピー
setup-backup-download = ダウンロード
setup-backup-copied = コピーしました。保存したらクリップボードを消去してください。
setup-backup-machine-warning = { -brand } が保管する暗号化コピーは、このコンピューターでしか開けません。コンピューターを紛失したり買い替えたりした場合、ウォレットを復元できるのは今保存する鍵だけです。紙やパスワードマネージャーなど、オフラインで保管してください。
setup-backup-ack-saved = 秘密鍵をこのコンピューター以外の安全な場所に保存しました。
setup-backup-ack-control = この鍵を持つ人は誰でもこのウォレットの資金を引き出せることを理解しています。
setup-backup-ack-recovery = { -brand } がこの鍵を復元できないことを理解しています。鍵がなければ、コンピューターの紛失は資金の喪失を意味します。
setup-backup-create-another = 別のウォレットを作成
setup-backup-replace-confirm = この鍵を破棄して新しいウォレットを作成しますか？まだ何も送金していない場合にのみ行ってください。
setup-connection-title = { -brand } を { -solana } にどう接続しますか？
setup-connection-description = 後から「設定 → 接続」で変更できます。
setup-connection-choice =
    .aria-label = 接続
setup-connection-service-title = { -brand } サービス
setup-connection-service-badge = RPC 不要
setup-connection-service-detail = 無料の { -brand } アカウントを使います。
setup-connection-service-prices = 価格は数秒ごとに更新されます
setup-connection-service-features = コピートレード、ウォレットのウォッチ、他のウォレットの残高、古い履歴、プールへの直接スワップ、ライブのプール価格にはご自身の RPC が必要です
setup-connection-service-pause = サインアウトしたり screenerbot.io に接続できなかったりすると、取引は一時停止します
setup-connection-service-privacy = 秘密鍵がこのコンピューターから出ることはありません。スワップはここで署名され、screenerbot.io はウォレットの公開アドレスだけを使って残高と取引を読み取ります
setup-connection-own-title = 独自の RPC
setup-connection-own-badge = 最速
setup-connection-own-detail = 価格は毎秒約 2 回更新され、すべての機能を使えます。プロバイダーの RPC エンドポイントが必要です。
setup-connection-signin-required = { -brand } サービスを使うには、サインインするか無料アカウントを作成してください。
setup-connection-reauthorize = { -brand } サービスにウォレットの読み取りを許可するには、もう一度サインインしてください。
setup-connection-sending = { -brand } 経由でトランザクションを送信
setup-connection-sending-hint = サインイン中に利用できます。利用できない場合はご自身の RPC を使います。
setup-wizard-account-required = 必須
setup-wizard-verify-connection = 接続
setup-verify-service-checking = { -brand } サービスを確認中
setup-verify-service-ready = { -brand } サービスの準備完了
setup-verify-service-busy = { -brand } サービスは現在混雑しています。しばらくしてからもう一度試すか、独自の RPC を使ってください。
setup-verify-service-daily-limit = アカウントが本日の { -brand } サービスの上限に達しました。00:00 UTC 以降にもう一度試すか、独自の RPC を使ってください。
setup-verify-service-unreachable = screenerbot.io に接続できません。インターネット接続を確認してください。
setup-verify-service-version = このバージョンは { -brand } サービスでサポートされなくなりました。続けるには { -brand } を更新してください。
setup-verify-service-signin = { -brand } サービスを使うにはサインインしてください。
setup-verify-save-done-service-detail = 秘密鍵を暗号化しました。{ -brand } サービスを選択しました。
setup-action-try-again = もう一度試す
setup-action-use-own-rpc = 独自の RPC を使う
setup-action-sign-in-again = もう一度サインイン
setup-ready-title = { -brand } の準備ができました
setup-fund-title = 取引用ウォレットに入金
setup-fund-waiting = { -sol } の着金を待っています…
setup-fund-received = { $amount } { -sol } を受け取りました
setup-fund-balance-unavailable = 残高を読み込み中… 自動で更新されます。
setup-fund-open-dashboard = ダッシュボードを開く
