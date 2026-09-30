# Source: templates/pages/onboarding.html

## Slide: welcome

onboarding-welcome-title = 欢迎使用 { -brand }
onboarding-welcome-description = 您的本地优先 Solana 交易伙伴，以 Rust 构建，拥有原生速度。在您自己的设备上发现代币、分析市场并控制交易。
onboarding-welcome-free-title = 免费且源码可见
onboarding-welcome-free-description = 无订阅，无付费墙。可查看发布在 { -github } 上的代码。
onboarding-welcome-custody-title = 自主托管
onboarding-welcome-custody-description = 私钥静态加密存储，绝不会传输到任何地方。
onboarding-welcome-engine-title = 常驻引擎
onboarding-welcome-engine-description = 服务统一编排，配有健康检查和平稳的生命周期控制。
onboarding-welcome-realtime-title = 链上实时数据
onboarding-welcome-realtime-description = 直接计算流动性池储备，而非延迟的 API 快照。

## Slide: discover

onboarding-discover-title = 发现与过滤
onboarding-discover-description = 扫描三个数据源中的 Solana 新交易对，在链上解码 12+ 种 DEX 流动性池类型，再让每个代币通过可配置的质量和安全规则。
onboarding-discover-dex-title = 多 DEX 发现
onboarding-discover-dex-description = 通过 { -dexscreener }、{ -geckoterminal } 和 Raydium 数据源获取 { -sol } 新交易对。
onboarding-discover-scanner-title = 智能代币扫描
onboarding-discover-scanner-description = 流动性、成交量、代币上线时间、持有者分布和 { -rugcheck } 规则。
onboarding-discover-intelligence-title = 代币情报
onboarding-discover-intelligence-description = 市场、安全和黑名单数据汇总在同一视图中。
onboarding-discover-price-action-title = 价格走势追踪
onboarding-discover-price-action-description = 七个时间周期的 K 线数据，含缺口检测和动量信号。

## Slide: trade

onboarding-trade-title = 智能交易
onboarding-trade-description = 采用六级出场优先级系统的自动交易。可对仓位 DCA、设置追踪止损、构建策略树，也可一键手动交易。
onboarding-trade-auto-title = 自动交易
onboarding-trade-auto-description = 入场/出场评估器、DCA 轮次、部分出场和追踪止损。
onboarding-trade-strategy-title = 策略引擎
onboarding-trade-strategy-description = 结合价格、成交量和时间信号的条件树。
onboarding-trade-routing-title = 最优价格路由
onboarding-trade-routing-description = 并发获取所有已启用路由的报价，最优路由胜出。
onboarding-trade-safety-title = 安全控制
onboarding-trade-safety-description = 紧急停止、按周期的亏损限额，以及独立的监控开关。

## Slide: connect

onboarding-connect-title = 保持连接
onboarding-connect-description = 随时随地监控您的投资组合。由九家 LLM 提供商支持的助手、支持内联交易的 { -telegram } 提醒，以及可搜索的事件日志。
onboarding-connect-assistant-title = 助手
onboarding-connect-assistant-description = 以对话驱动的分析，可通过工具调用处理交易、配置和投资组合。
onboarding-connect-telegram-title = { -telegram } 集成
onboarding-connect-telegram-description = 在手机上接收通知、使用内联命令，并享受 2FA 保护的会话。
onboarding-connect-wallets-title = 多钱包追踪
onboarding-connect-wallets-description = 您所有的 Solana 钱包和代币持仓集中在同一个仪表盘中。
onboarding-connect-events-title = 实时事件流
onboarding-connect-events-description = 每一笔交易、兑换和系统事件都会记录类别和严重程度。

## Slide: data

onboarding-data-title = { -brand } 数据
onboarding-data-description = 我们运营一项共享的市场数据服务，这样每个安装实例就不必各自受到公共提供商的速率限制。拥有 { -brand } 账户即可免费使用，没有账户 { -brand } 也能正常工作。
onboarding-data-candles-title = 汇集的 K 线历史
onboarding-data-candles-description = 七个时间周期的共享历史数据，长达数年，由同一缓存提供。
onboarding-data-pools-title = 已解析的流动性池与安全信息
onboarding-data-pools-description = 集中的流动性池注册表和已缓存的 { -rugcheck } 报告，均已预先获取。
onboarding-data-signin-title = 登录后使用
onboarding-data-signin-description = 没有账户时无法使用这些数据，将改用公共提供商。
onboarding-data-reading-title = 仅读取
onboarding-data-reading-description = 我们只能看到您查询了哪些代币，绝不会接触密钥、余额、仓位或交易。

## Slide: privacy

onboarding-privacy-title = 您的密钥，您的数据
onboarding-privacy-description = 您的配置、密钥和交易历史都保留在这台设备上。接下来，您可以选择探索模式，无需凭据即可发现代币；也可以关联钱包和 RPC 以启用完整的机器人，如需 { -brand } 数据，请在那里登录。
onboarding-privacy-local-title = 本地优先架构
onboarding-privacy-local-description = 配置、分析数据和数据库都存储在您的桌面上。
onboarding-privacy-wallet-title = 加密钱包
onboarding-privacy-wallet-description = 您的私钥静态加密存储，绝不会传输。
onboarding-privacy-security-title = 仪表盘安全
onboarding-privacy-security-description = 密码锁、TOTP 双重验证和会话超时保护。
onboarding-privacy-config-title = 灵活配置
onboarding-privacy-config-description = 设置完成后，大多数设置都可以在仪表盘中调整。

## Footer

onboarding-setup-shortcut =
    .aria-label = 直接前往钱包和 RPC 设置，或选择探索模式
onboarding-setup-shortcut-label = 前往设置
onboarding-progress-dot =
    .aria-label = 前往第 { $number } 张幻灯片
onboarding-action-continue-to-setup = 继续设置
