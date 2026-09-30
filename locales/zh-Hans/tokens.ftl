# Token data source status. Ids come from build_source_status in
# src/webserver/routes/tokens/source_status.rs. $label is a provider name and is
# not translated.

tokens-result-source-live = 实时市场数据
tokens-result-source-unavailable = { $label } 不可用，正在重试
tokens-result-source-not-listed = 未在 { $label } 上架
tokens-result-security-available = 安全报告可用
tokens-result-security-missing = 无 { -rugcheck } 报告
tokens-result-chart-available = 图表数据可用
tokens-result-chart-missing = 暂无图表数据

# Token details dialog: shared tab states (ui/token_details/state_handling.js)

tokens-state-error-title = 无法加载数据
tokens-state-offline = 您似乎已离线。
tokens-state-request-failed = 多次尝试后请求仍然失败。
tokens-state-waiting = 等待数据…

# Token details dialog: chart (ui/token_details/chart_tab.js)

tokens-chart-marker-entry = 入场
tokens-chart-level-stop-loss = 止损
tokens-chart-level-take-profit = 止盈

# Token details dialog: transactions tab (ui/token_details/transactions_tab.js)

tokens-transactions-loading = 正在加载交易…
tokens-transactions-empty-title = 暂无交易
tokens-transactions-empty-history = 此代币暂无钱包交易历史。
tokens-transactions-empty-data = 此代币暂无交易数据。
tokens-transactions-error-title = 无法加载交易
tokens-transactions-error-message = 交易历史暂时不可用。
tokens-transactions-activity-title = 24h 活动
tokens-transactions-activity-subtitle = 每小时钱包交易
tokens-transactions-metric-total = 总计
tokens-transactions-metric-buys = 买入
tokens-transactions-metric-sells = 卖出
tokens-transactions-recent-title = 最近交易
tokens-transactions-shown = 显示 { $count } 条
tokens-transactions-column-time = 时间
tokens-transactions-column-type = 类型
tokens-transactions-column-price = 价格
tokens-transactions-column-total = 总额
tokens-transactions-chart-missing = 缺少图表库
tokens-transactions-view-solscan = 在 { -solscan } 上查看交易

# Token details dialog: positions tab (ui/token_details/positions_tab.js)

tokens-positions-empty-title = 无仓位
tokens-positions-no-token = 未选择代币。
tokens-positions-empty-message = 此代币暂无仓位。点击买入即可开仓。
tokens-positions-loading = 正在加载仓位…
tokens-positions-from-wallet-history = 来自钱包历史
tokens-positions-frozen = 已冻结，无法卖出
tokens-positions-no-cost-basis = 无成本数据
tokens-positions-history-incomplete = 历史不完整
tokens-positions-dca-count = DCA：{ $count }
tokens-positions-exit-count = 出场：{ $count }
tokens-positions-fact-avg-entry = 平均入场价
tokens-positions-fact-current = 当前
tokens-positions-fact-tokens = 代币
tokens-positions-fact-opened = 开仓时间
tokens-positions-fact-exit-price = 出场价格
tokens-positions-fact-sol-received = 收到的 { -sol }
tokens-positions-fact-closed-reason = 平仓原因
tokens-positions-fact-target-min = 最低盈利目标
tokens-positions-fact-target-max = 最高盈利目标
tokens-positions-fact-highest = 最高价
tokens-positions-fact-lowest = 最低价
tokens-positions-section-range = 目标与区间
tokens-positions-section-market = 市场与持仓
tokens-positions-kicker = 仓位
tokens-positions-fallback-symbol = 代币
tokens-positions-realized-pnl = 已实现盈亏
tokens-positions-unrealized-pnl = 未实现盈亏
tokens-positions-size = 规模

# Token details dialog: security tab (ui/token_details/security_tab.js)
# Risk names and descriptions come from the RugCheck report and render as sent.

