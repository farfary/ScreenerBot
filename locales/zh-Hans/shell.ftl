# Dashboard shell: header, ticker, notification drawer and status bar.

# Source: templates/base.html
# Document title: the page title, then the product name.
shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-brand-logo =
    .alt = { -brand }
# Version label; the number itself is passed as an argument.
shell-version = v{ $version }

## Header

shell-header-brand =
    .aria-label = 打开仪表盘首页
    .title = 仪表盘首页
shell-bot-card =
    .aria-label = 自动交易状态加载中
shell-bot-label = 自动
shell-bot-status-loading = 加载中
shell-bot-today = 今日
shell-explore-control =
    .aria-label = 探索模式。连接钱包和 RPC 端点以启用全部功能
    .title = 连接钱包和 RPC 端点以启用交易、余额和链上实时数据
shell-explore-title = 探索模式
shell-explore-detail = 钱包和 RPC 未连接
shell-explore-action = 完成设置
shell-wallet-card =
    .aria-label = 钱包总值；打开仓位
    .title = 钱包总值（{ -sol } + 代币）· 打开仓位
shell-wallet-worth-label = 总值
shell-wallet-sol-label = { -sol }
shell-wallet-tokens-label = 代币
shell-sol-price-card =
    .aria-label = { -sol } 的 USD 价格，打开图表
    .title = { -sol } 价格 · 点击查看图表
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24h
shell-copy-card =
    .aria-label = 跟单交易；打开跟单交易
    .title = 跟单交易 · 打开跟单交易
shell-copy-label = 跟单
shell-actions-more =
    .aria-label = 更多顶栏操作
    .title = 更多操作
shell-actions-group =
    .aria-label = 顶栏操作
shell-action-search =
    .aria-label = 搜索代币
    .title = 搜索代币（Ctrl/Cmd+K）
shell-action-featured =
    .aria-label = 精选代币
    .title = 精选代币
shell-action-notifications =
    .aria-label = 操作和通知
    .title = 操作和通知
shell-action-restart =
    .aria-label = 重启应用
    .title = 重启应用
shell-action-theme =
    .aria-label = 切换主题
    .title = 切换主题
shell-action-settings =
    .aria-label = 设置
    .title = 设置

## Ticker

shell-ticker-monitoring-segment =
    .title = 流动性池服务正在监控的代币
shell-ticker-monitoring = 监控中：
shell-ticker-filtering-segment =
    .title = 通过/未通过过滤条件的代币
shell-ticker-passed = 通过：
shell-ticker-rejected = 未通过：
shell-ticker-pnl-segment =
    .title = 今日已实现盈亏
shell-ticker-pnl = 今日盈亏：
shell-ticker-rpc-segment =
    .title = 每分钟 RPC 调用数和成功率
shell-ticker-rpc = RPC：
shell-ticker-rpc-per-minute = /分钟
shell-ticker-services-segment =
    .title = 后台服务健康状态
shell-ticker-services-loading = 服务：<strong>加载中</strong>

## Notification drawer

shell-notification-title = 操作
shell-notification-mark-all-read =
    .title = 全部标为已读
shell-notification-clear-all =
    .title = 全部清除
shell-notification-close =
    .aria-label = 关闭
shell-notification-tab-all = 全部
shell-notification-tab-active = 进行中
shell-notification-tab-done = 已完成
shell-notification-tab-failed = 失败
shell-notification-filter-type-all = 全部类型
shell-notification-filter-type-buy = 买入
shell-notification-filter-type-sell = 卖出
shell-notification-filter-type-open = 开仓
shell-notification-filter-type-close = 平仓
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = 部分出场
shell-notification-filter-state-all = 全部状态
shell-notification-filter-state-in-progress = 进行中
shell-notification-filter-state-completed = 已完成
shell-notification-filter-state-failed = 失败
shell-notification-filter-state-cancelled = 已取消
shell-notification-list =
    .aria-label = 通知
