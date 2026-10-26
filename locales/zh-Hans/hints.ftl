# Contextual hints: one title and one body per hint, keyed by the hint id.
# Bodies use a small markdown subset (**bold** and bullet lines) that the hint popover renders.

# Source: scripts/core/hints.js

## Categories

hints-category-positions = 仓位
hints-category-filtering = 过滤
hints-category-trader = 自动交易
hints-category-services = 服务
hints-category-wallet = 钱包
hints-category-wallets = 多钱包
hints-category-config = 配置
hints-category-config-telegram = { -telegram }
hints-category-token-details = 代币详情
hints-category-ui = 界面

## positions

hints-positions-overview-title = 仓位概览
hints-positions-overview-content =
    您当前的代币持仓和交易仓位。

    关键指标：
    • **入场价格**——平均买入价格（含 DCA）
    • **当前价格**——来自流动性池储备的实时价格
    • **盈亏**——以 SOL 和 % 表示的未实现盈亏
    • **数量**——持有的代币总数

    点击任意仓位可查看详细的管理选项。
hints-positions-dca-title = DCA（定投加仓）
hints-positions-dca-content =
    DCA 允许在不同价格对现有仓位加仓。

    触发 DCA 时：
    • 买入额外代币
    • 入场价格按加权平均重新计算
    • 仓位规模增加
    • 入场次数加一

    可在**自动交易**设置中配置 DCA 规则。
hints-positions-partial-exit-title = 部分出场
hints-positions-partial-exit-content =
    卖出部分仓位，同时保留其余部分。

    优势：
    • 锁定部分利润，同时保持敞口
    • 无需完全平仓即可缩小仓位规模
    • 实现分批止盈

    每次部分出场都会单独记录，以便准确追踪盈亏。
hints-positions-management-title = 仓位管理
hints-positions-management-content =
    管理方式决定哪些自动化功能可以操作某个仓位。

    • 自动交易：安全出场、策略出场和自动 DCA
    • 仅用户：不执行任何自动操作
    • 跟单任务：安全出场和跟随卖出
    • 混合：安全出场、策略出场和跟随卖出

    您需要自行卖出或加仓。手动买入默认采用手动管理，因此机器人不会卖出您有意买入的代币。关闭此项即可将仓位交还给自动交易。

## filtering

hints-filtering-overview-title = 代币过滤
hints-filtering-overview-content =
    过滤决定哪些代币有资格参与交易。

    代币必须通过**所有已启用的条件**才会出现在已通过列表中：
    • { -dexscreener } 指标（流动性、成交量等）
    • { -geckoterminal } 指标（市值、FDV）
    • { -rugcheck } 安全分析
    • 元数据过滤（代币年龄等）

    已停用的条件会被完全跳过。
hints-filtering-dexscreener-title = { -dexscreener } 过滤器
hints-filtering-dexscreener-content =
    基于 { -dexscreener } 市场数据的过滤器。

    • **流动性**——流动性池中的最低美元流动性
    • **24h 成交量**——最低交易量
    • **交易笔数**——活跃度阈值（买入/卖出）
    • **价格变化**——波动率过滤

    { -dexscreener } 数据每隔几分钟更新一次。
hints-filtering-geckoterminal-title = { -geckoterminal } 过滤器
hints-filtering-geckoterminal-content =
    基于 { -geckoterminal } 市场数据的过滤器。

    • **市值**——最低市值
    • **FDV**——完全稀释估值上限
    • **储备比率**——流动性池健康指标

    { -geckoterminal } 通常有较新代币的数据。
hints-filtering-rugcheck-title = 安全过滤器
hints-filtering-rugcheck-content =
    来自 { -rugcheck }.xyz 的安全分析。

    • **风险评分**——总体风险评级（0-100）
    • **增发权限**——是否可以增发新代币？
    • **冻结权限**——是否可以冻结转账？
    • **前几大持有者**——集中度风险

    风险评分越高，表示潜在的危险信号越多。
hints-filtering-meta-title = 元数据过滤器
hints-filtering-meta-content =
    其他过滤条件。

    • **代币年龄**——自代币创建以来的最短时间
    • **流动性池年龄**——自流动性池创建以来的最短时间
    • **有网站**——要求提供社交/网站链接
    • **有社交账号**——要求提供 { -twitter }/{ -telegram }

    这些条件有助于过滤掉非常新或可疑的代币。

