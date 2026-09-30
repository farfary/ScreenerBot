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
setup-error-restart-timeout = セットアップは保存されましたが、{ -brand } がまだ再接続していません。

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
setup-verify-save-skipped = 未保存
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
setup-error-gateway-failed = ゲートウェイの設定を保存できませんでした。
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

setup-dialog-title = ウォレットと RPC のセットアップ
setup-dialog-subtitle = Solana ウォレットとプレミアム RPC エンドポイントを接続すると、取引とライブのオンチェーンデータを利用できます。秘密鍵はこのデバイス上で暗号化され、外部に送信されることはありません。
setup-dialog-close =
    .title = 閉じる
    .aria-label = 閉じる
setup-dialog-wallet-label = ウォレットの秘密鍵
setup-dialog-wallet-input =
    .placeholder = Base58 文字列または JSON 配列 [1,2,3,...]
setup-dialog-rpc-label = RPC エンドポイント
setup-dialog-rpc-input =
    .placeholder = https://your-endpoint...（1行に1つ）
setup-dialog-rpc-hint = プレミアムプロバイダー（{ -helius }、{ -quicknode }、{ -alchemy }）を強く推奨します。公開 Solana RPC はレート制限があり、動作しない場合があります。
setup-dialog-submit = 検証して接続
setup-dialog-working = 処理中…
setup-dialog-validating = 検証中…
setup-dialog-saving = 保存中…
setup-dialog-restarting = 再起動中…
setup-dialog-saved = セットアップを保存しました。フルモードで { -brand } を再起動しています…
setup-dialog-error-missing-fields = ウォレットの秘密鍵と、RPC URL を1つ以上入力してください。
setup-dialog-error-validation = 検証に失敗しました。
setup-dialog-error-incomplete = セットアップを完了できませんでした。
setup-dialog-error-restart-helper = 自動再起動ヘルパーを利用できません。しばらくしてからダッシュボードを再読み込みしてください。
setup-dialog-error-unexpected = 予期しないエラーです。

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = セットアップの進捗
setup-wizard-step-credentials = 認証情報
setup-wizard-step-verification = 検証
setup-wizard-step-complete = 完了
setup-wizard-credentials-title = 認証情報を設定
setup-wizard-credentials-description = ローカルウォレットと、信頼できる Solana メインネット RPC エンドポイントを接続します。
setup-wizard-wallet-toggle =
    .title = 秘密鍵を表示
    .aria-label = 秘密鍵を表示
setup-wizard-wallet-security-note = 保存前に暗号化されます。
setup-wizard-rpc-title = RPC エンドポイント
setup-wizard-rpc-input =
    .placeholder = 1行に HTTPS URL を1つ
setup-wizard-rpc-guidance = 継続的なポーリングには、信頼できるメインネット RPC を推奨します。
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = 推奨
setup-wizard-gateway-title = 無料のトランザクション送信
setup-wizard-gateway-hint = サインイン時に利用できます。RPC はフォールバックとして引き続き利用できます。
setup-wizard-account-title = { -brand } アカウント
setup-wizard-account-optional = 任意
setup-wizard-account-loading = アカウントの状態を確認中…
setup-wizard-verify-title = 検証して保存
setup-wizard-verify-list =
    .aria-label = セットアップ検証のステータス
setup-wizard-verify-wallet = ウォレット
setup-wizard-verify-rpc = Solana RPC
setup-wizard-verify-save = 安全な設定
setup-wizard-complete-title = セットアップを保存しました
setup-wizard-reconnect = 接続を再試行
setup-wizard-reload = ダッシュボードを再読み込み
setup-wizard-error-title = セットアップの確認が必要です
setup-wizard-explore = ダッシュボードを見る
setup-wizard-continue = 続ける
