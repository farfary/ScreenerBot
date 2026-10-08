# API error messages. Each key names the failed operation; technical causes are
# carried separately in the response `details` and are never part of a message.

errors-with-details = { $message }：{ $details }

# Configuration
errors-config-save-failed = 保存配置失败

# Authentication
errors-auth-current-password-incorrect = 当前密码不正确
errors-auth-current-password-required = 修改密码需要输入当前密码
errors-auth-password-too-short = 密码至少需要 4 个字符
errors-auth-password-too-long = 密码最多 128 个字符
errors-auth-hash-failed = 密码哈希处理失败
errors-auth-not-enabled = 未启用身份验证
errors-auth-no-password = 尚未配置密码
errors-auth-password-incorrect = 密码不正确
errors-auth-required = 需要身份验证。请登录后访问此接口。
errors-auth-totp-invalid = 2FA 验证码无效或已过期
errors-auth-totp-verify-failed = 验证 2FA 验证码失败
errors-auth-totp-password-required = 启用 2FA 前必须先设置密码
errors-auth-totp-uri-failed = 生成 TOTP URI 失败
errors-auth-totp-qr-failed = 生成二维码失败
errors-auth-totp-secret-required = 密钥为必填项
errors-auth-totp-save-failed = 保存 TOTP 配置失败
errors-auth-totp-code-invalid = 验证码无效。请检查验证码后重试。
errors-auth-totp-code-verify-failed = 验证验证码失败

# Request security
errors-security-invalid-local-request = 请求必须来自本地仪表盘
errors-security-invalid-token = 安全令牌无效
errors-security-token-required = 需要安全令牌。此接口仅可在 { -brand } 内部访问。

# Lockscreen
errors-lockscreen-no-password-set = 尚未设置密码
errors-lockscreen-invalid-type = 密码类型无效。必须为 'pin4'、'pin6' 或 'text'
errors-lockscreen-invalid-format = 密码与所选类型不匹配
errors-lockscreen-no-current-password = 当前未设置密码
errors-lockscreen-password-incorrect = 密码不正确
errors-lockscreen-enable-needs-password = 未先设置密码，无法启用锁屏

# Account
errors-account-signin-failed = 登录失败
errors-account-signin-refused = { $reason }
errors-account-signin-unavailable = 登录不可用
errors-account-browser-open-failed = 无法打开您的浏览器。请打开默认浏览器后重试。
errors-account-signup-open-failed = 无法打开注册页面。请在浏览器中打开 screenerbot.io/signup。
errors-account-credentials-required = 请输入您的邮箱地址和密码。
errors-account-signout-failed = 退出登录失败
errors-account-gateway-update-failed = 无法更新网关设置

# Localization
errors-i18n-locale-not-registered = 语言区域未注册
errors-i18n-catalog-encode-failed = 无法编码语言目录

# System
errors-system-paths-init-failed = 无法创建应用程序目录
errors-system-open-data-failed = 无法打开数据文件夹
errors-system-url-empty = URL 不能为空
errors-system-open-url-failed = 无法打开该 URL

# Initialization
errors-initialization-required = 访问此接口前需要先完成机器人初始化。请通过网页界面完成初始化流程。
errors-initialization-onboarding-update-failed = 更新引导状态失败
errors-initialization-validation-required = 保存设置前需要先验证凭据
errors-initialization-encrypt-failed = 加密私钥失败

# Dashboard state
errors-ui-state-save-failed = 保存状态失败
errors-ui-state-clear-failed = 清除状态失败

# Agent control
errors-agent-config-failed = 代理控制配置失败
errors-agent-invalid-parameters = 参数无效
errors-agent-wallet-key-material = 钱包密钥材料不允许代理访问
errors-agent-store-failed = 代理控制存储失败
errors-agent-invalid-pairing-request = 配对请求无效
errors-agent-pairing-rejected = 配对凭据被拒绝
errors-agent-disabled = 代理控制已停用
errors-agent-approval-not-pending = 该批准请求已不在待处理状态
errors-agent-approval-not-found = 未找到批准请求
errors-agent-bridge-task-failed = 代理控制桥接任务失败
errors-agent-task-failed = 代理控制任务失败
errors-agent-pairing-not-found = 没有该 ID 对应的有效配对
errors-agent-permissions-update-failed = 更新权限失败

# Connectivity
errors-connectivity-endpoint-not-found = 未找到端点 '{ $endpoint }'，或该端点未被监控

