# Tools page: the shell, the token tools, the trading tools, the wallet tools and the
# multi-wallet tools.

## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = 工具
tools-category-wallet = 钱包
tools-category-token = 代币
tools-category-single-token = 单个代币
tools-category-utilities = 实用工具
tools-sidebar-hint = 选择一个工具开始使用
tools-help-button =
    .aria-label = 查看此工具的帮助
tools-help-unavailable = 暂无帮助
tools-placeholder-title = 选择工具
tools-placeholder-subtitle = 从侧边栏选择一个工具开始使用
tools-placeholder-hint-wallets = 钱包工具可帮助您管理 Solana 钱包
tools-placeholder-hint-secure = 所有操作均受到保护，并尽可能支持撤销

tools-status-ready = 可使用
tools-status-coming = 即将推出
tools-status-beta = 测试版，可能存在缺陷
tools-status-disabled = 当前已停用
tools-status-badge-coming = 即将推出
tools-status-badge-beta = 测试版
tools-toast-coming-soon = 此工具即将推出
tools-toast-disabled = 此工具当前已停用

## Tool names.

tools-tool-wallet-cleanup-title = 钱包清理
tools-tool-wallet-cleanup-summary = 关闭空 ATA
tools-tool-wallet-cleanup-description = 关闭空的关联代币账户（ATA）以回收 { -sol }
tools-tool-burn-tokens-title = 销毁代币
tools-tool-burn-tokens-summary = 永久销毁代币
tools-tool-burn-tokens-description = 从您的钱包中永久销毁代币
tools-tool-token-analyzer-title = 代币分析器
tools-tool-token-analyzer-summary = 深度代币分析
tools-tool-token-analyzer-description = 对任意 Solana 代币进行多维度深度分析
tools-tool-create-token-title = 创建代币
tools-tool-create-token-summary = 部署新的 SPL 代币
tools-tool-create-token-description = 在 Solana 上部署新的 SPL 代币
tools-tool-trade-watcher-title = 交易监控
tools-tool-trade-watcher-summary = 监控交易并自动执行操作
tools-tool-trade-watcher-description = 监控代币交易并触发自动买入/卖出操作
tools-tool-token-watch-title = 持有者监控
tools-tool-token-watch-summary = 追踪新增代币持有者
tools-tool-token-watch-description = 实时追踪并监控新增代币持有者
tools-tool-buy-multi-wallets-title = 批量买入
tools-tool-buy-multi-wallets-summary = 跨钱包协同买入
tools-tool-buy-multi-wallets-description = 使用随机数量在多个钱包间执行协同买入订单
tools-tool-sell-multi-wallets-title = 批量卖出
tools-tool-sell-multi-wallets-summary = 跨钱包协同卖出
tools-tool-sell-multi-wallets-description = 在多个钱包间执行协同卖出订单，并归集 { -sol }
tools-tool-wallet-consolidation-title = 钱包归集
tools-tool-wallet-consolidation-nav-title = 归集
tools-tool-wallet-consolidation-summary = 归集钱包资金
tools-tool-wallet-consolidation-description = 将子钱包中的 { -sol } 和代币归集回主钱包
tools-tool-airdrop-checker-title = 空投检查器
tools-tool-airdrop-checker-summary = 检查待领取空投
tools-tool-airdrop-checker-description = 检查待领取的空投和可领取的奖励
tools-tool-wallet-generator-title = 钱包生成器
tools-tool-wallet-generator-summary = 生成新密钥对
tools-tool-wallet-generator-description = 安全生成新的 Solana 密钥对

## Shared by the tools

tools-validation-mint-required = 请输入代币铸造地址
tools-validation-mint-format = 代币铸造地址格式无效
tools-validation-mint-invalid = 请输入有效的铸造地址

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = 代币详情
tools-create-token-name-label = 代币名称
tools-create-token-name-input =
    .placeholder = My Token
tools-create-token-symbol-label = 符号
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = 小数位数
tools-create-token-supply-label = 初始供应量
tools-create-token-description-label = 描述
tools-create-token-description-input =
    .placeholder = 代币描述…
