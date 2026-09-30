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

## Generic failures.

startup-generic-title = { -brand } を起動できませんでした
startup-generic-remedy = 詳細はログファイルを確認してから、アプリを再起動してください。解決しない場合は、t.me/screenerbotio_support のサポートにお問い合わせください。
startup-generic-detail = { $error }
startup-failure-directories = 必要なディレクトリを作成できませんでした: { $error }
startup-failure-config-load = 設定の読み込みに失敗しました: { $error }
startup-failure-actions-init = アクションデータベースを初期化できませんでした: { $error }
startup-failure-actions-sync = データベースからアクションを同期できませんでした: { $error }
startup-failure-strategy-init = ストラテジーシステムを初期化できませんでした: { $error }
startup-failure-analysis-init = 分析エンジンを初期化できませんでした: { $error }
startup-failure-assistant-init = アシスタントのチャットエンジンを初期化できませんでした: { $error }
startup-failure-wallets-init = ウォレットを初期化できませんでした: { $error }
startup-failure-wallet-validation = ウォレットの整合性を検証できませんでした: { $error }