# Copy trading
errors-copy-task-not-found = 未找到跟单任务
errors-copy-holding-not-found = 该代币没有未平仓的模拟持仓
errors-copy-live-confirmation-required = 启用实盘跟单交易需要明确确认
errors-copy-task-invalid = 跟单任务无效
errors-copy-request-rejected = 跟单交易请求被拒绝
errors-copy-task-limit = 已达到活跃跟单任务数量上限
errors-copy-watch-rejected = 无法监控该跟单目标
errors-copy-live-unavailable = 实盘跟单交易不可用
errors-copy-task-live = 删除前请先暂停实盘任务
errors-copy-task-owns-positions = 跟单任务仍持有持仓中的仓位
errors-copy-request-failed = 跟单交易请求失败

# Strategies
errors-strategies-not-found = 未找到策略
errors-strategies-invalid-type = 策略类型无效。必须为 ENTRY 或 EXIT
errors-strategies-list-failed = 获取策略失败
errors-strategies-get-failed = 获取策略失败
errors-strategies-serialize-rules-failed = 序列化规则失败
errors-strategies-invalid-rules-json = 规则 JSON 无效
errors-strategies-already-exists = ID 为 '{ $id }' 的策略已存在
errors-strategies-validation-failed = 策略验证失败
errors-strategies-create-failed = 创建策略失败
errors-strategies-update-failed = 更新策略失败
errors-strategies-update-enabled-failed = 更新策略启用状态失败
errors-strategies-delete-failed = 删除策略失败
errors-strategies-deploy-failed = 部署策略失败
errors-strategies-no-performance = 此策略暂无表现数据
errors-strategies-performance-failed = 获取表现统计失败
errors-strategies-schemas-failed = 获取条件结构失败
errors-strategies-evaluation-failed = 策略评估失败

# Transactions
errors-transactions-own-wallet-unavailable = 未配置主钱包
errors-transactions-invalid-subject = 交易主体不是有效的 Solana 地址
errors-transactions-subject-not-watched = 交易主体不是受监控的钱包
errors-transactions-watch-store-unavailable = 受监控钱包不可用

# Wallet
errors-wallet-unavailable = 主钱包不可用
errors-wallet-changed = 主钱包已更改，请刷新后重试
errors-wallet-qr-failed = 无法生成钱包二维码

# Updates
errors-updates-none-available = 没有可下载的更新
errors-updates-version-changed = 可用更新已变化，请重新检查更新
errors-updates-check-failed = 检查更新失败
errors-updates-download-failed = 无法开始下载更新
errors-updates-history-unavailable = 版本历史不可用
errors-updates-apply-failed = 无法应用更新
errors-updates-install-failed = 无法打开更新安装程序

# Telegram
errors-telegram-settings-update-failed = 更新设置失败
errors-telegram-disabled = { -telegram } 未启用
errors-telegram-not-configured = 未配置机器人令牌或聊天 ID
errors-telegram-send-failed = 发送消息失败
errors-telegram-notifier-failed = 创建通知器失败
errors-telegram-token-required = 必须先配置机器人令牌
errors-telegram-discovery-failed = 启动发现失败
errors-telegram-chat-select-failed = 选择聊天失败

# Assistant chat
errors-chat-message-empty = 消息不能为空
errors-chat-message-too-long = 消息超过 10,000 个字符的最大长度
errors-chat-database-unavailable = 对话数据库未初始化
errors-chat-session-not-found = 未找到对话会话 { $id }
errors-chat-session-validate-failed = 验证会话失败
errors-chat-engine-unavailable = 对话引擎未初始化
errors-chat-process-failed = 处理对话消息失败
errors-chat-stream-serialize-failed = 序列化对话事件失败
errors-chat-sessions-list-failed = 获取对话会话列表失败
errors-chat-session-create-failed = 创建对话会话失败
errors-chat-session-get-failed = 获取对话会话失败
errors-chat-messages-get-failed = 获取对话消息失败
errors-chat-session-delete-failed = 删除对话会话失败
errors-chat-messages-load-failed = 获取消息失败
errors-chat-summarize-empty = 无法为空对话会话生成摘要
errors-chat-provider-invalid = 提供商无效：{ $provider }
errors-chat-summary-save-failed = 保存摘要失败
errors-chat-title-empty-session = 无法为空对话会话生成标题
errors-chat-no-user-message = 会话中未找到用户消息
errors-chat-title-save-failed = 更新会话标题失败
errors-chat-confirmation-save-failed = 保存确认响应失败
errors-chat-confirmation-failed = 处理确认失败
errors-chat-summary-failed = 生成摘要失败

