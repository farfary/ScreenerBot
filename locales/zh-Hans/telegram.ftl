# Telegram bot text. Server-only: rendered by src/telegram/text.rs, never sent to the dashboard.
#
# Messages are sent as Telegram HTML. The only tags are b, i, u, s, code and pre,
# without attributes; links are built in Rust. A line break is a literal newline.
# Icons are prepended by Rust and never appear here. Copyable values (chat ids)
# arrive as arguments and are wrapped in code inside the message. Keep the
# command names (/status) and the literal ampersand placeable unchanged.

## Reply keyboard words. Icons come from ReplyCommand in src/telegram/reply.rs.

telegram-reply-status = 状态
telegram-reply-balance = 余额
telegram-reply-positions = 仓位
telegram-reply-pause = 暂停
telegram-reply-resume = 恢复
telegram-reply-stop = 停止
telegram-reply-stats = 统计
telegram-reply-menu = 菜单
telegram-reply-help = 帮助

## Inline keyboard buttons.

telegram-button-positions = 仓位
telegram-button-balance = 余额
telegram-button-stats = 统计
telegram-button-tokens = 代币
telegram-button-pause = 暂停
telegram-button-stop = 停止
telegram-button-settings = 设置
telegram-button-refresh = 刷新
telegram-button-menu = 菜单
telegram-button-back = 返回
telegram-button-back-to-menu = 返回菜单
telegram-button-back-to-tokens = 返回代币
telegram-button-cancel = 取消
telegram-button-close-all-positions = 全部平仓
telegram-button-sell-percent = 卖出 { $percent }%
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = 加入黑名单
telegram-button-blacklist-symbol = 将 { $symbol } 加入黑名单
telegram-button-close-position = 平仓
telegram-button-confirm-close = 确认平仓
telegram-button-confirm-close-all = 全部平仓
telegram-button-confirm-sell = 确认卖出 { $percent }%
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = 确认强制停止
telegram-button-confirm-buy = 买入 { $amount } { -sol }
telegram-button-notifications = 通知
telegram-button-trading = 交易
telegram-button-entry-monitor = 入场监控
telegram-button-exit-monitor = 出场监控
telegram-button-auto-trading = 自动交易
telegram-button-force-stop = 强制停止
telegram-button-notify-opened = 开仓
telegram-button-notify-closed = 平仓
telegram-button-notify-partial = 部分出场
telegram-button-notify-dca = DCA
telegram-button-notify-errors = 错误
telegram-button-details = 详情
telegram-button-position = 仓位
telegram-button-sell-more = 继续卖出
telegram-button-more-dca = 继续 DCA
telegram-button-history = 历史
telegram-button-status = 状态
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = 重新验证
telegram-button-previous = 上一页
telegram-button-next = 下一页
telegram-button-passed = 通过
telegram-button-rejected = 未通过
telegram-button-new-24h = 新增（24h）
telegram-button-all-tokens = 全部代币
telegram-button-search-token = 搜索代币
telegram-button-filter-stats = 过滤统计
telegram-button-refresh-stats = 刷新统计
telegram-button-view-position = 查看仓位
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    未知命令：{ $command }

    使用 /help 查看可用命令。
telegram-session-expired =
    <b>会话已过期</b>

    使用 /login 重新验证。
telegram-2fa-required =
    <b>需要 2FA 验证</b>

    请输入您的 6 位验证器代码。
telegram-account-locked =
    <b>账户已锁定</b>

    失败次数过多。
    请在 { $seconds ->
       *[other] { $seconds } 秒
    }后重试。
telegram-code-invalid = 请输入有效的 6 位代码。
telegram-authenticated =
    <b>验证成功！</b>

    您现在可以使用机器人命令。
telegram-wrong-code =
    <b>代码错误</b>

    { $remaining ->
       *[other] 还可尝试 { $remaining } 次。
    }
telegram-auth-required =
    <b>需要身份验证</b>

    请输入您的密码以继续。

    <i>输入密码并发送。</i>
telegram-login-required =
    <b>需要登录</b>

    请输入您的 6 位验证器代码：
telegram-session-activated =
    <b>会话已激活</b>

    尚未配置 2FA。您的会话现已生效。

    <i>提示：在安全设置中启用 2FA 可获得更高的安全性。</i>

## Chat discovery.

