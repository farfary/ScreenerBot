# Contextual hints: one title and one body per hint, keyed by the hint id.
# Bodies use a small markdown subset (**bold** and bullet lines) that the hint popover renders.

# Source: scripts/core/hints.js

## Categories

hints-category-tokens = 代币
hints-category-positions = 仓位
hints-category-filtering = 过滤
hints-category-trader = 自动交易
hints-category-services = 服务
hints-category-wallet = 钱包
hints-category-wallets = 多钱包
hints-category-tools = 工具
hints-category-config = 配置
hints-category-config-telegram = { -telegram }
hints-category-token-details = 代币详情
hints-category-ui = 界面

## tokens

hints-tokens-pool-service-title = 流动性池服务代币
hints-tokens-pool-service-content =
    此处显示的代币满足以下条件：

    • **通过全部过滤条件**——流动性、成交量、年龄和安全检查
    • **拥有有效的 SOL 流动性池**——受我们的 DEX 解码器支持（Raydium、Orca、Meteora 等）
    • **价格计算成功**——价格直接由链上流动性池储备计算得出

    这是最可靠的交易代币列表，因为价格来自实际的流动性池数据，而非外部 API。

    点击任意代币可查看详细信息并管理黑名单状态。
hints-tokens-no-market-title = 无市场数据
hints-tokens-no-market-content =
    在链上发现但缺少 { -dexscreener } 或 { -geckoterminal } 市场数据的代币。

    常见原因：
    • **非常新的代币**——尚未被聚合器收录
    • **成交量低**——低于聚合器的收录门槛
    • **未收录的交易对**——在聚合器未追踪的 DEX 上交易

    这些代币可能仍有有效的流动性池并可交易，但缺少外部市场指标。
hints-tokens-all-title = 全部代币
hints-tokens-all-content =
    所有已发现代币的完整数据库，不论过滤状态如何。

    包括：
    • 通过过滤的代币
    • 被拒绝的代币
    • 无市场数据的代币
    • 已加入黑名单的代币

    可使用此视图进行研究，或查找可能被过滤掉的代币。
hints-tokens-passed-title = 已通过过滤
hints-tokens-passed-content =
    通过所有已启用过滤条件的代币。

    过滤检查包括：
    • **流动性**——最低 SOL 流动性门槛
    • **成交量**——24h 成交量要求
    • **代币年龄**——自创建以来的最短时间
    • **安全性**——{ -rugcheck } 风险评分上限
    • **市值**——可选的 FDV/市值过滤

    可在**过滤**页面配置过滤器。
hints-tokens-rejected-title = 被拒绝的代币
hints-tokens-rejected-content =
    未满足一项或多项过滤条件的代币。

    每个代币都会显示具体的拒绝原因：
    • 哪个过滤器未通过
    • 实际值与要求阈值的对比
    • 检查发生的时间

    查看被拒绝的代币，以微调您的过滤设置。
hints-tokens-blacklisted-title = 黑名单代币
hints-tokens-blacklisted-content =
    被永久排除在交易之外的代币。

    加入黑名单的原因包括：
    • **手动加入黑名单**——您明确屏蔽的代币
    • **安全风险**——检测到跑路迹象
    • **亏损阈值**——超出配置的亏损限额
    • **交易失败**——多次兑换失败

    黑名单代币不会出现在已通过列表中，也不会被自动交易考虑。
hints-tokens-positions-title = 仓位代币
hints-tokens-positions-content =
    当前在持仓中的仓位所持有的代币。

    显示您活跃持仓的实时数据：
    • 来自流动性池储备的当前价格
    • 未实现盈亏
    • 仓位大小与入场价格
    • 持仓时间

    点击任意代币可进行详细的仓位管理。
hints-tokens-recent-title = 最近发现
hints-tokens-recent-content =
    按发现时间排序的新发现代币。

    适用于：
    • 发现新代币上线
    • 监控新增流动性
    • 寻找早期入场机会

    注意：新代币一开始可能缺少完整的市场数据。
hints-tokens-ohlcv-title = OHLCV 数据管理
hints-tokens-ohlcv-content =
    查看并管理为代币存储的 OHLCV（K 线）数据。

    显示：
    • **K 线数量**——已存储的数据点总数
    • **回填进度**——各时间周期的完成状态
    • **数据跨度**——以小时计的时间覆盖范围
    • **流动性池数量**——已追踪的流动性池
    • **状态**——活跃监控或未激活

    操作：
    • **删除**——移除某个代币的全部 OHLCV 数据
    • **清理**——批量移除未激活代币的数据

    OHLCV 数据会被永久保留，不会被自动删除。

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
    管理方式决定哪些自动化功能可以操作某个仓位：

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
    基于 { -dexscreener } 市场数据的过滤器：

    • **流动性**——流动性池中的最低美元流动性
    • **24h 成交量**——最低交易量
    • **交易笔数**——活跃度阈值（买入/卖出）
    • **价格变化**——波动率过滤

    { -dexscreener } 数据每隔几分钟更新一次。
