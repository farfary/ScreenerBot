# Account and ScreenerBot data access status. Each message is the headline;
# `.detail` explains what happens instead. Ids come from DataAccess in
# src/data_server/access.rs.

account-data-access-ready = { -brand } データは有効です
    .detail = 共有のローソク足、プールレジストリ、セキュリティレポート、トークン情報を screenerbot.io から取得しています。

account-data-access-disabled = { -brand } データはオフです
    .detail = 設定で { -brand } のデータソースがオフになっているため、公開プロバイダーのみからデータを取得します。

account-data-access-offline = { -brand } データはオフラインです
    .detail = ネットワークに接続されていません。接続が回復すると、データ取得は自動的に再開されます。

account-data-access-signed-out = { -brand } データにはアカウントが必要です
    .detail = チャート、プール、セキュリティレポート、トークン情報は、代わりに公開プロバイダーから取得されます。速度が遅く、レート制限があり、履歴も少なくなります。サインインは無料で、{ -brand } のそれ以外の動作は変わりません。

account-data-access-reauthorization-required = { -brand } データを使うには再サインインが必要です
    .detail = このデバイスは { -brand } データが提供される前に認可されました。再度サインインすると復旧します。それまでは公開プロバイダーを使用します。

account-data-access-version-unsupported = { -brand } データには新しいバージョンが必要です
    .detail = このバージョンは現在サポートされていません。{ -brand } データを再び使うには、{ $minimum } 以降にアップデートしてください。それまでは公開プロバイダーを使用します。

account-data-access-unreachable = { -brand } データが応答していません
    .detail = サービスから応答がありませんでした。公開プロバイダーを使用し、{ -brand } は再試行を続けます。

account-data-access-unknown = { -brand } データはまだ確認されていません
    .detail = { -brand } は、このセッションでまだ共有データを必要としていません。

## Account panel (ui/account/panel.js), shared by Setup and Settings

account-scope-data-read = { -brand } マーケットデータ
account-scope-rpc-submit = 署名済みトランザクションの無料送信
account-scope-vote = トークン投票
account-scope-referral-read = 紹介報酬
account-scope-account-read = アカウント詳細

account-panel-request-failed = うまくいきませんでした。もう一度お試しください。
account-panel-checking = アカウントの状態を確認中…
account-panel-status-unavailable = アカウントの状態を取得できません。
account-panel-browser-notice = ブラウザでサインインを完了してから、こちらに戻ってください。このパネルが更新されます。
account-panel-browser-timeout = ブラウザでのサインインが完了しませんでした。もう一度開始できます。
account-panel-unavailable = アカウント機能は現在利用できません。サインインせずにセットアップを続けてください。
account-panel-retry-status = アカウントの状態を再確認
account-panel-signed-in-fallback = サインイン済み
account-panel-features =
    .aria-label = アカウント機能
account-panel-sign-out = サインアウト
account-panel-signing-out = サインアウト中…
account-panel-sign-in = サインイン
account-panel-signing-in = サインイン中…
account-panel-sign-in-wallet = ウォレットでサインイン
account-panel-opening-browser = ブラウザを開いています…
account-panel-continue-browser = ブラウザで続行
account-panel-sign-in-email = メールでサインイン
account-panel-new-to = { -brand } は初めてですか？
account-panel-create-account = アカウントを作成
account-panel-unlocks-title = アカウントに含まれるもの
account-panel-back-to-options = サインイン方法に戻る
account-panel-email-label = メールアドレス
account-panel-email-input =
    .placeholder = you@example.com
account-panel-password-label = パスワード
account-panel-password-input =
    .placeholder = パスワード
account-panel-need-account = アカウントが必要ですか？ またはパスワードをお忘れですか？
account-panel-open-website = screenerbot.io を開く