tokens-security-analysis-pending = { -rugcheck } 分析进行中…
tokens-security-analyzing = 正在分析安全性…
tokens-security-pulse-title = 安全脉搏
tokens-security-pending-caption = 仍在收集风险信号。
tokens-security-control-title = 代币控制
tokens-security-control-meta = 权限状态
tokens-security-updated = 更新于 { $time }
tokens-security-score-caption = 以 100 为满分的标准化代币风险评分。
tokens-security-score-label = 评分
tokens-security-rugged = 已跑路
tokens-security-grade-analyzing = 分析中
tokens-security-grade-shielded = 受保护
tokens-security-grade-safe = 安全
tokens-security-grade-caution = 谨慎
tokens-security-grade-vulnerable = 易受攻击
tokens-security-grade-unknown = 未知
tokens-security-metric-token-type = 代币类型
tokens-security-metric-total-holders = 持有者总数
tokens-security-metric-lp-providers = LP 提供者
tokens-security-metric-graph-insiders = 图谱内部人
tokens-security-insiders-detected = 已检测到（{ $count }）
tokens-security-insiders-clean = 无
tokens-security-authority-mint = 增发
tokens-security-authority-freeze = 冻结
tokens-security-authority-immutable = 不可变
tokens-security-authority-mutable = 可变
tokens-security-authority-revoked = 已撤销
tokens-security-authority-active = 有效
tokens-security-holder-health-title = 持有者健康度
tokens-security-holders-unique = 独立
tokens-security-creator-share = 创建者占比
tokens-security-gauge-top-10 = 前 10
tokens-security-concentration-unknown = 未知
tokens-security-concentration-critical = 严重
tokens-security-concentration-high = 高
tokens-security-concentration-moderate = 中等
tokens-security-concentration-healthy = 健康
tokens-security-transfer-title = 转账税
tokens-security-transfer-no-fee = 无手续费
tokens-security-transfer-fee-percentage = 手续费比例
tokens-security-transfer-max-fee = 最大手续费金额
tokens-security-transfer-authority = 手续费权限
tokens-security-transfer-note = 每次转账都会收取 { $percent } 的手续费。
tokens-security-transfer-none = 未检测到转账手续费。
tokens-security-risks-title = 安全风险
tokens-security-risks-none = 未检测到安全风险。
tokens-security-risk-fallback-name = 安全信号
tokens-security-risks-critical = { $count } 项严重
tokens-security-risks-warnings =
    { $count ->
       *[other] { $count } 项警告
    }
tokens-security-risks-info = { $count } 项提示
tokens-security-risks-incidents =
    { $count ->
       *[other] 发现 { $count } 起事件
    }
tokens-security-top-holders-title = 前几大持有者
tokens-security-top-holders-concentration = 集中度 { $percent }
tokens-security-insider = 内部人

# Token details dialog: overview tab (ui/token_details/overview_tab.js)
# 5M/1H/6H/24H period codes and the chart timeframe buttons are id codes shared
# with the chart and stay as sent.

tokens-overview-chart-checking = 正在检查数据…
tokens-overview-banner-open = 打开代币横幅
tokens-overview-headline-label = 核心市场指标
tokens-overview-price = 价格
tokens-overview-market-cap = 市值
tokens-overview-liquidity = 流动性
tokens-overview-volume = 成交量
tokens-overview-volume-24h = 24H 成交量
tokens-overview-no-tags = 无标签
tokens-overview-info-title = 代币信息
tokens-overview-profile = 已发布的资料
tokens-overview-fact-mint = 铸造地址
tokens-overview-fact-decimals = 小数位数
tokens-overview-fact-age = 年龄
tokens-overview-fact-dex = DEX
tokens-overview-fact-holders = 持有者
tokens-overview-fact-top-10 = 前 10 持仓
tokens-overview-tags = 标签
tokens-overview-liquidity-title = 流动性与市场
tokens-overview-fact-fdv = FDV
tokens-overview-fact-pool-sol = 池内 { -sol }
tokens-overview-fact-pool-token = 池内代币
tokens-overview-pool = 流动性池
tokens-overview-pulse-title = 市场脉搏
tokens-overview-activity-title = 交易活动
tokens-overview-buy-share = 买入 { $percent }
tokens-overview-buy-sell-ratio = 买卖比 { $ratio }
tokens-overview-buys-24h = 24H 买入
tokens-overview-sells-24h = 24H 卖出
tokens-overview-net-flow = 净流入
tokens-overview-total-24h = 24H 总计
tokens-overview-average-24h = 24H 均值
tokens-overview-spike-5m = 5M 激增
tokens-overview-rate-per-hour = { $amount }/小时
tokens-overview-rate-per-minute = { $amount }/分钟
tokens-overview-spike-factor = { $factor }×
tokens-overview-flow-counts = 买入：{ $buys }（{ $buyPercent }），卖出：{ $sells }（{ $sellPercent }），总计：{ $total }
tokens-overview-flow-no-data = 无交易数据

