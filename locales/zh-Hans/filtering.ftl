# Filter rejection reasons. Message ids derive from the stored rejection codes
# (src/filtering/sources/rejection.rs); rows hold codes, never this text.

filtering-reject-no-decimals = 数据库中没有小数位数
filtering-reject-token-too-new = 代币上线时间过短
filtering-reject-cooldown-filtered = 冷却时间过滤
filtering-reject-dex-data-missing = 缺少 { -dexscreener } 数据
filtering-reject-gecko-data-missing = 缺少 { -geckoterminal } 数据
filtering-reject-rug-data-missing = 缺少 { -rugcheck } 数据
filtering-reject-onchain-numeric-symbol = 符号仅含数字（诈骗）
filtering-reject-onchain-empty-symbol = 符号为空（诈骗）
filtering-reject-onchain-suspicious-symbol = 符号可疑（诈骗）
filtering-reject-onchain-known-scam-authority = 已知诈骗权限
filtering-reject-onchain-immutable-with-freeze = 不可变 + 冻结权限（诈骗）
filtering-reject-onchain-high-risk-score = 链上风险评分过高
filtering-reject-dex-empty-name = 名称为空
filtering-reject-dex-empty-symbol = 符号为空
filtering-reject-dex-empty-logo = Logo URL 为空
filtering-reject-dex-empty-website = 网站 URL 为空
filtering-reject-dex-txn-5m = 5m 交易数过低
filtering-reject-dex-txn-1h = 1h 交易数过低
filtering-reject-dex-zero-liq = 流动性为零
filtering-reject-dex-liq-low = 流动性过低
filtering-reject-dex-liq-high = 流动性过高
filtering-reject-dex-mcap-low = 市值过低
filtering-reject-dex-mcap-high = 市值过高
filtering-reject-dex-vol-low = 成交量过低
filtering-reject-dex-vol-missing = 缺少成交量
filtering-reject-dex-fdv-low = FDV 过低
filtering-reject-dex-fdv-high = FDV 过高
filtering-reject-dex-vol5m-low = 5m 成交量过低
filtering-reject-dex-vol5m-missing = 缺少 5m 成交量
filtering-reject-dex-vol1h-low = 1h 成交量过低
filtering-reject-dex-vol1h-missing = 缺少 1h 成交量
filtering-reject-dex-vol6h-low = 6h 成交量过低
filtering-reject-dex-vol6h-missing = 缺少 6h 成交量
filtering-reject-dex-price-change-5m-low = 5m 价格变化过低
filtering-reject-dex-price-change-5m-high = 5m 价格变化过高
filtering-reject-dex-price-change-low = 价格变化过低
filtering-reject-dex-price-change-high = 价格变化过高
filtering-reject-dex-price-change-6h-low = 6h 价格变化过低
filtering-reject-dex-price-change-6h-high = 6h 价格变化过高
filtering-reject-dex-price-change-24h-low = 24h 价格变化过低
filtering-reject-dex-price-change-24h-high = 24h 价格变化过高
filtering-reject-gecko-liq-low = 流动性过低
filtering-reject-gecko-liq-high = 流动性过高
filtering-reject-gecko-mcap-low = 市值过低
filtering-reject-gecko-mcap-high = 市值过高
filtering-reject-gecko-vol5m-low = 5m 成交量过低
filtering-reject-gecko-vol5m-missing = 缺少 5m 成交量
filtering-reject-gecko-vol1h-low = 1h 成交量过低
filtering-reject-gecko-vol1h-missing = 缺少 1h 成交量
filtering-reject-gecko-vol24h-low = 24h 成交量过低
filtering-reject-gecko-vol24h-missing = 缺少 24h 成交量
filtering-reject-gecko-price-change-5m-low = 5m 价格变化过低
filtering-reject-gecko-price-change-5m-high = 5m 价格变化过高
filtering-reject-gecko-price-change-1h-low = 1h 价格变化过低
filtering-reject-gecko-price-change-1h-high = 1h 价格变化过高
filtering-reject-gecko-price-change-24h-low = 24h 价格变化过低
filtering-reject-gecko-price-change-24h-high = 24h 价格变化过高
filtering-reject-gecko-pool-count-low = 流动性池数量过少
filtering-reject-gecko-pool-count-high = 流动性池数量过多
filtering-reject-gecko-pool-count-missing = 缺少流动性池数量
filtering-reject-gecko-reserve-low = 储备过低
filtering-reject-gecko-reserve-missing = 缺少储备
filtering-reject-rug-rugged = 已跑路的代币
filtering-reject-rug-score = 风险评分过高
filtering-reject-rug-level-danger = 危险风险等级
filtering-reject-rug-mint-authority = 存在铸造权限
filtering-reject-rug-freeze-authority = 存在冻结权限
filtering-reject-rug-top-holder = 最大持有者占比过高
filtering-reject-rug-top3-holders = 前 3 大持有者占比过高
filtering-reject-rug-min-holders = 持有者数量不足
filtering-reject-rug-insider-count = 内部人持有者过多
filtering-reject-rug-insider-pct = 内部人占比过高
filtering-reject-rug-creator-pct = 创建者持仓过高
filtering-reject-rug-transfer-fee-present = 存在转账费用
filtering-reject-rug-transfer-fee-high = 转账费用过高
filtering-reject-rug-graph-insiders = 图谱内部人过多
filtering-reject-rug-lp-providers-low = LP 提供者过少
filtering-reject-rug-lp-providers-missing = 缺少 LP 提供者
filtering-reject-rug-lp-lock-low = LP 锁定比例过低
filtering-reject-rug-lp-lock-missing = 缺少 LP 锁定信息
filtering-reject-llm-analysis-rejected = LLM 分析未通过：{ $reason }（置信度 { $confidence }%，{ $provider }）
filtering-reject-llm-analysis-rejected-generic = LLM 分析未通过
filtering-reject-unknown = { $code }