# Assistant automation
errors-automation-database-unavailable = 数据库未初始化
errors-automation-tasks-list-failed = 获取任务列表失败
errors-automation-name-empty = 任务名称不能为空
errors-automation-instruction-empty = 任务指令不能为空
errors-automation-schedule-type-invalid = schedule_type 无效。必须为：interval、daily 或 weekly
errors-automation-schedule-value-invalid = schedule_value 无效
errors-automation-task-create-failed = 创建任务失败
errors-automation-task-not-found = 未找到任务
errors-automation-task-get-failed = 获取任务失败
errors-automation-schedule-invalid = 计划无效
errors-automation-tool-permissions-invalid = tool_permissions 必须为 'full' 或 'readonly'
errors-automation-priority-invalid = priority 必须为 'low'、'medium' 或 'high'
errors-automation-task-update-failed = 更新任务失败
errors-automation-task-running-delete = 任务运行期间无法删除
errors-automation-task-delete-failed = 删除任务失败
errors-automation-task-toggle-failed = 切换任务状态失败
errors-automation-task-disabled = 无法运行已停用的任务
errors-automation-task-already-running = 任务已在运行
errors-automation-runs-list-failed = 获取运行记录列表失败
errors-automation-recent-runs-failed = 获取最近运行记录失败
errors-automation-run-not-found = 未找到运行记录
errors-automation-run-get-failed = 获取运行记录失败
errors-automation-stats-failed = 获取统计数据失败

# LLM providers
errors-llm-config-update-failed = 更新 LLM 配置失败
errors-llm-provider-unknown = 未知提供商：{ $provider }
errors-llm-manager-unavailable = LLM 管理器未初始化
errors-llm-provider-disabled = 提供商 '{ $provider }' 未配置或已停用
errors-llm-provider-config-update-failed = 更新提供商配置失败
errors-llm-provider-test-failed = 提供商测试失败
errors-llm-provider-refused = { $reason }

# LLM analysis
errors-llm-analysis-config-update-failed = 更新分析配置失败
errors-llm-analysis-unavailable = 分析引擎未初始化
errors-llm-analysis-disabled = LLM 功能已停用。请先启用 [llm]。
errors-llm-analysis-priority-invalid = 优先级无效：'{ $priority }'。请使用 'high'、'medium' 或 'low'。
errors-llm-analysis-evaluation-failed = 模型分析失败
errors-llm-analysis-instructions-list-failed = 获取指令列表失败
errors-llm-analysis-instruction-not-found = 未找到指令 { $id }
errors-llm-analysis-instruction-get-failed = 获取指令失败
errors-llm-analysis-instruction-created-retrieve-failed = 获取已创建的指令失败
errors-llm-analysis-instruction-create-failed = 创建指令失败
errors-llm-analysis-instruction-updated-retrieve-failed = 获取已更新的指令失败
errors-llm-analysis-instruction-update-failed = 更新指令失败
errors-llm-analysis-instruction-delete-failed = 删除指令失败
errors-llm-analysis-instructions-reorder-failed = 重新排序指令失败
errors-llm-analysis-decisions-list-failed = 获取决策历史失败
errors-llm-analysis-decision-not-found = 未找到决策 { $id }
errors-llm-analysis-decision-get-failed = 获取决策失败

