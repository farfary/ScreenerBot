# Account and ScreenerBot data access status. Each message is the headline;
# `.detail` explains what happens instead. Ids come from DataAccess in
# src/data_server/access.rs.

account-data-access-ready = { -brand } データは有効です
    .detail = チャート、プールデータ、セキュリティレポートを screenerbot.io から読み込んでいます。

account-data-access-disabled = { -brand } データはオフです
    .detail = 設定で { -brand } のデータソースがオフになっているため、公開プロバイダーのみからデータを取得します。

account-data-access-offline = { -brand } データはオフラインです
    .detail = ネットワークに接続されていません。接続が回復すると、データ取得は自動的に再開されます。

account-data-access-signed-out = { -brand } データにはアカウントが必要です
    .detail = チャート、プール、セキュリティレポートは、代わりに公開プロバイダーから取得されます。速度が遅く、レート制限があり、履歴も少なくなります。サインインは無料で、{ -brand } のそれ以外の動作は変わりません。

account-data-access-reauthorization-required = { -brand } データを使うには再サインインが必要です
    .detail = このデバイスは { -brand } データが提供される前に認可されました。再度サインインすると復旧します。それまでは公開プロバイダーを使用します。

account-data-access-version-unsupported = { -brand } データには新しいバージョンが必要です
    .detail = このバージョンは現在サポートされていません。{ -brand } データを再び使うには、{ $minimum } 以降にアップデートしてください。それまでは公開プロバイダーを使用します。

account-data-access-unreachable = { -brand } データが応答していません
    .detail = サービスから応答がありませんでした。公開プロバイダーを使用し、{ -brand } は再試行を続けます。

account-data-access-unknown = { -brand } データはまだ確認されていません
    .detail = { -brand } は、このセッションでまだこのデータを読み込んでいません。

## Account panel (ui/account/panel.js), shared by Setup and Settings

account-scope-data-read = { -brand } マーケットデータ
account-scope-rpc-submit = { -brand } 経由でのトランザクション送信
account-scope-vote = トークン投票
account-scope-referral-read = 紹介報酬
account-scope-account-read = アカウント詳細
account-scope-chain-read = { -brand } サービスによるウォレットの読み取り

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
account-panel-sign-in-code = コードでサインイン
account-panel-getting-code = コードを取得中…
account-panel-code-instructions = 任意のデバイスでこのページを開き、下のコードを入力してこのインストールを承認してください。
account-panel-code-label = あなたのコード
account-panel-code-waiting = 承認を待っています…
account-panel-code-expired = このコードは承認される前に期限切れになりました。新しいコードを取得してもう一度お試しください。
account-panel-session-ended = screenerbot.io でセッションが終了しました。アカウント機能を戻すにはもう一度サインインしてください。
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
