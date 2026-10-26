# Fatal startup errors. Server-only: rendered by src/errors/startup.rs into the
# finished text that the Electron shell displays, never sent to the dashboard.
#
# Remedies are written for the compiled binary. Paths, ports, wallet addresses
# and error details arrive as arguments. Keep the command-line flag, the file
# names and the support handle unchanged.

## Wallet mismatch.

startup-wallet-mismatch-title = ウォレットが変更されました
startup-wallet-mismatch-detail =
    設定内のウォレットが、このコンピューターのローカル履歴に記録されているウォレットと一致しません。

    現在のウォレット: { $current }
    以前のウォレット: { $stored }

    影響を受けるローカルデータ: { $systems }

    これは通常、別の秘密鍵をインポートしたときや、別の設定を復元したときに発生します。取引、ポジション、履歴は以前のウォレットのものであり、新しいウォレットを安全に開始するには消去する必要があります。
startup-wallet-mismatch-systems-default = トランザクション、ポジション、ウォレット履歴
startup-wallet-mismatch-remedy =
    続行するには、以前のウォレットのローカル履歴を消去してください（事前にデータベースが自動でバックアップされます）:

      - アプリ内: 下の「{ $action }」を選択します。
      - ターミナル: screenerbot --clean-wallet-data を実行します。

    オンチェーンの資金には影響しません。リセットされるのは、このコンピューターのローカルの取引 / ポジション履歴のみです。バックアップの保存先:
      { $path }
startup-recovery-reset-wallet = ウォレットデータをリセットして再起動

## Port in use.

startup-port-in-use-title = ネットワークポートが使用中です
startup-port-in-use-detail = ダッシュボードのポート { $address } はすでに使用されています。
startup-port-in-use-remedy = { -brand } が必要とするポートを別のプログラムが使用しています。そのプログラムを終了するか、設定で Web サーバーのポートを変更してから、{ -brand } をもう一度起動してください。

## Another instance is running.

startup-lock-held-title = { -brand } はすでに実行中です
startup-lock-held-detail = このコンピューターでは { -brand } の別のコピーがすでに実行中のため、2つ目は起動できません。
startup-lock-held-remedy = すでに開いているウィンドウに切り替えてください。見当たらない場合は、バックグラウンドの { -brand } プロセスを終了してから、もう一度お試しください。再起動後も解決しない場合は、ロックファイルが古くなっている可能性があります。データフォルダから削除できます（.screenerbot.lock）。

## Configuration.

startup-config-invalid-title = 設定を読み込めませんでした
startup-config-parse-detail = config.toml を解析できませんでした: { $detail }
startup-config-load-parse-detail = 設定の読み込みに失敗しました: config.toml を解析できませんでした: { $detail }
startup-config-parse-remedy = 設定ファイルを読み込めませんでした。データフォルダのバックアップから復元するか、設定を初期状態にリセットして、ウォレットと RPC を再設定してください。
startup-config-load-parse-remedy = 有効な設定を復元するか、セットアップをもう一度完了してください。
startup-option-invalid-title = 起動オプションが無効です
startup-option-invalid-remedy = コマンドラインのオプションが無効です。そのオプションを付けずに { -brand } を起動するか、修正してもう一度お試しください。

## Storage upgrade.

startup-storage-upgrade-title = データをアップグレードできませんでした
startup-storage-upgrade-detail =
    { -brand } は { $database } をこのバージョンにアップグレードできず、変更する前に停止しました。データは変更されていません。

    原因:
    { $error }
startup-storage-upgrade-remedy = 詳細をコピーし、ログファイルと一緒に t.me/screenerbotio_support のサポートへ送ってください。データベースを編集、移動、削除しないでください。修正がインストールされると { -brand } が再び開きます。

## Wallet replacement and wallet data reset.

startup-wallet-replacement-incomplete-title = ウォレットの置き換えを完了できませんでした
startup-wallet-replacement-incomplete-detail =
    { -brand } は新しい取引ウォレットへの切り替え中に停止しました。何も削除されていません。以前のウォレットの鍵はウォレット保管庫に残り、ローカル履歴は元の場所か、ログファイルに記載されたバックアップフォルダーにあります。

    原因:
    { $error }
startup-wallet-replacement-incomplete-remedy = { -brand } を再度起動すると置き換えが再開されます。このメッセージが再び表示される場合は、詳細をコピーしてログファイルと一緒に t.me/screenerbotio_support のサポートに送ってください。wallet_replacement.json とバックアップフォルダーは編集、移動、削除しないでください。
startup-wallet-reset-refused-title = ウォレットデータはリセットされませんでした
startup-wallet-reset-refused-open-positions = 取引ウォレットにオープンポジションがあるため ({ $count })、ウォレットデータはリセットされませんでした。
startup-wallet-reset-refused-positions-unreadable = ポジションストアを読み取れず、オープンポジションがないことを確認できないため、ウォレットのローカル履歴は保持されました。
startup-wallet-reset-refused-remedy = 何も移動されていません。リセットなしで { -brand } を起動し、取引ウォレットのオープンポジションをクローズまたはアーカイブしてから、ウォレットデータをもう一度リセットしてください。
startup-wallet-reset-refused-unreadable-remedy = 何も移動されていません。リセットなしで { -brand } を起動してポジションストアを確認させてから、ウォレットデータをもう一度リセットしてください。このメッセージが再び表示される場合は、ログファイルを t.me/screenerbotio_support のサポートに送ってください。
startup-wallet-reset-incomplete-title = ウォレットデータのリセットを完了できませんでした
startup-wallet-reset-incomplete-detail =
    { -brand } はウォレットデータを { $directory } に移動している途中で停止し、すべてのファイルを元に戻せませんでした。ウォレットのローカル履歴の一部はそのフォルダーに、一部は元の場所にあります。

    原因:
    { $error }
startup-wallet-reset-incomplete-remedy = { -brand } を再度起動せず、{ $directory } 内のファイルを移動、編集、削除しないでください。詳細をコピーしてログファイルと一緒に t.me/screenerbotio_support のサポートに送ってください。

## Generic failures.

startup-generic-title = { -brand } を起動できませんでした
startup-generic-remedy = 詳細はログファイルを確認してから、アプリを再起動してください。解決しない場合は、t.me/screenerbotio_support のサポートにお問い合わせください。
startup-generic-detail = { $error }
startup-failure-directories = 必要なディレクトリを作成できませんでした: { $error }
startup-failure-config-load = 設定の読み込みに失敗しました: { $error }
startup-failure-actions-sync = データベースからアクションを同期できませんでした: { $error }
startup-failure-analysis-init = 分析エンジンを初期化できませんでした: { $error }
startup-failure-assistant-init = アシスタントのチャットエンジンを初期化できませんでした: { $error }
startup-failure-wallets-init = ウォレットを初期化できませんでした: { $error }
startup-failure-wallet-validation = ウォレットの整合性を検証できませんでした: { $error }
startup-failure-storage-phase = ストレージフェーズが予期せず停止しました: { $error }
