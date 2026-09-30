# Fatal startup errors. Server-only: rendered by src/errors/startup.rs into the
# finished text that the Electron shell displays, never sent to the dashboard.
#
# Remedies are written for the compiled binary. Paths, ports, wallet addresses
# and error details arrive as arguments. Keep the command-line flag, the file
# names and the support handle unchanged.

## Wallet mismatch.

startup-wallet-mismatch-title = Wallet changed
startup-wallet-mismatch-detail =
    The wallet in your configuration does not match the wallet recorded in this computer's local history.

    Current wallet: { $current }
    Previous wallet: { $stored }

    Affected local data: { $systems }

    This usually happens after importing a different private key or restoring a different configuration. Trading, positions, and history belong to the previous wallet and must be cleared before the new wallet can start safely.
startup-wallet-mismatch-systems-default = Transactions, Positions, Wallet History
startup-wallet-mismatch-remedy =
    Clear the previous wallet's local history to continue (your databases are backed up automatically first):

      - In the app: choose "{ $action }" below.
      - From a terminal: run  screenerbot --clean-wallet-data

    No on-chain funds are affected; only this computer's local trade/position history is reset. Backups are written under:
      { $path }
startup-recovery-reset-wallet = Reset wallet data & restart

## Port in use.

startup-port-in-use-title = Network port is busy
startup-port-in-use-detail = The dashboard port { $address } is already in use.
startup-port-in-use-remedy = Another program is using the port { -brand } needs. Close that program, or change the webserver port in Settings, then start { -brand } again.

## Another instance is running.

startup-lock-held-title = { -brand } is already running
startup-lock-held-detail = Another copy of { -brand } is already running on this computer, so a second one cannot start.
startup-lock-held-remedy = Switch to the window that's already open. If you don't see one, quit any background { -brand } process and try again. If the problem persists after a reboot, the lock file may be stale and can be removed from the data folder (.screenerbot.lock).

## Configuration.

startup-config-invalid-title = Configuration could not be read
startup-config-parse-detail = config.toml could not be parsed: { $detail }
startup-config-load-parse-detail = Failed to load config: config.toml could not be parsed: { $detail }
startup-config-parse-remedy = Your configuration file could not be read. Restore a backup from the data folder, or reset configuration to defaults and set up your wallet and RPC again.
startup-config-load-parse-remedy = Restore a valid configuration or complete setup again.
startup-option-invalid-title = Invalid startup option
startup-option-invalid-remedy = A command-line option is invalid. Start { -brand } without that option, or correct it and try again.

## Generic failures.

startup-generic-title = { -brand } could not start
startup-generic-remedy = Check the log file for details, then restart the app. If the problem persists, contact support at t.me/screenerbotio_support.
startup-generic-detail = { $error }
startup-failure-directories = Failed to create required directories: { $error }
startup-failure-config-load = Failed to load config: { $error }
startup-failure-actions-init = Failed to initialize actions database: { $error }
startup-failure-actions-sync = Failed to sync actions from database: { $error }
startup-failure-strategy-init = Failed to initialize strategy system: { $error }
startup-failure-analysis-init = Failed to initialize analysis engine: { $error }
startup-failure-assistant-init = Failed to initialize Assistant chat engine: { $error }
startup-failure-wallets-init = Failed to initialize wallets: { $error }
startup-failure-wallet-validation = Failed to validate wallet consistency: { $error }