telegram-discovery-hello = 您好，{ $name }！
telegram-discovery-default-name = 用户
telegram-discovery-detected = <b>已检测到聊天！</b>
telegram-discovery-details =
    聊天 ID：<code>{ $chat_id }</code>
    类型：{ $chat_type }

    请前往 { -brand } 仪表盘并点击此聊天以选中。
telegram-chat-type-private = 私聊
telegram-chat-type-group = 群组
telegram-chat-type-supergroup = 超级群组
telegram-chat-type-channel = 频道

## Menus.

telegram-menu-title =
    <b>控制面板</b>

    选择一个选项以查看信息或控制机器人。
telegram-menu-positions-empty =
    <b>没有持仓中的仓位</b>

    正在等待新机会…
telegram-menu-positions-title = <b>仓位（{ $count }）</b>
telegram-menu-positions-hint = <i>点击仓位即可管理。</i>
telegram-menu-settings =
    <b>设置</b>

    配置通知和交易参数。
telegram-settings-notifications =
    <b>通知设置</b>

    开启或关闭通知：
telegram-settings-trading =
    <b>交易控制</b>

    开启或关闭交易功能：
telegram-pagination-expired = 分页会话已过期。

## Status commands.

telegram-status-state-stopped = <b>已停止</b>（强制停止已生效）
telegram-status-state-active = <b>运行中</b>
telegram-status-state-paused = <b>已暂停</b>
telegram-status-on = 已启用
telegram-status-off = 已停用
telegram-status-body =
    <b>系统状态</b>

    <b>系统</b>
    状态 — { $state }
    运行时长 — { $uptime }
    版本 — v{ $version }

    <b>交易</b>
    入场 — { $entries }
    出场 — { $exits }
    仓位 — { $positions }
telegram-positions-empty =
    <b>没有持仓中的仓位</b>

    正在等待机会…
telegram-positions-title = <b>持仓中的仓位（{ $count }）</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol }（{ $pnl_pct }%）
telegram-positions-more = <i>另有 { $count } 个…</i>
telegram-positions-summary =
    <b>投资组合摘要</b>
    已投入 — { $invested } { -sol }
    净盈亏 — { $pnl } { -sol }
telegram-balance-body =
    <b>钱包余额</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>每日统计</b>

    仓位 — { $positions }
    已投入 — { $invested } { -sol }
    盈亏 — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } 已就绪！</b>

    交易已<b>启用</b>。

    使用下方键盘控制机器人。
    输入 /help 查看可用命令。
telegram-stop-already = <b>交易已处于停用状态</b>
telegram-stop-done =
    <b>交易已停用</b>

    所有交易监控（入场和出场）均已停止。
    使用 /pause 仅停止入场。
telegram-stop-failed =
    <b>停用交易失败</b>

    错误：{ $detail }
telegram-pause-done =
    <b>入场监控已暂停</b>

    不会再开新仓位。
    出场监控继续运行。
telegram-pause-failed =
    <b>暂停入场失败</b>

    错误：{ $detail }
telegram-resume-done =
    <b>入场监控已恢复</b>

    正在关注入场信号。
telegram-resume-failed =
    <b>恢复入场失败</b>

    错误：{ $detail }
telegram-force-stop-confirm =
    <b>强制停止</b>

    这将立即中止所有交易活动：
    • 不再入场
    • 不再出场（包括止损）
    • 不再执行 DCA
telegram-force-stop-warning = <b>这是紧急操作！</b>
telegram-force-stop-question = 确定要继续吗？
telegram-force-stop-active =
    <b>强制停止已激活</b>

    所有交易均已中止。

    使用 /resume_trading 清除此标志。
telegram-resume-trading-not-stopped =
    <b>交易未处于强制停止状态</b>

    无需任何操作。
telegram-resume-trading-done =
    <b>交易已恢复</b>

    强制停止标志已清除。
    现在可以恢复正常的交易操作。

## Help.

telegram-help-title = <b>{ -brand } 帮助</b>
telegram-help-heading-dashboard = 仪表盘
telegram-help-heading-market = 市场
telegram-help-heading-trading = 交易
telegram-help-heading-safety = 安全
telegram-help-heading-system = 系统
telegram-help-commands-dashboard =
    /status — 系统状态与运行时长
    /stats — 每日表现
    /balance — 钱包余额
    /positions — 持仓中的仓位
telegram-help-commands-market =
    /tokens — 代币浏览器
    /rejected — 被过滤的代币