hints-filtering-geckoterminal-title = { -geckoterminal } 过滤器
hints-filtering-geckoterminal-content =
    基于 { -geckoterminal } 市场数据的过滤器：

    • **市值**——最低市值
    • **FDV**——完全稀释估值上限
    • **储备比率**——流动性池健康指标

    { -geckoterminal } 通常有较新代币的数据。
hints-filtering-rugcheck-title = 安全过滤器
hints-filtering-rugcheck-content =
    来自 { -rugcheck }.xyz 的安全分析：

    • **风险评分**——总体风险评级（0-100）
    • **增发权限**——是否可以增发新代币？
    • **冻结权限**——是否可以冻结转账？
    • **前几大持有者**——集中度风险

    风险评分越高，表示潜在的危险信号越多。
hints-filtering-meta-title = 元数据过滤器
hints-filtering-meta-content =
    其他过滤条件：

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
    健康指标显示服务状态：

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

    空的代币账户可在设置中清理。

## wallets

hints-wallets-main-title = 主钱包
hints-wallets-main-content =
    用于所有交易操作的主要钱包。

    • **自动交易**——入场/出场交易均由此钱包执行
    • **余额显示**——显示在页眉和仪表盘中
    • **代币持仓**——此钱包持有的 SPL 代币

    在任意副钱包上选择“设为主钱包”即可更换主钱包。
hints-wallets-secondary-title = 副钱包
hints-wallets-secondary-content =
    用于多钱包操作的额外钱包。

    • **多钱包交易**——跨钱包协同买入/卖出
    • **资产分离**——按策略或用途分类管理
    • **独立余额**——每个钱包都有自己的 SOL/代币

    除非明确配置，否则自动交易不会使用副钱包。

## tools

hints-tools-wallet-cleanup-title = 钱包清理工具
hints-tools-wallet-cleanup-content =
    { "*" }*从空代币账户回收 SOL**

    { "*" }*什么是 ATA？**
    关联代币账户（ATA）是持有您代币的 Solana 账户。您每交互一种代币都会创建一个 ATA，并需要约 0.002 SOL 的租金。

    { "*" }*为什么要清理空 ATA？**
    • 回收租金（每个 ATA 约 0.002 SOL）
    • 活跃交易者可能积累数百个空 ATA
    • 100 个空 ATA = 约 0.2 SOL 可回收

    { "*" }*工作原理：**
    • 扫描钱包中余额为零的 ATA
    • 显示可回收的 SOL 总额
    • 关闭空账户以收回租金

    { "*" }*自动清理：**
    启用后，会在后台每 5 分钟自动扫描并关闭空 ATA。

    { "*" }*重要提示：**
    • 仅关闭余额恰好为 0 的账户
    • 关闭失败的账户会被缓存，避免反复重试
    • 大型钱包可能需要多轮清理
hints-tools-burn-tokens-title = 销毁代币工具
hints-tools-burn-tokens-content =
    { "*" }*永久销毁代币**

    销毁代币会将其从您的钱包和流通中永久移除。

    { "*" }*销毁时会发生什么：**
    • 代币被发送到销毁地址（无法恢复）
    • 代币余额变为零
    • 之后可通过钱包清理关闭该 ATA，回收约 0.002 SOL 租金

    { "*" }*代币类别：**
    • **持仓中的仓位** - 无法销毁（活跃交易）
    • **已平仓仓位** - 过往交易的剩余代币
    • **有价值** - 有流动性的代币（建议改为卖出）
    • **零流动性** - 粉尘/无价值代币（可安全销毁）

    { "*" }*警告：** 此操作**不可撤销**。销毁的代币在任何情况下都无法恢复。

    { "*" }*销毁之后：** 运行钱包清理以关闭空 ATA 并回收 SOL 租金。
hints-tools-wallet-generator-title = 钱包生成器工具
hints-tools-wallet-generator-content =
    { "*" }*生成新的 Solana 密钥对**

    在您的设备上安全地创建新钱包。

    { "*" }*功能：**
    • 生成加密安全的密钥对
    • 可选的靓号地址前缀（例如 "SOL..."）
    • 可导出为 base58 或 JSON 数组

    { "*" }*安全：**
    • 密钥在本地生成
    • 绝不通过网络传输
    • 请务必安全备份密钥