tools-create-token-image-title = 代币图片
tools-create-token-image-drop = 将图片拖到此处或点击上传
tools-create-token-image-hint = 推荐：512x512 PNG
tools-create-token-action-preview = 预览
tools-create-token-action-create = 创建代币

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = 正在加载设置…
tools-holder-watch-saved = 持有者监控设置已保存
tools-holder-watch-save-failed = 保存设置失败
tools-holder-watch-save-error = 保存设置时出错
tools-holder-watch-settings-title = 持有者监控设置
tools-holder-watch-enabled-label = 启用持有者监控
tools-holder-watch-interval-label = 检查间隔（秒）
tools-holder-watch-interval-hint = 检查持有者数量的频率（10-3600 秒）
tools-holder-watch-max-tokens-label = 最大监控代币数
tools-holder-watch-max-tokens-hint = 可同时监控的最大代币数量
tools-holder-watch-notify-new-label = 新增持有者时通知
tools-holder-watch-notify-drop-label = 持有者减少时通知
tools-holder-watch-min-change-label = 最小持有者变化
tools-holder-watch-min-change-hint = 触发通知所需的最小持有者变化
tools-holder-watch-drop-percent-label = 持有者下降阈值（%）
tools-holder-watch-drop-percent-hint = 触发警报的下降百分比
tools-holder-watch-action-save = 保存设置
tools-holder-watch-tokens-title = 监控中的代币
tools-holder-watch-token-input =
    .placeholder = 输入代币铸造地址…
tools-holder-watch-empty = 没有正在监控的代币
tools-holder-watch-empty-hint = 在上方添加代币铸造地址即可开始监控
tools-holder-watch-coming-soon = 代币监控功能即将推出

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = 分析代币
tools-analyzer-mint-input =
    .placeholder = 粘贴代币铸造地址…
tools-analyzer-action-analyze = 分析
tools-analyzer-action-analyzing = 分析中…
tools-analyzer-action-copy-report = 复制报告
tools-analyzer-loading = 正在分析代币…
tools-analyzer-failed = 分析代币失败
tools-analyzer-empty = 输入代币铸造地址进行分析
tools-analyzer-empty-hint = 获取任意 Solana 代币的全面洞察
tools-analyzer-tab-overview = 概览
tools-analyzer-tab-security = 安全
tools-analyzer-tab-market = 市场
tools-analyzer-tab-liquidity = 流动性
tools-analyzer-unknown-token = 未知代币

tools-analyzer-favorite-add =
    .title = 添加到收藏
    .aria-label = 添加到收藏
tools-analyzer-favorite-already = 已在收藏中
tools-analyzer-favorite-added = 已将 { $symbol } 添加到收藏
tools-analyzer-favorite-failed = 添加到收藏失败
tools-analyzer-blacklist-add =
    .title = 加入黑名单
    .aria-label = 加入黑名单
tools-analyzer-blacklist-title = 将代币加入黑名单
tools-analyzer-blacklist-message = 将 { $symbol } 加入黑名单？该代币将被排除在交易之外。
tools-analyzer-blacklist-confirm = 加入黑名单
tools-analyzer-blacklisted = 已加入黑名单
tools-analyzer-blacklist-done = 已将 { $symbol } 加入黑名单
tools-analyzer-blacklist-failed = 将代币加入黑名单失败

tools-analyzer-card-quick-stats = 快速统计
tools-analyzer-card-market-summary = 市场摘要
tools-analyzer-card-token-info = 代币信息
tools-analyzer-stat-holders = 持有者
tools-analyzer-stat-decimals = 小数位数
tools-analyzer-stat-safety-score = 安全评分
tools-analyzer-stat-pools = 流动性池
tools-analyzer-stat-volume-24h = 24h 成交量
tools-analyzer-stat-change-24h = 24h 涨跌
tools-analyzer-stat-market-cap = 市值
tools-analyzer-stat-liquidity = 流动性
tools-analyzer-info-mint = 铸造地址
tools-analyzer-info-description = 描述
tools-analyzer-info-supply = 供应量