## trader

hints-trader-overview-title = 自动交易
hints-trader-overview-content =
    自动交易引擎，负责监控代币并执行交易。

    组成部分：
    • **入场监控**——关注买入机会
    • **出场监控**——管理卖出和止盈
    • **DCA 监控**——处理仓位均价
    • **风险控制**——亏损限额和安全闸门

    可在控制面板中启动/停止交易。
hints-trader-entry-title = 入场监控
hints-trader-entry-content =
    关注已过滤代币的入场信号。

    入场评估检查：
    • 代币通过当前过滤
    • 尚未持有该仓位
    • 未被加入黑名单
    • 未超出仓位数量上限
    • 满足策略条件（如已配置）

    可在配置中设置入场规模和限制。
hints-trader-exit-title = 出场监控
hints-trader-exit-content =
    监控持仓中的仓位的出场信号。

    出场触发条件：
    • **止盈**——达到价格目标
    • **止损**——超出最大亏损
    • **追踪止损**——价格自峰值回落
    • **策略出场**——满足自定义条件
    • **基于时间**——达到最长持仓时长

    可在配置中设置阈值。

## services

hints-services-overview-title = 系统服务
hints-services-overview-content =
    支撑 { -brand } 运行的后台服务。

    服务状态：
    • **运行中**（绿色）——运行正常
    • **启动中**（黄色）——正在初始化
    • **已停止**（红色）——未运行
    • **错误**（警告）——已失败，可能会自动重启

    服务之间存在依赖关系，并按顺序启动。
hints-services-health-title = 服务健康状况
hints-services-health-content =
    健康指标显示服务状态。

    • **运行时长**——自上次启动以来的时间
    • **任务**——活跃的后台操作
    • **错误**——近期错误数量
    • **指标**——性能数据（如有）

    关键服务会影响交易能力。

## wallet

hints-wallet-overview-title = 钱包概览
hints-wallet-overview-content =
    您已连接的 Solana 钱包状态。

    显示：
    • **SOL 余额**——用于 gas 和交易的原生 SOL
    • **代币持仓**——带价值的 SPL 代币
    • **24h 变化**——投资组合价值变化
    • **历史**——随时间变化的余额快照

    余额每分钟刷新一次。
hints-wallet-tokens-title = 代币余额
hints-wallet-tokens-content =
    您钱包中持有的 SPL 代币。

    显示：
    • 代币符号和名称
    • 持有数量
    • 以 SOL/USD 计的当前价值
    • 来自流动性池或市场数据的价格

    空的代币账户会自动关闭，其租金退回钱包。

## wallets

hints-wallets-main-title = 主钱包
hints-wallets-main-content =
    用于所有交易操作的主要钱包。

    • **自动交易**——入场/出场交易均由此钱包执行
    • **余额显示**——显示在页眉和仪表盘中
    • **代币持仓**——此钱包持有的 SPL 代币

## config

hints-config-overview-title = 配置
hints-config-overview-content =
    { -brand } 的系统级设置。

    分类：
    • **交易引擎**——入场/出场规则、仓位大小
    • **过滤**——代币过滤阈值
    • **兑换**——路由与滑点设置
    • **RPC**——节点配置
    • **服务**——后台服务设置

    更改会立即生效（热重载）。
hints-config-telegram-title = { -telegram } 通知
hints-config-telegram-content =
    { "*" }*通过 { -telegram } 接收即时交易提醒**

    直接在 { -telegram } 中获取交易、仓位和重要事件的通知。

    { "*" }*设置步骤：**

    1. **创建机器人：**
       • 打开 { -telegram } 并向 @BotFather 发送消息
       • 发送 /newbot 并按提示操作
       • 复制机器人令牌（形如：123456:ABC-DEF...）

    2. **获取您的聊天 ID：**
       • 向 @userinfobot 或 @getidsbot 发送消息
       • 复制其返回的数字 ID

    3. **在 { -brand } 中配置：**
       • 启用通知开关
       • 粘贴机器人令牌和聊天 ID
       • 点击“测试连接”进行验证

    { "*" }*您将收到：**
    • 交易执行确认
    • 仓位更新（入场/出场）
    • 错误通知

    { "*" }*隐私：**
    消息由 { -brand } 直接发送到您的 { -telegram } 机器人，不经过任何第三方服务器。