# Token details dialog: pools tab (ui/token_details/pools_links_tab.js)
# DEX names in pool data render as sent.

tokens-pools-empty-title = 无流动性池
tokens-pools-empty-message = 未检测到此代币的流动性池。
tokens-pools-unknown = 未知
tokens-pools-unknown-dex = 未知 DEX
tokens-pools-total = 流动性池总数
tokens-pools-liquidity = 流动性
tokens-pools-volume-24h = 24h 成交量
tokens-pools-base-role = 基础代币角色
tokens-pools-quote-role = 计价代币角色
tokens-pools-canonical-title = 标准流动性池
tokens-pools-canonical = 标准
tokens-pools-dex = DEX
tokens-pools-summary-title = 流动性池摘要
tokens-pools-breakdown-title = DEX 明细
tokens-pools-all-title = 全部流动性池
tokens-pools-updated = 更新时间
tokens-pools-role-base = 基础
tokens-pools-role-quote = 计价
tokens-pools-role-unknown = 未知
tokens-pools-reserves = 储备账户
tokens-pools-no-reserves = 无储备账户
tokens-pools-address-copy = 复制地址
tokens-pools-address-pool = 流动性池
    .title = 复制流动性池地址
tokens-pools-address-base = 基础代币铸造地址
    .title = 复制基础代币铸造地址
tokens-pools-address-quote = 计价代币铸造地址
    .title = 复制计价代币铸造地址
tokens-pools-address-paired = 配对代币铸造地址
    .title = 复制配对代币铸造地址

# Token details dialog: links tab (ui/token_details/pools_links_tab.js)

tokens-links-empty = 此代币暂无官方网站或社交链接。
tokens-links-info-title = 代币信息
tokens-links-mint-address = 铸造地址
tokens-links-data-source = 数据来源
tokens-links-security = 安全
tokens-links-profile-title = 代币资料
tokens-links-profile-published-title = 已发布的资料内容
tokens-links-profile-published-note = 媒体、描述和官方链接均为付费资料内容，发布前经过审核。这并不代表验证了所有权或代币安全性。
tokens-links-profile-create-note = 为此代币的公开资料添加经审核的徽标、项目描述和官方链接。
tokens-links-profile-update-hint = 在 screenerbot.io 上更新此代币资料
tokens-links-profile-create-hint = 在 screenerbot.io 上创建代币资料
tokens-links-profile-update = 更新资料
tokens-links-profile-create = 创建资料
tokens-links-media-title = 媒体素材
tokens-links-media-fallback-symbol = 代币
tokens-links-media-logo = 徽标
tokens-links-media-banner = 横幅
tokens-links-media-banner-alt = { $symbol } 横幅
tokens-links-media-open = 打开图片
tokens-links-description-title = 描述
tokens-links-explorers-title = 浏览器与分析
tokens-links-websites-title = 官方网站
tokens-links-socials-title = 社交媒体
tokens-links-explorer-solana-explorer = { -solana-explorer }
tokens-links-explorer-geckoterminal = { -geckoterminal }
tokens-links-explorer-dextools = { -dextools }
tokens-links-explorer-coingecko = { -coingecko }
tokens-links-explorer-jupiter-swap = { -jupiter } Swap
tokens-links-social-twitter = { -twitter } / { -x }
tokens-links-social-x = { -x }（{ -twitter }）
tokens-links-social-telegram = { -telegram }
tokens-links-social-discord = { -discord }
tokens-links-social-medium = { -medium }
tokens-links-social-github = { -github }
tokens-links-social-youtube = { -youtube }
tokens-links-social-reddit = { -reddit }
tokens-links-social-facebook = { -facebook }
tokens-links-social-instagram = { -instagram }
tokens-links-social-linkedin = { -linkedin }
tokens-links-social-tiktok = { -tiktok }
tokens-links-social-fallback = 社交

# Token details dialog: frame, header and data sources (ui/token_details_dialog.js)