tools-analyzer-security-empty = 暂无安全数据
tools-analyzer-security-empty-hint = 此代币暂无安全分析
tools-analyzer-card-safety-score = 安全评分
tools-analyzer-score-good = 良好
tools-analyzer-score-moderate = 中等
tools-analyzer-score-risky = 高风险
tools-analyzer-raw-score = 原始风险评分：{ $score }
tools-analyzer-card-authorities = 代币权限
tools-analyzer-authority-mint = 增发权限
tools-analyzer-authority-freeze = 冻结权限
tools-analyzer-authority-transfer-fee = 转账手续费
tools-analyzer-authority-mutable = 可变
tools-analyzer-authority-active = 有效
tools-analyzer-authority-revoked = 已撤销
tools-analyzer-card-holder-concentration = 持有者集中度
tools-analyzer-top-holders = 由前 10 名持有者持有
tools-analyzer-risks-title = 安全风险（{ $count }）
tools-analyzer-risks-title-none = 安全风险
tools-analyzer-risks-none = 未检测到安全风险

tools-analyzer-market-empty = 暂无市场数据
tools-analyzer-market-empty-hint = 此代币暂无市场数据
tools-analyzer-card-price = 当前价格
tools-analyzer-card-price-changes = 价格变化
tools-analyzer-card-volume = 交易量
tools-analyzer-card-transactions = 24h 交易
tools-analyzer-card-valuation = 估值
tools-analyzer-stat-window-1h = 1h
tools-analyzer-stat-window-6h = 6h
tools-analyzer-stat-window-24h = 24h
tools-analyzer-stat-volume-1h = 1h 成交量
tools-analyzer-stat-volume-6h = 6h 成交量
tools-analyzer-stat-fdv = 完全稀释估值
tools-analyzer-txn-buys = 买入
tools-analyzer-txn-sells = 卖出

tools-analyzer-liquidity-empty = 暂无流动性数据
tools-analyzer-liquidity-empty-hint = 未找到此代币的流动性池
tools-analyzer-card-total-liquidity = 总流动性
tools-analyzer-card-pools = 流动性池
tools-analyzer-active-pools =
    { $count ->
       *[other] 活跃流动性池
    }
tools-analyzer-card-pool-details = 流动性池详情
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = 流动性（{ -sol }）
tools-analyzer-pools-column-status = 状态
tools-analyzer-pool-primary = 主池

