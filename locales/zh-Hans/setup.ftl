# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

# Source: scripts/core/setup_runtime.js

## Wallet key validation

setup-wallet-required = 请输入钱包私钥。
setup-wallet-json-recognized = 已识别 64 字节 JSON 密钥格式。
setup-wallet-json-invalid = 请使用恰好包含 64 个字节值（0–255）的 JSON 数组。
setup-wallet-format-invalid = 请使用 base58 私钥或 64 字节 JSON 数组。
setup-wallet-base58-recognized = 已识别 Base58 密钥格式。

## RPC endpoint validation

setup-rpc-required = 请至少输入一个 RPC 端点。
setup-dialog-rpc-optional-note = 可选：留空以保留已配置的 RPC 端点。
setup-rpc-too-many = RPC 端点不能超过 10 个。
setup-rpc-url-invalid = 每个端点都必须是有效的 HTTPS URL。
setup-rpc-url-credentials = RPC URL 不能包含用户名或密码。
setup-rpc-url-fragment = RPC URL 不能包含片段。
setup-rpc-public-endpoint = 公共 Solana RPC 无法支持持续轮询。
setup-rpc-private-host = RPC 端点不能使用本地或私有网络主机。
setup-rpc-duplicate = 请移除重复的 RPC 端点。
setup-rpc-ready =
    { $count ->
       *[other] { $count } 个 HTTPS 端点已就绪，可开始测试。
    }

## Verification results

setup-wallet-verified = 钱包已验证
setup-wallet-unverified = 无法验证钱包
setup-wallet-address-detail = 地址 { $address }
setup-wallet-format-hint = 请检查私钥格式。
setup-rpc-none-working = 没有可用的主网 RPC
setup-rpc-health-failed = 没有端点通过主网健康检查。
setup-rpc-partial = { $working } 个可用；{ $failed } 个不可用
setup-rpc-verified =
    { $count ->
       *[other] 已验证 { $count } 个主网端点
    }
setup-rpc-fastest = 最快：{ $url }（{ $latency } 毫秒）。
setup-error-request-failed = 请求失败（{ $status }）
setup-error-restart-timeout = { -brand } 尚未重新连接。

# Source: scripts/core/setup.js

## Verification steps

setup-verify-wallet-parsing = 正在解析私钥
setup-verify-wallet-parsing-detail = 正在检查密钥并推导其公开地址。
setup-verify-wallet-waiting = 等待验证
setup-verify-rpc-testing = 正在测试 Solana 主网
setup-verify-rpc-testing-detail =
    { $count ->
       *[other] 正在检查 { $count } 个端点。
    }
setup-verify-rpc-waiting = 等待测试端点
setup-verify-save-waiting = 等待保存
setup-verify-save-running = 正在加密并保存
setup-verify-save-running-detail = 正在将已验证的配置写入此设备。
setup-verify-save-done = 配置已保存
setup-verify-save-done-detail = 私钥已加密；可用的 RPC 端点已存储。
setup-verify-save-failed = 无法保存设置
setup-verify-save-skipped = 未保存。请先解决上述问题，然后重新验证。
setup-verify-request-failed = 验证请求失败
setup-verify-summary-checking = 正在检查您的钱包和 Solana 主网连接。
setup-verify-summary-running = 正在验证您输入的确切凭据。
setup-verify-summary-saving = 凭据已验证。正在安全保存。
setup-verify-summary-failed = 请检查问题，然后重新验证。

## Errors

setup-error-credentials-failed = 凭据验证失败。
setup-error-save-failed = 无法保存设置。
setup-error-verify-failed = 验证失败。
setup-error-explore-failed = 无法启动探索模式。
setup-error-gateway-failed = 无法保存交易发送设置。
setup-action-review-credentials = 检查凭据

## Completion

setup-explore-opening = 正在打开探索模式…
setup-complete-restarting = 正在使用您已验证的配置重启 { -brand }。
setup-complete-finishing = 正在完成重启…
setup-complete-ready = { -brand } 已就绪。正在打开仪表盘…
setup-complete-stored = 您已验证的配置已安全存储在此设备上。

## Wallet controls (shared with the setup dialog)