tokens-dialog-tab-overview = 概览
tokens-dialog-tab-security = 安全
tokens-dialog-tab-positions = 仓位
tokens-dialog-tab-pools = 流动性池
tokens-dialog-tab-links = 链接
tokens-dialog-tab-transactions = 交易
tokens-dialog-sections = 代币详情分区
tokens-dialog-close =
    .title = 关闭（ESC）
    .aria-label = 关闭代币详情
tokens-dialog-unknown-symbol = 未知
tokens-dialog-unknown-name = 未知代币
tokens-dialog-market-summary = 市场摘要
tokens-dialog-price-loading = 正在加载价格
tokens-dialog-unit-sol = { -sol }
tokens-dialog-market-metrics = 市场指标
tokens-dialog-metric-market-cap = 市值
tokens-dialog-metric-volume-24h = 24h 成交量
tokens-dialog-change-24h = 24 小时涨跌 { $change }
tokens-dialog-buy = 买入
    .title = 买入此代币
tokens-dialog-sell = 卖出
    .title = 卖出仓位
tokens-dialog-sell-unavailable = 没有可卖出的持仓中的仓位
tokens-dialog-details = 详情
tokens-dialog-sources = 来源
tokens-dialog-sources-status = 数据源状态
tokens-dialog-updated-label = 更新时间
tokens-dialog-just-now = 刚刚
tokens-dialog-updated-at = 更新于 { $time }
tokens-dialog-updated-unavailable = 更新时间不可用
tokens-dialog-error-title = 无法加载代币数据
tokens-dialog-waiting-token = 等待代币数据…
tokens-dialog-loading-overview = 正在加载概览…
tokens-dialog-loading-security = 正在加载安全信息…
tokens-dialog-loading-pools = 正在加载流动性池…
tokens-dialog-loading-links = 正在加载链接…
tokens-dialog-chart-still-checking = 暂无图表数据，仍在检查…
tokens-dialog-no-data = 暂无数据
tokens-dialog-source-token = 代币
tokens-dialog-source-market = 市场
tokens-dialog-source-security = 安全
tokens-dialog-source-chart = 图表
tokens-dialog-status-pending = 等待中
tokens-dialog-status-loading = 加载中
tokens-dialog-status-ready = 就绪
tokens-dialog-status-unavailable = 不可用
tokens-dialog-status-cached = 已缓存
tokens-dialog-source-summary = { $source }数据：{ $status }
tokens-dialog-badge-pool-price = 池价格
tokens-dialog-badge-pool-price-hint = 来自实时链上流动性池的价格
tokens-dialog-badge-api-price = API 价格
tokens-dialog-badge-api-price-hint = 来自缓存市场数据（API）的价格
tokens-dialog-badge-profile = 已发布的资料
    .title = 付费资料内容，发布前经过审核；并非审计或所有权验证。
tokens-dialog-badge-low-risk-hint = 根据当前 { -rugcheck } 评分为低风险；并非身份验证。
tokens-dialog-badge-immutable = 不可变
tokens-dialog-badge-mutable = 可变
tokens-dialog-badge-auth = 权限：
tokens-dialog-badge-update-authority = 更新权限：
tokens-dialog-badge-position = 仓位
tokens-dialog-badge-blacklisted = 已加入黑名单

# Tokens page: sub-tabs (scripts/pages/tokens/constants.js)
# Ids are the view values of /api/tokens/list.

tokens-view-favorites = 收藏
tokens-view-pool = 流动性池服务
tokens-view-no-market = 无市场数据
tokens-view-all = 全部代币
tokens-view-passed = 已通过
tokens-view-rejected = 未通过
tokens-view-blacklisted = 黑名单
tokens-view-positions = 仓位
tokens-view-recent = 最近
tokens-view-ohlcv = OHLCV 数据

# Tokens page: token cell (scripts/pages/tokens/formatters.js)

tokens-cell-logo-enlarge = 点击放大
tokens-boost-title = 已在 screenerbot.io 上加速 { $boosts } 次
tokens-cell-action-add =
    .title = 加仓（DCA）
    .aria-label = 加仓
tokens-cell-action-sell =
    .title = 卖出（全部或按 % 部分卖出）
    .aria-label = 卖出代币
tokens-cell-action-buy =
    .title = 买入仓位
    .aria-label = 买入代币