hints-config-telegram-password-title = 机器人验证密码
hints-config-telegram-password-content =
    { "*" }*使用密码验证保护您的 { -telegram } 机器人**

    与您的 { -brand } { -telegram } 机器人交互时，执行敏感命令前需要使用此密码进行验证。

    { "*" }*为什么要设置密码？**
    • 防止未经授权的用户控制您的机器人
    • 通过 { -telegram } 执行交易命令时必需
    • 长度至少为 8 个字符

    { "*" }*工作原理：**
    1. 在仪表盘中设置密码
    2. 向机器人发送交易命令时，机器人会要求验证
    3. 输入密码以验证您的身份
    4. 可选择启用 2FA 以获得额外安全保障

    { "*" }*注意：** 密码以安全的 SHA256 哈希形式存储，我们绝不会存储明文。
hints-config-telegram-totp-title = 双重验证（2FA）
hints-config-telegram-totp-content =
    { "*" }*使用 TOTP 2FA 增加一层安全保护**

    双重验证使用 Google Authenticator、Authy 或 1Password 等应用生成的基于时间的一次性密码（TOTP）。

    { "*" }*为什么要启用 2FA？**
    • 即使他人知道您的密码，没有验证码也无法访问您的机器人
    • 6 位验证码每 30 秒更换一次
    • 设置完成后可离线使用

    { "*" }*设置流程：**
    1. 点击“启用 2FA”并输入您的密码
    2. 使用验证器应用扫描二维码
    3. 输入 6 位验证码以完成设置验证

    { "*" }*兼容的应用：**
    • Google Authenticator
    • Authy
    • 1Password
    • Microsoft Authenticator
    • 任何兼容 TOTP 的应用

    { "*" }*重要提示：** 请将您的密钥保存在安全的地方。如果您无法再访问验证器应用，需要在此仪表盘中停用 2FA。

## token_details

hints-token-details-chart-title = 价格图表（OHLCV）
hints-token-details-chart-content =
    { "*" }*重要提示：** 此图表显示的是用于策略评估的**缓存 OHLCV 数据**，*而非*实时执行价格。

    { "*" }*为什么使用缓存数据？**
    • **用途：** 供自动策略和指标使用（例如 RSI、MA）。
    • **新鲜度：** 更新取决于代币优先级（持仓中的仓位 = 更新更快）。
    • **来源：** 聚合自 { -dexscreener }/{ -geckoterminal }，而非直接来自链上 RPC。

    { "*" }*DEX 价格的真实情况：**
    在 DeFi 中，代币会在**多个流动性池**（Raydium、Orca、Meteora）中交易。每个流动性池的价格因流动性深度和近期交易而异。
    • **图表价格：** 各市场的平均/聚合价格。
    • **兑换价格：** 交易时刻通过最佳路由获得的具体汇率。

    { "*" }预期此图表与您的最终执行价格之间会存在小幅差异。*

    { "*" }*状态：** “等待数据”表示后台工作进程正在获取最新的 K 线。
hints-token-details-token-info-title = 代币信息
hints-token-details-token-info-content =
    来自链上和市场来源的基本代币元数据。

        • **铸造地址**——Solana 上唯一的代币地址（点击复制）
        • **小数位数**——代币精度（通常为 6-9）
        • **年龄**——自主流动性池/代币创建以来的时间
        • **DEX**——该代币的主要交易场所
        • **持有者**——持有该代币的独立钱包数
        • **前 10 持仓**——前 10 名钱包持有的百分比

        持有者数量越多、集中度越低，通常表示分布越健康。
hints-token-details-liquidity-title = 流动性与市场数据
hints-token-details-liquidity-content =
    来自流动性最高的 SOL 流动性池的市场指标。

        • **FDV**——价格 × 总供应量（聚合器价格）
        • **流动性**——流动性池储备的美元价值
        • **池内 SOL** / **池内代币**——决定池价格的实时储备

        { "*" }*为什么重要：**
        • 流动性越深 = 滑点越低
        • 较浅的流动性池可能因小额交易而波动
        • 流动性池储备直接决定兑换的执行价格

        数据会定期从 { -dexscreener }/{ -geckoterminal } 以及链上流动性池读取中刷新。