telegram-help-commands-trading =
    /start — 启用交易系统
    /stop — 停用交易系统
    /pause — 暂停新入场
    /resume — 恢复新入场
    /menu — 交互式菜单
telegram-help-commands-safety =
    /force_stop — <b>紧急中止</b>
    /resume_trading — 清除紧急状态
telegram-help-commands-system =
    /update — 更新状态与安装
    /login — 2FA 验证
telegram-help-tip = <i>提示：点击命令即可运行。</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>已是最新版本</b>

    当前运行 v{ $version }，已自动安装。
telegram-update-up-to-date =
    <b>已是最新版本</b>

    当前运行 v{ $version }。
telegram-update-check-failed =
    <b>检查更新失败</b>

    { $reason }
telegram-update-unreachable = 无法连接到 screenerbot.io。
telegram-update-installing = <b>正在安装 v{ $version }</b>
telegram-update-restarting =
    { -brand } 正在重启以切换到新版本。交易将自动恢复。
telegram-update-install-failed =
    <b>无法安装 v{ $version }</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } 已下载</b>

    此版本同时更新桌面应用，因此需要在该设备上运行其安装程序。请在那里打开设置 → 更新。
telegram-update-downloading =
    <b>正在下载 v{ $version }</b>

    { $percent }%，共 { $size } MB。
telegram-update-available =
    <b>v{ $version } 可用</b>

    { $how }
    下载大小：{ $size } MB。

    它会自动下载；准备就绪后请再次发送 /update。
telegram-update-how-core = 静默安装，需短暂重启。
telegram-update-how-installer = 需要运行一次桌面安装程序。

## Shared values and units. Numbers arrive formatted; only the unit words live here.

telegram-value-unknown = 未知
telegram-value-na = 暂无
telegram-percent-value = { $percent }%
telegram-price-sol = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds } 秒
telegram-duration-minutes = { $minutes } 分钟
telegram-duration-minutes-seconds = { $minutes } 分钟 { $seconds } 秒
telegram-duration-hours = { $hours } 小时
telegram-duration-hours-minutes = { $hours } 小时 { $minutes } 分钟
telegram-duration-days = { $days } 天
telegram-duration-days-hours = { $days } 天 { $hours } 小时
telegram-pnl = { $sol } { -sol }（{ $percent }%）
telegram-amount-sol = { $amount } { -sol }
telegram-error-line = 错误：{ $detail }
telegram-ai-reasoning =
    <b>LLM 分析</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens. Icons come from Rust.

telegram-row-entry = 入场 — { $price } { -sol }
telegram-row-exit = 出场 — { $price } { -sol }
telegram-row-current = 当前 — { $price } { -sol }
telegram-row-invested = 已投入 — { $amount } { -sol }
telegram-row-received = 已收到 — { $amount } { -sol }
telegram-row-value = 价值 — { $amount } { -sol }
telegram-row-total = 总计 — { $amount } { -sol }
telegram-row-tokens = 代币 — { $tokens }
telegram-row-duration = 时长 — { $duration }
telegram-row-reason = 原因 — { $reason }
telegram-row-remaining = 剩余 — { $percent }%
telegram-row-pnl = 盈亏 — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>仓位已开启</b>
telegram-notify-opened-size = 规模 — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = 价格 — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>仓位已平仓</b> — 盈利
telegram-notify-closed-title-loss = <b>仓位已平仓</b> — 亏损
telegram-notify-closed-reason-unspecified = 已平仓
telegram-notify-partial-title = <b>部分出场</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — 已卖出 { $percent }%
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = 已加仓 — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = 均价 — { $price } { -sol }
telegram-notify-severity-critical = <b>严重错误</b>
telegram-notify-severity-error = <b>错误</b>
telegram-notify-severity-warning = <b>警告</b>
telegram-notify-severity-info = <b>信息</b>
telegram-notify-alert-title = <b>交易提醒</b>
telegram-notify-alert-token = 代币：<code>${ $symbol }</code>
telegram-notify-alert-mint = 铸造地址：<code>{ $mint }</code>
telegram-notify-alert-bought = 操作：买入 { $amount } { -sol }
telegram-notify-alert-sold = 操作：卖出 { $amount } { -sol }
telegram-notify-alert-wallet = 钱包：<code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b>（模拟）
telegram-notify-copy-task = 任务：{ $task }
telegram-notify-scheduled-completed = <b>计划任务已完成</b>
telegram-notify-scheduled-failed = <b>计划任务失败</b>
telegram-notify-scheduled-timed-out = <b>计划任务已超时</b>
telegram-notify-scheduled-error = 错误：{ $error }
telegram-notify-summary-title = <b>每日摘要</b> — { $date }
telegram-notify-summary-performance = <b>表现</b>
telegram-notify-summary-trades = 交易 — { $total }（{ $wins }{ $win_icon } { $losses }{ $loss_icon }）
telegram-notify-summary-win-rate = 胜率 — { $percent }%
telegram-notify-summary-pnl = 盈亏 — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = 持仓中的仓位 — { $count }
telegram-notify-started-title = <b>{ -brand } 已启动</b>
telegram-notify-started-version = <b>版本</b> — { $version }
telegram-notify-started-mode = <b>模式</b> — { $mode }
telegram-notify-started-ready = 已准备好交易！
telegram-notify-stopped-title = <b>{ -brand } 已停止</b>
telegram-notify-stopped-reason = <b>原因</b> — { $reason }
telegram-notify-stopped-goodbye = 再见！{ $icon }
telegram-notify-start-mode-normal = 正常
telegram-notify-stop-reason-graceful = 正常关闭
telegram-notify-update-available =
    <b>更新 v{ $version } 可用</b>

    { $how }
    下载大小：{ $size } MB