# Codes no longer emitted; they appear only in stored rows and keep their wording.
filtering-reject-dex-fdv-missing = 缺少 FDV
filtering-reject-dex-price-change-5m-missing = 缺少 5m 价格变化
filtering-reject-dex-price-change-missing = 缺少价格变化
filtering-reject-dex-price-change-6h-missing = 缺少 6h 价格变化
filtering-reject-dex-price-change-24h-missing = 缺少 24h 价格变化
filtering-reject-gecko-liq-missing = 缺少流动性
filtering-reject-gecko-mcap-missing = 缺少市值
filtering-reject-gecko-price-change-5m-missing = 缺少 5m 价格变化
filtering-reject-gecko-price-change-1h-missing = 缺少 1h 价格变化
filtering-reject-gecko-price-change-24h-missing = 缺少 24h 价格变化
filtering-reject-rug-transfer-fee-missing = 缺少转账费用数据

# Rejection categories used to group reasons.
filtering-reject-category-security = 安全问题
filtering-reject-category-distribution = 持有者分布
filtering-reject-category-liquidity-lock = LP 锁定问题
filtering-reject-category-fees = 转账费用
filtering-reject-category-liquidity = 流动性
filtering-reject-category-volume = 交易成交量
filtering-reject-category-market-cap = 市值/FDV
filtering-reject-category-price-action = 价格波动
filtering-reject-category-activity = 交易活跃度
filtering-reject-category-data-quality = 数据缺失
filtering-reject-category-timing = 时间过滤
filtering-reject-category-market = 市场数据
filtering-reject-category-other = 其他

# Filtering page: sub-tabs, sources, status, analytics, explorer and configuration.

## Sub-tabs and sources. Source ids are FilterSource::as_str plus the `meta` settings tab.

filtering-tab-status = 状态
filtering-tab-analytics = 分析
filtering-tab-explorer = 浏览器
filtering-source-core = 核心
filtering-source-onchain = 链上
filtering-source-dexscreener = { -dexscreener }
filtering-source-geckoterminal = { -geckoterminal }
filtering-source-rugcheck = { -rugcheck }
filtering-source-llm-analysis = LLM 分析

## Time range

filtering-range-1h = 1H
filtering-range-6h = 6H
filtering-range-24h = 24H
filtering-range-7d = 7D
filtering-range-all = 全部
filtering-range-all-time = 全部时间
filtering-range-custom = 自定义
filtering-range-now = 现在
# $start and $end are formatted moments, or the open-ended markers.
filtering-range-span = { $start } → { $end }
# $min and $max are the two ends of a value range.
filtering-range-bounds = { $min } – { $max }

## Footer status line

filtering-footer-saving = 正在保存更改...
filtering-footer-refreshing = 正在刷新快照...
filtering-footer-unsaved = 有未保存的更改
# $time is a relative time such as "5m ago".
filtering-footer-last-saved = 上次保存于 { $time }
filtering-footer-in-sync = 配置已同步

## Info bar and status metrics

filtering-info-total = 总计：
filtering-info-priced = 有价格：
filtering-info-passed = 已通过：
filtering-info-positions = 仓位：
filtering-info-blacklisted = 黑名单：
filtering-info-cache = 缓存：
# A count followed by its share of the total, e.g. "120 (4.0%)".
filtering-count-share = { $count }（{ $share }）
filtering-refresh-building = 构建中…
filtering-refresh-never = 从未

