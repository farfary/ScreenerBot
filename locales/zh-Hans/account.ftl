# Account and ScreenerBot data access status. Each message is the headline;
# `.detail` explains what happens instead. Ids come from DataAccess in
# src/data_server/access.rs.

account-data-access-ready = { -brand } 数据已启用
    .detail = 共享的 K 线、流动性池注册表、安全报告和代币身份信息均由 screenerbot.io 提供。

account-data-access-disabled = { -brand } 数据已关闭
    .detail = { -brand } 数据源已在您的设置中关闭，因此数据仅来自公共提供商。

account-data-access-offline = { -brand } 数据已离线
    .detail = 当前没有网络连接。连接恢复后，数据将自动恢复。

account-data-access-signed-out = { -brand } 数据需要账户
    .detail = 图表、流动性池、安全报告和代币身份信息将改由公共提供商提供，速度较慢、有速率限制，且历史数据较少。登录免费，并且不会改变 { -brand } 的其他运行方式。

account-data-access-reauthorization-required = { -brand } 数据需要您重新登录
    .detail = 此设备是在 { -brand } 数据推出之前授权的。请重新登录以恢复，在此之前将使用公共提供商。

account-data-access-version-unsupported = { -brand } 数据需要更新版本
    .detail = 此版本已不再提供服务。请更新到 { $minimum } 或更高版本以重新使用 { -brand } 数据；在此之前将使用公共提供商。

account-data-access-unreachable = { -brand } 数据无响应
    .detail = 服务未作应答。目前使用公共提供商，{ -brand } 会持续重试。

account-data-access-unknown = { -brand } 数据尚未检查
    .detail = 本次会话中，{ -brand } 尚未需要共享数据。

## Account panel (ui/account/panel.js), shared by Setup and Settings

account-scope-data-read = { -brand } 市场数据
account-scope-rpc-submit = 免费提交已签名交易
account-scope-vote = 代币投票
account-scope-referral-read = 推荐收益
account-scope-account-read = 账户详情

account-panel-request-failed = 操作未成功，请重试。
account-panel-checking = 正在检查账户状态…
account-panel-status-unavailable = 账户状态不可用。
account-panel-browser-notice = 请在浏览器中完成登录，然后返回此处。此面板将自动更新。
account-panel-browser-timeout = 浏览器登录未完成。您可以重新开始。
account-panel-unavailable = 账户功能当前不可用。可不登录，继续设置。
account-panel-retry-status = 重试获取账户状态
account-panel-signed-in-fallback = 已登录
account-panel-features =
    .aria-label = 账户功能
account-panel-sign-out = 退出登录
account-panel-signing-out = 正在退出登录…
account-panel-sign-in = 登录
account-panel-signing-in = 正在登录…
account-panel-sign-in-wallet = 使用钱包登录
account-panel-opening-browser = 正在打开浏览器…
account-panel-continue-browser = 在浏览器中继续
account-panel-sign-in-email = 使用邮箱登录
account-panel-new-to = 初次使用 { -brand }？
account-panel-create-account = 创建账户
account-panel-unlocks-title = 账户包含的功能
account-panel-back-to-options = 返回登录选项
account-panel-email-label = 邮箱
account-panel-email-input =
    .placeholder = you@example.com
account-panel-password-label = 密码
account-panel-password-input =
    .placeholder = 您的密码
account-panel-need-account = 需要账户或忘记密码？
account-panel-open-website = 打开 screenerbot.io