telegram-notify-update-how-installer = 此版本同时更新桌面应用，因此需要运行一次其安装程序。
telegram-notify-update-ready =
    <b>更新 v{ $version } 已就绪</b>

    { $how }
telegram-notify-update-ready-silent = 发送 /update 即可立即应用，否则将在 { -brand } 下次启动时安装。
telegram-notify-update-ready-installer = 打开设置 → 更新以运行安装程序。
telegram-notify-update-applying =
    <b>正在安装 v{ $version }</b>

    后端正在重启；交易将自动恢复。
telegram-notify-new-tokens =
    <b>过滤提醒</b>

    { $count ->
       *[other] 发现 { $count } 个符合您条件的新代币。
    }
telegram-notify-crash =
    <b>机器人已崩溃！</b>

    <b>位置：</b><code>{ $location }</code>
    <b>错误：</b><code>{ $error }</code>
telegram-notify-crash-restart = 请重启机器人。

## Filter results page.

telegram-filter-results-title = <b>过滤结果</b>（{ $count }）
telegram-filter-results-empty = <i>未找到代币。</i>
telegram-filter-results-page = <i>第 { $page } 页，共 { $total } 页</i>

## Position screens.

telegram-position-not-found = 未找到仓位
telegram-position-no-positions = 没有可平仓的仓位
telegram-position-history-empty =
    <b>交易历史</b>

    暂无已平仓仓位。
telegram-position-history-title = <b>最近交易</b>
telegram-position-history-more = <i>另有 { $count } 笔交易…</i>
telegram-position-confirm-hint = <i>请在 30 秒内确认以执行。</i>
telegram-position-confirm-close-title = <b>确认平仓？</b>
telegram-position-confirm-close-selling = 正在卖出 { $tokens } 个代币
telegram-position-confirm-close-estimated = 预估 — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>请在 30 秒内确认</i>
telegram-position-confirm-sell =
    <b>确认卖出</b>

    代币 — { $symbol }
    数量 — { $percent }%
    代币数 — { $tokens }
telegram-position-confirm-dca =
    <b>确认继续买入</b>

    代币 — { $symbol }
    加仓 — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>全部平仓？</b>

    数量 — { $count }
telegram-position-confirm-close-all-hint =
    <i>这将以市价卖出所有持仓中的仓位。
    请在 30 秒内确认。</i>
telegram-position-confirm-force-stop =
    <b>强制停止</b>

    这将立即中止所有交易：
    • 不再入场
    • 不再出场
    • 不再 DCA
telegram-position-confirm-force-stop-warning = <b>这是紧急操作。</b>
telegram-position-confirm-blacklist =
    <b>将代币加入黑名单？</b>

    代币 — { $symbol }
    铸造地址 — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>这将平掉该仓位，并阻止以后再次入场。</i>
telegram-position-selling = 正在卖出 { $symbol } 的 { $percent }%…
telegram-position-sell-done =
    <b>卖出已执行</b>

    代币 — { $symbol }
    已卖出 — { $percent }%
    已收到 — { $amount } { -sol }