tokens-cell-external-links =
    .title = 外部链接
    .aria-label = 外部链接

# Tokens page: table states shared by the token lists (scripts/pages/tokens/*.js)

tokens-table-loading-title = 正在加载代币…
tokens-table-loading-description = 正在准备所选的代币视图。
tokens-table-retry-hint = 请切换标签页或重试。
tokens-filter-all = 全部

# Tokens page: favorites (scripts/pages/tokens/favorites.js)

tokens-favorites-load-failed-title = 无法加载收藏
tokens-favorites-load-failed-toast = 无法加载收藏
tokens-favorites-total = 收藏总数
tokens-favorites-empty-title = 暂无收藏
tokens-favorites-empty-description = 使用搜索（{ $shortcut }）查找代币并添加到收藏。

# Tokens page: OHLCV data view (scripts/pages/tokens/ohlcv.js)
# Status ids come from /api/ohlcv/tokens; priority ids are Priority::as_str in src/ohlcvs/types.rs.

tokens-column-token = 代币
tokens-column-status = 状态
tokens-ohlcv-delete =
    .title = 删除 OHLCV 数据
    .aria-label = 删除 OHLCV 数据
tokens-ohlcv-status-active = 活跃
tokens-ohlcv-status-inactive = 未激活
tokens-ohlcv-priority-critical = 严重
tokens-ohlcv-priority-high = 高
tokens-ohlcv-priority-medium = 中
tokens-ohlcv-priority-low = 低
tokens-ohlcv-column-priority = 优先级
tokens-ohlcv-column-backfill = 回填
tokens-ohlcv-column-data-span = 数据跨度
tokens-ohlcv-column-gaps = 缺口
tokens-ohlcv-column-pools = 流动性池
tokens-ohlcv-column-last-fetch = 上次获取
tokens-ohlcv-timeframe-complete = { $timeframe }：已完成
tokens-ohlcv-timeframe-pending = { $timeframe }：待处理
tokens-ohlcv-load-failed-title = 无法加载 OHLCV 数据
tokens-ohlcv-load-failed-toast = 无法加载 OHLCV 数据
tokens-ohlcv-total = 代币总数
tokens-ohlcv-active = 活跃
tokens-ohlcv-db-size = 数据库大小
tokens-ohlcv-cleanup = 清理未激活
tokens-ohlcv-delete-title = 删除 OHLCV 数据
tokens-ohlcv-delete-message = 删除 { $mint }… 的所有 OHLCV 数据？
tokens-ohlcv-delete-done =
    已删除：{ $candles ->
       *[other] { $candles } 根 K 线
    }，{ $pools ->
       *[other] { $pools } 个流动性池
    }
tokens-ohlcv-delete-failed = 删除 OHLCV 数据失败
tokens-ohlcv-cleanup-title = 删除未激活代币
tokens-ohlcv-cleanup-message = 删除超过指定小时数的未激活代币
tokens-ohlcv-cleanup-placeholder = 小时数…
tokens-ohlcv-cleanup-invalid = 请输入正数
tokens-ohlcv-cleanup-done =
    已清理 { $count ->
       *[other] { $count } 个未激活代币
    }
tokens-ohlcv-cleanup-failed = 清理 OHLCV 数据失败

# Tokens page: token lists (scripts/pages/tokens.js)
# The list statuses shown in the Status column come from row flags, not ids.

tokens-summary-total = 总计
tokens-summary-priced = 有价格
tokens-summary-positions = 仓位
tokens-summary-blacklisted = 黑名单
tokens-search-placeholder = 按符号或铸造地址搜索…
tokens-table-waiting-title = 仍在加载代币…
tokens-table-waiting-description = 正在等待后端响应，将自动重试。
tokens-load-failed-toast = 无法加载代币
tokens-row-data-missing = 未找到代币数据
tokens-column-price-sol = 价格（{ -sol }）
tokens-column-liquidity = 流动性
tokens-column-volume-24h = 24h 成交量
tokens-column-fdv = FDV
tokens-column-market-cap = 市值
tokens-column-change-1h = 1h
tokens-column-change-24h = 24h
tokens-column-txns-5m = 5m 交易
tokens-column-txns-1h = 1h 交易
tokens-column-txns-6h = 6h 交易
tokens-column-txns-24h = 24h 交易
tokens-column-risk-score = 风险评分
tokens-column-reject-reason = 未通过原因
tokens-column-blacklist-reason = 黑名单原因
tokens-column-updated = 更新时间
tokens-column-birth = 创建时间
tokens-column-first-seen = 首次发现
tokens-badge-price = 价格
tokens-badge-ohlcv = OHLCV
tokens-badge-position = 仓位
tokens-badge-blacklisted = 黑名单
tokens-badge-blacklisted-title = 黑名单代币
tokens-badge-blacklisted-reasons = 已加入黑名单：{ $reasons }
tokens-links-menu-copy-mint = 复制铸造地址
tokens-links-copy-failed = 复制铸造地址失败
tokens-lightbox-token-age = 代币年龄