setup-wallet-show-key = 显示私钥
setup-wallet-hide-key = 隐藏私钥
setup-wallet-copy =
    .aria-label = 复制钱包地址
    .title = 复制钱包地址
setup-wallet-copy-done =
    .aria-label = 钱包地址已复制
    .title = 已复制
setup-wallet-copy-failed =
    .aria-label = 无法复制钱包地址
    .title = 复制失败

# Source: scripts/ui/setup_dialog.js

## Setup dialog

setup-dialog-title = 设置钱包和 RPC
setup-dialog-subtitle = 连接您的 Solana 钱包和高级 RPC 端点，以启用交易和链上实时数据。您的私钥会在此设备上加密，且不会离开此设备。
setup-dialog-close =
    .title = 关闭
    .aria-label = 关闭
setup-dialog-wallet-label = 钱包私钥
setup-dialog-wallet-input =
    .placeholder = Base58 字符串或 JSON 数组 [1,2,3,...]
setup-dialog-submit = 验证并连接
setup-dialog-working = 处理中…
setup-dialog-validating = 正在验证…
setup-dialog-saving = 正在保存…
setup-dialog-restarting = 正在重启…
setup-dialog-saved = 设置已保存，正在以完整模式重启 { -brand }…
setup-dialog-error-validation = 验证失败。
setup-dialog-error-incomplete = 无法完成设置。
setup-dialog-error-restart-helper = 自动重启助手不可用。请稍后重新加载仪表盘。
setup-dialog-error-unexpected = 意外错误。
setup-dialog-replace-title = 更换交易钱包
setup-dialog-replace-subtitle = 使用另一个钱包交易。其私钥在本设备上加密，绝不会离开本设备。
setup-dialog-replace-submit = 验证并更换
setup-dialog-replace-saved = 已准备更换，正在使用新钱包重启 { -brand }…
setup-dialog-replace-confirm-intro = 更换交易钱包将：
setup-dialog-replace-confirm-stop = 停止交易器和跟单交易。重启后请重新开启。
setup-dialog-replace-confirm-backup = 将当前钱包的持仓、交易记录和历史移入备份文件夹。
setup-dialog-replace-confirm-keep = 将当前钱包的密钥保留在“其他钱包”中，你仍可在那里导出它。
setup-dialog-replace-confirm-funds = 不转移任何资金：代币和 { -sol } 仍留在当前钱包中。
setup-dialog-replace-confirm-rpc = 用你输入的 RPC 端点替换已配置的端点；字段留空则保留当前端点。
setup-dialog-replace-confirm-restart = 使用新钱包重启 { -brand }。

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = 设置进度
setup-wizard-wallet-security-note = 保存前会先加密。
setup-wizard-rpc-title = RPC 端点
setup-wizard-rpc-input =
    .placeholder = 每行一个 HTTPS URL
setup-wizard-rpc-guidance = 请使用可靠的主网服务商；不支持公共端点。
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = 推荐
setup-wizard-account-title = { -brand } 账户
setup-wizard-account-optional = 可选
setup-wizard-account-loading = 正在检查账户状态…
setup-wizard-verify-title = 验证并保存
setup-wizard-verify-list =
    .aria-label = 设置验证状态
setup-wizard-verify-wallet = 钱包
setup-wizard-verify-save = 安全配置
setup-wizard-complete-title = 设置已保存
setup-wizard-reconnect = 重试连接
setup-wizard-reload = 重新加载仪表盘
setup-wizard-error-title = 设置需要处理
setup-wizard-explore = 探索仪表盘
setup-wizard-continue = 继续

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = 钱包
setup-wizard-step-connection = 连接
setup-wizard-step-verify = 验证
setup-wizard-step-ready = 就绪
setup-wallet-title = 设置你的交易钱包
setup-wallet-description = { -brand } 用于交易的钱包。其密钥在这台电脑上加密，永远不会离开这台电脑。
setup-wallet-source =
    .aria-label = 钱包来源