hints-tools-multi-buy-title = 批量买入工具
hints-tools-multi-buy-content =
    { "*" }*跨多个钱包协同买入**

    使用随机数量在多个子钱包间执行买入订单，以模拟自然的买入行为。

    { "*" }*工作原理：**
    1. 创建或使用现有子钱包
    2. 将 SOL 从主钱包分发到子钱包
    3. 以随机数量和延迟执行买入订单
    4. 每个钱包独立买入，签名各不相同

    { "*" }*钱包设置：**
    • **钱包数量**——要使用的子钱包数量（2-10）
    • **SOL 缓冲**——每个钱包为手续费预留的 SOL（约 0.015）

    { "*" }*数量设置：**
    • **最小/最大 SOL**——每个钱包的买入数量范围
    • **总限额**——可选的 SOL 总支出上限

    { "*" }*执行设置：**
    • **延迟**——交易之间的随机延迟
    • **并发数**——并行执行（1 = 顺序执行）
    • **滑点**——可接受的最大滑点
    • **路由**——兑换路由（自动、{ -jupiter }、Raydium）

    { "*" }*重要提示：**
    • 需要主钱包中有足够的 SOL
    • 买入失败会被记录，但不会中断整个会话
    • 子钱包可在多个会话中重复使用
hints-tools-multi-sell-title = 批量卖出工具
hints-tools-multi-sell-content =
    { "*" }*跨多个钱包协同卖出**

    从所有持有特定代币的子钱包中卖出该代币，并自动归集 SOL。

    { "*" }*工作原理：**
    1. 扫描子钱包的代币余额
    2. 可选：为 SOL 不足以支付手续费的钱包充值
    3. 按可配置的比例执行卖出订单
    4. 将所得资金归集回主钱包

    { "*" }*卖出设置：**
    • **卖出 %**——要卖出的代币百分比（默认 100%）
    • **手续费所需最小 SOL**——交易所需的最小 SOL
    • **自动充值**——需要时从主钱包转入 SOL

    { "*" }*卖出后操作：**
    • **归集 SOL**——将所有 SOL 转回主钱包
    • **关闭 ATA**——关闭代币账户以回收租金（每个约 0.002 SOL）

    { "*" }*执行设置：**
    • **延迟**——交易之间的随机延迟
    • **并发数**——并行执行
    • **滑点**——可接受的最大滑点
    • **路由**——兑换路由偏好

    { "*" }*小贴士：**
    • 预览会显示所有持有该代币的钱包
    • 取消勾选不想卖出的钱包
    • 归集会在所有卖出完成后进行
hints-tools-trade-watcher-title = 交易监控工具
hints-tools-trade-watcher-content =
    { "*" }*监控交易并触发自动操作**

    关注某个代币的交易活动，并在发生交易时自动做出反应。

    { "*" }*监控类型：**
    • **卖出时买入**——有人卖出时自动买入（抄底）
    • **买入时卖出**——有人买入时自动卖出（跟随市场）
    • **仅通知**——只发送提醒，不执行操作

    { "*" }*工作原理：**
    1. 输入代币铸造地址
    2. 点击“搜索流动性池”查找可用的流动性池
    3. 选择要监控的流动性池（买入/卖出操作必需）
    4. 设置触发数量（需要响应的最小交易规模）
    5. 设置操作数量（买入/卖出多少 SOL）
    6. 开始监控

    { "*" }*要求：**
    • 有效的代币铸造地址
    • 已选择流动性池（用于买入/卖出操作）
    • SOL 余额足以支付操作数量

    { "*" }*{ -telegram } 集成：**
    在配置 → { -telegram } 中配置 { -telegram }，即可在监控触发时收到即时通知。
hints-tools-wallet-consolidation-title = 钱包归集工具
hints-tools-wallet-consolidation-content =
    { "*" }*管理并归集子钱包资金**

    查看所有子钱包，并将 SOL、代币和 ATA 租金归集回您的主钱包。

    { "*" }*摘要显示：**
    • **子钱包**——已创建的子钱包总数
    • **SOL 总额**——所有子钱包的 SOL 余额合计
    • **代币种类**——持有的不同代币数量
    • **可回收租金**——锁定在空 ATA 中的 SOL

    { "*" }*操作：**
    • **转移 SOL**——将所选钱包中的所有 SOL 转入主钱包
    • **转移代币**——将所有代币转入主钱包
    • **清理 ATA**——关闭空代币账户以退回租金

    { "*" }*表格信息：**
    • 复选框用于选择要批量操作的钱包
    • 名称、地址、SOL 余额、代币数量、空 ATA
    • 空钱包会变暗，便于识别

    { "*" }*小贴士：**
    • 在批量卖出后使用，以收集剩余的 SOL
    • 定期清理 ATA 以回收租金
    • 空钱包可在后续操作中重复使用

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
    • 交易监控提醒
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