# Global search dialog (scripts/ui/search_dialog.js)

tokens-search-placeholder-dialog = 搜索名称、符号或铸造地址…
tokens-search-input-label = 搜索代币
tokens-search-results-label = 搜索结果
tokens-search-hint = 输入代币名称、符号或粘贴铸造地址
tokens-search-no-matches = 无匹配项，请尝试其他关键词
tokens-search-tip-nav = 导航
tokens-search-tip-open = 打开
tokens-search-tip-close = 关闭
tokens-search-failed = 搜索失败
tokens-search-error = 错误：{ $message }
tokens-search-action-favorite =
    .title = 添加到收藏
    .aria-label = 添加到收藏
tokens-search-action-blacklist =
    .title = 加入黑名单
    .aria-label = 加入黑名单
tokens-search-no-mint = 代币没有铸造地址
tokens-search-open-failed = 打开代币详情失败
tokens-search-copy-failed = 复制到剪贴板失败
tokens-search-favorite-added = 已将 { $symbol } 添加到收藏
tokens-search-favorite-already = 已在收藏中
tokens-search-favorite-failed = 添加到收藏失败
tokens-search-blacklist-message = 将 { $symbol } 加入黑名单？该代币将被排除在交易之外。
tokens-search-blacklist-done = 已将 { $symbol } 加入黑名单
tokens-search-blacklisted = 已加入黑名单
tokens-search-blacklist-failed = 将代币加入黑名单失败

# Featured dialog (scripts/ui/featured_dialog.js)
# Category and source ids are those of CATEGORIES; provider names are terms.

tokens-featured-category-boosted = 已加速
tokens-featured-category-jupiter-organic = { -jupiter } 自然热度榜
tokens-featured-category-jupiter-traded = { -jupiter } 交易榜
tokens-featured-category-dexscreener-trending = { -dexscreener } 热门
tokens-featured-source-jupiter = { -jupiter }
tokens-featured-source-dexscreener = { -dexscreener }
tokens-featured-note-boosted = 由其团队推广
tokens-featured-security-risky = 高风险
tokens-featured-load-failed = 加载精选失败
tokens-featured-network-error = 网络错误：{ $message }
tokens-featured-title = 精选
tokens-featured-subtitle = 优先显示已加速的代币，其后是 Solana 上的热门代币
tokens-featured-boost = 加速代币
tokens-featured-close =
    .title = 关闭（ESC）
tokens-featured-loading = 正在加载精选与热门…
tokens-featured-error-hint = 请检查网络连接或重试
tokens-featured-empty = 当前没有可用的代币
tokens-featured-count =
    { $count ->
       *[other] { $count } 个代币
    }
tokens-featured-stat-market-cap = 市值
tokens-featured-stat-liquidity = 流动性
tokens-featured-stat-volume = 24H 成交量
tokens-featured-stat-holders = 持有者
tokens-featured-stat-txns = 24H 交易
tokens-featured-buy = 买入
    .title = 买入 { $symbol }
tokens-featured-security-score = 安全评分：{ $score }/100
tokens-featured-social-website = 网站
tokens-featured-social-twitter = { -twitter }

# Featured row (scripts/ui/featured_row.js)

tokens-featured-row-view-all = 全部
    .title = 打开完整的精选视图
tokens-featured-row-scroll-start =
    .aria-label = 显示上一批代币
tokens-featured-row-scroll-end =
    .aria-label = 显示更多代币
tokens-featured-row-empty = 暂无精选代币
tokens-featured-row-title = { $name }（{ $symbol }）
tokens-featured-row-boosted-title = { $name }（{ $symbol }）— 已加速 { $boosts } 次