# Wallets
errors-wallets-list-failed = 获取钱包列表失败
errors-wallets-name-empty = 钱包名称不能为空
errors-wallets-create-failed = 创建钱包失败
errors-wallets-key-empty = 私钥不能为空
errors-wallets-already-exists = 钱包已存在
errors-wallets-key-invalid = 私钥格式无效
errors-wallets-import-failed = 导入钱包失败
errors-wallets-summary-failed = 获取钱包摘要失败
errors-wallets-no-main-wallet = 未配置主钱包
errors-wallets-main-get-failed = 获取主钱包失败
errors-wallets-not-found = 未找到钱包
errors-wallets-get-failed = 获取钱包失败
errors-wallets-update-failed = 更新钱包失败
errors-wallets-delete-failed = 删除钱包失败
errors-wallets-export-failed = 导出钱包失败
errors-wallets-set-main-failed = 设置主钱包失败
errors-wallets-archive-failed = 归档钱包失败
errors-wallets-restore-failed = 恢复钱包失败
errors-wallets-export-format-unsupported = 目前仅支持 CSV 格式
errors-wallets-export-confirmation-required = 您必须通过输入以下内容进行确认：“{ $confirmation }”
errors-wallets-export-no-ids = 未提供钱包 ID
errors-wallets-export-bulk-failed = 导出钱包失败
errors-wallets-export-no-match = 未找到与所提供 ID 匹配的钱包
errors-wallets-import-file-too-large = 文件超过 { $megabytes }MB 的最大限制
errors-wallets-import-read-failed = 读取上传的文件失败
errors-wallets-import-no-file = 未上传文件。请使用 multipart 表单中的 'file' 字段
errors-wallets-import-encoding-invalid = CSV 文件必须为 UTF-8 编码
errors-wallets-import-csv-parse-failed = 解析 CSV 文件失败
errors-wallets-import-excel-parse-failed = 解析 Excel 文件失败
errors-wallets-import-format-unsupported = 不支持的文件格式。请使用 .csv、.xlsx 或 .xls
errors-wallets-import-file-empty = 文件不包含数据行
errors-wallets-import-existing-check-failed = 检查现有钱包失败
errors-wallets-import-mapping-invalid = 缺少必需的列：{ $columns }
errors-wallets-import-session-not-found = 未找到导入会话或会话已过期。请重新上传文件
errors-wallets-import-no-valid-rows = 没有可导入的有效行

# Wallet watching
errors-wallet-watch-list-failed = 获取监控目标列表失败
errors-wallet-watch-address-empty = 地址不能为空
errors-wallet-watch-add-failed = 添加监控目标失败
errors-wallet-watch-remove-failed = 移除监控目标失败
errors-wallet-watch-update-failed = 更新监控目标失败
errors-wallet-watch-budget-failed = 无法更新监控预算
errors-wallet-watch-resume-failed = 无法恢复监控
errors-wallet-watch-approval-failed = 无法更新 { -helius } 批准状态
errors-wallet-watch-status-failed = 获取监控状态失败

# Tools
errors-tools-wallet-failed = 获取钱包失败
errors-tools-wallet-address-failed = 获取钱包地址失败
errors-tools-accounts-scan-failed = 扫描账户失败
errors-tools-token-accounts-scan-failed = 扫描代币账户失败
errors-tools-token-accounts-get-failed = 获取代币账户失败
errors-tools-cleanup-failed = 清理失败
errors-tools-cache-clear-failed = 清除缓存失败
errors-tools-no-tokens = 未选择要销毁的代币
errors-tools-burn-failed = 销毁代币失败
errors-tools-favorites-list-failed = 获取收藏失败
errors-tools-favorite-type-invalid = 工具类型无效。必须为以下之一：{ $types }
errors-tools-favorite-add-failed = 添加收藏失败
errors-tools-favorite-not-found = 未找到收藏
errors-tools-favorite-update-failed = 更新收藏失败
errors-tools-favorite-delete-failed = 删除收藏失败
errors-tools-favorite-use-failed = 更新使用次数失败
errors-tools-pool-search-failed = 搜索代币 { $mint } 的流动性池失败
errors-tools-watched-list-failed = 获取监控代币列表失败
errors-tools-watched-add-failed = 添加监控代币失败
errors-tools-watched-delete-failed = 删除监控代币失败
errors-tools-mint-invalid = 代币铸造地址无效
errors-tools-wallets-get-failed = 获取钱包失败
errors-tools-balance-failed = 获取钱包余额失败
errors-tools-session-active = 另一项多钱包操作正在进行中
errors-tools-config-invalid = 工具配置无效：{ $reason }
errors-tools-config-rejected = 工具配置无效
errors-tools-consolidate-failed = 归集钱包失败
errors-tools-ata-cleanup-failed = 清理 ATA 失败
errors-tools-routers-unavailable = 兑换路由尚未就绪
errors-tools-router-disabled-chain-settings = { $router } 已在设置 > 区块链中停用
errors-tools-router-unknown = 未知的兑换路由 '{ $router }'
errors-tools-session-type-mismatch = 会话为 { $actual }，而非 { $expected }
errors-tools-session-not-found = 未找到会话
errors-tools-session-complete = 会话已完成