filtering-status-loading = 正在加载统计数据...
filtering-status-total = 代币总数
filtering-status-total-detail = 在过滤缓存中
filtering-status-total-detail-building = 快照构建中，数量将在下次刷新时显示
filtering-status-priced = 有价格
filtering-status-priced-detail = { $share } 有价格
filtering-status-passed = 通过过滤器
filtering-status-passed-detail = { $share } 已通过
filtering-status-positions = 持仓中的仓位
filtering-status-positions-detail = 活跃交易
filtering-status-blacklisted = 黑名单
filtering-status-blacklisted-detail = 已标记的代币
filtering-status-ohlcv = 有 OHLCV
filtering-status-ohlcv-detail = 历史数据
filtering-status-refresh = 上次刷新
filtering-status-refresh-building = 首个快照生成中
filtering-status-refresh-none = 尚未刷新
filtering-status-no-rejections = 暂无未通过数据

## Analytics

filtering-analytics-loading = 正在加载 { $range } 的分析数据…
filtering-analytics-scanned = 扫描总数
# $time is a relative time such as "5m ago".
filtering-analytics-updated = 更新于 { $time }
filtering-analytics-passed = 通过的代币
filtering-analytics-pass-rate = 通过率 <strong>{ $share }</strong>
filtering-analytics-rejected = 未通过的代币
filtering-analytics-rejection-rate = 未通过率 <strong>{ $share }</strong>
filtering-analytics-by-category = 按类别统计未通过
filtering-analytics-by-source = 按来源统计未通过
filtering-analytics-no-category = 暂无类别数据
filtering-analytics-no-source = 暂无来源数据
filtering-analytics-top-reasons = 主要未通过原因
filtering-analytics-no-data = 暂无数据
filtering-analytics-column-reason = 原因
filtering-analytics-column-category = 类别
filtering-analytics-column-count = 数量
filtering-analytics-column-share = %
filtering-analytics-column-impact = 影响
# $amount is the formatted count, $count selects the plural.
filtering-tokens-count =
    { $count ->
       *[other] { $amount } 个代币
    }

## Explorer

filtering-explorer-top-reasons = 主要原因
filtering-explorer-recent = 最近未通过
filtering-explorer-none = 暂无数据
filtering-explorer-none-recent = 暂无最近记录
filtering-explorer-search =
    .placeholder = 搜索原因...
filtering-explorer-overview = 概览
filtering-explorer-no-match = 没有匹配的原因
filtering-explorer-column-token = 代币
filtering-explorer-column-source = 来源
filtering-explorer-column-time = 时间
filtering-explorer-page = 第 { $page } 页
filtering-explorer-no-results = 无结果
filtering-explorer-empty = 未找到代币
filtering-explorer-empty-filtered = 未找到符合筛选条件的代币
filtering-explorer-load-failed = 加载代币失败

## Configuration panels

filtering-config-loading = 正在加载配置…
# $query is the text typed in the filter box.
filtering-config-no-match = 没有与“{ $query }”匹配的参数
filtering-config-no-parameters = 此来源没有可用参数
# $source is the source name.
filtering-source-off = { $source } 过滤已关闭，这些参数不会被评估。
filtering-toolbar-filter =
    .placeholder = 筛选参数
    .aria-label = 筛选参数
filtering-toolbar-clear =
    .aria-label = 清除筛选
# $count selects the plural, $amount is the number shown.
filtering-parameter-count =
    { $count ->
       *[other] { $amount } 个参数
    }
# $count is the total and selects the plural.
filtering-parameter-count-filtered =
    { $count ->
       *[other] { $visible } / { $total } 个参数
    }
filtering-group-enable =
    .aria-label = 启用 { $group } 检查
filtering-field-min = 最小
filtering-field-max = 最大
# $label is the parameter name.
filtering-field-min-aria =
    .aria-label = 最小 { $label }
filtering-field-max-aria =
    .aria-label = 最大 { $label }
# $default is the shipped value, $label the parameter name.
filtering-field-reset =
    .title = 重置为默认值（{ $default }）
    .aria-label = 将 { $label } 重置为默认值

## Toasts. A message value is the title; `.message` is the body.

filtering-toast-saved = 配置已保存
    .message = 过滤设置已保存，快照已刷新
filtering-toast-save-failed = 保存失败
    .message = 保存过滤配置失败
filtering-toast-reset = 更改已重置
    .message = 配置已恢复到上次保存的状态
filtering-toast-refresh-failed = 刷新失败
    .message = 刷新过滤快照失败
filtering-toast-exported = 配置已导出
    .message = 过滤设置已保存到文件
filtering-toast-imported = 配置已导入
    .message = 已从文件加载过滤设置
filtering-toast-import-failed = 导入失败
    .message = 导入配置失败：文件格式无效
filtering-toast-load-failed = 加载失败
    .message = 加载过滤配置失败
filtering-toast-range-missing = 请同时选择开始和结束日期
filtering-toast-range-order = 开始时间必须早于结束时间
filtering-toast-range-future = 结束时间不能晚于当前时间