tools-analyzer-report-empty = 没有可复制的分析
tools-analyzer-report-label = 分析报告
tools-analyzer-report-title = 代币分析报告
tools-analyzer-report-token = 代币：{ $symbol }（{ $name }）
tools-analyzer-report-mint = 铸造地址：{ $mint }
tools-analyzer-report-price = 价格：{ $sol }
tools-analyzer-report-price-with-usd = 价格：{ $sol }（{ $usd }）
tools-analyzer-report-security = 安全：
tools-analyzer-report-safety-score = - 安全评分：{ $score }/100
tools-analyzer-report-mint-authority = - 增发权限：{ $state }
tools-analyzer-report-freeze-authority = - 冻结权限：{ $state }
tools-analyzer-report-risks = - 风险：{ $count }
tools-analyzer-report-market = 市场：
tools-analyzer-report-volume = - 24h 成交量：{ $amount }
tools-analyzer-report-change = - 24h 涨跌：{ $amount }
tools-analyzer-report-market-cap = - 市值：{ $amount }
tools-analyzer-report-liquidity = 流动性：
tools-analyzer-report-liquidity-total = - 总计：{ $amount }
tools-analyzer-report-pools = - 流动性池：{ $count }
tools-analyzer-report-generated = 生成时间：{ $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

tools-watch-type-buy-on-sell = 卖出时买入
tools-watch-type-sell-on-buy = 买入时卖出
tools-watch-type-notify = 通知
tools-watch-type-notify-only = 仅通知

tools-trade-watcher-setup-title = 设置监控
tools-trade-watcher-mint-label = 代币铸造地址
tools-trade-watcher-mint-input =
    .placeholder = 输入代币铸造地址…
tools-trade-watcher-action-search-pools = 搜索流动性池
tools-trade-watcher-pool-label = 已选流动性池
tools-trade-watcher-pool-none = 未选择流动性池
tools-trade-watcher-pool-clear =
    .title = 清除流动性池
tools-trade-watcher-pool-selected = 已选流动性池：{ $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = 监控类型
tools-trade-watcher-type-hint = 卖出时买入：有人卖出时自动买入。买入时卖出：有人买入时自动卖出。
tools-trade-watcher-trigger-label = 触发数量（{ -sol }）
tools-trade-watcher-trigger-hint = 触发操作所需的最小交易规模（以 { -sol } 计）
tools-trade-watcher-action-amount-label = 操作数量（{ -sol }）
tools-trade-watcher-action-amount-hint = 触发时买入/卖出的数量
tools-trade-watcher-slippage-label = 滑点（%）
tools-trade-watcher-slippage-hint = 交易可接受的最大滑点
tools-trade-watcher-active-title = 运行中的监控
tools-trade-watcher-empty = 没有运行中的监控
tools-trade-watcher-empty-hint = 在上方配置监控，然后点击“开始监控”即可开始
tools-trade-watcher-action-start = 开始监控
tools-trade-watcher-action-starting = 启动中…
tools-trade-watcher-action-stop-all = 全部停止
tools-trade-watcher-action-stopping = 停止中…
tools-trade-watcher-started = 已开始监控 { $token }…
tools-trade-watcher-start-failed = 启动监控失败
tools-trade-watcher-stopped = 监控已停止
tools-trade-watcher-stop-failed = 停止监控失败
tools-trade-watcher-stopped-all = 已停止所有监控
tools-trade-watcher-stop-all-failed = 停止监控失败
tools-trade-watcher-load-failed = 加载监控失败
tools-trade-watcher-column-token = 代币
tools-trade-watcher-column-type = 类型
tools-trade-watcher-column-trigger = 触发
tools-trade-watcher-column-action = 操作
tools-trade-watcher-column-triggered = 已触发
tools-trade-watcher-stop-watch =
    .title = 停止监控

## Results returned by the tools backend.

tools-burn-failure-native-asset = 无法销毁 { -sol }
tools-burn-failure-open-position = 无法销毁持仓中的仓位所含代币
tools-burn-failure-account-not-found = 未找到代币账户
tools-burn-failure-zero-balance = 代币余额已为零
tools-burn-failure-transaction = 交易失败
tools-burn-warning-open-position = 无法销毁持仓中的仓位所含代币
tools-burn-warning-closed-position = 已平仓仓位的剩余代币
tools-burn-warning-worth = 价值约 { $amount } { -sol }
tools-multi-buy-warning-insufficient = 余额不足。需要 { $needed } { -sol }，当前 { $have } { -sol }
tools-multi-buy-warning-over-limit = 所需 { -sol } 总额（{ $needed }）超过限额（{ $limit }）
tools-multi-sell-warning-no-wallets = 未找到副钱包
tools-multi-sell-warning-no-balance = 没有钱包持有该代币余额
tools-multi-op-buy-failed = 买入失败
tools-multi-op-sell-failed = 卖出失败
tools-multi-op-transfer-failed = 转账失败
tools-multi-op-balance-failed = 获取余额失败
tools-multi-op-mint-invalid = 铸造地址无效
tools-multi-buy-session-failed = 批量买入失败
tools-multi-sell-session-failed = 批量卖出失败
tools-multi-session-aborted = 操作已被用户中止

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = 扫描钱包
tools-wallet-action-scanning = 扫描中…
tools-wallet-scan-failed = 扫描失败：{ $reason }
tools-wallet-amount-approx = ~{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
       *[other] 已选择：{ $count } 个钱包
    }
tools-wallet-transfer-failed = 转账失败：{ $reason }
tools-wallet-cleanup-failed = 清理失败：{ $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = 扫描结果
tools-wallet-cleanup-stat-empty = 空 ATA
tools-wallet-cleanup-stat-reclaimable = 可回收 { -sol }
tools-wallet-cleanup-stat-failed = 失败（已缓存）
tools-wallet-cleanup-prompt = 点击“扫描钱包”查找空 ATA
tools-wallet-cleanup-prompt-hint = 此操作将检查您钱包中的所有代币账户
tools-wallet-cleanup-action-cleanup = 全部清理
tools-wallet-cleanup-action-cleaning = 清理中…
tools-wallet-cleanup-scanning = 正在扫描钱包…
tools-wallet-cleanup-found =
    { $count ->
       *[other] 发现 { $count } 个空 ATA，价值约 { $amount }
    }
tools-wallet-cleanup-clean = 未发现空 ATA，钱包很干净！
tools-wallet-cleanup-scan-failed = 扫描 ATA 失败
tools-wallet-cleanup-done =
    { $count ->
       *[other] 已清理 { $count } 个 ATA
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = 销毁代币
tools-burn-info-title = 什么是销毁？
tools-burn-info-body = 销毁会永久销毁代币，且无法恢复。销毁后，请运行钱包清理以关闭空 ATA，每个代币可回收约 0.002 { -sol } 租金。
tools-burn-stat-total = 代币总数
tools-burn-stat-selected = 已选择
tools-burn-stat-rent = 可回收租金
tools-burn-prompt = 点击“扫描钱包”查找代币
tools-burn-scanning = 正在扫描钱包中的代币…
tools-burn-scan-failed = 扫描代币失败
tools-burn-empty = 钱包中未发现代币
tools-burn-action-burn = 销毁所选（{ $count }）
tools-burn-action-burning = 销毁中…
tools-burn-cannot-burn = 无法销毁
tools-burn-no-value = 无价值

tools-burn-category-open-position = 持仓中的仓位
tools-burn-category-has-value = 有价值
tools-burn-category-closed-position = 已平仓仓位
tools-burn-category-zero-liquidity = 零流动性
tools-burn-category-hint-open-position = 无法销毁持仓中的仓位所含代币
tools-burn-category-hint-has-value = 建议卖出而非销毁
tools-burn-category-hint-closed-position = 已平仓交易的剩余代币
tools-burn-category-hint-zero-liquidity = 可安全销毁，无市场价值

tools-burn-confirm-title = 确认销毁
tools-burn-confirm-message =
    { $count ->
       *[other] 确定要销毁 <strong>{ $count }</strong> 个代币吗？
    }
tools-burn-confirm-value = 预估总价值：<strong>{ $amount }</strong>
tools-burn-confirm-continue = 继续
tools-burn-final-title = 最终警告
tools-burn-final-headline = 此操作不可撤销！
tools-burn-final-message =
    { $count ->
       *[other] 以下 { $count } 个代币将被永久销毁，在任何情况下都无法恢复。
    }
tools-burn-final-confirm = 是，销毁代币
tools-burn-toast-burned =
    { $total ->
       *[other] 已销毁 { $successful }/{ $total } 个代币。运行钱包清理可回收约 { $amount }
    }
tools-burn-toast-failed =
    { $count ->
       *[other] { $count } 个代币销毁失败
    }
tools-burn-failed = 销毁失败：{ $reason }
tools-burn-failures-title =
    { $count ->
       *[other] { $count } 个代币无法销毁
    }
tools-burn-failure-unknown = 未报告原因

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = 关于
tools-airdrop-about-body = 检查主流 Solana 协议中待领取的空投、可领取的奖励和未领取的份额。
tools-airdrop-list-title = 可领取的空投
tools-airdrop-prompt = 点击“检查空投”扫描可领取的项目
tools-airdrop-action-check = 检查空投
tools-airdrop-action-claim-all = 全部领取

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = 生成器选项
tools-generator-warning-title = 请妥善保管您的私钥！
tools-generator-warning-body = 生成的密钥对在本地创建，不会被传输。请务必将密钥备份到安全的位置。
tools-generator-count-label = 钱包数量
tools-generator-vanity-label = 靓号地址（以指定字符开头）
tools-generator-prefix-label = 前缀
tools-generator-prefix-input =
    .placeholder = 例如 SOL
tools-generator-prefix-hint = 前缀越长，生成时间呈指数级增长
tools-generator-list-title = 已生成的钱包
tools-generator-empty = 尚未生成钱包
tools-generator-action-generate = 生成
tools-generator-action-generating = 生成中…
tools-generator-count-invalid = 请输入 1 到 10 之间的数字
tools-generator-no-keypairs = 未返回密钥对
tools-generator-generated =
    { $count ->
       *[other] 已生成 { $count } 个钱包
    }
tools-generator-failed = 生成钱包失败：{ $reason }
tools-generator-copy-public-key =
    .title = 复制公钥
tools-generator-copy-private-key =
    .title = 复制私钥
tools-generator-remove =
    .title = 从列表中移除
tools-generator-reveal =
    .title = 显示私钥
tools-generator-public-key-label = 公钥：
tools-generator-private-key-label = 私钥：
tools-generator-public-key-name = 公钥
tools-generator-private-key-copied = 私钥已复制
tools-generator-private-key-warning = 任何持有此密钥的人都可控制该钱包
tools-generator-export-empty = 没有可导出的钱包
tools-generator-exported = 钱包已导出，请妥善保管

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = 摘要
tools-consolidation-stat-wallets = 子钱包
tools-consolidation-stat-native = { -sol } 总额
tools-consolidation-stat-tokens = 代币种类
tools-consolidation-stat-rent = 可回收租金
tools-consolidation-wallets-title = 钱包
tools-consolidation-loading-wallets = 正在加载钱包…
tools-consolidation-loading-data = 正在加载钱包数据…
tools-consolidation-action-transfer-native = 转移 { -sol }
tools-consolidation-action-transfer-tokens = 转移所有代币
tools-consolidation-action-cleanup = 清理 ATA
tools-consolidation-action-transferring = 转移中…
tools-consolidation-column-name = 名称
tools-consolidation-column-native = { -sol } 余额
tools-consolidation-column-tokens = 代币
tools-consolidation-column-atas = 空 ATA
tools-consolidation-empty = 未找到子钱包
tools-consolidation-empty-hint = 使用批量买入创建子钱包即可开始
tools-consolidation-load-failed = 加载失败：{ $reason }
tools-consolidation-select-prompt = 选择要归集的钱包
tools-consolidation-selection-totals = | { $amount } | { $tokens ->
       *[other] { $tokens } 个代币
    } | { $atas ->
       *[other] { $atas } 个空 ATA
    }
tools-consolidation-transferred-native = 已将 { $amount } 转入主钱包
tools-consolidation-transferred-tokens =
    { $count ->
       *[other] 已将 { $count } 个代币转入主钱包
    }
tools-consolidation-cleaned =
    { $count ->
       *[other] 已关闭 { $count } 个 ATA，回收 { $amount }
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = 代币
tools-multi-mint-label = 代币铸造地址
tools-multi-mint-input =
    .placeholder = 粘贴代币铸造地址…
tools-multi-execution-title = 执行设置
tools-multi-delay-min-label = 最小延迟（毫秒）
tools-multi-delay-max-label = 最大延迟（毫秒）
tools-multi-concurrency-label = 并发数
tools-multi-concurrency-sequential = { $count }（顺序）
tools-multi-concurrency-parallel = { $count } 并行
tools-multi-slippage-label = 滑点（%）
tools-multi-router-label = 路由
tools-multi-router-auto = 自动（最佳路由）
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = 直连流动性池
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = 进度
tools-multi-progress-preparing = 准备中…
tools-multi-status-line = { $label }（{ $completed }/{ $total }）
tools-multi-column-wallet = 钱包
tools-multi-column-route = 路径
tools-multi-column-status = 状态
tools-multi-op-completed = 已完成
tools-multi-op-failed = 失败
tools-multi-action-stop = 停止
tools-multi-action-loading = 加载中…
tools-multi-start-failed = 启动失败：{ $reason }

tools-multi-state-pending = 待处理
tools-multi-state-funding = 注资中
tools-multi-state-executing = 执行中
tools-multi-state-consolidating = 归集中
tools-multi-state-completed = 已完成
tools-multi-state-failed = 失败
tools-multi-state-aborted = 已中止

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = 您想要跨多个钱包买入的代币
tools-multi-buy-wallets-title = 钱包设置
tools-multi-buy-wallet-count-label = 钱包数量
tools-multi-buy-wallet-count-option =
    { $count ->
       *[other] { $count } 个钱包
    }
tools-multi-buy-wallet-count-hint = 要使用的子钱包数量
tools-multi-buy-buffer-label = 每个钱包的 { -sol } 缓冲
tools-multi-buy-buffer-hint = 预留用于手续费（最少 0.015 { -sol }）
tools-multi-buy-amounts-title = 数量设置
tools-multi-buy-min-label = 每个钱包最小 { -sol }
tools-multi-buy-min-hint = 最小买入数量
tools-multi-buy-max-label = 每个钱包最大 { -sol }
tools-multi-buy-max-hint = 最大买入数量
tools-multi-buy-limit-label = { -sol } 总限额（可选）
tools-multi-buy-limit-hint = 最大总支出
tools-multi-buy-preview-title = 预览
tools-multi-buy-preview-create = 待创建钱包
tools-multi-buy-preview-amount = 每个钱包数量
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = 所需 { -sol } 总额
tools-multi-buy-preview-balance = 主钱包余额
tools-multi-buy-action-preview = 预览
tools-multi-buy-action-start = 开始批量买入
tools-multi-buy-executing = 正在执行买入…
tools-multi-buy-column-spent = 已花费 { -sol }
tools-multi-buy-column-tokens = 代币
tools-multi-buy-preview-failed = 预览失败：{ $reason }
tools-multi-buy-started = 批量买入已开始
tools-multi-buy-stopped = 批量买入已停止
tools-multi-buy-completed = 批量买入已完成！成功 { $successful }/{ $total }

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = 输入代币地址以扫描持有该代币的钱包
tools-multi-sell-action-scan = 扫描
tools-multi-sell-settings-title = 卖出设置
tools-multi-sell-percent-label = 卖出比例
tools-multi-sell-percent-hint = 每个钱包卖出代币的百分比
tools-multi-sell-min-fee-label = 手续费所需最小 { -sol }
tools-multi-sell-min-fee-hint = 交易手续费所需的最小 { -sol }
tools-multi-sell-topup-label = 需要时自动充值
tools-multi-sell-topup-hint = 子钱包余额不足时，从主钱包转入 { -sol }
tools-multi-sell-post-title = 卖出后操作
tools-multi-sell-consolidate-label = 将 { -sol } 归集到主钱包
tools-multi-sell-consolidate-hint = 将子钱包中的所有 { -sol } 转回主钱包
tools-multi-sell-close-atas-label = 卖出后关闭代币 ATA
tools-multi-sell-close-atas-hint = 每个 ATA 可回收约 0.002 { -sol }
tools-multi-sell-wallets-title = 持有该代币的钱包
tools-multi-sell-empty = 没有子钱包持有该代币
tools-multi-sell-column-tokens = 代币
tools-multi-sell-column-native = { -sol } 余额
tools-multi-sell-column-topup = 需要充值
tools-multi-sell-none-selected = 未选择钱包
tools-multi-sell-select-required = 请至少选择一个钱包
tools-multi-sell-action-start = 开始批量卖出
tools-multi-sell-executing = 正在执行卖出…
tools-multi-sell-column-sold = 已卖出代币
tools-multi-sell-column-received = 已收到 { -sol }
tools-multi-sell-started = 批量卖出已开始
tools-multi-sell-stopped = 批量卖出已停止
tools-multi-sell-completed = 批量卖出已完成！已收到 { $amount }

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = 收藏
tools-favorites-saved = 已保存的收藏
tools-favorites-save-current = 保存当前
tools-favorites-empty = 尚未保存收藏
tools-favorites-no-label = 无标签
tools-favorites-uses = { $count }x
tools-favorites-remove = 移除
tools-favorites-loaded = 已加载收藏：{ $name }
tools-favorites-default-name = 配置
tools-favorites-mint-required = 请先输入代币铸造地址
tools-favorites-add-title = 添加收藏
tools-favorites-add-message = 为此收藏输入标签
tools-favorites-add-placeholder = 标签（可选）…
tools-favorites-saved-toast = 已保存到收藏
tools-favorites-save-failed = 保存收藏失败
tools-favorites-remove-title = 移除收藏
tools-favorites-remove-message = 移除此收藏？
tools-favorites-removed-toast = 收藏已移除
tools-favorites-remove-failed = 移除收藏失败
