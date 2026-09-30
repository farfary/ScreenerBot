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
setup-error-restart-timeout = 设置已保存，但 { -brand } 尚未重新连接。

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
setup-verify-save-skipped = 未保存
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
setup-error-gateway-failed = 无法保存网关偏好。
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
setup-dialog-rpc-label = RPC 端点
setup-dialog-rpc-input =
    .placeholder = https://your-endpoint...（每行一个）
setup-dialog-rpc-hint = 强烈建议使用高级提供商（{ -helius }、{ -quicknode }、{ -alchemy }）。公共 Solana RPC 有速率限制，可能无法正常工作。
setup-dialog-submit = 验证并连接
setup-dialog-working = 处理中…
setup-dialog-validating = 正在验证…
setup-dialog-saving = 正在保存…
setup-dialog-restarting = 正在重启…
setup-dialog-saved = 设置已保存，正在以完整模式重启 { -brand }…
setup-dialog-error-missing-fields = 请输入钱包私钥和至少一个 RPC URL。
setup-dialog-error-validation = 验证失败。
setup-dialog-error-incomplete = 无法完成设置。
setup-dialog-error-restart-helper = 自动重启助手不可用。请稍后重新加载仪表盘。
setup-dialog-error-unexpected = 意外错误。

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = 设置进度
setup-wizard-step-credentials = 凭据
setup-wizard-step-verification = 验证
setup-wizard-step-complete = 完成
setup-wizard-credentials-title = 配置凭据
setup-wizard-credentials-description = 连接本地钱包和可靠的 Solana 主网 RPC 端点。
setup-wizard-wallet-toggle =
    .title = 显示私钥
    .aria-label = 显示私钥
setup-wizard-wallet-security-note = 保存前会先加密。
setup-wizard-rpc-title = RPC 端点
setup-wizard-rpc-input =
    .placeholder = 每行一个 HTTPS URL
setup-wizard-rpc-guidance = 建议使用可靠的主网 RPC 以支持持续轮询。
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = 推荐
setup-wizard-gateway-title = 免费发送交易
setup-wizard-gateway-hint = 登录后可用。您的 RPC 仍可作为备用。
setup-wizard-account-title = { -brand } 账户
setup-wizard-account-optional = 可选
setup-wizard-account-loading = 正在检查账户状态…
setup-wizard-verify-title = 验证并保存
setup-wizard-verify-list =
    .aria-label = 设置验证状态
setup-wizard-verify-wallet = 钱包
setup-wizard-verify-rpc = Solana RPC
setup-wizard-verify-save = 安全配置
setup-wizard-complete-title = 设置已保存
setup-wizard-reconnect = 重试连接
setup-wizard-reload = 重新加载仪表盘
setup-wizard-error-title = 设置需要处理
setup-wizard-explore = 探索仪表盘
setup-wizard-continue = 继续