# Configuration import and reload
errors-config-reload-failed = 重新加载配置失败
errors-config-reset-failed = 重置配置失败
errors-config-disk-parse-failed = 解析磁盘配置失败
errors-config-disk-read-failed = 读取磁盘配置失败
errors-config-update-failed = 更新配置失败
errors-config-import-not-object = 配置必须为 JSON 对象
errors-config-import-no-sections = 未找到可导入的有效配置节
errors-config-import-validation-failed = 配置验证失败。未应用任何更改。
errors-config-import-commit-failed = 提交配置更改失败
errors-config-import-failed = 导入配置失败

# Filtering
errors-filtering-analytics-failed = 获取分析数据失败
errors-filtering-refresh-failed = 重建过滤快照失败
errors-filtering-rejection-stats-failed = 获取拒绝统计失败
errors-filtering-rejected-tokens-failed = 获取被拒绝的代币失败
errors-filtering-csv-header-failed = 写入 CSV 表头失败
errors-filtering-csv-record-failed = 写入 CSV 记录失败
errors-filtering-csv-finalize-failed = 完成 CSV 失败
errors-filtering-export-response-failed = 构建响应失败

# OHLCV
errors-ohlcv-fetch-failed = 获取 OHLCV 数据失败
errors-ohlcv-pools-failed = 获取流动性池失败
errors-ohlcv-gaps-failed = 获取缺口失败
errors-ohlcv-refresh-failed = 刷新失败
errors-ohlcv-monitor-start-failed = 启动监控失败
errors-ohlcv-monitor-stop-failed = 停止监控失败
errors-ohlcv-activity-failed = 记录活动失败
errors-ohlcv-list-failed = 获取 OHLCV 代币列表失败
errors-ohlcv-delete-failed = 删除代币数据失败
errors-ohlcv-clear-failed = 清除 OHLCV 缓存失败
errors-ohlcv-cleanup-failed = 清理未激活代币失败

# Trader and manual trading
errors-trade-already-running = 交易引擎已在运行
errors-trade-already-stopped = 交易引擎已停止
errors-trade-config-update-failed = 交易引擎配置更新失败
errors-trade-trader-unavailable = 使用自动交易前，请先完成钱包和 RPC 设置
errors-trade-force-stop-active = 紧急停止已生效，请先解除
errors-trade-template-not-found = 没有名为 { $template } 的交易引擎模板
errors-trade-manual-force-stopped = 紧急停止生效期间，手动交易已停用
errors-trade-core-services-not-ready = 核心服务尚未就绪，无法交易：{ $pending }
errors-trade-mint-invalid = 代币铸造地址无效：{ $mint }
errors-trade-blacklisted = 代币 { $mint } 已加入黑名单
errors-trade-slippage-invalid = 滑点 { $slippage }% 必须在 (0, { $maximum }] 范围内
errors-trade-percentage-invalid = 卖出比例 { $percentage } 必须在 (0, 100] 范围内
errors-trade-record-failed = 无法记录手动交易
errors-trade-task-cancelled = 应用正在关闭，手动交易在返回结果前已停止；其结果以持仓为准
errors-trade-no-open-position = 代币 { $mint } 没有持仓中的仓位
errors-trade-size-invalid = 交易规模无效：{ $amount } { -sol }
errors-trade-management-invalid = 仓位管理方式无效：{ $management }
errors-trade-strategy-evaluation-failed = 代币 { $mint } 的策略评估失败
errors-trade-token-data-missing = 无法获取 { $mint } 的代币数据
errors-trade-endpoints-unhealthy = 没有可用的健康端点
errors-trade-dependency-failed = { $dependency } 依赖项失败
errors-trade-storage-failed = 无法完成该交易请求
errors-trade-manual-failed = 手动交易失败
errors-trade-manual-refused = { $reason }
errors-trade-swap-too-large = 没有任何兑换路由能构建出足够小、可以发送的交易
    .hint = 最佳路由所需的账户数量超出了单笔交易的容量，因此没有发送任何内容，也没有产生任何花费。请稍后重试以获取其他路由，或启用另一个兑换路由器。
errors-trade-wallet-not-configured = 未配置钱包
errors-trade-amount-sol-invalid = 买入时必须提供 amount_sol，且必须为正数
errors-trade-no-tokens-in-wallet = 钱包中未找到此仓位的代币。代币余额为 0，无法通过兑换平仓。
errors-trade-percentage-range = percentage 必须在 (0, 100] 范围内
errors-trade-amount-tokens-invalid = amount_tokens 必须为正数
errors-trade-sell-amount-zero = 计算出的卖出数量为零