shell-notification-empty = 暂无操作
shell-notification-loading-more = 正在加载更多...
shell-notification-back-to-top =
    .title = 回到顶部

## Status bar

shell-status-bar-version = v
shell-status-bar-uptime = 运行
shell-status-bar-memory = 内存
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/分钟
shell-status-bar-trading = 交易
shell-status-bar-positions = 仓位
shell-status-bar-tokens = 代币

# Source: templates/pages/splash.html, scripts/core/splash.js

## Splash

shell-splash-starting = 正在启动 { -brand }
shell-splash-waiting = 正在等待本地核心程序响应。
shell-splash-failed = { -brand } 无法启动
shell-splash-failed-detail = 请查看日志文件，然后重启应用。

# Source: scripts/core/header.js, scripts/core/connectivity_watcher.js, scripts/core/router.js

## Connection state

shell-connection-connected = 核心程序已连接
shell-connection-waiting = 正在等待核心程序…
shell-connection-retry-now = 立即重试
shell-connection-overlay-detail = 无法连接核心程序。交易已暂停，连接恢复后将自动继续。
shell-connection-restored = 核心程序连接已恢复

# Source: scripts/core/header.js
shell-trader-control-failed = 交易引擎控制失败
shell-notification-button-unread = 操作和通知，{ $count } 条未读
shell-restart-confirm-title = 重启机器人
shell-restart-confirm-message =
    确定要重启机器人吗？

    这将：
    • 停止所有服务
    • 重启进程
    • 耗时约 10-15 秒

    所有进行中的操作都将被中断。
shell-restart-confirm-action = 重启
shell-restart-progress = 正在重启机器人
shell-restart-failed = 重启失败
shell-restart-failed-status = 重启失败：{ $status }
shell-restart-helper-unavailable = 自动重启助手不可用。请稍后重新加载仪表盘。

# Source: scripts/core/router.js
shell-page-title-fallback = 仪表盘
shell-page-load-failed = 页面加载失败
shell-page-offline-detail = 目前无法连接核心程序。连接恢复后，此页面将自动加载。

# Source: scripts/core/header_metrics.js

## Auto Trader card

shell-bot-state-explore = 探索
shell-bot-state-halted = 已暂停
shell-bot-state-off = 已关闭
shell-bot-state-waiting = 等待中
shell-bot-state-idle = 空闲
shell-bot-state-entry-paused = 入场已暂停
shell-bot-state-running = 运行中
shell-bot-control-explore = 探索模式下无法使用自动交易。请打开钱包和 RPC 设置。
shell-bot-control-halted = 紧急停止已启用。请打开自动交易控制。
shell-bot-control-off = 自动交易已关闭。点击启用。
shell-bot-control-waiting = 自动交易已启用，正在等待核心服务。点击停用。
shell-bot-control-idle = 自动交易已启用，但两个监控均已关闭。请打开自动交易控制。
shell-bot-control-entry-paused = 亏损保护已暂停入场，出场可继续。请打开自动交易控制。
shell-bot-control-running = 自动交易运行中。点击停用。

## Wallet and copy cards

shell-wallet-card-summary = 钱包总值：{ $equity } { -sol }（现金 { $balance } { -sol }，代币 { $tokens }）；打开仓位
shell-copy-running-live = 实盘 { $count }
shell-copy-running-paper = 模拟 { $count }
shell-copy-value-paused = 已暂停
shell-copy-value-idle = 空闲
shell-copy-sub-active = 已启用 { $active } / { $total }

## Ticker services state

shell-ticker-services-healthy = 服务：<strong>正常</strong>
shell-ticker-services-issues =
    { $count ->
       *[other] 服务：<strong>{ $count } 个问题</strong>
    }

# Source: scripts/core/agent_approvals.js

## Agent approval prompt

