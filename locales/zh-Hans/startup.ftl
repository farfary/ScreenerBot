# Fatal startup errors. Server-only: rendered by src/errors/startup.rs into the
# finished text that the Electron shell displays, never sent to the dashboard.
#
# Remedies are written for the compiled binary. Paths, ports, wallet addresses
# and error details arrive as arguments. Keep the command-line flag, the file
# names and the support handle unchanged.

## Wallet mismatch.

startup-wallet-mismatch-title = 钱包已更改
startup-wallet-mismatch-detail =
    您配置中的钱包与此计算机本地历史记录中的钱包不一致。

    当前钱包：{ $current }
    之前的钱包：{ $stored }

    受影响的本地数据：{ $systems }

    此情况通常出现在导入了不同的私钥或恢复了不同的配置之后。交易、仓位和历史记录属于之前的钱包，必须先清除，新钱包才能安全启动。
startup-wallet-mismatch-systems-default = 交易、仓位、钱包历史
startup-wallet-mismatch-remedy =
    清除之前钱包的本地历史记录后即可继续（您的数据库会先自动备份）：

      - 在应用中：选择下方的“{ $action }”。
      - 在终端中：运行  screenerbot --clean-wallet-data

    链上资金不受影响；仅重置此计算机本地的交易/仓位历史。备份写入到：
      { $path }
startup-recovery-reset-wallet = 重置钱包数据并重启

## Port in use.

startup-port-in-use-title = 网络端口被占用
startup-port-in-use-detail = 仪表盘端口 { $address } 已被占用。
startup-port-in-use-remedy = 另一个程序正在使用 { -brand } 所需的端口。请关闭该程序，或在设置中更改 Web 服务器端口，然后重新启动 { -brand }。

## Another instance is running.

startup-lock-held-title = { -brand } 已在运行
startup-lock-held-detail = 此计算机上已有另一个 { -brand } 实例在运行，因此无法启动第二个。
startup-lock-held-remedy = 请切换到已打开的窗口。如果看不到窗口，请退出所有后台 { -brand } 进程后重试。如果重启电脑后问题仍然存在，可能是锁文件已失效，可从数据文件夹中删除它（.screenerbot.lock）。

## Configuration.

startup-config-invalid-title = 无法读取配置
startup-config-parse-detail = 无法解析 config.toml：{ $detail }
startup-config-load-parse-detail = 加载配置失败：无法解析 config.toml：{ $detail }
startup-config-parse-remedy = 无法读取您的配置文件。请从数据文件夹中恢复备份，或将配置重置为默认值，并重新设置钱包和 RPC。
startup-config-load-parse-remedy = 请恢复有效的配置，或重新完成设置。
startup-option-invalid-title = 启动选项无效
startup-option-invalid-remedy = 某个命令行选项无效。请不带该选项启动 { -brand }，或更正后重试。

## Storage upgrade.

startup-storage-upgrade-title = 无法升级你的数据
startup-storage-upgrade-detail =
    { -brand } 无法将 { $database } 升级到此版本，已在更改之前停止。你的数据未被更改。

    原因：
    { $error }
startup-storage-upgrade-remedy = 复制详细信息，并连同日志文件发送给 t.me/screenerbotio_support 的支持团队。请勿编辑、移动或删除数据库：安装修复后，{ -brand } 会再次打开它。

## Generic failures.

startup-generic-title = { -brand } 无法启动
startup-generic-remedy = 请查看日志文件了解详情，然后重启应用。如果问题仍然存在，请通过 t.me/screenerbotio_support 联系支持。
startup-generic-detail = { $error }
startup-failure-directories = 创建必需的目录失败：{ $error }
startup-failure-config-load = 加载配置失败：{ $error }
startup-failure-actions-init = 初始化操作数据库失败：{ $error }
startup-failure-actions-sync = 从数据库同步操作失败：{ $error }
startup-failure-strategy-init = 初始化策略系统失败：{ $error }
startup-failure-analysis-init = 初始化分析引擎失败：{ $error }
startup-failure-assistant-init = 初始化助手对话引擎失败：{ $error }
startup-failure-wallets-init = 初始化钱包失败：{ $error }
startup-failure-wallet-validation = 验证钱包一致性失败：{ $error }