# Swap quotes. The message is the dialog headline; `.hint` is what the user can do.
errors-trade-quote-registry-unavailable = 兑换路由尚未就绪
    .hint = 兑换服务仍在启动中。请等待服务就绪后重试。
errors-trade-quote-no-routers-enabled = 未启用任何兑换提供商
    .hint = 请在交易引擎设置中至少启用一个兑换路由，然后重试。
errors-trade-quote-not-tradable = 此代币当前无法交易
    .hint = 没有可用的流动性或兑换路径。该代币可能尚未上线、已被废弃或没有流动性池。请稍后重试或选择其他代币。
errors-trade-quote-no-route = 没有可用的兑换路径
    .hint = 没有提供商能按请求的数量为此交易规划路径。请尝试较小的数量，或稍后重试。
errors-trade-quote-rate-limited = 兑换提供商正在限流
    .hint = 兑换提供商正在限制请求。请等待几秒后重试。
errors-trade-quote-timeout = 报价请求超时
    .hint = 兑换提供商未在规定时间内响应。请检查您的网络连接后重试。
errors-trade-quote-router-rejected = 报价被拒绝
    .hint = 提供商返回的报价未通过我们的安全检查，已被丢弃。请重试以获取新的报价。
errors-trade-quote-not-offered-exact-out = 没有已启用的兑换路由支持按精确输出数量报价
    .hint = 已启用的兑换路由只按支付金额计算交易。请输入要支付的金额，或启用其他兑换路由。
errors-trade-quote-not-offered-unsupported-venue = 没有已启用的兑换路由可在此代币的池中交易
    .hint = 该代币所在的交易所尚未被已启用的兑换路由支持。请启用其他兑换路由后重试。
errors-trade-quote-unavailable = 无法获取报价
    .hint = 兑换提供商无法为此交易报价。请稍后重试。

# Positions
errors-positions-not-found = 未找到仓位
errors-positions-already-closed = 仓位已平仓
errors-positions-force-close-failed = 强制平仓失败
errors-positions-already-archived = 仓位已归档
errors-positions-unverified-entry-archive = 此仓位的买入尚未确认。确认后再归档。
errors-positions-not-archived = 仓位未归档
errors-positions-archive-failed = 归档仓位失败
errors-positions-unarchive-failed = 取消归档仓位失败
errors-positions-unarchive-duplicate-open = 该代币有多个持仓中的仓位（{ $positions }），只能有一个处于活跃状态。请先平仓或归档其他仓位。
errors-positions-management-invalid = 跟单管理方式要求仓位来源为跟单
errors-positions-management-failed = 更新仓位管理方式失败
errors-positions-delete-failed = 删除仓位失败
errors-positions-bulk-delete-failed = 删除已归档仓位失败
errors-positions-detail-failed = 加载仓位详情失败
errors-positions-resolve-failed = 解析仓位失败
errors-positions-wrapped-sol-activity = 包装 SOL 没有代币活动

# Tokens
errors-tokens-database-unavailable = 代币数据库不可用
errors-tokens-blacklist-failed = 将代币加入黑名单失败
errors-tokens-blacklist-internal = 黑名单操作期间发生内部错误
errors-tokens-unblacklist-failed = 从黑名单移除失败
errors-tokens-unblacklist-internal = 移除黑名单操作期间发生内部错误
errors-tokens-blacklist-status-failed = 检查黑名单状态失败
errors-tokens-blacklist-status-internal = 检查黑名单状态期间发生内部错误
errors-tokens-favorites-fetch-failed = 获取收藏失败
errors-tokens-favorite-add-failed = 添加收藏失败
errors-tokens-favorite-remove-failed = 移除收藏失败
errors-tokens-favorite-update-failed = 更新收藏失败
errors-tokens-detail-not-found = 在数据库或外部来源中未找到该代币
errors-tokens-fetch-failed = 获取代币失败
errors-tokens-refresh-all-failed = 所有数据源均失败
errors-tokens-refresh-failed = 刷新代币失败
errors-tokens-search-query-required = 搜索查询参数 'q' 为必填项
errors-tokens-search-failed = 代币搜索失败

# Actions and services
errors-actions-not-found = 未找到操作 { $id }
errors-services-not-found = 未找到服务 '{ $name }'