shell-agent-request-title = 智能体请求
shell-agent-request-client-fallback = 已配对的智能体
shell-agent-request-message = { $client } 想在 { -brand } 中运行“{ $tool }”。此请求 { $expiry }。
shell-agent-request-message-arguments = { $client } 想在 { -brand } 中运行“{ $tool }”。参数：{ $summary }。此请求 { $expiry }。
shell-agent-request-expires-minutes = 将在 { $minutes } 分钟后过期
shell-agent-request-expires-seconds = 将在 { $seconds } 秒后过期
shell-agent-request-approve = 批准
shell-agent-request-deny = 拒绝

# Source: scripts/core/utils.js, scripts/core/toast.js, scripts/ui/toast.js, scripts/ui/confirmation_dialog.js

## Toasts, dialogs and shared widgets

shell-toast-copied = 已复制 { $label }
shell-toast-copy-failed = 复制失败
shell-toast-still-running = 仍在运行，请查看通知中心
shell-toast-dismiss =
    .aria-label = 关闭
shell-confirm-title = 确认操作
shell-confirm-message = 确定吗？
shell-address-open-solscan = — 在 { -solscan } 中打开
shell-address-copy = 复制地址

# Source: scripts/core/global_chat.js
shell-assistant-label = 助手
shell-assistant-dialog =
    .aria-label = 助手

# Source: scripts/core/status_bar.js
shell-status-bar-trading-active = 运行中
shell-status-bar-trading-inactive = 未运行

# Source: scripts/core/action_toasts.js

## Action toasts

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title }已取消
shell-action-swap-buy-live = 买入中
shell-action-swap-buy-done = 已买入
shell-action-swap-buy-failed = 买入失败
shell-action-swap-sell-live = 卖出中
shell-action-swap-sell-done = 已卖出
shell-action-swap-sell-failed = 卖出失败
shell-action-position-open-live = 正在开仓
shell-action-position-open-done = 已开仓
shell-action-position-open-failed = 开仓失败
shell-action-position-close-live = 正在平仓
shell-action-position-close-done = 已平仓
shell-action-position-close-failed = 平仓失败
shell-action-position-dca-live = 正在加仓
shell-action-position-dca-done = 已加仓
shell-action-position-dca-failed = 加仓失败
shell-action-partial-exit-live = 部分出场
shell-action-partial-exit-done = 部分出场
shell-action-partial-exit-failed = 部分出场失败
shell-action-manual-order-live = 正在下单
shell-action-manual-order-done = 已下单
shell-action-manual-order-failed = 下单失败
shell-action-trade-live = 交易
shell-action-trade-done = 交易完成
shell-action-trade-failed = 交易失败
shell-action-via-router = { $action }（经由 { $router }）
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = 正在避开 { $venue }
shell-action-cost-guard-avoiding-cost = 正在避开 { $venue } · { $cost }
shell-action-cost-guard-avoiding-unnamed = 正在避开某个交易场所
shell-action-cost-guard-avoiding-unnamed-cost = 正在避开某个交易场所 · { $cost }
shell-action-cost-guard-avoided = { $outcome } · 已避免在 { $venue } 支付 { $cost } 租金
shell-action-cost-guard-avoided-unnamed = { $outcome } · 已避免 { $cost } 交易场所租金
shell-action-exit-full = 全部出场
shell-action-exit-percent = 出场 { $percent }

## Exit dialog (ui/exit_dialog.js)

shell-exit-title = 关闭 { -brand }？
shell-exit-description = 请选择关闭应用的方式
shell-exit-minimize = 最小化到托盘
shell-exit-minimize-detail = 继续在后台运行
shell-exit-quit = 退出应用
shell-exit-quit-detail = 完全关闭并停止所有服务

## Image lightbox (ui/image_lightbox.js)

shell-lightbox-save =
    .title = 保存图片
shell-lightbox-close =
    .title = 关闭（ESC）

## Theme control (scripts/theme.js)

shell-theme-light = 浅色
shell-theme-dark = 深色
shell-theme-switch-to-light = 切换到浅色主题
shell-theme-switch-to-dark = 切换到深色主题