telegram-position-sell-failed = <b>卖出失败</b>
telegram-position-adding = 正在向 { $symbol } 加仓 { $amount } { -sol }…
telegram-position-dca-done =
    <b>DCA 已执行</b>

    代币 — { $symbol }
    已加仓 — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA 失败</b>
telegram-position-closing-all = 正在全部平仓…
telegram-position-close-all-done =
    <b>全部平仓完成</b>

    已平仓 — { $closed }
    失败 — { $failed }
telegram-position-blacklisted =
    <b>代币已加入黑名单</b>

    代币 — { $symbol }
    状态 — 已平仓并加入黑名单

## Token screens.

telegram-token-not-found = 未找到代币
telegram-token-not-found-prefix = 未找到代币。请尝试使用更长的前缀搜索。
telegram-token-stats-failed = 获取统计失败：{ $detail }
telegram-token-list-failed = 获取代币失败：{ $detail }
telegram-token-list-empty = <b>{ $view }</b> 视图中未找到代币。
telegram-token-view-passed = 已通过过滤
telegram-token-view-rejected = 未通过
telegram-token-view-recent = 最近添加
telegram-token-view-all = 全部代币
telegram-token-list-title = <b>{ $name }</b>（第 { $page }/{ $total } 页）
telegram-token-list-stats = 流动性：{ $liquidity } • 价格：{ $price }
telegram-token-list-hint = <i>点击 /token_ID 查看详情</i>
telegram-token-explorer =
    <b>市场浏览器</b>

    <b>概览</b>
    已通过过滤 — { $passed }
    未通过 — { $rejected }
    有实时价格 — { $priced }
    已发现总数 — { $total }

    <i>选择分类进行浏览：</i>
telegram-token-filter-title = <b>过滤分析</b>
telegram-token-filter-distribution = <b>分布</b>
telegram-token-filter-passed = 通过 — { $count }（{ $percent }%）
telegram-token-filter-rejected = 未通过 — { $count }（{ $percent }%）
telegram-token-filter-blacklisted = 黑名单 — { $count }
telegram-token-filter-coverage = <b>覆盖情况</b>
telegram-token-filter-priced = 有流动性池价格 — { $count }
telegram-token-filter-open = 持仓中的仓位 — { $count }
telegram-token-filter-total = 已发现总数 — { $count }
telegram-token-filter-updated = <b>最后更新</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>每 { $interval } 自动刷新</i>
telegram-token-detail-active = <b>持仓中的仓位</b>
telegram-token-detail-price = 价格 — { $price } { -sol }
telegram-token-detail-liquidity = 流动性 — { $value }
telegram-token-detail-volume = 24h 成交量 — { $value }
telegram-token-detail-change = 24h 涨跌 — { $value }
telegram-token-detail-risk = 风险评估：{ $score }/100
telegram-token-detail-risk-unknown = 风险评估：未知
telegram-token-detail-action = <i>选择操作：</i>
telegram-token-search =
    <b>搜索市场</b>

    输入符号或铸造地址进行搜索：

    <i>示例：/token_BONK 或 /token_So11111</i>
telegram-token-confirm-buy =
    <b>确认直接买入</b>

    代币 — ${ $symbol }
    铸造地址 — <code>{ $mint }</code>
    数量 — { $amount } { -sol }

    <i>请在 30 秒内确认以执行。</i>
telegram-token-confirm-blacklist =
    <b>将代币加入黑名单？</b>

    代币 — ${ $symbol }
    铸造地址 — <code>{ $mint }</code>

    <i>这将使该代币无法通过过滤。</i>
telegram-token-blacklisted =
    <b>代币已加入黑名单</b>

    代币 — ${ $symbol }
    状态 — 已加入黑名单
telegram-token-blacklist-failed = <b>加入黑名单失败</b>
telegram-token-buy-processing =
    <b>正在处理买入…</b>

    代币 — ${ $symbol }
    数量 — { $amount } { -sol }
telegram-token-buy-done =
    <b>买入成功</b>

    代币 — ${ $symbol }
    数量 — { $amount } { -sol }

    <i>在 /positions 中查看详情</i>
telegram-token-buy-failed =
    <b>买入失败</b>

    代币 — ${ $symbol }
    错误 — { $detail }