setup-wallet-create-title = 创建新钱包
setup-wallet-create-detail = { -brand } 创建一个新钱包。你保存它的密钥，然后向它发送 { -sol }。
setup-wallet-import-title = 导入钱包
setup-wallet-import-detail = 使用你已有的私钥（base58 或包含 64 个数字的 JSON 数组）。
setup-wallet-creating = 正在创建你的钱包…
setup-wallet-create-failed = 无法创建钱包。请重试。
setup-backup-title = 保存你的私钥
setup-backup-description = 任何持有此密钥的人都能控制该钱包。{ -brand } 无法找回它。
setup-backup-address = 地址
setup-backup-key = 私钥
setup-backup-copy-address = 复制地址
setup-backup-download = 下载
setup-backup-copied = 已复制。保存后请清空剪贴板。
setup-backup-machine-warning = { -brand } 保留的加密副本只能在这台电脑上打开。如果电脑丢失或更换，只有你现在保存的密钥能恢复钱包。请离线保存，例如写在纸上或存入密码管理器。
setup-backup-ack-saved = 我已将私钥保存在这台电脑以外的安全位置。
setup-backup-ack-control = 我明白任何持有此密钥的人都能取走这个钱包中的资金。
setup-backup-ack-recovery = 我明白 { -brand } 无法找回此密钥。没有它，电脑丢失就意味着资金丢失。
setup-backup-create-another = 另建一个
setup-backup-replace-confirm = 放弃此密钥并创建新钱包？仅在你尚未向它发送任何资产时这样做。
setup-connection-title = { -brand } 应如何连接到 { -solana }？
setup-connection-description = 你之后可以在“设置 → 连接”中更改。
setup-connection-choice =
    .aria-label = 连接
setup-connection-service-title = { -brand } 服务
setup-connection-service-badge = 无需 RPC
setup-connection-service-detail = 使用你的免费 { -brand } 账户。
setup-connection-service-prices = 价格每隔几秒刷新一次
setup-connection-service-features = 跟单交易、钱包监控、其他钱包余额、较早的历史记录、直接池兑换和实时池价格需要你自己的 RPC
setup-connection-service-pause = 如果你退出登录或无法访问 screenerbot.io，交易会暂停
setup-connection-service-privacy = 你的私钥绝不会离开这台电脑：兑换在本机签名，screenerbot.io 只用你钱包的公开地址读取余额和交易记录
setup-connection-own-title = 你自己的 RPC
setup-connection-own-badge = 最快
setup-connection-own-detail = 价格约每秒刷新两次，并提供全部功能。需要服务商提供的 RPC 端点。
setup-connection-signin-required = 登录或创建免费账户以使用 { -brand } 服务。
setup-connection-reauthorize = 请重新登录，以允许 { -brand } 服务读取钱包。
setup-connection-sending = 通过 { -brand } 发送交易
setup-connection-sending-hint = 登录后可用。无法使用时将改用你的 RPC。
setup-wizard-account-required = 必需
setup-wizard-verify-connection = 连接
setup-verify-service-checking = 正在检查 { -brand } 服务
setup-verify-service-ready = { -brand } 服务已就绪
setup-verify-service-busy = { -brand } 服务当前繁忙。请稍后重试，或使用你自己的 RPC。
setup-verify-service-daily-limit = 你的账户已达到今日 { -brand } 服务上限。请在 00:00 UTC 之后重试，或使用你自己的 RPC。
setup-verify-service-unreachable = 无法访问 screenerbot.io。请检查你的网络连接。
setup-verify-service-version = { -brand } 服务已不再支持此版本。请更新 { -brand } 以继续。
setup-verify-service-signin = 登录以使用 { -brand } 服务。
setup-verify-save-done-service-detail = 私钥已加密；已选择 { -brand } 服务。
setup-action-try-again = 重试
setup-action-use-own-rpc = 使用我自己的 RPC
setup-action-sign-in-again = 重新登录
setup-ready-title = { -brand } 已就绪
setup-fund-title = 为你的交易钱包充值
setup-fund-waiting = 正在等待 { -sol }…
setup-fund-received = 已收到 { $amount } { -sol }
setup-fund-balance-unavailable = 正在读取余额…余额会自动更新。
setup-fund-open-dashboard = 打开仪表盘