# Pool selector dialog (scripts/ui/pool_selector.js)

tokens-pool-selector-title = 选择流动性池
tokens-pool-selector-loading = 正在加载流动性池…
tokens-pool-selector-empty = 未找到此代币的流动性池
tokens-pool-selector-load-failed = 加载流动性池失败：{ $message }
tokens-pool-selector-count =
    { $count ->
       *[other] 找到 { $count } 个流动性池
    }
tokens-pool-selector-liquidity = 流动性 { $amount }
    .title = 流动性
tokens-pool-selector-volume = 24h { $amount }
    .title = 24h 成交量

# Token identity chips and address rows (scripts/ui/token_identity.js)

tokens-identity-unknown-asset = 未知资产
tokens-identity-copy-address =
    .title = 复制地址
    .aria-label = 复制地址
tokens-identity-copy-signature =
    .title = 复制签名
    .aria-label = 复制签名

tokens-rugcheck-risk-single-holder-ownership = 单一持有者占比高
    .description = 单个持有者持有大量代币供应。
tokens-rugcheck-risk-low-liquidity = 流动性低
    .description = 代币池中的流动性很少。
tokens-rugcheck-risk-few-lp-providers = LP 提供者少
    .description = 只有少数用户提供流动性。
tokens-rugcheck-risk-high-holder-concentration = 持有者集中度高
    .description = 前 10 名持有者持有超过 50% 的代币供应。
tokens-rugcheck-risk-top-10-holders-high-ownership = 前 10 名持有者占比高
    .description = 前 10 名持有者持有超过 70% 的代币供应。
tokens-rugcheck-risk-high-ownership = 持有占比高
    .description = 头部持有者持有超过 80% 的代币供应。
tokens-rugcheck-risk-creator-rug-history = 创建者有跑路记录
    .description = 创建者有代币跑路的历史。
tokens-rugcheck-risk-large-lp-unlocked = 大量 LP 未锁定
    .description = 大量 LP 代币未锁定，所有者可随时撤出流动性。
tokens-rugcheck-risk-mutable-metadata = 元数据可变
    .description = 所有者可以修改代币元数据。
tokens-rugcheck-risk-few-holders = 持有者少
    .description = 持有该代币的钱包不多。
tokens-rugcheck-risk-copycat-token = 仿冒代币
    .description = 此代币使用了已验证代币的符号。
tokens-rugcheck-risk-fee-config-enabled = 已启用费用配置
    .description = 所有者可随时更改费用。
tokens-rugcheck-risk-high-holder-correlation = 持有者相关性高
    .description = 头部持有者持有的供应数量相近。
tokens-rugcheck-risk-freeze-authority-enabled = 冻结权限仍启用
    .description = 代币可能被冻结并禁止交易。
tokens-rugcheck-risk-mint-authority-enabled = 增发权限仍启用
    .description = 所有者可以增发更多代币。
tokens-rugcheck-risk-missing-file-metadata = 缺少元数据文件
    .description = 没有与此代币关联的元数据文件。
tokens-rugcheck-risk-high-market-cap-per-holder = 每位持有者市值高
    .description = 相对于持有者数量，市值非常高。
tokens-rugcheck-risk-symbol-mismatch = 符号不匹配
    .description = 代币符号与其元数据文件不符。
tokens-rugcheck-risk-name-mismatch = 名称不匹配
    .description = 代币名称与其元数据文件不符。
tokens-rugcheck-risk-permanent-control-enabled = 已启用永久控制
    .description = 代币创建者可以永久控制所有代币。
tokens-rugcheck-risk-missing-metadata = 缺少元数据
    .description = 未找到此代币的元数据。
tokens-rugcheck-risk-lp-unlock-soon = LP 即将解锁
    .description = LP 代币即将解锁，届时所有者可撤出流动性。
tokens-rugcheck-risk-lp-vault-unlocked = LP 金库已解锁
    .description = 金库中的 LP 代币可以被取回。
tokens-rugcheck-risk-mint-authority-locked = 增发权限已锁定
    .description = 增发新代币已被锁定。
tokens-rugcheck-risk-high-transfer-fee = 转账税高
    .description = 此代币的每笔转账都需缴纳高额税费。