hints-token-details-market-pulse-title = 市场脉搏
hints-token-details-market-pulse-content =
    价格走势和美元成交量使用相同的 **5M / 1H / 6H / 24H** 时间线，便于直接比较动量与参与度。

    { "*" }*解读：**
    • **价格**——由聚合器得出的百分比变化，而非实时的流动性池执行价格。
    • **高成交量**——兴趣更强、价格发现更有效、更容易出场。
    • **低成交量**——滑点更大、价差更宽、大额出场更困难。
    • **高成交量 + 低流动性**——波动性和执行风险上升。

    市场数据通过 { -dexscreener }/{ -geckoterminal } 聚合自各大 DEX，因此价格变化可能与当前的链上流动性池价格不同。
hints-token-details-activity-title = 交易活动（笔数）
hints-token-details-activity-content =
    分析多个时间周期内的**交易笔数**（买入与卖出）。无论交易规模大小，都能反映交易者的意图。

    { "*" }*指标详解：**
    • **时间周期：** 5M、1H、6H、24H 窗口。
    • **条形图：** 买入笔数（绿色）与卖出笔数（红色）的可视化比例。
    • **速率：** 每分钟交易笔数（例如 "12.5/m"）。速率越高 = 传播越火热。
    • **笔数：** 买入/卖出的确切笔数及其百分比占比。

    { "*" }*汇总指标：**
    • **24H 买入 %：** >50% 为看涨（买家更多），{ "<" }50% 为看跌（卖家更多）。
    • **净流入：** 买入总数减去卖出总数。为正 = 吸筹。
    • **5M 激增：** *当前*交易速度相比 1H 平均值快了多少。
      • **>1.0x：** 兴趣正在加速。
      • **>3.0x：** 爆发性突破或恐慌事件。
      • **{ "<" }1.0x：** 正在降温。

    { "*" }*策略提示：** 较高的“买入 %”配合较高的“激增系数”，通常预示着强势突破入场机会。
hints-token-details-security-title = 安全分析
hints-token-details-security-content =
    来自 { -rugcheck }.xyz 的风险评估和链上分析。

    { "*" }*安全评分（0-100）：**
    评分越高，代币越安全。影响因素包括：
    • 权限设置（增发/冻结）
    • 持有者集中度
    • LP 锁定状态
    • 已知风险模式

    { "*" }*关键风险指标：**
    • **增发权限**——可以创建新代币（通胀风险）
    • **冻结权限**——可以冻结代币账户
    • **最大持有者 %**——集中度风险
    • **LP 提供者**——流动性提供者数量

    在进行大额交易前，请务必核实安全性。
hints-token-details-pools-title = 流动性池
hints-token-details-pools-content =
    此代币所有已发现的流动性池。

    { "*" }*为什么多个流动性池很重要：**
    • 每个流动性池的流动性和定价各不相同
    • 兑换路由会在各流动性池之间寻找最佳路径
    • 不同流动性池之间的价格可能相差 1-5%

    { "*" }*流动性池信息：**
    • **DEX**——托管该流动性池的交易所
    • **流动性**——流动性池储备的美元价值
    • **成交量**——近期交易活动
    • **价格**——当前流动性池价格

    流动性池服务根据流动性最高的 SOL 交易对计算价格。

## ui

hints-ui-featured-title = 精选
hints-ui-featured-content =
    优先显示已加速的代币，其后是来自 { -jupiter } 和 { -dexscreener } 的热门项目。

    { "*" }*您将看到：**
    • 已加速的代币（其团队付费推广）固定在最前，以金色标记
    • 其后是来自发现榜单的热门代币
    • 点击任意代币可打开其完整详情

    { "*" }*加速代币：**
    加速购买的是曝光度，绝非推荐。已加速的行在所有出现的位置
    （包括您的代币表格）都以金色标记，因此您始终能分辨。可在
    { "*" }*screenerbot.io/boost** 加速代币。

    { "*" }*停用此行：**
    可在**设置 → 界面 → 显示精选行**中隐藏。页眉操作仍会打开
    完整的精选视图。

## Hint popover chrome (ui/hint_popover.js)

hints-trigger =
    .aria-label = 帮助：{ $title }
hints-popover-close =
    .aria-label = 关闭
hints-popover-learn-more = 了解更多
hints-popover-dismiss = 不再显示
